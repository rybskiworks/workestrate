# NixOS

[Installation](README.md) · [Linux with Nix](linux.md) · [First workload](../getting-started.md)

Install Workestrate declaratively on an x86_64 NixOS host. The same package
can be installed in a physical host and a NixOS microVM; its wrapped CLI keeps
the exact Microsandbox runtime and agentd selected by the core flake.

## Configure the host

Add the core flake to your deployment flake and commit the resulting lockfile.
This example uses the core's shared package authority and Lix profile. Keep
your existing hardware configuration, bootloader and `system.stateVersion`.

```nix
{
  inputs.workestrate.url = "github:rybskiworks/workestrate";
  inputs.tooling.follows = "workestrate/tooling";
  inputs.nixpkgs.follows = "tooling/nixpkgs";

  outputs = inputs: {
    nixosConfigurations.workstation = inputs.nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        inputs.tooling.nixosModules.lixSystem
        inputs.workestrate.nixosModules.workestrateHost
        {
          programs.workestrate.runtime = {
            enable = true;
            users = [ "alice" ];
          };
        }
      ];
    };
  };
}
```

Replace `alice` with an existing account. The runtime module enables the CLI,
adds the account to `kvm`, grants that group read/write access to `/dev/kvm`,
and loads the generic KVM module on a physical host. Your hardware configuration
still owns the Intel/AMD driver, firmware virtualization and nested-virtualization
settings. It does not enable a networking service or change the host firewall.

For CLI/configuration work without local microVM execution, import
`nixosModules.workestrate` and set `programs.workestrate.enable = true` instead.
The common module installs the CLI, its explicit state initializer, Git and tar
without changing kernel, networking, user groups or the selected Nix daemon.

Apply your configuration using your usual deployment workflow. For a standard
deployment flake in the current directory:

```sh
sudo nixos-rebuild switch --flake .#workstation
```

The [NixOS rebuild guide](https://wiki.nixos.org/wiki/Nixos-rebuild) covers other
configuration layouts. Log in again to pick up group membership, then verify
that your user can read and write `/dev/kvm`.

## Keep host and guest packages identical

The module's `programs.workestrate.package` defaults to this core flake's
`packages.x86_64-linux.workestrate`. Consumers can pass that same derivation
explicitly. The `workestrate-nixos-base` image installs it in a NixOS/Lix system:

```nix
programs.workestrate.package = inputs.workestrate.packages.x86_64-linux.workestrate;
```

An overlay may expose that existing package under another attribute. Rebuilding
the CLI with a different package set produces a different derivation and is not
needed to customize desktop, hardware or workload packages. Compare both
`drvPath` and `outPath` when qualifying a deployment; the package's
`microsandbox` attribute exposes the paired runtime derivation.

The [NixOS guest image contract](../nixos-images.md) covers layering, init and
the separate runtime acceptance gates.

## Prepare operator state

System activation installs software and KVM access only. It does not clone a
fleet, decrypt secrets, initialize or migrate runtime state, stop workloads, or
grant an account Nix daemon trust. Keep the operator's configuration and writable
state outside the Nix store.

For a fresh account with no `~/.microsandbox` path and no explicit `MSB_HOME`,
run the package-paired initializer before the first workload:

```sh
workestrate-init-state --dry-run
workestrate-init-state
```

It creates private `~/.microsandbox/generations/<runtime-key>` directories and a
relative `current` symlink for the exact runtime paired with the installed
package. It invokes no VM or database command and refuses any existing root,
including an empty directory or dangling symlink. Interrupted initialization
leaves its private partial tree for inspection rather than deleting state.
The same command is available as the core flake's `init-state` app.
Initialization does not validate future workload socket paths; their actual
endpoint length depends on the backend and instance name, as described in
[runtime provisioning](../runtime-provisioning.md#msb-state-generations).

Existing installations require the separate, quiesced generation procedure in
[runtime provisioning](../runtime-provisioning.md). The user-profile provisioning
script is a different installation route; do not use it to replace a
system-managed package. The fresh initializer does not perform migration or GC.

After applying the configuration and logging in again, inspect `workestrate
versions` and `workestrate doctor` before launching workloads. Having the CLI
inside a guest does not prove that guest can run another VM: nested KVM needs
the outer runtime/firmware support and a separate native test.

**Next: [set up your fleet →](../getting-started.md#configuration)**
