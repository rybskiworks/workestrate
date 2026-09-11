# ADR 0017: Synthetic Reference Config and Final Strip-Down

## Status

Accepted

## Context

The workestrator migration introduced a tool+XDG model where a user's real
deployment lives in their personal config repo (`~/.local/share/workestrate/repos/personal/`),
not in the tool repository. During the migration we created `config.reference/`
as a tracked, sanitized copy of the user's deployment. It served two roles:

1. **Golden-test fixture + fresh-install fallback**: gives `workestrate plan` and
   `workestrate validate-config` something deterministic to run against on a
   fresh clone before any config repo is registered.
2. **Flake build driver**: `flake.nix` discovered `workload-images` from
   `config.reference/workestrate.toml`, and the devshell `_build_agents` read
   `local_build` recipes from it.

These two roles created tension. Role (ii) required `config.reference/` to
contain real deployment definitions (pi, odysseus, litellm, opencode, tempest)
so that the nix expressions could discover image names and build commands. Role
(i) only needs a minimal, machine-independent fixture that exercises the
vocabulary. Meanwhile, the root repository still contained the user's real
`.env.enc`, `.sops.yaml`, `workestrate.toml`, `infra/litellm/`, and
`agents/*/config/` files — a triplication with the personal config repo and
`config.reference/`.

The goal of this decision is to collapse the triplication:
- tool = engine + machinery + synthetic reference fixture
- user's real definitions live only in their personal config repo

## Options considered

1. **Keep config.reference as sanitized clone of user deployment; keep root files.**
   - Pro: zero change to existing flake/devshell logic.
   - Con: perpetuates triplication; root `.env.enc`/`.sops.yaml` remain in the
     tool repo; config.reference must stay in sync with the personal repo.

2. **Make config.reference synthetic; keep flake config-driven from it.**
   - Pro: config.reference becomes a true fixture.
   - Con: workload-images and devshell builds would be driven by synthetic
     workloads (example-service, example-agent, example-offensive), which are
     not the images we actually want to ship (`workestrator-pi`, `tempest`).

3. **Make config.reference synthetic; make flake outputs explicit.**
   - Pro: config.reference is a lightweight, machine-independent fixture; real
     flake outputs (`.#pi`, `.#pi-bun`, `.#tempest-built`, etc.) are explicit and
     unchanged; workload-images is explicit and unchanged.
   - Con: slightly more boilerplate in `flake.nix` (reverts Phase 0b
     config-driven discovery).

4. **Delete config.reference entirely; rely on project-layer or registry only.**
   - Pro: no fixture at all.
   - Con: fresh clone has no fallback config; `workestrate plan` fails before
     `workestrate init`/`config add`; violates fail-closed convenience for new
     users and breaks golden tests.

## Decision

We choose option 3:

(a) `config.reference/` becomes **synthetic** — its only role is golden-test
    fixtures and fresh-install fallback. It contains three workloads
    (`example-service`, `example-agent`, `example-offensive`) that exercise every
    recipe type, egress recipe, secret kind, and policy feature without using any
    `${CWD}` mounts.

(b) The flake keeps **explicit outputs** for the real deployment artifacts:
    `.#pi`, `.#pi-bun`, `.#tempest-built`, `.#tempest-image`, `.#odysseus-built`,
    `.#opencode-built`, `.#workestrate`. Their derivation paths must remain
    unchanged.

(c) `workload-images` in `flake.nix` is **explicit** (not config-driven). It
    reverts the Phase 0b config-driven discovery back to an explicit attrset
    because `config.reference` is now synthetic and does not describe the real
    images.

(d) The devshell `_build_agents` continues to read `config.reference/` (now
    synthetic), so the default devshell is lighter. Real agent builds are
    explicit user commands (`just dev-build-pi`, `workestrate source build`, or
    the nix derivations `.#pi`, `.#odysseus-built`, `.#opencode-built`).

(e) The user's real deployment builds happen in the user's own config context
    (personal config repo). Root `workestrate.toml`, `.env.enc`, `.sops.yaml`,
    root `infra/litellm/`, and root `agents/*/config/` are removed from the
    tool repo.

## Consequences

- `config.reference/workestrate.toml` is small, machine-independent, and stable.
- Golden tests are regenerated from the synthetic workloads and remain stable
  across machines.
- Fresh clones can run `workestrate plan` and `workestrate validate-config`
  immediately using the synthetic reference config.
- Real workloads (`litellm`, `pi`, `odysseus`, `opencode`, `tempest`) are no
  longer present in the tool repo. The typed clap subcommands for these names
  still exist, but they require a config repo that defines those workloads.
- `flake.nix` workload-images and devshell `_build_agents` no longer depend on
  the user's deployment shape, decoupling the tool from any personal config.
- The `.env.enc` and `.sops.yaml` files remain in git history; new clones must
  not rely on them being absent from history (see `docs/migration/70-open-items.md`).

## Rejected options

- Option 1 was rejected because it preserves triplication and keeps user secrets
  in the tool repo.
- Option 2 was rejected because it would make flake outputs depend on synthetic
  example workloads.
- Option 4 was rejected because it removes the fresh-install fallback and the
  golden-test fixture.
