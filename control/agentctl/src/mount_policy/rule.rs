//! Compiled rules and their provenance (spec 22 §4, §11).
//!
//! Every compiled rule carries its [`RuleOrigin`] into the program so
//! diagnostics (explain traces, spec 22 §13) and compile errors (spec 22
//! §4-§6) can name the layer, file, and scope kind that declared it.

use crate::mount_policy::pattern::Pattern;
use crate::mount_policy::scope::ScopeKind;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// The effect a matching rule applies to a path (spec 22 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleEffect {
    /// Hide the path from the sandboxed agent (the fail-closed direction).
    Mask,
    /// Re-expose a path a lower-authority or same-scope mask hid.
    Unmask,
}

impl fmt::Display for RuleEffect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuleEffect::Mask => f.write_str("mask"),
            RuleEffect::Unmask => f.write_str("unmask"),
        }
    }
}

/// Provenance of a compiled rule (spec 22 §11): which layer, file, and scope
/// kind declared it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOrigin {
    /// The layer name (registry, overrides, reference, repo name, workload,
    /// mount entry).
    pub layer: String,
    /// The provenance path (`<repo>#<relpath>` granularity per spec 17).
    pub file: PathBuf,
    /// Which of the six scopes (spec 22 §2) declared the rule.
    pub scope_kind: ScopeKind,
}

impl fmt::Display for RuleOrigin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "layer '{}' ({}, {} scope)",
            self.layer,
            self.file.display(),
            self.scope_kind.label()
        )
    }
}

/// One compiled mask/unmask rule: an effect, a compiled pattern, the
/// overridable/terminal flag, and its origin (spec 22 §4, §11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathPolicyRule {
    /// Mask or unmask (spec 22 §4).
    pub effect: RuleEffect,
    /// The compiled glob (spec 22 §6).
    pub pattern: Pattern,
    /// Whether a later scope's matching rule may supersede this one (spec 22
    /// §4). `false` makes the rule terminal: its match freezes the decision
    /// per path.
    pub overridable: bool,
    /// Where the rule was declared (spec 22 §11).
    pub origin: RuleOrigin,
}

impl PathPolicyRule {
    /// True when this rule's match freezes the decision per path (spec 22
    /// §4: `overridable = false`).
    pub fn is_terminal(&self) -> bool {
        !self.overridable
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn origin() -> RuleOrigin {
        RuleOrigin {
            layer: "personal".to_string(),
            file: PathBuf::from("config.toml"),
            scope_kind: ScopeKind::HomeRegistry,
        }
    }

    #[test]
    fn origin_display_names_layer_file_and_scope_kind() {
        let text = origin().to_string();
        assert!(text.contains("personal"), "{text}");
        assert!(text.contains("config.toml"), "{text}");
        assert!(text.contains("home-registry"), "{text}");
    }

    #[test]
    fn terminal_is_the_negation_of_overridable() {
        let rule = PathPolicyRule {
            effect: RuleEffect::Mask,
            pattern: Pattern::parse(".env").unwrap(),
            overridable: false,
            origin: origin(),
        };
        assert!(rule.is_terminal());
        let rule = PathPolicyRule {
            overridable: true,
            ..rule
        };
        assert!(!rule.is_terminal());
    }
}
