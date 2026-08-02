# STATUS — comprehensive state report for the next session

> **STATUS: CURRENT (2026-08-01, HEAD `7624aaf`)**
> Prerequisites / see-also: [README.md](README.md) · [NEXT-SESSION.md](NEXT-SESSION.md) ·
> [07-execution-order.md](07-execution-order.md) · [06-improvements/00-index.md](06-improvements/00-index.md) ·
> [02-config-requirements.md](02-config-requirements.md)

This file is the self-contained status report for a contextless session picking
up the `migration/tool-model` branch. It consolidates the two 2026-08-01 audits
(the per-spec status audit and the ADR/doc-currency audit) with the execution
docs. Where this file conflicts with an older doc, **this file wins for state**;
the cited docs win for semantics and procedures. Read this file first; then pick
work from §5.

---

## 0. LATEST LANDING — cleanup PHASE 4 (2026-08-02, lands as one commit on top of phase 3)

**Phase 4 of the approved cleanup: the CLI surface, policy layer, scaffold,
and devshell are generic — the last hardcoded personal names leave the tool's
non-test code paths.** Uncommitted in the working tree at time of writing;
the lead commits it as `refactor(cli): generic CLI surface, policy, and
scaffold (cleanup phase 4)`. What changed and why, for contextless sessions:

- **Typed CLI subcommands removed:** the 5 personal subcommands
  (`litellm`/`pi`/`odysseus`/`opencode`/`tempest`) are deleted from
  `control/agentctl/src/main.rs` along with their dead helper fns — the
  generic `workestrate workload <verb> <name>` path (ADR 0027) is now the
  ONLY lifecycle path. Stale user-facing strings fixed in the same pass: the
  `ps` stale-instance footer and the ADR-0021-pinned refuse message now emit
  `workestrate workload down <name>` (the pinned test updated to match); the
  stale `workestrate down --all` footer now reads `workestrate down-all`.
- **policy.rs de-personalized:** `SECRET_HOST_BINDINGS` (the hardcoded
  personal 7-secret inventory) and `DEFAULT_DENY_FALSE_ENTITLEMENT` (the
  hardcoded `tempest` name) are deleted. Generic replacements: (1) secrets
  with `env_var` may bind only hosts in `ALLOWED_EGRESS_HOSTS` — fail-closed;
  secrets without `env_var` are skipped (preserves the prior gating
  semantics); (2) new config-schema field `workloads.{name}.entitlements`
  with the closed vocabulary `ALLOWED_ENTITLEMENTS = ["default_deny_false"]`
  — `default_deny = false` now requires declaring
  `entitlements = ["default_deny_false"]` in config (config-declared, no core
  names); merge unions entitlements grant-only with provenance, applied
  before the network gate.
- **Scaffold/templates genericized:** `templates/workestrate-config/*` and
  `control/agentctl/src/scaffold/template/*` now emit generic
  `EXAMPLE_API_KEY` + `GITHUB_TOKEN` secrets only. The copier.yml post-copy
  `generate-env-example` task is REMOVED (it re-personalized output from the
  operator's ambient config); `generate-env-example` itself no longer emits
  the stale `AI_WORKBENCH_WORKSPACES_DIR`/`AI_WORKBENCH_VAR_DIR` lines.
- **`litellm_proxy` egress recipe KEPT as generic OSS vocabulary** (verdict
  with evidence: TCP 4000 is litellm's upstream default; the recipe contains
  no personal models/hosts; the personal config repo never references it
  directly, only `agent_base`). The decision is documented in code comments.
- **Devshell containment fix** (`nix/devshells/default.nix`): the shellHook
  now mutates the repo ONLY when the caller's toplevel is the workestrate
  tool checkout (marker probe: `flake.nix` + `control/agentctl/Cargo.toml` +
  `config.reference/workestrate.toml`); running `nix develop` from another
  repo no longer litters vendor symlinks / agents build dirs there. This
  discharges the "phase 4 owns devshell genericization" forward reference
  from phase 3. Banner/echo strings `ai-workbench`→`workestrate` adjacent to
  the edits.
- **Flaky test fix:** the `probe_free_ports_*` assertions now re-probe on
  TOCTOU bind conflict (up to 8 cycles, holding bound listeners during the
  simultaneous-bindability check).
- **`check_required_files` optional entries genericized** to
  `example-{agent,offensive}`.
- **Coordinated personal-repo change** (separate repo
  `workestrate-dev-home/config-repos/personal`, committed by the lead):
  `tempest/workload.toml` declares `entitlements = ["default_deny_false"]`
  (required by the new policy gate above).
- **Validation:** 3 consecutive full `cargo test` runs = 640 passed / 0
  failed / 3 ignored each (matches the phase-3 baseline); `just verify` exit
  0; smoke green — `--help` free of personal subcommands, TempDir `config
  new` emits only generic secrets, `check` green, devshell scratch-cwd
  write-free. All validation ran in-container; no new HOST gates.
- **Closing sweep done — remaining hits classified:** test-fixture vocabulary
  (~600 hits in `#[cfg(test)]` modules + doctests) deferred as a future
  dedicated sweep; `config.reference` example-* vocabulary is by design;
  litellm OSS mentions are acceptable.

---

## 0.1 PREVIOUS LANDING — cleanup PHASE 3 (2026-08-02, lands as one commit on top of phase 2)

**Phase 3 of the approved cleanup: workflow image builds moved out of the
tool repo into the config repo flake.** What changed and why, for
contextless sessions:

- **Before:** the tool flake owned the personal workflow end to end —
  flake inputs for the agent forks (`pi`, `odysseus`, `opencode`,
  `tempest`), package derivations (`nix/packages/{pi,pi-bun,pi-image,
  tempest,tempest-image}.nix`), the `workload-images` attrset, the
  `load-images` package + justfile recipe, the `.#workestrate-sandbox`
  wrappers (`apps.default` pointed at the pi wrapper), the devshell's
  `WORKESTRATE_PI_BUILD` export + `agents/*/repo` shellHook population,
  and the just recipes `dev-build-pi` / `dev-run-pi` / `update-hashes` /
  `store-delta-check` / `load-images`.
- **After:** the tool flake is tool-only. Packages: `workestrate`,
  `microsandbox`, `microsandbox-filesystem-patched`, `msb-wrapped`,
  `decrypt-env`, `write-env`, `setup-secrets`, `tombi` (`default =
  workestrate`); `apps.default` runs the workestrate CLI directly. The
  image-build mechanism is generic and lives in
  `lib.<system>.buildImagesFromConfig` (+ `recipes`, `vocabulary`,
  `config`, `buildWorkloadImage`, `checks` — the whole `lib` is intact;
  the config repo depends on it). The devshell keeps core tooling
  (cargo/rust/node/bun/tombi/msb-wrapped/secrets) and drops the
   agent-repo population and image-loaded check; **devshell genericization
   landed in phase 4** (§0 above).
- **Moved to the personal config repo**
  (`workestrate-dev-home/config-repos/personal`, commit `1000e60` — the
  config-repo flake landed BEFORE this tool-side deletion): the images
  `workestrate-pi:latest` and `tempest:latest` now build there via the
  tool's lib recipes + `buildImagesFromConfig`; one flake input per
  `flake://` source with revs pinned in the config repo's flake.lock;
  nix-only enrichment (pi `binary_name`/`install_dir`/`assets`, tempest
  `npm_deps_hash` HOST-GATE placeholder, pi 4-workspace `build_phase`/
  `install_phase`) attached post-parse; drvPath-eval verified. Its
  justfile owns `update-hashes` + `load-images` now.
- **Also in this commit (workstream A):** the `bun-compile` recipe gained
  `binaryName`/`installDir` params and `buildImagesFromConfig` passes the
  enrichment fields through; `workestrate source clone` resolves
  `flake://` guidance against the DECLARING config layer's repo; the
  5-workload test fixture's litellm workload is sanitized to
  `example-litellm`.
- **Templates synced (both, in lockstep):**
  `templates/workestrate-config/flake.nix.jinja` and
  `control/agentctl/src/scaffold/template/flake.nix.tpl` are now
  byte-identical and document the current reality — sources declaration
  pattern, nix-only enrichment fields, directory-mode assembly pointer,
  HOST-GATE FOD-hash workflow, image name/tag ↔ msb store parity.
- **Mooted by deletion:** the pre-existing `tempest.nix` npmDepsHash
  lambda bug that made `nix flake show` fail at phase-2 HEAD is gone with
  the file (approved decision).
- **Known doc-citation staleness (intentional, NOT fixed here):**
  `.agents/skills/*` still cite the deleted recipes/attrs
  (`just update-hashes`, `store-delta-check`, `load-images`,
  `nix/packages/tempest.nix`, `pi-image` eval examples) as historical
  text; `docs/migration/*`, `docs/nix/*`, and this file's older sections
  likewise. The README's recipe table still lists `just litellm-check` in
  the `just verify` composition (pre-existing phase-2 staleness). Rust
  test references to `${WORKESTRATE_PI_BUILD}` are the LIVE runtime
  env_override mechanism (`Workload::build_path()`), deliberately kept.
- **Gates after landing:** 640 passed / 0 failed / 3 ignored; `just
  verify` fully green (incl. scaffold-check with REAL copier parity, not
  a skip); `nix flake show` evaluates clean with no pi/tempest/
  workload-images attrs; `lib.buildImagesFromConfig` +
  `packages.workestrate.drvPath` eval OK.
- **Host follow-ups (cannot run in this container — no network/KVM):**
  `nix build .#workestrate-pi .#tempest` in the config repo; `just
  update-hashes` there (tempest real npmDepsHash); `just load-images`;
  exercise `workestrate source clone`/`source build` flows; run the 2+1
  ignored KVM tests; verify `msb image ls` name parity
  (workestrate-pi:latest / tempest:latest).

---

## 0.2 PREVIOUS LANDING — cleanup PHASE 2 (2026-08-02, lands as one commit on top of phase 1)

**Phase 2 of the approved cleanup: the tool repo is decoupled from personal
workflow content.** What changed and why, for contextless sessions:

- **Check gates decoupled:** `profiles/*` and
  `infra/microsandbox/sdk-notes.md` removed from the check gates.
- **`config.reference` base layer is now opt-in** via
  `WORKESTRATE_REFERENCE_CONFIG=1` (the spec-05 cwd gate stays unchanged on
  top).
- **Fixture is synthetic example-\*-only:** the real litellm workload in
  `config.reference/workestrate.toml` was replaced with `example-litellm`;
  goldens regenerated.
- **Moved to the personal config repo**
  (`workestrate-dev-home/config-repos/personal`):
  - `profiles/` (litellm.md + agents/{pi,odysseus,opencode,tempest}.md) →
    `docs/profiles/`;
  - `docs/litellm/schemas/` → `docs/litellm/schemas/`;
  - `.agents/skills/validation-litellm-config-check/` → same path in the
    personal repo; the `litellm-check` justfile recipe moved with it
    (personal-repo justfile, args re-pointed at
    `workestrate/workloads/litellm/config.yaml`).
- **Deleted as redundant:** `config.reference/infra/litellm/{config,models}.yaml`
  — `models.yaml` byte-identical to the personal-repo canonical copy;
  `config.yaml` differed by one comment word (the personal copy's
  "workestrator" typo fixed to "workestrate" on merge).
- **Live gate-list citations updated:** `nix-ci-cd` and
  `workflow-nix-hardening-05-verify` skills no longer list `litellm-check` in
  the `just verify` composition.
- **Known staleness (intentional, deferred):** the remaining litellm skill
  corpus under `.agents/skills/` still cites `docs/litellm/schemas/*` and
  `validation-litellm-config-check` — those resolve in the personal repo now
  and move with the deferred docs/litellm knowledge-repo decision (phase 3).
  Historical docs under `docs/` keep their stale citations as record.
- **Gates after landing:** 635 passed / 0 failed / 3 ignored.
- **Phase-3 follow-ups:** `tests/fixtures` 5-workload fixture still contains a
  real litellm workload (ignored KVM tests only); the docs/litellm corpus
  knowledge-repo decision is deferred; tempest package deletion is planned
  (the pre-existing tempest.nix npmDepsHash lambda bug becomes moot).

---

## 0.3 PREVIOUS LANDING — cleanup PHASE 1 (2026-08-02, lands on top of phase-0 HEAD `b9a3ed3`)

**Phase 1 of the approved cleanup landed** as one commit on
`migration/tool-model` (the phase-1 commit; hash assigned at commit time).
What changed and why, for contextless sessions:

- **Deleted (verified safe pre-execution):**
  - `config.reference/agents/{pi,odysseus,opencode}` — duplicated agent
    configs (byte-identical copies live in the dev-home personal config
    repo); `config.reference/agents/example-*` golden-test fixtures KEPT.
  - `nix/packages/{odysseus,opencode}.nix` — consumerless; the
    `odysseus-built`/`opencode-built` flake attrs + package re-exports and
    the corresponding `just update-hashes` steps removed with them
    (tempest/pi attrs untouched; confirm step is now `nix build .#tempest`).
  - Repo-root `var/` + `workspaces/` — the `.gitignore` exception lines,
    the two `optional(...)` checks in `control/agentctl/src/config/loading.rs`
    (optional-count test 10 → 8), and the two entries in the README
    top-level layout sentence removed with them. The mounts.rs/config.rs
    `workspaces//var/` XDG-state prefix-mapping logic is UNTOUCHED.
  - `.assets/opencode-agent/` — legacy personal docker fleet, superseded;
    recoverable from git history.
- **Moved:** `docs/odysseus-full-capability.md` → the user's personal config
  repo (personal-workload content, not generic tooling; the move itself is
  done by a parallel worker). In-repo citations at `docs/gaps.md:14` and
  `docs/integration-plan.md:141` replaced with plain-text pointers.
- **Spec-17 text reconciled with the phase-0 implementation**
  (`17-config-repo-directory-mode.md`): the :73-75 bullet and the migration
  mapping-table row now say mount/seed/local_build paths resolve against the
  DECLARING CONFIG LAYER's directory (capsule-relative names for colocated
  artifacts, e.g. `host = "config.yaml"` next to `workload.toml`), zero
  schema change; a note marks the text as reconciled post-implementation
  (phase 0, `b9a3ed3`). **Correction to the phase-0 drift flag:** spec 20
  contains NO mount/seed path-semantics text (grep-verified) — the "spec
  17/20" flag was spec-17-only; spec 20 needed no amendment.
- **Known doc-citation staleness (follow-ups, NOT fixed here):**
  `12-per-instance-addressing.md:171` cites the deleted pi `models.json`;
  `10-config-repos-as-working-copies.md:297` cites the deleted `.assets`
  fleet; `docs/nix/*.md` + `docs/migration/*` still describe
  `odysseus-built`/`opencode-built`.

---

## 0.4 PREVIOUS LANDING — cleanup PHASE 0 (2026-08-02, landed on top of `09b624f`)

**Phase 0 of the approved mount/seed path-resolution cleanup landed** (one
commit; HEAD moves past `09b624f` — the snapshot below still cites `7624aaf`/
`09b624f` ancestry). What changed and why, for contextless sessions:

- **Root cause (confirmed):** `build_sandbox` called
  `crate::config::project_root()` UNCONDITIONALLY (`runtime/run.rs`), and
  `project_root()` hard-errors when the resolved root lacks `flake.nix`.
  Detached service children re-exec and inherit the operator's cwd
  (`runtime/spawn.rs`), so `workestrate workload up litellm` from a flake-less
  directory died at that gate even though litellm is a registry image with
  zero flake artifacts. **Misattribution hazard:** the per-sandbox log was
  append-mode with no run delimiter, so stale errors from older binaries
  looked current — the log line was NOT the live failure; the flake-root gate
  was. (F4 now writes a `===== workestrate <version> spawn <ts> pid <n> =====`
  delimiter per run.)
- **The fix set:** (F1) repo-relative mount hosts + seed-file sources now
  resolve against the DECLARING CONFIG LAYER's directory (layer-name →
  content-dir plumbing in `merge.rs`: `Layer::source_path`,
  `layer_dirs_from`, `set_layer_dirs`/`get_layer_dirs`); (F2) `project_root()`
  is LAZY in `build_sandbox` — called only for nix-layered image recipes,
  `local_build` configs, or relative build-path mounts, with the error naming
  the triggering feature; (F3) `prepare()`'s silent cwd fallback is removed
  (hard error when seed files exist and no content root resolves); (F4) the
  log delimiter above; (F5) tests incl. `tests/flake_root_gate.rs` (3 non-KVM
  + 1 KVM-gated).
- **Known spec drift (flagged, intentional per approval):** spec 17/20 say
  mount/seed paths "stay repo-relative" (config-REPO root); phase 0 resolves
  them against the declaring FILE's directory (capsule-relative, e.g.
  `config.yaml` next to `workload.toml`). `config.reference` was updated
  accordingly (`infra/litellm`, `agents/example-service/...`; goldens
  regenerated). Spec text itself not yet amended.
- **Gates after landing:** 634 passed / 0 failed / 3 ignored (was 614/0/2).

---

## 1. SNAPSHOT (as of 2026-08-01, HEAD `7624aaf`)

- **HEAD:** `7624aaf` — `fix(config): gate cwd-derived reference config behind
  explicit opt-in (spec 05)`. Branch `migration/tool-model`; working tree clean;
  **2 stashes exist — leave them untouched** (foreign WIP, not ours).
- **In-flight fix (uncommitted at time of writing; lands as a new commit on
  `migration/tool-model`):** `workestrate home clone <src> <dest>` from a
  git-initialized but COMMITLESS source home produced an empty tree (git-clone
  path; `config.toml` uncommitted), breaking provisioning. The git-clone path
  now requires a resolvable HEAD (`git_has_head`,
  `control/agentctl/src/git.rs`); commitless sources fall back to the
  file-copy path. Regression test
  `from_commitless_git_src_falls_back_to_file_copy`
  (`control/agentctl/tests/cmd_home_provision.rs`).
- **Gates:** green — ~613 cargo tests; `just lint-nix` passes; `just
  tombi-check` is wired into `just verify`. All cargo-linked gates run
  in-container via `nix develop` (nix lives at
  `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the
  devshell provides the C toolchain). This container has **no KVM** and **no
  sops age key**.
- **`schema_version = 1` everywhere.** The intermediate v2 model
  (delivery-on-def + the `fold_legacy_secret_model` v1 shim) was retracted
  pre-release — no production deployment, no migration debt, no shim.
- **Current architecture (the state in ≤10 lines):**
  1. The tool home is `~/.workestrate` only (ADR 0023; no repo-local home, no
     discovery tier — spec 08). It is a dotfiles-style git repo with a
     gitlink-guard pre-commit hook (spec 10).
  2. Config repos are first-class **working copies** at
     `~/.workestrate/config-repos/<name>/`; the remote is canonical (spec 10);
     `workestrate.lock` pins url/ref/rev (spec 11, ADR 0025).
  3. Secret defs are a **pure catalog** (spec 16): `[secrets.<ID>]` carries
     `env_var` (default = the ID), `allowed_hosts`, `required` (default true),
     `placeholder` — and nothing else.
  4. Workload `env` is ONE unified name-keyed map with **four env value
     forms**; exposure is the per-binding **`bound`** (`host` default =
     placeholder, `guest` = real value); `allowed_hosts` is the
     credential-level egress substitution policy (spec 16).
  5. Config repos may be **directory-mode** (spec 17):
     `workestrate/{default,secrets}.toml` + `workloads/` capsules that colocate
     each workload's app-native artifacts.
  6. The personal config (`personal-v2`) is at directory-mode + final model;
     the old v1 checkout (`config-repos/personal` @ `c41a707`) refreshes during
     the host-side home setup (§6).

---

## 2. THE MODEL (one page)

Authoritative sources: [02-config-requirements.md](02-config-requirements.md)
§1.1–§1.3.4 and [06-improvements/16-unified-secret-env-model.md](06-improvements/16-unified-secret-env-model.md)
(secret/env) + [06-improvements/17-config-repo-directory-mode.md](06-improvements/17-config-repo-directory-mode.md)
(directory mode). Condensed here for execution; do not re-derive.

### 2.1 Secret definitions — pure catalog

```toml
[secrets.LITELLM_MASTER_KEY]
env_var = "LITELLM_MASTER_KEY"          # optional; default = the secret ID
required = true                         # optional; default true
placeholder = "change_me_before_first_boot"   # optional

[secrets.GITHUB_TOKEN]
allowed_hosts = ["host.microsandbox.internal"]  # substitution restriction
required = true
```

- `env_var`: host env var the value is read from (default: the secret ID).
- `allowed_hosts`: egress hosts whose rewrites may substitute the real value.
  ALWAYS valid regardless of binding mode. Omitted = deny-all; explicit `[]` =
  clear inherited, then deny-all. (`hosts` was RENAMED to `allowed_hosts`.)
- `required`: missing value is a hard error when true (default true).
- `placeholder`: known-bad placeholder value to reject; merge tri-state.
- REMOVED fields (no shim — they hard-error as unknown fields): `delivery`,
  `description`, and the `source`/`exposed_as` remap pair. Remaps live at the
  binding site; exposure lives in `bound`.
- Merge: secrets are a map, deep-merged per field (last-layer-wins per field).

### 2.2 The unified env map + desugar table

`[workloads.<name>.env]` is ONE name-keyed, document-order map of `EnvBinding`.
The map KEY is the exposed env name. Cascading defaults: `secret` defaults to
the KEY name; `bound` defaults to `host`. The `secret` property is
**rename-only**; `KEY = false` is a hard error ("did you mean true?").

Desugar table (verbatim from 02-config-requirements.md §1.3.2):

| You write | Desugars to | Meaning |
|---|---|---|
| KEY = "value" | literal | plain env value |
| KEY = true | { bound: host } | host-bound placeholder for secret KEY (no secret property → secret defaults to key name; false → hard error "did you mean true?") |
| KEY = { bound = "guest" } | { secret: KEY, bound: guest } | real value for secret KEY |
| KEY = { secret = "ID" } | { secret: ID, bound: host } | placeholder for ID ≠ KEY (rename) |
| KEY = { secret = "ID", bound = "guest" } | full form | real value for ID ≠ KEY (renamed real value) |

Resolution at plan build: literal → `builder.env`; **host-bound** (default) →
`builder.secret_env` (placeholder in the guest; the egress rewrite substitutes
the real value only for the credential's `allowed_hosts`); **guest-bound** →
real-value `builder.env` injection for that workload only (verifier opt-in).

### 2.3 Merge rules + version collapse

- `env` merges **union-by-name** (last-layer-wins per key, ADR 0020 Ruling 1);
  bindings are ATOMIC (a binding replaces a same-key binding wholesale).
  Provenance is keyed `workloads.{wl}.env.{NAME}`.
- `secret_env` is REMOVED — a layer declaring it is a hard error.
- `schema_version` stays **1**: the intermediate v2 event was withdrawn
  pre-release (ADR 0018 second addendum). Missing/0 = legacy warning; 1 =
  native; ≥2 = hard error. Post-launch bump policy lives in spec 20.

### 2.4 Directory-mode rules (spec 17)

- **Either/or:** a repo has `workestrate.toml` (single-file) XOR `workestrate/`
  (directory). Both present → hard error.
- **Load order:** `workestrate/default.toml` → optional
  `workestrate/secrets.toml` → `workestrate/workloads/` entries sorted
  **lexicographically** by entry name (flat `<name>.toml` files and `<name>/`
  capsule directories interleaved).
- **Filename-implied names:** a flat file's workload name is its basename; a
  capsule's name is its dirname; the capsule's required entry file is
  `workload.toml`. All other capsule files are opaque artifacts addressed by
  repo-relative paths.
- **Duplicate names** across files/dirs → hard error naming BOTH provenance
  paths. (Within-file duplicates stay the TOML parse error.)
- **`schema_version` authority:** `default.toml` only; presence in any other
  directory-mode file → hard error.
- **Provenance** carries per-file granularity: `<repo>#<relpath>` (e.g.
  `personal#workestrate/workloads/litellm/workload.toml`).

---

## 3. ARTIFACTS MAP

### 3.1 Repo (`/home/node/Development/ai-workbench`, branch `migration/tool-model`)

- `control/agentctl/` — the workestrate Rust tool (config loader
  `src/config/loading.rs`, types `src/config/types.rs`, plan build
  `src/microsandbox/`).
- `docs/migration/` — the authoritative design record (27 ADRs in
  `50-decisions/`; addenda on 0005/0011/0018/0021/0022/0023/0025/0026).
- `docs/validation-and-improvements/` — this operational tree (00–07,
  NEXT-SESSION, 20 improvement specs under `06-improvements/`).
- `schemas/` — vendored `workestrate.schema.json` (compile-time
  `include_str!`; `schema_drift.rs` freshness guard).
- `templates/workestrate-config/` — config-repo scaffold template.
- `nix/` — packages (incl. `tombi.nix` pinned v1.2.5), devshells, recipes; the
  interim 0.5.6 microsandbox-filesystem agentd patch (slimmed `19d94e8`).
- `justfile` — gates; `CARGO_TARGET_DIR` is relocated to
  `~/.cache/ai-workbench/agentctl-target` (justfile:5).

### 3.2 Home (`~/.workestrate`) — container home, ephemeral but restored

- `config.toml` (registry) + `workestrate.lock` (pins).
- `config-repos/personal` — v1 checkout @ `c41a707` (refreshes at host setup).
- `config-repos/personal-v2` — @ `0e0cb08`, **directory-mode + final model**
  (`workestrate/{default,secrets}.toml` + `workloads/` capsules).
- `tombi.toml` (include globs cover `config-repos/*/workestrate/**/*.toml` —
  `ea48f45`), `schemas/`, the gitlink-guard pre-commit hook, `secrets/`,
  `sources/`, `state/`. The home's own git is uncommitted — fine, ephemeral.

### 3.3 `.tmp/` keepers (gitignored, do not delete)

- `.tmp/config-repos/personal-v2` — the ORIGIN of the home checkout.
- `.tmp/microsandbox` — the fork clone. Branch
  `fix/filesystem-agentd-path-override` is at **`27d84216`** — NOTE: this
  differs from the `bc7640b8` recorded in NEXT-SESSION/spec-09; the branch was
  likely amended and carries the same fix (a `bkp/` branch exists).
  **Verify the diff before any push.** Push is a USER action.
- `.tmp/msb-upstream/` — `PR.md` (PR draft), `ISSUE.md`, `REASONING.md`.
- `.tmp/archive/` — `agentd`, `agentd-0.6.8`, `config-repos-export-personal`,
  `microsandbox-fork`, `microsandbox-main`, `msb-home`, `tombi`, `devenv.sh`,
  `devshell.env`.

### 3.4 Fork state

`fix/filesystem-agentd-path-override` @ `27d84216` on
`github.com/georgrybski/microsandbox` — hardened (8-point review + 6
build-script tests), **pending user force-push + PR open**.

---

## 4. PER-SPEC STATUS TABLE

Condensed from [06-improvements/00-index.md](06-improvements/00-index.md)
(post-2026-08-01 flips — the A2 audit). One delta since the index: **spec 05**
landed at HEAD (`7624aaf`) after the index flip pass; its banner/index row
still say SPEC (bookkeeping pending, §5 item 9).

| Spec | Title | ACTUAL status | What remains |
|---|---|---|---|
| 01 | Mount filtering / shadowing | SPEC (not implemented); Phase 0 spike NEEDS-KVM | WP1–WP3 (in-container); Phase 0 KVM spike → WP4; WP5 conditional on spike failure |
| 02 | main standardization | OBSOLETE (2026-08-01) — mooted by spec 08 | none (branch-detection stays a deferred option) |
| 03 | Dogfooding | PARTIAL — Phase 0 env pinning READY-TO-EXECUTE; B1/B2/B3 not implemented | Phase 0 wrapper; B1/B2 (verifiable-here); B3 (HOST-KVM tail) |
| 04 | CLI config authoring | DEFERRED — gated on `02-config-requirements.md` sign-off | nothing until sign-off; must be additive-tolerant / schema-driven |
| 05 | cwd-fallback fix | EXECUTED (2026-08-01, `7624aaf`) — Δ: banner flip pending | bookkeeping only (banner + index row) |
| 06 | `--home` flag | EXECUTED (2026-07-30, `d991252`) | none |
| 07 | Naming consistency | DONE (3 commits; cargo gates pending) | mechanical residue sweep + cargo gates via `nix develop` |
| 08 | No repo-local home | EXECUTED (2026-07-30) | none |
| 09 | agentd offline build | PR PREPARED, READY TO OPEN (fork @ `27d84216`; push pending USER); option 2 REVERSED; option 3 NEEDS-DEVSHELL + HOST-NIX | user push → open PR → post-merge cleanup (delete compensation machinery, bump pin); option 3 (`=0.6.8`) separate track |
| 10 | Config repos as working copies | EXECUTED (2026-07-30) | none |
| 11 | Home provisioning + lockfile | EXECUTED (2026-07-30/31) | none |
| 12 | Per-instance addressing | IMPLEMENTED (Waves 1+2); refuse-only/no-auto-start SUPERSEDED by the ADR 0026 addendum default-on lifecycle; `--use` retained | E1 guest-reachability (HOST-KVM, B10); W5 config wiring (spec §4); §5 follow-ups (DependsOnSpec scheme/path_suffix; wait-for-port → guest healthchecks) |
| 13 | secret_env shorthand | EXECUTED → SUPERSEDED (final model's `true` sugar) | none |
| 14 | env map form | EXECUTED → map survives as the final unified map with `bound` | none |
| 15 | tombi toolchain | EXECUTED (2026-08-01) | doc include-set touch-up (§5 item 8) |
| 16 | Unified secret/env model | EXECUTED (2026-08-01, `19b2cf0`) | B13 runtime smoke (HOST-KVM host batch) |
| 17 | Config repo directory mode | EXECUTED (2026-08-01, `e3194d9`; tombi glob `ea48f45`) | runtime smoke of restructured repo (HOST-KVM); loader follow-ups (§5 item 8) |
| 18 | Cross-home dependencies | INTENT-TO-EXPLORE | exploration only (questions enumerated in spec) |
| 19 | Visualization + inspection | INTENT-TO-EXPLORE | exploration only (candidates + data sources in spec) |
| 20 | Schema evolution + migrations | SPEC (design written, `0b401d9`) | implement: schema pull+lock (vendored schema refresh, lockfile provenance, fail-closed) + `config migrate` framework |

The index file itself (`06-improvements/00-index.md`) is STATUS: INDEX and
current post-flips except the spec-05 row above.

---

## 5. WHAT REMAINS (ordered, with blockers)

1. **Upstream microsandbox PR push — BLOCKED ON USER.** Verify
   `.tmp/microsandbox` branch `fix/filesystem-agentd-path-override` @
   `27d84216` (vs the recorded `bc7640b8` — likely amended, same fix; `bkp/`
   branch exists), then USER: `git -C .tmp/microsandbox push
   --force-with-lease origin fix/filesystem-agentd-path-override` and open the
   PR from `.tmp/msb-upstream/PR.md`. Post-merge (later): delete compensation
   machinery, bump the pin, `nix flake check`.
2. **Host batch B1–B12 + B13 + E1 — BLOCKED ON HOST-KVM/HOST-NIX.** One batched
   host pass per [07-execution-order.md](07-execution-order.md) Step 6 and
   [05-host-validation.md](05-host-validation.md): runtime parity (B1–B12), the
   final-model secret-delivery smoke suite (B13, re-pointed at spec 16), and
   Experiment E1 (guest-reachability of non-`127.0.0.1` loopbacks, B10 — gates
   spec 12's deferred binding decision). Also fold in: tempest FOD hash,
   `ODYSSEUS_ADMIN_PASSWORD` sops provisioning, `just verify-full`,
   `just generate-schema`, and the spec-01 Phase 0 KVM spike.
3. **Spec 01 mounts — in-container work unblocked.** WP1 (schema+glob) → WP2
   (policy+trust) → WP3 (render+audit) are `verifiable-here`; the Phase 0 KVM
   spike gates WP4 (runtime shadows); WP5 (staging-copy fallback) only if the
   spike fails.
4. **Spec 03 dogfooding.** Phase 0 env-pinning wrapper (READY-TO-EXECUTE, no
   code change); B1 self-home mount guard + B2 teardown regression test
   (verifiable-here); B3 spawn provenance (KVM tail).
5. **Spec 09 option 3 — `=0.6.8` bump** (separate combinable track): rewrite
   the patch against 0.6.x build.rs, re-validate `msb --version`, cross-check
   RESOLVE_BENEATH vs spec 01; NEVER the yanked 0.6.5. NEEDS-DEVSHELL +
   HOST-NIX.
6. **Specs 18/19 exploration** — docs-only, no decisions yet; enumerate and
   sharpen the open questions.
7. **Spec 20 implementation** — schema pull+lock (refresh the config repo's
   vendored schema from the installed binary only; provenance header +
   additive `schema` section on `HomeLock`/`workestrate.lock`; fail-closed on
   downgrade) and the `config migrate [--dry-run] [--to <version>]` framework
   (versioned pure toml_edit steps, `.bak` backup, never auto-commits).
8. **Loader follow-ups (identified during the Phase B session):**
   - order-churn fix (lexicographic directory-mode loading vs document-order
     stability in merged output/golden plans);
   - `WORKESTRATE_CONFIG_DIR` directory-mode support — the env override path
     currently reads only the single-file `workestrate.toml`
     (`control/agentctl/src/config/loading.rs` ~line 624);
   - workload-rooted tombi schema in the scaffold template (capsule
     `workload.toml` files validate against the full-file schema; they need a
     workload-rooted schema);
   - tombi toolchain doc include-set touch-up (spec 15 doc vs the landed
     `workestrate/**/*.toml` globs).
9. **Bookkeeping tail:** spec-05 banner/index flip (code landed `7624aaf`);
   NEXT-SESSION.md refresh (HEAD `7624aaf`, ~613 tests); spec-07 cargo gates.
10. **Stale in-tree binary note:** `control/agentctl/target/debug/workestrate`
    (Aug 1 20:59) is STALE — pre-dates HEAD. The live binary is built into the
    devshell target dir `~/.cache/ai-workbench/agentctl-target/debug/`
    (justfile:5 relocates `CARGO_TARGET_DIR`). Never run the in-tree one.
11. **Cleanup phases 5–6 (remaining genericization) — USER DECISIONS.**
    Phases 0–4 landed (§0–§0.4 above). The remaining genericization items
    await user decision and are NOT in flight:
    - age-key path rename (`ai-workbench-secrets.txt`);
    - cache path renames (`~/.cache/ai-workbench-msb`, `CARGO_TARGET_DIR`
      ai-workbench);
    - README/SPEC reframing (headline personal examples);
    - canonical config-flake input URL (`git+file://` vs
      `github:georgrybski/...`);
    - broad `ai-workbench` user-facing string sweep;
    - `ALLOWED_EGRESS_HOSTS` still contains personal provider hosts
      (api.kimi.com / api.neuralwatt.com / api.minimax.io) — the same
      violation class as the phase-4-retired `SECRET_HOST_BINDINGS` table;
    - test-fixture personal-name sweep (~600 `#[cfg(test)]`/doctest hits,
      classified and deferred by the phase-4 closing sweep).
    Standing threads that outlive the cleanup: container-home
    ephemerality/host-side home (§6); `stash@{0}` on `406b5b5` never to be
    touched (§7).

---

## 6. HOST SETUP (the sequence)

Operator task on the host (open thread ⑦ of NEXT-SESSION.md). The host runs
this container via docker compose; the compose file lives on the host, not in
the repo.

1. **Layout (the `agent-workbench` dir on the host):**
   - `workestrate/` — the tool checkout (this repo);
   - `workestrate-dev-config/` — the dev/experiment home;
   - `workestrate-config/` — the personal config repo.
   **Pre-create all three as uid 1000 BEFORE `docker compose up`** (bind-mount
   ownership; container user is uid 1000).
2. **Mount discipline:** the REAL home `~/.workestrate` stays OUTSIDE the
   mounts — if mounted at all, read-only. **NEVER mount `~/.config/sops`**
   into the container (secret material stays host-only; the container has no
   age key by design and decryption fails closed).
3. **Home recreation on the host (3 commands):**
   `workestrate home init` → `workestrate config add
   <repo>/.tmp/config-repos/personal-v2 personal` (from the `.tmp` origin) →
   `workestrate config trust /home/node/Development/ai-workbench`. Then refresh
   the old v1 checkout (`config-repos/personal` @ `c41a707`) to the
   directory-mode + final-model content.
4. **Compose anchors:** use YAML anchors for the shared mount/env blocks so
   the three paths above are declared once and stay in lockstep across
   services.

---

## 7. RULES (standing)

- **ADR-cited decisions via addenda.** No code/config semantic change without
  a cited ADR; when reality drifts from an ADR, record an addendum dated in
  the ADR + an index marker — never silently edit the decision text (pattern
  set by the 0005/0011/0018/0021/0022/0023/0025/0026 addenda).
- **Pre-release = no legacy framing.** Breaking changes are absorbed and the
  version retracts (the spec-16 `schema_version` collapse is the precedent);
  record the supersession in docs, but do not build shims or migrations for
  pre-release states (spec 20 pins the post-launch policy).
- **Validation honesty.** Report `validated` / `partially_validated` /
  `not_validated`; mark gates `verifiable-here` vs `HOST-NIX` vs `HOST-KVM`.
  Runtime claims are specifications, not observations, unless the command was
  actually run and its output cited.
- **Never stage foreign artifacts.** Stage only the files you intentionally
  changed; inspect `git status` before every stage.
- **Stashes (2) untouched.** `stash@{0}` (WIP on main) and `stash@{1}` (WIP on
  master) are foreign WIP — never pop, drop, or reference them in commits.
- **Real home is read-mostly.** No probe writes to `~/.workestrate` or the
  real bundle; destructive operations ONLY in the dev/experiment home
  (`WORKESTRATE_HOME=/tmp/workestrate-exp` in-container, guard:
  `test "$WORKESTRATE_HOME" = "/tmp/workestrate-exp"`).

---

## 8. REPORT BACK

End every session with three things:

- **What was confirmed** — with evidence (command + output, or file:line
  citation). *This session (2026-08-01):* HEAD `7624aaf` verified via
  `git log --oneline -15`; fork @ `27d84216` verified via
  `git -C .tmp/microsandbox log --oneline -3`; home `config-repos/personal-v2`
  @ `0e0cb08` directory-mode verified via `ls` + `git log`; stale in-tree
  binary verified via `stat`/`ls -la` (20:59 in-tree vs 22:56 devshell target
  dir); the desugar table verified verbatim against
  `02-config-requirements.md` §1.3.2; per-spec statuses verified against
  `06-improvements/00-index.md` post-flips (one delta: spec 05).
- **What is blocked and exactly why** — name the missing capability and the
  step it gates. *Currently:* KVM (host batch B1–B13, E1, spec-01 Phase 0
  spike, spec-03 B3); user push access (spec-09 upstream PR); sops age key
  (secret provisioning — host-only by design); nix-on-host (`nix build`,
  `just verify-full`, `just generate-schema`). NOT blocked: cargo gates (nix
  develop provides `cc` in-container).
- **The next concrete action** — one sentence. *Currently:* USER force-pushes
  `.tmp/microsandbox` `fix/filesystem-agentd-path-override` @ `27d84216` and
  opens the upstream PR from `.tmp/msb-upstream/PR.md`; the next agent session
  starts spec 01 WP1 (schema+glob) in-container via `nix develop`.
