//! # gns3fy
//!
//! Rust wrapper around the [GNS3 server REST API](http://api.gns3.net/en/2.2/index.html)
//! (GNS3 2.2+), ported from the Python `gns3fy` library. Its goal is to drive a GNS3 server
//! programmatically: network CI/CD pipelines, scripts, automation tooling.
//!
//! ```no_run
//! use std::sync::Arc;
//! use gns3fy_rs::{Gns3Connector, Lookup, Project};
//!
//! # fn main() -> gns3fy_rs::Result<()> {
//! let server = Arc::new(Gns3Connector::new("http://localhost:3080")?);
//!
//! let mut lab = Project::with_connector(server.clone()).with_name("API_TEST");
//! lab.get()?;
//! lab.open()?;
//! println!("{:?} {:?}", lab.status, lab.stats);
//!
//! for node in lab.nodes_summary()? {
//!     println!("{node}");
//! }
//!
//! if let Some(node) = lab.get_node_mut(Lookup::Name("alpine-1"))? {
//!     node.start()?;
//! }
//! # Ok(()) }
//! ```

mod util;

pub mod compute;
pub mod connector;
pub mod drawing_utils;
pub mod error;
pub mod link;
pub mod node;
pub mod project;
pub mod properties;
pub mod template;
pub mod template_kinds;
pub mod types;

pub use compute::{Compute, ComputeCapabilities, ComputeImage, ComputePorts};
pub use connector::{
    Body, Gns3Connector, Gns3ConnectorBuilder, ProjectSummary, TemplateSummary, Version,
    LOCAL_COMPUTE,
};
pub use error::{Error, Result};
pub use link::{Link, LinkUpdate};
pub use node::{Node, NodeUpdate};
pub use project::{
    LinkSummary, NodeInventory, NodeSummary, Project, ProjectUpdate, DEFAULT_POLL_WAIT,
};
pub use properties::NodeProperties;
pub use template::Template;
pub use template_kinds::{
    AtmSwitchTemplate, CloudTemplate, DockerTemplate, DynamipsTemplate, EthernetHubTemplate,
    EthernetSwitchTemplate, FrameRelaySwitchTemplate, IouTemplate, NatTemplate, QemuTemplate,
    TemplateKind, TracengTemplate, VirtualboxTemplate, VmwareTemplate, VpcsTemplate,
    VpcsTemplateProperties,
};
pub use types::{
    CloudInterface, ConsoleType, CustomAdapter, Drawing, Label, LinkEndpoint, LinkFilters,
    LinkStyle, LinkType, Lookup, NodeStatus, NodeType, Port, PortMapping, ProjectStats,
    ProjectStatus, Snapshot, Supplier, TemplateType, Variable,
};
