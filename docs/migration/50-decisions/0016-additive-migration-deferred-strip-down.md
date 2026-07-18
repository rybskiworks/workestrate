# ADR 0016: Additive migration / deferred repo strip-down

**Status:** Accepted
**Date:** 2026-07-18

## Context

The ai-workbench repo is today both tool and workspace. The migration to
tool+XDG model could be done as a big-bang rewrite (strip the repo, move
everything) or as an additive migration (new model works alongside old).

## Options considered

1. **Big-bang strip-down** — remove all workspace content from the repo in
   one pass; tool-only repo. Rejected: high risk; breaks existing workflows
   during migration; no fallback.
2. **Additive migration** — new XDG model works alongside the existing root
   `workestrate.toml`; repo strip-down is deferred/optional. Selected.

## Decision

Migration is additive. Root `workestrate.toml` keeps working as a project
layer. The XDG model is added on top; users can adopt it incrementally. Repo
strip-down (removing `.env.enc`, `.sops.yaml`, `infra/litellm/`,
`agents/*/config/`, `workspaces/`, `var/` from the repo) is deferred and
optional — it happens when the user is confident the XDG model works.

## Consequences

- During migration, both models work: root `workestrate.toml` (project layer)
  and XDG config repos.
- `just verify` stays green throughout migration.
- The user can dogfood the XDG model before committing to the strip-down.
- `just init-dev` recipe for dogfooding (sets up XDG paths from current repo
  contents).
- Strip-down (M.10) is a separate, optional step.

## Rejected why

Big-bang: high risk; breaks workflows; no fallback.
