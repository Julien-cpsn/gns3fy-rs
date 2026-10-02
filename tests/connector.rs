mod common;

use common::*;
use gns3fy::{Error, Gns3Connector, Lookup, LOCAL_COMPUTE};
use serde_json::json;

fn connector(server: &MockServer) -> Gns3Connector {
    Gns3Connector::new(&server.url).unwrap()
}

fn link_by_id(id: &str) -> serde_json::Value {
    json("links.json")
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["link_id"] == id)
        .unwrap()
        .clone()
}

#[test]
fn base_url_is_normalised() {
    let c = Gns3Connector::new("http://gns3server:3080/").unwrap();
    assert_eq!(c.base_url(), "http://gns3server:3080/v2");
    let c = Gns3Connector::builder("http://gns3server:3080").api_version(3).build().unwrap();
    assert_eq!(c.base_url(), "http://gns3server:3080/v3");
}

#[test]
fn version_and_call_counter() {
    let server = Routes::new().on("GET", "/v2/version", 200, data("version.json")).start();
    let c = connector(&server);
    let v = c.get_version().unwrap();
    assert!(v.version.starts_with("2."));
    assert_eq!(c.api_calls(), 1);
}

#[test]
fn api_errors_use_server_status_and_message() {
    let server = Routes::new()
        .on("GET", "/v2/projects/nope", 404, r#"{"status": 404, "message": "Project 'nope' not found"}"#)
        .on("GET", "/v2/version", 500, "plain text failure")
        .start();
    let c = connector(&server);
    match c.get_project(Lookup::Id("nope")).unwrap_err() {
        Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Project 'nope' not found");
        }
        other => panic!("unexpected {other:?}"),
    }
    match c.get_version().unwrap_err() {
        Error::Api { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "plain text failure");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn basic_auth_header_is_sent() {
    let server = Routes::new().on("GET", "/v2/version", 200, data("version.json")).start();
    let c = Gns3Connector::builder(&server.url).user("admin").cred("secret").build().unwrap();
    c.get_version().unwrap();
    // base64("admin:secret")
    assert_eq!(
        server.recorded()[0].authorization.as_deref(),
        Some("Basic YWRtaW46c2VjcmV0")
    );
    // no credentials, no header
    let anon = connector(&server);
    anon.get_version().unwrap();
    assert_eq!(server.recorded()[1].authorization, None);
}

#[test]
fn projects_summary_uses_stats() {
    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("GET", "/v2/projects/c9dc56bf-37b9-453b-8f95-2845ce8908e3/stats", 200, r#"{"drawings":0,"links":9,"nodes":10,"snapshots":0}"#)
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/stats"), 200, r#"{"drawings":2,"links":4,"nodes":6,"snapshots":2}"#)
        .start();
    let summary = connector(&server).projects_summary().unwrap();
    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].name, "test2");
    assert_eq!((summary[0].total_nodes, summary[0].total_links), (10, 9));
    assert_eq!(summary[1].project_id, PROJECT_ID);
    assert_eq!(
        summary[1].to_string(),
        format!("API_TEST: {PROJECT_ID} -- Nodes: 6 -- Links: 4 -- Status: opened")
    );
}

#[test]
fn get_project_by_name_and_id() {
    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}"), 200, json("projects.json")[1].to_string())
        .start();
    let c = connector(&server);
    let by_name = c.get_project(Lookup::Name("API_TEST")).unwrap().unwrap();
    assert_eq!(by_name["project_id"], PROJECT_ID);
    assert!(c.get_project(Lookup::Name("missing")).unwrap().is_none());
    let by_id = c.get_project(Lookup::Id(PROJECT_ID)).unwrap().unwrap();
    assert_eq!(by_id["name"], "API_TEST");
}

#[test]
fn create_and_delete_project() {
    let server = Routes::new()
        .on("POST", "/v2/projects", 201, json("projects.json")[1].to_string())
        .on("DELETE", &format!("/v2/projects/{PROJECT_ID}"), 204, "")
        .start();
    let c = connector(&server);
    assert!(matches!(c.create_project(json!({"zoom": 1})), Err(Error::InvalidInput(_))));
    let created = c.create_project(json!({"name": "API_TEST"})).unwrap();
    assert_eq!(created["name"], "API_TEST");
    assert_eq!(server.last_json("POST", "/v2/projects"), json!({"name": "API_TEST"}));
    c.delete_project(PROJECT_ID).unwrap();
}

#[test]
fn templates() {
    let templates = data("templates.json");
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, templates.clone())
        .on("GET", &format!("/v2/templates/{TEMPLATE_ID}"), 200, json("templates.json")[3].to_string())
        .on("PUT", &format!("/v2/templates/{TEMPLATE_ID}"), 200, json("templates.json")[3].to_string())
        .on("POST", "/v2/templates", 201, r#"{"name": "new", "template_id": "x"}"#)
        .on("DELETE", &format!("/v2/templates/{TEMPLATE_ID}"), 204, "")
        .start();
    let c = connector(&server);

    let summary = c.templates_summary().unwrap();
    assert_eq!(summary.len(), 11);
    assert_eq!(summary[0].name, "IOU-L3");

    let alpine = c.get_template(Lookup::Name("alpine")).unwrap().unwrap();
    assert_eq!(alpine["template_id"], TEMPLATE_ID);
    assert!(c.get_template(Lookup::Name("nope")).unwrap().is_none());
    assert!(c.get_template(Lookup::Id(TEMPLATE_ID)).unwrap().is_some());

    c.update_template(Lookup::Name("alpine"), json!({"ram": 512})).unwrap();
    assert_eq!(server.last_json("PUT", &format!("/v2/templates/{TEMPLATE_ID}"))["ram"], 512);

    assert!(matches!(
        c.create_template(json!({"name": "alpine", "template_type": "docker"})),
        Err(Error::InvalidInput(m)) if m == "Template already used: alpine"
    ));
    assert!(matches!(c.create_template(json!({})), Err(Error::InvalidInput(_))));
    c.create_template(json!({"name": "new", "template_type": "docker"})).unwrap();
    // compute_id defaults to local
    assert_eq!(server.last_json("POST", "/v2/templates")["compute_id"], LOCAL_COMPUTE);

    c.delete_template(Lookup::Name("alpine")).unwrap();
    c.delete_template(Lookup::Id(TEMPLATE_ID)).unwrap();
    assert_eq!(server.count("DELETE", &format!("/v2/templates/{TEMPLATE_ID}")), 2);
    assert!(matches!(c.delete_template(Lookup::Name("ghost")), Err(Error::NotFound(_))));
}

#[test]
fn nodes_and_links_raw() {
    let server = Routes::new()
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/nodes"), 200, data("nodes.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/nodes/{ALPINE_ID}"), 200, json("nodes.json")[4].to_string())
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/links"), 200, data("links.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/links/{LINK_ID}"), 200, link_by_id(LINK_ID).to_string())
        .start();
    let c = connector(&server);
    assert_eq!(c.get_nodes(PROJECT_ID).unwrap().len(), 6);
    assert_eq!(c.get_node(PROJECT_ID, ALPINE_ID).unwrap()["name"], "alpine-1");
    assert_eq!(c.get_links(PROJECT_ID).unwrap().len(), 7);
    assert_eq!(c.get_link(PROJECT_ID, LINK_ID).unwrap()["link_id"], LINK_ID);
}

#[test]
fn computes() {
    let server = Routes::new()
        .on("GET", "/v2/computes", 200, data("computes.json"))
        .on("GET", "/v2/computes/local", 200, json("computes.json")[0].to_string())
        .on("GET", "/v2/computes/local/qemu/images", 200, data("compute_qemu_images.json"))
        .on("GET", "/v2/computes/local/ports", 200, data("compute_ports.json"))
        .on("POST", "/v2/computes/local/qemu/images/disk.qcow2", 204, "")
        .start();
    let c = connector(&server);
    assert_eq!(c.get_computes().unwrap().len(), 1);
    assert_eq!(c.get_compute(LOCAL_COMPUTE).unwrap()["compute_id"], "local");
    let images = c.get_compute_images("qemu", LOCAL_COMPUTE).unwrap();
    assert_eq!(images.len(), json("compute_qemu_images.json").as_array().unwrap().len());
    assert_eq!(images[0]["filename"], "cumulus-linux-3.7.8-vx-amd64-qemu.qcow2");
    assert_eq!(c.get_compute_ports(LOCAL_COMPUTE).unwrap()["console_port_range"], json!([5000, 10000]));

    // upload streams the file content
    let dir = std::env::temp_dir().join(format!("gns3fy-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("disk.qcow2");
    std::fs::write(&file, b"QCOW-DATA").unwrap();
    c.upload_compute_image("qemu", &file, LOCAL_COMPUTE).unwrap();
    let rec = server.recorded();
    let up = rec.iter().find(|r| r.method == "POST").unwrap();
    assert_eq!(up.body, "QCOW-DATA");
    assert!(matches!(
        c.upload_compute_image("qemu", dir.join("missing.img"), LOCAL_COMPUTE),
        Err(Error::Io(_))
    ));
    std::fs::remove_dir_all(dir).unwrap();
}
