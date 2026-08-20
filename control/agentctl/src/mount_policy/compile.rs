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
//! - exact-duplicate same-scope terminal read.deny+read.allow for the same
//!   raw pattern is an error naming BOTH origins (spec 22 §4);
//! - a non-operator scope declaring a terminal (final) read.allow is
//!   rejected (spec 22 §5);
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
    /// Exact-duplicate same-scope terminal mask+unmask for the same raw
    /// pattern (spec 22 §4): a contradiction the author must resolve, not a
    /// precedence question. Boxed to keep the error type small.
    DuplicateTerminalConflict(Box<DuplicateConflict>),
    /// A non-operator scope declared a terminal unmask (spec 22 §5): a repo
    /// or project layer may not permanently reopen paths an operator expects
    /// maskable.
    TerminalUnmaskFromNonOperator {
        /// Where the terminal unmask was declared.
        origin: RuleOrigin,
        /// The offending raw pattern string.
        pattern: String,
    },
    /// A terminal protect from an untrusted scope (spec 22 §5).
    // S2 note: the protect bucket is not populated in this slice (see the
    // TODO(S3) in `compile_scope`), so this variant is currently
    // unconstructed; S3 reinstates protect-bucket routing and its trust gate.
    TerminalProtectFromNonOperator { origin: RuleOrigin, pattern: String },
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
    /// Where the mask entry was declared.
    pub mask_origin: RuleOrigin,
    /// Where the unmask entry was declared.
    pub unmask_origin: RuleOrigin,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompileError::Pattern { source } => {
                write!(f, "invalid mount policy pattern: {source}")
            }
            CompileError::DuplicateTerminalConflict(conflict) => write!(
                f,
                "conflicting mount policy rules for pattern '{}': mask declared at \
                 {} and unmask declared at {} are exact duplicates in \
                 the same scope with a terminal rule; a same-scope terminal mask+unmask pair \
                 for the same pattern is a contradiction the author must resolve (spec 22 \u{a7}4)",
                conflict.pattern, conflict.mask_origin, conflict.unmask_origin
            ),
            CompileError::TerminalUnmaskFromNonOperator { origin, pattern } => write!(
                f,
                "terminal unmask '{pattern}' declared at {origin} is not allowed: non-operator \
                 scopes may not declare non-overridable unmasks (spec 22 \u{a7}5)"
            ),
            CompileError::TerminalProtectFromNonOperator { origin, pattern } => write!(
                f,
                "terminal protect '{pattern}' declared at {origin} is not allowed: non-operator \
                 scopes may not declare non-overridable protection (spec 22 \u{a7}5)"
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
/// sort is stable, so same-kind scopes (e.g. config-repo layers) keep their
/// registry stack order. Within each scope, read.deny rules are emitted
/// before read.allow rules (per-scope deny-then-allow — the mask-then-unmask
/// emission order of spec 22 §4).
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
    if let Some(value) = &scope.fragment.case_sensitivity {
        if value != "sensitive" {
            return Err(CompileError::UnsupportedCaseSensitivity {
                origin: scope.origin(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}

/// Compile one scope's fragment into rules (read.deny-then-read.allow),
/// applying trust validation (spec 22 §5) and the exact-duplicate conflict
/// check (spec 22 §4).
fn compile_scope(
    scope: &PolicyScope,
) -> Result<(Vec<PathPolicyRule>, Vec<PathPolicyRule>, CompiledRuleSet), CompileError> {
    let origin = scope.origin();
    let mut rules = Vec::new();
    if let Some(read) = &scope.fragment.read {
        for entry in &read.deny {
            rules.push(compile_rule(RuleEffect::Mask, entry, &origin)?);
        }
        for entry in &read.allow {
            rules.push(compile_rule(RuleEffect::Unmask, entry, &origin)?);
        }
    }
    // TODO(S3): route operator-scope final read.deny entries to the protect
    // wire bucket (with the TerminalProtectFromNonOperator trust gate for
    // non-operator scopes). S2 treats EVERY final read.deny the way the old
    // compiler treated a terminal mask — a Mask rule in the rules array —
    // and leaves the protect bucket empty.
    let protect = Vec::new();
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

    // Trust validation (spec 22 §5): a terminal read.allow from a
    // non-operator scope would let an untrusted repo permanently reopen
    // paths an operator expects maskable. Terminal read.deny entries are
    // allowed from any scope — denying visibility is the fail-closed
    // direction.
    for rule in &rules {
        if rule.effect == RuleEffect::Unmask
            && rule.is_terminal()
            && !scope.scope_kind.is_operator()
        {
            return Err(CompileError::TerminalUnmaskFromNonOperator {
                origin: rule.origin.clone(),
                pattern: rule.pattern.raw().to_string(),
            });
        }
    }

    // Exact-duplicate conflict (spec 22 §4): the same raw pattern appearing
    // as both a read.deny and a read.allow in ONE scope, where at least one
    // of the pair is terminal, is a contradiction the author must resolve.
    // Relaxable-only duplicates resolve harmlessly by deny-then-allow
    // ordering.
    let masks = rules.iter().filter(|rule| rule.effect == RuleEffect::Mask);
    let unmasks: Vec<&PathPolicyRule> = rules
        .iter()
        .filter(|rule| rule.effect == RuleEffect::Unmask)
        .collect();
    for mask in masks {
        for unmask in &unmasks {
            if mask.pattern.raw() == unmask.pattern.raw()
                && (mask.is_terminal() || unmask.is_terminal())
            {
                return Err(CompileError::DuplicateTerminalConflict(Box::new(
                    DuplicateConflict {
                        pattern: mask.pattern.raw().to_string(),
                        mask_origin: mask.origin.clone(),
                        unmask_origin: unmask.origin.clone(),
                    },
                )));
            }
        }
    }

    Ok((rules, protect, writes))
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
                ScopeKind::HomeRegistry,
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
        // Operator terminal mask freezes `.env`; the repo scope's matching
        // unmask is recorded but frozen out.
        let program = compile(vec![
            scope(
                ScopeKind::HomeRegistry,
                "registry",
                vec![terminal_entry(".env")],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![],
                vec![entry(".env")],
            ),
        ])
        .unwrap();
        let explained = program.decide(&LexicalPath::new(".env").unwrap());
        assert_eq!(explained.decision, Decision::Masked);
        assert_eq!(
            explained.frozen_by.map(|origin| origin.layer),
            Some("registry".to_string())
        );
        assert_eq!(explained.matches.len(), 2);
        assert!(!explained.matches[0].frozen_out);
        assert!(explained.matches[1].frozen_out);
        assert!(!explained.matches[1].terminal);
    }

    #[test]
    fn terminal_unmask_from_an_operator_scope_sticks() {
        // Operator terminal unmask freezes `.env` visible; a later scope's
        // overridable mask cannot re-mask it.
        let program = compile(vec![
            scope(
                ScopeKind::HomeRegistry,
                "registry",
                vec![],
                vec![terminal_entry(".env")],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![entry(".env")],
                vec![],
            ),
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
    fn terminal_unmask_from_a_non_operator_scope_is_a_compile_error() {
        for kind in [
            ScopeKind::ReferenceConfig,
            ScopeKind::ConfigRepoLayer,
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
            let text = err.to_string();
            assert!(
                text.contains("repo") && text.contains(".env"),
                "error must name the origin and pattern: {text}"
            );
        }
    }

    #[test]
    fn terminal_mask_is_allowed_from_any_scope() {
        for kind in [
            ScopeKind::HomeRegistry,
            ScopeKind::ReferenceConfig,
            ScopeKind::Workload,
            ScopeKind::MountEntry,
        ] {
            let program = compile(vec![scope(
                kind,
                "any",
                vec![terminal_entry(".env")],
                vec![],
            )]);
            assert!(program.is_ok(), "terminal mask must compile from {kind:?}");
        }
    }

    #[test]
    fn same_scope_mask_then_unmask_carves_out_an_exception() {
        // Within one scope, unmask entries evaluate after mask entries
        // (spec 22 §4): the scope carves an exception to its own mask.
        let program = compile(vec![scope(
            ScopeKind::ConfigRepoLayer,
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
    fn exact_duplicate_same_scope_terminal_mask_unmask_is_a_compile_error() {
        let err = compile(vec![scope(
            ScopeKind::HomeRegistry,
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
    fn overridable_only_same_scope_duplicates_resolve_by_ordering() {
        // Design resolution (spec 22 §4 / ADR 0028): only TERMINAL
        // duplicates are contradictions; an overridable mask+unmask pair for
        // the same pattern resolves by mask-then-unmask ordering.
        let program = compile(vec![scope(
            ScopeKind::ConfigRepoLayer,
            "repo",
            vec![entry(".env")],
            vec![entry(".env")],
        )])
        .unwrap();
        assert_eq!(decide(&program, ".env"), Decision::Visible);
    }

    #[test]
    fn duplicates_in_different_scopes_are_not_conflicts() {
        // The same pattern as mask in one scope and unmask in another is
        // normal precedence, not a compile error.
        let program = compile(vec![
            scope(
                ScopeKind::HomeRegistry,
                "registry",
                vec![terminal_entry(".env")],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![],
                vec![entry(".env")],
            ),
        ]);
        assert!(program.is_ok());
    }

    #[test]
    fn case_sensitivity_defaults_to_sensitive_and_rejects_other_values() {
        let program = compile(vec![]).unwrap();
        assert_eq!(program.case_sensitivity, CaseSensitivity::Sensitive);
        let program = compile(vec![scope_with(
            ScopeKind::HomeRegistry,
            "registry",
            vec![],
            vec![],
            Some("sensitive"),
        )])
        .unwrap();
        assert_eq!(program.case_sensitivity, CaseSensitivity::Sensitive);
        let err = compile(vec![scope_with(
            ScopeKind::HomeRegistry,
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
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![entry(raw)],
                vec![],
            )])
            .unwrap_err();
            let text = err.to_string();
            assert!(
                text.contains("repo") && text.contains("config-repo-layer"),
                "error must name the origin for pattern {raw:?}: {text}"
            );
        }
    }

    #[test]
    fn writes_default_allow_and_deny_wins() {
        let program = compile(vec![axis_scope(
            ScopeKind::ConfigRepoLayer,
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
            crate::mount_policy::WriteDecision::Deny
        );
        let empty = compile(vec![]).unwrap();
        assert_eq!(
            empty
                .decide_write(&LexicalPath::new("new").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
    }

    /// S3 target behavior: an operator-scope final read.deny routes to the
    /// protect wire bucket; protected paths are Masked AND write-denied even
    /// under a broad write.allow. S2 emits final read.deny as a plain
    /// (terminal) Mask rule and leaves the protect bucket empty, so the
    /// protect assertions below do not hold yet.
    #[test]
    #[ignore = "S3 reinstates protect-bucket routing"]
    fn protect_bucket_forces_masked_and_write_deny() {
        let program = compile(vec![axis_scope(
            ScopeKind::HomeRegistry,
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

    /// S3 target behavior: a final read.deny from a non-operator scope is
    /// routed to the protect bucket and rejected by the protect trust gate.
    /// S2 treats final read.deny as an ordinary terminal mask (allowed from
    /// any scope), so the rejection below does not hold yet.
    #[test]
    #[ignore = "S3 reinstates protect-bucket routing"]
    fn terminal_protect_is_operator_only() {
        let err = compile(vec![axis_scope(
            ScopeKind::Workload,
            "workload",
            AxisFragment {
                deny: vec![terminal_entry("secret")],
                allow: vec![],
            },
            AxisFragment::default(),
        )])
        .unwrap_err();
        assert!(err.to_string().contains("workload") && err.to_string().contains("secret"));
        assert!(compile(vec![axis_scope(
            ScopeKind::HomeRegistry,
            "operator",
            AxisFragment {
                deny: vec![terminal_entry("secret")],
                allow: vec![],
            },
            AxisFragment::default()
        )])
        .is_ok());
    }
}
