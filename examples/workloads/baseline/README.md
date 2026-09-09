# Baseline workload

A minimal service image with BusyBox, coreutils and a local user database.
The workload prints `SMOKE_OK`, remains alive for five minutes, and permits no
ingress or egress. It has no host mounts, credentials or external providers.

```sh
nix flake check --no-update-lock-file
nix build --no-update-lock-file .#workestrate-smoke
```

The check inspects the actual image archive without extracting it or starting a
VM. Copy this complete directory, including `flake.lock`, as a starting point
for an independent workload repository. Rename both the workload and image when
creating a different workload. Its flake follows the exact shared tooling pin;
it does not require a Workestrate checkout or a development shell.

`workestrate.toml` uses the currently supported standalone config-repository
layout. Set `WORKESTRATE_CONFIG_DIR` to this directory for CLI validation or use
an explicitly isolated home registry for runtime testing. Keep `workdir = "/tmp"`:
the CLI's default `/app` is not present in this minimal image.

Building an image does not launch the workload. `workload build smoke` additionally
loads it into the selected microsandbox store; `workload up smoke` starts it.
Use only a reviewed test home/store and stop the exact workload afterwards.
