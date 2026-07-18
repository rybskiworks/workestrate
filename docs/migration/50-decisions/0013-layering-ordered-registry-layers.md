# ADR 0013: Layering via ordered registry layers + contexts deferred

**Status:** Accepted
**Date:** 2026-07-18

## Context

Phase 3 introduces multi-layer config merging. The mechanism for declaring
layer order and selecting active layers needs to be simple for the current
single-user case while supporting future multi-context use.

## Options considered

1. **Named contexts (kubectl-style) from the start** — `[[contexts.work]]
  layers = ["team", "personal"]`. Rejected: over-engineered for the current
  single-user case with 1-2 layers.
2. **Single ordered `layers` list in registry** — `layers = ["work",
  "personal"]`. Selected.
3. **Separate manifest file (`layers.toml`)** — rejected: another file to
  track; the registry is already the home for config-repo references.

## Decision

Ship a single ordered `layers = [...]` array in the registry
(`~/.config/workestrate/config.toml`). The order is the merge order (earlier
layers overridden by later). Named contexts (`[[contexts.<name>]]`) are
deferred until 3+ layers exist.

## Consequences

- Phase 3 implements the merge engine against the ordered `layers` array.
- `workestrate --context <name>` is deferred (not implemented in Phase 3).
- When contexts are needed, they're added as `[[contexts.<name>]]` with
  `layers = [...]` arrays; the top-level `layers` becomes the default context.

## Rejected why

Named contexts: over-engineered for 1-2 layers. Separate manifest: another
file to track.
