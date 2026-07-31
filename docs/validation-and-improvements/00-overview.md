# 00 — Overview (the whole effort in ≤2 pages)

> **STATUS: READY-TO-EXECUTE**
> Prerequisites / see-also: [README.md](README.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)

## Goal

This effort has two objectives, in order:

1. **Prove parity.** The new config-driven workestrate (data-driven TOML
   workloads, ADR 0001) must reproduce the pre-migration behavior of the
   original 5 workloads (`litellm`, `pi`, `odysseus`, `opencode`, `tempest`).
   The originals were hardcoded Rust modules deleted in commit `32040c3` and
   have **no committed golden plans** — parity is proven by a baseline
   recovery procedure that generates old plans from the base commit `840e8b7`,
   generates new plans from the current config, and diffs them
   ([04-baseline-validation.md](04-baseline-validation.md)).

2. **Implement prioritized improvements.** Mount filtering/shadowing (Track A),
   main standardization, dogfooding guardrails (Track C Phase 0), and the
   cwd-fallback fix. The CLI authoring tool is **deferred** — its requirements
   are pinned in [02-config-requirements.md](02-config-requirements.md) so the
   work is not redone.

## The 3 phases at a glance

| Phase | Scope | Docs |
|---|---|---|
| **1 — Establish baseline + validate** | Recover pre-migration plans via git worktree; prove config-driven parity (Lane A, no-KVM); Lane B runtime parity on a KVM host. | [01](01-current-state-and-prereqs.md), [03](03-sibling-config-setup.md), [04](04-baseline-validation.md), [05](05-host-validation.md) |
| **2 — Quick wins + guardrails** | Bundle hygiene (main rename), dogfooding Phase 0 env pinning, cwd-fallback fix. | [06-improvements/02](06-improvements/02-main-standardization.md), [06-improvements/03](06-improvements/03-dogfooding.md) §4, [06-improvements/05](06-improvements/05-config-reference-cwd-fallback.md) |
| **3 — Track A mount filtering** | WP1–WP4 (schema+glob, policy+trust, render+audit, runtime shadows), gated by Phase 0 KVM spike. | [06-improvements/01](06-improvements/01-mount-filtering-shadowing.md) |

Phase 1 is the validation gate; Phase 2 is independently shippable quick wins;
Phase 3 is the largest spec and depends on the Phase 0 KVM spike confirming
nested mount ordering. Detailed sequencing lives in
[07-execution-order.md](07-execution-order.md).

## Current state snapshot

- **Bundle** at `.workestrate/` with the personal layer at rev `d2cd0c3` on
  branch `master` — but the registry declares `ref = "main"` (mismatch; see
  [06-improvements/02](06-improvements/02-main-standardization.md)).
- **5 workloads** are config-driven from
  `.workestrate/repos/personal/workestrate.toml` (359 lines): both image
  recipes, three build recipes, the secret alias pattern, and
  `default_deny = false` entitlement (tempest).
- **`just golden-check`** covers only the 3 synthetic `config.reference`
  workloads (`example-service/agent/offensive`) — NOT the original 5.
- **`just verify`** IS runnable in this container via `nix develop`: nix is
  installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`
  (not on PATH) and the devshell provides a full C toolchain (verified
  2026-07-29: gcc 15.2.0, cargo 1.97.1). Run cargo gates with the store-path
  PATH prefix from `control/agentctl/`. A bare shell has no `cc` and runs only
  the shell/python/git-based subset (`toolchain-check`, `litellm-check`,
  `lint-nix`, `store-audit` SKIP, `Cargo.lock` stability).
- **No KVM** in the authoring container (and nix is present but not on PATH) —
  runtime gates are `HOST-KVM`; the genuine `HOST-NIX` gates are only
  `nix build` image builds, `nix run nixpkgs#...` FOD prefetch jobs,
  `just verify-full`, `just generate-schema`, sops secret provisioning, and
  copier-live (see [01](01-current-state-and-prereqs.md) §Environment honesty).

## Key risks

| Risk | Impact | Mitigation | Where addressed |
|---|---|---|---|
| Pre-migration baseline recoverable only via git history (no committed golden plans for the original 5) | Parity cannot be checked with `just golden-check`; requires a worktree-based recovery procedure on a nix-capable host | Baseline recovery procedure: build old binary at `840e8b7`, generate plans, diff against new | [04-baseline-validation.md](04-baseline-validation.md) |
| Nested-mount ordering unverifiable without KVM | The shadow-mount design depends on the microsandbox runtime honoring nested `volume()` call order; if it does not, shadows are silently ineffective | Phase 0 KVM spike is the gate before WP4 ships; WP5 (staging-copy fallback) activates if the spike fails | [06-improvements/01](06-improvements/01-mount-filtering-shadowing.md) §7 Phase 0 |
| `config.reference` cwd-fallback backdoor when dogfooding | An agent in a worktree can edit `config.reference/workestrate.toml` and the driver's next invocation silently loads it as the base layer | Phase 0 env-pinning wrapper (`WORKESTRATE_CONFIG_DIR`); standalone fix: gate reference loading behind an opt-in env | [06-improvements/05](06-improvements/05-config-reference-cwd-fallback.md) + [06-improvements/03](06-improvements/03-dogfooding.md) §3–4 |
| Branch mismatch (`master` vs `main`) breaks `config update` | `workestrate config update personal` fails with `git pull failed` because the clone is on `master` but the registry says `main` | One-shot `git branch -m master main` on the personal clone | [06-improvements/02](06-improvements/02-main-standardization.md) |
| Secrets / SOPS age key are host-only | Secret-dependent commands (`up`, `exec`, `run`) fail without the age key; the authoring container cannot decrypt | Plan-plane commands render secrets as `(redacted)`; runtime decryption is deferred to a host with the age key | [01](01-current-state-and-prereqs.md) §Environment honesty, [05](05-host-validation.md) |

## What this tree is NOT

- **Not a redesign.** The 23 ADRs
  ([`../migration/50-decisions/`](../migration/50-decisions/)) are the final
  word on every load-bearing choice. This tree validates and improves; it does
  not re-litigate.
- **Not code.** This is a docs-only tree. All implementation happens in
  subsequent commits, gated by [07-execution-order.md](07-execution-order.md).
- **Not the CLI authoring tool.** The toml_edit-based CLI config authoring
  tool is **deferred** (see
  [06-improvements/04](06-improvements/04-cli-config-authoring.md)). Its
  requirements are pinned in [02-config-requirements.md](02-config-requirements.md)
  so the work is not redone, but no CLI code exists today.
