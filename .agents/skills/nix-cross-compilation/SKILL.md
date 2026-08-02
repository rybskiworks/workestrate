---
name: nix-cross-compilation
description: |
  Operational guide for Nix cross-compilation — build/host/target platforms,
  pkgsCross, pkgsStatic, the nine dependency types, depsBuildBuild/depsHostHost,
  qemu-user emulation, multi-platform flakes with flake-utils, and Docker image
  cross-compilation. Load when cross-compiling, building for foreign arches,
  making static binaries, or writing multi-platform flakes. Distilled from
  docs/nix/cross-compilation.md; consult that doc for full detail and source URLs.
---

# Nix Cross-Compilation

Distilled from [`docs/nix/cross-compilation.md`](../../../docs/nix/cross-compilation.md).
That doc holds the canonical rules and upstream source links; this skill is the
actionable subset.

## Triggers

Load this skill when:

- Cross-compiling (building for a foreign arch on the current host).
- Using `pkgsCross` or `pkgsStatic`.
- Building static binaries with musl.
- Writing multi-platform flakes (`flake-utils.lib.eachDefaultSystem`).
- Cross-building Docker images (`dockerTools` + `pkgsCross`).
- Diagnosing cross-compilation failures (wrong binutils, test failures).
- Setting `strictDeps` on derivations that may be cross-compiled.

## Build, Host, Target Platforms

| Platform | Runs where | Fundamental? |
|---|---|---|
| `buildPlatform` | Where the package is built | Yes |
| `hostPlatform` | Where the package runs | Yes |
| `targetPlatform` | Where code emitted by the package runs | Only for compilers (GCC, Binutils, GHC) |

Native compilation is the degenerate case where `buildPlatform == hostPlatform`.
Cross-compilation is the general case where they differ. `crossSystem` is `null`
(or omitted) when not cross-compiling.

## Platform Config Strings

Format: `<cpu>-<vendor>-<os>-<abi>` (the LLVM target triple). Examples:

- `aarch64-unknown-linux-gnu`
- `aarch64-apple-darwin14`
- `x86_64-w64-mingw32`
- `armv6l-unknown-linux-gnueabihf`
- `aarch64-apple-ios`

## pkgsCross

`pkgsCross` is the predefined set of cross package sets. Retrieve a target's
config string with `pkgsCross.<attr>.stdenv.hostPlatform.config`.

| `pkgsCross` attribute | Platform config | Use case |
|---|---|---|
| `aarch64-multiplatform` | `aarch64-unknown-linux-gnu` | ARM64 Linux (Pi 4/5, Graviton) |
| `aarch64-multiplatform-musl` | `aarch64-unknown-linux-musl` | Static ARM64 Linux |
| `aarch64-darwin` | `aarch64-apple-darwin14` | Apple Silicon macOS |
| `musl64` | `x86_64-unknown-linux-musl` | Static x86_64 Linux |
| `mingwW64` | `x86_64-w64-mingw32` | Windows |
| `raspberryPi` | `armv6l-unknown-linux-gnueabihf` | Raspberry Pi 1/Zero |
| `armv7l-hf-multiplatform` | `armv7l-unknown-linux-gnueabihf` | 32-bit ARM Linux |
| `riscv64` | `riscv64-unknown-linux-gnu` | RISC-V 64 |

macOS limitation: cross-compilation is only possible between `aarch64-darwin`
and `x86_64-darwin`.

## pkgsStatic

`pkgsStatic` is `pkgsCross` with a static platform (musl libc, `isStatic = true`).
Dev shell snippet:

```nix
let
  pkgs = (import nixpkgs {}).pkgsCross.aarch64-multiplatform;
in
pkgs.pkgsStatic.callPackage ({ mkShell, zlib, pkg-config, file }: mkShell {
  nativeBuildInputs = [ pkg-config file ];
  buildInputs = [ zlib ];
}) {}
```

For `-static` outside an `isStatic` platform on glibc, add
`stdenv.cc.libc.static` to `buildInputs`.

## Dependency Types

The nine dependency types (only these matter in practice):

| Dependency type | Dependency's host platform | Dependency's target platform |
|---|---|---|
| `build → build` | `build` | `build` |
| `build → host` | `build` | `host` |
| `build → target` | `build` | `target` |
| `host → host` | `host` | `host` |
| `host → target` | `host` | `target` |
| `target → target` | `target` | `target` |

Adjacent package sets (the splice):

| Adjacent package set | Their host platform | Their target platform |
|---|---|---|
| `pkgsBuildBuild` | Our build | Our build |
| `pkgsBuildHost` / `buildPackages` | Our build | Our host |
| `pkgsBuildTarget` | Our build | Our target |
| `pkgsHostHost` | Our host | Our host |
| `pkgsHostTarget` / `pkgs` | Our host | Our target |
| `pkgsTargetTarget` / `targetPackages` | Our target | Our target |

Backwards-compat aliases: `depsBuildHost` = `nativeBuildInputs`,
`depsHostTarget` = `buildInputs`. Prefer `buildPackages` over `targetPackages`
— target-mentioning sets cause infinite recursions and have no canonical instance.

## qemu-user Emulation

Every elaborated platform exposes an `emulator` function on its `hostPlatform`
that returns the path to an emulator capable of running binaries for that
platform. `checkPhase` pattern:

```nix
doCheck = stdenv.hostPlatform.emulatorAvailable buildPackages;
checkPhase = ''
  ${stdenv.hostPlatform.emulator buildPackages} ./my-binary --self-test
'';
```

For Meson packages that run host binaries during build, add `mesonEmulatorHook`
to `nativeBuildInputs` when `!stdenv.buildPlatform.canExecute stdenv.hostPlatform`.

## Multi-Platform Flakes

```nix
{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      { packages.default = pkgs.hello; });
}
```

Contrast with the ai-workbench flake, which hardcodes `system = "x86_64-linux"`.
This is intentional — the project depends on Microsandbox/KVM, which is
Linux+KVM-specific. Adding aarch64 would require moving every
`nixpkgs.legacyPackages.${system}` call inside a per-system scope.

## Docker Image Cross-Compilation

`pkgsCross.<target>` + `dockerTools.buildLayeredImage` produces foreign-arch
images. The image contents are target-arch binaries; the image runs on
matching-arch hosts or under `qemu-user`/`qemu-system` emulation. Use
`pkgsStatic` for fully-static binaries that run anywhere.

## Practical Rules

1. Use `pkgsCross.<target>.<package>` for one-off cross builds; `import nixpkgs { crossSystem = ...; }` for a whole cross package set.
2. Set `strictDeps = true` on any package that may be cross-compiled.
3. Put build-time tools (cmake, pkg-config, makeWrapper) in `nativeBuildInputs`; runtime libraries in `buildInputs`.
4. Use `${stdenv.cc.targetPrefix}cc` instead of bare `cc`/`ar`/`ld` — nixpkgs only provides prefixed binutils.
5. Gate tests that run host binaries with `doCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform`.
6. Add `mesonEmulatorHook` to `nativeBuildInputs` for Meson packages that run host binaries during build.
7. Prefer `buildPackages` over `targetPackages` — target-mentioning sets cause infinite recursions.
8. Use `pkgsStatic` (musl + `isStatic = true`) for static binaries, not manual `-static` flags.
9. Use `flake-utils.lib.eachDefaultSystem` for multi-platform flakes; hardcode a single `system` only when there's a hard platform dependency.
10. Tag cross-built Docker images with the target arch (not assumed x86_64).
11. Set `crossSystem = null` (or omit) when not cross-compiling.
12. Check `cache.nixos.org` / the Hydra cross-trunk jobset before building a cross-compiled GCC from source (can take hours).

## Review Checklist

- [ ] `strictDeps = true` is set on any cross-compilable package.
- [ ] Build-time tools in `nativeBuildInputs`; runtime libraries in `buildInputs`.
- [ ] No bare `cc`/`ar`/`ld` — uses `${stdenv.cc.targetPrefix}cc` etc.
- [ ] Tests that run host binaries gated on `stdenv.buildPlatform.canExecute stdenv.hostPlatform`.
- [ ] Meson packages with host-binary build steps use `mesonEmulatorHook`.
- [ ] Static builds use `pkgsStatic` (not manual `-static`).
- [ ] `buildPackages` used, not `targetPackages`.
- [ ] Multi-platform flakes use `flake-utils.lib.eachDefaultSystem` (or explicitly enumerate systems).
- [ ] Cross-built Docker images tagged with the target arch.
- [ ] `crossSystem` is `null` (or omitted) when not cross-compiling.

## Validation Commands

> **HOST-GATE:** This container has no nix. The commands below are documented
> Nix semantics, not runtime-verified in this environment.

```bash
nix-build '<nixpkgs>' -A pkgsCross.aarch64-multiplatform.hello   # cross-compile a package
nix build .#packages.aarch64-linux.<name>                         # build a flake output for aarch64
nix-shell -p qemu --run 'qemu-aarch64 ./result/bin/hello'         # run a cross binary under qemu
nix repl -f '<nixpkgs>'                                          # inspect pkgsCross.<TAB>
```

## Common Mistakes

1. Putting build tools in `buildInputs` instead of `nativeBuildInputs` — works natively, breaks cross-compilation.
2. Not setting `strictDeps = true` — dependency placement is lenient natively but silently breaks under cross.
3. Using bare `cc`/`ar`/`ld` — nixpkgs only provides prefixed binutils.
4. Running host-platform tests unconditionally — tests fail under cross-compilation; gate with `canExecute`.
5. Using `targetPackages` instead of `buildPackages` — no canonical instance for native sets; infinite recursions.
6. Hardcoding `system = "x86_64-linux"` in a flake that should be multi-platform — prevents eval on aarch64/darwin.
7. Assuming cross-built Docker images are x86_64 — they contain target-arch binaries and fail without qemu.
8. Building a cross-compiled GCC from source unnecessarily — can take hours; check the binary cache first.
9. Manual `-static` flags instead of `pkgsStatic` — fails with `cannot find -lm`/`-lc` on glibc.

## Strict Rules

- `strictDeps = true` on any cross-compilable package.
- `nativeBuildInputs` vs `buildInputs` split is strict.
- Prefixed binutils (`${stdenv.cc.targetPrefix}cc`) are strict.
- `buildPackages` over `targetPackages` is strict.
- `pkgsStatic` for static builds is strict (prefer over manual `-static`).
- Gate host-binary tests on `canExecute` is strict.
- Cross-built Docker images tagged with target arch is strict.
- `crossSystem = null` when not cross-compiling is strict.

## Related Docs

- Full reference: [`docs/nix/cross-compilation.md`](../../../docs/nix/cross-compilation.md) (canonical; upstream source URLs there).
- [`docs/nix/derivations-and-builds.md`](../../../docs/nix/derivations-and-builds.md) — `mkDerivation`, dependency attributes, `strictDeps`.
- [`docs/nix/flake-anatomy.md`](../../../docs/nix/flake-anatomy.md) — flake inputs, outputs, system keying.
- [`docs/nix/devshells.md`](../../../docs/nix/devshells.md) — `mkShell`, `nativeBuildInputs`/`buildInputs` in dev shells.

## Related Skills

- [`nix-usage`](../nix-usage/SKILL.md) — project flake (single-system x86_64-linux, intentional).
- [`nix-docker-images`](../nix-docker-images/SKILL.md) — `dockerTools`, `streamLayeredImage`, OCI image building.
