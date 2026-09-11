# 04 — CLI Config Authoring (DEFERRED vision + requirements traceability)

> **STATUS: DEFERRED** — pending sign-off of
> [`02-config-requirements.md`](../02-config-requirements.md) (the contract this
> tool will target). Do not implement before that sign-off.
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [`02-config-requirements.md`](../02-config-requirements.md) (the contract) ·
> [`../../migration/50-decisions/0002-toml-config-format.md`](../../migration/50-decisions/0002-toml-config-format.md)

This document is the **vision and requirements-traceability** record for the
future toml_edit-based CLI config-authoring tool. It is explicitly DEFERRED:
the user deferred the CLI build but wants the requirements pinned first so
the CLI work is not redone. The contract this tool targets is
[`02-config-requirements.md`](../02-config-requirements.md) — that document
pins the schema, merge semantics, policy ceiling, and trust model. This
document pins what the CLI must do to preserve that contract during edits.

> Nothing in this document describes an existing CLI. The existing `config`
> surface is enumerated in §3.1; everything in §3.2 onward is PROPOSED and
> does not exist in the codebase today.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | The eventual CLI runs in-container (TOML edits, in-process validate). |
| `HOST-NIX` | Not required for the CLI itself. |
| `HOST-KVM` | Not required for the CLI itself. |

---

## 1. Rationale recap

The user's explicit position: the CLI config-authoring tool is **deferred**, but
the requirements must be pinned first so that when the CLI is built, it targets
a frozen contract and is not re-litigated or redone. This split is:

- [`02-config-requirements.md`](../02-config-requirements.md) — **the contract**:
  the `workestrate.toml` schema, the merge/layering semantics, the policy
  ceiling, the trust model, and the planned extensions. The schema structs and
  merge engine already implement this contract.
- **This document** — **the vision + requirements traceability**: what the CLI
  must do (lossless round-trip, layer targeting, validate-after-mutation, secret
  hygiene, trust gating, schema-driven field sets) to preserve that contract
  during interactive edits.

The CLI is the human-friendly editor for a config format that ADR 0002 chose
specifically for human-editability. A comment-destroying CLI would betray that
rationale (§4.1).

---

## 2. Dependency on the mount exclude/shadow schema

The CLI must not ship before the planned mount filtering/shadowing schema is
finalized, or must be additive-tolerant. The mount extension adds fields to
`MountPlan` (`exclude`, `[[mounts.shadow]]`, `allow_sensitive`) per
[`02-config-requirements.md`](../02-config-requirements.md) §7.1. If the CLI
ships first with a hardcoded field set, it will not know how to edit those
fields and will either drop them (lossy) or refuse to touch the table
(partial). Two acceptable orderings:

1. **Schema-first:** finalize the mount exclude/shadow schema, then build the
   CLI against the complete field set.
2. **Additive-tolerant:** the CLI is schema-driven (§4.6) from the start, so
   new fields added to the structs automatically appear in the CLI's editable
   surface without a CLI change.

Option 2 is the intended design (§4.6), which makes the CLI naturally
additive-tolerant. But the mount schema must still be finalized before the CLI
is considered complete, because the CLI's validation-after-mutation (§4.3)
must validate the new fields.

---

## 3. Envisioned command surface

### 3.1 Existing `config` surface (verified — do not collide)

The existing `workestrate config` subcommands are defined in
`control/agentctl/src/cli_actions.rs:100-181` and dispatched in
`control/agentctl/src/main.rs:264`:

| Existing verb | Purpose | Citation |
|---|---|---|
| `config add <url> <name> [--ref main]` | Clone a config repo into the managed store and register it. | `cli_actions.rs:102-107` |
| `config update [name]` | Pull latest for a config repo (or all) and update rev in registry. | `cli_actions.rs:108-109` |
| `config list` | List registered config repos with rev + dirty status. | `cli_actions.rs:110-111` |
| `config trust <dir>` | Trust a project directory for project-layer config loading. | `cli_actions.rs:112-113` |
| `config untrust <dir>` | Remove trust from a project directory. | `cli_actions.rs:114-115` |
| `config new <name> [...]` | Scaffold a new config repo locally. | `cli_actions.rs:116-168` |
| `config remove <name> [--delete] [--force]` | Unregister a config repo from the registry. | `cli_actions.rs:170-180` |

The existing `context` surface (`cli_actions.rs:185-190`): `context list`,
`context current`.

The top-level validation surface (`main.rs:81-85`): `validate-config`,
`generate-schema`, `secrets-schema`, `generate-env-example`.

### 3.2 Proposed authoring surface (DOES NOT EXIST — proposed shape)

The authoring verbs target the **contents** of a `workestrate.toml` layer
(workloads, env, mounts, secrets, contexts), not the **registry** of config
repos. They must nest under `config` without colliding with the existing
repo-management verbs (`add`, `update`, `list`, `trust`, `untrust`, `new`,
`remove`).

**Proposed shape: a `config edit` family.** All authoring verbs live under a
new `config edit` subcommand, keeping the existing repo-management verbs
untouched and avoiding name collisions:

```
workestrate config edit workload add <name> [--layer <layer>] ...
workestrate config edit workload set <name> <field> <value> [--layer <layer>]
workestrate config edit workload rm <name> [--layer <layer>]
workestrate config edit workload show <name>

workestrate config edit env add <workload> <name> <value|secret> [--layer <layer>]
workestrate config edit env rm <workload> <name> [--layer <layer>]

workestrate config edit mount add <workload> <host> <guest> [--read-only] [--layer <layer>]
workestrate config edit mount rm <workload> <index> [--layer <layer>]
# (mount add/rm must support the exclude/shadow fields from 02-config-requirements.md §7.1
#  once that schema is finalized — see §2)

workestrate config edit secret add <name> [--env-var <VAR>] [--layer <layer>]
workestrate config edit secret rm <name> [--layer <layer>]

workestrate config edit context add <name> --layers <a,b,c>
workestrate config edit context rm <name>
```

**Justification for the `edit` namespace:**

- The existing `config` verbs are repo-lifecycle verbs (clone, pull, register,
  trust). The authoring verbs are file-content verbs (set a field, add a row).
  Putting them under `config edit` separates these concerns cleanly.
- A flat `config workload add` would collide conceptually with `config add`
  (which adds a *repo*, not a workload) and risk user confusion about what
  "add" operates on.
- `config edit` is a single new subcommand enum variant in `ConfigAction`
  (`cli_actions.rs:100`), keeping the clap tree shallow.

**Alternative considered (rejected):** workload-scoped top-level verbs like
`workestrate workload add`. Rejected because workloads are config data, not
top-level CLI objects — they live inside a `workestrate.toml` layer, and the
`--layer` targeting (§4.2) is essential to every edit. Burying `--layer` under
a top-level verb would make the layer ambiguity worse, not better.

---

## 4. Hard requirements (the pinned list)

These are non-negotiable. The CLI must satisfy all six.

### 4.1 Lossless round-trip (toml_edit)

The CLI must use `toml_edit` (not `toml`/serde round-trip) for every read and
write. A serde round-trip normalizes formatting and drops comments, which is
unacceptable for a config file that humans read and edit.

**Must preserve across every edit:**

| Property | Why |
|---|---|
| Comments | All `#` comments. ADR 0002 chose TOML for human-editing; a comment-destroying CLI betrays that. |
| Key order | Table/key declaration order as the human wrote it. |
| Whitespace | Blank lines, inline-vs-array-of-tables style, indentation. |
| Unrelated sections | An edit to `[workloads.litellm]` must not touch `[workloads.pi]` or `[secrets.*]`. |

This is the contract pinned in
[`02-config-requirements.md`](../02-config-requirements.md) §9.1.

### 4.2 `--layer <name>` targeting

Every edit must target a specific layer (which file the edit lands in). The
default target uses `resolve_active_config_dir()` semantics
(`control/agentctl/src/config/paths.rs:353-392`):

| `--layer` value | Target file | Resolution |
|---|---|---|
| *(omitted)* | The active config dir | `resolve_active_config_dir()` (`paths.rs:353-392`): `WORKESTRATE_CONFIG_DIR` env → trusted project `./workestrate.toml` → registry context's first layer. |
| `reference` | `config.reference/workestrate.toml` | The base layer shipped with the tool. |
| `<repo-name>` | `<store>/repos/<name>/workestrate.toml` | A registered config repo's layer. |
| `global` | `$WORKESTRATE_HOME/overrides.toml` `[global]` | User-global overrides applied to every context (ADR 0019). |
| `config:<name>` | `$WORKESTRATE_HOME/overrides.toml` `[configs.<name>]` | User-global overrides for a specific context layer. |
| `project` | `./workestrate.toml` | Trusted project layer (ADR 0014). |
| `local` | `./workestrate.local.toml` | Trusted local overrides (ADR 0020 Ruling 3). |

This matches the `--layer` contract in
[`02-config-requirements.md`](../02-config-requirements.md) §9.2.

### 4.3 Validate after every mutation (atomic write)

After every edit, the CLI must run the equivalent of `workestrate
validate-config` (the in-process equivalent of `cmd_validate_config`,
`main.rs:256`) to catch schema/policy violations immediately. A config that
parses but violates a policy invariant (e.g. a non-allowlisted egress host per
`ALLOWED_EGRESS_HOSTS`, `policy.rs:4-15`) must be rejected before the edit is
committed to disk.

**Atomic write:** the edit is written to a temp file in the same directory,
validated, then atomically renamed over the target. On validation failure, the
temp file is discarded and the original is left untouched (roll back). No
partial or invalid config ever reaches disk.

This is the contract pinned in
[`02-config-requirements.md`](../02-config-requirements.md) §9.3.

### 4.4 Never write secret values — only secret references

The CLI must never write or accept secret *values*. It only writes secret
*references*:

- `env_var` names (the env var the resolved value is injected as).
- `[secrets.<NAME>]` definitions (`env_var`, `hosts`, `required`,
  `placeholder`, `source`, `exposed_as`, `description` — see
  [`02-config-requirements.md`](../02-config-requirements.md) §1.2).
- `secret_env` references (names pointing into the top-level `secrets` map).

Actual secret values stay in the SOPS-encrypted `.env.enc` file. The CLI edits
the *declarations* (which secrets exist and how they are referenced), never the
*material*. This is consistent with the existing `generate-env-example` command
(`main.rs:85`), which emits placeholder values, not real ones.

### 4.5 Trust gating respected

Editing a `project` (`./workestrate.toml`) or `local`
(`./workestrate.local.toml`) layer requires the directory to be trusted (listed
in `[[trusted_projects]]` in the registry, ADR 0014). The CLI must check trust
via `is_trusted_project` (the same check `resolve_active_config_dir` uses at
`paths.rs:370`) before writing to a project or local layer, and refuse (or
warn) if untrusted.

The bootstrap exception (no registry → project/local layers load without trust,
`loading.rs:349-352`, `loading.rs:378-381`) applies: on a fresh install with no
registry, the CLI may write to `./workestrate.toml` without a trust check,
mirroring the loader's behavior.

This is the trust contract pinned in
[`02-config-requirements.md`](../02-config-requirements.md) §5.

### 4.6 Schema-driven field set

The set of editable fields must be generated/derived from the same `schemars`
types as `workestrate generate-schema` (ADR 0021 §8,
`docs/migration/50-decisions/0021-instance-lifecycle-model.md:208-213`), so the
CLI and the schema never drift. `generate-schema` calls
`schemars::schema_for!(crate::config::ConfigFile)` (`0021:213`); the CLI must
derive its editable field set from the same `ConfigFile` struct
(`control/agentctl/src/config/types.rs:194`) and its sub-structs.

This means: when a field is added to a config struct (e.g. the mount
`exclude`/`shadow` fields from §2), it automatically appears in both the
committed JSON Schema (`schemas/workestrate.schema.json`) and the CLI's editable
surface, with no separate CLI change. The schema drift guard (ADR 0021 §8,
`just schema-check`) already enforces that the committed schema matches the
structs; the CLI inherits this guarantee for free.

---

## 5. Non-goals for v1

| Non-goal | Rationale |
|---|---|
| No interactive TUI. | v1 is a scripted, idempotent CLI (set/add/rm with explicit `--layer`). A TUI is a later UX layer on top of the same toml_edit + validate primitives. |
| No remote registry sync. | The CLI edits local layer files. Syncing a config repo to a remote is the existing `config update` / `git push` workflow, not an authoring concern. |
| No copier re-render. | The CLI edits `workestrate.toml` in place. Re-rendering a config repo from a copier template is the existing `copier update` workflow (`scaffold/mod.rs:36`), not an authoring concern. |

---

## 6. Effort estimate and phased sketch

**Estimate: M (Medium).** The toml_edit integration, `--layer` resolution
(reusing `resolve_active_config_dir`), and validate-after-mutation (reusing
`cmd_validate_config`) are the bulk; the per-field verbs are mechanical once
the primitives are in place.

**Phased sketch:**

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | Read-only `show` | `config edit workload show <name>` — loads a layer via toml_edit, prints a workload's effective merged config (read-only, no writes). Validates the toml_edit read path and `--layer` resolution without mutation risk. |
| 2 | Scalar `set` | `config edit workload set <name> <field> <value> --layer <layer>` — edits a single scalar field (e.g. `cpus`, `memory_mib`, `workdir`). Validates the atomic-write + validate-after-mutation path (§4.3) on the simplest edit shape. |
| 3 | List-of-rows `add`/`rm` | `config edit env add/rm`, `config edit mount add/rm`, `config edit secret add/rm` — edits to `Vec<...>` fields (array-of-tables). Exercises toml_edit's array-of-tables manipulation, which is the hardest round-trip case. |
| 4 | Contexts | `config edit context add/rm` — edits to the registry's `[contexts.*]` (a different file: `$WORKESTRATE_HOME/config.toml`, not a `workestrate.toml` layer). Extends `--layer` to target the registry file. |

Each phase is independently shippable. Phase 1 is pure read (zero mutation
risk); phases 2–4 each add one edit shape, each gated by the
validate-after-mutation invariant (§4.3).
