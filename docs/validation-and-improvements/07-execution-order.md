# 07 — Execution Order (recommended sequence)

> **STATUS: READY-TO-EXECUTE**
> Prerequisites / see-also: [README.md](README.md) ·
> [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [04-baseline-validation.md](04-baseline-validation.md) ·
> [05-host-validation.md](05-host-validation.md) ·
> [06-improvements/00-index.md](06-improvements/00-index.md)

This document defines the **recommended execution sequence** for the entire
validation-and-improvements effort: bundle hygiene, repo-local-home
retirement, Lane A config-plane validation, baseline recovery, quick-win
improvements, dogfooding guardrails, experiment-home probes, the batched host
pass, Track A mount filtering, and the deferred remainder. A contextless
operator follows it top-to-bottom.

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
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). INCLUDES cargo-linked gates run via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH — prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c '<cmd>'`; verified 2026-07-29: `cc --version` → gcc 15.2.0, `cargo check` compiles in ~27s). A bare shell (outside `nix develop`) has no `cc`. |
| `HOST-NIX` | Requires nix on the user's host: `nix build` image builds, `nix run nixpkgs#...` prefetch jobs, full `just verify-full`, and `just generate-schema` (devshell RUSTFLAGS/libcap-ng). Cargo-linked `just` gates are NOT here — they run in-container via `nix develop` (see `verifiable-here`). |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

---

## Recommended sequence

### Step 0 — Bundle hygiene

| | |
|---|---|
| **Goal** | Resolve the bundle-drift items flagged in [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle fixes needed" that are prerequisites for everything downstream. |
| **Files to follow** | [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle fixes needed" (items a–e); [06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md) for item (a). |
| **Sub-steps** | **(e) [CRITICAL — first] APPLIED.** `install_layout = "app"` has been REMOVED from tempest's `binary` inline table in `.workestrate/repos/personal/workestrate.toml:317` (verified: `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no match). `BinarySpec` (`control/agentctl/src/config/types.rs:44-52`) has `#[serde(deny_unknown_fields)]` and no `install_layout` field (the nix-side param was removed as a silent no-op, `nix/lib/recipes/npm-build.nix:16-25`). **PENDING:** runtime parse verification — `workestrate validate-config` is cargo-linked and runs in this container via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`; verified 2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`); it is the **first Lane A action** (`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c 'just workestrate validate-config'`). `verifiable-here` (config edit applied; runtime parse verification is `verifiable-here` via `nix develop`). **(a) APPLIED.** Branch rename `master` → `main` in the personal clone is DONE — this is [06-improvements/02](06-improvements/02-main-standardization.md) in full; `verifiable-here` (plain `git branch -m`, no remote, no registry edit). Verified: `git -C .workestrate/repos/personal branch --show-current` → `main`, HEAD `d2cd0c3` unchanged (matches `config.toml:10`). **Still pending:** the clone has NO origin remote (verified: `git remote -v` → empty), so `workestrate config update` fails for that reason until the config repo is pushed to a remote. **(b)** `scratch/` vs `cache/` directory naming — **open decision** (flag for resolution): `.workestrate/scratch/` exists but ADR 0023 lines 73–82 specify `cache/` in the single-home layout. Either rename `scratch/` → `cache/` or amend the ADR. `verifiable-here`. **(c)** Tempest `npm_deps_hash` placeholder (`workestrate.toml:317`) — the real FOD hash must be computed via `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json` (`justfile:233-234`). This is `HOST-NIX` and **gets batched into Step 6** — do not attempt it here. **(d)** `ODYSSEUS_ADMIN_PASSWORD` placeholder (`workestrate.toml:47`) — must be replaced with a real SOPS secret before first boot. Secret provisioning is **HOST-only** (sops age key absent in this container). Batch into Step 6 or resolve on the host before B7. |
| **Gate / exit criteria** | Items (e) and (a) LANDED (edits applied; verified via grep and `git branch --show-current`). What remains for (e) is the runtime `validate-config` parse verification — the **first Lane A action** in Step 1 (runnable in this container via `nix develop`, store-path prefix; bare shell lacks `cc`). Item (b) resolved or explicitly deferred with a recorded decision; items (c) and (d) acknowledged as HOST-batched (not blocking Steps 1–5). |
| **Env marker** | `verifiable-here` for (e) [APPLIED] and (a) [APPLIED] and (b); runtime parse verification for (e) is `verifiable-here` via `nix develop`; `HOST-NIX` for (c); `HOST-KVM` runtime / HOST-only secret provisioning for (d). |
| **Parallelization** | (e) and (a) have landed (no longer blocking). (b) is independent of everything else. (c) and (d) are deferred to Step 6. The remaining blocker for config-plane work is the runtime `validate-config` verification in Lane A (Step 1). |

### Step 0.5 — Retire the repo-local tool home (spec 08) — **DONE**

| | |
|---|---|
| **Goal** | Execute [06-improvements/08-no-repo-local-home.md](06-improvements/08-no-repo-local-home.md): the tool home becomes user-global `~/.workestrate` ONLY (ADR 0023 default) — the repo-local bundle `.workestrate/` and all repo-local-home machinery (`.envrc` pin, `scripts/local-xdg.sh`, `scripts/migrate-xdg-to-repo.sh`, `.gitignore:46` entry, and the trusted-ancestor discovery tier in `paths.rs`) are retired. Inserted as an early step BEFORE Lane A / the host batch **because it changes the paths those steps reference** (`WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate` spellings in Steps 1–6 go away; default resolution finds `~/.workestrate`). **AMENDED by spec 10:** step (a) now follows the **working-copy model** (no standalone `/home/node/Development/workestrate-personal` sibling; the personal working repo lives at `~/.workestrate/config-repos/personal` — or `repos/personal` until the rename lands — and gets a remote when ready); see [06-improvements/10-config-repos-as-working-copies.md](06-improvements/10-config-repos-as-working-copies.md). |
| **Files to follow** | [06-improvements/08-no-repo-local-home.md](06-improvements/08-no-repo-local-home.md) (the full ordered plan a–g; do not re-derive). |
| **Sub-steps** | **ALL DONE.** (a) Personal config preserved as the in-home working copy at `~/.workestrate/config-repos/personal` (clone HEAD `c41a707` "fix(tempest): remove retired install_layout field" on `main`; no remote — local-path classification; `ref`/`rev` lines removed from registry). (b) Repo-local bundle copied to `~/.workestrate`; `scratch/` dropped; `config.toml [configs.personal] url` → `/home/node/.workestrate/config-repos/personal`. (c) Machinery removal commit `418530a` (`.envrc` pin removed; `scripts/local-xdg.sh` + `scripts/migrate-xdg-to-repo.sh` deleted; `/.workestrate/` gitignore entry removed; justfile `local-setup` recipe + README activation refs cleaned). (d) Repo-local bundle `/home/node/Development/ai-workbench/.workestrate` deleted. (e) Discovery tier removed (commit `bef1c37`); `workestrate home init` added (commit `3894fb7`); `repos/` → `config-repos/` rename (commit `d7c5a83`); dirty-guard test (commit `bd99481`). (f) ADR 0023 addendum follow-through landed in `bef1c37`. (g) Compose/mount guidance updated in README (this wave). **Home adopted into git** as root commit `a42e597` "home: adopt registry + personal config repo" — `git ls-files` shows ONLY `.gitignore` + `config.toml`; no gitlinks (mode-160000 count = 0); pre-commit hook did not block legitimate files. `workestrate home init` ran idempotently (second run: "already initialized … nothing to do"). |
| **Gate / exit criteria** | **ALL MET.** (a) In-home working copy exists at `~/.workestrate/config-repos/personal` with `c41a707` as HEAD. (b) `workestrate config list` with NO `WORKESTRATE_HOME` export shows `personal` @ `c41a707` (rev unknown — local-path) from `~/.workestrate` (verified: "personal: /home/node/.workestrate/config-repos/personal (ref main, rev unknown, clean) [OK]"). `workestrate validate-config` → "workestrate.toml is valid." exit 0 — **runtime-proves the `install_layout` removal**. `workestrate doctor` → "home: OK (/home/node/.workestrate (Default))", "config_repos: OK (1 repo(s) checked)"; exit 1 solely due to `dev_kvm` (expected — no KVM in container). (c) Machinery commit `418530a` landed. (d) Bundle deleted. (e) Discovery tier removed (`bef1c37`); cargo gates passed during STAGE A. New precedence documented: **flag (`--home`) > env > legacy XDG > default `~/.workestrate`** (no discovery). **Cargo.lock stability:** verified stable (sha256 `7580f399…` identical across 2 consecutive `cargo check --locked` runs; no commit needed). |
| **Env marker** | All steps `verifiable-here` (ops + code via `nix develop`). |
| **Parallelization** | **COMPLETE.** Steps 1–6 now run against the default-resolved home `~/.workestrate` (no `WORKESTRATE_HOME` export needed). |

### Step 1 — Lane A gate (config-plane validation)

| | |
|---|---|
| **Goal** | Prove the config-driven setup is structurally correct and produces well-formed plans — the green gate that must hold before any improvement work begins. |
| **Files to follow** | [04-baseline-validation.md](04-baseline-validation.md) §"Lane A gate suite" (A1–A7); [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Gates status snapshot". |
| **Sub-steps** | **A1:** `just verify` (full pre-merge gate suite). **A3:** `WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate validate-config`. **A4:** `just workestrate check` + `just workestrate doctor`. **A5:** Plan generation for all 5 workloads (`for name in litellm pi odysseus opencode tempest; do WORKESTRATE_HOME=... just workestrate workload plan "$name" > "/tmp/new/${name}.plan.txt"; done`). **A6:** `just workestrate secrets-schema` + `just workestrate generate-env-example`. **A7:** `just workestrate config list` + `context list` + `context current`. |
| **Gate / exit criteria** | All Lane A gates green (or explicitly SKIP for `store-audit` when nix is absent). `validate-config` prints `workestrate.toml is valid.`; all 5 plan files are non-empty and match the `SandboxPlan` Display format; `secrets-schema` lists exactly the 7 env-var-backed secrets. |
| **Env marker** | **RESOLVED (was disputed).** [01](01-current-state-and-prereqs.md) §"Gates status snapshot" previously claimed `just verify` overall: **PASS** in this container, contradicting [04](04-baseline-validation.md) §"Lane A gate suite" which verified no `cc` linker in a bare shell. **Resolution (superseded 2026-07-29):** the earlier "no `cc` ⇒ cargo gates are HOST-NIX" resolution was superseded by the discovery that nix IS installed in this container at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` (not on PATH) and `nix develop` provides a full C toolchain (verified 2026-07-29: `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c 'cc --version'` → gcc 15.2.0; `cargo 1.97.1`, `rustc 1.97.1`; `cargo check` compiles in ~27s; first devshell build takes minutes). All cargo-linked gates (`check`, `test`, `spec-examples`, `golden-check`, `schema-check`, `scaffold-check`) are runnable in this container via `nix develop` (`verifiable-here`); a bare shell still lacks `cc` and runs only the shell/python/git-based subset (`toolchain-check`, `litellm-check`, `lint-nix`, `store-audit` SKIP, `Cargo.lock` stability). Only `nix build` image builds, `nix run nixpkgs#...` prefetch jobs, full `just verify-full`, and `just generate-schema` (devshell RUSTFLAGS/libcap-ng) remain `HOST-NIX`; KVM runtime and sops secret provisioning remain host gates. 01 and 04 have been corrected to match this evidence. **Recommendation:** run the full `just verify` here via `nix develop`; batch only the genuine host gates (nix build, KVM, sops, generate-schema) into Step 6. **Step 0(e) blocker LANDED** — the tempest `install_layout` field has been REMOVED from the bundle (verified via grep; see Step 0(e)). What remains is the runtime `validate-config` parse verification itself (cargo-linked, runnable here via `nix develop`). The **first Lane A action** is now `workestrate validate-config` + `workestrate workload plan tempest` in `nix develop`; the synthetic `config.reference` gates (A2 `golden-check`) are unaffected. |
| **Parallelization** | Must complete (or be explicitly deferred) before Steps 2–8. No parallelization within Step 1 — it is the gate. |

### Step 2 — Baseline recovery (parity proof)

| | |
|---|---|
| **Goal** | Prove the new config-driven setup reproduces the pre-migration behavioral baseline for the original 5 workloads, by generating old plans from the base commit and diffing against new plans. |
| **Files to follow** | [04-baseline-validation.md](04-baseline-validation.md) §"Baseline recovery procedure" (Steps 1–5) and §"Why there is no committed golden for the original 5". |
| **Sub-steps** | **(1)** `git worktree add /tmp/workestrate-base 840e8b7` (the pre-migration base commit; verified: `git log --oneline 840e8b7 -1` → `840e8b7 docs: document workestrate run subcommand...`). **(2)** Build and run the OLD binary; capture baseline plans: `AGENTCTL_ROOT=/tmp/workestrate-base cargo run --manifest-path /tmp/workestrate-base/control/agentctl/Cargo.toml -- "$name" plan > "/tmp/baseline/${name}.plan.txt"` for each of the 5 workloads (pin cwd to `/tmp/workestrate-base` for `${CWD}`-based mounts). **(3)** Generate NEW config-driven plans: `WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate workload plan "$name" > "/tmp/new/${name}.plan.txt"`. **(4)** Diff after normalization (replace absolute paths with `${CWD}` token) + field-by-field comparison + semantic checklist (12 invariants from `network.rs` unit tests). **(5)** Acceptance: exact byte-match after normalization for all 5, PLUS every semantic invariant holds. |
| **Gate / exit criteria** | Parity verdict documented: either "all 5 match after normalization" or a table of justified deltas (any unjustified delta is a regression and blocks merge). |
| **Env marker** | **`verifiable-here` via `nix develop`** — **adjustment from the draft spine.** The draft said "verifiable-here (cargo)"; [04](04-baseline-validation.md) §"Baseline recovery procedure" now states the procedure is runnable in this container via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`; verified 2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`; `cargo check` compiles in ~27s; first devshell build takes minutes). The SOPS age key is **not** required (plans render secrets as `(redacted)`). **Note:** Step 3 (generating NEW config-driven plans via `just workestrate workload plan "$name"`) loads the personal config layer — the Step 0(e) blocker edit has **LANDED** (tempest `install_layout` removed; verified via grep), so the parse no longer hard-errors; what remains is the runtime verification itself, runnable here via `nix develop`. The OLD-binary baseline plans (Step 2) are unaffected — the pre-migration binary predates the config-driven model. |
| **Parallelization** | Depends on Step 1 being green. Can run in parallel with Steps 3–5 (different working trees / branches). Runnable in this container via `nix develop`; only batch into Step 6's host pass if you prefer to amortize the devshell build. **Note:** the Step 0(e) blocker edit has landed; the NEW-plan half (Step 3) is unblocked at the parse level — runtime verification runnable here via `nix develop`. |

### Step 3 — Quick wins (main rename + cwd-fallback fix)

| | |
|---|---|
| **Goal** | Land the two smallest, highest-leverage improvements that are independently shippable. |
| **Files to follow** | [06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md) (main rename); [06-improvements/05-config-reference-cwd-fallback.md](06-improvements/05-config-reference-cwd-fallback.md) (cwd-fallback fix). |
| **Sub-steps** | **(3a) Main rename — DONE (already applied in Step 0a).** The `git -C .workestrate/repos/personal branch -m master main` surgery has been applied — `verifiable-here` git surgery, no code change, no registry edit (registry already says `ref = "main"`). Verified: `git branch --show-current` → `main`, HEAD `d2cd0c3` unchanged. For the record, the verification command is: `just workestrate config list` shows `ref main, rev d2cd0c3, clean) [OK]` (note: the clone has NO origin remote, so `config update` fails until the config repo is pushed — see Step 0a). See [06-improvements/02](06-improvements/02-main-standardization.md) §4 for exact commands. **(3b) Cwd-fallback fix:** implement option (d) from [06-improvements/05](06-improvements/05-config-reference-cwd-fallback.md) — gate reference loading behind `WORKESTRATE_ALLOW_CWD_REFERENCE=1` when `project_root()` resolved via cwd, plus option (c)'s warning. This is a **code change** (touches `config/paths.rs` + `config/mod.rs` + a regression test); land it on a **separate branch/commit** from the rename. Gate: `cargo test` (new regression test). `verifiable-here`. |
| **Gate / exit criteria** | (3a) clone branch is `main`, rev unchanged, `config list` clean. (3b) regression test passes: cwd-fallback + `flake.nix` + `config.reference/` does NOT silently load the reference layer without the opt-in env. |
| **Env marker** | (3a) `verifiable-here`. (3b) `verifiable-here` via `nix develop` (gate is `cargo test`, runnable in this container via the store-path prefix; bare shell lacks `cc`); reproduction of the original bug is `HOST-NIX` (requires the `nix build .#workestrate`-installed binary). |
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
| **Sub-steps** | Bootstrap: `export WORKESTRATE_HOME=/tmp/workestrate-exp` → `just workestrate config new exp --empty` → (optional) `just workestrate config trust /tmp/workestrate-exp-proj` → verify with `config list` / `context current` / `validate-config` / `doctor`. Then run probes: **P1** contexts (define, switch, `--show-source` provenance); **P2** instances (parallel-slot plan-plane: `plan --instance <id>` shows the prospective per-instance bind); **P3** mounts (add probe mount, verify plan rendering); **P4** secrets layering (`secrets-schema`, `secrets = "none"` toggle); **P5** `seed_files` (validate + plan). Cleanup per §6. |
| **Gate / exit criteria** | All 5 probes produce the expected outcomes documented in [03](03-sibling-config-setup.md) §4. Real bundle never touched (guard invariant: `test "$WORKESTRATE_HOME" = "/tmp/workestrate-exp"`). |
| **Env marker** | `verifiable-here` (all probes are config-plane / plan-plane only; runtime execution is `HOST-KVM` — see [05](05-host-validation.md)). |
| **Parallelization** | Can run concurrently with Steps 3–4 (uses a disposable `/tmp` home, does not touch the real bundle or the source tree). |

### Step 6 — HOST batch (single host pass: B1–B12)

| | |
|---|---|
| **Goal** | Execute every `HOST-NIX` and `HOST-KVM` gate in **one batched pass** on the host — mirroring the migration doc's batching principle: "The host-batched HOST-NIX and HOST-KVM gates run in one pass at the end so the host environment is not context-switched mid-remediation" ([docs/migration/40-migration-process.md](../migration/40-migration-process.md) lines 368–369). |
| **Files to follow** | [05-host-validation.md](05-host-validation.md) (B1–B12); [04-baseline-validation.md](04-baseline-validation.md) §"Baseline recovery procedure" (if not already done in Step 2); [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"HOST-NIX" and §"HOST-KVM" gate tables. |
| **Sub-steps** | **B1** `nix build .#workestrate` (CLI binary). **B2** `nix build .#workestrate-pi` + `nix build .#tempest` (nix-layered images; includes tempest FOD hash pin from Step 0c: `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`, copy into both `nix/packages/tempest.nix` and `workestrate.toml:317`). **B3** `workestrate doctor` (all checks OK). **B4** `workestrate validate-config`. **B5** `workestrate workload up litellm` + `curl /health/liveliness` → 200. **B6** LiteLLM smoke test (`GET /v1/models`, `GET /health/liveliness`, `POST /v1/chat/completions` → 200). **B7** `workestrate workload up odysseus` + `curl /api/health` → 200 (requires real `ODYSSEUS_ADMIN_PASSWORD` from Step 0d). **B8** `workestrate workload exec pi` (interactive TUI attach). **B9** `workestrate workload exec opencode` + `workestrate workload exec tempest` (interactive). **B10** Instance lifecycle (`--new` on per-instance IPs, `--port-auto`, `down --all-instances`, `down-all --yes`; Experiment E1 guest-reachability probe). **CODE DONE (spec 12, 2026-07-30):** per-instance addressing + discovery-lite fully landed — Wave 1 `9107b87` / `de9aa62` / `f9fd2f0` / `c5837e7`, Wave 2 `4adad3f` / `7b65ad1` / `39c1694`; what remains in B10 is the KVM runtime verification + the E1 probe (the deferred binding decision stays NEEDS-KVM per ADR 0026). **B11** Runtime parity (plan parity from Lane A + service boot; interactive egress check via `workestrate workload exec pi`). **B12** Teardown (`down-all --yes`, `clean --yes`, `ps` empty). Also batch in: `just verify-full` (adds `nix build .#workestrate`), `just generate-schema` (schema regen, requires devshell RUSTFLAGS/libcap-ng), and the baseline recovery from Step 2 if not already done on the host. |
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
| **Gate / exit criteria** | `grep -ri workestrator` returns only the spec §5 residuals; `just lint-nix` shows only the known foreign `docs/nix` failure; renamed flake attrs eval. Cargo gates runnable in this container via `nix develop` (store-path prefix; bare shell lacks `cc`); `nix build` remains `HOST-NIX` — batch only `nix build` into Step 6. |
| **Env marker** | `verifiable-here` (grep/lint-nix/nix eval); cargo gates are `verifiable-here` via `nix develop` (bare shell lacks `cc`); `nix build` is `HOST-NIX`. |
| **Parallelization** | Independent of Steps 0–8 code work; must land before the user renames the origin/checkout dir (spec §6). |

---

### Step 8b — microsandbox-filesystem agentd offline build (spec 09, ADR 0011 carrier)

| | |
|---|---|
| **Goal** | Execute [06-improvements/09-microsandbox-agentd-offline-build.md](06-improvements/09-microsandbox-agentd-offline-build.md): retire the build-time `agentd` network download in `microsandbox-filesystem` 0.5.6 that breaks sandboxed/offline Nix builds. Spec 09 is **independent of the validation lanes (Lane A/B)** and can run in parallel with them. |
| **Files to follow** | [06-improvements/09-microsandbox-agentd-offline-build.md](06-improvements/09-microsandbox-agentd-offline-build.md) (root cause, compensation inventory, upstream research, three options, recommended sequence). |
| **Sub-steps** | **Option 2 (INTERIM CARRIER, ADR 0011) — REVERSED per the ADR 0011 addendum (2026-07-30), NOT executed:** the fork (`github.com/georgrybski/microsandbox`) exists only as the transient vehicle to open the upstream PR; it is NEVER consumed as a dependency; the nix-side patch machinery stays as the interim. ~~Original plan (superseded): point `[patch.crates-io]` at the fork git URL and delete the nix-side machinery.~~ **Option 1 (UPSTREAM FIX — preferred end-state):** contribute an MSB_HOME-based agentd check to `crates/filesystem/build.rs` mirroring the SDK crate's own pattern (precedent: upstream #704); after an upstream release containing the fix, delete the fork + all compensation machinery and update the `control/agentctl/Cargo.toml` pin. **PR HARDENED, READY TO OPEN:** branch `fix/filesystem-agentd-path-override` @ `bc7640b8` (amended 2026-08-01 — 8-point review hardening + build-script tests; interim 0.5.6 patch aligned daa5140) ready to force-push (PR draft `.tmp/msb-upstream/PR.md`); force-push + open pending USER. **Option 3 (VERSION BUMP — separate combinable track):** `=0.5.6` → `=0.6.8` (`control/agentctl/Cargo.toml:17`); requires rewriting the agentd patch against 0.6.x build.rs (insertion point immediately before the new `if local.is_file() { copy_agentd(&local, &dest); return; }` branch — the 0.5.6 CI-gate anchor was deleted by upstream PR #1019) and re-validating the `msb --version` match check (`nix/packages/agentctl.nix:67-70`); 0.5.8+ added guest-write quotas and 0.6.6 added RESOLVE_BENEATH symlink protection (cross-check [06-improvements/01](06-improvements/01-mount-filtering-shadowing.md)); NEVER the yanked 0.6.5. |
| **Gate / exit criteria** | Option 2: REVERSED — no gate (never consumed as a dependency). Option 1: upstream PR merged (referencing #704 precedent) + released; machinery deleted; Cargo.toml pin updated. Option 3: `=0.6.8` pin; rewritten patch applies and build passes offline; `msb --version` match re-validated; spec 01 cross-checked for RESOLVE_BENEATH interaction. |
| **Env marker** | Option 2: REVERSED (no env gate — not executed). Option 1: upstream-latency-bound (no local env gate beyond the post-merge deletion + `nix flake check`). Option 3: `NEEDS-DEVSHELL` + `HOST-NIX` (patch rewrite + cargo build/test + runtime `msb --version` re-validation). |
| **Parallelization** | Independent of Lane A/B and of Steps 0–8a — runs in parallel. Option 2 is REVERSED (ADR 0011 addendum 2026-07-30) — no longer the first step and no longer gated on fork push access; the fork is a transient PR vehicle only. Option 1 is upstream-latency-bound and pending the USER force-push (branch amended @ `bc7640b8` 2026-08-01, PR ready to open). Option 3 is `HOST-NIX` and combinable with option 1. Recommended sequence: option 1 (push PR → upstream merge + release; interim nix-patch machinery stays until then) → bump the microsandbox pin + delete fork + ALL compensation machinery after release; option 3 separate track. |

---

### Step 8c — Config repos as working copies + dotfiles home (spec 10)

| | |
|---|---|
| **Goal** | Execute [06-improvements/10-config-repos-as-working-copies.md](06-improvements/10-config-repos-as-working-copies.md): record the two user decisions (config repos as first-class working copies in the home; the home as a dotfiles-style git repo) and the `repos/` → `config-repos/` rename, then land the three code tasks. Amends spec 08 step (a) (no standalone sibling) and `03-sibling-config-setup.md` topology. |
| **Files to follow** | [06-improvements/10-config-repos-as-working-copies.md](06-improvements/10-config-repos-as-working-copies.md) (Decisions A/B, name decision, three code tasks, mount model). |
| **Sub-steps** | **Task 1 (code-S): rename `repos/` → `config-repos/`** — sites: `paths.rs:216-218` (`config_repo_dir`), `loading.rs:313, 468, 930, 1027`, config add/update/remove machinery, tests; doc follow-through: ADR 0023 layout (`0023-single-tool-home.md:76`), glossary (`60-glossary.md:55-58`), this tree's docs. Gates: `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`. **Task 2 (test-S): dirty-safe `config update` regression test** — guard VERIFIED present (`config_cmd.rs:540-544`); add a regression test asserting `config update` bails on a dirty clone (mirroring `config remove --delete` dirty-refusal at `config_cmd.rs:52`); document the invariant. Gate: `cargo test`. **Task 3 (code-M): `workestrate home init` scaffolding** — new explicit command (propose `workestrate home init` or `workestrate init --home`; existing init at `init.rs:24`); implements git init + `.gitignore` (ignoring `/config-repos/`, `/sources/`, `/state/`, `/cache/`, secret-material patterns; keeping `/secrets/*.enc` committable) + pre-commit hook (rejecting mode-160000 gitlinks, staged paths under `config-repos/`/`sources/`/`state/`, and secret-material patterns) + next-steps printout; NEVER auto-git-init. Gates: `cargo test` (hook content generation, gitignore generation, idempotency). |
| **Gate / exit criteria** | Task 1: rename landed, cargo gates green. Task 2: dirty-guard regression test green. Task 3: `home init` scaffolds gitignore + hook and rejects gitlinks/secret paths (test-verified). `repos/` fallback documented. |
| **Env marker** | `verifiable-here` via `nix develop` (all three tasks are code/test work requiring the `cc` linker from the nix devshell — runnable in this container via the store-path prefix; bare shell lacks `cc`; verified 2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`). |
| **Parallelization** | Independent of Lane A/B. The `config-repos/` rename (Task 1) gates the final paths referenced by Steps 0.5/3/5 prose (which cite both spellings until the rename lands); land the rename before those prose references are finalized to `config-repos/`. Tasks 2 and 3 are independent of Task 1 and of each other. |

---

### Step 8d — --home flag + home provisioning + lockfile (specs 06 + 11) — **DONE**

| | |
|---|---|
| **Goal** | Execute spec 06 `--home` global flag and spec 11 provisioning + lockfile; SAME WAVE RECOMMENDED — both touch the same CLI/home-resolution surface: `cli_actions.rs` args, `home.rs` init, home-resolution precedence prose. |
| **Files to follow** | [06-improvements/06-config-home-flag.md](06-improvements/06-config-home-flag.md) + [06-improvements/11-home-provisioning-and-lockfile.md](06-improvements/11-home-provisioning-and-lockfile.md) (§2–§4); ADR 0025 ([../migration/50-decisions/0025-home-provisioning-and-lockfile.md](../migration/50-decisions/0025-home-provisioning-and-lockfile.md)). |
| **Sub-steps** | **ALL DONE (2026-07-30).** Code commits `172d5dd` (6b provisioning), `19ff272` (6b lockfile), `be356f7` (6b lock consumption), `d991252` (6a --home flag); spec 11 §4 test plan green (6c). Ops-verified end-to-end (`home init --from ~/.workestrate` → locked rev `c41a707`, dest-local url rewrite, empty `state/`, trusted_projects warning; `--home <dest> config list` + `litellm plan`). Original sub-steps: **(6a)** `--home` flag per spec 06 (sets `WORKESTRATE_HOME` from `async_main`; zero change to `paths.rs`). **(6b)** `home init --from`/positional dest + lockfile per spec 11 §2–§4 (`HomeAction::Init` gains `from`/`dest` at `cli_actions.rs:185`; provisioning at `commands/home.rs:67`; lock writers at `config_cmd.rs:42/188/510` + `registry.rs` `load_home_lock`/`save_home_lock` siblings; `looks_like_git_url` visibility at `registry.rs:126`). **(6c)** Tests per spec 11 §4 (positional dest, `--from` local/remote, fail-before-write residue, lock round-trip, newer-version hard error, trusted_projects warning, locked-rev checkout). **ANNOTATION (2026-07-31):** the `home init --from`/positional-dest interface described above was superseded by the `home init` / `home clone` verb split @ `c406630` — see ADR 0025 addendum. |
| **Gate / exit criteria** | `cargo test` green incl. spec 11 test plan; `--home` composition e.g. `workestrate --home <dir> home init --from <src>` works. **ANNOTATION (2026-07-31):** `home init --from` superseded by `home clone <src> [<dest>]` @ `c406630`. |
| **Env marker** | `verifiable-here` via `nix develop` (cargo-linked gates; bare shell lacks `cc`). |
| **Parallelization** | Independent of Lane A/B and Steps 7–8; recommended same wave as spec 06. |

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
| 0.5 (retire repo-local home) | Nothing until 0.5(a) verified; then sequential a→d | Step 0 (acknowledged) | Changes the paths Steps 1–6 reference — land before Lane A / host batch; (b)+(e) run via `nix develop` in-container (bare shell lacks `cc`) |
| 1 (Lane A gate) | — | Step 0 (acknowledged) | Must be green before Steps 2–8 |
| 2 (baseline recovery) | Steps 3, 4, 5 | Step 1 | `cc` linker via `nix develop` in-container (bare shell lacks `cc`) |
| 3 (quick wins) | Steps 2, 4, 5 | Step 1 | 3(a) is standalone; 3(b) is the only bug-fix improvement (can jump queue) |
| 4 (dogfooding Phase 0) | Steps 2, 3, 5 | Step 1 | 4(a) wrapper is standalone; 4(b) B1 is a code change |
| 5 (experiment probes) | Steps 2, 3, 4 | Step 1 | Uses disposable `/tmp` home; does not touch real bundle |
| 6 (HOST batch) | — | Steps 1–5 (or their HOST-deferred subsets) | Single host trip; batches all HOST-NIX/HOST-KVM gates |
| 7 (Track A) | WP1–WP3 ∥ Steps 3–5; Phase 0 spike ∥ Step 6 | Step 1 (for WP1–WP3); Step 6 (for Phase 0 spike if batched) | Phase 0 spike gates WP4; WP5 conditional on spike failure |
| 8 (remainder) | B2 ∥ B3 ∥ WP5 ∥ CLI (when unblocked) | Step 7 (for WP5); external sign-off (for CLI) | CLI gated on `02-config-requirements.md` sign-off |
| 8c (spec 10: config repos as working copies + dotfiles home) | Tasks 2 ∥ 3; independent of Lane A/B and Steps 0–8 | — | Task 1 (`config-repos/` rename) gates the final paths referenced by Steps 0.5/3/5 prose |
| 8d (specs 06 + 11: --home flag + home provisioning + lockfile) | 6a ∥ 6b (same wave recommended); independent of Lane A/B and Steps 7–8 | — | Both touch the same CLI/home-resolution surface (`cli_actions.rs` args, `home.rs` init, precedence prose) |

---

## Remaining improvements — recommended order (2026-08-01)

**Superseded-in-part by the W-wave (2026-08-01).** The remaining workestrate
work is sequenced as the W-wave (W1→W6, recorded 2026-08-01). ADR 0027 +
the ADR 0026/0021 addenda (commit 33eeb87) are authoritative for semantics;
the W1–W6 wave split is recorded from the 2026-08-01 session brief. The six-spec
improvement backlog below runs after/parallel to the wave.

| Wave | Content | Refs |
|---|---|---|
| W1 | default-on dependency lifecycle core — deps start by default on up/exec, topo-ordered closure, singleton slots, `--no-deps` opt-out, occupied = satisfied, plan never starts | [ADR 0026 addendum (2026-08-01)](../migration/50-decisions/0026-per-instance-addressing-and-discovery.md); spec 12 §3 supersession |
| W2 | readiness posture — service-kind detached + wait-for-port (~15s); agent-kind refuse | [ADR 0026 addendum (2026-08-01)](../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) |
| W3 | `workestrate workloads` discovery verb | [ADR 0027](../migration/50-decisions/0027-verb-first-workload-dispatch.md); successors explored in [06-improvements/19](06-improvements/19-visualization-inspection.md) |
| W4 | actual-record injection — depends_on env injection reads the ACTUAL port-registry record (actual assigned port, not declared) | ADR 0026 addendum (2026-08-01); spec 12 §4 |
| W5 | config wiring — declare `depends_on` in config.reference + personal (agents → litellm, required=true); retire hardcoded URLs via `${VAR}` templating (`OPENAI_BASE_URL = "http://${LITELLM_ADDR}/v1"`) | spec 12 §4 ([06-improvements/12](06-improvements/12-per-instance-addressing.md)) |
| W6 | propagation/closeout + follow-ups triage (DependsOnSpec scheme/path_suffix; wait-for-port v1 → guest healthchecks v2+; restart posture) | ADR 0026 addendum follow-ups; spec 12 §5 |

This section supersedes nothing else — it sequences the six not-yet-landed specs
(Steps 0.5/8a/8c/8d are DONE; spec 15 EXECUTED 2026-08-01; specs 16/17 EXECUTED 2026-08-01). Work it
top-to-bottom.

1. **05 — config-reference cwd-fallback** (small) — the only bug-fix improvement; closes a silent config-discovery backdoor; gate `cargo test` runs in-container via `nix develop`.
2. **01 — mount filtering WP1–WP3** — highest-value hardening; WP1–WP3 are `verifiable-here`; the Phase 0 spike + WP4 are `HOST-KVM` and fold into the Step 6 host batch.
3. **03 — dogfooding B1/B2** — structural (not convention-based) driver/target isolation; B1/B2 are `verifiable-here` (B3 keeps a `HOST-KVM` tail).
4. **07 — naming-consistency leftover** (trivial) — mechanical residue sweep + the pending cargo gates via `nix develop`; clears residue before the user's repo rename (spec §6).
5. **09 — microsandbox agentd post-merge cleanup** — upstream-latency-bound; the PR is HARDENED and ready to open after the user's force-push, so local work resumes only after merge+release: delete compensation machinery, bump the pin, `nix flake check`.
6. **04 — CLI config authoring** — DEFERRED by design; gated on `02-config-requirements.md` sign-off, not on any code step.

---

## Phase A / Phase B — final-model + directory-mode waves (2026-08-01)

The unified secret/env model and the config-repo directory-mode layout are
sequenced as two waves on top of the W-wave above.

### Phase A (docs waves) — THIS batch

Docs-only. Landed together:

- **Spec 16** — [06-improvements/16-unified-secret-env-model.md](06-improvements/16-unified-secret-env-model.md):
  the FINAL unified secret/env model (unified secrets catalog; the four env
  value forms; bound defaults; per-secret `allowed_hosts`).
- **Spec 17** — [06-improvements/17-config-repo-directory-mode.md](06-improvements/17-config-repo-directory-mode.md):
  config-repo directory mode (`workestrate/{default,secrets}.toml` +
  `workloads/` capsules), READY-TO-EXECUTE (design).
- Bookkeeping: `NEXT-SESSION.md` wave-state entries + this section.

### Phase B (implementation) — LANDED 2026-08-01 (steps 1–4; step 5 partial — interim-patch slim landed, fork push pending user)

1. **Final-model code model — LANDED (2026-08-01, `19b2cf0`).** Superseded the v2 delivery/shim: `EnvBinding`
   `delivery` (`env` | `host_bound`) + the `fold_legacy_secret_model` shim fold
   into the final model per spec 16. `verifiable-here` via `nix develop`
   (cargo gates).
2. **Config conversion to the final model — LANDED (2026-08-01, `19b2cf0`; personal-v2 restructured to directory mode in `e3194d9`).** The `env_var` strip, the
   personal-v2 migration, and old-personal removal. `verifiable-here` via
   `nix develop` (cargo gates + golden diffs).
3. **Directory-mode loader per spec 17 — LANDED (2026-08-01, `e3194d9`; tombi include globs `ea48f45`)** + the **personal config restructure**
   as the proof case: `workestrate.toml` → `workestrate/{default,secrets}.toml`
   + `workloads/` capsules; the `agents/` + `infra/` trees collapse into
   capsules. Includes the queued `tombi.toml.tpl` include-glob edit (spec 17
   §2.6). `verifiable-here` via `nix develop` (cargo gates + byte-identical
   merged-config golden); runtime smoke of the restructured repo is
   `HOST-KVM` (folds into the Step 6 host batch).
4. **Version collapse execution — LANDED (2026-08-01, `19b2cf0` — v2 retracted pre-release; `schema_version = 1` everywhere).**
   `verifiable-here` via `nix develop`.
5. **Fork push + interim-patch slim reconciliation — interim-patch slim LANDED (`19d94e8`); fork force-push + PR open PENDING USER (branch @ `bc7640b8`).** The spec 09 thread:
   `.tmp/microsandbox` fork branch `fix/filesystem-agentd-path-override` +
   the interim 0.5.6 nix patch `daa5140` (see Step 8b). Upstream-latency-bound;
   local post-merge work is `verifiable-here` via `nix develop` +
   `HOST-NIX` (`nix flake check`).

---

## Definition of done (whole effort)

- [ ] **Lane A green** — `just verify` passes in this container via `nix develop` (store-path prefix; the shell/python/git subset passes in a bare shell, the cargo-based gates run via `nix develop` — both `verifiable-here`). Lane A against the real bundle: the Step 0(e) blocker edit has LANDED (tempest `install_layout` removed); the first Lane A action is now `workestrate validate-config` + `workestrate workload plan tempest` in `nix develop`.
- [ ] **Baseline parity verdict documented** — all 5 workloads match after normalization, or every delta is justified in a table ([04-baseline-validation.md](04-baseline-validation.md) §5 acceptance criteria).
- [ ] **Host batch B1–B12 green** — every step in [05-host-validation.md](05-host-validation.md) passes per the capability → proof → acceptance matrix.
- [x] **Main rename done** — personal clone branch is `main`, `config list` shows `ref main, rev d2cd0c3, clean) [OK]` ([06-improvements/02](06-improvements/02-main-standardization.md)). **(applied; rev `d2cd0c3` unchanged; clone has no origin remote — `config update` pending push)** *(2026-08-01: spec 02 closed as OBSOLETE — the mismatch was mooted by spec 08's execution.)*
- [x] **Repo-local home retired** — [06-improvements/08](06-improvements/08-no-repo-local-home.md) executed end-to-end: personal config preserved as the in-home working copy at `~/.workestrate/config-repos/personal` (@ `c41a707`) — AMENDED by spec 10 (no standalone sibling), real home at `~/.workestrate` (registry `ref`/`rev` removed — local-path classification), machinery commit `418530a` landed, bundle deleted, discovery tier removed from `paths.rs` (commit `bef1c37`), ADR 0023 addendum follow-through landed in `bef1c37`. **Home adopted into git** as root commit `a42e597`; `workestrate home init` idempotent (commit `3894fb7`); `config-repos/` rename (commit `d7c5a83`); dirty-guard test (commit `bd99481`). `validate-config` → "workestrate.toml is valid." exit 0 (runtime-proves `install_layout` removal). `doctor` → "home: OK (/home/node/.workestrate (Default))". **Cargo.lock** verified stable (no commit needed).
- [ ] **microsandbox-filesystem agentd offline build (spec 09)** — [06-improvements/09](06-improvements/09-microsandbox-agentd-offline-build.md) executed: option 2 (ADR 0011 fork carrier) REVERSED per the ADR 0011 addendum (2026-07-30) — the fork is a transient PR vehicle only, never consumed as a dependency; the nix-side patch machinery stays as the interim (slimmed to match the fork in `19d94e8`); option 1 (upstream MSB_HOME fix) PR HARDENED @ `bc7640b8` — force-push + open PENDING USER PUSH, then merged + released + machinery deleted + Cargo.toml pin updated; option 3 (`=0.6.8` bump) separate combinable track: rewritten patch + `msb --version` re-validated + spec 01 RESOLVE_BENEATH cross-check. **(PENDING — option 1 upstream PR push is a user action)**
- [x] **Config repos as working copies + dotfiles home (spec 10)** — [06-improvements/10](06-improvements/10-config-repos-as-working-copies.md) executed: docs/decision landed; spec 08 step (a) + `03-sibling-config-setup.md` topology carry AMENDED-by-spec-10 markers; code tasks landed — Task 1 `config-repos/` rename (commit `d7c5a83`), Task 2 dirty-guard regression test (commit `bd99481`), Task 3 `home init` scaffolding (commit `3894fb7`) with cargo gates green in `nix develop`. Home adopted into git as root commit `a42e597`; `home init` idempotent over populated home; pre-commit hook rejects gitlinks/store-dirs/secret material.
- [x] **--home flag + home provisioning + lockfile (specs 06 + 11, Step 8d)** — commits `172d5dd` / `19ff272` / `be356f7` / `d991252`; per-commit gates green (`cargo fmt --check`, `clippy -D warnings`, full `cargo test` — 411 passed at wave end, `just golden-check`/`schema-check`/`spec-examples`/`scaffold-check`, clean `Cargo.lock`); ops verification 2026-07-30: `home init --from ~/.workestrate` → dev home at locked rev `c41a707` with dest-local url rewrite, empty `state/`/`secrets/`, loud trusted_projects warning; `--home <dest> config list` and `litellm plan` resolve the provisioned home. *(2026-07-31: `home init --from` superseded by the `home clone` verb split @ `c406630`.)*
- [x] **Per-instance addressing + discovery-lite (spec 12)** — [06-improvements/12](06-improvements/12-per-instance-addressing.md) IMPLEMENTED: Wave 1 (`9107b87`, `de9aa62`, `f9fd2f0`, `c5837e7`) + Wave 2 (`4adad3f`, `7b65ad1`, `39c1694`) landed on `migration/tool-model`. **(E1 guest-reachability probe + the deferred binding decision remain NEEDS-KVM — pending the Step 6 host batch, B10.)**
- [x] **TOML toolchain: tombi (spec 15)** — [06-improvements/15](06-improvements/15-toml-toolchain-tombi.md) EXECUTED (2026-08-01): tombi 1.2.5 nix package + devshell (`92b6b6f`); scaffold `tombi.toml` + vendored schema + config-repo pre-commit hook (`d97576d` + gap-fix wave); repo-wide gates (`9ffad0e`): root `tombi.toml`, `scripts/check-toml.sh`, `just tombi-check` in `just verify`, `lib.checks.tombiCheck`, home-hook gate + home `tombi.toml`/schema emission; personal config repo applied (`0750876`). Planted-violation evidence: unknown key → "not allowed", `cpus = "two"` → type error, both exit 1.
- [x] **Final unified secret/env model (spec 16)** — [06-improvements/16](06-improvements/16-unified-secret-env-model.md) EXECUTED (2026-08-01, `19b2cf0`): per-binding `bound` (`host` default placeholder / `guest` real value), `KEY = true` sugar, `allowed_hosts` credential policy, intermediate v2 machinery retracted, `schema_version = 1` everywhere; personal-v2 migrated. **(B13 runtime smoke re-pointed at the final model stays NEEDS-KVM — host batch.)**
- [x] **Config repo directory mode (spec 17)** — [06-improvements/17](06-improvements/17-config-repo-directory-mode.md) EXECUTED (2026-08-01, `e3194d9`; home tombi glob `ea48f45`): directory-mode loader with hard-error semantics, personal-v2 restructured to `workestrate/{default,secrets}.toml` + `workloads/` capsules, tombi include globs cover capsule files. **(Runtime smoke of the restructured repo stays NEEDS-KVM — host batch.)**
- [ ] **Dogfooding guardrails active** — Phase 0 env-pinning wrapper in use; B1 self-home mount guard merged (or explicitly deferred with rationale).
- [ ] **Track A WP1–WP4 merged** (or WP5 triggered) — mount filtering/shadowing schema, policy, render, and runtime shadows landed; or, if the Phase 0 spike failed, the staging-copy fallback (WP5) is landed instead.
- [ ] **Improvements index statuses updated** — [06-improvements/00-index.md](06-improvements/00-index.md) master table reflects the actual post-execution status of each spec (SPEC → IMPLEMENTED / MERGED / DEFERRED).
- [x] **Tempest `install_layout` drift removed** — `install_layout = "app"` removed from tempest's `binary` inline table in the personal config (Bundle fix e, CRITICAL). **(edit applied; runtime parse verification DONE — `workestrate validate-config` → "workestrate.toml is valid." exit 0)**
- [ ] **Tempest FOD hash pinned** — `npm_deps_hash` in both `nix/packages/tempest.nix` and `workestrate.toml:317` is the real computed hash.
- [ ] **`ODYSSEUS_ADMIN_PASSWORD` real** — placeholder replaced with a SOPS-encrypted secret; odysseus first boot succeeds (B7).

---

## Note on host trips

**Host trips are expensive.** Every genuine `HOST-NIX` and `HOST-KVM` gate
requires a context switch to a nix-capable, KVM-capable host. This container
has no KVM (`ls /dev/kvm` → not found) and nix is not on PATH
(`command -v nix` → not found), but nix IS installed at
`/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` and `nix develop`
provides a full C toolchain (verified 2026-07-29: `cc --version` → gcc 15.2.0;
`cargo check` compiles in ~27s). So cargo-linked `just` gates run in-container
via `nix develop`; only `nix build` image builds, prefetch jobs,
`just verify-full`, `just generate-schema`, KVM runtime, and sops secret
provisioning genuinely require the host (see
[01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §Environment
honesty). The batching principle from the migration doc applies:

> "The host-batched HOST-NIX and HOST-KVM gates run in one pass at the end so
> the host environment is not context-switched mid-remediation."
> — [docs/migration/40-migration-process.md](../migration/40-migration-process.md)
> lines 368–369.

**Batch everything genuinely `HOST-NIX`/`HOST-KVM` into the Step 6 single
pass wherever possible:**

- The tempest FOD hash computation (Step 0c).
- The `ODYSSEUS_ADMIN_PASSWORD` secret provisioning (Step 0d).
- `just verify-full` (adds `nix build .#workestrate`).
- `just generate-schema` (schema regen, requires devshell RUSTFLAGS/libcap-ng).
- The Phase 0 KVM spike (Step 7) — it is a standalone HOST-KVM PoC with no
  dependency on WP1–WP3 code, so it can run during the same host pass as
  B1–B12. The only reason to make a second host trip is if WP4 code (which
  depends on the spike result) was not ready during the first pass.

**NOT batched to the host (runnable in this container via `nix develop`):**
the cargo-based Lane A gates (`check`, `test`, `spec-examples`, `golden-check`,
`schema-check`, `scaffold-check`) and the baseline recovery procedure
(Step 2) — all run via
`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
then `nix develop -c bash -c '<cmd>'` (first devshell build takes minutes). A
bare shell lacks `cc` (verified: `command -v cc gcc` → not found); the
01-vs-04 contradiction on this is RESOLVED — cargo gates are `verifiable-here`
via `nix develop`.
