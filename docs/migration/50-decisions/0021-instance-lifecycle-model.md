# ADR 0021: Instance lifecycle model + AI-native surfaces

**Status:** Accepted
**Date:** 2026-07-20
**References:** `20-target-system-spec.md` §6 (CLI surface), §13 (instance model);
`60-glossary.md` (instance, slot, singleton, parallel instance, blue-green);
ADR 0019 (instance namespacing via `<context>-<workload>`); ADR 0020 Ruling 4
(spec-code CI guard pattern, reused for schema drift).

## Context

Historically, `workestrate <name> up` / `exec` always **replaced** any existing
sandbox for that workload name. The implementation called
`Sandbox::builder().replace()` unconditionally (the motivating site is
`runtime.rs:381` in the implementation track — `builder.replace()`), which
silently tears down the running instance and starts a fresh one on the same
slot. This was acceptable for a single-operator, single-context-at-a-time
model, but two pressures make it the wrong default now:

1. **User requirement — choose replace vs coexist.** Operators want to run a
   canary instance of a workload alongside the live one (e.g. a LiteLLM proxy
   with a candidate `config.yaml` on an offset port) without destroying the
   serving instance. Silent replace destroys the canary's baseline.
2. **AI-native operation.** When an agent is modifying the project (editing
   `workestrate.toml`, agent configs, LiteLLM values) and validating the
   change end-to-end, it needs a **safe blue-green mutation workflow**: bring
   up the new revision on a parallel slot, smoke-test it, then atomically
   cut over by replacing the old slot. A CLI that silently replaces on `up`
   cannot express this workflow — the agent cannot hold two revisions live
   long enough to compare them.

The same pressure motivates a small set of **AI-native operational surfaces**:
machine-readable output (`--json`), a process-status command (`ps`), a
committed JSON schema for `workestrate.toml` with editor integration
(`generate-schema` + taplo `#:schema`), and explicit instance-targeting flags
so that an agent (or a human) can drive the lifecycle deterministically.

## Decision

### 1. Instance slots

A workload's sandbox identity is a **slot**, not a bare name. A slot is one
of:

- **Singleton slot** — `<workload>` when no context is active, or
  `<context>-<workload>` when a context is active (per ADR 0019). At most one
  sandbox may occupy a singleton slot.
- **Parallel instance slot** — `<slot>@<id>`, where `<slot>` is the singleton
  name above and `<id>` is an instance slug. Multiple parallel instances of
  the same workload may coexist.

**Instance id slug rules** (enforced at parse time by `validate_instance_id`
in `src/microsandbox/slots.rs`; this section is the authoritative contract):

- `id` matches `^[a-z0-9][a-z0-9-]*$`, lowercase, length 1–32.
- `id` MUST NOT be `all` (reserved by `down --all` / `--all-instances`).
- `id` MUST NOT be purely numeric (to avoid ambiguity with `--port-offset N`).
- `id` is case-normalized to lowercase on parse.

Examples: `litellm` (singleton), `personal-litellm` (singleton in context
`personal`), `litellm@canary`, `personal-litellm@canary` (parallel instance).

### 2. Refuse-on-occupied default

`up` / `exec` on a slot that is already occupied **refuses** with a non-zero
exit and a remediation message naming the occupying instance and the three
escape flags. This is the fail-closed default.

The operator opts into one of three explicit behaviors:

| Flag | Behavior on occupied slot |
|---|---|
| *(none)* | **Refuse.** Print the occupying instance id + remediation, exit non-zero. |
| `--replace` | Tear down the occupying instance and start a fresh one on the same slot. Destructive; explicit. |
| `--instance <id>` | Target the parallel slot `<slot>@<id>`. If that parallel slot is occupied, refuse (apply the same rule recursively). |
| `--new` | Synthesize a fresh `<id>` (short random slug, e.g. 4-char base32) and target `<slot>@<id>`. Guarantees a non-colliding parallel instance. |

`exec` on a singleton slot follows the same rule: refuse if occupied, unless
`--instance`/`--new`/`--replace` is given. (Agents are typically interactive;
silent replace of a live agent session is the worst-case destructive action
the old default permitted.)

### 3. `down` variants

| Command | Behavior |
|---|---|
| `workestrate <name> down` | Stop the singleton slot's instance (refuse if the slot has parallel instances — name them). |
| `workestrate <name> down --instance <id>` | Stop the parallel instance `<slot>@<id>`. |
| `workestrate <name> down --all-instances` | Stop the singleton AND every parallel instance of `<name>`. Destructive; explicit. |
| `workestrate down --all` | Stop every running workestrate sandbox across all workloads/contexts. Destructive; explicit; confirms unless `--yes`. |

### 4. `ps`

| Command | Behavior |
|---|---|
| `workestrate ps` | List running workestrate sandboxes for the active context (singleton + parallel instances). |
| `workestrate ps --json` | Same, as a JSON array (see §7 for the shape). |
| `workestrate ps --all-contexts` | List across all contexts (still scoped to this host + state dir). |

`ps` reads the port-registry state files at
`${state_dir}/var/run/<instance>.json` (the existing `port_registry.rs`
store). Stale state files from crashed sandboxes are reported as `stale: true`
with a remediation hint (`workestrate <name> down --instance <id>` or
`--all-instances`).

### 5. `--port-offset` for port-publishing instances

A parallel instance would collide with the singleton's host ports by default.
`--port-offset N` (non-negative integer) shifts **host** ports by `+= N` for
the duration of that `up`/`exec` invocation. **Guest ports are unchanged.**

Semantics:

- For each `[[workloads.<name>.ports]]` entry, the published host port becomes
  `host + N`. The guest port stays `guest`.
- `N = 0` is the default and means no shift (singleton behavior).
- The shifted host port is checked against the port registry for collisions
  (same mechanism as `check_port_collisions` in `port_registry.rs`).
- `--port-offset` is only meaningful for workloads that publish ports. For
  agents (no `ports`), it is accepted but a no-op (with an INFO log).
- `--port-offset` is per-invocation as a CLI flag, but the effective offset
  IS persisted in the port-registry record (`port_offset: Option<u16>` field)
  so that `down`/`logs`/`ps` recover it without re-passing the flag. Offset 0
  serializes as `None` (legacy/default records).

### 6. `--json` output modes

Machine-readable output for AI-native / programmatic operation. `--json` is
accepted on: `ps`, `plan`, `validate-config`, `check`, `config list`,
`source list`, and the refuse/error envelope for `up`/`exec`/`down`.

- **Success:** the command's natural JSON shape (see §7).
- **Refuse / error:** a uniform error envelope (see §7) with `kind`
  (`"refuse_occupied"`, `"port_collision"`, `"missing_secrets"`,
  `"config_error"`, …), a human-readable `message`, and a `remediation`
  object with the suggested flags. Exit code is non-zero.

### 7. JSON output shapes

```jsonc
// spec-test: skip
// `workestrate ps --json` — array of instance records
[
  {
    "instance": "personal-litellm",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "singleton",
    "started_at": "2026-07-20T14:03:11Z",
    "ports": [{"host": 4000, "guest": 4000}],
    "stale": false
  },
  {
    "instance": "personal-litellm@canary",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "parallel",
    "started_at": "2026-07-20T14:05:42Z",
    "ports": [{"host": 14000, "guest": 4000}],
    "port_offset": 10000,
    "stale": false
  }
]
```

```jsonc
// spec-test: skip
// Error / refuse envelope (uniform across commands)
{
  "kind": "refuse_occupied",
  "message": "slot 'personal-litellm' is occupied by instance 'personal-litellm'",
  "slot": "personal-litellm",
  "occupying_instance": "personal-litellm",
  "remediation": {
    "replace": "workestrate litellm up --replace",
    "instance": "workestrate litellm up --instance <id> [--port-offset N]",
    "new": "workestrate litellm up --new [--port-offset N]",
    "list": "workestrate ps --json"
  }
}
```

### 8. `generate-schema` + committed schema + CI drift guard

`workestrate generate-schema` prints the JSON Schema for `workestrate.toml`
to stdout, derived from the same `serde`/`schemars` types the config loader
uses (`JsonSchema` is derived on `ConfigFile` and all sub-structs;
`generate-schema` calls `schemars::schema_for!(crate::config::ConfigFile)`).

- The generated schema is committed at
  `schemas/workestrate.schema.json` and regenerated by a
  `just generate-schema` recipe.
- A CI drift guard (mirroring ADR 0020 Ruling 4's `spec_examples_parse`
  pattern) regenerates the schema in a temp file and diffs against the
  committed copy. Drift fails `just verify`. This catches the class where a
  `serde` struct change ships without a schema update.
- The schema is the contract for config-repo CI: a config repo can run
  `workestrate validate-config` (already exists) OR validate its
  `workestrate.toml` against the committed schema directly with any
  JSON-Schema validator.

### 9. taplo `#:schema` editor integration

Users wiring editor validation in their config repo add a top-level schema
pointer to their `workestrate.toml`:

```toml
# spec-test: skip
#:schema https://raw.githubusercontent.com/georgrybski/ai-workbench/main/schemas/workestrate.schema.json
schema_version = 1
# …rest of file
```

taplo (and any editor using taplo as the TOML language server — VS Code,
Helix, Neovim via LSP) reads the `#:schema` comment and validates the file
against the published schema in real time. For air-gapped / local-first
workflows, the same pointer can reference a vendored copy:

```toml
# spec-test: skip
#:schema ../schemas/workestrate.schema.json
```

The schema pointer is a taplo convention (not a TOML standard) and is
ignored by the `workestrate` config loader — it is a comment. The
`spec_examples_parse` guard (ADR 0020) already skips `#`-prefixed lines, so
the pointer does not interfere with the spec-code CI guard.

## Options considered

1. **Always-replace (status quo).** `up`/`exec` silently tears down the
   existing instance. Rejected: destroys canaries, makes blue-green
   workflows impossible, and is the single most foot-gun-y default for an
   AI agent driving the lifecycle (an agent that runs `up` twice in a
   validation loop nukes its own baseline).
2. **Fail always (no replace, no parallel).** `up` on an occupied slot is a
   hard error with no escape. Rejected: removes the legitimate replace
   use case (operator knows the slot is occupied and wants to recycle it)
   and forces every operator into `down && up` two-step rituals.
3. **Auto-replace with `--instance` for parallel.** Default = replace, opt
   out with `--instance`/`--new`. Rejected: inverts the safety polarity.
   The destructive action (replace) should be explicit, not the default;
   this is the same reasoning that makes `git push --force` opt-in.
4. **Refuse-on-occupied default with `--replace`/`--instance`/`--new` (selected).**
   Fail-closed by default; destruction is explicit; parallel instances are
   first-class; blue-green is native (`up --new --port-offset N` → smoke →
   `down --instance <old>` or `up --replace`). Superior for human operators
   and the only sane default for an AI agent that should not destroy state
   it cannot reconstruct.

## Consequences

- **BEHAVIOR CHANGE.** `up`/`exec` on an occupied slot now **refuses**
  (was: silent replace). This is a migration note for existing users:
  scripts that relied on `up` as an idempotent restart must add `--replace`
  (or `down && up`). The refuse error envelope includes the exact
  `--replace` invocation in `remediation`, so the fix is one flag.
- The port registry (`port_registry.rs`) becomes the source of truth for
  "what is running"; `ps` is its read surface. Stale-state handling is
  documented (not hidden).
- `--port-offset` enables side-by-side execution of the same workload from
  the same context, which was previously a hard error. This resolves the
  deferred `--port-offset` open item (see `70-open-items.md`).
- `generate-schema` + the committed schema + the drift guard close the
  spec-code drift class for the config schema itself (ADR 0020 closed it
  for spec examples; this closes it for the schema).
- `#:schema` editor integration makes config-repo authoring self-validating
  without running `workestrate validate-config` on every save.
- `--json` makes the CLI scriptable by agents and CI without screen-scraping.
- Multi-context batch remains deferred (ADR 0019); `--instance`/`--new` are
  single-context parallelism, not multi-context merge.

## Rejected why

- **Always-replace:** foot-gun for agents and canary workflows; the
  destructive action must be explicit.
- **Fail always:** removes the legitimate recycle use case; forces
  `down && up` rituals.
- **Auto-replace + opt-out:** inverts safety polarity; `--force`-style
  defaults are wrong for a lifecycle tool.
- **Hand-maintained schema (vs schemars-derived):** rejected for the same
  reason ADR 0020 Ruling 4 introduced `spec_examples_parse` — drift
  between code and docs is the failure mode the guard exists to catch,
  and a hand-maintained schema reintroduces it at the schema layer.
  (Resolved: the impl derives `JsonSchema` cleanly via `schemars`; the
  hand-maintained-schema fallback was not needed. The drift guard remains
  load-bearing for catching schema/code drift.)
