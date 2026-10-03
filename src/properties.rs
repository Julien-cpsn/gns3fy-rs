//! Typed `properties` of a node.
//!
//! GNS3 reports different properties per emulator (QEMU, Docker, IOU, ...). All of them are
//! optional fields of one struct, so a node can be read and patched without a lookup by type.
//! A node only carries the properties of its own type; the others stay `None`.
//!
//! Properties the server sends that are not listed here are ignored (and therefore not sent
//! back by an update).

use serde::{Deserialize, Serialize};

use crate::types::{CloudInterface, ConsoleType, PortMapping};

/// Emulator specific settings of a [`Node`](crate::Node).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeProperties {
    // ---- shared by several emulators -------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nvram: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_clone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_config_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_config_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aux: Option<i64>,

    // ---- ethernet switch / hub / cloud -------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_mapping: Option<Vec<PortMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<CloudInterface>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_http_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_console_type: Option<ConsoleType>,

    // ---- IOU -----------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethernet_adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_adapters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_keepalives: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_default_iou_values: Option<bool>,

    // ---- QEMU ----------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qemu_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_throttling: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_networking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_command_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hda_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hda_disk_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hda_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdb_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdb_disk_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdb_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdc_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdc_disk_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdc_disk_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdd_disk_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdd_disk_image_md5sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdd_disk_interface: Option<String>,

    // ---- Docker --------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
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

    // ---- VPCS ----------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_script_path: Option<String>,

    // ---- TraceNG -------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_destination: Option<String>,

    // ---- VirtualBox / VMware -------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_any_adapter: Option<bool>,

    // ---- Dynamips ------------------------------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamips_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chassis: Option<String>,
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
    pub mac_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midplane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_supplies: Option<Vec<i64>>,
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