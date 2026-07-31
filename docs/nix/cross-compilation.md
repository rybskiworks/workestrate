---
type: Reference
resource: https://nix.dev/tutorials/cross-compilation.html
title: Cross-Compilation
description: Nix cross-compilation platform tuples (build/host/target), pkgsCross, pkgsStatic, static musl builds, qemu-user emulation, multi-platform flakes, and Docker image cross-compilation.
tags: [nix, cross-compilation, pkgsCross, static, multi-platform]
timestamp: 2026-07-24T02:00:00Z
---

# Cross-Compilation

## Purpose

This document is the OKF-compliant topic reference for cross-compilation in Nix. It covers the build/host/target platform distinction, `pkgsCross` and `pkgsStatic` package sets, the nine dependency categories, `qemu-user` emulation, multi-platform flakes, and Docker image cross-compilation. It is sourced from the nix.dev tutorial and the nixpkgs manual, with local references to the ai-workbench flake and sibling docs.

## Sources used

### Crawl ledger

SEED:
- https://nix.dev/tutorials/cross-compilation.html
- https://nixos.org/manual/nixpkgs/stable/#chap-cross

DISCOVERED & VISITED:
- https://www.gnu.org/software/autoconf/manual/autoconf-2.69/html_node/Hosts-and-Cross_002dCompilation.html
- https://gcc.gnu.org/onlinedocs/gccint/Configure-Terms.html
- https://github.com/NixOS/nixpkgs/blob/master/lib/systems/examples.nix
- https://hydra.nixos.org/jobset/nixpkgs/cross-trunk
- https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies

SKIPPED (out of scope / tangential):
- https://nixos.org/manual/nixpkgs/stable/#ssec-bootstrapping — full bootstrapping stage graph (referenced at summary level only)
- https://nixos.org/manual/nixpkgs/stable/#ssec-cross-dependency-implementation — splice.nix internals (referenced at summary level only)

## Core guidance

### What is cross-compilation?

> "Cross-compilation" means compiling a program on one machine for another type of machine. [2]

> When compiling code, there is a distinction between the **build platform**, where the executable is *built*, and the **host platform**, where the compiled executable *runs*. [1]

> **Native compilation** is the special case where those two platforms are the same.
> **Cross compilation** is the general case where those two platforms are not. [1]

> Nixpkgs adopts the opinion that packages should be written with cross-compilation in mind, and Nixpkgs should evaluate in a similar way (by minimizing cross-compilation-specific special cases) whether or not one is cross-compiling. [2]

Cross-compilation in Nix is the general case of building a derivation on one platform (the **build platform**) for execution on another (the **host platform**). Native compilation is the degenerate case where build and host coincide. Nixpkgs is designed so that the same package expression evaluates correctly whether or not cross-compilation is in effect, minimizing cross-specific special cases.

### Build, host, and target platforms

> Nixpkgs follows the conventions of GNU autoconf. We distinguish between 3 types of platforms when building a derivation: _build_, _host_, and _target_. In summary, _build_ is the platform on which a package is being built, _host_ is the platform on which it will run. The third attribute, _target_, is relevant only for certain specific compilers and build tools. [2]

> The "build platform" is the platform on which a package is built. Once someone has a built package, or pre-built binary package, the build platform should not matter and can be ignored. [2]

> The "host platform" is the platform on which a package will run. This is the simplest platform to understand, but also the one with the worst name. [2]

> The "target platform" attribute is, unlike the other two attributes, not actually fundamental to the process of building software. Instead, it is only relevant for compatibility with building certain specific compilers and build tools. It can be safely ignored for all other packages. [2]

| Platform | Runs where | Fundamental? |
| --- | --- | --- |
| `buildPlatform` | Where the package is built | Yes |
| `hostPlatform` | Where the package runs | Yes |
| `targetPlatform` | Where code emitted by the package runs | Only for compilers (GCC, Binutils, GHC) |

### Platform config strings

> `<cpu>-<vendor>-<os>-<abi>` [1]

> This is a 3- or 4- component shorthand for the platform. Examples of this would be `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin14`. This is a standard format called the "LLVM target triple", as it was pioneered by LLVM. In the 4-part form, this corresponds to `[cpu]-[vendor]-[os]-[abi]`. [2]

Common platform config strings:

- `aarch64-unknown-linux-gnu`
- `aarch64-apple-darwin14`
- `x86_64-w64-mingw32`
- `aarch64-apple-ios`
- `armv6l-unknown-linux-gnueabihf`

### `localSystem` and `crossSystem`

> Nixpkgs can be instantiated with `localSystem` alone, in which case there is no cross-compiling and everything is built by and for that system, or also with `crossSystem`, in which case packages run on the latter, but all building happens on the former. [2]

`localSystem` expresses the build platform intent; `crossSystem` expresses the host platform intent and is `null` (or omitted) when not cross-compiling.

Import form:

```nix
let
  nixpkgs = fetchTarball { url = "..."; hash = "sha256-..."; };
  pkgs = import nixpkgs { crossSystem = { config = "aarch64-unknown-linux-gnu"; }; };
in
  pkgs.hello
```

`--arg crossSystem` form:

```sh
$ nix-build '<nixpkgs>' --arg crossSystem '{ config = "aarch64-unknown-linux-gnu"; }' -A hello
```

### `pkgsCross`: the cross package set

> `nixpkgs` comes with a set of predefined host platforms for cross compilation called `pkgsCross`. [1]

> These attribute names for cross compilation packages have been chosen somewhat freely over the course of time.
> They usually do not match the corresponding platform config string. [1]

> You can retrieve the platform string from `pkgsCross.<platform>.stdenv.hostPlatform.config` [1]

> 1. Take the build platform configuration and apply it to the current package set, called `pkgs` by convention.
> 2. Apply the appropriate host platform configuration to all the packages in `pkgsCross`. [1]

Representative `pkgsCross` attributes (not exhaustive):

- `pkgsCross.aarch64-multiplatform` → `aarch64-unknown-linux-gnu`
- `pkgsCross.aarch64-multiplatform-musl`
- `pkgsCross.aarch64-darwin`
- `pkgsCross.musl64` → x86_64 musl
- `pkgsCross.musl32`
- `pkgsCross.mingwW64` → Windows
- `pkgsCross.raspberryPi` → `armv6l-unknown-linux-gnueabihf`
- `pkgsCross.armv7l-hf-multiplatform`
- `pkgsCross.riscv64`
- `pkgsCross.ghcjs`
- `pkgsCross.wasi32`
- `pkgsCross.iphone64`

Retrieving the config string:

```shell-session
nix-repl> pkgsCross.aarch64-multiplatform.stdenv.hostPlatform.config
"aarch64-unknown-linux-gnu"
```

### `stdenv.buildPlatform` vs `hostPlatform` vs `targetPlatform`

These are always-defined attributes on `stdenv`. Access pattern:

```nix
{ stdenv, fooDep, barDep, ... }:
{
  # ...stdenv.buildPlatform...
}
```

Each platform value exposes:

- `system` — two-component form (e.g. `x86_64-linux`)
- `config` — the LLVM target triple
- `parsed` — structured platform record
- `libc` — libc identifier
- `is*` predicates (e.g. `isLinux`, `isDarwin`, `isAarch64`)
- `platform` — additional platform facts

### Dependency categorization: the nine types

> A run time dependency between two packages requires that their host platforms match. [2]

> A build time dependency, however, has a shift in platforms between the depending package and the depended-on package. "build time dependency" means that to build the depending package we need to be able to run the depended-on's package. The depending package's build platform is therefore equal to the depended-on package's host platform. [2]

> Only nine dependency types matter in practice [2]

| Dependency type | Dependency's host platform | Dependency's target platform |
| --- | --- | --- |
| `build → *` | `build` | (none) |
| `build → build` | `build` | `build` |
| `build → host` | `build` | `host` |
| `build → target` | `build` | `target` |
| `host → *` | `host` | (none) |
| `host → host` | `host` | `host` |
| `host → target` | `host` | `target` |
| `target → *` | `target` | (none) |
| `target → target` | `target` | `target` |

### `depsBuildBuild`, `depsBuildTarget`, `depsHostHost`, `depsTargetTarget`

The `deps<theirHost><theirTarget>` naming encodes the dependency's host and target platforms relative to the depending package. Backwards-compat aliases: `depsBuildHost` = `nativeBuildInputs`, `depsHostTarget` = `buildInputs`.

| Adjacent package set | Their host platform | Their target platform |
| --- | --- | --- |
| `pkgsBuildBuild` | Our build platform | Our build platform |
| `pkgsBuildHost` / `buildPackages` | Our build platform | Our host platform |
| `pkgsBuildTarget` | Our build platform | Our target platform |
| `pkgsHostHost` | Our host platform | Our host platform |
| `pkgsHostTarget` / `pkgs` | Our host platform | Our target platform |
| `pkgsTargetTarget` / `targetPackages` | Our target platform | Our target platform |

> To make this work, we "splice" together the six `pkgs<theirHost><theirTarget>` package sets and have `callPackage` actually take its arguments from that. [2]

> It is much better to refer to `buildPackages` than `targetPackages`, or more broadly package sets that do not mention "target". [2]

### Static builds: `pkgsStatic`, musl, static linking

> It's also possible to provide an environment with a compiler configured for **cross-compilation to static binaries using musl**. [1]

`pkgsStatic` is `pkgsCross` with a static platform (musl libc, `isStatic = true`). Dev shell example from crawl [1]:

```nix
let
  nixpkgs = fetchTarball { url = "..."; hash = "sha256-..."; };
  pkgs = (import nixpkgs {}).pkgsCross.aarch64-multiplatform;
in
pkgs.pkgsStatic.callPackage ({ mkShell, zlib, pkg-config, file }: mkShell {
  nativeBuildInputs = [ pkg-config file ];
  buildInputs = [ zlib ];
}) {}
```

For `-static` outside an `isStatic` platform, add `stdenv.cc.libc.static` to `buildInputs`. The `stdenvAdapters.makeStatic` adapter wraps a stdenv to produce static binaries.

### Common cross targets

| `pkgsCross` attribute | Platform config | Use case |
| --- | --- | --- |
| `aarch64-multiplatform` | `aarch64-unknown-linux-gnu` | ARM64 Linux (Raspberry Pi 4/5, Graviton) |
| `aarch64-multiplatform-musl` | `aarch64-unknown-linux-musl` | Static ARM64 Linux |
| `aarch64-darwin` | `aarch64-apple-darwin14` | Apple Silicon macOS |
| `x86_64-darwin` | (Intel macOS) | Intel macOS (cross only between darwin variants) |
| `armv7l-hf-multiplatform` | `armv7l-unknown-linux-gnueabihf` | 32-bit ARM Linux |
| `raspberryPi` | `armv6l-unknown-linux-gnueabihf` | Raspberry Pi 1/Zero |
| `musl64` | `x86_64-unknown-linux-musl` | Static x86_64 Linux |
| `mingwW64` | `x86_64-w64-mingw32` | Windows |
| `riscv64` | `riscv64-unknown-linux-gnu` | RISC-V 64 |

macOS limitation from crawl [1]:

> macOS/Darwin is a special case, as not the whole OS is open-source.
> It's only possible to cross compile between `aarch64-darwin` and `x86_64-darwin`. [1]

### `qemu-user` for transparent emulation

> Every elaborated platform exposes an `emulator` function on its `hostPlatform` attribute that returns the path to an emulator capable of running binaries for that platform. [2]

> `qemu-user` for foreign Linux targets on a Linux builder [2]

The emulator dispatch selects among: no-op (native), wine, qemu-user, wasmtime, nodejs-slim, and mmix, depending on the platform pair. `checkPhase` pattern:

```nix
stdenv.mkDerivation {
  # ...
  doCheck = stdenv.hostPlatform.emulatorAvailable buildPackages;
  checkPhase = ''
    ${stdenv.hostPlatform.emulator buildPackages} ./my-binary --self-test
  '';
}
```

Manual invocation:

```sh
$ nix-shell -p qemu --run 'qemu-aarch64 ./result/bin/hello'
Hello, world!
```

### Multi-platform flakes with `flake-utils.lib.eachDefaultSystem`

A flake's `outputs` are keyed by system string. Multi-platform pattern:

```nix
{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      {
        packages.default = pkgs.hello;
        devShells.default = pkgs.mkShell { buildInputs = [ pkgs.hello ]; };
      });
}
```

Contrast with the ai-workbench flake, which hardcodes `system = "x86_64-linux"` [5].

### Docker image cross-compilation

Building aarch64 Docker images on an x86_64 host. Two approaches:

- `pkgsCross.aarch64-multiplatform` + `dockerTools.buildLayeredImage` — the image contents are aarch64 binaries; the image runs on aarch64 hosts or under qemu emulation.
- `pkgsStatic` for fully-static binaries that run anywhere.

`dockerTools` images are platform-tagged; cross-built images need `--system` or a cross pkgs set. See `/docs/nix/derivations-and-builds.md` [6] for dockerTools.

## Practical rules

1. Use `pkgsCross.<target>.<package>` for one-off cross builds; use `import nixpkgs { crossSystem = ...; }` for a whole cross package set [1].
2. Set `strictDeps = true` on any package that may be cross-compiled, so `nativeBuildInputs`/`buildInputs` are correctly spliced [6][9].
3. Put build-time tools (cmake, pkg-config, makeWrapper) in `nativeBuildInputs` (= `depsBuildHost`); put runtime libraries in `buildInputs` (= `depsHostTarget`) [9].
4. Use `${stdenv.cc.targetPrefix}cc` instead of bare `cc`/`ar`/`ld` — nixpkgs only provides prefixed binutils [2].
5. For tests that run host-platform binaries, gate with `doCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform` [2].
6. For Meson packages that run host binaries during build, add `mesonEmulatorHook` to `nativeBuildInputs` when `!stdenv.buildPlatform.canExecute stdenv.hostPlatform` [2].
7. Prefer `buildPackages` over `targetPackages` — target-mentioning package sets cause infinite recursions and have no canonical instance [2].
8. For static binaries, prefer `pkgsStatic` (musl + `isStatic = true`) over manual `-static` flags [2].
9. For `-static` outside an `isStatic` platform on glibc, add `stdenv.cc.libc.static` to `buildInputs` [2].
10. Use `flake-utils.lib.eachDefaultSystem` for multi-platform flakes; avoid hardcoding a single `system` unless the project is genuinely single-platform [5].
11. Cross-compiled Docker images (via `dockerTools` + `pkgsCross`) produce foreign-arch images; run them under `qemu-user`/`qemu-system` on the build host or deploy to matching-arch hosts [6].
12. Check `cache.nixos.org` / the Hydra cross-trunk jobset [8] before building a cross-compiled GCC from source (can take hours).

## Review checklist

- [ ] `strictDeps = true` is set on any cross-compilable package.
- [ ] Build-time tools are in `nativeBuildInputs`; runtime libraries in `buildInputs`.
- [ ] No bare `cc`/`ar`/`ld` — uses `${stdenv.cc.targetPrefix}cc` etc.
- [ ] Tests that run host binaries are gated on `stdenv.buildPlatform.canExecute stdenv.hostPlatform`.
- [ ] Meson packages with host-binary build steps use `mesonEmulatorHook`.
- [ ] Static builds use `pkgsStatic` (not manual `-static`).
- [ ] `buildPackages` is used, not `targetPackages`.
- [ ] Multi-platform flakes use `flake-utils.lib.eachDefaultSystem` (or explicitly enumerate supported systems).
- [ ] Cross-built Docker images are tagged with the target arch (not assumed x86_64).
- [ ] `crossSystem` is `null` (or omitted) when not cross-compiling.

## Implementation checklist

1. **Identify the target platform.** Run `config.guess` on the host, or construct `<cpu>-<vendor>-<os>-<abi>` manually [1].
2. **Choose the `pkgsCross` attribute.** Check `nix repl -f '<nixpkgs>'` for `pkgsCross.<TAB>` [1]. If none exists, contribute to `lib/systems/examples.nix` [7].
3. **Decide static vs dynamic.** Use `pkgsStatic` for fully-static binaries (musl); use the plain `pkgsCross.<target>` for dynamic linking against the target's glibc [2].
4. **Set `strictDeps = true`** on derivations that may be cross-compiled [6].
5. **Sort dependencies** into `nativeBuildInputs` (build→host) and `buildInputs` (host→target) [9].
6. **Gate tests** with `stdenv.buildPlatform.canExecute stdenv.hostPlatform` or use `stdenv.hostPlatform.emulator` [2].
7. **For Docker images**, build with `pkgsCross.<target>` + `dockerTools.buildLayeredImage`; the resulting tarball contains foreign-arch binaries [6].
8. **Verify with qemu** if the build host can't natively run the target arch: `qemu-<arch> ./result/bin/<binary>` [2].
9. **HOST-GATE note**: This container has no nix; cross-build claims are based on documented Nix semantics, not runtime verification.

## Validation hooks

> **HOST-GATE:** This container has no nix. The commands below are documented Nix semantics, not runtime-verified in this environment.

```bash
# Cross-compile a package (one-off)
nix-build '<nixpkgs>' -A pkgsCross.aarch64-multiplatform.hello

# Cross-compile via --arg crossSystem
nix-build '<nixpkgs>' --arg crossSystem '{ config = "aarch64-unknown-linux-gnu"; }' -A hello

# Inspect a cross package set's platform config
nix repl -f '<nixpkgs>'
nix-repl> pkgsCross.aarch64-multiplatform.stdenv.hostPlatform.config

# Get the emulator path for a crossSystem
nix-instantiate --eval --strict -E \
  '(import <nixpkgs> { crossSystem.config = "aarch64-unknown-linux-gnu"; }).stdenv.hostPlatform.emulator (import <nixpkgs> {})'

# Run a cross-compiled binary under qemu
nix-shell -p qemu --run 'qemu-aarch64 ./result/bin/hello'

# Cross-build a Docker image (aarch64 on x86_64 host)
nix-build -A pkgsCross.aarch64-multiplatform.dockerTools.buildLayeredImage ...
```

## Examples

### Example 1: Cross-compile GNU Hello (from crawl [1])

```sh
$ nix-build '<nixpkgs>' -A pkgsCross.aarch64-multiplatform.hello
/nix/store/1dx87l5rav8679lqigf9xxkb7wvh2m4k-hello-aarch64-unknown-linux-gnu-2.12.1
```

### Example 2: Static cross-compile dev shell (from crawl [1])

```nix
# shell.nix
let
  nixpkgs = fetchTarball { url = "https://github.com/NixOS/nixpkgs/tarball/release-23.11"; hash = "sha256-..."; };
  pkgs = (import nixpkgs {}).pkgsCross.aarch64-multiplatform;
in
pkgs.pkgsStatic.callPackage ({ mkShell, zlib, pkg-config, file }: mkShell {
  nativeBuildInputs = [ pkg-config file ];
  buildInputs = [ zlib ];
}) {}
```

```sh
$ nix-shell --run '$CC hello.c -o hello' shell.nix
$ nix-shell --run 'file hello' shell.nix
hello: ELF 64-bit LSB executable, ARM aarch64, version 1 (SYSV), statically linked, with debug_info, not stripped
```

### Example 3: Cross-compile + emulate (from crawl [1])

```nix
# cross-compile.nix
let
  nixpkgs = fetchTarball { url = "..."; hash = "sha256-..."; };
  pkgs = import nixpkgs {};
  helloWorld = pkgs.writeText "hello.c" ''
    #include <stdio.h>
    int main (void) { printf ("Hello, world!\n"); return 0; }
  '';
  crossCompileFor = hostPkgs:
    hostPkgs.runCommandCC "hello-world-cross-test" {} ''
      HOME=$PWD
      $CC ${helloWorld} -o hello
      ${hostPkgs.stdenv.hostPlatform.emulator hostPkgs.buildPackages} hello > $out
      cat $out
    '';
in {
  rpi = crossCompileFor pkgs.pkgsCross.raspberryPi;
  windows = crossCompileFor pkgs.pkgsCross.mingwW64;
}
```

### Example 4: Multi-platform flake with flake-utils

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      {
        packages.default = pkgs.hello;
        devShells.default = pkgs.mkShell { buildInputs = [ pkgs.hello ]; };
      });
}
```

### Example 5: The ai-workbench flake (single-system, NOT multi-platform) [5]

The current `flake.nix` hardcodes a single system:

```nix
{
  outputs = { self, nixpkgs, ... }:
    let
      system = "x86_64-linux";   # <-- single system
      pkgs = nixpkgs.legacyPackages.${system};
      # ...
    in {
      devShells.${system}.default = ...;
      packages.${system} = ...;
      apps.${system}.default = ...;
    };
}
```

This is intentional: the project targets x86_64-linux only (Microsandbox/KVM dependency). To add aarch64-linux support, the `system` let-binding would need to become a function over a system list (or `flake-utils.lib.eachSystem`), and every `nixpkgs.legacyPackages.${system}` / `fenix.packages.${system}` call would need to move inside the per-system scope.

## Common mistakes

### 1. Putting build tools in `buildInputs` instead of `nativeBuildInputs`

Works natively, breaks cross-compilation: the tool won't be on `$PATH` during the build on the foreign host [6][9].

### 2. Not setting `strictDeps = true`

Without `strictDeps`, dependency placement is lenient and works natively but silently breaks under cross-compilation [6].

### 3. Using bare `cc`/`ar`/`ld`

nixpkgs only provides prefixed binutils. Use `${stdenv.cc.targetPrefix}cc` [2].

### 4. Running host-platform tests unconditionally

Tests that execute the built binary fail under cross-compilation. Gate with `stdenv.buildPlatform.canExecute stdenv.hostPlatform` [2].

### 5. Using `targetPackages` instead of `buildPackages`

`targetPackages` has no canonical instance for native package sets and is a frequent source of infinite recursions. Use `buildPackages` [2].

### 6. Hardcoding `system = "x86_64-linux"` in a flake that should be multi-platform

Prevents the flake from evaluating on aarch64/darwin hosts. Use `flake-utils.lib.eachDefaultSystem` unless there's a hard platform dependency [5].

### 7. Assuming cross-built Docker images are x86_64

A `dockerTools.buildLayeredImage` built with `pkgsCross.aarch64-multiplatform` contains aarch64 binaries. It will fail to run on an x86_64 host without qemu emulation [6].

### 8. Building a cross-compiled GCC from source unnecessarily

Can take hours. Check `cache.nixos.org` / the Hydra cross-trunk jobset [8] first.

### 9. Manual `-static` flags instead of `pkgsStatic`

Manual `-static` on a glibc platform fails with `cannot find -lm` / `cannot find -lc`. Use `pkgsStatic` (musl, `isStatic = true`) or add `stdenv.cc.libc.static` [2].

## Strict vs contextual guidance

| Rule | Strict (always) | Contextual (depends) |
| --- | --- | --- |
| `strictDeps = true` for cross-compilable packages | Strict | — |
| `nativeBuildInputs` vs `buildInputs` split | Strict | — |
| Prefixed binutils (`${stdenv.cc.targetPrefix}cc`) | Strict | — |
| `buildPackages` over `targetPackages` | Strict | — |
| `pkgsStatic` for static builds | Strict (prefer over manual `-static`) | — |
| Gate host-binary tests on `canExecute` | Strict | — |
| `flake-utils.lib.eachDefaultSystem` for multi-platform flakes | — | Contextual (strict if multi-platform; single-system hardcode OK if hard platform dependency) |
| Cross-built Docker images tagged with target arch | Strict | — |
| `crossSystem = null` when not cross-compiling | Strict | — |
| `mesonEmulatorHook` | — | Contextual (only for Meson packages that run host binaries during build) |

## Policy decisions for individual repos

The `ai-workbench` repo [5] applies the following cross-compilation-relevant decisions:

- **Single-system flake**: `flake.nix` hardcodes `system = "x86_64-linux"`. This is intentional because the project depends on Microsandbox/KVM, which is Linux+KVM-specific. The flake does NOT use `flake-utils.lib.eachDefaultSystem` and does not expose `aarch64-linux` or `darwin` outputs [5].
- **`strictDeps` on cross-compilable packages**: Derivations like `agentctl.nix` and `pi.nix` set `strictDeps`/sort dependencies correctly so they remain cross-compilable in principle, even though the flake currently only builds for x86_64-linux [6].
- **Docker images via `dockerTools`**: Workload images (`pi-image`, `tempest-image`) use `dockerTools.buildLayeredImage`. These are currently x86_64-linux only. Cross-building an aarch64 image would require switching the `pkgs` callPackage argument to `pkgsCross.aarch64-multiplatform` [6].
- **No `--impure`**: The `just lint-nix` guard forbids `--impure`, `builtins.getFlake` + `toString`, and unfiltered source paths — all of which would also break cross-compilation reproducibility [6].
- **HOST-GATE convention**: This container has no nix; cross-compilation claims are based on documented Nix semantics, not runtime verification.

## Related docs

- `/docs/nix/derivations-and-builds.md` — Derivations, `mkDerivation`, build phases, dependency attributes (`nativeBuildInputs`/`buildInputs`), `strictDeps`
- `/docs/nix/flake-anatomy.md` — Flake inputs, outputs, system keying
- `/docs/nix/devshells.md` — `mkShell`, `nativeBuildInputs`/`buildInputs` in dev shells

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and Microsandbox runtime reference

## Citations

1. [Cross compilation — nix.dev tutorial](https://nix.dev/tutorials/cross-compilation.html)
2. [nixpkgs Manual — Cross-compilation](https://nixos.org/manual/nixpkgs/stable/#chap-cross)
3. [GNU Autoconf — Hosts and Cross Compilation](https://www.gnu.org/software/autoconf/manual/autoconf-2.69/html_node/Hosts-and-Cross_002dCompilation.html)
4. [GCC Internals — Configure Terms](https://gcc.gnu.org/onlinedocs/gccint/Configure-Terms.html)
5. Local: `/home/node/Development/ai-workbench/flake.nix` — ai-workbench flake (single-system x86_64-linux)
6. Local: `/home/node/Development/ai-workbench/docs/nix/derivations-and-builds.md` — derivations, mkDerivation, dependency attributes
7. [nixpkgs — lib/systems/examples.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/systems/examples.nix)
8. [Hydra — nixpkgs cross-trunk jobset](https://hydra.nixos.org/jobset/nixpkgs/cross-trunk)
9. [nixpkgs Manual — Specifying dependencies](https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies)
