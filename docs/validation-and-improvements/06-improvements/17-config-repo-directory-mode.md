# 17 — Config repo directory mode (workestrate/ + workloads/ capsules)

> **STATUS: READY-TO-EXECUTE (design)**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [15-toml-toolchain-tombi.md](15-toml-toolchain-tombi.md) ·
> [16-unified-secret-env-model.md](16-unified-secret-env-model.md) ·
> [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md) ·
> [11-home-provisioning-and-lockfile.md](11-home-provisioning-and-lockfile.md) ·
> [../../migration/50-decisions/0002-toml-config-format.md](../../migration/50-decisions/0002-toml-config-format.md) ·
> [../../migration/50-decisions/0003-config-purity-closed-vocabulary.md](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md) ·
> [../../migration/50-decisions/0022-config-repo-scaffolding.md](../../migration/50-decisions/0022-config-repo-scaffolding.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md) ·
> [../../migration/50-decisions/0024-dotfiles-home-and-working-copy-config-repos.md](../../migration/50-decisions/0024-dotfiles-home-and-working-copy-config-repos.md)

This document specifies **directory mode** for config repos: an alternative to
the single-file `workestrate.toml` in which the config repo is a `workestrate/`
directory of cross-cutting files plus a `workloads/` tree of per-workload files
or **capsule directories** that colocate each workload's definition with its
app-native artifacts. Directory mode is a **layout change only** — the workload
schema, the merge semantics, and the final secret/env model of
[16-unified-secret-env-model.md](16-unified-secret-env-model.md) are unchanged.

### Environment markers

- `verifiable-here` — the loader work and all its gates (cargo `check`/`test`/
  `golden-check`/`schema-check`) are runnable in-container via `nix develop`
  (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on
  PATH; prefix with `export
  PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then
  `nix develop -c bash -c '<cmd>'` — the store-path nix prefix convention used
  by the sibling specs). No KVM is needed for the loader or the personal-config
  restructure proof.
- `HOST-KVM` — only the runtime smoke of a restructured config (guest boot
  against capsule-mounted artifacts) requires the host batch.

---

## 1. Structure

A config repo uses **either** the single `workestrate.toml` at its root (the
current form, unchanged) **or** directory mode. Directory mode layout:

```
<config-repo>/
├── workestrate/
│   ├── default.toml            # schema_version + cross-cutting config
│   ├── secrets.toml            # OPTIONAL — the [secrets.*] catalog only
│   └── workloads/
│       ├── <name>.toml         # flat single-workload file, OR
│       └── <name>/             # workload CAPSULE directory
│           ├── workload.toml   # the capsule entry file (the workload def)
│           ├── config.yaml     # app-native artifacts, colocated:
│           ├── models.yaml     #   litellm: config.yaml, models.yaml
│           ├── opencode.jsonc  #   opencode: opencode.jsonc
│           ├── settings.json   #   odysseus: settings.json
│           ├── models.json     #   pi: models.json
│           ├── <seed files>    #   seed_files payloads, colocated
│           └── flake.nix       #   image-build flake for the workload
└── … (repo plumbing: tombi.toml, schemas/, .sops.yaml, .env.enc, README.md)
```

- **`workestrate/default.toml`** — `schema_version` plus everything
  cross-cutting: contexts, registry-level policy, shared mounts, defaults —
  i.e. every field that is not a per-workload table or the secrets catalog.
- **`workestrate/secrets.toml`** (optional) — the `[secrets.*]` catalog only,
  per the final secret/env model of spec 16.
- **`workestrate/workloads/`** — holds **either** flat single-workload files
  `workloads/<name>.toml`, **or** workload capsule directories
  `workloads/<name>/` whose `workload.toml` entry file carries the workload
  definition and whose sibling files carry that workload's artifacts under
  their **app-native names** (`config.yaml`, `models.yaml`, `opencode.jsonc`,
  `settings.json`, `models.json`, seed files, `flake.nix`).
- **Mount/seed/local_build paths stay repo-relative strings.** They simply
  point into the capsule dir (e.g.
  `mounts = [{ host = "workestrate/workloads/litellm/config.yaml", … }]`).
  **Zero schema change** to the workload schema — directory mode is purely a
  loader + layout concern.

---

## 2. Rules (hard semantics)

### 2.1 Either/or vs `workestrate.toml`

If both `workestrate.toml` and `workestrate/` exist in the same config repo →
**HARD ERROR** naming both paths. No precedence, no merge-across-modes: one
config repo, one mode.

### 2.2 Load order

`default.toml` → `secrets.toml` → `workloads/` entries. Workload entries are
loaded in **lexicographic order**, flat files and capsule directories
interleaved, sorted by entry name (`litellm.toml` and `litellm/` compare as
equals on the workload name — which the duplicate rule §2.4 then rejects).

### 2.3 Filename/dirname-implied workload names

A **bare table** (workload fields at top level, no `[workloads.<name>]`
wrapper) in `workloads/<name>.toml` or `workloads/<name>/workload.toml`
**implies the workload name from the filename/dirname**. The full
`[workloads.<name>]` table form remains allowed in any `workloads/` file for
flexibility — including multi-workload files — but **one-thing-per-file is the
recommended posture**.

### 2.4 Duplicate workload names

The same workload name defined across more than one file/dir → **HARD ERROR**
naming both provenance paths (e.g. `workloads/litellm.toml` vs
`workloads/litellm/workload.toml`). Within-file duplicates stay the existing
TOML parse error.

### 2.5 Provenance: per-FILE granularity

Provenance strings carry the repo-relative path: **`<repo>#<relpath>`** — e.g.
`personal#workestrate/workloads/litellm/workload.toml` — recorded **per
field**. This is strictly better than layer-level provenance (`personal`)
because it tells you **WHICH file set the field**, and it composes with the
existing `--show-source` provenance surface unchanged in shape.

### 2.6 tombi per-file schema validation

Every directory-mode file is schema-validated **individually**. The scaffolded
config-repo template `control/agentctl/src/scaffold/template/tombi.toml.tpl`
(currently `include = ["workestrate.toml", "overrides.toml"]` in its
`[[schemas]]` block) must gain the directory-mode glob:

```toml
include = ["workestrate.toml", "overrides.toml", "workestrate/**/*.toml"]
```

**QUEUED CODE CHANGE — recorded, not landed in this docs batch.** The `.tpl`
edit, the matching copier-template static copy, and the schema-drift guard
extension per [15-toml-toolchain-tombi.md](15-toml-toolchain-tombi.md) §3/§5
land with the loader implementation.

### 2.7 `schema_version` authority

`schema_version` is required **only** in `default.toml`. `secrets.toml` and
`workloads/` files must **NOT** repeat it; presence elsewhere → **HARD ERROR**
(chosen over a warning: a single authority for the version — a stray
`schema_version` in a workload file is always an authoring mistake, never
meaningful, so failing closed costs nothing and removes an ambiguity class).

---

## 3. Home files decision (decided)

- **The registry `config.toml` STAYS single-file.** It is machine-managed
  under `RegistryLock` (see
  [11-home-provisioning-and-lockfile.md](11-home-provisioning-and-lockfile.md));
  multi-file machine writes are complexity without benefit.
- **`workestrate.lock` is generated** → single-file forever. Never hand-edited,
  never split.
- **`overrides.d/` is DEFERRED** until user-global overrides actually grow past
  what a single `overrides.toml` comfortably carries. Not part of this spec.

---

## 4. Rationale

- **Precedent.** docker-compose multi-file/`extends`; kustomize
  bases+overlays; nix modules (`default.nix` entry point, one-thing-per-file).
  The pattern — a directory with a fixed entry point and lexicographic merge —
  is the industry default once a single file outgrows head-sized.
- **Capsules kill the split trees.** The current personal repo splits one
  logical workload across `agents/<wl>/config/*` and `infra/litellm/*`; a
  capsule colocates a workload's dependencies with its definition, so adding /
  moving / deleting a workload is a single-directory operation.
- **The config repo becomes a self-contained deployment unit.** Clone the repo,
  and every artifact every workload needs is inside it, addressed by
  repo-relative paths — the working-copy model of
  [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md)
  carried to its conclusion.

---

## 5. Migration: personal config → directory mode

Current layout (proof-case repo `.tmp/config-repos/personal-v2` @ `99c9985`):
`workestrate.toml` (309 lines, `schema_version = 2`,
`[secrets.LITELLM_MASTER_KEY]` etc. at lines 1–32+),
`agents/{odysseus,opencode,pi,tempest}/config/*` (`settings.json`,
`opencode.jsonc`, `models.json`), and
`infra/litellm/{config.yaml,models.yaml,README.md}`.

Mapping:

| From (current personal-v2) | To (directory mode) |
|---|---|
| `workestrate.toml` cross-cutting (contexts, defaults, `schema_version = 2`) | `workestrate/default.toml` |
| `[secrets.*]` catalog | `workestrate/secrets.toml` |
| Per-workload tables (`[workloads.litellm]` …) | `workestrate/workloads/<name>/workload.toml` (capsule form) |
| `agents/<wl>/config/*` artifacts | `workestrate/workloads/<wl>/` under app-native names (`settings.json`, `opencode.jsonc`, `models.json`) |
| `infra/litellm/{config.yaml,models.yaml}` | `workestrate/workloads/litellm/{config.yaml,models.yaml}` |
| Mount/seed paths referencing `agents/…` / `infra/…` | Re-pointed into the capsule — repo-relative strings, **no schema change** |

The `agents/` and `infra/` trees collapse into capsules; the restructured repo
must load to a **byte-identical merged config** (acceptance §6).

---

## 6. Acceptance criteria

- [ ] Both-modes hard error: a repo containing both `workestrate.toml` and
      `workestrate/` fails naming both paths (test-covered).
- [ ] Duplicate workload name across files/dirs fails naming both provenance
      paths (test-covered).
- [ ] `schema_version` outside `default.toml` fails as a hard error
      (test-covered).
- [ ] Filename/dirname-implied names: a bare table in
      `workloads/<name>.toml` and in `workloads/<name>/workload.toml` loads as
      workload `<name>`; the `[workloads.<name>]` wrapper form still loads
      (test-covered, both forms).
- [ ] The restructured personal config repo loads to a **byte-identical merged
      config** vs the single-file form (golden diff).
- [ ] Provenance strings carry the relpath form `<repo>#<relpath>` per field
      (test-covered; `--show-source` output shape unchanged).
- [ ] tombi validates each directory-mode file individually via the
      `workestrate/**/*.toml` include glob (the queued `tombi.toml.tpl` edit,
      §2.6, landed with scaffold + copier parity per spec 15 §5.2).
- [ ] Golden plans **byte-unchanged** across the restructure (no plan-surface
      drift).

---

## 7. Effort & gate

| Item | Value |
|---|---|
| Effort | **M** (loader: mode detection + lexicographic multi-file load + implied names + provenance relpath strings; scaffold `tombi.toml.tpl` glob + copier parity; personal restructure proof case; hard-error test matrix). |
| Gate | `verifiable-here` via `nix develop` for all loader/cargo gates (`check`, `test`, `golden-check`, `schema-check`); tombi per-file validation via `just tombi-check`; runtime smoke of a restructured repo is `HOST-KVM` (folds into the host batch). |
| Files touched | Loader (`control/agentctl/src/config/loading.rs` + `types.rs` provenance); `control/agentctl/src/scaffold/template/tombi.toml.tpl` (queued include-glob edit, §2.6) + copier static copy + parity test; `schemas/` (no change — layout-only); `.tmp/config-repos/personal-v2` (restructure proof case). |
| Risk | Low–moderate. Additive mode; single-file repos are untouched by construction (§2.1 keeps the modes disjoint). The main risk is provenance-string churn, contained by the per-field relpath rule and golden-plan byte-stability. |
