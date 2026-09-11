# ADR 0015: Per-workload repos rejected

**Status:** Accepted
**Date:** 2026-07-18

## Context

The user asked whether workloads should be individually-distributable repos
(devcontainer-features-style), each with their own versioning and discovery.

## Options considered

1. **Per-workload repos** — each workload definition is a separate repo.
   Rejected.
2. **Workload definitions in config repos** — workload defs are ~20-line TOML
   entries in the config repo. Selected.

## Decision

Per-workload repos are rejected. Workload definitions are ~20-line TOML entries
that live in config repos. The actual unit of behavioral distribution is the
**recipe vocabulary** (egress recipes, build recipes, image recipes, features),
which lives in core (the tool). Workload definitions are consumer-specific
config that selects from the vocabulary.

## Rationale

- N workloads × N repos × versioning × discovery = O(N²) complexity for
  ~20-line data files.
- The devcontainer-features analogy doesn't apply: devcontainer features ARE
  code (install scripts). Workload definitions are pure data referencing core
  code. Distributing data files as repos is the wrong granularity.
- If a team wants to share a workload definition, they put it in their team
  config repo. If an individual wants a custom workload, they put it in their
  personal config repo. The config repo is the distribution unit.

## Consequences

- No per-workload repo infrastructure (no discovery, no per-workload
  versioning).
- Workload definitions are entries in config repos.
- The recipe vocabulary (in core) is the shared, versioned, reviewable unit.

## Rejected why

Per-workload repos: O(N²) complexity for ~20-line data files; wrong
granularity.
