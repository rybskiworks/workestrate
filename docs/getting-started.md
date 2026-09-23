# Getting started

Connect a fleet, inspect its plan, and start a workload. A **fleet** supplies
workload definitions, image builds, policies, and encrypted secrets. Your
**config** registers the fleets you use and keeps local overrides and state.

Have your fleet's Git URL and operating instructions ready, or
[create a fleet](#create-a-fleet) of your own. Replace angle-bracket placeholders
with your values. To explore without launching a VM, try the
[reference preview](#preview-the-reference-configuration).

## Install the CLI

Follow the [installation guide for your system](install/README.md), then return
here to connect a fleet. Installation covers host prerequisites, the CLI, and
runtime provisioning.

## Configuration

### Register an existing fleet

Initialize your config, then clone and register your fleet as `personal`:

```sh
workestrate config init
workestrate fleet add <fleet-git-url> personal
workestrate fleet list
workestrate --fleet personal validate-config
workestrate --fleet personal workloads
```

`personal` is the name you chose in `fleet add`. The first registered fleet
becomes the default; this guide names it explicitly in workload commands.
`fleet add` uses the repository's `main` branch by default; add
`--ref <branch-or-tag>` to select another ref.

The last two commands validate the merged configuration and list its workloads.
Choose a service or agent from that list. Run subsequent commands from the
project directory you intend to expose: `${CWD}` mounts and per-directory
instances use that directory. If the project supplies a configuration layer,
[review and enable it](#enable-project-configuration) before continuing.

## SOPS workflow

For a fleet that needs secrets, inspect the required names and target:

```sh
workestrate --fleet personal secrets schema
workestrate secrets target personal
```

`schema` lists configured secret environment names. `target` reports the fleet
folder, encrypted file, and age key path. The Nix package supplies SOPS and age.
Keep the age private key on the host, outside repositories, the config, and
workload mounts; back it up securely.

Create a new encrypted secrets file with:

```sh
workestrate secrets init --fleet personal
```

`init` creates the age key when needed and opens the secrets editor. Enter values
and confirm the save. For an existing file, use
`workestrate secrets update --fleet personal`; its SOPS recipients must include
your age public key.

Before interactive editing, clear unrelated secret variables from your shell:
nonempty schema keys select targeted replacement mode. The
[secrets guide](secrets.md) covers recipients, input modes, and automation.

For a fleet registered from a Git URL, commit the encrypted file and any
`.sops.yaml` changes in the checkout reported by `secrets target`, following the
fleet's Git workflow. Then refresh the revision used by workload commands:

```sh
workestrate fleet update personal
```

This makes the committed ciphertext available in the fleet's pinned archive.
Local-path fleets created by `fleet new` read their files directly.

## Before launching

From your project directory, check host readiness and inspect the workload plan:

```sh
workestrate --fleet personal check
workestrate doctor
workestrate --fleet personal workload plan <workload> --show-source
```

`check` reports configuration paths and fleet selection. `doctor` checks host
tools, capabilities, runtime pairing, and state health. The plan shows the image,
command, mounts, network policy, and credential bindings; `--show-source` reveals
the configuration layer behind each field.

Review the plan and resolve host failures before starting. Follow any additional
setup in your fleet's instructions. `workload up` and `workload exec` prepare
configured images and start dependencies automatically.

## Run a workload

Start a service in the background and read its logs:

```sh
workestrate --fleet personal workload up <service>
workestrate --fleet personal workload logs <service>
```

Or start an interactive agent in your project:

```sh
workestrate --fleet personal workload exec <agent>
```

In a second terminal, inspect instances and check the fleet's documented health
endpoint or readiness signal:

```sh
workestrate instances
workestrate ps --json
```

`instances` reconciles registry and backend state; `ps` shows registered
instances. Configuration validation and VM readiness are separate checks; verify
the running application's health and policy behavior for deployment acceptance.
See [testing](testing.md) for those checks and the
[CLI guide](cli.md#workloads-and-instances) for parallel instances, dependency
selection, and foreground services.

## Stop a workload

Stop the workload's current slot from the same project directory:

```sh
workestrate --fleet personal workload down <workload>
```

To stop all managed workloads belonging to this fleet:

```sh
workestrate down --fleet personal
```

The fleet operation asks for confirmation. Check `workestrate instances`
afterward. The [CLI guide](cli.md#stopping-workloads) explains the other teardown
scopes.

## Other setup paths

### Choose a config path

The usual location is `~/.workestrate`. `WORKESTRATE_CONFIG` and existing XDG
settings can select another location; `workestrate check` reports the resolved
paths. Use `--config` to select a registry for one invocation, and `--fleet` to
choose a fleet inside it:

```sh
workestrate --config /path/to/operator-config --fleet personal workloads
workestrate --config /path/to/operator-config secrets update --fleet personal
```

`--config` also selects associated Workestrate directories. Microsandbox runtime
state uses `MSB_HOME`; the packaged default is `~/.microsandbox/current`.

### Create a fleet

Use this in place of `fleet add`:

```sh
workestrate fleet new personal
```

This registers a local-path fleet in the config's managed store. Edit the
`workestrate.toml` at the printed path, then run
`workestrate --fleet personal validate-config` and
`workestrate --fleet personal workloads`. Local-path fleets load edits directly.
The [workload guide](workloads.md) explains definitions and image flakes.

An explicit destination outside the managed store creates a standalone scaffold.
Publish that repository and register its URL to use it as a managed fleet.

### Set up another machine

Clone an existing config in place of `config init` and `fleet add`:

```sh
workestrate config clone <config-path-or-git-url>
workestrate fleet list
```

This provisions the registry and its fleet checkouts. An optional second
argument selects the destination; use `--config <destination>` afterward.
Transfer your age private key separately through your secure key backup process
when the cloned fleets already contain encrypted secrets.

### Enable project configuration

Review the project's configuration layer, then permit Workestrate to load it:

```sh
workestrate fleet trust /path/to/project
```

Use `workestrate fleet untrust /path/to/project` to remove permission, or the
top-level `--no-project-config` flag to skip project layers for one invocation.

## Preview the reference configuration

From a Workestrate checkout, build the CLI and inspect the bundled examples:

```sh
nix build --no-update-lock-file .#workestrate
./result/bin/workestrate --help
WORKESTRATE_FLEET_DIR="$PWD/config.reference" ./result/bin/workestrate validate-config
WORKESTRATE_FLEET_DIR="$PWD/config.reference" ./result/bin/workestrate workload plan example-service
```

The preview builds the CLI, validates a synthetic fixture, and prints a plan.
It needs neither KVM nor secrets. Use your own fleet to launch an application.

## Setup checklist

For a human or agent carrying out setup, record these checkpoints in the handoff:

1. **Host:** CLI version, installation result, and `doctor` result.
2. **Selection:** config path, fleet name and Git ref, and project directory.
3. **Configuration:** validation result and selected workloads.
4. **Secrets:** required names, target, and provisioning result. Keep values private.
5. **Plan:** reviewed image, mounts, policy, and credential bindings.
6. **Execution:** requested instances and observed application readiness.
7. **Cleanup:** teardown scope and resulting instance state.

Carry out the provisioning and lifecycle steps the operator requested. For a
configuration review, finish at the relevant inspection checkpoint. Name any
unavailable tool or host so the next action is clear.
