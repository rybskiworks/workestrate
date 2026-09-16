# Documentation map

Start with the [human README](../README.md) for the project and first commands.
Use [README.agents.md](../README.agents.md) for optional architecture, repository
ownership, and the operating reference. [AGENTS.md](../AGENTS.md) contains working
instructions, not an architectural onboarding requirement for every agent.

## Understand the system

| Document | Owns |
| :--- | :--- |
| [Specification map](../SPEC.md) | Entry point to the maintained system contracts and their owners. |
| [Target specification](migration/20-target-system-spec.md) | Detailed configuration, CLI, seed, and lifecycle semantics. |
| [Security model](migration/30-security-model.md) | Policy layering and authority boundaries. |
| [Architecture decisions](migration/50-decisions/README.md) | Rationale, alternatives, and decision history. Check supersession and implementation status. |

## Build and operate

| Document | Owns |
| :--- | :--- |
| [Getting started](getting-started.md) | Initial build, operator setup, and secrets entry points. |
| [Nix build ownership](nix-build.md) | Toolchain inputs, SDK pairing, packaging, and verification entry points. |
| [Nix purity](nix-purity.md) / [devshell rules](nix/devshells.md) | Source/build separation and shell behavior. |
| [Workload layouts](workloads.md) | Standalone config repositories, fleet capsules, image ownership, and current composition limits. |
| [Operating model](operating-model.md) | Instances, addressing, teardown, and operational scope. |
| [Event agents](event-agents.md) | GitHub-event role workloads: implementer, reviewer, and qa instances. |
| [Runtime provisioning](runtime-provisioning.md) | Runtime homes, state generations, build/runtime separation, guest init, and provisioning contracts. |
| [Secrets](secrets.md) | SOPS workflow, credential exposure, and the secrets threat model. |
| [Signing keys](signing-keys.md) | Explicit encrypted Ed25519 provisioning and public-key export without granting workload authority. |

## Change and validate

| Document | Owns |
| :--- | :--- |
| [Contributing](../CONTRIBUTING.md) | Integration, review, tests, and repository hygiene. |
| [Security reporting](../SECURITY.md) | Vulnerability reporting and disclosure guidance. |
| [Testing](testing.md) | Repository checks, properties, regressions, and separate VM acceptance. |
| [CI and release foundation](ci-release-foundation.md) | Check selection, required gates, and release boundaries. |
| [GitHub governance](github-governance.md) | Proposed protections and their activation sequence. |
| [Repository audit](repository-audit-2026-09-11.md) | Dated consolidation findings, fixes, and remaining work. |
| [Beads contract](../BEADS.md) / [tracker](../.beads/README.md) | Work tracking, acceptance criteria, and dependencies. |
| [Presentation assets](assets/README.md) | Light/dark README artwork and editing constraints. |

## Keep the layers distinct

The root human README explains what the project is and how to approach it.
The agent README provides a selective architectural map and technical reference.
Detailed procedures, contracts, and rationale stay with the focused guides above.
Working instructions stay in the applicable `AGENTS.md` files.

When changing a contract, update its owning document and the relevant entry-point
summary or link. Prefer links to duplicate specifications. A proposal, an accepted
decision, an implemented interface, and a measured runtime property are different
kinds of evidence; name which one a document supplies.
