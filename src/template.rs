//! GNS3 template object.

use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::connector::{Body, Gns3Connector, LOCAL_COMPUTE};
use crate::error::{Error, Result};
use crate::types::{ConsoleType, Lookup, TemplateType};
use crate::util::{merge_update, payload, str_field};

/// A template: the blueprint a [`Node`](crate::Node) is created from.
///
/// The fields shared by every template type are typed. Everything specific to a type
/// (`ram`, `image`, `hda_disk_image`, `adapters`, `start_command`, `nvram`...) lives in
/// [`properties`](Template::properties) and is preserved when the template is sent back to
/// the server, so nothing is lost on a round trip. Use [`property`](Template::property) /
/// [`set_property`](Template::set_property) to access them.
///
/// ```no_run
/// use std::sync::Arc;
/// use gns3fy_rs::{Gns3Connector, Lookup, Template};
///
/// # fn main() -> gns3fy_rs::Result<()> {
/// let server = Arc::new(Gns3Connector::new("http://localhost:3080")?);
///
/// for t in Template::list(&server)? {
///     println!("{} ({:?})", t.name.as_deref().unwrap_or("?"), t.template_type);
/// }
///
/// let mut alpine = Template::find(&server, Lookup::Name("alpine"))?.expect("template exists");
/// alpine.set_property("start_command", "sh");
/// alpine.save()?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Template {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_type: Option<TemplateType>,
    /// `router`, `switch`, `guest`, `firewall`...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Built-in templates (cloud, NAT, VPCS, switches...) cannot be modified or deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_type: Option<ConsoleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_auto_start: Option<bool>,
    /// Type-specific settings, exactly as the server reports them.
    #[serde(flatten)]
    pub properties: Map<String, Value>,
    #[serde(skip)]
    pub connector: Option<Arc<Gns3Connector>>,
}

/// Two templates are equal when all their data is equal; the connector is ignored.
impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        self.template_id == other.template_id
            && self.name == other.name
            && self.template_type == other.template_type
            && self.category == other.category
            && self.builtin == other.builtin
            && self.compute_id == other.compute_id
            && self.default_name_format == other.default_name_format
            && self.symbol == other.symbol
            && self.usage == other.usage
            && self.console_type == other.console_type
            && self.console_auto_start == other.console_auto_start
            && self.properties == other.properties
    }
}

impl Template {
    /// An empty template bound to a connector, ready to be filled and [`create`](Template::create)d.
    pub fn with_connector(connector: Arc<Gns3Connector>) -> Self {
        Template {
            connector: Some(connector),
            ..Default::default()
        }
    }

    /// A template to create: `name` and `template_type` are the required fields.
    pub fn new(
        connector: Arc<Gns3Connector>,
        name: impl Into<String>,
        template_type: TemplateType,
    ) -> Self {
        Template {
            name: Some(name.into()),
            template_type: Some(template_type),
            connector: Some(connector),
            ..Default::default()
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    pub fn with_template_type(mut self, template_type: TemplateType) -> Self {
        self.template_type = Some(template_type);
        self
    }

    pub fn with_compute_id(mut self, compute_id: impl Into<String>) -> Self {
        self.compute_id = Some(compute_id.into());
        self
    }

    /// Sets a type-specific property (builder form).
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// A type-specific property, e.g. `property("ram")` on a QEMU template.
    pub fn property(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    /// Sets a type-specific property locally; send it with [`save`](Template::save).
    pub fn set_property(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.properties.insert(key.into(), value.into());
    }

    /// Whether this is one of the server's built-in templates.
    pub fn is_builtin(&self) -> bool {
        self.builtin.unwrap_or(false)
    }

    // ---- listing / lookup ---------------------------------------------------------------

    /// All the templates defined on the server, bound to `connector`.
    pub fn list(connector: &Arc<Gns3Connector>) -> Result<Vec<Template>> {
        connector
            .get_templates()?
            .into_iter()
            .map(|v| Template::from_server(v, connector))
            .collect()
    }

    /// A template by ID (a 404 is an error) or by name (`None` when not found).
    pub fn find(connector: &Arc<Gns3Connector>, lookup: Lookup<'_>) -> Result<Option<Template>> {
        connector
            .get_template(lookup)?
            .map(|v| Template::from_server(v, connector))
            .transpose()
    }

    fn from_server(value: Value, connector: &Arc<Gns3Connector>) -> Result<Template> {
        let mut t: Template = serde_json::from_value(value)?;
        t.connector = Some(connector.clone());
        Ok(t)
    }

    // ---- helpers ------------------------------------------------------------------------

    /// Overlay a server answer on this object, keeping the connector.
    fn apply(&mut self, data: &Value) -> Result<()> {
        let mut new: Template = merge_update(&*self, data)?;
        new.connector = self.connector.take();
        *self = new;
        Ok(())
    }

    fn connector(&self) -> Result<Arc<Gns3Connector>> {
        self.connector.clone().ok_or(Error::MissingConnector)
    }

    /// Connector and template id; resolves `template_id` from `name` when missing.
    fn require(&mut self) -> Result<(Arc<Gns3Connector>, String)> {
        let conn = self.connector()?;
        if self.template_id.is_none() {
            let name = self
                .name
                .clone()
                .ok_or_else(|| Error::invalid("Need to either submit template_id or name"))?;
            let found = conn
                .get_template(Lookup::Name(&name))?
                .ok_or_else(|| Error::not_found(format!("Template not found: {name}")))?;
            self.template_id = str_field(&found, "template_id").map(String::from);
        }
        let id = self.template_id.clone().unwrap_or_default();
        Ok((conn, id))
    }

    fn refuse_builtin(&self, action: &str) -> Result<()> {
        if self.is_builtin() {
            return Err(Error::invalid(format!(
                "Cannot {action} built-in template {}",
                self.name.as_deref().unwrap_or("?")
            )));
        }
        Ok(())
    }

    // ---- server operations ----------------------------------------------------------------

    /// Retrieves the template (by `template_id`, or by `name`) and updates this object.
    pub fn get(&mut self) -> Result<()> {
        let (conn, id) = self.require()?;
        let data = conn.call_json(Method::GET, &format!("/templates/{id}"), Body::Empty)?;
        self.apply(&data)
    }

    /// Creates the template on the server. `name` and `template_type` are required,
    /// `compute_id` defaults to `"local"`; fails when the name is already used.
    pub fn create(&mut self) -> Result<()> {
        if self.template_id.is_some() {
            return Err(Error::invalid("Template already created"));
        }
        let conn = self.connector()?;
        if self.name.is_none() {
            return Err(Error::invalid("Parameter 'name' is mandatory"));
        }
        if self.template_type.is_none() {
            return Err(Error::invalid("Parameter 'template_type' is mandatory"));
        }
        if self.compute_id.is_none() {
            self.compute_id = Some(LOCAL_COMPUTE.to_string());
        }
        let body = payload(&*self, &["connector"])?;
        let data = conn.create_template(body)?;
        self.apply(&data)
    }

    /// Sends the **whole** local template to the server (`PUT`), so edit fields and
    /// properties locally, then call `save`.
    pub fn save(&mut self) -> Result<()> {
        self.refuse_builtin("modify")?;
        let (conn, id) = self.require()?;
        let body = payload(&*self, &["connector"])?;
        let data = conn.call_json(Method::PUT, &format!("/templates/{id}"), Body::Json(body))?;
        self.apply(&data)
    }

    /// Updates the template with the given JSON fields. The stored template is fetched,
    /// the fields are overlaid and the result is sent back (like the Python library).
    pub fn update(&mut self, fields: Value) -> Result<()> {
        self.refuse_builtin("modify")?;
        let (conn, id) = self.require()?;
        let data = conn.update_template(Lookup::Id(&id), fields)?;
        self.apply(&data)
    }

    /// Deletes the template on the server and clears `template_id` and `name`.
    pub fn delete(&mut self) -> Result<()> {
        self.refuse_builtin("delete")?;
        let (conn, id) = self.require()?;
        conn.call(Method::DELETE, &format!("/templates/{id}"), Body::Empty)?;
        self.template_id = None;
        self.name = None;
        Ok(())
    }
}
