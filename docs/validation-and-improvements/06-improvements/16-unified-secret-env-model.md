# 16 — Final unified secret/env model

> **STATUS: EXECUTED (2026-08-01; commit `19b2cf0` — final model landed: per-binding `bound`, `true` sugar, `allowed_hosts`, `schema_version` back to 1; supersedes the intermediate v2 delivery-on-def model of 1ed2e6d)**
> Prerequisites / see-also: [../README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [13-secret-env-shorthand.md](13-secret-env-shorthand.md) ·
> [14-env-map-form.md](14-env-map-form.md) ·
> [ADR 0018](../../migration/50-decisions/0018-secrets-layering-and-per-repo-config.md) ·
> [../02-config-requirements.md](../02-config-requirements.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs-only (`verifiable-here`); the cargo/schema/tombi gates it
specifies run via `nix develop`. Runtime smoke items are `HOST-KVM`.

---

## Summary

The secret/env surface collapses into its final form:

- **Secret definitions** `[secrets.<ID>]` are a pure catalog of intrinsic
  credential properties — source env var, substitution restriction,
  requiredness, placeholder. No exposure mode, no remaps, no prose.
- **Workload env** is ONE unified map `[workloads.<name>.env]` with cascading
  defaults. The binding decides exposure via a per-binding `bound` property
  (`host` default = placeholder; `guest` = real value). There is no
  `secret_env` namespace, no remap aliases, no `delivery` field anywhere.
- The intermediate v2 model (commit `1ed2e6d` — the `delivery` field on the
  definition plus the `fold_legacy_secret_model` v1 shim) is SUPERSEDED and
  its machinery REMOVED. It never shipped to production and there is no
  migration to preserve, so the v2 event is retracted and `schema_version`
  collapses back to `1` — v1 remains the native and only schema version.

This spec pins the final syntax, the runtime split, the merge rules, the
rationale, and a per-construct migration guide from both v1 and the
intermediate v2 forms.

---

## 1. Secret definitions `[secrets.<ID>]`

A secret definition is a pure catalog of the credential's intrinsic
properties. It says nothing about how any workload sees the value — exposure
is the workload's business, declared at the binding site (§2–§3).

```toml
[secrets.LITELLM_MASTER_KEY]
env_var = "LITELLM_MASTER_KEY"                # optional; default = the secret ID
required = true                                # optional; default true
placeholder = "change_me_before_first_boot"   # optional

[secrets.GITHUB_TOKEN]
allowed_hosts = ["host.microsandbox.internal"] # substitution restriction
required = true
```

| Field | Type | Required | Semantics |
|---|---|---|---|
| `env_var` | `Option<String>` | no | Host env var the resolved value is read from (default: the secret ID). |
| `allowed_hosts` | `Option<Vec<String>>` | no | Credential-level substitution restriction: the egress hosts whose rewrites may substitute the real value. **Always valid** regardless of any workload's binding mode — there is NO binding-mode validation on it. Omitted → deny-all (the value is never substituted anywhere). Explicit `[]` → clears any inherited value, then deny-all. |
| `required` | `Option<bool>` | no | Missing value is a hard error when `true` (default `true`). |
| `placeholder` | `Option<String>` | no | Known-bad placeholder value to reject; merge tri-state (inherit / set / clear). |

**Removed from the def** (no replacement — the concerns moved to the binding
site or were dropped): `description`, the `source`/`exposed_as` remap pair,
and the intermediate-v2 `delivery` field. Because v2 was retracted
pre-release there is no shim: these keys are unknown fields and hard-error at
parse time under `deny_unknown_fields`.

**`allowed_hosts` semantics.** The restriction is intrinsic to the
credential, not to any binding: a credential that must never leave the host
says so in exactly one place, and the rule applies however workloads bind it.
Omission is deny-all — the value is never substituted anywhere — because
fail-safe defaults apply to the catalog too. An explicit empty array `[]` is
not a no-op: it CLEARS any inherited (lower-layer) host list and then behaves
as deny-all, which is the only way an upper layer can revoke a lower layer's
substitution grants (see §5 merge rules).

---

## 2. The unified workload env map

Each workload has ONE env map, `[workloads.<name>.env]`. Values desugar with
cascading defaults: **the secret defaults to the KEY name**; **`bound`
defaults to `host`**.

| You write | Desugars to | Meaning |
|---|---|---|
| KEY = "value" | literal | plain env value |
| KEY = true | { bound: host } | host-bound placeholder for secret KEY (no secret property → secret defaults to key name; false → hard error "did you mean true?") |
| KEY = { bound = "guest" } | { secret: KEY, bound: guest } | real value for secret KEY |
| KEY = { secret = "ID" } | { secret: ID, bound: host } | placeholder for ID ≠ KEY (rename) |
| KEY = { secret = "ID", bound = "guest" } | full form | real value for ID ≠ KEY (renamed real value) |

Cascading defaults in detail:

- **`secret` defaults to the key name.** A binding that never names a secret
  binds the secret whose ID IS the env name (`KEY = true`,
  `KEY = { bound = "guest" }`). The `secret` property is therefore
  **rename-only**: it is written only when the exposed name differs from the
  secret ID (`OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY" }`). Names
  never repeat unless renaming — there is no form in which
  `KEY = { secret = "KEY" }` is anything but redundant noise.
- **`bound` defaults to `host`.** Any binding that does not say otherwise
  renders the placeholder (least exposure). The real value requires the
  explicit `bound = "guest"` opt-in (§3).
- **`KEY = false` is a hard error** ("did you mean true?"). `false` has no
  meaning in this desugar — a binding either binds or is absent — so the
  parser rejects it with a corrective hint rather than silently dropping the
  binding.

Example:

```toml
[workloads.litellm.env]
PORT = "4000"                                            # literal
LITELLM_LOCAL_MODEL_COST_MAP = "True"                    # literal
LITELLM_MASTER_KEY = { bound = "guest" }                 # real value (verifier)
OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY" }       # rename, placeholder (host)

[workloads.pi.env]
OPENAI_API_KEY = true                                    # placeholder for secret OPENAI_API_KEY
GITHUB_TOKEN = { secret = "GITHUB_TOKEN" }               # same-name rename — legal but redundant; prefer `true`
```

There is NO `secret_env` namespace, NO remap aliases, NO `delivery` field
anywhere. Map-form invariants from spec 14 continue to hold: document order
is preserved, duplicate keys are a hard TOML parse error, and mixing the
array-of-tables and map forms for one workload is a TOML redefinition parse
error.

---

## 3. `bound` semantics — per-binding exposure

`bound = guest | host`, default `host`:

- **`host` (the default) — the placeholder.** The workload's env renders a
  known-bad placeholder value; the real value is substituted by the egress
  rewrite only for hosts in the credential's `allowed_hosts`. This is the
  presenter posture: the workload shows a placeholder to anything inside the
  sandbox, and only allowlisted egress endpoints ever see the credential.
- **`guest` — the real value.** The real resolved value is injected as a
  plain sandbox env var. This is the explicit opt-in for **verifier
  workloads** — workloads that legitimately need the credential in-process to
  authenticate against a peer:
  - litellm itself: `LITELLM_MASTER_KEY = { bound = "guest" }` — the proxy
    must hold its own master key to verify incoming callers.
  - odysseus: `ODYSSEUS_ADMIN_PASSWORD = { bound = "guest" }` — the admin
    API must hold the admin password to verify admin requests.

The placeholder is the least-exposure default because it is what the natural
write produces (`KEY = true`, `KEY = { secret = "ID" }`). Getting the real
value requires writing `bound = "guest"` — an act that shows up in review and
in `plan --show-source` provenance.

---

## 4. Runtime split (plan build)

At plan build, a single ordered pass over the workload's env bindings splits
three ways:

- **Literal** → `builder.env` (plain value).
- **Host-bound** → `builder.secret_env`: renders the placeholder in the
  sandbox env; the egress rewrite substitutes the real value only for hosts
  in the credential's `allowed_hosts` (omitted/empty = deny-all, so the value
  never leaves the host).
- **Guest-bound** → real-value env injection: the resolved value lands in
  `builder.env` as the actual credential for that workload only.

The runtime security posture is preserved exactly relative to every prior
model: presenter workloads carry placeholders plus an egress rewrite;
verifier workloads carry the real value. What changed is WHERE the choice is
declared — at the binding (correctly scoped per workload) instead of on the
definition (def-global).

---

## 5. Merge rules

- **Scalars replace** (last-layer-wins).
- **Maps merge by key** (deep-merge per key).
- **Arrays wholesale-replace** (no partial row merge).
- **Env bindings are atomic**: a binding replaces a same-key binding
  wholesale — it never field-merges. `X = { secret = "A" }` in a lower layer
  followed by `X = "literal"` in an upper layer yields the literal, not some
  merged hybrid.
- **Omission = inherit.** A layer that does not mention a field/binding
  inherits the merged-so-far value.
- **Explicit empty = clear.** An explicitly empty value (`[]`, `""`) replaces
  the inherited value with emptiness (e.g. `allowed_hosts = []` clears a
  lower layer's host list, then deny-all).
- **Defaults are applied ONLY after the full merge.** The `secret`-defaults-
  to-key-name and `bound`-defaults-to-`host` desugars, and field defaults
  like `required = true`, apply to the final merged config — never per-layer.
  This keeps overlay inheritance intact: an upper layer can rely on a lower
  layer's binding without re-stating defaulted properties.
- **Deletion is a separate layer operation**, not a merge value. There is no
  `null`-to-delete in the merge algebra; removal is expressed by the
  layer-operation mechanism, keeping the merge total and order-independent of
  sentinel values.

---

## 6. Naming and version decisions

- **`hosts` → `allowed_hosts`.** The definition field is renamed: the old
  name read as a workload-binding property and implied binding-mode coupling
  that does not exist. `allowed_hosts` states what it is — the set of egress
  hosts allowed to receive the credential by substitution — and it is valid
  regardless of binding mode.
- **`schema_version` collapses to `1`.** The intermediate v2 event (commit
  `1ed2e6d`: `delivery` on the def, the `fold_legacy_secret_model` v1 shim,
  the one-cycle legacy tolerance) is SUPERSEDED AND REMOVED. There was no
  production deployment of v2 and no migration to preserve, so the v2 event
  is retracted: v1 remains the native and only schema version, `missing/0`
  stays accepted as legacy with a stderr warning, and `>= 2` is a hard error.

---

## 7. Rationale

The model is locked on six reasons. Syntax follows from these; they are not
decorations.

### 7.1 Fail-safe defaults

The natural write — `KEY = true` or `KEY = { secret = "ID" }` — produces the
placeholder, the least-exposure posture. The real value requires the explicit
`bound = "guest"` opt-in. An author who never learns the model cannot
accidentally hand a credential to a sandbox: the shortest, most ergonomic
form is also the safest, and escalation is always deliberate and greppable.

### 7.2 No def-global mode

The intermediate v2 put `delivery` on the definition, making exposure
def-global: any workload binding the def inherited the def's mode. In
practice that gave the pi workload the real `LITELLM_MASTER_KEY` it never
needed — because the def said `delivery = "env"` (so litellm itself could
verify callers), every other binding of that def also received the real
value. Per-binding `bound` fixes the scope: exposure is a property of THIS
workload's relationship to the credential, so only the workloads that verify
(litellm, odysseus) opt into `guest`; everyone else stays on the placeholder.

### 7.3 Minimal repetition

`secret` is rename-only — it appears only when the exposed name differs from
the secret ID, so names never repeat unless renaming. And verifier workloads
need no name repeat either: `LITELLM_MASTER_KEY = { bound = "guest" }` binds
the same-named secret without restating it. The only property ever written is
one that carries information.

### 7.4 Catalog purity

Definitions carry only intrinsic credential properties: the source env var
the value is read from, the substitution restriction (`allowed_hosts`),
requiredness, and the placeholder. Exposure is the workload's business. This
is the same separation ADR 0018's first addendum reached for remaps (a remap
is binding material, not definition material) carried to its conclusion:
delivery mode was binding material too, and catalog purity demands it leave
the def.

### 7.5 Runtime security posture preserved exactly

The plan/runtime split is unchanged in effect: host-bound placeholders plus
egress rewrite for presenter workloads; real values only for verifiers. No
workload gains or loses exposure under the final model relative to the
intended v2 posture — the pi over-exposure was a v2 BUG the final model
fixes, not a behavior to preserve.

### 7.6 Merge correctness

Defaults are applied only after the full merge. If defaults were applied
per-layer, a lower layer's `KEY = { secret = "ID" }` would bake in
`bound = host` before an upper layer merges, and overlay inheritance would
degrade to per-layer snapshots. Applying defaults once, post-merge, keeps an
upper layer's partial overlay meaningfully partial: it overrides what it
states and inherits everything else.

---

## 8. Migration guide

Per-construct mapping from both v1 and the intermediate v2 forms to the final
model. All forms desugar under the same rules (§2); `schema_version` is `1`
throughout.

| From | To | Notes |
|---|---|---|
| v1: `[[workloads.x.secret_env]] secret = "NAME"` (and spec 13's shorthand `secret_env = ["NAME"]`) | `NAME = true` | The `true` sugar is the new ergonomic shorthand replacing spec 13's bare-string array. Same-name host-bound placeholder; the `secret_env` namespace is gone. |
| v1: `env` entry `{ name = "EXPOSED", secret = "SOURCE" }` backed by a remap def (`source`/`exposed_as`) | `EXPOSED = { secret = "SOURCE" }` | The remap lives at the binding site; the map key IS the exposed name. The remap def is dropped. |
| v2: `{ secret = "ID" }` with def `delivery = "host_bound"` (or delivery omitted) | unchanged — `{ secret = "ID" }` (or `ID = true` when same-name) | Meaning unchanged: default `bound = host` is the placeholder. |
| v2: `{ secret = "ID" }` with def `delivery = "env"` | `ID = { bound = "guest" }` (same-name) or `EXPOSED = { secret = "ID", bound = "guest" }` (renamed — e.g. an env-delivery remap `OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY", bound = "guest" }`) | The def-global mode becomes the per-binding opt-in, applied only at the workloads that actually verify. |
| def: `hosts = [...]` | `allowed_hosts = [...]` | Pure rename; semantics now binding-mode-independent. |
| def: `delivery`, `description`, `source`, `exposed_as` | deleted — no replacement | Exposure moved to the binding site; prose and remaps have no home in the catalog. Unknown-field hard error under the final schema. |
| layer: `schema_version = 2` | `schema_version = 1` | v2 retracted pre-release; v1 is the native and only version. |

---

## 9. Acceptance criteria

- **Desugar unit tests** — one test per row of the §2 desugar table,
  including the `KEY = false` hard error asserting the "did you mean true?"
  message, and the cascading defaults (`secret` defaults to key name; `bound`
  defaults to `host`).
- **Golden plans** — byte-identical to the current v2 golden plans except the
  intended litellm/odysseus real-value bindings (`LITELLM_MASTER_KEY` and
  `ODYSSEUS_ADMIN_PASSWORD` guest-bound) and the removal of any v2-only
  rendering.
- **`allowed_hosts`** — rename tests (`hosts` rejected as unknown field),
  omitted = deny-all substitution test, explicit `[]` = clear-then-deny-all
  merge test.
- **Merge** — defaults-after-merge tests proving overlay inheritance survives
  (lower-layer binding + upper-layer partial overlay → defaults resolve once,
  post-merge); env-binding atomicity tests (same-key replace, never
  field-merge).
- **Schema** — schema regen with `schema_version` back to `1` (`EXPECTED_SCHEMA_VERSION = 1`);
  `>= 2` hard error test; the committed schema shows the final def shape (no
  `delivery`/`description`/`source`/`exposed_as`) and the binding shape with
  `secret`/`bound`.
- **tombi/schema gates** — `just tombi-check`, schema drift guard, and the
  scaffold schema emission all pass against the final schema (cargo gates via
  `nix develop`, `verifiable-here`).
- **B13 runtime smoke** — the eight secret-delivery smoke items in
  [../05-host-validation.md](../05-host-validation.md) **B13** are re-pointed
  at the final model: guest-bound real values reaching the verifier workloads
  (litellm master key, odysseus admin password), `models.json`
  substitutions, host-bound placeholders + TLS substitution against
  `allowed_hosts`, deny-all behavior for credentials with no `allowed_hosts`,
  and failure semantics (`required` + missing value). `HOST-KVM`.

---

## 10. Doc-intent requirement

> This spec requires that user-facing documentation explain the model's
> INTENT — the fail-safe placeholder default, the rename-only `secret`
> property, and why `bound` is per-binding — not just its syntax.

Concretely:

- **Config-repo READMEs** (scaffolded and existing) must state, next to the
  first env example: bindings default to the placeholder; the real value is
  an explicit `bound = "guest"` opt-in reserved for workloads that verify the
  credential; `secret` appears only when renaming.
- **The generated env-example output** (`workestrate generate-env-example`)
  must emit commented examples that teach the same three facts, not merely
  list the keys.
- **Scaffold templates** (`config new`) must ship examples that exercise the
  sugar (`KEY = true`, `KEY = { bound = "guest" }`) with comments explaining
  why each form was chosen, so a new config author copies the intent along
  with the syntax.

Documentation that shows only the grammar re-creates the v2 mistake: authors
will write `bound = "guest"` everywhere because it "just works". The intent
IS the security property, and the docs must carry it.
