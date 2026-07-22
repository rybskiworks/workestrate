# Migration Documentation — workestrator Tool+XDG Model

**Status:** PRE-IMPLEMENTATION (authoritative design record)
**Branch:** `migration/tool-model`
**Base:** `840e8b7` on `main`
**Authored:** 2026-07-18

This documentation tree is the authoritative pre-implementation record for
migrating the `ai-workbench` monorepo into the **workestrate-as-tool + XDG +
dotfiles-registry** model. It captures: the verified current state, the target
system specification, the security model, the phased migration process, and
twenty-three Architecture Decision Records (ADRs) that pin every load-bearing
choice.

Implementation may proceed from this record without re-deriving decisions.
Where earlier session rounds expressed options or wavered, the ADRs in
`50-decisions/` are the final word.

## Reading order

1. `00-executive-summary.md` — the whole migration in ≤2 pages.
2. `10-current-state.md` — verified current state with file:line citations.
3. `20-target-system-spec.md` — the main spec of the migrated system.
4. `30-security-model.md` — threat model, invariants, enforcement points.
5. `40-migration-process.md` — phased process, per-file consequence sweep, gates; review & remediation status.
6. `50-decisions/README.md` — ADR index (0001–0023).
7. `50-decisions/NNNN-*.md` — individual ADRs (0001–0023).
8. `60-glossary.md` — canonical vocabulary.
9. `70-open-items.md` — pending user defaults, KVM gates, residual risks.
10. `80-remediation-plan.md` — review findings (2026-07) remediation plan, PENDING APPROVAL.

## Scope

This tree documents the **design**. It does not contain code changes. All
implementation (Rust, nix, shell, TOML) happens in subsequent commits against
this branch or follow-on branches, gated by the phase plan in
`40-migration-process.md`.

## Environment honesty

This documentation was authored in a container with **no nix** and **no KVM**.
All nix-eval and KVM-runtime gates are marked `HOST-NIX` or `HOST-KVM` in the
migration process. Claims about nix behavior (`builtins.fromTOML`, flake source
filtering, pure-eval invisibility) are based on documented Nix semantics, not
runtime verification in this session. External pattern precedents were
live-verified via web research; URLs are cited in the relevant ADRs.
