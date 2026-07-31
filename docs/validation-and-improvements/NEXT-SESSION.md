# NEXT-SESSION — handoff prompt

> **STATUS: HANDOFF**
> Prerequisites / see-also: [README.md](README.md) · [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [07-execution-order.md](07-execution-order.md)

This file is the self-contained handoff for the next contextless session working
the `migration/tool-model` branch. It assumes no prior conversation. Everything
below is drawn from the four cited docs, which are the source of truth — do not
re-derive their contents.

> **⚠ CONTAINER-HOME WIPE CAVEAT (2026-07-30):** the container `$HOME` is
> ephemeral. The `~/.workestrate` home built on 2026-07-30 (root commit
> `a42e597`) was **WIPED by a container restart** — every statement in this
> handoff that describes the home as present is point-in-time evidence from
> before the wipe, NOT current state. The personal config content survives at
> `ai-workbench/.tmp/config-repos-export` (verified `c41a707`). To restore:
> `workestrate home init` + `workestrate config add
> <repo>/.tmp/config-repos-export/personal personal` + `workestrate config
> trust /home/node/Development/ai-workbench` — then verify with
> `workestrate config list`, `workestrate validate-config`, and confirming
> all 5 plans render. A parallel task is restoring the home now; the durable
> fix is the host-side home (operator task).

---

## The prompt (copy verbatim)

> You are picking up the `migration/tool-model` validation-and-improvements
> effort on the `ai-workbench` repo. Work strictly from the cited docs; do not
> re-investigate facts they already verify.
>
> **1. Where you start.** Start in the PARENT directory of `ai-workbench/`
> (i.e. `/home/node/Development/`), NOT inside the checkout. You are one
> directory up so you can manage config homes as siblings of the repo rather
> than inside it: the experiment home (`/tmp/workestrate-exp`, per
> `03-sibling-config-setup.md`) and the driver home (`~/.workestrate-driver`,
> per `06-improvements/03-dogfooding.md` Phase 0) live alongside the checkout,
> never inside it. `cd /home/node/Development/` first.
>
> **2. Required reading (source of truth — do not re-derive).** Read these in
> order before touching anything:
>   - `ai-workbench/docs/validation-and-improvements/README.md`
>   - `ai-workbench/docs/validation-and-improvements/00-overview.md`
>   - `ai-workbench/docs/validation-and-improvements/01-current-state-and-prereqs.md`
>   - `ai-workbench/docs/validation-and-improvements/07-execution-order.md`
>
> These carry verified `file:line` citations and are the execution record. Trust
> them over re-investigation. Every env marker, command spelling, and citation
> below is reproduced from them.
>
> **3. Current state.** The migration to the config-driven tool model is
> COMPLETE — ~370 tests green per the migration record. The home migration
> (spec 08 + spec 10) is **EXECUTED** — the next session starts at Lane A
> full gates / host batch / remaining improvements.
>
> **Home migration — EXECUTED (ops, not repo commits):**
>   - The workestrate tool home now lives at `~/.workestrate` as a dotfiles
>     git repo (root commit `a42e597` "home: adopt registry + personal config
>     repo"). `git ls-files` shows ONLY `.gitignore` + `config.toml` tracked;
>     store dirs (`config-repos/`, `sources/`, `state/`) are gitignored; no
>     gitlinks (mode-160000 count = 0). The pre-commit hook (installed by
>     `workestrate home init`) rejects gitlinks, store-dirs, and secret
>     material; it did not block the legitimate root-commit files.
>   - The personal config is a local working clone at
>     `~/.workestrate/config-repos/personal` (no remote; `ref`/`rev` lines
>     removed from the registry — local-path classification; provenance in
>     the clone's own git log: `c41a707` "fix(tempest): remove retired
>     install_layout field").
>   - `workestrate home init` ran idempotently over the populated home
>     (first run: added `.git`/`.gitignore`/`.git/hooks/pre-commit` only,
>     content untouched; second run: "already initialized … nothing to do").
>   - The repo-local bundle `/home/node/Development/ai-workbench/.workestrate`
>     is DELETED.
>
> **STAGE A commits (code):** `421a54a` (docs env claims), `d7c5a83` (rename
> `repos/` → `config-repos/`), `bd99481` (dirty-guard test in config update),
> `3894fb7` (`workestrate home init`), `bef1c37` (remove trusted-ancestor home
> discovery).
>
> **STAGE B commit:** `418530a` "chore: remove repo-local tool home machinery"
> (`.envrc` pin removed; `scripts/local-xdg.sh` +
> `scripts/migrate-xdg-to-repo.sh` deleted; `/.workestrate/` gitignore entry
> removed; justfile `local-setup` recipe + README activation refs cleaned).
>
> **Cargo.lock stability:** `control/agentctl/Cargo.lock` is stable (sha256
> `7580f399…` identical across 2 consecutive `cargo check --locked` runs from
> `control/agentctl` cwd; vendored microsandbox-filesystem symlink present and
> resolving to
> `/nix/store/syksqgy5…-microsandbox-filesystem-patched-0.5.6`). NO commit
> needed.
>
> **Home-provisioning wave — EXECUTED (2026-07-30, Step 8d, specs 06 + 11):**
> commits `172d5dd` (`home init --from` + positional dest), `19ff272`
> (generated `workestrate.lock`), `be356f7` (lock consumption + version
> evolution), `d991252` (global `--home` flag). Ops-verified: `home init
> --from ~/.workestrate <dest>` produces a reproducible home (personal pinned
> at `c41a707`, dest-local url rewrite, empty `state/`, loud trusted_projects
> warning); `--home <dest> config list` / `litellm plan` work. CONTAINER
> GOTCHA: `/home/node/Development/` is root-owned — scratch provisioning dests
> must go under `/home/node/Development/worktrees/` or `/tmp`.
>
> **install_layout removal — RUNTIME-VERIFIED:** `workestrate validate-config`
> → "workestrate.toml is valid." exit 0 (run with `WORKESTRATE_HOME` unset,
> binary from `cargo build`, from neutral cwd `/tmp`). This runtime-proves the
> tempest `install_layout` field removal (bundle fix e).
>
> **Consumption verification — POINT-IN-TIME EVIDENCE from 2026-07-30 (pre-wipe; the home it ran against no longer exists — see the wipe caveat above; NOT current state). All run with `WORKESTRATE_HOME` unset, binary
> from `cargo build`, from neutral cwd `/tmp` unless noted):**
>   - `workestrate config list` → "personal: /home/node/.workestrate/config-repos/personal (ref main, rev unknown, clean) [OK]", Layers: ["personal"], trusted project /home/node/Development/ai-workbench [OK]; exit 0.
>   - `workestrate validate-config` → "workestrate.toml is valid." exit 0.
>   - All 5 plans render, exit 0: litellm, pi, odysseus, opencode, tempest.
>   - `workestrate ps` → "(no running workestrate instances)" exit 0.
>   - `workestrate doctor` → "home: OK (/home/node/.workestrate (Default))",
>     "config_repos: OK (1 repo(s) checked)"; `dev_kvm`: FAIL (expected — no
>     KVM in container); `age_key_file` WARN (expected); exit 1 solely due to
>     `dev_kvm`.
>   - `workestrate secrets-schema` → exactly 7 env var names: GITHUB_TOKEN,
>     KIMI_CODE_API_KEY, LITELLM_MASTER_KEY, MINIMAX_CODING_API_KEY,
>     NEURALWATT_API_KEY, ODYSSEUS_ADMIN_PASSWORD, OPENROUTER_API_KEY.
>   - From INSIDE the repo (`WORKESTRATE_HOME` unset): `doctor` reports the
>     SAME home "/home/node/.workestrate (Default)" — no repo-local discovery,
>     no legacy-XDG note.
>
> A bare shell in this container has NO `cc` linker (verified:
> `command -v cc gcc` → not found), but nix IS installed at
> `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` (not on PATH)
> and `nix develop` provides a full C toolchain (verified 2026-07-29:
> `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
> then `nix develop -c bash -c 'cc --version'` → gcc 15.2.0; `cargo 1.97.1`,
> `rustc 1.97.1`; `cargo check` compiles in ~27s; first devshell build takes
> minutes, subsequent runs are fast). The home migration is done; the next
> session starts at **Lane A full gates / host batch / remaining
> improvements** (`07-execution-order.md` Steps 1+).
>
> **4. Environment honesty.** This container has:
>   - **No KVM** (`ls /dev/kvm` → not found) — no sandbox runtime can execute.
>   - **No `cc` linker in a bare shell** — but cargo-linked gates (`check`,
>     `test`, `spec-examples`, `golden-check`, `schema-check`, `scaffold-check`)
>     ARE runnable here via `nix develop` (store-path prefix; verified
>     2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`).
>   - **No sops age key** (`~/.config/sops/age/` absent, `SOPS_AGE_KEY` unset,
>     `sops` not on PATH) — secret decryption FAILS CLOSED. Do NOT attempt to
>     work around it; secret provisioning is HOST-only.
>
> HOST-KVM gates and the genuine HOST-NIX gates (`nix build` image builds,
> `nix run nixpkgs#...` prefetch jobs, `just verify-full`, `just generate-schema`)
> are DEFERRED per `05-host-validation.md` and BATCHED into the single host
> pass at `07-execution-order.md` Step 6 (B1–B12). Do not make a host trip for
> one gate — batch.
>
> **5. How to work.** Work `07-execution-order.md` in order, top to bottom
> (Step 0 → 0.5 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8). Step 0.5 (spec 08, retire
> the repo-local home) is new — its step (a) is a hard prerequisite for any
> container rebuild. Report lane-by-lane honestly:
> `verifiable-here` (this container: TOML, golden files, git, shell/python,
> AND cargo-linked gates via `nix develop`) vs `HOST-NIX` (host only: nix
> builds, FOD hashes, `just verify-full`, `just generate-schema`) vs `HOST-KVM`
> (runtime: service boot, agent exec, instance lifecycle). The
> `verifiable-here` subset that passes in a bare shell is: `toolchain-check`,
> `litellm-check`, `lint-nix`, `store-audit` (SKIP), and the `Cargo.lock`
> stability `git diff`; the cargo-linked gates additionally pass via
> `nix develop`.
>
> **6. Experiment home.** The disposable experiment home setup is
> `03-sibling-config-setup.md` — bootstrap is
> `export WORKESTRATE_HOME=/tmp/workestrate-exp` → `just workestrate config new
> exp --empty`. Keep it OUTSIDE the repo (you are one directory up, so this is
> natural). Destructive probes (P1–P5: contexts, instances, mounts, secrets,
> seed_files) ONLY EVER happen there — never against the real bundle at
> `.workestrate/`. Guard invariant: `test "$WORKESTRATE_HOME" =
> "/tmp/workestrate-exp"`.
>
> **7. Constraints.**
>   - **No code/config semantic change without a cited ADR.** ADRs 0001–0023
>     live at `ai-workbench/docs/migration/50-decisions/`; they stand and are
>     not re-litigated here.
>   - **The config-requirements contract is `02-config-requirements.md`** —
>     schema, merge/layering, policy ceiling, trust model. It is frozen pending
>     sign-off.
>   - **The CLI authoring tool (`06-improvements/04-cli-config-authoring.md`)
>     stays DEFERRED** until that contract is signed off. Do not start it.
>   - **The real bundle `.workestrate/` is read-mostly.** Destructive
>     operations only in the experiment home. The only bundle edits in flight
>     are the already-applied fixes (a) and (e); do not add more without an
>     ADR.
>   - **Report validation lanes honestly:** `validated` /
>     `partially_validated` / `not_validated`. Do not imply correctness beyond
>     what was actually run.
>
> **8. Upcoming improvements.** The agreed upcoming improvements include the
> `--home` global CLI flag — spec at
> `06-improvements/06-config-home-flag.md` (STATUS: SPEC, small, additive). It
> sets `WORKESTRATE_HOME` from `async_main`
> (`control/agentctl/src/main.rs:237`) — following the exact precedent of
> `WORKESTRATE_NO_PROJECT_CONFIG` / `WORKESTRATE_CONTEXT` at `main.rs:239-244` —
> so it becomes the highest-precedence home override with NO path-resolution
> change (it reuses env-var precedence step 1 in `resolve_home_with_kind()`,
> `paths.rs:106-110`).
>
> **9. Report back.** End every session with three things:
>   - **What was confirmed** — with evidence (command + output, or file:line
>     citation).
>   - **What is blocked and exactly why** — name the missing capability
>     (KVM / sops age key; note: `cc` is available via `nix develop` in this
>     container, so cargo gates are NOT blocked) and the step it gates.
>   - **The next concrete action** — one sentence, the very next command or
>     edit.

> **Note (2026-07-29):** improvement spec 07 — naming consistency (purge `workestrator` residue, standardize on `workestrate`) — is IN-PROGRESS on branch `migration/tool-model`; see [06-improvements/07-naming-consistency.md](06-improvements/07-naming-consistency.md), including the personal-config-repo image-name FLAG (§4) and the checkout-dir-rename implications (§6).

> **Note (2026-07-30, spec 08):** **EXECUTED.** The workestrate tool home
> must NEVER live inside the repo checkout — the home is the user-global
> `~/.workestrate` only. Spec:
> [06-improvements/08-no-repo-local-home.md](06-improvements/08-no-repo-local-home.md)
> (STATUS: EXECUTED). The repo-local bundle at `.workestrate/` is **deleted**;
> the home lives at `~/.workestrate` as a dotfiles git repo (root commit
> `a42e597`). Commit refs: `d7c5a83`, `bd99481`, `3894fb7`, `bef1c37`,
> `418530a`; home commit `a42e597`. The `--home` flag spec
> ([06-improvements/06-config-home-flag.md](06-improvements/06-config-home-flag.md))
> gains weight: with the discovery tier removed, `--home` becomes THE explicit
> override (precedence: flag > env > legacy XDG > default).

> **Note (2026-07-29, spec 09):** improvement spec 09 — microsandbox-filesystem agentd offline build (ADR 0011 carrier) — is READY-TO-EXECUTE with option 2 blocked on fork push access (`github:georgrybski/microsandbox-filesystem`); see [06-improvements/09-microsandbox-agentd-offline-build.md](06-improvements/09-microsandbox-agentd-offline-build.md).

> **Note (2026-07-30, spec 09):** ADR 0011 vendor→fork-carrier decision REVERSED (see ADR 0011 addendum 2026-07-30) — the georgrybski/microsandbox fork is a transient PR vehicle only, never consumed as a dependency; the upstream PR (MSB_HOME parity for crates/filesystem/build.rs, mirroring #704) is IN-FLIGHT; the nix-side patch interim stays unchanged until the upstream release lands and the machinery is deleted.

> **Note (2026-07-30, spec 10):** **EXECUTED (code tasks landed).** (A)
> consumed config repos are FIRST-CLASS working copies inside the tool home at
> `$WORKESTRATE_HOME/config-repos/<name>/` (remote is canonical; supersedes the
> standalone-sibling model), and (B) the home itself becomes a dotfiles-style
> git repo via explicit `workestrate home init` scaffolding (gitignore +
> pre-commit hook guarding against mode-160000 gitlinks and secret material).
> Spec: [06-improvements/10-config-repos-as-working-copies.md](06-improvements/10-config-repos-as-working-copies.md)
> (STATUS: code tasks landed — `d7c5a83` rename, `bd99481` dirty-guard test,
> `3894fb7` home init; docs/spec fully done). The personal working repo lives
> at `~/.workestrate/config-repos/personal`, NOT a standalone sibling.

> **Note (2026-07-30, spec 12):** **IMPLEMENTED (code landed).** Per-instance
> addressing + discovery-lite (ADR 0026) is code-complete on
> `migration/tool-model`: Wave 1 (`9107b87`, `de9aa62`, `f9fd2f0`,
> `c5837e7`) + Wave 2 (`4adad3f`, `7b65ad1`, `39c1694`). That leaves 12 specs
> total in `06-improvements/` with these remaining open: 01 (mounts), 03
> (dogfooding), 04 (CLI authoring — stays DEFERRED), 05 (cwd-fallback), 07
> (naming leftovers), 09 (agentd offline build). Experiment E1
> (guest-reachability of non-`127.0.0.1` loopbacks) is a first-class
> HOST-KVM host-batch item (`07-execution-order.md` Step 6 B10); the deferred
> binding decision stays NEEDS-KVM per ADR 0026 until E1 runs and its
> outcome is recorded in the spec's §open-decisions.

---

## Maintenance

Update this file whenever `README.md`, `01-current-state-and-prereqs.md`, or
`07-execution-order.md` state changes materially — e.g. a bundle fix moves
from APPLIED to VERIFIED, a Step's env marker changes, a new improvement spec
is added, or the `--home` flag moves from SPEC to IMPLEMENTED. The handoff
prompt must never go stale: its facts are a strict subset of the four cited
docs, so when those docs change, re-derive the affected prompt section from
them and note the update here.
