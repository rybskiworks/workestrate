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

## Explicit remote synchronization

Native Dolt history is shared through the existing private Workestrate Git
repository. Dolt uses `refs/dolt/data`, separate from code branches and tags;
an ordinary code push does not publish database changes. The original checkout
uses the Dolt remote name `workestrate`:

```sh
just beads dolt remote list
just beads backup sync
just beads dolt commit -m 'chore: preserve tracker updates'
just beads dolt push --remote workestrate
```

The corresponding remote URL is
`git+ssh://git@github.com/rybskiworks/workestrate.git`. Remote configuration is
clone-local. Register that explicit name only when it is missing:

```sh
just beads dolt remote add workestrate git+ssh://git@github.com/rybskiworks/workestrate.git
```

Use an explicit remote name. In Beads 1.2.2, adding `origin` or letting a bare
`dolt push` adopt Git origin can also stage and commit `config.yaml`. The
explicit `workestrate` registration does not make a Git commit. Keep automatic
push disabled and do not install Beads Git hooks over existing validation hooks.

For a **fresh clone with no local database**, inspect the bootstrap plan before
restoring native history:

```sh
just beads bootstrap --dry-run --json
just beads bootstrap --yes
just beads context --json
just beads vc status --json
just beads branch --json
just beads list --all --json
just beads dolt remote list
```

Bootstrap discovers `refs/dolt/data` through Git origin and normally names its
Dolt remote `origin`; use `--remote origin` in that clone. Do not bootstrap or
pull over unpublished local work. Stop writers, take a native backup and an
issue export, and compare histories in a separate checkout before integrating
another writer's changes. Never resolve divergence with a force push or by
deleting the database without an explicit recovery decision.

The first publication was verified by native local-backup restoration and a
separate empty checkout bootstrapped from the private remote. Both recovered
the same 70 issue records byte-for-byte, the same project identity, and the same
sole `main` branch and content-addressed Dolt commit. This is a historical
verification result, not a fixed expected issue count for future clones.

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

## Continuing cross-repository work

Start with `just beads ready --json`, then inspect the selected issue and its
prerequisites with `just beads show <id> --json`. Read the owning repository's
instructions and current source before executing historical commands. Claim
only the bounded work being performed; a parent epic is a grouping, not a
reason to block every independent child.

The current ownership map is:

| Area | Tracker scope | Implementation owner |
| --- | --- | --- |
| Runtime build inputs and fork provenance | `wrk-2n0` | Microsandbox, libkrun, libkrunfw; consumer pins in Workestrate |
| Lifecycle, nesting policy and mount mediation | `wrk-0ew`, `wrk-fwb` | Workestrate and the applicable runtime fork |
| Native services and client adapters | `wrk-lcj.7`, `wrk-lcj.8` | ai-memory source; application configuration and integration tests in the personal fleet |
| OpenCode Go conversation headers | `wrk-lcj.7.12` | LiteLLM workload plugin and tests in the personal fleet |
| Shared image and guest Nix foundations | `wrk-8pl` | nix-tooling, with consumer-owned workload tests |
| Remote builder and signed binary cache | `wrk-lcj.16` | Shared profiles in nix-tooling; builder/cache deployment and integration tests in the fleet |
| External workload imports and independent test catalog | `wrk-lcj`, `wrk-lcj.10` | Generic import behavior in Workestrate; workload-specific packaging/tests in fleet or workload repositories |
| SSH custody and state migration | `wrk-dcy`, `wrk-d40` | Source fixes plus separately reviewed operator configuration |
| Publication, promotion and recovery | `wrk-847`, `wrk-d06` | Each repository's normal PR flow; native tracker publication separately |

Native two-client capture/recall, built workload images, full Workestrate guest
startup and live deployment are distinct acceptance gates. Keep exact revisions
and remaining coverage in the leaf issue, rather than treating an intermediate
pass as completion of its parent. Likewise, a merged integration PR is not a
promotion to `main`; inspect the actual branch ancestry and checks first.

Remote issue history is recoverable from `refs/dolt/data`; the checked-in JSONL
is its independently published review snapshot. Neither contains application
databases, credentials, Nix store outputs, or local runtime logs. Retain needed
sanitized test reports in their owning repository and rebuild immutable outputs
from their recorded pins. Local operator state requires its own backup and
explicit migration decision.
