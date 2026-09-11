<h1 align="center">workestrate</h1>
<p align="center"><strong>Define the workload. Bound its authority. Let it work.</strong></p>
<p align="center">Declarative microVM workloads, policy-aware configuration, and a Nix-pinned control plane.</p>
<p align="center">
  <a href="https://github.com/rybskiworks/workestrate/actions/workflows/ci.yml"><img alt="CI on main" src="https://github.com/rybskiworks/workestrate/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  &nbsp; Linux x86_64 &nbsp; | &nbsp; Rust + Nix &nbsp; | &nbsp;
  <a href="LICENSE-MIT">MIT</a> / <a href="LICENSE-APACHE">Apache-2.0</a>
</p>
<p align="center">
  <a href="#start-here">Get started</a> &middot;
  <a href="SPEC.md">Architecture</a> &middot;
  <a href="docs/runtime-provisioning.md">Runtime</a> &middot;
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

---

**Your agent is a workload, not your identity.** Workestrate combines a Rust CLI
with a Nix flake to run agents and services inside Microsandbox microVMs.
Configuration, network access, mounts and secret bindings are explicit inputs,
not privileges inherited simply because a program runs on your machine.

Agent-first, not agent-specific. Workloads live in fleet/config repositories;
the tool does not hardcode your preferred model, coding harness or service stack.

## One control plane, explicit boundaries

```text
fleet / config repositories       pinned nix-tooling
  workloads + policy + sources      compiler + build tools
                 |                         |
                 +------ workestrate ------+
                         plan / validate
                         lifecycle / exec
                               |
                      pinned Microsandbox
                       /               \
                  agent microVM    service microVM
```

| Concern | Workestrate's role |
| :--- | :--- |
| Configuration | Compose declared layers, expose provenance, apply security-aware policy merges. |
| Isolation | Launch workloads through the pinned Microsandbox backend on Linux/KVM. |
| Credentials | Resolve SOPS-encrypted inputs and explicit workload secret bindings. |
| Reproducibility | Share reviewed Nix/toolchain inputs; keep runtime source and SDK patches aligned. |
| Lifecycle | Plan, start, attach, inspect and stop workloads without turning the repository into operator state. |

**Active development.** `main` is the integration branch. The former
`migration/tool-model` line was integrated in [PR #32](https://github.com/rybskiworks/workestrate/pull/32).
Development history is not a second release channel.

## Start here

Build and inspect the CLI with **Nix and flakes enabled**:

```sh
git clone https://github.com/rybskiworks/workestrate.git
cd workestrate
nix build --no-update-lock-file .#workestrate
./result/bin/workestrate --help
./result/bin/workestrate versions
```

Actually running a microVM additionally requires a Linux x86_64 host with
virtualization enabled and `/dev/kvm` accessible. No Docker daemon is involved.
Building does not provision a live operator home, migrate backend state or grant
access to credentials.

Initialize a home and inspect the bundled synthetic reference configuration:

```sh
./result/bin/workestrate home init
./result/bin/workestrate validate-config
./result/bin/workestrate workload plan example-service
```

A fresh setup ships examples, not your live agents. Register or scaffold your own
config repository before running real workloads:

```sh
workestrate config new personal
# Alternative: workestrate config add <repository-url> personal
```

Use `./result/bin/workestrate` until the built CLI is on your `PATH`. Configure
workload definitions, trust and SOPS keys deliberately; follow the
[setup and secrets guide](docs/getting-started.md) and
[runtime provisioning guide](docs/runtime-provisioning.md), not a copied operator
home or a stale session handoff.

## A small CLI surface for a larger system

```sh
workestrate workload plan <name> --show-source
workestrate workload up <service>
workestrate workload logs <service>
workestrate workload exec <agent>
workestrate workload down <name>
workestrate workloads
workestrate ps --json
workestrate doctor
```

Services run detached; agents can be attached interactively. Instance replacement,
parallel instances and scoped teardown are explicit operations. The complete
lifecycle contract is in the [operating model](docs/operating-model.md).

## One toolchain authority

[`flake.nix`](flake.nix) selects an immutable revision of
[`rybskiworks/nix-tooling`](https://github.com/rybskiworks/nix-tooling).
The package set, Fenix compiler and development modules follow that supplier.
Microsandbox follows the same tooling input rather than bringing a competing pin.

CI resolves its exact Rust release from the **locked Fenix manifest**, not a
separate `stable` channel or hand-maintained compiler version in YAML. Inspect
and validate the relationship without building the application:

```sh
python3 scripts/ci/toolchain.py check --role consumer
```

Updating the supplier is a reviewed `flake.nix` + `flake.lock` change. A local
supplier override is an experiment, not a distributable lock update. See
[build ownership](docs/nix-build.md) and [the CI contract](docs/ci-release-foundation.md).

## Security is a contract, not a badge

Network defaults deny access unless allowed by effective policy. This does not
make arbitrary configuration, mounts, operator escape hatches or host privileges
safe. The current Linux nested-virtualization `off` flag is **not** an independently
enforced confinement boundary.

Read the [security model](docs/migration/30-security-model.md),
[runtime limitations](docs/runtime-provisioning.md) and
[reporting guidance](SECURITY.md). Hosted CI, native integration checks and
KVM/runtime tests prove different things; a green aggregate does not imply all ran.

## Build with us

With Nix and `just` available, `just bootstrap` provides pinned development tools,
`just shell` opens the interactive environment, and `just verify` is the local
verification entrypoint. Shell entry is not permission to initialize trackers,
replace hooks, migrate homes or start workloads.

| Read | For |
| :--- | :--- |
| [Specification map](SPEC.md) | Component boundaries and authoritative design records. |
| [Runtime provisioning](docs/runtime-provisioning.md) | Backend generations, initialization, readiness and nested virtualization. |
| [Nix build guide](docs/nix-build.md) | Immutable SDK inputs, bootstrap and verification. |
| [Decision records](docs/migration/50-decisions/README.md) | Rationale, compatibility and historical context. |
| [Contributing](CONTRIBUTING.md) | Review, tests and repository hygiene. |
| [GitHub governance](docs/github-governance.md) | Proposed protections and their activation sequence. |
| [Repository audit](docs/repository-audit-2026-09-11.md) | Findings, fixes, evidence and remaining work. |
| [Beads](BEADS.md) | The intentional task-tracking contract. |

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
