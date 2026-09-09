# Workestrate task tracking

This repository uses the pinned Beads 1.2.2 CLI from shared Nix tooling.
Use the repository entry point from its root:

```sh
just beads version
just beads ready --json
just beads list --all --json
just beads show wrk-d06 --json
```

The entry point selects this repository's `.beads` directory and keeps Dolt's
local configuration under `.beads/dolt-global`. It does not initialize storage,
install hooks, modify repository instructions, or push tracker data. Avoid a
different `bd` binary from an unrelated shell: changing the CLI can change the
database schema even when the requested command appears read-only.

## Storage and recovery

The source of truth is the embedded Dolt database at
`.beads/embeddeddolt/wrk`. Embedded mode is single-writer: serialize tracker
commands instead of running concurrent writers. No separate SQL server is
needed.

Git tracks `config.yaml`, `metadata.json`, this guide, and `issues.jsonl`.
The JSONL file is a reviewable issue export, not a full database backup. Native
database files, local configuration, backup destinations, and runtime state
remain ignored.

Refresh the tracked export explicitly after changing issues:

```sh
just beads export --all -o .beads/issues.jsonl
git diff -- .beads/issues.jsonl
```

The September 2026 adoption preserved the existing native database and both
exports before upgrading. The database had 31 issues and the tracked JSONL had
41: 30 were identical, 11 existed only in JSONL, and `wrk-23b` existed only in
Dolt. Reconciliation retained the complete union of 42 issues and their
existing IDs, content, and dependency links. Native snapshots and original
exports were retained outside this repository. Migration changed some issue
modification timestamps; the original values remain in those snapshots.

Automatic backup and Git push are disabled in `config.yaml`. A local native
backup destination is clone-specific and can be inspected and refreshed with:

```sh
just beads backup status
just beads backup sync
```

Before another upgrade, stop writers, preserve the full `.beads` directory,
and rehearse the upgrade on a separate copy. Do not use `bd init --force`,
`bd bootstrap`, or a JSONL import to replace the existing database casually.
These operations can replace local state; exports do not contain full Dolt
history. [Beads release notes](https://github.com/gastownhall/beads/releases/tag/v1.2.2)
explain why this version is pinned instead of the retracted 1.2.0/1.2.1 releases.

No Dolt remote has been configured for this clone. Adding a remote and publishing
`refs/dolt/data` are explicit operations, separate from ordinary Git pushes.
Do not install Beads Git hooks over this repository's existing validation hooks.

## Issue workflow

Check existing work before creating a duplicate. Relevant existing epics include
`wrk-d06` for tracker integration, `wrk-0ew` for host runtime validation,
`wrk-bvu` for image lifecycle, and `wrk-fwb` for mount policy.

```sh
just beads update <issue-id> --claim
just beads create 'Describe a concrete change' --type task --parent <epic-id> --json
just beads dep add <blocked-issue-id> <prerequisite-issue-id>
just beads update <issue-id> --notes 'Evidence, remaining work, and relevant commit references'
just beads close <issue-id> --reason 'Completed change and concrete verification'
```

Use dependency edges for prerequisites and parent-child edges for grouping.
The existing `decision` and `docs` issue types are explicitly configured.
For fork-family work, identify the owning repository and affected revision in
the issue; keep one authoritative issue instead of copying it into every fork.

Record exact validation commands and outcomes. Distinguish a package build,
unit checks, disposable workload tests, and host-dependent KVM/SSH verification.
Do not close a runtime issue merely because a build passes. Older issues may
describe obsolete branches, commands, or history-rewriting operations: preserve
their history, then update the intended work against current code before acting.

Issue text and Git artifacts should describe the project and its behavior,
without secrets or implementation-session metadata.
