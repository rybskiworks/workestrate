# STATUS — pre-KVM build/deploy pipeline state

> Snapshot produced by the in-container pre-KVM build pass (2026-08-03).
> The nix store is shared with the host, so built images are available to
> the host without rebuild. Host-side steps (msb load + boot sequence) are
> in NEXT-SESSION.md / the host runbook.

## Host-boot fix pass (2026-08-03) — fixes landed; host steps remain

Two host-boot failures fixed in-container; the host relock + load-images + boot
sequence remains.

- **Failure 1 (mount doubling) — FIXED** (commit `b675db2`): directory-mode
  content root now resolves at the `<repo>/workestrate/` root (not the capsule
  dir), so a `host = "workloads/litellm"` mount no longer doubles to
  `workestrate/workloads/litellm/workloads/litellm`. The phase-1 spec-17
  amendment (`08a75d2`) that codified the wrong capsule-relative root is
  superseded-with-correction in spec 17.

- **Failure 2 (personal flake URL) — FIXED** (personal config repo commit):
  `workestrate.url` repointed from the container-only
  `git+file:///home/node/...` to the host path `git+file:///home/rybski/...`.
  In-container relock is intentionally NOT done (the URL does not resolve
  here); the user relocks on the host. Canonical fix (pinned `github:` input)
  is a Phase-5 decision (beads `wrk-ayz`).

- **Plan-time existence preflight — ADDED** (commit `c6a6b47`): `plan` now
  fails fast on a missing read-only mount source / seed source (the failure-1
  signal) before any KVM work; `validate-config` runs the same check warn-only.
  Fulfills the security-model §enforcement-points plan-time promise. New tests:
  597 lib tests pass (clippy + fmt clean).

- **Beads filed:** `wrk-ayz` (canonical config-flake input URL, Phase-5
  decision); `wrk-23b` (ensure-images eval-error fail-closed vs nix-absent
  trust posture — decision). Both noted in spec 21 §15.

### What remains host-side (ordered)
1. `nix flake lock --update-input workestrate` (in the personal config repo) —
   relock against the repointed host-path URL.
2. `just load-images` (personal repo) — load the built `workestrate-pi` +
   `tempest` tarballs into the msb store.
3. Boot sequence: `workestrate workload up litellm` → health →
   `workestrate workload plan pi` / `plan tempest` (the new preflight should
   now pass — mount sources resolve) → `exec` the agents → `batch up` → `ps`
   → `down-all` → the 3 ignored KVM tests → E1.


## Commits made this pass

| Repo | Commit | Message |
|------|--------|---------|
| personal | 50a5fef | build(tempest): real npmDepsHash |
| tool     | 5532eb7 | fix(recipes): add musl to npm-build buildInputs for musl native node deps |
| personal | f5f2013 | build: relock tool input for npm-build musl fix |

## Tempest npmDepsHash

- Real hash computed via `nix run nixpkgs#prefetch-npm-deps` on tempest's
  package-lock.json: `sha256-eAueD4q+ibdZ+7E3RhmEnRkGJ62p/4ZRR/uMaStr4qE=`
- Inlined into personal `flake.nix` (`tempestNpmDepsHash`), replacing the
  `lib.fakeHash` placeholder. (pi's `piNpmDepsHash` was already real.)
- NOTE: `just update-hashes` (prefetch-npm-deps) for pi fails on an optional
  platform-specific dep (@biomejs/cli-darwin-arm64) — irrelevant for linux-x64;
  pi's existing hash is correct and the pi build succeeds with it.

## Recipe fix: musl for auto-patchelf (tool 5532eb7)

- Symptom: `nix build .#tempest` / `.#workestrate-pi` failed at auto-patchelf:
  `could not satisfy dependency libc.musl-x86_64.so.1` (musl-linked native
  node deps: lightningcss-linux-x64-musl, @rolldown/binding-linux-x64-musl,
  @biomejs/cli-linux-x64-musl, esbuild).
- Root cause: `nix/lib/recipes/npm-build.nix` `buildInputs` lacked musl, so
  autoPatchelfHook could not find `libc.musl-x86_64.so.1` / `ld-musl-x86_64.so.1`.
- Fix: `buildInputs = [ stdenv.cc.cc.lib libcap_ng pkgs.musl ];` (one line).
- Personal `flake.lock` relocked to tool rev 5532eb7 (revCount 291). The
  npm-deps FOD drv hash is independent of the tool rev, so already-fetched
  deps stay reusable across the relock.

## Image builds (real, in shared nix store)

| Image | outPath | size | tag | tarball sanity |
|-------|---------|------|-----|----------------|
| workestrate-pi | /nix/store/vj844190hg0shr4r42d8gyrbysgr5v1c-workestrate-pi.tar.gz | 58M | workestrate-pi:latest | /app/bin/pi + asset mirror (CHANGELOG, README, assets/, docs/, examples/, export-html/, photon wasm) verified in layer |
| tempest | /nix/store/ajgqk1winvx5mb64vhxq3571jazp36c1-tempest.tar.gz | 141M | tempest:latest | valid docker archive (manifest.json + layers); command ["node","dist/cli.js"]; nodejs_24 in contents |

- pi command: `["/app/bin/pi"]`; tempest command: `["node", "dist/cli.js"]`.
- Both built with the musl fix; auto-patchelf passed.
- NOT loaded into the msb store (`msb load` is forbidden in-container — the
  msb store is the user's). `workload build --check` reports STORE=gone for
  both (msb store empty); the host's `just load-images` populates it.

## Source builds (local_build) — HOST-only

- odysseus (pip-install), opencode (bun-install), tempest (npm-build) all
  have `local_build` recipes with `flake://` sources.
- `workestrate source clone <name>` is a guidance no-op for flake:// sources
  (sources are nix flake inputs, already materialized in the store); the
  host must populate `sources/<name>/repo` (copy from the flake input or
  `git clone`) then `source build`.
- odysseus & tempest source repos lack a `flake.nix`, so the CLI's
  `nix shell .` build path cannot provide the toolchain in-container.
- opencode's source repo has a flake.nix, but the in-container disk ceiling
  (/nix at ~99%, ~1.7GB free, 1.5GB hard stop) prevented running the build
  (bun install + nix flake input fetch would breach the stop).
- All three source builds are deferred to the host. Build commands:
  - odysseus: `python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./.deps -r requirements.txt` (in sources/odysseus/build)
  - opencode: `HUSKY=0 bun install && bun run build` (in sources/opencode/build)
  - tempest: `npm install && npm run build` (in sources/tempest/build)

## End-state validation (read-only, WORKESTRATE_HOME=<dev-home>)

- `validate-config`: workestrate.toml is valid.
- `workloads`: litellm (service), odysseus (service), opencode (agent),
  pi (agent, nix-layered:workestrate-pi:latest), tempest (agent,
  nix-layered:tempest:latest).
- `workload plan litellm`: renders fully (image, command, env, secrets,
  port 4000, mounts, network ingress/egress).
- `workload plan pi` / `plan tempest`: refuse — "dependency 'litellm' is
  required but not running". BY DESIGN (dependents refuse while litellm is
  down); they render once litellm is up on the host. (expected)
- `workload build --check` (spec 21 §3.4 staleness matrix):
  pi       — RECORD=absent, STORE=gone, DECISION=would build
  tempest  — RECORD=absent, STORE=gone, DECISION=would build
  (STORE=gone = msb image store; nix-store tarballs ARE built.)

## What remains host-side

1. `just load-images` (or `msb load` per image with the store paths above) —
   loads the built tarballs into the msb store.
2. Boot sequence: up litellm → health → plan/exec pi, opencode, tempest →
   batch up → ps → down-all → KVM tests → E1. See the host runbook
   (delivered in the session summary / NEXT-SESSION.md).

## Disk note

- /nix started at ~4.1GB free, ended at ~1.7GB free (never breached the
  1.5GB hard stop). No `nix-collect-garbage` run (shared store — would
  delete host artifacts). The musl fix + image builds consumed ~2.4GB.
