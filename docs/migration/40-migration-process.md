# 40 — Migration Process

This document defines the full phased migration with every step, its gate, and
environment marker. It also includes the per-file consequence sweep, the
repo migration path (M-steps), golden-test strategy, rollback strategy, and
definition of done per phase.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (cargo, TOML, golden files) |
| `HOST-NIX` | Requires nix on the user's host (this container has no nix) |
| `HOST-KVM` | Requires KVM on the user's host (this container has no KVM) |

## Phase 0a: Data-driven workloads (cargo-verifiable here)

Includes odysseus/opencode nix derivations prerequisite (ADR 0012).

| Step | Action | Gate | Env |
|---|---|---|---|
| 0a.1 | Add `serde` (with derive) + `toml` crate to `Cargo.toml`. Derive `Deserialize` on `SandboxPlan` + all sub-structs (`plan.rs:57-138`) + `SecretDefinition` (`secrets.rs:5-17`). | `cargo check` passes | verifiable-here |
| 0a.2 | Define `workestrate.toml` schema. Implement config discovery + loader in `config.rs` (reads root `workestrate.toml`). | `cargo test` — loader tests pass (load test config, produce `SandboxPlan`) | verifiable-here |
| 0a.3 | Build egress recipe vocabulary: Rust enum in `recipes.rs` (`dns`, `litellm_proxy`, `github`, `agent_base`, `https`). Nix recipes in `nix/lib/recipes.nix`. Package vocabulary in `nix/lib/vocabulary.nix`. | `cargo check` (Rust enum); `nix eval` of recipe functions | verifiable-here (Rust); HOST-NIX (nix) |
| 0a.4 | Implement security allowlist in `policy.rs` (`ALLOWED_EGRESS_HOSTS`, `SECRET_HOST_BINDINGS`, `ALLOWED_PACKAGES`, `DEFAULT_DENY_FALSE_ENTITLEMENT`). Implement `workestrate validate-config` command. | `cargo test` — allowlist validation tests pass | verifiable-here |
| 0a.5 | Migrate 5 workloads from `workloads/*.rs` to root `workestrate.toml`. Delete `workloads/*.rs` (after golden parity, step 0a.6). Remove `workloads!` macro (`main.rs:49-69`). Implement hybrid CLI dispatch (ADR 0006): typed subcommands for known names + catch-all for config-defined. | `cargo check` + `cargo clippy` + `cargo test` | verifiable-here |
| 0a.6 | Generate golden files from `workestrate <name> plan` for all 5 workloads. Add `golden-check` to `just verify`. Verify golden parity: plans match pre-migration output. Delete `workloads/*.rs` only after parity proven. | `just golden-check` passes; golden plans match pre-migration | verifiable-here |
| 0a.7 | Update `setup-secrets.sh` to call `workestrate secrets-schema` for REQUIRED_KEYS (replaces `.env.example` grep at `setup-secrets.sh:39-44`). Implement `workestrate generate-env-example` command. | `just validate-secrets` passes | verifiable-here |
| 0a.8 | Update `cmd_new` (`main.rs:178-227`) to scaffold a config entry (append to `workestrate.toml`) + `agents/<name>/config/` directory. No Rust code instructions. | `workestrate new test-agent` creates config entry + dir | verifiable-here |
| 0a.9 | Create odysseus/opencode nix derivations (ADR 0012). `nix/packages/odysseus.nix` + `nix/packages/opencode.nix`. Update `flake.nix:77` to remove "Future:" comment. | `nix build .#odysseus-built` + `.#opencode-built` succeed | HOST-NIX |
| 0a.10 | Full validation. | `just verify` passes (compile + clippy + test + fmt + litellm-check + golden-check + Cargo.lock stability) | verifiable-here |

**Definition of done (Phase 0a)**: `just verify` passes. All 5 workload plans
produce output matching committed golden files. `workestrate validate-config`
passes. `workestrate new` scaffolds correctly. No `workloads/*.rs` files
remain. Odysseus/opencode nix derivations exist (HOST-NIX gate).

## Phase 0b: Nix recipe parameterization + devshell bridge (HOST-NIX)

| Step | Action | Gate | Env |
|---|---|---|---|
| 0b.1 | Implement `nix/lib/config.nix`: reads `config.reference/workestrate.toml` via `builtins.fromTOML`. Exports `workloadNames`, `nixLayeredImages`, `localBuilds` attrsets. | `nix eval .#lib.config.workloadNames` | HOST-NIX |
| 0b.2 | Update devshell `_build_agents` (`nix/devshells/default.nix:169-241`) to read `config.reference/workestrate.toml` via `builtins.fromTOML`. Devshell reads ONLY reference config (tool-dev). | `nix develop` enters; build plan correct | HOST-NIX |
| 0b.3 | Update `workload-images` attrset (`flake.nix:74-97`) to be config-driven (from `config.reference/`). | `nix eval .#workload-images` | HOST-NIX |
| 0b.4 | Parameterize `pi.nix`/`pi-bun.nix`/`tempest.nix` into named build recipes under `nix/lib/recipes/`. | `nix build .#pi-bun` succeeds | HOST-NIX |
| 0b.5 | Parameterize `pi-image.nix`/`tempest-image.nix` into generic `buildWorkloadImage` in `nix/lib/recipes/nix-layered.nix`. | `nix build .#workestrator-pi` succeeds | HOST-NIX |
| 0b.6 | Migrate microsandbox vendor symlink to git-fork dependency (ADR 0011). Remove `vendor-unlock`/`vendor-lock` recipes. Update `nix/packages/agentctl.nix:41-47` preBuild. | `nix build .#workestrate` succeeds; `cargo check` passes | HOST-NIX |
| 0b.7 | Full validation. | `just verify` + `nix build .#workestrate` | HOST-NIX |

**Definition of done (Phase 0b)**: `nix build .#workestrate` succeeds.
`nix build .#workestrator-pi` succeeds. Devshell enters and builds agents from
reference config. No vendor symlink. (All HOST-NIX gates — cannot verify in
this container.)

## Phase 1: Tool+XDG model (additive; KVM gate on host)

| Step | Action | Gate | Env |
|---|---|---|---|
| 1.1 | Create `config.reference/` directory: `workestrate.toml` (sanitized copy of root `workestrate.toml` with placeholder secrets), `agents/*/config/` reference copies, `infra/litellm/` reference values. | `workestrate validate-config config.reference/workestrate.toml` passes | verifiable-here |
| 1.2 | Implement XDG path resolution in `config.rs`: `~/.config/workestrate/config.toml` (registry), `~/.local/share/workestrate/repos/<name>/` (managed clones), `~/.local/state/workestrate/` (state). Keep root `workestrate.toml` as project-layer fallback. | `cargo test` — XDG path resolution tests pass | verifiable-here |
| 1.3 | Implement `workestrate config add/update/list/trust` commands. `config add` clones to `~/.local/share/workestrate/repos/<name>/`, adds to registry. `config update` pulls, updates `rev`. `config trust` adds to `[trusted_projects]`. | `cargo test` — config commands work against a test repo | verifiable-here |
| 1.4 | Implement `workestrate init <dotfiles-url>`: clone dotfiles, read registry, clone config repos, run setup-secrets for each. | `cargo test` — init creates expected XDG layout | verifiable-here |
| 1.5 | Implement `workestrate source clone/build/list/reset` commands. Store at `~/.local/share/workestrate/sources/<name>/`. `WORKESTRATE_<NAME>_BUILD` env unchanged. | `cargo test` — source commands work | verifiable-here |
| 1.6 | Update `setup-secrets.sh` with `--config <name>` flag targeting a config repo by name. Default: active context's personal layer. | `just validate-secrets` passes | verifiable-here |
| 1.7 | Update `workestrate check` for XDG model: report registered config repos (rev, dirty status), XDG state dirs, agent source checkouts. | `workestrate check` on fresh install reports `config.reference/` only | verifiable-here |
| 1.8 | Update runtime paths: `workspaces/` → `${state_dir}/workspaces/`, `var/` → `${state_dir}/var/`, `agents/<name>/repo` → `${store_dir}/sources/<name>/repo`. Update `mounts.rs:6-15` path resolution. | `cargo test` — path resolution tests pass | verifiable-here |
| 1.9 | Create user's personal config repo: move root `workestrate.toml` + `.env.enc` + `.sops.yaml` + `infra/litellm/` + `agents/*/config/` to a new git repo. `workestrate config add <url> personal`. | `workestrate config add` works; `workestrate pi plan` uses personal config | verifiable-here |
| 1.10 | Update all path references (consequence sweep below). Update README/SPEC/docs. | `just verify` passes (uses `config.reference/`); `workestrate pi plan` uses personal config via registry | verifiable-here |
| 1.11 | Runtime validation on KVM host. | `workestrate litellm up` + `workestrate pi exec` succeed on KVM host | **HOST-KVM** |

**Definition of done (Phase 1)**: `just verify` passes with XDG model.
`workestrate config add/update/list/trust` work. `workestrate init` bootstraps.
`workestrate source clone/build/list/reset` work. Runtime sandbox execution
succeeds on KVM host (HOST-KVM gate). Root `workestrate.toml` still works as
project layer (additive migration).

## Phase 2: Config-repo-flake (inverted dependency, optional mode)

| Step | Action | Gate | Env |
|---|---|---|---|
| 2.1 | Core exports `lib.*` in `flake.nix` outputs: `lib.recipes`, `lib.vocabulary`, `lib.buildWorkloadImage`, `lib.buildImagesFromConfig`, `lib.checks.validateConfig`. | `nix eval .#lib.recipes` succeeds | HOST-NIX |
| 2.2 | Add optional `flake.nix` to config repo (inverted dependency: takes core as input). | `nix build` in config repo succeeds | HOST-NIX |
| 2.3 | Add `checks` to config-repo flake: `workestrate validate-config` on `nix flake check`. | `nix flake check` in config repo passes | HOST-NIX |
| 2.4 | Copier template for config repo scaffolding (includes `flake.nix` with inverted dependency). | `copier copy` produces valid config repo | HOST-NIX |

**Definition of done (Phase 2)**: Core exports `lib.*`. Config repo with
`flake.nix` builds its own nix-layered images. Copier template produces valid
config repos. (All HOST-NIX gates.)

## Phase 3: Layering engine (fixtures, multi-recipient SOPS, copier)

| Step | Action | Gate | Env |
|---|---|---|---|
| 3.1 | Implement merge engine: RFC 7396 for non-security fields + security-aware merge (monotonic `default_deny`, additive `deny_rules`/`egress_rules`/`secret_env`, per-recipe scoping). Layer order from registry `layers` array. | `cargo test` — fixture-repo merge tests pass (base + team + personal) | verifiable-here |
| 3.2 | Implement `plan --show-source` (per-field provenance with layer names). | `cargo test` — source attribution tests pass | verifiable-here |
| 3.3 | Multi-recipient `.sops.yaml` layout: per-path `creation_rules` (shared vs personal-only). Per-domain secret files in config repos. | `just validate-secrets` passes with multi-recipient config | verifiable-here |
| 3.4 | `workestrate config add/update` with `config.lock.json`-style rev tracking (in registry, not separate lockfile). | `workestrate config add` + `update` work with rev tracking | verifiable-here |
| 3.5 | Copier template finalization (includes `flake.nix`, `.sops.yaml` with placeholder recipient, `workestrate.toml` skeleton). | `copier copy` + `copier update` work | HOST-NIX (copier) |
| 3.6 | Full validation. | `just verify` + fixture-repo merge tests + copier template | verifiable-here |

**Definition of done (Phase 3)**: Fixture-repo merge tests pass (base + team +
personal → expected merged output). `plan --show-source` annotates fields.
Multi-recipient SOPS works. Copier template produces valid config repos.
Contexts remain deferred (ship single ordered `layers` list; named contexts
only when 3+ layers).

## Per-file consequence sweep

Exhaustive table of every file/tool/doc that changes under the migration.

| File/Tool | Current state | Change | Phase |
|---|---|---|---|
| `control/agentctl/Cargo.toml` | `serde_json = "1"` only | Add `serde` (with derive) + `toml` crate | 0a.1 |
| `control/agentctl/src/main.rs` | `workloads!` macro (49-69); `cmd_new` scaffolds Rust (178-227) | Remove macro; hybrid dispatch (typed + catch-all); `cmd_new` scaffolds config entry; new commands (config, init, source, validate-config, etc.) | 0a.5, 0a.8, 1.3-1.5 |
| `control/agentctl/src/config.rs` | `project_root()` (4-33); `check_required_files()` (77-129) hardcoded list | XDG path resolution; config discovery (registry → layers → project → reference); `check_required_files` updated for XDG paths | 0a.2, 1.2, 1.7 |
| `control/agentctl/src/microsandbox/plan.rs` | `SandboxPlan` IR (57-70); egress helpers (270-305) | Add `Deserialize` derives; egress helpers → `recipes.rs` enum | 0a.1, 0a.3 |
| `control/agentctl/src/microsandbox/secrets.rs` | `const SecretDefinition` (33-92) | Becomes config-driven (deserialized from `secrets:` section); consts move to `policy.rs` as allowlist | 0a.1, 0a.4 |
| `control/agentctl/src/microsandbox/runtime.rs` | `apply_plan_secrets` (107-145); `build_sandbox` (335-382) | Unchanged (enforcement point); path resolution updated for XDG state | 1.8 |
| `control/agentctl/src/microsandbox/workload.rs` | `Workload` trait (45-96); `build_path()` (80-90); `config_path()` (92-95) | One generic `ConfigWorkload` impl; `build_path` reads `local_build.env_override`; `config_path` becomes config-relative | 0a.5 |
| `control/agentctl/src/microsandbox/secrets_loader.rs` | `load_secrets()` (16-61) | Unchanged (sops decrypt + env inject); targets config repo by path | 1.6 |
| `control/agentctl/src/microsandbox/mounts.rs` | `resolve_mount_host()` (6-15) | Updated for XDG state paths (`workspaces/` → `${state_dir}/workspaces/`) | 1.8 |
| `control/agentctl/src/microsandbox/env.rs` | `resolve_templated_value()` (7-28) | Unchanged | — |
| `control/agentctl/src/recipes.rs` | Does not exist | NEW: egress recipe enum + `expand()` | 0a.3 |
| `control/agentctl/src/policy.rs` | Does not exist | NEW: `ALLOWED_EGRESS_HOSTS`, `SECRET_HOST_BINDINGS`, `ALLOWED_PACKAGES`, `DEFAULT_DENY_FALSE_ENTITLEMENT` consts | 0a.4 |
| `control/agentctl/src/workloads/*.rs` | 5 hardcoded workload files | DELETE after golden parity (0a.6) | 0a.5-0a.6 |
| `flake.nix` | Inputs (7-25); wrapper (105-119); workload-images (74-97); secrets wrappers (134-196) | Add `lib.*` exports (Phase 2); workload-images config-driven from `config.reference/`; odysseus/opencode derivations (0a.9); remove vendor symlink (0b.6) | 0a.9, 0b.3, 0b.6, 2.1 |
| `nix/lib/config.nix` | Does not exist | NEW: reads `config.reference/workestrate.toml` via `builtins.fromTOML` | 0b.1 |
| `nix/lib/recipes.nix` | Does not exist | NEW: build/image recipe functions | 0a.3, 0b.4 |
| `nix/lib/vocabulary.nix` | Does not exist | NEW: package + feature vocabulary | 0a.3 |
| `nix/lib/recipes/*.nix` | Does not exist | NEW: `npm-build.nix`, `bun-compile.nix`, `pip-install.nix`, `bun-install.nix`, `nix-layered.nix` | 0b.4-0b.5 |
| `nix/packages/pi.nix` | 105 lines, hardcoded | Parameterized into `nix/lib/recipes/npm-build.nix` (pi-specific params) | 0b.4 |
| `nix/packages/pi-bun.nix` | 90 lines, hardcoded | Parameterized into `nix/lib/recipes/bun-compile.nix` (incl. `removeReferencesTo`) | 0b.4 |
| `nix/packages/pi-image.nix` | 73 lines, hardcoded | Parameterized into `nix/lib/recipes/nix-layered.nix` | 0b.5 |
| `nix/packages/tempest.nix` | 61 lines, hardcoded | Parameterized into `nix/lib/recipes/npm-build.nix` (tempest params) | 0b.4 |
| `nix/packages/tempest-image.nix` | 47 lines, hardcoded | Parameterized into `nix/lib/recipes/nix-layered.nix` | 0b.5 |
| `nix/packages/odysseus.nix` | Does not exist | NEW: odysseus nix derivation (ADR 0012) | 0a.9 |
| `nix/packages/opencode.nix` | Does not exist | NEW: opencode nix derivation (ADR 0012) | 0a.9 |
| `nix/packages/agentctl.nix` | Vendor symlink in preBuild (41-47) | Migrate to git-fork dependency (ADR 0011) | 0b.6 |
| `nix/packages/microsandbox-filesystem-patched.nix` | Patched crate | Migrate to git-fork dependency | 0b.6 |
| `nix/packages/microsandbox-filesystem-agentd.patch` | Patch file | Removed (fork-carries-compat) | 0b.6 |
| `nix/devshells/default.nix` | `_setup_agent_repos` (127-167); `_build_agents` (169-241); vendor link (99-125) | `_build_agents` reads `config.reference/`; `_setup_agent_repos` → `workestrate source clone`; vendor link removed | 0b.2, 0b.6, 1.5 |
| `scripts/setup-secrets.sh` | REQUIRED_KEYS from `.env.example` grep (39-44); paths hardcoded to repo root | `--config <name>` flag; REQUIRED_KEYS from `workestrate secrets-schema`; paths XDG-relative | 0a.7, 1.6 |
| `scripts/host-check.sh` | KVM/Nix/memory/disk check | Unchanged (core tooling) | — |
| `scripts/validate-secrets-workflow.sh` | Tests secrets lifecycle | Updated for `--config` flag + XDG paths | 1.6 |
| `justfile` | `check`, `litellm-check`, `verify`, `verify-full`, `build`, `fmt`, `clippy`, `test`, `workestrate`, `plan`, `host-check`, `setup-secrets`, `validate-secrets`, `vendor-unlock`, `vendor-lock`, `dev-build-pi`, `dev-run-pi`, `load-images` | Add `golden-check`, `golden-generate`; remove `vendor-unlock`/`vendor-lock`; `litellm-check` path updated; add `init-dev` (dogfooding) | 0a.6, 0b.6, 1.10 |
| `.gitignore` | `agents/*/repo`, `agents/*/build`, `agents/*/.build-hash` (20-22); `!.env.enc`, `!.env.example` (4-5) | Remove `!.env.enc`/`!.env.example` (no longer at root); no `config.d/` entries expected under XDG model; remove if present from prior experiments | 1.10 |
| `.agents/skills/validation-litellm-config-check/scripts/check_config.py` | `--config infra/litellm/config.yaml` | `--config` path resolves to active config repo's `infra/litellm/config.yaml` | 1.10 |
| `README.md` | Path references to `.env.enc`, `infra/litellm/`, `agents/*/config/`, `workspaces/`, `var/` | Updated for XDG model; quick start updated for `workestrate init`; architecture diagram updated | 1.10 |
| `SPEC.md` | Path references; M1 acceptance criteria | Updated for XDG model; milestone criteria updated | 1.10 |
| `docs/secrets.md` | Path references to `.env.enc`, `.sops.yaml` | Updated for XDG model + `--config` flag | 1.10 |
| `config.reference/` | Does not exist | NEW: tracked, sanitized reference config (workestrate.toml + agents/*/config + infra/litellm) | 1.1 |
| `workestrate.toml` (root) | Does not exist (Phase 0a creates it) | Created in 0a.5; becomes project layer in Phase 1; removed from repo in M.10 (optional) | 0a.5, 1.9 |
| `.env.enc` | At repo root | Moves to `~/.local/share/workestrate/repos/personal/.env.enc` | 1.9 |
| `.sops.yaml` | At repo root | Moves to config repo | 1.9 |
| `.env.example` | At repo root (24 lines) | Generated by `workestrate generate-env-example`; moves to config repo | 0a.7, 1.9 |
| `infra/litellm/` | At repo root | Values move to config repo; schemas stay in `docs/litellm/` | 1.9 |
| `agents/*/config/` | At repo root (tracked) | Moves to config repo | 1.9 |
| `workspaces/` | At repo root (gitignored) | Moves to `~/.local/state/workestrate/workspaces/` | 1.8 |
| `var/` | At repo root (gitignored) | Moves to `~/.local/state/workestrate/var/` | 1.8 |
| `agents/*/repo/`, `agents/*/build/` | At repo root (gitignored) | Moves to `~/.local/share/workestrate/sources/<name>/` | 1.5, 1.8 |
| `profiles/` | At repo root (tracked, docs) | Stays in core as reference profiles | — |

## Repo migration path (M-steps)

Ordered steps for migrating the current ai-workbench repo contents to the
tool+XDG model. Each step keeps `just verify` green.

| Step | Action | Gate |
|---|---|---|
| M.1 | Create `config.reference/workestrate.toml` (sanitized copy of root `workestrate.toml` with placeholder secrets). Copy `agents/*/config/` and `infra/litellm/` to `config.reference/`. | `workestrate validate-config config.reference/workestrate.toml` passes |
| M.2 | Implement XDG path resolution in `config.rs` (Phase 1, step 1.2). | `cargo test` — XDG path tests |
| M.3 | Implement `workestrate config add/update/list/trust` (Phase 1, step 1.3). | `cargo test` — config commands |
| M.4 | Implement `workestrate init <url>` (Phase 1, step 1.4). | `cargo test` — init creates XDG layout |
| M.5 | Implement `workestrate source clone/build/list/reset` (Phase 1, step 1.5). | `cargo test` — source commands |
| M.6 | Update `setup-secrets.sh` with `--config` flag (Phase 1, step 1.6). | `just validate-secrets` passes |
| M.7 | Update `workestrate check` for XDG model (Phase 1, step 1.7). | `workestrate check` on fresh install reports `config.reference/` only |
| M.8 | Update runtime paths: `workspaces/`, `var/`, `agents/<name>/repo` → XDG state/store (Phase 1, step 1.8). | `cargo test` — path resolution |
| M.9 | Create user's personal config repo: `git init` in a temp dir, move root `workestrate.toml` + `.env.enc` + `.sops.yaml` + `infra/litellm/` + `agents/*/config/` to it. Push to remote. `workestrate config add <url> personal`. | `workestrate config add` works; `workestrate pi plan` uses personal config |
| M.10 | Remove migrated files from ai-workbench repo (root `workestrate.toml`, `.env.enc`, `.sops.yaml`, `infra/litellm/`, `agents/*/config/`, `workspaces/`, `var/`). **OPTIONAL/DEFERRED** — root `workestrate.toml` keeps working as project layer; migration is additive. | `just verify` passes (uses `config.reference/`); `workestrate pi plan` uses personal config via registry |
| M.11 | Update docs (README, SPEC, docs/secrets.md) for tool+XDG model. Add `just init-dev` recipe for dogfooding. | docs reviewed |
| M.12 | Runtime validation on KVM host. | `workestrate litellm up` + `workestrate pi exec` succeed | **HOST-KVM** |

## Golden-test strategy

### Format

Golden files use the `SandboxPlan` `Display` impl output (`plan.rs:140-198`) —
the same text that `workestrate <name> plan` prints. This means golden output
matches exactly what users see.

### Location

`control/agentctl/tests/golden/<workload>.plan.txt` — one file per workload,
committed to git.

### Recipes

```makefile
# justfile
golden-generate:
	@for name in litellm pi odysseus opencode tempest; do \
		cargo run --manifest-path control/agentctl/Cargo.toml -- $$name plan \
		  > control/agentctl/tests/golden/$$name.plan.txt; \
	done

golden-check:
	@for name in litellm pi odysseus opencode tempest; do \
		cargo run --manifest-path control/agentctl/Cargo.toml -- $$name plan \
		  | diff - control/agentctl/tests/golden/$$name.plan.txt \
		  || (echo "golden mismatch for $$name; run 'just golden-generate' to update" && exit 1); \
	done
```

`golden-check` is added to `just verify`.

### Fate of existing struct-assertion tests (`runtime.rs:454-678`)

- **Value-equality tests** (e.g. `assert_eq!(plan.image.as_deref(), Some("workestrator-pi:latest"))` at `runtime.rs:485`): **rewritten as loader tests** — load config, produce plan, diff against golden file.
- **Invariant tests** (e.g. "pi must NOT host-bind LITELLM_MASTER_KEY" at `runtime.rs:622-643`): **kept as semantic checks** on the loaded plan. These verify properties that golden-file diffing alone can't catch.

## Rollback strategy per phase

| Phase | Rollback strategy |
|---|---|
| 0a | Revert to `workloads/*.rs` (git history preserves). Golden files and config loader are additive. |
| 0b | Revert nix recipe parameterization (git history preserves per-agent nix files). Vendor symlink restored. |
| 1 | Root `workestrate.toml` still works as project layer (additive migration). XDG commands are additive. Revert = don't use XDG paths; use root config. |
| 2 | Config-repo-flake is optional. Revert = remove `flake.nix` from config repo. |
| 3 | Layering engine is additive. Revert = use single-layer (no `layers` in registry). |

## Definition of done per phase

| Phase | Done means |
|---|---|
| 0a | `just verify` passes. Golden parity proven. `workloads/*.rs` deleted. Odysseus/opencode derivations exist (HOST-NIX). |
| 0b | `nix build .#workestrate` + `.#workestrator-pi` succeed. Devshell reads `config.reference/` only. No vendor symlink. (All HOST-NIX.) |
| 1 | `just verify` passes with XDG model. `workestrate config/init/source` commands work. Runtime on KVM host succeeds (HOST-KVM). Root `workestrate.toml` works as project layer. |
| 2 | Core exports `lib.*`. Config-repo-flake builds images. Copier template works. (All HOST-NIX.) |
| 3 | Fixture-repo merge tests pass. `plan --show-source` works. Multi-recipient SOPS works. Copier template finalized. |
