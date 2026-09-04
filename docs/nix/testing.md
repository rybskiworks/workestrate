---
type: Reference
resource: https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html
title: Testing
description: Nix testing reference — nix flake check, checks output, checkPhase/doCheck, nixosTests (VM test framework), testers, runCommand, and derivation testing.
tags: [nix, testing, nixosTests, checkPhase, nix-flake-check]
timestamp: 2026-07-24T02:00:00Z
---

# Testing

## Purpose

Provide concrete, repo-independent guidance for testing in Nix: the
flake-level `nix flake check` command, the `checks` flake output, the
derivation-level `checkPhase` / `doCheck` / `checkInputs` mechanism, the
NixOS VM test framework (`nixosTests` / `testers.runNixOSTest`), the
`testers` helper functions, `runCommand` for simple derivation tests, and
integration testing of services in virtual machines. This document is
intended as generic reference material for future AI coding agents working
in any Nix flake. It is not specific to the `ai-workbench` repository.

Agents should use this document as the authoritative reference when adding
a test derivation to a flake's `checks` output, enabling `checkPhase` in a
derivation, writing a NixOS VM test, choosing between `runCommand` and a
full `nixosTest`, or running `nix flake check` in CI.

## Sources used

- <https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html>
- <https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html>
- <https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv>
- <https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases>
- <https://nix.dev/concepts/flakes.html>
- <https://nix.dev/guides/recipes/continuous-integration-github-actions.html>
- Local: `/flake.nix` — `checks.validateConfig` (via `libForSystem`)
- Local: `/nix/lib/config.nix` — `referenceConfig` (parsed TOML config)

### Crawl ledger

SEED:

- <https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html>
- <https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv>
- <https://nix.dev/concepts/flakes.html>

DISCOVERED & VISITED:

- <https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html>
- <https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases>
- <https://nix.dev/guides/recipes/continuous-integration-github-actions.html>

SKIPPED (out of scope / tangential):

- <https://nixos.org/manual/nixos/stable/index.html#sec-nixos-tests> — full
  NixOS test options reference (referenced at summary level only)
- <https://nixos.org/manual/nixos/stable/index.html#ssec-machine-objects> —
  full machine object method catalogue (key methods covered here)

## Core guidance

### The testing hierarchy

Nix testing operates at three levels, from cheapest to most expensive:

| Level | Mechanism | What it tests | Cost |
|-------|-----------|---------------|------|
| Flake | `nix flake check` + `checks` output | Evaluates all outputs; builds `checks` derivations | Low–medium |
| Derivation | `checkPhase` / `doCheck` | Runs the package's own test suite during the build | Medium |
| Integration | `nixosTests` / `testers.runNixOSTest` | Boots QEMU VMs and runs a Python test script against them | High |

### nix flake check

`nix flake check` is the flake-level validation command. It does two things:

1. **Evaluates every output** in the flake to confirm the attribute tree is
   well-formed (no evaluation errors, no missing attributes).
2. **Builds every derivation in `checks.<system>`** — if any check derivation
   fails to build, the command fails.

From the flakes concept page [1]:

> Nix checks `flake.nix`'s structure is valid.

The command is listed among the core flake commands:

| command | purpose |
|---------|---------|
| `nix flake check` | validate flake structure and run `checks` |
| `nix flake show` | show all outputs (attribute tree) |

`nix flake check` requires the `nix-command flakes` experimental features:

```sh
nix --experimental-features 'nix-command flakes' flake check
```

Or with the features enabled persistently in `nix.conf`:

```sh
nix flake check
```

> **HOST-GATE:** `nix flake check` requires Nix on the host. This container
> has no nix; the command semantics below are documented, not
> runtime-verified in this environment.

### The checks flake output

The `checks.<system>.<name>` output type holds test derivations that `nix
flake check` builds. From the flakes concept page [1], `checks` is a
built-in output type:

| output | purpose |
|--------|---------|
| `checks.<system>.<name>` | test derivations |

Each `checks` entry is a standard Nix derivation. If it builds
successfully, the check passes. If the build fails, the check fails. The
derivation's `buildPhase` (or `installPhase`) contains the test logic.

A `checks` entry can be:

- A `runCommand` derivation that runs a script and produces `$out` on
  success (the simplest form).
- A `stdenv.mkDerivation` with `doCheck = true` that runs a test suite.
- A `nixosTest` / `testers.runNixOSTest` derivation that boots VMs.
- Any derivation whose build success/failure encodes a test result.

To run an individual check without running all of them:

```bash
nix build .#checks.x86_64-linux.<name>
```

### checkPhase in derivations

The `checkPhase` is a stdenv build phase that runs the package's test
suite. From the nixpkgs manual [3]:

> The check phase checks whether the package was built correctly by running
> its test suite. The default `checkPhase` calls `make $checkTarget`, but
> only if the `doCheck` variable is enabled.

#### Variables controlling the check phase

| Variable | Default | Description |
|----------|---------|-------------|
| `doCheck` | `false` | Controls whether the check phase is executed. Set `doCheck = true` to enable. Skipped under cross-compilation. |
| `checkTarget` | `check` (or `test`) | The `make` target that runs the tests. If unset, uses `check` if it exists, otherwise `test`. |
| `checkFlags` / `checkFlagsArray` | `[]` | Flags passed to `make` during the check phase only. `checkTarget` is auto-added. |
| `checkInputs` | `[]` | Host dependencies used by the check phase (libraries linked into test executables). Included in `buildInputs` when `doCheck` is set. |
| `nativeCheckInputs` | `[]` | Native dependencies used by the check phase (tools on `$PATH`, setup hooks). Included in `nativeBuildInputs` when `doCheck` is set. |
| `preCheck` | — | Hook executed at the start of the check phase. |
| `postCheck` | — | Hook executed at the end of the check phase. |

From the nixpkgs manual [3]:

> Controls whether the check phase is executed. By default it is skipped,
> but if `doCheck` is set to true, the check phase is usually executed.

> The exception is cross compilation. Cross compiled builds never run tests,
> no matter how `doCheck` is set, as the newly-built program won't run on the
> platform used to build it.

The `checkInputs` / `nativeCheckInputs` split mirrors the
`buildInputs` / `nativeBuildInputs` split [4]:

> `nativeCheckInputs` for test tools needed on `$PATH` (such as `ctest`) and
> setup hooks (for example `pytestCheckHook`)

> `checkInputs` for libraries linked into test executables (for example the
> `qcheck` OCaml package)

> These dependencies are only injected when `doCheck` is set to `true`.

#### installCheckPhase

There is a parallel `installCheckPhase` that runs tests against the
*installed* output (not the build directory). From [3]:

> The installCheck phase checks whether the package was installed correctly
> by running its test suite against the installed directories. The default
> `installCheck` calls `make installcheck`.

Controlled by `doInstallCheck` (default `false`), with
`installCheckInputs` / `nativeInstallCheckInputs` dependencies. Also
skipped under cross-compilation.

The nixpkgs manual recommends [3]:

> It is often better to add tests that are not part of the source
> distribution to `passthru.tests`. This avoids adding overhead to every
> build and enables us to run them independently.

### nixosTests: the NixOS VM test framework

NixOS VM tests are the integration-testing tier. From the nix.dev
integration-testing tutorial [5]:

> Nixpkgs provides a test environment to automate integration testing for
> distributed systems. It allows defining tests based on a set of
> declarative NixOS configurations and using a Python shell to interact with
> them through QEMU as the backend. Those tests are widely used to ensure
> that NixOS works as intended, so in general they are called NixOS Tests.

> Nix's design properties make integration tests reproducible, which makes
> them valuable in a continuous integration (CI) pipeline.

#### testers.runNixOSTest

NixOS VM tests are defined using the `testers.runNixOSTest` function [5]:

> NixOS VM tests are defined using the `testers.runNixOSTest` function.

The pattern:

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-23.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in

pkgs.testers.runNixOSTest {
  name = "test-name";
  nodes = {
    machine1 = { config, pkgs, ... }: {
      # ...
    };
    machine2 = { config, pkgs, ... }: {
      # ...
    };
  };
  testScript = { nodes, ... }: ''
    # ...
  '';
}
```

The required configuration values [5]:

- **`name`** — defines the name of the test.
- **`nodes`** — a set of named NixOS configurations, one per virtual
  machine. Each VM is created from a NixOS configuration.
- **`testScript`** — the Python test script, either as a literal string or
  as a function taking a `nodes` attribute. The script accesses VMs via
  their `nodes` names. It has superuser rights in the VMs.

> The test framework automatically starts the virtual machines and runs the
> Python script.

#### Minimal example

From the nix.dev tutorial [5], a minimal test checking that user `alice`
can run Firefox but `root` cannot:

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-23.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in

pkgs.testers.runNixOSTest {
  name = "minimal-test";

  nodes.machine = { config, pkgs, ... }: {

    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      packages = with pkgs; [
        firefox
        tree
      ];
    };

    system.stateVersion = "23.11";
  };

  testScript = ''
    machine.wait_for_unit("default.target")
    machine.succeed("su -- alice -c 'which firefox'")
    machine.fail("su -- root -c 'which firefox'")
  '';
}
```

Run it:

```shell-session
$ nix-build minimal-test.nix
```

#### VM lifecycle: machine object methods

Each VM is accessible in the Python test script as a `machine` object (or
the name given in `nodes`). Key methods [5]:

| Method | Purpose |
|--------|---------|
| `machine.start()` | Start a specific VM. |
| `start_all()` | Start all VMs. |
| `machine.wait_for_unit("default.target")` | Wait until systemd reaches the given unit. |
| `machine.succeed("command")` | Run a shell command; fail the test if it exits non-zero. |
| `machine.fail("command")` | Run a shell command; fail the test if it exits zero (inverse of `succeed`). |
| `machine.shell_interact()` | Enter an interactive shell on the VM. |

From the tutorial [5]:

> If a virtual machine is not yet started, the test environment takes care
> of it on the first call of a method on a `machine` object.

Interactive debugging:

```shell-session
$ $(nix-build -A driverInteractive minimal-test.nix)/bin/nixos-test-driver
```

#### Multi-machine tests

Tests can involve multiple VMs, for example to test client-server
communication [5]:

```nix
pkgs.testers.runNixOSTest {
  name = "client-server-test";

  nodes.server = { pkgs, ... }: {
    networking.firewall.allowedTCPPorts = [ 80 ];
    services.nginx = {
      enable = true;
      virtualHosts."server" = {};
    };
  };

  nodes.client = { pkgs, ... }: {
    environment.systemPackages = with pkgs; [ curl ];
  };

  testScript = ''
    server.wait_for_unit("default.target")
    client.wait_for_unit("default.target")
    client.succeed("curl http://server/ | grep -o \"Welcome to nginx!\"")
  '';
}
```

#### Caching and re-running

From the tutorial [5]:

> Because test results are kept in the Nix store, a successful test is
> cached. This means that Nix will not run the test a second time as long as
> the test setup (node configuration and test script) stays semantically
> the same. Therefore, to run a test again, one needs to remove the result.

To re-run a cached test, remove the `result` symlink and delete the store
path:

```shell-session
result=$(readlink -f ./result) rm ./result && nix-store --delete $result
```

#### CI and hardware acceleration

From the tutorial [5]:

> Running integration tests on CI requires hardware acceleration, which many
> CIs do not support.

For GitHub Actions, see [how to disable hardware acceleration](https://github.com/cachix/install-nix-action#how-do-i-run-nixos-tests).

### runCommand: simple derivation tests

`runCommand` (and `runCommandCC`) create a derivation that runs a shell
script and produces `$out` on success. They are the simplest way to write a
check derivation — no `mkDerivation` phases, no `doCheck`, just a script
that either succeeds (produces `$out`) or fails (non-zero exit).

```nix
pkgs.runCommand "my-check" {
  nativeBuildInputs = [ pkgs.someTool ];
} ''
  someTool --version
  mkdir -p $out
  echo "check passed" > $out/result
''
```

`runCommandCC` is identical but adds a C compiler (`stdenv.cc`) to the
environment, for checks that need to compile a test program.

A `runCommand` check fails if the script exits non-zero; it passes if the
script produces `$out`. This makes it ideal for:

- Validating that a config file parses.
- Running a linter or formatter check.
- Asserting that a built binary produces expected output.

### testers: nixpkgs tester functions

The `pkgs.testers` attribute set provides reusable test-derivation
builders:

| Function | Purpose |
|----------|---------|
| `testers.runNixOSTest` | The NixOS VM test framework (see above). |
| `testers.testBuildFailure` | Asserts that a derivation *fails* to build (for testing error paths). |
| `testers.equalContents` | Asserts that two store paths have identical contents. |
| `testers.testEqualContents` | Same, as a check derivation. |
| `testers.runCommand` | Alias for `runCommand` with tester conventions. |

`testers.testBuildFailure` is useful for verifying that invalid input is
rejected:

```nix
testers.testBuildFailure {
  derivation = someDerivationThatShouldFail;
  expectedErrors = [ "expected error message" ];
}
```

### Property testing in derivations

Property-based testing (QuickCheck-style) is not a Nix-specific feature — it
runs inside `checkPhase` like any other test suite. The Nix role is to
ensure the test framework is available via `checkInputs` /
`nativeCheckInputs` and that `doCheck = true` is set.

For Rust crates built with `buildRustPackage`, `cargo test` (including
`proptest` / `quickcheck` tests) runs during `checkPhase` when
`doCheck = true` (the default for `buildRustPackage`). For Haskell
packages, `checkPhase` runs the test suite including QuickCheck properties.

## Practical rules

1. Set `doCheck = true` in derivations that have a test suite (unless
   cross-compiling) [3].
2. Put test-only tools on `$PATH` in `nativeCheckInputs`; put test-only
   libraries in `checkInputs` [4].
3. Use `checks.<system>.<name>` in the flake output for derivations that
   `nix flake check` should run [1].
4. Use `runCommand` for simple assertion-style checks (no phases needed).
5. Use `testers.runNixOSTest` for integration tests that need a running
   service or multi-machine interaction [5].
6. Use `passthru.tests` for tests that should not run on every build but
   should be available to `nix flake check` [3].
7. Run `nix build .#checks.x86_64-linux.<name>` to run a single check
   without running all checks.
8. NixOS VM tests require Linux + QEMU; they do not run on macOS without
   extra setup [5].
9. NixOS VM tests require hardware acceleration (KVM) in CI; many CI
   providers need it disabled [5].
10. Successful NixOS VM tests are cached in the Nix store; delete the
    `result` symlink and store path to re-run [5].
11. `nix flake check` evaluates *all* outputs, not just `checks` — an
    evaluation error in any output fails the command.
12. `checkPhase` runs in the build sandbox (no network); all test
    dependencies must arrive via derivations.

## Review checklist

- [ ] `doCheck = true` is set if the derivation has a test suite (and not
      cross-compiling).
- [ ] Test tools are in `nativeCheckInputs`; test libraries are in
      `checkInputs`.
- [ ] `checks.<system>` entries are derivations that fail on test failure.
- [ ] `runCommand` checks produce `$out` on success.
- [ ] NixOS VM tests have a descriptive `name`.
- [ ] NixOS VM test `testScript` uses `machine.succeed()` /
      `machine.fail()` / `machine.wait_for_unit()`.
- [ ] Multi-machine tests reference VMs by their `nodes` names.
- [ ] `nix flake check` passes (all outputs evaluate, all checks build).
- [ ] CI config has `experimental-features = nix-command flakes` if using
      flake commands.
- [ ] NixOS VM tests in CI have hardware acceleration configured or
      disabled.

## Implementation checklist

1. **Decide the test tier.** Unit tests → `checkPhase` / `doCheck`.
   Assertion checks → `runCommand` in `checks`. Integration tests →
   `testers.runNixOSTest` in `checks`.
2. **For derivation tests**, set `doCheck = true`, add `nativeCheckInputs`
   / `checkInputs`, and ensure the test suite runs via `make check` (or
   override `checkPhase`).
3. **For `runCommand` checks**, write a script that exits non-zero on
   failure and produces `$out` on success.
4. **For NixOS VM tests**, define `nodes`, write a `testScript` using
   `machine.succeed()` / `machine.fail()` / `machine.wait_for_unit()`.
5. **Add to `checks` output**: `checks.${system}.<name> = <derivation>;`.
6. **Run `nix flake check`** to validate all outputs and build all checks.
7. **Run individual checks** via `nix build .#checks.x86_64-linux.<name>`.
8. **Add a HOST-GATE note** if the check can only be verified on a host
   with nix. This container has no nix; any `nix flake check` claim here is
   based on documented Nix semantics, not runtime verification.

## Validation hooks

> **HOST-GATE:** This container has no nix. The commands below are
> documented Nix semantics, not runtime-verified in this environment.

Validate the flake and run all checks:

```bash
nix flake check                    # evaluate all outputs + build all checks
nix flake show                     # list all outputs (attribute tree)
```

Run an individual check:

```bash
nix build .#checks.x86_64-linux.<name>     # build one check
nix build .#checks.x86_64-linux.<name> --no-link  # no ./result symlink
```

Run a NixOS VM test interactively:

```bash
nix build .#checks.x86_64-linux.<name>.driverInteractive
# then run the driver:
./result/bin/nixos-test-driver
```

Run derivation tests in a dev shell:

```bash
just shell
cd "$(mktemp -d)"
export out=$(pwd)/out
phases="unpackPhase patchPhase configurePhase buildPhase checkPhase" genericBuild
```

CI with GitHub Actions [6]:

```yaml
- uses: cachix/install-nix-action@v25
  with:
    nix_path: nixpkgs=channel:nixos-unstable
- run: nix flake check
```

## Examples

### Example 1: Minimal doCheck derivation (from crawl 66)

A derivation with `doCheck = true` and `nativeCheckInputs`, from the
nixpkgs manual's `solo5` example [3]:

```nix
stdenv.mkDerivation (finalAttrs: {
  pname = "solo5";
  version = "0.7.5";

  src = fetchurl {
    url = "https://github.com/Solo5/solo5/releases/download/v${finalAttrs.version}/solo5-v${finalAttrs.version}.tar.gz";
    hash = "sha256-viwrS9lnaU8sTGuzK/+L/PlMM/xRRtgVuK5pixVeDEw=";
  };

  nativeBuildInputs = [ makeWrapper pkg-config ];
  buildInputs = [ libseccomp ];

  doCheck = true;
  nativeCheckInputs = [ util-linux qemu ];
  # checkPhase elided
})
```

### Example 2: Minimal NixOS VM test (from crawl 25)

The `minimal-test` from the nix.dev tutorial [5]:

```nix
pkgs.testers.runNixOSTest {
  name = "minimal-test";

  nodes.machine = { config, pkgs, ... }: {
    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      packages = with pkgs; [ firefox tree ];
    };
    system.stateVersion = "23.11";
  };

  testScript = ''
    machine.wait_for_unit("default.target")
    machine.succeed("su -- alice -c 'which firefox'")
    machine.fail("su -- root -c 'which firefox'")
  '';
}
```

### Example 3: Multi-machine NixOS VM test (from crawl 25)

The `client-server-test` from the nix.dev tutorial [5]:

```nix
pkgs.testers.runNixOSTest {
  name = "client-server-test";

  nodes.server = { pkgs, ... }: {
    networking.firewall.allowedTCPPorts = [ 80 ];
    services.nginx = {
      enable = true;
      virtualHosts."server" = {};
    };
  };

  nodes.client = { pkgs, ... }: {
    environment.systemPackages = with pkgs; [ curl ];
  };

  testScript = ''
    server.wait_for_unit("default.target")
    client.wait_for_unit("default.target")
    client.succeed("curl http://server/ | grep -o \"Welcome to nginx!\"")
  '';
}
```

### Example 4: runCommand check (from the repo)

The project's `checks.validateConfig` from `/flake.nix` [7], which uses
`runCommand` to validate a workestrate TOML config:

```nix
checks.validateConfig = { pkgs, config, workestrate }:
  let
    # config is the TOML text (string). A config-repo flake can pass
    # builtins.readFile ./workestrate.toml directly.
    configFile = pkgs.writeText "workestrate.toml" config;
  in
  pkgs.runCommand "validate-config" {
    nativeBuildInputs = [ workestrate ];
    passAsFile = [ ];
  } ''
    mkdir -p $out
    cp ${configFile} workestrate.toml
    workestrate validate-config
    touch $out/ok
  '';
```

This check:

1. Writes the TOML config text to a store file via `writeText`.
2. Runs `workestrate validate-config` — if it exits non-zero, the
   derivation fails.
3. Produces `$out/ok` on success.

The `referenceConfig` it validates is parsed in `/nix/lib/config.nix` [8]
via `builtins.fromTOML (builtins.readFile configPath)`, exposing
`workloadNames`, `nixLayeredImages`, and `localBuilds` for flake-level
evaluation.

### Example 5: Simple runCommand assertion

A standalone `runCommand` check that asserts a built binary works:

```nix
checks.${system}.smoke-test = pkgs.runCommand "smoke-test" {
  nativeBuildInputs = [ myPackage ];
} ''
  my-binary --version | grep "1\.0\.0"
  mkdir -p $out
''
```

## Common mistakes

### 1. Forgetting doCheck = true

The check phase is skipped by default [3].

```nix
# WRONG (tests silently skipped):
{ ... }

# CORRECT:
{
  doCheck = true;
  # ...
}
```

### 2. Putting test tools in buildInputs instead of nativeCheckInputs

Test tools needed on `$PATH` go in `nativeCheckInputs`, not `buildInputs`
[4]. Putting them in `buildInputs` works natively but breaks
cross-compilation and pollutes runtime dependencies.

### 3. Not adding checks to the checks output

A derivation with `doCheck = true` runs tests during `nix build`, but
`nix flake check` only runs derivations listed in `checks.<system>`. To
make a test visible to `nix flake check`, add it to `checks`.

### 4. Expecting nix flake check to only run checks

`nix flake check` evaluates *all* outputs (packages, devShells, apps,
lib, checks). An evaluation error in any output fails the command, even if
the `checks` themselves are fine.

### 5. Running NixOS VM tests without KVM in CI

NixOS VM tests use QEMU. Without hardware acceleration (KVM), they are
extremely slow or fail. In GitHub Actions, disable hardware acceleration
[5].

### 6. Not deleting cached NixOS test results

Successful NixOS VM tests are cached in the Nix store. Re-running
`nix-build` returns the cached result without re-executing the test. Delete
the `result` symlink and store path to force a re-run [5].

### 7. Referencing a VM by the wrong name in testScript

Each VM is accessible in the `testScript` by its `nodes` key name. If the
node is named `server`, use `server.succeed(...)`, not
`machine.succeed(...)`.

### 8. Forgetting experimental-features in CI

Flake commands (`nix flake check`, `nix build .#...`) require
`experimental-features = nix-command flakes`. If CI doesn't set this, the
commands fail [1].

## Strict vs contextual guidance

| Rule | Strict (always) | Contextual (depends) |
|------|-----------------|----------------------|
| `doCheck = true` when tests exist | — | Contextual (strict if tests exist and aren't broken; skipped under cross-compilation) |
| Test tools in `nativeCheckInputs` | Strict | — |
| `checks` output for `nix flake check` visibility | Strict | — |
| `runCommand` for simple assertions | — | Contextual (preferred for simple checks; `mkDerivation` for complex) |
| `testers.runNixOSTest` for integration tests | — | Contextual (only when a running service/VM is needed) |
| `experimental-features` in CI | Strict | — |
| KVM for NixOS VM tests | — | Contextual (required for speed; can be disabled in CI) |
| `passthru.tests` for non-build-critical tests | — | Contextual (preferred for tests that shouldn't slow every build) |

## Policy decisions for individual repos

The `ai-workbench` repo applies the general rules above via the following
repo-specific mechanisms:

- **`checks.validateConfig`**: exposed via `libForSystem` in `flake.nix`
  [7], not directly in a top-level `checks` output. A config-repo flake
  calls `lib.${system}.checks.validateConfig { inherit pkgs config workestrate; }`
  to produce a `runCommand` derivation that validates its TOML config. This
  pattern lets the core flake stay pure while config repos own their own
  checks.
- **`doCheck = false` in `agentctl.nix`**: the `workestrate` Rust package
  explicitly disables `doCheck` because the test suite requires a running
  Microsandbox daemon (not available in the build sandbox). Tests are run
  via `just` recipes instead.
- **HOST-GATE convention**: This container has no nix; all `nix flake
  check` and `nix build .#checks.*` claims are based on documented Nix
  semantics, not runtime verification.

## Related docs

- `/docs/nix/flake-anatomy.md` — Flake structure, including the `checks`
  output type.
- `/docs/nix/derivations-and-builds.md` — Derivation build phases, including
  `checkPhase` and `doCheck`.
- `/docs/nix/source-map.md` — Nix source map (provenance index).

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference.

## Citations

1. [Flakes — nix.dev](https://nix.dev/concepts/flakes.html)
2. [NixOS virtual machines — nix.dev](https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html)
3. [nixpkgs — stdenv and mkDerivation (Phases)](https://nixos.org/manual/nixpkgs/stable/#sec-stdenv-phases)
4. [nixpkgs — Specifying dependencies](https://nixos.org/manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview)
5. [Integration testing with NixOS virtual machines — nix.dev](https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html)
6. [Continuous integration with GitHub Actions — nix.dev](https://nix.dev/guides/recipes/continuous-integration-github-actions.html)
7. Local: `/flake.nix` — `checks.validateConfig` (via `libForSystem`)
8. Local: `/nix/lib/config.nix` — `referenceConfig` (parsed TOML config)
