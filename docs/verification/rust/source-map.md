# Verus Source Map

## Purpose

This document maps every source used by the `docs/verification/rust/` corpus to the documents that cite it. It is the authoritative provenance index: future agents can trace any claim back to its primary source, identify whether a source is the local clone (Tier 1, verbatim) or an upstream web resource, and assess coverage before extending the corpus.

The Verus source is a local git clone at `.tmp/verus/` (gitignored, not committed). All Tier 1 content is read directly from this clone — no content is invented. Verbatim copies of key source artifacts are persisted under `.crawl/` so claims can be audited without re-cloning.

## Part A — The Verus repository (local clone)

The clone lives at `.tmp/verus/` and mirrors `github.com/verus-lang/verus` (branch `main`, fetched 2026-07-03). The upstream URL prefix is `https://github.com/verus-lang/verus/blob/main/`.

| Local path | Upstream URL | Authority | Used by | Notes |
|---|---|---|---|---|
| `.tmp/verus/README.md` | https://github.com/verus-lang/verus/blob/main/README.md | official primary | index.md, getting-started.md | Project overview, status, documentation links, examples. |
| `.tmp/verus/INSTALL.md` | https://github.com/verus-lang/verus/blob/main/INSTALL.md | official primary | getting-started.md | Binary release install, platform support, toolchain. |
| `.tmp/verus/BUILD.md` | https://github.com/verus-lang/verus/blob/main/BUILD.md | official primary | getting-started.md, index.md | Building from source, Z3 setup, `vargo`, running the verifier, `--compile`. |
| `.tmp/verus/CONTRIBUTING.md` | https://github.com/verus-lang/verus/blob/main/CONTRIBUTING.md | official primary | getting-started.md, proof-engineering.md | `verusfmt`, testing (`vargo test`), `--log-all`, coverage, vstd conventions, `--record`. |
| `.tmp/verus/rust-toolchain.toml` | https://github.com/verus-lang/verus/blob/main/rust-toolchain.toml | official primary | index.md | Toolchain pin: `1.96.0`, components. |
| `.tmp/verus/source/CODE.md` | https://github.com/verus-lang/verus/blob/main/source/CODE.md | official primary | index.md, verification-model.md | Architecture: pipeline, crate architecture, detailed stages, ghost erasure, key abstractions. Verbatim copy at `.crawl/CODE.md`. |
| `.tmp/verus/source/docs/guide/src/SUMMARY.md` | https://github.com/verus-lang/verus/blob/main/source/docs/guide/src/SUMMARY.md | official primary | index.md, all topic docs | Tutorial TOC (207 lines): fundamentals, proof development, verification & Rust, installation, reference. Verbatim copy at `.crawl/guide-SUMMARY.md`. |
| `.tmp/verus/source/docs/guide/src/overview.md` | https://github.com/verus-lang/verus/blob/main/source/docs/guide/src/overview.md | official primary | index.md, verification-model.md | Verus overview: goals, SMT/Z3, linear types for memory, what Verus does/doesn't intend. |
| `.tmp/verus/source/docs/guide/src/tcb.md` | https://github.com/verus-lang/verus/blob/main/source/docs/guide/src/tcb.md | official primary | index.md, verification-model.md | Assumptions and trusted components: `assume`, `external_body`, `external`, axioms. |
| `.tmp/verus/source/docs/guide/src/memory-safety.md` | https://github.com/verus-lang/verus/blob/main/source/docs/guide/src/memory-safety.md | official primary | index.md, verification-model.md, rust-features.md | Memory safety conditional on verification; Rust safe/unsafe vs Verus. |
| `.tmp/verus/source/docs/guide/src/call-from-unverified-code.md` | https://github.com/verus-lang/verus/blob/main/source/docs/guide/src/call-from-unverified-code.md | official primary | index.md, verification-model.md, rust-features.md | Calling verified code from unverified code; Drop trait requirements. |
| `.tmp/verus/source/docs/state_machines/src/SUMMARY.md` | https://github.com/verus-lang/verus/blob/main/source/docs/state_machines/src/SUMMARY.md | official primary | concurrency-and-state-machines.md | State-machines guide TOC: tokenized state machines, VerusSync, transition language, invariants. |
| `.tmp/verus/source/vstd/vstd.rs` | https://github.com/verus-lang/verus/blob/main/source/vstd/vstd.rs | official primary | vstd-library.md, index.md | vstd crate root: module declarations, feature flags, `group_vstd_default` broadcast group. Generated tree at `.crawl/vstd-module-tree.txt`. |
| `.tmp/verus/source/vstd/` (all `.rs`) | https://github.com/verus-lang/verus/tree/main/source/vstd | official primary | vstd-library.md | 125 `.rs` files, ~51,957 lines. Module tree with line counts at `.crawl/vstd-module-tree.txt`. |
| `.tmp/verus/examples/` | https://github.com/verus-lang/verus/tree/main/examples | official primary | getting-started.md, proofs.md, rust-features.md | Standalone examples (vectors.rs, adts.rs, atomics.rs, etc.). |
| `.tmp/verus/examples/guide/` | https://github.com/verus-lang/verus/tree/main/examples/guide | official primary | specifications.md, proofs.md, rust-features.md | Guide examples (bst_map, calc, datatypes, equality, etc.). |
| `.tmp/verus/source/rust_verify_test/tests/` | https://github.com/verus-lang/verus/tree/main/source/rust_verify_test/tests | official primary | all topic docs | Unit tests containing syntax/feature examples (referenced by README and CONTRIBUTING). |

## Part B — Key upstream / web sources

These are external (non-clone) sources cited by the corpus. They provide the published guide, API docs, research papers, and the SMT solver.

| Source | URL | Authority | Used by | Notes |
|---|---|---|---|---|
| Verus tutorial & reference (rendered guide) | https://verus-lang.github.io/verus/guide/ | official primary | all topic docs | Rendered mdBook of `source/docs/guide/`. The clone's `SUMMARY.md` is the TOC. |
| Verus overview (rendered) | https://verus-lang.github.io/verus/guide/overview.html | official primary | index.md, verification-model.md | Rendered `overview.md`. |
| Guarantees chapter (rendered) | https://verus-lang.github.io/verus/guide/guarantees.html | official primary | index.md, verification-model.md | Parent page for tcb, memory-safety, call-from-unverified-code. |
| vstd verusdoc (API docs) | https://verus-lang.github.io/verus/verusdoc/vstd/ | official primary | vstd-library.md | Modified rustdoc for the verified standard library. |
| State machines guide (rendered) | https://verus-lang.github.io/verus/state_machines/ | official primary | concurrency-and-state-machines.md | Rendered mdBook of `source/docs/state_machines/`. |
| Verus Playground | https://play.verus-lang.org/ | official primary | getting-started.md | In-browser Verus. |
| Verus publications & projects | https://verus-lang.github.io/verus/publications-and-projects/ | official primary | index.md | List of research papers and industry projects using Verus. |
| OOPSLA 2023 paper | https://arxiv.org/abs/2303.05491 | official primary (peer-reviewed) | verification-model.md, index.md | Lattuada et al., "Verus: Verifying Rust Programs using Linear Ghost Types". Foundational paper on Verus's linear ghost type approach. |
| SOSP 2024 tutorial | https://verus-lang.github.io/event-sites/2024-sosp/ | official primary | getting-started.md, proof-engineering.md | Day-long Verus tutorial: videos, slides, exercises. |
| Z3 SMT solver | https://github.com/Z3Prover/z3/releases | official primary (Microsoft Research) | index.md, arithmetic-and-provers.md | Z3 4.12.5 is the pinned version. Set via `VERUS_Z3_PATH`. |
| Z3 guide | https://microsoft.github.io/z3guide/docs/logic/intro | official primary | arithmetic-and-provers.md | SMT solver background; referenced from Verus overview.md. |
| cvc5 (experimental solver) | https://cvc5.github.io/ | official primary | arithmetic-and-provers.md | Experimental alternative to Z3 (mentioned in CODE.md). Does not support per-function rlimit. |
| Singular (computer algebra) | https://www.singular.uni-kl.de/ | official primary | arithmetic-and-provers.md | Optional integration for polynomial reasoning (guide: install-singular.md). |
| verusfmt (formatter) | https://github.com/verus-lang/verusfmt | official primary | getting-started.md, proof-engineering.md | Auto-formatter for `verus! { }` code. |
| cargo-verus | https://github.com/verus-lang/verus/blob/main/source/docs/CARGO-VERUS.md | official primary | getting-started.md | Cargo integration for Verus projects. |
| Verus Zulip | https://verus-lang.zulipchat.com/ | community (official) | getting-started.md | Community support channel. |
| Human-eval-verus examples | https://github.com/secure-foundations/human-eval-verus/ | official primary | proofs.md, specifications.md | Standalone Verus examples for small tasks. |
| Dafny (related verifier) | https://github.com/dafny-lang/dafny | official primary (related project) | verification-model.md | Referenced in overview.md as a related verification framework. |
| Rustonomicon (safe/unsafe) | https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html | official primary (rust-lang) | verification-model.md, rust-features.md | Referenced from memory-safety.md for the safe/unsafe philosophy. |

## `.crawl/` artifacts

The following verbatim source artifacts are persisted under `docs/verification/rust/.crawl/` for auditability:

| Artifact | Source | Type |
|---|---|---|
| `.crawl/guide-SUMMARY.md` | `source/docs/guide/src/SUMMARY.md` | verbatim copy (tutorial TOC) |
| `.crawl/CODE.md` | `source/CODE.md` | verbatim copy (architecture doc) |
| `.crawl/vstd-module-tree.txt` | `source/vstd/vstd.rs` + `find source/vstd -name '*.rs'` | generated (mod declarations + file listing with line counts) |
| `.crawl/verus-pipeline.txt` | `source/CODE.md` (pipeline sections) | verbatim excerpt + ASCII flow diagram |
