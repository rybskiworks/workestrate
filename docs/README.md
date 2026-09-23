# Workestrate documentation

[What is Workestrate?](../README.md) · [Get started](getting-started.md) ·
[CLI guide](cli.md) · [Architecture and source map](../README.agents.md)

## Set up and run workloads

| I want to… | Read |
| :--- | :--- |
| Install Workestrate and run my first workload | [Getting started](getting-started.md), including a [setup checklist](getting-started.md#setup-checklist) for humans and agents. |
| Find a command, flag, or fleet selector | [CLI guide](cli.md). |
| Define workloads and build their images | [Workload layouts](workloads.md). |
| Manage instances, ports, and teardown | [Operating model](operating-model.md). |
| Provision or update credentials | [Secrets](secrets.md) and [signing keys](signing-keys.md). |
| Configure event-driven agents | [Event agents](event-agents.md). |
| Inspect runtime state or provision a host | [Runtime provisioning](runtime-provisioning.md). |

## Understand the system

| Document | Covers |
| :--- | :--- |
| [Architecture and source map](../README.agents.md) | Vocabulary, component ownership, configuration flow, and task-to-code navigation. |
| [Specification map](../SPEC.md) | Maintained system contracts and their owners. |
| [Target specification](migration/20-target-system-spec.md) | Detailed configuration, CLI, seed, and lifecycle semantics. |
| [Security model](migration/30-security-model.md) | Policy layering, trust, and authority boundaries. |
| [Architecture decisions](migration/50-decisions/README.md) | Design rationale, decision status, and supersession. |

## Build, test, and contribute

| Document | Covers |
| :--- | :--- |
| [Contributing](../CONTRIBUTING.md) and [repository instructions](../AGENTS.md) | Branches, review, required checks, and working conventions. |
| [Nix build ownership](nix-build.md) | Toolchain inputs, SDK pairing, packaging, and verification entry points. |
| [Nix purity](nix-purity.md) and [development shells](nix/devshells.md) | Source/build separation and pinned shell behavior. |
| [Testing](testing.md) | Repository checks, properties, regressions, and VM acceptance. |
| [Agent test environment](agent-test-env.md) | KVM, Lix, the pinned CLI, and verification-container setup. |
| [CI and release foundation](ci-release-foundation.md) | Check selection, required gates, and release qualification. |
| [GitHub governance](github-governance.md) | Repository protections and their activation sequence. |
| [Beads contract](../BEADS.md) and [tracker](../.beads/README.md) | Work tracking, acceptance criteria, and dependencies. |
| [Security reporting](../SECURITY.md) | Private vulnerability reporting and disclosure. |
| [Presentation assets](assets/README.md) | README artwork and editing conventions. |

For the dated consolidation findings and follow-up work, see the
[repository audit](repository-audit-2026-09-11.md).
