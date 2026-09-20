# Workload flakes and repositories

A workload's configuration and image build belong together. A separate Git
repository is optional: a fleet can keep multiple workload capsules, each with
its own flake, or a standalone fleet can own one workload.

## Supported layouts

Standalone fleet:

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

## Workload ownership and validation

Runnable examples, application packages and their black-box tests belong in
fleet or workload repositories. The baseline, dependency-communication and
nested-Workestrate fixtures are maintained under `tests/workloads/` in the
[personal fleet repository](https://github.com/rybskiworks/workestrate-fleet-georgrybski).
Each fixture has its own locked flake, image check and operating instructions.
Copy a complete fixture directory, including its lock, to use it independently.

Workestrate retains generic configuration, provenance, policy and orchestration
regressions. `just verify` does not require a fleet checkout, build application
images or invoke a fleet's VM suite. The external runner accepts an explicitly
selected Workestrate binary; it is not a dependency of the tool's build.

Keep static image checks, disposable configuration validation and opt-in VM
tests separate. Runtime tests need fresh tool, state and Microsandbox roots,
an explicitly empty backend configuration, bounded readiness and exact teardown.
They must not inherit personal mounts or credentials. Image builds alone do not
prove startup, guest service health, nested confinement or cleanup. Set an
explicit guest `workdir`: the default `/app` need not exist in a minimal image.

## Not yet implemented

A fleet cannot yet import and pin standalone workload repositories. Registering
another whole fleet is supported, but is not a fleet-level workload
import: it can change precedence, identity and dependency namespaces. A bare root
`workload.toml` is not a standalone fleetsitory layout today.

The current `workload new` scaffolder still targets the older file-mode layout;
do not use it to populate an existing directory-mode fleet. Copy/adapt the
baseline example or create a capsule explicitly until the scaffolder is
layout-aware. Do not relocate existing state, rewrite user changes, or delete old
image records as part of adding flakes. The no-workload-flake builder proposal in
ADR 0038 is not the interface described here.
