# ADR 0004: Security allowlist in policy.rs

**Status:** Accepted
**Date:** 2026-07-18

## Context

Config defines security policy (egress hosts, secret→host bindings). The
trust boundary is: config is untrusted data; the compiled tool is trusted. A
mechanism is needed to enforce that config cannot exceed a core-defined
ceiling.

## Options considered

1. **Rust consts in `policy.rs`** — type-safe, compile-checked, baked into
   binary. Selected.
2. **Data file (`policy.toml` via `include_str!`)** — adds parsing complexity
   with no benefit at this scale (~10 hosts, ~7 bindings, ~6 packages).
   Rejected.

## Decision

Rust consts in a dedicated `policy.rs` module:

- `ALLOWED_EGRESS_HOSTS`: the set of hosts config may reference.
- `SECRET_HOST_BINDINGS`: which secrets may bind to which hosts.
- `ALLOWED_PACKAGES`: package vocabulary for nix-layered images.
- `DEFAULT_DENY_FALSE_ENTITLEMENT`: workloads entitled to `default_deny=false`.

Per-recipe scoping: recipes without host params (`dns`, `litellm_proxy`,
`github`, `agent_base`) are fully fixed in core. The `https` recipe takes
`hosts: Vec<String>` validated against `ALLOWED_EGRESS_HOSTS`.

Enforcement at three points: `validate-config` (pre-flight), `plan`
(fail-closed), `apply_plan_secrets` (runtime, `runtime.rs:107-145`).

## Consequences

- The allowlist is small and changes infrequently — consts are appropriate.
- Adding a new egress host or secret binding requires a core code change
  (reviewed, versioned).
- The current `secrets.rs:33-92` const `SecretDefinition` pattern already
  demonstrates this approach.

## Rejected why

Data file adds parsing complexity with no benefit at this scale.
