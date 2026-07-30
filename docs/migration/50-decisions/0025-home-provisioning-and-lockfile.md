# ADR 0025: Home provisioning (`home init --from`) + `workestrate.lock`

**Status:** Accepted
**Date:** 2026-07-30
**References:** ADR 0023 (home_version, migrate-home precedent), ADR 0024
(the home is a git repo; working-copy config repos),
`docs/validation-and-improvements/06-improvements/03-dogfooding.md` §5.3
(B3 spawn provenance) and the Track C pin-at-spawn item (designated B4; to be
recorded in 03-dogfooding.md when Phase 1 is extended),
`docs/validation-and-improvements/06-improvements/11-home-provisioning-and-lockfile.md`
(the execution spec).

## Context

Homes must be reproducible: dev-home-as-clone, machine rebuild/restore.

The registry records each config repo's `rev` but it is
recorded-not-enforced: the loader reads the live tree, so a reproduced home
gets "whatever main is today", not what the source home actually ran.

Track C dogfooding analysis
(`docs/validation-and-improvements/06-improvements/03-dogfooding.md`) called
for spawn provenance (B3, spec §5.3) and pin-at-spawn (designated B4; to be
recorded in the dogfooding spec when Phase 1 is extended) — both need a
machine-readable pin source.

## Options considered

1. **`--path` for the source** — ambiguous against `--from`'s path value and
   against the rejected `--path` on `home init` (ADR 0024d). Rejected.

2. **`--into <dest>`** — no precedent in the tool; positional dest is the
   git-clone idiom. Rejected.

3. **Positional SOURCE** — creates src/dest ambiguity on two-argument
   invocations. Rejected.

4. **Whole-home `cp -a`** — copies `state/` (ephemeral workspaces, PIDs,
   ports) — actively harmful on a new machine. Rejected.

5. **Image-digest locking in v1** — DEFERRED; the v1 lock pins git revs only;
   OCI image digests are a later evolution via the versioned-migration path.

6. **`home init [--from <src>] [<dest>]` + generated `workestrate.lock`
   (selected)** — positional dest (git-clone idiom), source only via `--from`,
   selective copy (never `state/`), generated lockfile pins url/ref/rev.

## Decision

(a) **INTERFACE:** `workestrate home init [--from <src>] [<dest>]`. `dest` is
POSITIONAL (the `git clone <src> <dest>` idiom). Bare `home init` is unchanged
(ADR 0024d). The source is accepted ONLY via `--from`, so a lone positional
can only ever be a dest — no src/dest ambiguity. `--from` accepts an absolute
path, a relative path (resolved against cwd), or a git URL (reuse the
`looks_like_git_url` classifier at `config/registry.rs:126`).

(b) **SELECTIVE COPY — never whole-home `cp -a`:** the registry is reproduced
via git (the home is a repo, ADR 0024b — clone it); config repos are RE-CLONED
from their registered `url` when reproducible, and COPIED from the source home
when local-only; a remote source containing a local-only config repo produces
an explicit "unreproducible, clone manually" report entry — NEVER a hollow
home. `state/` is NEVER copied (ephemeral workspaces/var). `sources/` is
excluded (re-derivable from source overrides). `secrets/` is created EMPTY
(secrets are machine-local by design, ADR 0018).

(c) **VALIDATION, fail-before-write:** pre-flight checks the source shape
(`config.toml` parses as a `Registry`), `home_version` compatibility, and
produces a reproducibility report — ALL BEFORE writing anything to dest (no
residue on failure). Post-flight runs `validate-config` against the dest home.
`[[trusted_projects]]` entries are carried over with a LOUD per-entry warning
(they contain machine-specific absolute paths that are likely wrong on the new
machine).

(d) **URL/ORIGIN WIRING:** each registry `url` pointing into the SOURCE home's
tree is rewritten to the dest-local copy. The dest home repo's `origin` is
wired to the SOURCE home's repo (pull-based sync is the default direction: dest
pulls from source). Setting `receive.denyCurrentBranch=updateInstead` on the
source home is documented as an opt-in convenience for push-back workflows, not
a default.

(e) **LOCKFILE — new generated file `workestrate.lock` in the home root.** A
typed serde struct with: a lockfile `version` field, `home_version`,
`tool_version`, and `[repos.<name>]` entries pinning `url`/`ref`/`rev`.
WRITERS: `config add`, `config update`, `config remove`, `home init`
(including `--from`). CONSUMERS: `--from` provisioning (checks out the LOCKED
revs — a reproduced home is what the source ran, not "whatever main is today")
and the future `up --pin` / spawn-provenance work (cross-ref
`06-improvements/03-dogfooding.md` B3 + the pin-at-spawn item designated B4 —
the lock is THEIR mechanism; do not build a second one). EVOLUTION: an absent
`version` means the oldest format; a version newer than supported is a hard
error ("home created by a newer workestrate"); format changes go through
explicit migrations (the `migrate-home` precedent, ADR 0023). The registry
(`config.toml`) stays the human-edited file; the lock is GENERATED — never
hand-edited. The lock IS committed to the home repo (lockfiles exist to be
shared, like `Cargo.lock`); it is not in the home `.gitignore`.

## Consequences

- Homes become reproducible artifacts: `home init --from <src>` + the lock
  yields the same registry, the same config-repo revs, empty state/secrets.
- The recorded-but-not-enforced registry `rev` gap is closed by the lock (the
  lock, not the loader, becomes the enforcement point — loader behavior
  unchanged).
- B3/B4 (spawn provenance, pin-at-spawn) have their pin source; no parallel
  mechanism.
- Lock merge conflicts are possible when a home repo is synced across machines;
  they resolve like any generated-lockfile conflict (regenerate, don't
  hand-merge).

## Rejected why

- `--path` for the source: ambiguous against `--from`'s path value and against
  the rejected `--path` on `home init` (ADR 0024d).
- `--into <dest>`: no precedent in the tool; positional dest is the git-clone
  idiom.
- Positional SOURCE: creates src/dest ambiguity on two-argument invocations.
- Whole-home `cp -a`: copies `state/` (ephemeral workspaces, PIDs, ports) —
  actively harmful on a new machine.
- Image-digest locking in v1: DEFERRED — the v1 lock pins git revs only; OCI
  image digests are a later evolution via the versioned-migration path.

---

The execution spec is
`docs/validation-and-improvements/06-improvements/11-home-provisioning-and-lockfile.md`
(READY-TO-EXECUTE; implementation NEEDS-DEVSHELL).

---

## Implemented

**Date:** 2026-07-30

- `172d5dd` feat(agentctl): home init --from provisioning with positional dest — CLI (`--from <src>`, positional `<dest>`, `--config`/`--name` usage-error conflicts), the spec-11 §2 provisioning algorithm (steps 1–7, 9, 10), `looks_like_git_url` made `pub(crate)`, new git helpers (`git_clone_full`, `git_checkout_rev`, remote get/add-or-set-url).
- `19ff272` feat(agentctl): generated workestrate.lock (typed, versioned) — new `config::lockfile` module (`HomeLock`/`LockedRepo`, atomic save, absent lock tolerated, `version` newer-than-supported → "home created by a newer workestrate"); writers wired: `config add` / `config update` / `config remove` / `home init` (bare + `--from`).
- `be356f7` feat(agentctl): home init --from consumes lockfile + version evolution — the locked rev wins over the registry rev (lock url preferred over a drifted registry url), local-only copies honor the lock, legacy no-lock path preserved.
- `d991252` feat(agentctl): global --home flag (spec 06) — same wave (shared CLI/home-resolution surface).

Gates per commit (all green): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full `cargo test` (411 passed at wave end), `just golden-check` / `schema-check` / `spec-examples` / `scaffold-check`, clean `control/agentctl/Cargo.lock`.

Ops verification (2026-07-30): `home init --from ~/.workestrate <scratch-dest>` provisioned a dev home end-to-end — registry cloned (origin → `~/.workestrate`), `config-repos/personal` copied at rev `c41a707` with origin wired to the source repo, registry url rewritten dest-local, `workestrate.lock` written pinning `c41a707`, `state/`/`secrets/` empty, loud per-entry `[[trusted_projects]]` warning, warn-only post-flight `validate-config` passed silently, and `--home <dest> config list` / `--home <dest> litellm plan` both resolved the provisioned home. (Container note: `/home/node/Development` is root-owned, so the scratch dest lived under `/home/node/Development/worktrees/`.)
