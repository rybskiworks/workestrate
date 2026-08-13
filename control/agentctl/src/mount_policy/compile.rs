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
//! - exact-duplicate same-scope terminal mask+unmask for the same raw
//!   pattern is an error naming BOTH origins (spec 22 §4);
//! - a non-operator scope declaring a terminal (non-overridable) unmask is
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
/// registry stack order. Within each scope, mask rules are emitted before
/// unmask rules (per-scope mask-then-unmask, spec 22 §4).
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

/// Compile one scope's fragment into rules (mask-then-unmask), applying
/// trust validation (spec 22 §5) and the exact-duplicate conflict check
/// (spec 22 §4).
fn compile_scope(
    scope: &PolicyScope,
) -> Result<(Vec<PathPolicyRule>, Vec<PathPolicyRule>, CompiledRuleSet), CompileError> {
    let origin = scope.origin();
    let mut rules = Vec::new();
    for entry in &scope.fragment.mask {
        rules.push(compile_rule(RuleEffect::Mask, entry, &origin)?);
    }
    for entry in &scope.fragment.unmask {
        rules.push(compile_rule(RuleEffect::Unmask, entry, &origin)?);
    }
    let protect: Vec<_> = scope
        .fragment
        .protect
        .iter()
        .map(|entry| compile_rule(RuleEffect::Mask, entry, &origin))
        .collect::<Result<_, _>>()?;
    for rule in &protect {
        if rule.is_terminal() && !scope.scope_kind.is_operator() {
            return Err(CompileError::TerminalProtectFromNonOperator {
                origin: rule.origin.clone(),
                pattern: rule.pattern.raw().to_string(),
            });
        }
    }
    let mut writes = CompiledRuleSet::default();
    if let Some(fragment) = &scope.fragment.writes {
        writes.allow = fragment
            .allow
            .iter()
            .map(|entry| compile_rule(RuleEffect::Unmask, entry, &origin))
            .collect::<Result<_, _>>()?;
        writes.deny = fragment
            .deny
            .iter()
            .map(|entry| compile_rule(RuleEffect::Mask, entry, &origin))
            .collect::<Result<_, _>>()?;
    }

    // Trust validation (spec 22 §5): a terminal unmask from a non-operator
    // scope would let an untrusted repo permanently reopen paths an operator
    // expects maskable. Terminal MASKS are allowed from any scope — masking
    // is the fail-closed direction.
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
    // as both a mask and an unmask in ONE scope, where at least one of the
    // pair is terminal, is a contradiction the author must resolve.
    // Overridable-only duplicates resolve harmlessly by mask-then-unmask
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

/// Compile one mask/unmask entry into a rule; pattern rejections name the
/// declaring origin (spec 22 §6).
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
        overridable: entry.overridable,
        origin: origin.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mount_policy::lexical::LexicalPath;
    use crate::mount_policy::program::Decision;
    use crate::mount_policy::scope::{MountsFragment, ScopeKind, WritesFragment};
    use std::path::PathBuf;

    fn scope_with(
        kind: ScopeKind,
        layer: &str,
        mask: Vec<PolicyValue<String>>,
        unmask: Vec<PolicyValue<String>>,
        case_sensitivity: Option<&str>,
    ) -> PolicyScope {
        PolicyScope::new(
            kind,
            layer,
            PathBuf::from(format!("{layer}.toml")),
            MountsFragment {
                mask,
                unmask,
                protect: vec![],
                writes: None,
                case_sensitivity: case_sensitivity.map(str::to_string),
            },
        )
    }

    fn scope(
        kind: ScopeKind,
        layer: &str,
        mask: Vec<PolicyValue<String>>,
        unmask: Vec<PolicyValue<String>>,
    ) -> PolicyScope {
        scope_with(kind, layer, mask, unmask, None)
    }

    fn mask_entry(pattern: &str) -> PolicyValue<String> {
        PolicyValue::overridable(pattern.to_string())
    }

    fn mask_terminal(pattern: &str) -> PolicyValue<String> {
        PolicyValue::terminal(pattern.to_string())
    }

    fn write_scope(
        kind: ScopeKind,
        layer: &str,
        protect: Vec<PolicyValue<String>>,
        writes: WritesFragment,
    ) -> PolicyScope {
        PolicyScope::new(
            kind,
            layer,
            format!("{layer}.toml"),
            MountsFragment {
                protect,
                writes: Some(writes),
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
                vec![mask_entry("workload-only")],
                vec![],
            ),
            scope(
                ScopeKind::HomeRegistry,
                "registry",
                vec![mask_entry("registry-only")],
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
                vec![mask_terminal(".env")],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![],
                vec![mask_entry(".env")],
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
                vec![mask_terminal(".env")],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![mask_entry(".env")],
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
                vec![mask_terminal(".env")],
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
                vec![mask_terminal(".env")],
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
            vec![mask_entry("docs/secrets/**")],
            vec![mask_entry("docs/secrets/README.md")],
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
            vec![mask_terminal(".env")],
            vec![mask_entry(".env")],
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
            vec![mask_entry(".env")],
            vec![mask_entry(".env")],
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
                vec![mask_terminal(".env")],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![],
                vec![mask_entry(".env")],
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
                vec![mask_entry(raw)],
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
    fn writes_default_allow_deny_wins_and_protect_wins() {
        let program = compile(vec![write_scope(
            ScopeKind::ConfigRepoLayer,
            "repo",
            vec![mask_entry("protected")],
            WritesFragment {
                allow: vec![mask_entry("**")],
                deny: vec![mask_entry("denied")],
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
        let empty = compile(vec![]).unwrap();
        assert_eq!(
            empty
                .decide_write(&LexicalPath::new("new").unwrap())
                .decision,
            crate::mount_policy::WriteDecision::Allow
        );
    }

    #[test]
    fn terminal_protect_is_operator_only() {
        let err = compile(vec![write_scope(
            ScopeKind::Workload,
            "workload",
            vec![mask_terminal("secret")],
            WritesFragment::default(),
        )])
        .unwrap_err();
        assert!(err.to_string().contains("workload") && err.to_string().contains("secret"));
        assert!(compile(vec![write_scope(
            ScopeKind::HomeRegistry,
            "operator",
            vec![mask_terminal("secret")],
            WritesFragment::default()
        )])
        .is_ok());
    }
}
