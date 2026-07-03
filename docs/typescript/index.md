# TypeScript Guidance Index

## Purpose

This corpus is a project-independent reference for TypeScript (and, where relevant, JavaScript) guidance. It is **newly scaffolded** and intentionally minimal: the workspace's TypeScript coverage is currently concentrated in property-based testing, which lives under [`docs/testing/property-based-testing/typescript/`](../testing/property-based-testing/typescript/) (the `fast-check` library). This `docs/typescript/` corpus will expand as TypeScript adoption grows across the workspace and additional TS-specific guidance (language fundamentals, tooling, type system, testing beyond PBT) is needed.

The intended audience is **future AI agents** — agents that will write, review, refactor, debug, or validate TypeScript code. Every document is sourced from the official TypeScript/JavaScript documentation, library docs, and standards, with verbatim quotations so claims can be audited.

## Relationship to the property-based-testing corpus

TypeScript PBT coverage does **not** live here. It lives in the cross-language PBT corpus:

- **PBT home:** [`docs/testing/property-based-testing/index.md`](../testing/property-based-testing/index.md)
- **TypeScript PBT sub-corpus:** [`docs/testing/property-based-testing/typescript/`](../testing/property-based-testing/typescript/) — `fast-check` core concepts, arbitraries, shrinking, state-machine testing, race conditions, and runner integration.

This `docs/typescript/` corpus will host non-PBT TypeScript guidance as it is authored.

## Corpus status

**Nascent.** No topic docs yet beyond this index and the stub `source-map.md`. Expand as TypeScript coverage grows. See [`source-map.md`](./source-map.md) for the current (minimal) provenance map.
