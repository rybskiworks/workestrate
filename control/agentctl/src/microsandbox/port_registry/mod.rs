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
    /// A3 (ADR 0032 §Provenance stamps): the image out-path hash segment of
    /// the computed store tag the sandbox was CREATED from (the trailing sha
    /// of a `<name>:<sha>` / `<name>:<ctx>:<sha>` tag). `None` for registry
    /// refs and legacy declared tags — they have no content hash, so
    /// staleness then rides `config_hash` alone. Absent on disk = pre-stamp
    /// record (unknown-version posture): parses fine, NEVER auto-stale,
    /// never triggers replace.
    #[serde(default)]
    pub image_out_hash: Option<String>,
    /// A3 (ADR 0032 §Provenance stamps): FNV-1a-64 config hash over the
    /// canonical RUNTIME-RELEVANT serialization of the creating plan
    /// (`microsandbox::provenance::config_hash_of_plan` — image out path,
    /// env incl baked, mounts + policies, command, resources, network,
    /// secret names; comments/docs/formatting/unrelated-workload edits
    /// cannot churn it by construction). Consumed by the ADR 0030 §V4
    /// on_skew disposition and the `ps` staleness display. Absent/None =
    /// PRE-STAMP record: parses fine, NEVER auto-stale, never triggers
    /// replace.
    #[serde(default)]
    pub config_hash: Option<String>,
}

/// The default namespace for a registry record: the declaring config repo of
/// the dependent workload, defaulting to "default" when no repo identity is
/// resolvable (legacy records, synthetic layers).
pub fn default_namespace() -> String {
    "default".to_string()
}
