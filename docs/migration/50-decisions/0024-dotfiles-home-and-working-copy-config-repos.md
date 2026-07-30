# ADR 0024: Dotfiles-style home repo + working-copy config repos

**Status:** Accepted
**Date:** 2026-07-30
**References:** ADR 0007 (dotfiles-registry intent — completed here),
ADR 0008 (config repos via uniform `config add` — term of art), ADR 0010
(source overrides; `sources/` collision), ADR 0023 + its 2026-07-30 addendum
(single tool home; discovery removed),
`docs/validation-and-improvements/06-improvements/10-config-repos-as-working-copies.md`
(the executing spec, now EXECUTED).

## Context

ADR 0023 established the single tool home `~/.workestrate`; the 2026-07-30
addendum removed repo-local homes + discovery, leaving one user-global home.

The prior two-copy model (a standalone "canonical" checkout at e.g.
`~/Development/workestrate-personal` PLUS a never-edit managed clone inside the
home) imposed a sync dance and a standing "which copy is real" ambiguity. The
ADRs never mandated read-only home clones; the "never edit home clones" gloss
was an orchestrator convention, not an ADR invariant.

Home clones are full git repos — there is no technical reason they cannot be
worked in.

ADR 0007's dotfiles model intended the registry to live in the user's
dotfiles; until now the home was not itself versioned.

## Options considered

1. **Keep the two-copy model (status quo)** — a standalone canonical sibling
   clone plus a never-edit managed clone in the home. Rejected: sync dance +
   "which copy is real" ambiguity; the never-edit gloss was convention, not
   invariant.

2. **Auto-git-init the home** — silently create `.git` + hooks on first use.
   Rejected: the 5-point rationale in Decision (c).

3. **`--path` on `home init`** — a one-off path flag for selecting the target
   home. Rejected: a third home-selection mechanism that rots against the
   global `--home` flag (spec 06).

4. **Working-copy config repos + explicit-init dotfiles home (selected)** —
   home clones ARE the working repos; the home itself is an explicit-init git
   repo; `repos/` renamed `config-repos/`.

## Decision

(a) **Home config-repo clones are WORKING COPIES.** The user edits, commits,
pushes, and branches directly in `$WORKESTRATE_HOME/config-repos/<name>/`. The
REMOTE is canonical (gitops) — push early/often; `workestrate config update
<name>` is how changes made elsewhere arrive (fetch + fast-forward). A
local-only repo (no remote) works but is a documented SOLE-COPY risk — give it
a remote when it matters. (Today's reality: `config-repos/personal` is
local-only, no remote.)

(b) **The home ITSELF is a git repo** (completes ADR 0007's dotfiles model): it
tracks `config.toml` + `overrides.toml`; `.gitignore` excludes
`/config-repos/`, `/sources/`, `/state/`, `/cache/` plus secret-material
patterns (`*.agekey`, `age.txt`, `*.pem`, `id_rsa*`, `.env`); `*.enc` age
ciphertext stays committable. A pre-commit hook rejects gitlinks (mode 160000),
staged store-dir paths, and secret material. Landed: root commit `a42e597`.

(c) **NEVER auto-git-init the home.** The full rationale:

1. The home is created IMPLICITLY on first use — a silently-created `.git`
   leaks into unrelated tooling (shell prompts show repo state, `git status`
   lists runtime files, backup/dotfiles scanners treat the home as a repo);
   silent state transitions erode CLI trust.
2. Multiple homes exist (real home, experiment homes, dogfood-driver home, CI
   homes) — auto-init cannot tell them apart and would git-init throwaway
   homes; explicit init doubles as the user DECLARING "this is my real,
   persistent home".
3. Hooks are executable code fired by the USER's git commands — silently
   installing a pre-commit hook is what hostile tooling does, and a
   silently-installed hook that starts rejecting commits looks like breakage;
   the hook is the most valuable part of the scaffolding and precisely the
   part that needs consent.
4. No good implicit trigger exists — on-home-creation fires when most first
   homes are experiments; on-every-command polling is ambient and
   unpredictable; `migrate-home` already has one job.
5. Established idiom for this tool class: `chezmoi init`, `git init`,
   `pass init` — every comparable tool makes repo creation an explicit verb.

Division of labor: AUTOMATIC = everything the tool needs to function (home
layout, registry, clones, state); EXPLICIT = everything encoding user workflow
intent (git repo, remote, hooks, branches). Home init is therefore explicit,
idempotent over populated homes (adds only the git layer), and refuses only on
a conflicting pre-existing `.git`. Landed: `3894fb7`.

(d) **`home init` takes NO `--path`.** It operates on the RESOLVED home —
selection stays singular: `WORKESTRATE_HOME` today, the global `--home` flag
once spec 06
(`docs/validation-and-improvements/06-improvements/06-config-home-flag.md`) is
implemented. Composition (`workestrate --home <dir> home init`) is the
idiomatic pattern; a one-off `--path` would be a third "which home" mechanism
that rots against the global flag. Experiment/throwaway homes do NOT need init
— the layout auto-materializes on first use; init is only for homes the user
wants VERSIONED.

(e) **`repos/` → `config-repos/` rename.** `repos/` collided conceptually with
`sources/` (agent source-override checkouts, ADR 0010); `config-repos/`
matches ADR 0008's term of art and the `[configs.<name>]` registry section.
Landed: `d7c5a83`.

(f) **`config update` MUST be dirty-safe.** It bails on uncommitted changes
("commit or stash first") rather than clobbering a working copy — already
implemented at `config_cmd.rs:540-544` and pinned by regression test
`bd99481`.

## Consequences

- Mount model for dev-agent containers: the home is mounted READ-ONLY at the
  default guest path PLUS a nested `config-repos/` READ-WRITE shadow mount
  (the spec 01 nested-shadow pattern), so agents read the registry/layers ro
  while committing/pushing config-repo edits via the rw shadow.
- `WORKESTRATE_CONFIG_DIR` remains the ephemeral live-read override (foreign
  configs, tests, golden fixtures) — it is NOT a working-copy mechanism and is
  unaffected by decisions (a)/(b).
- The two-copy sync dance is eliminated; one copy at a standardized location;
  the remote is the source of truth.
- Local-only config repos carry sole-copy risk until given a remote
  (documented).
- Home history becomes auditable (registry changes are commits); the
  pre-commit hook makes the gitlink/secret failure modes unreachable by
  accident.

## Rejected why

- Standalone canonical sibling clone (the two-copy model): sync dance + "which
  copy is real" ambiguity; the never-edit gloss was convention, not invariant.
- Auto-git-init (any implicit trigger): the 5-point rationale in decision (c).
- `--path` on `home init`: a third home-selection mechanism that would rot
  against the global `--home` flag.
- Alternative dir names `configs/` (ambiguous vs `config.toml`), `layers/`
  (names the role not the content), `workloads/` (too narrow).

---

**Implemented:** Home adopted into git as root commit `a42e597` (tracks
`.gitignore` + `config.toml` only); pre-commit hook installed (rejects
mode-160000 gitlinks, staged store-dir paths, and secret material).
`repos/` → `config-repos/` rename landed (`d7c5a83`). Dirty-safe `config
update` regression test landed (`bd99481`). `workestrate home init` landed
(`3894fb7`) — no `--path`, idempotent over populated homes, refuses only a
conflicting pre-existing `.git`. Spec 10
(`docs/validation-and-improvements/06-improvements/10-config-repos-as-working-copies.md`)
executed these decisions and is EXECUTED as of 2026-07-30.
