# NEXT-SESSION — workestrate host operations

> Purpose: this file is the DEFINITIVE operations doc for running the
> workestrate test stack on the host (dblab42). It is self-contained — a
> host operator with no prior context can follow it top-to-bottom. The
> HOST RUNBOOK below is the primary content.

## What landed (2026-08-06/07)

- **Migration committed** on `migration/tool-model`: `84a901d`…`2ec1c6e`
  (WIP 1–6) + `c413b8a` (pin `microsandbox-fork` flake input at validated
  rev `74919059`, flake=false; user decision: idiomatic-Nix baseline and
  make the migration work). The 0.5.6 pinned trio is replaced by a
  source-built 0.6.8 stack from the fork (msb + agentd), vendored fork
  workspace, libkrunfw v0.6.8 tar (hash verified real), build.rs version
  wiring, in-flake `host-provision.sh`.
- **Container cleanups** (2026-08-07): removed `workestrate/result` (broken
  symlink → GC'd store path), stale `control/agentctl/vendor/microsandbox-filesystem-0.5.6`
  symlink (untracked/gitignored), and `workestrate/core` (1.7 GB ELF core dump).
- **Doc commits** — `4dfeecd` ("docs: refresh NEXT-SESSION/STATUS for
  committed migration") and the doc commit "docs: refresh status and embed
  host runbook; fix migration drift" (this file + `STATUS.md` +
  `SPEC.md`/`README.md` drift fixes); branch is **3 ahead of origin
  (migration `c413b8a` + the two doc commits), NOT pushed**.
- **Beads: DEFERRED (user decision 2026-08-07)** — wrk-23b refile,
  wrk-vic/wrk-ayz/wrk-wv0/wrk-8yg all parked; do not chase.

## Current repo state (verified 2026-08-07)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | HEAD `migration/tool-model` (migration `c413b8a` + 2 doc commits) | clean, 3 ahead of origin, NOT pushed; origin remote is SSH |
| personal config repo | `e3d65e3` | clean; flake.lock pins workestrate @ `c45494b` (pre-migration) |
| dev home | workestrate-dev-home | clean; `sources/` EMPTY; workestrate.lock pins personal @ `b1c87416` |
| microsandbox fork | `74919059` `fix/filesystem-agentd-path-override` | local clean; origin/fix == `74919059` (pushed); origin/main = `b43d7522` (divergent); remote state AMBIGUOUS — live `ls-remote` showed `caee6378` not in local refs; **re-verify before fork work** |

**Store (shared):** NO 0.6.8 builds yet — no fork-source fetch, no 0.6.8 msb
output, no agentd output (drv only); pre-migration image tarballs
(`workestrate-pi` `vj844190…`, `tempest` `ajgqk1…`) are GC'd; only
`workestrate-0.1.0` output + 0.5.6 crate drvs remain.

## Readiness verdict

**NOT test-ready yet**: `flake.lock` lacks the `microsandbox-fork` input,
`control/agentctl/Cargo.lock` still pins registry 0.5.6, the
`vendor/microsandbox-fork` symlink does not exist until `nix develop`, and
no 0.6.8 build exists. The container cannot build/run anything (no
nix/cargo/ssh/KVM; container msb needs GLIBC 2.38+). Everything below is
host-side.

---

# HOST RUNBOOK — clean setup before tests (execute on dblab42, top-to-bottom)

This runbook is self-contained. Work from `/home/rybski/Development/agent-workbench`;
`cd` into each repo where a step says so. The container has already done all
container-side prep (cleanups + docs). The microsandbox fork is untouched
and read-only. Convention per step: exact command → expected output →
checkpoint (stop-and-report condition) → contingency → evidence to paste
back to the fleet.

## PRE — baseline (before anything else)

```bash
df -h /                                   # need >= 20G free (host-check threshold)
cd ~/Development/agent-workbench/workestrate
./scripts/host-check.sh                   # KVM, nix, flakes, mem>=4G, disk>=20G
```
- **Expected:** `[host-check] Host looks ready…` (exit 0).
- **Checkpoint:** any FAIL (missing KVM/nix/flakes) → stop and report; do not proceed.
- **Contingency / optional, ONLY if disk is tight:** `just gc`
  (nix-collect-garbage + store optimise). WARNING: the store previously hit a
  1.5 GB hard stop in-container and the old image tarballs are already GC'd —
  GC deliberately, not by default.
- **Evidence:** host-check output + `df -h /`.

## B2 — lock the flake (adds microsandbox-fork input)

```bash
cd ~/Development/agent-workbench/workestrate
nix flake lock                          # adds microsandbox-fork (github pin 74919059, flake=false)
git add flake.lock && git commit -m "build(nix): relock flake for microsandbox-fork input"
nix flake metadata | grep -A3 microsandbox-fork
ls -d /nix/store/*microsandbox-fork*    # fetched fork source now in store
```
- **Expected:** `flake.lock` diff adds the `microsandbox-fork` node; the
  fetch lands a `…-source` path in `/nix/store`.
- **Checkpoint:** if the fetch fails (network/hash), stop and paste the error.
- **Evidence:** `git show --stat HEAD` + the store path.

## B3 — build chain, in order, with evidence

```bash
cd ~/Development/agent-workbench/workestrate
nix build .#agentd --print-out-paths
```
- **Expected:** outPath; `libexec/agentd` exists.
- **Static check (must be static musl):**
```bash
file $(nix path-info .#agentd)/libexec/agentd
readelf -d $(nix path-info .#agentd)/libexec/agentd | grep NEEDED || echo "no NEEDED (static)"
```
- **Expected:** `ELF … statically linked` and **no NEEDED lines**.
- **Checkpoint:** edition-2024/rustc errors → stop and paste.
- **Evidence:** `file`/`readelf` output.

```bash
nix build .#microsandbox --print-out-paths
$(nix path-info .#microsandbox)/bin/msb --version
```
- **Expected:** `msb 0.6.8` (or the fork-derived version string).
- **Contingency:** if the libkrunfw fail-closed check trips (tar fetch or
  hash mismatch), capture the EXACT error; the fix is host-side — point the
  fetch at the standalone `libkrunfw-linux-x86_64.so` release asset path.
  Stop and report rather than hacking around it.
- **Checkpoint:** 0.6.8 msb must print its version; else stop + paste.

```bash
nix build .#workestrate --print-out-paths
$(nix path-info .#workestrate)/bin/workestrate --version
```
- **Expected:** version string with the embedded rev (build.rs wiring,
  e.g. `0.1.0-<rev>`; plain dev builds show `0.1.0-dev`).
- **Evidence:** all three outPaths + version outputs + the readelf/file result.

## B4 — devshell: recreate vendor symlink + refresh Cargo.lock

```bash
cd ~/Development/agent-workbench/workestrate
nix develop -c bash
ls -la control/agentctl/vendor/          # expect microsandbox-fork -> /nix/store/…
cargo check --manifest-path control/agentctl/Cargo.toml
git diff --stat control/agentctl/Cargo.lock
```
- **Expected:** vendor symlink recreated by the shellHook; `cargo check`
  succeeds; Cargo.lock diff shows the 0.5.6 → 0.6.8 re-resolve off the
  patched fork crates.
- **Contingency (likely-ish):** if SDK 0.6.8 API breaks surface, DO NOT fix
  ad hoc — paste the compiler errors back to the fleet for fix planning.
- **Commit the refresh (exit the shell first):**
```bash
exit
git add control/agentctl/Cargo.lock && git commit -m "build(agentctl): relock Cargo.lock to microsandbox 0.6.8 fork crates"
```
- **Evidence:** `git show --stat HEAD`, Cargo.lock before/after summary.

## B5 — personal config relock

```bash
cd ~/Development/agent-workbench/workestrate-dev-home/config-repos/personal
nix flake lock --update-input workestrate
git add flake.lock && git commit -m "build(flake): relock workestrate input to post-migration rev"
nix flake metadata | grep -A4 '"workestrate"'
```
- **Expected:** the `workestrate` input rev moves from `c45494b` to the
  current workestrate HEAD (post-migration); narHash updates.
- **Also refresh the dev-home lock** (currently pins `b1c87416`):
```bash
cd ~/Development/agent-workbench/workestrate-dev-home
workestrate --home ~/Development/agent-workbench/workestrate-dev-home home init   # idempotent; rewrites workestrate.lock (Step 8)
git diff -- workestrate.lock                                                     # confirm it now pins the new personal HEAD
git add workestrate.lock && git commit -m "home: relock personal config to post-migration rev"
```
- **Evidence:** both lock diffs.
- **Checkpoint:** if the host-path URL fails to resolve, stop and paste (the
  canonical `github:` input is a deferred decision, not this step).

## B6 — PUSH (USER DECISION)

```bash
cd ~/Development/agent-workbench/workestrate
git log --oneline origin/migration/tool-model..HEAD   # expect 3 commits: migration c413b8a + 2 doc commits
git push origin migration/tool-model                  # origin is SSH
```
- **USER DECISION:** pushing is explicit and only with your approval.
- **Why it matters:** makes the committed migration available upstream and
  unblocks the later `github:` input adoption for the personal flake.
- **Evidence:** push output + ref/PR confirmation.

### Fork PR re-verify (USER DECISION — before any fork work)

Local refs say `origin/fix/filesystem-agentd-path-override` == `74919059`
and `origin/main` == `b43d7522`, but a prior live `ls-remote` showed
`caee6378` (not present in local refs). Re-verify the live remote before
deciding anything about the fork:

```bash
cd ~/Development/agent-workbench/forks/microsandbox/repo
git ls-remote origin | grep -E "main|fix/filesystem"
```
- **USER DECISION:** whether to open/close/leave the fork PR
  (`b43d7522` "#1" is a different line — diverged from the local branch;
  only a cosmetic `crates/filesystem/build.rs` rerun-if-env-changed reorder
  differs). The fork is read-only for agents; only you act on it.
- **Evidence:** `git ls-remote` output + your decision.

## B7 — load-images (personal repo)

```bash
cd ~/Development/agent-workbench/workestrate-dev-home/config-repos/personal
just load-images
msb image ls
```
- **Expected:** full rebuild (old tarballs GC'd) then
  `workestrate-pi:latest` + `tempest:latest` loaded into the msb store.
- **USER DECISION / flag:** a stale `workestrator-pi:latest` tag may still
  exist from earlier sessions — report it; decide whether to prune.
- **Evidence:** `msb image ls` output.
- **Checkpoint:** eval errors from the stale flake.lock would mean B2/B5
  didn't complete — do not skip them.

## B8 — host-provision (binary sync + doctor)

```bash
cd ~/Development/agent-workbench/workestrate
just host-provision            # or ./scripts/host-provision.sh
```
- **Expected:** host-check → binary sync (installs `.#workestrate` into the
  nix profile when stale — the original boot-failure root cause) → doctor →
  readiness verdict.
- **Contingency:** if the STALE-binary verdict persists, follow the script's
  auto-reinstall path (`--force` if needed) and re-run.
- **USER DECISION (age key):** full doctor OK for secret checks needs the
  real age key (`~/.config/sops/age/…`) — provide it if missing.
- **Evidence:** readiness table + verdict (exit 0).

## B9 — sources (optional for core tests, required for odysseus/opencode/tempest)

```bash
cd ~/Development/agent-workbench/workestrate-dev-home
mkdir -p sources/{odysseus,opencode,tempest}/repo
git clone https://github.com/georgrybski/odysseus sources/odysseus/repo
git clone https://github.com/georgrybski/opencode sources/opencode/repo
git clone https://github.com/georgrybski/T3MP3ST sources/tempest/repo
# pin to the personal flake.lock revs (odysseus fc8e6366 / opencode cfddb240 / tempest ae32cf50):
git -C sources/odysseus/repo checkout fc8e6366
git -C sources/opencode/repo checkout cfddb240
git -C sources/tempest/repo checkout ae32cf50
# then run the source build for each (pip / bun / npm per README "Source builds")
```
- **Expected:** `ls sources/*/repo` populated at the locked revs.
- **Evidence:** populated dirs + build results.

## C — test gates in dependency order

```bash
cd ~/Development/agent-workbench/workestrate
just toolchain-check      # rustc matches fenix pin
just check                # fmt + clippy(-D warnings) + cargo check
just test                 # ~611 src #[test] + integration suite
```
- **Expected:** all green. Then the **1 DB-pool ignored test ALONE** (not
  with the KVM trio):
```bash
cargo test --manifest-path control/agentctl/Cargo.toml down_all_instances_returns_notfound_when_msb_db_empty_but_openable -- --ignored --nocapture
```
- **Full validation:**
```bash
just verify               # toolchain-check + check + test + spec-examples + tombi-check + golden-check + schema-check + scaffold-check + lint-nix + store-audit + Cargo.lock-diff gate
just verify-full          # verify + nix build .#workestrate
```
- **Checkpoint:** any red in `just check`/`just test` → stop and paste
  (especially Cargo.lock re-diffing after tests).
- **Runtime gates on the real home:**
```bash
workestrate doctor
workestrate check
workestrate --home ~/Development/agent-workbench/workestrate-dev-home validate-config
```
- **Host runtime batch (B7/B8/B9 must be done):**
  `workestrate workload up litellm` → health → `plan`/`exec` pi, opencode,
  tempest → `batch up` → `ps` → `down-all`.
- **Evidence:** per-gate output; any failures verbatim.

### The 3 ignored KVM tests — LAST

```bash
cd ~/Development/agent-workbench/workestrate
cargo test --manifest-path control/agentctl/Cargo.toml --test lifecycle_detached -- --ignored --nocapture
cargo test --manifest-path control/agentctl/Cargo.toml --test flake_root_gate -- --ignored --nocapture
MSB_PATH=$(nix path-info .#microsandbox)/bin/msb cargo test --manifest-path control/agentctl/Cargo.toml --test ensure_images_e2e -- --ignored --nocapture
```
- **Expected:** all 3 pass with a loaded `python:3.12-slim` image + KVM.
- **Evidence:** per-test pass + MSB_HOME isolation respected.

### E1 loopback experiment (docs/validation-and-improvements/05-host-validation.md:282-321)

```bash
python3 -m http.server 8081 --bind 127.0.0.1 &
python3 -m http.server 8082 --bind 127.0.0.2 &
python3 -m http.server 8083 --bind 127.0.0.3 &
python3 -m http.server 8084 --bind 0.0.0.0 &
ss -tlnp | grep 808
# inside the pi sandbox (workestrate workload exec pi, interactive):
#   getent hosts host.microsandbox.internal
#   curl -sS http://host.microsandbox.internal:8081/   # 127.0.0.1
#   curl -sS http://host.microsandbox.internal:8082/   # 127.0.0.2
#   curl -sS http://host.microsandbox.internal:8083/   # 127.0.0.3
#   curl -sS http://host.microsandbox.internal:8084/   # 0.0.0.0
# cleanup: kill the four python servers
```
- Record the outcome row in `06-improvements/12-per-instance-addressing.md`
  §open-decisions (feeds ADR 0026). **Evidence:** which binds reachable.

## USER-DECISION POINTS (summary)

| Point | Decision needed |
|-------|-----------------|
| B6 | Push `migration/tool-model` (SSH origin)? |
| Fork PR | Re-verify with live `git ls-remote`; open/close/leave the fork PR? |
| Age key | Provide host age key so B8 doctor is fully OK |
| B7 | Prune stale `workestrator-pi:latest` tag if present? |
