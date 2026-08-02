//! Port registry: per-instance state files under `${state_dir}/var/run/`.
//!
//! WP2 split of the former `port_registry.rs` god-file into `lock` (registry
//! lock), `slug` (`--new` slug allocation), and `store` (record CRUD +
//! collision checks). Purely mechanical — no behavior changes.

use crate::microsandbox::plan::PortMapping;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// `lock` is crate-visible so `images::lock` (spec 21 §3.3) can reuse the
// stale dead-PID lock recovery probe instead of duplicating it.
pub(crate) mod lock;
mod slug;
mod store;

pub use slug::*;
pub use store::*;

/// A running sandbox instance record stored in the port registry.
///
/// **Backward-compat:** `ports` (host-only u16 list) and the original four
/// fields are always present. Newer fields (`port_pairs`, `created_at`,
/// `bind_ip`) are `#[serde(default)]` so older state files parse cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceRecord {
    pub instance: String,
    pub context: Option<String>,
    pub workload: String,
    pub ports: Vec<u16>,
    /// Full host:guest port pairs. Populated by the instance-lifecycle path;
    /// absent (empty) on legacy records.
    #[serde(default)]
    pub port_pairs: Vec<PortMapping>,
    /// RFC3339 timestamp the instance was registered. Empty for legacy records.
    #[serde(default)]
    pub created_at: String,
    /// Instance bind IP (ADR 0026). 127.0.0.1 = singleton/shared bind;
    /// 127.0.0.N (N>=2) = parallel slot. Legacy records without the field
    /// parse as 127.0.0.1.
    #[serde(default = "crate::microsandbox::plan::default_bind_ip")]
    pub bind_ip: IpAddr,
}
