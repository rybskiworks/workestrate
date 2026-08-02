# Verus — Formal Verification for Rust

## Purpose

This corpus is a deep operational reference for proving Rust code correct with [Verus](https://github.com/verus-lang/verus), an SMT/Z3-based deductive verification tool. It covers the verification model (requires/ensures/invariant, ghost code, the three execution modes), the internal pipeline (HIR → VIR → AIR → SMT-LIB → Z3), the trusted computing base and guarantees, the verified standard library (`vstd`), supported and unsupported Rust features, concurrency and state-machine verification, and proof-engineering practice. Every claim is sourced from the Verus repository (cloned locally at `.tmp/verus/`, gitignored) and the official published guide, with verbatim source artifacts preserved under `.crawl/` so claims can be audited.

The intended audience is future AI agents that will write, review, debug, or validate Verus-verified Rust code. The corpus is project-independent: it captures Verus semantics and tooling, not repo-specific proof decisions.

## What Verus is

Verus is a tool for verifying the correctness of code written in Rust. Developers write specifications of what their code should do (via `requires`, `ensures`, `invariant`, and mathematical types like `int` and `nat`), and Verus statically checks — for all possible executions — that the executable Rust code will always satisfy those specifications. Rather than adding run-time checks, Verus relies on automated theorem proving (an SMT solver, Z3) to discharge the generated verification conditions. Ghost code (specifications, proof functions, `Ghost<T>`/`Tracked<T>` wrappers) is erased before compilation, so verified code has zero runtime verification overhead and compiles to ordinary Rust via `rustc`/LLVM.

Verus reuses Rust's own borrow checker and type system to handle memory and aliasing reasoning, then layers deductive verification on top. Code is written inside a `verus! { ... }` macro that extends Rust syntax with verification constructs. Verus supports three modes: **`spec`** (pure mathematical specification functions, erased at compile time), **`proof`** (ghost proof functions and blocks, erased at compile time), and **`exec`** (executable code, compiled and verified). The `verus!` macro parses the extended syntax (via a forked `syn` crate), rewrites it into standard Rust with appropriate attributes, and hands it to `rustc` for type-checking and the Verus verifier for VC generation.

## The verification pipeline

Verus transforms Rust source through multiple intermediate representations before dispatching verification conditions to Z3. The pipeline (sourced from `source/CODE.md`, verbatim copy at `.crawl/CODE.md`):

```
Rust Source Code
    ↓ (rustc: parsing, macro expansion)
Rust HIR (High-level Intermediate Representation)
    ↓ (rust_verify: HIR → VIR)
VIR-AST (Verification Intermediate Representation)
    ↓ (vir: AST → SST)
VIR-SST (Statement-oriented Syntax Tree)
    ↓ (vir: SST → AIR)
AIR (Assertion Intermediate Representation)
    ↓ (air: SMT encoding)
SMT-LIB queries
    ↓ (Z3, and experimentally cvc5)
Verification Results
```

One line per stage:

1. **Rust Source → HIR** (`rustc`): parsing and macro expansion. The `verus!` macro (in `builtin_macros`) rewrites Verus-extended syntax into standard Rust with `#[verifier(...)]` attributes. `rustc` produces HIR.
2. **HIR → VIR-AST** (`rust_verify::rust_to_vir`): the only crate that interacts with `rustc`. Converts HIR items (functions, structs, enums, traits) into the Verification Intermediate Representation. Extracts Verus attributes, converts types, parses specifications (requires/ensures/invariants), and handles mode annotations.
3. **VIR simplification** (`vir`): prune unused definitions, merge external traits, structural validation (`well_formed`), mode checking (`modes::check_crate` — Spec/Proof/Exec), and optimization passes (`ast_simplify`).
4. **VIR-AST → VIR-SST** (`vir::ast_to_sst`): statement extraction (move statements out of expressions), variable renaming (eliminate shadowing), local declaration collection. SST expressions cannot contain statements, simplifying AIR translation.
5. **Bucketing and parallelization** (`vir::buckets`): functions are grouped into buckets; each bucket runs in a separate thread with its own SMT solver. The `#[spinoff_prover]` attribute also creates separate buckets.
6. **VIR-SST → AIR** (`vir::sst_to_air_func`): per function — generate declarations for parameters/return values, generate axioms from specifications, convert body to AIR assertions (Assume, Assert, Havoc, Assign).
7. **AIR → SMT-LIB → Z3** (`air::context`): build SMT-LIB declarations and assertions; for each assertion, push scope, add assumptions (preconditions), assert goal (postcondition), `(check-sat)`, pop scope. Parse Z3/cvc5 response. `--rlimit` sets the per-function Z3 resource budget.

When `--compile` is passed, a second `rustc` pass erases ghost code (spec/proof functions, `Ghost<T>`/`Tracked<T>` contents, specifications, assertions) and produces a normal executable via MIR → LLVM → linking.

## TCB / guarantees

Verus is **not** a foundational verifier — correctness depends on a trusted computing base (TCB). The guarantees chapter (`source/docs/guide/src/tcb.md`, `memory-safety.md`, `call-from-unverified-code.md`) documents what is trusted and what is proved.

**Trusted (assumed):**
- `rustc`'s borrow checker and type checker (Verus reuses these for memory/aliasing and type reasoning).
- Verus's verification-condition generation (HIR → VIR → SST → AIR → SMT-LIB). A bug in VC gen could admit incorrect code.
- Ghost code erasure (the `EraseMacro` callback that removes spec/proof code before compilation). Erasure must be sound — ghost code must not affect exec behavior.
- The SMT solver (Z3 4.12.5, experimentally cvc5). A solver bug or `Canceled`/timeout could mask a real failure.
- Explicit assumptions: `assume` statements, `#[verifier::external_body]` functions (axioms), `#[verifier::external_fn_specification]`, and `#[verifier::external]` items. These introduce unverified assumptions the developer must justify.

**Proved (discharged by Z3):**
- Functional correctness: every `exec` function satisfies its `requires`/`ensures` clauses for all inputs meeting the preconditions.
- Loop invariants hold on every iteration.
- Termination (via `decreases` clauses).
- Absence of overflow (when specified).
- Memory safety **conditional on verification**: Verus has no safe/unsafe distinction. If code verifies, it is memory safe and executes according to its specifications. If it fails to verify, "all bets are off" (`memory-safety.md`). Verified code may use `unsafe` internally (e.g., `get_unchecked`) provided the `requires` clause rules out the UB case.

**Not proved / out of scope:**
- Verus does not verify itself (the verifier's own implementation).
- Verus does not verify the Rust/LLVM compilers.
- Verus does not support all Rust features and libraries (a subset only).
- Calling verified code from **unverified** code is not checked at call sites — the developer must meet `requires` clauses and trait `ensures` obligations manually (`call-from-unverified-code.md`). Memory safety of a verified program is conditional on verification; unverified callers that violate contracts can introduce UB.

## Corpus map

This corpus comprises 11 documents: this index, the source-map, and 9 topic docs.

| Doc | Purpose |
|---|---|
| `index.md` (this file) | Corpus index: what Verus is, the pipeline, TCB, corpus map, version pins, related corpora. |
| `source-map.md` | Provenance: maps every source (local clone path + upstream URL) to the docs that use it. |
| `getting-started.md` | Installing Verus, the toolchain, building, running the verifier, IDE setup, first verification. |
| `verification-model.md` | The verification model: requires/ensures/invariant, ghost code erasure, the three modes (spec/proof/exec), the `verus!` macro, mathematical types (`int`/`nat`). |
| `specifications.md` | The specification language: spec functions, quantifiers (`forall`/`exists`/`choose`), triggers, operators, equality, views (`@`), Seq/Set/Map/ISet/IMap/Multiset. |
| `proofs.md` | Proof constructs: `assert`/`assume`, `assert ... by`, proof functions, induction, `reveal`/`hide`, `opaque`, `calc`, `assert_by_compute`. |
| `arithmetic-and-provers.md` | Integer arithmetic, nonlinear arithmetic, bit vectors, prover modes (`bit_vector`, `nonlinear`, `integer_ring`, `compute`), overflow proofs, Singular integration. |
| `vstd-library.md` | The verified standard library (`vstd`): module map, core types (Seq/Set/Map/Multiset), arithmetic, concurrency primitives, memory (raw pointers, cells), std_specs. |
| `rust-features.md` | Supported and unsupported Rust features: mutation/borrowing, traits, iterators, higher-order functions, unsafe code, complex ownership, strings, macros. |
| `concurrency-and-state-machines.md` | Concurrent verification: atomics, ghost atomics, tokens, the state-machine macro language, VerusSync tokenization, invariants, inductive proofs, refinements. |
| `proof-engineering.md` | Proof engineering practice: managing verification performance, quantifier profiling, opacity/reveal, breaking proofs into pieces, SMT failures and automation limits, LLM-assisted proof, the "proofs went wrong" checklist. |

## Versions pinned

- **Rust toolchain:** `1.96.0` (from `rust-toolchain.toml`; components: `rustc`, `rust-std`, `cargo`, `rustfmt`, `rustc-dev`, `llvm-tools`). Note: the INSTALL.md binary-release path references an older `1.86.0` toolchain for end-users; building from source uses the `rust-toolchain.toml` pin of `1.96.0`.
- **Z3:** `4.12.5` (from `BUILD.md`: "Make sure you get Z3 4.12.5"). Set via `VERUS_Z3_PATH` or fetched by `tools/get-z3.sh`.
- **Verus:** calendar-versioned (e.g., `verus-0.2025.06.24.77d5bbe`). Releases are weekly point releases or a rolling pre-release tracking `main`. For reproducibility, pin a dated release tag rather than tracking `main`.

## Related corpora

- [`docs/rust/testing.md`](../../rust/testing.md) — Rust testing overview (unit/integration/doctests, `cargo test`). Verification is distinct from testing: testing samples inputs, verification proves for all inputs. The two are complementary.
- [`docs/testing/property-based-testing/`](../../testing/property-based-testing/index.md) — Property-based testing (PBT). PBT generates many random inputs to find counterexamples; Verus proves no counterexample exists. PBT is cheaper and finds bugs fast; Verus is stronger but slower and requires specifications. See the PBT corpus's cross-language recommendation for when PBT suffices.
- [`docs/rust/`](../../rust/index.md) — The Rust language corpus (ownership, types, unsafe, async, Cargo, etc.). Verus builds on Rust's syntax and type system; consult the Rust corpus for the underlying language mechanics Verus reuses.
