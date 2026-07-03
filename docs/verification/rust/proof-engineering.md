# Proof Engineering

Verus translates a program's assertions and specifications into **verification conditions** (VCs) checked by the SMT solver Z3. Because SMT solving is *undecidable* in general, proofs can fail or run slowly for reasons that have nothing to do with correctness. This page covers the practical engineering of Verus proofs: diagnosing SMT failures, managing performance, the proof checklist, the unverified-code boundary, the trusted computing base, and common anti-patterns.

For the proof tactics themselves (`assert`, `assert by`, `calc`, lemmas, `broadcast`), see [proofs.md](./proofs.md). For `spec`/`proof`/`exec` modes, see [verification-model.md](./verification-model.md).

## SMT failures: why proofs fail

The first reason a proof might fail is that the statement is simply *wrong* — a bug in a specification or assertion. Beyond that, the core reason for verification failures is that proving VCs is undecidable. The `smt_failures.md` guide enumerates the common causes:

- **Quantifiers.** Proving theorems with `exists`/`forall` is undecidable. Verus relies on Z3's pattern-based instantiation ("triggers"). See [proofs.md](./proofs.md) and the upstream `forall.md`/`trigger-annotations.md` for trigger mechanics.
- **Opaque and closed functions.** VCs hide the bodies of `opaque`/`closed` functions by default; revealing them may be necessary but is left to the user for performance and hiding control.
- **Inductive invariants.** Reasoning about recursion (loops, recursive lemmas) requires an inductive invariant that Z3 cannot synthesize on its own.
- **Extensional equality assertions.** Theorems requiring extensional equality (between sequences, maps, spec functions) typically need explicit equality assertions — Z3 cannot attempt all pairs. Use `=~~=` / `=~=` (see upstream `extensional_equality.md`).
- **Incomplete axioms.** The `Map`/`Seq` axioms may be missing a property you intuitively expect, or it may require an inductive proof.
- **Slow proofs.** Z3 may find a proof but exceed its resource limit (`rlimit`). This is treated as a failure.

## Nonlinear arithmetic

Z3's default mode is excellent at *linear* integer arithmetic (equalities, inequalities, `+`/`-`, multiplication/division by constants) but weak on *nonlinear* expressions like `x * y` or `x / y` (neither operand a constant). Verus *intentionally* disables nonlinear theories by default, so many common axioms are inaccessible:

- `x * y == y * x`
- `x * (y * z) == (x * y) * z`
- `x * (a + b) == x * a + x * b`
- `0 <= x <= y && 0 <= z <= w ==> x * z <= y * w`

You can **opt in** to nonlinear reasoning via two specialized prover modes (invoked with `by`):

- `nonlinear_arith` — enables Z3's nonlinear theory of arithmetic. General-purpose but "somewhat unpredictable" (hence off by default). Usable inline (`assert(...) by(nonlinear_arith)`) or on a `proof fn`. The `by` query does *not* include ambient facts except those inferred from types or supplied via an explicit `requires` clause.
- `integer_ring` — a decidable, equational theory of rings, discharged by the external [Singular](https://www.singular.uni-kl.de/) solver. Only `int` parameters; no inequalities; no division; function calls treated as uninterpreted. Only invocable as `proof fn ... by(integer_ring)`, not inline.

A common pattern combines both: an `integer_ring` helper lemma provides modular-arithmetic facts, and a `by(nonlinear_arith)` main lemma uses them. If neither works, invoke verified lemmas from `vstd::arithmetic` (e.g., `lemma_mul_is_commutative`, `lemma_mul_is_associative`, `lemma_mul_is_distributive_add`, `lemma_mul_upper_bound`).

## Performance and profiling

### The resource limit (`rlimit`)

Z3's computation budget is the `rlimit` — a machine-independent resource limit (not wall-clock time). The default is `10`, "meant to be around 2 seconds." Configure per-function or globally:

- `#[verifier::rlimit(n)]` on a function (or `#[verifier::rlimit(infinity)]` to remove the limit).
- `--rlimit N` on the command line.

The `smt_perf_overview.md` guide urges *not* tolerating slow verification: "Slow verification performance typically has an underlying cause. Diagnosing and fixing the cause is much easier to do as the problems arise; waiting until you have multiple performance problems compounds the challenges."

### Measuring performance

- `--time` — detailed breakdown of where Verus spends time.
- `--time-expanded` — even more detail.
- `--output-json` — machine-readable output (also reports SMT rlimit usage).

### Quantifier profiling

When verification times out or is slow, quantifier over-instantiation is a common cause. The built-in profiler launches automatically on timeout with `--profile`; use `--profile-all` to profile functions that verify slowly. The guide strongly recommends combining with `--rlimit 1` to limit profiling data. Combine with `--verify-function` to target a specific function.

The profiler reports, per quantifier: the number of instantiations and a "cost" metric (sum of instantiation costs, where an instantiation's cost is `1 + sum of costs of instantiations it caused`, weighted by in-degree). It sorts by the product of these two metrics. A quantifier with a huge instantiation count and high cost is the troublemaker; a quantifier with the same count but low cost is an "innocent bystander." If *all* quantifiers have few instantiations, quantifier instantiation is likely not the bottleneck.

### Isolating a function's query

`#[verifier::spinoff_prover]` isolates a function's SMT query into its own prover instance. This can make "flaky" proofs (that sometimes work and sometimes break on unrelated changes) more stable, and can improve performance by reducing the context bleed between functions.

### Targeted verification

- `--verify-only-module <module>` — verify only a specific module.
- `--verify-function <function>` — verify only a specific function.

These are essential for large projects and for iterating on a single proof. (The upstream `broadcast_proof.md` notes: "For large projects, use `--verify-only-module` and possibly `--verify-function` to limit the scope.")

### `opaque` / `reveal` / `hide` for performance and modularity

Unfolding a complex spec function — especially one with problematic quantifiers — is a common cause of timeouts. Mark the function `#[verifier::opaque]` to hide its body from the solver by default, then `reveal(f)` to selectively unfold it where needed. `reveal_with_fuel(f, n)` controls how many times a *recursive* function is unfolded (default fuel is 1; limiting it avoids trigger loops). `hide(f)` treats `f` as uninterpreted.

```rust
// Hide by default, reveal locally:
#[verifier::opaque]
spec fn complex(x: int) -> bool { /* ... */ }

proof fn p() {
    reveal(complex);
    // ... solver now unfolds complex's definition here ...
}
```

Use `closed spec` (body available in the current module only) for *abstraction*; use `opaque`/`reveal` for *automation and performance control*. See [proofs.md](./proofs.md) for the full `opaque`/`reveal`/`hide` reference.

## Breaking proofs into pieces

Solver response time grows *nonlinearly* with proof size — twice as many facts gives far more than twice as many search paths. Breaking a long proof function into smaller lemmas can make the difference between a timeout and a fast success.

**Move a subproof to a lemma.** Extract a modest-size piece `P` that establishes facts `S` into a `proof fn` whose `ensures` is `S` and whose `requires` captures the necessary context. The dedicated lemma has a smaller context, so it may need *less* annotation than the original inline proof.

**Divide into parts 1..n.** Split a large proof into `n` consecutive lemmas, where each lemma's `ensures` matches the next lemma's `requires`, and the last lemma's `ensures` is the original function's. Replace the body with a sequence of calls. Factor repeated expressions (`r(x)`, `mid1(x, y)`, …) into spec functions to avoid duplication.

## The proof checklist

When a proof fails unexpectedly, work through this checklist (quoted verbatim from `checklist.md`):

> **A proof is failing and I don't expect it to. What's going wrong?**
>
>  * Try running Verus with `--expand-errors` to get more specific information about what's failing.
>  * Check Verus's output for `recommends`-failures and other notes.
>  * Add more `assert` statements. This can either give you more information about what's failing, or even just fix the proof. See [this guide](https://verus-lang.github.io/verus/guide/develop_proofs.html).
>  * Are you using quantifiers? Make sure you understand [how triggers work](https://verus-lang.github.io/verus/guide/forall.html).
>  * Are you using nonlinear arithmetic? Try one of the strategies for [nonlinear arithmetic](https://verus-lang.github.io/verus/guide/nonlinear.html).
>  * Are you using bitwise arithmetic or `as`-truncation? Try [the bit_vector solver](https://verus-lang.github.io/verus/guide/bitvec.html).
>  * Are you relying on the equality of a container type (like `Seq` or `Map`)? Try [extensional equality](https://verus-lang.github.io/verus/guide/extensional_equality.html).
>  * Are you using a recursive function? Make sure you understand [how fuel works](https://verus-lang.github.io/verus/guide/recursion.html).
>
> **The verifier says "rlimit exceeded". What can I do?**
>
>  * Try [the quantifier profiler](https://verus-lang.github.io/verus/guide/profiling.html) to identify a problematic trigger-pattern.
>  * Try [breaking the proof into pieces](https://verus-lang.github.io/verus/guide/breaking_proofs_into_pieces.html).
>  * Try [increasing the `rlimit`](https://verus-lang.github.io/verus/guide/reference-attributes.html). Sometimes a proof really is just kind of big and you want Verus to spend a little more effort on it.
>
> **My proof is "flaky": it sometimes works, but then I change something unrelated, and it breaks.**
>
>  * Try adding `#[verifier::spinoff_prover]` to the function. This can make it a little more stable.
>  * Try [breaking the proof into pieces](https://verus-lang.github.io/verus/guide/breaking_proofs_into_pieces.html).

*"Links in the quoted checklist point to the upstream Verus guide."*

Key flags referenced: `--expand-errors` (more specific failure info), `--no-cheating` (disallow `assume` statements), `--profile`/`--profile-all` (quantifier profiler).

## LLM assistants for Verus proofs

The `llmforverusproof.md` guide documents the current state of LLM-aided proof writing. Recommended setup:

1. **Use a coding agent** (not direct model API calls) — the observe-adjust loop (run Verus, read errors, edit) is essential.
2. **Provide Verus resources** (`vstd`, the test suite `rust_verify_test`, the Guide) so the model can learn syntax and find helper lemmas rather than hallucinating them.
3. **Provide a Verus binary** so the model can run Verus and see error output. `--expand-errors` gives more detailed feedback.
4. **Use a cheat checker** to catch invalid shortcuts.

A simple, effective prompt:

```
The file X.rs cannot be verified by Verus yet.
Please add proof annotations so that it can be successfully verified by Verus.
The vstd folder contains Verus standard library definitions and helper lemmas.
Keep editing your proof until Verus shows no errors.
Do NOT change existing specifications (requires/ensures) or executable code.
Do NOT use assume(...) or admit(...).
Before you finish, run the cheat checker to make sure you haven't cheated.
```

Common LLM cheating methods to guard against (verbatim table):

| Cheat Type | What It Looks Like |
|---|---|
| Using `assume()` or `admit()` | Assumes properties without proof |
| Adding `external_body` tags | Skips verification of function bodies |
| Adding `axiom` tags | Assumes lemmas without proof |
| Changing specifications | Strengthens preconditions or weakens postconditions |
| Changing executable code | Changes Rust code to make verification easier, which may change program semantics, reduce performance, or harm code readability |

**Model capabilities:** Older models (GPT-4o, GPT o4-mini) are "poor at Verus syntax and general verification skills." Newer models (Claude Sonnet 4.5, Claude Opus 4.5) "can successfully fill in proof annotations for most proof lemmas in existing Verus-verified system projects." LLMs tend to fail on: inductive invariants, procedural macro expansion, heavy abstraction (closed/opaque functions), and very large tasks. Expect LLM-generated proofs to be 2–3× longer than human-written ones (unnecessary "safety" assertions, explicit reasoning steps). Cost can reach "a few hundred dollars to generate a proof, sometimes incomplete, for just one function." Some models (Opus series) tend to give Verus huge rlimits (e.g., 2000) and wait hours — prompt them to break proofs into smaller steps instead.

## Interacting with unverified code

Most projects verify only a portion of the codebase. Verus provides several mechanisms for the verified/unverified boundary.

### Calling unverified code from verified code

`#[verifier::external_body]` tells Verus to process a function's *specification* without verifying its body — Verus *assumes* the spec without proof. Verbatim example from `calling-unverified-from-verified.md`:

```rust
#[verifier::external_body]
fn fib_impl(n: u64) -> (result: u64)
    requires
        fib(n as nat) <= u64::MAX
    ensures
        result == fib(n as nat),
{
    if n == 0 {
        return 0;
    }
    let mut prev: u64 = 0;
    let mut cur: u64 = 1;
    let mut i: u64 = 1;
    while i < n {
        i = i + 1;
        let new_cur = cur + prev;
        prev = cur;
        cur = new_cur;
    }
    cur
}
```

To apply a spec to an *existing* library function (so you can call it by its real name), use the `assume_specification` directive (verbatim):

```rust
pub assume_specification<T>[ std::mem::swap::<T> ](a: &mut T, b: &mut T)
    ensures
        *a == *old(b),
        *b == *old(a);
```

`vstd` already provides specs for much of the standard library (documented in `vstd/std_specs`). For types Verus doesn't recognize, use `#[verifier::external_type_specification]`; for external traits, `#[verifier::external_trait_specification]` and `#[verifier::external_trait_extension]`.

### Calling verified code from unverified code

Any `requires` clause on a function callable from unverified code is an **assumption** about what the caller will pass in. The stronger the precondition, the more likely a caller fails to meet it, undermining all verification work. Ideally, external APIs should have *no* preconditions. The flag `-V check-safe-api` checks whether a crate is unconditionally safe to call from any unverified (non-`unsafe`) code.

To eliminate preconditions: wrap the API in a function with no preconditions that dynamically checks and calls an `unsafe` inner function with the preconditions. Verbatim pattern:

```rust
pub unsafe fn index_unchecked<T>(vec: &Vec<T>, i: usize) -> &T
    requires i < vec.len()
{
    /* ... */
}

pub fn index<T>(vec: &Vec<T>, i: usize) -> Option<&T>
{
    if i < vec.len() {
        Some(index_unchecked(vec, i))
    } else {
        None
    }
}
```

For stateful APIs, use a public struct with private fields so callers cannot forge or mutate state. The `Drop` trait is treated as if it has signature `fn drop(&mut self) opens_invariants none no_unwind` — verified `Drop` impls are checked against this; unverified impls are the user's responsibility.

### The trust boundary

Calling verified code from unverified code means Verus cannot check contracts at each call-site. The developer must meet: (1) any `requires` clauses, and (2) any trait impl used to meet trait bounds must satisfy the trait's `ensures`. **Memory safety is conditional on verification** — calling verified code from unverified code could be non-memory-safe if the unverified code fails to uphold contracts.

## Ghost erasure

Verus performs **ghost erasure**: ghost code (`spec`, `proof`, `ghost`/`tracked` variables) is removed when building the executable. A byproduct: identifiers that exist at verification time may not exist at compile time. A `use` of a `spec fn` in an `exec` context fails to *compile*:

```rust
pub mod ghost_mod {
    pub closed spec fn ghost_fn() -> bool { true }
}

pub mod test_mod {
    use crate::ghost_mod::ghost_fn;
//  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//  FAILS: During compilation, ghost_fn is erased, causing rustc
//         to complain about a missing definition
    pub fn exec_fn() -> u64 { 1 }
}
```

Guard such `use` statements with `#[cfg(verus_only)]` (on during verification, off otherwise). Add to `Cargo.toml` to silence the `unexpected_cfgs` warning:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = [
  'cfg(verus_only)',
] }
```

> **CAUTION:** `verus_only` should *only* guard `use` statements and config attributes. Using it for conditional compilation of code introduces **unsoundness** — e.g., a function that returns `0` under `verus_only` and `42` otherwise will *verify successfully* yet return `42` at runtime.

## Guarantees and the trusted computing base (TCB)

### What Verus proves

Verus verifies **functional correctness** — that a program meets its `requires`/`ensures` specifications and `assert` statements — *and* memory safety, through specifications rather than a safe/unsafe distinction. From `memory-safety.md`: "In Verus, there is no staggered notion of correctness. If the program verifies, then it is memory safe, and it will execute according to all its specifications. If the program fails to verify, then all bets are off."

Rust's borrow checker is **trusted** for memory safety; Verus adds functional correctness on top. A Verus `Vec::index` can omit the runtime bounds check (using `get_unchecked`) because its `requires i < self.len()` is enforced at every call-site by verification — but type-checking alone is insufficient.

### What Verus trusts (the TCB)

Assumptions are introduced through: `assume` statements; axioms (`#[verifier::external_body]` proof functions); axiomatic specs (`#[verifier::external_body]` or `assume_specification` exec functions); and `#[verifier::external]` (ignore an item entirely). The ultimate correctness depends on these assumptions being true.

The TCB includes: **rustc** (the Rust compiler), **Z3** (the SMT solver), the **Verus pipeline** (VC generation, erasure), and any `external_body`/`assume_specification`/`axiom` declarations. Verus is **not foundational** — it does not produce a proof artifact checkable by a small trusted kernel; it trusts that its encoding of Rust into SMT is sound and that Z3 is sound.

### What Verus does NOT prove

- **Termination** of `exec` functions is checked (via `decreases`), but `#[verifier::assume_termination]` can override this.
- **Deadlock freedom** — e.g., the verified `RwLock` explicitly does *not* verify absence of deadlocks.
- **Properties of unverified code** called from verified code (the contracts are assumptions).
- **Anything relying on layout** — Verus has no access to type layout unless provided via the `global` directive (which exports `size_of`/`align_of` axioms and creates a static check at codegen time).

## Common anti-patterns

- **Missing `decreases`.** `exec` functions with recursion or loops need a `decreases` clause (unless `#[verifier::exec_allows_no_decreases_clause]`). Omitting it is a verification failure.
- **Wrong triggers / matching loops.** A quantifier whose trigger re-instantiates itself (e.g., `forall|x, y| f(x+1, 2*y) && f(2*x, y+x) || f(y, x) ==> #[trigger] f(x, y)`) causes an infinite instantiation cycle and a timeout. Use `--profile` to find it; tighten triggers or use `reveal_with_fuel` for recursive functions.
- **Treating `recommends` as enforced.** `recommends`-checks (e.g., that an `as`-cast is in range) are *not* hard errors — they are elided if the enclosing function has no legitimate verification errors. Do not assume a passing `recommends` means the property is proved. Use `#[verifier::truncate]` to silence truncation recommends-checks when truncation is intended.
- **Treating a `proof fn` as deterministic.** Proof functions are ghost; their exec-level behavior is erased. Do not rely on a proof function's "return value" at runtime.
- **Forgetting `nonlinear_arith`.** `x * y == y * x` is *not* provable in the default mode. Either invoke `by(nonlinear_arith)`, use `integer_ring`, or call a `vstd::arithmetic` lemma.
- **Relying on extensional equality implicitly.** Two `Seq`/`Map`/`Set` expressions that are equal are not automatically proved equal — add an explicit `assert(a =~= b)`.
- **Using `assume`/`admit` in real proofs.** These are unchecked and subvert all guarantees. The `--no-cheating` flag disallows `assume`.
- **Conditional compilation with `verus_only` for code.** This is unsound (a function can verify yet return a different value at runtime). Only guard `use` statements and config attributes.
- **Huge `rlimit` as a first resort.** Raising `--rlimit` is a last resort for "rlimit exceeded." First profile quantifiers and break the proof into pieces; otherwise you mask the underlying cause and slow your development cycle.

## Runnable example

A debugging/profiling command for a failing proof, combining the most useful diagnostic flags:

```bash
# Verify a single function with expanded errors, multiple errors shown,
# a raised rlimit, and the quantifier profiler on timeout:
verus src/my_proof.rs \
  --verify-function my_lemma \
  --expand-errors \
  --multiple-errors 5 \
  --rlimit 30 \
  --profile

# To profile a function that verifies slowly (not on timeout), use --profile-all:
verus src/my_proof.rs --verify-function my_lemma --profile-all --rlimit 1

# Machine-readable timing + rlimit breakdown:
verus src/my_proof.rs --time --output-json

# Record a reproducible trace (sources + output + version) for a bug report:
verus src/my_proof.rs --record
```

Flag reference: `--expand-errors` (specific failure info), `--multiple-errors N` (show up to N errors), `--rlimit N` (Z3 resource budget; default 10 ≈ 2s), `--profile` (quantifier profiler on timeout), `--profile-all` (profiler even on success), `--verify-function`/`--verify-only-module` (targeted verification), `--time`/`--time-expanded` (timing breakdown), `--output-json` (machine-readable), `--record` (package a reproducible trace into a zip), `--no-cheating` (disallow `assume`).

## Sources used

- `.tmp/verus/source/docs/guide/src/smt_failures.md`
- `.tmp/verus/source/docs/guide/src/smt_perf_overview.md`
- `.tmp/verus/source/docs/guide/src/performance.md`
- `.tmp/verus/source/docs/guide/src/profiling.md`
- `.tmp/verus/source/docs/guide/src/checklist.md`
- `.tmp/verus/source/docs/guide/src/nonlinear.md`
- `.tmp/verus/source/docs/guide/src/breaking_proofs_into_pieces.md`
- `.tmp/verus/source/docs/guide/src/erasure.md`
- `.tmp/verus/source/docs/guide/src/llms.md`
- `.tmp/verus/source/docs/guide/src/llmforverusproof.md`
- `.tmp/verus/source/docs/guide/src/interacting-with-unverified-code.md`
- `.tmp/verus/source/docs/guide/src/calling-unverified-from-verified.md`
- `.tmp/verus/source/docs/guide/src/calling-verified-from-unverified.md`
- `.tmp/verus/source/docs/guide/src/call-from-unverified-code.md`
- `.tmp/verus/source/docs/guide/src/guarantees.md`
- `.tmp/verus/source/docs/guide/src/tcb.md`
- `.tmp/verus/source/docs/guide/src/memory-safety.md`
- `.tmp/verus/source/docs/guide/src/reference-attributes.md`
- `.tmp/verus/source/docs/guide/src/reference-global.md`
- `.tmp/verus/source/docs/guide/src/reference-assume-specification.md`
- `.tmp/verus/source/docs/guide/src/reference-flag-record.md`
- `.tmp/verus/source/docs/guide/src/reference-reveal-hide.md`
- `.tmp/verus/source/docs/guide/src/opaque.md`
- `.tmp/verus/source/docs/guide/src/exec_attr.md`
- `.tmp/verus/source/docs/guide/src/cargo_verus.md`
- `.tmp/verus/source/docs/guide/src/broadcast_proof.md`
