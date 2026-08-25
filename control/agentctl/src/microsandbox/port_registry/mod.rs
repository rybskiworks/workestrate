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
/// `bind_ip`, `namespace`) are `#[serde(default)]` so older state files parse
/// cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceRecord {
    pub instance: String,
    pub context: Option<String>,
    pub workload: String,
    pub ports: Vec<u16>,
    /// Full host:guest port pairs. Populated by the instance-lifecycle path;
    /// absent (empty) on legacy records.
    /// Namespaced ports (P3): the mappings carry the declared port `name`, so
    /// `ps` and `depends_on` resolution surface the same names the workload
    /// declared.
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
    /// The declaring config repo of the DEPENDENT workload (ADR 0030 Phase 2
    /// T1 namespace scoping). A RESOLUTION FILTER, not a slot prefix: the
    /// singleton slot stays `<context>-<workload>` and `instance` is
    /// unchanged. Legacy records without the field parse as "default".
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// The CANONICAL invocation cwd recorded at create time (ADR 0030
    /// V-addendum §V3) — populated ONLY for `per-dir`-strategy workloads;
    /// the source-gone reconcile state compares it against the filesystem.
    /// `None` (or a missing/null field on disk) = unknown = LEGACY posture:
    /// never source-gone, never a hard fail (A1 interop).
    #[serde(default)]
    pub source_dir: Option<String>,
    /// A2 (ADR 0032 §Image tags — RESOLVED user decision 3): the immutable
    /// computed store tag (`<name>:<ctx>:<sha>`) this sandbox was CREATED
    /// with, recorded ONLY where the image is known at create time (the
    /// create-from-plan paths; re-start/adopt paths pass `None` — a
    /// re-adoption does not know the running tag). Feeds the keep-last-N GC
    /// protection set: every record's `image_tag` is never pruned.
    /// Conservative by design — stale records over-protect until teardown
    /// unregisters them. Legacy records without the field parse as `None`
    /// → unprotected, but their tags are legacy-shape and never GC
    /// candidates anyway.
    #[serde(default)]
    pub image_tag: Option<String>,
}

/// The default namespace for a registry record: the declaring config repo of
/// the dependent workload, defaulting to "default" when no repo identity is
/// resolvable (legacy records, synthetic layers).
pub fn default_namespace() -> String {
    "default".to_string()
}
