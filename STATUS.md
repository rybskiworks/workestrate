# STATUS — post-migration + in-container execution snapshot (2026-08-07)

> Snapshot after: the 0.6.8 fork migration committed, and this session's
> in-container execution of B2–B4 + Change #1. The nix store is shared with
> the host and now holds real 0.6.8 build outputs. The full operations doc
> is `NEXT-SESSION.md`.

## Repo states (verified 2026-08-07)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | HEAD on `migration/tool-model` | clean, **15 ahead of origin, NOT pushed**; origin SSH |
| personal config repo | `e3d65e3` | clean; flake.lock pins workestrate @ `c45494b` (B5 HOST-GATED) |
| dev home | (workestrate-dev-home) | clean; `sources/` EMPTY; workestrate.lock pins `b1c87416` |
| microsandbox fork | `74919059` `fix/filesystem-agentd-path-override` | clean; origin/fix == 74919059 (pushed); origin/main `b43d7522` (divergent); remote ambiguous — live ls-remote before fork work |

## Store / artifact state (verified 2026-08-07)

- `microsandbox-fork` input LOCKED (c162c7a): fork source
  `/nix/store/y9rc99j2n2gcr3wvgi4n36kqbhd396v5-source`.
- `.#agentd` → `…97ngwbmkq…-microsandbox-agentd-static-x86_64-unknown-linux-musl-0.6.8`
  (fully static, no .dynamic).
- `.#microsandbox` → `/nix/store/8fi13yc38pwij5lwwxrl1rmpvlmzjp0h-microsandbox-0.6.8`
  (`msb 0.6.8`; RPATH baked in via autoPatchelfHook — runs with no `LD_LIBRARY_PATH`).
- `.#workestrate` → `/nix/store/zk6qxvlj8pm34gqiqkvbvpzlhl0mvq6s-workestrate-0.1.0`
  (rebuilt against the RPATH-encapsulated msb; `0.1.0-<rev>` once committed).
- Cargo.lock refreshed (165ef88): microsandbox 0.6.8 fork crates, sea-orm 2.0.1;
  no registry 0.5.6 pins remain.
- `control/agentctl/vendor/microsandbox-fork` symlink present (→ fork source);
  old 0.5.6 symlink removed.
- NO image tarballs in the store (`workestrate-pi`/`tempest` GC'd earlier);
  container msb store empty; nothing loaded.

## What landed this session

## Host build failure fixed (2026-08-07 evening)

- Host `nix build .#workestrate` failed — drv `94lv9jaln8siy729pwv1xqxk30cpr41l`, exit 101. Root cause: the fork SDK `sdk/rust/build.rs` took its runtime-deps DOWNLOAD branch because the staged `msb` version probe (`installed_msb_version`, build.rs:118-133) could not exec — msb links libcap-ng dynamically without an RPATH. Hermetic host sandbox blocks the download → exit 101. In-container the same branch succeeded earlier (container sandbox permits network) — that impurity masked the bug.
- Fix: RPATH encapsulation — `nix/packages/microsandbox.nix` adds `autoPatchelfHook` to `nativeBuildInputs` so msb is patched in `postFixup` with an RPATH to libcap-ng + libgcc; `nix/packages/agentctl.nix` carries NO env additions (the intermediate `LD_LIBRARY_PATH` export, commit 2593604, was reverted). Zero `LD_LIBRARY_PATH` anywhere in `nix/`. `MSB_HOME` + `MSB_AGENTD_PATH` staging unchanged; no feature changes.
- Validated in-container with `--option sandbox true` (sandbox confirmed engaged): `.#microsandbox` → `/nix/store/8fi13yc38pwij5lwwxrl1rmpvlmzjp0h-microsandbox-0.6.8`; `patchelf --print-rpath …/bin/msb` shows libcap-ng + libgcc store paths; `env -u LD_LIBRARY_PATH …/bin/msb --version` → `msb 0.6.8` (was `error while loading shared libraries: libcap-ng.so.0`); `.#workestrate` drv `200x0mjwd6klb3i6svn810j3sszh0hyf` → `/nix/store/zk6qxvlj8pm34gqiqkvbvpzlhl0mvq6s-workestrate-0.1.0`, 0 "download" lines in `nix log`; `.#agentd` no-op (cached `97ngwbmkqn3ig3mrma6y7y4kk7g5fgci`); `workestrate --version` OK; `scripts/check-nix-paths.sh` clean.
- Repo state: workestrate now **15 ahead of origin, NOT pushed** (4 new commits: the LD_LIBRARY_PATH fix 2593604 + its docs 93df4b0 + the RPATH encapsulation fix + this docs entry).

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

**TEST-READY in-container (all gates green, 2026-08-07):** disk freed by
user; `nix build .#workestrate` (Change #1) PASSED → `7cgrm2ryfk…` /
`0.1.0-1b1b098`; fmt + clippy `-D warnings --all-targets` clean; full
`cargo test` → **755 passed / 0 failed** (incl. all 7 Change #1 tests); the
DB-pool ignored test alone → ok; Cargo.lock stable; lint-nix / tombi-check /
golden-check / store-audit OK; toolchain 1.97==1.97 (note: `just
toolchain-check` recipe needs the flake.nix:28 marker uncommented — user
decision); binary smoke `validate-config` OK. **Host-only:** 3 KVM tests, E1,
boot batch, host-provision, B5 relock, B6 push, B7 load-images + stale
`workestrator-pi:latest`, age key. Beads deferred.

## Remaining (ordered)

See `NEXT-SESSION.md` "Remaining runbook": free disk → in-container test
gates → B5 host relock → B6 push (USER) → fork PR re-verify (USER) → B7
load-images (USER: stale tag) → B8 host-provision (age key) → boot batch →
3 KVM tests LAST → E1.
