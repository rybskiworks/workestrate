//! Policy scopes: the six `[policy.mounts]` declaration points (spec 22 §2).
//!
//! Scopes are COLLECTED per layer in stack order and handed to the compiler
//! (ADR 0028: collect-and-compile, never merge). Compile order IS authority
//! order: operator scopes (home registry, user-global overrides) come first
//! and outrank repo/project scopes — an operator's terminal decision cannot
//! be reversed by anything later in the stack (spec 22 §4, §5).

use crate::mount_policy::rule::RuleOrigin;
use crate::mount_policy::value::PolicyValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// One of the six policy scopes (spec 22 §2). Declaration order IS authority
/// order (lower = higher authority = earlier in compile order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    /// Home registry `config.toml` — the operator's global policy.
    HomeRegistry,
    /// `overrides.toml` (user-global overrides, ADR 0019).
    UserGlobalOverrides,
    /// The reference config, which ships the sensitive defaults (spec 22 §9).
    ReferenceConfig,
    /// A config-repo layer, in registry stack order.
    ConfigRepoLayer,
    /// `[workloads.<name>.policy.mounts]`.
    Workload,
    /// A `[[workloads.<name>.mounts]]` entry's policy (declaring layer only;
    /// spec 22 §8.3 v1 limitation).
    MountEntry,
}

impl ScopeKind {
    /// Authority rank (spec 22 §2, §5): lower rank = higher authority =
    /// earlier in compile order.
    pub fn authority(self) -> u32 {
        self as u32
    }

    /// Operator scopes (home registry, user-global overrides) may declare
    /// final allows, and their final read.deny entries route to the protect
    /// wire bucket; non-operator scopes may not declare final allows
    /// (spec 22 §5).
    pub fn is_operator(self) -> bool {
        matches!(
            self,
            ScopeKind::HomeRegistry | ScopeKind::UserGlobalOverrides
        )
    }

    /// Human label used in origin display (spec 22 §11).
    pub fn label(self) -> &'static str {
        match self {
            ScopeKind::HomeRegistry => "home-registry",
            ScopeKind::UserGlobalOverrides => "user-global-overrides",
            ScopeKind::ReferenceConfig => "reference-config",
            ScopeKind::ConfigRepoLayer => "config-repo-layer",
            ScopeKind::Workload => "workload",
            ScopeKind::MountEntry => "mount-entry",
        }
    }
}

/// The raw `[policy.mounts]` fragment one scope declares (spec 22 §15).
///
/// Entries hold RAW pattern strings: the compiler validates and compiles
/// them against the declaring origin (spec 22 §6 pattern rejections name the
/// origin), so fragments must not pre-compile patterns. `case_sensitivity`
/// is likewise raw so the compiler can name the offending value and origin
/// in a `CompileError` (spec 22 §6). The former vocabulary (`mask`,
/// `unmask`, `protect`, `writes`, `masked_writes`, entry key `overridable`)
/// is intentionally gone: this pre-release surface rejects those keys as
/// unknown fields rather than silently accepting old policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MountsFragment {
    /// Read axis (visibility): `deny` hides paths, `allow` carves out
    /// exceptions; applied deny-then-allow within the scope (spec 22 §4:
    /// per-scope mask-then-unmask emission order).
    #[serde(default)]
    pub read: Option<AxisFragment>,
    /// Write axis (write admission): pattern-keyed `allow`/`deny`.
    #[serde(default)]
    pub write: Option<AxisFragment>,
    /// Raw `case_sensitivity` setting; v1 accepts exactly `"sensitive"`
    /// (spec 22 §6).
    #[serde(default)]
    pub case_sensitivity: Option<String>,
}

/// One policy axis (read or write) for one scope: `deny`/`allow` entry
/// lists. Entries are relaxable by default; `{ pattern = "...", final =
/// true }` freezes an entry against later scopes (terminal at compile time,
/// spec 22 §4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AxisFragment {
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<PolicyValueStringSchema>"))]
    pub deny: Vec<PolicyValue<String>>,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<PolicyValueStringSchema>"))]
    pub allow: Vec<PolicyValue<String>>,
}

/// The public schema for a raw string policy value. Runtime deserialization
/// also supports the historical `value` alias, but the config surface is the
/// compact string or the documented `{ pattern, final }` table.
// `pub` so the `[[mounts]]` sugar fields on `MountPlanWire`
// (`microsandbox/plan.rs`) can project the same schema shape.
#[cfg(feature = "schema")]
#[derive(schemars::JsonSchema)]
#[schemars(rename = "PolicyValue_for_String")]
#[serde(untagged)]
#[allow(dead_code)]
pub enum PolicyValueStringSchema {
    Compact(String),
    Expanded(PolicyValueStringExpandedSchema),
}

#[cfg(feature = "schema")]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct PolicyValueStringExpandedSchema {
    pattern: String,
    #[serde(default = "policy_value_final_default")]
    #[schemars(default = "policy_value_final_default")]
    r#final: bool,
}

#[cfg(feature = "schema")]
fn policy_value_final_default() -> bool {
    false
}

/// One collected scope: a policy fragment plus the provenance of where it
/// was declared (spec 22 §8: collected per layer, never merged).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyScope {
    /// Which of the six scopes declared this fragment (spec 22 §2).
    pub scope_kind: ScopeKind,
    /// The layer name (registry, overrides, reference, repo name, workload,
    /// mount entry) — spec 22 §11.
    pub layer_name: String,
    /// The provenance path (`<repo>#<relpath>` granularity per spec 17) —
    /// spec 22 §11.
    pub source_path: PathBuf,
    /// The raw `[policy.mounts]` fragment this scope declares.
    pub fragment: MountsFragment,
    /// Guest path of the mount declaring this entry policy, when applicable.
    pub mount_guest: Option<String>,
}

/// Collected policy fragments retained separately from the ordinary merged
/// config. Global scopes are shared by every workload; workload scopes are
/// keyed by workload name. Mount-entry scopes are already attached to the
/// corresponding workload list and are selected by the collector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedPolicy {
    pub global: Vec<PolicyScope>,
    pub workloads: HashMap<String, Vec<PolicyScope>>,
}

impl CollectedPolicy {
    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.workloads.values().all(Vec::is_empty)
    }
}

impl PolicyScope {
    /// Collect one scope's fragment with its provenance.
    pub fn new(
        scope_kind: ScopeKind,
        layer_name: impl Into<String>,
        source_path: impl Into<PathBuf>,
        fragment: MountsFragment,
    ) -> Self {
        Self {
            scope_kind,
            layer_name: layer_name.into(),
            source_path: source_path.into(),
            fragment,
            mount_guest: None,
        }
    }

    /// Tag this scope with the guest path of its declaring mount.
    pub fn for_mount(mut self, guest: impl Into<String>) -> Self {
        self.mount_guest = Some(guest.into());
        self
    }

    /// The origin rules compiled from this scope will carry (spec 22 §11).
    pub(crate) fn origin(&self) -> RuleOrigin {
        RuleOrigin {
            layer: self.layer_name.clone(),
            file: self.source_path.clone(),
            scope_kind: self.scope_kind,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn authority_order_is_compile_order_operator_first() {
        let mut kinds = [
            ScopeKind::MountEntry,
            ScopeKind::Workload,
            ScopeKind::ConfigRepoLayer,
            ScopeKind::HomeRegistry,
            ScopeKind::ReferenceConfig,
            ScopeKind::UserGlobalOverrides,
        ];
        kinds.sort_by_key(|kind| kind.authority());
        assert_eq!(
            kinds,
            [
                ScopeKind::HomeRegistry,
                ScopeKind::UserGlobalOverrides,
                ScopeKind::ReferenceConfig,
                ScopeKind::ConfigRepoLayer,
                ScopeKind::Workload,
                ScopeKind::MountEntry,
            ]
        );
    }

    #[test]
    fn operator_scopes_are_home_registry_and_user_global_overrides() {
        assert!(ScopeKind::HomeRegistry.is_operator());
        assert!(ScopeKind::UserGlobalOverrides.is_operator());
        for kind in [
            ScopeKind::ReferenceConfig,
            ScopeKind::ConfigRepoLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            assert!(
                !kind.is_operator(),
                "{kind:?} must not be an operator scope"
            );
        }
    }

    #[test]
    fn fragment_parses_the_spec_15_surface() {
        let fragment: MountsFragment = toml::from_str(
            r#"
            [read]
            deny = [".env", { pattern = ".workestrate/", final = true }]
            allow = [".env.example"]

            [write]
            allow = [".tmp/**"]
            deny = [{ pattern = ".secret/**", final = true }]
            "#,
        )
        .unwrap();
        let read = fragment.read.unwrap();
        assert_eq!(read.deny.len(), 2);
        assert!(!read.deny[0].terminal);
        assert!(read.deny[1].terminal);
        assert_eq!(read.deny[1].value, ".workestrate/");
        assert_eq!(read.allow.len(), 1);
        let write = fragment.write.unwrap();
        assert_eq!(write.allow[0].value, ".tmp/**");
        assert!(write.deny[0].terminal);
        assert_eq!(fragment.case_sensitivity, None);
    }

    #[test]
    fn fragment_rejects_unknown_fields() {
        let err = toml::from_str::<MountsFragment>(r#"reads = [".env"]"#).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "unknown-field error must surface: {err}"
        );
    }

    #[test]
    fn former_vocabulary_is_rejected_intentionally() {
        // The old surface words are REMOVED (no aliases): each must be a
        // hard unknown-field error.
        for doc in [
            r#"mask = [".env"]"#,
            r#"unmask = [".env"]"#,
            r#"protect = [".env"]"#,
            r#"masked_writes = "deny""#,
            r#"[writes]
               deny = [".env"]"#,
        ] {
            let err = toml::from_str::<MountsFragment>(doc).unwrap_err();
            assert!(
                err.to_string().contains("unknown field"),
                "{doc:?} must be rejected as an unknown field: {err}"
            );
        }
        // And the removed entry key inside an expanded form.
        let err = toml::from_str::<MountsFragment>(
            r#"[read]
               deny = [{ pattern = ".env", overridable = false }]"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unknown field") && err.to_string().contains("overridable"),
            "removed `overridable` entry key must be a hard unknown-field error: {err}"
        );
    }
}
