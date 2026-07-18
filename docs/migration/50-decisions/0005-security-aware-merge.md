# ADR 0005: Security-aware merge

**Status:** Accepted
**Date:** 2026-07-18

## Context

Phase 3 introduces multi-layer config merging. A naive RFC 7396 JSON Merge
Patch (last-wins for all fields) would allow a less-trusted layer to weaken a
more-trusted layer's security policy — e.g., setting `default_deny = false` or
removing a deny rule.

## Options considered

1. **Uniform RFC 7396 for all fields** — simple, but creates a security hole:
   a personal layer could set `default_deny = false`, overriding a team
   layer's `true`. Rejected.
2. **Security-aware field-specific merge** — different rules for security vs
   non-security fields. Selected.

## Decision

| Field type | Merge rule | Rationale |
|---|---|---|
| `default_deny` | Monotonic-true: if any layer sets `true`, merged is `true`. Core per-workload entitlement for `false` (only tempest). | A less-trusted layer cannot weaken default-deny. |
| `deny_rules` | Additive-union within policy.rs ceiling. | Cannot remove a more-trusted layer's deny rule. |
| `egress_rules` | Additive-union within policy.rs ceiling + per-recipe scoping. | Cannot remove egress rules (only add, within ceiling). |
| `secret_env` | Additive-union. | Cannot deprive a workload of required secrets. |
| All other fields | RFC 7396: last-wins scalars, deep-merge maps, replace lists, null-deletes. | Standard merge for non-security fields. |

## Consequences

- The merge engine is slightly more complex than pure RFC 7396, but the
  security fields are a small, known set.
- `default_deny = false` requires core entitlement
  (`DEFAULT_DENY_FALSE_ENTITLEMENT` in `policy.rs`).
- The policy.rs ceiling is enforced post-merge, so the merge engine itself
  doesn't need to know about the allowlist.

## Rejected why

Uniform RFC 7396 creates a security hole in `default_deny` (the most critical
security field).
