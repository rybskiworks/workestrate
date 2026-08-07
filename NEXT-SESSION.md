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

## Host build failure fixed — SDK runtime-deps download eliminated (2026-08-07 evening)

Root cause (host `nix build .#workestrate` → drv `94lv9jaln8siy729pwv1xqxk30cpr41l`, exit 101):

- The fork's `sdk/rust/build.rs` (vendored at `control/agentctl/vendor/microsandbox-fork/sdk/rust/build.rs`) decides download-vs-skip by: if `$MSB_HOME/lib/libkrunfw.so.5.6.1` exists AND `$MSB_HOME/bin/msb --version` execs and prints `msb 0.6.8` (== `PREBUILT_VERSION`, utils crate version), it skips; otherwise it prints `warning: downloading microsandbox runtime dependencies (v0.6.8)...` and fetches the release bundle over the network (build.rs:63-84; `installed_msb_version` at build.rs:118-133).
- The staged `msb` is dynamically linked against libcap-ng with no RPATH, so the version probe could not exec in the build → probe returned `None` → download branch → host sandbox has no network → build failed. In-container the same branch ran, but the container's sandbox permits network, so the download "succeeded" — the impurity masked the bug.
- Fix: bake the deps into the binary instead of the environment. `nix/packages/microsandbox.nix` adds `autoPatchelfHook` to `nativeBuildInputs`, so in `postFixup` the msb ELF gets an RPATH to its dynamic deps (build log: `setting RPATH to: …libcap-ng-0.9.3/lib:…gcc-15.2.0-libgcc/lib`, `auto-patchelf: 0 dependencies could not be satisfied`). `nix/packages/agentctl.nix` carries NO env additions. The SDK probe works purely because the staged msb is self-contained — zero `LD_LIBRARY_PATH` anywhere (the whole `nix/` tree has no `LD_LIBRARY_PATH` references, devshell included). The intermediate `LD_LIBRARY_PATH` export (commit 2593604) was REVERTED and replaced by this RPATH encapsulation. The `MSB_AGENTD_PATH` contract (fork `crates/filesystem/build.rs:40-45,89-104`) was already satisfied.
- Contract recap (read the fork commit series d8a9bf50/3d26f202/b43d7522/74919059 + code): `MSB_PATH` is RUNTIME-only (sdk/rust/bin/main.rs:65, sdk/rust/lib/config/mod.rs:765); `MSB_AGENTD_PATH` is BUILD-time-only (crates/filesystem/build.rs); `MSB_HOME` is build+runtime home (sdk/rust/build.rs:28, crates/utils/lib/lib.rs:164-171). The BUILD needs only MSB_HOME + MSB_AGENTD_PATH; the version probe needs no env because the msb binary carries its own RPATH. `MSB_PATH` is NOT read by any build script.
- microsandbox.nix builds the CLI with `--no-default-features --features net,ssh` (mirrors fork justfile `build-msb`), so the SDK `prebuilt` feature is OFF and its build.rs does nothing; the filesystem crate takes the non-prebuilt branch satisfied by the staged `build/agentd` (crates/filesystem/build.rs:57-86). The only change there is the `autoPatchelfHook` addition.
- Verified in-container WITH `--option sandbox true` (sandbox engages: seccomp + no-new-privs probe): `.#microsandbox` rebuild → `/nix/store/8fi13yc38pwij5lwwxrl1rmpvlmzjp0h-microsandbox-0.6.8`; `patchelf --print-rpath …/bin/msb` → `…/libcap-ng-0.9.3/lib:…/gcc-15.2.0-libgcc/lib`; `env -u LD_LIBRARY_PATH …/bin/msb --version` → `msb 0.6.8` (previously `error while loading shared libraries: libcap-ng.so.0`). `.#workestrate` build PASSES with NO env exports, drv `200x0mjwd6klb3i6svn810j3sszh0hyf` → `/nix/store/zk6qxvlj8pm34gqiqkvbvpzlhl0mvq6s-workestrate-0.1.0`; `nix log` has ZERO "download" lines; `workestrate --version` OK; `.#agentd` no-op (cached `97ngwbmkqn3ig3mrma6y7y4kk7g5fgci`); `scripts/check-nix-paths.sh` clean.
- HOST RE-RUN (command unchanged; no env vars needed anywhere — the binary is self-contained): after syncing this branch, `cd /home/rybski/Development/agent-workbench/workestrate && nix build .#workestrate --print-out-paths` then `nix log $(nix path-info .#workestrate 2>/dev/null || true)` and confirm no `downloading microsandbox runtime dependencies` line. Expect the build to succeed with no network fetch (host sandbox now blocks nothing — there is nothing left to fetch).

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
| workestrate | HEAD on `migration/tool-model` | clean, 11 ahead of origin, NOT pushed; origin SSH |
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

## HOST RUNBOOK — shell-context annotated (host, dblab42)

> User question answered: **"Will I need to run within `nix develop` / `nix run`, or are these encapsulated in the current ones?"** — Each step is tagged below. The workestrate CLI is the ONLY thing that works in any shell (it is nix-profile-installed with MSB_HOME/MSB_PATH baked in); `just`, `msb`, `cargo`, and `clippy` live ONLY in the tool devshell; git/nix/python/curl work in any shell.

### Shell-context legend

- **[ANY-SHELL]** — plain bash on the host with nix on PATH; no devshell needed.
- **[DEV-SHELL]** — needs the tool devshell. One-shot: `nix develop -c bash -c '<cmd>'` (run from `~/Development/agent-workbench/workestrate`). Enter-once: `nix develop` then run commands inside.
- **[PROFILE]** — the nix-profile-installed `workestrate` binary (nix-profile-installed with `MSB_PATH` + `MSB_HOME` baked by the wrapper — works in ANY shell; **always** pass `--home <dev-home>`).

### How to get ready on the host

1. nix with flakes on PATH (`command -v nix && nix --version`; host-check.sh needs flakes enabled).
2. Disk ≥ 20G free (`df -h /`); KVM available (`ls /dev/kvm`).
3. SSH access to GitHub for B6 (`ssh -T git@github.com`); git identity configured for commits.
4. Age key present (`ls ~/.config/sops/age/`) for B8 full doctor OK.
5. `dev-home` = `~/Development/agent-workbench/workestrate-dev-home`.

---

### PRE — baseline [ANY-SHELL]

```bash
df -h /                                   # need >= 20G free (host-check threshold)
cd ~/Development/agent-workbench/workestrate
./scripts/host-check.sh                   # KVM, nix, flakes, mem>=4G, disk>=20G
```
- Expected: `[host-check] Host looks ready…` (exit 0). Checkpoint: any FAIL → stop and report.
- Optional GC only if disk tight: `nix-collect-garbage --delete-old && nix store optimise` **[ANY-SHELL]** (or `just gc` **[DEV-SHELL]**). Deliberate only — image tarballs are already GC'd.

### B5 — personal config relock [ANY-SHELL]

```bash
cd ~/Development/agent-workbench/workestrate-dev-home/config-repos/personal
nix flake lock --update-input workestrate      # the git+file:///home/rybski/... URL resolves HERE
git add flake.lock && git commit -m "build(flake): relock workestrate input to post-migration rev"
nix flake metadata | grep -A4 '"workestrate"'
```
- Also refresh the dev-home lock: `workestrate --home ~/Development/agent-workbench/workestrate-dev-home home init` **[PROFILE]** (rewrites workestrate.lock; idempotent), then commit `workestrate.lock`.
- Expected: workestrate input moves from `c45494b` to the post-migration HEAD. Checkpoint: URL resolution failure → stop and paste.

### B6 — PUSH (USER DECISION) [ANY-SHELL]

```bash
cd ~/Development/agent-workbench/workestrate
git log --oneline origin/migration/tool-model..HEAD   # expect 11 commits (migration + docs + build/feat + kvm-tests)
git push origin migration/tool-model                  # origin is SSH
```
- Why it matters: also unblocks the later `github:` input adoption (wrk-ayz deferred).

### Fork PR re-verify (USER DECISION) [ANY-SHELL]

```bash
cd ~/Development/agent-workbench/forks/microsandbox/repo
git ls-remote origin | grep -E "main|fix/filesystem"
```
- Local refs say origin/fix == 74919059, origin/main == b43d7522, but a prior live ls-remote showed caee6378 — re-verify before deciding open/close/leave the PR. Fork is read-only for agents.

### B7 — load-images (personal repo; USER DECISION on stale tag) [DEV-SHELL]

**MSB_HOME rule (verified):** the runtime store is `~/.microsandbox` — the `workestrate` wrapper (`agentctl.nix:122-125`) and `msb-wrapped` (`flake.nix:192-201`) BOTH force `MSB_HOME="$HOME/.microsandbox"`; the devshell's `MSB_HOME=~/.cache/ai-workbench-msb` export (`default.nix:117`) is only for offline cargo-check staging (its staged msb is NOT on PATH). Inside the devshell, `msb` on PATH = `msb-wrapped` → **`just load-images` lands images in `~/.microsandbox`, exactly where the runtime looks.** Do NOT run it with an unwrapped msb while `MSB_HOME` points at the cache path.

```bash
cd ~/Development/agent-workbench/workestrate-dev-home/config-repos/personal
nix develop ~/Development/agent-workbench/workestrate -c bash -c 'just load-images'
```
- This enters the tool devshell (provides `just` + `msb-wrapped`; nix stays from host PATH), cwd stays the personal repo so `nix build ".#workestrate-pi"/".#tempest"` and `msb load` resolve correctly. Expected: full rebuild → `workestrate-pi:latest` + `tempest:latest` in `~/.microsandbox`. Checkpoint: stale `workestrator-pi:latest` tag → report; decide prune (USER).

### B8 — host-provision [ANY-SHELL]

```bash
cd ~/Development/agent-workbench/workestrate
./scripts/host-provision.sh          # plain bash + nix; installs/syncs the profile binary when stale
# DEV-SHELL equivalent: nix develop -c bash -c 'just host-provision'
```
- Expected: host-check → binary sync (`nix profile install .#workestrate`) → `workestrate doctor` → readiness verdict. Needs the real age key for full doctor OK (USER). Contingency: STALE verdict persists → `--force` and re-run.

### Host boot batch [PROFILE] (+ [ANY-SHELL] curl)

```bash
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload up litellm
curl -sS http://host.microsandbox.internal:4000/health/liveliness    # [ANY-SHELL] expect 200
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload plan pi
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload exec pi
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload exec opencode
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload exec tempest
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload batch up
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload ps
```
- Runtime uses `~/.microsandbox` (wrapper-baked). Requires B7 (images loaded) + B8 (binary current).

### C — leftover gates (optional host re-run) [DEV-SHELL]

```bash
cd ~/Development/agent-workbench/workestrate
nix develop -c bash -c 'cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check && cargo clippy --manifest-path control/agentctl/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path control/agentctl/Cargo.toml'
# enter-once form:
nix develop
cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check
cargo clippy --manifest-path control/agentctl/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path control/agentctl/Cargo.toml
```
- Expected: fmt/clippy clean; 755 passed / 0 failed (already green in-container; host re-run confirms). Caveat: `just toolchain-check` needs flake.nix:28 marker uncommented (user decision) — the equivalent check (rustc 1.97 == fenix pin) passes.

### 3 ignored KVM tests — LAST [ANY-SHELL]

One command runs all three (no MSB_PATH/MSB_HOME/devshell knowledge required):

```bash
cd ~/Development/agent-workbench/workestrate
bash scripts/kvm-tests.sh          # or: just kvm-tests
```
- The script preflights (`/dev/kvm`, nix, raw msb store path, `python:3.12-slim`
  in the runtime store `~/.microsandbox`), then runs `lifecycle_detached`,
  `flake_root_gate`, and `ensure_images_e2e` serially — each in its own
  `nix develop -c` (devshell provides cargo + the vendor symlink) — and
  prints a PASS/FAIL summary (exit 0 only if all three pass). It does NOT
  auto-pull missing images: it prints the exact `pull` command and stops.
- This encapsulates the old three-command [DEV-SHELL] form, including the
  `MSB_PATH=<unwrapped msb>` requirement for `ensure_images_e2e`.

### E1 loopback experiment [ANY-SHELL] servers + [PROFILE] exec

```bash
python3 -m http.server 8081 --bind 127.0.0.1 &    # [ANY-SHELL]
python3 -m http.server 8082 --bind 127.0.0.2 &
python3 -m http.server 8083 --bind 127.0.0.3 &
python3 -m http.server 8084 --bind 0.0.0.0 &
ss -tlnp | grep 808
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload exec pi   # [PROFILE] interactive
#   getent hosts host.microsandbox.internal
#   curl -sS http://host.microsandbox.internal:8081/  # 127.0.0.1
#   curl -sS http://host.microsandbox.internal:8082/  # 127.0.0.2
#   curl -sS http://host.microsandbox.internal:8083/  # 127.0.0.3
#   curl -sS http://host.microsandbox.internal:8084/  # 0.0.0.0
# cleanup: kill the four python servers
```
- Record the outcome row in `06-improvements/12-per-instance-addressing.md` §open-decisions (feeds ADR 0026).

### Final teardown [PROFILE]

```bash
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload down-all --yes
workestrate --home ~/Development/agent-workbench/workestrate-dev-home workload ps   # expect empty
```

### Beads — deferred (user decision 2026-08-07); no bead work.
