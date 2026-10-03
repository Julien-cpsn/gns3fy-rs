use std::fmt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    TemplateType {
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
    /// Valid link types.
    LinkType {
        Ethernet => "ethernet",
        Serial => "serial",
    }
);

string_enum!(
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


/// A text label attached to a node, or to one side of a link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Label {
    pub text: String,
    /// CSS-like style string, e.g. `font-family: TypeWriter;font-size: 10.0;`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i64>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Label {
            text: text.into(),
            ..Default::default()
        }
    }
}

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

/// How a link is drawn in the GUI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// Line style (`0` solid, `1` dash, ...).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub style_type: Option<i64>,
}

/// Packet filters applied on a link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkFilters {
    /// BPF expressions; matching packets are dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpf: Option<Vec<String>>,
    /// `[percentage]` of packets to corrupt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrupt: Option<Vec<i64>>,
    /// `[latency_ms]` or `[latency_ms, jitter_ms]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<Vec<i64>>,
    /// `[n]`: drop one packet out of n.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_drop: Option<Vec<i64>>,
    /// `[percentage]` of packets to lose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<Vec<i64>>,
}

/// A node port, as reported in `Node::ports`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Port {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    pub adapter_number: i64,
    pub port_number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    /// Capture link types, e.g. `{"Ethernet": "DLT_EN10MB"}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_link_types: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
}

/// One side of a [`Link`](crate::Link).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkEndpoint {
    pub node_id: String,
    pub adapter_number: i64,
    pub port_number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// A drawing (SVG element) of a project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Project statistics (`/projects/{id}/stats`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectStats {
    pub drawings: u64,
    pub links: u64,
    pub nodes: u64,
    pub snapshots: u64,
}

/// Project supplier (logo shown in the GUI).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Supplier {
    pub logo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A project variable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Variable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Per-adapter override (QEMU, Docker, VirtualBox, VMware).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomAdapter {
    pub adapter_number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
}

/// One entry of a `ports_mapping` (Ethernet switch/hub and cloud nodes and templates).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PortMapping {
    pub name: String,
    pub port_number: i64,
    /// `access`, `dot1q`, `qinq` (switch) or `ethernet`, `tap`, `udp` (cloud).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub port_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethertype: Option<String>,
    /// Host interface a cloud port is bound to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
}

/// A host interface known by a cloud node.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudInterface {
    pub name: String,
    pub special: bool,
    #[serde(rename = "type")]
    pub interface_type: String,
}