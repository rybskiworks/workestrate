# 07 — Execution Order (recommended sequence)

> **STATUS: READY-TO-EXECUTE**
> Prerequisites / see-also: [README.md](README.md) ·
> [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [04-baseline-validation.md](04-baseline-validation.md) ·
> [05-host-validation.md](05-host-validation.md) ·
> [06-improvements/00-index.md](06-improvements/00-index.md)

This document defines the **recommended execution sequence** for the entire
validation-and-improvements effort: bundle hygiene, Lane A config-plane
validation, baseline recovery, quick-win improvements, dogfooding guardrails,
experiment-home probes, the batched host pass, Track A mount filtering, and
the deferred remainder. A contextless operator follows it top-to-bottom.

The spine below is the recommended ordering. It was adjusted from the initial
draft **only** where sibling files contradicted it — adjustments are called
out inline with the reason.

> Every env marker, command spelling, and file:line citation below is drawn
> from the sibling docs cited. No command is invented. The codebase is on
> branch `migration/tool-model`.

---

## Environment markers

Reprodu from [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)
§Environment markers:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). NOTE: cargo-linked gates are NOT runnable here — no `cc` linker; they run on the host (HOST-NIX devshell) |
| `HOST-NIX` | Requires nix on the user's host (this container has no nix / no `cc` linker) |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

---

## Recommended sequence

### Step 0 — Bundle hygiene

| | |
|---|---|
| **Goal** | Resolve the bundle-drift items flagged in [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle fixes needed" that are prerequisites for everything downstream. |
| **Files to follow** | [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle fixes needed" (items a–e); [06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md) for item (a). |
| **Sub-steps** | **(e) [CRITICAL — first] APPLIED.** `install_layout = "app"` has been REMOVED from tempest's `binary` inline table in `.workestrate/repos/personal/workestrate.toml:317` (verified: `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no match). `BinarySpec` (`control/agentctl/src/config/types.rs:44-52`) has `#[serde(deny_unknown_fields)]` and no `install_layout` field (the nix-side param was removed as a silent no-op, `nix/lib/recipes/npm-build.nix:16-25`). **PENDING:** runtime parse verification — `workestrate validate-config` cannot run in this container (no `cc` linker); it is the **first Lane A action** in the HOST-NIX devshell (`nix develop`). `verifiable-here` (config edit applied); runtime parse verification is `HOST-NIX`. **(a) APPLIED.** Branch rename `master` → `main` in the personal clone is DONE — this is [06-improvements/02](06-improvements/02-main-standardization.md) in full; `verifiable-here` (plain `git branch -m`, no remote, no registry edit). Verified: `git -C .workestrate/repos/personal branch --show-current` → `main`, HEAD `d2cd0c3` unchanged (matches `config.toml:10`). **Still pending:** the clone has NO origin remote (verified: `git remote -v` → empty), so `workestrate config update` fails for that reason until the config repo is pushed to a remote. **(b)** `scratch/` vs `cache/` directory naming — **open decision** (flag for resolution): `.workestrate/scratch/` exists but ADR 0023 lines 73–82 specify `cache/` in the single-home layout. Either rename `scratch/` → `cache/` or amend the ADR. `verifiable-here`. **(c)** Tempest `npm_deps_hash` placeholder (`workestrate.toml:317`) — the real FOD hash must be computed via `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json` (`justfile:233-234`). This is `HOST-NIX` and **gets batched into Step 6** — do not attempt it here. **(d)** `ODYSSEUS_ADMIN_PASSWORD` placeholder (`workestrate.toml:47`) — must be replaced with a real SOPS secret before first boot. Secret provisioning is **HOST-only** (sops age key absent in this container). Batch into Step 6 or resolve on the host before B7. |
| **Gate / exit criteria** | Items (e) and (a) LANDED (edits applied; verified via grep and `git branch --show-current`). What remains for (e) is the runtime `validate-config` parse verification — the **first Lane A action** in Step 1 (HOST-NIX devshell; no `cc` linker in this container). Item (b) resolved or explicitly deferred with a recorded decision; items (c) and (d) acknowledged as HOST-batched (not blocking Steps 1–5). |
| **Env marker** | `verifiable-here` for (e) [APPLIED] and (a) [APPLIED] and (b); runtime parse verification for (e) is `HOST-NIX`; `HOST-NIX` for (c); `HOST-KVM` runtime / HOST-only secret provisioning for (d). |
| **Parallelization** | (e) and (a) have landed (no longer blocking). (b) is independent of everything else. (c) and (d) are deferred to Step 6. The remaining blocker for config-plane work is the runtime `validate-config` verification in Lane A (Step 1). |

### Step 1 — Lane A gate (config-plane validation)

| | |
|---|---|
| **Goal** | Prove the config-driven setup is structurally correct and produces well-formed plans — the green gate that must hold before any improvement work begins. |
| **Files to follow** | [04-baseline-validation.md](04-baseline-validation.md) §"Lane A gate suite" (A1–A7); [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Gates status snapshot". |
| **Sub-steps** | **A1:** `just verify` (full pre-merge gate suite). **A3:** `WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate validate-config`. **A4:** `just workestrate check` + `just workestrate doctor`. **A5:** Plan generation for all 5 workloads (`for name in litellm pi odysseus opencode tempest; do WORKESTRATE_HOME=... just workestrate "$name" plan > "/tmp/new/${name}.plan.txt"; done`). **A6:** `just workestrate secrets-schema` + `just workestrate generate-env-example`. **A7:** `just workestrate config list` + `context list` + `context current`. |
| **Gate / exit criteria** | All Lane A gates green (or explicitly SKIP for `store-audit` when nix is absent). `validate-config` prints `workestrate.toml is valid.`; all 5 plan files are non-empty and match the `SandboxPlan` Display format; `secrets-schema` lists exactly the 7 env-var-backed secrets. |
| **Env marker** | **RESOLVED (was disputed).** [01](01-current-state-and-prereqs.md) §"Gates status snapshot" previously claimed `just verify` overall: **PASS** in this container, contradicting [04](04-baseline-validation.md) §"Lane A gate suite" which verified no `cc` linker. **Resolution:** this container has no C toolchain (verified: `command -v cc gcc` → not found; `cargo` exists at `~/.cargo/bin/cargo` but cannot link). All cargo-linked gates (`check`, `test`, `spec-examples`, `golden-check`, `schema-check`, `scaffold-check`) are uniformly `HOST-NIX` / host-devshell gates; only the shell/python/git-based subset (`toolchain-check`, `litellm-check`, `lint-nix`, `store-audit` SKIP, `Cargo.lock` stability) is `verifiable-here`. 01 has been corrected to match 04's evidence. **Recommendation:** run the `verifiable-here` subset here; batch the cargo-based gates into Step 6. **Step 0(e) blocker LANDED** — the tempest `install_layout` field has been REMOVED from the bundle (verified via grep; see Step 0(e)). What remains is the runtime `validate-config` parse verification itself (cargo-linked, HOST-NIX devshell). The **first Lane A action** is now `workestrate validate-config` + `workestrate tempest plan` in `nix develop`; the synthetic `config.reference` gates (A2 `golden-check`) are unaffected. |
| **Parallelization** | Must complete (or be explicitly deferred) before Steps 2–8. No parallelization within Step 1 — it is the gate. |

### Step 2 — Baseline recovery (parity proof)

| | |
|---|---|
| **Goal** | Prove the new config-driven setup reproduces the pre-migration behavioral baseline for the original 5 workloads, by generating old plans from the base commit and diffing against new plans. |
| **Files to follow** | [04-baseline-validation.md](04-baseline-validation.md) §"Baseline recovery procedure" (Steps 1–5) and §"Why there is no committed golden for the original 5". |
| **Sub-steps** | **(1)** `git worktree add /tmp/workestrate-base 840e8b7` (the pre-migration base commit; verified: `git log --oneline 840e8b7 -1` → `840e8b7 docs: document workestrate run subcommand...`). **(2)** Build and run the OLD binary; capture baseline plans: `AGENTCTL_ROOT=/tmp/workestrate-base cargo run --manifest-path /tmp/workestrate-base/control/agentctl/Cargo.toml -- "$name" plan > "/tmp/baseline/${name}.plan.txt"` for each of the 5 workloads (pin cwd to `/tmp/workestrate-base` for `${CWD}`-based mounts). **(3)** Generate NEW config-driven plans: `WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate "$name" plan > "/tmp/new/${name}.plan.txt"`. **(4)** Diff after normalization (replace absolute paths with `${CWD}` token) + field-by-field comparison + semantic checklist (12 invariants from `network.rs` unit tests). **(5)** Acceptance: exact byte-match after normalization for all 5, PLUS every semantic invariant holds. |
| **Gate / exit criteria** | Parity verdict documented: either "all 5 match after normalization" or a table of justified deltas (any unjustified delta is a regression and blocks merge). |
| **Env marker** | **`HOST-NIX`** — **adjustment from the draft spine.** The draft said "verifiable-here (cargo)"; [04](04-baseline-validation.md) line 148–152 explicitly states: "This procedure is `HOST-NIX` (the old binary must be built with cargo, which requires the dev shell's `cc` linker and native crates — not available in this container; verified: `cargo run ... -- example-service plan` fails with `error: linker 'cc' not found`). Run it on a nix-capable host inside `nix develop`." The SOPS age key is **not** required (plans render secrets as `(redacted)`). **Note:** Step 3 (generating NEW config-driven plans via `just workestrate "$name" plan`) loads the personal config layer — the Step 0(e) blocker edit has **LANDED** (tempest `install_layout` removed; verified via grep), so the parse no longer hard-errors; what remains is the runtime verification itself on the host. The OLD-binary baseline plans (Step 2) are unaffected — the pre-migration binary predates the config-driven model. |
| **Parallelization** | Depends on Step 1 being green. Can run in parallel with Steps 3–5 (different working trees / branches). Batch into Step 6's host pass if cargo gates are HOST-NIX. **Note:** the Step 0(e) blocker edit has landed; the NEW-plan half (Step 3) is unblocked at the parse level — runtime verification pending on the host. |

### Step 3 — Quick wins (main rename + cwd-fallback fix)

| | |
|---|---|
| **Goal** | Land the two smallest, highest-leverage improvements that are independently shippable. |
| **Files to follow** | [06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md) (main rename); [06-improvements/05-config-reference-cwd-fallback.md](06-improvements/05-config-reference-cwd-fallback.md) (cwd-fallback fix). |
| **Sub-steps** | **(3a) Main rename — DONE (already applied in Step 0a).** The `git -C .workestrate/repos/personal branch -m master main` surgery has been applied — `verifiable-here` git surgery, no code change, no registry edit (registry already says `ref = "main"`). Verified: `git branch --show-current` → `main`, HEAD `d2cd0c3` unchanged. For the record, the verification command is: `just workestrate config list` shows `ref main, rev d2cd0c3, clean) [OK]` (note: the clone has NO origin remote, so `config update` fails until the config repo is pushed — see Step 0a). See [06-improvements/02](06-improvements/02-main-standardization.md) §4 for exact commands. **(3b) Cwd-fallback fix:** implement option (d) from [06-improvements/05](06-improvements/05-config-reference-cwd-fallback.md) — gate reference loading behind `WORKESTRATE_ALLOW_CWD_REFERENCE=1` when `project_root()` resolved via cwd, plus option (c)'s warning. This is a **code change** (touches `config/paths.rs` + `config/mod.rs` + a regression test); land it on a **separate branch/commit** from the rename. Gate: `cargo test` (new regression test). `verifiable-here`. |
| **Gate / exit criteria** | (3a) clone branch is `main`, rev unchanged, `config list` clean. (3b) regression test passes: cwd-fallback + `flake.nix` + `config.reference/` does NOT silently load the reference layer without the opt-in env. |
| **Env marker** | (3a) `verifiable-here`. (3b) `verifiable-here` (gate is `cargo test`); reproduction of the original bug is `HOST-NIX` (requires the nix-installed binary). |
| **Parallelization** | (3a) and (3b) are independent — different files, different commit types. **Note:** 3b is the **only improvement that is a bug fix** (not an enhancement); it could jump the queue ahead of Steps 4–8 if prioritized, since it closes a silent config-discovery backdoor that affects every nix-installed-binary invocation from a workbench checkout. |

### Step 4 — Dogfooding Phase 0 guardrails

| | |
|---|---|
| **Goal** | Close the config-reference cwd-fallback backdoor for the dogfooding topology via the Phase 0 env-pinning wrapper (convention-based, no code change), then begin structural hardening with B1. |
| **Files to follow** | [06-improvements/03-dogfooding.md](06-improvements/03-dogfooding.md) §4 (Phase 0 wrapper) and §5.1 (B1 self-home mount guard). |
| **Sub-steps** | **(4a) Phase 0 wrapper:** create `workestrate-driver.sh` setting `WORKESTRATE_HOME=~/.workestrate-driver`, `WORKESTRATE_CONFIG_DIR=$WORKESTRATE_HOME`, `AGENTCTL_ROOT=~/Development/ai-workbench`, `WORKESTRATE_NO_PROJECT_CONFIG=1`, then `exec workestrate "$@"`. `verifiable-here` (wrapper script, no code change). Verify invariants per §6.4 of the spec (`.workestrate/` absent from worktree, driver home ≠ worktree, `WORKESTRATE_CONFIG_DIR` set, reference layer not in provenance). **(4b) B1 self-home mount guard:** at plan time, after mount resolution, check each resolved host path against the driver's resolved `WORKESTRATE_HOME` (and `state/`, `secrets/`, `repos/`); refuse the plan (fail-closed) if a mount's host path is the home or a descendant/ancestor. `verifiable-here` (static check at plan time; unit test: workload mounting `${CWD}` where cwd == `WORKESTRATE_HOME` → plan fails). |
| **Gate / exit criteria** | (4a) wrapper invariants all pass. (4b) unit test passes: self-home mount → plan rejected. |
| **Env marker** | (4a) `verifiable-here`. (4b) `verifiable-here` (static plan-time check). |
| **Parallelization** | 4a and 4b are independent. Can run in parallel with Steps 3 and 5. |

### Step 5 — Experiment home probes (config-plane)

| | |
|---|---|
| **Goal** | Exercise the config-driven setup's layering, context, instance, mount, secret, and seed_file surfaces via the disposable experiment home — proving the config plane works end-to-end without touching the real bundle. |
| **Files to follow** | [03-sibling-config-setup.md](03-sibling-config-setup.md) §2 (bootstrap) and §4 (probes P1–P5). |
| **Sub-steps** | Bootstrap: `export WORKESTRATE_HOME=/tmp/workestrate-exp` → `just workestrate config new exp --empty` → (optional) `just workestrate config trust /tmp/workestrate-exp-proj` → verify with `config list` / `context current` / `validate-config` / `doctor`. Then run probes: **P1** contexts (define, switch, `--show-source` provenance); **P2** instances (`--port-offset` plan-plane); **P3** mounts (add probe mount, verify plan rendering); **P4** secrets layering (`secrets-schema`, `secrets = "none"` toggle); **P5** `seed_files` (validate + plan). Cleanup per §6. |
| **Gate / exit criteria** | All 5 probes produce the expected outcomes documented in [03](03-sibling-config-setup.md) §4. Real bundle never touched (guard invariant: `test "$WORKESTRATE_HOME" = "/tmp/workestrate-exp"`). |
| **Env marker** | `verifiable-here` (all probes are config-plane / plan-plane only; runtime execution is `HOST-KVM` — see [05](05-host-validation.md)). |
| **Parallelization** | Can run concurrently with Steps 3–4 (uses a disposable `/tmp` home, does not touch the real bundle or the source tree). |

### Step 6 — HOST batch (single host pass: B1–B12)

| | |
|---|---|
| **Goal** | Execute every `HOST-NIX` and `HOST-KVM` gate in **one batched pass** on the host — mirroring the migration doc's batching principle: "The host-batched HOST-NIX and HOST-KVM gates run in one pass at the end so the host environment is not context-switched mid-remediation" ([docs/migration/40-migration-process.md](../migration/40-migration-process.md) lines 368–369). |
| **Files to follow** | [05-host-validation.md](05-host-validation.md) (B1–B12); [04-baseline-validation.md](04-baseline-validation.md) §"Baseline recovery procedure" (if not already done in Step 2); [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"HOST-NIX" and §"HOST-KVM" gate tables. |
| **Sub-steps** | **B1** `nix build .#workestrate` (CLI binary). **B2** `nix build .#workestrator-pi` + `nix build .#tempest` (nix-layered images; includes tempest FOD hash pin from Step 0c: `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`, copy into both `nix/packages/tempest.nix` and `workestrate.toml:317`). **B3** `workestrate doctor` (all checks OK). **B4** `workestrate validate-config`. **B5** `workestrate litellm up` + `curl /health/liveliness` → 200. **B6** LiteLLM smoke test (`GET /v1/models`, `GET /health/liveliness`, `POST /v1/chat/completions` → 200). **B7** `workestrate odysseus up` + `curl /api/health` → 200 (requires real `ODYSSEUS_ADMIN_PASSWORD` from Step 0d). **B8** `workestrate pi exec` (interactive TUI attach). **B9** `workestrate opencode exec` + `workestrate tempest exec` (interactive). **B10** Instance lifecycle (`--new`, `--port-offset`, `down --all-instances`, `down-all --yes`). **B11** Runtime parity (plan parity from Lane A + service boot; interactive egress check via `workestrate pi exec`). **B12** Teardown (`down-all --yes`, `clean --yes`, `ps` empty). Also batch in: `just verify-full` (adds `nix build .#workestrate`), `just generate-schema` (schema regen, requires devshell RUSTFLAGS/libcap-ng), and the baseline recovery from Step 2 if not already done on the host. |
| **Gate / exit criteria** | All B1–B12 green per the capability → proof → acceptance matrix in [05-host-validation.md](05-host-validation.md). Tempest FOD hash pinned. `ODYSSEUS_ADMIN_PASSWORD` real. Runtime parity verdict recorded. |
| **Env marker** | `HOST-NIX` (B1, B2) + `HOST-KVM` (B3–B12). |
| **Parallelization** | Strictly ordered within the host pass (B1 → B2 → B3 → ... → B12). This is the single host trip — everything HOST-NIX/HOST-KVM lands here. |

### Step 7 — Track A: mount filtering (WP1–WP3 + Phase 0 spike + WP4)

| | |
|---|---|
| **Goal** | Implement the mount filtering/shadowing extension: schema + glob (WP1), policy + trust (WP2), render + audit (WP3) — all `verifiable-here` — then the Phase 0 KVM spike and WP4 (runtime shadows) on the host. |
| **Files to follow** | [06-improvements/01-mount-filtering-shadowing.md](06-improvements/01-mount-filtering-shadowing.md) §7 (phased plan) and §4–§5 (schema + policy). |
| **Sub-steps** | **WP1** (schema + glob): extend `MountPlan` (`plan.rs:89-93`) with `exclude`, `shadow`, `allow_sensitive` (all `#[serde(default)]`); add `ShadowMount` / `ShadowKind` types; add `globset = "0.4"` to `Cargo.toml`; regen `schemas/workestrate.schema.json` via `just generate-schema` (HOST-NIX). Gate: `just check` + `just test` + `just schema-check` + `just golden-check` (3 synthetic workloads stay green — no shadow fields, no output change). **WP2** (policy + trust): add `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` to `policy.rs`; implement trust gate for `allow_sensitive` (hard error from untrusted layers); implement plan-time expansion (§5.3). Gate: `just check` + `just test`. **WP3** (render + audit): extend `SandboxPlan::Display` with conditional additive rendering; add `--show-mounts` and `--mount-ls` flags. Gate: `just golden-check` (green without regeneration) + `just check` + `just test`. **Phase 0 spike** (HOST-KVM): minimal Rust PoC confirming nested mount ordering works, `readonly()` produces `EROFS`, `tmpfs()` shadows correctly. **WP4** (runtime shadows, HOST-KVM): extend `apply_plan_mounts` to emit shadow `volume()` calls after the parent bind. Gate: Phase 0 spike must have passed; live KVM test confirming `/work/.env` is empty and read-only inside the guest. |
| **Gate / exit criteria** | WP1–WP3 green (`verifiable-here`); Phase 0 spike report documents whether nested ordering works; WP4 live KVM test passes. If Phase 0 spike fails → WP5 (staging-copy fallback) is activated instead. |
| **Env marker** | WP1–WP3: `verifiable-here` (schema regen is `HOST-NIX` per `justfile:95-96`). Phase 0 + WP4: `HOST-KVM`. |
| **Parallelization** | WP1 → WP2 → WP3 are sequentially ordered (WP2 depends on WP1 types; WP3 depends on WP1 + WP2). **The Phase 0 spike can fold into the Step 6 host batch** to avoid a second host trip — [06-improvements/01](06-improvements/01-mount-filtering-shadowing.md) §7 Phase 0 says the spike is a standalone HOST-KVM PoC with no dependency on WP1–WP3, so it can run during the same host pass as B1–B12. WP4 depends on the spike result and must run after it (potentially a second host trip if the spike was batched into Step 6 and WP4 code wasn't ready yet). |

### Step 8a — Naming consistency (branch `migration/tool-model`, THIS BRANCH)

| | |
|---|---|
| **Goal** | Purge `workestrator` residue repo-internally; standardize on `workestrate`. Local rename only — no remote/checkout-dir changes. |
| **Files to follow** | [06-improvements/07-naming-consistency.md](06-improvements/07-naming-consistency.md) (decision, inventory, rename map, FLAG §4, dir-rename implications §6). |
| **Sub-steps** | Phase 2 spec (landed with this edit). Phase 3 docs sweep. Phase 4 code/nix/templates rename (template `git mv`, flake attr graph, image name, JSON keys, scaffold strings). Phase 5 final report. |
| **Gate / exit criteria** | `grep -ri workestrator` returns only the spec §5 residuals; `just lint-nix` shows only the known foreign `docs/nix` failure; renamed flake attrs eval. Cargo gates PENDING (`HOST-NIX`, no `cc` here) — batch into Step 6. |
| **Env marker** | `verifiable-here` (grep/lint-nix/nix eval); cargo + `nix build` are `HOST-NIX`. |
| **Parallelization** | Independent of Steps 0–8 code work; must land before the user renames the origin/checkout dir (spec §6). |

---

### Step 8 — Remainder (B2/B3 dogfooding, WP5 conditional, CLI authoring DEFERRED)

| | |
|---|---|
| **Goal** | Land the remaining hardening and deferred work that is not on the critical path. |
| **Files to follow** | [06-improvements/03-dogfooding.md](06-improvements/03-dogfooding.md) §5.2 (B2), §5.3 (B3); [06-improvements/01-mount-filtering-shadowing.md](06-improvements/01-mount-filtering-shadowing.md) §7 WP5; [06-improvements/04-cli-config-authoring.md](06-improvements/04-cli-config-authoring.md). |
| **Sub-steps** | **B2** (config-free teardown verification): add a regression test that runs `down`/`clean` with a corrupt/absent registry and asserts graceful fallback. The B2 premise ("teardown requires config load") was **refuted** — teardown already avoids `load_config()`; this is a verification + test item, not a behavior change. `verifiable-here`. **B3** (spawn provenance): extend `SandboxInstanceRecord` with a `spawned_by` field (driver binary path, resolved `WORKESTRATE_HOME`, `AGENTCTL_ROOT`); populate at registration; display in `ps`. `verifiable-here` (state-file round-trip) + `HOST-KVM` (live sandbox verification). **WP5** (staging-copy fallback): **only if the Phase 0 spike failed** in Step 7 — copy the mount tree minus excluded paths into a state-dir staging location and bind-mount the staging copy. `HOST-KVM`. **CLI authoring (04):** `DEFERRED` — do not start until [02-config-requirements.md](02-config-requirements.md) sign-off is recorded. When unblocked, the CLI must be additive-tolerant of 01's mount schema (ship after 01 WP1, or be schema-driven per §4.6). |
| **Gate / exit criteria** | B2 regression test passes. B3 round-trip test passes + `ps` displays `spawned_by`. WP5 (if triggered) live KVM test passes. CLI authoring remains `DEFERRED` until sign-off. |
| **Env marker** | B2: `verifiable-here`. B3: `verifiable-here` + `HOST-KVM`. WP5: `HOST-KVM`. CLI: `verifiable-here` (when unblocked). |
| **Parallelization** | B2, B3, and WP5 (if triggered) are independent of each other. CLI authoring is gated on an external sign-off, not on any code step here. |

---

## Parallelization table

| Step | Can run concurrently with | Strictly ordered after | Blocking dependency |
|---|---|---|---|
| 0 (bundle hygiene) | 0(e) first; then 0(a) ∥ 0(b); 0(c)/0(d) deferred to Step 6 | — | 0(e) blocks all config loads (CRITICAL) |
| 1 (Lane A gate) | — | Step 0 (acknowledged) | Must be green before Steps 2–8 |
| 2 (baseline recovery) | Steps 3, 4, 5 | Step 1 | `cc` linker (HOST-NIX per 04) |
| 3 (quick wins) | Steps 2, 4, 5 | Step 1 | 3(a) is standalone; 3(b) is the only bug-fix improvement (can jump queue) |
| 4 (dogfooding Phase 0) | Steps 2, 3, 5 | Step 1 | 4(a) wrapper is standalone; 4(b) B1 is a code change |
| 5 (experiment probes) | Steps 2, 3, 4 | Step 1 | Uses disposable `/tmp` home; does not touch real bundle |
| 6 (HOST batch) | — | Steps 1–5 (or their HOST-deferred subsets) | Single host trip; batches all HOST-NIX/HOST-KVM gates |
| 7 (Track A) | WP1–WP3 ∥ Steps 3–5; Phase 0 spike ∥ Step 6 | Step 1 (for WP1–WP3); Step 6 (for Phase 0 spike if batched) | Phase 0 spike gates WP4; WP5 conditional on spike failure |
| 8 (remainder) | B2 ∥ B3 ∥ WP5 ∥ CLI (when unblocked) | Step 7 (for WP5); external sign-off (for CLI) | CLI gated on `02-config-requirements.md` sign-off |

---

## Definition of done (whole effort)

- [ ] **Lane A green** — `just verify` passes on the host (the `verifiable-here` subset passes in-container; the cargo-based gates require `cc` and run in Step 6). Lane A against the real bundle: the Step 0(e) blocker edit has LANDED (tempest `install_layout` removed); the first Lane A action is now `workestrate validate-config` + `workestrate tempest plan` in `nix develop`.
- [ ] **Baseline parity verdict documented** — all 5 workloads match after normalization, or every delta is justified in a table ([04-baseline-validation.md](04-baseline-validation.md) §5 acceptance criteria).
- [ ] **Host batch B1–B12 green** — every step in [05-host-validation.md](05-host-validation.md) passes per the capability → proof → acceptance matrix.
- [x] **Main rename done** — personal clone branch is `main`, `config list` shows `ref main, rev d2cd0c3, clean) [OK]` ([06-improvements/02](06-improvements/02-main-standardization.md)). **(applied; rev `d2cd0c3` unchanged; clone has no origin remote — `config update` pending push)**
- [ ] **Dogfooding guardrails active** — Phase 0 env-pinning wrapper in use; B1 self-home mount guard merged (or explicitly deferred with rationale).
- [ ] **Track A WP1–WP4 merged** (or WP5 triggered) — mount filtering/shadowing schema, policy, render, and runtime shadows landed; or, if the Phase 0 spike failed, the staging-copy fallback (WP5) is landed instead.
- [ ] **Improvements index statuses updated** — [06-improvements/00-index.md](06-improvements/00-index.md) master table reflects the actual post-execution status of each spec (SPEC → IMPLEMENTED / MERGED / DEFERRED).
- [x] **Tempest `install_layout` drift removed** — `install_layout = "app"` removed from tempest's `binary` inline table in `.workestrate/repos/personal/workestrate.toml:317` (Bundle fix e, CRITICAL). **(edit applied; runtime parse verification pending — first Lane A action)**
- [ ] **Tempest FOD hash pinned** — `npm_deps_hash` in both `nix/packages/tempest.nix` and `workestrate.toml:317` is the real computed hash.
- [ ] **`ODYSSEUS_ADMIN_PASSWORD` real** — placeholder replaced with a SOPS-encrypted secret; odysseus first boot succeeds (B7).

---

## Note on host trips

**Host trips are expensive.** Every `HOST-NIX` and `HOST-KVM` gate requires a
context switch to a nix-capable, KVM-capable host — this container has neither
(`command -v nix` → not found; `ls /dev/kvm` → not found; see
[01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §Environment
honesty). The batching principle from the migration doc applies:

> "The host-batched HOST-NIX and HOST-KVM gates run in one pass at the end so
> the host environment is not context-switched mid-remediation."
> — [docs/migration/40-migration-process.md](../migration/40-migration-process.md)
> lines 368–369.

**Batch everything `HOST-NIX`/`HOST-KVM` into the Step 6 single pass wherever
possible:**

- The tempest FOD hash computation (Step 0c).
- The `ODYSSEUS_ADMIN_PASSWORD` secret provisioning (Step 0d).
- The cargo-based Lane A gates (`check`, `test`, `spec-examples`, `golden-check`,
  `schema-check`, `scaffold-check`) — this container has no `cc` linker
  (verified: `command -v cc gcc` → not found; the 01-vs-04 contradiction on
  this is RESOLVED — cargo gates are uniformly HOST-NIX).
- The baseline recovery procedure (Step 2) if not already done on the host.
- `just verify-full` (adds `nix build .#workestrate`).
- `just generate-schema` (schema regen, requires devshell RUSTFLAGS/libcap-ng).
- The Phase 0 KVM spike (Step 7) — it is a standalone HOST-KVM PoC with no
  dependency on WP1–WP3 code, so it can run during the same host pass as
  B1–B12. The only reason to make a second host trip is if WP4 code (which
  depends on the spike result) was not ready during the first pass.
