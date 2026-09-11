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

---

## Addendum (2026-07-19, ADR 0020)

Two clarifications recorded permanently in ADR 0020:

1. **env union-by-name.** `workloads.<name>.env` is name-keyed and merges
   union-by-name (last-write-wins per env-var key), NOT REPLACE. This
   aligns env with `secret_env`'s union-by-secret-name pattern. The other
   REPLACE lists (`ports`, `mounts`, `seed_files`, `local_build`,
   `network.ingress`) stay REPLACE — they are list-of-rows where partial
   replacement is ambiguous. (Closes review finding A3.)
2. **Entitlement before monotonic-true.** In `merge_network`, the
   `DEFAULT_DENY_FALSE_ENTITLEMENT` check runs BEFORE the monotonic-true
   check. Entitled workloads (`tempest`, `example-offensive`) may relax
   `default_deny` from `true` to `false` at a higher layer. Monotonic-true
   remains defense-in-depth for non-entitled workloads. (Closes review
   finding A4.)

See ADR 0020 for full rationale. This ADR's original decision table is
unchanged; this addendum clarifies two details that the original text
left ambiguous.

---

## Addendum (2026-08-01): `secret_env` removed; env bindings merge atomically per exposed name

Two updates from the final unified secret/env model (ADR 0018's second
2026-08-01 addendum; spec 16):

1. **The `secret_env` namespace is REMOVED.** The `secret_env` additive-union
   row in the decision table above no longer exists — loading a config that
   declares `secret_env` is a HARD ERROR per the final model. The 2026-07-19
   addendum's "aligns env with `secret_env`'s union-by-secret-name pattern"
   comparison is superseded in that respect (env's union-by-name rule
   itself stands).
2. **env bindings merge ATOMICALLY per exposed name.** In the final model,
   exposure is declared on the env binding itself (`KEY = true`,
   `{ secret = "ID" }`, `{ bound = "guest" }`). A binding merges
   union-by-name per exposed env-var name, but each binding is replaced
   WHOLESALE by a later layer — never field-merged (no partial merge of a
   binding's `secret`/`bound`/value parts).

The security-aware invariants of this ADR stand UNCHANGED: monotonic
`default_deny` (with entitlement-before-monotonic ordering per the 2026-07-19
addendum), additive deny/egress unions within the `policy.rs` allowlist
ceilings, and the entitlement checks.
