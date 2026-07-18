# ADR 0014: Trust-gated project config

**Status:** Accepted
**Date:** 2026-07-18

## Context

A project-layer `./workestrate.toml` in the current working directory could
inject config when the user runs `workestrate` from an untrusted project
directory. This is a security concern: a malicious project could widen egress
or rebind secrets (within the policy.rs ceiling, but still undesirable).

## Options considered

1. **Always load `./workestrate.toml`** — no trust gating. Rejected: malicious
   project dirs can inject config.
2. **Never load `./workestrate.toml`** — no project layer. Rejected: loses
   useful per-project override capability.
3. **Trust-gated: only load `./workestrate.toml` from trusted directories** —
   `[[trusted_projects]]` in registry; `workestrate config trust <dir>`.
   Selected.

## Decision

Project-layer config (`./workestrate.toml` in cwd) is trust-gated. Only
projects listed in `[trusted_projects]` in the registry have their
`./workestrate.toml` loaded. `workestrate config trust <dir>` adds a project.
`--no-project-config` flag disables project-layer loading entirely (escape
hatch for running in untrusted directories).

This is the `direnv allow` model: explicit trust required before loading
project-local config.

## Consequences

- `cd` into an untrusted project → `./workestrate.toml` is silently ignored.
- `workestrate config trust $(pwd)` → adds to `[trusted_projects]`.
- `--no-project-config` → disables even for trusted projects (escape hatch).
- The project layer is still bounded by policy.rs (defense in depth).

## Rejected why

Always load: security risk. Never load: loses useful capability.
