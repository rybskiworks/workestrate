# ADR 0023: Single tool home (WORKESTRATE_HOME)

**Status:** Accepted
**Date:** 2026-07-22
**References:** ADR 0007 (Tool+XDG+dotfiles organization model — its layout is
superseded by this ADR; the tool-as-tool + dotfiles-registry decision stands);
`docs/migration/20-target-system-spec.md` §1;
`docs/migration/70-open-items.md` (container persistence).

## Context

ADR 0007 established the workestrate-as-tool + XDG + dotfiles-registry model.
The tool-as-tool decision (workestrate is a decoupled CLI, not a workspace)
and the dotfiles-registry decision (the registry IS the user's dotfiles entry,
kubeconfig-style) both stand. What this ADR supersedes is the **XDG three-home
layout** that ADR 0007 specified for where those artifacts live on disk.

The XDG three-home model mirrors `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and
`XDG_STATE_HOME` into a repo-local `.workestrate/` wrapper. This produces a
concrete triplication:

```
.workestrate/
├── config/workestrate/config.toml   # XDG_CONFIG_HOME/workestrate/config.toml
├── data/workestrate/                # XDG_DATA_HOME/workestrate/ (repos, sources)
│   ├── repos/
│   └── sources/
└── state/workestrate/               # XDG_STATE_HOME/workestrate/ (workspaces, var)
    ├── workspaces/
    └── var/
```

This is 3× `workestrate` directory nesting (`config/workestrate`,
`data/workestrate`, `state/workestrate`) plus 2× `config` nesting
(`.workestrate/config/` and then `workestrate/config.toml`). The user rejected
this repetition: the `workestrate` segment appears once per XDG home, and the
`config` segment appears twice in the registry path.

The `.envrc` (direnv) and `scripts/local-xdg.sh` had to export **three** XDG
env vars to wire this up:

```
export XDG_CONFIG_HOME="$PWD/.workestrate/config"
export XDG_DATA_HOME="$PWD/.workestrate/data"
export XDG_STATE_HOME="$PWD/.workestrate/state"
```

Three env vars, three nested `workestrate/` dirs, and a doubled `config/` — all
to hold one tool's state. The repetition is accidental complexity with no
user-facing benefit.

## Options considered

1. **Keep XDG three-home (status quo)** — the repetition/triplication persists.
   This is the problem this ADR exists to solve, not a viable option.

2. **Rename bundle only / cosmetic rename** — rename `.workestrate/` subdirs
   (e.g. `cfg/`, `dat/`, `st/`) without collapsing the three-home split.
   Rejected: cosmetic; does not fix the nesting repetition. The three-home
   split itself is the source of the triplication.

3. **Single tool home `$WORKESTRATE_HOME` with flat layout (selected)** —
   collapse the three XDG homes into one tool home directory with a flat
   layout. One env var instead of three. No `workestrate/` nesting inside the
   home (the home IS the workestrate dir). Precedents: `~/.kube`, `~/.docker`,
   `~/.cargo`.

## Decision

Single tool home `$WORKESTRATE_HOME` (default `~/.workestrate`; container:
`<repo>/.workestrate`) with a flat layout:

```
$WORKESTRATE_HOME/
├── config.toml          # registry: tool settings + config-repo registry + layers + trusted_projects
├── overrides.toml       # user-global overrides (was: $XDG_CONFIG_HOME/workestrate/overrides.toml)
├── secrets/             # .env.local.enc (machine-local secrets; was: $XDG_CONFIG_HOME/workestrate/.env.local.enc)
├── repos/               # managed config-repo clones (was: $XDG_DATA_HOME/workestrate/repos/)
├── sources/             # agent source checkouts + builds (was: $XDG_DATA_HOME/workestrate/sources/)
├── state/               # workspaces/, var/ (was: $XDG_STATE_HOME/workestrate/)
└── cache/               # cache
```

### Resolution precedence

1. `WORKESTRATE_HOME` env var (explicit override)
2. Auto-discovery (walk-up from cwd, trust-gated — finds a `.workestrate/`
   in a parent dir), **but only when no `XDG_*_HOME` var is set**. An
   explicit XDG var is a deliberate legacy-layout signal that discovery
   must not override (32 XDG-pinned tests depend on this invariant).
3. Legacy XDG (read-only compat + deprecation note — reads old
   `XDG_CONFIG_HOME/workestrate/config.toml` etc. if present, does NOT write)
4. Default (`~/.workestrate`)

### `home_version` field

A field in `config.toml` (e.g. `home_version = 1`) that tracks the home
layout version for future layout migrations. `workestrate migrate-home`
stamps `home_version = 2` after consolidating into the single home.
Bumping the version signals that a `migrate-home` path exists to upgrade
an older layout to the current one.

### `workestrate migrate-home` command

Migrates a legacy XDG three-home layout into the single home layout. Moves
`config/workestrate/config.toml` → `config.toml`,
`data/workestrate/repos/` → `repos/`, etc. Idempotent; warns on conflicts
(existing files at the target are not silently overwritten). Emits a
deprecation note when legacy XDG paths are detected. After moving, the
relocated registry at `dest/config.toml` has `store_dir`/`state_dir`
cleared (derivation from the new home takes over), `home_version` stamped
to `2`, and `configs.<name>.url` fields pointing into the old layout
rewritten to the new `dest/repos/<name>` path. Remote URLs (`http://`,
`https://`, `ssh://`, `git@`, `flake://`, or any `://` scheme) are left
untouched.

### `.envrc` / `local-xdg.sh` collapse

The three XDG env vars collapse to ONE:

```
export WORKESTRATE_HOME="$PWD/.workestrate"
```

### Auto-discovery trust model

Auto-discovery (precedence step 2) is trust-gated: the repo containing
`.workestrate/config.toml` must be listed in `[[trusted_projects]]` in the
**global base registry** (resolved via Env/LegacyXdg/Default only — never
via discovery itself, so a hostile `.workestrate/` cannot self-trust).
Discovered files are not self-trusting. First-time discovery from a fresh
clone requires `workestrate config trust <repo>` or setting
`WORKESTRATE_HOME`. An untrusted discovery prints a one-time warning and
*stops* walking (does not keep looking higher), then falls through to
LegacyXdg/Default. Discovery is recursion-safe: the trust check reads the
base registry directly (`resolve_home_base_with_kind`), never recursing
through `resolve_home_with_kind`.

### Precedent (live-verified)

- **kubectl** — `~/.kube/` is a single tool home directory. `~/.kube/config`
  holds cluster references (the kubeconfig model ADR 0007 cited). kubectl does
  not split config/data/state across XDG homes.
  [Source: https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/]
- **Docker** — `~/.docker/` is a single tool home directory holding config,
  credentials, and context state in one flat tree.
  [Source: https://docs.docker.com/engine/reference/commandline/cli/#configuration-files]
- **Cargo / Rust** — `~/.cargo/` is a single tool home directory with a flat
  layout: `registry/`, `git/`, `bin/`, `credentials.toml`. Cargo does not use
  XDG's config/data/state split.
  [Source: https://doc.rust-lang.org/cargo/reference/environment-variables.html]

All three chose a single tool home over XDG's config/data/state split. The
XDG spec is a reasonable default for general-purpose tools, but a tool with a
cohesive artifact set (registry + clones + sources + state) benefits from
co-location.

## Consequences

- One env var (`WORKESTRATE_HOME`) instead of three (`XDG_CONFIG_HOME`,
  `XDG_DATA_HOME`, `XDG_STATE_HOME`).
- Flat layout eliminates the `workestrate/` nesting repetition and the
  doubled `config/` segment.
- `home_version` field enables future layout migrations (bump version, provide
  a `migrate-home` path).
- Legacy XDG layout is read in compat mode (read-only + deprecation note
  emitted on first use). Existing users are not broken; they are nudged toward
  `workestrate migrate-home`.
- `workestrate migrate-home` provides the upgrade path from legacy XDG to the
  single home.
- Registry content (`config.toml` schema: `[settings]`, `[configs.*]`,
  `layers`, `[contexts.*]`, `[[trusted_projects]]`) is UNCHANGED — only its
  location moves from `~/.config/workestrate/config.toml` to
  `$WORKESTRATE_HOME/config.toml`.
- Expansion is additive: a new artifact class becomes a new top-level entry in
  the home (e.g. `logs/`, `plugins/`), with no breakage to existing entries.
- `store_dir` and `state_dir` in `[settings]` become home-relative by default
  (`$WORKESTRATE_HOME` and `$WORKESTRATE_HOME/state` respectively) but remain
  overridable for users who want to point them elsewhere.

## Rejected why

- **XDG three-home:** the repetition/triplication persists — this is the
  problem the ADR solves.
- **Cosmetic rename:** renaming subdirs without collapsing the three-home
  split does not fix the nesting repetition; it only hides it behind shorter
  names.

---

**Implemented:** The `.envrc` and `scripts/local-xdg.sh` collapse from
three XDG vars to `WORKESTRATE_HOME` is complete (commit 6a6cece). Both
files now export a single `WORKESTRATE_HOME="$PWD/.workestrate"`.

**WP-F5 (clobber guard + partial-failure reporting):** `workestrate
migrate-home` now scans every planned destination path (not just
`config.toml`) before moving and refuses — listing all pre-existing
destinations — unless `--force` is passed. A pre-flight pass verifies
every source exists and every destination parent is writable before any
move begins. The migration remains non-transactional: if a move fails
mid-loop, entries already moved stay moved, and the summary reports
`partial: true` with `failed_at` naming the destination that could not be
moved and `moved` listing the entries that succeeded up to that point.
