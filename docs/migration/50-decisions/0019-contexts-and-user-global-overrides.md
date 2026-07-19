# ADR 0019: Contexts + user-global overrides

**Status:** Accepted
**Date:** 2026-07-19

## Context

Phase 3 shipped a single ordered `layers` array in the registry (ADR 0013).
Contexts (named layer-sets) were deferred until 3+ layers exist. With the
addition of user-global overrides and user-global secrets, the single-layer
model is no longer sufficient: a user with multiple deployment targets
(personal, work, lab) needs to switch between layer-sets without editing
the registry. Additionally, user-global overrides and secrets need a
well-defined precedence position relative to context layers.

## Options considered

1. **Namespaced-union (prefix-based contexts)** — each context's secrets
   and config get a prefix (e.g. `PERSONAL_LITELLM_MASTER_KEY`). Rejected:
   breaks env-name contracts with consumers (agents expect
   `LITELLM_MASTER_KEY`, not a prefixed variant); requires config changes
   per context; provenance is lost; doesn't compose with the existing
   per-key merge model (ADR 0018). The same rejection rationale as ADR 0018
   option 3 (prefix-based namespacing) applies.
2. **Option A contexts (selected)** — `[contexts.<name>] layers = [...]`
   in the registry. One context resolved per invocation via `--context` flag
   > `WORKESTRATE_CONTEXT` env > `[settings] default_context` > bare-layers
   backward-compat. Selected: preserves env-name contracts; composes with
   the existing merge engine and per-key secrets model; backward-compatible
   (no contexts = current behavior).
3. **Multi-context batch** — resolve multiple contexts in one invocation
   and merge them. Rejected for now: complicates instance naming and port
   collision detection; no current use case. Deferred (see 70-open-items.md).

## Decision

### Contexts

Registry gains `[contexts.<name>] layers = [...]`. Selection precedence:
`--context <name>` flag > `WORKESTRATE_CONTEXT` env > `[settings]
default_context` > if NO contexts defined: bare `layers` (backward compat —
existing registries and golden-check stay byte-identical).

One invocation resolves exactly ONE context; its layers merge per the
existing engine; secrets merge per-key within the context.

### Instance namespacing

Sandbox instance names become `<context>-<workload>` when contexts exist
(bare names preserved otherwise). This prevents port and state collisions
between contexts. Port collision detection via state-dir tracking at
`${state_dir}/var/run/<instance>.json` hard-errors before up/exec when
two workestrate sandboxes claim the same host port.

### User-global overrides

`$XDG_CONFIG_HOME/workestrate/overrides.toml` (optional) with sections:
- `[global]` — ConfigFile fragment applied to every context
- `[configs.<name>]` — ConfigFile fragment applied only when `<name>` is
  in the active context's layers

Precedence: reference < context layers < [global] < [configs.<name>] <
trusted project < project local. Merged with the SAME engine + security
rules. LENIENT semantics: unknown config/workload section → skip + INFO
log; unknown field → WARNING; policy violations → hard error.

### User-global secrets

`$XDG_CONFIG_HOME/workestrate/.env.local.enc` (optional) applied per-key
AFTER the context's domain layers, BEFORE project layers. setup-secrets
gains `--global init|update` targeting this file.

## Consequences

- Existing registries without `[contexts.*]` continue to work unchanged
  (bare `layers` backward-compat).
- `--context` flag and `WORKESTRATE_CONTEXT` env allow switching layer-sets
  without editing the registry.
- Instance namespacing prevents port/state collisions between contexts.
- User-global overrides provide a machine-local customization layer without
  touching config repos.
- User-global secrets provide a machine-local secrets layer for overrides
  that need secret values (e.g., a personal API key for a workload that
  the context layers define but don't provide a value for).
- Port collision detection is best-effort via state-dir tracking; stale
  state files from crashed sandboxes may cause false collisions (remediation
  message tells the user to run `workestrate <name> down`).

## Rejected why

Namespaced-union: breaks env-name contracts; doesn't compose with per-key
merge (same rationale as ADR 0018 option 3). Multi-context batch: no
current use case; complicates naming and collision detection.
