<a id="workestrate"></a>

<a href="https://github.com/rybskiworks">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/workestrate-dark.svg">
    <img src="docs/assets/workestrate-light.svg" width="1200" alt="workestrate / rybskiworks. Systems that let software act. Declarative policy, a shared control plane, individual microVM workloads.">
  </picture>
</a>

<p align="center">
  <a href="#take-a-look">get started</a> /
  <a href="docs/README.md">documentation</a> /
  <a href="README.agents.md">architecture for agents</a> /
  <a href="https://github.com/rybskiworks/workestrate/issues">open work</a>
</p>

**Give software a place to work, and an explicit boundary to work within.**
Workestrate is a Rust CLI and Nix flake for running declaratively configured
agents and services in Microsandbox microVMs. Workloads, network policy, mounts,
and credential bindings live in configuration, not in a particular agent's prompt.

<a id="positioning"></a>

## a runtime, not an agent

Bring the agent, service, or development workload that suits the job. Workestrate
handles configuration and lifecycle; your fleet owns the workload definitions
and image builds. No particular model, provider, or coding harness is required.

The foundation is deliberately small: **TOML for intent, Nix for reproducible
build inputs, microVMs for execution.** Networking defaults to deny. Secrets are
SOPS-encrypted, with explicit host-bound or guest-bound delivery. The current
runtime uses the pinned Microsandbox fork on x86_64 Linux, without Docker.

<a id="how-it-works-today"></a>

## from intent to execution

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/operating-loop-dark.svg">
  <img src="docs/assets/operating-loop-light.svg" width="1200" alt="Define policy. Bound authority. Delegate work. Validate results. An operating philosophy, not a claim that every runtime property has been verified.">
</picture>

Declare what a workload needs. Inspect the resolved plan. Run it within the
selected policy. Check what actually happened. A configuration setting describes
intent; runtime evidence establishes what was enforced.

<a id="setup"></a>

## take a look

With Git, Nix (flakes enabled), and `just` on x86_64 Linux:

```sh
git clone --branch migration/tool-model https://github.com/rybskiworks/workestrate.git
cd workestrate
just shell

# Inside the pinned development shell:
workestrate --help
WORKESTRATE_CONFIG_DIR="$PWD/config.reference" workestrate validate-config
WORKESTRATE_CONFIG_DIR="$PWD/config.reference" workestrate workload plan example-service
```

This inspects the **synthetic reference configuration**, not your personal fleet,
and does not launch a VM. First shell entry may build the pinned packages.
`migration/tool-model` is the current engineering branch; this guide describes it.
Actual VM execution additionally requires accessible `/dev/kvm` and host
provisioning. Start with the [setup reference](README.agents.md#setup) and
[Nix build guide](docs/nix-build.md).

<a id="config-repos-and-the-layering-model"></a>
<a id="secrets-workflow"></a>

## bring your own fleet

A fleet is your collection of workload definitions and operator configuration.
Keep application choices, image flakes, and encrypted credentials there rather
than baking them into the orchestration tool. Services run detached; agents
attach interactively. Workload names come from your configuration.

The [workload guide](docs/workloads.md) explains standalone repositories and
fleet-local capsules. The [configuration reference](README.agents.md#configuration-model)
and [secrets workflow](README.agents.md#secrets-workflow) cover the next steps.
The bundled example names are placeholders, not ready-to-run applications.

<a id="cli-surface"></a>
<a id="development-workflow"></a>
<a id="canonical-docs"></a>

## find your depth

| Looking for | Start here |
| :--- | :--- |
| Setup, CLI commands, and operating details | [Technical reference](README.agents.md#cli-reference) |
| Architecture, ownership, and where a change belongs | [Agent-oriented README](README.agents.md) |
| Always-applicable working instructions | [AGENTS.md](AGENTS.md) |
| Specifications, decisions, and focused guides | [Documentation index](docs/README.md) |

`README.md` introduces the project to people. `README.agents.md` is an optional,
text-first architectural map, readable by humans too. `AGENTS.md` stays short
and supplies working instructions; it points agents to broader context when
their task needs it. A narrowly scoped worker can go straight to the relevant
code and applicable instructions.

For development, start with `just bootstrap` or `just shell`, and use
`just verify` for the repository gate. See the
[development reference](README.agents.md#development-workflow) for checks,
Beads, hooks, and contribution conventions.

<a id="runtime-status"></a>

## status, without the mythology

Workestrate is under active development. Repository checks validate the CLI,
configuration, and pinned package contracts, **not every property of a deployed
fleet**. Lifecycle readiness and cleanup, mount-policy enforcement, SSH custody,
and nested confinement require their own runtime evidence. In particular, a
nested guest booting does not prove that disabling nesting enforces a boundary.

See [runtime status](README.agents.md#runtime-status), the
[testing guide](docs/testing.md), and
[runtime provisioning](docs/runtime-provisioning.md) before relying on a property.

---

<sub>Part of <a href="https://github.com/rybskiworks">rybskiworks</a>. Explicit intent. Bounded execution. Evidence over assumption. <a href="LICENSE-MIT">MIT</a> / <a href="LICENSE-APACHE">Apache-2.0</a>.</sub>
