# Workestrate specification map

This is the entrypoint to the maintained system contract, not a live deployment
report. Detailed semantics belong in the documents below, with amendments in the
ADR corpus. Implementation changes must update the affected contract and tests.

The previous monolithic specification mixed old `ai-workbench` naming, personal
agent deployment instructions, contradictory validation snapshots and obsolete
nested-virtualization claims. It is preserved in
[Git history at the consolidation baseline](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/SPEC.md),
not presented as current security guidance. This refactor does not change runtime
behavior or create new enforcement guarantees.

## Ownership boundaries

**Workestrate is the tool and control plane.** Its Rust implementation is in
`control/agentctl/`; Nix packaging and image recipes live under `nix/`. A fleet declares workloads and operator policy. A personal provider
list, agent model catalog or host path is not a universal tool requirement.

**nix-tooling owns shared build inputs.** Workestrate pins one immutable supplier
revision and follows its package set, Fenix compiler and development modules.
The runtime fork shares that authority. Workload definitions do not gain the
authority to silently replace the tool's compiler or runtime.

**Microsandbox owns backend execution.** Workestrate integrates the pinned backend
and SDK contract; it does not claim that all future backends implement identical
capabilities. Backend generation, guest initialization, readiness and observed
enforcement must be treated explicitly.

**Operator state is not repository source.** Homes, runtime state, caches, live
secrets, guest disks and local handoffs must not become tracked application
artifacts. Intentional schemas, fixtures, ADRs and task exports remain source.
Changing a build pin or entering a shell is not permission to migrate live state.

## Contract index

| Area | Maintained source |
| :--- | :--- |
| Configuration, provenance, lifecycle and teardown | [Operating model](docs/operating-model.md) and [detailed system design](docs/migration/20-target-system-spec.md). |
| Network policy, trust, seals and secret bindings | [Security model](docs/migration/30-security-model.md) and its cited ADR amendments. |
| Backend generations, init and nested virtualization | [Runtime provisioning](docs/runtime-provisioning.md). |
| Immutable inputs, packaging and bootstrap | [Nix build ownership](docs/nix-build.md) and [purity rules](docs/nix-purity.md). |
| Architecture and compatibility transitions | [ADR index](docs/migration/50-decisions/README.md). |
| CI guarantees and release qualification | [CI contract](docs/ci-release-foundation.md). |
| Task state and contributor operation | [Beads](BEADS.md) and [contributing](CONTRIBUTING.md). |

## Security and evidence rules

A policy request, a backend capability and observed enforcement are different
facts. In particular, the current Linux nested-virtualization `off` flag is not
an independently enforced confinement boundary. Consult runtime provisioning and
ADR 0036 before using it in a security argument.

Planning and schema validation do not prove a microVM booted. Compiling tests does
not prove they executed. Passing ordinary Rust/Nix checks does not imply ignored,
optional native or KVM tests passed. Record the source revision, runtime pin,
host and exact tier with any runtime claim.

Default-deny networking does not make broad mounts, operator escape hatches or
unrestricted credentials harmless. Deployment-specific secret names and host
key locations belong in operator documentation, not a fixed global secret list.

`main` is the integration branch following PR #32. Historical branch names and
verification dates inside ADRs remain historical evidence, not current routing
instructions. New behavior belongs in reviewed code, tests and contract updates,
not a root-level session note.
