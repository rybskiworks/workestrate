# STATUS — post-migration + in-container execution snapshot (2026-08-07)

> Snapshot after: the 0.6.8 fork migration committed, and this session's
> in-container execution of B2–B4 + Change #1. The full operations doc
> is `NEXT-SESSION.md`.
>
> **2026-08-13 UPDATE (SUPERSEDED CLAIM):** the next line was removed because
> it was WRONG — "The nix store is shared with the host". Container and host
> nix STORES ARE SEPARATE (shared filesystem only); each side realizes its own
> store paths (handover `handovers/2026-08-11-prime-agent-workload.md` §5.5).
> Latest authoritative state: that handover (prime-agent workload, node-pivot,
> host-validated).

## Repo states (verified 2026-08-08)

| Repo | HEAD / branch | State |
|------|---------------|-------|
| workestrate | HEAD on `migration/tool-model` | clean, 11 ahead of origin/migration/tool-model (9 prior + fce23c9 schema-docs regen + this docs entry), 310 ahead of origin/main; NOT pushed; origin SSH |
| personal config repo | `3dcaf9e` | 7 commits above `c184b29` (0f5be2e, 7cd9e0e, 7613931, e779c83, b41cdaa, 6917f3d, 3dcaf9e); no remote; clean (user WIP resolved) |
| dev home | (workestrate-dev-home) `c3dd7d5` | clean, 6 ahead of origin/master (`/home/rybski/.workestrate`); `sources/` EMPTY; workestrate.lock pins `b1c87416` |
| microsandbox fork | `74919059` `fix/filesystem-agentd-path-override` | clean; origin/fix == 74919059 (pushed); origin/main `b43d7522` (divergent); remote ambiguous — live ls-remote before fork work |

## Store / artifact state (verified 2026-08-07)

- `microsandbox-fork` input LOCKED (c162c7a): fork source
  `/nix/store/y9rc99j2n2gcr3wvgi4n36kqbhd396v5-source`.
- `.#agentd` → `…97ngwbmkq…-microsandbox-agentd-static-x86_64-unknown-linux-musl-0.6.8`
  (fully static, no .dynamic).
- `.#microsandbox` → `/nix/store/k3v2l4wa0ya94gi3k92fbijbd2pvlqgp-microsandbox-0.6.8`
  (`msb 0.6.8`; `DT_RUNPATH` embedded at link time via RUSTFLAGS — runs with no `LD_LIBRARY_PATH`, no patchelf).
- `.#workestrate` → `/nix/store/b2zvpp1iw2kp3bd14v0x3cmkg846rj16-workestrate-0.1.0`
  (rebuilt against the link-time-RPATH msb; `0.1.0-<rev>` once committed).
- Cargo.lock refreshed (165ef88): microsandbox 0.6.8 fork crates, sea-orm 2.0.1;
  no registry 0.5.6 pins remain.
- `control/agentctl/vendor/microsandbox-fork` symlink present (→ fork source);
  old 0.5.6 symlink removed.
- NO image tarballs in the store (`workestrate-pi`/`tempest` GC'd earlier);
  container msb store empty; nothing loaded.

## What landed this session

- seed_files template/glob feature P0-P3 landed (see NEXT-SESSION.md summary): templated seed rendering from the guest-visible env view ($MSB_<binding key> placeholders; no process env) + glob-capable seeds with plan preflight/validate-config warnings; personal config pi seeds rendered on next up (0f5be2e).

## namespaced ports + auto-allocation (P0-P4) — 2026-08-10

- Schema: `[[ports]]` entries accept `name` (slug `^[a-z0-9][a-z0-9-]*$`) and
  `host = 0` (auto-allocation); `depends_on.<dep>` accepts `env` (primary) OR
  `exports = { <port-name> = "<ENV_VAR>" }` (at least one required; exports
  keys must name a declared port on the dependency).
- `host = 0` probes a free port on the slot's bind at boot (per-port
  `--port-auto`); recorded in the instance record; `ps` shows the effective
  port. A not-running dep with an auto port refuses at plan time.
- Namespaced resolution + provenance: plan Display renders
  `port: <name>:<host>:<guest>`, `(auto)`, and
  `(injected: depends_on '<dep>' port '<port>')`; `ps` renders named ports.
- Reference example (config.reference): example-litellm gains named `api`
  (4000) + auto `metrics` (0→9090) ports; new exports-only `example-exports`
  workload; goldens for example-service/example-agent/example-offensive
  byte-identical.
- Commits: 217f7a3 (P0), 262802e (P1), 1f2336f (P2), 3e7c2e0 (P3), this
  docs commit (P4).

## plain-HTTP secret substitution fix (require_tls_identity) — 2026-08-10

- Root cause: the fork's host egress proxy substitutes secret placeholders
  (`$MSB_LITELLM_MASTER_KEY` → real value) ONLY when the secret's
  `require_tls_identity` is false on plain-HTTP connections (or under TLS
  interception). The fork's SecretBuilder defaults `require_tls_identity:
  true` and the SDK's 3-arg `secret_env` leaves it true, so workestrate's
  host-bound secrets were never substituted over the plain-HTTP pi →
  `host.microsandbox.internal:4000` path — litellm received the literal
  placeholder → 400 "No connected db".
- Fix (33afae4): `apply_plan_secrets` now builds each host-bound SecretEntry
  via the full builder — `require_tls_identity(false)` for the local proxy
  alias `host.microsandbox.internal`, `true` for external hosts
  (`secret_requires_tls_identity` helper; `$MSB_<name>` placeholder
  auto-naming unchanged). 3 new tests; full suite green (849 passed / 0
  failed / 4 ignored).

## host-provision false "NOT READY" fixed (binary path capture) — 2026-08-10

- Root cause: `scripts/host-provision.sh` Step B captured the fresh store
  path with `want=$(nix build .#workestrate --no-link --print-out-paths
  2>&1)`; when a build is actually needed, nix's stderr progress lines
  ("these N derivations will be built:", "building '...'") merged into
  `want`, so the fresh comparison failed even after a successful
  `nix profile install` → false "binary still mismatched after install"
  / "NOT READY" (observed on host at rev 4cad345).
- Fix: separate capture — `want=$(nix build ... 2>"$build_log")`, failure
  reports `$(cat "$build_log")`, `$build_log` removed after; behavior
  otherwise unchanged. Validated: `bash -n` + in-container capture proof
  (single clean store path; simulated uncached build shows old `2>&1`
  noise vs new clean capture).

## config-model docs + schema understandability — 2026-08-10

- fce23c9: schema doc-comment improvements + regen — `host = 0`
  auto-allocation, named-port identity/unnamed-primary, depends_on
  env/exports, and seed template/glob/`$$` escape now appear as concise
  descriptions in the generated schema; copier template copies synced.
- This docs commit: spec 20 annotated example demonstrates the new fields
  (named + auto ports, depends_on env/exports, template/glob seeds); README
  gains a "Ports, dependencies, and seed files" subsection; SPEC.md drops
  the removed `--port-offset` form.
- dev-home (`c3dd7d5`) and personal (`3dcaf9e`) consumer schema copies are
  one regen behind (pre-description schemas); next `workestrate schemas
  update` syncs them.

## Host build failure fixed (2026-08-07 evening)

- Host `nix build .#workestrate` failed — drv `94lv9jaln8siy729pwv1xqxk30cpr41l`, exit 101. Root cause: the fork SDK `sdk/rust/build.rs` took its runtime-deps DOWNLOAD branch because the staged `msb` version probe (`installed_msb_version`, build.rs:118-133) could not exec — msb links libcap-ng dynamically without an RPATH. Hermetic host sandbox blocks the download → exit 101. In-container the same branch succeeded earlier (container sandbox permits network) — that impurity masked the bug.
- Fix: link-time RPATH via flake-declared `RUSTFLAGS` — `nix/packages/microsandbox.nix` sets `RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath [ pkgs.libcap_ng pkgs.stdenv.cc.cc.lib ]}"` so rustc embeds the libcap-ng + libgcc search path into msb's `DT_RUNPATH` at link time; NO patchelf (the intermediate `autoPatchelfHook` approach, commit 11ec06c, was replaced; the `LD_LIBRARY_PATH` export, commit 2593604, was already reverted). Zero `LD_LIBRARY_PATH` and zero patchelf in the derivations. `MSB_HOME` + `MSB_AGENTD_PATH` staging unchanged; no feature changes.
- Validated in-container with `--option sandbox true` (sandbox confirmed engaged): `.#microsandbox` → `/nix/store/k3v2l4wa0ya94gi3k92fbijbd2pvlqgp-microsandbox-0.6.8`; `readelf -d …/bin/msb` shows `RUNPATH` with libcap-ng + libgcc store paths (embedded by the linker); `env -u LD_LIBRARY_PATH …/bin/msb --version` → `msb 0.6.8` (was `error while loading shared libraries: libcap-ng.so.0`); `.#workestrate` drv `p50dh1mkr9d2ypgi7qffccfryjdvqvh6` → `/nix/store/b2zvpp1iw2kp3bd14v0x3cmkg846rj16-workestrate-0.1.0`, 0 "download" lines in `nix log`; `.#agentd` no-op (cached `97ngwbmkqn3ig3mrma6y7y4kk7g5fgci`); `workestrate --version` OK; `scripts/check-nix-paths.sh` clean.
- Repo state: workestrate now **9 ahead of origin, NOT pushed** (8 commits before this entry: seed_files feature f475565 + c374be3 + 1a02d8e + f468519 + 238cd7d + e9f66b1, + 273b1b5 docs record, + 650bcbe `$$` escape — plus this docs commit).

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
