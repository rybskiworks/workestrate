# NEXT-SESSION — handoff prompt

> **STATUS: HANDOFF**
> Prerequisites / see-also: [README.md](README.md) · [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [07-execution-order.md](07-execution-order.md)

This file is the self-contained handoff for the next contextless session working
the `migration/tool-model` branch. It assumes no prior conversation. Everything
below is drawn from the four cited docs, which are the source of truth — do not
re-derive their contents.

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
> COMPLETE — ~370 tests green per the migration record. Both bundle fixes are
> APPLIED but RUNTIME-UNVERIFIED:
>   - **(e) tempest `install_layout = "app"`** — REMOVED from
>     `.workestrate/repos/personal/workestrate.toml:317`. `BinarySpec`
>     (`control/agentctl/src/config/types.rs:44-52`) has
>     `#[serde(deny_unknown_fields)]` and no `install_layout` field; the nix-side
>     param was removed as a silent no-op (`nix/lib/recipes/npm-build.nix:16-25`).
>     Verified by grep (no match). **PENDING:** runtime parse verification.
>   - **(a) personal clone branch** — renamed `master` → `main` (verified:
>     `git branch --show-current` → `main`), HEAD `d2cd0c3` unchanged, no
>     registry edit (`config.toml:9` already declared `ref = "main"`). NOTE: the
>     clone has NO origin remote (verified: `git remote -v` → empty), so
>     `workestrate config update` fails until the config repo is pushed to a
>     remote.
>
> **NEW DECISION (spec 08):** the workestrate tool home must NEVER live
> inside the repo checkout — home = user-global `~/.workestrate` only. Spec:
> `06-improvements/08-no-repo-local-home.md` (READY-TO-EXECUTE; code step
> NEEDS-DEVSHELL). The repo-local bundle `.workestrate/` is STILL PRESENT
> until the spec is executed — and until its step (a) lands it holds the ONLY
> committed copy of the personal config (`c41a707`); do NOT rebuild the
> container or delete the bundle (see spec §5 interim warning). Execution is
> sequenced EARLY: `07-execution-order.md` Step 0.5, before Lane A / the host
> batch, because it changes the paths those reference.
>
> The authoring container has NO `cc` linker (verified:
> `command -v cc gcc` → not found). Your FIRST job is to verify the parse:
> inside `nix develop` (HOST-NIX devshell provides `cc`), run
> `workestrate validate-config` and `workestrate tempest plan` against the real
> bundle (`WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate`).
> This is Step 0(e) runtime verification + the Lane A gate entry action.
>
> **4. Environment honesty.** This container has:
>   - **No KVM** (`ls /dev/kvm` → not found) — no sandbox runtime can execute.
>   - **No `cc` linker** — every cargo-linked gate (`check`, `test`,
>     `spec-examples`, `golden-check`, `schema-check`, `scaffold-check`) is NOT
>     runnable here; use `nix develop` (HOST-NIX) for those.
>   - **No sops age key** (`~/.config/sops/age/` absent, `SOPS_AGE_KEY` unset,
>     `sops` not on PATH) — secret decryption FAILS CLOSED. Do NOT attempt to
>     work around it; secret provisioning is HOST-only.
>
> HOST-KVM and HOST-NIX gates are DEFERRED per `05-host-validation.md` and
> BATCHED into the single host pass at `07-execution-order.md` Step 6 (B1–B12).
> Do not make a host trip for one gate — batch.
>
> **5. How to work.** Work `07-execution-order.md` in order, top to bottom
> (Step 0 → 0.5 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8). Step 0.5 (spec 08, retire
> the repo-local home) is new — its step (a) is a hard prerequisite for any
> container rebuild. Report lane-by-lane honestly:
> `verifiable-here` (this container: TOML, golden files, git, shell/python) vs
> `HOST-NIX` (nix devshell: cargo gates, nix builds, FOD hashes) vs `HOST-KVM`
> (runtime: service boot, agent exec, instance lifecycle). The
> `verifiable-here` subset that passes in this container is: `toolchain-check`,
> `litellm-check`, `lint-nix`, `store-audit` (SKIP), and the `Cargo.lock`
> stability `git diff`.
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
>     (`cc` linker / KVM / sops age key) and the step it gates.
>   - **The next concrete action** — one sentence, the very next command or
>     edit.

> **Note (2026-07-29):** improvement spec 07 — naming consistency (purge `workestrator` residue, standardize on `workestrate`) — is IN-PROGRESS on branch `migration/tool-model`; see [06-improvements/07-naming-consistency.md](06-improvements/07-naming-consistency.md), including the personal-config-repo image-name FLAG (§4) and the checkout-dir-rename implications (§6).

> **Note (2026-07-29, spec 08):** a NEW user decision exists — **the workestrate tool home must NEVER live inside the repo checkout**; the home is the user-global `~/.workestrate` only. Spec: [06-improvements/08-no-repo-local-home.md](06-improvements/08-no-repo-local-home.md) (READY-TO-EXECUTE; code step NEEDS-DEVSHELL). The repo-local bundle at `.workestrate/` is **still present** until the spec is executed. **INTERIM WARNING (spec §5):** until execution step (a) lands (clone `.workestrate/repos/personal` @ `c41a707` → `/home/node/Development/workestrate-personal`), the only committed copy of the personal config lives in the ephemeral container bundle — do NOT rebuild the container or delete the bundle. Execution is sequenced EARLY (07-execution-order.md Step 0.5), before Lane A / the host batch, because it changes the paths those reference. The `--home` flag spec ([06-improvements/06-config-home-flag.md](06-improvements/06-config-home-flag.md)) gains weight: with the discovery tier removed, `--home` becomes THE explicit override (precedence: flag > env > legacy XDG > default).

---

## Maintenance

Update this file whenever `README.md`, `01-current-state-and-prereqs.md`, or
`07-execution-order.md` state changes materially — e.g. a bundle fix moves
from APPLIED to VERIFIED, a Step's env marker changes, a new improvement spec
is added, or the `--home` flag moves from SPEC to IMPLEMENTED. The handoff
prompt must never go stale: its facts are a strict subset of the four cited
docs, so when those docs change, re-derive the affected prompt section from
them and note the update here.
