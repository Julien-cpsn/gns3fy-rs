mod common;

use std::sync::Arc;

use common::*;
use gns3fy_rs::{ConsoleType, DockerTemplate, Error, Gns3Connector, Lookup, Node, QemuTemplate, Template, TemplateKind, TemplateType, VpcsTemplate};
use serde_json::{json, Value};

fn conn(server: &MockServer) -> Arc<Gns3Connector> {
    Arc::new(Gns3Connector::new(&server.url).unwrap())
}

fn by_name(name: &str) -> Value {
    json("templates.json")
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no fixture template {name}"))
        .clone()
}

fn id_of(name: &str) -> String {
    by_name(name)["template_id"].as_str().unwrap().to_string()
}

fn path(id: &str) -> String {
    format!("/v2/templates/{id}")
}

// ---------------------------------------------------------------- parsing

#[tokio::test]
pub async fn every_fixture_template_round_trips_without_losing_fields() {
    let all = json("templates.json");
    assert_eq!(all.as_array().unwrap().len(), 11);
    for original in all.as_array().unwrap() {
        let t: Template = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(t.name.as_str(), original["name"].as_str().unwrap());
        assert_eq!(t.template_id.as_deref(), original["template_id"].as_str());
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            *original,
            "template {} changed on round trip",
            original["name"]
        );
    }
}

#[tokio::test]
pub async fn typed_fields_and_type_specific_properties() {
    let alpine: Template = serde_json::from_value(by_name("alpine")).unwrap();
    assert_eq!(alpine.template_type(), TemplateType::Docker);
    assert_eq!(alpine.category.as_deref(), Some("guest"));
    assert_eq!(alpine.console_type, Some(ConsoleType::Telnet));
    assert!(!alpine.is_builtin());
    let docker_template_kind = alpine.kind.as_docker().unwrap();
    assert_eq!(docker_template_kind.image.as_ref().unwrap().as_str(), by_name("alpine")["image"].as_str().unwrap());
    assert_eq!(alpine.name, "alpine");

    let veos: Template = serde_json::from_value(by_name("vEOS")).unwrap();
    assert_eq!(veos.template_type(), TemplateType::Qemu);
    let qemu_template_kind = veos.kind.as_qemu().unwrap();
    assert_eq!(qemu_template_kind.ram.unwrap(), by_name("vEOS").get("ram").unwrap().as_i64().unwrap());
    assert!(qemu_template_kind.hda_disk_image.is_some());

    let cloud: Template = serde_json::from_value(by_name("Cloud")).unwrap();
    assert!(cloud.is_builtin());
}

#[tokio::test]
pub async fn invalid_template_or_console_type_is_rejected() {
    assert!(serde_json::from_value::<Template>(json!({"template_type": "toaster"})).is_err());
    assert!(serde_json::from_value::<Template>(json!({"console_type": "smoke"})).is_err());
}

#[tokio::test]
pub async fn template_type_wire_names_and_node_type_conversion() {
    for t in [
        "cloud", "nat", "ethernet_hub", "ethernet_switch", "frame_relay_switch", "atm_switch",
        "docker", "dynamips", "vpcs", "traceng", "virtualbox", "vmware", "iou", "qemu",
    ] {
        let tt: TemplateType = serde_json::from_value(json!(t)).unwrap();
        assert_eq!(tt.as_str(), t);
        assert_eq!(serde_json::to_value(tt).unwrap(), json!(t));
        // TemplateType is validated independently; NodeType no longer implements
        // a conversion from TemplateType.
    }
}

#[tokio::test]
pub async fn equality_ignores_the_connector() {
    let server = Routes::new().start();
    let a: Template = serde_json::from_value(by_name("alpine")).unwrap();
    let mut b = a.clone();
    b.connector = Some(conn(&server));
    assert_eq!(a, b);
    b.kind.as_docker_mut().unwrap().adapters = Some(999);
    assert_ne!(a, b);
}

// ---------------------------------------------------------------- list / find

#[tokio::test]
pub async fn list_and_find() {
    let alpine_id = id_of("alpine");
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("GET", &path(&alpine_id), 200, by_name("alpine").to_string())
        .on("GET", &path("missing"), 404, r#"{"status": 404, "message": "Template ID missing doesn't exist"}"#)
        .start();
    let c = conn(&server);

    let all = Template::list(&c).await.unwrap();
    assert_eq!(all.len(), 11);
    assert!(all.iter().all(|t| t.connector.is_some()));
    assert_eq!(all.iter().filter(|t| t.is_builtin()).count(), 7);
    assert_eq!(all[0].name.as_str(), "IOU-L3");

    let by_n = Template::find(&c, Lookup::Name("alpine")).await.unwrap().unwrap();
    assert_eq!(by_n.template_id.as_deref(), Some(alpine_id.as_str()));
    let by_i = Template::find(&c, Lookup::Id(&alpine_id)).await.unwrap().unwrap();
    assert_eq!(by_i.name.as_str(), "alpine");
    assert!(Template::find(&c, Lookup::Name("ghost")).await.unwrap().is_none());
    assert!(matches!(
        Template::find(&c, Lookup::Id("missing")).await,
        Err(Error::Api { status: 404, .. })
    ));
}

// ---------------------------------------------------------------- get

#[tokio::test]
pub async fn get_resolves_id_from_name() {
    let id = id_of("vEOS");
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("GET", &path(&id), 200, by_name("vEOS").to_string())
        .start();
    let mut t = Template::new(
        conn(&server),
        "vEOS",
        TemplateKind::Qemu(QemuTemplate::default()),
    );
    t.get().await.unwrap();
    assert_eq!(t.template_id.as_deref(), Some(id.as_str()));
    assert_eq!(t.template_type(), TemplateType::Qemu);
    assert_eq!(
        t.kind.as_qemu().unwrap().ram.unwrap().to_string(),
        by_name("vEOS")["ram"].to_string()
    );
    assert!(t.connector.is_some());

    // second call goes straight to the id
    let before = server.count("GET", "/v2/templates");
    t.get().await.unwrap();
    assert_eq!(server.count("GET", "/v2/templates"), before);
}

#[tokio::test]
pub async fn get_errors() {
    let server = Routes::new().on("GET", "/v2/templates", 200, data("templates.json")).start();
    let c = conn(&server);

    let mut missing_connector: Template =
        serde_json::from_value(by_name("alpine")).unwrap();
    assert!(matches!(missing_connector.get().await, Err(Error::MissingConnector)));

    let mut ghost = Template::new(
        c,
        "ghost",
        TemplateKind::Docker(DockerTemplate::default()),
    );
    assert!(matches!(
        ghost.get().await,
        Err(Error::NotFound(m)) if m == "Template not found: ghost"
    ));
}

// ---------------------------------------------------------------- create

fn created_response() -> Value {
    json!({
        "template_id": "new-id", "name": "my-alpine", "template_type": "docker",
        "category": "guest", "builtin": false, "compute_id": "local",
        "image": "alpine:latest", "adapters": 2, "console_type": "telnet"
    })
}

#[tokio::test]
pub async fn create_posts_typed_and_extra_fields() {
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("POST", "/v2/templates", 201, created_response().to_string())
        .start();
    let mut t = Template::new(
        conn(&server),
        "my-alpine",
        TemplateKind::Docker(DockerTemplate::default()),
    );
    {
        let docker = t.kind.as_docker_mut().unwrap();
        docker.image = Some(String::from("alpine:latest"));
        docker.adapters = Some(2);
    }
    t.console_type = Some(ConsoleType::Telnet);
    t.create().await.unwrap();

    assert_eq!(
        server.last_json("POST", "/v2/templates"),
        json!({
            "name": "my-alpine", "template_type": "docker", "compute_id": "local",
            "console_type": "telnet", "image": "alpine:latest", "adapters": 2
        })
    );
    assert_eq!(t.template_id.as_deref(), Some("new-id"));
    assert_eq!(t.category.as_deref(), Some("guest"));
    assert_eq!(t.builtin, Some(false));
    assert_eq!(t.kind.as_docker().unwrap().adapters, Some(2));
    assert!(t.connector.is_some());
}

#[tokio::test]
pub async fn create_keeps_an_explicit_compute_id() {
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, "[]")
        .on("POST", "/v2/templates", 201, created_response().to_string())
        .start();
    Template::new(conn(&server), "x", TemplateKind::Vpcs(VpcsTemplate::default()))
        .with_compute_id("remote-1")
        .create()
        .await
        .unwrap();
    assert_eq!(server.last_json("POST", "/v2/templates")["compute_id"], "remote-1");
}

#[tokio::test]
pub async fn create_validations() {
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("POST", "/v2/templates", 201, created_response().to_string())
        .start();
    let c = conn(&server);

    // A Template constructed from server data has no connector until one is attached.
    let mut missing_connector: Template = serde_json::from_value(by_name("alpine")).unwrap();
    missing_connector.template_id = None;
    assert!(matches!(missing_connector.create().await, Err(Error::MissingConnector)));

    // name already used on the server
    assert!(matches!(
        Template::new(c.clone(), "alpine", TemplateKind::Docker(DockerTemplate::default())).create().await,
        Err(Error::InvalidInput(m)) if m == "Template already used: alpine"
    ));

    // already created
    let mut done = Template::new(c, "fresh", TemplateKind::Docker(DockerTemplate::default()));
    done.create().await.unwrap();
    assert!(matches!(done.create().await, Err(Error::InvalidInput(m)) if m == "Template already created"));
    assert_eq!(server.count("POST", "/v2/templates"), 1);
}

#[tokio::test]
pub async fn save_sends_the_whole_local_template() {
    let id = id_of("alpine");
    let mut stored = by_name("alpine");
    stored["start_command"] = json!("sh");
    let server = Routes::new()
        .on("GET", &path(&id), 200, by_name("alpine").to_string())
        .on("PUT", &path(&id), 200, stored.to_string())
        .start();
    let c = conn(&server);
    let mut t = Template::find(&c, Lookup::Id(&id)).await.unwrap().unwrap();
    t.kind.as_docker_mut().unwrap().start_command = Some(String::from("sh"));
    t.symbol = Some(":/symbols/docker.svg".into());
    t.save().await.unwrap();

    let body = server.last_json("PUT", &path(&id));
    assert_eq!(body["start_command"], "sh");
    assert_eq!(body["symbol"], ":/symbols/docker.svg");
    // untouched fields travel too, including type-specific ones the struct has no field for
    assert_eq!(body["image"], by_name("alpine")["image"]);
    assert_eq!(body["template_id"], id);
    assert!(body.get("connector").is_none());
    assert_eq!(
        t.kind.as_docker().unwrap().start_command,
        Some(String::from("sh"))
    );
}

#[tokio::test]
pub async fn save_updates_typed_fields() {
    let id = id_of("vEOS");
    let mut after = by_name("vEOS");
    after["ram"] = json!(4096);
    let server = Routes::new()
        .on("GET", &path(&id), 200, by_name("vEOS").to_string())
        .on("PUT", &path(&id), 200, after.to_string())
        .start();

    let c = conn(&server);
    let mut t = Template::find(&c, Lookup::Id(&id)).await.unwrap().unwrap();
    t.kind.as_qemu_mut().unwrap().ram = Some(4096);
    t.save().await.unwrap();

    let body = server.last_json("PUT", &path(&id));
    assert_eq!(body["ram"], 4096);
    assert_eq!(body["name"], "vEOS");
    assert_eq!(body["hda_disk_image"], by_name("vEOS")["hda_disk_image"]);
    assert_eq!(t.kind.as_qemu().unwrap().ram, Some(4096));
    assert_eq!(t.template_id.as_deref(), Some(id.as_str()));
}

#[tokio::test]
pub async fn builtin_templates_cannot_be_modified_or_deleted() {
    let server = Routes::new().start();
    let mut cloud: Template = serde_json::from_value(by_name("Cloud")).unwrap();
    cloud.connector = Some(conn(&server));
    for err in [
        cloud.save().await.unwrap_err(),
        cloud.delete().await.unwrap_err(),
    ] {
        assert!(matches!(err, Error::InvalidInput(ref m) if m.contains("built-in template Cloud")), "{err}");
    }
    assert!(server.recorded().is_empty(), "no request must reach the server");
}

// ---------------------------------------------------------------- delete

#[tokio::test]
pub async fn delete_by_id_and_by_name() {
    let id = id_of("alpine");
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("DELETE", &path(&id), 204, "")
        .start();
    let c = conn(&server);

    let mut by_id = Template::new(
        c.clone(),
        "",
        TemplateKind::Docker(DockerTemplate::default()),
    )
        .with_template_id(id.as_str());
    by_id.delete().await.unwrap();
    assert!(by_id.template_id.is_none());
    assert_eq!(by_id.name, "");

    let mut by_n = Template::new(
        c,
        "alpine",
        TemplateKind::Docker(DockerTemplate::default()),
    );
    by_n.delete().await.unwrap();
    assert_eq!(server.count("DELETE", &path(&id)), 2);
    assert!(by_n.template_id.is_none());
    assert_eq!(by_n.name, "alpine");
}

#[tokio::test]
pub async fn server_errors_surface_and_leave_the_object_intact() {
    let id = id_of("alpine");
    let server = Routes::new()
        .on("DELETE", &path(&id), 409, r#"{"status": 409, "message": "Template is used"}"#)
        .start();
    let mut t = Template::new(conn(&server), "alpine", TemplateKind::Qemu(QemuTemplate::default())).with_template_id(id.as_str());
    match t.delete().await.unwrap_err() {
        Error::Api { status, message } => {
            assert_eq!((status, message.as_str()), (409, "Template is used"));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(t.template_id.as_deref(), Some(id.as_str()));
    assert_eq!(t.name.as_str(), "alpine");
}

// ---------------------------------------------------------------- with nodes

#[tokio::test]
pub async fn a_template_can_seed_a_node() {
    let p = format!("/v2/projects/{PROJECT_ID}");
    let alpine_id = id_of("alpine");
    let fresh = json!({"name": "a1", "node_id": "n1", "node_type": "docker", "status": "stopped", "compute_id": "local"});
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("POST", &format!("{p}/templates/{alpine_id}"), 201, fresh.to_string())
        .on("PUT", &format!("{p}/nodes/n1"), 200, fresh.to_string())
        .start();
    let c = conn(&server);
    let template = Template::find(&c, Lookup::Name("alpine")).await.unwrap().unwrap();

    let mut node = Node::with_connector(c)
        .with_project_id(PROJECT_ID)
        .with_name("a1")
        .with_template_id(template.template_id.clone().unwrap());
    node.create().await.unwrap();
    assert_eq!(node.node_id.as_deref(), Some("n1"));
    assert_eq!(server.count("POST", &format!("{p}/templates/{alpine_id}")), 1);
}