# 20 — Schema evolution policy + config migration tooling (post-launch)

> **STATUS: SPEC (design; not yet implemented)**
> Prerequisites / see-also: [../README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [15-toml-toolchain-tombi.md](15-toml-toolchain-tombi.md) ·
> [16-unified-secret-env-model.md](16-unified-secret-env-model.md) ·
> [../../migration/50-decisions/0002-toml-config-format.md](../../migration/50-decisions/0002-toml-config-format.md) ·
> [../../migration/50-decisions/0025-home-provisioning-and-lockfile.md](../../migration/50-decisions/0025-home-provisioning-and-lockfile.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs-only (`verifiable-here`); the cargo gates it specifies
(schema/migration tests) run via `nix develop`.

---

## Summary

This spec pins two things for the post-launch life of the workestrate config
schema: **(1) a version policy** — what earns a `schema_version` major bump
versus what stays additive under ADR 0021 §8 — and **(2) the tooling** that
carries a bump when one eventually happens: a schema pull command that
refreshes the vendored `schemas/workestrate.schema.json` in a config repo from
the installed binary (with provenance locked and fail-closed downgrade
protection), and a `workestrate config migrate` framework that applies
versioned, pure-function TOML-document transforms with guided-migration
dry-run output in the style of the v1-shim deprecation warnings. Today the
machinery around the schema is freshness-only: the schema is generated from
the Rust types via schemars (`workestrate generate-schema`,
`control/agentctl/src/commands/diagnostics.rs:516`), vendored into config
repos at scaffold time, validated by tombi via a relative `#:schema`
directive, and guarded by the `schema_drift.rs` byte-parity test. Nothing yet
defines when `schema_version` may move or how a user config moves with it —
this spec does, before the first post-launch breaking change forces the
question ad hoc. Pre-launch, the policy is inert: `schema_version` stays `1`,
absorbed breaking changes retract rather than bump (the spec-16 collapse
precedent). Effort: **M** overall (M on the migrate engine, S on pull+lock).

---

## 1. Version policy

`EXPECTED_SCHEMA_VERSION` (`control/agentctl/src/config/validation.rs:24`) is
`1` and **stays `1` for the entire pre-launch period**. An absent
`schema_version` warns and is treated as `1`
(`control/agentctl/src/config/types.rs:572` comment area). The FIRST breaking
change that ships to users outside the dev loop earns `2`. Additive changes
never bump.

| Class | Definition | Version effect |
|---|---|---|
| BREAKING | A field is REMOVED | bumps major |
| BREAKING | Semantics of an existing field CHANGE (same key, different meaning/behavior) | bumps major |
| BREAKING | Validation on existing content becomes STRICTER (previously-valid configs start failing) | bumps major |
| ADDITIVE | New OPTIONAL field with `#[serde(default)]` (ADR 0021 §8) | no bump |
| ADDITIVE | New sugar/shorthand forms that desugar to existing structure (precedent: specs 13/14 forms, spec 16 `true` sugar) | no bump |
| ADDITIVE | New enum variants in open vocabularies IF the closed-vocabulary rules (ADR 0003) permit | no bump |
| ADDITIVE | Loosening validation (previously-invalid content becomes valid) | no bump |

**Pre-launch collapse precedent.** Spec 16 established that pre-release,
breaking changes are ABSORBED and the version retracts rather than bumps: the
intermediate v2 model (commit `1ed2e6d`) never shipped to production, was
retracted pre-release, and `schema_version` collapsed back to `1` — there is
no production migration to preserve. This policy (bump on breaking) only
engages once a config has shipped to users outside the dev loop; until then
the collapse pattern remains the correct move and this spec's migration
machinery is built but unused.

**Key decision:** the bump trigger is user exposure, not the change itself —
a breaking change with zero external configs in the wild is absorbed, not
versioned.

---

## 2. Schema pull + lock

### 2.1 Command name (TBD — two candidate spellings)

| Candidate | Shape |
|---|---|
| (a) `workestrate schema pull` | new top-level `schema` verb family |
| (b) `workestrate config update-schema` | subcommand of the existing `config` verb family |

**Tradeoff.** `schema pull` is shorter and mirrors the `git pull` refresh
idiom, but it opens a new top-level verb family for exactly one command and
severs the operation from the surface that owns config-repo lifecycle
(`config add` / `config update` / `config remove` — the same family whose
commands already write `workestrate.lock`). `config update-schema` groups
the refresh with the verbs that already mutate config repos and the home
lockfile, at the cost of a longer spelling. **Recommendation: (b)
`workestrate config update-schema`** — verb-family cohesion with the
lockfile-writing config commands outweighs the shorter spelling. (Recorded as
an open decision in §7 until implementation.)

### 2.2 Behavior

`workestrate config update-schema` refreshes the config repo's vendored
`schemas/workestrate.schema.json` from the **installed tool's** generated
schema — i.e. the stdout of the same `generate-schema` machinery
(`diagnostics.rs:516`) that the `schema_drift.rs` freshness guard diffs
against the committed copy. There is **no network fetch**: the schema always
comes from the installed binary, so the vendored schema can never describe a
schema newer (or older) than the tool that will consume it (non-goal §6).

### 2.3 Provenance lock (two mechanisms, one recommendation)

The pull records provenance — source tool version, generation date, and
`schema_version` — via two complementary mechanisms:

- **(a) Header comment in the schema JSON.** A `#:`-style / `"$comment"`
  header block at the top of the vendored schema file recording
  `generated-by = "workestrate <tool-version>"`, `generated-at = <date>`,
  `schema_version = <n>`. Human-visible; travels with the file; survives a
  config repo being read outside a home.
- **(b) Sidecar entry in the home `workestrate.lock`.** A new OPTIONAL
  `schema` provenance section on the `HomeLock` typed serde struct
  (`control/agentctl/src/config/lockfile.rs`), added as an additive
  `#[serde(default)]` field per ADR 0021 §8, written through the existing
  atomic tmp+rename write pattern already used for
  `LOCK_FILE_NAME = "workestrate.lock"`. Machine-checked; this is the entry
  the downgrade protection (§2.4) reads.

**Recommendation: both.** The lock sidecar is authoritative for the
fail-closed check; the header comment is for humans and for the
config-repo-side view when the lock is not in reach. Neither alone covers
both audiences.

### 2.4 Fail-closed downgrade protection

If the installed tool's `EXPECTED_SCHEMA_VERSION` is **LOWER** than the
version recorded in the lock, the pull REFUSES with an error naming both
versions, e.g.:

```
error: refusing to refresh vendored schema with an older tool
  installed tool EXPECTED_SCHEMA_VERSION = 1
  recorded in workestrate.lock          = 2
upgrade the workestrate binary before pulling the schema
```

This prevents an older binary from silently clobbering a newer vendored
schema — the failure mode where a user keeps two tool versions around and the
stale one rewrites the schema its successor generated.

---

## 3. Migration framework

### 3.1 Command

```
workestrate config migrate [--dry-run] [--to <version>]
```

### 3.2 Registry of versioned steps

Migrations are a **registry of versioned steps**: each step is a `v_n →
v_n+1` transform implemented as a **pure function on the parsed TOML
document** — a `toml_edit` document tree, preserving comments and formatting
where feasible. Steps are **NOT string edits and NOT regex**: they operate on
the document structure, so a step that renames a field renames the parsed
key, not every byte sequence that resembles it.

Steps apply **LINEARLY** in v1 of the framework: a v1→v2 chain exists only
when version 2 exists, and the framework never half-applies a multi-version
chain (non-goal §6). `--to <version>` pins the target; the default is the
tool's `EXPECTED_SCHEMA_VERSION`.

### 3.3 Guided-migration dry run

`--dry-run` prints the EXACT per-entry changes, guided-migration style,
following the v1-shim (`fold_legacy_secret_model`) deprecation-warning
pattern: name the entry, the old form, the new form, per occurrence. Example
transcript of a synthetic v1→v2 dry run (toy step: rename the workload field
`timeout` → `timeout_secs`):

```
$ workestrate config migrate --dry-run
config repo: personal ($WORKESTRATE_HOME/config-repos/personal)
declared schema_version: 1 → target: 2

step v1→v2: rename workload field `timeout` → `timeout_secs`
  workestrate.toml:87   [workloads.pi]       timeout = 30  →  timeout_secs = 30
  workestrate.toml:142  [workloads.litellm]  timeout = 60  →  timeout_secs = 60

2 entries would change across 1 file.
re-run without --dry-run to apply; a backup (workestrate.toml.bak) is written first.
hint: your config repo is a git working copy — review the diff and commit it yourself.
```

### 3.4 Invariants

- **IDEMPOTENT.** Running `migrate` twice is a no-op the second time: each
  step detects already-migrated entries (the new form present, the old form
  absent) and reports nothing to do.
- **REFUSES newer configs.** If the config's declared `schema_version` is
  GREATER than the tool's `EXPECTED_SCHEMA_VERSION`, migrate refuses — you
  are running an older tool against a newer config — with an error naming
  both versions (same shape as §2.4).
- **BACKUP before writing.** Before any write, the command saves
  `workestrate.toml.bak` (or `workestrate/default.toml.bak` in directory
  mode, where `schema_version` is authoritative in `default.toml` only).
  Config repos are git working copies (spec 10), so git is the second safety
  net — and the command **warns if the working tree is dirty** before
  applying, so the migration diff is never tangled with unrelated uncommitted
  edits.

---

## 4. Relationship to existing machinery

### 4.1 `schema_drift.rs` (freshness guard)

`control/agentctl/tests/schema_drift.rs` diffs `generate-schema` stdout
against the committed `schemas/workestrate.schema.json` (byte-parity after
bootstrap) — it guards the **repo-side** committed schema against Rust-type
drift. The pull command (§2) is its **config-repo-side counterpart**: drift
keeps the tool's committed schema fresh against its own types; pull keeps
each config repo's vendored copy fresh against the installed tool. Neither
replaces the other.

### 4.2 tombi validation (post-migration re-lint)

After migrate writes, the touched files must be re-validated: migrate either
runs tombi format+lint semantics itself or instructs the config-repo
pre-commit hook (spec 15 machinery — `tombi.toml`,
`templates/workestrate-config/schemas/`, the `TOMBI_REQUIRED` version guard)
to catch it on the user's commit. Whether migrate invokes tombi as a
subprocess or defers to the hook is an open decision (§7); either way, a
migrated file that fails tombi schema validation is a migrate bug, not a user
error.

### 4.3 Config repos as working copies

Migrations run against the config repo **working copy** at
`$WORKESTRATE_HOME/config-repos/<name>/`. The **user commits** — the tool
never auto-commits (non-goal §6). This matches the spec-10 model: the remote
is canonical, the working copy is the user's, and the tool's writes (pull,
migrate) are staged for the user to review with ordinary `git diff` /
`git commit`.

### 4.4 Home lockfile

The schema-provenance entry from §2.3(b) lives on `HomeLock` in
`workestrate.lock` — the same typed serde struct and atomic tmp+rename write
pattern that `config add`/`update`/`remove` and `home init` already use (ADR
0025). The lock remains the single pin mechanism; the schema section is
additive to it, not a second file.

---

## 5. Acceptance criteria + effort

- [ ] Version policy documented (§1): breaking vs additive table; pre-launch
      collapse precedent; `schema_version` stays `1` until the first shipped
      breaking change.
- [ ] Pull command spec'd (§2): refreshes vendored schema from the installed
      binary only; provenance locked (header comment + lock sidecar);
      fail-closed downgrade protection naming both versions.
- [ ] Migrate command spec'd (§3): registry of pure-function toml_edit
      document steps; linear application; `--dry-run` guided-migration output
      per the v1-shim pattern; idempotent; refuses newer configs; backup +
      dirty-tree warning; `--to` pins the target.
- [ ] **Test plan:** a synthetic v1→v2 step (rename a toy field on a fixture
      config) exercised end-to-end:
  - [ ] dry-run output asserted (per-entry old→new lines);
  - [ ] apply asserted byte-wise against an expected post-migration fixture;
  - [ ] second run asserted no-op (idempotency);
  - [ ] downgrade-refusal asserted (lock records 2, tool EXPECTED is 1 →
        error naming both);
  - [ ] backup file asserted (`workestrate.toml.bak` written before apply).

**Effort: M overall** — M on the migrate engine (registry + toml_edit
transforms + dry-run renderer + idempotency), S on pull+lock (one
file-refresh command plus an additive serde-default lock section).

---

## 6. Non-goals

- **No runtime auto-migration on load.** Migration is an explicit command
  only; the loader keeps today's behavior — it hard-errors/warns per
  `EXPECTED_SCHEMA_VERSION` and never rewrites user config on the read path.
- **No network fetching of schemas from remotes.** The schema always comes
  from the installed binary (§2.2); there is no URL, registry, or release
  artifact to pull from.
- **No half-applied multi-version chains in v1.** Steps apply linearly; the
  framework does not pause mid-chain, and a multi-hop migration (v1→v3)
  applies each step in order to completion or not at all.
- **No auto-commit to the config repo.** The tool writes the working copy;
  the user reviews and commits (§4.3).

---

## 7. Open decisions

1. **Command name** (§2.1): `workestrate schema pull` vs `workestrate config
   update-schema`. Recommendation: `config update-schema` for verb-family
   cohesion with the lockfile-writing config commands; not yet locked.
2. **Provenance carrier** (§2.3): header comment in the schema JSON vs lock
   sidecar vs both. Recommendation: both — lock sidecar authoritative for the
   fail-closed check, header comment for humans; not yet locked.
3. **tombi re-lint mechanism** (§4.2): migrate invokes tombi as a subprocess
   after writing, vs leaving re-lint to the config-repo pre-commit hook.
   Tradeoff: subprocess catches a broken migration immediately but adds a
   tombi-availability dependency to the migrate path; the hook defers the
   signal to commit time. Not yet decided.
