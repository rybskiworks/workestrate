<a id="workestrate"></a>

<a href="https://github.com/rybskiworks">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/workestrate-dark.svg">
    <img src="docs/assets/workestrate-light.svg" width="1200" alt="Workestrate: declarative configuration, a shared control plane, and individual microVMs for agents and services.">
  </picture>
</a>

<p align="center">
  <samp>
    <a href="docs/install/README.md">install</a> &nbsp;·&nbsp;
    <a href="docs/getting-started.md">your first fleet</a> &nbsp;·&nbsp;
    <a href="docs/cli.md">commands</a> &nbsp;·&nbsp;
    <a href="docs/README.md">docs</a>
  </samp>
</p>

Workestrate runs **agents and the services they depend on** in individual
microVMs. Define the tools, files, network access, and credentials each workload
needs in a fleet you keep in Git. Inspect the plan, start the services, and attach
to an agent from one CLI.

<a id="positioning"></a>
<a id="one-control-plane-explicit-boundaries"></a>

TOML describes your workloads and how they fit together. Nix pins the toolchain
and supplies image-building recipes. Microsandbox runs the VMs.

<a id="how-it-works-today"></a>
<a id="define-plan-run"></a>
<a id="a-small-cli-surface-for-a-larger-system"></a>

## A fleet, in a few commands

<a href="docs/getting-started.md">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/operating-loop-dark.svg">
    <img src="docs/assets/operating-loop-light.svg" width="1200" alt="Setup workflow: connect a fleet, inspect its workloads, prepare credentials and host, then run agents and services. Open the setup guide.">
  </picture>
</a>

With a configured fleet called `dev` and an agent called `coder`, start its
services and open the agent:

```sh
workestrate --fleet dev workload up
workestrate --fleet dev workload exec coder
```

Services start in dependency order. Agents can also bring up their declared
dependencies automatically.

<details>
<summary><samp>inspect it. find it. stop it.</samp></summary>

See the resolved environment and the source of each setting:

```sh
workestrate --fleet dev workload plan coder --show-source
```

Inspect instances, then stop the fleet when you're finished:

```sh
workestrate instances
workestrate down --fleet dev
```

Fleet teardown asks for confirmation. The [command guide](docs/cli.md) covers
logs, parallel instances, image builds, and the rest of the interface.

</details>

<a id="setup"></a>
<a id="start-here"></a>
<a id="take-a-look"></a>
<a id="install-the-tool"></a>

## Get started

Workestrate currently targets **x86_64 Linux with KVM**. Choose your installation
path for the CLI and host setup:

**[Linux + Nix](docs/install/linux.md)** &nbsp;·&nbsp;
**[NixOS](docs/install/nixos.md)** &nbsp;·&nbsp;
**[Windows](docs/install/windows.md)**

<a id="provision-config-fleet"></a>
<a id="fleets-and-the-layering-model"></a>
<a id="secrets-workflow"></a>

Installed? **[Set up your first fleet →](docs/getting-started.md)**
The walkthrough covers configuration, secrets, planning, and launch. For a look
around first, [preview the bundled configuration](docs/getting-started.md#preview-the-reference-configuration).

<a id="tear-down-safely"></a>
<a id="one-toolchain-authority"></a>
<a id="cli-surface"></a>
<a id="development-workflow"></a>
<a id="canonical-docs"></a>
<a id="build-with-us"></a>

[Write a workload](docs/workloads.md) · [Explore the architecture](README.agents.md) ·
[Contribute](CONTRIBUTING.md)

<a id="runtime-status"></a>
<a id="security-is-a-contract-not-a-badge"></a>

Actively developed. See [runtime status](README.agents.md#runtime-status) and
[VM acceptance](docs/testing.md#vm-acceptance) when planning a deployment.

---

<sub>Part of <a href="https://github.com/rybskiworks">rybskiworks</a>. Original material: <a href="LICENSE">Apache-2.0</a>. Copyright (c) 2026 Georg Rybski. <a href="LICENSING.md">License scope</a> · <a href="THIRD-PARTY.md">Third-party notices</a> · <a href="SECURITY.md">Security</a>.</sub>
