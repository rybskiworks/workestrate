# Getting started and configuring secrets

Use the [README build commands](../README.md#start-here) first. Building is separate
from provisioning a home, creating configuration and starting a microVM. Runtime
execution requires Linux/KVM; reading CLI help does not.

## Configuration

```sh
workestrate home init
workestrate config new personal
# Alternative: workestrate config add <repository-url> personal
workestrate config list
workestrate validate-config
workestrate workload plan example-service
```

Use the built CLI's full path until it is on PATH. `home init` is idempotent; it is
not a request to migrate an existing Microsandbox home. Read effective operator
configuration first: a different `--home` alone does not relocate all backend state.

A fresh tool ships synthetic examples, not preferred agents or providers. Real
workload names and secret schemas come from registered config repositories.
Project-layer trust is explicit via `workestrate config trust <dir>`; do not grant
it to arbitrary agent checkouts.

## SOPS workflow

Config repositories can hold encrypted `.env.enc` and `.sops.yaml` files. The
helper honors encrypted-file/key overrides from the registry. With Nix and just:

```sh
just setup-secrets --config personal init
just setup-secrets --config personal update
# Explicit directory: just setup-secrets --config-dir /path/to/config-repo update
# Another home: just setup-secrets --home /path/to/home --config personal update
```

`init` refuses to overwrite an existing encrypted file; use `update` for changes.
Named configs resolve through the tool, without silently falling back to an
unrelated directory for an unknown name. Use `workestrate secrets-target --help`
for the current targeting interface.

Global encrypted layers and multiple recipients are deliberate operator choices.
Do not copy provider-secret lists or host key locations from old session notes.
Keep private keys and plaintext outside agent-reachable source and mounts, and
verify the exact target before editing or re-encrypting anything.

`workestrate run -- <command>` intentionally grants a command decrypted secret
access. It is the operator escape hatch, not the default for an untrusted agent.
Review bindings, mounts and egress policy independently.

## Before launching

```sh
workestrate check
workestrate doctor
workestrate workload plan <name> --show-source
```

Check host/runtime capability, policy provenance, mounts, credentials and images
before starting a configured service or attaching an agent. Read the
[operating model](operating-model.md), [runtime guide](runtime-provisioning.md) and
[security model](migration/30-security-model.md) for the full contracts.
