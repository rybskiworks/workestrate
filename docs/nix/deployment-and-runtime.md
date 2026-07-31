---
type: Reference
resource: https://nix.dev/tutorials/nixos/
title: Deployment and Runtime
description: NixOS deployment and runtime operations — nixos-rebuild subcommands, system generations and rollback, flake-based and remote deployment, nixos-anywhere provisioning, Terraform, Raspberry Pi and Docker image deployment, system activation scripts, and runtime tooling.
tags: [nix, deployment, nixos-rebuild, activation, generations, nixos-anywhere]
timestamp: 2026-07-24T01:35:00Z
---

# Deployment and Runtime

## Purpose

Provide concrete, repo-independent guidance for deploying and operating NixOS
systems and Nix-built container images. Covers `nixos-rebuild` subcommands
(`switch`, `test`, `boot`, `build`, `dry-build`, `dry-activate`, `build-vm`,
`build-image`, `repl`), system generations and rollback, flake-based and
remote deployment (`--target-host`, `--build-host`), `nixos-anywhere`
provisioning with `disko`, Terraform `deploy_nixos`, Raspberry Pi SD image
deployment, Docker/OCI image loading, the `switch-to-configuration` activation
script, and runtime tooling (`nixos-option`, `nixos-container`, `nixos-enter`,
`nixos-generate`).

This document is intended as generic reference material for future AI coding
agents working in any Nix flake or NixOS deployment. Project-specific examples
from `ai-workbench` (which deploys Docker/OCI images into Microsandbox, not
NixOS hosts) are included as real-world illustrations of the container-image
deployment surface, not as the authoritative scope.

Agents should use this document as the authoritative reference when choosing
a `nixos-rebuild` subcommand, deciding between `switch`/`test`/`boot`,
performing remote deployment, provisioning bare metal with `nixos-anywhere`,
loading a Nix-built Docker image, or diagnosing an activation or runtime
failure.

## Sources used

- Crawl file: `docs/nix/.crawl/73-nixos-rebuild.md` —
  <https://nixos.org/manual/nixos/stable/#sec-changing-config> (NixOS 26.05
  manual)
- Crawl file: `docs/nix/.crawl/27-provisioning-remote-machines-via-ssh.md` —
  <https://nix.dev/tutorials/nixos/provisioning-remote-machines.html>
  (nixos-anywhere + disko tutorial)
- Crawl file: `docs/nix/.crawl/22-deploying-nixos-using-terraform.md` —
  <https://nix.dev/tutorials/nixos/deploying-nixos-using-terraform.html>
- Crawl file: `docs/nix/.crawl/24-installing-nixos-on-a-raspberry-pi.md` —
  <https://nix.dev/tutorials/nixos/installing-nixos-on-a-raspberry-pi.html>
- Crawl file: `docs/nix/.crawl/26-nixos-virtual-machines.md` —
  <https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html>
- Crawl file: `docs/nix/.crawl/20-building-and-running-docker-images.md` —
  <https://nix.dev/tutorials/nixos/building-and-running-docker-images.html>
- Crawl file: `docs/nix/.crawl/21-building-bootable-iso-image.md` —
  <https://nix.dev/tutorials/nixos/building-bootable-iso-image.html>
- Crawl file: `docs/nix/.crawl/72-nixos-configuration-syntax.md` —
  <https://nixos.org/manual/nixos/stable/#sec-configuration-syntax>
- Project file: `flake.nix` — Docker image deployment for microsandboxes
  (`pi-image`, `tempest-image`, `load-images`)

## Core guidance

### `nixos-rebuild` subcommands

`nixos-rebuild` is the primary tool for building, activating, and managing
NixOS system configurations. From the NixOS manual (crawl 73):

> "to build the new configuration, make it the default configuration for
> booting, and try to realise the configuration in the running system (e.g.,
> by restarting system services)." (crawl 73, on `nixos-rebuild switch`)

> "to build the configuration and switch the running system to it, but without
> making it the boot default. So if (say) the configuration locks up your
> machine, you can just reboot to get back to a working configuration." (crawl
> 73, on `nixos-rebuild test`)

> "to build the configuration and make it the boot default, but not switch to
> it now (so it will only take effect after the next reboot)." (crawl 73, on
> `nixos-rebuild boot`)

> "to build the configuration but nothing more. This is useful to see whether
> everything compiles cleanly." (crawl 73, on `nixos-rebuild build`)

> "These commands must be executed as root, so you should either run them from
> a root shell or by prefixing them with `sudo -i`." (crawl 73)

> "This command doesn't start/stop user services automatically.
> `nixos-rebuild` only runs a `daemon-reload` for each user with running user
> services." (crawl 73, warning on user services)

Summary table:

| Subcommand       | Effect                                                        | Persists across reboot? | Use case |
|------------------|---------------------------------------------------------------|-------------------------|----------|
| `switch`         | Build, set boot default, activate in running system           | Yes                     | Standard deployment |
| `test`           | Build, activate in running system, do NOT set boot default    | No (reboot reverts)     | Risky changes; quick rollback via reboot |
| `boot`           | Build, set boot default, do NOT activate now                   | Yes (after reboot)      | Kernel/bootloader changes requiring reboot |
| `build`          | Build only; no activation, no boot default                    | N/A                     | Compile check |
| `dry-build`      | Show what would be built                                      | N/A                     | Preview build plan |
| `dry-activate`   | Show activation actions without applying                      | N/A                     | Preview service restarts |
| `build-vm`       | Build a QEMU VM from the config                               | N/A                     | Test config in a sandbox |
| `build-image`    | Build cloud/VM images (amazon, proxmox-lxc, etc.)             | N/A                     | Cloud provisioning |
| `repl`           | Inspect the evaluated config interactively                    | N/A                     | Debug option values |
| `--rollback`     | Revert to the previous generation                            | Yes                     | Undo a bad deployment |
| `-p <profile>`   | Use a named GRUB submenu profile                              | Yes                     | Separate test configs |
| `--upgrade`      | Pull the latest nixpkgs channel before building               | Yes                     | Channel-based upgrades |

Examples:

```shell-session
# Standard deployment (persist + activate)
$ sudo -i nixos-rebuild switch

# Risky change (activate, but reboot reverts)
$ sudo -i nixos-rebuild test

# Kernel change (activate on next reboot)
$ sudo -i nixos-rebuild boot

# Compile check only
$ nixos-rebuild build

# Preview what would be built / activated
$ nixos-rebuild dry-build
$ nixos-rebuild dry-activate

# Revert to the previous generation
$ sudo -i nixos-rebuild --rollback switch

# Test config in a separate GRUB submenu
$ sudo -i nixos-rebuild switch -p test
```

### System generations and rollback

Each `nixos-rebuild switch` or `boot` creates a new *generation* in
`/nix/var/nix/profiles/system`. Old generations are not deleted — they remain
available in the GRUB boot menu, so a failed activation can be recovered by
selecting an older generation at boot.

`nixos-rebuild --rollback switch` reverts the running system to the previous
generation. Garbage collection of old generations is performed with
`nix-collect-garbage --delete-old` or `nixos-rebuild delete-generations`.

The `-p` / `--profile-name` flag separates test configurations from stable
ones:

> "which causes the new configuration (and previous ones created using `-p
> test`) to show up in the GRUB submenu "NixOS - Profile 'test'". This can be
> useful to separate test configurations from "stable" configurations." (crawl
> 73, on `nixos-rebuild switch -p test`)

```shell-session
# Revert to the previous generation
$ sudo -i nixos-rebuild --rollback switch

# Delete generations older than 30 days
$ sudo -i nix-collect-garbage --delete-old

# Delete all generations except the current
$ sudo -i nixos-rebuild delete-generations

# Use a named profile to isolate test configs
$ sudo -i nixos-rebuild switch -p test
```

### Flake-based deployment

For flake-based NixOS configurations, use `--flake`:

```shell-session
$ sudo -i nixos-rebuild --flake .#hostname switch
```

The flake URI form is `.#<flake-output>`, where the output is typically a
`nixosConfigurations.<hostname>` attribute. Using `--flake .#` (with no
explicit name) selects the `nixosConfigurations` entry matching the current
hostname.

Flakes must be enabled. This requires
`experimental-features = nix-command flakes` in `nix.conf`, or the
`nix.settings.experimental-features` NixOS option:

```nix
nix.settings.experimental-features = [ "nix-command" "flakes" ];
```

```shell-session
# Deploy the flake config for a specific hostname
$ sudo -i nixos-rebuild --flake .#myhost switch

# Use the current hostname's config
$ sudo -i nixos-rebuild --flake .# switch

# Build only (compile check)
$ nixos-rebuild --flake .#myhost build
```

### Remote deployment: `--target-host` and `--build-host`

`nixos-rebuild` supports deploying to a remote host over SSH:

```shell-session
# Build locally, activate on the remote host
$ sudo -i nixos-rebuild --flake .#myhost \
    --target-host root@remote-host switch

# Build on a remote build host, activate on the target
$ sudo -i nixos-rebuild --flake .#myhost \
    --target-host root@remote-host \
    --build-host local-host switch
```

- `--target-host` copies the built system closure to the remote machine and
  activates it there. SSH credentials are required (key-based or password).
- `--build-host` builds on a remote machine — useful when the local machine
  lacks the resources or architecture to build (e.g. cross-compiling for
  aarch64). The build host must have Nix installed.

For routine updates to a machine provisioned with `nixos-anywhere`, the
tutorial notes that `nixos-anywhere` is no longer needed:

> "To update the system, run `npins` and re-deploy the configuration:" (crawl
> 27)

> "`nixos-anywhere` is not needed any more, unless you want to change the disk
> layout." (crawl 27)

The crawl 27 update example:

```shell-session
nixos-rebuild switch --no-flake --target-host root@target-host
```

### `nixos-anywhere` — provisioning bare metal / VMs

`nixos-anywhere` replaces any Linux installation with a NixOS configuration on
a running system via SSH, using `disko` for declarative disk partitioning. It
partitions, formats, mounts, installs, and reboots the target.

> "It is possible to replace any Linux installation with a NixOS configuration
> on running systems using `nixos-anywhere` and `disko`." (crawl 27)

> "`nixos-anywhere` will now log into the target system, partition, format,
> and mount the disk, and install the NixOS configuration. Then, it reboots
> the system." (crawl 27)

Requirements:

- QEMU VM or live USB with kexec support
- x86-64 or aarch64 architecture
- ≥ 1 GB RAM
- DHCP networking
- SSH access as root (or sudo)

The deploy example from crawl 27:

```shell-session
toplevel=$(nixos-rebuild build --no-flake)
diskoScript=$(nix-build -E "((import <nixpkgs> {}).nixos [ ./configuration.nix ]).diskoScript")
nixos-anywhere --store-paths "$diskoScript" "$toplevel" root@target-host
```

For password-based SSH authentication, use `--env-password` and set
`SSH_PASS`:

```shell-session
$ SSH_PASS=secret nixos-anywhere --env-password root@target-host
```

### Terraform + NixOS

Terraform provisions cloud instances (e.g. AWS AMIs) and the `deploy_nixos`
module from `terraform-nixos` runs `nixos-rebuild` over SSH to deploy
incremental configuration changes.

> "Once the AWS instance is running a NixOS image via Terraform, we can teach
> Terraform to always build the latest NixOS configuration and apply those
> changes to your instance." (crawl 22)

Caveats:

> "The `deploy_nixos` module requires NixOS to be installed on the target
> machine and Nix on the host machine." (crawl 22, caveats)

> "The `deploy_nixos` module doesn't work when the client and target
> architectures are different (unless you use distributed builds)." (crawl 22,
> caveats)

Additional caveat: per-machine evaluation can consume significant memory on
the host running Terraform.

```terraform
module "deploy_nixos" {
    source = "git::https://github.com/tweag/terraform-nixos.git//deploy_nixos?ref=5f5a0408b299874d6a29d1271e9bffeee4c9ca71"
    nixos_config = "${path.module}/configuration.nix"
    target_host = aws_instance.machine.public_ip
    ssh_private_key_file = local_file.machine_ssh_key.filename
    ssh_agent = false
}
```

### Raspberry Pi deployment

Download a prebuilt SD image from Hydra, decompress, and write to the SD card:

```shell-session
# Decompress the zstd-compressed image
$ unzstd nixos-sd-image-23.11pre500597.0fbe93c5a7c-aarch64-linux.img.zst

# Write to the SD card (verify the device with lsblk first!)
$ sudo dd if=nixos-sd-image-23.11pre500597.0fbe93c5a7c-aarch64-linux.img \
    of=/dev/sdX bs=4096 conv=fsync status=progress
```

Or build a custom SD image with `nixos-generate`:

```shell-session
$ nixos-generate --format sd-aarch64 --configuration ./rpi.nix
```

The SD image already has NixOS installed, so only `nixos-rebuild` is needed to
apply a custom configuration:

> "Due to the way the `nixos-sd-image` is designed, NixOS is actually
> *already installed* at this point, so we only need to `nixos-rebuild` with
> our new configuration:" (crawl 24)

```shell-session
# nixos-rebuild boot
# reboot
```

For upgrades, use `--upgrade`:

```shell-session
$ sudo -i nixos-rebuild switch --upgrade
```

### Docker image deployment

Build a Docker/OCI image as a Nix derivation, then load it into the Docker
daemon (or any OCI-compatible runtime). No Dockerfile or Docker daemon is
needed during the build.

> "The image tag (`y74sb4nrhxr975xs7h83izgm8z75x5fc`) refers to the Nix build
> hash and makes sure that the Docker image corresponds to our Nix build."
> (crawl 20)

> "To work with the container, load this image into Docker's image registry
> from the default `result` symlink created by `nix-build`:" (crawl 20)

```shell-session
$ docker load < result
Loaded image: hello-docker:y74sb4nrhxr975xs7h83izgm8z75x5fc
```

The one-liner form:

```shell-session
$ docker load < $(nix-build hello-docker.nix)
```

With flakes:

```shell-session
$ nix build .#dockerImage
$ docker load < result
```

### System activation: `switch-to-configuration` and `activate`

The built system closure contains a `bin/switch-to-configuration` script,
invoked by `nixos-rebuild switch`/`test`. This script performs the activation:

- Reload systemd units
- Restart services whose config changed
- Set up users and groups
- Swap the `/etc` symlink to point at the new generation's `/etc`
- Run activation scripts

The `activate` script runs per-user activation (e.g. home-manager user
activation).

`nixos-rebuild` is a wrapper around two steps:

1. Building the system closure (`nixos-rebuild build`).
2. Calling `switch-to-configuration` with the appropriate verb
   (`switch`, `test`, `boot`, `dry-activate`).

`dry-activate` runs `switch-to-configuration dry-activate`, which prints the
actions that *would* be taken (service restarts, unit reloads) without
applying them.

### Runtime tooling

| Tool | Purpose |
|------|---------|
| `systemctl restart <service>` | Restart a service after config change |
| `systemctl status <service>` | Check service state |
| `journalctl -u <service> -f` | Tail service logs |
| `nixos-version` | Print the NixOS version |
| `nixos-option <option>` | Query the effective value of a NixOS option |
| `nix repl -f '<nixpkgs/nixos>'` | Interactively inspect the evaluated config |

> "The command `nixos-option` allows you to find out:" (crawl 72)

```shell-session
$ nixos-option services.xserver.enable
true
```

> "Interactive exploration of the configuration is possible using `nix repl`,
> a read-eval-print loop for Nix expressions." (crawl 72)

```shell-session
$ nixos-version
26.05 (Xenon)

$ nixos-option services.openssh.enable
true

$ nix repl -f '<nixpkgs/nixos>'
Welcome to Nix version 2.x. Type :? for help.
nix-repl> config.services.openssh.enable
true
```

### `nixos-container`

`nixos-container` manages declarative NixOS containers (systemd-nspawn).
Containers are configured via the `containers.<name>` NixOS options; each
container is itself a NixOS configuration.

```nix
containers.webserver = {
  autoStart = true;
  config = { config, pkgs, ... }: {
    services.nginx.enable = true;
  };
};
```

```shell-session
$ sudo -i nixos-container start webserver
$ sudo -i nixos-container status webserver
$ sudo -i nixos-container root-login webserver
$ sudo -i nixos-container update webserver
$ sudo -i nixos-container stop webserver
```

### `nixos-enter`

`nixos-enter` chroots/mounts into a NixOS system root (e.g. from a live USB or
a mounted disk) to run commands in that system's context. This is useful for
rescue and repair when the system does not boot.

```shell-session
# Mount the broken system's root at /mnt, then enter it
$ sudo mount /dev/sda2 /mnt
$ sudo nixos-enter --root /mnt

# Run a specific command
$ sudo nixos-enter --root /mnt -- nixos-rebuild switch
$ sudo nixos-enter --root /mnt -- passwd root
```

### `nixos-generate`

`nixos-generate` (from nixos-generators) builds NixOS images in various
formats: `iso`, `sd-aarch64`, `amazon`, `qcow2`, `vm`, etc.

```shell-session
$ nixos-generate --format iso --configuration ./myimage.nix
$ nixos-generate --format sd-aarch64 --configuration ./rpi.nix
$ nixos-generate --format qcow2 --configuration ./vm.nix
```

The newer built-in equivalent is `nixos-rebuild build-image`:

> "Nixpkgs contains a variety of modules to build custom images for different
> virtualization platforms and cloud providers, such as e.g.
> `amazon-image.nix` and `proxmox-lxc.nix`." (crawl 73)

> "All of those images can be built via both, their `system.build.image`
> attribute and the `nixos-rebuild build-image` command." (crawl 73)

```shell-session
$ nixos-rebuild build-image --image-variant amazon
```

### Project deployment context: Docker images for microsandboxes

The `ai-workbench` repo does **not** deploy NixOS hosts. It builds Docker/OCI
images via `pkgs.dockerTools.buildLayeredImage` (`pi-image`,
`tempest-image`) and loads them into Microsandbox via `just load-images` /
`just load-pi-image`.

The `load-images` derivation iterates `workload-images` and pipes
`nix build .#<name> --no-link --print-out-paths` into
`gunzip | msb load -t <name>:latest`. This is the project's actual deployment
surface — container images, not NixOS system activations.

The load pattern from `flake.nix`:

```shell-session
out=$(nix build .#${name} --no-link --print-out-paths)
gunzip -c "$out" | msb load -t ${name}:latest
```

See `/docs/nix/docker-images.md` for the image-building details
(`buildLayeredImage`, `contents`, `extraCommands`, glibc matching).

## Practical rules

- Run `nixos-rebuild` as root (`sudo -i` or a root shell).
- Use `nixos-rebuild test` first for risky changes; reboot reverts.
- Use `nixos-rebuild boot` when a reboot is acceptable or required (kernel
  changes, bootloader changes).
- Use `--rollback` to revert to the previous generation; old generations
  persist in the GRUB menu.
- Use `--flake .#hostname` for flake-based NixOS configs.
- Use `--target-host` for remote activation, `--build-host` for remote
  building.
- Use `nixos-anywhere` only for initial provisioning / disk layout changes;
  subsequent updates use `nixos-rebuild --target-host`.
- For Docker/OCI workloads (this project), use `nix build .#<image>` +
  `docker load` (or `msb load` for Microsandbox), NOT `nixos-rebuild`.
- Pin nixpkgs at the flake level; do not rely on channels for reproducible
  deployment.
- Use `nixos-rebuild dry-build` / `dry-activate` to preview changes before
  applying.
- Verify the target disk device with `lsblk` before `dd` or `disko`.
- Use `nixos-rebuild build-vm` to test a config in an isolated QEMU sandbox
  before deploying to a real host.
- Use `-p <profile>` to isolate test configurations from stable ones in the
  GRUB menu.

## Review checklist

- [ ] Correct `nixos-rebuild` subcommand chosen for the change's risk profile
      (`test` for risky, `boot` for kernel changes, `switch` for standard)
- [ ] Flake URI matches a defined `nixosConfigurations.<name>`?
- [ ] Remote host reachable via SSH as root (for `--target-host`)?
- [ ] `--build-host` specified when local arch ≠ target arch?
- [ ] Disk layout validated with `installTest` before `nixos-anywhere`?
- [ ] Disk device verified with `lsblk` before `dd` / `disko`?
- [ ] Docker image loaded with `docker load` / `msb load` and tag verified?
- [ ] nixpkgs pinned at the flake level (no channel reliance)?
- [ ] `dry-build` / `dry-activate` run before production deployment?
- [ ] User services restarted manually (nixos-rebuild only does
      `daemon-reload`)?
- [ ] Old generations not prematurely garbage-collected (rollback safety)?

## Implementation checklist

- [ ] Edit the NixOS configuration (`configuration.nix` or flake module)
- [ ] Run `nixos-rebuild build` to check compilation
- [ ] Run `nixos-rebuild dry-build` / `dry-activate` to preview changes
- [ ] Run `nixos-rebuild test` for risky changes (reboot reverts)
- [ ] Run `nixos-rebuild switch` to persist and activate
- [ ] Verify with `systemctl status <service>` / `journalctl -u <service>`
- [ ] For remote hosts: `nixos-rebuild --flake .#host --target-host root@host switch`
- [ ] For Docker images: `nix build .#<image>` then `docker load < result`
- [ ] For Microsandbox: `just load-images` (or `just load-pi-image`)
- [ ] Confirm the new generation appears in the GRUB menu

## Runtime / debugging checklist

- [ ] Boot fails → select an older generation in the GRUB boot menu
- [ ] Boot fails and GRUB unavailable → boot a live USB, mount root, use
      `nixos-enter --root /mnt`
- [ ] Service didn't restart → `nixos-rebuild` only does `daemon-reload` for
      user services; restart manually with `systemctl --user restart <service>`
- [ ] Wrong config value → `nixos-option <opt>` to query the effective value
- [ ] Config value unclear → `nixos-rebuild repl` or
      `nix repl -f '<nixpkgs/nixos>'` to inspect interactively
- [ ] Container won't start → `nixos-container status <name>`, then
      `journalctl -u container-<name>`
- [ ] Rescue/repair → `nixos-enter --root /mnt -- <command>`
- [ ] Need to revert → `nixos-rebuild --rollback switch`
- [ ] VM state stale → delete `nixos.qcow2` when the VM user config changes,
      so stale state does not mask the new config
- [ ] Remote deploy fails → verify SSH access as root, Nix installed on build
      host, matching architecture or distributed builds configured

## Validation hooks

- `nixos-rebuild dry-build` — compile check, no activation.
- `nixos-rebuild dry-activate` — show activation actions without applying.
- `nixos-rebuild build-vm` + `./result/bin/run-*-vm` — test in a QEMU sandbox.
- `nix-build -E "((import <nixpkgs> {}).nixos [ ./configuration.nix ]).installTest"`
  — disko install test (crawl 27).
- `nix flake check` — validate flake outputs (for flake-based NixOS).
- `nixos-rebuild repl` — inspect the evaluated config interactively.
- `nixos-option <option>` — query the effective value of a NixOS option.
- `docker load < result` — verify a Nix-built image loads into Docker.
- `msb image ls` — verify images loaded into Microsandbox.

## Examples

### Example 1: Local flake-based switch

```shell-session
$ sudo -i nixos-rebuild --flake .#myhost switch
```

Builds the `nixosConfigurations.myhost` flake output, sets it as the boot
default, and activates it in the running system.

### Example 2: Remote deploy

```shell-session
$ sudo -i nixos-rebuild --flake .#myhost \
    --target-host root@10.0.0.5 switch
```

Builds locally, copies the closure to `10.0.0.5` over SSH, and activates it
there. The remote host must be reachable as `root` via SSH.

### Example 3: nixos-anywhere initial provisioning

The three-line provisioning snippet from crawl 27:

```shell-session
toplevel=$(nixos-rebuild build --no-flake)
diskoScript=$(nix-build -E "((import <nixpkgs> {}).nixos [ ./configuration.nix ]).diskoScript")
nixos-anywhere --store-paths "$diskoScript" "$toplevel" root@target-host
```

This partitions, formats, mounts, installs, and reboots the target. Subsequent
updates use `nixos-rebuild --target-host root@target-host switch` —
`nixos-anywhere` is only needed again if the disk layout changes.

### Example 4: Project Docker image load

```shell-session
$ just load-images
```

This runs the `load-images` derivation, which iterates `workload-images` and
for each image executes:

```shell-session
out=$(nix build .#workestrator-pi --no-link --print-out-paths)
gunzip -c "$out" | msb load -t workestrator-pi:latest
```

To load a single image:

```shell-session
$ just load-pi-image
```

### Example 5: Raspberry Pi SD image dd

From crawl 24:

```shell-session
# Verify the SD card device first!
$ lsblk
NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINT
sdb      8:16   1  29.7G  0 disk

# Decompress the image
$ unzstd nixos-sd-image-23.11pre500597.0fbe93c5a7c-aarch64-linux.img.zst

# Write to the SD card
$ sudo dd if=nixos-sd-image-23.11pre500597.0fbe93c5a7c-aarch64-linux.img \
    of=/dev/sdb bs=4096 conv=fsync status=progress
```

After booting the Pi, apply a custom config:

```shell-session
# nixos-rebuild boot
# reboot
```

## Common mistakes

- Using `switch` for a kernel/bootloader change that needs a reboot — use
  `boot` instead.
- Expecting `nixos-rebuild` to restart user services — it only does
  `daemon-reload`; restart user services manually.
- Running `nixos-anywhere` for routine updates — use
  `nixos-rebuild --target-host` instead; `nixos-anywhere` is for initial
  provisioning or disk layout changes only.
- Forgetting `--build-host` when local arch ≠ target arch (build fails or
  produces the wrong closure).
- Loading a Nix-built Docker image with `docker run` before `docker load` —
  the image must be imported into the daemon first.
- Using `nixos-rebuild test` and expecting persistence across reboot — `test`
  does not set the boot default; reboot reverts.
- Hardcoding `/dev/sda` in disko config without verifying with `lsblk` on the
  target — device names vary; always confirm.
- Forgetting to delete `nixos.qcow2` when the VM user config changes, so stale
  state masks the new config.
- Running `nixos-rebuild` as a non-root user without `sudo -i` — activation
  requires root.
- Relying on nixpkgs channels for reproducible deployment instead of pinning
  at the flake level.
- Garbage-collecting old generations too aggressively, removing the ability to
  roll back.
- Confusing `--target-host` (where to activate) with `--build-host` (where to
  build).

## Strict vs contextual guidance

**Strict (always do):**

- Always run `nixos-rebuild` as root (`sudo -i` or root shell).
- Always validate with `dry-build` first for production hosts.
- Always verify the target disk device with `lsblk` before `dd` or `disko`.
- Always pin nixpkgs at the flake level for reproducible deployment.
- Always load a Nix-built Docker image (`docker load` / `msb load`) before
  attempting to run it.
- Always use `nixos-enter` (not raw `chroot`) for rescue/repair on a NixOS
  root.

**Contextual (depends):**

- `switch` vs `boot` depends on whether a reboot is acceptable (kernel/boot
  changes require `boot`).
- `nixos-anywhere` vs `nixos-rebuild --target-host` depends on whether the
  disk layout changed (nixos-anywhere for initial/disk changes; rebuild for
  routine updates).
- `--build-host` is needed only when the local machine cannot build the
  closure (architecture mismatch or insufficient resources).
- `-p <profile>` is useful when separating test configs from stable ones in
  GRUB; not needed for single-purpose hosts.
- `nixos-rebuild test` vs `switch` depends on risk tolerance — `test` for
  risky changes (reboot reverts), `switch` for standard deployment.
- `nixos-generate` vs `nixos-rebuild build-image` — the latter is the newer
  built-in; the former is the nixos-generators equivalent with broader format
  support.
- `docker load` vs `msb load` depends on the target runtime (Docker daemon vs
  Microsandbox).

## Policy decisions for individual repos

- This repo (`ai-workbench`) deploys Docker/OCI images into Microsandbox, NOT
  NixOS hosts. Deployment = `nix build .#<image>` + `msb load` (via
  `just load-images`).
- nixpkgs pinned at the flake level in `flake.nix`.
- No `nixos-rebuild`, `nixos-anywhere`, or NixOS configuration in this repo.
- Image loading uses `--no-link --print-out-paths` to avoid GC roots in
  `/tmp`.
- The `load-images` derivation is driven by the `workload-images` attrset —
  no hardcoded image names; adding a new image to the attrset automatically
  includes it in the load cycle.
- `pi-image` and `tempest-image` are also exposed as top-level flake outputs
  for direct `nix build .#pi-image` / `nix build .#tempest-image` access.

## Related docs

- `/docs/nix/overview.md` — Nix overview
- `/docs/nix/flake-anatomy.md` — Flake anatomy (nixpkgs pinning, flake outputs)
- `/docs/nix/modules-and-config.md` — NixOS modules and configuration
- `/docs/nix/docker-images.md` — Docker/OCI image building (dockerTools)
- `/docs/nix/cross-compilation.md` — Cross compilation (remote/arch-mismatch
  builds)
- `/docs/nix/ci-cd-integration.md` — CI/CD integration
- `/docs/nix/source-map.md` — Nix source map (provenance index)

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference

## Citations

1. [NixOS Manual — Changing the Configuration](https://nixos.org/manual/nixos/stable/#sec-changing-config) (crawl 73)
2. [NixOS Manual — Configuration Syntax](https://nixos.org/manual/nixos/stable/#sec-configuration-syntax) (crawl 72)
3. [nix.dev — Provisioning remote machines via SSH](https://nix.dev/tutorials/nixos/provisioning-remote-machines.html) (crawl 27)
4. [nix.dev — Deploying NixOS using Terraform](https://nix.dev/tutorials/nixos/deploying-nixos-using-terraform.html) (crawl 22)
5. [nix.dev — Installing NixOS on a Raspberry Pi](https://nix.dev/tutorials/nixos/installing-nixos-on-a-raspberry-pi.html) (crawl 24)
6. [nix.dev — NixOS virtual machines](https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html) (crawl 26)
7. [nix.dev — Building and running Docker images](https://nix.dev/tutorials/nixos/building-and-running-docker-images.html) (crawl 20)
8. [nix.dev — Building a bootable ISO image](https://nix.dev/tutorials/nixos/building-bootable-iso-image.html) (crawl 21)
9. [nixos-anywhere](https://github.com/nix-community/nixos-anywhere)
10. [disko](https://github.com/nix-community/disko)
11. [terraform-nixos (deploy_nixos module)](https://github.com/tweag/terraform-nixos)
12. [nixos-generators](https://github.com/nix-community/nixos-generators)
13. Local: `/flake.nix` — `load-images`, `workload-images`, `pi-image`,
    `tempest-image`
