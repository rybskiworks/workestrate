# ADR 0001: Data-driven workloads

**Status:** Accepted
**Date:** 2026-07-18

## Context

The 5 workload definitions (litellm, pi, odysseus, opencode, tempest) are
hardcoded in Rust (`workloads/*.rs`). Each workload's `plan()` method returns a
`SandboxPlan` with hardcoded image strings, ports, env vars, egress hosts,
mounts, and commands. Adding or modifying a workload requires editing Rust
code and recompiling.

The `SandboxPlan` IR (`plan.rs:57-70`) is a clean data model, but it's
populated by code, not data. `Cargo.toml:13-19` has no `serde` with derive
and no TOML/YAML parser — only `serde_json` for parsing sops output.

## Options considered

1. **Keep Rust workloads, add config override layer** — config can override
   individual fields; Rust remains the default. Rejected: two sources of truth;
   precedence confusion.
2. **Migrate to data-driven config, delete Rust workloads** — config is the
   single source of truth; one generic `ConfigWorkload` implements the
   `Workload` trait. Selected.
3. **Generate Rust from config at build time (build.rs codegen)** — config
   drives code generation. Rejected: Phase 2 incompatibility (config-repo-flake
   can't run build.rs from core); adds build complexity.

## Decision

Migrate all 5 workloads to `workestrate.toml` (TOML config). Delete
`workloads/*.rs` after golden-file parity proves the config produces identical
plans. One generic `ConfigWorkload` struct implements the `Workload` trait
(`workload.rs:45-96`) by loading from config.

## Consequences

- Adding a workload = adding a TOML entry (no recompile).
- `workloads/*.rs` deleted; git history preserves the reference implementation.
- The `workloads!` macro (`main.rs:49-69`) is removed.
- Golden-file tests (`control/agentctl/tests/golden/`) ensure parity.
- `prepare()` semantics (seed files) captured by `seed_files` config field.
- `build_path()` semantics (`WORKESTRATE_<NAME>_BUILD`) captured by
  `local_build.env_override` + `fallback`.
- `config_path()` semantics captured by config-relative mount paths.

## Rejected why

Option 1 (override layer) creates two sources of truth and precedence
confusion. Option 3 (build.rs codegen) is incompatible with the config-repo-
flake model (Phase 2) where config repos have their own flake that can't run
core's build.rs.
