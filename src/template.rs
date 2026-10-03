//! GNS3 template object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::connector::{Body, Gns3Connector, LOCAL_COMPUTE};
use crate::error::{Error, Result};
use crate::template_kinds::TemplateKind;
use crate::types::{ConsoleType, Lookup, TemplateType};

/// A template: the blueprint a [`Node`](crate::Node) is created from.
///
/// The fields shared by every template type are plain fields; everything specific to a type
/// lives in [`kind`](Template::kind), an enum with one struct per type
/// ([`QemuTemplate`](crate::QemuTemplate), [`DockerTemplate`](crate::DockerTemplate), ...).
///
/// ```no_run
/// use std::sync::Arc;
/// use gns3fy_rs::{Gns3Connector, Lookup, Template, TemplateKind};
///
/// # fn main() -> gns3fy_rs::Result<()> {
/// let server = Arc::new(Gns3Connector::new("http://localhost:3080")?);
///
/// for t in Template::list(&server)? {
///     println!("{} ({})", t.name, t.template_type());
/// }
///
/// let mut alpine = Template::find(&server, Lookup::Name("alpine"))?.expect("template exists");
/// if let TemplateKind::Docker(docker) = &mut alpine.kind {
///     docker.start_command = Some("sh".into());
/// }
/// alpine.save()?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    pub name: String,
    /// `router`, `switch`, `guest`, `firewall`...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Built-in templates (cloud, NAT, VPCS, switches...) cannot be modified or deleted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_name_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_type: Option<ConsoleType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_auto_start: Option<bool>,
    /// The type specific settings; its variant is the `template_type`.
    #[serde(flatten)]
    pub kind: TemplateKind,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

/// Two templates are equal when all their data is equal; the connector is ignored.
impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        self.template_id == other.template_id
            && self.name == other.name
            && self.category == other.category
            && self.builtin == other.builtin
            && self.compute_id == other.compute_id
            && self.default_name_format == other.default_name_format
            && self.symbol == other.symbol
            && self.usage == other.usage
            && self.console_type == other.console_type
            && self.console_auto_start == other.console_auto_start
            && self.kind == other.kind
    }
}

impl Template {
    /// A template to [`create`](Template::create) on the server.
    pub fn new(connector: Arc<Gns3Connector>, name: impl Into<String>, kind: TemplateKind) -> Self {
        Template {
            template_id: None,
            name: name.into(),
            category: None,
            builtin: None,
            compute_id: None,
            default_name_format: None,
            symbol: None,
            usage: None,
            console_type: None,
            console_auto_start: None,
            kind,
            connector: Some(connector),
        }
    }

    pub fn with_template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    pub fn with_compute_id(mut self, compute_id: impl Into<String>) -> Self {
        self.compute_id = Some(compute_id.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_console_type(mut self, console_type: ConsoleType) -> Self {
        self.console_type = Some(console_type);
        self
    }

    /// The `template_type` of this template (the variant of [`kind`](Template::kind)).
    pub fn template_type(&self) -> TemplateType {
        self.kind.template_type()
    }

    /// Whether this is one of the server's built-in templates.
    pub fn is_builtin(&self) -> bool {
        self.builtin.unwrap_or(false)
    }

    // ---- listing / lookup ---------------------------------------------------------------

    /// All the templates defined on the server, bound to `connector`.
    pub fn list(connector: &Arc<Gns3Connector>) -> Result<Vec<Template>> {
        let mut templates = connector.get_templates()?;
        for t in &mut templates {
            t.connector = Some(connector.clone());
        }
        Ok(templates)
    }

    /// A template by ID (a 404 is an error) or by name (`None` when not found).
    pub fn find(connector: &Arc<Gns3Connector>, lookup: Lookup<'_>) -> Result<Option<Template>> {
        Ok(connector.get_template(lookup)?.map(|mut t| {
            t.connector = Some(connector.clone());
            t
        }))
    }

    // ---- helpers ------------------------------------------------------------------------

    fn connector(&self) -> Result<Arc<Gns3Connector>> {
        self.connector.clone().ok_or(Error::MissingConnector)
    }

    /// Replace the data with a server answer, keeping the connector.
    fn replace_with(&mut self, new: Template) {
        let connector = self.connector.take();
        *self = Template { connector, ..new };
    }

    /// Connector and template id; resolves `template_id` from `name` when missing.
    fn require(&mut self) -> Result<(Arc<Gns3Connector>, String)> {
        let conn = self.connector()?;
        if self.template_id.is_none() {
            let found = conn
                .get_template(Lookup::Name(&self.name))?
                .ok_or_else(|| Error::not_found(format!("Template not found: {}", self.name)))?;
            self.template_id = found.template_id;
        }
        let id = self.template_id.clone().unwrap_or_default();
        Ok((conn, id))
    }

    fn refuse_builtin(&self, action: &str) -> Result<()> {
        if self.is_builtin() {
            return Err(Error::invalid(format!(
                "Cannot {action} built-in template {}",
                self.name
            )));
        }
        Ok(())
    }

    // ---- server operations ----------------------------------------------------------------

    /// Refreshes the template from the server (by `template_id`, or by `name`).
    pub fn get(&mut self) -> Result<()> {
        let (conn, id) = self.require()?;
        let fresh = conn
            .get_template(Lookup::Id(&id))?
            .ok_or_else(|| Error::not_found(format!("Template not found: {id}")))?;
        self.replace_with(fresh);
        Ok(())
    }

    /// Creates the template on the server; `compute_id` defaults to `"local"` and the call
    /// fails when the name is already used.
    pub fn create(&mut self) -> Result<()> {
        if self.template_id.is_some() {
            return Err(Error::invalid("Template already created"));
        }
        let conn = self.connector()?;
        if self.compute_id.is_none() {
            self.compute_id = Some(LOCAL_COMPUTE.to_string());
        }
        let created = conn.create_template(self)?;
        self.replace_with(created);
        Ok(())
    }

    /// Sends the local template to the server (`PUT`): change [`kind`](Template::kind) and
    /// the other fields locally, then call `save`.
    pub fn save(&mut self) -> Result<()> {
        self.refuse_builtin("modify")?;
        let (conn, _) = self.require()?;
        let saved = conn.update_template(self)?;
        self.replace_with(saved);
        Ok(())
    }

    /// Deletes the template on the server and clears `template_id`.
    pub fn delete(&mut self) -> Result<()> {
        self.refuse_builtin("delete")?;
        let (conn, id) = self.require()?;
        conn.call(Method::DELETE, &format!("/templates/{id}"), Body::Empty)?;
        self.template_id = None;
        Ok(())
    }
}