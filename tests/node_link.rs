mod common;

use std::sync::Arc;

use common::*;
use gns3fy::{
    ConsoleType, Error, Gns3Connector, Link, LinkType, Node, NodeStatus, NodeType, Port,
};
use serde_json::{json, Value};

fn conn(server: &MockServer) -> Arc<Gns3Connector> {
    Arc::new(Gns3Connector::new(&server.url).unwrap())
}

fn alpine() -> Value {
    json("nodes.json")[4].clone()
}

fn with_status(status: &str) -> String {
    let mut v = alpine();
    v["status"] = json!(status);
    v.to_string()
}

fn node_path() -> String {
    format!("/v2/projects/{PROJECT_ID}/nodes/{ALPINE_ID}")
}

fn links_of_alpine() -> String {
    json!([link_json()]).to_string()
}

// ---------------------------------------------------------------- Node

#[test]
fn node_get_resolves_id_from_name_and_loads_links() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let server = Routes::new()
        .on("GET", &format!("{p}/nodes"), 200, data("nodes.json"))
        .on("GET", &node_path(), 200, alpine().to_string())
        .on("GET", &format!("{}/links", node_path()), 200, links_of_alpine())
        .start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_name("alpine-1");
    node.get().unwrap();

    assert_eq!(node.node_id.as_deref(), Some(ALPINE_ID));
    assert_eq!(node.node_type, Some(NodeType::Docker));
    assert_eq!(node.status, Some(NodeStatus::Started));
    assert_eq!(node.console, Some(5005));
    assert_eq!(node.console_type, Some(ConsoleType::Telnet));
    assert_eq!(node.ports.as_ref().unwrap().len(), 2);
    assert_eq!(node.port("eth1").unwrap().adapter_number, 1);
    assert!(node.port("nope").is_none());
    assert_eq!(node.links.len(), 1);
    assert_eq!(node.links[0].link_id.as_deref(), Some(LINK_ID));
    assert!(node.links[0].connector.is_some());
    // the connector survives the update
    assert!(node.connector.is_some());
}

#[test]
fn node_get_without_links() {
    let server = Routes::new().on("GET", &node_path(), 200, alpine().to_string()).start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    node.get_with_links(false).unwrap();
    assert_eq!(server.calls(), vec![format!("GET {}", node_path())]);
}

#[test]
fn node_precondition_errors() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let server = Routes::new()
        .on("GET", &format!("{p}/nodes"), 200, json!([
            {"name": "dup", "node_id": "1"}, {"name": "dup", "node_id": "2"}
        ]).to_string())
        .start();
    let c = conn(&server);

    assert!(matches!(Node::default().get(), Err(Error::MissingConnector)));
    assert!(matches!(
        Node::with_connector(c.clone()).with_name("x").get(),
        Err(Error::InvalidInput(m)) if m == "Need to submit project_id"
    ));
    assert!(matches!(
        Node::with_connector(c.clone()).with_project_id(PROJECT_ID).get(),
        Err(Error::InvalidInput(m)) if m == "Need to either submit node_id or name"
    ));
    assert!(matches!(
        Node::with_connector(c.clone()).with_project_id(PROJECT_ID).with_name("dup").get(),
        Err(Error::InvalidInput(m)) if m.starts_with("Multiple nodes found")
    ));
    assert!(matches!(
        Node::with_connector(c).with_project_id(PROJECT_ID).with_name("none").get(),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn node_power_actions_update_status() {
    let server = Routes::new()
        .on("POST", &format!("{}/stop", node_path()), 200, with_status("stopped"))
        .on("POST", &format!("{}/start", node_path()), 200, with_status("started"))
        .on("POST", &format!("{}/suspend", node_path()), 200, with_status("suspended"))
        .on("POST", &format!("{}/reload", node_path()), 200, with_status("started"))
        .start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    node.stop().unwrap();
    assert_eq!(node.status, Some(NodeStatus::Stopped));
    node.start().unwrap();
    assert_eq!(node.status, Some(NodeStatus::Started));
    node.suspend().unwrap();
    assert_eq!(node.status, Some(NodeStatus::Suspended));
    node.reload().unwrap();
    assert_eq!(node.status, Some(NodeStatus::Started));
    // answers already carried the expected status, so no extra GET was needed
    assert_eq!(server.count("GET", &node_path()), 0);
}

#[test]
fn node_action_refetches_when_status_is_not_final() {
    let server = Routes::new()
        .on("POST", &format!("{}/start", node_path()), 200, with_status("stopped"))
        .on("GET", &node_path(), 200, with_status("started"))
        .on("GET", &format!("{}/links", node_path()), 200, "[]")
        .start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    node.start().unwrap();
    assert_eq!(node.status, Some(NodeStatus::Started));
    assert_eq!(server.count("GET", &node_path()), 1);
}

#[test]
fn node_update_and_delete() {
    let mut moved = alpine();
    moved["x"] = json!(42);
    let server = Routes::new()
        .on("PUT", &node_path(), 200, moved.to_string())
        .on("DELETE", &node_path(), 204, "")
        .start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    node.update(json!({"x": 42})).unwrap();
    assert_eq!(node.x, Some(42));
    assert_eq!(server.last_json("PUT", &node_path()), json!({"x": 42}));
    node.delete().unwrap();
    assert!(node.project_id.is_none() && node.node_id.is_none() && node.name.is_none());
}

#[test]
fn node_create_from_template_name() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let fresh = json!({"name": "Alpine-9", "node_id": "n9", "node_type": "docker",
                       "status": "stopped", "compute_id": "local"});
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("POST", &format!("{p}/templates/{TEMPLATE_ID}"), 201, fresh.to_string())
        .on("PUT", &format!("{p}/nodes/n9"), 200, fresh.to_string())
        .start();
    let mut node = Node::with_connector(conn(&server))
        .with_project_id(PROJECT_ID)
        .with_name("my-alpine")
        .with_template("alpine");
    node.x = Some(10);
    node.create().unwrap();

    assert_eq!(node.template_id.as_deref(), Some(TEMPLATE_ID));
    assert_eq!(node.node_id.as_deref(), Some("n9"));
    assert_eq!(
        server.last_json("POST", &format!("{p}/templates/{TEMPLATE_ID}")),
        json!({"x": 0, "y": 0, "compute_id": "local"})
    );
    assert_eq!(
        server.last_json("PUT", &format!("{p}/nodes/n9")),
        json!({"name": "my-alpine", "compute_id": "local", "x": 10})
    );

    // creating twice is rejected
    assert!(matches!(node.create(), Err(Error::InvalidInput(m)) if m == "Node already created"));
}

#[test]
fn node_create_errors() {
    let server = Routes::new().on("GET", "/v2/templates", 200, data("templates.json")).start();
    let c = conn(&server);
    let base = || Node::with_connector(c.clone()).with_project_id(PROJECT_ID).with_name("n");
    assert!(matches!(base().create(), Err(Error::InvalidInput(m)) if m == "Need either 'template' of 'template_id'"));
    assert!(matches!(base().with_template("ghost").create(), Err(Error::InvalidInput(m)) if m == "Template ghost not found"));
    let mut no_project = Node::with_connector(c.clone()).with_template("alpine");
    assert!(matches!(no_project.create(), Err(Error::InvalidInput(_))));
    assert!(matches!(Node::default().create(), Err(Error::MissingConnector)));
}

#[test]
fn node_files() {
    let f = format!("{}/files/config/start.sh", node_path());
    let server = Routes::new()
        .on("GET", &f, 200, "echo hi")
        .on("POST", &f, 201, "")
        .start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    assert_eq!(node.get_file("config/start.sh").unwrap(), "echo hi");
    node.write_file("config/start.sh", b"echo bye".to_vec()).unwrap();
    assert_eq!(server.recorded().last().unwrap().body, "echo bye");
}

// ---------------------------------------------------------------- Link

fn link_json() -> Value {
    json("links.json")
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["link_id"] == LINK_ID)
        .unwrap()
        .clone()
}

fn link_path() -> String {
    format!("/v2/projects/{PROJECT_ID}/links/{LINK_ID}")
}

#[test]
fn link_get_update_delete() {
    let mut suspended = link_json();
    suspended["suspend"] = json!(true);
    let server = Routes::new()
        .on("GET", &link_path(), 200, link_json().to_string())
        .on("PUT", &link_path(), 200, suspended.to_string())
        .on("DELETE", &link_path(), 204, "")
        .start();
    let mut link = Link::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_link_id(LINK_ID);

    link.get().unwrap();
    assert_eq!(link.link_type, Some(LinkType::Ethernet));
    assert_eq!(link.suspend, Some(false));
    assert_eq!(link.capturing, Some(false));
    let nodes = link.nodes.as_ref().unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].node_id, ALPINE_ID);
    assert_eq!(link.link_style.as_ref().unwrap()["color"], "#75507b");

    link.update(json!({"suspend": true})).unwrap();
    assert_eq!(link.suspend, Some(true));
    assert_eq!(server.last_json("PUT", &link_path()), json!({"suspend": true}));

    link.delete().unwrap();
    assert!(link.project_id.is_none() && link.link_id.is_none());
    assert!(matches!(link.get(), Err(Error::InvalidInput(_))));
}

#[test]
fn link_create_posts_nodes_and_project() {
    let server = Routes::new()
        .on("POST", &format!("/v2/projects/{PROJECT_ID}/links"), 201, link_json().to_string())
        .start();
    let mut link = Link::with_connector(conn(&server)).with_project_id(PROJECT_ID);
    link.nodes = Some(serde_json::from_value(link_json()["nodes"].clone()).unwrap());
    link.create().unwrap();
    assert_eq!(link.link_id.as_deref(), Some(LINK_ID));

    let body = server.last_json("POST", &format!("/v2/projects/{PROJECT_ID}/links"));
    assert_eq!(body["project_id"], PROJECT_ID);
    assert_eq!(body["nodes"].as_array().unwrap().len(), 2);
    assert!(body.get("link_id").is_none(), "unset attributes are not sent");
    assert!(body.get("connector").is_none());
}

#[test]
fn link_preconditions() {
    assert!(matches!(Link::default().get(), Err(Error::MissingConnector)));
    assert!(matches!(Link::default().create(), Err(Error::MissingConnector)));
    let server = Routes::new().start();
    let mut l = Link::with_connector(conn(&server)).with_project_id(PROJECT_ID);
    assert!(matches!(l.get(), Err(Error::InvalidInput(m)) if m == "Need to submit link_id"));
}

// ---------------------------------------------------------------- types / validation

#[test]
fn invalid_enum_values_are_rejected() {
    let bad_type = serde_json::from_value::<Node>(json!({"node_type": "toaster"}));
    assert!(bad_type.is_err());
    let bad_console = serde_json::from_value::<Node>(json!({"console_type": "smoke-signals"}));
    assert!(bad_console.is_err());
    let bad_status = serde_json::from_value::<Node>(json!({"status": "exploded"}));
    assert!(bad_status.is_err());
    let bad_link = serde_json::from_value::<Link>(json!({"link_type": "quantum"}));
    assert!(bad_link.is_err());
    // a bad value coming back from the server surfaces as an error, not silent corruption
    let server = Routes::new().on("PUT", &node_path(), 200, r#"{"status": "exploded"}"#).start();
    let mut node = Node::with_connector(conn(&server)).with_project_id(PROJECT_ID).with_node_id(ALPINE_ID);
    node.status = Some(NodeStatus::Started);
    assert!(matches!(node.update(json!({"x": 1})), Err(Error::Json(_))));
    assert_eq!(node.status, Some(NodeStatus::Started), "failed update leaves the node untouched");
    assert!(node.connector.is_some());
}

#[test]
fn enum_wire_names() {
    assert_eq!(serde_json::to_value(ConsoleType::SpiceAgent).unwrap(), json!("spice+agent"));
    assert_eq!(serde_json::to_value(ConsoleType::NoConsole).unwrap(), json!("none"));
    assert_eq!(serde_json::from_value::<ConsoleType>(json!("null")).unwrap(), ConsoleType::Null);
    assert_eq!(serde_json::to_value(NodeType::EthernetSwitch).unwrap(), json!("ethernet_switch"));
    assert_eq!(serde_json::to_value(NodeType::FrameRelaySwitch).unwrap(), json!("frame_relay_switch"));
    assert_eq!(NodeType::Qemu.to_string(), "qemu");
    assert_eq!(NodeStatus::Suspended.to_string(), "suspended");
}

#[test]
fn every_python_node_and_console_type_is_accepted() {
    for t in [
        "cloud", "nat", "ethernet_hub", "ethernet_switch", "frame_relay_switch", "atm_switch",
        "docker", "dynamips", "vpcs", "traceng", "virtualbox", "vmware", "iou", "qemu",
    ] {
        let n: NodeType = serde_json::from_value(json!(t)).unwrap();
        assert_eq!(n.as_str(), t);
    }
    for t in ["vnc", "telnet", "http", "https", "spice", "spice+agent", "none", "null"] {
        let c: ConsoleType = serde_json::from_value(json!(t)).unwrap();
        assert_eq!(c.as_str(), t);
    }
}

#[test]
fn every_fixture_node_and_link_deserializes_and_ports_keep_unknown_fields() {
    for n in json("nodes.json").as_array().unwrap() {
        let node: Node = serde_json::from_value(n.clone()).unwrap();
        assert_eq!(node.name.as_deref(), n["name"].as_str());
        assert_eq!(node.compute_id, "local");
    }
    for l in json("links.json").as_array().unwrap() {
        serde_json::from_value::<Link>(l.clone()).unwrap();
    }
    let port: Port = serde_json::from_value(json!({
        "name": "e0", "port_number": 0, "adapter_number": 1, "mac_address": "aa:bb"
    })).unwrap();
    assert_eq!(port.extra["mac_address"], "aa:bb");
    assert_eq!(serde_json::to_value(&port).unwrap()["mac_address"], "aa:bb");
}
