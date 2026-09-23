# CLI guide

Use the [setup guide](getting-started.md) for a first deployment. This page maps
the public command families and explains the selectors used in everyday work.
For every option, consult the installed CLI's help:

```sh
workestrate --help
workestrate workload --help
workestrate workload up --help
```

## Command shape

```text
workestrate [global options] <command> [subcommand] [arguments] [options]
workestrate --config <directory> --fleet <fleet> workload plan <workload>
```

Replace angle-bracket placeholders with your values. Workload names are
arguments after a verb: `workload up web`, `workload exec assistant`, and
`workload down web`.

| Common option | Purpose |
| :--- | :--- |
| `--help`, `--version` | Inspect the interface or installed version. |
| `--config <DIR>` | Select the operator config and its registry. |
| `--fleet <NAME>` | Select a registered fleet for workload/config commands or a fleet target for secret provisioning. |
| `--config-ref <REF>` | Read Git-backed configuration entries at a branch or commit. |
| `--show-source` | Include configuration provenance in a workload plan. |
| `--no-project-config` | Skip project configuration layers; place this flag before the top-level command. |
| `--json` | Request JSON from commands that support it, plus structured error output. |
| `--secrets-source layered\|env` | Choose normal layered loading or explicit environment-only secret input. See [development secrets](development-env-secrets.md). |

JSON output is available for fleet list/new, workload plans, instances, `ps`,
`doctor`, `versions`, secrets target, signing keys, image builds, and scoped
teardown. Other commands can still print text with `--json`; consult command help
before depending on an output shape. Launch failures use the JSON error envelope
when requested.

## Selecting config and fleets

A config contains `config.toml` (the fleet registry), overrides, and the associated
store/state directories. A fleet contains workload definitions and policy. Select
both when an invocation needs a particular operator setup:

```sh
workestrate --config /path/to/operator-config --fleet personal workloads
workestrate --config /path/to/operator-config secrets target personal
```

Config path precedence is `--config`, `WORKESTRATE_CONFIG`, the legacy layout
selected by nonempty XDG directory variables, then `~/.workestrate`.
`workestrate check` reports the chosen paths.

Active fleet selection uses an explicit `--fleet`/`WORKESTRATE_FLEET` first,
then a branch derived from `--config-ref`, then the first layer's checkout
branch, then `[settings].default_fleet`. Derived branch names are
normalized for instance naming. When a derived name has no registered fleet,
it can name a runtime slot while using the default fleet's content.
Explicit `--fleet` is the clearest choice for repeatable operations.

Fleets registered from remote or `git+file://` URLs consume a recorded revision;
`fleet update` refreshes it. Plain local-path entries, including `fleet new`
scaffolds, load the working files directly. `--config-ref` reads a particular ref
across remote and `git+file://` entries; `workload plan <name>:<ref>` reads that
workload's declaring repository at a ref. See
[configuration resolution](../README.agents.md#configuration-model) for layering
and pinned content.

`WORKESTRATE_FLEET_DIR` loads a directory's `workestrate.toml` directly for local
inspection. Set it for a single command, as in the
[reference preview](getting-started.md#preview-the-reference-configuration).
Run lifecycle commands from the intended project directory: `${CWD}` mounts and
per-directory slots use the captured invocation directory.

Microsandbox state is selected separately with `MSB_HOME`, defaulting in the
package to `~/.microsandbox/current`. Runtime isolation for tests requires
explicitly disposable backend state as well as a disposable config; see
[runtime provisioning](runtime-provisioning.md) and [testing](testing.md).

## Configuration and fleet management

| Command | Effect |
| :--- | :--- |
| `config init` | Initialize the selected config as a Git repository with a registry, schemas, and hooks. |
| `config clone <src> [dest]` | Provision a config and its fleet checkouts from a local config or Git URL. |
| `fleet add <url> <name> [--ref <ref>]` | Clone/register a fleet and record its revision; default ref is `main`. |
| `fleet new <name> [dest]` | Scaffold a fleet; the managed-store destination is registered automatically. |
| `fleet list` | Inspect registered fleets, revisions, and working-copy status. |
| `fleet update [name]` | Refresh one fleet, or all fleets when the name is omitted. |
| `fleet remove <name>` | Remove the registry entry; `--delete` also removes its stored checkout. |
| `fleet trust <dir>` / `fleet untrust <dir>` | Enable or disable a project's configuration layer. |

`fleet update` refuses dirty Git checkouts. Review and preserve edits before
updating. `fleet remove --delete --force` permits deleting a dirty checkout.

`fleet new --help` lists scaffolding choices, including age recipients,
`--with-flake`, and `--empty`. A destination outside the managed store creates an
unregistered scaffold. The [workload guide](workloads.md) describes file and
directory layouts.

## Workloads and instances

| Command | Use |
| :--- | :--- |
| `workloads` | List configured workload names, kinds, images, and running status. |
| `workload plan <name>` | Inspect the resolved workload before launch. |
| `workload up <service>` | Start a service detached, including declared dependencies. |
| `workload up` | Start all services in the active fleet in dependency order. |
| `workload exec <agent>` | Start or attach to an interactive agent workload. |
| `workload logs <service>` | Tail the detached service's log file. |
| `workload down <name>` | Stop and remove the workload's selected slot. |
| `workload build [name]` | Build/load Nix images for one workload or all Nix-image workloads in the active fleet. |
| `workload new <name> [--kind agent\|service]` | Scaffold a workload in a file-mode configuration. |
| `ps` | Read Workestrate's instance registry. |
| `instances [workload]` | Reconcile registry and backend state, optionally filtered by workload. |

Use `workload new` with file-mode fleets; directory-mode fleets need an explicit
capsule following the [workload guide](workloads.md).

### Launch and instance options

| Option | Applies to | Meaning |
| :--- | :--- | :--- |
| `--foreground` | Service `up` | Keep the service attached until Ctrl-C. |
| `--instance <id>` | Named `up`, `exec`, `plan`, `logs`, `down` | Address a particular parallel instance. |
| `--new` | Named `up`, `exec` | Allocate the next free numeric parallel instance. |
| `--port-auto` | `up`, `exec` | Choose free host ports and record the resulting mappings. |
| `--replace` | `up`, `exec` | Tear down an existing selected instance before starting. |
| `--use <dependency>@<instance>` | Named `up`, `exec`, `plan` | Choose the running instance of a declared dependency; repeat for several dependencies. |
| `--no-deps` | `up`, `exec` | Leave dependency startup to the caller. |
| `--reseed` | Named `up`, `exec` | Render template seed files over existing targets. |
| `--reload-images` | `up`, `exec` | Force Nix image rebuild/load before launch. |
| `--all-instances` | `down` | Stop every instance of the named workload. |

For example, inspect a second service instance's plan, start it, and view logs:

```sh
workestrate --fleet personal workload plan <service> --instance 2
workestrate --fleet personal workload up <service> --instance 2 --port-auto
workestrate --fleet personal workload logs <service> --instance 2
```

`workload build --check` reports image staleness without building or loading.
Use `--force` to rebuild, `--fleet <name>` for one registered fleet, or
`--all-fleets` for all registered fleets. Image recipes and locked flakes are
covered in [workloads](workloads.md).

## Stopping workloads

Choose the smallest scope that covers the intended cleanup:

| Command | Scope |
| :--- | :--- |
| `workload down <name>` | The workload's current slot. |
| `workload down <name> --instance <id>` | One parallel instance. |
| `workload down <name> --all-instances` | Every instance of that workload. |
| `down --fleet <name>` | Managed instances belonging to the named fleet. |
| `down --config-ref <branch>` | The fleet identity derived from a branch-shaped ref. |
| `down --all` | Every target classified as Workestrate-managed in the selected state and backend homes. |
| `down --everything --everything --yes` | All backend sandboxes in the resolved scope, including unmanaged ones. |

The top-level `down` requires one scope selector and asks for confirmation;
`--yes` supports deliberate noninteractive use. The `--everything` selector must
be repeated. `--config-ref` teardown accepts branch-shaped refs; use a fleet or
instance selector for workloads launched from a commit SHA.

Use `--config <path>` to select the Workestrate state being inspected and
review `MSB_HOME` before broad teardown. `--all` and `--everything` also sweep
retained runtime generations. Management classification uses registry records,
instance names, or Workestrate log artifacts. See
[runtime provisioning](runtime-provisioning.md) for generation ownership, and
verify the result with `instances`.

## Secrets and signing keys

| Command | Use |
| :--- | :--- |
| `secrets schema` | List secret environment names declared by the active configuration. |
| `secrets target <fleet>` | Inspect the registered fleet's directory, encrypted file, and age key path. |
| `secrets init --fleet <name>` | Create a new encrypted file and bootstrap its age key/recipient. |
| `secrets update --fleet <name>` | Change values in an existing encrypted file. |
| `generate-env-example [--output <path>]` | Generate an environment template from the loaded secret schema. |
| `credentials signing generate <NAME> --fleet <name>` | Add a new encrypted Ed25519 signing key to one fleet's existing secrets file. |
| `credentials signing public-key <NAME> --fleet <name>` | Display its public key and fingerprint. |
| `run -- <command> [args...]` | Run a host command with the loaded secrets in its environment. |

For `init` and `update`, choose one target: `--fleet <name>`, an existing
`--fleet-dir <directory>`, or `--global`. Named targets honor registry overrides
for encrypted-file and age-key paths. An explicit fleet name is checked against
the selected registry.

With no selector, provisioning tries `WORKESTRATE_FLEET_DIR`, a single registered
fleet, then a `.sops.yaml` in the invocation directory. Prefer explicit selectors
in scripts. The current `--global` provisioner writes
`${XDG_CONFIG_HOME:-~/.config}/workestrate/.env.local.enc`. Layered loading reads
that path in the legacy XDG layout; the single-config layout reads
`<config>/secrets/.env.local.enc`. Use an explicit fleet target for the setup
workflow while these global paths differ.

`init` creates a new file; `update` requires the existing encrypted file and age
key. Values come from the environment, stdin, or an interactive editor. On
`update`, any nonempty schema key in the process environment selects targeted
replacement of those keys. For editor use, clear unrelated secret variables first.

Provisioning writes the fleet checkout. For remote or `git+file://` fleets,
commit the ciphertext and recipient changes, then run `fleet update <name>` to
refresh the archive read by workload commands. Local-path fleets read edits
directly.

`run` executes on the host and gives its child access to the loaded secrets.
Workload launch commands handle their configured secret bindings themselves.
See [secrets management](secrets.md), [development input](development-env-secrets.md),
and [encrypted signing keys](signing-keys.md) for their respective workflows.
Signing-key provisioning stores material; workload grants and signing integration
have their own configuration.

## Diagnostics and policy

| Command | Evidence provided |
| :--- | :--- |
| `validate-config` | Configuration parses and satisfies schema/policy validation. |
| `check` | Config locations, fleet selection, registered directories, and layout checks. |
| `doctor` | Host tools, KVM access, runtime/state pairing, and remediation guidance. |
| `versions` | Workestrate, Microsandbox, agentd, firmware, schema, and pinned fork identities. |
| `policy mounts explain --workload <name> --mount <guest-path> --path <relative-path>` | Effective mount masking for one path. |
| `policy mounts preview --workload <name> --mount <guest-path> [--root <path>]` | Annotated view of a mount tree. |

Plans and diagnostics inspect the selected configuration and environment.
Application readiness, policy enforcement, and cleanup require observed runtime
checks. See the [testing guide](testing.md) and [mount-policy guide](mount-policy/00-overview.md).

## Maintenance and control

| Command | Use |
| :--- | :--- |
| `source clone <name> [path]` | Clone the source checkout described by a workload. |
| `source build <name>` | Build it using the configured `local_build` recipe. |
| `source list` | Inspect source checkout status. |
| `source reset <name>` | Discard local source edits and restore the canonical checkout. |
| `images gc` | Prune computed Nix-image tags according to retention policy. |
| `clean [--yes]` | Remove the selected state's `workspaces`, `var`, and `run` contents. Stop workloads first. |
| `generate-schema` | Print the configuration JSON Schema, or write schema artifacts with output options. |
| `schemas update [--fleet <name>] [--check]` | Distribute generated schemas to existing consumers, or check for drift. |
| `migrate-config [--from xdg\|bundle] [--dry-run] [--force]` | Move a legacy config layout into the selected config. Inspect the dry run first. |
| `completions <shell> [--for <name>]` | Generate shell completion definitions. |
| `msb -- <args...>` | Forward arguments to the paired Microsandbox CLI. |
| `control serve --state-dir <path> --instance <selector> [--initialize]` | Serve a private operator endpoint for selected running instances. |

`control serve` requires an existing canonical, private, operator-owned directory.
Repeat `--instance` to adopt several running launches; `--initialize` creates a
new desired-state store. Its current native operations cover inspection, stop,
and guest execution. SSH broker integration remains incomplete. Read the
[control owner contract](runtime-provisioning.md#explicit-host-control-owner)
before operating it.

`generate-schema --help` lists the three output paths for configuration,
workload, and registry schemas. `schemas update --check` reports stale copies
without writing. Schema distribution and runtime-state maintenance are detailed
in [runtime provisioning](runtime-provisioning.md).
