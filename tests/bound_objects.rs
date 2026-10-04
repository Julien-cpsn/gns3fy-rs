//! Objects handed out by the connector must be usable right away: they carry the connector,
//! so `project.delete()`, `node.start()`... work without any further setup.

mod common;

use std::sync::Arc;

use common::*;
use gns3fy_rs::{Gns3Connector, Link, Lookup, Node, Project, Template, TemplateKind};

const OTHER_PROJECT_ID: &str = "c9dc56bf-37b9-453b-8f95-2845ce8908e3";

fn connector(server: &MockServer) -> Arc<Gns3Connector> {
    Arc::new(Gns3Connector::new(&server.url).unwrap())
}

fn project_path(id: &str) -> String {
    format!("/v2/projects/{id}")
}

/// The exact scenario of the bug report.
#[test]
fn delete_every_project_returned_by_get_projects() {
    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("DELETE", &project_path(PROJECT_ID), 204, "")
        .on("DELETE", &project_path(OTHER_PROJECT_ID), 204, "")
        .start();
    let connector = connector(&server);

    for mut project in connector.get_projects().unwrap() {
        project.delete().unwrap();
    }

    assert_eq!(server.count("DELETE", &project_path(PROJECT_ID)), 1);
    assert_eq!(server.count("DELETE", &project_path(OTHER_PROJECT_ID)), 1);
}

#[test]
fn projects_from_the_connector_are_bound() {
    let api_test = load::<Vec<Project>>("projects.json").remove(1);
    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("GET", &project_path(PROJECT_ID), 200, body(&api_test))
        .on("GET", &format!("{}/stats", project_path(PROJECT_ID)), 200, r#"{"drawings":0,"links":0,"nodes":0,"snapshots":0}"#)
        .on("GET", &format!("{}/nodes", project_path(PROJECT_ID)), 200, "[]")
        .on("GET", &format!("{}/links", project_path(PROJECT_ID)), 200, "[]")
        .on("POST", &format!("{}/open", project_path(PROJECT_ID)), 201, body(&api_test))
        .on("POST", "/v2/projects", 201, body(&api_test))
        .on("DELETE", &project_path(PROJECT_ID), 204, "")
        .start();
    let c = connector(&server);

    assert!(c.get_projects().unwrap().iter().all(|p| p.connector.is_some()));

    let mut by_name = c.get_project(Lookup::Name("API_TEST")).unwrap().unwrap();
    assert!(by_name.connector.is_some());
    by_name.get().unwrap();
    by_name.open().unwrap();

    let mut by_id = c.get_project(Lookup::Id(PROJECT_ID)).unwrap().unwrap();
    assert!(by_id.connector.is_some());

    let mut created = c.create_project(&Project::default().with_name("API_TEST")).unwrap();
    assert!(created.connector.is_some());
    created.delete().unwrap();
    by_id.delete().unwrap();
}

#[test]
fn templates_from_the_connector_are_bound() {
    let alpine = load::<Vec<Template>>("templates.json")
        .into_iter()
        .find(|t| t.name == "alpine")
        .unwrap();
    let id = alpine.template_id.clone().unwrap();
    let mut created = alpine.clone();
    created.name = "fresh".into();
    created.template_id = Some("fresh-id".into());
    let server = Routes::new()
        .on("GET", "/v2/templates", 200, data("templates.json"))
        .on("GET", &format!("/v2/templates/{id}"), 200, body(&alpine))
        .on("PUT", &format!("/v2/templates/{id}"), 200, body(&alpine))
        .on("POST", "/v2/templates", 201, body(&created))
        .on("DELETE", &format!("/v2/templates/{id}"), 204, "")
        .on("DELETE", "/v2/templates/fresh-id", 204, "")
        .start();
    let c = connector(&server);

    assert!(c.get_templates().unwrap().iter().all(|t| t.connector.is_some()));

    // a user-facing flow: fetch, edit, save
    let mut t = c.get_template(Lookup::Name("alpine")).unwrap().unwrap();
    assert!(t.connector.is_some());
    if let TemplateKind::Docker(d) = &mut t.kind {
        d.start_command = Some("sh".into());
    }
    t.save().unwrap();

    let by_id = c.get_template(Lookup::Id(&id)).unwrap().unwrap();
    assert!(by_id.connector.is_some());

    let mut fresh = Template::new(c.clone(), "fresh", alpine.kind.clone());
    fresh.template_id = None;
    let mut stored = c.create_template(&fresh).unwrap();
    assert!(stored.connector.is_some());
    stored.delete().unwrap();

    let mut updated = c.update_template(&t).unwrap();
    assert!(updated.connector.is_some());
    updated.delete().unwrap();
    assert_eq!(server.count("DELETE", "/v2/templates/fresh-id"), 1);
    assert_eq!(server.count("DELETE", &format!("/v2/templates/{id}")), 1);
}

#[test]
fn nodes_and_links_from_the_connector_are_bound() {
    let p = project_path(PROJECT_ID);
    let alpine = load::<Vec<Node>>("nodes.json").remove(4);
    let started = {
        let mut n = alpine.clone();
        n.status = Some(gns3fy_rs::NodeStatus::Started);
        n
    };
    let link = load::<Vec<Link>>("links.json")
        .into_iter()
        .find(|l| l.link_id.as_deref() == Some(LINK_ID))
        .unwrap();
    let server = Routes::new()
        .on("GET", &format!("{p}/nodes"), 200, data("nodes.json"))
        .on("GET", &format!("{p}/nodes/{ALPINE_ID}"), 200, body(&alpine))
        .on("POST", &format!("{p}/nodes/{ALPINE_ID}/start"), 200, body(&started))
        .on("DELETE", &format!("{p}/nodes/{ALPINE_ID}"), 204, "")
        .on("GET", &format!("{p}/links"), 200, data("links.json"))
        .on("GET", &format!("{p}/links/{LINK_ID}"), 200, body(&link))
        .on("DELETE", &format!("{p}/links/{LINK_ID}"), 204, "")
        .start();
    let c = connector(&server);

    let nodes = c.get_nodes(PROJECT_ID).unwrap();
    assert_eq!(nodes.len(), 6);
    assert!(nodes.iter().all(|n| n.connector.is_some()));
    let mut node = c.get_node(PROJECT_ID, ALPINE_ID).unwrap();
    assert!(node.connector.is_some());
    node.start().unwrap();
    node.delete().unwrap();

    let links = c.get_links(PROJECT_ID).unwrap();
    assert_eq!(links.len(), 7);
    assert!(links.iter().all(|l| l.connector.is_some()));
    let mut l = c.get_link(PROJECT_ID, LINK_ID).unwrap();
    assert!(l.connector.is_some());
    l.get().unwrap();
    l.delete().unwrap();
}

/// `once_cell::Lazy<Arc<Gns3Connector>>` statics deref to the `Arc`; the methods must be
/// reachable through such a wrapper.
#[test]
fn works_through_a_lazy_static_style_wrapper() {
    struct LazyConnector(Arc<Gns3Connector>);
    impl std::ops::Deref for LazyConnector {
        type Target = Arc<Gns3Connector>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    let server = Routes::new()
        .on("GET", "/v2/projects", 200, data("projects.json"))
        .on("DELETE", &project_path(PROJECT_ID), 204, "")
        .on("DELETE", &project_path(OTHER_PROJECT_ID), 204, "")
        .start();
    let connector = LazyConnector(connector(&server));

    for mut project in connector.get_projects().unwrap() {
        project.delete().unwrap();
    }
    assert_eq!(server.count("DELETE", &project_path(PROJECT_ID)), 1);
    assert_eq!(server.count("DELETE", &project_path(OTHER_PROJECT_ID)), 1);
}
