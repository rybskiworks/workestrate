# Nix builds and dependency ownership

`nix-tooling` owns the shared nixpkgs, Fenix, devenv and formatting input
versions. Workestrate follows those inputs, including for its Microsandbox
dependency. Compiler and formatter packages use the same Fenix toolchain.

The `microsandbox-fork` flake owns the `microsandbox` and static `agentd`
packages. Workestrate re-exports those packages and compiles against the SDK
source from that package's filtered Rust workspace. Nix evaluation asserts
that their versions match the SDK pins in `control/agentctl/Cargo.toml`.

`control/agentctl/.cargo/config.toml` supplies the SDK path patches for both
development and Nix builds, including the fork's exact `msb-vm-memory` Git
patch. Cargo does not inherit a dependency workspace's patches, so updating
the fork requires checking these root patches as well as the version pins.
The `vendor/microsandbox-fork` path is a symlink
to the pinned source, not a checked-in copy of third-party crates. The
`microsandbox-filesystem-patched` package output remains a compatibility
alias for that source.

Cargo registry and Git dependencies are fetched through Nix's locked Cargo
dependency support before compilation. Git sources have explicit output
hashes. Build phases use explicit Nix-managed runtime artifacts; they must not fetch
dependencies from the network. The native libkrun Rust crates are selected
by `Cargo.lock`; they are not replaced by building a standalone libkrun C
library. Workestrate imports the pinned Microsandbox input's `nix/cargo-lock.nix`
helper with its own lockfile, so the fork owns the hashes of its Git sources
instead of duplicating those hashes here. Firmware selection belongs to the
Microsandbox package.

## Build and development entry points

```sh
nix build .#workestrate
nix build .#checks.x86_64-linux.package .#checks.x86_64-linux.unit
just verify
just bootstrap
just bootstrap -c rustc --version
just shell
```

`just bootstrap` enters a tooling-only devenv shell without requiring a
working application or Microsandbox build. It does not stage a runtime,
change the SDK symlink, install Git hooks or mark runtime setup complete.
Use it for Nix and source investigation; it does not configure SDK build inputs.

`just sdk-prepare` explicitly creates or refreshes only the SDK source symlink
from the locked `microsandbox-filesystem-patched` source alias. It does not build
Workestrate or the Microsandbox runtime, enter the default shell, run Cargo,
or update either lockfile. An existing unlocked vendor directory is preserved
and refused; resolve it explicitly rather than overwriting local edits.

To refresh Workestrate's own Cargo lock after an intentional fork pin update:

```sh
just sdk-prepare
just bootstrap -c bash -c 'cd control/agentctl && cargo metadata --offline --format-version 1'
just bootstrap -c bash -c 'cd control/agentctl && cargo metadata --locked --offline --format-version 1'
just lock-guard
git diff -- control/agentctl/Cargo.lock
```

The first metadata command permits the required lock changes; the second
confirms resolution without further changes. Run from `control/agentctl` so
Cargo loads its SDK path patches. Metadata does not compile or execute build
scripts, so it needs no runtime artifacts or default-shell marker. Offline
resolution still requires the selected Git objects and registry dependencies
to be cached; a missing source is a separate acquisition prerequisite, not a
reason to use an old SDK or copy the fork's lockfile. Review the resulting diff
for only the intended source/dependency changes, with no unrelated registry
updates. `just sdk-source-check` tests the preparation contract without Nix.

`just shell` provides the full development environment and the SDK symlink.
`MSB_BUILD_RUNTIME` points the SDK build script at the immutable runtime
package independently of `MSB_HOME`, which remains a runtime-state selector.
An invalid explicit build runtime fails without downloading a replacement.
Shell entry does not build workloads, clean runtime directories, install
hooks or format the checkout. Both entry points pass an explicit worktree
root to devenv for pure evaluation and keep Cargo outputs outside the source tree.

Shell entry points preserve command argument boundaries, including quoted
shell programs and empty arguments. `just shell-arguments-check` exercises
that contract without entering devenv.

To test an unpublished fork packaging change locally:

```sh
nix build .#workestrate --no-link --no-write-lock-file \
  --override-input microsandbox-fork /absolute/path/to/microsandbox
```

Use a Git checkout reference so ignored build artifacts stay outside the
flake source. Add new required files to Git's index before evaluation. A
successful local override build does not prove that the unchanged remote
pin contains those fixes; publish and pin the reviewed fork change before
claiming a reproducible remote build.

`just versions-check`, `just lock-guard` and `just lint-nix` check version,
Cargo source and purity invariants. `just verify` is the broader
validation gate. Neither building nor entering either shell runs
state migration or starts sandboxes.

`just check` runs the sandboxed `checks.x86_64-linux.rust` gate: formatting,
Clippy with warnings denied, and Cargo checking with locked, offline
dependencies. It does not enter the runtime-aware shell, so Git's pre-push
check is independent of interactive development setup.

`just verify` builds the sandboxed checks directly, without entering devenv or
reading consumer homes. Its schema-copy check compares only repository-owned
template files; deployed consumers can be inspected separately with
`workestrate --home <tool-home> schemas update --check`. The store audit is
informational and requests closure sizes explicitly.

`nix run .#install-hooks` explicitly installs the flake-managed Git hooks
without entering devenv. The generated configuration is protected from Nix
garbage collection. Existing custom legacy hooks remain part of the chain;
stale generated hooks pointing to removed tools may need a backed-up local
repair before reinstalling.

`just hooks-check` verifies legacy hook chaining and the unconditional secret
guard in isolated Git fixtures.

The package check verifies the installed CLI, runtime pairing and reported
fork revision against the immutable flake input without creating runtime
state. It requires a revision-bearing fork input; an uncommitted local override
can exercise the package build but does not certify published provenance.
The unit check reuses the package's offline build
environment with test fixtures and an isolated home. Existing ignored KVM
tests, daemon-backed Nix image tests and optional Copier round trips remain
separate host gates; a passing sandbox test check is not a VM deployment test.
The unit derivation omits development/test debug symbols to bound temporary
artifact size. Debug assertions, test selection, optimization defaults and the
production package profile are unchanged; interactive Cargo keeps its defaults.

The unit output also retains `bin/workestrate-native-fixture`, a wrapper for the
library test executable reported by the original Cargo test invocation. The
existing test hook runs once through a joined log capture; packaging selects
exactly its own library-test artifact and stages that file before installation.
It does not invoke Cargo again or guess a test executable from directory names.
The wrapper selects the pinned Microsandbox runtime and agentd
and requires an explicit `MSB_HOME`; it never defaults to deployed state.
Ignored native VM tests still require a separate, explicitly selected host run
with disposable state, image inputs and KVM. Merely building or listing the
test executable does not boot a VM or validate broker readiness. The production
Workestrate package does not include this test executable.

The build script tracks `WORKESTRATE_REV` explicitly, so a retained Cargo target
refreshes the compiled version when that variable changes or is removed. Plain
Cargo builds without it report the `dev` suffix; production Nix builds retain
the selected source revision. The dependency-free `buildRevision` check copies
the real script into a tiny fixture, rebuilds the same target for two revisions
and then the unset variable, and confirms that an unchanged repeat stays fresh.
A negative control without environment tracking demonstrates the stale-version
failure. Run it independently with
`nix build --no-link --no-update-lock-file .#checks.x86_64-linux.buildRevision`;
it also runs under `just verify`, without compiling the application.
