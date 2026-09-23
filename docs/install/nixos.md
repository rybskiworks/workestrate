# NixOS

[Installation](README.md) · [Linux with Nix](linux.md) · [First workload](../getting-started.md)

Nix is already at home here. Prepare an x86_64 host, then install Workestrate
for the account that will run your fleets.

## Configure the host

Merge these settings into your NixOS configuration. Replace `alice` with your
existing username and use `kvm-amd` instead of `kvm-intel` on an AMD CPU.

```nix
{ pkgs, ... }:
{
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
  environment.systemPackages = [ pkgs.git ];
  boot.kernelModules = [ "kvm-intel" ];
  users.users.alice.extraGroups = [ "kvm" ];
}
```

These settings enable [flakes](https://wiki.nixos.org/wiki/Flakes), provide Git,
load the [KVM module](https://nixos.org/manual/nixos/stable/#sec-kernel-config),
and add your account to the device's
[access group](https://nixos.org/manual/nixos/stable/#sec-user-management).
Enable hardware virtualization in your firmware as well.

Apply your configuration using your usual deployment workflow. For a standard
`/etc/nixos/configuration.nix` installation:

```sh
sudo nixos-rebuild switch
```

The [NixOS rebuild guide](https://wiki.nixos.org/wiki/Nixos-rebuild) covers other
configuration layouts. Log in again to pick up group membership, then verify
that your user can read and write `/dev/kvm`.

## Install Workestrate

Follow [the shared installation steps](linux.md#install-workestrate) to install
the CLI in your user profile and prepare its runtime state. The repository's
flake exports `packages.x86_64-linux.workestrate`; the provisioning wrapper
keeps that package and the selected Microsandbox generation together.

**Next: [set up your fleet →](../getting-started.md#configuration)**
