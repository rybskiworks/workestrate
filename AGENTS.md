# AGENTS.md — workestrate orientation

## Gist

Workestrate is an agent-first microVM workload orchestrator built on the
microsandbox (msb) fork. It is config-driven (workloads are declared in TOML
config repos under the dev home) and nix-native (builds, devshells, and
tool pins all flow through the flake).

## Layout

- `control/agentctl` — the Rust CLI behind the `workestrate` binary (plan/up/exec/down/versions/msb).
- `nix/` — flake packages (`nix/packages/`) plus shared lib and recipes (`nix/lib/`).
- `docs/` — guides and the migration tree; `docs/migration/50-decisions/` holds the ADRs.
- `scripts/` — host provisioning, secrets workflow, git hooks, store/gc utilities.
- `templates/` — scaffold templates for new config repos.
- Config repos live in the dev home: `../workestrate-dev-home/config-repos/*`.

## Workflow

- Run `just <recipe>` from a plain host shell. Repository verification builds
  pinned Nix checks directly; interactive Cargo recipes enter the development shell.
- `just shell` opens the interactive devshell (extra args pass through, e.g. `just shell -c <cmd>`).
- `just bootstrap` supplies pinned tools without building the application or
  runtime. Neither shell installs hooks, builds agent workloads or migrates state.
- `just verify` is the gate before pushing.
- Plain `cargo` works inside recipes and the devshell.
- Nix builds see tracked files only — `git add` new files before building.
- Purity rules live in `docs/nix-purity.md`.

## Canonical docs

- `SPEC.md` — the normative system specification.
- `README.md` — setup, secrets workflow, CLI tour, commit conventions.
- `docs/runtime-provisioning.md` — msb runtime provisioning, state layout, pin wiring.
- `docs/nix-build.md` — build ownership, immutable SDK inputs, bootstrap and verification.
- `docs/migration/50-decisions/README.md` — ADR index (0035 hierarchical policy, 0036 nested-virt, 0037 msb state generations).
- `docs/migration/20-target-system-spec.md` — target system spec: CLI surface, seeds, lifecycle.
- `docs/migration/30-security-model.md` — the policy ladder and secrets model.
- `docs/nix/devshells.md` — devshell rules and patterns.
- `docs/nix-purity.md` — nix purity rules for this repo.

## Read first before assuming

- `up` starts services (detached, topo-ordered); `exec` runs agents (interactive TUI) — CLI table in `docs/migration/20-target-system-spec.md`.
- msb semantics are inspired by Docker and intentionally different — read `docs/runtime-provisioning.md` before assuming anything.
- msb state is generation-keyed under `~/.microsandbox/current` — `docs/runtime-provisioning.md` + ADR 0037.
- Policy is a ladder: `allow` stands alone, home `final` seals veto, and `entitlements` is a retired key (hard parse error) — ADR 0035's 2026-09-04 note + `docs/migration/30-security-model.md`.
- Nested virtualization: `virtualization.nested` requests nesting on supported
  hosts; the bundled runtime/firmware can already run nested guests. The current
  Linux off flag is not an enforced confinement boundary. Keep capability,
  operator policy and observed enforcement separate — ADR 0036 and
  `docs/runtime-provisioning.md`.
- `MSB_BUILD_RUNTIME` selects immutable SDK compilation inputs; `MSB_HOME`
  selects mutable runtime state. Building or entering a shell does not migrate
  an existing home or complete SSH broker setup.
- `workestrate msb -- <args>` reaches the exact pinned msb; `workestrate versions` prints the pin quadruple — ADR 0036 + `docs/runtime-provisioning.md`.
- Seeds re-render when `only_if_missing = false`; `--reseed` forces template re-render — `docs/migration/20-target-system-spec.md`.
- Commits use conventional prefixes; ADR numbers belong in bodies/footers while subjects stay clean; secrets stay encrypted (`.env.enc`) — `README.md`.

## Operational state

Deployment-specific instructions belong to the selected operator and fleet
repositories. Inspect their effective configuration before runtime changes;
`--home` alone does not relocate all state or override Microsandbox backend
configuration. Build verification does not authorize a live migration.
