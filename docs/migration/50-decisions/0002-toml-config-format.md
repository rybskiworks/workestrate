# ADR 0002: TOML config format

**Status:** Accepted
**Date:** 2026-07-18

## Context

The config format must be readable by both the Rust CLI (serde) and Nix
(builtins). The existing YAML estate (`infra/litellm/config.yaml`,
`models.yaml`) stays YAML because LiteLLM consumes those files directly —
they're a different concern.

## Options considered

1. **YAML** — `serde_yaml` is archived (dtolnay archived it in 2024); `serde_yml`
   is a fork but less battle-tested. Nix has NO `builtins.fromYAML` — would
   require a non-builtin nix YAML library (fragile) or invoking
   `workestrate build-plan --json` (chicken-and-egg: Rust not built when
   entering devshell). Rejected.
2. **TOML** — `toml` crate (v0.8+) is actively maintained with good error spans.
   Nix has `builtins.fromTOML` (stable built-in since Nix 2.0). Selected.
3. **JSON** — no comments; verbose. Rejected.

## Decision

TOML format, `toml` crate (v0.8+), filename `workestrate.toml`.

## Consequences

- Nix can parse `workestrate.toml` at eval time via `builtins.fromTOML` — no
  Rust invocation needed for the devshell bridge.
- The devshell reads `config.reference/workestrate.toml` (tracked) for
  `_build_agents` and `workload-images` generation.
- TOML's array-of-tables syntax (`[[workloads.pi.env]]`) is more verbose than
  YAML inline arrays, but acceptable for a config file that changes
  infrequently.
- Existing YAML files (`infra/litellm/`) stay YAML (consumed by LiteLLM, not
  by workestrate's config loader).

## Rejected why

YAML: no `builtins.fromYAML` in Nix; `serde_yaml` archived. JSON: no comments,
verbose.
