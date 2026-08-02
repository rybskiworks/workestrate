# Beads (bd) Procedures — workestrate

Beads tracks **WORK** (actionable issues, claims, status). Docs track **STATE** (STATUS.md = narrative snapshot; NEXT-SESSION.md = resumption context). Boundary: if it's a task someone could pick up → beads issue; if it's context/decisions/progress narrative → docs. Never duplicate: docs reference issue IDs, beads references doc paths; neither restates the other's content.

## Setup facts
- `bd` currently via `nix shell nixpkgs#beads` (ephemeral; currently bd 1.0.3); devshell integration is planned (serialized behind in-flight cleanup phases). Binary source is irrelevant to state: any `bd` (native install, devshell, nix shell) operates on the same `.beads/` workspace.
- Backend: embedded Dolt at `.beads/embeddeddolt/` (untracked). Tracked: `.beads/{config.yaml,metadata.json,.gitignore}` — nothing else may be committed from `.beads/`.
- Issue prefix: `wrk`. Sync: git ref `refs/dolt/data` on origin via `bd dolt push` / `bd dolt pull` (host checkout is the canonical pusher; origin is the host-local repo path, so all pushes happen host-side with user approval).
- Version discipline: keep all binaries on the same minor version across checkouts; the DB schema migrates one-way and older binaries refuse newer DBs.

## Sync discipline
- **Session start (any checkout):** `git pull` then `bd dolt pull` (when a remote dolt ref exists).
- **Session end / before switching machine:** `bd dolt push` — **push requires user approval**; default: the user (or host session) runs the push on the host checkout. Container sessions propose; they do not push autonomously.
- Never run raw `dolt` CLI against `.beads/`. Never enable `dolt.auto-push`.
- `bd dolt pull` refuses with a dirty working set → `bd dolt commit` first.
- Merge conflict or `cannot merge ... different primary keys` → STOP; the host clone is canonical — re-`bd bootstrap` the other clone (export local issues first if unpushed work exists).

## Single-writer rules
- One `bd` writer per checkout at a time (file lock). Concurrent agent sessions in the SAME checkout must serialize bd calls; sessions in DIFFERENT checkouts (host vs container vs sibling clones) work freely and converge via `refs/dolt/data`.
- Microsandbox microVMs never run `bd` — they are runtime sandboxes, not dev environments.

## Daily loops by role
- **Orchestrator / lead:** session start: `bd dolt pull`, `bd ready` → choose work; create/refine issues (`bd create --title ... --type task|epic`); assign and order with `bd dep add <child> <parent>`; end: push proposal + update STATUS.md/NEXT-SESSION.md pointers to issue IDs.
- **Worker (incl. parallel implementation agents):** claim, don't create: `bd ready --claim <id>` (atomic/idempotent). Work the claimed issue; `bd update <id> --status in_progress`; close with `bd close <id> --reason "<validation tags + evidence>"`. Blocked mid-task → file ONE new issue for the blocker and `bd dep add`; don't batch-create.
- **Tester:** open issues for confirmed defects with repro in the body; verify `bd close --reason` carries validation evidence before accepting a close.
- **Parallel-work rule (current Phase 0–4 situation):** the orchestrator owns issue creation for phase/epic structure; the implementation agent claims existing issues only. Duplicates are closed `bd close <id> --reason "duplicate of wrk-NNN"`.

## Conventions tie-in
- Commits: conventional commits referencing issue IDs, e.g. `fix(agentctl): handle stale msb lock (wrk-42)`.
- Validation: `--reason` on close carries the validation tag, e.g. `validated: just verify green @ <sha>` (or partially_validated/not_validated with the env tag no-KVM/HOST-KVM/HOST-NIX).
- STATUS.md/NEXT-SESSION.md: keep their current format; their task lists become links to `wrk-*` IDs; narrative state stays in docs.
- **Icebox / deferred:** `bd update <id> --defer` (or label `icebox`); deferred items stay OUT of `bd ready` and are revisited at phase boundaries via `bd list --deferred`. STATUS.md records WHY deferred.

## Upgrades
- bd upgrades = binary change only; nothing in $HOME. Before switching binary versions on any clone: `bd dolt push` everywhere with the OLD binary.
- Multi-clone migration: HOST is the designated migrator (`bd export --all` backup → new binary → `bd migrate` → `bd dolt push`); other clones then `bd bootstrap`. Then `bd doctor` on both.

## Troubleshooting
- `bd doctor` / `bd doctor --fix` first. Version skew: `bd version` must match across checkouts. Ref missing: `git ls-remote origin | grep dolt`.
