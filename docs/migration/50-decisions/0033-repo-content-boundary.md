# ADR 0033: Repo content boundary: project conventions in-repo, general knowledge agent-supplied

**Status:** Accepted
**Date:** 2026-08-24
**References:** spec 24
(`docs/validation-and-improvements/06-improvements/24-repo-content-boundary-migration.md`
— primary execution spec), knowledge repo `MANIFEST.md`
(`agent-workbench/knowledge/MANIFEST.md` — extraction record + full
classification), the 2026-08-23 knowledge-and-skills-extraction handover
(`agent-workbench/handovers/2026-08-23-knowledge-and-skills-extraction.md`),
ADR 0016 (additive migration precedent).

## Context

The repo accumulated **general language/framework knowledge** alongside
project conventions: the `docs/{beam,elixir,gleam,rust,nix,litellm,testing,
verification}` trees plus `languages.md`, and **255 generic skills** in
`.agents/skills/` covering general language use (beam/elixir/gleam/rust/nix),
generic linting constraints, generic validation recipes, and full
per-language workflow families. None of that is workestrate-specific; every
project would carry it identically.

The user's framing (2026-08-24, quoted): "The repo should enforce how the
repo works but not how the agent achieves that... at most all that is here
should be about the project and its deps, and the specific SDLC we'll be
using, and the agent should have docs about overarching things (like the
langs) supplied externally. We need to document some spec to fix this."

The extraction program already built the destinations, destination-side
only: the **knowledge repo** (`agent-workbench/knowledge`, branch main) holds
the generic docs trees copy-only refreshed 2026-08-23 and byte-identical to
workestrate per its `MANIFEST.md`; the **skills repo**
(`agent-workbench/skills`, single commit `b4f5ada`) holds the 255 generic
skills copied from `.agents/skills` @ `36a8be2`. But **no boundary RULE
existed**, and the workestrate source side was never cleaned — no deletions
were executed (the 2026-08-23 handover records the extraction as
destination-side only).

The **beads case** makes the gap concrete: `BEADS.md` mixes **project
conventions** (the `wrk-` prefix, the work/state boundary — beads tracks
WORK, docs track STATE — sync discipline via refs/dolt/data, single-writer
rules, daily loops by role, conventions tie-in) with **tooling mechanics**
(binary sourcing `nix shell nixpkgs#beads`, bd 1.0.3 version discipline, the
upgrades procedure, troubleshooting commands) that were never given any
configured home: no beads skill exists anywhere agent-side (absent from both
the skills repo and `~/.config/opencode/skills`).

> NOTE 2026-09-18: the parenthetical tooling description is stale as written.
> The CLI is now the pinned Beads 1.2.2 via `just beads` (see `BEADS.md` and
> `.beads/README.md`); `nix shell nixpkgs#beads` (bd 1.0.3) must not be used.
> The boundary point stands — mechanics belong agent-side, conventions stay
> in-repo — but read the version/binary details as historical.

## Options considered

1. **Keep everything in-repo.** REJECTED: agent-mechanics and general
   knowledge leak into every project's boundary; the repo ends up
   prescribing how the agent works, not just how the repo works, and every
   consumer of the repo inherits knowledge it neither owns nor maintains.
2. **Move everything non-code out.** REJECTED: project conventions, ADRs,
   OUR-config dependency docs, and the project's own validators ARE how the
   repo works — stripping them would gut the repo's self-description.
3. **Classification rule + split with staged migration.** SELECTED: one
   reviewable rule classifies every artifact; migration executes in phases
   via spec 24, against destinations that already exist.

## Decision

### The principle

In the user's framing: the repo enforces **how the repo works** but not
**how the agent achieves that**. Restated: the repo carries **project
conventions, decisions, architecture, and the project's SDLC**; it does not
prescribe **tooling mechanics, installation, or general
language/framework knowledge** — those are supplied externally by the agent
environment.

### The classification rule

- **Repo:** "how does THIS project work / what did we decide / what
  conventions do we enforce".
- **Agent environment:** "how does X work in general / how does an agent
  operate X" — the **knowledge repo** for docs, the **skills repo** for
  workflows/shims.

Examples in both directions:

- `docs/rust/*` (general rust knowledge) → **agent-side**;
  `docs/migration/50-decisions/*` (our decisions) → **repo**.
- A `rust-error-handling` skill → **agent-side**; `validation-nix-build`
  (runs THIS repo's gates) → **repo**.
- The litellm upstream corpus → **agent-side**;
  `docs/litellm/workestrate-recommended-patterns.md` → **repo**, because
  litellm is a project dependency and that file encodes OUR configuration
  decisions.

### The canonical case: beads

- **Conventions stay in-repo:** `BEADS.md` keeps the `wrk-` prefix, the
  work/state boundary, sync discipline, and the single-writer rules.
- **Mechanics move out:** install/run/version/troubleshooting → a new
  agent-side **beads skill** authored in the skills repo.
- **Recorded gap:** the tooling was never configured anywhere agent-side;
  the move fixes that by giving it a real home.

### Inventory and migration table

Full enumeration lives in spec 24 and knowledge `MANIFEST.md`; summary by
family:

| Artifact | Classification | Destination |
|---|---|---|
| `docs/{beam,elixir,gleam,rust,typescript}/` trees | general knowledge | knowledge repo `docs/` |
| `docs/testing/property-based-testing`, `docs/verification/rust`, `docs/languages.md` | general knowledge | knowledge repo `docs/` |
| `docs/nix/` generic files | general knowledge | knowledge repo `docs/nix/` (project files STAY — below) |
| `docs/litellm/` upstream corpus | general knowledge (upstream) | knowledge repo `docs/litellm/` (OUR-config pair STAYS — below) |
| 255 generic skills (`beam-*`, `gleam-*`, `elixir-*`, `rust-*`, `nix-*` language, `litellm-*`, `constraint-*`, `validation-*`, 156 `workflow-*`) | agent operation | skills repo (already imported @ `b4f5ada`; source-side deletion only) |
| `BEADS.md` | SPLIT | conventions STAY in-repo; mechanics → new beads skill in the skills repo |

**STAYS (project truth):** all ADRs and migration specs; `docs/migration/`;
`docs/mount-policy/`; `docs/validation-and-improvements/`; top-level
`docs/{adr,gaps,testing,integration-plan,secrets,nix-store-accumulation-report}.md`;
`docs/nix-purity.md` (the project's own purity-rules enforcement doc) plus
the project files within `docs/nix/` (`overview.md`, `ci-cd-integration.md`,
`validation.md`, `nix-commands.md`, `workflows/{hardening,validation,
packaging}.md`, `log.md`);
`docs/litellm/{workestrate-recommended-patterns,validation-report}.md`; the
**34 project-specific skills** (`nix-usage`, `constraint-nix-purity`,
`constraint-litellm-in-memory-no-db`, `validation-nix-{build,eval,
flake-check,format,lint,supply-chain,test}` (7),
`workflow-nix-hardening-00..05` (6), `workflow-litellm-config-change-00..05`
(6), `workflow-litellm-hardening-00..05` (6),
`workflow-litellm-validation-00..05` (6)); and the devshell / justfile /
`scripts/` / `flake.nix` build+test tooling (the PROJECT needs it;
agent-workflow tooling is out of bounds). Recorded fact:
`validation-litellm-config-check` no longer exists anywhere (deleted
source-side earlier; its remaining references are known-dangling per
knowledge `MANIFEST.md` §8a) — it is not part of either set.

### Agent-side homes

The canonical agent-side homes are the **knowledge repo** (docs) and the
**skills repo** (workflows/shims), both git repos on the shared filesystem
(container `/home/node/...` == host `/home/rybski/...`).
`~/.config/opencode/skills` is container-local and is **NOT** a canonical
home.

### Execution

Via **spec 24**. This round is spec-only: **no moves are executed**; the
migration executes on the user's go, phase-gated per spec 24.

## Consequences

**Positive:**

- **A single boundary rule, enforceable at review time** — new content is
  classified on entry by one question: is this how THIS repo works, or how
  an agent works in general?
- **Agent-side homes exist and are committed** — the knowledge and skills
  repos are real git repos with the content already imported; the migration
  is a cleanup, not a construction project.
- **Beads mechanics get a real home** — the never-configured tooling gains
  a configured agent-side skill instead of in-repo leakage.
- **Repo docs shrink to project truth** — what remains is conventions,
  decisions, architecture, and OUR dependency configuration.

**Negative / costs:**

- **Git history does not follow files across repos** — moves lose per-file
  history in the new home (the knowledge/skills imports are fresh trees).
  Acceptable; recorded here.
- **Pointer strategy discipline** — staying files that reference moved
  artifacts must be rewired to sibling-relative pointers
  (`../../knowledge/...` / `../../skills/...`, depth-adjusted per file) or
  `(moved to the agent-side knowledge repo)` annotations; **no dangling
  references may be introduced** (the knowledge/skills repos already use
  this convention; pre-existing known-dangling references are recorded in
  knowledge `MANIFEST.md` §8a and are out of scope).
- **The boundary is enforced at REVIEW time**, not by tooling — new content
  is classified on entry; there is no lint gate.
- **Sync question stays open** — the workestrate↔knowledge/skills
  consumption mechanism is undecided (§ Open questions).

## Open questions

- **(a) Generic litellm skills: move or stay?** Litellm is a project
  dependency. **RECOMMENDATION recorded:** generic upstream litellm
  knowledge moves; project-config-specific skills/docs stay (the OUR-config
  boundary already applied to the docs pair and to
  `workflow-litellm-config-change/hardening/validation`).
- **(b) Consumption mechanism** — how workestrate consumes the knowledge
  and skills repos: subtree, submodule, or sibling-relative-only. Carried
  from the 2026-08-23 extraction follow-ups.
- **(c) Beads skill home** — the skills repo (RECOMMENDED: shared) vs a
  host-global skills dir.
