//! GNS3 link object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::connector::{Body, Gns3Connector};
use crate::error::{Error, Result};
use crate::types::{LinkEndpoint, LinkFilters, LinkStyle, LinkType};
use crate::util::merge_some;

/// A link between two node ports of a project.
///
/// Fields mirror the GNS3 API. Methods talk to the server through the assigned
/// [`connector`](Link::connector) and refresh the object with the server's answer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Link {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_type: Option<LinkType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_style: Option<LinkStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<LinkEndpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<LinkFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capturing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_compute_id: Option<String>,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

/// The attributes of a link that can be changed on the server; `None` fields are left as
/// they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<LinkFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_style: Option<LinkStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<LinkEndpoint>>,
    /// `true` pauses the link (packets are dropped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<bool>,
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

    /// Overlay the fields present in a server answer on this object.
    fn apply(&mut self, new: Link) {
        merge_some!(
            self, new;
            link_id, link_type, link_style, project_id, suspend, nodes, filters, capturing,
            capture_file_path, capture_file_name, capture_compute_id
        );
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
        let data: Link = conn.call_json(Method::GET, &format!("/projects/{pid}/links/{lid}"), Body::Empty)?;
        self.apply(data);
        Ok(())
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
        let data: Link = conn.call_json(
            Method::POST,
            &format!("/projects/{pid}/links"),
            Body::json(&*self)?,
        )?;
        self.apply(data);
        Ok(())
    }

    /// Updates the link on the server with the `Some` fields of `patch`.
    pub fn update(&mut self, patch: &LinkUpdate) -> Result<()> {
        let (conn, pid, lid) = self.require()?;
        let data: Link = conn.call_json(
            Method::PUT,
            &format!("/projects/{pid}/links/{lid}"),
            Body::json(patch)?,
        )?;
        self.apply(data);
        Ok(())
    }
}