# Getting started and configuring secrets

Use the [README build commands](../README.md#start-here) first. Building is separate
from provisioning a config, creating configuration and starting a microVM. Runtime
execution requires Linux/KVM; reading CLI help does not.

## Configuration

```sh
# Provision the config once per machine, with a fleet attached:
workestrate config init --fleet <your-fleet-url> --name personal
# Second machine from an existing config:
# workestrate config clone <src-config-or-git-url> [dest-dir]
# Explicit config for one invocation:
# workestrate --config <config-path> config init
workestrate --config <config-path> fleet list
workestrate fleet list
workestrate context current
workestrate validate-config
workestrate workload plan example-service
```

`--config <DIR>` selects the config root (registry, overrides,
state/store). It does not select which config layers are active, and a
different `--config` alone does not relocate all backend state (`MSB_HOME`
stays separate). `config init` is idempotent; it is not a request to migrate
an existing Microsandbox home.

A fresh tool ships synthetic examples, not preferred agents or providers. Real
workload names and secret schemas come from registered fleets.
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
workestrate secrets init --fleet personal   # first bootstrap only
workestrate secrets update --fleet personal
# Explicit directory: workestrate secrets update --fleet-dir /path/to/fleet
# Pair --config (WHERE the registry lives) with --fleet (WHICH fleet to target):
workestrate --config <config-path> secrets update --fleet personal
```

`--config` without `--fleet` on a `secrets` command selects no target; always
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
