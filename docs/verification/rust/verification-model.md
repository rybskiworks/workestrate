# The Verus Verification Model

## Purpose

This is the core reference for Verus's verification model: the three code modes (`spec`/`proof`/`exec`), the `verus!` macro, `requires`/`ensures` and modular verification, `assert`/`assume`, loop invariants, `decreases`/fuel and termination, ghost/tracked variables, `open`/`closed` spec functions, and ghost erasure. For the specification language itself (operators, quantifiers, triggers, `int`/`nat`), see [specifications.md](./specifications.md). For proof constructs (`assert ... by`, `calc`, `broadcast`, induction), see [proofs.md](./proofs.md). For install and first-run tooling, see [getting-started.md](./getting-started.md).

> **Source fidelity:** Every code snippet below is copied verbatim from the Verus clone at `.tmp/verus/`. Code from example files is sourced from `examples/guide/*.rs` (anchor names reference the `// ANCHOR:` markers). Type signatures (e.g. `Ghost<A>`, `Tracked<A>`, `View`) are directly quoted from `source/builtin/` or `source/vstd/`. Inline examples quoted from tutorial chapters under `source/docs/guide/src/` are marked as such.

---

## The three modes: `spec`, `proof`, `exec`

Verus classifies code into three *modes*: `spec`, `proof`, and `exec`, where:

- `spec` code describes properties about programs
- `proof` code proves that programs satisfy properties
- `exec` code is ordinary Rust code that can be compiled and run

Both `spec` code and `proof` code are forms of ghost code, so we can organize the three modes in a hierarchy:

- code
    - ghost code
        - `spec` code
        - `proof` code
    - `exec` code

Every function in Verus is either a `spec` function, a `proof` function, or an `exec` function (from `examples/guide/modes.rs`, anchor `fun_modes`):

```rust
spec fn f1(x: int) -> int {
    x / 2
}

proof fn f2(x: int) -> int {
    x / 2
}

// "exec" is optional, and is usually omitted
exec fn f3(x: u64) -> u64 {
    x / 2
}
```

`exec` is the default function annotation, so it is usually omitted (i.e. `fn f3(x: u64) -> u64 { x / 2 }` is an `exec` function).

### Mode-compatibility table

The three modes have a strict calling hierarchy. `spec` is the most permissive (it can be called from anywhere); `exec` is the most restrictive (it can only be called from `exec` code):

|                        | spec code      | proof code       | exec code        |
|------------------------|----------------|------------------|------------------|
| can contain `spec` code, call `spec` functions   | yes            | yes              | yes              |
| can contain `proof` code, call `proof` functions | no             | yes              | yes              |
| can contain `exec` code, call `exec` functions   | no             | no               | yes              |

Read the table row-wise: `spec` code cannot contain `proof` or `exec` code; `proof` code cannot contain `exec` code; `exec` code can contain all three. This hierarchy ensures that ghost code (which is erased) never depends on executable code (which runs), preserving the soundness of erasure.

---

## The `verus!` macro

Verus code is Rust source that uses the `verus!` macro to embed Verus. The `verus!` macro extends Rust's syntax with verification-related features such as preconditions, postconditions, assertions, `forall`, `exists`, etc. Each file in a crate will typically take the following form:

```rust
use vstd::prelude::*;

verus! {
    // ...
}
```

The `vstd::prelude` exports the `verus!` macro along with some other Verus utilities. The `verus!` macro is a proc-macro: it parses the extended spec/proof syntax (via a forked `syn` crate that handles Verus's additional syntax), rewrites it into standard Rust with `#[verifier(...)]` attributes, and hands the result to `rustc` for type-checking and to the Verus verifier for verification-condition generation. Besides extending Rust's syntax, the `verus!` macro also *tells Verus to verify the functions contained within*. By default, Verus verifies everything inside the `verus!` macro and ignores anything defined outside the `verus!` macro.

Verus also supports an alternate, attribute-based syntax (`#[verus::spec]`, etc.), which may be helpful when minimizing changes to an existing Rust project. However, the `verus!` syntax is cleaner and simpler and is the recommended form.

---

## `requires` / `ensures` and modular verification

### Preconditions (`requires`)

Preconditions (also known as "`requires` clauses") specify which argument values are allowed when calling a function. In Verus, preconditions are written with `requires` followed by zero or more boolean expressions separated by commas. Consider a function `octuple` that multiplies a number by 8. Without a precondition, Verus reports an error about possible arithmetic overflow:

```
error: possible arithmetic underflow/overflow
   |
   |     let x2 = x1 + x1;
   |              ^^^^^^^
```

Adding `requires -16 <= x1 < 16` constrains the input so that all the intermediate additions fit in `i8`.

### Postconditions (`ensures`)

Postconditions (`ensures` clauses) specify properties of a function's return value. To write a property about the return value, give the return value a name with the syntax `-> (name: return_type)`. The complete, verifying example (verbatim from `examples/guide/requires_ensures.rs`):

```rust
use vstd::prelude::*;

verus! {

#[verifier::external_body]
fn print_two_digit_number(i: i8)
    requires
        -99 <= i < 100,
{
    println!("The answer is {}", i);
}

fn octuple(x1: i8) -> (x8: i8)
    requires
        -16 <= x1 < 16,
    ensures
        x8 == 8 * x1,
{
    let x2 = x1 + x1;
    let x4 = x2 + x2;
    x4 + x4
}

fn main() {
    let n = octuple(10);
    assert(n == 80);
    print_two_digit_number(n);
}

} // verus!
```

Here, `-> (x8: i8)` allows the postcondition `x8 == 8 * x1` to use the name `x8` for `octuple`'s return value. Verus also lets you chain multiple inequalities together in a single expression (e.g. `-16 <= x1 < 16` is equivalent to `-16 <= x1 && x1 < 16`); see [specifications.md](./specifications.md) for chained operators.

### Modular verification

Preconditions and postconditions establish a modular verification protocol between functions. When `main` calls `octuple`, Verus checks that the arguments in the call satisfy `octuple`'s preconditions. When Verus verifies the body of `octuple`, it can **assume** that the preconditions are satisfied, without knowing anything about the exact arguments passed in by `main`. Likewise, when Verus verifies the body of `main`, it can **assume** that `octuple` satisfies its postconditions, without knowing anything about the body of `octuple`. In this way, Verus verifies each function independently. This modular verification approach breaks verification into small, manageable pieces, which makes verification more efficient than verifying all functions together simultaneously.

---

## `assert` and `assume`

While `requires` and `ensures` connect functions together, `assert` makes a local, private request to the SMT solver to prove a certain fact. (Note: `assert(...)` should not be confused with the Rust `assert!(...)` macro — the former is statically checked using the SMT solver, while the latter is checked at run-time.)

`assert` has an evil twin named `assume`, which asks the SMT solver to simply accept some boolean expression as a fact without proof. While `assert` is harmless and will not cause any unsoundness in a proof, `assume` can easily enable a "proof" of a fact that is not true. In fact, by writing `assume(false)`, you can prove anything you want:

```rust
assume(false);
assert(2 + 2 == 5); // succeeds
```

Verus programmers often use `assert` and `assume` to help develop and debug proofs. They may add temporary `assert`s to determine which facts the SMT solver can prove and which it cannot, and they may add temporary `assume`s to see which additional assumptions are necessary for the SMT solver to complete a proof, or as a placeholder for parts of the proof that have not yet been written. As the proof evolves, the programmer replaces `assume`s with `assert`s, and may eventually remove the `assert`s. A complete proof may contain `assert`s, but should not contain any `assume`s.

(In some situations, `assert` can help the SMT solver complete a proof, by giving the SMT hints about how to manipulate `forall` and `exists` expressions. There are also special forms of `assert`, such as `assert(...) by(bit_vector)`, to help prove properties about bit vectors, nonlinear integer arithmetic, `forall` expressions, etc. See [proofs.md](./proofs.md) for `assert ... by` and [arithmetic-and-provers.md](./arithmetic-and-provers.md) for prover modes.)

### `#[verifier::external_body]` — the verified/unverified boundary

In the `octuple` example above, `print_two_digit_number` is marked `#[verifier::external_body]`. This tells Verus to pay attention to the function's preconditions and postconditions but ignore the function's body. This is common in projects using Verus: you may want to verify some of it (perhaps the program's core algorithms), but leave other aspects, such as input-output operations, unverified. More generally, since verifying all the software in the world is still infeasible, there will be some boundary between verified code and unverified code, and `#[verifier::external_body]` can be used to mark this boundary.

---

## Loop invariants (`invariant`)

Verus verifies loops separately from the enclosing function (internally, it treats the loop as its own function). Where a recursive function has preconditions, a loop has *loop invariants* that describe what must be true before and after each iteration. For example, if `n = 10`, then the loop invariant must be true 11 times: before each of the 10 iterations, and after the final iteration.

The following `while` loop (from `examples/guide/recursion.rs`, anchor `loop`) computes the triangular number iteratively, with three invariants:

```rust
fn loop_triangle(n: u32) -> (sum: u32)
    requires
        triangle(n as nat) <= u32::MAX,
    ensures
        sum == triangle(n as nat),
{
    let mut sum: u32 = 0;
    let mut idx: u32 = 0;
    while idx < n
        invariant
            idx <= n,
            sum == triangle(idx as nat),
            triangle(n as nat) <= u32::MAX,
        decreases n - idx,
    {
        idx = idx + 1;
        assert(sum + idx <= u32::MAX) by {
            triangle_is_monotonic(idx as nat, n as nat);
        }
        sum = sum + idx;
    }
    sum
}
```

Notice that the invariant `idx <= n` allows for the possibility that `idx == n`, since this will be the case after the final iteration. After the loop exits, Verus knows that `idx <= n` (because of the loop invariant) and that the loop condition `idx < n` must have been false. Putting these together allows Verus to prove that `idx == n` after exiting the loop. Since we also have the invariant `sum == triangle(idx as nat)`, Verus can then substitute `n` for `idx` to conclude `sum == triangle(n as nat)`, which proves the postcondition.

Because the loop is verified separately, it does not automatically inherit preconditions like `triangle(n as nat) <= u32::MAX` from the surrounding function — if the loop relies on these preconditions, they must be listed explicitly in the loop invariants. (This improves SMT-solving efficiency for large functions with large loops. You can opt out with `#[verifier::loop_isolation(false)]`.)

> See [proofs.md](./proofs.md) for `assert(...) by { ... }`, `invariant_except_break`, and techniques for devising loop invariants.

---

## `decreases`, fuel, and termination

### Termination via `decreases`

In order to ensure soundness, a recursive `spec` function must terminate on all inputs — infinite recursive calls are not allowed. (If Verus accepted a nonterminating definition like `spec fn bogus(i: int) -> int { bogus(i) + 1 }`, you could prove `false`, because the definition insists `bogus(3) == bogus(3) + 1`, which implies `0 == 1`.) To help prove termination, Verus requires that each recursive `spec` function definition contain a `decreases` clause (from `examples/guide/recursion.rs`, anchor `spec`):

```rust
spec fn triangle(n: nat) -> nat
    decreases n,
{
    if n == 0 {
        0
    } else {
        n + triangle((n - 1) as nat)
    }
}
```

Each recursive call must decrease the expression in the `decreases` clause by at least 1. Furthermore, the call cannot cause the expression to decrease below 0. With these restrictions, the expression in the `decreases` clause serves as an upper bound on the depth of calls that `triangle` can make to itself, ensuring termination.

For mutually recursive functions, the number of elements in the decreases-measure must be the same across the mutually recursive collection, and the measures are compared lexicographically. The `via` clause lets you supply a separate `#[via_fn]` proof function to demonstrate that the decreases-measure actually decreases at each recursive call site (useful when Verus cannot prove termination automatically, e.g. for bit-shift recursion).

### Fuel and reasoning about recursive functions

Given the definition of `triangle` above, the assertion `assert(triangle(0) == 0)` succeeds, but somewhat surprisingly, `assert(triangle(10) == 55)` **fails**, despite the fact that `triangle(10)` really is equal to 55. This is a limitation of automated reasoning: SMT solvers cannot automatically prove all true facts about all recursive functions.

For nonrecursive functions, an SMT solver can reason about the functions simply by inlining them. However, this strategy does not completely work with recursive functions, because inlining the function produces another expression with a call to the same function, leading to infinite inlining. To avoid this, Verus limits the number of recursive calls that any given call can spawn in the SMT solver. This limit is called the *fuel*; each nested recursive inlining consumes one unit of fuel. By default, the fuel is 1, which is just enough for `assert(triangle(0) == 0)` to succeed but not enough for `assert(triangle(10) == 55)` to succeed.

To increase the fuel to a larger amount, use the `reveal_with_fuel` directive (from `examples/guide/recursion.rs`, anchor `fuel`):

```rust
fn test_triangle_reveal() {
    proof {
        reveal_with_fuel(triangle, 11);
    }
    assert(triangle(10) == 55);
}
```

Here, 11 units of fuel is enough to inline the 11 calls `triangle(0)`, ..., `triangle(10)`. Note that even with only 1 unit of fuel, you could still prove `assert(triangle(10) == 55)` through a long series of assertions (anchor `step_by_step`):

```rust
fn test_triangle_step_by_step() {
    assert(triangle(0) == 0);
    assert(triangle(1) == 1);
    assert(triangle(2) == 3);
    assert(triangle(3) == 6);
    assert(triangle(4) == 10);
    assert(triangle(5) == 15);
    assert(triangle(6) == 21);
    assert(triangle(7) == 28);
    assert(triangle(8) == 36);
    assert(triangle(9) == 45);
    assert(triangle(10) == 55);  // succeeds
}
```

This works because 1 unit of fuel is enough to prove `assert(triangle(0) == 0)`, and then once we know that `triangle(0) == 0`, we only need to inline `triangle(1)` once to get `triangle(1) == 1`, and so on. However, it is probably best to avoid long series of assertions if you can, and instead write a proof that makes it clear why the SMT proof fails by default (not enough fuel) and fixes exactly that problem (anchor `fuel_by`):

```rust
fn test_triangle_assert_by() {
    assert(triangle(10) == 55) by {
        reveal_with_fuel(triangle, 11);
    }
}
```

> See [proofs.md](./proofs.md) for `reveal`/`hide`/`opaque` and [arithmetic-and-provers.md](./arithmetic-and-provers.md) for the `compute` prover mode as an alternative to fuel.

---

## Ghost and tracked variables

In addition to the three function modes, Verus has three *variable* modes: `exec`, `tracked`, and `ghost`. Only `exec` variables exist in the compiled code, while `ghost` and `tracked` variables are "erased" from the compiled code.

### Variable-mode compatibility

Which variables are allowed depends on the expression mode, according to the following table (from `source/docs/guide/src/reference-var-modes.md`):

|            | Default variable mode | `ghost` variables | `tracked` variables | `exec` variables |
|------------|-----------------------|-------------------|---------------------|------------------|
| spec code  | `ghost`               | yes               |                     |                  |
| proof code | `ghost`               | yes               | yes                 |                  |
| exec code  | `exec`                | yes               | yes                 | yes              |

Although `exec` code allows variables of any mode, there are restrictions: arguments and return values for an `exec` function must be `exec` mode, and struct fields of an `exec` struct must be `exec` mode. Variables marked `tracked` or `ghost` may be declared anywhere in an `exec` block, but may only be *assigned* to from inside a `proof { ... }` block.

### `Ghost<T>` and `Tracked<T>` wrappers

To mix-and-match tracked and ghost data across the exec/proof boundary, Verus provides the `Ghost<A>` and `Tracked<A>` wrapper types. These are defined in the builtin library (directly quoted from `source/builtin/src/lib.rs`):

```rust
pub struct Ghost<A> {
    // ...
}

pub struct Tracked<A> {
    // ...
}
```

`Ghost<T>` wraps a ghost-mode value so it can be stored in an exec-mode struct or passed through an exec function signature. `Tracked<T>` wraps a tracked-mode value — a linear resource token that cannot be duplicated and must be consumed exactly once. The `tracked` mode is an advanced feature used primarily in concurrent verification with permission tokens; see [concurrency-and-state-machines.md](./concurrency-and-state-machines.md).

The lower-case `tracked` keyword is used to indicate the right-hand side has `proof` mode, in order to allow a `tracked` call. The upper-case `Tracked` and `Ghost` are used in pattern matching to unwrap the inner values. For example, to return both a tracked and a ghost value from a proof function and unwrap them at the call site (directly quoted from `source/docs/guide/src/reference-var-modes.md`):

```rust
proof fn some_call() -> (tracked ret: (Tracked<X>, Ghost<Y>)) { ... }

proof fn example() {
    // The lower-case `tracked` keyword is used to indicate the right-hand side
    // has `proof` mode, in order to allow the `tracked` call.
    // The upper-case `Tracked` and `Ghost` are used in the pattern matching to unwrap
    // the `X` and `Y` objects.
    let tracked (Tracked(x), Ghost(y)) = some_call();
}
```

### Ghost code abilities

Ghost code has two particular abilities worth keeping in mind:

- Ghost code can copy values of any type, even if the type does not implement the Rust `Copy` trait.
- Ghost code can create a value of any type, even if the type has no public constructors (e.g. even if the type is a struct whose fields are all private to another module).

For example, the following `spec` functions create and duplicate values of type `S`, defined in another module with private fields and without the `Copy` trait (from `examples/guide/modes.rs`, anchor `ghost_abilities1`):

```rust
mod MA {
    // does not implement Copy
    // does not allow construction by other modules
    pub struct S {
        private_field: u8,
    }

}

mod MB {
    use verus_builtin::*;
    use crate::MA::*;

    // construct a ghost S
    spec fn make_S() -> S;

    // duplicate an S
    spec fn duplicate_S(s: S) -> (S, S) {
        (s, s)
    }

}
```

These operations are not allowed in `exec` code. Furthermore, values from ghost code are not allowed to leak into `exec` code — what happens in ghost code stays in ghost code.

---

## `open` and `closed` spec functions

Unlike `exec` functions, the bodies of `spec` functions are visible to other functions in the same module, so callers can see inside the function. Across modules, the bodies of `spec` functions can be made public to other modules or kept private to the current module:

- `pub open spec fn` — the body is public; other modules can see inside the function. Think of these as defining **abbreviations**.
- `pub closed spec fn` — the body is private; other modules can see the function's declaration but not its body. Think of these as defining **abstractions**.

All `pub` `spec` functions must be marked either `open` or `closed`; Verus will complain if the function lacks this annotation. Functions within the same module *can* view a `closed spec fn`'s body.

The following example (from `examples/guide/modes.rs`, anchor `spec_fun3`) defines `min` and `min3` as `spec` functions and `compute_min3` as an `exec` function whose postcondition is expressed in terms of the `spec` function `min3`:

```rust
spec fn min(x: int, y: int) -> int {
    if x <= y {
        x
    } else {
        y
    }
}

spec fn min3(x: int, y: int, z: int) -> int {
    min(x, min(y, z))
}

fn compute_min3(x: u64, y: u64, z: u64) -> (m: u64)
    ensures
        m == min3(x as int, y as int, z as int),
{
    let mut m = x;
    if y < m {
        m = y;
    }
    if z < m {
        m = z;
    }
    m
}

fn test() {
    let m = compute_min3(10, 20, 30);
    assert(m == 10);
}
```

The difference between `min3` and `compute_min3` highlights the difference between `spec` code and `exec` code. While `exec` code may use imperative language features like mutation, `spec` code is restricted to purely functional mathematical code. On the other hand, `spec` code is allowed to use `int` and `nat`, while `exec` code is restricted to compilable types like `u64`.

When a `spec` function is `closed`, other modules cannot see its body. To reveal properties about a `closed spec fn` without revealing its definition, use a `proof fn` (lemma) with `ensures` clauses. See [proofs.md](./proofs.md) for proof functions and lemmas.

---

## Proof blocks (`proof { }`)

`exec` functions can contain pieces of `proof` code in *proof blocks*, written with `proof { ... }`. A proof block can use all ghost-code features that `proof` functions can, such as the `int` and `nat` types. The entire block (including local variables) is erased before compilation. For example (from `examples/guide/modes.rs`, anchor `spec_fun_proof_block1`):

```rust
fn test_consts_infer() {
    let u: u8 = 1;
    proof {
        let i: int = 2;
        let n: nat = 3;
        assert(0 <= u < i < n < 4);
    }
}
```

Proof blocks can call `proof` functions. In fact, any calls from an `exec` function to a `proof` function must appear inside `proof` code such as a proof block, rather than being called directly from the `exec` function's `exec` code. This helps clarify which code is executable and which code is ghost, both for the compiler and for programmers reading the code.

---

## `const` declarations

In Verus, `const` declarations are treated internally as 0-argument function calls. Thus just like functions, `const` declarations can be marked `spec`, `proof`, `exec`, or left without an explicit mode. By default, a `const` without an explicit mode is assigned a dual `spec/exec` mode.

A `spec const` is like a `spec` function with no arguments — it is always ghost and cannot be used as an `exec` value (from `examples/guide/const.rs`, anchor `spec_const`):

```rust
spec const SPEC_ONE: int = 1;

spec fn spec_add_one(x: int) -> int {
    x + SPEC_ONE
}
```

A `const` without an explicit mode is dual-use: it is usable as both an `exec` value and a `spec` value (anchor `spec_exec_const`):

```rust
const ONE: u8 = 1;

fn add_one(x: u8) -> (ret: u8)
    requires
        x < 0xff,
    ensures
        ret == x + ONE,  // use "ONE" in spec code
{
    x + ONE  // use "ONE" in exec code

}
```

Therefore, the `const` definition is restricted to obey the rules for both `exec` code and `spec` code. For example, as with `exec` code, its type must be compilable (e.g. `u8`, not `int`), and, as with `spec` code, it cannot call any `exec` or `proof` functions.

`proof` and `exec` consts can have `ensures` clauses to tie the declaration to a `spec` expression (anchor `exec_const_syntax`):

```rust
exec const C: u64
    ensures
        C == 7,
{
    7
}
```

To use an `exec const` in a `spec` or `proof` context, annotate the declaration with `#[verifier::when_used_as_spec(SPEC_DEF)]`, where `SPEC_DEF` is the name of a `spec const` or a `spec` function with no arguments.

---

## Erasure

Verus erases all ghost code before compilation so that it imposes no run-time overhead. Ghost code includes `requires`, `ensures`, `assert`, `assume`, all `spec` and `proof` functions, `Ghost<T>`/`Tracked<T>` contents, and proof blocks. The `--compile` flag triggers the second `rustc` pass that erases ghost code and produces a normal executable via MIR → LLVM → linking.

We can compile the `octuple` example using the `--compile` option:

```bash
./target-verus/release/verus --compile ../examples/guide/requires_ensures.rs
```

This produces an executable that prints a message when run:

```
The answer is 80
```

Note that the generated executable does not contain the `requires`, `ensures`, and `assert` code, since these are only needed during static verification, not during run-time execution. See [getting-started.md](./getting-started.md) for the `verus_only` cfg flag and the `Cargo.toml` configuration needed to guard `use` statements for ghost items that are erased at compile time.

---

## Sources used

Tutorial chapters read (`.tmp/verus/source/docs/guide/src/`):

- `modes.md` — the three modes (spec/proof/exec), the mode-compatibility table, the ghost-code hierarchy
- `verus_macro_intro.md` — the `verus!` macro, `use vstd::prelude::*`, alternate attribute syntax
- `spec_functions.md` — `open`/`closed spec fn`, `pub` vs body visibility, spec-vs-exec differences
- `proof_functions.md` — proof functions (lemmas), proof blocks, `assert(...) by { ... }`
- `requires_ensures.md` — preconditions, postconditions, named returns, modular verification, `assert`/`assume`, `#[verifier::external_body]`, erasure
- `recursion.md` — recursive spec functions, `decreases`, termination, fuel, `reveal_with_fuel`
- `reference-decreases.md` — `decreases ... when ... via ...`, the decreases-measure, helping Verus prove termination
- `while.md` — loops and invariants, loop isolation, `#[verifier::loop_isolation(false)]`
- `invariants.md` — devising loop invariants (Fibonacci, account balance examples)
- `ghost_vs_exec.md` — ghost code abilities (copy any type, create any value), ghost/exec boundary
- `reference-var-modes.md` — variable modes (`exec`/`ghost`/`tracked`), `Ghost<T>`/`Tracked<T>` wrappers, cheat sheet
- `const.md` — `spec`/`exec`/`spec-exec` consts, `ensures` on consts, `when_used_as_spec`
- `erasure.md` — ghost erasure, `verus_only` cfg flag
- `overview.md` — Verus overview (goals, SMT/Z3, linear types for memory)

Example files read (verbatim code snippets sourced from):

- `examples/guide/modes.rs` — anchors: `fun_modes`, `spec_fun3`, `spec_fun_proof_block1`, `ghost_abilities1`
- `examples/guide/requires_ensures.rs` — full file (the `octuple` example with `requires`/`ensures`/`external_body`)
- `examples/guide/recursion.rs` — anchors: `spec`, `step_by_step`, `fuel`, `fuel_by`, `loop`
- `examples/guide/const.rs` — anchors: `spec_const`, `exec_const_syntax`, `spec_exec_const`

Source signatures quoted (directly from `source/builtin/` and `source/vstd/`):

- `source/builtin/src/lib.rs:395` — `pub struct Ghost<A>` (ghost wrapper type)
- `source/builtin/src/lib.rs:402` — `pub struct Tracked<A>` (tracked wrapper type)
- `source/vstd/view.rs` — `pub trait View { type V; spec fn view(&self) -> Self::V; }` (the `View` trait, referenced by the `@` operator; see [specifications.md](./specifications.md))

Upstream URLs:

- https://verus-lang.github.io/verus/guide/modes.html — rendered modes chapter
- https://verus-lang.github.io/verus/guide/verus_macro_intro.html — rendered `verus!` macro chapter
- https://verus-lang.github.io/verus/guide/requires_ensures.html — rendered requires/ensures chapter
- https://verus-lang.github.io/verus/guide/recursion.html — rendered recursion/fuel chapter
- https://verus-lang.github.io/verus/guide/while.html — rendered loops and invariants chapter
- https://verus-lang.github.io/verus/guide/ghost_vs_exec.html — rendered ghost vs exec chapter
- https://verus-lang.github.io/verus/guide/reference-var-modes.html — rendered variable modes reference
- https://verus-lang.github.io/verus/guide/const.html — rendered const declarations chapter
- https://verus-lang.github.io/verus/guide/erasure.html — rendered ghost erasure chapter
- https://verus-lang.github.io/verus/guide/reference-decreases.html — rendered decreases reference
- https://verus-lang.github.io/verus/verusdoc/vstd/view/trait.View.html — `View` trait verusdoc
- https://arxiv.org/abs/2303.05491 — OOPSLA 2023 paper: "Verus: Verifying Rust Programs using Linear Ghost Types"
