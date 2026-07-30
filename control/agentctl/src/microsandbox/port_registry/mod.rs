//! Port registry: per-instance state files under `${state_dir}/var/run/`.
//!
//! WP2 split of the former `port_registry.rs` god-file into `lock` (registry
//! lock), `slug` (`--new` slug allocation), and `store` (record CRUD +
//! collision checks). Purely mechanical — no behavior changes.

use crate::microsandbox::plan::PortMapping;
use serde::{Deserialize, Serialize};

mod lock;
mod slug;
mod store;

pub use slug::*;
pub use store::*;

/// A running sandbox instance record stored in the port registry.
///
/// **Backward-compat:** `ports` (host-only u16 list) and the original four
/// fields are always present. Newer fields (`port_pairs`, `created_at`) are
/// `#[serde(default)]` so older state files parse cleanly.
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
}
