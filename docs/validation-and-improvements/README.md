# Validation and Improvements — workestrate Config-Driven Setup

**Status:** ACTIVE — execution-ready tree
**Branch:** `migration/tool-model`
**Authored:** 2026-07-28

This documentation tree is the execution record for validating that the new
config-driven workestrate setup (data-driven TOML workloads, ADR 0001)
reproduces the pre-migration behavior of the 5 real workloads
(litellm/pi/odysseus/opencode/tempest), then implementing the prioritized
improvements (mount shadowing, main standardization, dogfooding guardrails,
cwd-fallback fix; CLI authoring deferred). It is the operational companion to
the authoritative design record at [`../migration/README.md`](../migration/README.md).

## How a contextless session uses this tree

No prior conversation is needed. Every file in this tree carries its own status
banner and a see-also header, and commands are spelled identically across
files. Read in this order:

1. **This index** (`README.md`) — you are here.
2. [`00-overview.md`](00-overview.md) — the whole effort in ≤2 pages.
3. [`01-current-state-and-prereqs.md`](01-current-state-and-prereqs.md) —
   verified current state + prerequisites checklist.
4. **Pick a lane:**
   - **Lane A (no-KVM, verifiable-here):**
     [`04-baseline-validation.md`](04-baseline-validation.md) — config-plane
     and plan-plane validation. Establishes the pre-migration baseline via git
     history (the original 5 have no committed golden plans).
   - **Lane B (KVM host):**
     [`05-host-validation.md`](05-host-validation.md) — the ordered HOST-KVM /
     HOST-NIX runbook (B1–B12) for runtime parity.
5. [`03-sibling-config-setup.md`](03-sibling-config-setup.md) — the disposable
   experiment home for destructive probes (never touches the real bundle).
6. [`06-improvements/`](06-improvements/) — improvement specs, indexed by
   [`06-improvements/00-index.md`](06-improvements/00-index.md).
7. [`07-execution-order.md`](07-execution-order.md) — recommended sequencing
   across all tracks.

## Doc map

| File | Description | Status banner |
|---|---|---|
| `README.md` (this file) | Index + entry protocol | ACTIVE — execution-ready tree |
| [`00-overview.md`](00-overview.md) | The whole effort in ≤2 pages | `STATUS: READY-TO-EXECUTE` |
| [`01-current-state-and-prereqs.md`](01-current-state-and-prereqs.md) | Verified current-state snapshot + prerequisites checklist | `STATUS: READY-TO-EXECUTE` |
| [`02-config-requirements.md`](02-config-requirements.md) | The `workestrate.toml` contract: schema, merge/layering, policy ceiling, trust model, planned extensions | `STATUS: READY-TO-EXECUTE (requirements frozen pending sign-off)` |
| [`03-sibling-config-setup.md`](03-sibling-config-setup.md) | Disposable experiment tool home for destructive config/plan probes | `STATUS: READY-TO-EXECUTE (verifiable-here; no KVM needed for plan/config-plane probes)` |
| [`04-baseline-validation.md`](04-baseline-validation.md) | Baseline recovery procedure + Lane A gate suite (no-KVM) | `STATUS: READY-TO-EXECUTE (Lane A verifiable-here; runtime parity is HOST-KVM — see 05-host-validation.md)` |
| [`05-host-validation.md`](05-host-validation.md) | Lane B ordered runbook: HOST-KVM / HOST-NIX steps B1–B12 | `STATUS: NEEDS-KVM` — the entire file is a `HOST-KVM` / `HOST-NIX` runbook |
| [`06-improvements/00-index.md`](06-improvements/00-index.md) | Improvements index with status/dependencies/effort | STATUS: INDEX |
| [`06-improvements/01-mount-filtering-shadowing.md`](06-improvements/01-mount-filtering-shadowing.md) | Track A: nested shadow mounts spec (exclude/shadow/allow_sensitive) | `STATUS: SPEC (not yet implemented); Phase 0 spike is NEEDS-KVM` |
| [`06-improvements/02-main-standardization.md`](06-improvements/02-main-standardization.md) | Rename the personal clone `master` → `main` | `STATUS: READY-TO-EXECUTE` |
| [`06-improvements/03-dogfooding.md`](06-improvements/03-dogfooding.md) | Track C: workestrate developing workestrate (driver/target isolation) | `STATUS: SPEC (Phase 0 env pinning READY-TO-EXECUTE; B1/B2/B3 not implemented)` |
| [`06-improvements/04-cli-config-authoring.md`](06-improvements/04-cli-config-authoring.md) | DEFERRED vision + requirements traceability for the toml_edit CLI authoring tool | `STATUS: DEFERRED` |
| [`06-improvements/05-config-reference-cwd-fallback.md`](06-improvements/05-config-reference-cwd-fallback.md) | Standalone fix spec for the config-reference cwd-fallback quirk | `STATUS: SPEC (bug fix candidate, small)` |
| [`06-improvements/06-config-home-flag.md`](06-improvements/06-config-home-flag.md) | `--home` global CLI flag (idiomatic config-home override) | `STATUS: EXECUTED (2026-07-30; commit d991252)` |
| [`06-improvements/07-naming-consistency.md`](06-improvements/07-naming-consistency.md) | Purge `workestrator` residue; standardize on `workestrate` | `STATUS: IN-PROGRESS THIS BRANCH` |
| [`06-improvements/08-no-repo-local-home.md`](06-improvements/08-no-repo-local-home.md) | Retire the repo-local tool home: `~/.workestrate` only, never inside the checkout; removes the discovery tier | `STATUS: EXECUTED (2026-07-30); commits d7c5a83, bd99481, 3894fb7, bef1c37, 418530a; home commit a42e597` |
| [`06-improvements/09-microsandbox-agentd-offline-build.md`](06-improvements/09-microsandbox-agentd-offline-build.md) | microsandbox-filesystem agentd build-time download: offline-build fix options (upstream MSB_HOME fix / ADR 0011 git-fork carrier / 0.6.8 bump) | `STATUS: READY-TO-EXECUTE (option 2 gated on fork push access; option 1 gated on upstream responsiveness; option 3 NEEDS-DEVSHELL + HOST-NIX)` |
| [`06-improvements/10-config-repos-as-working-copies.md`](06-improvements/10-config-repos-as-working-copies.md) | Config repos as working copies in the tool home + dotfiles-style home repo (gitlink-guarded `home init`; `repos/` → `config-repos/` rename) | `STATUS: EXECUTED (2026-07-30); code tasks landed (d7c5a83 rename, bd99481 dirty-guard test, 3894fb7 home init)` |
| [`06-improvements/11-home-provisioning-and-lockfile.md`](06-improvements/11-home-provisioning-and-lockfile.md) | Home provisioning (`home clone <src> [<dest>]`) + generated `workestrate.lock` pin file (executes ADR 0025; verb split per the ADR 0025 addendum) | `STATUS: EXECUTED (2026-07-30; commits 172d5dd, 19ff272, be356f7; verb split c406630)` |
| [`06-improvements/12-per-instance-addressing.md`](06-improvements/12-per-instance-addressing.md) | Per-instance addressing + discovery-lite (executes ADR 0026; supersedes ADR 0021 §5 `--port-offset`) | `STATUS: IMPLEMENTED (Waves 1+2 landed; Experiment E1 guest-reachability NEEDS-KVM)` |
| [`06-improvements/13-secret-env-shorthand.md`](06-improvements/13-secret-env-shorthand.md) | Config ergonomics: string-or-table shorthand for `secret_env` (additive; `schema_version` stays 1) | `STATUS: EXECUTED (2026-07-31; commits a349d03, 1e7dc25)` |
| [`07-execution-order.md`](07-execution-order.md) | Recommended sequencing across all tracks | STATUS: READY-TO-EXECUTE |
| [`NEXT-SESSION.md`](NEXT-SESSION.md) | Self-contained handoff prompt for the next contextless session | `STATUS: HANDOFF` |

## The three tracks

**Track A — mount filtering/shadowing**
([`06-improvements/01-mount-filtering-shadowing.md`](06-improvements/01-mount-filtering-shadowing.md)).
Three of the five real workloads bind-mount `${CWD}` read-write into the
sandbox, exposing host-repo secrets (`.env`, `.git/`, `.workestrate/`). Track A
adds `exclude` globs, `[[mounts.shadow]]` entries, and a trust-gated
`allow_sensitive` opt-out, enforced post-merge at plan time via a new
`SENSITIVE_MOUNT_EXCLUDE_PATTERNS` const in `policy.rs`. The design is
zero-copy (nested shadow mounts over the live bind) and ADR 0005-conformant
(monotonic deny). A Phase 0 KVM spike gates whether the microsandbox runtime
honors nested mount ordering before WP4 ships.

**Track B — config home + validation**
([`01`](01-current-state-and-prereqs.md) +
[`03`](03-sibling-config-setup.md) +
[`04`](04-baseline-validation.md) +
[`05`](05-host-validation.md)).
Establish the pre-migration behavioral baseline for the original 5 workloads
via a git-worktree-based recovery procedure (the originals were deleted in
commit `32040c3` and have no committed golden plans), then prove the new
config-driven `ConfigWorkload` reproduces them. Lane A (no-KVM) covers
config-plane and plan-plane validation; Lane B (HOST-KVM) covers runtime
parity — service boot, agent attach, egress posture, instance lifecycle.

**Track C — dogfooding**
([`06-improvements/03-dogfooding.md`](06-improvements/03-dogfooding.md)).
Use workestrate to develop workestrate: an agent in a sandbox edits the
workestrate source tree while the driver binary that spawned it must remain
insulated. Phase 0 (READY-TO-EXECUTE) closes the config-reference
cwd-fallback backdoor via an env-pinning wrapper with no code change; Phase 1
(NOT IMPLEMENTED) adds three structural hardening features (B1 self-home mount
guard, B2 config-free teardown verification, B3 spawn provenance).

**Cross-cutting additions:**

- **Main standardization**
  ([`06-improvements/02-main-standardization.md`](06-improvements/02-main-standardization.md))
  — a one-shot `git branch -m master main` on the personal clone to resolve
  the registry/clone mismatch (`ref = "main"` vs actual branch `master`).
  `verifiable-here`; no KVM/nix needed.
- **CLI config authoring (DEFERRED)**
  ([`06-improvements/04-cli-config-authoring.md`](06-improvements/04-cli-config-authoring.md))
  — the toml_edit-based CLI authoring tool is deferred pending sign-off of
  [`02-config-requirements.md`](02-config-requirements.md). The requirements
  are pinned so the work is not redone.
- **cwd-fallback fix**
  ([`06-improvements/05-config-reference-cwd-fallback.md`](06-improvements/05-config-reference-cwd-fallback.md))
  — a standalone small fix for the quirk where the nix-installed binary
  silently loads `config.reference/workestrate.toml` from cwd as the base
  config layer. Recommended fix: gate behind an explicit opt-in env.

## Environment honesty

This documentation was authored in a container with nix present only at a store path (`/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH) and **no KVM**,
mirroring the convention in
[`docs/migration/README.md`](../migration/README.md) §Environment honesty.
Environment markers (reproduced from
[`docs/migration/40-migration-process.md`](../migration/40-migration-process.md)
lines 8-14):

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no KVM) |

All nix-eval and KVM-runtime gates are marked `HOST-NIX` or `HOST-KVM` in the
relevant files. Runtime claims are **specifications, not verified
observations**, unless a file explicitly says a command was actually run (e.g.
the git inspections in [`01`](01-current-state-and-prereqs.md) and the
`config update` failure trace in
[`06-improvements/02-main-standardization.md`](06-improvements/02-main-standardization.md)
were run in this container and their outputs pasted verbatim).

## Key invariants

- **Docs-only tree.** This tree contains no code changes. All implementation
  (Rust, nix, shell, TOML) happens in subsequent commits against this branch,
  gated by the sequencing in [`07-execution-order.md`](07-execution-order.md).
- **The real bundle is read-mostly.** `.workestrate/` at the repo root is the
  live personal deployment. Destructive experiments go to the disposable
  experiment home at `/tmp/workestrate-exp` (see
  [`03-sibling-config-setup.md`](03-sibling-config-setup.md)). No probe in
  this tree writes to the real bundle.
- **No committed pre-migration golden plans for the original 5.** The original
  hardcoded Rust workload modules were deleted in commit `32040c3`.
  `just golden-check` covers only the 3 synthetic `config.reference`
  workloads. Parity for the original 5 is proven by the baseline recovery
  procedure in [`04-baseline-validation.md`](04-baseline-validation.md).
- **CRITICAL known blocker (edit LANDED; runtime verification PENDING).** The tempest `install_layout = "app"` field has been **REMOVED** from `.workestrate/repos/personal/workestrate.toml:317` (fix e applied, verified via `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no match). `BinarySpec` rejects it via `deny_unknown_fields` (`control/agentctl/src/config/types.rs:44-52`); the nix-side `installLayout` param was removed as a silent no-op (`nix/lib/recipes/npm-build.nix:16-25`). The `master`→`main` rename is also **DONE** (fix a applied; clone on `main`, verified: `git branch --show-current` → `main`, rev `d2cd0c3` unchanged). **PENDING:** runtime parse verification — `workestrate validate-config` is cargo-linked and RUNS in this container via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; store-path PATH prefix then `nix develop -c bash -c 'just workestrate validate-config'`; a bare shell has no `cc`); it is the **first Lane A action**. See [`01-current-state-and-prereqs.md`](01-current-state-and-prereqs.md) bundle fixes (a) and (e) and [`07-execution-order.md`](07-execution-order.md) Step 0.

## Authoritative design record

The migration design record — including the 26 ADRs that pin every
load-bearing decision — lives at
[`../migration/README.md`](../migration/README.md). Individual ADRs are at
[`../migration/50-decisions/`](../migration/50-decisions/). This tree is the
operational companion: it validates the design and implements the
improvements, but does not re-litigate decisions (ADRs 0001–0026 stand).
