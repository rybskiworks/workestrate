# Workestrate documentation

[Overview](../README.md) · [Install](install/README.md) ·
[First workload](getting-started.md) · [CLI reference](cli.md)

<a id="set-up-and-run-workloads"></a>

## Start here

[**Choose your installation path**](install/README.md):
[Linux](install/linux.md), [NixOS](install/nixos.md), or the
[Windows detour](install/windows.md).

[**Run your first workload**](getting-started.md): connect a fleet, provision
secrets, inspect a plan, and launch. The
[setup checklist](getting-started.md#setup-checklist) works for humans and agents.

[**Find a command**](cli.md): command families, flags, fleet selection, and
examples for everyday use.

## Work with fleets

- [Define workloads](workloads.md): configuration layouts and image builds.
- [Manage instances](operating-model.md): identity, ports, and teardown.
- [Manage secrets](secrets.md) and [signing keys](signing-keys.md).
- [Configure event agents](event-agents.md).
- [Provision the runtime](runtime-provisioning.md): host setup and state generations.
- [Compose NixOS images](nixos-images.md): shared host/guest packages and init.

## Understand the system

- [Architecture and source map](../README.agents.md): concepts, ownership, and
  task-to-code navigation.
- [Specification map](../SPEC.md): maintained contracts and their owners.
- [Security model](migration/30-security-model.md): policy, trust, and authority.
- [Architecture decisions](migration/50-decisions/README.md): design rationale
  and decision status.

The [target specification](migration/20-target-system-spec.md) covers detailed
configuration, CLI, seed, and lifecycle semantics.

<a id="build-test-and-contribute"></a>

## Contribute

Start with [Contributing](../CONTRIBUTING.md) and the
[repository instructions](../AGENTS.md) for the development workflow and required
checks.

- [Build with Nix](nix-build.md): toolchains, SDK pairing, and packaging.
- [Development shells](nix/devshells.md) and [Nix purity](nix-purity.md): pinned
  environments and source/build separation.
- [Test changes](testing.md): repository checks, regressions, and VM acceptance.
- [Agent test environment](agent-test-env.md): KVM, Lix, and verification containers.
- [Presentation assets](assets/README.md): README artwork and editing conventions.

<details>
<summary>Project maintenance and governance</summary>

- [CI and release foundation](ci-release-foundation.md): gates and release qualification.
- [GitHub governance](github-governance.md): repository protections.
- [Beads contract](../BEADS.md) and [tracker](../.beads/README.md): work tracking and acceptance criteria.
- [Security reporting](../SECURITY.md): private vulnerability disclosure.
- [Repository audit](repository-audit-2026-09-11.md): dated findings and follow-up work.

</details>
