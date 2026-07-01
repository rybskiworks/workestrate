---
name: constraint-rust-scope-discipline
description: |
  Enforces scope discipline rules during any Rust task. Load when executing any
  Rust workflow phase (implementation, debugging, refactoring, review). Does
  NOT cover what the workflow phases themselves are (see
  docs/rust/workflows/index.md).
metadata:
  org.kind: constraint
---

# Constraint: Rust Scope Discipline

This constraint enforces scope discipline across all Rust workflow phases. Each
task has a single intent; unrelated work is recorded as follow-up, not folded
in. Mixing concerns makes changes hard to review, hard to revert, and hard to
validate.

## Triggers

Load this skill when:

- Executing any Rust workflow phase: implementation, debugging, refactoring, or
  review.
- Tempted to fix an unrelated defect encountered mid-task.
- Tempted to refactor a module the task only touches at a call site.
- Tempted to add a dependency, feature flag, or lint policy change beyond the
  task's requirement.
- Reviewing a diff for scope creep.

## Rules

1. Only change what the current task requires.
2. If unrelated cleanup appears, record it as a follow-up; do not fold it into
   the current change.
3. Do not refactor modules the task only touches at call sites.
4. Do not add dependencies, feature flags, or lint policy changes beyond what
   the task requires.
5. Do not fix defects encountered during refactoring — switch to the debugging
   workflow.
6. Do not add features during debugging — switch to the implementation
   workflow.
7. Keep each incremental step small enough to revert cheaply.

## References

- Docs: `docs/rust/workflows/index.md` (the decision tree for switching between
  implementation, debugging, refactoring, and review phases).

## Out of scope

- What the workflow phases themselves are and how to choose among them — see
  `docs/rust/workflows/index.md`.
- Code style and lint policy — see `constraint-frontend-code-style` (frontend)
  and `rust-lints-and-clippy` (Rust).
- Validation gates — see the `validation-rust-*` skills.

## Violation examples

### Fixing an unrelated bug during a feature task

Task: "Add a `timeout` field to `Config`."

```rust
// config.rs
pub struct Config {
    pub timeout: Duration,   // intended change
    // ...
}

// parser.rs — UNRELATED fix folded in:
pub fn parse(raw: &str) -> Config {
    // ... fixed an off-by-one in token splitting (FORBIDDEN: not the task)
}
```

Correct: implement only the `timeout` field; record the parser bug as a
follow-up and switch to the debugging workflow separately.

### Refactoring a module only touched at a call site

Task: "Call `parse` with a `&str` instead of `String`."

```rust
// caller.rs
let cfg = parse(&raw);   // intended change

// parser.rs — UNRELATED refactor folded in:
pub fn parse(raw: &str) -> Config {
    // ... rewrote the whole parser to use iterators (FORBIDDEN)
}
```

Correct: change only the call site; record the parser refactor as a follow-up.

### Adding a dependency beyond the task

Task: "Add a `timeout` field of type `Duration`."

```toml
# Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }  # FORBIDDEN: not required
```

Correct: `Duration` is in `std`; no new dependency needed.

### Adding a feature during debugging

Task: "Debug the intermittent `parse` panic."

```rust
pub fn parse(raw: &str) -> Config {
    // ... added a new `--strict` mode (FORBIDDEN: feature work during debug)
}
```

Correct: fix the panic only; record the `--strict` feature as a follow-up and
switch to the implementation workflow.

## How to check

```bash
git diff --stat                # only intended files changed
git diff                       # every hunk maps to the task intent
git log --oneline -1           # commit message matches the task
```

Manual review:

- Every changed file is required by the task.
- Every hunk is required by the task.
- No new dependencies, feature flags, or lint policy changes unless the task
  requires them.
- Unrelated findings are recorded as follow-ups, not folded in.
- Each step is small enough to revert with a single `git revert`.
