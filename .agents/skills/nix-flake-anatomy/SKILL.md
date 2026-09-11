---
name: nix-flake-anatomy
description: |
  Operational guide for the Nix flake.nix structure — inputs, outputs,
  flake.lock, flake registry, flake commands, and flake purity. Load when
  writing or reviewing flake.nix, adding inputs, structuring outputs, updating
  flake.lock, or diagnosing flake evaluation/purity errors. Does NOT cover the
  Nix expression language (see nix-language), derivations/mkDerivation (see
  nix-derivations), or devshells (see nix-devshells).
---

# Flake Anatomy

Distilled operational reference. Full theory and citations live in
`docs/nix/flake-anatomy.md` (which cites nix.dev/flakes, the NixOS wiki, and
RFC 49).

## Triggers

Load this skill when:

- Writing or reviewing `flake.nix` — adding inputs, structuring outputs,
  wiring `follows`.
- Updating or debugging `flake.lock`.
- Diagnosing "untracked file" errors, `--impure` misuse, or missing
  `experimental-features`.
- Deciding output classes (`packages`, `devShells`, `apps`, `checks`, `lib`).
- Reviewing a PR for flake reproducibility and purity.

## References

This skill is an index into the docs corpus. Read the relevant doc for full
detail; upstream source URLs are listed in each doc's `## Sources used`
section.

- `docs/nix/flake-anatomy.md`
  - https://nix.dev/concepts/flakes.html
  - https://nix.dev/concepts/faq.html
  - https://nix.dev/reference/pinning-nixpkgs.html
  - https://wiki.nixos.org/wiki/Flakes
  - https://github.com/NixOS/rfcs/pull/49

## Key Rules

### What is a flake?
A flake is a directory containing `flake.nix` (and a generated `flake.lock`).
It declares dependencies (`inputs`) and exports (`outputs`) with a standard
structure. Flakes are an experimental feature requiring Nix 2.4+ and
`experimental-features = nix-command flakes`.

### flake.nix structure
The top-level is a function `{ ... }: { ... }` returning an attribute set with
three keys: `description`, `inputs`, `outputs`.

```nix
{
  description = "My example flake";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default = self.packages.x86_64-linux.hello;
  };
}
```

### inputs
- Each input has a `url` (required) using a schema: `github:owner/repo`,
  `path:./subdir`, `tarball+https://...`, `git+https://...`, `https://...`.
- `inputs.<name>.follows = "<root-input>"` — override a transitive input to
  follow your version (use for nixpkgs to deduplicate across the dep graph).
- `inputs.<name>.flake = false` — import a non-flake repo as a plain source
  path (no `outputs` expected).
- `inputs.<name>.overrides` — override sub-inputs of a transitive dependency.

### outputs
- `outputs` is a function receiving `{ self, ...inputs }`. `self` is the
  flake's own outputs (for self-reference).
- Built-in output types: `packages.<system>.<name>`,
  `devShells.<system>.<name>`, `lib.<system>`/`lib`, `apps.<system>.<name>`,
  `checks.<system>.<name>`, `nixosModules.<name>`,
  `nixosConfigurations.<name>`, `overlays.<name>`, `templates.<name>`,
  `formatter.<system>`.
- The `default` name (e.g. `packages.<system>.default`) is the canonical
  entry point for each output class.

### flake.lock
- Pins all inputs by content hash. Generated/updated automatically by Nix.
- Structure: `nodes` (keyed by node name, each with `original`, `locked`,
  `inputs`), `root` (points to root node), `version` (currently `7`).
- `follows` serializes as an array path: `fenix.inputs.nixpkgs` =>
  `["nixpkgs"]`.
- Commit `flake.lock` to the repo for reproducibility.

### Flake registry
- `nix registry` manages aliases mapping short names (`nixpkgs`) to flake refs.
- `nix registry list` / `add` / `pin <name>` (pin freezes an alias to its
  current locked version).

### Flake purity
Flakes default to pure mode (hermetic evaluation):
- **Git-tracked files only** — untracked files are invisible. `git add` (even
  without committing) before referencing new files.
- No access to environment variables unless explicitly passed.
- No network during evaluation (fetches happen via inputs, not ad-hoc
  `builtins.fetchurl`).
- Escapes: `--impure` (breaks reproducibility), `--override-input <name>
  <ref>` (override an input at the CLI for testing local forks).

## Quick Commands

```bash
nix flake check                  # validate flake structure + run checks
nix flake show                   # list all outputs (confirms attr tree)
nix flake update                 # re-lock ALL inputs (deliberate; not in CI)
nix flake lock                   # create lock without updating existing
nix flake update --dry-run       # preview lock changes without writing
nix build .#<attr>               # build a flake output
nix develop                       # enter devShells.<system>.default
nix run .#<app>                   # run an app
nix registry list                # list registry aliases
```

All flake commands require `experimental-features = nix-command flakes` (set
in `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`, or pass
`--experimental-features 'nix-command flakes'`).

## Anti-patterns

- `--impure` in committed scripts/CI — breaks reproducibility silently.
- `builtins.getFlake` in pure evaluation paths — use native `.#` refs.
- Untracked files referenced by the flake — `git add` first or the flake
  cannot see them (error looks like a missing file/attr).
- Forgetting `flake = false` on a non-flake repo — Nix tries to evaluate its
  `outputs` and fails.
- Not using `follows` for nixpkgs — multiple nixpkgs versions bloat the store
  and cause glibc mismatches.
- Committing `flake.lock` with stale entries after editing `inputs` without
  running `nix flake lock`.
- Setting `experimental-features` only in user config, not CI — flakes work
  locally but fail in CI.
- Confusing `nix flake update` (re-locks all) with `nix flake lock` (creates
  without updating existing).
- `--override-input` baked into committed scripts — use deliberately for
  local testing only.
- Bare `nixpkgs.url = "github:nixos/nixpkgs"` (no ref) — floats with the
  default branch; prefer a tagged ref like `nixos-unstable`.

## Related Skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, Microsandbox.
- nix-language — Nix expression language fundamentals.
- nix-derivations — mkDerivation, build phases, FODs, sandbox.
- nix-devshells — mkShell, shellHook, nix develop.
