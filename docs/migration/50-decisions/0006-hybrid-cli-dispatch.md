# ADR 0006: Hybrid CLI dispatch

**Status:** Accepted
**Date:** 2026-07-18

## Context

After removing the `workloads!` macro (`main.rs:49-69`), the CLI must dispatch
`workestrate <name> <action>` commands. The `workestrate <name> <action>` UX is
referenced 20+ times in README.md, 6 times in SPEC.md, and throughout
docs/secrets.md, justfile, and scripts.

## Options considered

1. **Pure `external_subcommand`** — clap captures any unrecognized subcommand
   as `Vec<String>`. Rejected: no typed subcommands for known workloads;
   completions and help text degrade.
2. **Dynamic clap subcommands via builder API** — generate subcommands from
   config at parse time. Rejected: config loading at parse time produces
   confusing errors; significant refactor from derive API.
3. **`build.rs` codegen** — generate clap subcommand code from config at build
   time. Rejected: Phase 2 incompatibility (config-repo-flake can't run core's
   build.rs); config changes require recompile.
4. **Hybrid: typed subcommands for known names + catch-all** — typed clap
   subcommands for litellm/pi/odysseus/opencode/tempest (preserves completions
   + help) + `external_subcommand` catch-all for config-defined names.
   Selected.

## Decision

Hybrid dispatch: typed clap subcommands for the 5 known workload names
(preserving `workestrate pi exec` UX, completions, and help text) + an
`external_subcommand` catch-all variant for config-defined workload names
(no recompile needed for new workloads).

## Consequences

- Known workloads get full clap support (completions, help, arg parsing).
- Config-defined workloads work via catch-all (slightly less polished UX).
- No config loading at parse time (config loads after arg capture).
- Static commands (`check`, `new`, `completions`, `run`, `config`, `init`,
  `source`, `validate-config`, etc.) always work.

## Rejected why

Pure external_subcommand: degrades completions/help. Builder API: config
loading at parse time is confusing. build.rs codegen: Phase 2 incompatible.
