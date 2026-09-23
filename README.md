<a id="workestrate"></a>

<a href="https://github.com/rybskiworks">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/workestrate-dark.svg">
    <img src="docs/assets/workestrate-light.svg" width="1200" alt="Workestrate: declarative configuration, a shared control plane, and individual microVMs for agents and services.">
  </picture>
</a>

<p align="center">
  <a href="docs/getting-started.md">get started</a> /
  <a href="docs/cli.md">command reference</a> /
  <a href="docs/README.md">documentation</a> /
  <a href="README.agents.md">architecture</a>
</p>

**Give your agents a place to work.**

Workestrate runs agents and services in microVMs, with their environments defined
in Git. Declare the image, files, network access, and credentials each workload
needs; inspect the plan, then launch it from one CLI.

Build a fleet around the way you work: a coding agent alongside its development
services, a shared model gateway, or a collection of task-specific environments.
Your fleet brings the applications. Workestrate manages their configuration and
lifecycle.

<a id="positioning"></a>
<a id="one-control-plane-explicit-boundaries"></a>

## An environment you can read, review, and repeat

- **Describe the whole workspace.** TOML brings workloads, dependencies, mounts,
  and access policy together in a versioned fleet.
- **See what will run.** Plans show the resolved configuration and where its
  settings came from.
- **Give each workload its own microVM.** Agents attach interactively; services
  run in the background through Microsandbox.
- **Make access deliberate.** Declare network destinations and credential
  bindings, and keep stored secrets encrypted with SOPS.
- **Build on reproducible inputs.** Nix pins the CLI and runtime toolchain and
  provides recipes for fleet-owned images.

<a id="setup"></a>
<a id="start-here"></a>
<a id="take-a-look"></a>
<a id="install-the-tool"></a>

## Get started

Install on **x86_64 Linux** with Git and Nix, with flakes enabled. Running
workloads also requires access to `/dev/kvm`.

```sh
nix profile install github:rybskiworks/workestrate
workestrate --version
workestrate --help
```

The [setup guide](docs/getting-started.md) walks through connecting a fleet,
preparing credentials, checking the host, and launching your first workload.
It includes checkpoints for agents carrying out the setup. To explore the CLI
with the bundled examples, start with the
[preview](docs/getting-started.md#preview-the-reference-configuration).

<a id="how-it-works-today"></a>
<a id="provision-config-fleet"></a>
<a id="define-plan-run"></a>
<a id="fleets-and-the-layering-model"></a>
<a id="secrets-workflow"></a>
<a id="a-small-cli-surface-for-a-larger-system"></a>

<a href="docs/getting-started.md">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/operating-loop-dark.svg">
    <img src="docs/assets/operating-loop-light.svg" width="1200" alt="Setup workflow: connect a fleet, inspect its workloads, prepare credentials and host, then run agents and services. Open the setup guide.">
  </picture>
</a>

**Connect → inspect → prepare → run.** Follow the
[walkthrough](docs/getting-started.md), or use the
[command reference](docs/cli.md) for configuration, workload lifecycle, secrets,
and diagnostics.

<a id="tear-down-safely"></a>
<a id="one-toolchain-authority"></a>
<a id="cli-surface"></a>
<a id="development-workflow"></a>
<a id="canonical-docs"></a>
<a id="build-with-us"></a>

## Explore further

| You want to… | Start here |
| :--- | :--- |
| Set up a machine and run a workload | [Getting started](docs/getting-started.md) |
| Choose commands and understand their options | [Command reference](docs/cli.md) |
| Build your own fleet or workload | [Workload guide](docs/workloads.md) |
| Understand the architecture or find the code for a task | [Architecture and task map](README.agents.md) |
| Contribute a change | [Contributing](CONTRIBUTING.md) · [Working instructions](AGENTS.md) |
| Find a specification or focused guide | [Documentation index](docs/README.md) |

<a id="runtime-status"></a>
<a id="security-is-a-contract-not-a-badge"></a>

Workestrate is under active development. For deployment decisions, see the
[current runtime status](README.agents.md#runtime-status) and
[VM acceptance guidance](docs/testing.md#vm-acceptance). Report vulnerabilities
through the [security guidance](SECURITY.md).

---

<sub>Part of <a href="https://github.com/rybskiworks">rybskiworks</a>. Explicit intent. Bounded execution. Evidence over assumption. Original material: <a href="LICENSE">Apache-2.0</a>. Copyright (c) 2026 Georg Rybski. <a href="LICENSING.md">License scope and historical grants</a>; <a href="THIRD-PARTY.md">third-party exceptions</a>.</sub>
