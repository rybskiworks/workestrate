---
name: nix-testing
description: |
  Operational guide for writing and running Nix tests — nix flake check, the
  checks output, checkPhase/doCheck, nixosTests/testers.runNixOSTest, runCommand
  checks, and derivation testing. Load when writing or running Nix tests, adding
  check derivations, enabling checkPhase, writing NixOS VM tests, or running
  nix flake check. Distilled from docs/nix/testing.md; consult that doc for full
  detail and source URLs.
---

# Nix Testing

Distilled from [`docs/nix/testing.md`](../../../docs/nix/testing.md). That doc
holds the canonical rules and upstream source links; this skill is the
actionable subset.

## Triggers

Load this skill when:

- Writing or running Nix tests, or adding `checks.*` flake outputs.
- Enabling `checkPhase` / `doCheck` in a derivation.
- Writing `nixosTest` / `testers.runNixOSTest` VM tests.
- Writing `runCommand` assertion checks.
- Running `nix flake check` (locally or in CI).
- Choosing between test tiers (flake / derivation / integration).

## Testing Hierarchy

| Level | Mechanism | What it tests | Cost |
|---|---|---|---|
| Flake | `nix flake check` + `checks` output | Evaluates all outputs; builds `checks` derivations | Low–medium |
| Derivation | `checkPhase` / `doCheck` | Runs the package's own test suite during the build | Medium |
| Integration | `nixosTests` / `testers.runNixOSTest` | Boots QEMU VMs and runs a Python test script | High |

## checkPhase Variables

| Variable | Default | Description |
|---|---|---|
| `doCheck` | `false` | Whether the check phase runs. Set `doCheck = true` to enable. Skipped under cross-compilation. |
| `checkTarget` | `check` (or `test`) | The `make` target that runs tests. |
| `checkFlags` / `checkFlagsArray` | `[]` | Flags passed to `make` during check only. |
| `checkInputs` | `[]` | Host deps used by check (libraries linked into test executables). |
| `nativeCheckInputs` | `[]` | Native deps used by check (tools on `$PATH`, setup hooks). |
| `preCheck` | — | Hook at the start of the check phase. |
| `postCheck` | — | Hook at the end of the check phase. |

## checks Output

The `checks.<system>.<name>` output holds test derivations that `nix flake
check` builds. Each entry is a standard derivation — if it builds, the check
passes; if the build fails, the check fails. A `checks` entry can be a
`runCommand`, a `mkDerivation` with `doCheck = true`, a `nixosTest`, or any
derivation whose build success/failure encodes a test result.

Run an individual check without running all of them:

```bash
nix build .#checks.x86_64-linux.<name>
```

## runCommand Checks

The simplest check form — a derivation that runs a script and produces `$out`
on success (non-zero exit = failure):

```nix
checks.${system}.smoke-test = pkgs.runCommand "smoke-test" {
  nativeBuildInputs = [ myPackage ];
} ''
  my-binary --version | grep "1\.0\.0"
  mkdir -p $out
'';
```

## NixOS VM Tests

`testers.runNixOSTest` boots QEMU VMs from NixOS configurations and runs a
Python test script. Required fields: `name`, `nodes` (set of NixOS configs),
`testScript` (Python, accesses VMs by their `nodes` keys).

Machine object methods:

| Method | Purpose |
|---|---|
| `machine.start()` | Start a specific VM. |
| `start_all()` | Start all VMs. |
| `machine.wait_for_unit("default.target")` | Wait until systemd reaches the given unit. |
| `machine.succeed("command")` | Run a command; fail the test if it exits non-zero. |
| `machine.fail("command")` | Run a command; fail the test if it exits zero. |
| `machine.shell_interact()` | Enter an interactive shell on the VM. |

Multi-machine pattern: define multiple `nodes` keys and reference each by name
in the `testScript` (e.g. `server.wait_for_unit(...)`, `client.succeed(...)`).

Caching: successful VM tests are cached in the Nix store; delete the `result`
symlink and store path to re-run. KVM requirement: VM tests need Linux + QEMU;
in CI they require hardware acceleration (KVM) or it must be disabled.

## testers Functions

| Function | Purpose |
|---|---|
| `testers.runNixOSTest` | The NixOS VM test framework. |
| `testers.testBuildFailure` | Asserts a derivation *fails* to build (error paths). |
| `testers.equalContents` | Asserts two store paths have identical contents. |
| `testers.testEqualContents` | Same, as a check derivation. |
| `testers.runCommand` | Alias for `runCommand` with tester conventions. |

## Practical Rules

1. Set `doCheck = true` in derivations that have a test suite (unless cross-compiling).
2. Put test-only tools on `$PATH` in `nativeCheckInputs`; test-only libraries in `checkInputs`.
3. Use `checks.<system>.<name>` in the flake output for derivations `nix flake check` should run.
4. Use `runCommand` for simple assertion-style checks (no phases needed).
5. Use `testers.runNixOSTest` for integration tests that need a running service or multi-machine interaction.
6. Use `passthru.tests` for tests that should not run on every build but should be available to `nix flake check`.
7. Run `nix build .#checks.x86_64-linux.<name>` to run a single check.
8. NixOS VM tests require Linux + QEMU; they do not run on macOS without extra setup.
9. NixOS VM tests require KVM in CI; many CI providers need it disabled.
10. Cached NixOS VM tests need the `result` symlink + store path deleted to re-run.
11. `nix flake check` evaluates ALL outputs, not just `checks` — an eval error in any output fails the command.
12. `checkPhase` runs in the build sandbox (no network); all test deps must arrive via derivations.

## Review Checklist

- [ ] `doCheck = true` is set if the derivation has a test suite (and not cross-compiling).
- [ ] Test tools in `nativeCheckInputs`; test libraries in `checkInputs`.
- [ ] `checks.<system>` entries are derivations that fail on test failure.
- [ ] `runCommand` checks produce `$out` on success.
- [ ] NixOS VM tests have a descriptive `name`.
- [ ] VM `testScript` uses `machine.succeed()` / `machine.fail()` / `machine.wait_for_unit()`.
- [ ] Multi-machine tests reference VMs by their `nodes` names.
- [ ] `nix flake check` passes (all outputs evaluate, all checks build).
- [ ] CI config has `experimental-features = nix-command flakes` if using flake commands.
- [ ] NixOS VM tests in CI have hardware acceleration configured or disabled.

## Validation Commands

> **HOST-GATE:** This container has no nix. The commands below are documented
> Nix semantics, not runtime-verified in this environment.

```bash
nix flake check                                       # eval all outputs + build all checks
nix flake check --no-build                            # eval only (faster)
nix flake show                                        # list all outputs (attribute tree)
nix build .#checks.x86_64-linux.<name>                # build one check
nix build .#checks.x86_64-linux.<name>.driverInteractive  # interactive VM test driver
```

## Common Mistakes

1. Forgetting `doCheck = true` — the check phase is skipped by default.
2. Putting test tools in `buildInputs` instead of `nativeCheckInputs` — breaks cross-compilation.
3. Not adding checks to the `checks` output — `nix flake check` only runs derivations listed in `checks.<system>`.
4. Expecting `nix flake check` to only run checks — it evaluates ALL outputs.
5. Running NixOS VM tests without KVM in CI — extremely slow or fails.
6. Not deleting cached NixOS test results — re-running returns the cached result.
7. Referencing a VM by the wrong name in `testScript` — use the `nodes` key name.
8. Forgetting `experimental-features = nix-command flakes` in CI — flake commands fail.

## Strict Rules

- Test tools go in `nativeCheckInputs`; test libraries in `checkInputs`.
- `checks` output is the only mechanism for `nix flake check` visibility.
- `experimental-features = nix-command flakes` is required for flake commands in CI.
- `checkPhase` runs in the sandbox with no network access.

## Related Docs

- Full reference: [`docs/nix/testing.md`](../../../docs/nix/testing.md) (canonical; upstream source URLs there).
- [`docs/nix/flake-anatomy.md`](../../../docs/nix/flake-anatomy.md) — the `checks` output type.
- [`docs/nix/derivations-and-builds.md`](../../../docs/nix/derivations-and-builds.md) — `checkPhase` and `doCheck`.

## Related Skills

- [`nix-usage`](../nix-usage/SKILL.md) — project flake, dev shell, Rust toolchain reference.
- [`nix-ci-cd`](../nix-ci-cd/SKILL.md) — `nix flake check` in CI, GitHub Actions, Cachix.
