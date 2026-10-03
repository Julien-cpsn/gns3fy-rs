//! GNS3 node object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::connector::{Body, Gns3Connector, LOCAL_COMPUTE};
use crate::error::{Error, Result};
use crate::link::Link;
use crate::properties::NodeProperties;
use crate::types::{ConsoleType, CustomAdapter, Label, Lookup, NodeStatus, NodeType, Port};
use crate::util::merge_some;

/// A node (device) of a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Node {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub compute_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NodeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<Port>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_segment_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_port_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_type: Option<ConsoleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_auto_start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    /// Emulator specific settings (`ram`, `image`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<NodeProperties>,
    /// Template *name*, used by [`Node::create`] to find `template_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// The attributes of a node that can be changed on the server; `None` fields are left as
/// they are.
///
/// ```
/// use gns3fy_rs::{NodeProperties, NodeUpdate};
///
/// let patch = NodeUpdate {
///     x: Some(100),
///     y: Some(-50),
///     properties: Some(NodeProperties { ram: Some(1024), ..Default::default() }),
///     ..Default::default()
/// };
/// assert_eq!(patch.x, Some(100));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_type: Option<ConsoleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_auto_start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_port_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_segment_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<NodeProperties>,
}

impl From<&Node> for NodeUpdate {
    /// The user-settable attributes of a node, as a patch.
    fn from(n: &Node) -> Self {
        NodeUpdate {
            name: n.name.clone(),
            compute_id: Some(n.compute_id.clone()),
            x: n.x,
            y: n.y,
            z: n.z,
            locked: n.locked,
            label: n.label.clone(),
            symbol: n.symbol.clone(),
            console: n.console,
            console_type: n.console_type,
            console_auto_start: n.console_auto_start,
            first_port_name: n.first_port_name.clone(),
            port_name_format: n.port_name_format.clone(),
            port_segment_size: n.port_segment_size,
            custom_adapters: n.custom_adapters.clone(),
            properties: n.properties.clone(),
        }
    }
}

/// Body of `POST /projects/{id}/templates/{template_id}`.
#[derive(Serialize)]
struct FromTemplateRequest<'a> {
    x: i64,
    y: i64,
    compute_id: &'a str,
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

    /// Overlay the fields present in a server answer on this object.
    fn apply(&mut self, new: Node) {
        self.compute_id = new.compute_id;
        merge_some!(
            self, new;
            name, project_id, node_id, node_type, node_directory, status, ports,
            port_name_format, port_segment_size, first_port_name, locked, label, console,
            console_host, console_type, console_auto_start, command_line, custom_adapters,
            height, width, symbol, x, y, z, template_id, properties, template
        );
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
            let ids: Vec<String> = conn
                .get_nodes(&pid)?
                .into_iter()
                .filter(|n| n.name.as_deref() == Some(name.as_str()))
                .filter_map(|n| n.node_id)
                .collect();
            match ids.as_slice() {
                [] => return Err(Error::not_found(format!("Node not found: {name}"))),
                [id] => self.node_id = Some(id.clone()),
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
        let data: Node = conn.call_json(Method::GET, &format!("/projects/{pid}/nodes/{nid}"), Body::Empty)?;
        self.apply(data);
        if get_links {
            self.get_links()?;
        }
        Ok(())
    }

    /// Retrieves the links attached to this node.
    pub fn get_links(&mut self) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let mut links: Vec<Link> = conn.call_json(
            Method::GET,
            &format!("/projects/{pid}/nodes/{nid}/links"),
            Body::Empty,
        )?;
        for link in &mut links {
            link.connector = Some(conn.clone());
        }
        self.links = links;
        Ok(())
    }

    /// POST an action (`start`, `stop`, `reload`, `suspend`); update from the answer when
    /// it already reports the expected status, otherwise re-fetch the node.
    fn action(&mut self, action: &str, expected: NodeStatus) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data: Node = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/nodes/{nid}/{action}"),
            Body::Empty,
        )?;
        if data.status == Some(expected) {
            self.apply(data);
            Ok(())
        } else {
            self.get()
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.action("start", NodeStatus::Started)
    }

    pub fn stop(&mut self) -> Result<()> {
        self.action("stop", NodeStatus::Stopped)
    }

    pub fn reload(&mut self) -> Result<()> {
        self.action("reload", NodeStatus::Started)
    }

    pub fn suspend(&mut self) -> Result<()> {
        self.action("suspend", NodeStatus::Suspended)
    }

    /// Updates the node on the server with the `Some` fields of `patch`.
    pub fn update(&mut self, patch: &NodeUpdate) -> Result<()> {
        let (conn, pid, nid) = self.require()?;
        let data: Node = conn.call_json(
            Method::PUT,
            &format!("/projects/{pid}/nodes/{nid}"),
            Body::json(patch)?,
        )?;
        self.apply(data);
        Ok(())
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
            self.template_id = template.template_id;
        }
        let cached = NodeUpdate::from(&*self);
        let template_id = self.template_id.clone().unwrap_or_default();
        let created: Node = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/templates/{template_id}"),
            Body::json(&FromTemplateRequest {
                x: 0,
                y: 0,
                compute_id: &self.compute_id,
            })?,
        )?;
        self.apply(created);
        self.update(&cached)
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