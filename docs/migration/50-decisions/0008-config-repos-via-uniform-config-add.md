# ADR 0008: Config repos via uniform `workestrate config add`

**Status:** Accepted
**Date:** 2026-07-18

## Context

Config repos need to be cloned, pinned, and updated. The mechanism must be
uniform (same for personal, team, project repos) and must not require editing
the parent flake.

## Options considered

1. **Flake inputs + devshell materialization** — config repos as `flake =
   false` inputs, materialized by devshell (like `agents/<name>/repo`).
   Rejected as primary: requires editing `flake.nix` for each config repo;
   personal secrets would be copied to /nix/store (encrypted, but unnecessary
   exposure).
2. **`workestrate config add/update/list` (git clone into managed store)** —
   uniform git clone into `~/.local/share/workestrate/repos/<name>/`, tracked
   in registry. Selected.
3. **Hybrid: team repos as flake inputs, personal as git clone** — considered
   in earlier rounds. Rejected: non-uniform; flake-input editing for team
   repos still required.

## Decision

All config repos consumed uniformly via `workestrate config add <url> <name>
[--ref main]`. The command clones to `~/.local/share/workestrate/repos/<name>/`,
adds an entry to the registry (`~/.config/workestrate/config.toml`), and tracks
the pinned `rev`. `workestrate config update [name]` pulls latest and updates
`rev`.

Flake materialization is optional sugar for team repos (a devshell function
can call `workestrate config update` instead of manual clone), but the
primary mechanism is the uniform `config add` command.

## Consequences

- Adding a config repo is one command, no flake editing.
- The registry tracks URL + ref + rev for each config repo.
- `workestrate config list` shows all repos with dirty status.
- Personal secrets stay out of /nix/store (git clone, not flake input).

## Rejected why

Flake inputs: requires editing flake.nix; secrets in /nix/store. Hybrid:
non-uniform.
