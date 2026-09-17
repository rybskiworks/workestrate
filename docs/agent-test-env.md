# Agent test environment

Automated agent containers used for verification provide KVM, Lix, and a
pinned Microsandbox CLI. Do not assume these are missing: run `workestrate
check` and `workestrate doctor` first and record what is actually present
before skipping a VM gate.

## Provided

- KVM: `/dev/kvm` is present and usable, nested virtualization is enabled,
  and the Nix system features include `kvm`.
- Lix on x86_64-linux with the project flake inputs. Use `just bootstrap`
  for pinned tools or `just shell` for the full development environment;
  do not rely on ambient host binaries.
- Microsandbox CLI: use the wrapper, `workestrate msb -- <args>`, which
  forwards verbatim to the exact pinned build (`MSB_PATH` set by
  `nix/packages/agentctl.nix`). `workestrate versions` reports the
  tool/runtime/guest-agent identities without starting guests.
- OCI reachability: the container registry is reachable, so image pulls work.

## Caveats

- `msb` is not on `PATH`, and no workestrate binary exists until built from
  source. Outside the wrapper, set `MSB_PATH` to the pinned store binary
  explicitly; `doctor` probes `MSB_PATH` before falling back to `PATH`.
- Disk on the source-checkout mount is tight and `/tmp` is a small tmpfs.
  Keep prover outputs, VM disks, and large downloads in disposable scratch
  outside the checkout, following [Nix purity](nix-purity.md).
- `MSB_HOME` defaults to the generation-keyed home under
  `$HOME/.microsandbox/current`. Relocate it explicitly when the default
  home mount is small; see [runtime provisioning](runtime-provisioning.md).

Exact versions and device permissions above are measured properties of the
verification containers as of 2026-09-17, not system contracts. Re-check
them when a gate behaves unexpectedly.
