//! GNS3 project object.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::connector::{Body, Gns3Connector};
use crate::error::{Error, Result};
use crate::link::Link;
use crate::node::Node;
use crate::types::{
    ConsoleType, Drawing, LinkEndpoint, Lookup, NodeType, Port, ProjectStats, ProjectStatus,
    Snapshot,
};
use crate::util::{merge_update, payload, str_field};

/// Default wait between a bulk node action (start/stop/...) and the node refresh.
pub const DEFAULT_POLL_WAIT: Duration = Duration::from_secs(5);

/// A GNS3 project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub name: Option<String>,
    pub project_id: Option<String>,
    pub status: Option<ProjectStatus>,
    pub path: Option<String>,
    pub filename: Option<String>,
    pub auto_start: Option<bool>,
    pub auto_close: Option<bool>,
    pub auto_open: Option<bool>,
    pub drawing_grid_size: Option<i64>,
    pub grid_size: Option<i64>,
    pub scene_height: Option<i64>,
    pub scene_width: Option<i64>,
    pub show_grid: Option<bool>,
    pub show_interface_labels: Option<bool>,
    pub show_layers: Option<bool>,
    pub snap_to_grid: Option<bool>,
    pub supplier: Option<Value>,
    pub variables: Option<Vec<Value>>,
    pub zoom: Option<i64>,
    pub stats: Option<ProjectStats>,
    pub snapshots: Option<Vec<Snapshot>>,
    pub drawings: Option<Vec<Drawing>>,
    #[serde(skip)]
    pub nodes: Vec<Node>,
    #[serde(skip)]
    pub links: Vec<Link>,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

/// One row of [`Project::nodes_summary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSummary {
    pub name: Option<String>,
    pub status: Option<String>,
    pub console: Option<i64>,
    pub node_id: Option<String>,
}

impl fmt::Display for NodeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = |o: &Option<String>| o.clone().unwrap_or_else(|| "None".into());
        let console = self.console.map_or("None".to_string(), |c| c.to_string());
        write!(
            f,
            "{}: {} -- Console: {} -- ID: {}",
            s(&self.name),
            s(&self.status),
            console,
            s(&self.node_id)
        )
    }
}

/// One entry of [`Project::nodes_inventory`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInventory {
    pub server: Option<String>,
    pub name: Option<String>,
    pub console_port: Option<i64>,
    pub console_type: Option<ConsoleType>,
    #[serde(rename = "type")]
    pub node_type: Option<NodeType>,
    pub template: Option<String>,
}

/// One row of [`Project::links_summary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSummary {
    pub node_a: String,
    pub port_a: String,
    pub node_b: String,
    pub port_b: String,
}

impl fmt::Display for LinkSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} ---- {}: {}",
            self.node_a, self.port_a, self.node_b, self.port_b
        )
    }
}

impl Project {
    pub fn with_connector(connector: Arc<Gns3Connector>) -> Self {
        Project {
            connector: Some(connector),
            ..Default::default()
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    fn apply(&mut self, data: &Value) -> Result<()> {
        let mut new: Project = merge_update(&*self, data)?;
        new.nodes = std::mem::take(&mut self.nodes);
        new.links = std::mem::take(&mut self.links);
        new.connector = self.connector.take();
        *self = new;
        Ok(())
    }

    fn require(&self) -> Result<(Arc<Gns3Connector>, String)> {
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let pid = self
            .project_id
            .clone()
            .ok_or_else(|| Error::invalid("Need to submit project_id"))?;
        Ok((conn, pid))
    }

    // ---- project lifecycle -------------------------------------------------------------

    /// Retrieves the project (by `project_id`, or by `name` when no id is known) together
    /// with stats, snapshots, drawings, nodes and links.
    pub fn get(&mut self) -> Result<bool> {
        self.get_with(true, true, true)
    }

    /// Like [`get`](Project::get), choosing which related objects to fetch.
    pub fn get_with(&mut self, get_links: bool, get_nodes: bool, get_stats: bool) -> Result<bool> {
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        if self.project_id.is_none() {
            let name = self
                .name
                .clone()
                .ok_or_else(|| Error::invalid("Need to submit either project_id or name"))?;
            for p in conn.get_projects()? {
                if str_field(&p, "name") == Some(name.as_str()) {
                    self.project_id = str_field(&p, "project_id").map(String::from);
                }
            }
            if self.project_id.is_none() {
                return Err(Error::not_found(format!("Project not found: {name}")));
            }
        }
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}"), Body::Empty)?;
        self.apply(&data)?;
        if get_stats {
            self.get_stats()?;
            let stats = self.stats.clone().unwrap_or_default();
            if stats.snapshots > 0 {
                self.get_snapshots()?;
            }
            if stats.drawings > 0 {
                self.get_drawings()?;
            }
        }
        if get_nodes {
            self.get_nodes()?;
        }
        if get_links {
            self.get_links()?;
        }
        Ok(true)
    }

    /// Creates the project on the server (`name` is required).
    pub fn create(&mut self) -> Result<()> {
        if self.name.is_none() {
            return Err(Error::invalid("Need to submit project name"));
        }
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let body = payload(&*self, &["stats", "nodes", "links", "connector"])?;
        let data = conn.call_json(Method::POST, "/projects", Body::Json(body))?;
        self.apply(&data)
    }

    /// Updates the project on the server with the given JSON fields.
    pub fn update(&mut self, fields: Value) -> Result<()> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::PUT, &format!("/projects/{pid}"), Body::Json(fields))?;
        self.apply(&data)
    }

    /// Deletes the project and clears `project_id` / `name`.
    pub fn delete(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        conn.call(Method::DELETE, &format!("/projects/{pid}"), Body::Empty)?;
        self.project_id = None;
        self.name = None;
        Ok(())
    }

    /// Closes the project.
    pub fn close(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let resp = conn.call(Method::POST, &format!("/projects/{pid}/close"), Body::Empty)?;
        if resp.status().as_u16() == 204 {
            self.status = Some(ProjectStatus::Closed);
        }
        Ok(())
    }

    /// Opens the project.
    pub fn open(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::POST, &format!("/projects/{pid}/open"), Body::Empty)?;
        self.apply(&data)
    }

    /// Refreshes `stats`.
    pub fn get_stats(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}/stats"), Body::Empty)?;
        self.stats = Some(serde_json::from_value(data)?);
        Ok(())
    }

    /// Reads a file from the project directory.
    pub fn get_file(&self, path: &str) -> Result<String> {
        let (conn, pid) = self.require()?;
        Ok(conn
            .call(Method::GET, &format!("/projects/{pid}/files/{path}"), Body::Empty)?
            .text()?)
    }

    /// Writes a file in the project directory.
    pub fn write_file(&self, path: &str, data: impl Into<Vec<u8>>) -> Result<()> {
        let (conn, pid) = self.require()?;
        conn.call(
            Method::POST,
            &format!("/projects/{pid}/files/{path}"),
            Body::Bytes(data.into()),
        )?;
        Ok(())
    }

    // ---- nodes -------------------------------------------------------------------------

    /// Refreshes `nodes` from the server.
    pub fn get_nodes(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let raw: Vec<Value> = serde_json::from_value(conn.call_json(
            Method::GET,
            &format!("/projects/{pid}/nodes"),
            Body::Empty,
        )?)?;
        let mut nodes = Vec::with_capacity(raw.len());
        for v in raw {
            let mut node: Node = serde_json::from_value(v)?;
            node.connector = Some(conn.clone());
            node.project_id = Some(pid.clone());
            nodes.push(node);
        }
        self.nodes = nodes;
        Ok(())
    }

    /// Refreshes `links` from the server.
    pub fn get_links(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let raw: Vec<Value> = serde_json::from_value(conn.call_json(
            Method::GET,
            &format!("/projects/{pid}/links"),
            Body::Empty,
        )?)?;
        let mut links = Vec::with_capacity(raw.len());
        for v in raw {
            let mut link: Link = serde_json::from_value(v)?;
            link.connector = Some(conn.clone());
            link.project_id = Some(pid.clone());
            links.push(link);
        }
        self.links = links;
        Ok(())
    }

    fn nodes_action(&mut self, action: &str, poll_wait: Duration) -> Result<()> {
        let (conn, pid) = self.require()?;
        conn.call(Method::POST, &format!("/projects/{pid}/nodes/{action}"), Body::Empty)?;
        std::thread::sleep(poll_wait);
        self.get_nodes()
    }

    /// Starts all nodes, waits `poll_wait`, then refreshes the nodes.
    pub fn start_nodes(&mut self, poll_wait: Duration) -> Result<()> {
        self.nodes_action("start", poll_wait)
    }

    pub fn stop_nodes(&mut self, poll_wait: Duration) -> Result<()> {
        self.nodes_action("stop", poll_wait)
    }

    pub fn reload_nodes(&mut self, poll_wait: Duration) -> Result<()> {
        self.nodes_action("reload", poll_wait)
    }

    pub fn suspend_nodes(&mut self, poll_wait: Duration) -> Result<()> {
        self.nodes_action("suspend", poll_wait)
    }

    fn ensure_nodes(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            self.get_nodes()?;
        }
        Ok(())
    }

    fn ensure_links(&mut self) -> Result<()> {
        if self.links.is_empty() {
            self.get_links()?;
        }
        Ok(())
    }

    /// `(name, status, console, node_id)` for every node.
    pub fn nodes_summary(&mut self) -> Result<Vec<NodeSummary>> {
        self.ensure_nodes()?;
        Ok(self
            .nodes
            .iter()
            .map(|n| NodeSummary {
                name: n.name.clone(),
                status: n.status.map(|s| s.to_string()),
                console: n.console,
                node_id: n.node_id.clone(),
            })
            .collect())
    }

    /// Inventory of the nodes keyed by node name (useful for Ansible-like tooling).
    pub fn nodes_inventory(&mut self) -> Result<BTreeMap<String, NodeInventory>> {
        self.ensure_nodes()?;
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let server = url::Url::parse(conn.base_url())?.host_str().map(String::from);
        Ok(self
            .nodes
            .iter()
            .map(|n| {
                (
                    n.name.clone().unwrap_or_default(),
                    NodeInventory {
                        server: server.clone(),
                        name: n.name.clone(),
                        console_port: n.console,
                        console_type: n.console_type,
                        node_type: n.node_type,
                        template: n.template.clone(),
                    },
                )
            })
            .collect())
    }

    /// Resolves a port of the node `node_id` for a link endpoint.
    fn endpoint_port(&self, endpoint: &LinkEndpoint) -> Result<(&Node, &Port)> {
        let node = self
            .nodes
            .iter()
            .find(|n| n.node_id.as_deref() == Some(endpoint.node_id.as_str()))
            .ok_or_else(|| Error::not_found(format!("Node not found: {}", endpoint.node_id)))?;
        let port = node
            .ports
            .iter()
            .flatten()
            .find(|p| {
                p.port_number == endpoint.port_number && p.adapter_number == endpoint.adapter_number
            })
            .ok_or_else(|| {
                Error::not_found(format!(
                    "Port {}/{} not found on node {}",
                    endpoint.adapter_number,
                    endpoint.port_number,
                    node.name.as_deref().unwrap_or("?")
                ))
            })?;
        Ok((node, port))
    }

    /// `(node_a, port_a, node_b, port_b)` for every link of the project.
    pub fn links_summary(&mut self) -> Result<Vec<LinkSummary>> {
        self.ensure_nodes()?;
        self.ensure_links()?;
        let mut out = Vec::new();
        for link in &self.links {
            let Some([a, b, ..]) = link.nodes.as_deref() else {
                continue;
            };
            let (node_a, port_a) = self.endpoint_port(a)?;
            let (node_b, port_b) = self.endpoint_port(b)?;
            out.push(LinkSummary {
                node_a: node_a.name.clone().unwrap_or_default(),
                port_a: port_a.name.clone(),
                node_b: node_b.name.clone().unwrap_or_default(),
                port_b: port_b.name.clone(),
            });
        }
        Ok(out)
    }

    /// Finds a node by name or ID (loading the nodes when not yet loaded).
    pub fn get_node(&mut self, lookup: Lookup<'_>) -> Result<Option<&Node>> {
        self.ensure_nodes()?;
        Ok(self.nodes.iter().find(|n| match lookup {
            Lookup::Id(id) => n.node_id.as_deref() == Some(id),
            Lookup::Name(name) => n.name.as_deref() == Some(name),
        }))
    }

    /// Mutable variant of [`get_node`](Project::get_node), to act on the node directly.
    pub fn get_node_mut(&mut self, lookup: Lookup<'_>) -> Result<Option<&mut Node>> {
        self.ensure_nodes()?;
        Ok(self.nodes.iter_mut().find(|n| match lookup {
            Lookup::Id(id) => n.node_id.as_deref() == Some(id),
            Lookup::Name(name) => n.name.as_deref() == Some(name),
        }))
    }

    /// Finds a link by ID (loading the links when not yet loaded).
    pub fn get_link(&mut self, link_id: &str) -> Result<Option<&Link>> {
        self.ensure_links()?;
        Ok(self.links.iter().find(|l| l.link_id.as_deref() == Some(link_id)))
    }

    /// Creates a node from a template: pass a [`Node`] with `name` and `template` (or
    /// `template_id`) set; `project_id` and `connector` are filled in for you.
    pub fn create_node(&mut self, mut node: Node) -> Result<&Node> {
        self.ensure_nodes()?;
        let (conn, pid) = self.require()?;
        node.project_id = Some(pid);
        node.connector = Some(conn);
        node.create()?;
        self.nodes.push(node);
        Ok(self.nodes.last().expect("just pushed"))
    }

    /// Resolves `(node, port)` names into a [`Node`] clone and [`Port`] clone.
    fn resolve_port(&mut self, node: &str, port: &str, label: &str) -> Result<(Node, Port)> {
        let n = self
            .get_node(Lookup::Name(node))?
            .ok_or_else(|| Error::invalid(format!("{label}: {node} not found")))?
            .clone();
        let p = n
            .port(port)
            .ok_or_else(|| Error::invalid(format!("port_{}: {port} not found", &label[5..])))?
            .clone();
        Ok((n, p))
    }

    /// Creates a link between `node_a:port_a` and `node_b:port_b` (names as in the GUI).
    /// Fails when one of the ports is already used by another link.
    pub fn create_link(
        &mut self,
        node_a: &str,
        port_a: &str,
        node_b: &str,
        port_b: &str,
    ) -> Result<&Link> {
        self.ensure_nodes()?;
        self.ensure_links()?;
        let (na, pa) = self.resolve_port(node_a, port_a, "node_a")?;
        let (nb, pb) = self.resolve_port(node_b, port_b, "node_b")?;
        let ea = endpoint(&na, &pa);
        let eb = endpoint(&nb, &pb);

        let used = self.links.iter().find(|l| {
            l.nodes
                .iter()
                .flatten()
                .any(|e| e.same_port(&ea) || e.same_port(&eb))
        });
        if let Some(l) = used {
            return Err(Error::invalid(format!(
                "At least one port is used, ID: {}",
                l.link_id.as_deref().unwrap_or("?")
            )));
        }

        let (conn, pid) = self.require()?;
        let mut link = Link {
            project_id: Some(pid),
            connector: Some(conn),
            nodes: Some(vec![ea, eb]),
            ..Default::default()
        };
        link.create()?;
        self.links.push(link);
        Ok(self.links.last().expect("just pushed"))
    }

    /// Deletes the link between `node_a:port_a` and `node_b:port_b` (either orientation).
    pub fn delete_link(
        &mut self,
        node_a: &str,
        port_a: &str,
        node_b: &str,
        port_b: &str,
    ) -> Result<()> {
        self.ensure_nodes()?;
        self.ensure_links()?;
        let (na, pa) = self.resolve_port(node_a, port_a, "node_a")?;
        let (nb, pb) = self.resolve_port(node_b, port_b, "node_b")?;
        let ea = endpoint(&na, &pa);
        let eb = endpoint(&nb, &pb);

        let index = self
            .links
            .iter()
            .position(|l| match l.nodes.as_deref() {
                Some([x, y, ..]) => {
                    (x.same_port(&ea) && y.same_port(&eb)) || (x.same_port(&eb) && y.same_port(&ea))
                }
                _ => false,
            })
            .ok_or_else(|| {
                Error::invalid(format!("Link not found: {node_a}:{port_a} <-> {node_b}:{port_b}"))
            })?;
        let mut link = self.links.remove(index);
        link.delete()
    }

    // ---- snapshots ---------------------------------------------------------------------

    /// Refreshes `snapshots` from the server.
    pub fn get_snapshots(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}/snapshots"), Body::Empty)?;
        self.snapshots = Some(serde_json::from_value(data)?);
        Ok(())
    }

    /// Finds a snapshot by name or ID (loading snapshots when not yet loaded).
    pub fn get_snapshot(&mut self, lookup: Lookup<'_>) -> Result<Option<Snapshot>> {
        if self.snapshots.as_ref().map_or(true, Vec::is_empty) {
            self.get_snapshots()?;
        }
        Ok(self.snapshots.iter().flatten().find(|s| match lookup {
            Lookup::Id(id) => s.snapshot_id == id,
            Lookup::Name(name) => s.name == name,
        }).cloned())
    }

    /// Creates a snapshot; fails if one with the same name exists.
    pub fn create_snapshot(&mut self, name: &str) -> Result<Snapshot> {
        let (conn, pid) = self.require()?;
        self.get_snapshots()?;
        if self.get_snapshot(Lookup::Name(name))?.is_some() {
            return Err(Error::invalid("Snapshot already created"));
        }
        let data = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/snapshots"),
            Body::Json(json!({ "name": name })),
        )?;
        let snapshot: Snapshot = serde_json::from_value(data)?;
        self.snapshots.get_or_insert_with(Vec::new).push(snapshot.clone());
        Ok(snapshot)
    }

    fn existing_snapshot(&mut self, lookup: Lookup<'_>) -> Result<Snapshot> {
        self.require()?;
        self.get_snapshots()?;
        self.get_snapshot(lookup)?
            .ok_or_else(|| Error::not_found("Snapshot not found"))
    }

    /// Deletes a snapshot by name or ID.
    pub fn delete_snapshot(&mut self, lookup: Lookup<'_>) -> Result<()> {
        let (conn, pid) = self.require()?;
        let snap = self.existing_snapshot(lookup)?;
        conn.call(
            Method::DELETE,
            &format!("/projects/{pid}/snapshots/{}", snap.snapshot_id),
            Body::Empty,
        )?;
        self.get_snapshots()
    }

    /// Restores a snapshot by name or ID, then refreshes the project.
    pub fn restore_snapshot(&mut self, lookup: Lookup<'_>) -> Result<bool> {
        let (conn, pid) = self.require()?;
        let snap = self.existing_snapshot(lookup)?;
        conn.call(
            Method::POST,
            &format!("/projects/{pid}/snapshots/{}/restore", snap.snapshot_id),
            Body::Empty,
        )?;
        self.get()
    }

    // ---- layout & drawings -------------------------------------------------------------

    /// Arranges the nodes on a circle of the given radius around the origin.
    pub fn arrange_nodes_circular(&mut self, radius: f64) -> Result<()> {
        self.get()?;
        if self.status != Some(ProjectStatus::Opened) {
            self.open()?;
        }
        if self.nodes.is_empty() {
            return Ok(());
        }
        let angle = (2.0 * PI) / self.nodes.len() as f64;
        for (index, node) in self.nodes.iter_mut().enumerate() {
            let x = (radius * (angle * index as f64).sin()) as i64;
            let y = (radius * -(angle * index as f64).cos()) as i64;
            node.update(json!({ "x": x, "y": y }))?;
        }
        Ok(())
    }

    /// Refreshes `drawings` from the server.
    pub fn get_drawings(&mut self) -> Result<()> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}/drawings"), Body::Empty)?;
        self.drawings = Some(serde_json::from_value(data)?);
        Ok(())
    }

    /// Finds a drawing by ID (loading drawings when not yet loaded).
    pub fn get_drawing(&mut self, drawing_id: &str) -> Result<Option<Drawing>> {
        if self.drawings.as_ref().map_or(true, Vec::is_empty) {
            self.get_drawings()?;
        }
        Ok(self
            .drawings
            .iter()
            .flatten()
            .find(|d| d.drawing_id == drawing_id)
            .cloned())
    }

    /// Creates a drawing from an SVG string. Positions default to `x=10, y=10, z=1` in the
    /// Python library; pass them explicitly here.
    pub fn create_drawing(&mut self, svg: &str, locked: bool, x: i64, y: i64, z: i64) -> Result<Drawing> {
        let (conn, pid) = self.require()?;
        let data = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/drawings"),
            Body::Json(json!({ "svg": svg, "locked": locked, "x": x, "y": y, "z": z })),
        )?;
        let drawing: Drawing = serde_json::from_value(data)?;
        self.drawings.get_or_insert_with(Vec::new).push(drawing.clone());
        Ok(drawing)
    }

    /// Updates a drawing; `None` fields keep their current value.
    pub fn update_drawing(
        &mut self,
        drawing_id: &str,
        svg: Option<&str>,
        locked: Option<bool>,
        x: Option<i64>,
        y: Option<i64>,
        z: Option<i64>,
    ) -> Result<Drawing> {
        let (conn, pid) = self.require()?;
        let current = self
            .get_drawing(drawing_id)?
            .ok_or_else(|| Error::not_found("drawing not found"))?;
        let body = json!({
            "svg": svg.map(String::from).unwrap_or(current.svg),
            "locked": locked.or(current.locked),
            "x": x.or(current.x),
            "y": y.or(current.y),
            "z": z.or(current.z),
        });
        let data = conn.call_json(
            Method::PUT,
            &format!("/projects/{pid}/drawings/{drawing_id}"),
            Body::Json(body),
        )?;
        self.get_drawings()?;
        Ok(serde_json::from_value(data)?)
    }

    /// Deletes a drawing by ID.
    pub fn delete_drawing(&mut self, drawing_id: &str) -> Result<()> {
        let (conn, pid) = self.require()?;
        self.get_drawings()?;
        let drawing = self
            .get_drawing(drawing_id)?
            .ok_or_else(|| Error::not_found("drawing not found"))?;
        conn.call(
            Method::DELETE,
            &format!("/projects/{pid}/drawings/{}", drawing.drawing_id),
            Body::Empty,
        )?;
        self.get_drawings()
    }
}

fn endpoint(node: &Node, port: &Port) -> LinkEndpoint {
    LinkEndpoint {
        node_id: node.node_id.clone().unwrap_or_default(),
        adapter_number: port.adapter_number,
        port_number: port.port_number,
        label: Some(json!({ "text": port.name })),
        extra: Default::default(),
    }
}
