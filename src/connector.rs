//! Connector to the GNS3 server controller API.

use std::fmt;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::compute::{Compute, ComputeImage, ComputePorts};
use crate::error::{Error, Result};
use crate::link::Link;
use crate::node::Node;
use crate::project::Project;
use crate::template::Template;
use crate::types::{Lookup, ProjectStats};

/// Name of the default compute.
pub const LOCAL_COMPUTE: &str = "local";

/// Request body for [`Gns3Connector::http_call`].
#[derive(Debug, Default)]
pub enum Body {
    #[default]
    Empty,
    /// An already serialized JSON document; build it with [`Body::json`].
    Json(String),
    /// Raw bytes (file contents...).
    Bytes(Vec<u8>),
    /// A file streamed as request body (image uploads).
    File(File),
}

impl Body {
    /// Serializes `value` as a JSON request body.
    pub fn json<T: Serialize + ?Sized>(value: &T) -> Result<Body> {
        Ok(Body::Json(serde_json::to_string(value)?))
    }
}

/// Error document returned by the GNS3 server.
#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    status: Option<u16>,
    message: Option<String>,
}


/// Response of `GET /version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub local: bool,
    pub version: String,
}

/// One row of [`Gns3Connector::projects_summary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub project_id: String,
    pub total_nodes: u64,
    pub total_links: u64,
    pub status: String,
}

impl fmt::Display for ProjectSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} -- Nodes: {} -- Links: {} -- Status: {}",
            self.name, self.project_id, self.total_nodes, self.total_links, self.status
        )
    }
}

/// One row of [`Gns3Connector::templates_summary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub name: String,
    pub template_id: String,
    pub template_type: String,
    pub builtin: bool,
    /// `"N/A"` when the template has no console type.
    pub console_type: String,
    pub category: String,
}

impl fmt::Display for TemplateSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} -- Type: {} -- Builtin: {} -- Console: {} -- Category: {}",
            self.name,
            self.template_id,
            self.template_type,
            self.builtin,
            self.console_type,
            self.category
        )
    }
}

/// Builder for [`Gns3Connector`].
#[derive(Debug, Clone)]
pub struct Gns3ConnectorBuilder {
    url: String,
    user: Option<String>,
    cred: Option<String>,
    verify: bool,
    api_version: u32,
    timeout: Option<Duration>,
}

impl Gns3ConnectorBuilder {
    /// Username for HTTP basic authentication.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
    /// Password for HTTP basic authentication.
    pub fn cred(mut self, cred: impl Into<String>) -> Self {
        self.cred = Some(cred.into());
        self
    }
    /// Verify the server TLS certificate (default `false`, like the Python library).
    pub fn verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }
    /// REST API version (default `2`).
    pub fn api_version(mut self, version: u32) -> Self {
        self.api_version = version;
        self
    }
    /// Per-request timeout (default: none, like the Python library).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    pub fn build(self) -> Result<Gns3Connector> {
        let client = Client::builder()
            .danger_accept_invalid_certs(!self.verify)
            .timeout(self.timeout)
            .build()?;
        Ok(Gns3Connector {
            base_url: format!("{}/v{}", self.url.trim_matches('/'), self.api_version),
            user: self.user,
            cred: self.cred,
            verify: self.verify,
            api_version: self.api_version,
            api_calls: AtomicU64::new(0),
            client,
        })
    }
}

/// Connector used to interact with the GNS3 server controller API.
///
/// Wrap it in an [`Arc`](std::sync::Arc) to share it between [`Project`](crate::Project),
/// [`Node`](crate::Node) and [`Link`](crate::Link) objects.
///
/// ```no_run
/// use std::sync::Arc;
/// use gns3fy_rs::Gns3Connector;
///
/// # fn main() -> gns3fy_rs::Result<()> {
/// let server = Arc::new(Gns3Connector::new("http://localhost:3080")?);
/// println!("{:?}", server.get_version()?);
/// # Ok(()) }
/// ```
pub struct Gns3Connector {
    base_url: String,
    user: Option<String>,
    cred: Option<String>,
    verify: bool,
    api_version: u32,
    api_calls: AtomicU64,
    client: Client,
}

impl fmt::Debug for Gns3Connector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gns3Connector")
            .field("base_url", &self.base_url)
            .field("user", &self.user)
            .field("verify", &self.verify)
            .field("api_calls", &self.api_calls())
            .finish()
    }
}
impl Gns3Connector {
    /// Connector with default options (API v2, no auth, no TLS verification).
    pub fn new(url: impl AsRef<str>) -> Result<Self> {
        Self::builder(url).build()
    }

    pub fn builder(url: impl AsRef<str>) -> Gns3ConnectorBuilder {
        Gns3ConnectorBuilder {
            url: url.as_ref().to_string(),
            user: None,
            cred: None,
            verify: false,
            api_version: 2,
            timeout: None,
        }
    }

    /// `url` + `/v{api_version}`.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn api_version(&self) -> u32 {
        self.api_version
    }

    /// Number of HTTP calls performed so far.
    pub fn api_calls(&self) -> u64 {
        self.api_calls.load(Ordering::Relaxed)
    }

    /// Performs an HTTP operation against an absolute `url`.
    ///
    /// Non-2xx answers are turned into [`Error::Api`] using the `status`/`message` fields
    /// GNS3 returns.
    pub fn http_call(
        &self,
        method: Method,
        url: &str,
        body: Body,
        params: &[(&str, &str)],
    ) -> Result<Response> {
        let mut req = self.client.request(method, url).header("Accept", "application/json");
        if let Some(user) = &self.user {
            req = req.basic_auth(user, self.cred.as_deref());
        }
        if !params.is_empty() {
            req = req.query(params);
        }
        req = match body {
            Body::Empty => req,
            Body::Json(text) => req.header(CONTENT_TYPE, "application/json").body(text),
            Body::Bytes(b) => req.body(b),
            Body::File(f) => req.body(f),
        };
        let response = req.send()?;
        self.api_calls.fetch_add(1, Ordering::Relaxed);

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let text = response.text().unwrap_or_default();
            return Err(match serde_json::from_str::<ApiErrorBody>(&text) {
                Ok(ApiErrorBody {
                       status: body_status,
                       message: Some(message),
                   }) => Error::Api {
                    status: body_status.unwrap_or_else(|| status.as_u16()),
                    message,
                },
                _ => Error::Api {
                    status: status.as_u16(),
                    message: text,
                },
            });
        }
        Ok(response)
    }

    /// Call a path relative to [`base_url`](Self::base_url) (must start with `/`).
    pub(crate) fn call(&self, method: Method, path: &str, body: Body) -> Result<Response> {
        let url = format!("{}{}", self.base_url, path);
        self.http_call(method, &url, body, &[])
    }

    /// Like [`call`](Self::call), decoding the JSON answer into `T`.
    pub(crate) fn call_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Body,
    ) -> Result<T> {
        let text = self.call(method, path, body)?.text()?;
        Ok(serde_json::from_str(&text)?)
    }

    fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.call_json(Method::GET, path, Body::Empty)
    }

    /// Version information of the GNS3 server.
    pub fn get_version(&self) -> Result<Version> {
        self.get_json("/version")
    }

    // ---- projects ----------------------------------------------------------------------

    /// Summary of the projects in the server (with node/link counts from the stats API).
    pub fn projects_summary(&self) -> Result<Vec<ProjectSummary>> {
        let mut out = Vec::new();
        for p in self.get_projects()? {
            let project_id = p.project_id.unwrap_or_default();
            let stats: ProjectStats = self.get_json(&format!("/projects/{project_id}/stats"))?;
            out.push(ProjectSummary {
                name: p.name.unwrap_or_default(),
                total_nodes: stats.nodes,
                total_links: stats.links,
                status: p.status.map(|s| s.to_string()).unwrap_or_default(),
                project_id,
            });
        }
        Ok(out)
    }

    /// The projects on the server. The returned objects have no connector assigned; use
    /// [`Project::with_connector`] to work on one.
    pub fn get_projects(&self) -> Result<Vec<Project>> {
        self.get_json("/projects")
    }

    /// Retrieves a project by ID (404 is an error) or by name (`None` when not found).
    pub fn get_project(&self, lookup: Lookup<'_>) -> Result<Option<Project>> {
        match lookup {
            Lookup::Id(id) => Ok(Some(self.get_json(&format!("/projects/{id}"))?)),
            Lookup::Name(name) => Ok(self
                .get_projects()?
                .into_iter()
                .find(|p| p.name.as_deref() == Some(name))),
        }
    }

    /// Creates a project (`name` is required) and returns it as the server stored it.
    pub fn create_project(&self, project: &Project) -> Result<Project> {
        if project.name.is_none() {
            return Err(Error::invalid("Parameter 'name' is mandatory"));
        }
        self.call_json(Method::POST, "/projects", Body::json(project)?)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        self.call(Method::DELETE, &format!("/projects/{project_id}"), Body::Empty)?;
        Ok(())
    }

    // ---- templates ---------------------------------------------------------------------

    /// Summary of the templates in the server.
    pub fn templates_summary(&self) -> Result<Vec<TemplateSummary>> {
        Ok(self
            .get_templates()?
            .into_iter()
            .map(|t| TemplateSummary {
                template_type: t.template_type().to_string(),
                builtin: t.is_builtin(),
                console_type: t.console_type.map_or("N/A".to_string(), |c| c.to_string()),
                category: t.category.unwrap_or_default(),
                template_id: t.template_id.unwrap_or_default(),
                name: t.name,
            })
            .collect())
    }

    /// The templates defined on the server. The returned objects have no connector
    /// assigned; [`Template::list`] returns them bound to one.
    pub fn get_templates(&self) -> Result<Vec<Template>> {
        self.get_json("/templates")
    }

    /// Retrieves a template by ID (404 is an error) or by name (`None` when not found).
    pub fn get_template(&self, lookup: Lookup<'_>) -> Result<Option<Template>> {
        match lookup {
            Lookup::Id(id) => Ok(Some(self.get_json(&format!("/templates/{id}"))?)),
            Lookup::Name(name) => Ok(self.get_templates()?.into_iter().find(|t| t.name == name)),
        }
    }

    /// Creates a template. `compute_id` defaults to `"local"`; fails if a template with the
    /// same name already exists. Returns the template as the server stored it.
    pub fn create_template(&self, template: &Template) -> Result<Template> {
        if template.name.is_empty() {
            return Err(Error::invalid("Parameter 'name' is mandatory"));
        }
        if self.get_template(Lookup::Name(&template.name))?.is_some() {
            return Err(Error::invalid(format!(
                "Template already used: {}",
                template.name
            )));
        }
        let mut body = template.clone();
        body.connector = None;
        if body.compute_id.is_none() {
            body.compute_id = Some(LOCAL_COMPUTE.to_string());
        }
        self.call_json(Method::POST, "/templates", Body::json(&body)?)
    }

    /// Sends `template` (which needs a `template_id`) to the server and returns the stored
    /// template.
    pub fn update_template(&self, template: &Template) -> Result<Template> {
        let id = template
            .template_id
            .as_deref()
            .ok_or_else(|| Error::invalid("Need to submit template_id"))?;
        self.call_json(Method::PUT, &format!("/templates/{id}"), Body::json(template)?)
    }

    /// Deletes a template by ID or name.
    pub fn delete_template(&self, lookup: Lookup<'_>) -> Result<()> {
        let id = match lookup {
            Lookup::Id(id) => id.to_string(),
            Lookup::Name(name) => self
                .get_template(lookup)?
                .and_then(|t| t.template_id)
                .ok_or_else(|| Error::not_found(format!("Template not found: {name}")))?,
        };
        self.call(Method::DELETE, &format!("/templates/{id}"), Body::Empty)?;
        Ok(())
    }

    // ---- nodes and links of a project ----------------------------------------------------

    /// Nodes defined on a project (without connector; see [`Project::get_nodes`]).
    pub fn get_nodes(&self, project_id: &str) -> Result<Vec<Node>> {
        self.get_json(&format!("/projects/{project_id}/nodes"))
    }

    pub fn get_node(&self, project_id: &str, node_id: &str) -> Result<Node> {
        self.get_json(&format!("/projects/{project_id}/nodes/{node_id}"))
    }

    /// Links defined on a project (without connector; see [`Project::get_links`]).
    pub fn get_links(&self, project_id: &str) -> Result<Vec<Link>> {
        self.get_json(&format!("/projects/{project_id}/links"))
    }

    pub fn get_link(&self, project_id: &str, link_id: &str) -> Result<Link> {
        self.get_json(&format!("/projects/{project_id}/links/{link_id}"))
    }

    // ---- computes ------------------------------------------------------------------------

    /// List of computes, with attributes such as cpu/memory usage.
    pub fn get_computes(&self) -> Result<Vec<Compute>> {
        self.get_json("/computes")
    }

    /// A compute (use [`LOCAL_COMPUTE`] for the default one).
    pub fn get_compute(&self, compute_id: &str) -> Result<Compute> {
        self.get_json(&format!("/computes/{compute_id}"))
    }

    /// Images available on a compute for an emulator (`qemu`, `iou`, `docker`...).
    pub fn get_compute_images(&self, emulator: &str, compute_id: &str) -> Result<Vec<ComputeImage>> {
        self.get_json(&format!("/computes/{compute_id}/{emulator}/images"))
    }

    /// Uploads an image file to a compute.
    pub fn upload_compute_image(
        &self,
        emulator: &str,
        file_path: impl AsRef<Path>,
        compute_id: &str,
    ) -> Result<()> {
        let path = file_path.as_ref();
        if !path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not find file: {}", path.display()),
            )));
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::invalid("Invalid file name"))?;
        let file = File::open(path)?;
        self.call(
            Method::POST,
            &format!("/computes/{compute_id}/{emulator}/images/{filename}"),
            Body::File(file),
        )?;
        Ok(())
    }

    /// Ports used and configured by a compute.
    pub fn get_compute_ports(&self, compute_id: &str) -> Result<ComputePorts> {
        self.get_json(&format!("/computes/{compute_id}/ports"))
    }
}