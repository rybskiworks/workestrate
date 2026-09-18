# Getting started and configuring secrets

Use the [README build commands](../README.md#start-here) first. Building is separate
from provisioning a home, creating configuration and starting a microVM. Runtime
execution requires Linux/KVM; reading CLI help does not.

## Configuration

```sh
# Provision the tool home once per machine, with a config repo attached:
workestrate home init --config <your-config-repo-url> --name personal
# Second machine from an existing home:
# workestrate home clone <src-home-or-git-url> [dest-dir]
# Explicit home for one invocation:
# workestrate --home <tool-home-path> home init
workestrate --home <tool-home-path> config list
workestrate config list
workestrate context current
workestrate validate-config
workestrate workload plan example-service
```

`--home <DIR>` selects the tool-home root (registry, overrides,
state/store). It does not select which config layers are active, and a
different `--home` alone does not relocate all backend state (`MSB_HOME`
stays separate). `home init` is idempotent; it is not a request to migrate
an existing Microsandbox home.

A fresh tool ships synthetic examples, not preferred agents or providers. Real
workload names and secret schemas come from registered config repositories.
Project-layer trust is explicit via `workestrate config trust <dir>`; do not grant
it to arbitrary agent checkouts. Run workload verbs from the intended project
directory: `${CWD}` mounts and per-directory slots resolve against the
captured invocation directory (`WORKESTRATE_INVOKE_CWD`).

## SOPS workflow

Config repositories can hold encrypted `.env.enc` and `.sops.yaml` files. The
`workestrate secrets` provisioning commands honor encrypted-file/key overrides
from the registry; the nix-installed CLI bundles `sops` and `age`, so no
devshell is needed:

```sh
workestrate secrets init --config personal   # first bootstrap only
workestrate secrets update --config personal
# Explicit directory: workestrate secrets update --config-dir /path/to/config-repo
# Pair --home (WHERE the registry lives) with --config (WHICH config to target):
workestrate --home <tool-home-path> secrets update --config personal
```

`--home` without `--config` on a `secrets` command selects no target; always
pair them. `init` refuses to overwrite an existing encrypted file; use `update`
for changes.
Named configs resolve through the registry, without silently falling back to an
unrelated directory for an unknown name. Use `workestrate secrets target --help`
for the targeting inspection interface. The legacy `just setup-secrets` recipe
and `scripts/setup-secrets.sh` still work as deprecated delegates to the same
CLI commands.

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
