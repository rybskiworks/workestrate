<!-- Source: https://github.com/jmid/pbt-frameworks (README.md, fetched 2026-07-02). Jan Midtgaard. -->
<!-- This is a RECONSTRUCTION of the relevant rows only. The full matrix covers ~50 frameworks across ~31 languages. Only the Rust, JavaScript/TypeScript, and Elixir rows are reproduced here, plus the legend, footnote 3, and the foundational reading list. Consult the upstream README for the complete matrix. -->

# pbt-frameworks — Feature Matrix (Reconstructed Excerpt)

> **Provenance note:** This file is a persisted verbatim capture of the relevant portions of the `jmid/pbt-frameworks` README, fetched 2026-07-02. The repository is README-only (no separate docs site), maintained by Jan Midtgaard, last commit November 2025. The README self-warns that entries go stale as frameworks evolve. The full matrix evaluates ~50 frameworks across ~31 languages along the feature axes below; only the rows for our three target languages (Rust, JS/TS, Elixir) are reproduced here.

## Feature axes (column headers)

The matrix evaluates each framework along these axes:

1. **Generator EDSL** — a domain-specific language for writing value generators.
2. **Shrinking** — support for shrinking failing counterexamples toward a minimal case.
3. **Integrated shrinking** — automatic/integrated shrinking (framework shrinks for you), as opposed to manual shrinking.
4. **State machine** — support for stateful/state-machine-based property testing.
5. **Parallel state machine** — support for *true* parallel state-machine testing (linearizability checking).
6. **Coverage guidance** — support for targeted/coverage-guided PBT.

## Legend

- `✔` — supported
- `(✔)` — partial / footnoted
- `?` — uncertain

## Matrix rows (Rust, JS/TS, Elixir only)

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

## Footnotes

**Footnote 3 (verbatim):** "Hypothesis and fast-check support asynchronous state machine testing, which can find race conditions (although it is strictly speaking not using parallel testing)."

## Foundational reading list

The README points to the foundational PBT literature. The Tier 1 must-reads (full URLs in `source-map.md` Part A):

1. **Claessen & Hughes, "QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs" (ICFP 2000).**
   The origin of property-based testing. Introduces generators, shrinking, and the random-testing-with-falsification model.
   - https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quickcheck.pdf

2. **Hughes, "Testing Erlang Code with QuickCheck" (PADL 2007).**
   Introduces state-machine testing via command sequences — the foundation of state-machine frameworks in proptest, fast-check, and PropCheck.
   - http://www.cse.chalmers.se/~rjmh/Papers/QuickCheckSTLC.pdf

3. **Claessen, Palka, Smallbone, Hughes, et al., "Finding Race Conditions in Erlang with QuickCheck and PULSE" (ICFP 2009).**
   The PULSE linearizability checker — the lineage behind PropCheck's true parallel state-machine testing.
   - https://www.cse.chalmers.se/~rjmh/Papers/pulse.pdf

4. **MacIver & Donaldson, "Test-case reduction via structured generator shrinking" (ECOOP 2020).**
   The theory behind fast-check's and Hypothesis's structured/integrated shrinking.
   - https://drops.dagstuhl.de/opus/volltexte/2020/13164/

## Related articles (integrated shrinking)

- de Vries — integrated shrinking: https://hypothesis.works/articles/integrated-shrinking/
- MacIver — "Integrated shrinking is not the real prize": https://www.drmaciver.com/2019/12/integrated-shrinking-is-not-the-real-prize/

## PropEr book

- Fred Hebert, "PropEr Testing" (propertesting.org): https://propertesting.org/
