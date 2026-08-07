# NEXT-SESSION — workestrate host/agent operations

> Purpose: definitive operations doc for the workestrate test stack. Read
> top-to-bottom; the runbook is self-contained. Beads are DEFERRED (user
> decision 2026-08-07) — do not chase wrk-*.

## What landed (2026-08-07 session set)

- **nix activated in-container** (no install): `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin/nix`
  (export PATH to use it). The shared store is node-owned and writable;
  sandbox builds worked with the DEFAULT sandbox (no `--option sandbox false`
  needed). The 2026-08-03 in-container-build pattern is confirmed real.
- **B2 flake lock** — `c162c7a`: `microsandbox-fork` input added (github pin
  74919059, narHash verified); fork source at `/nix/store/y9rc99j2n2gcr3wvgi4n36kqbhd396v5-source`.
- **B3 builds (all in-container, shared store)**:
  - `.#agentd` → `/nix/store/97ngwbmkqn3ig3mrma6y7y4kk7g5fgci-microsandbox-agentd-static-x86_64-unknown-linux-musl-0.6.8`
    — **fully static** (no .dynamic section, readelf-verified).
  - `.#microsandbox` → `/nix/store/2zx3nga6z0jxdyrnhk0klqx2djqjvhfn-microsandbox-0.6.8`
    (deterministic: same path as the old GC'd `result` target) — `msb 0.6.8`
    (run raw needs `LD_LIBRARY_PATH=<libcap-ng store>/lib`; the wrapped
    runtime handles it).
  - `.#workestrate` → `/nix/store/7cgrm2ryfk5nbr1958q7ssax14bqr0n9-workestrate-0.1.0`
    — `workestrate 0.1.0-1b1b098` (rev-embedded; rebuilt from the current tree
    with Change #1).
- **B4 devshell + Cargo.lock + API fix**:
  - The devshell env depends on `.#workestrate`, whose build failed on the
    STALE lock (fork requires `sea-orm ^2.0.0`; tool lock had 1.1.20) — a
    lock→devshell circularity. Broke it with
    `nix shell nixpkgs#cargo nixpkgs#rustc -c cargo generate-lockfile …`
    (resolve-only, no compile) against a manually created
    `control/agentctl/vendor/microsandbox-fork → <fork source>` symlink.
  - `165ef88` Cargo.lock relock (627 pkgs; microsandbox 0.6.8 fork; sea-orm 2.0.1).
  - `44ee1a0` the ONE 0.6.8 API break: `SandboxHandle::status()` removed →
    `refresh().status_snapshot()` (src/microsandbox/runtime/mod.rs:35).
  - fmt clean; `cargo clippy -D warnings` clean (verified on the 44ee1a0 tree).
- **CHANGE #1 landed** — `574a2b6` (feat(agentctl): materialize flake://
  sources from the config repo's flake.lock): `workestrate source clone`
  now reads the declaring config repo's `flake.lock` (github/git nodes) and
  does `git clone_full` + `git checkout <rev>` into `sources/<name>/repo`
  instead of printing guidance. **Runbook B9 is ELIMINATED** — no more
  hand-cloning; the config's pinned inputs are the source of truth. 7 new
  unit tests added (network-free) but NOT yet executed (disk — see below).
- **B5 personal relock HOST-GATED**: `nix flake lock --update-input workestrate`
  fails exactly as expected — `Git repository "/home/rybski/Development/agent-workbench/workestrate" does not exist`
  (host path not visible in-container). URL NOT rewritten. Dev-home
  `workestrate home init` is a no-op (lock still pins `b1c87416`).

## Validation gates — ALL GREEN (2026-08-07, post disk-free)

- **Disk freed by user** (15G cargo cache deleted): `/` back to 49–61G free.
- **Compile gate (Change #1)**: `nix build .#workestrate` from the current
  tree → `7cgrm2ryfk…` / `workestrate 0.1.0-1b1b098`. PASSED.
- **fmt** clean; **clippy `-D warnings --all-targets`** clean.
- **Full test suite**: `cargo test` → **755 passed / 0 failed** (604 lib +
  151 integration/unit/doc incl. ALL 7 new Change #1 tests); 4 ignored
  (3 KVM + 1 DB-pool) skipped by default as designed. The DB-pool ignored
  test run ALONE (`-- --ignored`) → **ok** (17.95s).
- **Lock stability**: `git diff --exit-code -- Cargo.lock` → stable.
- **Repo gates**: lint-nix OK (16 nix + 6 shell/justfile + 507 docs);
  tombi-check OK (5 files); golden-check OK; schema-check/scaffold-check/
  spec-examples OK (in the suite); store-audit OK (no oversized source);
  toolchain equivalent OK (rustc 1.97 == fenix pin) — NOTE the `just
  toolchain-check` recipe itself would fail to parse because flake.nix:28
  has the `RUST_TOOLCHAIN_VERSION` marker commented out (pre-existing;
  uncommenting is a flake.nix edit — user decision).
- **Binary smoke** (`workestrate --home <dev-home> validate-config`) → OK
  ("workestrate.toml is valid"); `check` → exit 1 only on environment-gated
  findings (host-path trusted project MISSING, sources not checked out,
  reference config unresolved) — expected in-container.
- GC remains prohibited in-container (shared store).

## Current repo state (verified 2026-08-07)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | HEAD on `migration/tool-model` | clean, 9 ahead of origin, NOT pushed; origin SSH |
| personal config repo | `e3d65e3` | clean; flake.lock pins workestrate @ `c45494b` (B5 HOST-GATED) |
| dev home | workestrate-dev-home | clean; `sources/` EMPTY; workestrate.lock pins `b1c87416` |
| microsandbox fork | `74919059` `fix/filesystem-agentd-path-override` | local clean; origin/fix == 74919059 (pushed); origin/main = `b43d7522` (divergent); remote state AMBIGUOUS — re-verify with live `git ls-remote` before fork work |

## Readiness verdict

- **In-container: TEST-READY** — all validation gates green (see above);
  Change #1 fully verified (compile + 7 tests). `just verify`/`verify-full`
  equivalents all pass.
- **Host-only (hard constraints):** the 3 ignored KVM tests, E1 loopback,
  host boot batch, `just host-provision`, `msb load` (B7), B5 relock,
  B6 push, stale `workestrator-pi:latest` prune, age key.
- Nothing of the image stack is loaded (`workestrate-pi`/`tempest` tarballs
  remain GC'd; container msb store empty).

## Remaining runbook (host + user-decision)

1. ~~User frees disk~~ **DONE** (15G cache deleted; validation gates all
   green in-container — see "Validation gates" above).
2. **Remaining in-container (optional):** `just verify` / `just verify-full`
   as the aggregated gate (note the `toolchain-check` parse caveat above).
3. **B5 (HOST-GATED):** `nix flake lock --update-input workestrate` in the
   personal repo (host path resolves there); commit; refresh dev-home
   `workestrate.lock` via `workestrate --home <dev-home> home init`.
4. **B6 PUSH (USER DECISION):** `git push origin migration/tool-model` (SSH;
   8 unpushed commits incl. migration + this session's work). Unblocks the
   later `github:` input adoption.
5. **Fork PR re-verify (USER DECISION):** live `git ls-remote origin` in
   forks/microsandbox/repo first; then decide open/close/leave the PR.
6. **B7 load-images (personal repo, USER DECISION on stale tag):**
   `just load-images` (full rebuild; old tarballs GC'd) → `msb image ls` →
   prune stale `workestrator-pi:latest` if present.
7. **B8 host-provision:** `just host-provision` (needs real age key for full doctor OK).
8. **Host boot batch:** up litellm → health → plan/exec pi, opencode, tempest
   → batch up → ps → down-all.
9. **3 ignored KVM tests LAST** (`lifecycle_detached`, `flake_root_gate`,
   `ensure_images_e2e` with `MSB_PATH=$(nix path-info .#microsandbox)/bin/msb`)
   → **E1** loopback experiment (docs/validation-and-improvements/05-host-validation.md:282-321).
10. **Bead close-out:** still deferred (user decision); no bead work.
