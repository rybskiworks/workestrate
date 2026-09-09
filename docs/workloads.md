# Workload flakes and repositories

A workload's configuration and image build belong together. A separate Git
repository is optional: a fleet can keep multiple workload capsules, each with
its own flake, or a standalone config repository can own one workload.

## Supported layouts

Standalone config repository:

```text
flake.nix
flake.lock
workestrate.toml     # schema_version plus [workloads.<name>]
image.nix
```

Embedded capsules in a directory-mode fleet:

```text
workestrate/
  default.toml      # the sole schema_version declaration
  workloads/
    example/
      workload.toml # bare workload fields, no schema_version
      flake.nix
      flake.lock
      image.nix
```

For `image.recipe = "nix-layered"`, `image.name` selects the image output, usually
`packages.<system>.<image.name>`. The CLI searches for the nearest `flake.nix`
starting beside the source file declaring the image. A capsule-local flake wins;
an existing fleet-root flake remains the fallback. Image overrides use the
overriding layer's source directory, and inline config refs use archived files.
`flake://` local-build sources likewise use their declaring source's flake lock.
Local-build artifact mounts follow that local-build declaration even when a
different layer overrides the image with its own flake.

Mount and seed paths do **not** change with image ownership. In directory mode,
relative paths remain rooted at `workestrate/`, so an existing
`source = "workloads/example/settings.json"` stays valid. In file mode they are
relative to `workestrate.toml`'s directory. Repository identities and runtime
dependency namespaces also remain unchanged when adding a capsule-local flake.

Every flake must include its lock file. Image evaluation and builds refuse
implicit lock updates; deliberately change pins with `nix flake lock`, review
the result, and validate with `nix flake check --no-update-lock-file`. When working
inside Git, add new files before asking Nix to build them.

Registry-image workloads retain registry acquisition: merely adding a flake does
not switch them to a Nix-built image. Explicitly choose `nix-layered` and the
matching package name when migrating that behavior.

## Examples and validation

- [Baseline](../examples/workloads/baseline/README.md): a reusable, minimal
  workload template with a named image, exact tooling pin, lock and static check.
- [Communication fixture](../examples/workloads/comms/README.md): two services
  sharing one image, exercising a required dependency and a narrow local port.
- [Nested fixture](../examples/workloads/nested/README.md): a pinned Workestrate
  runtime launches, execs into, and removes a child VM inside a workload. This
  requires nested host KVM and build-time access to the private source input;
  it does not mount the host store or Nix daemon into the guest.

Build and check each flake without entering a development shell. Validate all
example configs against a particular installed CLI using disposable homes:

```sh
python3 examples/workloads/validate-config.py /absolute/path/to/workestrate
```

For a real host-KVM test, explicitly select the CLI and Nix binaries:

```sh
python3 examples/workloads/smoke.py baseline --run \
  --workestrate /absolute/path/to/workestrate --nix /absolute/path/to/nix
python3 examples/workloads/smoke.py comms --run \
  --workestrate /absolute/path/to/workestrate --nix /absolute/path/to/nix
```

The runner builds/imports through Workestrate, checks guest markers, and stops
only its exact test instances. It creates fresh short paths under `/tmp`, clears
ambient backend/home/credential variables, supplies an empty Microsandbox JSON
config, and retains logs and state for diagnosis. Commands and readiness waits
are bounded. Inspect any reported cleanup failure before removing test artifacts.
`python3 examples/workloads/test_smoke.py` checks isolation and cleanup behavior
without KVM or launching a process through Workestrate.

Static image checks and config validation do not start VMs. Runtime tests require
an explicitly selected isolated Workestrate home and microsandbox store. The
examples name their expected markers and teardown order; no personal mounts or
credentials belong in these fixtures. Keep an explicit guest `workdir`: the
default `/app` need not exist in a minimal image.

## Not yet implemented

A fleet cannot yet import and pin standalone workload repositories. Registering
another whole config repository is supported, but is not a fleet-level workload
import: it can change precedence, identity and dependency namespaces. A bare root
`workload.toml` is not a standalone config-repository layout today.

The current `workload new` scaffolder still targets the older file-mode layout;
do not use it to populate an existing directory-mode fleet. Copy/adapt the
baseline example or create a capsule explicitly until the scaffolder is
layout-aware. Do not relocate existing state, rewrite user changes, or delete old
image records as part of adding flakes. The no-workload-flake builder proposal in
ADR 0038 is not the interface described here.
