# 00 — Executive Summary

## Why migrate

The `ai-workbench` repo is today both a **tool** (the `workestrate` Rust CLI,
nix flake, microsandbox integration, SOPS wrappers) and a **workspace** (the
user's personal workload definitions, encrypted secrets, agent configs, LiteLLM
values, runtime state). This coupling creates three problems:

1. **Registry recursion.** If fleets are gitignored children
   inside the workspace repo (`config.d/<name>/`), the registry of *which*
   fleets exist has no tracked config. A second machine cannot reconstruct
   the set of fleets without out-of-band knowledge. The recursion
   ("where do the references live?") terminates only at a user-level dotfiles
   repo — which is a solved problem (chezmoi, yadm, stow, kubeconfig).

2. **Distribution friction.** The tool is coupled to a specific workspace
   checkout. Running `workestrate workload exec pi` from a different project requires
   `cd`-ing into the workspace repo. Distributing the tool to a teammate
   means distributing the workspace (or surgically extracting the tool).

3. **Day-to-day overhead.** Adding a team config requires editing `flake.nix`
   inputs. Switching between personal and work configs requires editing
   `config.d/layers.toml`. The mental model is "which child am I in, and is it
   up to date?" instead of "run the tool from anywhere."

## What — the tool+XDG+dotfiles model

`workestrate` becomes a **tool** (installed via `nix profile install` or `nix
run`), decoupled from any workspace. Configuration lives in three XDG layers:

| Layer | Path | Contents | Tracked where? |
|---|---|---|---|
| **Registry** | `~/.config/workestrate/config.toml` | Tool settings, fleet registry (name→url→ref→rev), ordered `layers` list, `[trusted_projects]` | User's dotfiles repo |
| **Config repos** | `~/.local/share/workestrate/repos/<name>/` | `workestrate.toml`, `.env.enc`, `.sops.yaml`, `infra/litellm/`, `agents/*/config/` | Each fleet's own git |
| **State** | `~/.local/state/workestrate/` | `sources/<name>/` (agent source checkouts + builds), `workspaces/`, `var/` | Not tracked (runtime state) |

The registry is the **home** for fleet references — it IS the user's
dotfiles. This is the kubeconfig model: `~/.kube/config` holds cluster
references; `~/.config/workestrate/config.toml` holds fleet references.
The recursion terminates at the dotfiles repo, which is bootstrapped via
`workestrate init <dotfiles-url>` (chezmoi-init style).

Config repos are consumed **uniformly** via `workestrate config add/update/list`
(git clone into the managed store). No parent-flake-input editing is required
for fleets. Flake materialization is optional sugar for team repos.

## How — the phases

| Phase | Scope | Verifiable here? | Gate |
|---|---|---|---|
| **0a** | Data-driven workloads: serde+toml, `workestrate.toml` schema, recipe vocabulary, `policy.rs` allowlist, migrate 5 workloads, golden-file parity, `external_subcommand`-hybrid dispatch. Includes odysseus/opencode nix derivations prerequisite. | Yes (cargo) | `just verify` + golden parity |
| **0b** | Nix recipe parameterization: `nix/lib/recipes.nix`, `nix/lib/vocabulary.nix`, `buildWorkloadImage`, devshell reads `config.reference/` only. | No (HOST-NIX) | `nix build .#workestrate` + `.#workestrate-pi` |
| **1** | Tool+XDG model: XDG path resolution, `workestrate config add/update/list/trust`, `workestrate init`, `workestrate source clone/build/list/reset`, runtime path migration, personal fleet creation. Additive — root `workestrate.toml` keeps working as project layer. | Yes (cargo) except KVM gate | `just verify` + `HOST-KVM` runtime |
| **2** | Config-repo-flake (inverted dependency): core exports `lib.*`; fleets optionally have own `flake.nix` taking core as input. Optional mode. | No (HOST-NIX) | `nix build` in fleet |
| **3** | Layering engine: ordered `layers` merge (RFC 7396 + security-aware merge), `plan --show-source` provenance, multi-recipient SOPS, copier template. Contexts deferred. | Yes (cargo) | Fixture-repo merge tests |

## What didn't change

- **Security invariants.** Config purity (data+secrets+static files only; no
  scripts/nix/`extraCommands`), closed recipe vocabulary in core, `policy.rs`
  allowlist ceiling, three enforcement points (`validate-config`, `plan`
  fail-closed, runtime `apply_plan_secrets`). These are independent of the
  organization model.
- **The `SandboxPlan` IR** (`plan.rs:57-70`). The data model is unchanged; only
  its population source shifts from Rust `plan()` methods to TOML config.
- **The `Workload` trait** (`workload.rs:45-96`). One generic `ConfigWorkload`
  implements it; the trait itself is unchanged.
- **SOPS+age secrets infrastructure.** `setup-secrets.sh`, `decrypt-env`,
  `write-env` wrappers stay in core; they target a fleet by name or path.
- **Microsandbox SDK integration.** `runtime.rs`, `mounts.rs`, `env.rs` are
  unchanged. Only `secrets.rs` const definitions become config-driven.
