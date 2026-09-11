---
name: validation-nix-supply-chain
description: |
  Verifies flake inputs are pinned, no mutable refs exist, and no stale
  lockfile entries. Load after flake.lock changes or before release. Does
  NOT cover eval validation (see validation-nix-eval) or check builds (see
  validation-nix-test).
metadata:
  org.kind: validation
---

# Validation: Nix Supply Chain

This gate verifies that all flake inputs are pinned with exact revisions
and content hashes, no mutable refs leak into the supply chain, and no
stale lockfile entries remain. It runs after `flake.lock` changes and
before release.

## Triggers

Load this skill when:

- After `flake.lock` changes (adding/updating inputs via `nix flake
  update`).
- After adding a new flake input to `flake.nix`.
- After changing a `fetchFromGitHub`/`fetchurl`/FOD `outputHash`.
- Before release, as a supply chain audit.
- As a periodic CI gate (e.g., nightly/weekly).

## Command

```bash
nix flake metadata
nix flake metadata --json
just store-audit
git diff flake.lock
```

`nix flake metadata` lists every input with rev/narHash/lastModified;
`just store-audit` runs `nix path-info --all --json | python3
scripts/store-audit.py --warn-if-source-over 50`; inspect `flake.lock`
directly for `follows` and `narHash` presence.

## Pass criteria

- All inputs have a `locked.rev` (commit SHA) and `locked.narHash` in
  `flake.lock`.
- No input uses a mutable ref without a pin (e.g., no `github:owner/repo`
  without a branch/rev — though the lock file pins the exact rev).
- No stale lockfile entries (inputs removed from `flake.nix` but still in
  `flake.lock`).
- `just store-audit` passes (no `ai-workbench` `*-source` paths over
  50 MB).
- `follows` chains resolve correctly (e.g., `fenix` follows root
  `nixpkgs`).

## Fail criteria

- Any input missing `narHash` or `rev` in its `locked` entry.
- A `fetchTarball` without a hash, or `builtins.fetchGit` of a mutable
  ref.
- An unhashed fetch (no `outputHash`/`hash`/`sha256` on a FOD).
- A stale lockfile entry (input in `flake.lock` but not in `flake.nix`).
- `just store-audit` fails (oversized source copy — impure path leak).
- `lib.fakeSha256` / `lib.fakeHash` still present in a committed FOD.

## Evidence to report

- `nix flake metadata` output (input name, resolved rev, narHash,
  lastModified).
- `flake.lock` diff (changed `rev`/`narHash` entries).
- `just store-audit` output (top-20 store paths, pass/fail on source-path
  threshold).
- Any input missing `narHash` or with `lib.fakeSha256`.
- The `follows` chain for each flake input that uses it.

## Notes

- HOST-GATE: This container has no nix. Run `nix flake metadata` and
  `just store-audit` on a nix-capable host; the semantics are documented,
  not runtime-verified here.
- `flake.lock` structure: each node has `original` (declared ref),
  `locked` (pinned rev + narHash + lastModified), `inputs` (transitive),
  and `flake` (boolean).
- `follows` keeps transitive inputs aligned with the root `nixpkgs`,
  avoiding duplicate/stale nixpkgs copies. The project's `fenix` input
  uses `inputs.nixpkgs.follows = "nixpkgs"`.
- The project has 6 top-level inputs: `nixpkgs`, `pi`, `odysseus`,
  `opencode`, `tempest`, `fenix`. Five are GitHub sources; `fenix` is a
  flake that follows `nixpkgs`.
- `just store-audit` is non-blocking on missing prerequisites (nix/python3
  unavailable → exits 0); it fails ONLY on actual oversized `ai-workbench`
  `*-source` paths (the impure source-copy probe).
- The 50 MB threshold for `*-source` paths detects unbounded
  `builtins.path`/`cleanSourceWith` without a `filter` — the same class
  the `lint-nix` guard catches statically.
- `just update-hashes` surfaces correct FOD hashes (`npmDepsHash`,
  `bunDeps.outputHash`, `pipDeps.outputHash`) for manual inlining; it
  does NOT auto-rewrite files.
- NEVER run `nix flake update` in CI — a new nixpkgs revision pulls a
  multi-GB toolchain closure. Update deliberately, review the diff, then
  run `just gc`.
- For deeper integrity verification: `nix store verify --recursive
  .#workestrate` (signature/hash check), `nix build .#workestrate
  --rebuild` (reproducibility check).
- CVE scanning: Nix has no built-in CVE scanner; `vulnix` scans a store
  closure against the NVD database.
- Run inside `just shell` (see `nix-usage` skill).
- Related docs: `docs/nix/supply-chain-security.md`,
  `docs/nix/validation.md`.
