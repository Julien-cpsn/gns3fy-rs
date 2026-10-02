//! GNS3 node object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::connector::{Body, Gns3Connector, LOCAL_COMPUTE};
use crate::error::{Error, Result};
use crate::link::Link;
use crate::types::{ConsoleType, Lookup, NodeStatus, NodeType, Port};
use crate::util::{merge_update, payload, str_field};

/// A node (device) of a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Node {
    pub name: Option<String>,
    pub project_id: Option<String>,
    pub node_id: Option<String>,
    pub compute_id: String,
    pub node_type: Option<NodeType>,
    pub node_directory: Option<String>,
    pub status: Option<NodeStatus>,
    pub ports: Option<Vec<Port>>,
    pub port_name_format: Option<String>,
    pub port_segment_size: Option<i64>,
    pub first_port_name: Option<String>,
    pub locked: Option<bool>,
    pub label: Option<Value>,
    pub console: Option<i64>,
    pub console_host: Option<String>,
    pub console_type: Option<ConsoleType>,
    pub console_auto_start: Option<bool>,
    pub command_line: Option<String>,
    pub custom_adapters: Option<Vec<Value>>,
    pub height: Option<i64>,
    pub width: Option<i64>,
    pub symbol: Option<String>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub z: Option<i64>,
    pub template_id: Option<String>,
    pub properties: Option<Value>,
    /// Template *name*, used by [`Node::create`] to find `template_id`.
    pub template: Option<String>,
    /// Links attached to this node (filled by [`Node::get_links`]).
    #[serde(skip)]
    pub links: Vec<Link>,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            name: None,
            project_id: None,
            node_id: None,
            compute_id: LOCAL_COMPUTE.to_string(),
            node_type: None,
            node_directory: None,
            status: None,
            ports: None,
            port_name_format: None,
            port_segment_size: None,
            first_port_name: None,
            locked: None,
            label: None,
            console: None,
            console_host: None,
            console_type: None,
            console_auto_start: None,
            command_line: None,
            custom_adapters: None,
            height: None,
            width: None,
            symbol: None,
            x: None,
            y: None,
            z: None,
            template_id: None,
            properties: None,
            template: None,
            links: Vec::new(),
            connector: None,
        }
    }
}

impl Node {
    pub fn with_connector(connector: Arc<Gns3Connector>) -> Self {
        Node {
            connector: Some(connector),
            ..Default::default()
        }
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// Template *name* to build the node from on [`create`](Node::create).
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    pub fn with_template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    pub fn with_compute_id(mut self, compute_id: impl Into<String>) -> Self {
        self.compute_id = compute_id.into();
        self
    }

    /// Looks a port up by name.
    pub fn port(&self, name: &str) -> Option<&Port> {
        self.ports.as_ref()?.iter().find(|p| p.name == name)
    }

    fn apply(&mut self, data: &Value) -> Result<()> {
        let mut new: Node = merge_update(&*self, data)?;
        new.links = std::mem::take(&mut self.links);
        new.connector = self.connector.take();
        *self = new;
        Ok(())
    }

    /// Connector, project id and node id; resolves `node_id` from `name` when missing.
    fn require(&mut self) -> Result<(Arc<Gns3Connector>, String, String)> {
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let pid = self
            .project_id
            .clone()
            .ok_or_else(|| Error::invalid("Need to submit project_id"))?;
        if self.node_id.is_none() {
            let name = self
                .name
                .clone()
                .ok_or_else(|| Error::invalid("Need to either submit node_id or name"))?;
            let nodes = conn.get_nodes(&pid)?;
            let ids: Vec<&str> = nodes
                .iter()
                .filter(|n| str_field(n, "name") == Some(name.as_str()))
                .filter_map(|n| str_field(n, "node_id"))
                .collect();
            match ids.as_slice() {
                [] => return Err(Error::not_found(format!("Node not found: {name}"))),
                [id] => self.node_id = Some((*id).to_string()),
                _ => {
                    return Err(Error::invalid(
                        "Multiple nodes found with same name. Need to submit node_id",
                    ))
                }
            }
        }
        let nid = self.node_id.clone().unwrap_or_default();
        Ok((conn, pid, nid))
    }

    /// Retrieves the node (and its links) from the server.
    pub fn get(&mut self) -> Result<()> {
        self.get_with_links(true)
    }

    /// Like [`get`](Node::get), optionally skipping the links request.
    pub fn get_with_links(&mut self, get_links: bool) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}/nodes/{nid}"), Body::Empty)?;
        self.apply(&data)?;
        if get_links {
            self.get_links()?;
        }
        Ok(())
    }

    /// Retrieves the links attached to this node.
    pub fn get_links(&mut self) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data = conn.call_json(
            Method::GET,
            &format!("/projects/{pid}/nodes/{nid}/links"),
            Body::Empty,
        )?;
        let raw: Vec<Value> = serde_json::from_value(data)?;
        let mut links = Vec::with_capacity(raw.len());
        for l in raw {
            let mut link: Link = serde_json::from_value(l)?;
            link.connector = Some(conn.clone());
            links.push(link);
        }
        self.links = links;
        Ok(())
    }

    /// POST an action (`start`, `stop`, `reload`, `suspend`); update from the answer when
    /// it already reports the expected status, otherwise re-fetch the node.
    fn action(&mut self, action: &str, expected: &str) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/nodes/{nid}/{action}"),
            Body::Empty,
        )?;
        if str_field(&data, "status") == Some(expected) {
            self.apply(&data)
        } else {
            self.get()
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.action("start", "started")
    }

    pub fn stop(&mut self) -> Result<()> {
        self.action("stop", "stopped")
    }

    pub fn reload(&mut self) -> Result<()> {
        self.action("reload", "started")
    }

    pub fn suspend(&mut self) -> Result<()> {
        self.action("suspend", "suspended")
    }

    /// Updates the node on the server with the given JSON fields.
    pub fn update(&mut self, fields: Value) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data = conn.call_json(
            Method::PUT,
            &format!("/projects/{pid}/nodes/{nid}"),
            Body::Json(fields),
        )?;
        self.apply(&data)
    }

    /// Creates the node from a template (`template` name or `template_id`), then applies
    /// the remaining attributes set on this object.
    pub fn create(&mut self) -> Result<()> {
        if self.node_id.is_some() {
            return Err(Error::invalid("Node already created"));
        }
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let pid = self
            .project_id
            .clone()
            .ok_or_else(|| Error::invalid("Node object needs to have project_id attribute"))?;
        if self.template_id.is_none() {
            let name = self
                .template
                .clone()
                .ok_or_else(|| Error::invalid("Need either 'template' of 'template_id'"))?;
            let template = conn
                .get_template(Lookup::Name(&name))?
                .ok_or_else(|| Error::invalid(format!("Template {name} not found")))?;
            self.template_id = str_field(&template, "template_id").map(String::from);
        }
        let cached = payload(
            &*self,
            &["project_id", "template", "template_id", "links", "connector"],
        )?;
        let template_id = self.template_id.clone().unwrap_or_default();
        let data = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/templates/{template_id}"),
            Body::Json(json!({"x": 0, "y": 0, "compute_id": self.compute_id})),
        )?;
        self.apply(&data)?;
        self.update(cached)
    }

    /// Deletes the node on the server and clears `project_id`, `node_id` and `name`.
    pub fn delete(&mut self) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        conn.call(Method::DELETE, &format!("/projects/{pid}/nodes/{nid}"), Body::Empty)?;
        self.project_id = None;
        self.node_id = None;
        self.name = None;
        Ok(())
    }

    /// Reads a file from the node directory.
    pub fn get_file(&mut self, path: &str) -> Result<String> {
        let (conn, pid, nid) = self.require()?;
        Ok(conn
            .call(Method::GET, &format!("/projects/{pid}/nodes/{nid}/files/{path}"), Body::Empty)?
            .text()?)
    }

    /// Writes a file in the node directory.
    pub fn write_file(&mut self, path: &str, data: impl Into<Vec<u8>>) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        conn.call(
            Method::POST,
            &format!("/projects/{pid}/nodes/{nid}/files/{path}"),
            Body::Bytes(data.into()),
        )?;
        Ok(())
    }
}
