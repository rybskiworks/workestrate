# Beads (bd) Procedures — workestrate

Beads tracks **WORK** (actionable issues, claims, status). Docs track **STATE** (STATUS.md = narrative snapshot; NEXT-SESSION.md = resumption context). Boundary: if it's a task someone could pick up → beads issue; if it's context/decisions/progress narrative → docs. Never duplicate: docs reference issue IDs, beads references doc paths; neither restates the other's content.

## Setup facts
- `bd` only via `just beads` from the repo root: the pinned Beads 1.2.2 CLI from shared Nix tooling (`nix run .#beads`, sets `BEADS_DIR` and `DOLT_ROOT_PATH`, disables metrics/event flush; see `justfile` and `.beads/README.md`). Never use another `bd` binary (`nix shell nixpkgs#beads`, a native install, another checkout's shell): changing the CLI can migrate the database schema even when the requested command appears read-only. A `nix shell nixpkgs#beads` binary (stale docs said 1.0.3) is NOT the pinned CLI — that is the exact mixed-binary hazard this discipline guards against.
- Backend: embedded Dolt at `.beads/embeddeddolt/` (untracked; single-writer — serialize tracker commands instead of running concurrent writers). Tracked: `.beads/{config.yaml,metadata.json,README.md}` plus `.beads/issues.jsonl` — the JSONL file is an explicit reviewable export (`just beads export --all -o .beads/issues.jsonl`), not a full database backup. Refresh it explicitly after changing issues and stage it deliberately with issue-related commits; expect commit churn on issue writes. All other `.beads/` content (native database files, local configuration, backup destinations, runtime state) stays untracked. Automatic backup and git push are disabled in `config.yaml`; inspect and refresh the clone-local native backup with `just beads backup status` / `just beads backup sync`.
- Issue prefix: `wrk`. Sync: git ref `refs/dolt/data` on origin via `just beads dolt push --remote workestrate` / `just beads dolt pull --remote workestrate` (host checkout is the canonical pusher; origin is the host-local repo path, so all pushes happen host-side with user approval). Always use the explicit `--remote` name: in 1.2.2, adding `origin` or letting a bare `dolt push` adopt git origin can also stage and commit `config.yaml`. A fresh bootstrap clone names its Dolt remote `origin` — use `--remote origin` there. See `.beads/README.md` for the full remote procedure.
- Version discipline: every checkout uses the pinned 1.2.2 CLI; the DB schema migrates one-way and older binaries refuse newer DBs. These procedures are verified against bd 1.2.2 — recheck flags on upgrade (flags and subcommand shapes may change).

## Sync discipline
- **Session start (any checkout):** `git pull` then `just beads dolt pull --remote workestrate` (when a remote dolt ref exists).
- **Session end / before switching machine:** `just beads dolt commit -m 'chore: preserve tracker updates'` then `just beads dolt push --remote workestrate` — **push requires user approval**; default: the user (or host session) runs the push on the host checkout. Container sessions propose; they do not push autonomously.
- Never run raw `dolt` CLI against `.beads/`. Never enable `dolt.auto-push`. Never install Beads git hooks over the existing validation hooks.
- `just beads dolt pull` refuses with a dirty working set → `just beads dolt commit` first.
- Merge conflict or `cannot merge ... different primary keys` → STOP; the host clone is canonical — compare histories in a separate checkout before integrating, back up (native backup plus issue export) first, and never resolve divergence with a force push, a bootstrap/pull over unpublished work, or by deleting the database without an explicit recovery decision (export local issues first if unpushed work exists).

## Single-writer rules
- One `bd` writer per checkout at a time (file lock). Concurrent agent sessions in the SAME checkout must serialize bd calls; sessions in DIFFERENT checkouts (host vs container vs sibling clones) work freely and converge via `refs/dolt/data`.
- Microsandbox microVMs never run `bd` — they are runtime sandboxes, not dev environments.

## Daily loops by role
- **Orchestrator / lead:** session start: `just beads dolt pull --remote workestrate`, `just beads ready` → choose work; create/refine issues (`just beads create --title ... --type task|epic`); assign and order with `just beads dep add <blocker> <blocked>` (task→task blocks only — epics cannot be blocked; epic hierarchy is set via `just beads update <child> --parent <epic>`); end: push proposal + update STATUS.md/NEXT-SESSION.md pointers to issue IDs.
- **Worker (incl. parallel implementation agents):** claim, don't create: `just beads update <id> --claim` (atomic/idempotent). Work the claimed issue; `just beads update <id> --status in_progress`; close with `just beads close <id> --reason "<validation tags + evidence>"`. Blocked mid-task → file ONE new issue for the blocker and `just beads dep add`; don't batch-create.
- **Tester:** open issues for confirmed defects with repro in the body; verify `just beads close --reason` carries validation evidence before accepting a close.
- **Parallel-work rule (current Phase 0–4 situation):** the orchestrator owns issue creation for phase/epic structure; the implementation agent claims existing issues only. Duplicates are closed `just beads close <id> --reason "duplicate of wrk-NNN"`.

## Conventions tie-in
- Commits: conventional commits referencing issue IDs, e.g. `fix(agentctl): handle stale msb lock (wrk-42)`.
- Validation: `--reason` on close carries the validation tag, e.g. `validated: just verify green @ <sha>` (or partially_validated/not_validated with the env tag no-KVM/HOST-KVM/HOST-NIX).
- STATUS.md/NEXT-SESSION.md: keep their current format; their task lists become links to `wrk-*` IDs; narrative state stays in docs.
- **Icebox / deferred:** `just beads update <id> --defer +1d` (`--defer` requires a duration, e.g. `+1d`; `--defer ""` clears) (or label `icebox`); deferred items stay OUT of `just beads ready` and are revisited at phase boundaries via `just beads list --deferred`. STATUS.md records WHY deferred.

## Upgrades
- Before another upgrade: stop writers, preserve the full `.beads` directory, and rehearse the upgrade on a separate copy. Do not use `bd init --force`, `bd bootstrap`, or a JSONL import to replace the existing database casually — these operations can replace local state, and exports do not contain full Dolt history. Details and the 1.2.2 pin rationale (retracted 1.2.0/1.2.1) are in `.beads/README.md`.
- Multi-clone migration: HOST is the designated migrator (native backup plus issue export → new binary → rehearse on a copy → `just beads dolt push --remote workestrate`); other clones then pull. Then `just beads doctor` on both.

## Troubleshooting
- `just beads doctor` / `just beads doctor --fix` first. Version skew: `just beads version` must report the pinned 1.2.2 on every checkout. Ref missing: `git ls-remote origin | grep dolt`; Dolt remotes: `just beads dolt remote list`.
