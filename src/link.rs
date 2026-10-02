//! GNS3 link object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::connector::{Body, Gns3Connector};
use crate::error::{Error, Result};
use crate::types::{LinkEndpoint, LinkType};
use crate::util::{merge_update, payload};

/// A link between two node ports of a project.
///
/// Fields mirror the GNS3 API. Methods talk to the server through the assigned
/// [`connector`](Link::connector) and refresh the object with the server's answer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Link {
    pub link_id: Option<String>,
    pub link_type: Option<LinkType>,
    pub link_style: Option<Value>,
    pub project_id: Option<String>,
    pub suspend: Option<bool>,
    pub nodes: Option<Vec<LinkEndpoint>>,
    pub filters: Option<Map<String, Value>>,
    pub capturing: Option<bool>,
    pub capture_file_path: Option<String>,
    pub capture_file_name: Option<String>,
    pub capture_compute_id: Option<String>,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

impl Link {
    pub fn with_connector(connector: Arc<Gns3Connector>) -> Self {
        Link {
            connector: Some(connector),
            ..Default::default()
        }
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_link_id(mut self, link_id: impl Into<String>) -> Self {
        self.link_id = Some(link_id.into());
        self
    }

    /// Overlay the fields present in `data` (a server answer) on this object.
    fn apply(&mut self, data: &Value) -> Result<()> {
        let mut new: Link = merge_update(&*self, data)?;
        new.connector = self.connector.take();
        *self = new;
        Ok(())
    }

    fn connector_and_project(&self) -> Result<(Arc<Gns3Connector>, String)> {
        let conn = self.connector.clone().ok_or(Error::MissingConnector)?;
        let pid = self
            .project_id
            .clone()
            .ok_or_else(|| Error::invalid("Need to submit project_id"))?;
        Ok((conn, pid))
    }

    fn require(&self) -> Result<(Arc<Gns3Connector>, String, String)> {
        let (conn, pid) = self.connector_and_project()?;
        let lid = self
            .link_id
            .clone()
            .ok_or_else(|| Error::invalid("Need to submit link_id"))?;
        Ok((conn, pid, lid))
    }

    /// Retrieves the link from the server and updates this object.
    pub fn get(&mut self) -> Result<()> {
        let (conn, pid, lid) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/projects/{pid}/links/{lid}"), Body::Empty)?;
        self.apply(&data)
    }

    /// Deletes the link on the server and clears `project_id` / `link_id`.
    pub fn delete(&mut self) -> Result<()> {
        let (conn, pid, lid) = self.require()?;
        conn.call(Method::DELETE, &format!("/projects/{pid}/links/{lid}"), Body::Empty)?;
        self.project_id = None;
        self.link_id = None;
        Ok(())
    }

    /// Creates the link on the server (needs `project_id` and `nodes`).
    pub fn create(&mut self) -> Result<()> {
        let (conn, pid) = self.connector_and_project()?;
        let body = payload(&*self, &["connector"])?;
        let data = conn.call_json(Method::POST, &format!("/projects/{pid}/links"), Body::Json(body))?;
        self.apply(&data)
    }

    /// Updates the link on the server with the given JSON fields
    /// (`filters`, `suspend`, `nodes`, `link_style`...).
    pub fn update(&mut self, fields: Value) -> Result<()> {
        let (conn, pid, lid) = self.require()?;
        let data = conn.call_json(
            Method::PUT,
            &format!("/projects/{pid}/links/{lid}"),
            Body::Json(fields),
        )?;
        self.apply(&data)
    }
}
