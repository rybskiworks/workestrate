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
    /// terminal unmasks; non-operator scopes may not (spec 22 §5).
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
/// Mask/unmask entries hold RAW pattern strings: the compiler validates and
/// compiles them against the declaring origin (spec 22 §6 pattern rejections
/// name the origin), so fragments must not pre-compile patterns.
/// `masked_writes` / `case_sensitivity` are likewise raw so the compiler can
/// name the offending value and origin in a `CompileError` (spec 22 §10, §6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MountsFragment {
    /// Mask entries, applied before this scope's `unmask` entries (spec 22
    /// §4: per-scope mask-then-unmask).
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<PolicyValueStringSchema>"))]
    pub mask: Vec<PolicyValue<String>>,
    /// Unmask entries: carve-outs to this scope's (or a lower-authority
    /// scope's provisional) masks.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<PolicyValueStringSchema>"))]
    pub unmask: Vec<PolicyValue<String>>,
    /// Raw `masked_writes` setting; the compiler accepts exactly `"deny"`
    /// (spec 22 §10).
    #[serde(default)]
    pub masked_writes: Option<String>,
    /// Raw `case_sensitivity` setting; v1 accepts exactly `"sensitive"`
    /// (spec 22 §6).
    #[serde(default)]
    pub case_sensitivity: Option<String>,
}

/// The public schema for a raw string policy value. Runtime deserialization
/// also supports the historical `value` alias, but the config surface is the
/// compact string or the documented `{ pattern, overridable }` table.
#[cfg(feature = "schema")]
#[derive(schemars::JsonSchema)]
#[schemars(rename = "PolicyValue_for_String")]
#[serde(untagged)]
#[allow(dead_code)]
enum PolicyValueStringSchema {
    Compact(String),
    Expanded(PolicyValueStringExpandedSchema),
}

#[cfg(feature = "schema")]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PolicyValueStringExpandedSchema {
    pattern: String,
    #[serde(default = "policy_value_overridable_default")]
    #[schemars(default = "policy_value_overridable_default")]
    overridable: bool,
}

#[cfg(feature = "schema")]
fn policy_value_overridable_default() -> bool {
    true
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
        }
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
            mask = [".env", { pattern = ".workestrate/", overridable = false }]
            unmask = [".env.example"]
            masked_writes = "deny"
            "#,
        )
        .unwrap();
        assert_eq!(fragment.mask.len(), 2);
        assert!(fragment.mask[0].overridable);
        assert!(!fragment.mask[1].overridable);
        assert_eq!(fragment.mask[1].value, ".workestrate/");
        assert_eq!(fragment.unmask.len(), 1);
        assert_eq!(fragment.masked_writes.as_deref(), Some("deny"));
        assert_eq!(fragment.case_sensitivity, None);
    }

    #[test]
    fn fragment_rejects_unknown_fields() {
        let err = toml::from_str::<MountsFragment>(r#"masks = [".env"]"#).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "unknown-field error must surface: {err}"
        );
    }
}
