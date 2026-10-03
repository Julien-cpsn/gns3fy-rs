//! Type specific settings of a [`Template`](crate::Template), one struct per template type.
//!
//! Fields the server sends that are not modelled here are ignored. The server accepts
//! partial updates, so they are left untouched on the server when a template is saved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::{ConsoleType, CustomAdapter, PortMapping, TemplateType};

/// The settings of a template, selected by its `template_type`.
///
/// On the wire the variant name is the `template_type` field and the variant's fields sit
/// next to the common ones (`name`, `category`, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "template_type", rename_all = "snake_case")]
pub enum TemplateKind {
    Cloud(CloudTemplate),
    Nat(NatTemplate),
    EthernetHub(EthernetHubTemplate),
    EthernetSwitch(EthernetSwitchTemplate),
    FrameRelaySwitch(FrameRelaySwitchTemplate),
    AtmSwitch(AtmSwitchTemplate),
    Docker(DockerTemplate),
    Dynamips(DynamipsTemplate),
    Vpcs(VpcsTemplate),
    Traceng(TracengTemplate),
    Virtualbox(VirtualboxTemplate),
    Vmware(VmwareTemplate),
    Iou(IouTemplate),
    Qemu(QemuTemplate),
}

impl TemplateKind {
    /// The `template_type` of this kind.
    pub fn template_type(&self) -> TemplateType {
        match self {
            TemplateKind::Cloud(_) => TemplateType::Cloud,
            TemplateKind::Nat(_) => TemplateType::Nat,
            TemplateKind::EthernetHub(_) => TemplateType::EthernetHub,
            TemplateKind::EthernetSwitch(_) => TemplateType::EthernetSwitch,
            TemplateKind::FrameRelaySwitch(_) => TemplateType::FrameRelaySwitch,
            TemplateKind::AtmSwitch(_) => TemplateType::AtmSwitch,
            TemplateKind::Docker(_) => TemplateType::Docker,
            TemplateKind::Dynamips(_) => TemplateType::Dynamips,
            TemplateKind::Vpcs(_) => TemplateType::Vpcs,
            TemplateKind::Traceng(_) => TemplateType::Traceng,
            TemplateKind::Virtualbox(_) => TemplateType::Virtualbox,
            TemplateKind::Vmware(_) => TemplateType::Vmware,
            TemplateKind::Iou(_) => TemplateType::Iou,
            TemplateKind::Qemu(_) => TemplateType::Qemu,
        }
    }

    pub fn as_qemu(&self) -> Option<&QemuTemplate> {
        match self {
            TemplateKind::Qemu(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_qemu_mut(&mut self) -> Option<&mut QemuTemplate> {
        match self {
            TemplateKind::Qemu(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_docker(&self) -> Option<&DockerTemplate> {
        match self {
            TemplateKind::Docker(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_docker_mut(&mut self) -> Option<&mut DockerTemplate> {
        match self {
            TemplateKind::Docker(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_iou(&self) -> Option<&IouTemplate> {
        match self {
            TemplateKind::Iou(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_iou_mut(&mut self) -> Option<&mut IouTemplate> {
        match self {
            TemplateKind::Iou(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_dynamips(&self) -> Option<&DynamipsTemplate> {
        match self {
            TemplateKind::Dynamips(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_vpcs(&self) -> Option<&VpcsTemplate> {
        match self {
            TemplateKind::Vpcs(t) => Some(t),
            _ => None,
        }
    }
}

/// Cloud template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_mapping: Option<Vec<PortMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_http_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_type: Option<ConsoleType>,
}

/// NAT template (no settings).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatTemplate {}

/// Ethernet hub template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EthernetHubTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_mapping: Option<Vec<PortMapping>>,
}

/// Ethernet switch template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EthernetSwitchTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_mapping: Option<Vec<PortMapping>>,
}

/// Frame Relay switch template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameRelaySwitchTemplate {
    /// e.g. `{"1:101": "2:202"}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<BTreeMap<String, String>>,
}

/// ATM switch template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AtmSwitchTemplate {
    /// e.g. `{"1:0:100": "2:0:200"}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<BTreeMap<String, String>>,
}

/// Docker template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_command: Option<String>,
    /// One `KEY=value` per line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_hosts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_http_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_http_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
}

/// Dynamips (Cisco router) template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DynamipsTemplate {
    /// `c1700`, `c2600`, `c2691`, `c3600`, `c3725`, `c3745`, `c7200`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nvram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chassis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midplane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idlepc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idlemax: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idlesleep: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_area: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mmap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparsemem: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iomem: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk0: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk1: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_delete_disks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wic0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wic1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wic2: Option<String>,
}

/// The `properties` object some servers nest inside a VPCS template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VpcsTemplateProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_script_file: Option<String>,
}

/// VPCS template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VpcsTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_script_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<VpcsTemplateProperties>,
}

/// TraceNG template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TracengTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_destination: Option<String>,
}

/// VirtualBox template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VirtualboxTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_any_adapter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_clone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_port_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_segment_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
}

/// VMware template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VmwareTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_any_adapter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_clone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_port_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_segment_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
}

/// IOU template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IouTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethernet_adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nvram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_keepalives: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_default_iou_values: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_config: Option<String>,
}

/// QEMU template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QemuTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qemu_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_port_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_segment_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_adapters: Option<Vec<CustomAdapter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_throttling: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_networking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_clone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_command_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hda_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hda_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdb_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdb_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdc_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdc_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdd_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdd_disk_interface: Option<String>,
}