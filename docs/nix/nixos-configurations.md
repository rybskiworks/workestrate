---
type: Reference
resource: https://nixos.org/manual/nixos/stable/
title: NixOS Configurations
description: Reference for NixOS system configuration — nixosSystem, configuration.nix, module system, nixos-rebuild, generations, nixos-install, containers, VMs, nixosTest, flakes, and nixos-anywhere.
tags: [nix, nixos, configuration, nixosSystem, nixos-rebuild]
timestamp: 2026-07-24T00:00:00Z
---

# NixOS Configurations

## Purpose

This document is a reference for NixOS system configuration. It is intended for future AI agents who write, review, refactor, debug, or validate NixOS configurations. It covers the `nixosSystem` builder function, the `configuration.nix` file structure, the NixOS module system as used in system configuration, `nixos-rebuild` subcommands, system generations and rollback, `nixos-generate-config`, flake-based `nixosConfigurations`, the major NixOS option namespaces, `nixos-enter`, `nixos-install`, declarative containers, NixOS VMs, `nixosTest`, `nixos-anywhere`, and `nixos-rebuild build-image`.

NixOS configurations are the system-level application of the module system documented in `./modules-and-config.md`. This doc focuses on the system-builder and operational layer; for the module system internals (`mkOption`, `lib.types.*`, `evalModules`, merging), cross-reference that doc.

## Sources used

- https://nixos.org/manual/nixos/stable/#sec-configuration-syntax (NixOS configuration syntax, imports, merging, mkForce, mkBefore)
- https://nixos.org/manual/nixos/stable/#sec-changing-config (nixos-rebuild subcommands, build-vm, build-image)
- https://nixos.org/manual/nixos/stable/ (NixOS manual root)
- https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html (NixOS virtual machines, sample configuration)
- https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html (testers.runNixOSTest)
- https://nix.dev/tutorials/nixos/installing-nixos-on-a-raspberry-pi.html (real-world configuration.nix)
- https://nix.dev/tutorials/nixos/building-bootable-iso-image.html (bootable ISO)
- https://nix.dev/tutorials/nixos/deploying-nixos-using-terraform.html (cloud provisioning contrast)
- https://github.com/nix-community/nixos-anywhere (remote provisioning)
- https://github.com/Mic92/nixos-shell (community VM tool)

The crawl files `docs/nix/.crawl/24-installing-nixos-on-a-raspberry-pi.md`, `docs/nix/.crawl/25-integration-testing-with-nixos-virtual-machines.md`, `docs/nix/.crawl/26-nixos-virtual-machines.md`, `docs/nix/.crawl/72-nixos-configuration-syntax.md`, and `docs/nix/.crawl/73-nixos-rebuild.md` are from nix.dev commit 139034be (2026-07-21) and NixOS 26.05 respectively.

## Core guidance

### nixpkgs.lib.nixosSystem — the system builder function

`nixosSystem` is the function that turns a NixOS module (`configuration.nix`) into a full system configuration (a "toplevel" derivation). It lives at `nixpkgs.lib.nixosSystem`. Under the hood it runs `lib.evalModules` with the full set of NixOS base modules, producing `config` (the merged system configuration) and `config.system.build.toplevel` (the activation script + closure).

Classic (non-flake) usage:

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
in
  import "${nixpkgs}/nixos" {
    configuration = ./configuration.nix;
    system = "x86_64-linux";
  }
```

Flake-based form:

```nix
nixpkgs.lib.nixosSystem {
  system = "x86_64-linux";
  modules = [ ./configuration.nix ];
}
```

In a flake, the result is exposed as `nixosConfigurations.<name>`. See `./flake-anatomy.md` for flake outputs and `./nixpkgs-library.md` for `lib.nixosSystem` and `lib.evalModules`.

### configuration.nix — the main config file structure

> "The NixOS configuration file `/etc/nixos/configuration.nix` is actually a _Nix expression_, which is the Nix package manager's purely functional language for describing how to build packages and configurations."

(NixOS manual, "Configuration Syntax")

The canonical shape:

```nix
{ config, pkgs, ... }:

{
  # option definitions
}
```

> "The first line (`{ config, pkgs, ... }:`) denotes that this is actually a function that takes at least the two arguments `config` and `pkgs`."

(NixOS manual, "Configuration Syntax")

The function returns a set of option definitions of the form `name = value`. The httpd example from the manual:

```nix
{ config, pkgs, ... }:

{
  services.httpd.enable = true;
  services.httpd.adminAddr = "alice@example.org";
  services.httpd.virtualHosts.localhost.documentRoot = "/webroot";
}
```

defines a configuration with three option definitions that together enable the Apache HTTP Server with `/webroot` as the document root.

#### Dot-shorthand for nested sets

Dots in option names are shorthand for defining a set containing another set. The httpd example can also be written as:

```nix
{ config, pkgs, ... }:

{
  services = {
    httpd = {
      enable = true;
      adminAddr = "alice@example.org";
      virtualHosts = {
        localhost = {
          documentRoot = "/webroot";
        };
      };
    };
  };
}
```

which may be more convenient when many option definitions share the same prefix.

#### Type checking

NixOS checks option definitions for correctness. Defining a non-existent option produces:

```
The option `services.httpd.enable' defined in `/etc/nixos/configuration.nix' does not exist.
```

Values must have the correct type. `services.httpd.enable` must be a Boolean; giving it a string produces:

```
The option value `services.httpd.enable' in `/etc/nixos/configuration.nix' is not a boolean.
```

#### Value types

| Type | Example |
|---|---|
| Strings | `networking.hostName = "dexter";` (double-quoted; multi-line uses `''...''`) |
| Booleans | `networking.firewall.enable = true;` |
| Integers | `boot.kernel.sysctl."net.ipv4.tcp_keepalive_time" = 60;` |
| Sets | `fileSystems."/boot" = { device = "/dev/sda1"; fsType = "ext4"; options = [ "rw" "data=ordered" "relatime" ]; };` |
| Lists | `boot.kernelModules = [ "fuse" "kvm-intel" "coretemp" ];` (whitespace-separated) |
| Packages | `environment.systemPackages = [ pkgs.thunderbird pkgs.emacs ];` |

#### system.stateVersion

Every `configuration.nix` must set `system.stateVersion` to the NixOS version at install time. This value is set once and never changed — it pins compatibility assumptions (default package versions, module behavior) for the lifetime of the system.

```nix
system.stateVersion = "24.05";
```

### Module system in NixOS: imports, options, config

`configuration.nix` is itself a module. The NixOS configuration mechanism is modular: large configurations can be split into multiple files, and shared configuration can be moved into a shared file.

> "The NixOS configuration mechanism is modular. If your `configuration.nix` becomes too big, you can split it into multiple files. Likewise, if you have multiple NixOS configurations (e.g. for different computers) with some commonality, you can move the common configuration into a shared file."

> "Modules have exactly the same syntax as `configuration.nix`. In fact, `configuration.nix` is itself a module."

(NixOS manual, "Modularity")

The imports example from the manual:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./vpn.nix
    ./kde.nix
  ];
  services.httpd.enable = true;
  environment.systemPackages = [ pkgs.emacs ];
  # ...
}
```

Here, we include two modules from the same directory, `vpn.nix` and `kde.nix`. The latter might look like this:

```nix
{ config, pkgs, ... }:

{
  services.xserver.enable = true;
  services.displayManager.sddm.enable = true;
  services.desktopManager.plasma6.enable = true;
  environment.systemPackages = [ pkgs.vim ];
}
```

#### Merging behavior

When multiple modules define an option, NixOS will try to merge the definitions. For list types (e.g. `environment.systemPackages`), the lists are concatenated. The value in `configuration.nix` is merged last; to make a list element appear first, use `mkBefore`:

```nix
{ boot.kernelModules = mkBefore [ "kvm-intel" ]; }
```

For unique types (e.g. `services.httpd.adminAddr`), two definitions are an error:

```
The unique option `services.httpd.adminAddr' is defined multiple times, in `/etc/nixos/httpd.nix' and `/etc/nixos/configuration.nix'.
```

To force one definition to take precedence:

```nix
{ services.httpd.adminAddr = pkgs.lib.mkForce "bob@example.org"; }
```

See `./modules-and-config.md` for the full module system reference (priority table, `mkDefault`/`mkForce`/`mkOverride`, `mkIf`/`mkMerge`, `submodule`).

#### Inspecting the configuration

The `config` function argument contains the complete, merged system configuration. To inspect final values:

```console
$ nixos-option services.xserver.enable
true

$ nixos-option boot.kernelModules
[ "tun" "ipv6" "loop" ... ]
```

Interactive exploration via `nix repl`:

```console
$ nix repl -f '<nixpkgs/nixos>'

nix-repl> config.networking.hostName
"mandark"

nix-repl> map (x: x.hostName) config.services.httpd.virtualHosts
[ "example.org" "example.gov" ]
```

### nixos-rebuild: switch, boot, test, build, dry-build, dry-activate

> "The file `/etc/nixos/configuration.nix` contains the current configuration of your machine. Whenever you've changed something in that file, you should do `# nixos-rebuild switch` to build the new configuration, make it the default configuration for booting, and try to realise the configuration in the running system (e.g., by restarting system services)."

(NixOS manual, "Changing the Configuration")

| Subcommand | Effect |
|---|---|
| `switch` | Build, set as boot default, activate in running system |
| `boot` | Build, set as boot default, do NOT activate now (takes effect on next reboot) |
| `test` | Build, activate in running system, do NOT set as boot default (reboot reverts) |
| `build` | Build only; creates `./result` symlink. Useful to check compilation |
| `dry-build` | Show what would be built/activated without doing it |
| `dry-activate` | Build, then show what activation would do without actually activating |
| `build-vm` | Build a QEMU VM containing the configuration |
| `build-vm-with-bootloader` | Build a VM that simulates the bootloader |
| `repl` | Open a Nix REPL with `config` loaded |
| `rollback` | Roll back to the previous generation |
| `build-image` | Build a cloud/platform image (amazon, proxmox-lxc, etc.) |

> "to build the configuration and switch the running system to it, but without making it the boot default. So if (say) the configuration locks up your machine, you can just reboot to get back to a working configuration."

> "to build the configuration and make it the boot default, but not switch to it now (so it will only take effect after the next reboot)."

(NixOS manual, "Changing the Configuration")

The `-p` profile flag places the new configuration in a different GRUB submenu:

```console
# nixos-rebuild switch -p test
```

This causes the new configuration (and previous ones created using `-p test`) to show up in the GRUB submenu "NixOS - Profile 'test'". This can be useful to separate test configurations from "stable" configurations.

`nixos-rebuild repl` opens a read-eval-print loop with the configuration loaded into the `config` variable. Use tab for autocompletion, use the `:r` command to reload the configuration files.

> "This command doesn't start/stop user services automatically. `nixos-rebuild` only runs a `daemon-reload` for each user with running user services."

> "These commands must be executed as root, so you should either run them from a root shell or by prefixing them with `sudo -i`."

(NixOS manual, "Changing the Configuration")

### System generations: rollback

Each `nixos-rebuild switch` or `boot` creates a new *generation* — a boot menu entry. Old generations are kept, enabling rollback. `nixos-rebuild rollback` switches to the previous generation and activates it. Generations are listed in the bootloader (GRUB/systemd-boot).

```console
# List generations
$ nix-env --list-generations --profile /nix/var/nix/profiles/system

# Delete generations older than 5 days
$ nix-env --delete-generations --profile /nix/var/nix/profiles/system old 5

# Clean up unreferenced store paths
$ nix-collect-garbage -d
```

### nixos-generate-config: auto-generating configuration

`nixos-generate-config` generates a starter `configuration.nix` and `hardware-configuration.nix`. It detects hardware and mounts. `--root` generates config for a mounted target system; `--dir` sets the output location.

> "You can also use the `installation-cd-graphical-gnome.nix` module to generate the configuration file from scratch: `$ nixos-generate-config --dir ./`"

(nix.dev, "NixOS virtual machines")

`hardware-configuration.nix` is generated and contains `fileSystems` and `boot.initrd` from detected hardware — generally not hand-edited (it is regenerated by `nixos-generate-config`).

### Flake-based NixOS: nixosConfigurations output

A flake exposing a NixOS system:

```nix
{
  description = "My NixOS systems";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

  outputs = { self, nixpkgs, ... }@inputs: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./hosts/myhost/configuration.nix
        ./modules/common.nix
      ];
      specialArgs = { inherit inputs; };
    };
  };
}
```

Deploy with:

```console
# nixos-rebuild --flake .#myhost switch
```

The `.#myhost` selects the `nixosConfigurations.myhost` output. `.#` alone defaults to the hostname. `specialArgs` passes flake inputs to modules; the alternative is setting `_module.args` inside a module. See `./flake-anatomy.md` for flake outputs.

### NixOS modules: boot, services, networking, users, security, system

| Namespace | Covers |
|---|---|
| `boot` | Bootloader (systemd-boot, grub), kernel packages, initrd, kernel modules, sysctl |
| `services` | System services — httpd, nginx, postgresql, openssh, xserver, etc. |
| `networking` | hostname, firewall, wireless, interfaces, DHCP, hosts |
| `users` | users.users.\<name\>, groups, mutableUsers, passwords/keys |
| `security` | sudo, polkit, apparmor, acme (certificates) |
| `system` | stateVersion, autoUpgrade, activationScripts |
| `environment` | systemPackages, variables, etc. |
| `hardware` | enableRedistributableFirmware, bluetooth, video |
| `virtualisation` | containers, libvirtd, docker, qemu |
| `programs` | sway, fish, git, etc. (program-level enablement) |
| `fileSystems` | mount points |

A real-world `configuration.nix` from the Raspberry Pi tutorial demonstrates `boot`, `fileSystems`, `networking`, `users`, `services`, `environment`, `hardware`, and `system` together:

```nix
{ config, pkgs, lib, ... }:

let
  user = "guest";
  password = "guest";
  SSID = "mywifi";
  SSIDpassword = "mypassword";
  interface = "wlan0";
  hostname = "myhostname";
in {

  boot = {
    kernelPackages = pkgs.linuxKernel.packages.linux_rpi4;
    initrd.availableKernelModules = [ "xhci_pci" "usbhid" "usb_storage" ];
    loader = {
      grub.enable = false;
      generic-extlinux-compatible.enable = true;
    };
  };

  fileSystems = {
    "/" = {
      device = "/dev/disk/by-label/NIXOS_SD";
      fsType = "ext4";
      options = [ "noatime" ];
    };
  };

  networking = {
    hostName = hostname;
    wireless = {
      enable = true;
      networks."${SSID}".psk = SSIDpassword;
      interfaces = [ interface ];
    };
  };

  environment.systemPackages = with pkgs; [ vim ];

  services.openssh.enable = true;

  users = {
    mutableUsers = false;
    users."${user}" = {
      isNormalUser = true;
      password = password;
      extraGroups = [ "wheel" ];
    };
  };

  hardware.enableRedistributableFirmware = true;
  system.stateVersion = "23.11";
}
```

(nix.dev, "Installing NixOS on a Raspberry Pi")

### nixos-enter: entering a NixOS system from a chroot

`nixos-enter` chroots into a NixOS installation mounted at a path and runs a command (default: a shell). Usage:

```console
# nixos-enter --root /mnt
```

It is used during manual installation or recovery to run commands inside the target system's NixOS environment, with its activation script run. This is a recovery/installation tool.

### nixos-install: installing NixOS

`nixos-install` installs NixOS onto a mounted root filesystem. Typical flow:

1. Partition and format the disk.
2. Mount the root filesystem at `/mnt`.
3. Run `nixos-generate-config --root /mnt`.
4. Edit `/mnt/etc/nixos/configuration.nix`.
5. Run `nixos-install`.
6. `reboot`.

Flags: `--root` (target mount point), `--no-root-passwd` (skip root password prompt), `--flake` (flake-based install):

```console
# nixos-install --flake .#myhost
```

### NixOS containers: containers.\<name\>, nixos-container

NixOS supports declarative containers via the `containers.<name>` option:

```nix
containers.webserver = {
  autoStart = true;
  config = { config, pkgs, ... }: {
    services.nginx.enable = true;
  };
};
```

The `nixos-container` CLI manages imperative containers (`nixos-container list`, `nixos-container run`, `nixos-container root-login`). Containers use the host's Nix store and kernel (LXC-style, not OCI/Docker). See `./docker-images.md` for OCI images.

### NixOS VMs: nixos-shell, nixos-rebuild build-vm

> "One of the most important features of NixOS is the ability to configure the entire system declaratively, including packages to be installed, services to be run, as well as other settings and options."

> "NixOS configurations can be used to test and use NixOS using a virtual machine, which is a lighter weight option compared to a full 'bare metal' installation."

(nix.dev, "NixOS virtual machines")

Build a VM from a channel-based config:

```shell-session
$ nix-build '<nixpkgs/nixos>' -A vm -I nixpkgs=channel:nixos-24.05 -I nixos-config=./configuration.nix
```

Or from the current `configuration.nix`:

> "If you have a machine that supports hardware virtualisation, you can also test the new configuration in a sandbox by building and running a QEMU _virtual machine_ that contains the desired configuration. Just do `$ nixos-rebuild build-vm` `$ ./result/bin/run-*-vm`"

(NixOS manual, "Changing the Configuration")

`nixos-shell` (community tool) runs a NixOS config in a VM interactively:

```console
$ nix run github:Mic92/nixos-shell
```

Caveat: running the VM creates a `nixos.qcow2` state file in the working directory. Delete it when changing config to avoid stale state (e.g. stale user passwords):

```shell-session
$ rm nixos.qcow2
```

Port forwarding via `QEMU_NET_OPTS` (host port 2222 to guest port 22):

```shell-session
$ QEMU_NET_OPTS="hostfwd=tcp:127.0.0.1:2222-:22" ./result/bin/run-*-vm
$ ssh -p 2222 localhost
```

### NixOS tests: nixosTest

`testers.runNixOSTest` (also `nixosTest`) is the VM-based integration test framework. The minimal pattern:

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

> "Nixpkgs provides a test environment to automate integration testing for distributed systems. It allows defining tests based on a set of declarative NixOS configurations and using a Python shell to interact with them through QEMU as the backend."

(nix.dev, "Integration testing with NixOS virtual machines")

See `./testing.md` for the full testing reference — do NOT duplicate the full content here.

### nixos-anywhere: remote provisioning

`nixos-anywhere` is a community tool for provisioning NixOS on remote machines via SSH (disk formatting + kexec + `nixos-install` over SSH). Usage:

```console
$ nixos-anywhere root@host --flake .#myhost
```

It boots a NixOS installer via kexec on the target, then runs `nixos-install` with the flake config. It is the modern alternative to manual `nixos-install` for remote/bare-metal provisioning. Contrast with the Terraform approach, which provisions cloud VMs from prebuilt AMIs.

### Building images with nixos-rebuild build-image

> "Nixpkgs contains a variety of modules to build custom images for different virtualization platforms and cloud providers, such as e.g. `amazon-image.nix` and `proxmox-lxc.nix`."

> "All of those images can be built via both, their `system.build.image` attribute and the `nixos-rebuild build-image` command."

(NixOS manual, "Building Images")

Build an Amazon image from an existing NixOS configuration:

```console
$ nixos-rebuild build-image --image-variant amazon
[...]
Done. The disk image can be found in /nix/store/[hash]-nixos-image-amazon-25.05pre-git-x86_64-linux/nixos-image-amazon-25.05pre-git-x86_64-linux.vpc
```

Run `nixos-rebuild build-image` without arguments to list all available variants. The `image.modules` option customizes specific image variants (similar to `specialisations`):

```nix
{
  image.modules.linode = {
    boot.loader.systemd-boot.enable = lib.mkForce false;
  };
}
```

## Practical rules

- Always set `system.stateVersion` to the NixOS version at install time; never change it after.
- Run `nixos-rebuild test` first when unsure; `switch` only when confident.
- Use `nixos-rebuild build` to check evaluation without activating.
- Split large configurations into modules via `imports`.
- Use `nixos-option <path>` to inspect final merged values.
- In flakes, expose systems via `nixosConfigurations.<name>` and deploy with `nixos-rebuild --flake .#<name>`.
- Delete `nixos.qcow2` when changing VM config to avoid stale state.
- Use `nixos-generate-config` to bootstrap, then customize.
- Never hand-edit `hardware-configuration.nix` unless you know why.
- Use `--flake .#hostname` (the `.#` defaults to current hostname).
- Run `nixos-rebuild` as root (`sudo -i`) — it cannot activate the system otherwise.
- Remember `nixos-rebuild` only does `daemon-reload` for user services; it does not restart them.
- Use `mkForce` only when a unique option is defined in multiple modules and you need precedence.
- Use `mkBefore` to prepend list elements (the `configuration.nix` value is merged last by default).

## Review checklist

- [ ] `system.stateVersion` is set and matches the install-time NixOS version
- [ ] `configuration.nix` is a function `{ config, pkgs, ... }: { ... }` with `...`
- [ ] No option definitions reference non-existent options (run `nixos-rebuild build`)
- [ ] Value types match option types (boolean/string/integer/list/set/package)
- [ ] `imports` paths are relative and exist
- [ ] Unique options not defined in multiple modules without `mkForce`/`mkDefault`
- [ ] `hardware-configuration.nix` not hand-edited without justification
- [ ] No plaintext passwords in the Nix store (visible in `/nix/store`) — use `initialHashedPassword` or `ssh.authorizedKeys`
- [ ] Flake `nixosConfigurations.<name>` matches the deploy target (`.#<name>`)
- [ ] `specialArgs`/`_module.args` used to pass flake inputs to modules
- [ ] `nixos-rebuild test` run before `switch` on production systems
- [ ] Container `config` blocks are valid NixOS modules
- [ ] VM `nixos.qcow2` deleted when config changes affect users/state

## Implementation checklist

- [ ] Bootstrap with `nixos-generate-config --root /mnt` (or `--dir ./`)
- [ ] Edit `configuration.nix`: set `system.stateVersion`, `boot.loader.*`, `networking.hostName`
- [ ] Add users via `users.users.<name>` with `isNormalUser` and `extraGroups`
- [ ] Add packages via `environment.systemPackages`
- [ ] Enable services via `services.<name>.enable = true`
- [ ] Split large configs into modules via `imports = [ ./module.nix ];`
- [ ] For flakes: define `nixosConfigurations.<name>` with `nixpkgs.lib.nixosSystem`
- [ ] Pass flake inputs via `specialArgs = { inherit inputs; };`
- [ ] Validate with `nixos-rebuild build` (evaluation only)
- [ ] Test with `nixos-rebuild test` (activate, no boot default)
- [ ] Deploy with `nixos-rebuild switch` (activate + boot default)
- [ ] For VM testing: `nixos-rebuild build-vm` then `./result/bin/run-*-vm`
- [ ] For remote provisioning: `nixos-anywhere root@host --flake .#<name>`

## Validation hooks

- `nixos-rebuild build` — build only; check evaluation without activating
- `nixos-rebuild dry-build` — show what would be built/activated
- `nixos-rebuild dry-activate` — build, then show what activation would do
- `nixos-option <path>` — inspect the final merged value of a NixOS option
- `nix repl -f '<nixpkgs/nixos>'` — interactively explore `config.*`
- `nixos-rebuild build-vm` — build a QEMU VM containing the configuration
- `nix eval .#nixosConfigurations.<name>.config.<path>` — evaluate a flake NixOS config option
- `nix flake check` — validate flake outputs including `nixosConfigurations`
- `nixos-rebuild repl` — open a Nix REPL with `config` loaded

## Examples

### Example 1: Minimal configuration.nix

The sample configuration from the NixOS virtual machines tutorial:

```nix
{ config, pkgs, ... }:
{
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  users.users.alice = {
    isNormalUser = true;
    extraGroups = [ "wheel" ]; # Enable ‘sudo’ for the user.
    initialPassword = "test";
  };

  environment.systemPackages = with pkgs; [
    cowsay
    lolcat
  ];

  system.stateVersion = "24.05";
}
```

(nix.dev, "NixOS virtual machines")

### Example 2: Flake-based nixosConfigurations

```nix
{
  description = "My NixOS systems";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

  outputs = { self, nixpkgs, ... }@inputs: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./hosts/myhost/configuration.nix
        ./modules/common.nix
      ];
      specialArgs = { inherit inputs; };
    };
  };
}
```

Deploy with `sudo nixos-rebuild --flake .#myhost switch`.

### Example 3: NixOS container declaration

```nix
{ config, pkgs, ... }:

{
  containers.webserver = {
    autoStart = true;
    config = { config, pkgs, ... }: {
      services.nginx.enable = true;
    };
  };
}
```

Manage imperatively with `nixos-container list`, `nixos-container run webserver`, `nixos-container root-login webserver`.

### Example 4: nixos-rebuild build-vm workflow

```shell-session
$ nixos-rebuild build-vm
$ ./result/bin/run-*-vm
$ QEMU_NET_OPTS="hostfwd=tcp:127.0.0.1:2222-:22" ./result/bin/run-*-vm
$ ssh -p 2222 localhost
$ rm nixos.qcow2
```

(NixOS manual, "Changing the Configuration")

## Common mistakes

1. **Changing `system.stateVersion` after install.** This value pins compatibility assumptions; changing it breaks package version defaults and module behavior. Set it once at install time and never touch it.

2. **Running `nixos-rebuild switch` without testing first on a production system.** Use `nixos-rebuild test` to activate without setting the boot default; if the config locks up the machine, reboot reverts.

3. **Forgetting to delete `nixos.qcow2` when changing VM config.** The qcow2 file holds dynamic state (including user passwords); changes to users won't take effect until it is deleted.

4. **Hand-editing `hardware-configuration.nix` and losing changes on next `nixos-generate-config`.** This file is generated from detected hardware; regenerate it only when hardware changes.

5. **Using `nixos-rebuild test` and expecting changes to persist across reboot.** `test` activates but does not set the boot default — reboot reverts to the previous generation.

6. **Forgetting `--flake .#hostname`.** `.#` alone defaults to the current hostname, which may not match the intended `nixosConfigurations` output.

7. **Not running `nixos-rebuild` as root.** These commands must be executed as root; run from a root shell or prefix with `sudo -i`.

8. **Confusing `boot` (set boot default, no activate) with `test` (activate, no boot default).** They are opposite in activation/boot-default behavior.

9. **Storing plaintext passwords in the Nix store.** Credentials written into a NixOS configuration are stored in plain text in `/nix/store`. Use `initialHashedPassword` or `ssh.authorizedKeys` for secure alternatives.

10. **Forgetting that `nixos-rebuild` only does `daemon-reload` for user services.** It does not start/stop user services automatically; only system services are restarted.

## Strict vs contextual guidance

Strict: `configuration.nix` is a Nix module function `{ config, pkgs, ... }: { ... }`; `system.stateVersion` is set once and never changed; `nixos-rebuild switch` activates + sets boot default; `nixos-rebuild test` activates without boot default; `nixos-rebuild boot` sets boot default without activating; generations enable rollback; `nixosSystem` produces the system config from modules; `nixos-rebuild` must run as root; user services only get `daemon-reload`.

Contextual: flake vs channel-based configuration; `switch` vs `test` vs `boot` choice per change; container vs VM vs bare metal for testing; `nixos-anywhere` vs Terraform vs manual `nixos-install` for provisioning; module file organization (single file vs `imports` split); `specialArgs` vs `_module.args` for passing flake inputs; `nixos-shell` vs `nixos-rebuild build-vm` for ad-hoc VM testing.

## Policy decisions

- Decide whether to use flakes or channels for system configuration (flakes are the modern default).
- Decide `nixos-rebuild` subcommand per change: `test` for risky changes, `switch` for confident ones, `boot` for changes requiring reboot.
- Decide module file organization: single `configuration.nix` vs `imports` split across files.
- Decide provisioning approach: `nixos-install` (manual), `nixos-anywhere` (remote SSH), or Terraform (cloud AMIs).
- Decide testing approach: `nixos-rebuild build-vm` (ad-hoc), `nixos-shell` (interactive), or `testers.runNixOSTest` (automated integration tests).
- Decide secret management: `initialHashedPassword` (stored in store), `ssh.authorizedKeys`, or a community secrets solution (sops-nix, agenix).
- Decide whether to expose flake inputs via `specialArgs` (per-`nixosSystem`) or `_module.args` (per-module).
- Decide container strategy: NixOS declarative containers (`containers.<name>`, LXC-style) vs OCI/Docker images (see `./docker-images.md`).

## Related docs

- [Modules and Configuration](./modules-and-config.md) — NixOS module system (options, config, mkOption, evalModules)
- [Flake Anatomy](./flake-anatomy.md) — flake outputs including `nixosConfigurations` and `nixosModules`
- [Testing](./testing.md) — NixOS VM test framework (`nixosTests` / `testers.runNixOSTest`)
- [Nixpkgs Library](./nixpkgs-library.md) — `lib.nixosSystem` and `lib.evalModules`
- [Docker Images](./docker-images.md) — OCI/Docker images (contrast with NixOS containers)

## Related skills

- `nix-usage` — operational reference for the ai-workbench Nix flake, dev shell, Rust toolchain, and Microsandbox runtime

## Citations

[1] [NixOS Configuration Syntax — NixOS manual](https://nixos.org/manual/nixos/stable/#sec-configuration-syntax)
[2] [Changing the Configuration — NixOS manual](https://nixos.org/manual/nixos/stable/#sec-changing-config)
[3] [NixOS virtual machines — nix.dev](https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html)
[4] [Integration testing with NixOS virtual machines — nix.dev](https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html)
[5] [Building a bootable ISO image — nix.dev](https://nix.dev/tutorials/nixos/building-bootable-iso-image.html)
[6] [Deploying NixOS using Terraform — nix.dev](https://nix.dev/tutorials/nixos/deploying-nixos-using-terraform.html)
[7] [Installing NixOS on a Raspberry Pi — nix.dev](https://nix.dev/tutorials/nixos/installing-nixos-on-a-raspberry-pi.html)
[8] [NixOS Manual (stable)](https://nixos.org/manual/nixos/stable/)
[9] [nixos-anywhere — GitHub](https://github.com/nix-community/nixos-anywhere)
[10] [nixos-shell — GitHub](https://github.com/Mic92/nixos-shell)
