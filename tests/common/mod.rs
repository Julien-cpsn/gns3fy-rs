#![allow(dead_code)]
//! Tiny in-process HTTP mock of a GNS3 server, so tests need no extra dependencies.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: String,
    pub authorization: Option<String>,
    pub content_type: Option<String>,
}

struct Route {
    method: String,
    path: String,
    responses: Vec<(u16, String)>,
    hits: usize,
}

pub struct MockServer {
    /// e.g. `http://127.0.0.1:54321`
    pub url: String,
    pub requests: Arc<Mutex<Vec<Recorded>>>,
}

#[derive(Default)]
pub struct Routes(Vec<Route>);

impl Routes {
    pub fn new() -> Self {
        Routes(Vec::new())
    }

    /// Answer `method path` with `status` and `body` (every time).
    pub fn on(mut self, method: &str, path: &str, status: u16, body: impl Into<String>) -> Self {
        self.0.push(Route {
            method: method.into(),
            path: path.into(),
            responses: vec![(status, body.into())],
            hits: 0,
        });
        self
    }

    /// Answer successive calls with successive responses (the last one repeats).
    pub fn seq(mut self, method: &str, path: &str, responses: Vec<(u16, String)>) -> Self {
        self.0.push(Route {
            method: method.into(),
            path: path.into(),
            responses,
            hits: 0,
        });
        self
    }

    pub fn start(self) -> MockServer {
        MockServer::start(self.0)
    }
}

impl MockServer {
    fn start(routes: Vec<Route>) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let routes = Arc::new(Mutex::new(routes));
        let recorded = requests.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                handle(stream, &routes, &recorded);
            }
        });
        MockServer { url, requests }
    }

    pub fn recorded(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    /// All `METHOD path` pairs seen so far.
    pub fn calls(&self) -> Vec<String> {
        self.recorded()
            .iter()
            .map(|r| format!("{} {}", r.method, r.path))
            .collect()
    }

    /// JSON body of the last request matching `METHOD path`.
    pub fn last_json(&self, method: &str, path: &str) -> Value {
        let r = self
            .recorded()
            .into_iter()
            .rev()
            .find(|r| r.method == method && r.path == path)
            .unwrap_or_else(|| panic!("no request {method} {path}; saw {:?}", self.calls()));
        serde_json::from_str(&r.body).unwrap_or(Value::Null)
    }

    pub fn count(&self, method: &str, path: &str) -> usize {
        self.recorded()
            .iter()
            .filter(|r| r.method == method && r.path == path)
            .count()
    }
}

fn handle(mut stream: TcpStream, routes: &Arc<Mutex<Vec<Route>>>, recorded: &Arc<Mutex<Vec<Recorded>>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("").to_string();
    let target = first.next().unwrap_or("").to_string();
    let path = target.split('?').next().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let header = |name: &str| headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    let content_length = header("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let authorization = header("authorization");
    let content_type = header("content-type");
    while buf.len() < header_end + content_length {
        let n = stream.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&buf[header_end..]).to_string();
    recorded.lock().unwrap().push(Recorded {
        method: method.clone(),
        path: path.clone(),
        body,
        authorization,
        content_type,
    });

    let (status, payload) = {
        let mut routes = routes.lock().unwrap();
        match routes.iter_mut().find(|r| r.method == method && r.path == path) {
            Some(r) => {
                let i = r.hits.min(r.responses.len() - 1);
                r.hits += 1;
                r.responses[i].clone()
            }
            None => (
                404,
                format!(r#"{{"status": 404, "message": "no mock route for {method} {path}"}}"#),
            ),
        }
    };
    let reason = if status < 300 { "OK" } else { "Error" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

/// Raw text of a fixture file.
pub fn data(name: &str) -> String {
    std::fs::read_to_string(data_path(name)).unwrap()
}

/// Parsed fixture, as raw JSON. Only meant to describe the *wire format* (mock answers,
/// expected request bodies); use [`load`] to get typed fixtures.
pub fn json(name: &str) -> Value {
    serde_json::from_str(&data(name)).unwrap()
}

/// Typed fixture.
pub fn load<T: serde::de::DeserializeOwned>(name: &str) -> T {
    serde_json::from_str(&data(name)).unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

/// A typed value as the JSON text a mock server answers with.
pub fn body<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

pub const PROJECT_ID: &str = "4b21dfb3-675a-4efa-8613-2f7fb32e76fe";
pub const ALPINE_ID: &str = "ef503c45-e998-499d-88fc-2765614b313e";
pub const LINK_ID: &str = "4d9f1235-7fd1-466b-ad26-0b4b08beb778";
pub const TEMPLATE_ID: &str = "847e5333-6ac9-411f-a400-89838584371b";
