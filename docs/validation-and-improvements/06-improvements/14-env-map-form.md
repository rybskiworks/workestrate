# 14 — Config ergonomics: map form for `env` entries (formatting collapse)

> **STATUS: EXECUTED (2026-07-31; implementation commit 1035073, spec doc 564deca)**
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [13-secret-env-shorthand.md](13-secret-env-shorthand.md) ·
> [../../migration/50-decisions/0002-toml-config-format.md](../../migration/50-decisions/0002-toml-config-format.md) ·
> [../../migration/50-decisions/0003-config-purity-closed-vocabulary.md](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md)

This document specifies a standalone config-ergonomics improvement: allow
`env` entries to be written as a **nested map** (JSON-akin syntax) in addition
to the existing array-of-tables form. The change is a FORMATTING collapse —
no new schema concepts: the parsed data model, merge, validation, and
plan-build code paths are all untouched.

### Environment markers

- `verifiable-here` — all gates for this spec are cargo-linked gates runnable
  inside the container via `nix develop` (nix at
  `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH;
  prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
  then `nix develop -c bash -c '<cmd>'`). No KVM is needed.

---

## 1. Design

`env` accepts **EITHER** the current array-of-tables **OR** a map:

```toml
[workloads.litellm.env]
LITELLM_LOG = "INFO"
OPENAI_API_KEY = { secret = "OPENAI_API_KEY" }
```

Both forms are normalized at parse time to `Vec<EnvVarConfig>` via a custom
`deserialize_env` Visitor:

- `visit_seq` — the current path, **unchanged** (array-of-tables parses
  exactly as today);
- `visit_map` — the new path, pushing entries in **DOCUMENT ORDER** (see §2).

### 1.1 Why the map form fits

- `EnvVarConfig` (`control/agentctl/src/config/types.rs:68-74`) has exactly
  three fields — `name`, `value?`, `secret?` — with `deny_unknown_fields`.
  The XOR value/secret constraint is enforced at plan build
  (`control/agentctl/src/microsandbox/workload/secrets.rs:78-109`); neither
  set renders as the literal `""`. The name regex
  `^[A-Za-z_][A-Za-z0-9_]*$` is enforced at
  `control/agentctl/src/config/validation.rs:293-316`.
- In the map form the **KEY** carries the name, so only 2 of the 3 fields ever
  remain in the value — and the census confirms this is the real usage: the
  personal config has **20 `env` blocks across 5 workloads** (16 literal, 4
  secret), and only 2 of 3 fields are ever used per entry.
- litellm (`workestrate.toml:58-70`): 3 blocks → collapses to a 4-line
  `[workloads.litellm.env]` table.
- Remap cases (`TEMPEST_LOCAL_API_KEY` ← `LITELLM_MASTER_KEY`) require
  `{ secret = "..." }` inline-table values — bare strings cannot remap. Both
  value shapes coexist inside one map (see the helper in §1.2).

### 1.2 Implementation shape

A dedicated helper used **ONLY** at the serde + schemars boundary, normalized
to `Vec<EnvVarConfig>` at deserialize time:

```rust
// sketch — serde/schemars boundary only; never stored
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
enum EnvValueShorthand {
    Bare(String),               // LITELLM_LOG = "INFO"
    Full(EnvSecretValue),       // OPENAI_API_KEY = { secret = "NAME" }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EnvSecretValue {
    secret: String,
}
// normalized to EnvVarConfig { name: <key>, value/secret } at deserialize time
```

Do **NOT** reuse `EnvVarConfig` as the map value — it would allow `name`
inside the value, duplicating the key. The field type
(`Vec<EnvVarConfig>`), merge, validation, and plan build are all
**untouched** — they see the identical post-parse struct they see today.
Precedent: spec 13 ([13-secret-env-shorthand.md](13-secret-env-shorthand.md))
— untagged helper + `deserialize_with` normalization + `anyOf` in the schema
(`#[schemars(with)]` → `anyOf` [array, object]). External precedent:
docker-compose `environment:` supports map OR sequence.

### 1.3 Before / after (litellm workload, workestrate.toml:58-70)

Before — 9 lines of three `[[workloads.litellm.env]]` tables:

```toml
[[workloads.litellm.env]]
name = "LITELLM_LOG"
value = "INFO"

[[workloads.litellm.env]]
name = "OPENAI_API_KEY"
secret = "OPENAI_API_KEY"

# … one more identical-shape table …
```

After — a 4-line map:

```toml
[workloads.litellm.env]
LITELLM_LOG = "INFO"
OPENAI_API_KEY = { secret = "OPENAI_API_KEY" }
# …
```

---

## 2. ORDER (the gating constraint)

`plan.rs:216` renders `env` in Vec order; fixtures declare **non-alphabetical**
order and tempest **interleaves literal/secret** entries — sorting would
CHANGE rendered output. The map form must therefore preserve document order.

Verified evidence: toml 0.8.23's serde path goes through `toml_edit::de`,
which builds on `indexmap` — **unconditionally ordered**. A custom `visit_map`
therefore collects entries in **DOCUMENT ORDER** with no `preserve_order`
feature and no new dependency.

**Rule: never sort, never reorder.** `visit_map` pushes entries in document
order, and no downstream code may sort the normalized Vec.

---

## 3. Strictness

- **Duplicate keys** in the map form become TOML **parse errors** — stricter
  than the array form, and desirable. Verified: zero duplicate env names exist
  in any config today, so nothing relies on dups.
- **Mixing forms for one workload** (`[[workloads.x.env]]` AND
  `[workloads.x.env]` in the same file) is a TOML **redefinition parse
  error** — the parser rejects it before serde ever runs. Good; it is
  documented here so users are not surprised.
- **Name validation unchanged:** the name regex
  (`validation.rs:293-316`) applies to map keys **post-normalization**.
  Dashed names would need quoted keys and STILL fail validation — behavior
  unchanged.

---

## 4. `schema_version` decision

`schema_version` **STAYS 1.** The map form is additive and backward-compatible:
the old array-of-tables form parses indefinitely — the serde seq path is
unchanged. Per the project versioning rule of thumb (spec 13 §2), additive
serde-only shorthands do not bump the version; the first BREAKING change, if
any, would be the schema v2 event.

---

## 5. Non-goals

- **No shared-blocks / `use` mechanism** — explicitly REJECTED by the user;
  out of scope.
- No change to merge (`merge.rs:344-362`), validation, or plan code — all
  operate post-parse on the typed `Vec<EnvVarConfig>`; the merge's raw key
  check `contains_key("env")` is shape-agnostic.
- `mounts` / `egress` / `ports` stay arrays. The JSON-akin answer for those
  ALREADY WORKS TODAY with zero code: inline-array notation
  `mounts = [{...}]` is valid TOML against the existing seq deserialization —
  document only.
- `Serialize` emits the long form (array-of-tables) — accepted asymmetry,
  same as spec 13.

---

## 6. Acceptance criteria

**Executed 2026-07-31:** all criteria met — 10 new tests in
`types.rs`/`validation.rs` (incl. document-order assertion with
non-alphabetical keys ZEBRA/MIDDLE/ALPHA, duplicate-key parse error,
`[[env]]`+`[env]` redefinition parse error, and the typo regression asserting
the exact error `workload 'pi' env references undefined secret
'GITHUB_TOKEN_TYPO'`); full suite 498 tests green; golden plans byte-unchanged;
schema regen committed (`anyOf` [array, object]); `config.reference`
example-service converted (golden-equivalent).

- Parse tests: bare values; inline-table `{ secret }` values; mixed
  literal+secret in one map; an **ORDER** test with ≥3 **non-alphabetical**
  keys asserting document order is preserved; duplicate key → TOML parse
  error; `[[env]]` + `[env]` mix → redefinition parse error; typo'd secret
  name → validation error naming the missing secret.
- Merge + provenance behavior **byte-identical** (run the
  `merge.rs:344-362` tests unchanged).
- Schema regen committed with the `anyOf` ([array, object]) documented
  (`just schema-check` / schema_drift passes).
- Golden plans **BYTE-UNCHANGED** (`just golden-check`).
- `config.reference` one workload converted to the map form as golden proof.
- Follow-up (config-repo edit, NOT this repo): convert the personal config's
  20 env blocks.

---

## 7. ADR note

Additive to [ADR 0002](../../migration/50-decisions/0002-toml-config-format.md)
(TOML config format) + [ADR 0003](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md)
(config purity / closed vocabulary) — no vocabulary expansion, same data, no
new ADR required; cross-reference only. **Supersedes spec 13 §3's "`env`
genuinely needs tables" non-goal:** the census evidence (only 2 of 3 fields
are ever used per entry, and the map key carries the name) shows `env` does
NOT genuinely need tables — see §1.1.

---

## 8. Effort & gate

| Item | Value |
|---|---|
| Effort | **S** (same shape as spec 13's implementation: one helper + `deserialize_env` visitor + tests; schema regen). |
| Gate | Cargo-linked gates runnable in-container via `nix develop` (`cargo test`, `just golden-check`, `just schema-check`). `verifiable-here`. |
| Files touched | `control/agentctl/src/config/types.rs` (the helper + `deserialize_env` visitor + parse tests); `control/agentctl/src/config/validation.rs` (typo regression test); `control/agentctl/src/config/merge.rs` tests (unchanged, re-run); `schemas/workestrate.schema.json` regen (`anyOf`); `config.reference` one workload converted. |
| Risk | Low. Serde-boundary-only; every downstream code path sees the identical post-parse `Vec<EnvVarConfig>`. |
