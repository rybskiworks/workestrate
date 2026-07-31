# 13 — Config ergonomics: string-or-table shorthand for `secret_env`

> **STATUS: EXECUTED (2026-07-31; commits a349d03, 1e7dc25; personal config converted in .tmp — separate commit there)**
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [../../migration/50-decisions/0002-toml-config-format.md](../../migration/50-decisions/0002-toml-config-format.md) ·
> [../../migration/50-decisions/0003-config-purity-closed-vocabulary.md](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md)

This document specifies a standalone config-ergonomics improvement: allow
`secret_env` entries to be written as **bare strings** (shorthand for
`{ secret = "NAME" }`) in addition to the existing inline-table form. The
change is additive and serde-only: the parsed data model, merge, validation,
and plan-build code paths are all untouched.

### Environment markers

- `verifiable-here` — all gates for this spec are cargo-linked gates runnable
  inside the container via `nix develop` (nix at
  `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH;
  prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
  then `nix develop -c bash -c '<cmd>'`). No KVM is needed.

---

## 1. Design

`secret_env` accepts a **heterogeneous array**: bare strings (shorthand for
`{ secret = "NAME" }`) and inline tables (the existing full form — kept as
forward-compat for future per-entry fields such as per-workload `hosts`
narrowing).

### 1.1 Why the shorthand fits today

- `SecretEnvConfig` (`control/agentctl/src/config/types.rs:79-84`) has exactly
  **ONE** field (`secret: String`) plus `deny_unknown_fields`; a meaningful
  "full form" is impossible today — every table is noise.
- Census of the personal config: **9/9** `secret_env` entries across 4
  workloads are bare `secret = "NAME"` (100% shorthand-eligible); the litellm
  block at `workestrate.toml:71-81` is 11 lines → collapses to 1 line.
- All consumers read only `.secret`: validation
  (`control/agentctl/src/config/validation.rs:318-326`, the secret must exist
  in `config.secrets`), and plan build
  (`control/agentctl/src/microsandbox/workload/secrets.rs:112-158` →
  `HostBoundSecret::from`/remapped using the **DEFINITION's**
  `env_var`/`exposed_as`/`hosts`/`required`).
- Merge (`control/agentctl/src/config/merge.rs:385-404`): union-by-secret-name,
  last-layer-wins, replace-in-place, per-key provenance (FN-3) — operates
  post-parse on the struct, so expansion at deserialize time is fully
  compatible.

### 1.2 Implementation shape

An untagged helper enum used **ONLY** for serde + schemars, normalized to
`Vec<SecretEnvConfig>` at deserialize time (via `deserialize_with` or `From`):

```rust
// sketch — serde/schemars boundary only; never stored
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
enum SecretEnvShorthand {
    Bare(String),               // "NAME"
    Full(SecretEnvConfig),      // { secret = "NAME", ... }
}
// normalized to SecretEnvConfig { secret } at deserialize time
```

Field type (`Vec<SecretEnvConfig>`), merge, validation, and plan build are all
**untouched** — they see the identical post-parse struct they see today.

### 1.3 Untagged warts (assessed, acceptable)

There is **zero** `#[serde(untagged)]` precedent in the codebase
(`EgressRecipeRef` is internally-tagged; everything else is struct +
`deny_unknown_fields`). The known untagged warts:

- serde's "did not match any variant" diagnostics are vague — **Implemented:**
  avoided entirely with a custom `deserialize_secret_env` visitor; a bad element
  (e.g. `secret_env = [42]`) fails with
  `secret_env entry at index 0: invalid type: integer \`42\`, expected a bare
  secret name string, or an inline table like { secret = "NAME" }` (names the
  index, the offending type, and both expected forms);
- schemars emits an `anyOf` (not `oneOf`) in the JSON schema (schemars 0.8.22;
  editor UX slightly noisier).

Both are mitigated here because a typo'd bare name is caught **precisely** by
`validation.rs:318-326` (secret must exist in `config.secrets`) — the error
the user actually cares about names the missing secret.

### 1.4 Why not a map idiom

`depends_on` uses a map idiom (`HashMap<String, DependsOnSpec>`) — that fits
because each key carries per-key data. It does **not** fit `secret_env`: there
is no per-key data, so a map would force `NAME = {}` noise.

### 1.5 Before / after (litellm workload, workestrate.toml:71-81)

Before — 11 lines of nine `[[workloads.litellm.secret_env]]` tables:

```toml
[[workloads.litellm.secret_env]]
secret = "LITELLM_MASTER_KEY"

[[workloads.litellm.secret_env]]
secret = "ANTHROPIC_API_KEY"

[[workloads.litellm.secret_env]]
secret = "OPENAI_API_KEY"

# … six more identical-shape tables …
```

After — a single line (representative placeholder names):

```toml
[workloads.litellm]
secret_env = ["LITELLM_MASTER_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY", ...]
```

(Names shown are placeholders; the point is the 11-lines → 1-line collapse.
Mixed forms remain legal — e.g. one inline table among bare strings.)

---

## 2. `schema_version` decision

`schema_version` **STAYS 1.** Current value is 1 (`validation.rs:17` /
`validation.rs:117`: absent accepted as legacy with warning); no migration
event has ever occurred. The shorthand is **additive + backward-compatible**:
every config valid today remains valid; bare strings are additionally accepted.
No deployed/tested configs exist to migrate (pre-release), so no migration
machinery is needed.

**Corollary (project versioning rule of thumb):** the first **BREAKING** change
(if any) would be the schema v2 event. Additive serde-only shorthands do not
bump the version.

---

## 3. Non-goals

- `env` entries unchanged (they have 3 fields and genuinely need tables).
- No map idiom (see the `depends_on` contrast in §1.4).
- No forced config migration — the old table form keeps parsing indefinitely.
- No per-entry fields added now.

---

## 4. Acceptance criteria

**Executed 2026-07-31:** all criteria met — 8 new parse tests in `types.rs` +
1 typo regression test in `validation.rs`; full suite 492 tests green; golden
plans byte-unchanged; schema regen committed (`anyOf`); merge/provenance tests
re-run unchanged.

- Parse tests: bare-only, mixed, table-only, empty array.
- Typo'd bare name → validation error citing the missing secret name
  (`validation.rs:318-326`).
- Merge + provenance behavior byte-identical (run the
  `merge.rs:1066-1156` tests).
- Schema regen committed with the `anyOf` (not `oneOf`) documented (`just schema-check` /
  schema_drift passes).
- Golden plans **BYTE-UNCHANGED** (rendering unaffected; `just golden-check`).
- Optional follow-up: rewrite the personal config + `config.reference`
  `secret_env` blocks to shorthand (cosmetic; config-repo edit).

---

## 5. ADR note

Additive to [ADR 0002](../../migration/50-decisions/0002-toml-config-format.md)
(TOML config format) + [ADR 0003](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md)
(config purity / closed vocabulary) — no vocabulary expansion, same data, no
new ADR required; cross-reference only (this spec does **not** propose
recording anything in ADR 0003).

---

## 6. Effort & gate

| Item | Value |
|---|---|
| Effort | **S** (~1 enum + `deserialize_with` + tests; schema regen). |
| Gate | Cargo-linked gates runnable in-container via `nix develop` (`cargo test`, `just golden-check`, `just schema-check`). `verifiable-here`. |
| Files touched | `control/agentctl/src/config/types.rs` (the helper enum + `deserialize_secret_env` normalization + parse tests); `control/agentctl/src/config/validation.rs` (shorthand-typo regression test); `control/agentctl/src/merge.rs` tests (unchanged, re-run); `control/agentctl/tests/spec_examples_parse.rs` (promoted to real lib types — mirror deleted); `schemas/workestrate.schema.json` regen (`anyOf`). |
| Risk | Low. The change is serde-boundary-only; every downstream code path sees the identical post-parse struct. |
