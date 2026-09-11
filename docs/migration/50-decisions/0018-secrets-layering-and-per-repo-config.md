# ADR 0018: Secrets layering + per-repo secrets config

**Status:** Accepted
**Date:** 2026-07-18

## Context

Phase 3 implemented config layering (merge engine for workload definitions,
secret definitions, egress rules, etc.). However, secret VALUES were loaded
from only the FIRST (lowest-precedence) registry layer's `.env.enc` — a
documented limitation in `70-open-items.md`. This meant:

- A personal layer could not override a team layer's secret values.
- Each config repo could not carry its own `.env.enc` with its own age key.
- The `secrets = "none"` pattern (team repo carries definitions but no
  committed secrets; personal repo provides values) was not supported.

## Options considered

1. **First-layer-wins (status quo)**: only the first layer's `.env.enc` is
   loaded. Rejected: cannot override values; cannot have per-repo secrets.
2. **Per-key merge across layers**: load `.env.enc` from every layer in
   precedence order, merge per-key (later wins). Selected.
3. **Prefix-based namespacing**: each layer's secrets get a prefix (e.g.
   `TEAM_LITELLM_MASTER_KEY`). Rejected: breaks env-name contracts with
   consumers (agents expect `LITELLM_MASTER_KEY`, not a prefixed variant);
   requires config changes per layer; provenance is lost.

## Decision

Implement per-key value merge across layers:

- **Precedence**: process env (lowest) < reference < registry layers (in
  declared order) < trusted project < local.
- **Merge rule**: later layer wins per key. A key in process env but not in
  any layer's `.env.enc` is accepted at lowest precedence.
- **Per-repo config**: each `[configs.<name>]` in the registry supports:
  - `secrets = "file"` (default) | `"none"` — `"none"` skips loading
  - `secrets_file = ".env.enc"` (default, path relative to config repo)
  - `age_key_file = "~/.config/sops/age/team.txt"` (optional; default =
    `SOPS_AGE_KEY_FILE` env or default path)
- **Failure semantics**: undecryptable layer → WARNING naming the layer,
  continue with other layers. Required secret unsatisfied after all layers
  → hard fail naming the secret + layers tried + remediation.
- **Provenance**: per-key value provenance is tracked (which layer's
  `.env.enc` supplied the winning value, or process-env fallback).
  `plan --show-source` shows config-FIELD provenance (which layer's
  `workestrate.toml` defined each env/secret_env entry); it does NOT
  decrypt secrets or show value provenance (plan is side-effect-free and
  works without secrets — the fail-closed reference fallback depends on
  this). Secret-VALUE provenance is surfaced in secret-loading contexts
  (`run`/`up`/`exec`), with redacted values.

## Consequences

- Each config repo is its own secrets domain (own `.sops.yaml` + `.env.enc`).
- Secret DEFINITIONS merge across layers (Phase 3 merge.rs, later wins per
  key, hosts validated against `policy.rs`).
- Secret VALUES merge across layers (this ADR, later wins per key).
- Value-override cannot widen policy: the policy.rs allowlist is enforced
  post-merge on the merged secret definitions, not on individual values.
  A layer can provide a different VALUE for `LITELLM_MASTER_KEY`, but the
  HOST BINDING (`host.microsandbox.internal`) is fixed in `policy.rs`.
- `setup-secrets --config <name>` targets a specific repo's directory.
  The Rust loader (`load_secrets()`) honors per-repo `secrets_file` and
  `age_key_file` overrides from the registry. `setup-secrets.sh` currently
  honors directory targeting only (per-repo file/key overrides require
  `SOPS_AGE_KEY_FILE` set manually) — script alignment is a documented
  follow-up (see 70-open-items.md).

## Rejected why

First-layer-wins: cannot override values; cannot have per-repo secrets.
Prefix-based: breaks env-name contracts; requires config changes per layer.

## Addendum (2026-08-01): v2 unified secret model (P1 Wave 1, commit 1ed2e6d)

The secret definition surface was unified into a single `SecretDefConfig`
(`control/agentctl/src/config/types.rs:574`) with an explicit `delivery`
field (`env` | `host_bound`, snake_case TOML; default `host_bound` —
secure-by-default; `types.rs:555`). Remap defs (`source` / `exposed_as`) and
`description` were REMOVED from the v2 schema — they remain parseable for
one shim cycle only (the post-merge `fold_legacy_secret_model` fold,
`merge.rs:147-208`), are hard-rejected for `schema_version = 2` layers, and
are `#[schemars(skip)]` (absent from the emitted v2 JSON schema). `env_var`
now defaults from the secret ID (it names the HOST env var the resolved
value is read from). `hosts` is scoped to host-bound delivery only: omitted
= deny-all (the value never leaves the host); `delivery = "env"` + `hosts`
is a hard error (reachability is then governed by egress rules, not host
bindings).

**Rationale:** the LITELLM_AUTH remap was a workload-exposure concern, not
definition material — the remap now lives at the binding site
(`OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY" }`; the map key IS the
exposed name). The two delivery mechanisms are irreducible (local
consumption as an env var vs egress injection to bound hosts), so `delivery`
is declared on the DEFINITION, with a per-binding override reserved as a
possible future extension. Provenance is re-keyed to binding sites
`workloads.{wl}.env.{NAME}` (the def-name indirection is gone).

This addendum does not change the per-repo layering/merge decisions above;
it replaces the per-secret definition shape those decisions operate on.

## Addendum (2026-08-01, second): final unified secret/env model — supersedes delivery-on-def

The intermediate v2 `delivery` field on the definition is REMOVED. Exposure
mode moves to the binding as a per-binding `bound` property
(`bound = guest | host`, default `host`): `host` renders the placeholder
(least exposure; the egress rewrite substitutes the real value only for
hosts in `allowed_hosts`), and `guest` injects the real value as a plain
sandbox env var — the explicit opt-in for verifier workloads. The unified
workload env map gains cascading sugar: `KEY = true` desugars to a host-bound
placeholder for the same-named secret; `KEY = { bound = "guest" }` is the
bound-only object for a same-name real value (no name repeat); the `secret`
property is rename-only (written only when the exposed name differs from the
secret ID). `hosts` on the definition is RENAMED to `allowed_hosts` — always
valid regardless of binding mode (no binding-mode validation), omitted =
deny-all, explicit `[]` = clear-then-deny-all. `schema_version` collapses
back to `1`: the v2 shim/delivery machinery (commit `1ed2e6d`, the
`fold_legacy_secret_model` fold, `delivery` on the def) is retracted
pre-release — there was no production deployment and no migration to
preserve, so v1 remains the native and only schema version and `>= 2` is a
hard error.

**Rationale:** delivery-on-def over-exposed. Because the mode was def-global,
any workload binding the def inherited the def's mode — the pi workload
received the real `LITELLM_MASTER_KEY` it never needed, simply because the
def said `delivery = "env"` so that litellm itself could verify callers.
Per-binding `bound` is fail-safe (the natural write — `KEY = true`,
`KEY = { secret = "ID" }` — yields the placeholder) and correctly scoped
(only verifier workloads opt into the real value, e.g. litellm's
`LITELLM_MASTER_KEY = { bound = "guest" }` and odysseus's
`ODYSSEUS_ADMIN_PASSWORD = { bound = "guest" }`). The runtime security
posture is preserved exactly: presenter workloads keep placeholders plus
egress rewrite; real values reach verifiers only.

The execution spec is
`docs/validation-and-improvements/06-improvements/16-unified-secret-env-model.md`;
the requirements contract is
`docs/validation-and-improvements/02-config-requirements.md`. Neither this
addendum nor the first changes the per-repo layering/merge decisions above;
they replace the per-secret definition shape and binding surface those
decisions operate on.
