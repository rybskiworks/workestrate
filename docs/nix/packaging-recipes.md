---
type: Reference
resource: https://nix.dev/tutorials/packaging-existing-software.html
title: Packaging Recipes
description: Language-specific Nix packaging recipes — buildNpmPackage, buildPythonApplication, buildGoModule, bun build --compile, pip install --target, callPackage dependency injection, override/overrideAttrs, and the project's custom recipe system.
tags: [nix, packaging, buildNpmPackage, buildPythonApplication, bun, recipes]
timestamp: 2026-07-24T01:30:00Z
---

# Packaging Recipes

## Purpose

Provide concrete, repo-independent guidance for packaging language-specific
software with Nix: the `callPackage` dependency-injection pattern, the
nixpkgs language build helpers (`buildNpmPackage`, `buildPythonApplication`,
`buildGoModule`), standalone-binary recipes (`bun build --compile`), Python
dependency vendoring (`pip install --target`), fixed-output derivation (FOD)
dependency hashing, package overriding (`override`, `overrideAttrs`,
`overrideDerivation`), scoped package sets (`callPackageWith`), and the
project's custom recipe system under `nix/lib/recipes.nix`. This document is
intended as reference material for AI coding agents working in any Nix flake,
with real-world examples drawn from the `ai-workbench` repository's
`nix/packages/*.nix` and `nix/lib/recipes/*.nix` files.

Agents should use this document as the authoritative reference when packaging
a Node/Python/Go/Bun application, computing a dependency hash, overriding an
existing package, or wiring a custom recipe into the project's recipe
aggregator.

## Sources used

- <https://nix.dev/tutorials/packaging-existing-software.html>
- <https://nix.dev/tutorials/callpackage.html>
- <https://nix.dev/tutorials/working-with-local-files.html>
- <https://nixos.org/manual/nixpkgs/stable/#sec-language-stdenv>
- <https://nixos.org/manual/nixpkgs/stable/#javascript-buildNpmPackage>
- <https://nixos.org/manual/nixpkgs/stable/#buildpythonapplication-function>
- <https://nixos.org/manual/nixpkgs/stable/#buildgomodule>
- <https://nixos.org/manual/nixpkgs/stable/#function-library-lib.customisation.callPackageWith>
- <https://nixos.org/manual/nixpkgs/stable/#chap-overrides>
- Local: `/nix/lib/recipes.nix` — recipe aggregator
- Local: `/nix/lib/recipes/npm-build.nix` — npm recipe
- Local: `/nix/lib/recipes/bun-compile.nix` — bun compile recipe
- Local: `/nix/lib/recipes/pip-install.nix` — pip install recipe
- Local: `/nix/lib/recipes/bun-install.nix` — bun install recipe
- Local: `/nix/packages/pi.nix` — pi buildNpmPackage
- Local: `/nix/packages/tempest.nix` — tempest buildNpmPackage
- Local: `/nix/packages/odysseus.nix` — odysseus pip FOD build

### Crawl ledger

SEED:

- <https://nix.dev/tutorials/packaging-existing-software.html>
- <https://nix.dev/tutorials/callpackage.html>
- <https://nix.dev/tutorials/working-with-local-files.html>
- <https://nixos.org/manual/nixpkgs/stable/#sec-language-stdenv>

DISCOVERED & VISITED:

- <https://nixos.org/manual/nixpkgs/stable/#javascript-buildNpmPackage>
- <https://nixos.org/manual/nixpkgs/stable/#buildpythonapplication-function>
- <https://nixos.org/manual/nixpkgs/stable/#buildgomodule>
- <https://nixos.org/manual/nixpkgs/stable/#function-library-lib.customisation.callPackageWith>
- <https://nixos.org/manual/nixpkgs/stable/#chap-overrides>

SKIPPED (out of scope / tangential):

- <https://nixos.org/manual/nixpkgs/stable/#buildrustpackage> — Rust
  packaging (covered by `/docs/rust/cargo-dependencies.md`)
- <https://nixos.org/manual/nixpkgs/stable/#javascript-buildPnpmPackage> —
  pnpm packaging (not used in this repo)
- <https://nixos.org/manual/nixpkgs/stable/#python> — full Python
  interpreter override machinery (referenced at summary level only)

## Core guidance

### `callPackage`: the dependency-injection pattern

Every Nixpkgs package recipe is a function that takes an attribute set of
dependencies and returns a derivation. `callPackage` is the convention for
invoking these functions with automatic argument resolution. From the
callpackage tutorial [2]:

> "`callPackage` automatically passes attributes from `pkgs` to the given
> function, if they match attributes required by that function's argument
> attribute set. In this case, `callPackage` will supply `stdenv` and
> `fetchzip` to the function defined in `hello.nix`." [2]

The canonical pattern:

```nix
# default.nix
let
  pkgs = import <nixpkgs> { };
in
{
  hello = pkgs.callPackage ./hello.nix { };
}
```

Arguments not found in `pkgs` must be supplied explicitly in the second
argument to `callPackage`. Default-valued arguments (using `?`) are optional
and can be overridden the same way [2]:

```nix
{
  writeShellScriptBin,
  audience ? "world",
}:
writeShellScriptBin "hello" ''
  echo "Hello, ${audience}!"
''
```

```nix
hello = pkgs.callPackage ./hello.nix { audience = "people"; };
```

### `buildNpmPackage`

`buildNpmPackage` packages npm-based projects without an auto-generated
dependencies file. From the nixpkgs build-helpers reference [4]:

> "`buildNpmPackage` allows you to package npm-based projects in Nixpkgs
> without the use of an auto-generated dependencies file. It works by
> utilizing npm's cache functionality – creating a reproducible cache that
> contains the dependencies of a project, and pointing npm to it." [4]

Key arguments [4]:

| Argument | Purpose |
| --- | --- |
| `npmDepsHash` | Output hash of the dependencies. Computed with `prefetch-npm-deps`. |
| `npmBuildScript` | Script to run to build the project. Defaults to `"build"`. |
| `dontNpmBuild` | Disable running the build script. Defaults to `false`. |
| `npmFlags` | Flags to pass to all npm commands. |
| `npmPackFlags` | Flags to pass to `npm pack`. |
| `nodejs` | The `nodejs` package to build against. Defaults to `pkgs.nodejs`. |
| `npmWorkspace` | The workspace directory within the project to build and install. |

The default `installPhase` uses `npm pack --json --dry-run` to decide what
files to install in `$out/lib/node_modules/$name/` [4]. For applications
(not libraries), override `installPhase` to reproduce the runtime tree at
`$out`.

#### FOD dependency hashing for npm

`buildNpmPackage` internally splits the build into two stages: (1) a
fixed-output derivation (`fetchNpmDeps`) that runs `npm ci` against
`package-lock.json` with a hash-bounded output (`npmDepsHash`), and (2) the
main derivation that runs `npm run build` against the cached `node_modules`
offline [4]. The `npmDepsHash` is computed with `prefetch-npm-deps`:

```console
$ prefetch-npm-deps package-lock.json
sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
```

### `buildPythonApplication`

`buildPythonApplication` builds Python applications where only the
executables (not importable modules) are of interest. From the nixpkgs
build-helpers reference [4]:

> "The `buildPythonApplication` function is practically the same as
> `buildPythonPackage`. The main purpose of this function is to build a
> Python package where one is interested only in the executables, and not
> importable modules." [4]

It should be called with `callPackage` and passed `python3` or
`python3Packages` [4]:

```nix
{
  lib,
  python3Packages,
  fetchPypi,
}:

python3Packages.buildPythonApplication (finalAttrs: {
  pname = "luigi";
  version = "2.7.9";
  pyproject = true;

  src = fetchPypi {
    inherit (finalAttrs) pname version;
    hash = "sha256-...";
  };

  build-system = with python3Packages; [ setuptools ];
  # ...
})
```

Key parameters [4]:

| Parameter | Purpose |
| --- | --- |
| `pyproject` | Whether the pyproject format should be used. Recommended `true`. |
| `format` | Build format (set indirectly via `pyproject`). |
| `nativeBuildInputs` | Build-time only dependencies (typically executables). |
| `build-system` | Build-time only Python dependencies (`build-system.requires`). |
| `dependencies` | Runtime dependencies (`install_requires`). Propagated and wrapped into executables. |
| `propagatedBuildInputs` | Dependencies propagated to downstream consumers. |
| `nativeCheckInputs` | Test dependencies (added to `nativeBuildInputs` when `doCheck = true`). |

The `buildPythonPackage`/`buildPythonApplication` functions support
fixed-point arguments (`finalAttrs`) and `overridePythonAttrs` for
overriding [4].

### `buildGoModule`

`buildGoModule` builds Go programs managed with Go modules through a
two-phase build [4]:

> "The function `buildGoModule` builds Go programs managed with Go modules.
> It builds Go Modules through a two phase build: An intermediate fetcher
> derivation called `goModules`. This derivation will be used to fetch all
> the dependencies of the Go module. A final derivation will use the output
> of the intermediate derivation to build the binaries and produce the final
> output." [4]

```nix
{
  pet = buildGoModule (finalAttrs: {
    pname = "pet";
    version = "0.3.4";

    src = fetchFromGitHub {
      owner = "knqyf263";
      repo = "pet";
      tag = "v${finalAttrs.version}";
      hash = "sha256-...";
    };

    vendorHash = "sha256-ciBIR+a1oaYH+H1PcC8cD8ncfJczk1IiJ8iYNM+R6aA=";

    meta = { /* ... */ };
  });
}
```

Key arguments:

| Argument | Purpose |
| --- | --- |
| `vendorHash` | Output hash of the vendored Go module dependencies (FOD). Use `lib.fakeHash` for first-time computation. |
| `modBuildPhase` | Override the module-fetch build phase. |
| `modInstallPhase` | Override the module-install phase. |
| `proxyVendor` | Use `go mod download` to a proxy directory instead of `go mod vendor`. |

The `vendorHash` follows the same FOD pattern as `npmDepsHash`: set it to
`lib.fakeHash`, build once, and copy the `got:` hash from the failure
output.

### `bun build --compile`: standalone binary packaging

Bun's `--compile` flag embeds the Bun runtime into a self-contained binary.
This is wrapped as a `stdenv.mkDerivation` that runs `bun build --compile`
and strips store references to the source tree. From the project's
`nix/lib/recipes/bun-compile.nix` [10]:

```nix
buildPhase = ''
  runHook preBuild
  entry="${src}/${entrypoint}"
  wk="${src}/${worker}"
  mkdir -p $out/bin
  # --compile embeds the Bun runtime; the binary is self-contained.
  bun build --compile "$entry" "$wk" --outfile "$out/bin/app"
  runHook postBuild
'';
```

The `installPhase` strips the source-tree store reference from the compiled
binary (the binary is self-contained and does not need the source tree at
runtime) and mirrors runtime assets next to the binary [10]:

```nix
installPhase = ''
  runHook preInstall
  chmod -R +w $out/bin
  remove-references-to -t ${src} $out/bin/app
  ${mirrorAssets}
  runHook postInstall
'';
```

### `pip install --target`: Python dependency vendoring

For Python applications with requirements that include packages not in the
nixpkgs Python package set, vendoring wheels via `pip install --target` is a
pragmatic alternative to `buildPythonApplication`. The project uses a
two-stage FOD pattern [11]:

- **Stage 1 (FOD):** `pip download` fetches all wheels into a
  content-addressed directory. This is the only step with network access.
- **Stage 2 (offline):** `pip install --no-index --find-links` installs
  from the cached wheels into `./deps` with no network.

```nix
# Stage 2 — offline install
buildPhase = ''
  runHook preBuild
  export HOME=$TMPDIR
  export PIP_NO_CACHE_DIR=1
  REQ="$(cat ${pipDeps}/.requirements-marker)"
  mkdir -p .deps
  python3.12 -m pip install \
    --only-binary=:all: \
    --no-index \
    --find-links ${pipDeps} \
    --break-system-packages \
    --target ./.deps \
    -r "$REQ"
  runHook postBuild
'';
```

At runtime, `PYTHONPATH=/app/.deps` makes the vendored packages importable.

### FOD dependency hashing

Fixed-output derivations (FODs) are the only derivations permitted network
access. Their output is content-addressed by `outputHash`. The standard
workflow for computing a dependency hash:

1. Set the hash to `lib.fakeHash` (or an empty string).
2. Build once — the build fails loudly with a `specified: ... got: ...`
   mismatch.
3. Copy the `got:` value (prefixed with `sha256-`) into the derivation.

The project's `just update-hashes` recipe automates step 1–2 for all three
FOD types [12]:

```makefile
update-hashes:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== tempest: prefetch-npm-deps (buildNpmPackage internal FOD) ==="
    nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json
    echo "=== opencode-built: bunDeps FOD (first build fails with lib.fakeHash) ==="
    nix build .#opencode-built --no-link 2>&1 | grep -E 'got:|specified:|error: hash' || true
    echo "=== odysseus-built: pipDeps FOD (first build fails with lib.fakeHash) ==="
    nix build .#odysseus-built --no-link 2>&1 | grep -E 'got:|specified:|error: hash' || true
```

The operator manually inlines each `got:` hash into the matching
`nix/packages/*.nix` file [12].

### `override` and `overrideAttrs`

`callPackage` adds an `override` function to every derivation it produces,
allowing parameters to be customised after the fact [2]:

> "`callPackage` adds more convenience by allowing parameters to be
> customised _after the fact_ using the returned derivation's `override`
> function." [2]

```nix
rec {
  hello = pkgs.callPackage ./hello.nix { audience = "people"; };
  hello-folks = hello.override { audience = "folks"; };
}
```

`overrideAttrs` customises the derivation attributes (not the `callPackage`
arguments). It takes a function `prevAttrs: { ... }` that returns an
attribute set merged over the original:

```nix
myPackage = pkg.overrideAttrs (prevAttrs: {
  version = "2.0.0-custom";
  src = fetchFromGitHub { /* ... */ };
});
```

For `buildPythonPackage`/`buildPythonApplication`, use
`overridePythonAttrs` instead of `overrideAttrs` for Python-specific
arguments passed down via `passthru` (such as `format`, `pyproject`,
`build-system`, `dependencies`) [4].

### `overrideDerivation`: low-level override

`overrideDerivation` is a lower-level override that operates on the raw
derivation attribute set (the arguments to the primitive `derivation`
builtin), not on the `mkDerivation` arguments. It is discouraged for general
use — prefer `overrideAttrs` — but is occasionally needed when the
attribute to change is not exposed by `mkDerivation`. It does not re-run
`mkDerivation`'s argument processing, so `stdenv` hooks and phase logic
are not re-evaluated.

### `callPackageWith`: scoped package sets

For interdependent package sets where recipes depend on each other,
`lib.callPackageWith` creates a custom `callPackage` based on a merged
attribute set [2]:

```nix
let
  pkgs = import <nixpkgs> { };
  callPackage = pkgs.lib.callPackageWith (pkgs // packages);
  packages = {
    a = callPackage ./a.nix { };
    b = callPackage ./b.nix { };
    c = callPackage ./c.nix { };
    d = callPackage ./d.nix { };
    e = callPackage ./e.nix { };
  };
in
packages
```

From the callpackage tutorial [2]:

> "Your custom `callPackages` now makes available all the attributes in
> `pkgs` _and_ `packages` to the called package function (the same names
> from `packages` taking precedence), and `packages` is being built up
> recursively with each call." [2]

This relies on lazy evaluation — `packages` can be referenced before it is
fully defined [2].

### The project's recipe system

The project aggregates reusable build recipes in `nix/lib/recipes.nix` [9].
Each recipe is a function imported with its dependencies explicitly wired:

```nix
# nix/lib/recipes.nix
{ pkgs }:
let
  vocab = import ./vocabulary.nix { inherit pkgs; };
in {
  build = {
    npm-build = import ./recipes/npm-build.nix {
      inherit pkgs;
      inherit (pkgs) buildNpmPackage nodejs_24 autoPatchelfHook stdenv libcap_ng lib;
    };
    bun-compile = import ./recipes/bun-compile.nix {
      inherit pkgs;
      bun = pkgs.bun;
      stdenv = pkgs.stdenv;
      lib = pkgs.lib;
      removeReferencesTo = pkgs.removeReferencesTo;
    };
    pip-install = import ./recipes/pip-install.nix {
      inherit pkgs;
      python312 = pkgs.python312;
      stdenv = pkgs.stdenv;
    };
    bun-install = import ./recipes/bun-install.nix {
      inherit pkgs;
      bun = pkgs.bun;
      nodejs_24 = pkgs.nodejs_24;
      stdenv = pkgs.stdenv;
    };
  };
  image = {
    registry = import ./recipes/registry.nix;
    nix-layered = import ./recipes/nix-layered.nix { inherit pkgs vocab; };
  };
  inherit vocab;
}
```

Each recipe is a curried function: the outer function receives nixpkgs
dependencies (injected by the aggregator), and the inner function receives
per-package parameters (injected by the caller). This mirrors the
`callPackage` dependency-injection pattern but with explicit wiring so the
aggregator can control which nixpkgs revision and package versions each
recipe uses.

#### Recipe catalogue

| Recipe | File | Purpose |
| --- | --- | --- |
| `npm-build` | `nix/lib/recipes/npm-build.nix` | Wraps `buildNpmPackage` with autoPatchelf, `--ignore-scripts`, and optional custom `buildPhase`/`installPhase`. |
| `bun-compile` | `nix/lib/recipes/bun-compile.nix` | Produces a standalone Bun-compiled binary with store-reference stripping and asset mirroring. |
| `pip-install` | `nix/lib/recipes/pip-install.nix` | Installs Python deps into `.deps/` via `pip install --target`. |
| `bun-install` | `nix/lib/recipes/bun-install.nix` | Installs Bun/TypeScript deps via `bun install`. |
| `nix-layered` | `nix/lib/recipes/nix-layered.nix` | Builds a `dockerTools.buildLayeredImage`. |
| `registry` | `nix/lib/recipes/registry.nix` | No build; returns a container ref string. |

## Practical rules

1. **Use `callPackage` for every package recipe.** It gives `override` for
   free and follows Nixpkgs conventions [2].
2. **Set dependency hashes to `lib.fakeHash` initially.** The first build
   fails loudly with the correct `got:` hash. Never leave a hash as an
   empty string in committed code [11].
3. **Run `just update-hashes` on a nix-capable host** to recompute all
   FOD hashes after dependency changes. Inline each `got:` value manually
   [12].
4. **Override `installPhase` for app-style packages.** The default
   `buildNpmPackage` install puts things under `$out/lib/node_modules`
   (library style). Apps expect `dist/` + `node_modules/` +
   `package.json` at the root [5][6].
5. **Pass `--ignore-scripts` to npm.** Skips lifecycle hooks (husky
   `prepare`, canvas `node-gyp`) that fail in the offline sandbox [5][6].
6. **Use `autoPatchelfHook` for prebuilt native binaries.** tsgo/esbuild
   ship ELF binaries inside `node_modules`; `autoPatchelfHook` patches
   their interpreter/RPATH [5].
7. **Use the two-stage FOD pattern for pip.** Stage 1 (`pip download`,
   FOD, network) → Stage 2 (`pip install --no-index`, offline). Never run
   `pip install` with network in the main derivation [11].
8. **Prefer `overrideAttrs` over `overrideDerivation`.** `overrideAttrs`
   re-runs `mkDerivation` argument processing; `overrideDerivation` does
   not [4].
9. **Use `callPackageWith` for interdependent package sets.** It resolves
   dependencies automatically within a scoped attribute set [2].
10. **Strip store references from compiled binaries.** Use
    `removeReferencesTo` to strip the source-tree reference from
    Bun-compiled binaries (they are self-contained) [10].

## Review checklist

- [ ] Package recipe is a function taking an attribute set of dependencies
      (follows `callPackage` convention).
- [ ] `pname` and `version` are set (not `name`).
- [ ] Dependency hash (`npmDepsHash`, `vendorHash`, `outputHash`) is set to
      a real `sha256-...` value, not `lib.fakeHash` or empty string (in
      committed code).
- [ ] `installPhase` calls `runHook preInstall` and `runHook postInstall`.
- [ ] `buildPhase` calls `runHook preBuild` and `runHook postBuild`.
- [ ] App-style packages override `installPhase` to reproduce the runtime
      tree at `$out`.
- [ ] `--ignore-scripts` is passed to npm commands (defense-in-depth).
- [ ] `autoPatchelfHook` is in `nativeBuildInputs` when prebuilt native
      binaries are present.
- [ ] Python FOD builds use the two-stage pattern (download FOD + offline
      install).
- [ ] `override`/`overrideAttrs` is used (not `overrideDerivation`) unless
      there is a specific reason.
- [ ] `meta.platforms` is set.

## Implementation checklist

- [ ] Create the package file under `nix/packages/<name>.nix` as a
      function of dependencies.
- [ ] Wire the package into `flake.nix` via `callPackage` (or the recipe
      aggregator).
- [ ] Set the dependency hash to `lib.fakeHash`.
- [ ] Run `just update-hashes` (or `nix build .#<name>`) on a nix-capable
      host to compute the `got:` hash.
- [ ] Inline the computed hash into the package file.
- [ ] Run `nix build .#<name>` to confirm the build succeeds.
- [ ] Verify the output tree layout matches the runtime expectation
      (`$out/dist`, `$out/node_modules`, `$out/.deps`, etc.).
- [ ] Add a `# HOST-GATE:` comment if the build can only be verified on a
      host with nix.

## Validation hooks

- **`just update-hashes`**: prefetches `npmDepsHash` (tempest),
  `bunDeps.outputHash` (opencode), and `pipDeps.outputHash` (odysseus).
  Prints the `got:` hashes for manual inlining [12].
- **`nix build .#<name>`**: builds the package. With `lib.fakeHash`, the
  FOD stage fails loudly with the correct hash.
- **`nix eval .#<name>.drvPath`**: succeeds even with `lib.fakeHash` (drv
  instantiation does not require the FOD output hash to be correct — only
  the build does) [5][6][11].
- **`nix build .#<name> --no-link`**: builds without creating a `result`
  symlink.
- **`just lint-nix`**: static guard for eval-purity issues (no `--impure`,
  no unfiltered `builtins.path`, no `../` outside escape-hatch fields)
  [12].

## Examples

### Example 1: `buildNpmPackage` — tempest (single-package app)

From `nix/packages/tempest.nix` [6]:

```nix
{ tempest, buildNpmPackage, nodejs_24, lib }:

buildNpmPackage {
  pname = "t3mp3st";
  version = "1.0.0";

  src = tempest;

  # HOST-GATE: replace lib.fakeHash with the output of:
  #   nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json
  npmDepsHash = lib.fakeHash;

  npmFlags = [ "--ignore-scripts" ];
  nodejs = nodejs_24;

  # App-style output: reproduce the runtime tree at $out so that
  # `node $out/dist/cli.js` resolves external deps from $out/node_modules.
  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r dist $out/dist
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json
    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "T3MP3ST offensive-security multi-agent framework";
    mainProgram = "t3mp3st";
    platforms = platforms.linux;
  };
}
```

### Example 2: `buildNpmPackage` — pi (npm-workspaces monorepo)

From `nix/packages/pi.nix` [5]. Pi is an npm-workspaces monorepo whose root
build script fails offline (it deletes committed model catalogs then can't
refetch them). The derivation skips the default `npm run build`
(`dontNpmBuild = true`) and chains the four workspace builds manually in
dependency order:

```nix
{ pi, npmDepsHash, buildNpmPackage, nodejs_24, autoPatchelfHook, stdenv, libcap_ng, lib }:

buildNpmPackage {
  pname = "pi";
  version = "0.79.10";
  src = pi;
  npmDepsHash = npmDepsHash;

  dontNpmBuild = true;
  npmFlags = [ "--ignore-scripts" ];
  nodejs = nodejs_24;

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib libcap_ng ];

  buildPhase = ''
    runHook preBuild
    cd packages/tui && npm run build && cd ../..
    cd packages/ai && ../../node_modules/.bin/tsgo -p tsconfig.build.json && cd ../..
    cd packages/agent && npm run build && cd ../..
    cd packages/coding-agent && npm run build && cd ../..
    runHook postBuild
  '';

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

### Example 3: pip FOD two-stage build — odysseus

From `nix/packages/odysseus.nix` [11]. Odysseus has requirements not in
nixpkgs' Python package set, so it uses a FOD `pip download` stage followed
by an offline `pip install --target` stage:

```nix
let
  # Stage 1 — fixed-output wheel fetcher (network allowed; output hash-bounded)
  pipDeps = stdenv.mkDerivation {
    pname = "odysseus-pip-deps";
    version = "0.1.0";
    src = odysseus;
    nativeBuildInputs = [ python312 python312.pkgs.pip ];
    impureEnvVars = lib.fetchers.proxyImpureEnvVars;

    buildPhase = ''
      runHook preBuild
      export HOME=$TMPDIR
      REQ=$(if [ -f requirements.lock ]; then echo requirements.lock; else echo requirements.txt; fi)
      mkdir -p wheels
      python3.12 -m pip download --only-binary=:all: --dest wheels -r "$REQ"
      echo "$REQ" > wheels/.requirements-marker
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mkdir -p $out
      cp -r wheels/. $out/
      runHook postInstall
    '';

    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = lib.fakeHash;
  };
in
# Stage 2 — offline install (no network)
stdenv.mkDerivation {
  pname = "odysseus";
  version = "0.1.0";
  src = odysseus;
  nativeBuildInputs = [ python312 python312.pkgs.pip ];

  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR
    export PIP_NO_CACHE_DIR=1
    REQ="$(cat ${pipDeps}/.requirements-marker)"
    mkdir -p .deps
    python3.12 -m pip install \
      --only-binary=:all: --no-index --find-links ${pipDeps} \
      --break-system-packages --target ./.deps -r "$REQ"
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r . $out/
    runHook postInstall
  '';

  dontStrip = true;
  meta = with lib; {
    description = "Odysseus Python agent (hermetic nix build; pip wheels prefetched via FOD)";
    platforms = [ "x86_64-linux" ];
  };
}
```

### Example 4: `bun build --compile` recipe

From `nix/lib/recipes/bun-compile.nix` [10]. The recipe takes an
already-built source tree, compiles a standalone binary, strips the
source-tree store reference, and mirrors runtime assets:

```nix
{ pkgs, bun, stdenv, lib, removeReferencesTo }:

{ src, entrypoint, worker, assets ? [], ... }:
let
  mirrorAssets = lib.concatStringsSep "\n" (map (a:
    ''
      mkdir -p "$out/bin/$(dirname "${a.to}")"
      cp -r "${src}/${a.from}" "$out/bin/${a.to}"
    ''
  ) assets);
in
stdenv.mkDerivation {
  pname = "bun-compile";
  version = src.version or "0.1.0";
  dontUnpack = true;
  nativeBuildInputs = [ bun removeReferencesTo ];

  buildPhase = ''
    runHook preBuild
    entry="${src}/${entrypoint}"
    wk="${src}/${worker}"
    mkdir -p $out/bin
    bun build --compile "$entry" "$wk" --outfile "$out/bin/app"
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    chmod -R +w $out/bin
    remove-references-to -t ${src} $out/bin/app
    ${mirrorAssets}
    runHook postInstall
  '';

  dontStrip = true;
  meta = with lib; {
    description = "Bun-compiled standalone binary";
    platforms = platforms.linux;
  };
}
```

### Example 5: `npm-build` recipe with optional overrides

From `nix/lib/recipes/npm-build.nix` [9]. The recipe supports optional
`dontNpmBuild`, `buildPhase`, and `installPhase` passthroughs so monorepo
callers (like pi) can override the default single-package-app behavior:

```nix
{ pkgs, buildNpmPackage, nodejs_24, autoPatchelfHook, stdenv, libcap_ng, lib }:

{ src, npmDepsHash, nodeVersion ? "nodejs_24"
, dontNpmBuild ? false, buildPhase ? null, installPhase ? null, ... }:
let
  defaultInstallPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r dist $out/dist 2>/dev/null || true
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json 2>/dev/null || true
    runHook postInstall
  '';
in
buildNpmPackage ({
  pname = src.pname or "npm-build";
  version = src.version or "0.1.0";
  inherit src npmDepsHash;
  npmFlags = [ "--ignore-scripts" ];
  nodejs = nodejs_24;
  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib libcap_ng ];
  installPhase = if installPhase != null then installPhase else defaultInstallPhase;
  dontStrip = true;
  meta = with lib; {
    description = "npm-built application tree";
    platforms = platforms.linux;
  };
} // lib.optionalAttrs dontNpmBuild { inherit dontNpmBuild; }
  // lib.optionalAttrs (buildPhase != null) { inherit buildPhase; })
```

### Example 6: `override` and `overrideAttrs`

```nix
# override: change callPackage arguments
myPython = pkgs.python3.override { packageOverrides = self: super: {
  pandas = super.pandas.overridePythonAttrs (finalAttrs: prevAttrs: {
    version = "0.19.1";
  });
}; };

# overrideAttrs: change derivation attributes
customTempest = tempest.overrideAttrs (prevAttrs: {
  version = "1.0.1-custom";
});
```

## Common mistakes

### 1. Leaving `lib.fakeHash` in committed code

The build succeeds at `nix eval .#<name>.drvPath` but fails at `nix build`.
Always inline the computed `got:` hash before committing.

```nix
# WRONG:
npmDepsHash = lib.fakeHash;

# CORRECT:
npmDepsHash = "sha256-tuEfyePwlOy2/mOPdXbqJskO6IowvAP4DWg8xSZwbJw=";
```

### 2. Using the default `installPhase` for an app

`buildNpmPackage`'s default `installPhase` puts files under
`$out/lib/node_modules/$name/` (library style). Apps expect `dist/` +
`node_modules/` + `package.json` at the root [4].

```nix
# WRONG (for an app): relies on default installPhase
buildNpmPackage { pname = "my-app"; /* ... */ }

# CORRECT: override installPhase
installPhase = ''
  runHook preInstall
  mkdir -p $out
  cp -r dist $out/dist
  cp -r node_modules $out/node_modules
  cp package.json $out/package.json
  runHook postInstall
'';
```

### 3. Running `pip install` with network in the main derivation

The main derivation has no network access. Dependencies must be prefetched
in a FOD stage [11].

```nix
# WRONG: pip install with network in main derivation
buildPhase = ''
  pip install -r requirements.txt  # fails: no network
'';

# CORRECT: two-stage FOD pattern
pipDeps = stdenv.mkDerivation {
  # ... pip download (FOD, network allowed) ...
  outputHash = lib.fakeHash;
};
# main derivation: pip install --no-index --find-links ${pipDeps}
```

### 4. Forgetting `--ignore-scripts` for npm

Lifecycle hooks (husky `prepare`, canvas `node-gyp`) fail in the offline
sandbox [5][6].

```nix
# WRONG:
buildNpmPackage { /* no npmFlags */ }

# CORRECT:
npmFlags = [ "--ignore-scripts" ];
```

### 5. Using `overrideDerivation` instead of `overrideAttrs`

`overrideDerivation` operates on the raw derivation attributes and does not
re-run `mkDerivation` argument processing. Use `overrideAttrs` (or
`overridePythonAttrs` for Python packages) unless there is a specific
reason [4].

### 6. Not stripping store references from compiled binaries

Bun-compiled binaries embed the source-tree store path, bloating the
binary. Use `removeReferencesTo` to strip it [10].

```nix
# WRONG:
installPhase = ''
  runHook preInstall
  # binary still references the source tree
  runHook postInstall
'';

# CORRECT:
installPhase = ''
  runHook preInstall
  remove-references-to -t ${src} $out/bin/app
  runHook postInstall
'';
```

## Strict vs contextual guidance

| Rule | Strict (always) | Contextual (depends) |
| --- | --- | --- |
| `callPackage` for package recipes | Strict | — |
| FOD for all dependency fetching | Strict | — |
| `lib.fakeHash` for initial hash computation | Strict (during dev) | — |
| Real `sha256-...` hash in committed code | Strict | — |
| `--ignore-scripts` for npm | Strict | — |
| `runHook preXxx`/`postXxx` in custom phases | Strict | — |
| `autoPatchelfHook` for prebuilt native binaries | — | Contextual (only when present) |
| Override `installPhase` for app-style packages | — | Contextual (apps yes, libraries no) |
| `dontStrip = true` | — | Contextual (needed when strip breaks prebuilt binaries) |
| `dontNpmBuild = true` | — | Contextual (monorepos with broken root build script) |
| Two-stage FOD for pip | — | Contextual (when requirements not in nixpkgs) |
| `overrideAttrs` over `overrideDerivation` | Strict | — |

## Policy decisions for individual repos

The `ai-workbench` repo applies the general rules above via the following
repo-specific mechanisms:

- **Recipe aggregator** (`nix/lib/recipes.nix` [9]): centralizes all build
  recipes (`npm-build`, `bun-compile`, `pip-install`, `bun-install`) and
  image recipes (`nix-layered`, `registry`). Each recipe is imported with
  its nixpkgs dependencies explicitly wired, giving the aggregator control
  over which nixpkgs revision and package versions each recipe uses.
- **`just update-hashes` recipe** [12]: prefetches `npmDepsHash` (tempest),
  `bunDeps.outputHash` (opencode), and `pipDeps.outputHash` (odysseus). It
  prints the `got:` hashes; the operator manually inlines each into the
  matching `nix/packages/*.nix` file. It does not auto-rewrite the files.
- **HOST-GATE convention**: This container has no nix; all `nix build`
  claims are based on documented Nix semantics, not runtime verification.
  Derivations that can only be verified on a host with nix carry a
  `# HOST-GATE:` comment [5][6][11].
- **`--ignore-scripts` policy**: All npm-based derivations in this repo
  pass `npmFlags = [ "--ignore-scripts" ]` to skip lifecycle hooks that
  fail in the offline sandbox [5][6][9].
- **`autoPatchelfHook` for native binaries**: pi and the `npm-build`
  recipe use `autoPatchelfHook` to patch prebuilt ELF binaries (tsgo,
  esbuild) and `libcap_ng` for gondolin's libkrun runtime [5][9].
- **App-style install layout**: All app packages in this repo override
  `installPhase` to reproduce the runtime tree at `$out` (not the
  library-style `$out/lib/node_modules`) [5][6].

## Related docs

- `/docs/nix/derivations-and-builds.md` — Derivations, stdenv.mkDerivation,
  build phases, hooks, source fetchers, and FODs (the foundational
  reference)
- `/docs/nix/devshells.md` — Declarative shell environments
- `/docs/nix/flake-anatomy.md` — Flake structure and outputs
- `/docs/nix/source-map.md` — Nix source map (provenance index)
- `/docs/rust/cargo-dependencies.md` — Cargo manifests (relevant to
  `buildRustPackage` and `cargoLock`/`cargoHash`)

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference

## Citations

1. [Packaging existing software with Nix](https://nix.dev/tutorials/packaging-existing-software.html)
2. [Package parameters and overrides with `callPackage`](https://nix.dev/tutorials/callpackage.html)
3. [Working with local files](https://nix.dev/tutorials/working-with-local-files.html)
4. [nixpkgs — Build Helpers (buildGoModule, buildNpmPackage, buildPythonApplication)](https://nixos.org/manual/nixpkgs/stable/#sec-language-stdenv)
5. Local: `/nix/packages/pi.nix` — pi buildNpmPackage (monorepo)
6. Local: `/nix/packages/tempest.nix` — tempest buildNpmPackage (single-package app)
7. Local: `/nix/lib/recipes/npm-build.nix` — npm-build recipe
8. Local: `/nix/lib/recipes/bun-install.nix` — bun-install recipe
9. Local: `/nix/lib/recipes.nix` — recipe aggregator
10. Local: `/nix/lib/recipes/bun-compile.nix` — bun-compile recipe
11. Local: `/nix/packages/odysseus.nix` — odysseus pip FOD build
12. Local: `/justfile` — `update-hashes` and `lint-nix` recipes
