# STATUS — post-migration + in-container execution snapshot (2026-08-07)

> Snapshot after: the 0.6.8 fork migration committed, and this session's
> in-container execution of B2–B4 + Change #1. The nix store is shared with
> the host and now holds real 0.6.8 build outputs. The full operations doc
> is `NEXT-SESSION.md`.

## Repo states (verified 2026-08-07)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | `574a2b6` on `migration/tool-model` | clean, **8 ahead of origin, NOT pushed**; origin SSH |
| personal config repo | `e3d65e3` | clean; flake.lock pins workestrate @ `c45494b` (B5 HOST-GATED) |
| dev home | (workestrate-dev-home) | clean; `sources/` EMPTY; workestrate.lock pins `b1c87416` |
| microsandbox fork | `74919059` `fix/filesystem-agentd-path-override` | clean; origin/fix == 74919059 (pushed); origin/main `b43d7522` (divergent); remote ambiguous — live ls-remote before fork work |

## Store / artifact state (verified 2026-08-07)

- `microsandbox-fork` input LOCKED (c162c7a): fork source
  `/nix/store/y9rc99j2n2gcr3wvgi4n36kqbhd396v5-source`.
- `.#agentd` → `…97ngwbmkq…-microsandbox-agentd-static-x86_64-unknown-linux-musl-0.6.8`
  (fully static, no .dynamic).
- `.#microsandbox` → `/nix/store/2zx3nga6z0jxdyrnhk0klqx2djqjvhfn-microsandbox-0.6.8`
  (`msb 0.6.8`; deterministic — same path as the old GC'd `result` target).
- `.#workestrate` → `/nix/store/b68s5ik99lww3q154shvzp0yh3kzbyzy-workestrate-0.1.0`
  (`workestrate 0.1.0-44ee1a0`; NOTE: built before Change #1; current-tree
  build is disk-blocked).
- Cargo.lock refreshed (165ef88): microsandbox 0.6.8 fork crates, sea-orm 2.0.1;
  no registry 0.5.6 pins remain.
- `control/agentctl/vendor/microsandbox-fork` symlink present (→ fork source);
  old 0.5.6 symlink removed.
- NO image tarballs in the store (`workestrate-pi`/`tempest` GC'd earlier);
  container msb store empty; nothing loaded.

## What landed this session

- nix 2.35.1 activated from the store (no install); default sandbox worked.
- B2 flake lock `c162c7a`; B3 builds agentd/microsandbox/workestrate (evidence
  above); B4 Cargo.lock relock `165ef88` + 0.6.8 API fix `44ee1a0`
  (SandboxHandle::status → refresh+status_snapshot) + fmt/clippy clean.
- **Change #1 `574a2b6`**: `workestrate source clone` materializes flake://
  sources from the declaring config repo's flake.lock (github/git nodes →
  clone_full + checkout rev) — **B9 eliminated**; 7 new unit tests added
  (not yet executed — disk).
- B5 personal relock HOST-GATED (host path not visible in-container; error
  recorded, URL untouched; dev-home home init no-op).

## Readiness verdict

**Partially test-ready in-container**: nix + lock + vendor + Cargo.lock are
all in place, so the C gates (611+ tests incl. Change #1 tests, `just
verify`/`verify-full`) can run once disk is freed (**2.3G free now; the 15G
`~/.cache/ai-workbench/agentctl-target` clippy cache blocks compiles** —
user deletes it or host GC). **Host-only:** 3 KVM tests, E1, boot batch,
host-provision, B5 relock, B6 push, B7 load-images + stale
`workestrator-pi:latest`, age key. Beads deferred.

## Remaining (ordered)

See `NEXT-SESSION.md` "Remaining runbook": free disk → in-container test
gates → B5 host relock → B6 push (USER) → fork PR re-verify (USER) → B7
load-images (USER: stale tag) → B8 host-provision (age key) → boot batch →
3 KVM tests LAST → E1.
