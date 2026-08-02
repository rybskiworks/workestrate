---
name: nix-devshells
description: |
  Operational guide for Nix development shells — mkShell/mkShellNoCC,
  shellHook, packages vs buildInputs, nix develop, pinning nixpkgs, direnv,
  and real-world devshell patterns. Load when writing or reviewing devshells,
  mkShell attributes, shellHook scripts, or diagnosing nix develop / PATH /
  untracked-file errors. Does NOT cover the Nix expression language (see
  nix-language), flake structure (see nix-flake-anatomy), or derivations/
  mkDerivation (see nix-derivations).
---

# Development Shells

Distilled operational reference. Full theory and citations live in
`docs/nix/devshells.md` (which cites the nix.dev ad-hoc/declarative shell
tutorials, the nixpkgs manual mkShell section, and the nix develop command
reference).

## Triggers

Load this skill when:

- Writing or reviewing a devshell (`mkShell`, `mkShellNoCC`, `shell.nix`, or
  `devShells.<system>.<name>` in a flake).
- Writing or debugging a `shellHook` script.
- Deciding between `packages`, `nativeBuildInputs`, `buildInputs`,
  `inputsFrom`.
- Diagnosing `nix develop` errors (untracked files, missing tools, store
  growth, stale `flake.lock`).
- Setting up direnv (`use flake` / `use nix`).

## References

This skill is an index into the docs corpus. Read the relevant doc for full
detail; upstream source URLs are listed in each doc's `## Sources used`
section.

- `docs/nix/devshells.md`
  - https://nix.dev/tutorials/first-steps/ad-hoc-shell-environments.html
  - https://nix.dev/tutorials/first-steps/declarative-shell.html
  - https://nix.dev/tutorials/first-steps/reproducible-scripts.html
  - https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs.html
  - https://nix.dev/guides/recipes/direnv.html
  - https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
  - https://nixos.org/manual/nixpkgs/stable/#sec-mkShell

## Key Rules

### mkShell attributes
`pkgs.mkShell` is a specialized `stdenv.mkDerivation` for devshells. It takes
all `mkDerivation` attributes plus:

| Attribute | Default | Purpose |
|---|---|---|
| `name` | `nix-shell` | Derivation name. |
| `packages` | `[]` | Executable packages (alias for `nativeBuildInputs`). |
| `inputsFrom` | `[]` | Inherit build dependencies of listed derivations. |
| `shellHook` | `""` | Bash statements run on shell entry. |

- `mkShellNoCC` uses `stdenvNoCC` — use when no C compiler is needed (avoids
  pulling in the unnecessary C toolchain).
- `packages` is the preferred attribute for executable packages (it is an
  alias for `nativeBuildInputs`).
- Any attribute name not reserved, with a string-coercible value, becomes an
  environment variable.
- Protected env vars (e.g. `PS1`) cannot be set as direct attributes — use
  `shellHook`.

### packages vs buildInputs vs nativeBuildInputs
- `packages` = alias for `nativeBuildInputs` (build-time tools on `$PATH`).
- `buildInputs` = runtime libraries (headers, `.so` files).
- In a devshell, `packages` is the idiomatic choice for tools; `buildInputs`
  for libraries the tools link against.
- `inputsFrom` inherits the full build dependency set of a derivation —
  useful for getting the exact env a package builds with.

### shellHook
- A bash script run on shell entry. Used for env setup, toolchain config, and
  staging runtime artifacts.
- Use for protected env vars (`PS1`) and complex setup that attributes can't
  handle.
- Relocate large build outputs out of the source tree here (e.g.
  `export CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/<project>/target"`)
  to prevent 5–25 GB of build artifacts polluting the repo / store.
- Clean up helper functions with `unset -f <fn>` after use (otherwise they
  leak into the interactive shell).

### nix develop
- `nix develop` enters `devShells.<system>.default`, falling back to
  `packages.<system>.default`.
- Named shells: `nix develop .#<name>`.
- Non-interactive: `nix develop -c <command>` (runs the command in the
  devshell env, then exits).
- Build phase options: `--unpack`, `--configure`, `--build`, `--check`,
  `--install`, `--installcheck`.
- `--impure` allows access to mutable paths — use deliberately, not by
  accident (copies raw working tree into the store).

### Pinning nixpkgs
- `shell.nix`: pin via `fetchTarball` with a revision hash. NEVER bare
  `<nixpkgs>` for reproducible envs (the channel can change at any time).
- Flakes: `flake.lock` pins all inputs. Update deliberately with
  `nix flake update` (NOT the deprecated `--update-input`).

### Flake devshell purity
- Git-tracked files only — untracked files are invisible to `.#` refs. `git
  add -N` new files before eval.
- `--pure` (nix-shell) discards most host env vars; omit for dev, add for
  CI/isolation.
- `--impure` (nix develop) allows mutable paths — use deliberately.

### direnv integration
- `echo "use flake" > .envrc && direnv allow` (flake projects).
- `echo "use nix" > .envrc && direnv allow` (shell.nix projects).
- direnv auto-reloads on `shell.nix`/`flake.nix` changes. Re-run `direnv
  allow` after editing `.envrc`.

### Multi-system devshells
- `flake-utils.lib.eachDefaultSystem` defines `devShells` across all default
  systems.
- Single-system flakes can hardcode `system = "x86_64-linux"` (project-
  specific decision).

## Quick Commands

```bash
nix develop                        # enter devShells.<system>.default
nix develop .#<name>                # enter a named devshell
nix develop -c <cmd>               # run a command non-interactively
nix develop -c <tool> --version    # verify a tool is available
nix flake check --no-build         # validate flake structure
nix build .#devShells.<system>.default  # build devshell derivation (CI)
```

## Anti-patterns

- Impure `shellHook` — referencing host paths or network in `shellHook`
  breaks reproducibility. Stage artifacts via derivations or flake inputs.
- Untracked file references — `nix develop` cannot see untracked files; `git
  add -N` first (error looks like a missing file/attr).
- Missing `git add` for new devshell files — flake eval fails on untracked
  sources.
- Setting `PS1` as a direct attribute — it's protected; use `shellHook`.
- Using `mkShell` when `mkShellNoCC` suffices — pulls in unnecessary C
  toolchain.
- Not relocating `CARGO_TARGET_DIR` — 5–25 GB of build artifacts in the
  source tree (and accidental commits / store copies).
- `nix develop --impure` unnecessarily — copies raw working tree into store.
- Forgetting `direnv allow` after changing `.envrc`.
- Not unsetting `shellHook` helper functions — leaks into interactive shell.
- Deprecated `--update-input` instead of `nix flake update`.
- Bare `<nixpkgs>` without pinning — not reproducible.
- Expecting `nix develop` to see uncommitted flake changes without `git add`.

## Related Skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, Microsandbox.
- nix-language — Nix expression language fundamentals.
- nix-flake-anatomy — flake.nix structure, inputs, outputs, flake.lock.
- nix-derivations — mkDerivation, build phases, FODs, sandbox.
