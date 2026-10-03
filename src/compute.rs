//! Compute (emulation host) objects.

use serde::{Deserialize, Serialize};

use crate::types::NodeType;

/// What a compute can run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ComputeCapabilities {
    pub node_types: Vec<NodeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<u32>,
    /// Total memory in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    /// Total disk size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_size: Option<u64>,
}

/// A compute, as returned by `/computes`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Compute {
    pub compute_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `http` or `https`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_usage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ComputeCapabilities>,
}

/// An image stored on a compute (`/computes/{id}/{emulator}/images`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ComputeImage {
    pub filename: String,
    pub path: String,
    pub filesize: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5sum: Option<String>,
}

/// Ports used and available on a compute (`/computes/{id}/ports`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ComputePorts {
    /// `[first, last]` console port the compute may allocate.
    pub console_port_range: (u16, u16),
    /// Console ports currently in use.
    pub console_ports: Vec<u16>,
    pub udp_port_range: (u16, u16),
    /// UDP ports currently in use.
    pub udp_ports: Vec<u16>,
}