# Windows

[Installation](README.md) · [Linux with Nix](linux.md) · [NixOS](nixos.md)

Our favourite Windows feature is the option to install Linux.

Workestrate currently targets x86_64 Linux with KVM. For everyday use, choose a
Linux host and follow the [Linux](linux.md) or [NixOS](nixos.md) guide. You can
also keep Windows on your desk and operate Workestrate over SSH on a Linux
machine.

## WSL: for the curious

Running microVMs inside WSL 2 adds another layer of virtualization. Treat this
as an experiment, not a supported or recommended Workestrate setup. A Linux
host is the shorter route to running your fleet.

<details>
<summary>WSL settings and prerequisites</summary>

For that experiment, Microsoft's WSL settings provide `nestedVirtualization`
on Windows 11. Merge this into the `[wsl2]` section of
`%UserProfile%\.wslconfig`:

```ini
[wsl2]
nestedVirtualization=true
```

Save work in all WSL sessions before running
[`wsl --shutdown`](https://learn.microsoft.com/en-us/windows/wsl/basic-commands#shutdown)
in PowerShell, then reopen your distribution. This stops all running WSL
distributions. See
[Microsoft's WSL configuration reference](https://learn.microsoft.com/en-us/windows/wsl/wsl-config)
for the setting and restart procedure.

The setting alone does not establish usable KVM. Your hardware, Windows host,
and WSL kernel must expose a working `/dev/kvm` with access for your Linux user.
If that is available on x86_64 Linux, you can investigate the
[Linux installation steps](linux.md); `workestrate doctor` and an actual
workload launch remain separate checks.

</details>

Once your Linux host is ready, [set up your fleet →](../getting-started.md#configuration)
