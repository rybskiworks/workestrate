//! Dynamic mount masking policy: the pure mount-policy core (spec 22,
//! `docs/validation-and-improvements/06-improvements/22-dynamic-mount-masking-policy.md`;
//! ADR 0028, `docs/migration/50-decisions/0028-policy-scopes-collect-and-compile.md`).
//!
//! Host→guest bind mounts (notably the `${CWD}` project-root bind) expose
//! host-repo secrets to sandboxed agents. This module implements the
//! container-verifiable half of the remedy: a hierarchical `[policy.mounts]`
//! config surface whose fragments are COLLECTED per layer (never merged —
//! `merge.rs` is untouched, ADR 0028) and COMPILED by [`compile`] into a
//! per-mount [`MountPolicyProgram`] that serializes to JSON for the v1
//! runtime transmission channel (spec 22 §12).
//!
//! The pipeline:
//!
//! - [`value`]: [`PolicyValue`] compact/expanded entry forms (spec 22 §3),
//!   parsed with the `deserialize_any` visitor idiom (the `RawBindingVisitor`
//!   precedent in `config/types.rs`).
//! - [`scope`]: [`PolicyScope`] / [`ScopeKind`] — the six declaration scopes
//!   (spec 22 §2). Compile order IS authority order: operator scopes first.
//! - [`pattern`]: [`Pattern`], the glob dialect newtype (spec 22 §6):
//!   root-anchored by default, `**/`-prefixed floating, trailing-`/` dir-only,
//!   `*`/`?` never crossing `/`, absolute/NUL/`..`-escaping/empty patterns
//!   rejected with origin-named errors.
//! - [`lexical`]: [`LexicalPath`], pure component-wise relative paths — no
//!   filesystem access; non-UTF-8 paths fail closed (masked).
//! - [`rule`]: [`PathPolicyRule`] + [`RuleOrigin`] provenance (spec 22 §11).
//! - [`compile`]: the compiler — precedence, freeze semantics, trust
//!   validation (terminal unmask/protect rejected from non-operator scopes),
//!   exact-duplicate conflict detection, write-rule precedence, and explicit
//!   versioned fail-closed transmission.
//! - [`program`]: the compiled program and pure evaluator — the three
//!   decision states Visible / Masked / TraversalOnly (spec 22 §7),
//!   `may_unmask_descendant` literal-prefix analysis, and full explain
//!   traces (spec 22 §13).

pub mod compile;
pub mod lexical;
pub mod pattern;
pub mod program;
pub mod rule;
pub mod scope;
pub mod value;

pub use compile::{compile, CompileError, DuplicateConflict};
pub use lexical::{LexicalPath, LexicalPathError};
pub use pattern::{Pattern, PatternError, PatternErrorKind};
pub use program::{
    CaseSensitivity, CompiledRuleSet, Decision, Explained, MountPolicyProgram, RuleMatch,
    WriteDecision, WritePolicy, WriteRuleEffect, WriteRuleMatch,
};
pub use rule::{PathPolicyRule, RuleEffect, RuleOrigin};
pub use scope::{CollectedPolicy, MountsFragment, PolicyScope, ScopeKind, WritesFragment};
pub use value::{PolicyScalar, PolicyValue};

static COLLECTED: std::sync::Mutex<Option<CollectedPolicy>> = std::sync::Mutex::new(None);

pub fn set_collected_policy(policy: Option<CollectedPolicy>) {
    *COLLECTED.lock().unwrap_or_else(|e| e.into_inner()) = policy;
}

pub fn get_collected_policy() -> Option<CollectedPolicy> {
    COLLECTED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
