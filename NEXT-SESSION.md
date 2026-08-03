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
