# NixOS images with Workestrate

The core flake supplies `packages.x86_64-linux.workestrate-nixos-base`: a
headless NixOS/Lix image with the exact `packages.x86_64-linux.workestrate`
CLI and its paired Microsandbox runtime. The common
`nixosModules.workestrate` module also installs that package on a physical
NixOS host. Host hardware, desktop, users and secrets belong to the deployment
configuration, while fleet roles own workload packages and services.

Both receive the package-paired `workestrate-init-state` helper. It initializes
only a fresh canonical user runtime home when explicitly invoked; see
[NixOS installation](install/nixos.md#prepare-operator-state). Existing state,
custom backend homes and generation migration are outside that command.

The dependency direction is tooling → core → deployment/fleets. Tooling owns
generic NixOS/Lix/image construction; core adds its package. No tooling input
depends on a consumer's Workestrate image or configuration.

## Extend the image

Use the same tooling authority as core and its registered image constructor:

```nix
inputs.tooling.lib.guest.mkNixosLayer {
  inherit pkgs;
  base = inputs.workestrate.packages.x86_64-linux.workestrate-nixos-base;
  name = "example-workload";
  registrationName = "example";
  maxLayers = 100;
  contents = [ pkgs.hello ];
  config.Cmd = [ "${pkgs.hello}/bin/hello" ];
}
```

This extends the existing image and retains its NixOS system, init contract,
Lix and Workestrate derivations. A role needing different NixOS service options
can call `tooling.lib.guest.mkNixosImage` with the common Workestrate module,
`tooling.nixosModules.lixGuest` and an explicit
`programs.workestrate.package = inputs.workestrate.packages.${system}.workestrate`.
Pass the package itself instead of rebuilding it with the role's package set.

## Share the existing package closure

`packages.x86_64-linux.workestrate-closure` describes the complete closure of
that same Workestrate output. It supplies `manifest.json`, `store-paths`,
`registration` and `roots`; retaining the export also retains its admitted
store paths. The manifest accepts directory and regular-file store outputs,
and rejects top-level symlinks and special files instead of following them.

`packages.x86_64-linux.workestrate-nixos-shared-base` has the same NixOS system
and Workestrate package but omits those admitted paths from the image's store
layers. Every manifest path must be available read-only at its original
`/nix/store/...` path **before `/init` runs**. An extending `mkNixosLayer`
inherits that external-closure requirement. The ordinary base remains
self-contained and requires no store-sharing mounts.

The shared image is not standalone. Its deployment owner must retain the
export with a GC root for the entire lifetime of every referencing guest,
create exact read-only mounts, and verify mount coverage and package identity
before launch. Each guest still owns its writable Nix database and new store
outputs. Do not mount the host database, daemon socket or entire mutable store.
Changing the admitted package means a new image/export pair and an explicit
transition; it does not rewrite a running guest's mounts.

These outputs provide image composition, not an automatic mount/lease manager.
Native acceptance must demonstrate image payload omission, guest registration,
rejected writes, host retention, and restart with the same immutable paths.

## Launch contract

The image contains a NixOS userspace and stage-2 `/init`; Microsandbox supplies
its kernel. Declare the existing handoff explicitly in the workload capsule:

```toml
[init]
mode = "handoff"
cmd = "/init"
args = []
env = { container = "microsandbox" }
```

Use a pre-existing initial workdir such as `/`. The runtime prepares networking,
mounts and `/run` before transferring PID 1 to NixOS. The guest imports its
immutable image registration into a private Nix database before starting Lix.
This guest adapter is not the physical host's boot or networking configuration.

Workestrate's presence in an image does not grant nested KVM, host credentials,
the host daemon socket or host database access. Require nested virtualization
only for a role that will launch inner VMs, and qualify that capability against
the exact outer runtime and firmware.

## Verification

`just verify` includes `checks.x86_64-linux.nixosModules`, which evaluates
disabled/common/runtime configurations, preserves physical host defaults,
checks explicit package replacement, compares host/guest CLI, runtime and
Lix identities, and checks the shared image's export/root contract. It does not
realize the guest image or boot a VM.

Build the selected image separately, then test it through the ordinary
`workestrate --fleet <name> workload ...` interface with disposable state and
synthetic credentials. Check NixOS activation, Lix readiness, package/runtime
versions, a guest build, shutdown and restart. A package or evaluation check
does not establish those runtime properties. See
[runtime provisioning](runtime-provisioning.md) for state and image transitions.
