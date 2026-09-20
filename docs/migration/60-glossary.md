# 60 — Glossary

Canonical vocabulary for the workestrate tool+XDG migration. One-paragraph
definitions.

**Canonical (source)**
The default agent source code, provided by a flake input (e.g.
`github:georgrybski/pi` at `flake.nix:7-10`). Materialized by the devshell or
by `workestrate source clone`. Contrasted with a **source override** (a
user's local checkout that overrides the canonical source).

**Source override**
A user-cloned agent source checkout at a path different from the canonical
flake input, selected via `WORKESTRATE_<NAME>_BUILD` env var (existing
convention, `workload.rs:80-90`). Managed by `workestrate source
clone/build/list/reset`. Stored at `$WORKESTRATE_CONFIG/sources/<name>/`.

**Recipe**
A named, versioned, reviewable unit of executable logic in core that config
references by name. Four categories: **egress recipes** (`dns`,
`litellm_proxy`, `github`, `agent_base`, `https`), **build recipes**
(`npm-build`, `bun-compile`, `pip-install`, `bun-install`), **image recipes**
(`registry`, `nix-layered`), and **features** (`create_tmp`). Recipes are a
closed vocabulary — new recipes require core code review (ADR 0003).

**Closed vocabulary**
A bounded set of named items (recipes, packages, features) defined in core.
Config can reference items by name but cannot define new ones. This is the
config purity mechanism: config is data; all logic is named recipes in core.
Precedents: Kustomize (no-templating by design), NixOS modules (typed
options), devcontainer features (install scripts owned by feature).

**Layer**
A config source that participates in the merge order. Layers are ordered in
the registry's `layers = [...]` array. Earlier layers are overridden by later
ones (for non-security fields; security fields use security-aware merge, ADR
0005). Layer sources: `config.reference/` (base), fleets (registry
layers), trusted project (`./workestrate.toml`), local
(`./workestrate.local.toml`).

**Context (deferred)**
A named layer-set (kubectl-context style) that selects which fleets to
layer and in what order. **Deferred** until 3+ layers exist (ADR 0013). Phase
3 ships a single ordered `layers` array; named contexts (`[[contexts.<name>]]`)
are a future enhancement.

**Registry**
The user-level config file at `$WORKESTRATE_CONFIG/config.toml` that holds:
tool settings, fleet registry (name→url→ref→rev), ordered `layers` list,
and `[trusted_projects]`. Tracked in the user's dotfiles repo. This is the
registry for fleet references — where the homeless-registry recursion
terminates. Modeled on kubeconfig (`~/.kube/config` holds cluster references).

**Config repo**
A git repository containing deployment-specific configuration: `workestrate.toml`
(workload definitions + secrets schema), `.env.enc` (SOPS-encrypted secrets),
`.sops.yaml` (SOPS config), `infra/litellm/` (LiteLLM values), `agents/*/config/`
(agent config files). Cloned to `$WORKESTRATE_CONFIG/fleets/<name>/` by
`workestrate config add`. May optionally have its own `flake.nix` (inverted
dependency, Phase 2).

**Home (WORKESTRATE_CONFIG)**
The single config directory for workestrate. Default `~/.workestrate`.
Contains `config.toml` (registry),
`overrides.toml`, `secrets/`, `fleets/`, `sources/`, `state/`, `cache/`.
Resolution: `WORKESTRATE_CONFIG` env → legacy XDG (compat) → default
`~/.workestrate` (the trusted-ancestor auto-discovery tier was removed; see
spec 08 + ADR 0023 addendum). Precedents: `~/.kube`, `~/.docker`, `~/.cargo`.
See ADR 0023.

**Bundle (repo-local config)**
A `$WORKESTRATE_CONFIG` placed inside a repository (typically
`<repo>/.workestrate/`) for container persistence. Bind-mountable across
container restarts. The `.envrc`/`local-xdg.sh` machinery that set
`WORKESTRATE_CONFIG` to point at it is REMOVED (2026-09-04 note: `local-xdg.sh`
was removed in `418530a`, `.envrc` deleted in `7805648`); the bundle remains as
a layout concept. See ADR 0023.

**migrate-config**
`workestrate migrate-config` — migrates a legacy XDG three-directory layout
(config/data/state split) into the single config. Idempotent; warns on
conflicts. Stamps `config_version = 2`, clears `store_dir`/`state_dir`, and
rewrites `fleets.<name>.url` fields pointing into the old layout (remote
URLs are left untouched). Emits a deprecation note when legacy XDG is
detected.

**Reference config**
A sanitized, tracked copy of the config shipped with the tool at
`config.reference/workestrate.toml`. Contains placeholder secrets (rejected at
runtime by `reject_if_placeholder`, `runtime.rs:10-22`). Used as the fallback
when no fleets are registered (fresh install, CI). `plan`/`check`/
`validate-config` work with reference config; `up`/`exec` refuse (placeholder
secrets).

**Trusted project**
A project directory explicitly approved for project-layer config loading.
Listed in `[[trusted_projects]]` in the registry. `workestrate config trust
<dir>` adds a project. Untrusted projects' `./workestrate.toml` is silently
ignored. This is the `direnv allow` model (ADR 0014).

**Purity invariant**
The rule that fleets contain only declarative data + SOPS-encrypted
secrets + static files. No executable logic (scripts, nix expressions,
`extraCommands`). All logic is named recipes in core. Enforced by the config
loader (deserializes TOML into typed structs; no `eval`/`exec` path). This is
the security boundary for a sandboxing tool (ADR 0003).

**Policy ceiling**
The core-defined allowlist (`policy.rs`) that config cannot exceed. Comprises
`ALLOWED_PACKAGES` (the former `DEFAULT_DENY_FALSE_ENTITLEMENT` gate was
removed 2026-09-04 together with the entitlements mechanism; the
`ALLOWED_EGRESS_HOSTS` and `SECRET_HOST_BINDINGS` ceilings were retired
earlier — the latter in the pre-0035 secret-binding cleanup, the former by
the ADR 0035 hierarchical policy engine). Enforced at `validate-config` (pre-flight),
`plan` (fail-closed), and `apply_plan_secrets` (runtime). Config source
provenance is irrelevant — the ceiling is checked against contents, not
origin (ADR 0004).

**Provenance**
The attribution of each config field to the layer that set it. Displayed by
`workestrate plan --show-source`. Labels: `[core]` (tool defaults),
`[reference]` (config.reference/), `[<layer-name>]` (registry layer),
`[project]` (trusted project), `[local]` (local overrides). Helps users
understand "why does pi have egress to X?" across multiple layers.

**Golden parity**
The test strategy that verifies the data-driven config produces identical
`SandboxPlan` output to the pre-migration hardcoded Rust. Golden files
(`control/agentctl/tests/golden/<workload>.plan.txt`) use the `SandboxPlan`
`Display` impl (`plan.rs:140-198`). `just golden-check` diffs current output
against committed goldens. `workloads/*.rs` are deleted only after golden
parity is proven (ADR 0001).

**Instance**
A single running sandbox for a workload, identified by an instance name (see
**slot**). An instance is what `workestrate ps` lists and what `up`/`exec`
creates. Tracked in the port registry at `${state_dir}/var/run/<instance>.json`.
See ADR 0021.

**Slot**
A workload's sandbox identity, used as the collision key for instance
placement. A slot is either a **singleton slot** (`<workload>` when no
context is active, or `<context>-<workload>` when a context is active per
ADR 0019) or a **parallel instance slot** (`<slot>@<id>`). At most one
sandbox may occupy a singleton slot; parallel instance slots allow
side-by-side execution. See ADR 0021.

**Singleton (instance)**
The instance occupying a workload's singleton slot (no `@<id>` suffix). At
most one per slot. `workestrate workload up <name>` with no instance flags targets the
singleton slot and refuses if it is occupied. See ADR 0021.

**Parallel instance**
An instance on a parallel instance slot, named `<slot>@<id>` (e.g.
`litellm@canary`). Coexists with the singleton and with other parallel
instances of the same workload. Created via `--instance <id>` or `--new`.
Typically paired with `--port-offset N` to avoid host-port collisions
(removed pre-release; superseded by ADR 0026's per-instance addressing). See
ADR 0021.

**Blue-green (config change workflow)**
A safe mutation workflow for an agent (or operator) modifying the project:
bring up the new revision on a parallel instance slot with `--new
--port-offset N` (removed pre-release; superseded by ADR 0026's per-instance
addressing), smoke-test it on the offset port, then atomically cut over
by stopping the old instance (`down --instance <old>` or `up --replace` on
the singleton). The refuse-on-occupied default makes this workflow native —
the new revision comes up alongside the old one rather than destroying it.
See ADR 0021.
