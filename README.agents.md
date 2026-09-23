# Workestrate: architecture and source map

Use this guide to find the code and contracts behind a Workestrate task.
For installation and operation, follow [Getting started](docs/getting-started.md)
and the [CLI guide](docs/cli.md). Repository working instructions are in
[AGENTS.md](AGENTS.md).

## Navigation

[System model](#system-model) · [Ownership](#ownership-and-repository-map) ·
[Configuration](#configuration-model) · [Lifecycle](#lifecycle-and-runtime-state) ·
[Development](#development-workflow) · [Task map](#task-to-source-map)

## System model

Workestrate runs agents and services from declarative fleet configuration. The
Rust CLI resolves configuration, prepares images and policy, and manages workloads
through Microsandbox. Nix supplies the pinned toolchain, packages, and reusable
image recipes.

```text
Fleet repository
  workload declarations · image flakes · policy · encrypted secrets
          │
          ▼
Workestrate CLI
  resolve configuration → inspect plan → prepare image → manage lifecycle
          │
          ▼
Pinned Microsandbox SDK, runtime, and guest agent
  service microVMs · interactive agent microVMs
```

### Vocabulary

| Term | Meaning |
| :--- | :--- |
| Workload | A named agent or service with an image, command, environment, resources, mounts, and policy. |
| Fleet | A registered repository of workload configuration. Its active name also namespaces runtime instances. |
| Layer | A configuration input merged in precedence order, such as a fleet declaration or a trusted project override. |
| Config root | The operator directory containing `config.toml`, overrides, and default fleet, source, and state locations. The usual default is `~/.workestrate`. |
| Slot | A workload's runtime namespace: `<fleet>-<workload>`, or `<workload>` when there is no active fleet. |
| Instance | A running copy identified by its slot or by `<slot>@<id>` for a parallel instance. |
| Microsandbox home | Backend state selected through `MSB_HOME`; packaged defaults use `~/.microsandbox/current`. |

## Ownership and repository map

| Path or owner | Responsibility |
| :--- | :--- |
| [control/agentctl/](control/agentctl/) | Rust CLI, configuration, planning, policy, lifecycle, and backend integration. |
| [flake.nix](flake.nix), [flake.lock](flake.lock) | Dependency pins, package composition, checks, and reusable flake exports. |
| [nix/](nix/) | Tool packaging in `packages/` and shared configuration/image recipes in `lib/`. |
| [config.reference/](config.reference/) | Synthetic declarations for inspection and tests, enabled explicitly. |
| [templates/](templates/), [schemas/](schemas/) | Fleet scaffolding and configuration schemas. |
| [scripts/](scripts/) | Verification, host provisioning, secrets helpers, hooks, and store tools. |
| [.github/](.github/) | CI workflows, review ownership, and repository automation. |
| [docs/](docs/README.md) | Focused guides and contracts, indexed by [SPEC.md](SPEC.md). |
| Fleet and workload repositories | Application images, provider choices, deployment configuration, and application acceptance tests. |

### The dependency boundary

The `tooling` input supplies nixpkgs, Fenix, development modules, and formatting
tools. Workestrate and its Microsandbox input follow the shared supplier pins.
The selected revisions are in [flake.lock](flake.lock); the
[Nix build guide](docs/nix-build.md) describes SDK preparation and packaging.

The Microsandbox input supplies paired SDK, runtime, and guest-agent artifacts.
Workestrate exports image recipes such as `buildImagesFromConfig` and
`buildWorkloadImage`; fleet and workload repositories use them to build application
images. See [workload ownership](docs/workloads.md).

## Configuration model

A fleet declares workloads in `workestrate.toml` or a directory-mode
`workestrate/` tree. The config root's `config.toml` registers fleets and selects
defaults. `--config` selects that root; `--fleet` selects the active fleet.
The [selector guide](docs/cli.md#selecting-config-and-fleets) covers explicit
selection, environment variables, and Git refs.

The loader merges active fleet layers, user-global overrides, trusted project
configuration, and trusted local overrides in that order. Project layers are
`workestrate.toml` and `workestrate.local.toml` in the invocation directory;
`fleet trust` records permission to load them. The CLI captures that directory
at entry in `WORKESTRATE_INVOKE_CWD` for project discovery, `${CWD}` mounts, and
per-directory instance identity.

For development, `WORKESTRATE_FLEET_DIR` directly loads a single
`workestrate.toml`. `WORKESTRATE_REFERENCE_CONFIG=1` enables the synthetic reference
as a base layer; discovery from the current directory also requires
`WORKESTRATE_ALLOW_CWD_REFERENCE=1`. The source for discovery and precedence is
[config/loading.rs](control/agentctl/src/config/loading.rs), with path selection in
[config/paths.rs](control/agentctl/src/config/paths.rs).

Field merging records provenance for `workload plan <name> --show-source`.
Policy resolution applies the authority ladder and `final` seals. Consult
[merge.rs](control/agentctl/src/merge.rs),
[configuration validation](control/agentctl/src/config/validation.rs), and the
[security model](docs/migration/30-security-model.md) for changes to those rules.

### Images, seeds, and workload ownership

For `nix-layered` images, the CLI finds the nearest flake beside the configuration
that declares the image. Fleets can keep one root flake or individual workload
capsules with their own flakes and locks. Mount and seed paths resolve from their
declaring configuration roots. The [workload guide](docs/workloads.md) shows both
layouts and their build contracts.

Seed files prepare mounted workload data before start; templates render against
the guest-visible environment. The [target specification](docs/migration/20-target-system-spec.md)
describes copying, glob expansion, and reseeding.

## Lifecycle and runtime state

Services start with `workload up`; agents attach through `workload exec`.
Dependencies, instance selection, ports, and replacement behavior are explained
in the [workload commands](docs/cli.md#workloads-and-instances) and
[operating model](docs/operating-model.md). Use the
[stopping guide](docs/cli.md#stopping-workloads) to select a workload, fleet, or
wider teardown scope.

The tool's config root and Microsandbox home have separate path selectors.
The packaged backend uses runtime state generations under `~/.microsandbox`, with
`current` pointing to the selected generation; a nonempty `MSB_HOME` selects an
explicit backend home. `MSB_BUILD_RUNTIME` and `MSB_AGENTD_PATH` identify immutable
build artifacts. The [runtime guide](docs/runtime-provisioning.md) covers these
paths, generation migration, guest initialization, disk capacity, and broker
integration.

## Setup

Follow [Getting started](docs/getting-started.md) to install the CLI, register a
fleet, provision secrets, inspect a plan, and run a workload. Its
[setup checklist](docs/getting-started.md#setup-checklist) provides completion
criteria for a human or agent performing setup.

## CLI reference

The [CLI guide](docs/cli.md) covers command shape, selectors, workload lifecycle,
secrets, diagnostics, and maintenance. Each command's `--help` lists its flags.

## Secrets workflow

Fleet secret declarations define the credentials workloads need. SOPS/age encrypts
their values; private age keys stay on the host, outside repositories and the
config root. The [setup walkthrough](docs/getting-started.md#sops-workflow) covers
initial provisioning, and [Secrets](docs/secrets.md) covers targeting, overlays,
updates, and key recovery.

Host-bound bindings give guests placeholders and substitute credentials on
approved host-side traffic. Guest-bound bindings supply the value inside the
guest. SSH custody and signing keys have their own grants and runtime contracts;
see [runtime provisioning](docs/runtime-provisioning.md) and
[signing keys](docs/signing-keys.md).

## Worked example: new config to running workload

The [Getting started walkthrough](docs/getting-started.md) carries setup through
planning, launch, observation, and teardown. For image and configuration examples
to adapt, use the fixtures linked from [Workload layouts](docs/workloads.md).

## Development workflow

Target `main` and follow [CONTRIBUTING.md](CONTRIBUTING.md). Run `just` recipes from
a plain host shell; they select the pinned environment. The
[Nix build guide](docs/nix-build.md) documents individual checks and SDK setup.

| Recipe | Purpose |
| :--- | :--- |
| `just shell` | Full development environment with SDK preparation. |
| `just bootstrap` | Tooling environment for work that needs no application/runtime build. |
| `just verify` | Required local gate: source guards and pinned Nix checks, including Rust, units, packaging, formatting, schemas, and dependency policy. |
| `just check` / `just test` | Focused Rust compilation / unit tests. |
| `just host-check` / `just provision-check` | Host prerequisites / provisioning inspection. |
| `just kvm-tests` | Separate VM tests; prepare the disposable test environment described in the testing guides. |

Keep `CARGO_TARGET_DIR` outside the checkout and stage new source files before Nix
evaluation. See [Nix purity](docs/nix-purity.md). For VM tests, use the
[testing guide](docs/testing.md) and [agent test environment](docs/agent-test-env.md).

### Work tracking, hooks, and review

[BEADS.md](BEADS.md) describes the existing tracker and its single-writer workflow.
Install the pinned hooks with `nix run .#install-hooks`. The
[CI foundation guide](docs/ci-release-foundation.md) explains check selection and
review gates; [governance](docs/github-governance.md) covers release boundaries.

## Flag glossary

See [Selecting config and fleets](docs/cli.md#selecting-config-and-fleets) for
configuration flags and [Stopping workloads](docs/cli.md#stopping-workloads) for
teardown selectors. [Secrets commands](docs/cli.md#secrets-and-signing-keys) explain
which encrypted file a secrets operation targets.

## Task-to-source map

| Task | Start here |
| :--- | :--- |
| Understand system contracts | [Specification map](SPEC.md) and [architecture decisions](docs/migration/50-decisions/README.md). |
| Change CLI parsing or command routing | [main.rs](control/agentctl/src/main.rs), [cli_actions.rs](control/agentctl/src/cli_actions.rs), and [commands/](control/agentctl/src/commands/). |
| Change configuration, trust, or precedence | [config/](control/agentctl/src/config/), [merge.rs](control/agentctl/src/merge.rs), [schemas/](schemas/), and the [security model](docs/migration/30-security-model.md). |
| Change image builds or fleet layout | [images/](control/agentctl/src/images/), [nix/lib/](nix/lib/), and [workload layouts](docs/workloads.md). |
| Change instance identity or teardown | [slots.rs](control/agentctl/src/microsandbox/slots.rs), [runtime/](control/agentctl/src/microsandbox/runtime/), and the [operating model](docs/operating-model.md). |
| Change mount or network policy | [mount_policy/](control/agentctl/src/mount_policy/), [policy/](control/agentctl/src/policy/), and the [security model](docs/migration/30-security-model.md). |
| Change credentials or management authority | [control_plane/](control/agentctl/src/control_plane/), [broker/](control/agentctl/src/microsandbox/broker/), [secrets](docs/secrets.md), and [signing keys](docs/signing-keys.md). |
| Change runtime pins, state, init, or disks | [generation.rs](control/agentctl/src/microsandbox/generation.rs), [runtime provisioning](docs/runtime-provisioning.md), and the selected Microsandbox source. |
| Change compiler or SDK packaging | [flake.nix](flake.nix), [nix/packages/](nix/packages/), Cargo manifests, and [Nix ownership](docs/nix-build.md). |
| Add regression or VM coverage | [Testing](docs/testing.md) and [agent test environment](docs/agent-test-env.md). Application tests live with their fleet/workload owner. |
| Change CI or release gates | [.github/](.github/), [scripts/ci/](scripts/ci/), and [CI foundation](docs/ci-release-foundation.md). |
| Change documentation or artwork | [Docs index](docs/README.md) and [asset notes](docs/assets/README.md). |
| Report a vulnerability | [Private reporting guidance](SECURITY.md). |

## Runtime status

Describe verification by what it exercises: configuration checks resolve
declarations, package/unit checks exercise the tool, image checks inspect guest
artifacts, and VM acceptance observes readiness, behavior, and cleanup. Record
the source, runtime, and image revisions for VM results. The
[testing guide](docs/testing.md) gives the acceptance workflow.

Nested virtualization is available on supported hosts. The current Linux
`virtualization.nested = "off"` setting is not an enforced confinement boundary;
see [runtime provisioning](docs/runtime-provisioning.md) for the exact constraint.
The [workload guide](docs/workloads.md#not-yet-implemented) tracks the remaining
fleet-import and directory-mode scaffolding limitations.
