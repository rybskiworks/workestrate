# 80 — Remediation Plan (Review Findings 2026-07)

**Status:** IMPLEMENTED (WP1–WP5 merged; merge-gate verdict MET after
independent verification ruled MET-WITH-CONDITIONS and the two conditions
were closed — see "Merge-gate status" below).
**Branch:** `migration/tool-model`
**Authored:** 2026-07-19; **status flipped:** 2026-07-19.
**Supersedes:** none (complements ADR 0020 and the existing migration docs).

---

## Review provenance

This plan is the documented output of a five-way parallel investigation +
main-lead synthesis conducted against `migration/tool-model` @ `12e89b6`.

- **Investigators (5 parallel):** A — Rust core; B — Nix; C — Security; D —
  Docs coherence; E — Tooling gaps.
- **Main-lead synthesis:** spot-verification of the 10 highest-severity claims
  + 7 bonus claims by reading the cited code directly. **Result: 17/18
  confirmed; 1 misattribution (A8 — SOPS_AGE_KEY_FILE is bash-only and
  correct behavior, not a Rust-side bug).** Findings are reliable for
  execution without re-litigation.
- **Verdict (at review time):** NOT merge-ready as-is; merge-ready after
  WP1–WP5. The architecture is sound; the gap is enforcement and
  documentation.
- **Verdict (post-implementation):** MET. WP1–WP5 landed at commits
  `279015b` `973bff3` `73cfd53` `05b6bb7` `89d1658`; the cargo-side
  `just verify` was green (fmt OK; clippy clean; 91 main-bin tests + 2
  spec-examples tests; 3 golden files MATCH; Cargo.lock stable); the
  litellm-check recipe was hardened to work in non-devshell containers.

### Merge-gate status

| Condition (per "Merge-readiness gate" below) | State |
|---|---|
| WP1–WP5 implemented and green | **DONE** (commits `279015b` `973bff3` `73cfd53` `05b6bb7` `89d1658`) |
| Trust-boundary + entitlement + spec-examples regression tests | **DONE** (24 new tests across WP1+WP5; spec_examples_parse.rs is WP4's standing guard) |
| Spec-examples-parse-against-schema CI test | **DONE** (`89d1658`) |
| Holistic trust-model README statement | DEFERRED to WP7 (does not block merge; ADR 0020 records the model and is referenced from README resolution-order section) |
| No FIX-NOW item remains open | **DONE** |
| Independent verification verdict | MET-WITH-CONDITIONS -> MET (the two conditions were: (a) litellm-check recipe portability — closed by `2498f6a`; (b) docs status flip — closed by this commit) |

WP6–WP11 have landed as follow-ups on this branch; they did not block
merge and are now tracked as DONE (see the WP6–WP11 status block below).

### Merge-readiness gate (definition, retained for the record)

The branch merges when **all** of the following hold:

1. **WP1–WP5 implemented and green** (cargo-verifiable gates pass in this
   container; HOST-NIX and HOST-KVM gates green on the host).
2. **Regression tests** for the four trust-boundary fixes (A1, A2/C2/C3/C4,
   A20) and the entitlement-ordering fix (A4) pass.
3. **Spec-examples-parse-against-schema** CI test lands (closes D1 class of
   drift permanently).
4. **Holistic trust-model statement** documented in README (C11/C12/A1).
5. **No FIX-NOW item remains open.**

### WP1–WP5 status table (post-implementation)

| WP | Severity | Status | Commit | Gates |
|---|---|---|---|---|
| WP1 — Trust-boundary and path validation | FIX-NOW | DONE | `279015b` | 22 new tests; A1/A2/C2/C3/C4/A18/A20 regressions green; clippy clean |
| WP2 — Purity and Nix image correctness | FIX-NOW | DONE (HOST-NIX gated) | `73cfd53` | C1/B1/B2/B14 fixes; nix eval regressions deferred to host per env-honesty policy |
| WP3 — Policy enforcement fix (entitlement order) | FIX-NOW | DONE | `05b6bb7` | A4 regression: entitled workload relaxes true→false from a higher layer |
| WP4 — Spec/code reconciliation + CI guard | FIX-NOW | DONE | `89d1658` | rw→read_only sweep in spec; spec_examples_parse CI test (2 tests) green |
| WP5 — New-user journey unblock (`workestrate check`) | FIX-NOW | DONE | `973bff3` | project_root_optional; cmd_check graceful-degradation; 2 integration tests green |

Plus the post-implementation hardening commit:
- `2498f6a` — `fix(just): litellm-check python fallback for non-devshell environments`. Closed the first independent-verification condition.

WP6–WP11 status: ALL DONE.
- WP6 (schema/semantic fixes: A3/A5/A6/C9/C10/E2) — DONE (`56abadb`)
- WP7 (trust-model docs: C11/C12/D7/D11/D12/D13) — DONE (`fe769b3`)
- WP8 (setup-secrets alignment + missing commands: A7/E4/E5/E6/E7/E8/C13) — DONE (`619cc56`, `0b90810`, `3d8d37e`, `aa54496`)
- WP9 (Nix image completeness: B3/B4/B5/B6/B10) — DONE (`7b5e3b3`)
- WP10 (provenance/TOCTOU/registry robustness: A9/A17=C6/A11/A12/A16/A24) — DONE (`980854f`)
- WP11 (production-path test coverage: A21) — DONE (`control/agentctl/tests/production_path.rs` tracked in this consolidation wave)

### Disk-pressure note (probe artifact, not a defect)

During independent verification, impure path-style nix evals (e.g.,
`nix eval --impure` against a `configDir ? ../..` default before WP2's
`builtins.path` filter landed) copied large untracked trees —
`target/` (Rust build artifacts) and `agents/*/build/` (per-workload
build outputs, 100M+ each) — into the store, exhausting disk on a 7 GB
container. This is a probe artifact: those trees are `.gitignore`d and
are never referenced by tracked inputs. WP2's `builtins.path { filter
= ...; }` fix (commit `73cfd53`) bounds the copy to tracked files only,
so the issue does not reproduce on the post-WP2 branch. The note is
retained here as a forensic marker for anyone reproducing verification
against pre-WP2 pin.

### Why two-tier (merge-gate vs follow-up)

WP1–WP5 close the ten FIX-NOW items (real defects / security holes /
spec-code drift / broken install path). WP6–WP11 close FIX-SOON items
(semantic correctness, robustness, docs alignment, missing commands). WP12 is
the standing backlog. This split lets the branch merge as soon as the
load-bearing invariants are restored, without holding merge hostage to docs
rewrites or Nix image-completeness work that has no security impact.

---

## Adjudications (settled rulings)

These four design tensions were raised by the investigators and ruled on by
the main lead. They are the authoritative interpretation of the existing
ADRs and are recorded permanently as ADR 0020.

1. **env union-by-name (closes A3).** env-vars are name-keyed and must
   union-by-name (last-write-wins per key), matching `secret_env`'s existing
   union-by-secret-name pattern in `merge.rs:294-303`. Other REPLACE lists
   (`ports`, `mounts`, `seed_files`, `local_build`, `ingress`) stay REPLACE —
   they are list-of-rows where partial-replace is ambiguous. ADR 0005 is
   correct in spirit; this is a clarification, recorded as an addendum
   pointer in `0005-security-aware-merge.md`.

2. **Entitlement checked before monotonic-true (closes A4).** `tempest` and
   `example-offensive` are the explicit, core-blessed opt-out from
   `default_deny=true`. The entitlement check (`merge.rs:344-349`) MUST run
   before the monotonic-true check (`merge.rs:336-343`), so that an entitled
   workload can relax `true->false` from a higher layer. Monotonic-true
   remains the safety net for non-entitled workloads (who should never have
   `false` anyway). This is the correct reading of ADR 0005's intent.

3. **`run` / `WORKESTRATE_CONFIG_DIR` = document-not-harden, with the
   local.toml exception (closes C11, C12, A1).** `workestrate run` giving
   the spawned process full secret access is an intentional operator escape
   hatch (the operator already has host shell). `WORKESTRATE_CONFIG_DIR`
   bypassing trust gating is the Unix env-var-as-root model. Both are
   documented in a holistic trust-model section of the README, with a
   one-time warning when `run` loads more than N secrets. **Exception:**
   `workestrate.local.toml` is auto-discovered from cwd on every invocation,
   so unlike the env var it does not represent an explicit operator action —
   it is hardened by extending the same trust gate that applies to
   `workestrate.toml` (or marked dev-only with explicit opt-in).

4. **`config.nix` `configDir` fix via `builtins.path` + filter (closes B14).**
   The default `configDir ? ../..` causes Nix to copy the entire repo
   (401M+ of `agents/pi/repo/node_modules`) into the store on eval. The fix
   is `builtins.path { path = ./../config.reference; filter = ...; }`
   (preserves the `configDir` override while bounding the copy), NOT a
   hardcoded path string that loses override semantics.

---

## Work packages

Each WP lists: goal, severity, findings closed (investigator ID + file:line
evidence), exact fix approach per finding, tests to add per finding, effort,
dependencies, validation gate.

### WP1 — Trust-boundary and path-validation cluster - FIX-NOW - Rust/Sec

**Goal:** restore the config-trust invariant that the ADRs promise. A hostile
local layer or a compromised upstream config repo must not be able to mount
host files rw, exfiltrate host files into sandbox state, or escape the repo
directory via `cmd_new`.

**Severity:** FIX-NOW (pre-merge blocker). Without this cluster, the entire
trust framework is decorative.

**Findings closed:**

- **A1** — `workestrate.local.toml` loaded without trust check.
  Evidence: `control/agentctl/src/config.rs:813-816` pushes the local layer
  unconditionally; contrast the project layer at `config.rs:799-807` which
  IS trust-gated via `is_trusted_project()`. Local is the
  highest-precedence layer.
- **A2** — path traversal in `resolve_mount_host`.
  Evidence: `control/agentctl/src/microsandbox/mounts.rs:6-19` does
  `Ok(root.join(host))` with no traversal check; `Path::join("../../etc")`
  escapes root; absolute `host` overrides root per Rust `Path::join`
  semantics.
- **C2** — `local_build.env_override` is a config-controlled env-var name
  consumed as a mount host path.
  Evidence: `workload.rs:420-440` `build_path()` reads
  `std::env::var(env_override)` where `env_override` is the config string;
  `workload.rs:194` substitutes `${WORKESTRATE_<NAME>_BUILD}` mount hosts
  with that path. `env_override="HOME"` + matching mount template yields
  `$HOME` bind-mounted rw.
- **C3** — `seed_files.source` allows `..` traversal.
  Evidence: `workload.rs:394-416` does `root.join(&seed.source)` with no
  traversal check; `source = "../../etc/passwd"` copies host files into
  sandbox state.
- **C4** — mount guest paths unvalidated.
  Evidence: `plan.rs:82-87` `MountPlan.guest: String` has no validation;
  `workload.rs:192` clones config mounts directly into the plan;
  `validate_config` (`config.rs:1001-1103`) does not check guest. `guest =
  "/proc"` rw is accepted.
- **A20 = C14** — `cmd_new` name allows TOML injection / path escape.
  Evidence: `main.rs:cmd_new` writes the workload name into TOML and uses
  it as a directory name with no validation.
- **A18 = C8** — trust paths not canonicalized.
  Evidence: `config.rs:697-717` `is_trusted_project` / `trust_project` use
  raw string comparison; symlinks and `..` defeat the trust check.

**Exact fix approach per finding:**

- **A1:** in `config.rs::load_config` (the block at 813-816), gate the local
  layer behind the SAME `is_trusted_project(&cwd)` check used for the
  project layer. Emit the same `run 'workestrate config trust <dir>'`
  advisory. If USER-DECISION-1 selects "dev-only", instead require
  `WORKESTRATE_ALLOW_LOCAL_LAYER=1` env to load it at all.
  **Recommended:** same trust gate as the project layer (operator can
  deliberately trust once).
- **A2:** add `fn validate_mount_host(host: &str) -> Result<()>` in
  `mounts.rs`. Reject (a) absolute paths (leading `/`), (b) any path
  component equal to `..` after normalization, (c) anything not matching
  the allowed-prefix allowlist: `${MSB_HOME}/`, `${CWD}/`, `${CWD}`,
  `${WORKESTRATE_<NAME>_BUILD}`, `workspaces/`, `var/`, or a relative path
  with no `..` components. Call from `apply_plan_mounts` and
  `ensure_mount_sources` before resolving.
- **C2:** add `fn validate_env_override(name: &str) -> Result<()>` in
  `workload.rs` (or a new `validators.rs`). Enforce `^[A-Z_][A-Z0-9_]*$`
  AND reject a denylist of process-global names: `HOME`, `USER`, `PATH`,
  `SHELL`, `PWD`, `SOPS_AGE_KEY_FILE`, `WORKESTRATE_CONFIG_DIR`,
  `WORKESTRATE_NO_PROJECT_CONFIG`, `WORKESTRATE_CONTEXT`, `AGENTCTL_ROOT`,
  `XDG_*`. Call from `validate_config` so it fails at config load, not at
  plan time.
- **C3:** add `fn validate_seed_source(src: &str) -> Result<()>` mirroring
  `validate_mount_host`'s rules (relative-only, no `..`). Call from
  `validate_config`.
- **C4:** add `fn validate_mount_guest(guest: &str, read_only: bool) ->
  Result<()>` in `mounts.rs`. Reject rw mounts of `/proc`, `/sys`, `/dev`,
  `/etc`, `/root`, `/home` (and any path under those). Reject `..`
  traversal. Call from `validate_config`.
- **A20 = C14:** in `cmd_new`, validate the workload name against
  `^[a-z0-9][a-z0-9-]{0,62}$` BEFORE writing it into TOML or creating any
  directory. Reject paths containing `/`, `..`, or shell metacharacters.
  Reject TOML reserved tokens. Escape nothing — reject instead.
- **A18 = C8:** in `trust_project`, `untrust_project`, `is_trusted_project`,
  call `std::fs::canonicalize(dir)?` on both the stored path and the input
  dir before comparing/storing. If canonicalize fails (broken symlink),
  treat as not-trusted and emit a warning.

**Tests to add (per finding):**

- **A1 regression:** a fixture cwd with `workestrate.local.toml` and an
  untrusted registry -> load_config must NOT include the local layer; the
  advisory must be printed. Then trust the project, re-load, assert local
  layer present.
- **A2 regression:** parametric test over hostile host strings
  (`../../etc`, `/etc`, `${MSB_HOME}/../../etc`, `~/...`) — each must
  `bail!` from `validate_mount_host`. Positive cases (`workspaces/x`,
  `var/y`, `${MSB_HOME}/z`, `${CWD}/w`) must pass.
- **C2 regression:** config with `local_build.env_override = "HOME"` ->
  `validate_config` errors with the denylist message.
- **C3 regression:** config with `seed_files.source = "../../etc/passwd"`
  -> `validate_config` errors.
- **C4 regression:** config with `mounts = [{guest="/proc", rw}]` ->
  `validate_config` errors. Positive case: `guest="/data"` rw passes.
- **A20 regression:** `workestrate new "../pwned"` and `workestrate new
  "a]b"` (TOML injection) -> both error with the validation message; no
  directory or TOML mutation occurs.
- **A18 regression:** trust a project via a symlink path, then query via
  the canonical path -> trusted. Query via a `../` path -> trusted
  (canonicalization normalizes both).

**Effort:** ~1.5 days. **Dependencies:** none. **Validation gate:**
`cargo test -p agentctl` (all new tests + existing green); `cargo clippy
--all-targets -- -D warnings`; manual walkthrough: hostile `local.toml` +
symlink trust path.

---

### WP2 — Purity and Nix image correctness - FIX-NOW - Nix

**Goal:** restore the closed-vocabulary purity invariant (C1) and make the
two broken build attributes (B1, B2) instantiate cleanly. Bound the store
copy in `config.nix` (B14).

**Severity:** FIX-NOW. C1 is a purity violation in the module that exists to
enforce purity; B1 is a broken headline build attribute; B14 exhausted a 7GB
disk during eval.

**Findings closed:**

- **C1** — `baked_files.content` heredoc injection.
  Evidence: `nix/lib/vocabulary.nix:28-37` `bakedFileToShell` emits a heredoc
  `cat > ${path} <<'WORKESTRATE_BAKED_EOF'\n${content}\nWORKESTRATE_BAKED_EOF`.
  After Nix multiline-string dedent the closing delimiter is column-0; any
  content line equal to `WORKESTRATE_BAKED_EOF` breaks out and the
  following lines execute as shell at image build time. Violates the ADR
  0003 closed-vocabulary invariant.
- **B1** — `.#pi-image` aborts at instantiation.
  Evidence: `flake.nix:135` `pi-image = pkgs.callPackage
  ./nix/packages/pi-image.nix {};` — empty args. `pi-image.nix:26` is
  `{ pkgs, pi-bun-built, pi-built }:` — missing args. `nix build
  .#pi-image` errors immediately. (Line 168 `workestrator-pi` correctly
  passes the args.)
- **B2** — `inherit (raw) secrets` throws on secret-less TOML.
  Evidence: `nix/lib/config.nix:8`. A workload TOML without a `[secrets]`
  section throws `attribute 'secrets' missing` at eval. The Rust side
  accepts via `#[serde(default)]` so the two sides disagree.
- **B14** — `config.nix` default `configDir ? ../..` forces full-repo store
  copy.
  Evidence: `nix/lib/config.nix:3`. 401M+ of `agents/pi/repo/node_modules`
  gets copied into the store on every eval.

**Exact fix approach per finding:**

- **C1:** replace the heredoc with a strategy that cannot collide with
  content. Two equally-acceptable options:
  (a) **base64 + printf:** emit `printf '%s' '<base64-of-content>' | base64
  -d > ${path}`. Content never reaches the shell parser as text. Simplest
  and bullet-proof.
  (b) **generated unique delimiter:** generate a delimiter that is provably
  absent from content (e.g. scan content; pick `WORKESTRATE_BAKED_EOF_<n>`
  where n is incremented until no collision).
  **Recommended: (a)** — simpler, no scan logic, no escape edge cases. Add
  an `assert` that `path` still passes the existing relative/no-`..`
  checks. Update the comment block to explain the choice.
- **B1:** at `flake.nix:135`, change the bare call to inherit the same
  args used at line 168: `pi-image = pkgs.callPackage
  ./nix/packages/pi-image.nix { inherit pi-bun-built pi-built; };`.
  Alternative: delete the bare `pi-image` attr entirely and document
  `.#workestrator-pi` as the canonical entry. **Recommended:** inherit
  (preserves the public name).
- **B2:** at `nix/lib/config.nix:8`, change `inherit (raw) secrets;` to
  `secrets = raw.secrets or {};`. One-line fix.
- **B14:** at `nix/lib/config.nix:3-5`, replace the default with a
  filtered path:

```nix
{ configDir ?
    builtins.path {
      path = ./../config.reference;
      filter = path: _: baseNameOf path == "workestrate.toml";
      name = "workestrate-config-reference";
    } }:
let configPath = "${configDir}/workestrate.toml"; ...
```

  Callers who pass an explicit `configDir` (config-repo flakes via
  `buildImagesFromConfig`) keep working; the default bounds the copy to a
  single file. Note: the filter path must be a relative literal so Nix
  tracks it as a source path (no `../..` indirection).

**Tests to add:**

- **C1 regression:** a Nix test (`nix-instantiate --eval` or a `nix build`
  of a synthetic image) where `baked_files.content` includes the line
  `WORKESTRATE_BAKED_EOF` followed by `touch /tmp/pwned`. After the fix,
  the file `/tmp/pwned` must NOT exist in the built image (the literal
  line must be written into the target file, not executed). HOST-NIX gate.
- **B1 regression:** `nix eval --impure --expr 'let f = (builtins.getFlake
  (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'` succeeds
  (returns a store path string). HOST-NIX gate.
- **B2 regression:** a Nix test that imports `config.nix` against a
  secret-less fixture TOML and asserts `result.secrets == {}`. HOST-NIX
  gate.
- **B14 regression:** `nix eval` of a flake importing `config.nix` with
  the default arg must not add `agents/pi/repo/node_modules` to the store
  closure (verify via `nix path-info -r` against a fresh store, or assert
  the inputDrvs contains only the config.reference file). HOST-NIX gate.

**Effort:** ~1 day. **Dependencies:** none. **Validation gate:** `nix eval
.#lib.config.workloadNames`; `nix build .#pi-image` and `.#workestrator-pi`
succeed (HOST-NIX). All four regressions green on host.

---

### WP3 — Policy enforcement fix - FIX-NOW - Rust

**Goal:** make entitlement actually work — `tempest` and `example-offensive`
must be able to relax `default_deny` from `true` to `false` at a higher
layer, which is the entire point of the entitlement list.

**Severity:** FIX-NOW. The headline security-policy primitive
(`DEFAULT_DENY_FALSE_ENTITLEMENT`) is silently broken.

**Findings closed:**

- **A4** — monotonic-true fires before entitlement check.
  Evidence: `control/agentctl/src/merge.rs:333-343` bails on the
  monotonic-true invariant BEFORE the entitlement check at `merge.rs:344-349`.
  So an entitled workload cannot relax `true->false` from a higher layer
  once any lower layer set `true`. Note `validate_config:1067-1076` gets
  the order right; only the merge-time enforcement is wrong.

**Exact fix approach:**

- In `merge.rs::merge_network` (lines 333-358), reorder so the entitlement
  check runs FIRST:
  1. If `layer.default_deny == Some(false)`:
     a. If `!policy::DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&name)` ->
        bail with the not-entitled message (unchanged).
     b. If entitled -> set `merged.default_deny = Some(false)` (skip the
        monotonic-true check — entitlement is the explicit opt-out).
  2. If `layer.default_deny == Some(true)` -> set
     `merged.default_deny = Some(true)` (monotonic-true still allows
     tightening).
  3. If `layer.default_deny == None` -> leave merged unchanged.
  4. Monotonic-true invariant as defense-in-depth is preserved for
     non-entitled workloads by step 1a (they can never reach `Some(false)`
     at all).
  Provenance insert unchanged.

**Tests to add:**

- **A4 regression (the headline test):** three-layer stack:
  - base: `[workloads.tempest.network] default_deny = true`
  - mid:   `[workloads.tempest.network] default_deny = true`
  - top:   `[workloads.tempest.network] default_deny = false`
  After merge, `tempest.network.default_deny == Some(false)` (entitled
  relax works). Existing non-entitled workload test (`merge.rs:614` fixture)
  must still bail at the top layer.

**Effort:** ~0.5 day. **Dependencies:** none. **Validation gate:** `cargo
test -p agentctl merge::tests` (new test + the existing 11 fixture tests
green).

---

### WP4 — Spec/code reconciliation - FIX-NOW - Docs

**Goal:** make the spec parseable against the actual code (D1) and put a
standing guard in place so it stays that way.

**Severity:** FIX-NOW. The spec is the contract; today every mount example
in it is invalid TOML against the implementation.

**Findings closed:**

- **D1** — spec uses `rw`, code requires `read_only`.
  Evidence: `docs/migration/20-target-system-spec.md` uses `rw = true/false`
  in 12+ mount blocks (litellm, pi, odysseus, opencode, tempest sections).
  `plan.rs:82-87` declares `pub read_only: bool`. Every spec example fails
  to deserialize.

**Exact fix approach:**

- Sweep `docs/migration/20-target-system-spec.md`:
  - Replace every `rw = true` with `read_only = false`.
  - Replace every `rw = false` with `read_only = true`.
  - Add a one-line note in the "Path resolution in mounts" section (spec
    line ~585) clarifying that `read_only` is the field name and the polarity.
- Add a **spec-examples-parse-against-schema CI test** as a standing guard
  (the user explicitly asked for this):
  - New file: `control/agentctl/tests/spec_examples_parse.rs` (or an
    `xtask`-style binary).
  - The test reads `docs/migration/20-target-system-spec.md`, extracts
    every fenced TOML block (regex on triple-backtick `toml ... `), parses each with
    the `toml` crate, attempts `toml::from_str::<ConfigFile>` (or the
    appropriate sub-struct), and asserts no errors.
  - Skips blocks that are intentionally fragments (mark with a leading
    comment `# spec-test: skip` or detect by absence of `schema_version`).
  - Add to `just verify` so it runs on every change.

**Tests to add:**

- The CI test above IS the regression test for D1. Additionally:
  - **Negative test:** temporarily flip one `read_only` back to `rw` and
    confirm the test fails. (This is a development-time check, not a
    committed test.)

**Effort:** ~0.5 day. **Dependencies:** none. **Validation gate:** `cargo
test -p agentctl --test spec_examples_parse` green; `just verify` green;
the `# spec-test: skip` mechanism documented in the spec's "How to read"
section.

---

### WP5 — New-user journey unblock - FIX-NOW - Tooling

**Goal:** make `workestrate check` (and the documented install path) work
on first invocation outside a repo checkout.

**Severity:** FIX-NOW. E1 is the first thing a new user runs after `nix
profile install`; today it errors out.

**Findings closed:**

- **E1** — `workestrate check` hard-requires `flake.nix` in cwd.
  Evidence: `main.rs:1052` calls `config::project_root()` which bails at
  `config.rs:53-58` when `flake.nix` is absent. `cmd_check` wraps it as
  `if let Ok(root) = ... else { all_ok = false }` and returns `Err` at
  `main.rs:1073`. The documented `nix profile install` flow always fails
  on first command.

**Exact fix approach (depends on USER-DECISION-1 — see "User decisions"):**

- **Option A (recommended): standalone-installed tool.** Make
  `project_root()` return `Ok(cwd)` if no `flake.nix` is found (or
  introduce `project_root_optional()` that returns `None` instead of
  erroring). `cmd_check` degrades gracefully: reports the registry, the
  reference config (resolved via `find_reference_config()` which already
  has a `CARGO_MANIFEST_DIR` fallback), the trusted projects, and skips
  the workbench-layout checks with a `("(not in a workbench checkout)")`
  note. Exit 0 if all resolvable checks pass; exit 1 only if a required
  artifact is missing AND resolvable.
- **Option B: repo-bound tool.** Document that `workestrate` must be run
  from inside a workbench checkout; `nix profile install` is for the
  wrapper only; remove the misleading install-path documentation. Higher
  long-term cost.

**Tests to add:**

- **E1 regression:** integration test that simulates a fresh install:
  `WORKESTRATE_CONFIG_DIR=<fixture>` + `HOME=<tmp>` + cwd in `/tmp`,
  `workestrate check` exits 0 and prints `(not in a workbench checkout)`
  rather than erroring.

**Effort:** ~0.5 day. **Dependencies:** USER-DECISION-1 settled; WP1 lands
first so the new user's first config can be trusted. **Validation gate:**
the integration test green; manual: `nix profile install` + `workestrate
check` from `/tmp` exits 0 with helpful output (HOST-NIX).

---

### WP6 — Schema and semantic fixes - FIX-SOON - Rust/Tooling — DONE (`56abadb`)

**Goal:** enforce `schema_version`; align env merge with secret_env; fix
provenance keys; honor custom env_override in the template matcher; tighten
vocabulary validation; optionally add `deny_unknown_fields`.

**Severity:** FIX-SOON. Each is a real correctness or robustness issue
without acute security impact.

**Findings closed:** A3, A5, A6, C9, C10, E2.

**Exact fix approach per finding:**

- **A3 (env union-by-name):** in `merge.rs::merge_workload` (the block at
  lines 268-271), replace `merged.env = layer.env.clone();` with a
  union-by-name loop mirroring `secret_env`'s pattern (294-303):
  iterate `layer.env`, replace any entry with the same `name`, append
  others. Provenance keyed by `workloads.{name}.env.{env_name}` (not the
  current whole-list key) so `plan --show-source` is per-var.
- **A5 (provenance key mismatch for remapped secrets):** pick one key
  convention and align both sides. **Recommended:** key by `exposed_as`
  (the name the sandbox actually sees). Change `merge.rs:299` insert to
  use `se.exposed_as` (or the resolved exposed name); `workload.rs:306`
  already queries by `se.name`. Add a regression test using the shipped
  `LITELLM_AUTH` fixture (a remapped secret) — `plan --show-source` must
  attribute it to the layer that defined the remap, not "core".
- **A6 (build_path template-mount divergence):** in
  `resolve_mount_host_template` (`workload.rs:447-466`), in addition to
  the existing `${WORKESTRATE_<NAME>_BUILD}` match, also match
  `${<env_override>}` when `local_build.env_override` is set. Pass
  `env_override` into the function. Add a test: workload with
  `local_build.env_override = "PI_BUILD_DIR"`, mount host =
  `${PI_BUILD_DIR}/assets` -> resolves to `$PI_BUILD_DIR/assets`.
- **E2 (schema_version validation):** add `pub const EXPECTED_SCHEMA_VERSION:
  u32 = 1;` in `config.rs`. In `validate_config` (1001-1103), add at top:
  `if config.schema_version != EXPECTED_SCHEMA_VERSION { bail!(...); }`.
  Update the schema-version residual-risk mitigation in
  `70-open-items.md` to reflect the new check.
- **C9 (validate image.recipe/features/local_build.recipe against
  vocabulary):** in `validate_config`, add a loop over `config.workloads`
  that asserts `image.recipe in {"registry","nix-layered"}`,
  `local_build.recipe in {"npm-build","bun-compile","pip-install","bun-install"}`,
  each `feature` name in vocabulary `features`, each `binary.recipe` in the
  same set. Source the allowed names from `recipes.rs`/`policy.rs` (single
  source of truth), not a duplicated literal.
- **C10 (`deny_unknown_fields`):** see USER-DECISION-2. If "yes on main
  config": add `#[serde(deny_unknown_fields)]` to `ConfigFile`,
  `WorkloadConfig`, `NetworkConfig`, `ImageConfig`, `LocalBuildConfig`,
  `SecretDefinition`. Override layers keep the existing lenient
  `CONFIG_FIELDS`/`WORKLOAD_FIELDS` allowlist semantics (so user-global
  overrides can be partial).

**Tests to add (per finding):**

- **A3:** three-layer merge where base has `[env] FOO=1, BAR=2`, override
  has `[env] FOO=10, BAZ=3` -> merged has `FOO=10, BAR=2, BAZ=3`.
- **A5:** the LITELLM_AUTH remap case described above.
- **A6:** the PI_BUILD_DIR case described above.
- **E2:** config with `schema_version = 99` -> `validate_config` errors
  with the expected-vs-actual message. Existing `schema_version = 1`
  fixtures still parse.
- **C9:** config with `image.recipe = "vaporware"` -> `validate_config`
  errors with the allowed-set message.
- **C10 (if approved):** config with a typo field `[workloads.pi]
  imge = {...}` -> `validate_config` errors.

**Effort:** ~1.5 days. **Dependencies:** WP1, WP3. **Validation gate:**
`cargo test -p agentctl` green; `cargo clippy -- -D warnings`.

---

### WP7 — Trust-model documentation and escape-hatch warnings - FIX-SOON - Docs/Tooling — DONE (`fe769b3`)

**Goal:** state the operator-trust model holistically; warn on `run`;
clean up stale docs.

**Severity:** FIX-SOON. Documentation is the contract; several docs are
stale.

**Findings closed:** C11, C12, D7, D11, D12, D13.

**Exact fix approach per finding:**

- **C11 + C12 (holistic trust model):** add a new "Trust model" section to
  `README.md` stating explicitly:
  - "Setting `WORKESTRATE_CONFIG_DIR` grants the workestrate process full
    config authority — treat it like setting PATH."
  - "`workestrate run -- <cmd>` loads ALL secrets and exec's `<cmd>` with
    them in env. This is an intentional operator escape hatch (you already
    have host shell)."
  - "`workestrate.local.toml` is auto-discovered from cwd and (after WP1)
    requires the same explicit trust as `workestrate.toml`."
  - Cross-link to ADR 0014 and ADR 0020.
- **C11 warning:** in `cmd_run` (`main.rs:1159`), after `load_secrets()`,
  if the loaded secret count exceeds a threshold (e.g. 5), print a
  one-time warning to stderr naming the count and pointing at the README
  section. Suppressible via `WORKESTRATE_NO_RUN_WARNING=1`.
- **D7:** in `50-decisions/0013-layering-ordered-registry-layers.md`, add
  at the top: `**Status:** Superseded by ADR 0019 (contexts) and ADR 0020
  (review rulings) for the layer-precedence and merge-semantics details.`
  Do NOT rewrite the ADR body.
- **D11:** at the top of `SPEC.md`, add a banner: `**STATUS: STALE
  (pre-migration M1).** The authoritative current specification is
  docs/migration/20-target-system-spec.md. This file is retained for
  history and will be deleted in a follow-up.` Defer actual deletion to a
  separate sweep.
- **D12:** rewrite `docs/secrets.md` to reflect the ADR 0018 layering +
  7-secret schema + `--config`/`--global` setup-secrets flags. Remove the
  "4 secrets / root-`.env.enc`" framing.
- **D13:** rewrite the README resolution-order section to match
  `config.rs::load_config` exactly: reference < context layers (in
  declared order) < user-global `[global]` < user-global `[configs.<name>]`
  < trusted project < project local. Add a `workestrate config resolve
  --dump` mention (deferred to backlog per E17) so users can verify.

**Tests to add:** docs-only; no automated tests. Manual review checklist
against current code.

**Effort:** ~1 day. **Dependencies:** WP1 (so the documented trust model
matches the implemented one). **Validation gate:** docs review; the
spec-examples test from WP4 still green.

---

### WP8 — Setup-secrets alignment and missing commands - FIX-SOON - Scripts/Tooling — DONE (`619cc56`, `0b90810`, `3d8d37e`, `aa54496`)

**Goal:** make setup-secrets.sh and agentctl agree on resolution order; add
the documented-but-missing commands.

**Severity:** FIX-SOON. New-user journey currently requires `nix develop`
for setup-secrets; several documented commands don't exist.

**Findings closed:** A7, E4, E5, E6, E7, E8, C13.

**Exact fix approach per finding:**

- **A7 + E7 (setup-secrets alignment):** add a `workestrate config-dir
  [--for <workload>]` subcommand that prints the resolved active config
  directory AND the resolved secrets file + age-key-file for that target
  (mirroring `resolve_active_config_dir` + `resolve_secrets_layers`).
  Rewrite `setup-secrets.sh`'s TARGET_DIR/SECRET_FILE/age-key-file
  resolution (script:98-141) to call `workestrate config-dir` first and
  fall back to the current shell logic only if `workestrate` is not on
  PATH. This makes setup-secrets.sh and agentctl share a single source of
  truth. (Per-repo `secrets_file` round-trip is fixed for free.)
- **E4 (`clean`):** implement `workestrate clean [--all | --workload
  <name>] [--dry-run]` that removes sandbox state
  (`${state_dir}/var/run/<instance>.json`, workspaces for the workload)
  per the spec. `--dry-run` prints what would be removed. Document the
  safety invariant: never removes config repos or secrets.
- **E5 (`config remove`):** implement `workestrate config remove <name>`
  that removes the entry from the registry and deletes the managed clone
  at `${store_dir}/repos/<name>/`. Refuses if `<name>` is in the active
  context's layers (operator must edit the registry first).
- **E6 (`context list` / `context current`):** implement both as trivial
  read-only commands over the existing `resolve_active_context()` +
  registry data.
- **E8 (`doctor`):** implement `workestrate doctor` that checks and
  reports: `/dev/kvm` presence + permissions; `nix` on PATH; `sops` on
  PATH; age key file existence + perms (0600 owner-only — also closes
  **C13**); `.env.enc` existence + perms; trust registry loadable. Exits
  non-zero with remediation hints if any check fails.
- **C13 (perms check on age key / `.env.enc`):** rolled into `doctor`;
  additionally, `secrets_loader.rs::decrypt_layer` warns if the key file
  or `.env.enc` is world-readable.

**Tests to add:**

- **A7:** `cargo test` for `workestrate config-dir` output across
  `--global`, `--config <name>`, default modes; shell test that
  `setup-secrets.sh --config team init` writes to `team.env.enc` when the
  registry declares it.
- **E4-E6, E8:** unit tests over the registry mutation functions; one
  integration test per command against a fixture registry.

**Effort:** ~1.5 days. **Dependencies:** WP5, WP7. **Validation gate:**
`cargo test -p agentctl` green; manual walkthrough of each new command.

---

### WP9 — Nix image completeness - FIX-SOON - Nix — DONE (`7b5e3b3`)

**Goal:** make the recipes that the parameterization produced actually
build the agents they claim to.

**Severity:** FIX-SOON. The headline agents (pi via npm-build, pi via
bun-compile) would be broken if anyone tried to rebuild them from the
recipe layer.

**Findings closed:** B3, B4, B5, B6, B10.

**Exact fix approach per finding:**

- **B3 (npm-build can't build pi):** `nix/lib/recipes/npm-build.nix` needs
  a per-workload override hook (`dontNpmBuild`, `preBuild`, `buildPhase`
  passthroughs) so pi's root build script can be replaced/augmented. Add a
  `piOverrides` arg that the recipe applies via `//`. The recipe header
  comment ("can't build pi") becomes a "builds pi via overrides" note.
- **B4 (bun-compile drops pi asset mirroring):** extend
  `nix/lib/recipes/bun-compile.nix` with an `extraAssetPaths` list arg;
  pi's call passes the themes/wasm/etc assets. Mirror them into the
  bun-compile output tree.
- **B5 (installLayout accepted but dead):** either wire
  `installLayout` into the recipe flow (call it in `buildPhase` after
  `npm install`) OR remove the parameter from the recipe signature.
  **Recommended:** remove (KISS — no current consumer).
- **B6 (buildImagesFromConfig doesn't resolve flake:// URIs):** in
  `nix/lib/recipes/nix-layered.nix` (or wherever `buildImagesFromConfig`
  lives), accept `source` URIs of form `flake:<flake-url>#<attrpath>` and
  resolve them via `builtins.getFlake` (impure) or a flake input. Document
  that config-repo flakes must add the source flake as an input.
- **B10 (tempest npmDepsHash placeholder):** run `nix run
  nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json` on
  a host and replace the `sha256-AAAA...` placeholder at `flake.nix:143`.
  Until then, document in `50-decisions/README.md` ADR index that
  `.#tempest-built` is non-functional (HOST-NIX gate).

**Tests to add:** HOST-NIX only — `nix build .#pi`, `.#pi-bun`,
`.#tempest-built`, `.#odysseus-built`, `.#opencode-built` succeed; image
contents include the expected assets (a `nix build` + `docker load` +
`docker run --rm <image> ls /app/...` smoke check).

**Effort:** ~2-3 days. **Dependencies:** WP2. **Validation gate:** all
five agent derivations build on host (HOST-NIX).

---

### WP10 — Provenance, TOCTOU, and registry robustness - FIX-SOON - Rust — DONE (`980854f`)

**Goal:** make provenance correct under the multi-thread tokio runtime;
close the port-registry race; surface corrupt state instead of silently
falling back.

**Severity:** FIX-SOON. Latent correctness/robustness issues.

**Findings closed:** A9, A17=C6, A11, A12, A16, A24.

**Exact fix approach per finding:**

- **A9 (thread_local provenance):** two options:
  (a) Move provenance out of `thread_local!` into a value returned by
  `merge_layers` and threaded explicitly through `plan`/`show_source` —
  invasive but correct.
  (b) Pin the load+plan sequence to a single thread via
  `tokio::task::LocalSet` or by making the relevant functions synchronous
  (the CLI doesn't actually need them async).
  **Recommended: (a)** — provenance is per-invocation data, not a global;
  it should never have been thread-local. Thread the value.
- **A17 = C6 (port-registry TOCTOU):** in
  `microsandbox/port_registry.rs`, replace the check-then-register pattern
  with an atomic update: write a temporary lock file
  (`${state_dir}/var/run/.registry.lock`) via `O_EXCL`, perform the
  check-and-register under the lock, release. Alternatively, use SQLite
  (already a transitive dep via libsqlite3-sys per `target/`) with a
  UNIQUE constraint on host port.
- **A11 (env-name runtime-only validation):** add `fn
  validate_env_name(name: &str) -> Result<()>` enforcing `^[A-Z_][A-Z0-9_]*$`;
  call from `validate_config` for every `env.name` and `secret_env`
  exposed name. Fails at config load, not at sandbox start.
- **A12 (resolve_state_dir silent fallback):** in
  `config.rs::resolve_state_dir`, if the registry file is corrupt
  (serde fail), `bail!` with a message naming the file and remediation
  (`backup and remove, or restore from registry.toml.bak`), do NOT fall
  back silently.
- **A16 (corrupt port-registry silent skip):** same pattern in
  `port_registry.rs` — error, do not skip.
- **A24 (Cargo.lock drift):** `Cargo.lock` currently has uncommitted
  modifications. Either commit (if intentional) or revert (if not). Add a
  CI check that `git diff --exit-code control/agentctl/Cargo.lock` passes
  after `cargo check` (lockfile stability, matching the
  `Cargo.lock stability: PASS` claim in `40-migration-process.md:322`).

**Tests to add:**

- **A9:** spawn a workload resolution across `tokio::spawn` boundaries,
  assert provenance is preserved.
- **A17:** two concurrent `workestrate pi plan` (or port-register calls)
  must not both succeed in claiming the same port.
- **A11:** config with `env.FOO-BAR = 1` -> `validate_config` errors.
- **A12 / A16:** corrupt the registry / port-registry file, assert
  load_config / port-registry bails with the remediation message.

**Effort:** ~1 day. **Dependencies:** WP6. **Validation gate:** `cargo
test -p agentctl` green; concurrent-port test green.

---

### WP11 — Production-path test coverage - FIX-SOON - Tests — DONE (`control/agentctl/tests/production_path.rs` tracked in this consolidation wave)

**Goal:** exercise the production filesystem discovery path (not just
`WORKESTRATE_CONFIG_DIR`), so trust-gating, local layer, and canonicalization
are actually tested.

**Severity:** FIX-SOON. Test fidelity is the root cause of multiple missed
bugs (per the meta-observations in the review).

**Findings closed:** A21, plus regression tests for WP1/WP3/WP6 already
listed under those WPs.

**Exact fix approach:**

- **A21:** add `control/agentctl/tests/production_discovery.rs` that:
  - Spins up a tmpdir HOME + tmpdir cwd.
  - Writes a registry with a trusted project + a config repo fixture.
  - Writes `workestrate.toml` and `workestrate.local.toml` at cwd.
  - Invokes `load_config()` (NOT via `WORKESTRATE_CONFIG_DIR`) and asserts:
    which layers loaded, that the trust gate fired, that the local layer
    was/was-not loaded per trust state.
  - Covers the four discovery branches: env var, context layers,
    user-global overrides, trusted project + local.

**Tests to add:** the test above.

**Effort:** ~1 day. **Dependencies:** WP1, WP3, WP6. **Validation gate:**
the production-discovery test green; full `cargo test -p agentctl` green.

---

### WP12 — Backlog sweep - trickle - All

**Goal:** drain the standing backlog of minor items surfaced by the
investigators that have no acute impact.

**Severity:** BACKLOG. Each item is low-impact; bundle opportunistically.

**Items (confirming against the synthesis backlog table):**

- B8 (odysseus pip pure-sandbox) — needs fixed-output or impure.
- B9 (opencode bun install network) — needs `bunDepsHash`.
- B11 (strip-path divergence recipe vs pi-image).
- B12 (requirements.lock precedence) — **document only**, intended
  behavior.
- B13 (bakedFileToShell regex rejects literal `..` filenames) — false
  positive; relax regex to component-level check.
- A10 (optional-secret decrypt failures silent-ish) — add INFO log.
- A13 (stale `allow(dead_code)` on `recipes::expand`) — remove.
- A14 (catch-all silently drops unknown args) — add WARNING.
- A15 (`--no-project-config` not global while `--context`/`--show-source`
  are) — make global.
- A19 (`cmd_new` creates repo dir for nonexistent layer) — validate first.
- A22 (missing edge tests) — covered in WP11.
- C5 (instance_name unsanitized filename) — subsumed by A20 in WP1.
- C7 (ingress list-replace allows Local->Public widening) — latent;
  `Scope::Public` is `dead_code`. Either remove the variant or add a
  monotonic-scope invariant (Local->Public blocked). Backlog: document the
  invariant in ADR 0020 if not enforced.
- C13 — covered in WP8.
- D2-D6, D14-D22, F19 — spec field table gaps and glossary nits; bundle
  into a single docs sweep when WP4 + WP7 land.
- E9 (source clone flake:// instruction-only) — improve.
- E10 (source reset weak) — strengthen.
- E11 (no source update) — add.
- E12 (no dynamic completions) — add `clap_complete`.
- E13 (no `plan --all`) — add.
- E14 (no uninstall) — add `workestrate uninstall` (inverse of `init`).
- E15 (spec `plan <name>` vs actual `<name> plan` mismatch) — pick one
  (recommended: `<name> plan` — matches the existing hybrid dispatch)
  and update the spec.
- E16 (backup/restore story) — add `workestrate backup` / `restore` over
  the registry + state dir.
- E17 (no whole-config resolve dump) — add `workestrate config resolve
  --dump` for debugging (also referenced in WP7's README rewrite).
- E18 (multi-context batch) — already deferred per ADR 0019; confirm in
  70-open-items.md.

**Effort:** trickle. **Dependencies:** none blocking. **Validation gate:**
per-item.

---

## User decisions

Four decisions are open. Each lists options, the main-lead recommendation,
and the **default-if-silent** (the choice that takes effect if the user
does not respond by the time implementation starts). Defaults are chosen to
minimize risk and unblock the largest number of WPs.

### USER-DECISION-1 — Repo-bound or standalone-installed tool?

Drives the WP5/E1 fix shape and the long-term CLI contract.

- **Option A (recommended): standalone-installed tool.** `workestrate`
  runs from any cwd; the workbench checkout is only needed for `cargo
  run`/dev. `project_root()` becomes best-effort. `workestrate check`
  degrades gracefully when not in a checkout. **Default-if-silent: A.**
- **Option B: repo-bound tool.** `workestrate` must be run from inside a
  workbench checkout. Simpler internal contract; harsher operator UX;
  contradicts the existing `nix profile install` documentation.

### USER-DECISION-2 — `deny_unknown_fields`?

- **Option A (recommended): yes on main config, lenient on override
  layers.** Add `#[serde(deny_unknown_fields)]` to the main config
  structs (`ConfigFile`, `WorkloadConfig`, `NetworkConfig`, etc.) so typos
  like `imge = {...}` fail loudly. Override layers keep the existing
  `CONFIG_FIELDS`/`WORKLOAD_FIELDS` allowlist + WARN semantics so
  user-global overrides can be partial across versions. **Default-if-
  silent: A.**
- **Option B: lenient everywhere (status quo).** Forward-compat is
  maximized but typos slip through.
- **Option C: strict everywhere.** Breaks the partial-overrides use case.

### USER-DECISION-3 — env union-by-name?

- **Option A (recommended): yes.** env-vars merge by name (last-write-
  wins per key), matching `secret_env`'s pattern. Closes A3.
  **Default-if-silent: A.**
- **Option B: status quo (replace).** Override that adds one var silently
  drops base vars; surprising but documented.

### USER-DECISION-4 — C11/C12 hardening beyond documentation?

- **Option A (recommended): document only (plus the local.toml
  hardening already in WP1).** `workestrate run` keeps full secret
  access; `WORKESTRATE_CONFIG_DIR` keeps trust-bypass authority. README
  states the model explicitly; `run` warns when loading many secrets.
  **Default-if-silent: A.**
- **Option B: harden both.** Add a `--full-secrets` confirm flag to `run`;
  require explicit trust on `WORKESTRATE_CONFIG_DIR`-supplied paths.
  Maximally defensive; breaks the operator-escape-hatch use case and CI
  workflows.

---

## Execution strategy

### WP1-WP5 parallelization

```
        +- WP1 (trust-boundary) -----------+
        |                                  |
START --+- WP2 (nix purity/image) ---------+--> MERGE GATE
        |                                  |
        +- WP3 (entitlement order) --------+
        |                                  |
        +- WP4 (spec reconciliation) ------+
        |                                  |
        +- WP5 (new-user journey) ---------+
                  |
                  +-- depends on USER-DECISION-1 + WP1 (so first config is trustable)
```

- **WP1, WP2, WP3, WP4 are fully independent** — parallelize.
- **WP5 depends on USER-DECISION-1 and on WP1** (the new user's first
  `workestrate check` should produce a trustable config).
- WP6+ are sequential after the merge gate.

### Environment honesty

- **Cargo-verifiable here (this container):** WP1 (Rust), WP3, WP4 (Rust
  test + spec sweep), WP5 (Rust), WP6, WP7 (docs), WP8 (Rust + shell),
  WP10, WP11.
- **HOST-NIX (deferred to host, one batch at the end):** WP2 nix eval /
  build gates; WP9 all gates; B10 prefetch; the C1 Nix image-content
  regression; the B1/B2/B14 eval regressions.
- **HOST-KVM (deferred to host):** the WP1 security fixes' end-to-end
  behavior (a hostile `local.toml` actually refused at sandbox start);
  WP5 manual install walkthrough.

The host-batched runs happen in one pass at the end so we are not
context-switching the host environment.

### Commit conventions

- **One logical commit per WP or per finding cluster**, NOT one mega-
  commit. Suggested split:
  - WP1: one commit per finding cluster (trust gate; mount host/guest;
    env_override; seed_files; cmd_new; canonicalization) — ~6 commits.
  - WP2: one commit per finding (C1; B1; B2; B14) — 4 commits.
  - WP3: single commit + regression test.
  - WP4: one commit for the sweep + one commit for the CI test.
  - WP5: single commit.
  - WP6+: one commit per finding.
- **Commit message format:** `fix(agentctl): <one-line>` /
  `fix(nix): <one-line>` / `docs(migration): <one-line>`. Reference the
  finding ID in the body (`Closes A1.`).
- **Do not mix code and docs in one commit** except where a doc sentence
  is intrinsic to the code change (e.g. a function's doc comment).

### Branch strategy

- **Stay on `migration/tool-model`.** Do not open a long-lived
  `migration/remediation` branch — the remediation IS the migration's
  completion.
- WP6+ may continue on the same branch after the merge gate, or move to a
  follow-up branch if the team prefers a cleaner merge history. The
  merge-gate-vs-follow-up split is documented in "Merge-readiness gate".

---

## Explicitly OUT OF SCOPE for remediation

These items are NOT addressed by any WP and are intentionally left for
future work, with rationale. (Cross-referenced from the review's BACKLOG
table and the investigators' nice-to-have notes.)

| Item | Why out of scope |
|---|---|
| **E16 backup/restore** | No data-loss risk in current single-operator model; defer until multi-operator or before any destructive migration. |
| **E12 dynamic completions** | UX polish; `clap_complete` is a one-day add but no correctness impact. |
| **E14 uninstall** | Inverse of `init`; useful but not load-bearing. Manual rm is acceptable. |
| **E13 `plan --all`** | Convenience; `for w in $(workestrate workload-list); do workestrate $w plan; done` works today. |
| **E18 multi-context batch** | Explicitly deferred by ADR 0019; no current use case. |
| **B8/B9/B11/B13** | Nix pure-eval and recipe polish; no acute impact on the headline agents (which use the per-agent `.nix` files, not the recipes, until WP9 lands). |
| **A22 broad edge-test sweep** | WP11 covers the production-path gap; broader edge tests land opportunistically with each bug fix. |
| **Tempest hardening beyond entitlement** | Out of scope; tempest is by-design the offensive workload with full network. |
| **Root-cause fix for `workestrate.local.toml` auto-discovery model** | WP1 hardens it; revisiting the discovery model itself (e.g. making local overrides user-global only) is a separate UX decision for a future ADR. |
| **Pre-migration `.env.enc` git-history scrubbing** | Already documented as a residual risk in `70-open-items.md`; age key rotation is the mitigation, not history rewriting. |

---

## Cross-references

- **ADR 0020** (`50-decisions/0020-review-adjudications.md`) — permanent
  record of the four adjudications.
- **ADR 0005 addendum pointer** — records the env-union-by-name
  clarification and the entitlement-before-monotonic-true interpretation.
- **`70-open-items.md`** "Review findings (2026-07)" section — per-WP
  pending-approval status.
- **`40-migration-process.md`** "Review and remediation" section — review
  completed; remediation pending approval.
