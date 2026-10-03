mod common;

use common::*;
use gns3fy_rs::{
    Compute, ComputeImage, ComputePorts, Error, Gns3Connector, Link, Lookup, Node, NodeStatus,
    NodeType, Project, ProjectStatus, Template, TemplateKind, TemplateType, LOCAL_COMPUTE,
};
use serde_json::json;

fn connector(server: &MockServer) -> Gns3Connector {
    Gns3Connector::new(&server.url).unwrap()
}

fn project_by_name(name: &str) -> Project {
    load::<Vec<Project>>("projects.json")
        .into_iter()
        .find(|p| p.name.as_deref() == Some(name))
        .unwrap()
}

fn template_by_name(name: &str) -> Template {
    load::<Vec<Template>>("templates.json")
        .into_iter()
        .find(|t| t.name == name)
        .unwrap()
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
    assert!(v.local);
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
fn error_body_without_status_falls_back_to_the_http_status() {
    let server = Routes::new()
        .on("GET", "/v2/version", 409, r#"{"message": "conflict"}"#)
        .start();
    match connector(&server).get_version().unwrap_err() {
        Error::Api { status, message } => assert_eq!((status, message.as_str()), (409, "conflict")),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn undecodable_answers_are_json_errors() {
    let server = Routes::new().on("GET", "/v2/version", 200, r#"{"local": "maybe"}"#).start();
    assert!(matches!(connector(&server).get_version(), Err(Error::Json(_))));
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
fn json_bodies_carry_a_content_type() {
    let server = Routes::new()
        .on("POST", "/v2/projects", 201, body(&project_by_name("API_TEST")))
        .start();
    let project = Project::default().with_name("API_TEST");
    connector(&server).create_project(&project).unwrap();
    assert_eq!(
        server.recorded()[0].content_type.as_deref(),
        Some("application/json")
    );
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
fn projects_are_typed() {
    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}"), 200, body(&project_by_name("API_TEST")))
        .start();
    let c = connector(&server);

    let all: Vec<Project> = c.get_projects().unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|p| p.connector.is_none()));

    let by_name = c.get_project(Lookup::Name("API_TEST")).unwrap().unwrap();
    assert_eq!(by_name.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(by_name.status, Some(ProjectStatus::Opened));
    assert_eq!(by_name.auto_start, Some(true));
    assert!(c.get_project(Lookup::Name("missing")).unwrap().is_none());
    let by_id = c.get_project(Lookup::Id(PROJECT_ID)).unwrap().unwrap();
    assert_eq!(by_id.name.as_deref(), Some("API_TEST"));
}

#[test]
fn create_and_delete_project() {
    let server = Routes::new()
        .on("POST", "/v2/projects", 201, body(&project_by_name("API_TEST")))
        .on("DELETE", &format!("/v2/projects/{PROJECT_ID}"), 204, "")
        .start();
    let c = connector(&server);
    let nameless = Project { zoom: Some(1), ..Default::default() };
    assert!(matches!(c.create_project(&nameless), Err(Error::InvalidInput(_))));

    let created = c.create_project(&Project::default().with_name("API_TEST")).unwrap();
    assert_eq!(created.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(server.last_json("POST", "/v2/projects"), json!({"name": "API_TEST"}));
    c.delete_project(PROJECT_ID).unwrap();
}

#[test]
fn templates_are_typed() {
    let alpine_id = id_of_template("alpine");
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("GET", &format!("/v2/templates/{alpine_id}"), 200, body(&template_by_name("alpine")))
        .start();
    let c = connector(&server);

    let all: Vec<Template> = c.get_templates().unwrap();
    assert_eq!(all.len(), 11);
    assert!(all.iter().all(|t| t.connector.is_none()));
    assert_eq!(all[0].name, "IOU-L3");
    assert_eq!(all[0].template_type(), TemplateType::Iou);

    let alpine = c.get_template(Lookup::Name("alpine")).unwrap().unwrap();
    assert_eq!(alpine.template_id.as_deref(), Some(alpine_id.as_str()));
    let docker = alpine.kind.as_docker().expect("alpine is a docker template");
    assert_eq!(docker.image.as_deref(), Some("alpine"));
    assert!(c.get_template(Lookup::Name("nope")).unwrap().is_none());
    assert_eq!(c.get_template(Lookup::Id(&alpine_id)).unwrap().unwrap(), alpine);

    let summary = c.templates_summary().unwrap();
    assert_eq!(summary.len(), 11);
    assert_eq!(summary[0].name, "IOU-L3");
    assert_eq!(summary[0].template_type, "iou");
    assert_eq!(summary[0].console_type, "telnet");
    let cloud = summary.iter().find(|s| s.name == "Cloud").unwrap();
    assert!(cloud.builtin);
    assert_eq!(cloud.console_type, "N/A");
}

fn id_of_template(name: &str) -> String {
    template_by_name(name).template_id.unwrap()
}

#[test]
fn template_crud() {
    let alpine_id = id_of_template("alpine");
    let created = {
        let mut t = template_by_name("alpine");
        t.name = "new".into();
        t.template_id = Some("new-id".into());
        t
    };
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("PUT", &format!("/v2/templates/{alpine_id}"), 200, body(&template_by_name("alpine")))
        .on("POST", "/v2/templates", 201, body(&created))
        .on("DELETE", &format!("/v2/templates/{alpine_id}"), 204, "")
        .start();
    let c = connector(&server);

    // update sends the whole template, flattened, with its template_type
    let mut alpine = template_by_name("alpine");
    if let TemplateKind::Docker(d) = &mut alpine.kind {
        d.start_command = Some("sh".into());
    }
    c.update_template(&alpine).unwrap();
    let sent = server.last_json("PUT", &format!("/v2/templates/{alpine_id}"));
    assert_eq!(sent["template_type"], "docker");
    assert_eq!(sent["start_command"], "sh");
    assert_eq!(sent["image"], "alpine");
    assert!(sent.get("connector").is_none());
    let no_id = Template::new(
        std::sync::Arc::new(connector(&server)),
        "x",
        TemplateKind::Nat(Default::default()),
    );
    assert!(matches!(c.update_template(&no_id), Err(Error::InvalidInput(_))));

    // create: name must be free, compute_id defaults to local
    assert!(matches!(
        c.create_template(&alpine),
        Err(Error::InvalidInput(m)) if m == "Template already used: alpine"
    ));
    let mut fresh = template_by_name("alpine");
    fresh.name = "new".into();
    fresh.template_id = None;
    fresh.compute_id = None;
    let stored = c.create_template(&fresh).unwrap();
    assert_eq!(stored.template_id.as_deref(), Some("new-id"));
    let sent = server.last_json("POST", "/v2/templates");
    assert_eq!(sent["compute_id"], LOCAL_COMPUTE);
    assert_eq!(sent["name"], "new");
    let mut unnamed = fresh.clone();
    unnamed.name.clear();
    assert!(matches!(c.create_template(&unnamed), Err(Error::InvalidInput(_))));

    // delete by name and by id
    c.delete_template(Lookup::Name("alpine")).unwrap();
    c.delete_template(Lookup::Id(&alpine_id)).unwrap();
    assert_eq!(server.count("DELETE", &format!("/v2/templates/{alpine_id}")), 2);
    assert!(matches!(c.delete_template(Lookup::Name("ghost")), Err(Error::NotFound(_))));
}

#[test]
fn nodes_and_links_are_typed() {
    let server = Routes::new()
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/nodes"), 200, data("nodes.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/nodes/{ALPINE_ID}"), 200, json("nodes.json")[4].to_string())
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/links"), 200, data("links.json"))
        .on("GET", &format!("/v2/projects/{PROJECT_ID}/links/{LINK_ID}"), 200, link_json())
        .start();
    let c = connector(&server);

    let nodes: Vec<Node> = c.get_nodes(PROJECT_ID).unwrap();
    assert_eq!(nodes.len(), 6);
    let alpine = c.get_node(PROJECT_ID, ALPINE_ID).unwrap();
    assert_eq!(alpine.name.as_deref(), Some("alpine-1"));
    assert_eq!(alpine.node_type, Some(NodeType::Docker));
    assert_eq!(alpine.status, Some(NodeStatus::Started));
    let props = alpine.properties.as_ref().unwrap();
    assert_eq!(props.image.as_deref(), Some("alpine:latest"));
    assert_eq!(props.adapters, Some(2));
    assert_eq!(props.aux, Some(5006));

    let links: Vec<Link> = c.get_links(PROJECT_ID).unwrap();
    assert_eq!(links.len(), 7);
    let link = c.get_link(PROJECT_ID, LINK_ID).unwrap();
    assert_eq!(link.link_id.as_deref(), Some(LINK_ID));
    assert_eq!(link.nodes.as_ref().unwrap().len(), 2);
}

fn link_json() -> String {
    json("links.json")
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["link_id"] == LINK_ID)
        .unwrap()
        .to_string()
}

#[test]
fn computes_are_typed() {
    let server = Routes::new()
        .on("GET", "/v2/computes", 200, data("computes.json"))
        .on("GET", "/v2/computes/local", 200, json("computes.json")[0].to_string())
        .on("GET", "/v2/computes/local/qemu/images", 200, data("compute_qemu_images.json"))
        .on("GET", "/v2/computes/local/ports", 200, data("compute_ports.json"))
        .on("POST", "/v2/computes/local/qemu/images/disk.qcow2", 204, "")
        .start();
    let c = connector(&server);

    let computes: Vec<Compute> = c.get_computes().unwrap();
    assert_eq!(computes.len(), 1);
    let local = c.get_compute(LOCAL_COMPUTE).unwrap();
    assert_eq!(local, computes[0]);
    assert_eq!(local.compute_id, "local");
    assert!(local.connected);
    assert_eq!(local.protocol.as_deref(), Some("http"));
    assert_eq!(local.port, Some(3080));
    let caps = local.capabilities.as_ref().unwrap();
    assert!(caps.node_types.contains(&NodeType::Qemu));
    assert_eq!(caps.platform.as_deref(), Some("linux"));

    let images: Vec<ComputeImage> = c.get_compute_images("qemu", LOCAL_COMPUTE).unwrap();
    assert_eq!(images.len(), json("compute_qemu_images.json").as_array().unwrap().len());
    assert_eq!(images[0].filename, "cumulus-linux-3.7.8-vx-amd64-qemu.qcow2");
    assert_eq!(images[0].filesize, 619249664);

    let ports: ComputePorts = c.get_compute_ports(LOCAL_COMPUTE).unwrap();
    assert_eq!(ports.console_port_range, (5000, 10000));
    assert_eq!(ports.udp_port_range, (10000, 20000));
    assert!(ports.console_ports.contains(&5005));

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