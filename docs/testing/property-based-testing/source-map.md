# Property-Based Testing Source Map

## Purpose

This document maps every researched source URL to the property-based-testing documentation files that use it. It is the authoritative provenance index for the `docs/testing/property-based-testing/` corpus. Future agents can use it to trace any claim back to its primary source, identify which sources are taxonomy/discovery versus primary library docs, and assess coverage gaps before extending the corpus.

The corpus has two source tiers:

- **Tier A — taxonomy & foundational:** the `jmid/pbt-frameworks` feature matrix (discovery/taxonomy) and the foundational PBT papers (theory). These underpin the cross-language `landscape.md` and `index.md`.
- **Tier B — per-library primary sources:** the official docs, API reference, and package registries for each of the five in-scope libraries (`proptest`, `quickcheck`, `fast-check`, `StreamData`, `PropCheck`/PropEr). These underpin the per-language sub-corpora.

## Part A — Taxonomy & foundational sources

| URL | Authority | Used by |
|---|---|---|
| https://github.com/jmid/pbt-frameworks | Community (curated taxonomy; Jan Midtgaard) | `index.md`, `landscape.md`, `.crawl/pbt-frameworks-readme.md` |
| https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quickcheck.pdf (Claessen & Hughes, ICFP 2000 — QuickCheck origin) | Foundational paper (official) | `landscape.md` |
| http://www.cse.chalmers.se/~rjmh/Papers/QuickCheckSTLC.pdf (Hughes, PADL 2007 — state machines) | Foundational paper (official) | `landscape.md` |
| https://www.cse.chalmers.se/~rjmh/Papers/pulse.pdf (Claessen et al., ICFP 2009 — PULSE / parallel race testing) | Foundational paper (official) | `landscape.md` |
| https://drops.dagstuhl.de/opus/volltexte/2020/13164/ (MacIver & Donaldson, ECOOP 2020 — shrinking) | Foundational paper (official) | `landscape.md` |
| https://hypothesis.works/articles/integrated-shrinking/ (de Vries — integrated shrinking article) | Community | `landscape.md` (background) |
| https://www.drmaciver.com/2019/12/integrated-shrinking-is-not-the-real-prize/ (MacIver — integrated shrinking article) | Community | `landscape.md` (background) |
| https://propertesting.org/ (PropEr book — Fred Hebert) | Community (book) | `elixir/propcheck-state-machine.md`, `elixir/propcheck-targeted-pbt.md`, `elixir/streamdata-vs-propcheck.md` |

## Part B — Per-library primary sources

### Rust — proptest

| URL | Authority | Used by |
|---|---|---|
| https://github.com/proptest-rs/proptest | Official secondary (repo/README) | `rust/proptest-overview.md`, `rust/proptest-vs-quickcheck.md` |
| https://crates.io/crates/proptest | Official primary (registry) | `rust/proptest-overview.md` |
| https://proptest-rs.github.io/proptest/ (The proptest Book) | Official primary (guide) | `rust/proptest-overview.md`, `rust/proptest-generators.md`, `rust/proptest-shrinking.md`, `rust/proptest-state-machine.md` |
| https://docs.rs/proptest | Official primary (API reference) | `rust/proptest-generators.md`, `rust/proptest-shrinking.md` |
| https://crates.io/crates/proptest-state-machine | Official primary (registry) | `rust/proptest-state-machine.md` |
| https://crates.io/crates/test-strategy | Official primary (registry) | `rust/proptest-overview.md` (related-proc-macro note) |
| https://github.com/proptest-rs/proptest/issues/179 (async support discussion) | Official secondary (issue tracker) | `rust/proptest-async.md` |

### Rust — quickcheck

| URL | Authority | Used by |
|---|---|---|
| https://github.com/BurntSushi/quickcheck | Official secondary (repo/README) | `rust/quickcheck.md`, `rust/proptest-vs-quickcheck.md` |
| https://crates.io/crates/quickcheck | Official primary (registry) | `rust/quickcheck.md` |
| https://docs.rs/quickcheck | Official primary (API reference) | `rust/quickcheck.md` |
| https://crates.io/crates/quickcheck_macros | Official primary (registry) | `rust/quickcheck.md` |

### TypeScript — fast-check

| URL | Authority | Used by |
|---|---|---|
| https://github.com/dubzzz/fast-check | Official secondary (repo/README) | `typescript/fast-check-core-concepts.md` |
| https://fast-check.dev/ | Official primary (docs site) | `typescript/fast-check-core-concepts.md` |
| https://fast-check.dev/docs/core-blocks/arbitraries/ | Official primary (sub-page) | `typescript/fast-check-arbitraries.md` |
| https://fast-check.dev/docs/runners/ | Official primary (sub-page) | `typescript/fast-check-runner-integration.md` |
| https://fast-check.dev/docs/tutorials/model-based-testing/ | Official primary (sub-page) | `typescript/fast-check-state-machine.md` |
| https://fast-check.dev/docs/tutorials/race-conditions/ | Official primary (sub-page) | `typescript/fast-check-race-conditions.md` |
| https://www.npmjs.com/package/fast-check | Official primary (registry) | `typescript/fast-check-core-concepts.md` |
| https://www.npmjs.com/package/@fast-check/jest | Official primary (registry) | `typescript/fast-check-runner-integration.md` |
| https://www.npmjs.com/package/@fast-check/vitest | Official primary (registry) | `typescript/fast-check-runner-integration.md` |
| https://www.npmjs.com/package/@fast-check/ava | Official primary (registry) | `typescript/fast-check-runner-integration.md` |

### Elixir — StreamData

| URL | Authority | Used by |
|---|---|---|
| https://github.com/whatyouhide/stream_data | Official secondary (repo/README) | `elixir/streamdata-getting-started.md`, `elixir/streamdata-vs-propcheck.md` |
| https://hex.pm/packages/stream_data | Official primary (registry) | `elixir/streamdata-getting-started.md` |
| https://hexdocs.pm/stream_data/ | Official primary (docs) | `elixir/streamdata-getting-started.md`, `elixir/streamdata-combinators.md`, `elixir/streamdata-shrinking.md` |
| https://hexdocs.pm/stream_data/ExUnitProperties.html | Official primary (module) | `elixir/streamdata-exunit-integration.md` |
| https://hexdocs.pm/stream_data/StreamData.html | Official primary (module) | `elixir/streamdata-combinators.md`, `elixir/streamdata-shrinking.md` |

### Elixir — PropCheck / PropEr

| URL | Authority | Used by |
|---|---|---|
| https://github.com/alfert/propcheck | Official secondary (repo/README) | `elixir/propcheck-state-machine.md`, `elixir/propcheck-targeted-pbt.md`, `elixir/streamdata-vs-propcheck.md` |
| https://hex.pm/packages/propcheck | Official primary (registry) | `elixir/propcheck-state-machine.md` |
| https://hexdocs.pm/propcheck/ | Official primary (docs) | `elixir/propcheck-state-machine.md`, `elixir/propcheck-targeted-pbt.md` |
| https://hexdocs.pm/propcheck/PropCheck.StateM.ModelDSL.html | Official primary (module) | `elixir/propcheck-state-machine.md` |
| https://hexdocs.pm/propcheck/PropCheck.TargetedPBT.html | Official primary (module) | `elixir/propcheck-targeted-pbt.md` |
| https://github.com/proper-testing/proper | Official secondary (upstream PropEr repo) | `elixir/propcheck-state-machine.md`, `elixir/propcheck-targeted-pbt.md` |
| https://propertesting.org/ | Community (PropEr book) | `elixir/propcheck-state-machine.md`, `elixir/propcheck-targeted-pbt.md` |
