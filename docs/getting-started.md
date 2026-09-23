# Getting started

This guide takes an operator from a fresh host to a running workload. A **fleet**
provides workload definitions, image builds, policies, and encrypted secrets.
Your **config** keeps the fleet registry, local overrides, and Workestrate state.

You need x86_64 Linux, Git, and Nix with `nix-command` and `flakes` enabled.
Running workloads also requires accessible `/dev/kvm` and enabled hardware
virtualization. Have your fleet's Git URL and operating instructions ready;
[creating a fleet](#create-a-fleet) is another starting point.

For an initial look at configuration and plans, use the
[reference preview](#preview-the-reference-configuration). The
[CLI guide](cli.md) covers command families, selectors, and automation.
Replace angle-bracket placeholders in the examples with your own values.

## Install the CLI

The host provisioning wrapper checks prerequisites, installs the matching CLI
in your Nix profile, prepares the Microsandbox state generation for the pinned
runtime, and runs `doctor`. On a host with existing Microsandbox state, review
[runtime provisioning](runtime-provisioning.md) first: generation convergence
can migrate state and requires quiescent sandboxes. Use
`./scripts/host-provision.sh --check-only` to inspect an existing host.

For a fresh host, clone the project and run the wrapper:

```sh
git clone --branch main https://github.com/rybskiworks/workestrate.git
cd workestrate
./scripts/host-provision.sh
workestrate --version
workestrate --help
```

Resolve reported failures and review warnings before launching. The
[Nix build guide](nix-build.md) covers package builds and development shells.

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

`personal` is the registry name chosen in `fleet add`. Use the same name for the
following commands. `fleet add` uses the repository's `main` branch by default;
add `--ref <branch-or-tag>` when your fleet uses another ref.

The final two commands validate the merged configuration and list its workload
names and kinds. Choose a service or agent from that list. The first registered
fleet becomes the default; explicit `--fleet personal` keeps this guide's
selection consistent across project directories and Git branches.

The usual config location is `~/.workestrate`. `WORKESTRATE_CONFIG` and existing
XDG settings can select another location; `workestrate check` reports the resolved
paths. To use a particular config, prefix subsequent commands with its path:

```sh
workestrate --config /path/to/operator-config --fleet personal workloads
```

`--config` selects the registry and associated Workestrate directories.
`--fleet` selects a registered fleet. Microsandbox runtime state has its own
selector, `MSB_HOME`; the packaged default is `~/.microsandbox/current`.

### Create a fleet

To start with your own definitions, use this in place of `fleet add`:

```sh
workestrate fleet new personal
```

This creates and registers a local-path fleet in the config's managed store.
Local-path fleets load edits directly. Edit the
`workestrate.toml` at the printed path to define workloads, then run
`workestrate --fleet personal validate-config` and
`workestrate --fleet personal workloads`. The
[workload guide](workloads.md) explains configuration layouts and image flakes.
An explicit destination outside the managed store creates a standalone scaffold;
publish that repository and register its URL to use it as a managed fleet.

### Set up another machine

When you already have a config repository, clone it in place of `config init`
and `fleet add`:

```sh
workestrate config clone <config-path-or-git-url>
workestrate fleet list
```

The clone operation provisions the registry and its fleet checkouts. You can
supply a destination as a second argument and use `--config <destination>` on
later commands. Transfer the age private key separately through your secure key
backup process when the cloned fleets already contain encrypted secrets.

### Enable project configuration

Run workload commands from the project directory you intend to expose. `${CWD}`
mounts and per-directory instances use that invocation directory.

If the project includes a Workestrate configuration layer, review it and enable
it with:

```sh
workestrate fleet trust /path/to/project
```

This records permission to load that project's configuration. Use
`workestrate fleet untrust /path/to/project` to remove it, or the top-level
`--no-project-config` flag to skip project layers for one invocation.

## SOPS workflow

For a fleet that needs secrets, inspect the names and destination:

```sh
workestrate --fleet personal secrets schema
workestrate secrets target personal
```

`schema` prints configured secret environment names. `target` reports the fleet
folder, encrypted file, and age key path. The Nix package supplies SOPS and age.
Keep the age private key on the host, outside repositories, the config, and
workload mounts; back it up securely.

For a new encrypted secrets file:

```sh
workestrate secrets init --fleet personal
```

`init` creates the age key when needed and writes the encrypted file. Follow the
editor's instructions to enter values and confirm the save. An existing fleet's
SOPS recipients must include your age public key; see the
[secrets guide](secrets.md) for recipient and key handling.

To change an existing file:

```sh
workestrate secrets update --fleet personal
```

For interactive editing, clear unrelated secret variables from your shell first:
any nonempty schema key in the environment selects targeted replacement mode.
Automation can supply values through the process environment or stdin. Keep
values out of command arguments, logs, and reports. See
[secrets management](secrets.md) for input modes and layered secret loading.

For a fleet registered from a Git URL, commit the encrypted file and any
`.sops.yaml` changes in the checkout reported by `secrets target`, following the
fleet's Git workflow. Then refresh the revision used by workload commands:

```sh
workestrate fleet update personal
```

This makes the committed ciphertext available in the fleet's pinned archive.
Local-path fleets created by `fleet new` read their files directly.

With a custom config, select both the registry and the fleet explicitly:

```sh
workestrate --config /path/to/operator-config secrets update --fleet personal
```

## Before launching

From the intended project directory, inspect configuration and host readiness:

```sh
workestrate --fleet personal check
workestrate doctor
workestrate --fleet personal workload plan <workload> --show-source
```

`check` reports config paths, fleet selection, and layout. `doctor` reports host
capabilities, tools, runtime pairing, and state health. The plan describes the
resolved image, command, mounts, network policy, and credential bindings;
`--show-source` identifies the configuration layer behind each field.

Confirm the project path, permissions, destinations, and secret bindings in the
plan. Resolve host failures before starting a workload. Fleet instructions may
also require image builds or application setup; `workload up` and `exec` perform
configured image preparation and dependency startup automatically.

Configuration validation establishes that the declared setup is accepted.
Verify service health and policy behavior in the running VM for deployment
acceptance; [testing](testing.md) explains those separate checks.

## Run a workload

For a service, start it detached and inspect its logs:

```sh
workestrate --fleet personal workload up <service>
workestrate --fleet personal workload logs <service>
```

For an interactive agent, attach from the project directory:

```sh
workestrate --fleet personal workload exec <agent>
```

Use a second terminal to inspect instances and the fleet's documented health
endpoint or readiness signal:

```sh
workestrate instances
workestrate ps --json
```

`instances` reconciles registry and backend state; `ps` shows Workestrate's
registered instances. The [CLI guide](cli.md#workloads-and-instances) covers
parallel instances, dependency selection, image builds, and foreground services.

## Stop a workload

Stop the workload's current slot from the same project directory:

```sh
workestrate --fleet personal workload down <workload>
```

To stop all managed workloads belonging to this fleet, use the wider scope:

```sh
workestrate down --fleet personal
```

The fleet operation asks for confirmation. Inspect `workestrate instances`
afterward to check the resulting state. Further scopes and their effects are in
[stopping workloads](cli.md#stopping-workloads).

## Preview the reference configuration

From a Workestrate checkout, build the CLI and inspect the bundled examples:

```sh
nix build --no-update-lock-file .#workestrate
./result/bin/workestrate --help
WORKESTRATE_FLEET_DIR="$PWD/config.reference" ./result/bin/workestrate validate-config
WORKESTRATE_FLEET_DIR="$PWD/config.reference" ./result/bin/workestrate workload plan example-service
```

These commands build the tool, validate the synthetic fixture, and print a plan.
The preview needs neither KVM nor secret provisioning. To launch an application,
follow the setup above with your own fleet.

## Setup checklist

For an agent performing setup, capture these inputs and checkpoints in the
handoff. Keep secret values out of it.

| Checkpoint | Record |
| :--- | :--- |
| Installation | CLI version; host provisioning and `doctor` results. |
| Selection | Config path, fleet registry name and Git ref, project directory. |
| Configuration | `validate-config` result and selected workload names. |
| Secrets | Required names, resolved target, and provisioning result. |
| Plan | Reviewed image, mounts, policy, and credential bindings. |
| Execution | Requested instances and observed application readiness. |
| Cleanup | Teardown scope and observed instance state afterward. |

Carry out the provisioning and lifecycle steps the operator requested. When
asked only to review or validate configuration, finish at the relevant inspection
checkpoint. Report unavailable tools or hosts by name so the next action is clear.
