//! Enums and small helper structs shared by the models.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

macro_rules! string_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $s)] $variant),+
        }
        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $s),+ }
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

string_enum!(
    /// Valid GNS3 node types.
    NodeType {
        Cloud => "cloud",
        Nat => "nat",
        EthernetHub => "ethernet_hub",
        EthernetSwitch => "ethernet_switch",
        FrameRelaySwitch => "frame_relay_switch",
        AtmSwitch => "atm_switch",
        Docker => "docker",
        Dynamips => "dynamips",
        Vpcs => "vpcs",
        Traceng => "traceng",
        Virtualbox => "virtualbox",
        Vmware => "vmware",
        Iou => "iou",
        Qemu => "qemu",
    }
);

string_enum!(
    /// Valid node console types.
    ConsoleType {
        Vnc => "vnc",
        Telnet => "telnet",
        Http => "http",
        Https => "https",
        Spice => "spice",
        SpiceAgent => "spice+agent",
        NoConsole => "none",
        Null => "null",
    }
);

string_enum!(
    /// Valid link types.
    LinkType {
        Ethernet => "ethernet",
        Serial => "serial",
    }
);

string_enum!(
    /// Node status.
    NodeStatus {
        Stopped => "stopped",
        Started => "started",
        Suspended => "suspended",
    }
);

string_enum!(
    /// Project status.
    ProjectStatus {
        Opened => "opened",
        Closed => "closed",
    }
);

/// Selects an object either by its UUID or by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup<'a> {
    Id(&'a str),
    Name(&'a str),
}

/// A node port, as reported in `Node::ports`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Port {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    pub adapter_number: i64,
    pub port_number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_link_types: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One side of a [`Link`](crate::Link).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkEndpoint {
    pub node_id: String,
    pub adapter_number: i64,
    pub port_number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl LinkEndpoint {
    /// True when both endpoints point at the same node/adapter/port.
    pub fn same_port(&self, other: &LinkEndpoint) -> bool {
        self.node_id == other.node_id
            && self.adapter_number == other.adapter_number
            && self.port_number == other.port_number
    }
}

/// A project snapshot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A drawing (SVG element) of a project.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Drawing {
    pub drawing_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i64>,
    pub svg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Project statistics (`/projects/{id}/stats`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectStats {
    pub drawings: u64,
    pub links: u64,
    pub nodes: u64,
    pub snapshots: u64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
