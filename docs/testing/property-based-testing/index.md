# Property-Based Testing

## Purpose

This corpus is a project-independent, **cross-language** reference for property-based testing (PBT) in the three languages this workspace targets: **Rust, TypeScript, and Elixir**. It is not tied to any specific repository, framework version, or team convention; instead it captures the shared PBT theory (generators, shrinking, integrated shrinking, state-machine testing, parallel/async testing, coverage guidance), the per-language library landscape, and the comparative guidance an agent needs to pick the right tool and use it correctly. Every claim is sourced from the official library docs, the foundational PBT papers, and the curated `jmid/pbt-frameworks` feature matrix, with verbatim quotations preserved under `.crawl/` so claims can be audited.

The intended audience is **future AI agents** — agents that will write, review, refactor, debug, or validate property-based tests in Rust, TypeScript, or Elixir. Human developers may also find it useful, but the structure is optimized for agent consumption: each per-language sub-corpus states its library's API surface, semantics, and pitfalls; this shared index provides the cross-language routing and the foundational theory that does not belong to any single language.

This corpus is the **PBT home** for the workspace. The ExUnit skill (`validation-elixir-test`) explicitly scopes *out* property-based testing and defers here; the Rust testing skill (`docs/rust/testing.md`) and the Elixir ExUnit doc (`docs/elixir/testing-exunit.md`) similarly point here for PBT rather than duplicating it.

## Corpus map

The corpus is organized as a shared layer plus three per-language sub-corpora. The per-language subdirectories are authored by separate leads; the files below are the predetermined tree.

```
property-based-testing/
├── index.md                      (this file — cross-language routing)
├── landscape.md                  (cross-language feature matrix + foundational reading)
├── source-map.md                 (provenance: every source URL → which docs use it)
├── .crawl/                       (verbatim source artifacts)
│   └── pbt-frameworks-readme.md  (jmid/pbt-frameworks feature matrix, reconstructed)
├── rust/
│   ├── index.md
│   ├── proptest-overview.md
│   ├── proptest-generators.md
│   ├── proptest-shrinking.md
│   ├── proptest-state-machine.md
│   ├── proptest-async.md
│   ├── proptest-vs-quickcheck.md
│   └── quickcheck.md
├── typescript/
│   ├── index.md
│   ├── fast-check-core-concepts.md
│   ├── fast-check-arbitraries.md
│   ├── fast-check-shrinking.md
│   ├── fast-check-state-machine.md
│   ├── fast-check-race-conditions.md
│   └── fast-check-runner-integration.md
└── elixir/
    ├── index.md
    ├── streamdata-getting-started.md
    ├── streamdata-combinators.md
    ├── streamdata-shrinking.md
    ├── streamdata-exunit-integration.md
    ├── propcheck-state-machine.md
    ├── propcheck-targeted-pbt.md
    └── streamdata-vs-propcheck.md
```

## Cross-language recommendation

| Language | Recommended library | Runner-up / complement | Rationale |
|---|---|---|---|
| **Rust** | `proptest` | `quickcheck` | `proptest` has integrated shrinking, a state-machine framework, and the richer generator EDSL. `quickcheck` is the classic-lineage contrast (Haskell QuickCheck port) but lacks integrated shrinking — shrinking is manual via `Arbitrary` instances. |
| **TypeScript** | `fast-check` | (none — only mature option) | `fast-check` is the only production-grade PBT library in the JS/TS ecosystem. It has integrated shrinking, a state-machine framework, and async race-condition detection. |
| **Elixir** | `StreamData` | `PropCheck` (complement) | `StreamData` is the idiomatic, ExUnit-integrated default for generators and shrinking. `PropCheck` (a PropEr wrapper) is **complementary**, not a competitor: use it for stateful, targeted, and **parallel** state-machine testing that StreamData deliberately does not cover. |

## How these relate

Three relationships matter when navigating this corpus:

1. **StreamData + PropCheck are complementary, not competing.** The StreamData README explicitly defers stateful testing to PropCheck. StreamData provides generators, shrinking, and ExUnit integration for stateless properties; PropCheck (wrapping PropEr) adds state-machine testing, targeted PBT, and the only **true parallel** state-machine testing in the Elixir ecosystem (PULSE linearizability lineage). Use both. See `elixir/streamdata-vs-propcheck.md`.

2. **`quickcheck` is the classic-lineage contrast to `proptest`.** Both are Rust PBT libraries, but they descend from different traditions. `quickcheck` is a port of Haskell QuickCheck (the origin of PBT); `proptest` is a Rust-native design. The decisive difference is **shrinking**: `proptest` has integrated shrinking (the framework shrinks for you), `quickcheck` does not (you implement shrinking in your `Arbitrary` instance or skip it). See `rust/proptest-vs-quickcheck.md`.

3. **Integrated shrinking is the modern baseline.** Of the five tools in scope, only `quickcheck` lacks it. `proptest`, `fast-check`, `StreamData`, and `PropCheck`/PropEr all ship integrated shrinking. When choosing a tool, treat integrated shrinking as table stakes; the differentiators are state-machine support, parallel/async testing, and ecosystem fit. See `landscape.md`.

## Related corpora

- `docs/rust/testing.md` — Rust testing overview (unit/integration/doctests); defers here for PBT.
- `docs/elixir/testing-exunit.md` — ExUnit testing; defers here for PBT (ExUnit skill scopes out StreamData/PropCheck).
- `docs/typescript/index.md` — TypeScript language corpus (nascent); PBT coverage lives under `docs/testing/property-based-testing/typescript/`.
- `docs/beam/` — BEAM runtime concepts relevant to PropCheck's parallel state-machine testing (processes, links/monitors) where Elixir-specific.
