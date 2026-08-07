# STATUS — post-migration snapshot (2026-08-07)

> Snapshot after the microsandbox 0.6.8 fork migration landed and the doc
> refresh committed. The nix store is shared with the host, but NOTHING of
> the 0.6.8 stack is built yet — all builds, loads, and boots remain
> host-side. The HOST RUNBOOK is the primary content of `NEXT-SESSION.md`.

## Repo states (verified 2026-08-07)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | HEAD on `migration/tool-model` (migration `c413b8a` + 2 doc commits) | clean, **3 ahead of origin, NOT pushed**; origin remote is SSH (`git@github.com:georgrybski/workestrate.git`) |
| personal config repo | `e3d65e3` | clean; flake.lock pins workestrate @ `c45494b` (**pre-migration**) |
| dev home | (workestrate-dev-home) | clean; `sources/` EMPTY; workestrate.lock pins personal @ `b1c87416` |
| microsandbox fork | `74919059` on `fix/filesystem-agentd-path-override` | local clean; `origin/fix/filesystem-agentd-path-override` == `74919059` (pushed); `origin/main` = `b43d7522` (#1, divergent); **remote state ambiguous — a live `ls-remote` showed `caee6378` not present in local refs; re-verify before fork work** |

## Store / artifact state (verified 2026-08-07)

- `workestrate/flake.lock` — **`microsandbox-fork` input MISSING** (lock stale; runbook B2).
- `control/agentctl/Cargo.lock` — still pins **registry `microsandbox 0.5.6`** (runbook B4).
- `control/agentctl/vendor/` — empty in a clean checkout; the stale
  `microsandbox-filesystem-0.5.6` symlink was removed 2026-08-07 (untracked/gitignored);
  `vendor/microsandbox-fork` is recreated by `nix develop` (runbook B4).
- `/nix/store` — **NO 0.6.8 builds yet**: no fork-source fetch, no 0.6.8 msb
  output, no agentd output (only `agentd-x86_64.drv`). Pre-migration image
  tarballs `workestrate-pi` (`vj844190…`) and `tempest` (`ajgqk1…`) are
  **GC'd**. Remaining: `workestrate-0.1.0` output (`vg28f2s0c…`) + 0.5.6 drvs.
- Container `~/.microsandbox` — EMPTY store; `bin/msb` needs GLIBC 2.38+
  (not runnable in-container). Host store state (incl. any stale
  `workestrator-pi:latest` tag) is not visible from the container.
- Cleanups 2026-08-07: removed `workestrate/result` (broken symlink → GC'd
  target), stale vendor 0.5.6 symlink, `workestrate/core` (1.7 GB ELF core
  dump). Git tree unchanged (all untracked/gitignored).

## What landed

- **Migration committed** `84a901d`…`2ec1c6e` + `c413b8a` (pin
  `microsandbox-fork` flake input at validated rev `74919059`, flake=false;
  user decision). Replaces the 0.5.6 pinned trio: msb + agentd source-built
  from the fork (`microsandbox.nix` rewritten, `agentd.nix` NEW), full fork
  workspace vendored under `control/agentctl/vendor/microsandbox-fork` (all
  12 `microsandbox-*` crates patched via both `.cargo/config.toml` files),
  libkrunfw from the v0.6.8 release tar (sha256 verified real), build.rs
  version wiring, `scripts/host-provision.sh`.
- **Doc refresh** — `4dfeecd` ("docs: refresh NEXT-SESSION/STATUS for committed
  migration") then the doc commit "docs: refresh status and embed host runbook;
  fix migration drift": `STATUS.md` + `NEXT-SESSION.md` rewritten and the HOST
  RUNBOOK embedded; SPEC.md/README.md minimal drift fixes (0.6.8 SDK version,
  fork pin language).

## Readiness verdict

**NOT test-ready yet.** `flake.lock` is unrefreshed (no `microsandbox-fork`
input), `Cargo.lock` is unrefreshed (0.5.6 registry pins), the
`vendor/microsandbox-fork` symlink does not exist, and no 0.6.8 build exists
anywhere in the store. The container cannot build or run tests (no
nix/cargo/ssh/KVM; container msb needs GLIBC 2.38+). Host steps in
dependency order are in the **HOST RUNBOOK in `NEXT-SESSION.md`** (PRE
baseline → B2 flake lock → B3 build chain → B4 devshell/Cargo.lock → B5
personal relock → B6 push (USER DECISION) → B7 load-images → B8
host-provision → B9 sources → C test gates → 3 ignored KVM tests LAST → E1).

## Beads

**DEFERRED (user decision 2026-08-07): wrk-23b refile, wrk-vic/wrk-ayz/
wrk-wv0/wrk-8yg all parked; do not chase.** No bead bookkeeping is pending
in this session set.
