# Property-Based Testing Landscape (Cross-Language)

## Purpose

This is the cross-language topic doc for the property-based-testing corpus. It summarizes the **feature matrix** of PBT frameworks across our three target languages (Rust, TypeScript, Elixir), distills the comparative insights that drive the per-language recommendations in `index.md`, and provides a prioritized foundational reading list. The verbatim matrix is persisted under `.crawl/pbt-frameworks-readme.md`; this doc interprets it.

## What `jmid/pbt-frameworks` is

[`jmid/pbt-frameworks`](https://github.com/jmid/pbt-frameworks) is a single-file, community-curated **feature matrix** of property-based testing frameworks, maintained by Jan Midtgaard. Key characteristics:

- **README-only.** The entire deliverable is the repository's `README.md` — there is no separate documentation site. The matrix is a Markdown table.
- **Scope:** ~50 frameworks across ~31 languages. It is a discovery/taxonomy source, not primary documentation for any single library.
- **Actively maintained:** last commit November 2025 at time of capture (2026-07-02).
- **Self-warning:** the README explicitly notes that entries go stale as frameworks evolve; treat the matrix as a snapshot and always cross-check against the library's own docs before relying on a feature flag.
- **Authority:** community (curated, not the official docs of any single tool). Use it for cross-framework comparison and discovery; use each library's primary docs (see `source-map.md` Part B) for API detail.

## The seven feature axes

The matrix evaluates each framework along these axes (column headers, paraphrased):

1. **Generator EDSL** — does the framework provide a domain-specific language for writing value generators?
2. **Shrinking** — does the framework support shrinking failing counterexamples toward a minimal case?
3. **Integrated shrinking** — is shrinking automatic/integrated (the framework shrinks for you), as opposed to manual shrinking the user must author?
4. **State machine** — does the framework support stateful/state-machine-based property testing?
5. **Parallel state machine** — does the framework support *true* parallel state-machine testing (linearizability checking, à la PULSE)?
6. **Coverage guidance** — does the framework support targeted/coverage-guided PBT?

(See the verbatim header and legend in `.crawl/pbt-frameworks-readme.md`.)

## The verbatim matrix rows for our three languages

Quoted from the `jmid/pbt-frameworks` README (legend: `✔` = supported; `(✔)` = partial/footnoted; `?` = uncertain). The full matrix covers ~50 frameworks; only our three languages are reproduced here.

### Rust

| Framework | Gen | Shrinking | Int. shrinking | State machine | Par. st. mach. | Coverage |
|---|---|---|---|---|---|---|
| quickcheck | ✔ | ✔ | | | | |
| proptest | ✔ | ✔ | ✔ | ✔ | | |

### JavaScript / TypeScript

| Framework | Gen | Shrinking | Int. shrinking | State machine | Par. st. mach. | Coverage |
|---|---|---|---|---|---|---|
| fast-check | ✔ | ✔ | ✔ | ✔ | (✔) ³ | |

### Elixir

| Framework | Gen | Shrinking | Int. shrinking | State machine | Par. st. mach. | Coverage |
|---|---|---|---|---|---|---|
| PropCheck | ✔ | ✔ | ✔ | ✔ | ✔ | |
| StreamData | ✔ | ✔ | ✔ | | | |

**Footnote 3 (verbatim):** "Hypothesis and fast-check support asynchronous state machine testing, which can find race conditions (although it is strictly speaking not using parallel testing)."

## Three key comparative insights

1. **Integrated shrinking is the modern baseline.** Of the five in-scope tools, only `quickcheck` lacks integrated shrinking. `proptest`, `fast-check`, `StreamData`, and `PropCheck`/PropEr all ship it. When evaluating a PBT library, treat integrated shrinking as table stakes; the real differentiators are state-machine support, parallel/async testing, and ecosystem fit. This is why `proptest` is recommended over `quickcheck` for Rust (see `index.md`).

2. **PropCheck is the only tool with *true* parallel state-machine testing.** PropCheck (wrapping PropEr) inherits the PULSE linearizability-checking lineage (Claessen et al., ICFP 2009). Its parallel state-machine testing runs commands across multiple processes and checks the resulting interleavings for linearizability — genuine concurrency testing. By contrast, `fast-check`'s "parallel" mode (footnote 3) is **asynchronous** state-machine testing: it interleaves async operations to surface race conditions, which is valuable but "strictly speaking not parallel testing." Do not conflate the two. See `elixir/propcheck-state-machine.md` and `typescript/fast-check-race-conditions.md`.

3. **StreamData and PropCheck are complementary.** StreamData is the idiomatic Elixir default for generators, shrinking, and ExUnit-integrated stateless properties. PropCheck is the complement for stateful, targeted, and parallel testing that StreamData deliberately does not cover. The StreamData README itself defers stateful testing to PropCheck. Use both; do not pick one as a winner. See `elixir/streamdata-vs-propcheck.md`.

## Prioritized foundational reading list

These are the foundational PBT papers. Tier 1 = must-read for any agent doing serious PBT work in any language. Full URLs in `source-map.md` Part A.

### Tier 1 (must-read)

1. **Claessen & Hughes, "QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs" (ICFP 2000).**
   The origin of property-based testing. Introduces generators, shrinking, and the random-testing-with-falsification model that every tool in this corpus descends from. Read this first to understand what PBT *is*.
   - URL: https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quickcheck.pdf

2. **Hughes, "Testing Erlang Code with QuickCheck" (PADL 2007).**
   Introduces state-machine testing via command sequences — the foundation of `proptest`'s, `fast-check`'s, and PropCheck's state-machine frameworks. Essential before reading any state-machine doc in this corpus.
   - URL: http://www.cse.chalmers.se/~rjmh/Papers/QuickCheckSTLC.pdf

3. **Claessen, Palka, Smallbone, Hughes, et al., "Finding Race Conditions in Erlang with QuickCheck and PULSE" (ICFP 2009).**
   The PULSE linearizability checker — the lineage behind PropCheck's *true* parallel state-machine testing. Distinguishes genuine parallel testing from async race detection. Read alongside `elixir/propcheck-state-machine.md`.
   - URL: https://www.cse.chalmers.se/~rjmh/Papers/pulse.pdf

4. **MacIver & Donaldson, "Test-case reduction via structured generator shrinking" (ECOOP 2020).**
   The theory behind `fast-check`'s and Hypothesis's structured/integrated shrinking. Clarifies *why* integrated shrinking works and how it differs from the original QuickCheck manual-shrinking approach. Read alongside `typescript/fast-check-shrinking.md` and `rust/proptest-shrinking.md`.
   - URL: https://drops.dagstuhl.de/opus/volltexte/2020/13164/

## Sources used

- [jmid/pbt-frameworks README](https://github.com/jmid/pbt-frameworks) — the verbatim feature matrix (7 axes, ~50 frameworks); our 3 languages' rows quoted in "The verbatim matrix rows" above; footnote 3 quoted verbatim. Reconstructed capture in `.crawl/pbt-frameworks-readme.md`.
- [Claessen & Hughes, ICFP 2000](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quickcheck.pdf) — QuickCheck origin; cited in foundational reading list.
- [Hughes, PADL 2007](http://www.cse.chalmers.se/~rjmh/Papers/QuickCheckSTLC.pdf) — state-machine testing origin; cited in foundational reading list.
- [Claessen et al., ICFP 2009](https://www.cse.chalmers.se/~rjmh/Papers/pulse.pdf) — PULSE / parallel race-condition testing; cited in foundational reading list and the PropCheck parallel insight.
- [MacIver & Donaldson, ECOOP 2020](https://drops.dagstuhl.de/opus/volltexte/2020/13164/) — structured shrinking theory; cited in foundational reading list.
