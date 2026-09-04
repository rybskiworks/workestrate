---
type: Reference
resource: https://nix.dev/tutorials/packaging-existing-software.html
title: Derivations and Builds
description: Nix derivations, stdenv.mkDerivation, build phases, hooks, source fetchers, multi-output derivations, and fixed-output derivations (FODs).
tags: [nix, derivations, mkDerivation, stdenv, builds, FOD]
timestamp: 2026-07-24T01:15:00Z
---

# Derivations and Builds

## Purpose

Provide concrete, repo-independent guidance for authoring, reviewing, and
debugging Nix derivations: the primitive `derivation` builtin, the
`stdenv.mkDerivation` generic builder, the build-phase sequence, phase hooks,
dependency attributes, source fetchers, multi-output derivations, and
fixed-output derivations (FODs). This document is intended as generic
reference material for future AI coding agents working in any Nix flake. It is
not specific to the `ai-workbench` repository.

Agents should use this document as the authoritative reference when creating
or modifying a derivation, choosing between `nativeBuildInputs` and
`buildInputs`, wiring a custom phase with hooks, prefetching a source
hash, or deciding whether a build step must run under a FOD.

## Sources used

- <https://nix.dev/tutorials/packaging-existing-software.html>
- <https://nix.dev/manual/nix/2.34/language/derivations>
- <https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv>
- <https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases>
- <https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview>
- <https://github.com/NixOS/rfcs/pull/35> (RFC 0035 — pname and version)
- Local: `/docs/nix-purity.md` — workestrator purity rules
- Local: `/nix/packages/agentctl.nix` — agentctl derivation
- Local: `/nix/packages/pi.nix` — pi agent derivation

### Crawl ledger

SEED:

- <https://nix.dev/tutorials/packaging-existing-software.html>
- <https://nix.dev/manual/nix/2.34/language/derivations>
- <https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv>

DISCOVERED & VISITED:

- <https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases>
- <https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview>
- <https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-reference>
- <https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-propagated>
- <https://github.com/NixOS/rfcs/pull/35>

SKIPPED (out of scope / tangential):

- <https://nixos.org/manual/nixpkgs/stable/#chap-cross> — cross-compilation
  platform-offset formalism (referenced at summary level only)
- <https://nixos.org/manual/nixpkgs/stable/#chap-meta> — meta-attributes
  (covered at summary level only)
- <https://nixos.org/manual/nixpkgs/stable/#chap-pkgs-fetchers> — full fetcher
  catalogue (only `fetchurl`/`fetchzip`/`fetchFromGitHub` covered here)

## Core guidance

### What is a derivation?

The primitive `derivation` builtin is the lowest-level Nix construct for
describing a store layer. From the Nix manual [2]:

> The most important built-in function is `derivation`, which is used to
> describe a single store-layer store derivation.

This builtin function takes an attribute set as input, produces a store
derivation (`.drv`) as a side effect of evaluation, and returns an attribute
set describing the outputs. It is rarely invoked directly in nixpkgs;
`stdenv.mkDerivation` wraps it with a generic builder.

#### Required input attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `name` | String | A symbolic name for the derivation. Affects the store path: `/nix/store/<hash>-<name>.drv` and outputs `/nix/store/<hash>-<name>[-<output>]`. |
| `system` | String | The system type the derivation is built for (e.g. `x86_64-linux`). `builtins.currentSystem` defaults to the evaluating host's system. |
| `builder` | Path \| String | The builder executable. May be a store path (`/bin/bash`), a local file (`./builder.sh`), or a path from another derivation (`"${pkgs.python}/bin/python"`). |

#### Optional input attributes

| Attribute | Type | Default | Description |
| --- | --- | --- | --- |
| `args` | List of String | `[]` | Arguments passed to the builder executable. |
| `outputs` | List of String | `["out"]` | Symbolic outputs. Each output name becomes an environment variable in the builder pointing to its store path. |

The `outputs` attribute enables multi-output derivations. From the Nix manual
[2]:

> Imagine a library package that provides a dynamic library, header files,
> and documentation. A program that links against such a library doesn't need
> the header files and documentation at runtime, and it doesn't need the
> documentation at build time. Thus, the library package could specify:
>
> ```nix
> derivation {
>   # ...
>   outputs = [ "lib" "dev" "doc" ];
>   # ...
> }
> ```
>
> This will cause Nix to pass environment variables `lib`, `dev`, and `doc` to
> the builder containing the intended store paths of each output.

The first element of `outputs` determines the default output and ends up at
the top-level attribute. See [Multi-output derivations](#multi-output-derivations)
below.

#### Environment variable translation rules

Every attribute other than the recognized ones is passed as an environment
variable to the builder. Values are translated as follows [2]:

- Strings are passed unchanged.
- Integral numbers are converted to decimal notation.
- Floating point numbers are converted to simple decimal or scientific
  notation with a preset precision.
- A path causes the referenced file to be copied to the store; its store
  location is put in the environment variable.
- A derivation causes that derivation to be built first; the environment
  variable is set to the store path of the derivation's default output.
- Lists of the previous types are concatenated, separated by spaces.
- `true` is passed as the string `1`; `false` and `null` are passed as an
  empty string.

### stdenv.mkDerivation: the standard builder

`stdenv.mkDerivation` wraps the primitive `derivation` with a generic bash
builder. From the nixpkgs manual [3]:

> To build a package with the standard environment, you use the function
> `stdenv.mkDerivation`, instead of the primitive built-in function
> `derivation`

A minimal derivation needs a name and a source. From [3]:

> Specifying a `name` and a `src` is the absolute minimum Nix requires. For
> convenience, you can also use `pname` and `version` attributes and
> `mkDerivation` will automatically set `name` to `"${pname}-${version}"` by
> default. **Since [RFC 0035], this is preferred for packages in Nixpkgs**,
> as it allows us to reuse the version easily.

The `finalAttrs` pattern allows cross-referencing attributes within the
derivation (e.g. using `version` in the `src` URL) [3]:

```nix
stdenv.mkDerivation (finalAttrs: {
  pname = "libfoo";
  version = "1.2.3";
  src = fetchurl {
    url = "http://example.org/libfoo-source-${finalAttrs.version}.tar.bz2";
    hash = "sha256-tWxU/LANbQE32my+9AXyt3nCT7NBVfJ45CX757EMT3Q=";
  };
})
```

The generic builder loads the stdenv `setup.sh` bash library and calls
`genericBuild` [4]:

> `stdenv.mkDerivation` sets the Nix derivation's builder to a script that
> loads the stdenv `setup.sh` bash library and calls `genericBuild`. Most
> packaging functions rely on this default builder.

### Build phases

The generic builder splits a build into phases, each overridable individually.
The default phase sequence, when `phases` is unset, is [4]:

> `$prePhases unpackPhase patchPhase $preConfigurePhases configurePhase
> $preBuildPhases buildPhase checkPhase $preInstallPhases installPhase
> fixupPhase installCheckPhase $preDistPhases distPhase $postPhases`

| Phase | Default behavior |
| --- | --- |
| `unpackPhase` | Unpacks `src`/`srcs` to the current directory. Supports tar (gzip/bzip2/xz), zip (requires `unzip` in `nativeBuildInputs`), and store directories (copied, hash stripped). |
| `patchPhase` | Applies the `patches` list via the `patch` command (default flag `-p1`). |
| `configurePhase` | Runs `./configure` if it exists (Autoconf). Honors `configureFlags`, `configureFlagsArray`, `prefix` (default `$out`). |
| `buildPhase` | Runs `make` if a `Makefile`/`makefile`/`GNUmakefile` exists. Honors `makeFlags`, `makeFlagsArray`, `buildFlags`. |
| `checkPhase` | Runs `make $checkTarget` only if `doCheck = true`. Skipped entirely under cross-compilation. |
| `installPhase` | Creates `$out`, runs `make install`. Honors `installTargets` (default `install`), `installFlags`. |
| `fixupPhase` | Moves `man/`/`doc/`/`info/` to `share/`; strips debug info; runs `patchelf` to prune `RPATH` (Linux); rewrites shebangs to store paths. |
| `installCheckPhase` | Runs `make installcheck` only if `doInstallCheck = true`. Skipped under cross-compilation. |
| `distPhase` | Runs `make dist`, copies tarballs to `$out/tarballs/` only if `doDist` is set. |

### Phase hooks

Hooks are shell functions executed before and after each phase. From the
nix.dev packaging tutorial [1]:

> During derivation realisation, there are a number of shell functions
> ("hooks", in Nixpkgs) which may execute in each derivation phase. Hooks do
> things like set variables, source files, create directories, and so on.

The `runHook` pattern is mandatory when overriding a phase. From [4]:

> When overriding a phase, for example `installPhase`, it is important to
> start with `runHook preInstall` and end it with `runHook postInstall`,
> otherwise `preInstall` and `postInstall` will not be run. Even if you don't
> use them directly, it is good practice to do so anyways for downstream users
> who would want to add a `postInstall` by overriding your derivation.

The hook pairs, one per phase:

| Phase | Pre hook | Post hook |
| --- | --- | --- |
| unpack | `preUnpack` | `postUnpack` |
| patch | `prePatch` | `postPatch` |
| configure | `preConfigure` | `postConfigure` |
| build | `preBuild` | `postBuild` |
| check | `preCheck` | `postCheck` |
| install | `preInstall` | `postInstall` |
| fixup | `preFixup` | `postFixup` |
| installCheck | `preInstallCheck` | `postInstallCheck` |
| dist | `preDist` | `postDist` |

Additional phase-injection variables insert extra phases at specific points
without overriding the whole `phases` list: `prePhases`,
`preConfigurePhases`, `preBuildPhases`, `preInstallPhases`, `preFixupPhases`,
`preDistPhases`, `postPhases`.

### Common attributes

| Attribute | Purpose | Default |
| --- | --- | --- |
| `pname` | Package name (preferred over bare `name` per RFC 0035). | — |
| `version` | Package version (preferred over bare `name` per RFC 0035). | — |
| `name` | Full derivation name. Auto-derived as `"${pname}-${version}"` when `pname`+`version` are set. | — |
| `src` | Single source file or directory to unpack. | — |
| `srcs` | List of source files. Requires `sourceRoot` or `setSourceRoot`. | — |
| `sourceRoot` | Directory to enter after unpacking. | auto-detected (single dir) |
| `setSourceRoot` | Shell command that sets `sourceRoot` after unpack. | — |
| `buildInputs` | Runtime libraries. From [3]: "This attribute ensures that the `bin` subdirectories of these packages appear in the `PATH` environment variable during the build, that their `include` subdirectories are searched by the C compiler, and so on." | `[]` |
| `nativeBuildInputs` | Build-time tools on `$PATH` (cmake, pkg-config, makeWrapper, setup hooks). From [5]: "Add dependencies to `nativeBuildInputs` if they are executed during the build." | `[]` |
| `propagatedBuildInputs` | Runtime libraries made available to downstream dependents. From [5]: "Propagated dependencies are made available to all downstream dependencies." | `[]` |
| `propagatedNativeBuildInputs` | Build-time tools propagated to downstream dependents. | `[]` |
| `configureFlags` | List of strings passed to the configure script. | `[]` |
| `configureFlagsArray` | Bash array of configure args (use when args contain spaces). | — |
| `makeFlags` | List of strings passed to `make` (build, install, check). | `[]` |
| `makeFlagsArray` | Bash array of make args (set in shell code, not as a derivation attr). | — |
| `buildFlags` | Make flags for the build phase only. | `[]` |
| `installFlags` | Make flags for the install phase only. | `[]` |
| `checkFlags` | Make flags for the check phase only. | `[]` |
| `patches` | List of patches applied by `patch` in `patchPhase`. | `[]` |
| `patchFlags` | Flags passed to `patch`. | `[-p1]` |
| `phases` | Override the phase list. From [4]: "It is discouraged to set this variable, as it is easy to miss some important functionality hidden in some of the less obviously needed phases (like `fixupPhase` which patches the shebang of scripts)." | (see default above) |
| `dontUnpack` | Skip the unpack phase. | `false` |
| `dontPatch` | Skip the patch phase. | `false` |
| `dontConfigure` | Skip the configure phase. | `false` |
| `dontBuild` | Skip the build phase. | `false` |
| `dontInstall` | Skip the install phase. | `false` |
| `dontFixup` | Skip the fixup phase. | `false` |
| `doCheck` | Run the check phase. | `false` |
| `doInstallCheck` | Run the installCheck phase. | `false` |
| `doDist` | Run the dist phase. | `false` |
| `strictDeps` | Disable lenient dependency placement (enforce correct native/host split). | `false` |
| `outputs` | Multi-output list (e.g. `["out" "dev" "doc"]`). | `["out"]` |
| `separateDebugInfo` | Auto-enable a `debug` output with separated debug symbols. | `false` |
| `meta` | Meta-attributes: `description`, `mainProgram`, `platforms`, `license`, `homepage`, etc. | `{}` |

### src handling: source fetchers

Source fetchers are fixed-output derivations that download and verify upstream
source. The common ones:

- **`fetchurl`** — downloads a single URL. Requires `url` + `hash`.
- **`fetchzip`** — downloads and unpacks an archive. From [1]: "`fetchzip`
  can fetch more archives than just zip files!"
- **`fetchFromGitHub`** — fetches a GitHub repo. Requires `owner`, `repo`,
  `rev`, and `hash`/`sha256`.
- **`builtins.path`** — for local source with explicit filtering. Per the
  purity doc [7]: "Use `builtins.path { name = ...; path = ./subdir;
  filter = ...; }` with an explicit `name` and `filter`".
- **`cleanSourceWith` / `lib.cleanSource`** — filtered local source. Must
  always pass a `filter` predicate that excludes `target/`, `result*`,
  `node_modules/`, build dirs.

#### Hash discovery workflow

When the correct hash is unknown, set `hash = ""` or `hash = lib.fakeSha256`,
build, and read the `got:` hash from the error. From [1]:

```console
$ nix-build -A hello
error: hash mismatch in fixed-output derivation '/nix/store/pd2kiyfa0c06giparlhd1k31bvllypbb-source.drv':
         specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
            got:    sha256-1kJjhtlsAkpNB7f6tZEs+dbKd8z7KoNHyDHEJ0tmhnc=
error: 1 dependencies of derivation '/nix/store/b4mjwlv73nmiqgkdabsdjc4zq9gnma1l-hello-2.12.1.drv' failed to build
```

Inline the `got:` value into the derivation and rebuild.

### Fixed-output derivations (FODs)

A FOD declares that its output is verified by a content hash, not by the
build steps. This is what permits network access during the fetch phase: the
sandbox cannot restrict network for a FOD because the output hash is the
purity guarantee.

FOD attributes:

| Attribute | Purpose |
| --- | --- |
| `outputHash` | The expected content hash (SRI form preferred: `sha256-...`). |
| `outputHashAlgo` | Hash algorithm: `sha256`, `sha512`, etc. |
| `outputHashMode` | `flat` (hash of the single output file) or `recursive` (hash of the NAR serialization of the output path). |

All fetchers (`fetchurl`, `fetchzip`, `fetchFromGitHub`, `fetchPypi`, etc.)
are FODs under the hood. The hash mismatch error (shown above) is the primary
debugging signal when a FOD hash is wrong or missing.

Language-specific dependency hashes are FOD-backed:

- `buildNpmPackage` → `npmDepsHash`
- `buildRustPackage` → `cargoHash` / `cargoLock`
- `buildBunPackage` → `bunDeps.outputHash`
- `buildPythonApplication` (pip) → `pipDeps.outputHash`

See [Examples](#example-4-fod-template-from-purity-doc) for the FOD template.

### Sandbox: no network in buildPhase/installPhase

The build sandbox has no network in `buildPhase`/`installPhase`. All
dependencies must arrive via FODs with declared hashes. From the purity doc
[7]:

> The sandbox has no network in `buildPhase`/`installPhase`; all dependencies
> must arrive via fixed-output derivations (FODs) with declared
> `outputHash`es; and `--impure` is forbidden everywhere.

See `/docs/nix-purity.md` for the full purity rules and the store-growth
model.

### Multi-output derivations

`outputs = [ "out" "dev" "doc" ]` (or `lib`, `dev`, `doc`) splits a derivation
into separately-installable store paths. From [2]:

> Imagine a library package that provides a dynamic library, header files,
> and documentation. A program that links against such a library doesn't
> need the header files and documentation at runtime, and it doesn't need
> the documentation at build time.

Each output name becomes an environment variable in the builder pointing to
its store path. The first output in the list is the default output
(top-level attribute). `separateDebugInfo = true` auto-enables a `debug`
output without listing it in `outputs` explicitly.

## Practical rules

1. Always use `pname` + `version` over bare `name` (RFC 0035 [6]).
2. Always wrap custom phases with `runHook preXxx` ... `runHook postXxx` [4].
3. Put build-time tools (cmake, pkg-config, makeWrapper) in
   `nativeBuildInputs`; put runtime libraries in `buildInputs` [5].
4. Use `strictDeps = true` for any package that may be cross-compiled [5].
5. All source fetching goes through FODs with declared hashes — never
   `fetchTarball` without a hash, never network in `buildPhase` [7].
6. Use `builtins.path` with explicit `name` + `filter` for local sources —
   never bare `src = ./.` [7].
7. Discover unknown hashes by setting `hash = lib.fakeSha256`, building, and
   reading the `got:` value from the error [1].
8. Prefer `preXxx`/`postXxx` hooks or `*Phases` variables over overriding the
   `phases` attribute directly [4].
9. Set `doCheck = true` when the package has a test suite (unless
   cross-compiling) [4].
10. Use `''`-style multi-line strings for phase scripts (no escaping of `"`
    or `\`) [3].

## Review checklist

- [ ] Uses `pname` + `version`, not bare `name`.
- [ ] `src` is a FOD fetcher (`fetchurl`/`fetchzip`/`fetchFromGitHub`) or a
      filtered `builtins.path`/`cleanSourceWith` — never bare `./.`.
- [ ] Every hash is a real SRI string, not `lib.fakeSha256` or `""`.
- [ ] Build-time tools are in `nativeBuildInputs`; runtime libraries are in
      `buildInputs`.
- [ ] `strictDeps = true` is set if the package may be cross-compiled.
- [ ] Custom phases start with `runHook preXxx` and end with
      `runHook postXxx`.
- [ ] No network access in `buildPhase`/`installPhase` (all deps via FODs).
- [ ] No `--impure` anywhere in the derivation or wrapping scripts.
- [ ] `doCheck = true` is set if the package has a test suite (and not
      cross-compiling).
- [ ] `phases` attribute is not overridden (use hooks / `*Phases` instead).
- [ ] `meta` has at least `description` and `mainProgram` (for executables).
- [ ] No `../` parent-directory path literals outside bounded escape-hatch
      fields (`src =`, `lockFile =`, `path =`).
- [ ] `wrapProgram` is called once per binary (no double-wrapping).
- [ ] Phase scripts use `''`-style multi-line strings.

## Implementation checklist

Drawn from the purity doc's "How to add a new derivation" checklist [7]:

1. **Decide the source filter.** Use `builtins.path` with an explicit `name`
   and `filter`, or consume a flake input. Never `src = ./.`.
2. **Decide the dependency strategy.** All deps go through a FOD with
   `outputHash` / `npmDepsHash` / `cargoHash`. No `fetchTarball` without a
   hash.
3. **Confirm no network in build/install.** Any data the build needs is
   committed or fetched via a FOD before the build runs.
4. **Confirm no `--impure`.** Not in the derivation, not in a wrapping
   script, not in the justfile.
5. **Run `just lint-nix`.** The guard must pass.
6. **Build with `nix build .#<name>`.** Confirm it succeeds.
7. **If a hash is unknown**, use `fakeHash` / `lib.fakeSha256` as a
   placeholder, then run the `update-hashes` recipe to prefetch the real
   hash.
8. **Add a HOST-GATE note** if the build can only be verified on a host with
   nix. This container has no nix; any `nix build` claim here is based on
   documented Nix semantics, not runtime verification.

## Validation hooks

> **HOST-GATE:** This container has no nix. The commands below are documented
> Nix semantics, not runtime-verified in this environment.

Build a derivation:

```bash
nix build .#<name>              # build, creates ./result symlink
nix build .#<name> --no-link    # build without creating ./result
nix-build -A <attr>             # legacy build (creates ./result)
```

View build logs:

```bash
nix log $(nix path-info .#<name>)
nix log /nix/store/<hash>-<name>.drv
```

Prefetch source hashes:

```bash
nix-prefetch-url --unpack <url> --type sha256
nix run nixpkgs#prefetch-npm-deps -- <package-lock.json>
```

Interactive phase debugging in a dev shell:

```bash
just shell                           # enter dev shell
cd "$(mktemp -d)"
export out=$(pwd)/out
phases="unpackPhase patchPhase" genericBuild   # run early phases
phases="buildPhase" genericBuild               # re-run a single phase
echo "$buildPhase"                             # inspect a phase function
type buildPhase                               # or its type if empty
```

Repo-specific hygiene:

```bash
just lint-nix        # static guard: no --impure, no unfiltered paths, no ../
just update-hashes   # prefetch npmDepsHash / bunDeps / pipDeps hashes
just gc              # nix-collect-garbage --delete-old + nix store optimise
just store-audit     # top-20 store paths by closure size + stale source roots
```

## Examples

### Example 1: Minimal hello package (from crawl 14)

The canonical first package from the nix.dev tutorial [1]:

```nix
# hello.nix
{
  stdenv,
  fetchzip,
}:

stdenv.mkDerivation {
  pname = "hello";
  version = "2.12.1";

  src = fetchzip {
    url = "https://ftp.gnu.org/gnu/hello/hello-2.12.1.tar.gz";
    sha256 = "sha256-1kJjhtlsAkpNB7f6tZEs+dbKd8z7KoNHyDHEJ0tmhnc=";
  };
}
```

The `icat` example with `fetchFromGitHub`, `buildInputs`, and a custom
`installPhase` with hooks [1]:

```nix
# icat.nix
{
  stdenv,
  fetchFromGitHub,
  imlib2,
  xorg,
}:

stdenv.mkDerivation {
  pname = "icat";
  version = "v0.5";

  src = fetchFromGitHub {
    owner = "atextor";
    repo = "icat";
    rev = "v0.5";
    sha256 = "0wyy2ksxp95vnh71ybj1bbmqd5ggp13x3mk37pzr99ljs9awy8ka";
  };

  buildInputs = [ imlib2 xorg.libX11 ];

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp icat $out/bin
    runHook postInstall
  '';
}
```

### Example 2: agentctl.nix (from the repo)

Key parts of `/nix/packages/agentctl.nix` [8] — a `buildRustPackage` with a
filtered `cleanSourceWith` src, `nativeBuildInputs`/`buildInputs` split, a
`preBuild` hook that stages a vendored crate + `MSB_HOME`, a `postInstall`
hook that wraps the binary, and `doCheck = false`:

```nix
{ pkgs
, microsandbox
, microsandbox-filesystem-patched
, rustToolchain
}:

let
  # `src` is filtered to keep the nix build hermetic: no in-tree build
  # artifacts, no crash dumps, no locally-managed result symlinks, no stale
  # vendor directory (the preBuild hook recreates `vendor/` as a symlink to
  # the nix-managed patched crate, so any source-tree vendor/ is unused).
  src = pkgs.lib.cleanSourceWith {
    filter = path: type:
      let base = baseNameOf path; in
      !(base == "target"
        || base == "result" || base == "result-" || base == "result-man"
        || base == "core" || pkgs.lib.hasPrefix "core." base
        || base == "vendor"
        || (type == "regular" && base == "config.toml"
            && pkgs.lib.hasSuffix "/.cargo/config.toml" path));
    src = ../../control/agentctl;
  };

  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain.rustc;
    cargo = rustToolchain.cargo;
  };
in
(rustPlatform.buildRustPackage {
  pname = "workestrate";
  version = "0.1.0";

  inherit src;

  cargoLock = {
    lockFile = ../../control/agentctl/Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
  ];

  buildInputs = with pkgs; [
    libcap_ng
  ];

  preBuild = ''
    mkdir -p vendor
    ln -sfn "${microsandbox-filesystem-patched}" vendor/microsandbox-filesystem-0.5.6
    cat > .cargo/config.toml <<'CARGO_CONFIG'
    [patch.crates-io]
    microsandbox-filesystem = { path = "vendor/microsandbox-filesystem-0.5.6" }
    CARGO_CONFIG

    export MSB_HOME=$TMPDIR/.microsandbox
    mkdir -p $MSB_HOME/bin $MSB_HOME/lib
    cp ${microsandbox}/bin/msb $MSB_HOME/bin/msb
    cp ${microsandbox}/libexec/agentd $MSB_HOME/bin/agentd
    chmod +x $MSB_HOME/bin/agentd

    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -f "$f" ] || [ -L "$f" ]; then
        cp -P "$f" $MSB_HOME/lib/
      fi
    done
  '';

  postInstall = ''
    wrapProgram $out/bin/workestrate \
      --set MSB_PATH "${microsandbox}/bin/msb" \
      --prefix PATH : ${pkgs.sops}/bin \
      --run 'export MSB_HOME="$HOME/.microsandbox"'
  '';

  doCheck = false;

  meta = {
    description = "Control plane CLI for the AI workbench";
    mainProgram = "workestrate";
  };
}
)
```

### Example 3: pi.nix (from the repo)

Key parts of `/nix/packages/pi.nix` [9] — a `buildNpmPackage` with
`npmDepsHash` (FOD), `dontNpmBuild`, a custom `buildPhase` chaining four
workspace builds, a custom app-style `installPhase`, `autoPatchelfHook`, and
`dontStrip`:

```nix
{ pi
, npmDepsHash
, buildNpmPackage
, nodejs_24
, autoPatchelfHook
, stdenv
, libcap_ng
, lib
}:

buildNpmPackage {
  pname = "pi";
  version = "0.79.10";

  src = pi;

  # Computed via `nix run nixpkgs#prefetch-npm-deps -- <src>/package-lock.json`.
  npmDepsHash = npmDepsHash;

  # Skip the default `npm run build` (which chains generate-models +
  # generate-image-models + tsgo). The generate scripts delete committed
  # catalogs and can't regenerate them offline. See header comment.
  dontNpmBuild = true;

  # Skip lifecycle scripts (husky prepare, canvas node-gyp). See header comment.
  npmFlags = [ "--ignore-scripts" ];

  nodejs = nodejs_24;

  # tsgo (@typescript/native-preview) and esbuild ship prebuilt native ELF
  # binaries inside node_modules. autoPatchelfHook patches their interpreter /
  # RPATH so they run in the sandbox. stdenv.cc provides libstdc++.
  nativeBuildInputs = [ autoPatchelfHook ];
  # libcap-ng: gondolin's libkrun needs libcap-ng.so.0 at runtime;
  # autoPatchelfHook wires the RPATH.
  buildInputs = [ stdenv.cc.cc.lib libcap_ng ];

  # Build the four workspaces in dependency order, skipping the ai workspace's
  # generate-models/generate-image-models (offline; committed catalogs used).
  buildPhase = ''
    runHook preBuild
    cd packages/tui && npm run build && cd ../..
    cd packages/ai && ../../node_modules/.bin/tsgo -p tsconfig.build.json && cd ../..
    cd packages/agent && npm run build && cd ../..
    cd packages/coding-agent && npm run build && cd ../..
    runHook postBuild
  '';

  # App-style output: reproduce the monorepo runtime tree at $out so that
  # `node $out/packages/coding-agent/dist/cli.js` resolves workspace siblings
  # (../ai, ../agent, ../tui) and external deps from $out/node_modules.
  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r packages $out/packages
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "pi coding agent (hermetic nix build of the monorepo runtime tree)";
    mainProgram = "pi";
    platforms = platforms.linux;
  };
}
```

### Example 4: FOD template (from purity doc)

The `my-fod` FOD template from the purity doc [7], showing
`outputHashAlgo`/`outputHashMode`/`outputHash`:

```nix
{ lib, stdenv, fetchFromGitHub }:

stdenv.mkDerivation {
  pname = "my-fod";
  version = "0.1.0";

  # Source comes from a flake input or a hashed fetch — never ./.
  src = fetchFromGitHub {
    owner = "example";
    repo = "my-fod";
    rev = "v0.1.0";
    # hash = lib.fakeSha256; # TODO: replace via just update-hashes
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  # Fixed-output declaration — the output is verified by hash, not by
  # the build steps. This is what permits network in the fetch phase
  # (the fetcher runs under a FOD, not the main build).
  outputHashAlgo = "sha256";
  outputHashMode = "recursive";
  # outputHash = lib.fakeSha256; # TODO: replace via just update-hashes
  outputHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";

  # No network here. All deps arrived via the FOD fetch above.
  buildPhase = ''
    runHook preBuild
    make build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r dist/* $out/
    runHook postInstall
  '';

  # HOST-GATE: hash verified on host via just update-hashes
  # (this container has no nix; the hash above is a placeholder).
}
```

## Common mistakes

### 1. Bare `src = ./.` without filtering

Copies the entire working tree (including `target/`, `node_modules/`,
`result*`) into the store on every eval. This is the root cause of the 29
GB-per-eval incident documented in the purity doc [7].

```nix
# WRONG:
src = ./.;

# CORRECT:
src = builtins.path {
  name = "my-package-src";
  path = ./my-package;
  filter = path: type: baseNameOf path != "target";
};
```

### 2. Missing `runHook preInstall`/`postInstall` in a custom installPhase

Downstream `postInstall` overrides silently stop running.

```nix
# WRONG:
installPhase = ''
  mkdir -p $out/bin
  cp foo $out/bin
'';

# CORRECT:
installPhase = ''
  runHook preInstall
  mkdir -p $out/bin
  cp foo $out/bin
  runHook postInstall
'';
```

### 3. Putting a build tool in `buildInputs` instead of `nativeBuildInputs`

Works natively, breaks cross-compilation (the tool won't be on `$PATH`
during the build on the foreign host).

```nix
# WRONG:
buildInputs = [ cmake pkg-config ];

# CORRECT:
nativeBuildInputs = [ cmake pkg-config ];
buildInputs = [ zlib ];
```

### 4. `fetchTarball` without a hash

Impure and non-reproducible — the tarball content can change upstream.

```nix
# WRONG:
nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";

# CORRECT:
nixpkgs = fetchTarball {
  url = "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
};
```

### 5. Network access in buildPhase

Fails in the sandbox. All deps must arrive via FODs.

```nix
# WRONG:
buildPhase = ''
  curl -o data.json https://example.com/data.json
  make build
'';

# CORRECT:
# Fetch data via a FOD, then reference it in buildPhase.
dataJson = fetchurl {
  url = "https://example.com/data.json";
  hash = "sha256-...";
};
# ...
buildPhase = ''
  cp ${dataJson} data.json
  make build
'';
```

### 6. Overriding `phases` instead of using hooks

Loses `fixupPhase` shebang patching and other less-obvious default behavior
[4].

```nix
# WRONG:
phases = [ "unpackPhase" "buildPhase" "installPhase" ];

# CORRECT:
# Use dontConfigure = true / dontBuild = true / hooks instead.
dontConfigure = true;
preInstall = "...";
```

### 7. Forgetting `doCheck = true` when tests exist

The check phase is skipped by default [4].

```nix
# WRONG (tests silently skipped):
{ ... }

# CORRECT:
{
  doCheck = true;
  # ...
}
```

### 8. Hardcoding a hash instead of using the prefetch workflow

Guessing a hash by hand never works; use `lib.fakeSha256` + the `got:` error.

```nix
# WRONG:
hash = "sha256-0000000000000000000000000000000000000000000000=";

# CORRECT:
hash = lib.fakeSha256;  # then build, read got:, inline
```

### 9. Not setting `strictDeps = true`

Works natively, breaks cross-compilation silently [5].

```nix
# WRONG (for a cross-compilable package):
{ ... }

# CORRECT:
{
  strictDeps = true;
  # ...
}
```

### 10. Double-wrapping with `wrapProgram`

Calling `wrapProgram` twice on the same binary overwrites the first wrapper
[4].

```nix
# WRONG:
postInstall = ''
  wrapProgram $out/bin/foo --set FOO bar
  wrapProgram $out/bin/foo --set BAZ qux   # overwrites the FOO wrapper
'';

# CORRECT:
postInstall = ''
  wrapProgram $out/bin/foo \
    --set FOO bar \
    --set BAZ qux
'';
```

### 11. Using `name` instead of `pname`+`version`

Discouraged by RFC 0035 [6]; prevents version reuse in `finalAttrs`.

```nix
# WRONG:
name = "foo-1.2.3";

# CORRECT:
pname = "foo";
version = "1.2.3";
```

### 12. Referencing `../` parent paths in nix assignments

Trips the `just lint-nix` guard (`check-nix-paths.sh`) outside bounded
escape-hatch fields (`src =`, `lockFile =`, `path =`) [7].

```nix
# WRONG (outside escape-hatch fields):
extraConfig = ../config/foo.toml;

# CORRECT (inside an escape-hatch field, or use a flake input):
src = ../../control/agentctl;  # allowed: src is an escape-hatch field
```

## Strict vs contextual guidance

| Rule | Strict (always) | Contextual (depends) |
| --- | --- | --- |
| `pname` + `version` over `name` | Strict (RFC 0035) | — |
| `runHook preXxx`/`postXxx` in custom phases | Strict | — |
| FOD for all source fetching | Strict | — |
| `builtins.path` with `filter` for local `src` | Strict | — |
| `cleanSourceWith` with `filter` predicate | Strict (per purity rules [7]) | — |
| No network in `buildPhase`/`installPhase` | Strict | — |
| No `--impure` | Strict | — |
| `strictDeps = true` | — | Contextual (strict for cross-compilable packages, optional for native-only) |
| `doCheck = true` | — | Contextual (strict if tests exist and aren't broken; skipped under cross-compilation) |
| `phases` override | — | Contextual (discouraged but sometimes needed) |
| `dontStrip = true` | — | Contextual (needed when strip breaks prebuilt native binaries, e.g. pi.nix [9]) |

## Policy decisions for individual repos

The `ai-workbench` repo applies the general rules above via the following
repo-specific mechanisms:

- **`just lint-nix` guard** (`scripts/check-nix-paths.sh`): a bash `case`-pattern
  static guard wired into `just verify`. It scans `*.nix` under
  `flake.nix`/`nix`/`templates` plus `*.sh` under `scripts/` and the
  `justfile`. It fails the gate on: `nix ... --impure`, `builtins.getFlake`
  combined with `toString`, `builtins.path { ... }` without a `filter =` in
  the following 15 lines, `cleanSourceWith { ... }` without a `filter =` in
  the following 15 lines, and `../` parent-directory path literals outside
  bounded escape-hatch fields. Allowlist: lines containing `# allow: <reason>`
  are skipped. The guard is a static heuristic, not a full eval-purity prover
  [7].
- **`just update-hashes` recipe**: prefetches `npmDepsHash` (tempest),
  `bunDeps.outputHash` (opencode), and `pipDeps.outputHash` (odysseus). It
  prints the `got:` hashes; the operator manually inlines each into the
  matching `nix/packages/*.nix` file. It does not auto-rewrite the files [7].
- **`just gc` and `just store-audit` hygiene cadence**: `just gc` runs
  `nix-collect-garbage --delete-old` + `nix store optimise` (dedupe). `just
  store-audit` reports the top-20 store paths by closure size and flags any
  `*-source` paths referencing `ai-workbench` (indicating an unbounded
  source copy that should be bounded by a `cleanSourceWith` filter) [7].
- **HOST-GATE convention**: This container has no nix; all `nix build`
  claims are based on documented Nix semantics, not runtime verification.
  Derivations that can only be verified on a host with nix carry a
  `# HOST-GATE:` comment.
- **Authoritative purity reference**: `/docs/nix-purity.md` [7] is the
  canonical reference for eval-time and build-time purity rules, the
  store-growth model, and the 29 GB-per-eval incident.

## Related docs

- `/docs/nix-purity.md` — Writing and maintaining pure derivations (the
  authoritative purity reference for this repo)
- `/docs/nix/source-map.md` — Nix source map (provenance index)
- `/docs/rust/cargo-dependencies.md` — Cargo manifests (relevant to
  `buildRustPackage` and `cargoLock`/`cargoHash`)

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference
- `.agents/skills/nix-docker-images` — Docker image building
  (`streamLayeredImage` vs `buildLayeredImage` vs `buildImage`)

## Citations

1. [Packaging existing software with Nix](https://nix.dev/tutorials/packaging-existing-software.html)
2. [Nix Language — Derivations](https://nix.dev/manual/nix/2.34/language/derivations)
3. [nixpkgs — stdenv and mkDerivation](https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv)
4. [Nixpkgs Manual — Phases](https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases)
5. [Nixpkgs Manual — Specifying dependencies](https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview)
6. [RFC 0035 — pname and version](https://github.com/NixOS/rfcs/pull/35)
7. Local: `/docs/nix-purity.md` — workestrator purity rules
8. Local: `/nix/packages/agentctl.nix` — agentctl derivation
9. Local: `/nix/packages/pi.nix` — pi agent derivation
