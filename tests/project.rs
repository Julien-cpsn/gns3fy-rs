mod common;

use std::sync::Arc;
use std::time::Duration;

use common::*;
use gns3fy_rs::{ConsoleType, Error, Gns3Connector, Lookup, NodeStatus, NodeType, Project, ProjectStatus, ProjectUpdate};
use serde_json::json;

const ZERO: Duration = Duration::from_millis(0);
const SWITCH_ID: &str = "da28e1c0-9465-4f7c-b42c-49b2f4e1c64d";

fn api_test_project() -> serde_json::Value {
    json("projects.json")
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "API_TEST")
        .unwrap()
        .clone()
}

/// A mock server with all the read endpoints of the API_TEST project.
fn lab_routes() -> Routes {
    let p = format!("/v2/projects/{PROJECT_ID}");
    Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("GET", &p, 200, api_test_project().to_string())
        .on("GET", &format!("{p}/stats"), 200, r#"{"drawings":2,"links":7,"nodes":6,"snapshots":2}"#)
        .on("GET", &format!("{p}/snapshots"), 200, data("project_snapshots.json"))
        .on("GET", &format!("{p}/drawings"), 200, data("project_drawings.json"))
        .on("GET", &format!("{p}/nodes"), 200, data("nodes.json"))
        .on("GET", &format!("{p}/links"), 200, data("links.json"))
}

fn project(server: &MockServer) -> Project {
    let conn = Arc::new(Gns3Connector::new(&server.url).unwrap());
    Project::with_connector(conn).with_name("API_TEST")
}

#[tokio::test]
pub async fn get_loads_project_stats_snapshots_drawings_nodes_links() {
    let server = lab_routes().start();
    let mut lab = project(&server);
    lab.get().await.unwrap();

    assert_eq!(lab.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(lab.status, Some(ProjectStatus::Opened));
    let stats = lab.stats.clone().unwrap();
    assert_eq!((stats.nodes, stats.links, stats.snapshots, stats.drawings), (6, 7, 2, 2));
    assert_eq!(lab.snapshots.as_ref().unwrap().len(), 2);
    assert_eq!(lab.drawings.as_ref().unwrap().len(), 2);
    assert_eq!(lab.nodes.len(), 6);
    assert_eq!(lab.links.len(), 7);
    // children inherit the connector and project id
    assert!(lab.nodes.iter().all(|n| n.connector.is_some() && n.project_id.as_deref() == Some(PROJECT_ID)));
    assert!(lab.links.iter().all(|l| l.connector.is_some() && l.project_id.as_deref() == Some(PROJECT_ID)));
    let alpine = lab.get_node(Lookup::Name("alpine-1")).await.unwrap().unwrap();
    assert_eq!(alpine.node_type, Some(NodeType::Docker));
    assert_eq!(alpine.console_type, Some(ConsoleType::Telnet));
}

#[tokio::test]
pub async fn get_can_skip_related_objects() {
    let server = lab_routes().start();
    let mut lab = project(&server);
    lab.get_with(false, false, false).await.unwrap();
    assert!(lab.nodes.is_empty() && lab.links.is_empty() && lab.stats.is_none());
    assert_eq!(server.count("GET", &format!("/v2/projects/{PROJECT_ID}/nodes")), 0);
}

#[tokio::test]
pub async fn get_errors() {
    let server = lab_routes().start();
    let conn = Arc::new(Gns3Connector::new(&server.url).unwrap());

    let mut no_conn = Project::default().with_name("x");
    assert!(matches!(no_conn.get().await, Err(Error::MissingConnector)));

    let mut no_ident = Project::with_connector(conn.clone());
    assert!(matches!(no_ident.get().await, Err(Error::InvalidInput(_))));

    let mut unknown = Project::with_connector(conn).with_name("does-not-exist");
    assert!(matches!(unknown.get().await, Err(Error::NotFound(_))));
}

#[tokio::test]
pub async fn summaries_and_inventory() {
    let server = lab_routes().start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);

    let nodes = lab.nodes_summary().await.unwrap();
    assert_eq!(nodes.len(), 6);
    assert_eq!(nodes[0].name.as_deref(), Some("Ethernetswitch-1"));
    assert_eq!(nodes[0].console, Some(5000));
    assert_eq!(nodes[0].status.as_deref(), Some("started"));
    assert_eq!(nodes[0].to_string(), format!("Ethernetswitch-1: started -- Console: 5000 -- ID: {SWITCH_ID}"));
    assert_eq!(nodes[5].console, None); // Cloud-1 has no console

    let inv = lab.nodes_inventory().await.unwrap();
    assert_eq!(inv.len(), 6);
    let alpine = &inv["alpine-1"];
    assert_eq!(alpine.server.as_deref(), Some("127.0.0.1"));
    assert_eq!(alpine.console_port, Some(5005));
    assert_eq!(alpine.console_type, Some(ConsoleType::Telnet));
    assert_eq!(alpine.node_type, Some(NodeType::Docker));
    let as_json = serde_json::to_value(alpine).unwrap();
    assert_eq!(as_json["type"], "docker");

    let links = lab.links_summary().await.unwrap();
    // 7 links in the fixture, 2 of them have no endpoints
    let rows: Vec<_> = links
        .iter()
        .map(|l| (l.node_a.as_str(), l.port_a.as_str(), l.node_b.as_str(), l.port_b.as_str()))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("IOU1", "Ethernet0/0", "Ethernetswitch-1", "Ethernet1"),
            ("IOU1", "Ethernet1/0", "IOU2", "Ethernet1/0"),
            ("vEOS", "Management1", "Ethernetswitch-1", "Ethernet0"),
            ("vEOS", "Ethernet1", "alpine-1", "eth0"),
            ("Cloud-1", "eth1", "Ethernetswitch-1", "Ethernet7"),
        ]
    );
    assert_eq!(links[3].to_string(), "vEOS: Ethernet1 ---- alpine-1: eth0");
}

#[tokio::test]
pub async fn get_node_by_id_and_unknown() {
    let server = lab_routes().start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);
    assert_eq!(lab.get_node(Lookup::Id(ALPINE_ID)).await.unwrap().unwrap().name.as_deref(), Some("alpine-1"));
    assert!(lab.get_node(Lookup::Name("ghost")).await.unwrap().is_none());
    assert!(lab.get_link(LINK_ID).await.unwrap().is_some());
    assert!(lab.get_link("nope").await.unwrap().is_none());
}

#[tokio::test]
pub async fn lifecycle_open_close_update_delete() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let closed = {
        let mut v = api_test_project();
        v["status"] = json!("closed");
        v
    };
    let server = Routes::new()
        .on("POST", &format!("{p}/close"), 204, "")
        .on("POST", &format!("{p}/open"), 201, closed.to_string().replace("closed", "opened"))
        .on("PUT", &p, 200, {
            let mut v = api_test_project();
            v["auto_close"] = json!(true);
            v.to_string()
        })
        .on("DELETE", &p, 204, "")
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);

    lab.close().await.unwrap();
    assert_eq!(lab.status, Some(ProjectStatus::Closed));
    lab.open().await.unwrap();
    assert_eq!(lab.status, Some(ProjectStatus::Opened));
    let patch = ProjectUpdate {
        auto_close: Some(true),
        ..Default::default()
    };
    lab.update(&patch).await.unwrap();
    assert_eq!(lab.auto_close, Some(true));
    assert_eq!(server.last_json("PUT", &p), json!({"auto_close": true}));
    lab.delete().await.unwrap();
    assert!(lab.project_id.is_none() && lab.name.is_none());
    // After deletion further calls need a project id again
    assert!(matches!(lab.get_stats().await, Err(Error::InvalidInput(_))));
}

#[tokio::test]
pub async fn create_posts_set_attributes_only() {
    let server = Routes::new()
        .on("POST", "/v2/projects", 201, api_test_project().to_string())
        .start();
    let mut lab = project(&server);
    lab.auto_close = Some(false);
    lab.create().await.unwrap();
    assert_eq!(server.last_json("POST", "/v2/projects"), json!({"name": "API_TEST", "auto_close": false}));
    assert_eq!(lab.project_id.as_deref(), Some(PROJECT_ID));

    let mut nameless = Project::with_connector(lab.connector.clone().unwrap());
    assert!(matches!(nameless.create().await, Err(Error::InvalidInput(_))));
}

#[tokio::test]
pub async fn project_files() {
    let p = format!("/v2/projects/{PROJECT_ID}/files/project-files/docker/x/config.txt");
    let server = Routes::new()
        .on("GET", &p, 200, data("files.txt"))
        .on("POST", &p, 201, "")
        .start();
    let lab = project(&server).with_project_id(PROJECT_ID);
    let text = lab.get_file("project-files/docker/x/config.txt").await.unwrap();
    assert!(text.starts_with('#'));
    lab.write_file("project-files/docker/x/config.txt", "hostname r1").await.unwrap();
    assert_eq!(server.recorded().last().unwrap().body, "hostname r1");
}

#[tokio::test]
pub async fn bulk_node_actions_refresh_nodes() {
    let p = format!("/v2/projects/{PROJECT_ID}/nodes");
    let server = lab_routes()
        .on("POST", &format!("{p}/start"), 204, "")
        .on("POST", &format!("{p}/stop"), 204, "")
        .on("POST", &format!("{p}/reload"), 204, "")
        .on("POST", &format!("{p}/suspend"), 204, "")
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);
    lab.start_nodes(ZERO).await.unwrap();
    lab.stop_nodes(ZERO).await.unwrap();
    lab.reload_nodes(ZERO).await.unwrap();
    lab.suspend_nodes(ZERO).await.unwrap();
    assert_eq!(server.count("GET", &p), 4);
    assert_eq!(lab.nodes.len(), 6);
}

#[tokio::test]
pub async fn create_link_validates_and_posts_endpoints() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let created = json!({
        "link_id": "new-link", "link_type": "ethernet", "project_id": PROJECT_ID,
        "nodes": [], "suspend": false, "capturing": false
    });
    let server = lab_routes().on("POST", &format!("{p}/links"), 201, created.to_string()).start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);

    // unknown node / port
    let err = lab.create_link("ghost", "eth0", "alpine-1", "eth1").await.unwrap_err();
    assert_eq!(err.to_string(), "node_a: ghost not found");
    let err = lab.create_link("alpine-1", "eth9", "IOU1", "Ethernet0/1").await.unwrap_err();
    assert_eq!(err.to_string(), "port_a: eth9 not found");
    let err = lab.create_link("alpine-1", "eth1", "IOU1", "Ethernet9/9").await.unwrap_err();
    assert_eq!(err.to_string(), "port_b: Ethernet9/9 not found");

    // port already used: by endpoint A, by endpoint B, and in reversed orientation
    for (a, pa, b, pb) in [
        ("alpine-1", "eth0", "IOU1", "Ethernet0/1"),
        ("alpine-1", "eth1", "vEOS", "Ethernet1"),
        ("alpine-1", "eth1", "Ethernetswitch-1", "Ethernet7"),
    ] {
        let err = lab.create_link(a, pa, b, pb).await.unwrap_err();
        assert!(err.to_string().starts_with("At least one port is used, ID: "), "{err}");
    }
    assert_eq!(server.count("POST", &format!("{p}/links")), 0);

    let before = lab.links.len();
    let link = lab.create_link("alpine-1", "eth1", "IOU1", "Ethernet0/1").await.unwrap();
    assert_eq!(link.link_id.as_deref(), Some("new-link"));
    assert_eq!(lab.links.len(), before + 1);

    let body = server.last_json("POST", &format!("{p}/links"));
    assert_eq!(body["project_id"], PROJECT_ID);
    assert_eq!(body["nodes"][0]["node_id"], ALPINE_ID);
    // alpine-1 "eth1" is adapter 1, port 0
    assert_eq!(body["nodes"][0]["adapter_number"], 1);
    assert_eq!(body["nodes"][0]["port_number"], 0);
    assert_eq!(body["nodes"][0]["label"], json!({"text": "eth1"}));
    assert_eq!(body["nodes"][1]["label"], json!({"text": "Ethernet0/1"}));
}

#[tokio::test]
pub async fn delete_link_in_either_orientation() {
    let d7 = "d7dd01d6-9577-4076-b7f2-911b231044f8";
    let p = format!("/v2/projects/{PROJECT_ID}");
    let server = lab_routes()
        .on("DELETE", &format!("{p}/links/{d7}"), 204, "")
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);

    // fixture orientation is IOU1/Ethernet0/0 -> switch/Ethernet1; delete using the reverse
    lab.delete_link("Ethernetswitch-1", "Ethernet1", "IOU1", "Ethernet0/0").await.unwrap();
    assert_eq!(server.count("DELETE", &format!("{p}/links/{d7}")), 1);
    assert!(lab.links.iter().all(|l| l.link_id.as_deref() != Some(d7)));

    let err = lab.delete_link("alpine-1", "eth1", "IOU1", "Ethernet0/1").await.unwrap_err();
    assert!(err.to_string().starts_with("Link not found"), "{err}");
}

#[tokio::test]
pub async fn create_node_from_template() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let created = json!({"name": "alpine-2", "node_id": "new-node", "node_type": "docker",
                         "status": "stopped", "console": 5006, "compute_id": "local"});
    let server = lab_routes()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("POST", &format!("{p}/templates/{TEMPLATE_ID}"), 201, created.to_string())
        .on("PUT", &format!("{p}/nodes/new-node"), 200, created.to_string())
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);
    let node = gns3fy_rs::Node::default().with_name("alpine-2").with_template("alpine");
    let created = lab.create_node(node).await.unwrap();
    assert_eq!(created.node_id.as_deref(), Some("new-node"));
    assert_eq!(created.status, Some(NodeStatus::Stopped));
    assert_eq!(created.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(lab.nodes.len(), 7);
    assert_eq!(
        server.last_json("POST", &format!("{p}/templates/{TEMPLATE_ID}")),
        json!({"x": 0, "y": 0, "compute_id": "local"})
    );
    // attributes of the object are applied afterwards with a PUT
    assert_eq!(
        server.last_json("PUT", &format!("{p}/nodes/new-node")),
        json!({"name": "alpine-2", "compute_id": "local"})
    );
}

#[tokio::test]
pub async fn snapshots() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let first = json("project_snapshots.json")[0].clone();
    let snap_id = first["snapshot_id"].as_str().unwrap().to_string();
    let new_snap = json!({"snapshot_id": "snap-new", "name": "after", "project_id": PROJECT_ID, "created_at": 1});
    let server = lab_routes()
        .on("POST", &format!("{p}/snapshots"), 201, new_snap.to_string())
        .on("DELETE", &format!("{p}/snapshots/{snap_id}"), 204, "")
        .on("POST", &format!("{p}/snapshots/{snap_id}/restore"), 201, first.to_string())
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);
    let name = first["name"].as_str().unwrap().to_string();

    assert_eq!(lab.get_snapshot(Lookup::Name(&name)).await.unwrap().unwrap().snapshot_id, snap_id);
    assert_eq!(lab.get_snapshot(Lookup::Id(&snap_id)).await.unwrap().unwrap().name, name);
    assert!(lab.get_snapshot(Lookup::Name("ghost")).await.unwrap().is_none());

    assert!(matches!(lab.create_snapshot(&name).await, Err(Error::InvalidInput(m)) if m == "Snapshot already created"));
    let created = lab.create_snapshot("after").await.unwrap();
    assert_eq!(created.snapshot_id, "snap-new");
    assert_eq!(server.last_json("POST", &format!("{p}/snapshots")), json!({"name": "after"}));

    lab.delete_snapshot(Lookup::Name(&name)).await.unwrap();
    assert_eq!(server.count("DELETE", &format!("{p}/snapshots/{snap_id}")), 1);
    assert!(matches!(lab.delete_snapshot(Lookup::Name("ghost")).await, Err(Error::NotFound(_))));

    let gets_before = server.count("GET", &p);
    lab.restore_snapshot(Lookup::Id(&snap_id)).await.unwrap();
    assert_eq!(server.count("POST", &format!("{p}/snapshots/{snap_id}/restore")), 1);
    assert_eq!(server.count("GET", &p), gets_before + 1, "restore refreshes the project");
}

#[tokio::test]
pub async fn drawings() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let existing = json("project_drawings.json")[0].clone();
    let id = existing["drawing_id"].as_str().unwrap().to_string();
    let created = json!({"drawing_id": "d-new", "svg": "<svg/>", "locked": false, "x": 1, "y": 2, "z": 3});
    let server = lab_routes()
        .on("POST", &format!("{p}/drawings"), 201, created.to_string())
        .on("PUT", &format!("{p}/drawings/{id}"), 201, existing.to_string())
        .on("DELETE", &format!("{p}/drawings/{id}"), 204, "")
        .start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);

    assert_eq!(lab.get_drawing(&id).await.unwrap().unwrap().drawing_id, id);
    assert!(lab.get_drawing("nope").await.unwrap().is_none());

    let d = lab.create_drawing("<svg/>", false, 1, 2, 3).await.unwrap();
    assert_eq!(d.drawing_id, "d-new");
    assert_eq!(
        server.last_json("POST", &format!("{p}/drawings")),
        json!({"svg": "<svg/>", "locked": false, "x": 1, "y": 2, "z": 3})
    );
    assert_eq!(lab.drawings.as_ref().unwrap().len(), 3);

    // only z changes; everything else is kept from the stored drawing
    lab.update_drawing(&id, None, None, None, None, Some(9)).await.unwrap();
    let body = server.last_json("PUT", &format!("{p}/drawings/{id}"));
    assert_eq!(body["z"], 9);
    assert_eq!(body["svg"], existing["svg"]);
    assert_eq!(body["x"], existing["x"]);
    assert_eq!(body["locked"], existing["locked"]);
    assert!(matches!(lab.update_drawing("nope", None, None, None, None, None).await, Err(Error::NotFound(_))));

    lab.delete_drawing(&id).await.unwrap();
    assert_eq!(server.count("DELETE", &format!("{p}/drawings/{id}")), 1);
    assert!(matches!(lab.delete_drawing("nope").await, Err(Error::NotFound(_))));
}

#[tokio::test]
pub async fn arrange_nodes_on_a_circle() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let mut routes = lab_routes();
    for n in json("nodes.json").as_array().unwrap() {
        routes = routes.on(
            "PUT",
            &format!("{p}/nodes/{}", n["node_id"].as_str().unwrap()),
            200,
            n.to_string(),
        );
    }
    let server = routes.start();
    let mut lab = project(&server).with_project_id(PROJECT_ID);
    lab.arrange_nodes_circular(120.0).await.unwrap();

    let ids: Vec<String> = json("nodes.json")
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node_id"].as_str().unwrap().to_string())
        .collect();
    let pos: Vec<(i64, i64)> = ids
        .iter()
        .map(|id| {
            let b = server.last_json("PUT", &format!("{p}/nodes/{id}"));
            (b["x"].as_i64().unwrap(), b["y"].as_i64().unwrap())
        })
        .collect();
    // 6 nodes -> 60 degree steps starting at the top (y grows downwards). Coordinates are
    // truncated toward zero like Python's int(), hence 59 rather than 60 for two of them
    // (e.g. 120 * -cos(2*pi/3) = 59.99999999999998). Same values the Python library computes.
    assert_eq!(pos, vec![(0, -120), (103, -60), (103, 59), (0, 120), (-103, 60), (-103, -59)]);
    // project is already open: no open call
    assert_eq!(server.count("POST", &format!("{p}/open")), 0);
}
