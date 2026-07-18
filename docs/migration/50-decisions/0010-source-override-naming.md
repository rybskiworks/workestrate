# ADR 0010: Source override naming

**Status:** Accepted
**Date:** 2026-07-18

## Context

The mechanism for cloning and building agent source code (currently
`agents/<name>/repo` + `agents/<name>/build`, gitignored, materialized by
devshell) needs a name and command namespace in the tool+XDG model.

## Options considered

1. **"hack"** — `workestrate hack clone/build`. Rejected: unprofessional;
   implies unauthorized modification.
2. **"dev"** — `workestrate dev clone/build`. Rejected: ambiguous with
   "development" generally.
3. **"vendor"** — `workestrate vendor clone/build`. Rejected: `vendor` is
   reserved for frozen third-party deps (microsandbox patched crate, ADR 0011).
4. **"source"** — `workestrate source clone/build/list/reset`. Selected:
   precise, professional, matches the concept (agent source code).

## Decision

- **Concept name**: "source override" (a user's checkout that overrides the
  canonical flake input source).
- **Command namespace**: `workestrate source clone|build|list|reset`.
- **Store path**: `~/.local/share/workestrate/sources/<name>/` (with `repo/`
  and `build/` subdirectories).
- **`WORKESTRATE_<NAME>_BUILD` env var**: unchanged from current behavior
  (`workload.rs:80-90`).
- **`vendor`**: reserved for frozen third-party deps (microsandbox patched
  crate).

## Consequences

- Clear separation: `source` = agent source code; `vendor` = third-party deps.
- `workestrate source list` shows all agent sources with build status.
- `workestrate source reset` discards local edits (re-clones canonical).

## Rejected why

"hack": unprofessional. "dev": ambiguous. "vendor": reserved for deps.
