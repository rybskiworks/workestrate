# Property-Based Testing — TypeScript (fast-check)

## Purpose

This sub-corpus is the TypeScript entry in the cross-language property-based-testing corpus. It documents **fast-check** — the only seriously maintained property-based testing library in the JavaScript/TypeScript ecosystem. `jsverify` has been dormant since 2018 and `testcheck` is dormant; neither receives updates. fast-check is the sole production-grade option and is actively maintained by a single maintainer (Nicolas Dubien).

This index maps the six topic documents below. Each is a standalone operational reference with runnable TypeScript/JavaScript examples lifted from the official fast-check.dev docs and the npm READMEs. Cross-reference the shared corpus index at [`../index.md`](../index.md) for cross-language routing and foundational PBT theory, and the nascent TypeScript language corpus at [`docs/typescript/index.md`](../../../typescript/index.md).

## Version pin

All examples target **fast-check v4.8.0** (May 2026, MIT, ~30.5M weekly npm downloads). fast-check v4 is **ESM-only** — there is no CommonJS build. Requirements: Node ≥ 12.17.0, TypeScript ≥ 5.0. The first-party adapters are pinned at: `@fast-check/jest` 2.2.0, `@fast-check/vitest` 0.4.1, `@fast-check/ava` 3.0.1.

## Document map

- [`fast-check-core-concepts.md`](fast-check-core-concepts.md) — the `Arbitrary<T>` model (generator + shrinker paired), `fc.assert` vs `fc.check`, `fc.property` / `fc.asyncProperty`, pre-conditions, run parameters, reading failure reports.
- [`fast-check-arbitraries.md`](fast-check-arbitraries.md) — the five arbitrary families (primitives, composites, combiners, recursive, fake data) plus schedulers/commands, all preserving integrated shrinking.
- [`fast-check-shrinking.md`](fast-check-shrinking.md) — integrated automatic shrinking, trivial shrink targets, the commutative-add example, `fc.noShrink` / `fc.limitShrink`, command-aware shrinking.
- [`fast-check-state-machine.md`](fast-check-state-machine.md) — model-based testing via `fc.commands` + `fc.modelRun` / `asyncModelRun` / `scheduledModelRun`, the `Command<Model, Real>` pattern, replay.
- [`fast-check-race-conditions.md`](fast-check-race-conditions.md) — `fc.scheduler()` for async race detection via Promise-resolution reordering (single event loop, not true parallelism), the v4 scheduler API.
- [`fast-check-runner-integration.md`](fast-check-runner-integration.md) — first-party adapters (`@fast-check/jest`, `@fast-check/vitest`, `@fast-check/ava`), Bun, and the Node built-in test runner.

## Why fast-check is the only choice

The JS/TS PBT landscape collapsed to a single viable library:

- **fast-check** — actively maintained, integrated shrinking, model-based testing, async scheduler for race conditions, first-party framework adapters. ~30.5M weekly downloads.
- **jsverify** — last meaningful release 2018; dormant. No integrated shrinking, no model-based testing.
- **testcheck** — dormant; no shrinking, no maintenance.

When a TypeScript project needs property-based testing, fast-check is the answer. There is no runner-up.

## Related docs

- [`../index.md`](../index.md) — cross-language PBT corpus index (Rust / TypeScript / Elixir routing, foundational theory).
- [`../landscape.md`](../landscape.md) — cross-language feature matrix and foundational reading.
- [`../../../typescript/index.md`](../../../typescript/index.md) — TypeScript language corpus (nascent).
- [`../../../elixir/testing-exunit.md`](../../../elixir/testing-exunit.md) — ExUnit testing; defers here for PBT.

## Sources used

- [fast-check.dev — Introduction](https://fast-check.dev/docs/introduction/getting-started) — library overview and getting started
- [npmjs.com/package/fast-check](https://www.npmjs.com/package/fast-check) — version, download stats, license
- [github.com/dubzzz/fast-check](https://github.com/dubzzz/fast-check) — monorepo (library + adapters), maintainer
- [github.com/jsverify/jsverify](https://github.com/jsverify/jsverify) — dormant since 2018 (landscape contrast)
- [npmjs.com/package/testcheck](https://www.npmjs.com/package/testcheck) — dormant (landscape contrast)
