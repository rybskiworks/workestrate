---
type: Reference
resource: https://nix.dev/tutorials/first-steps/ad-hoc-shell-environments.html
title: Development Shells
description: Nix development shells — ad-hoc shells, declarative shell.nix with mkShell, shellHook, nix develop, pinning, direnv, and real-world devshell patterns.
tags: [nix, devshell, mkShell, shellHook, nix-develop]
timestamp: 2026-07-24T01:30:00Z
---

# Development Shells

## Purpose

Source-verified guidance for Nix development shells. Covers ad-hoc shells,
declarative `shell.nix` with `mkShell`, `shellHook`, `nix develop`, pinning
nixpkgs, direnv integration, and real-world devshell patterns. Agents who set
up, modify, or debug Nix devshells should follow these rules.

## Sources used

- Crawl file: `docs/nix/.crawl/06-ad-hoc-shell-environments.md`
  — https://nix.dev/tutorials/first-steps/ad-hoc-shell-environments.html
- Crawl file: `docs/nix/.crawl/07-declarative-shell-environments.md`
  — https://nix.dev/tutorials/first-steps/declarative-shell.html
- Crawl file: `docs/nix/.crawl/08-reproducible-interpreted-scripts.md`
  — https://nix.dev/tutorials/first-steps/reproducible-scripts.html
- Crawl file: `docs/nix/.crawl/09-towards-reproducibility-pinning-nixpkgs.md`
  — https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs.html
- Crawl file: `docs/nix/.crawl/36-direnv.md`
  — https://nix.dev/guides/recipes/direnv.html
- Crawl file: `docs/nix/.crawl/43-pinning-nixpkgs.md`
  — https://nix.dev/reference/pinning-nixpkgs.html
- Crawl file: `docs/nix/.crawl/63-nix-command-develop.md`
  — https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
- Crawl file: `docs/nix/.crawl/64-nix-command-run.md`
  — https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-run
- Crawl file: `docs/nix/.crawl/67-nixpkgs-mkShell.md`
  — https://nixos.org/manual/nixpkgs/stable/#sec-mkShell
- Project files: `flake.nix` and `justfile` — shared-tooling inputs,
  `devenv.shells.{bootstrap,default}` and the explicit shell entry points

## Core guidance

### Ad-hoc shell environments

The classic `nix-shell -p` approach enters a temporary shell with the named
packages available. From crawl 06:

> "In a Nix shell environment, you can immediately use any program packaged
> with Nix, without installing it permanently."

```shell
# Enter a shell with cowsay and lolcat
nix-shell -p cowsay lolcat
```

Run a program once without staying in the shell (crawl 06):

```shell
nix-shell -p cowsay --run "cowsay Nix"
```

Nested shells work — running `nix-shell -p python3` inside an existing
nix-shell stacks the new environment on top of the current one (crawl 06).

The modern flake-based equivalents use the new CLI:

```shell
# Enter a shell with nodejs
nix shell nixpkgs#nodejs

# Run a program once without entering a shell
nix run nixpkgs#cowsay -- hello
```

From crawl 64:

> "Run `vim` from the `nixpkgs` flake: `nix run nixpkgs#vim`"

> "Run `vim` with arguments: `nix run nixpkgs#vim -- --help`"

### Reproducible interpreted scripts

A Nix shebang makes a script self-contained: it declares its own interpreter,
packages, and pinned nixpkgs. From crawl 08:

> "We will use the shebang line `#!/usr/bin/env nix-shell`."

The multi-line shebang construct (crawl 08):

```shell
#!/usr/bin/env nix-shell
#! nix-shell -i bash --pure
#! nix-shell -p bash cacert curl jq python3Packages.xmljson
#! nix-shell -I nixpkgs=https://github.com/NixOS/nixpkgs/archive/2a601aafdc5605a5133a2ca506a34a3a73377247.tar.gz

curl https://github.com/NixOS/nixpkgs/releases.atom | xml2json | jq .
```

Flag meanings:

- `-i` — interpreter to run the script with (e.g. `bash`)
- `--pure` — excludes most environment variables from the calling shell
- `-p` — packages to make available
- `-I` — search path / nixpkgs pin (URL to a tarball)

### Declarative shell.nix with mkShell

A `shell.nix` file declares the devshell as a Nix expression. From crawl 07:

> "`nix-shell` by default looks for a file called `shell.nix` in the current
> directory and builds a shell environment from the Nix expression in this
> file."

Basic `shell.nix` using `mkShellNoCC` (crawl 07):

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
pkgs.mkShellNoCC {
  packages = with pkgs; [
    cowsay
    lolcat
  ];
}
```

Environment variables as direct attributes (crawl 07):

> "Any attribute name passed to `mkShellNoCC` that is not reserved otherwise
> and has a value which can be coerced to a string will end up as an
> environment variable."

Warning about protected variables like `PS1` (crawl 07): some environment
variables are reserved/protected by `mkShell` and cannot be set as direct
attributes — use `shellHook` for those.

### mkShell attributes (from crawl 67 — nixpkgs manual)

From crawl 67:

> "`pkgs.mkShell` is a specialized `stdenv.mkDerivation` that removes some
> repetition when using it with `nix-shell` (or `nix develop`)."

Attribute table (crawl 67):

| Attribute | Default | Purpose |
| --- | --- | --- |
| `name` | `nix-shell` | Set the name of the derivation. |
| `packages` | `[]` | Add executable packages to the `nix-shell` environment. |
| `inputsFrom` | `[]` | Add build dependencies of the listed derivations to the `nix-shell` environment. |
| `shellHook` | `""` | Bash statements that are executed by `nix-shell`. |

Plus: "… all the attributes of `stdenv.mkDerivation`."

`mkShellNoCC` variant (crawl 67):

> "uses `stdenvNoCC` instead of `stdenv` as base environment. This is useful
> if no C compiler is needed in the shell environment."

From crawl 07:

> "`mkShellNoCC` is a wrapper around `mkDerivation`, so it takes the same
> arguments as `mkDerivation`, such as `buildInputs` or `nativeBuildInputs`.
> The `packages` attribute argument to `mkShellNoCC` is simply an alias for
> `nativeBuildInputs`."

`inputsFrom` usage example from crawl 67:

```nix
{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  packages = [ pkgs.gnumake ];
  inputsFrom = [
    pkgs.hello
    pkgs.gnutar
  ];
  shellHook = ''
    export DEBUG=1
  '';
}
```

### shellHook

From crawl 07:

> "You may want to run some shell commands before entering the interactive
> shell environment. These commands can be placed in the `shellHook`
> attribute provided to `mkShellNoCC`."

Example (crawl 07):

```nix
shellHook = ''
  echo $GREETING | cowsay | lolcat
'';
```

`shellHook` is a bash script run on shell entry. It is used for environment
setup and toolchain configuration. For protected
env vars like `PS1`, use `shellHook` instead of direct attributes (crawl 07
warning).

### nix develop

From crawl 63:

> "`nix develop` — run a bash shell that provides the build environment of a
> derivation"

> "`nix develop` starts a `bash` shell that provides an interactive build
> environment nearly identical to what Nix would use to build _installable_."

Entering a devshell:

```shell
nix develop
```

This uses `devShells.<system>.default`, falling back to
`packages.<system>.default` (crawl 63).

Named shells (crawl 63):

```shell
nix develop .#<name>
```

Flake output attributes tried (crawl 63):

- Without a name: `devShells.<system>.default`, then `packages.<system>.default`
- With a name: `devShells.<system>.<name>`, `packages.<system>.<name>`,
  `legacyPackages.<system>.<name>`

Non-interactive mode (crawl 63):

> "Instead of starting an interactive shell, start the specified command and
> arguments."

```shell
nix develop -c <command>
```

Example (crawl 63):

```shell
nix develop --command bash -c "mkdir build && cmake .. && make"
```

Build phase options (crawl 63): `--unpack`, `--configure`, `--build`,
`--check`, `--install`, `--installcheck`.

`--impure` (crawl 63):

> "Allow access to mutable paths and repositories."

`--profile <path>`: record the build environment in a profile (crawl 63).

`--redirect` (crawl 63):

> "Replace all occurrences of the store path corresponding to `glibc.dev`
> with a writable directory"

### Pinning nixpkgs

From crawl 09:

> "the resulting Nix expression is not fully reproducible"

…when using `<nixpkgs>`, because the channel can change at any time.

`fetchTarball` pinning (crawl 09):

```nix
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/06278c77b5d162e62df170fec307e83f1812d94b.tar.gz") {}
}:
```

From crawl 09:

> "Picking the commit can be done via status.nixos.org, which lists all the
> releases and the latest commit that has passed all tests."

Recommended: follow the latest stable NixOS release (e.g. `nixos-24.05`) or
`nixos-unstable` (crawl 09).

Flake-based pinning: `flake.lock` pins all inputs. Note: `nix flake lock
--update-input nixpkgs` is deprecated per crawl 63; use `nix flake update`
instead.

From crawl 43: URL forms for pinning — specific commit tarball, channel
version, shorthand `channel:nixos-22.11`.

### direnv integration

Workestrate does NOT use direnv — there is no `.envrc` in this repo. The
canonical entry is a single command:

```shell
just shell
```

for an interactive shell, or `just shell -c <cmd>` for a one-shot command.
`just bootstrap` selects the tooling-only shell when the application/runtime
is not needed or does not yet build. Both recipes pass a `devenv-root` input
override: a file under `$HOME/.cache/workestrate/devenv-root/`, keyed by the
checkout path, contains the absolute worktree root. Argument boundaries are
preserved, including quoted command strings and empty arguments.

Repository `just check`, `just deny-check` and `just verify` build pinned Nix
checks directly; they do not enter the runtime-aware shell. Some interactive
Cargo recipes still re-enter the default shell when
`WORKESTRATE_DEVSHELL` is unset. That marker is supplied by the default shell,
not bootstrap, and should not be injected to bypass setup. No direnv,
`.envrc` or `direnv allow` is required.

(Background, retained for reference: crawl 36 describes direnv
auto-reloading a declarative shell on directory entry — `use nix` /
`use flake` + `direnv allow`, auto-reloading on `shell.nix` changes. That
pattern is intentionally not used here.)

### Devshell purity

`--pure` flag (nix-shell) (crawl 06):

> "discards most environment variables set on your system when running the
> shell"

From crawl 06:

> "we recommend to omit `--pure` for development environments, and to add it
> only when the extra isolation is needed"

Flake-based devshells: git-tracked files only — untracked files are invisible
to `.#` refs; `git add -N` before eval.

`--impure` flag for `nix develop` (crawl 63):

> "Allow access to mutable paths and repositories."

### Multi-system devshells

The `flake-utils.lib.eachDefaultSystem` pattern defines `devShells` across
multiple systems:

```nix
flake-utils.lib.eachDefaultSystem (system:
  let pkgs = nixpkgs.legacyPackages.${system}; in {
    devShells.default = pkgs.mkShellNoCC { /* ... */ };
  })
```

The Workestrate flake uses a single system (`systems = [ "x86_64-linux" ]`,
via flake-parts) instead — this is a project-specific decision
(see `flake.nix`).

### Real-world example: the Workestrate shells

Source: `flake.nix` — `devenv.shells.bootstrap` and `devenv.shells.default`.
These are devenv modules rather than standalone `mkShell` files. Shared
`nix-tooling` owns the nixpkgs/Fenix/devenv versions, and Workestrate follows
those inputs. See [Nix builds](../nix-build.md) for package ownership and
verification boundaries.

Shape of the live definition:

- `devenv.root`: NOT set in the flake. The auto-imported readDevenvRoot
  module derives it from the `devenv-root` input placeholder; entry points
  pass `--override-input devenv-root "file+file://<rootfile>"` (the root
  file under `$HOME/.cache/workestrate/devenv-root/` holds the worktree abs
  path), so pure eval resolves `devenv.root` to the worktree without impurity.
- `devenv.dotfile` / `devenv.state`: devenv defaults — dotfile =
  `<root>/.devenv` (gitignored in-tree; the task cache lives here too on
  the pinned devenv), state = `<dotfile>/state`.
- `imports`: shared `inputs.tooling.devenvModules.{beads,base,determinate,nix,toml,rust}`
  modules in both shells. Bootstrap adds only toolchain/build-investigation
  tools; default also supplies the Workestrate and paired Microsandbox
  packages and development helpers. Realizing default may build those packages
  when they are not cached; bootstrap does not require them.
- Both shells and the explicit hook installer select the shared supplier's
  pinned Determinate Nix binary output. The formatter module remains separate.
  Shell entry does not install or replace a daemon, select a different store,
  or alter daemon trust/sandbox settings. Guest daemon configuration belongs to
  the guest image; changing the host daemon is a separate operation.
- Both shells disable automatic Git-hook installation and tree formatting.
  Install hooks explicitly with `nix run .#install-hooks` when intended.
- Both preserve a nonempty explicit `CARGO_TARGET_DIR`, otherwise selecting
  `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`.
  The shared `scripts/cargo-target.sh` validator requires an absolute target
  outside the evaluated checkout root, resolving existing symlink aliases
  and `..` with GNU `realpath -m`. Invalid targets abort before shell setup.
  Neither validation nor shell entry creates or populates that directory.
- Default `enterShell` supplies `MSB_BUILD_RUNTIME` pointing at the immutable
  Microsandbox package, its `MSB_AGENTD_PATH`, and `WORKESTRATE_DEVSHELL=1`.
  The SDK validates explicit build inputs without downloading a replacement.
  `MSB_HOME` remains a separate runtime-state selector; shell entry does not
  set it or create runtime staging/cache homes.
- Default creates or refreshes only the SDK source symlink
  `control/agentctl/vendor/microsandbox-fork`, pointing at the pinned runtime
  package's filtered source. A real directory at that path is deliberately
  left alone and reported as unlocked. Existing workload source/build
  directories are not replaced.
- Bootstrap does not set the default-shell marker, configure SDK runtime
  inputs or refresh the SDK source link. It is not a substitute for default
  shell setup before interactive SDK compilation.
- Default removes its temporary shell helper functions after link setup.
  Neither shell clones/builds agents, installs workload dependencies, starts
  VMs, cleans runtime state or performs home/schema migration.

Normal devenv profile/task-cache bookkeeping can change `.devenv` on entry;
that is different from modifying hooks, workload builds or VM state. Neither
shell is an environment scrubber: use an explicitly isolated caller environment
when validating that no ambient HOME/XDG/runtime state is accessed.

There is a known build-identity limitation: the transient `devenv-root`
override can remove clean Git revision metadata, causing the application to
be rebuilt with a `dirty` display revision even when its filtered source is
unchanged. This does not justify faking the revision or using `--impure`.
Bootstrap avoids requiring the application when only tools are needed.

### Cold shell dependencies

The pinned devenv task module intentionally uses its own locked build inputs
so its binary matches the supplier cache. On a cold store, its source imports
require shell-only import-from-derivation; repository application and check
gates remain independent and can keep IFD disabled. Do not replace the task
package's inputs merely to eliminate these source imports: that changes its
binary-cache identity.

When the supplier cache is not already configured, it can be selected explicitly
for one shell invocation, using the public key declared by the pinned devenv
source:

```sh
just bootstrap \
  --option extra-substituters https://devenv.cachix.org \
  --option extra-trusted-public-keys 'devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw='
```

This grants trust to that cache for the command; it does not install a signing
key or alter global daemon configuration. A guest may instead declare reviewed
cache endpoints and public keys in its NixOS configuration. Inspect a dry run
before a cold build, and distinguish substituted artifacts from local builds.

## Practical rules

1. Use `nix develop` (flake-based) over `nix-shell` for new projects;
   `nix-shell` for legacy `shell.nix`.
2. Use `mkShellNoCC` when no C compiler is needed; `mkShell` when it is
   (crawl 67).
3. `packages` is an alias for `nativeBuildInputs` in `mkShell` (crawl 07).
4. Use `shellHook` for env setup that can't be done via attributes (e.g.
   `PS1`, complex setup scripts) (crawl 07).
5. Pin nixpkgs via `fetchTarball` in `shell.nix` or `flake.lock` in flakes
   (crawl 09).
6. `git add -N` new files before `nix develop`/`nix build` — untracked files
   are invisible to flake refs.
7. Use `nix develop -c <cmd>` for non-interactive commands in the devshell
   (crawl 63).
8. Distinguish legacy `nix-shell --pure` environment isolation from Nix
   evaluation purity. Workestrate flake builds and shell entry remain pure;
   do not add `--impure` to work around missing root/input declarations.
9. In this repo enter via `just shell` (`just shell -c <cmd>`
   non-interactive); bare `nix develop` fails pure eval on the
   `devenv.root` assertion (the override is wired in `just shell`). Some
   interactive Cargo recipes self-enshell via `$WORKESTRATE_DEVSHELL`,
   while repository verification directly builds sandboxed checks. No direnv
   is needed. Use `just bootstrap` for tooling-only work.
10. Relocate large build outputs (e.g. `CARGO_TARGET_DIR`) out of the source
    tree in `shellHook`.
11. Use `inputsFrom` to inherit build dependencies from existing derivations
    (crawl 67).
12. Clean up `shellHook` functions with `unset -f` after use (workestrator
    pattern).

## Review checklist

- [ ] nixpkgs pinned (`fetchTarball` in `shell.nix` or `flake.lock` in flakes)?
- [ ] `packages` used for executable packages (alias for `nativeBuildInputs`)?
- [ ] `shellHook` used for env setup that attributes can't handle?
- [ ] Protected env vars (`PS1` etc.) set via `shellHook`, not direct attributes?
- [ ] `mkShellNoCC` used when no C compiler needed?
- [ ] Large build outputs relocated out of source tree?
- [ ] `shellHook` functions unset after use?
- [ ] Untracked files `git add -N`'d before eval?
- [ ] No `.envrc`/direnv — explicit bootstrap/default entry points and direct verification checks?
- [ ] `inputsFrom` used to inherit deps from existing derivations where applicable?
- [ ] `just shell -c` used for non-interactive commands?
- [ ] Workestrate flake evaluation remains pure, without `--impure` workarounds?

## Implementation checklist

- [ ] Choose `mkShell` vs `mkShellNoCC` (C compiler needed?).
- [ ] Pin nixpkgs (`fetchTarball` URL or flake input).
- [ ] List packages in the `packages` attribute.
- [ ] Set env vars as direct attributes (coercible to string).
- [ ] Use `shellHook` for complex setup (bash script).
- [ ] Use `inputsFrom` to inherit deps from existing derivations.
- [ ] Add `devShells.<system>.default` to flake outputs.
- [ ] For multi-system: use `flake-utils.lib.eachDefaultSystem`.
- [ ] Relocate large build outputs (`CARGO_TARGET_DIR`) in `shellHook`.
- [ ] `unset -f` any helper functions defined in `shellHook`.
- [ ] Do NOT add `.envrc` — use explicit `just shell`/`just bootstrap`; keep verification independent of runtime shell setup.
- [ ] `git add -N` new files before first `nix develop`.

## Runtime / debugging checklist

- [ ] `just shell` enters the shell (check prompt changes).
- [ ] `just shell -c <tool> --version` verifies tools are available.
- [ ] `just verify` validates the repository's explicit check outputs without entering the default shell.
- [ ] Use the root override supplied by `just shell`/`just bootstrap`, not `--impure`, for shell entry.
- [ ] `shellHook` errors: check stderr on shell entry.
- [ ] Missing packages: `just shell -c which <tool>` to verify PATH.
- [ ] Store growth: inspect live roots and retained outputs before considering garbage collection; shell entry does not perform it.
- [ ] Untracked-file errors: `git add -N <file>` then retry.
- [ ] Unexpected lock changes: inspect the input graph; use `nix flake update <input>` only for an intentional dependency update.

## Validation hooks

- `nix flake check --no-build` — evaluates outputs, not their build success;
  Workestrate's development shells also require the root input supplied by
  the explicit shell entry points.
- `just verify` — builds the repository's check outputs and runs its script
  fixtures without entering the default shell.
- `just shell -c <tool> --version` — verifies tool availability.
- `just shell` — enters shell; check `shellHook` output.
- `nix build .#devShells.<system>.default` — builds the devshell derivation
  (useful for CI).

From crawl 67:

> "This derivation output will contain a text file that contains a reference
> to all the build inputs. This is useful in CI where we want to make sure
> that every derivation, and its dependencies, build properly."

## Examples

### Ad-hoc shell (modern flake CLI)

```shell
# Enter a shell with nodejs
nix shell nixpkgs#nodejs

# Run a program once without entering a shell
nix run nixpkgs#cowsay -- hello

# Run with arguments
nix run nixpkgs#vim -- --help
```

### Ad-hoc shell (classic nix-shell)

```shell
# Enter a shell with cowsay and lolcat
nix-shell -p cowsay lolcat

# Run a program once
nix-shell -p cowsay --run "cowsay Nix"

# Pure, pinned shell
nix-shell -p git --run "git --version" --pure -I nixpkgs=https://github.com/NixOS/nixpkgs/tarball/2a601aafdc5605a5133a2ca506a34a3a73377247
```

### Reproducible script with nix shebang

```shell
#!/usr/bin/env nix-shell
#! nix-shell -i bash --pure
#! nix-shell -p bash cacert curl jq python3Packages.xmljson
#! nix-shell -I nixpkgs=https://github.com/NixOS/nixpkgs/archive/2a601aafdc5605a5133a2ca506a34a3a73377247.tar.gz

curl https://github.com/NixOS/nixpkgs/releases.atom | xml2json | jq .
```

### Basic shell.nix

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
pkgs.mkShellNoCC {
  packages = with pkgs; [
    cowsay
    lolcat
  ];

  GREETING = "Hello, Nix!";

  shellHook = ''
    echo $GREETING | cowsay | lolcat
  '';
}
```

### mkShell with inputsFrom (from crawl 67)

```nix
{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  packages = [ pkgs.gnumake ];
  inputsFrom = [
    pkgs.hello
    pkgs.gnutar
  ];
  shellHook = ''
    export DEBUG=1
  '';
}
```

### Flake devShell (basic)

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShellNoCC {
        packages = with pkgs; [ git curl jq ];
      };
    };
}
```

### Flake devShell with flake-utils (multi-system)

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        devShells.default = pkgs.mkShellNoCC {
          packages = with pkgs; [ git curl jq ];
        };
      });
}
```

### Shell entry (no direnv)

```shell
just shell             # interactive shell
just shell -c <cmd>    # one-shot command in the devshell
```

`just bootstrap -c <cmd>` enters the tooling-only shell. `just check`,
`just deny` and `just verify` directly build sandboxed checks; some interactive
Cargo recipes still self-enshell when `$WORKESTRATE_DEVSHELL` is unset.
Consult the recipe before assuming it enters a shell. No `.envrc` or
`direnv allow` is required.

### Workestrate devshell excerpt (enterShell — CARGO_TARGET_DIR relocation)

```nix
# From flake.nix, devenv.shells.default.enterShell
CARGO_TARGET_DIR="$(${pkgs.bash}/bin/bash ${./scripts/cargo-target.sh} ${pkgs.lib.escapeShellArg config.devenv.shells.default.devenv.root})" || exit 1
export CARGO_TARGET_DIR
```

Bootstrap uses the same helper with its own evaluated `devenv.root`. Keeping
assignment separate from `export` preserves validation failures. Bare `just`
uses the same validation at parse time; a nested shell or recipe therefore
retains an already valid caller-owned target. Empty overrides use the existing
XDG/HOME fallback. See [the purity contract](../nix-purity.md#the-rules) for
alias, missing-directory and missing-HOME behavior.

Cargo creates its target directory when compilation runs; shell entry only
exports the validated location. This does not make mutable development
artifacts safe inputs to Nix package builds.

### Workestrate devshell excerpt (packages)

```nix
# From flake.nix, devenv.shells.default
packages = with pkgs; [
  actionlint
  age
  workestrate
  rustToolchain.cargo
  rustToolchain.clippy
  curl
  decrypt-env
  gcc
  git
  jq
  just
  libcap_ng
  msb-wrapped
  nodejs_24
  bun
  cargo-deny
  openssl
  pkg-config
  (python3.withPackages (p: [ p.pip ]))
  (python312.withPackages (ps: [
    ps.pip
    ps."pip-tools"
  ]))
  rustToolchain.rustc
  rustToolchain.rust-analyzer
  rustToolchain.rustfmt
  sops
  tombi
  write-env
  setup-secrets
  zizmor
];
```

## Common mistakes

- Using `<nixpkgs>` without pinning — not reproducible (crawl 09).
- Forgetting `git add -N` for new files — flake can't see untracked files.
- Setting `PS1` as a direct attribute — it's protected, use `shellHook`
  (crawl 07).
- Using `mkShell` when `mkShellNoCC` suffices — pulls in unnecessary C
  toolchain (crawl 67).
- Not relocating `CARGO_TARGET_DIR` — 5–25 GB of build artifacts in the
  source tree.
- Using `nix develop --impure` to hide undeclared evaluation inputs instead of
  supplying Workestrate's explicit root input.
- Forgetting `direnv allow` after changing `.envrc`.
- Not unsetting `shellHook` functions — leaks into interactive shell.
- Using deprecated `--update-input` instead of `nix flake update` (crawl 63).
- Expecting a Git flake to see untracked files before `git add -N`; edits to
  already tracked files are visible without staging.

## Strict vs contextual guidance

### Strict (always follow)

- Pin nixpkgs (`fetchTarball` or `flake.lock`) — never use bare `<nixpkgs>`
  for reproducible envs (crawl 09).
- `packages` is the preferred attribute for executable packages (alias for
  `nativeBuildInputs`) (crawl 07).
- Use `shellHook` for protected env vars and complex setup (crawl 07).
- `git add -N` new files before flake eval.
- Clean up `shellHook` functions with `unset -f`.

### Contextual (depends on the project)

- `mkShell` vs `mkShellNoCC` — depends on whether a C compiler is needed
  (crawl 67).
- `--pure` — omit for dev, add for CI/isolation (crawl 06).
- `nix-shell` (classic) vs `nix develop` (flake) — depends on project setup.
- Single-system vs `flake-utils.lib.eachDefaultSystem` — depends on target
  platforms.
- direnv integration — optional but convenient (crawl 36).
- `CARGO_TARGET_DIR` relocation — only for Rust projects with large build
  outputs.

## Policy decisions for individual repos

- Default to `mkShellNoCC` unless a C compiler is explicitly needed.
- Pin nixpkgs to a stable release branch (e.g. `nixos-24.05`) or
  `nixos-unstable`.
- Require `git add -N` before any flake eval in CI.
- Require `shellHook` functions to be unset after use.
- Require `CARGO_TARGET_DIR` relocation for Rust projects.
- Decide repo-wide whether to use direnv.
- Decide single-system vs multi-system (`flake-utils`).
- Require `nix flake check --no-build` in CI.

## Related docs

- /docs/nix/source-map.md — Nix source map (provenance index)
- (Future) /docs/nix/flakes.md — Nix flakes reference
- (Future) /docs/nix/nix-commands.md — Nix CLI commands reference
- (Future) /docs/nix/nixpkgs-stdenv.md — stdenv and mkDerivation reference

## Related skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference
