# NEXT-SESSION — workestrate resumption context

> Narrative resumption notes. Task lists link to `wrk-*` beads IDs; this doc
> carries context, not work items (see BEADS.md for the boundary). Read the
> 2026-08-07 handover FIRST — it carries the full copy-paste host runbook.

## Microsandbox 0.6.8 fork migration LANDED (commits 84a901d…2ec1c6e + c413b8a)

The 0.5.6 pinned-trio implementation is gone. The stack is now source-built
from the user's microsandbox fork, pinned via a flake input at the validated
rev. User decision: adopt the idiomatic-Nix baseline and make the migration
work — pin `74919059` via flake input (flake=false), commit `c413b8a`.

Commits on `migration/tool-model` (2 ahead of origin: migration `c413b8a` + doc commit `5dcf3cf`):

| Commit | Change |
|--------|--------|
| `84a901d` | WIP 1 — microsandbox 0.6.8 source-build migration + in-flake `host-provision` |
| `4a8c0dd` | WIP 2 — pre-fill libkrunfw tar sha256 from GitHub v0.6.8 release checksum (verified real) |
| `d37db2e` | WIP 3 — fix `agentd.nix` rec keyword + expose agentd as flake package |
| `9cacda8` | WIP 4 — stage build/agentd in `microsandbox.nix` preBuild for the filesystem crate |
| `b48c8d3` | WIP 5 — `find` to locate msb binary in installPhase (host-triple target dir) |
| `2ec1c6e` | WIP 6 — vendor the entire fork workspace; patch ALL microsandbox-* crates |
| `c413b8a` | Pin microsandbox fork via flake input at validated rev 74919059 (flake=false; deletes `microsandbox-filesystem-agentd.patch`) |

What changed vs the 0.5.6 trio:

- msb + agentd are source-built from the fork workspace — `nix/packages/microsandbox.nix`
  rewritten (`rustPlatform.buildRustPackage`, fenix-pinned toolchain, `-p microsandbox-cli`
  with `--no-default-features --features net,ssh`); `nix/packages/agentd.nix` NEW
  (`pkgsStatic.rustPlatform` musl guest-init binary).
- `microsandbox-fork` flake input (github pin 74919059, flake=false) replaces
  `builtins.fetchGit` and the agentd patch file.
- Full fork workspace vendored under `control/agentctl/vendor/microsandbox-fork`
  (symlink recreated by the devshell shellHook); all 12 `microsandbox-*` crates
  patched via both `.cargo/config.toml` files.
- libkrunfw comes from the upstream v0.6.8 release tar (sha256 verified real;
  the fakeHash is gone).
- `control/agentctl/build.rs` version wiring (`WORKESTRATE_REV`) and
  `scripts/host-provision.sh` added.

## What remains — HOST ONLY (B1–B9 + C test gates)

Nothing of the 0.6.8 stack is built yet: `flake.lock` lacks the
`microsandbox-fork` input, `control/agentctl/Cargo.lock` still pins registry
0.5.6, the vendor symlink does not exist until `nix develop`, and the old
`workestrate-pi`/`tempest` image tarballs were GC'd. In-container there is no
nix/cargo/ssh/KVM, and the container `~/.microsandbox` msb needs GLIBC 2.38+
so it cannot run here either. All execution is on the host
(`/home/rybski/Development/agent-workbench`, dblab42).

Ordered host steps (copy-paste runbook: `handovers/2026-08-07-…md` §HOST RUNBOOK):

1. **B1 baseline** — `df -h /` + `./scripts/host-check.sh` (+ deliberate `just gc` only if disk tight).
2. **B2 lock** — `nix flake lock` in workestrate + commit (adds the `microsandbox-fork` input).
3. **B3 builds** — `nix build .#agentd` → `nix build .#microsandbox` → `nix build .#workestrate` (evidence: static agentd, `msb 0.6.8`, rev-embedded `--version`).
4. **B4 devshell + Cargo.lock** — `nix develop -c bash` (recreates vendor/microsandbox-fork) → `cargo check` → commit the Cargo.lock refresh (0.6.8 fork-sourced).
5. **B5 personal relock** — `nix flake lock --update-input workestrate` in the personal config repo + commit (currently pins pre-migration `c45494b`); refresh dev-home `workestrate.lock` (pins `b1c87416`).
6. **B6 push (USER DECISION)** — `git push origin migration/tool-model` (origin is SSH; also unblocks wrk-ayz github: input adoption).
7. **B7 load-images** — `just load-images` in the personal repo (full rebuild; old tarballs GC'd) — expect `workestrate-pi:latest` + `tempest:latest` in the msb store.
8. **B8 host-provision** — `just host-provision` (needs the real age key for full doctor OK).
9. **B9 sources** — populate `workestrate-dev-home/sources/<odysseus|opencode|tempest>/repo` (optional for core tests).
10. **C tests** — dependency order → KVM tests LAST → E1 → bead close-out.

## Corrected pending list (beads / user items)

- **wrk-23b refile PENDING** — the ensure-images fail-closed decision was
  cited in spec 21 §15 / STATUS but never filed: ABSENT from
  `.beads/issues.jsonl` (30 issues). `bd` is not installed in-container;
  refile on the host (`nix shell nixpkgs#beads`, procedure in BEADS.md).
  Exact commands in the handover. Do not hand-edit `issues.jsonl`.
- **wrk-vic (fork verify/push/PR)** — expected head `27d84216` is stale;
  actual local head AND `origin/fix/filesystem-agentd-path-override` =
  `74919059` (pushed). `origin/main` is `b43d7522` (#1) — a DIFFERENT line
  (diverged from the local branch; only a cosmetic `crates/filesystem/build.rs`
  rerun-if-env-changed reorder differs). Re-verify remote state with a LIVE
  `git ls-remote` before any fork work (a prior live ls-remote showed
  `caee6378`; not present in local refs). PR state unknown — user question.
- **wrk-ayz (canonical `github:` config-flake input)** — partially executed:
  the github-input pattern is now used for `microsandbox-fork`, but the
  personal flake's `workestrate` input is still the host-path
  `file:///home/rybski/...`; adopt `github:` only after B6 push.
- **wrk-wv0 (0.6.8 pin strategy)** — executed by the migration (libkrunfw
  0.6.8 pin landed); annotate/close on the host.
- **wrk-8yg (first host beads sync / bd dolt push)** — never done; HOST +
  user approval (beads sync discipline in BEADS.md; dolt push NEVER autonomous).
- **Age key** — host-only (`~/.config/sops/age`); `.env.enc` unverifiable in-container.

---

## Open follow-ups

- **wrk-bvu (spec 21 epic) — revisit the KVM-test `MSB_HOME` convention for
  a more idiomatic approach.** The `common::short_msb_home()` `/tmp`-based
  helper + the `MSB_HOME`-vs-`HOME` decoupling in `lifecycle_detached.rs` /
  `ensure_images_e2e.rs` is a pragmatic workaround for the 108-byte unix
  socket limit; brainstorm a configurable msb run/socket dir or a canonical
  short-`MSB_HOME` test convention and refactor. See spec 21 §14.

## Historical note (superseded)

The 2026-08-03 pre-migration pass (host-boot fixes `b675db2`/`c6a6b47`, the
personal URL repoint, 597 lib tests, `workestrate-pi`+`tempest` nix-layered
image builds) is superseded by the migration. The image tarballs were GC'd;
`STATUS.md` keeps the snapshot for reference.
