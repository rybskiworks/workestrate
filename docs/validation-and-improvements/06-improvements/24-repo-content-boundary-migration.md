# 24 — Repo content boundary migration (ADR 0033 execution spec)

> **STATUS: SPEC (docs-only; migration executes on the user's go — no moves executed 2026-08-24)**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [../../migration/50-decisions/0033-repo-content-boundary.md](../../migration/50-decisions/0033-repo-content-boundary.md)
> (ADR 0033 — the boundary decision) · knowledge repo `MANIFEST.md`
> (`agent-workbench/knowledge/MANIFEST.md` — extraction record + full
> classification)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | This spec, its links, indexing, and the docs/link-check validation gates can be validated in this container. |
| host-side | Pushes of the knowledge and skills repos are user-approved, host-side steps (both repos are local-only git, same posture as the personal config repo decision 2026-08-24). |

This is a docs-only spec. No file moves, deletions, or skill edits are
executed by this round; execution begins only on the user's go (Phase 0).

---

## Summary

ADR 0033 draws the repo content boundary: the repo enforces how the repo
works (project conventions, decisions, architecture, SDLC), not how an agent
achieves that (general language/framework knowledge, tooling mechanics). The
extraction program already built the agent-side destinations — the knowledge
repo (generic docs trees, copy-only refreshed 2026-08-23, byte-identical per
its `MANIFEST.md`) and the skills repo (255 generic skills @ `b4f5ada`) — but
the workestrate source side was never cleaned. This spec enumerates the move
list, the pointer/redirect strategy, the validation gates, and the phased
execution that removes the source-side copies and rewires references without
leaving dangling links.

## Move list (source → destination)

### Docs

| Source (workestrate) | Destination | Notes |
|---|---|---|
| `docs/beam/` | knowledge `docs/beam/` | copies present, byte-identical per 2026-08-23 re-baseline; verify with `diff -rq` in Phase 0 |
| `docs/elixir/` | knowledge `docs/elixir/` | same re-verification |
| `docs/gleam/` | knowledge `docs/gleam/` | same re-verification |
| `docs/rust/` | knowledge `docs/rust/` | same re-verification |
| `docs/typescript/` | knowledge `docs/typescript/` | stub tree; same re-verification |
| `docs/testing/property-based-testing` | knowledge `docs/testing/property-based-testing` | same re-verification |
| `docs/verification/rust` | knowledge `docs/verification/rust` | same re-verification |
| `docs/languages.md` | knowledge `docs/languages.md` | same re-verification |
| `docs/litellm/` upstream corpus | knowledge `docs/litellm/` | `00-index.md`, `01-mental-model.md`, `extracted/`, `crawl/`, `config/`, `providers/`, `gateway/`, `routing/`, `endpoints/`, `auth-access-budget/`, `deployment-ops/`, `observability-cache-guardrails/`, `troubleshooting.md`, `examples/`; canonical annotated schemas already live in the personal config repo with a pointer at knowledge `docs/litellm/schemas.md` |
| `docs/nix/` generic files | knowledge `docs/nix/` | all files EXCEPT the project files listed below |

**STAYS (not in the move list):**

- `docs/litellm/workestrate-recommended-patterns.md` and
  `docs/litellm/validation-report.md` — OUR-config docs for a project
  dependency (MANIFEST-flagged project-specific).
- `docs/nix/` project files: `overview.md`, `ci-cd-integration.md`,
  `validation.md`, `nix-commands.md`,
  `workflows/{hardening,validation,packaging}.md`, `log.md` (corpus
  changelog).
- `docs/nix-purity.md` — STAYS whole; it is the project's own purity-rules
  enforcement doc (portable-core extraction is a future generalization
  candidate, same treatment as `docs/secrets.md`, reclassified
  PROJECT-SPECIFIC and returned 2026-08-02).
- All ADRs, migration specs, `docs/migration/`, `docs/mount-policy/`,
  `docs/validation-and-improvements/`, and top-level
  `docs/{adr,gaps,testing,integration-plan,secrets,nix-store-accumulation-report}.md`.

### Skills

| Source (workestrate `.agents/skills/`) | Destination | Notes |
|---|---|---|
| 255 generic skill dirs | skills repo (already imported @ `b4f5ada`) | **source-side deletion only** — no copy step; families: `beam-*` (8), `gleam-*` (3), `elixir-*` (8), `rust-*` (9), `nix-*` language (13: `nix-language`, `nix-flake-anatomy`, `nix-derivations`, `nix-devshells`, `nix-modules`, `nix-nixpkgs-library`, `nix-overlays`, `nix-testing`, `nix-ci-cd`, `nix-store-gc`, `nix-docker-images`, `nix-packaging-recipes`, `nix-cross-compilation`), `litellm-*` (13), `constraint-*` (25), `validation-*` (17: elixir 5, gleam 3, rust 6, litellm 3), `workflow-*` (156: rust/elixir/gleam/beam-runtime-diagnosis/nix/okf + litellm-debugging/config-review). Full named list: knowledge `MANIFEST.md` §3 |

**STAYS (34 project-specific skills):** `nix-usage`,
`constraint-nix-purity`, `constraint-litellm-in-memory-no-db`,
`validation-nix-{build,eval,flake-check,format,lint,supply-chain,test}` (7),
`workflow-nix-hardening-00..05` (6), `workflow-litellm-config-change-00..05`
(6), `workflow-litellm-hardening-00..05` (6),
`workflow-litellm-validation-00..05` (6). Recorded fact:
`validation-litellm-config-check` no longer exists anywhere (deleted
source-side earlier); references to it are known-dangling per knowledge
`MANIFEST.md` §8a — it is not part of either set.

### BEADS.md — a SPLIT, not a move

`BEADS.md` stays in-repo but is trimmed: the **conventions** (`wrk-` prefix,
work/state boundary — beads tracks WORK, docs track STATE — sync discipline
via refs/dolt/data, single-writer rules, daily loops by role, conventions
tie-in) remain; the **mechanics sections** (Setup facts — binary sourcing
`nix shell nixpkgs#beads`, bd 1.0.3 version discipline — Upgrades,
Troubleshooting) are extracted into a **new `beads` skill authored in the
skills repo**, and `BEADS.md` gains a pointer to it. The tooling was never
configured anywhere agent-side; this split gives it a real home.

> NOTE 2026-09-18: the parenthetical tooling description is stale as written.
> The CLI is now the pinned Beads 1.2.2 via `just beads` (`BEADS.md` setup
> sections rewritten accordingly); `nix shell nixpkgs#beads` (bd 1.0.3) must
> not be used. The split direction is unchanged.

## Pointer/redirect strategy

Where staying files reference moved artifacts (knowledge `MANIFEST.md` §5a
list), replace the reference with a **sibling-relative pointer**
(`../../knowledge/...` / `../../skills/<name>/SKILL.md` from repo root,
depth-adjusted per file) or a `(moved to the agent-side knowledge repo)`
annotation. **Never leave a dangling reference.** Historical docs (e.g.
spec 07, already-EXECUTED specs) get annotations, not rebased links, where
the reference is historical narrative. The knowledge/skills repos already
use this convention.

The known rewires:

| Staying file | References | Action |
|---|---|---|
| `README.md` | `docs/nix-purity.md` | STAYS — no action |
| `justfile` | `docs/litellm` | rewire to knowledge repo path / annotate |
| `docs/migration/70-open-items.md` | `docs/nix-purity.md` | STAYS — no action |
| `docs/migration/80-remediation-plan.md` | `nix-store-gc` skill + `nix-purity.md` | rewire the skill ref; nix-purity stays |
| `docs/migration/nix-store-gc-remediation-spec.md` | `nix-purity.md` | STAYS — no action |
| `docs/nix-store-accumulation-report.md` | `nix-store-gc` skill + `nix-purity.md` | rewire the skill ref; nix-purity stays |
| `docs/validation-and-improvements/05-host-validation.md` | `validation-litellm-smoke` skill | rewire to skills repo path / annotate |
| `docs/validation-and-improvements/06-improvements/07-naming-consistency.md` | `nix-ci-cd`/`nix-docker-images` skills + `docs/gleam`, `docs/litellm`, `docs/nix` | historical narrative — annotate, do not rebase links |

## Validation gates

1. **Phase 0 `diff -rq` per tree** — the knowledge copies are byte-identical
   to the workestrate sources before ANY deletion (`docs/beam`, `elixir`,
   `gleam`, `rust`, `typescript`, `testing/property-based-testing`,
   `verification/rust`, `languages.md`, the litellm upstream corpus, the
   generic `docs/nix` files).
2. **Skills repo frontmatter parse 255/255** — validated at import;
   re-verify in Phase 0 (confirm skills repo HEAD covers all 255).
3. **Post-move markdown link check** over workestrate docs — **zero dangling
   relative links introduced**. The known-dangling list in knowledge
   `MANIFEST.md` §8a is pre-existing and is **NOT** this spec's scope.
4. **Knowledge + skills repos each committed** — they are local-only git;
   pushes are user-approved host-side (standing personal/knowledge repo
   local-only decisions).
5. **ADR index + improvements index updated** (`50-decisions/README.md`
   row 0033; `06-improvements/00-index.md` row 24).
6. **Personal/host sync notes** — knowledge and skills repos stay local-only
   until the user pushes them (same posture as the personal config repo
   decision 2026-08-24).

## Phased execution

Each phase is separately committable; phases 1–2 may interleave per area.

- **Phase 0 — user go + destination re-verification.** The user approves
  execution. `diff -rq` each docs tree against the knowledge copies; confirm
  the skills repo HEAD covers all 255 generic skills (gate 1 + gate 2).
- **Phase 1 — pointer rewires in staying files.** `README.md`, `justfile`,
  and the §5a docs list (table above). No dangling references introduced.
- **Phase 2 — source-side deletions.** Delete the moved docs trees/files and
  the 255 generic skill dirs — **one commit per area** (docs, skills) for
  reviewable history.
- **Phase 3 — BEADS.md split.** Trim the mechanics sections to a pointer;
  author the `beads` skill in the skills repo (separate commit there).
- **Phase 4 — validation gates + index/ADR status updates.** Run gates 3–6;
  update this spec's banner and the index rows to EXECUTED.

## Open questions

Mirrors ADR 0033 § Open questions:

- **(a) Generic litellm skills: move or stay?** Litellm is a project
  dependency. **RECOMMENDATION:** generic upstream litellm knowledge moves;
  project-config-specific stays (the OUR-config boundary already applied to
  the docs pair and the `workflow-litellm-config-change/hardening/validation`
  families).
- **(b) Consumption mechanism** — workestrate↔knowledge/skills: subtree,
  submodule, or sibling-relative-only (carried from the 2026-08-23
  extraction follow-ups).
- **(c) Beads skill home** — the skills repo (RECOMMENDED: shared) vs a
  host-global skills dir.
