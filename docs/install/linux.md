# Linux with Nix

[Installation](README.md) · [NixOS](nixos.md) · [First workload](../getting-started.md)

Use Workestrate on your existing x86_64 Linux distribution. Nix builds the CLI
and supplies its pinned runtime, SOPS, and age.

## Prepare your host

**Install Git and Nix.** Use your distribution's package manager for Git and the
[official Nix installation guide](https://nix.dev/install-nix) for Nix. Open a
new terminal after installation and check `nix --version`.

Enable the Nix command interface and flakes in `~/.config/nix/nix.conf` (create
the directory and file if needed):

```ini
extra-experimental-features = nix-command flakes
```

The `extra-` form preserves other enabled features. See
[Nix configuration](https://nix.dev/manual/nix/stable/command-ref/conf-file.html).

**Make KVM available to your user.** Enable hardware virtualization in the
firmware and load your CPU's KVM module (`kvm_intel` or `kvm_amd`). Your account
needs read and write access to `/dev/kvm`; distributions commonly grant this
through the `kvm` group. Log in again after changing group membership. If Linux
itself runs in a VM, its hypervisor must expose nested virtualization.

## Install Workestrate

The provisioner builds the checkout, updates your Nix profile, prepares the
Microsandbox runtime state, and checks the host. On an existing Microsandbox
host, first read [runtime provisioning](../runtime-provisioning.md): preparing
a new runtime generation can migrate state and requires stopped workloads.

Run these commands as the user who will own your fleets:

```sh
git clone --branch main https://github.com/rybskiworks/workestrate.git
cd workestrate
./scripts/host-provision.sh
workestrate --version
workestrate doctor
```

`./scripts/host-provision.sh --check-only` reports what needs attention without
updating the profile or runtime state; it can still build the package.

Resolve host and runtime failures reported by `doctor`. A fresh installation
can report missing configuration, fleets, and an age key; the next steps cover
those. Workload readiness is checked after configuration and launch.

**Next: [set up your fleet →](../getting-started.md#configuration)**
