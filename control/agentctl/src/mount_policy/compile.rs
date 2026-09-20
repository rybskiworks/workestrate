//! The policy compiler: collected scopes → compiled program (spec 22 §8,
//! ADR 0028).
//!
//! Policy fragments are collected per scope in layer-stack order; this
//! compiler owns precedence, freeze semantics, trust validation, and
//! conflict detection. `merge.rs` never sees policy.
//!
//! Compile-time checks (each names the declaring origin, spec 22 §11):
//!
//! - patterns are compiled; rejections name the origin (spec 22 §6);
//! - exact-duplicate same-scope deny+allow pairs for the same raw pattern
//!   where at least one entry is final are an error naming BOTH origins —
//!   on the read axis (deny entries routed to the protect bucket included)
//!   AND mirrored on the write axis (spec 22 §4);
//! - a non-operator scope declaring a final allow (read.allow or
//!   write.allow) is rejected; final denies are allowed from ANY scope
//!   (denying is the fail-closed direction) (spec 22 §5);
//! - an OPERATOR scope's final read.deny routes to the protect wire bucket;
//! - `case_sensitivity` v1 accepts exactly `"sensitive"` (spec 22 §6).

use crate::mount_policy::pattern::{Pattern, PatternError};
use crate::mount_policy::program::{CaseSensitivity, CompiledRuleSet, MountPolicyProgram};
use crate::mount_policy::rule::{PathPolicyRule, RuleEffect, RuleOrigin};
use crate::mount_policy::scope::PolicyScope;
use crate::mount_policy::value::PolicyValue;
use std::fmt;

/// Compile-time rejection of a collected scope set. Every variant names the
/// declaring origin (spec 22 §11); the duplicate-conflict variant names BOTH
/// origins (spec 22 §4).
#[derive(Debug)]
pub enum CompileError {
    /// A pattern failed validation or glob compilation (spec 22 §6). The
    /// source error carries the origin.
    Pattern {
        /// The rejection, with the declaring origin attached.
        source: PatternError,
    },
    /// Exact-duplicate same-scope deny+allow for the same raw pattern with
    /// at least one final entry (spec 22 §4): a contradiction the author
    /// must resolve, not a precedence question. Applies to BOTH axes (read
    /// and write). Boxed to keep the error type small.
    DuplicateTerminalConflict(Box<DuplicateConflict>),
    /// A non-operator scope declared a final allow — on the read axis
    /// (`read.allow`) or the write axis (`write.allow`) (spec 22 §5): an
    /// untrusted layer may not permanently reopen paths an operator expects
    /// closable. Final DENIES are allowed from any scope (the fail-closed
    /// direction) and never reach this variant.
    FinalAllowFromNonOperator {
        /// Where the final allow was declared.
        origin: RuleOrigin,
        /// The offending raw pattern string.
        pattern: String,
        /// Which axis (`read` or `write`) carried the final allow.
        axis: PolicyAxis,
    },
    /// `case_sensitivity` other than `"sensitive"` (spec 22 §6): v1 compiles
    /// case-sensitive programs only, with the flag recorded explicitly.
    UnsupportedCaseSensitivity {
        /// Where the value was declared.
        origin: RuleOrigin,
        /// The offending value.
        value: String,
    },
}

/// Payload of [`CompileError::DuplicateTerminalConflict`] (spec 22 §4): the
/// duplicated raw pattern and BOTH declaring origins.
#[derive(Debug)]
pub struct DuplicateConflict {
    /// The duplicated raw pattern string.
    pub pattern: String,
    /// Where the deny entry was declared.
    pub deny_origin: RuleOrigin,
    /// Where the allow entry was declared.
    pub allow_origin: RuleOrigin,
}

/// The policy axis a compile-time diagnostic applies to: `read` (visibility)
/// or `write` (write admission) — the `[policy.mounts.read]` /
/// `[policy.mounts.write]` sub-tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAxis {
    /// The read (visibility) axis.
    Read,
    /// The write (write-admission) axis.
    Write,
}

impl fmt::Display for PolicyAxis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PolicyAxis::Read => f.write_str("read"),
            PolicyAxis::Write => f.write_str("write"),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompileError::Pattern { source } => {
                write!(f, "invalid mount policy pattern: {source}")
            }
            CompileError::DuplicateTerminalConflict(conflict) => write!(
                f,
                "conflicting mount policy rules for pattern '{}': deny declared at \
                 {} and allow declared at {} are exact duplicates in \
                 the same scope with a final rule; a same-scope final deny+allow pair \
                 for the same pattern is a contradiction the author must resolve (spec 22 \u{a7}4)",
                conflict.pattern, conflict.deny_origin, conflict.allow_origin
            ),
            CompileError::FinalAllowFromNonOperator {
                origin,
                pattern,
                axis,
            } => write!(
                f,
                "final {axis}.allow '{pattern}' declared at {origin} is not allowed: \
                 non-operator scopes may not declare final allows (spec 22 \u{a7}5)"
            ),
            CompileError::UnsupportedCaseSensitivity { origin, value } => write!(
                f,
                "case_sensitivity value '{value}' declared at {origin} is not supported: v1 \
                 accepts exactly \"sensitive\" (spec 22 \u{a7}6)"
            ),
        }
    }
}

impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompileError::Pattern { source } => Some(source),
            _ => None,
        }
    }
}

/// Compile collected scopes into a [`MountPolicyProgram`] (spec 22 §8).
///
/// Scopes are ordered by authority (operator scopes first, spec 22 §2); the
/// sort is stable, so same-kind scopes (e.g. fleet layers) keep their
/// registry stack order. Within each scope, read.deny rules are emitted
/// before read.allow rules (per-scope deny-then-allow, spec 22 §4), and
/// protect-bucket entries land in the same scope/authority order.
pub fn compile(mut scopes: Vec<PolicyScope>) -> Result<MountPolicyProgram, CompileError> {
    // Authority order = compile order (spec 22 §2, §5).
    scopes.sort_by_key(|scope| scope.scope_kind.authority());

    let mut rules = Vec::new();
    let mut protect = Vec::new();
    let mut writes = CompiledRuleSet::default();
    for scope in &scopes {
        validate_program_flags(scope)?;
        let (read, protected, write) = compile_scope(scope)?;
        rules.extend(read);
        protect.extend(protected);
        writes.allow.extend(write.allow);
        writes.deny.extend(write.deny);
    }

    Ok(MountPolicyProgram {
        rules,
        version: 1,
        protect,
        writes,
        // v1's explicit default, recorded so the runtime's behavior is
        // pinned by the program (spec 22 §6).
        case_sensitivity: CaseSensitivity::Sensitive,
    })
}

/// Validate the program-level flags one scope may set (spec 22 §6, §10).
fn validate_program_flags(scope: &PolicyScope) -> Result<(), CompileError> {
    if let Some(value) = &scope.fragment.case_sensitivity
        && value != "sensitive"
    {
        return Err(CompileError::UnsupportedCaseSensitivity {
            origin: scope.origin(),
            value: value.clone(),
        });
    }
    Ok(())
}

/// Compile one scope's fragment into rules (read.deny-then-read.allow),
/// protect-bucket entries, and write rules — applying trust validation
/// (spec 22 §5) and the exact-duplicate conflict check (spec 22 §4).
///
/// Protect-bucket routing: an OPERATOR scope's final read.deny compiles to
/// the protect wire bucket (wire effect Mask, terminal, origin attached);
/// the evaluator short-circuits protected paths to Masked before any rule
/// runs. A NON-operator scope's final read.deny is legal (denying
/// visibility is the fail-closed direction) and compiles to a terminal Mask
/// rule in the rules array. All other read entries compile to ordinary
/// Mask (deny) / Unmask (allow) rules, per-scope deny-before-allow.
fn compile_scope(
    scope: &PolicyScope,
) -> Result<(Vec<PathPolicyRule>, Vec<PathPolicyRule>, CompiledRuleSet), CompileError> {
    let origin = scope.origin();
    let operator = scope.scope_kind.is_operator();
    let mut rules = Vec::new();
    let mut protect = Vec::new();
    if let Some(read) = &scope.fragment.read {
        for entry in &read.deny {
            let rule = compile_rule(RuleEffect::Mask, entry, &origin)?;
            if entry.terminal && operator {
                protect.push(rule);
            } else {
                rules.push(rule);
            }
        }
        for entry in &read.allow {
            rules.push(compile_rule(RuleEffect::Unmask, entry, &origin)?);
        }
    }
    let mut writes = CompiledRuleSet::default();
    if let Some(write) = &scope.fragment.write {
        writes.allow = write
            .allow
            .iter()
            .map(|entry| compile_rule(RuleEffect::Unmask, entry, &origin))
            .collect::<Result<_, _>>()?;
        writes.deny = write
            .deny
            .iter()
            .map(|entry| compile_rule(RuleEffect::Mask, entry, &origin))
            .collect::<Result<_, _>>()?;
    }

    // Trust validation (spec 22 §5): a FINAL ALLOW on either axis from a
    // non-operator scope would let an untrusted layer permanently reopen
    // paths an operator expects closable. Final denies (read.deny /
    // write.deny) are allowed from ANY scope — denying is the fail-closed
    // direction.
    for (axis, rule) in rules
        .iter()
        .filter(|rule| rule.effect == RuleEffect::Unmask)
        .map(|rule| (PolicyAxis::Read, rule))
        .chain(writes.allow.iter().map(|rule| (PolicyAxis::Write, rule)))
    {
        if rule.is_terminal() && !operator {
            return Err(CompileError::FinalAllowFromNonOperator {
                origin: rule.origin.clone(),
                pattern: rule.pattern.raw().to_string(),
                axis,
            });
        }
    }

    // Exact-duplicate conflict (spec 22 §4), BOTH axes. Read axis: the deny
    // set is EVERY read.deny rule of the scope — INCLUDING operator-final
    // ones routed to the protect bucket (a same-scope final read.deny +
    // read.allow of the same pattern is a contradiction regardless of
    // routing). Write axis mirrors the check on writes.deny × writes.allow.
    // Relaxable-only duplicates resolve harmlessly by deny-then-allow
    // ordering.
    let read_denies = rules
        .iter()
        .chain(&protect)
        .filter(|rule| rule.effect == RuleEffect::Mask);
    let read_allows: Vec<&PathPolicyRule> = rules
        .iter()
        .filter(|rule| rule.effect == RuleEffect::Unmask)
        .collect();
    check_duplicate_conflicts(read_denies, &read_allows)?;
    let write_allows: Vec<&PathPolicyRule> = writes.allow.iter().collect();
    check_duplicate_conflicts(writes.deny.iter(), &write_allows)?;

    Ok((rules, protect, writes))
}

/// The exact-duplicate conflict check for one axis (spec 22 §4): the same
/// raw pattern appearing as both a deny and an allow in ONE scope, where at
/// least one of the pair is terminal, is a contradiction the author must
/// resolve.
fn check_duplicate_conflicts<'a>(
    denies: impl Iterator<Item = &'a PathPolicyRule>,
    allows: &[&PathPolicyRule],
) -> Result<(), CompileError> {
    for deny in denies {
        for allow in allows {
            if deny.pattern.raw() == allow.pattern.raw()
                && (deny.is_terminal() || allow.is_terminal())
            {
                return Err(CompileError::DuplicateTerminalConflict(Box::new(
                    DuplicateConflict {
                        pattern: deny.pattern.raw().to_string(),
                        deny_origin: deny.origin.clone(),
                        allow_origin: allow.origin.clone(),
                    },
                )));
            }
        }
    }
    Ok(())
}

/// Compile one read/write-axis entry into a rule; pattern rejections name
/// the declaring origin (spec 22 §6). The config surface's `final` flag
/// inverts onto the wire rule's `overridable` flag (wire format unchanged).
fn compile_rule(
    effect: RuleEffect,
    entry: &PolicyValue<String>,
    origin: &RuleOrigin,
) -> Result<PathPolicyRule, CompileError> {
    let pattern = Pattern::compile(&entry.value, origin)
        .map_err(|source| CompileError::Pattern { source })?;
    Ok(PathPolicyRule {
        effect,
        pattern,
        overridable: !entry.terminal,
        origin: origin.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mount_policy::lexical::LexicalPath;
    use crate::mount_policy::program::Decision;
    use crate::mount_policy::scope::{AxisFragment, MountsFragment, ScopeKind};
    use std::path::PathBuf;

    fn scope_with(
        kind: ScopeKind,
        layer: &str,
        read_deny: Vec<PolicyValue<String>>,
        read_allow: Vec<PolicyValue<String>>,
        case_sensitivity: Option<&str>,
    ) -> PolicyScope {
        PolicyScope::new(
            kind,
            layer,
            PathBuf::from(format!("{layer}.toml")),
            MountsFragment {
                read: Some(AxisFragment {
                    deny: read_deny,
                    allow: read_allow,
                }),
                write: None,
                case_sensitivity: case_sensitivity.map(str::to_string),
            },
        )
    }

    fn scope(
        kind: ScopeKind,
        layer: &str,
        read_deny: Vec<PolicyValue<String>>,
        read_allow: Vec<PolicyValue<String>>,
    ) -> PolicyScope {
        scope_with(kind, layer, read_deny, read_allow, None)
    }

    fn entry(pattern: &str) -> PolicyValue<String> {
        PolicyValue::relaxable(pattern.to_string())
    }

    fn terminal_entry(pattern: &str) -> PolicyValue<String> {
        PolicyValue::terminal(pattern.to_string())
    }

    fn axis_scope(
        kind: ScopeKind,
        layer: &str,
        read: AxisFragment,
        write: AxisFragment,
    ) -> PolicyScope {
        PolicyScope::new(
            kind,
            layer,
            format!("{layer}.toml"),
            MountsFragment {
                read: Some(read),
                write: Some(write),
                ..Default::default()
            },
        )
    }

    fn decide(program: &MountPolicyProgram, path: &str) -> Decision {
        program.decide(&LexicalPath::new(path).unwrap()).decision
    }

    #[test]
    fn scopes_compile_operator_first_regardless_of_input_order() {
        // Given out of order (workload first), the compiler orders by
        // authority: the operator scope's rules come first in the program.
        let program = compile(vec![
            scope(
                ScopeKind::Workload,
                "workload",
                vec![entry("workload-only")],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRegistry,
                "registry",
                vec![entry("registry-only")],
                vec![],
            ),
        ])
        .unwrap();
        assert_eq!(program.rules[0].origin.layer, "registry");
        assert_eq!(program.rules[1].origin.layer, "workload");
    }

    #[test]
    fn later_scopes_respect_earlier_terminal_freezes() {
        // A final read.deny from the (non-operator) reference-config scope
        // compiles to a terminal Mask rule that freezes `.env`; the repo
        // scope's matching allow is recorded but frozen out.
        let program = compile(vec![
            scope(
                ScopeKind::ReferenceConfig,
                "reference",
                vec![terminal_entry(".env")],
                vec![],
            ),
            scope(ScopeKind::FleetLayer, "repo", vec![], vec![entry(".env")]),
        ])
        .unwrap();
        let explained = program.decide(&LexicalPath::new(".env").unwrap());
        assert_eq!(explained.decision, Decision::Masked);
        assert_eq!(
            explained.frozen_by.map(|origin| origin.layer),
            Some("reference".to_string())
        );
        assert_eq!(explained.matches.len(), 2);
        assert!(!explained.matches[0].frozen_out);
        assert!(explained.matches[1].frozen_out);
        assert!(!explained.matches[1].terminal);
    }

    #[test]
    fn final_read_allow_from_an_operator_scope_sticks() {
        // Operator final read.allow freezes `.env` visible; a later scope's
        // relaxable deny cannot re-mask it.
        let program = compile(vec![
            scope(
                ScopeKind::ConfigRegistry,
                "registry",
                vec![],
                vec![terminal_entry(".env")],
            ),
            scope(ScopeKind::FleetLayer, "repo", vec![entry(".env")], vec![]),
        ])
        .unwrap();
        let explained = program.decide(&LexicalPath::new(".env").unwrap());
        assert_eq!(explained.decision, Decision::Visible);
        assert_eq!(
            explained.frozen_by.map(|origin| origin.layer),
            Some("registry".to_string())
        );
        assert!(explained.matches[1].frozen_out);
    }

    #[test]
    fn final_read_allow_from_a_non_operator_scope_is_a_compile_error() {
        for kind in [
            ScopeKind::ReferenceConfig,
            ScopeKind::FleetLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let err = compile(vec![scope(
                kind,
                "repo",
                vec![],
                vec![terminal_entry(".env")],
            )])
            .unwrap_err();
            assert!(
                matches!(
                    err,
                    CompileError::FinalAllowFromNonOperator {
                        axis: PolicyAxis::Read,
                        ..
                    }
                ),
                "expected FinalAllowFromNonOperator on the read axis: {err}"
            );
            let text = err.to_string();
            assert!(
                text.contains("repo") && text.contains(".env"),
                "error must name the origin and pattern: {text}"
            );
            assert!(
                text.contains("final read.allow"),
                "error must name the axis and surface key: {text}"
            );
        }
    }

    #[test]
    fn final_read_deny_is_allowed_from_any_scope() {
        // Final DENIES are the fail-closed direction: allowed from every
        // scope. Operator scopes route to the protect bucket; non-operator
        // scopes compile to a terminal Mask rule (routing is asserted by
        // `final_read_deny_protect_routing_is_operator_only`).
        for kind in [
            ScopeKind::ConfigRegistry,
            ScopeKind::UserGlobalOverrides,
            ScopeKind::ReferenceConfig,
            ScopeKind::FleetLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let program = compile(vec![scope(
                kind,
                "any",
                vec![terminal_entry(".env")],
                vec![],
            )]);
            assert!(
                program.is_ok(),
                "final read.deny must compile from {kind:?}"
            );
        }
    }

    #[test]
    fn same_scope_deny_then_allow_carves_out_an_exception() {
        // Within one scope, allow entries evaluate after deny entries
        // (spec 22 §4): the scope carves an exception to its own deny.
        let program = compile(vec![scope(
            ScopeKind::FleetLayer,
            "repo",
            vec![entry("docs/secrets/**")],
            vec![entry("docs/secrets/README.md")],
        )])
        .unwrap();
        assert_eq!(
            decide(&program, "docs/secrets/README.md"),
            Decision::Visible
        );
        assert_eq!(decide(&program, "docs/secrets/key.pem"), Decision::Masked);
    }

    #[test]
    fn exact_duplicate_same_scope_final_deny_allow_is_a_compile_error() {
        let err = compile(vec![scope(
            ScopeKind::ConfigRegistry,
            "registry",
            vec![terminal_entry(".env")],
            vec![entry(".env")],
        )])
        .unwrap_err();
        let text = err.to_string();
        assert!(matches!(err, CompileError::DuplicateTerminalConflict(_)));
        // Both origins are named (same scope, so the same origin twice).
        assert!(
            text.contains("registry"),
            "error must name the origin: {text}"
        );
        assert!(text.contains(".env"), "error must name the pattern: {text}");
    }

    #[test]
    fn relaxable_only_same_scope_read_duplicates_resolve_by_ordering() {
        // Design resolution (spec 22 §4 / ADR 0028): only FINAL duplicates
        // are contradictions; a relaxable deny+allow pair for the same
        // pattern resolves by deny-then-allow ordering.
        let program = compile(vec![scope(
            ScopeKind::FleetLayer,
            "repo",
            vec![entry(".env")],
            vec![entry(".env")],
        )])
        .unwrap();
        assert_eq!(decide(&program, ".env"), Decision::Visible);
    }

    #[test]
    fn duplicates_in_different_scopes_are_not_conflicts() {
        // The same pattern as deny in one scope and allow in another is
        // normal precedence, not a compile error.
        let program = compile(vec![
            scope(
                ScopeKind::ConfigRegistry,
                "registry",
                vec![terminal_entry(".env")],
                vec![],
            ),
            scope(ScopeKind::FleetLayer, "repo", vec![], vec![entry(".env")]),
        ]);
        assert!(program.is_ok());
    }

    #[test]
    fn case_sensitivity_defaults_to_sensitive_and_rejects_other_values() {
        let program = compile(vec![]).unwrap();
        assert_eq!(program.case_sensitivity, CaseSensitivity::Sensitive);
        let program = compile(vec![scope_with(
            ScopeKind::ConfigRegistry,
            "registry",
            vec![],
            vec![],
            Some("sensitive"),
        )])
        .unwrap();
        assert_eq!(program.case_sensitivity, CaseSensitivity::Sensitive);
        let err = compile(vec![scope_with(
            ScopeKind::ConfigRegistry,
            "registry",
            vec![],
            vec![],
            Some("insensitive"),
        )])
        .unwrap_err();
        assert!(
            err.to_string().contains("insensitive"),
            "error must name the offending value: {err}"
        );
    }

    #[test]
    fn pattern_errors_are_compile_errors_naming_the_origin() {
        for raw in ["/absolute", "a\0b", "../escape", ""] {
            let err = compile(vec![scope(
                ScopeKind::FleetLayer,
                "repo",
                vec![entry(raw)],
                vec![],
            )])
            .unwrap_err();
            let text = err.to_string();
            assert!(
                text.contains("repo") && text.contains("fleet-layer"),
                "error must name the origin for pattern {raw:?}: {text}"
            );
        }
    }

    #[test]
    fn writes_default_allow_and_allow_carves_exception_within_scope() {
        // Union semantics (fork rev 205a7b95): within one scope deny sorts
        // before allow, so the relaxable allow "**" matches after the deny
        // "denied" and wins — the allow carves an exception out of the deny.
        let program = compile(vec![axis_scope(
            ScopeKind::FleetLayer,
            "repo",
            AxisFragment::default(),
            AxisFragment {
                allow: vec![entry("**")],
                deny: vec![entry("denied")],
            },
        )])
        .unwrap();
        assert_eq!(
            program
                .decide_write(&LexicalPath::new("visible").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
        assert_eq!(
            program
                .decide_write(&LexicalPath::new("denied").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
        let empty = compile(vec![]).unwrap();
        assert_eq!(
            empty
                .decide_write(&LexicalPath::new("new").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
    }

    /// An operator-scope final read.deny routes to the protect wire bucket;
    /// protected paths are Masked AND write-denied even under a broad
    /// write.allow (the protect bucket short-circuits both evaluators).
    #[test]
    fn protect_bucket_forces_masked_and_write_deny() {
        let program = compile(vec![axis_scope(
            ScopeKind::ConfigRegistry,
            "operator",
            AxisFragment {
                deny: vec![terminal_entry("protected")],
                allow: vec![],
            },
            AxisFragment {
                allow: vec![entry("**")],
                deny: vec![],
            },
        )])
        .unwrap();
        assert_eq!(program.protect.len(), 1);
        assert!(program.protect[0].is_terminal());
        assert_eq!(program.protect[0].origin.layer, "operator");
        assert!(
            program.rules.is_empty(),
            "a protect-routed final read.deny must not also land in the rules array"
        );
        assert!(program.is_protected(&LexicalPath::new("protected").unwrap()));
        assert_eq!(
            program
                .decide_write(&LexicalPath::new("protected").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Deny
        );
        assert_eq!(
            program
                .decide(&LexicalPath::new("protected").unwrap())
                .decision,
            Decision::Masked
        );
    }

    /// Protect-bucket routing is OPERATOR-ONLY: an operator scope's final
    /// read.deny compiles to the protect wire bucket; a non-operator
    /// scope's final read.deny is legal (fail-closed direction) and compiles
    /// to a terminal Mask rule in the rules array (spec 22 §5).
    #[test]
    fn final_read_deny_protect_routing_is_operator_only() {
        let operator_program = compile(vec![axis_scope(
            ScopeKind::ConfigRegistry,
            "operator",
            AxisFragment {
                deny: vec![terminal_entry("secret")],
                allow: vec![],
            },
            AxisFragment::default(),
        )])
        .unwrap();
        assert_eq!(operator_program.protect.len(), 1);
        assert!(operator_program.protect[0].is_terminal());
        assert!(operator_program.rules.is_empty());

        for kind in [
            ScopeKind::ReferenceConfig,
            ScopeKind::FleetLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let program = compile(vec![axis_scope(
                kind,
                "layer",
                AxisFragment {
                    deny: vec![terminal_entry("secret")],
                    allow: vec![],
                },
                AxisFragment::default(),
            )])
            .unwrap();
            assert!(
                program.protect.is_empty(),
                "non-operator final read.deny must not route to protect ({kind:?})"
            );
            assert_eq!(program.rules.len(), 1);
            assert_eq!(program.rules[0].effect, RuleEffect::Mask);
            assert!(program.rules[0].is_terminal());
            assert_eq!(decide(&program, "secret"), Decision::Masked);
        }
    }

    #[test]
    fn final_write_allow_from_a_non_operator_scope_is_a_compile_error() {
        for kind in [
            ScopeKind::ReferenceConfig,
            ScopeKind::FleetLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let err = compile(vec![axis_scope(
                kind,
                "repo",
                AxisFragment::default(),
                AxisFragment {
                    allow: vec![terminal_entry(".env")],
                    deny: vec![],
                },
            )])
            .unwrap_err();
            assert!(
                matches!(
                    err,
                    CompileError::FinalAllowFromNonOperator {
                        axis: PolicyAxis::Write,
                        ..
                    }
                ),
                "expected FinalAllowFromNonOperator on the write axis: {err}"
            );
            let text = err.to_string();
            assert!(
                text.contains("repo") && text.contains(".env"),
                "error must name the origin and pattern: {text}"
            );
            assert!(
                text.contains("final write.allow"),
                "error must name the axis and surface key: {text}"
            );
        }
    }

    #[test]
    fn final_write_allow_from_an_operator_scope_compiles() {
        for kind in [ScopeKind::ConfigRegistry, ScopeKind::UserGlobalOverrides] {
            let program = compile(vec![axis_scope(
                kind,
                "operator",
                AxisFragment::default(),
                AxisFragment {
                    allow: vec![terminal_entry("gen/**")],
                    deny: vec![],
                },
            )])
            .unwrap();
            assert_eq!(program.writes.allow.len(), 1);
            assert!(program.writes.allow[0].is_terminal());
        }
    }

    #[test]
    fn final_write_deny_is_allowed_from_any_scope() {
        for kind in [
            ScopeKind::ConfigRegistry,
            ScopeKind::UserGlobalOverrides,
            ScopeKind::ReferenceConfig,
            ScopeKind::FleetLayer,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let program = compile(vec![axis_scope(
                kind,
                "any",
                AxisFragment::default(),
                AxisFragment {
                    allow: vec![],
                    deny: vec![terminal_entry(".env")],
                },
            )]);
            assert!(
                program.is_ok(),
                "final write.deny must compile from {kind:?}"
            );
        }
    }

    /// The duplicate-conflict check covers read.deny entries routed to the
    /// protect bucket: a same-scope final read.deny + read.allow of the same
    /// raw pattern is a contradiction regardless of routing (spec 22 §4).
    #[test]
    fn same_scope_final_read_deny_allow_duplicate_is_a_conflict_even_when_protect_routed() {
        let err = compile(vec![axis_scope(
            ScopeKind::ConfigRegistry,
            "operator",
            AxisFragment {
                deny: vec![terminal_entry(".env")],
                allow: vec![entry(".env")],
            },
            AxisFragment::default(),
        )])
        .unwrap_err();
        assert!(matches!(err, CompileError::DuplicateTerminalConflict(_)));
        let text = err.to_string();
        assert!(
            text.contains("operator"),
            "error must name the origin: {text}"
        );
        assert!(text.contains(".env"), "error must name the pattern: {text}");
    }

    /// The write axis mirrors the read axis's duplicate-conflict check: a
    /// same-scope write.deny + write.allow of the same raw pattern with at
    /// least one final entry is a contradiction (spec 22 §4). (The final
    /// entry is the DENY here: a final write.allow from this non-operator
    /// scope would trip the trust gate first.)
    #[test]
    fn same_scope_write_deny_allow_duplicate_with_a_final_is_a_conflict() {
        let err = compile(vec![axis_scope(
            ScopeKind::FleetLayer,
            "repo",
            AxisFragment::default(),
            AxisFragment {
                allow: vec![entry("gen/output.bin")],
                deny: vec![terminal_entry("gen/output.bin")],
            },
        )])
        .unwrap_err();
        assert!(matches!(err, CompileError::DuplicateTerminalConflict(_)));
        let text = err.to_string();
        assert!(text.contains("repo"), "error must name the origin: {text}");
        assert!(
            text.contains("gen/output.bin"),
            "error must name the pattern: {text}"
        );
    }

    #[test]
    fn relaxable_only_same_scope_write_duplicates_resolve_by_ordering() {
        // Write-axis mirror of the read-axis rule: relaxable-only duplicates
        // are NOT conflicts; the evaluator's per-scope deny-before-allow
        // ordering resolves them — the allow, evaluated last, wins (union
        // semantics, fork rev 205a7b95).
        let program = compile(vec![axis_scope(
            ScopeKind::FleetLayer,
            "repo",
            AxisFragment::default(),
            AxisFragment {
                allow: vec![entry("gen/output.bin")],
                deny: vec![entry("gen/output.bin")],
            },
        )])
        .unwrap();
        assert_eq!(
            program
                .decide_write(&LexicalPath::new("gen/output.bin").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
    }
}
