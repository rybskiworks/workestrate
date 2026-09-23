# Working in Workestrate

## Documentation contract

- [README.md](README.md) is the human-facing introduction and quick start.
- [README.agents.md](README.agents.md) is optional project context: architecture,
  ownership, terminology, operating reference, and task-to-source navigation.
  Consult the relevant sections for cross-cutting work or unfamiliar boundaries.
  Focused workers can use the relevant code and applicable instructions directly.
- Write documentation around the reader’s task and the current interface. Explain
  concepts directly, keep each procedure in one owning guide, and link to it from
  introductions. Give alternative workflows their own steps and place operational
  constraints beside the action they affect. Keep change history in commits and ADRs.
- This file holds repository-wide working instructions. Apply any additional
  instructions scoped to the files you change. [docs/README.md](docs/README.md)
  indexes focused guides; [SPEC.md](SPEC.md) maps the maintained system contracts.

## Workflow

- Target `main` for integration and follow [CONTRIBUTING.md](CONTRIBUTING.md).
- Use `just <recipe>` from a plain host shell. `just shell` provides the full
  development environment; `just bootstrap` provides pinned tools without
  building the application/runtime. Shell entry does not provision workloads.
- Stage new source files before Nix evaluation. Keep Cargo outputs outside the
  checkout, following [Nix purity](docs/nix-purity.md).
- Run `just verify` before pushing/review. Report checks actually run and any
  unavailable gates explicitly; keep VM acceptance separate from build checks.
- Verification containers provide KVM, Lix, and the pinned Microsandbox CLI;
  check availability with `workestrate check`/`doctor` before skipping a VM
  gate. See [agent test environment](docs/agent-test-env.md).
- Install hooks explicitly with `nix run .#install-hooks`; preserve the hook chain
  and fix findings with hooks enabled. Keep heavy `nix-ci` runs maintainer-triggered.
- Use conventional commit prefixes and put ADR identifiers in bodies/footers.
  Coordinate Beads mutations as a single writer; keep remote sync explicit.

## Authority and live state

- Keep secret material encrypted in repositories. Keep age private keys on the
  host, outside repositories and the config. Follow [secrets.md](docs/secrets.md).
- Inspect the selected operator/fleet configuration before runtime changes.
  `--config` alone does not isolate all state or override backend configuration.
  Use explicitly disposable state and synthetic credentials for runtime tests.
- Keep live provisioning, migration, and teardown explicitly authorized and
  separate from build verification. Read [runtime provisioning](docs/runtime-provisioning.md)
  before changing runtime homes, pins, guest init, or broker integration.
- Distinguish configured capability, operator policy, and observed enforcement.
  Validate both allowed and refused behavior before claiming a security boundary.
