# STATUS — post-migration snapshot (2026-08-07)

> Snapshot after the microsandbox 0.6.8 fork migration LANDED
> (commits `84a901d`…`2ec1c6e` + `c413b8a` on `migration/tool-model`).
> The nix store is shared with the host, but NOTHING of the 0.6.8 stack is
> built yet — all builds, loads, and boots remain host-side. The full host
> runbook is in `handovers/2026-08-07-…md`; this file is the narrative
> snapshot and `NEXT-SESSION.md` the resumption context.

## Repo states (verified 2026-08-07)

| Repo | HEAD / branch | Tree | Notes |
|------|---------------|------|-------|
| workestrate | `c413b8a` on `migration/tool-model` | clean, **2 ahead of origin** (c413b8a + doc commit 5dcf3cf) | migration committed; origin remote is SSH (`git@github.com:georgrybski/workestrate.git`) |
| personal config repo | `e3d65e3` | clean | workestrate input relocked to host path; smoke workload committed; `.env.enc` present (unverifiable in-container) |
| dev home | (workestrate-dev-home) | clean | `sources/` EMPTY; `workestrate.lock` pins personal @ `b1c87416` (pre-`e3d65e3` — refresh in B5) |
| microsandbox fork | `74919059` on `fix/filesystem-agentd-path-override` | clean | `origin/fix/filesystem-agentd-path-override` == `74919059` (pushed); `origin/main` = `b43d7522` (#1), a different line (diverged; only a cosmetic `build.rs` reorder differs) |

## Committed migration summary

Replaced the 0.5.6 pinned-trio with a source-built 0.6.8 stack from the
user's fork, pinned via flake input at the validated rev 74919059:

- **msb + agentd source-built** from the fork workspace: `nix/packages/microsandbox.nix`
  rewritten (`rustPlatform.buildRustPackage`, fenix-pinned toolchain,
  `-p microsandbox-cli --no-default-features --features net,ssh`); `nix/packages/agentd.nix`
  NEW (`pkgsStatic` musl guest-init binary).
- **`microsandbox-fork` flake input** (`flake = false`, github pin 74919059)
  replaces `builtins.fetchGit` + the deleted `microsandbox-filesystem-agentd.patch`.
- **Vendored fork workspace** under `control/agentctl/vendor/microsandbox-fork`
  (symlink recreated by the devshell shellHook); all 12 `microsandbox-*` crates
  patched via both `.cargo/config.toml` files.
- **libkrunfw** from the upstream v0.6.8 release tar (sha256 verified real —
  the fakeHash is gone).
- **build.rs version wiring** (`WORKESTRATE_REV`) + **`scripts/host-provision.sh`**.

## Store / artifact state (verified 2026-08-07)

- `workestrate/flake.lock` — **`microsandbox-fork` input MISSING** (lock stale; B2 fixes).
- `control/agentctl/Cargo.lock` — still pins **registry `microsandbox 0.5.6`** (B4 refresh).
- `control/agentctl/vendor/` — empty in a clean checkout; the stale
  `microsandbox-filesystem-0.5.6` symlink was removed 2026-08-07 (untracked/gitignored).
- `/nix/store` — **NO 0.6.8 outputs, no fork-source fetch, no agentd output**
  (only `agentd-x86_64.drv`). The pre-migration image tarballs
  `workestrate-pi` (`vj844190…`) and `tempest` (`ajgqk1…`) are **GC'd**.
  Remaining: `workestrate-0.1.0` output (`vg28f2s0c…`) + 0.5.6 crate drvs.
- `workestrate/result` symlink (dangling, target GC'd) removed 2026-08-07.
- Container `~/.microsandbox` — EMPTY store; `bin/msb` needs GLIBC 2.38+
  (not runnable in-container). Host store state (incl. any stale
  `workestrator-pi:latest` tag) is NOT visible from the container.
- Cleanups this pass: removed `workestrate/result` (broken), stale vendor
  symlink, and `workestrate/core` (1.7 GB ELF core dump). Git tree unchanged
  (all three were untracked/gitignored).

## What remains (host, ordered — B1–B9 + C)

1. **B1** baseline: `df -h /` + `./scripts/host-check.sh` (+ `just gc` only if disk tight).
2. **B2** `nix flake lock` in workestrate + commit.
3. **B3** `nix build .#agentd` → `nix build .#microsandbox` → `nix build .#workestrate`.
4. **B4** `nix develop -c bash` → `cargo check` → commit Cargo.lock refresh.
5. **B5** personal relock (`nix flake lock --update-input workestrate`) + commit; refresh dev-home `workestrate.lock`.
6. **B6** `git push origin migration/tool-model` (USER DECISION).
7. **B7** `just load-images` (personal repo) — full rebuild, old tarballs GC'd.
8. **B8** `just host-provision` (needs real age key for full doctor OK).
9. **B9** populate `sources/<odysseus|opencode|tempest>/repo` (optional for core tests).
10. **C** test gates in dependency order → 3 ignored KVM tests LAST → E1 → bead close-out.

## Bead state

- `wrk-23b` — ABSENT from `.beads/issues.jsonl` (refile PENDING, host; see NEXT-SESSION).
- `wrk-vic` — open; expected head `27d84216` stale vs actual `74919059` (branch pushed); PR state unknown.
- `wrk-ayz` — open; partially executed (github-input pattern used for microsandbox-fork); objective pending B6 push.
- `wrk-wv0` — open; executed by the migration (0.6.8 pin landed).
- `wrk-8yg` — open, blocked; first host beads sync / `bd dolt push` never done (needs user approval).

## Historical note (superseded 2026-08-07)

The 2026-08-03 pre-migration pass landed the host-boot fixes (`b675db2`
mount/seed path doubling; personal flake URL repoint; `c6a6b47` plan-time
existence preflight; 597 lib tests) and built the `workestrate-pi` + `tempest`
nix-layered images (real tarballs, musl recipe fix `5532eb7`, real tempest
npmDepsHash). Those tarballs were GC'd; the migration supersedes the pinned
trio they were built against.
