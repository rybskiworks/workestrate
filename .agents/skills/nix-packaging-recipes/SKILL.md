---
name: nix-packaging-recipes
description: |
  Operational reference for language-specific Nix packaging — callPackage
  dependency injection, buildNpmPackage, buildPythonApplication, buildGoModule,
  bun build --compile, pip install --target, FOD dependency hashing,
  override/overrideAttrs, and scoped package sets. Load when packaging
  Node/Python/Go/Bun software, computing dependency hashes, or overriding
  packages. Does NOT cover module system (see nix-modules) or flake structure
  (see nix-usage).
---

# Nix Packaging Recipes

Compact operational skill. Full detail in the authoritative long-form
reference: `docs/nix/packaging-recipes.md` (callPackage, buildNpmPackage,
buildPythonApplication, buildGoModule, bun compile, pip install --target,
FOD hashing, override/overrideAttrs, scoped sets). Do not link upstream URLs
from here — cite the local doc.

## Triggers

Load when:

- Packaging Node/Python/Go/Bun apps.
- Using `buildNpmPackage`/`buildPythonApplication`/`buildGoModule`.
- Using `bun build --compile` for standalone binaries.
- Vendoring Python deps via `pip install --target`.
- Computing `npmDepsHash`/`vendorHash`/`outputHash` (FOD hashing).
- Using `callPackage`/`callPackageWith` dependency injection.
- Using `override`/`overrideAttrs`/`overrideDerivation`.
- Reviewing a package recipe PR.

Do NOT load for: module system (use `nix-modules`), lib functions (use
`nix-nixpkgs-library`), flake outputs (use `nix-usage`).

## callPackage

Every nixpkgs recipe is a function taking an attrset of dependencies and
returning a derivation. `callPackage` auto-fills args from `pkgs` by name;
args not in `pkgs` must be supplied explicitly. Default-valued args (`?`) are
optional and overridable. `callPackage` gives `override` for free.

```nix
{ writeShellScriptBin, audience ? "world" }:
writeShellScriptBin "hello" ''echo "Hello, ${audience}!"''

# caller:
hello = pkgs.callPackage ./hello.nix { audience = "people"; };
hello-folks = hello.override { audience = "folks"; };
```

## buildNpmPackage

Packages npm projects without an auto-generated deps file. Internally splits
into two stages: (1) `fetchNpmDeps` FOD runs `npm ci` against
`package-lock.json` with hash-bounded output, (2) main derivation runs
`npm run build` against cached `node_modules` offline.

| Argument | Purpose |
|---|---|
| `npmDepsHash` | Output hash of deps. Computed with `prefetch-npm-deps`. |
| `npmBuildScript` | Script to run. Defaults to `"build"`. |
| `dontNpmBuild` | Disable running the build script. Defaults to `false`. |
| `npmFlags` | Flags to all npm commands. |
| `npmPackFlags` | Flags to `npm pack`. |
| `nodejs` | `nodejs` package to build against. Defaults to `pkgs.nodejs`. |
| `npmWorkspace` | Workspace directory within the project. |

The default `installPhase` is library-style (`$out/lib/node_modules/$name/`).
For applications, override `installPhase` to reproduce the runtime tree at
`$out` (`dist/` + `node_modules/` + `package.json`).

## buildPythonApplication

Builds Python apps where only executables (not importable modules) matter.
Call with `callPackage` and pass `python3`/`python3Packages`. Set
`pyproject = true` for pyproject-based builds.

| Parameter | Purpose |
|---|---|
| `pyproject` | Use pyproject format. Recommended `true`. |
| `format` | Build format (set indirectly via `pyproject`). |
| `nativeBuildInputs` | Build-time only deps (typically executables). |
| `build-system` | Build-time Python deps (`build-system.requires`). |
| `dependencies` | Runtime deps; propagated and wrapped into executables. |
| `propagatedBuildInputs` | Deps propagated to downstream consumers. |
| `nativeCheckInputs` | Test deps (added when `doCheck = true`). |

Supports fixed-point args (`finalAttrs`) and `overridePythonAttrs` for
Python-specific overrides.

## buildGoModule

Two-phase build: (1) `goModules` FOD fetches deps, (2) final derivation
builds binaries against the vendored modules.

| Argument | Purpose |
|---|---|
| `vendorHash` | Output hash of vendored Go module deps (FOD). Use `lib.fakeHash` first. |
| `modBuildPhase` | Override the module-fetch build phase. |
| `modInstallPhase` | Override the module-install phase. |
| `proxyVendor` | Use `go mod download` to a proxy dir instead of `go mod vendor`. |

`vendorHash` follows the same FOD pattern as `npmDepsHash`: set `lib.fakeHash`,
build once, copy the `got:` hash from the failure.

## bun build --compile

`--compile` embeds the Bun runtime into a self-contained binary. Wrapped as
`stdenv.mkDerivation` that runs `bun build --compile` and strips source-tree
store refs. Use `removeReferencesTo` to strip the source-tree ref (the binary
is self-contained). Mirror runtime assets next to the binary.

## pip install --target

For Python apps with requirements not in nixpkgs' Python set. Two-stage FOD:

- **Stage 1 (FOD, network):** `pip download` fetches all wheels into a
  content-addressed directory. Only stage with network access.
- **Stage 2 (offline):** `pip install --no-index --find-links` installs from
  cached wheels into `./deps` with no network.

At runtime, `PYTHONPATH=/app/.deps` makes vendored packages importable.

## FOD Hashing Workflow

1. Set the hash to `lib.fakeHash` (or empty string).
2. Build once — fails loudly with `specified: ... got: ...` mismatch.
3. Copy the `got:` value (prefixed `sha256-`) into the derivation.

**Never leave `lib.fakeHash` in committed code.** `nix eval .#<name>.drvPath`
succeeds even with `lib.fakeHash` (drv instantiation doesn't require the FOD
hash to be correct — only the build does), so this mistake is easy to miss.

## override / overrideAttrs

- `override { ... }` — changes `callPackage` arguments.
- `overrideAttrs (prevAttrs: { ... })` — changes derivation attrs; re-runs
  `mkDerivation` argument processing. Prefer over `overrideDerivation`.
- `overridePythonAttrs` — for Python packages (Python-specific args via
  `passthru`).
- `overrideDerivation` — low-level; operates on raw derivation attrs, does NOT
  re-run `mkDerivation`. Discouraged; use only when the attr isn't exposed by
  `mkDerivation`.
- `callPackageWith (pkgs // packages)` — scoped interdependent package sets;
  relies on lazy evaluation so `packages` can reference itself.

## Quick Commands

```bash
just update-hashes                              # prefetch all FOD hashes
nix build .#<name>                              # build a package
nix build .#<name> --no-link                    # build without result symlink
nix eval .#<name>.drvPath                       # eval drv path (works with fakeHash)
nix run nixpkgs#prefetch-npm-deps -- <package-lock.json>
just lint-nix                                   # static purity guard
```

## Review Checklist

1. Recipe is a function taking an attrset of deps (callPackage convention)?
2. `pname` + `version` set (not `name`)?
3. Hash is real `sha256-...`, not `lib.fakeHash`/empty (in committed code)?
4. `runHook preInstall`/`runHook postInstall` in `installPhase`?
5. `runHook preBuild`/`runHook postBuild` in `buildPhase`?
6. App-style packages override `installPhase` (not library-style default)?
7. `--ignore-scripts` passed to npm commands (defense-in-depth)?
8. `autoPatchelfHook` in `nativeBuildInputs` when prebuilt native binaries present?
9. Python FOD builds use the two-stage pattern (download FOD + offline install)?
10. `override`/`overrideAttrs` used (not `overrideDerivation`) unless specific reason?
11. `meta.platforms` set?

## Common Mistakes

1. **Leaving `lib.fakeHash` committed.** Build succeeds at `nix eval .#<name>.drvPath`
   but fails at `nix build`. Always inline the computed `got:` hash.
2. **Default `installPhase` for an app.** `buildNpmPackage` default puts files
   under `$out/lib/node_modules/$name/` (library style). Apps expect `dist/` +
   `node_modules/` + `package.json` at the root.
3. **`pip install` with network in the main derivation.** Main derivation has no
   network. Prefetch in a FOD stage.
4. **Forgetting `--ignore-scripts` for npm.** Lifecycle hooks (husky `prepare`,
   canvas `node-gyp`) fail in the offline sandbox.
5. **`overrideDerivation` instead of `overrideAttrs`.** `overrideDerivation`
   doesn't re-run `mkDerivation` argument processing.
6. **Not stripping store refs from compiled binaries.** Bun-compiled binaries
   embed the source-tree store path, bloating the binary. Use `removeReferencesTo`.

## Related Docs

- `docs/nix/packaging-recipes.md` — full packaging reference.
- `docs/nix/derivations-and-builds.md` — derivations, stdenv, phases, hooks, FODs.

## Related Skills

- `nix-usage` — ai-workbench flake, dev shell, Rust toolchain, Microsandbox.
- `nix-overlays` — overlays, override/extend patterns.
