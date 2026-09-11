---
name: nix-derivations
description: |
  Operational guide for Nix derivations — stdenv.mkDerivation, build phases,
  phase hooks, dependency attributes (nativeBuildInputs vs buildInputs),
  source fetchers, multi-output derivations, and fixed-output derivations
  (FODs). Load when writing or reviewing derivations, choosing dependency
  placement, wiring custom phases, prefetching source hashes, or debugging
  build failures. Does NOT cover the Nix expression language (see
  nix-language), flake structure (see nix-flake-anatomy), or devshells (see
  nix-devshells).
---

# Derivations and Builds

Distilled operational reference. Full theory and citations live in
`docs/nix/derivations-and-builds.md` (which cites the nix.dev packaging
tutorial, the Nix manual derivations page, and the nixpkgs manual stdenv
sections).

## Triggers

Load this skill when:

- Writing or reviewing a derivation (`stdenv.mkDerivation`,
  `buildRustPackage`, `buildNpmPackage`, etc.).
- Choosing between `nativeBuildInputs` and `buildInputs`.
- Wiring a custom build phase with `preXxx`/`postXxx` hooks.
- Prefetching a source hash (`fetchurl`, `fetchFromGitHub`, `npmDepsHash`).
- Deciding whether a build step must run under a FOD.
- Debugging sandbox build failures (network access, missing deps).

## References

This skill is an index into the docs corpus. Read the relevant doc for full
detail; upstream source URLs are listed in each doc's `## Sources used`
section.

- `docs/nix/derivations-and-builds.md`
  - https://nix.dev/tutorials/packaging-existing-software.html
  - https://nix.dev/manual/nix/2.34/language/derivations
  - https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv
  - https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases
  - https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview
  - https://github.com/NixOS/rfcs/pull/35 (RFC 0035 — pname and version)

## Key Rules

### Primitive `derivation` builtin
The lowest-level construct. Required attributes: `name` (string), `system`
(string), `builder` (path/string). Optional: `args` (list), `outputs` (list,
default `["out"]`). Every other attribute is passed as an environment
variable to the builder (strings unchanged, paths copied to store, derivations
built first, `true`=>`"1"`, `false`/`null`=>`""`). Rarely invoked directly —
`stdenv.mkDerivation` wraps it.

### stdenv.mkDerivation
- Always use `pname` + `version` over bare `name` (RFC 0035). `name` auto-
  derives as `"${pname}-${version}"`.
- The `finalAttrs` pattern allows cross-referencing attributes within the
  derivation: `stdenv.mkDerivation (finalAttrs: { version = "..."; src =
  fetchurl { url = "...${finalAttrs.version}..."; }; })`.
- Minimal derivation needs `pname`/`version` + `src`.

### Build phases (default sequence)
`unpackPhase` -> `patchPhase` -> `configurePhase` -> `buildPhase` ->
`checkPhase` (if `doCheck`) -> `installPhase` -> `fixupPhase` ->
`installCheckPhase` (if `doInstallCheck`) -> `distPhase` (if `doDist`).
- `unpackPhase`: unpacks `src`/`srcs`; supports tar/zip (zip needs `unzip` in
  `nativeBuildInputs`).
- `configurePhase`: runs `./configure` if present; honors `configureFlags`.
- `buildPhase`: runs `make` if a Makefile exists; honors `makeFlags`,
  `buildFlags`.
- `checkPhase`: runs `make $checkTarget` only if `doCheck = true`; skipped
  under cross-compilation.
- `installPhase`: creates `$out`, runs `make install`.
- `fixupPhase`: moves man/doc/info to `share/`, strips debug, rewrites
  shebangs to store paths.

### Phase hooks
- Every phase has `preXxx`/`postXxx` hook pairs.
- When overriding a phase, ALWAYS start with `runHook preXxx` and end with
  `runHook postXxx` — otherwise downstream hooks silently stop running.
- Prefer `preXxx`/`postXxx` hooks or `*Phases` variables
  (`preConfigurePhases`, `preBuildPhases`, etc.) over overriding the `phases`
  attribute directly (overriding `phases` loses `fixupPhase` shebang patching).

### Dependency attributes
| Attribute | Purpose |
|---|---|
| `nativeBuildInputs` | Build-time tools on `$PATH` (cmake, pkg-config, makeWrapper, setup hooks). Run on the build host. |
| `buildInputs` | Runtime libraries (headers searched by C compiler, `.so` on link path). |
| `propagatedBuildInputs` | Runtime libs made available to downstream dependents. |
| `propagatedNativeBuildInputs` | Build-time tools propagated to downstream dependents. |
- Put build-time tools in `nativeBuildInputs`; runtime libraries in
  `buildInputs`. Mixing breaks cross-compilation.
- `strictDeps = true` enforces correct native/host split — set for any
  package that may be cross-compiled.

### Source fetchers (all are FODs)
- `fetchurl` — single URL + `hash`.
- `fetchzip` — downloads and unpacks an archive + `hash`.
- `fetchFromGitHub` — `owner`, `repo`, `rev`, `hash`/`sha256`.
- `builtins.path` — local source with explicit `name` + `filter`. NEVER bare
  `src = ./.` (copies entire working tree including `target/`,
  `node_modules/` into the store).
- `cleanSourceWith` / `lib.cleanSource` — filtered local source; always pass
  a `filter` predicate excluding `target/`, `result*`, `node_modules/`.

### Hash discovery workflow
When the correct hash is unknown: set `hash = lib.fakeSha256` (or `""`),
build, read the `got:` hash from the error, inline it, rebuild.

### Fixed-output derivations (FODs)
- A FOD declares its output is verified by content hash, not build steps.
- This is what permits network access during the fetch phase: the output hash
  is the purity guarantee.
- Attributes: `outputHash` (SRI form `sha256-...`), `outputHashAlgo`
  (`sha256`/`sha512`), `outputHashMode` (`flat` or `recursive`).
- All fetchers are FODs. Language-specific dep hashes are FOD-backed:
  `buildNpmPackage` => `npmDepsHash`, `buildRustPackage` => `cargoHash`/
  `cargoLock`, `buildBunPackage` => `bunDeps.outputHash`,
  `buildPythonApplication` => `pipDeps.outputHash`.

### Sandbox: no network in buildPhase/installPhase
The build sandbox has no network in `buildPhase`/`installPhase`. All
dependencies must arrive via FODs with declared hashes. `--impure` is
forbidden.

### Multi-output derivations
`outputs = [ "out" "dev" "doc" ]` splits a derivation into separately-
installable store paths. Each output name becomes an env var in the builder.
The first output is the default (top-level attribute).
`separateDebugInfo = true` auto-enables a `debug` output.

## Quick Commands

```bash
nix build .#<name>                # build, creates ./result symlink
nix build .#<name> --no-link      # build without ./result
nix log .#<name>                  # view build log
nix log $(nix path-info .#<name>) # view log via path-info
nix-prefetch-url --unpack <url> --type sha256   # prefetch a source hash
nix run nixpkgs#prefetch-npm-deps -- <package-lock.json>  # npmDepsHash
nix develop .#<name>               # enter a package derivation's build env for phase debug
# In that env: export out=$(pwd)/out; phases="buildPhase" genericBuild
```

## Anti-patterns

- Network access in `buildPhase`/`installPhase` — fails in the sandbox. Fetch
  data via a FOD first, then reference it.
- Missing `outputHash` on a FOD — the hash is the purity guarantee.
- `src = ./.` without filtering — copies the entire working tree (including
  `target/`, `node_modules/`, `result*`) into the store. Use `builtins.path`
  with `name` + `filter`, or `cleanSourceWith` with a `filter` predicate.
- `fetchTarball` without a hash — impure and non-reproducible.
- Putting a build tool in `buildInputs` instead of `nativeBuildInputs` —
  works natively, breaks cross-compilation.
- Overriding `phases` instead of using hooks — loses `fixupPhase` and other
  default behavior.
- Missing `runHook preXxx`/`postXxx` in a custom phase — downstream hooks
  silently stop running.
- Hardcoding a guessed hash — use `lib.fakeSha256` + the `got:` error.
- `name = "foo-1.2.3"` instead of `pname` + `version` (RFC 0035).
- Double-wrapping with `wrapProgram` (second call overwrites the first) —
  chain all `--set`/`--prefix` flags in one call.
- `--impure` anywhere in the derivation or wrapping scripts.
- `../` parent-directory path literals outside bounded escape-hatch fields
  (`src =`, `lockFile =`, `path =`).
- Forgetting `doCheck = true` when a test suite exists (check is skipped by
  default).

## Related Skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, Microsandbox.
- nix-language — Nix expression language fundamentals.
- nix-flake-anatomy — flake.nix structure, inputs, outputs, flake.lock.
- nix-devshells — mkShell, shellHook, nix develop.
