# ADR 0039: Tool vocabulary rename — `config` for the tool home, `fleet` for config repos

**Status:** Accepted
**Date:** 2026-09-20
**References:** `control/agentctl/src/cli.rs` (global flags + subcommand tree); `control/agentctl/src/config/registry.rs` (`[fleets.<name>]`, `config_version`); `control/agentctl/src/config/lockfile.rs` (`[fleets.<name>]`); `control/agentctl/src/config/loading.rs` (overrides `[fleets.<name>]`); `control/agentctl/src/commands/{config_cmd.rs,fleet_cmd.rs,migrate.rs}`; `schemas/registry.schema.json`; amends the vocabulary (not the mechanics) of ADR 0008, 0014, 0023, 0025, 0034

## 1. Title / Status / Date

- **Title:** Tool vocabulary rename — the tool home becomes `config`, config repositories become `fleets`
- **Status:** Accepted.
- **Date:** 2026-09-20.
- **Amends:** user-facing vocabulary of ADR 0008 (config repos), ADR 0014 (`config trust`), ADR 0023 (single tool home), ADR 0025 (`workestrate.lock` `repos.*`), ADR 0034 (secrets targeting flags). Mechanics are unchanged; only names move.

## 2. Context

The old vocabulary had two collisions that kept confusing operators:

- **`home` meant two things.** The tool home (`--home`, `WORKESTRATE_HOME`, `workestrate home <verb>`) is the operator's registry+state root — but "home" also reads as the OS home directory and as the Microsandbox runtime home (`MSB_HOME`), both of which are real, separate concepts in the same sentences.
- **`config` meant two things.** A registered configuration repository (`workestrate config list|add|new|update|remove|trust`) is a git repo full of workload capsules — while `--config <name>` selected one, and "the config" could mean the tool home's own `config.toml` registry.

ADR 0038 already settled on "fleet" as the concept name for an operator-owned composition of workloads ("fleets trust the tool contract"). This ADR completes that choice at the CLI surface: a config repository IS a fleet, and the tool home is simply the **config** — the place where the tool's own configuration (registry, overrides, contexts) lives.

## 3. Decision

Clean break, no compatibility aliases. Old spellings are unrecognized input (clap unknown-flag / unknown-subcommand / unknown-field errors), not deprecated synonyms.

| Old | New |
|---|---|
| `--home <DIR>` | `--config <DIR>` |
| `WORKESTRATE_HOME` | `WORKESTRATE_CONFIG` |
| `workestrate home <verb>` | `workestrate config <verb>` |
| `workestrate config <verb>` (list/add/new/update/remove/trust/untrust) | `workestrate fleet <verb>` |
| `--config <name>` (fleet selector on secrets/build/schemas/signing) | `--fleet <name>` |
| `home init --config <url>` | `config init --fleet <url>` |
| secrets `--config-dir <DIR>` | `--fleet-dir <DIR>` |
| `WORKESTRATE_CONFIG_DIR` | `WORKESTRATE_FLEET_DIR` |
| registry `[configs.<name>]` | `[fleets.<name>]` |
| registry `[settings] home_version` | `config_version` |
| lockfile `[repos.<name>]` | `[fleets.<name>]` |
| overrides `[configs.<name>]` | `[fleets.<name>]` |
| layout dir `config-repos/` | `fleets/` |
| `workestrate migrate-home` | `workestrate migrate-config` |
| build `--repo` / `--all-repos` | `--fleet` / `--all-fleets` |
| prose "tool home" | "config" |
| prose "config repo(s)" | "fleet(s)" |

Contexts (`context list|current|use`), `~/.workestrate` as the default root, `--config-ref`, the copier template variable `config_name`, and the Microsandbox-side homes (`MSB_HOME`, state generations) are unchanged.

Consequences of the clean break, accepted deliberately:

- An old registry (`[configs.*]`) or lockfile (`[repos.*]`) fails to parse under the renamed serde types (`deny_unknown_fields`): re-register fleets with `fleet add`, or edit the keys by hand.
- `workestrate doctor --json` renames the `home`→`config` and `repos`→`fleets` checks and the per-fleet array key `repos`→`entries`.
- Shell completions and wrapper scripts forward the new flag spellings only.

## 4. Why a clean break instead of aliases

Aliases would keep both vocabularies alive in help text, error messages, completions, and every future doc — the confusion the rename exists to remove. The tool is pre-1.0 with a small operator set, and the failure mode (a clap "unrecognized" error naming the valid spellings) is self-explanatory. One migration note (this ADR) replaces a permanent dual-vocabulary tax.

## 5. Scope excluded on purpose

- Live operator state: existing `~/.workestrate` registries and lockfiles are NOT auto-migrated by this change; re-registration or a follow-up migration command is a separate decision.
- Historical documents: dated ADRs, assessment reports, and crawl archives keep the vocabulary of their time.
- The beads tracker content (issue text written before the rename).
