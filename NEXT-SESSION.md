# NEXT-SESSION — workestrate resumption context

> Narrative resumption notes. Task lists link to `wrk-*` beads IDs; this doc
> carries context, not work items (see BEADS.md for the boundary).

## Host-boot fix pass landed (2026-08-03) — host relock + boot remains

The two host-boot failures are fixed in-container; the host steps remain.

- Failure 1 (mount doubling) fixed — `b675db2`: directory-mode content root =
  `<repo>/workestrate/` (not the capsule dir). Spec 17 amendment `08a75d2`
  superseded-with-correction.
- Failure 2 (personal flake URL) fixed — personal config repo: `workestrate.url`
  repointed to `git+file:///home/rybski/...` (host path). Relock on host required.
- Plan-time existence preflight added — `c6a6b47`: `plan` fails fast on missing
  RO mount / seed source; `validate-config` warns. 597 lib tests pass.
- Beads: `wrk-ayz` (canonical config-flake input URL, Phase-5), `wrk-23b`
  (ensure-images fail-closed decision). Noted in spec 21 §15.

Host next steps (ordered):
1. `nix flake lock --update-input workestrate` (personal config repo).
2. `just load-images` (personal repo).
3. `workestrate workload up litellm` → health → `plan`/`exec` pi, opencode,
   tempest → batch up → ps → down-all → the 3 ignored KVM tests → E1.

Push-state check (host) for the Phase-5 `github:` input decision (wrk-ayz):
`git -C <workestrate> fetch && git log --oneline origin/migration/tool-model..HEAD`.

---

## Open follow-ups

- **wrk-bvu (spec 21 epic) — revisit the KVM-test `MSB_HOME` convention for
  a more idiomatic approach.** The `common::short_msb_home()` `/tmp`-based
  helper + the `MSB_HOME`-vs-`HOME` decoupling in `lifecycle_detached.rs` /
  `ensure_images_e2e.rs` is a pragmatic workaround for the 108-byte unix
  socket limit; brainstorm a configurable msb run/socket dir or a canonical
  short-`MSB_HOME` test convention and refactor. See spec 21 §14.

## Pre-KVM build pass (2026-08-03) — host runbook pending

The in-container pre-KVM build pass completed the nix-layered image builds
for `workestrate-pi` and `tempest` (real tarballs in the shared nix store),
inlined tempest's real `npmDepsHash`, and applied a one-line tool-recipe fix
(`nix/lib/recipes/npm-build.nix`: add `pkgs.musl` to `buildInputs` so
auto-patchelf can satisfy musl-linked native node deps). Source builds
(odysseus/opencode/tempest local_build) are deferred to the host (disk
ceiling + odysseus/tempest source repos lack a flake.nix). Full state,
outPaths, and the host runbook are in `STATUS.md`. Host next steps: load the
two built images into the msb store (`just load-images`), then run the boot
sequence (up litellm → health → plan/exec pi/opencode/tempest → batch up →
ps → down-all → the 3 ignored KVM tests → E1).


## host-provision (in-flake provisioner)

One idempotent command (`./scripts/host-provision.sh` or `just host-provision`)
that runs `scripts/host-check.sh`, syncs the nix-profile-installed `workestrate`
binary to the current tree (only mutation: `nix profile install .#workestrate`
when stale), runs `workestrate doctor`, and prints a readiness table + verdict
(exit 0 = ready, 1 = not). Flags: `--check-only` (report only), `--force`
(reinstall even when fresh). Uncommitted, pending review.

## microsandbox 0.6.8 fork migration (uncommitted)

Migrated the microsandbox stack from 0.5.6 to 0.6.8, built from source off the
user's fork branch `fix/filesystem-agentd-path-override` (local head rev
`74919059`, not yet pushed). msb + agentd are no longer fetched from the
upstream release tarball — they are built from the fork workspace:

- `nix/packages/agentd.nix` (NEW): `pkgsStatic.rustPlatform.buildRustPackage`
  building `-p microsandbox-agentd` as a static musl binary (guest init).
- `nix/packages/microsandbox.nix` (rewritten): `rustPlatform.buildRustPackage`
  (fenix-pinned toolchain) building `-p microsandbox-cli` with
  `--no-default-features --features net,ssh`; assembles `$out/bin/msb` (from
  cargo) + `$out/libexec/agentd` (from the agentd derivation) + libkrunfw
  (extracted from the upstream v0.6.8 release tarball, Branch A interim).
- `nix/packages/microsandbox-filesystem-patched.nix`: sources the filesystem
  crate from the same fork via `builtins.fetchGit` (MSB_AGENTD_PATH fix carried
  natively — no patch step).
- `flake.nix`: wires `agentd` + `rustToolchain` into `microsandbox`.
- Vendor symlink references renamed 0.5.6 → 0.6.8 across agentctl.nix preBuild,
  devshell default.nix, both `.cargo/config.toml` files, justfile, README.md.

The ONLY remaining `pkgs.lib.fakeHash` is the libkrunfw release-tar fetch in
microsandbox.nix (Branch A). If the v0.6.8 tar 404s, Branch B (build libkrunfw
from the fork's vendor/libkrunfw submodule @ c5503d82) becomes mandatory —
stubbed as a TODO in microsandbox.nix.

Pending host steps (ordered):
1. `nix build .#agentd` — validates the pkgsStatic musl build + edition 2024.
2. `nix build .#microsandbox` — paste the one libkrunfw tar sha256 (fakeHash).
   If the tar 404s, implement Branch B (submodule build).
3. `nix build .#workestrate` — full assembly (microsandbox + filesystem + agentctl).
4. `nix develop -c cargo check --manifest-path control/agentctl/Cargo.toml` —
   refresh Cargo.lock, surface any microsandbox 0.6.8 SDK API breaks.
5. `./scripts/host-provision.sh` (reinstall the synced binary) then re-run
   the smoke workload + litellm to confirm the host-boot fix landed.
6. Push the fork branch and swap `builtins.fetchGit` → `fetchFromGitHub` in
   agentd.nix, microsandbox.nix, microsandbox-filesystem-patched.nix.

NOTE: the vendor wiring is fully renamed to 0.6.8 across the source tree.
The remaining `0.5.6` references are historical docs (`docs/**`) and cargo's
`target/` build cache only.

Executes beads wrk-wv0 + the dependency half of wrk-vic.
