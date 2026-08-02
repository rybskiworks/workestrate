# The Verus Specification Language

## Purpose

This is the reference for Verus's specification language: the pure mathematical subset of Rust used in `spec` functions, `requires`/`ensures`/`invariant` clauses, and `assert`/`assume` expressions. It covers spec expressions and operators, the mathematical integer types (`int`/`nat`), equality and extensional equality, quantifiers (`forall`/`exists`/`choose`), implication and prefix conjunction/disjunction, **triggers** (the SMT quantifier-instantiation mechanism), the `@` view operator, and spec closures. For the verification model (modes, `requires`/`ensures` semantics, ghost erasure), see [verification-model.md](./verification-model.md). For proof constructs that *use* these specifications, see [proofs.md](./proofs.md).

> **Source fidelity:** Every code snippet below is copied verbatim from the Verus clone at `.tmp/verus/`. Code from example files is sourced from `examples/guide/*.rs` (anchor names reference the `// ANCHOR:` markers). Type signatures and tables are directly quoted from `source/vstd/`, `source/builtin/`, or tutorial chapters under `source/docs/guide/src/`.

---

## Spec expressions — the spec subset of Rust

Much of the spec language looks like a subset of the Rust language, though there are some subtle differences. Spec expressions are **pure** (no side effects, no short-circuiting) and **spec-mode only**. They support:

- Function calls (only to other `spec` functions or functions marked `when_used_as_spec`)
- `let`-bindings (but not `let mut`)
- `if` / `if let` / `match`
- `&&`, `||`, `!`
- `==`, `!=` (mathematical equality, not `PartialEq`)
- Arithmetic (`+`, `-`, `*`, `/`, `%`) with widening to `int`
- References (`&T`) and `Box<T>` (semantically the identity function in spec mode)
- Verus-specific operators: `==>`, `<==`, `<==>`, `&&&`, `|||`, `forall`, `exists`, `choose`, `@`, `has`, `is`, `matches`, chained comparisons

The full grammar of spec expressions is enumerated in `source/docs/guide/src/spec-expressions.md`.

---

## Operators and precedence

The table below defines operator precedence from tightest-binding (top) to loosest-binding (bottom) (directly quoted from `source/docs/guide/src/spec-operator-precedence.md`):

| Operator | Associativity |
|---|---|
| **Binds tighter** | |
| `.` `->` | left |
| `has`, `is`, `matches` | left |
| `*` `/` `%` | left |
| `+` `-` | left |
| `<<` `>>` | left |
| `&` | left |
| `^` | left |
| <code>&#124;</code> | left |
| `!==` `==` `!=` `<=` `<` `>=` `>` | requires parentheses |
| `&&` | left |
| <code>&#124;&#124;</code> | left |
| `==>` | right |
| `<==` | left |
| `<==>` | requires parentheses |
| `..` | left |
| `=` | right |
| closures; `forall`, `exists`; `choose` | right |
| `&&&` | left |
| <code>&#124;&#124;&#124;</code> | left |
| **Binds looser** | |

All operators that are from ordinary Rust have the same precedence-ordering as in ordinary Rust.

### Chained inequalities

Specifications can chain together multiple `<=`, `<`, `>=`, and `>` operations. For example, `0 <= i <= j < len` has the same meaning as `0 <= i && i <= j && j < len`. A chained comparison desugars into the conjunction of all adjacent pairs, with each intermediate value shared between consecutive comparisons: `a op1 b op2 c` is equivalent to `a op1 b && b op2 c`.

### Implication (`==>`, `<==`, `<==>`)

`P ==> Q`, read *P implies Q*, is `true` whenever `P` is `false` or `Q` is `true`:

```
P ==> Q  ≡  !P || Q
```

`P <== Q` (*P is implied by Q*) is the converse: it is equivalent to `Q ==> P`. `P <==> Q` (*P if and only if Q*) is true when both sides have the same truth value: `P <==> Q ≡ P == Q`. Note that `==>` has lower precedence than most other boolean operations: `a ==> b && c` means `a ==> (b && c)`.

### Prefix and/or (`&&&` and `|||`)

Because `&&`, `||`, and `==>` are so common in Verus specifications, Verus supports "triple-and" (`&&&`) and "triple-or" (`|||`) which are equivalent to `&&` and `||` except for their precedence. Implication `==>` and equivalence `<==>` bind more tightly than either `&&&` or `|||`. `&&&` and `|||` are also convenient for the "bulleted list" form:

```
&&& a ==> b
&&& c
&&& d <==> e && f
```

This has the same meaning as `(a ==> b) && c && (d <==> (e && f))`. The main motivation is readability: when writing a large conjunction or disjunction (such as a precondition or an invariant), the leading-prefix style makes it easy to add, remove, or reorder clauses without adjusting punctuation elsewhere.

---

## Integer types: `int` and `nat`

Rust supports fixed-bit-width integer types (`i8`...`isize`, `u8`...`usize`). To these, Verus adds two more integer types to represent arbitrarily large integers in specifications:

- `int` — all mathematical integers, both positive and negative. The SMT solver contains direct support for reasoning about values of type `int`.
- `nat` — natural numbers: a mathematical integer constrained to be greater than or equal to `0`.

Internally, Verus uses `int` to represent the other integer types, adding mathematical constraints to limit the range. For example, a `u8` value is an integer constrained to be greater than or equal to `0` and less than `256` (from `examples/guide/integers.rs`, anchor `test_u8`):

```rust
fn test_u8(u: u8) {
    assert(0 <= u < 256);
}
```

The mathematical domain of each integer type (directly quoted from `source/docs/guide/src/reference-types.md`):

| Type | Bound |
|---|---|
| `int` | `(-∞, ∞)` (i.e., no bound) |
| `nat` | `[0, ∞)` |
| `uN` | `[0, 2^N)` |
| `iN` | `[-2^(N-1), 2^(N-1))` |
| `usize` | `[0, 2^usize::BITS)` |
| `isize` | `[-2^(isize::BITS-1), 2^(isize::BITS-1))` |

### Using integer types in specifications

Use `int` by default, since this is the most general type and is supported most efficiently by the SMT solver. Use `nat` for return values and datatype fields where the 0 lower bound is likely to provide useful information, such as lengths of sequences. Use fixed-width integer types for fixed-width values such as bytes. Note that `int` and `nat` are usable only in ghost code; they cannot be compiled to executable code.

Integer constants can include their type as a suffix (e.g. `7u8` or `7int`). Usually, Verus and Rust can infer types for integer constants, so you can omit the suffixes (from `examples/guide/integers.rs`, anchor `test_consts_infer`):

```rust
fn test_consts_infer() {
    let u: u8 = 1;
    assert({
        let i: int = 2;
        let n: nat = 3;
        0 <= u < i < n < 4
    });
}
```

Note that the values `0`, `u`, `i`, `n`, and `4` in the expression `0 <= u < i < n < 4` are allowed to all have different types — you can compare values of different integer types inside ghost code (e.g. comparing a `u8` to an `int` in `u < i`).

### Integer arithmetic and overflow semantics

In **executable** code, Verus requires you to prove the absence of overflow. In **ghost** (spec) code, common arithmetic operations (`+`, `-`, `*`, `/`, `%`) never overflow or wrap — Verus widens the results of many operations to `int`. For example, adding two `u8` values is widened to type `int` (from `examples/guide/integers.rs`, anchor `test_sum2`):

```rust
fn test_sum2(x: u8, y: u8) {
    assert({
        let sum2: int = x + y;  // in ghost code, + returns int and does not overflow
        0 <= sum2 < 511
    });
}
```

Since `+` does not overflow in ghost code, you can easily write specifications *about* overflow. For example, to make sure that the executable `x + y` does not overflow, write `requires x + y < 256`, relying on the fact that `x + y` is widened to type `int` in the `requires` clause (anchor `test_sum3`):

```rust
fn test_sum3(x: u8, y: u8)
    requires
        x + y < 256,  // make sure "let sum1: u8 = x + y" can't overflow
{
    let sum1: u8 = x + y;  // succeeds
}
```

In spec code, `/` and `%` compute [Euclidean division and remainder](https://en.wikipedia.org/wiki/Euclidean_division), rather than Rust's truncating division and remainder, when operating on negative left-hand sides or negative right-hand sides. Division-by-0 and mod-by-0 are unspecified in ghost code (but not hard errors, since all spec functions are total). The named arithmetic functions `add(x, y)`, `sub(x, y)`, and `mul(x, y)` do not perform widening, and thus have truncating behavior, even in ghost code.

### Coercion with `as`

As in ordinary Rust, the `as` operator coerces one integer type to another. In ghost code, you can use `as int` or `as nat` to coerce to `int` or `nat` (from `examples/guide/integers.rs`, anchor `test_coerce`):

```rust
fn test_coerce() {
    let u: u8 = 1;
    assert({
        let i: int = u as int;
        let n: nat = u as nat;
        u == i && u == n
    });
}
```

You can use `as` to coerce a value `v` to a type `t` even if `v` is too small or too large to fit in `t`. However, if the value `v` is outside the bounds of type `t`, then `v as t` will produce some arbitrary value of type `t`. Casting to an `int` is always defined and does not require truncation. Casting to any finite-size integer type is defined as *truncation* — taking the lower N bits. The definition of truncation is not exported in Verus's default prover mode; to reason about truncation, use the bit-vector solver or the compute solver (see [arithmetic-and-provers.md](./arithmetic-and-provers.md)).

---

## Equality and extensional equality

Equality behaves differently in ghost code than in executable code. In executable code, Rust defines `==` to mean a call to the `eq` function of the `PartialEq` trait (from `examples/guide/equality.rs`, anchor `eq1`):

```rust
fn equal1(x: u8, y: u8) {
    let eq1 = x == y;  // means x.eq(y) in Rust
    let eq2 = y == x;  // means y.eq(x) in Rust
    assert(eq1 ==> eq2);  // succeeds
}
```

For user-defined types, `eq` could have other behaviors — it might have side effects, behave nondeterministically, or fail to fulfill its promise to implement an equivalence relation. In ghost code, by contrast, the `==` operator is **always** an equivalence relation (i.e. it is reflexive, symmetric, and transitive) (anchor `eq3`):

```rust
fn equal3(x: u8, y: u8) {
    assert({
        let eq1 = x == y;
        let eq2 = y == x;
        eq1 ==> eq2
    });
}
```

Verus defines `==` in ghost code to be true when:

- for two integers or booleans, the values are equal
- for two structs or enums, the types are the same and the fields are equal
- for two `&` references, two `Box` values, two `Rc` values, or two `Arc` values, the pointed-to values are the same
- for two `RefCell` values or two `Cell` values, the pointers to the interior data are equal (not the interior contents)

### Extensional equality (`=~=`, `=~~=`)

Collection datatypes such as `Seq<T>`, `Set<T>`, and `Map<Key, Value>` have their own definitions of `==`, where two sequences, two sets, or two maps are equal if their elements are equal. These sometimes require the "extensional equality" operator `=~=` to help prove equality between two sequences, two sets, or two maps.

The **shallow extensional equality operator** `=~=` and **deep extensional equality operator** `=~~=` are both *equivalent* to spec equality (`==`). These operators are distinguished only by their impact on the proof search: the use of `=~=` and `=~~=` will trigger the application of "extensional equality" axioms. See [vstd-library.md](./vstd-library.md) for the collection types and [proofs.md](./proofs.md) for extensional equality proofs.

---

## Quantifiers: `forall`, `exists`, `choose`

Both `forall` and `exists` are **spec-mode only** expressions. `choose` is also spec-mode only.

### `forall`

`forall|x: T| P(x)` is `true` if and only if `P(x)` is `true` for every value `x` of type `T`. The following example uses a `forall` expression in a `requires` clause (from `examples/guide/quants.rs`, anchor `quants_use_forall`):

```rust
proof fn test_use_forall(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int| 0 <= i < s.len() ==> #[trigger] is_even(s[i]),
{
    assert(is_even(s[3]));
}
```

The `forall` expression means that `0 <= i < s.len() ==> is_even(s[i])` for all possible integers `i`. There are infinitely many integers `i`, so the SMT solver cannot literally expand the `forall` into an infinite list of expressions. Instead, it uses *triggers* to choose likely relevant `i` (see [Triggers](#triggers-trigger) below).

### `exists`

`exists|x: T| P(x)` is `true` if and only if there exists at least one value `x` of type `T` such that `P(x)` is `true`. To prove an `exists` expression, the SMT solver has to find one value for `i` such that `f(i)` is true — this value is called a *witness*. As with `forall`, proofs about `exists` expressions are based on triggers (from `examples/guide/quants.rs`, anchor `test_exists_succeeds`):

```rust
proof fn test_exists_succeeds() {
    assert(is_even(4));
    assert(!is_even(5));
    assert(is_even(6));
    assert(exists|i: int| #[trigger] is_even(i));  // succeeds with witness i = 4 or i = 6
}
```

The two quantifiers are duals. Verus uses classical logic, so: `exists|x: T| P(x) ≡ !forall|x: T| !P(x)`. Both quantifiers support binding multiple variables simultaneously, which is equivalent to nesting single-variable quantifiers:

```rust
// These two are equivalent:
forall|i: int, j: int| i < j ==> f(i) <= f(j)
forall|i: int| forall|j: int| i < j ==> f(i) <= f(j)
```

### `choose` (definite description / Hilbert choice)

`choose|x: T| P(x)` implements the Hilbert choice operator (also known as the epsilon operator): it chooses some value `i` that satisfies `P(i)` if such a value exists. Otherwise, it picks an arbitrary value for `i`. The following example assumes `exists|i: int| f(i)` as a precondition; based on this, `choose` picks one of the witnesses arbitrarily (from `examples/guide/quants.rs`, anchor `test_choose_succeeds`):

```rust
spec fn f(i: int) -> bool;

proof fn test_choose_succeeds()
    requires
        exists|i: int| f(i),
{
    let i_witness = choose|i: int| f(i);
    assert(f(i_witness));
}
```

Regardless of whether we know `exists|i: int| f(i)` or not, the `choose|i: int| f(i)` expression always returns the same value (anchor `test_choose_same`):

```rust
proof fn test_choose_same() {
    let x = choose|i: int| f(i);
    let y = choose|i: int| f(i);
    assert(x == y);
}
```

You can also choose multiple values together, collecting the values in a tuple (anchor `test_choose_succeeds2`):

```rust
spec fn less_than(x: int, y: int) -> bool {
    x < y
}

proof fn test_choose_succeeds2() {
    assert(less_than(3, 7));  // promote i = 3, i = 7 as a witness
    let (x, y) = choose|i: int, j: int| less_than(i, j);
    assert(x < y);
}
```

For a single binder `x: T`, `choose` has type `T`. For multiple binders `x: T, y: U, ...`, the expression has tuple type `(T, U, ...)`. To use the result with the guarantee that `P` holds, you must separately establish `exists|x: T| P(x)`; Verus will then allow you to conclude `P(choose|x: T| P(x))`.

---

## Triggers (`#[trigger]`)

Triggers are the mechanism the SMT solver uses to instantiate quantifiers. Because quantifiers range over infinite domains, the SMT solver does not enumerate all possible instantiations. Instead it uses *triggers*: syntactic patterns that, when matched by terms in the proof context, cause the quantifier to be instantiated with the matching values. Triggers are the way you program the instantiations of `forall` expressions (and the way you program proofs of `exists` expressions).

### How triggers work

A *trigger* is an expression or set of expressions that the SMT solver uses as a pattern to match with. In the `forall` example above, the `#[trigger]` attribute marks the expression `is_even(s[i])` as the trigger. Based on this attribute, the SMT solver looks for expressions of the form `is_even(s[...])`. During verification, there is one expression that has this form: `is_even(s[3])`. This matches the trigger `is_even(s[i])` exactly for `i = 3`. Based on this pattern match, the SMT solver chooses `i = 3` and introduces the fact `0 <= 3 < s.len() ==> is_even(s[3])`, which allows it to complete the proof.

### Trigger choice matters

Suppose we change the assertion so that we assert `s[3] % 2 == 0` instead of `is_even(s[3])`. Mathematically, these are both equivalent. However, the assertion about `s[3] % 2 == 0` fails, because there are no expressions matching the pattern `is_even(s[...])` — the expression `s[3] % 2 == 0` does not mention `is_even` at all. In order to prove `s[3] % 2 == 0`, we would first have to mention `is_even(s[3])` explicitly (from `examples/guide/quants.rs`, anchor `test_use_forall_succeeds1`):

```rust
proof fn test_use_forall_succeeds1(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int| 0 <= i < s.len() ==> #[trigger] is_even(s[i]),
{
    assert(is_even(s[3]));  // triggers is_even(s[3])
    assert(s[3] % 2 == 0);  // succeeds, because previous line already instantiated the forall
}
```

Alternatively, we could choose a trigger that is less picky. For example, the trigger `s[i]` matches any expression of the form `s[...]`, which includes the `s[3]` inside `s[3] % 2 == 0` (anchor `test_use_forall_succeeds2`):

```rust
proof fn test_use_forall_succeeds2(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int| 0 <= i < s.len() ==> is_even(#[trigger] s[i]),
{
    assert(s[3] % 2 == 0);  // succeeds by triggering s[3]
}
```

In fact, if we omit the `#[trigger]` attribute entirely, Verus chooses the trigger `s[i]` automatically. Verus prints a note stating that it chose this trigger, because it has low confidence in the chosen triggers. The programmer can accept this decision by writing `#![auto]` before the quantifier body, which suppresses the note (anchor `test_use_forall_succeeds4`):

```rust
proof fn test_use_forall_succeeds4(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int|
            #![auto]
            0 <= i < s.len() ==> is_even(s[i]),  // Verus chooses s[i] as the trigger
{
    assert(s[3] % 2 == 0);  // succeeds by triggering s[3]
}
```

### Valid triggers and invalid triggers

In practice, a valid trigger needs to follow two rules:

1. A trigger for a statement needs to contain all of its non-free variables, meaning those variables that are instantiated by a `forall` or an `exists`. To achieve this with multiple variables, you can split the trigger into multiple parts (see [Multi-triggers](#multi-triggers) below).
2. A trigger cannot contain equality or disequality (`==`, `===`, `!=`, or `!==`), any basic integer arithmetic operator (like `<=` or `+`), or any basic boolean operator (like `&&`). A trigger must be a function call, a field access, or a bitwise operator.

### Trigger annotation syntax

The trigger annotation forms (directly quoted from `source/docs/guide/src/trigger-annotations.md`):

| Annotation | Meaning |
|---|---|
| `#[trigger]` on a sub-expression | That sub-expression is a trigger (grouped with other `#[trigger]` annotations) |
| `#[trigger(n)]` on a sub-expression | That sub-expression is part of trigger group `n` |
| `#![trigger expr1, expr2, ...]` at the root of the body | `expr1, expr2, ...` form a single trigger group |
| `#![auto]` at the root of the body | Use automatic trigger selection and suppress the trigger-logging note |
| `#![all_triggers]` at the root of the body | Use aggressive automatic trigger selection |

Trigger annotations have no impact on the *semantics* of a spec expression — triggers only impact the proof space explored by the automated theorem prover.

### Trigger groups and trigger expressions

Every quantifier has a collection of *trigger groups*. Every trigger group is a collection of *trigger expressions*. A trigger group is only well-formed if every quantifier variable is used by at least one trigger expression in the group. The SMT solver will instantiate a quantifier whenever *any* trigger group fires. However, a trigger group will only fire if *every* expression in the group matches. Therefore:

- Having more trigger groups makes the quantifier be instantiated *more* often.
- A trigger group with more trigger expressions will fire *less* often.

### Multi-triggers

A trigger does not need to be just a single expression. It can be split across multiple expressions. For example, with two quantifier variables `i` and `j`, the trigger can be the pair of expressions `s[i]`, `s[j]` (from `examples/guide/quants.rs`, anchor `test_distinct2`):

```rust
proof fn test_distinct2(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int, j: int| 0 <= i < j < s.len() ==> #[trigger] s[i] != #[trigger] s[j],
{
    assert(s[4] != s[2]);
}
```

Verus also supports an alternate, equivalent syntax `#![trigger ...]`, where the `#![trigger ...]` immediately follows the `forall|...|` (anchor `test_distinct3`):

```rust
proof fn test_distinct3(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int, j: int| #![trigger s[i], s[j]] 0 <= i < j < s.len() ==> s[i] != s[j],
{
    assert(s[4] != s[2]);
}
```

A trigger must mention each of the quantifier variables at least once. Otherwise, Verus will complain: `error: trigger does not cover variable i`.

### Multiple trigger groups

It is also possible, although rarer, to specify multiple triggers for a quantifier. The SMT solver will instantiate the quantifier if *any* of the triggers match. Thus, adding more triggers leads to *more* quantifier instantiations. (This stands in contrast to adding *expressions* to a trigger: adding more expressions to a trigger makes a trigger more restrictive and leads to *fewer* quantifier instantiations.) To specify multiple triggers, you must use the `#![trigger ...]` syntax rather than the `#[trigger]` syntax (anchor `test_distinct4`):

```rust
proof fn test_distinct4(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int, j: int|
            #![trigger s[i], s[j]]
            #![trigger is_even(i), is_even(j)]
            0 <= i < j < s.len() ==> s[i] != s[j],
{
    assert(s[4] != s[2]);
}
```

The following example specifies both `#![trigger a[i], b[j]]` and `#![trigger a[i], c[j]]` as triggers, since neither is obviously better than the other (anchor `test_multitriggers`):

```rust
proof fn test_multitriggers(a: Seq<int>, b: Seq<int>, c: Seq<int>)
    requires
        5 <= a.len(),
        a.len() == b.len(),
        a.len() == c.len(),
        forall|i: int, j: int|
            #![trigger a[i], b[j]]
            #![trigger a[i], c[j]]
            0 <= i < j < a.len() ==> a[i] != b[j] && a[i] != c[j],
{
    assert(a[2] != c[4]);  // succeeds, matches a[i], c[j]
}
```

### Matching loops — what they are and how to avoid them

Suppose we want to specify that a sequence is sorted. The best way to express sortedness in Verus is to quantify over both `i` and `j`, because the trigger `s[i], s[j]` works very well (anchor `test_sorted_good`):

```rust
proof fn test_sorted_good(s: Seq<int>)
    requires
        5 <= s.len(),
        forall|i: int, j: int| 0 <= i <= j < s.len() ==> s[i] <= s[j],
{
    assert(s[2] <= s[4]);
}
```

However, there is an alternate approach: quantify over just a single variable `i`, and compare `s[i]` to `s[i + 1]`. Verus rejects this, because the only candidate trigger `s[i]` potentially leads to an infinite *matching loop*. The SMT solver would match on `i = 2`, which creates `s[3]`, which leads to matching with `i = 3`, which creates `s[4]`, and so on — in principle, the SMT solver could loop forever. In practice, the SMT solver imposes a cutoff on quantifier instantiations which often (but not always) halts the infinite loops, but even when it halts, this is an inefficient process, and matching loops should be avoided.

### Auto-trigger selection

If, after collecting all explicit `#[trigger]` and `#![trigger ...]` annotations, no trigger groups have been identified, Verus may use heuristics to determine the trigger group(s) based on the body of the quantifier expression:

- If `#![all_triggers]` is provided, Verus uses an "aggressive" strategy, choosing all trigger groups that can reasonably be inferred.
- If `#![auto]` is provided, Verus uses a "conservative" strategy, choosing a single trigger group that is judged as optimal by various heuristics.
- If neither is provided, Verus uses the same "conservative" strategy as it does for `#![auto]`.

If Verus is unable to find any trigger groups, it produces an error. The `--triggers` command-line option prints all automatically chosen triggers; `--triggers-mode silent` suppresses the notes. See [proof-engineering.md](./proof-engineering.md) for quantifier profiling.

---

## The `@` view operator

The expression `expr@` desugars to the expression `expr.view()`, which is resolved as normal via Rust's method resolution. This usually resolves to the `view` method defined by the `View` trait. The `View` trait is defined in the vstd library (directly quoted from `source/vstd/view.rs`):

```rust
/// Types used in executable code can implement View to provide a mathematical abstraction
/// of the type.
/// For example, Vec implements a view method that returns a Seq.
/// For base types like bool and u8, the view V of the type is the type itself.
/// Types only used in ghost code, such as int, nat, and Seq, do not need to implement View.
pub trait View {
    type V;

    spec fn view(&self) -> Self::V;
}
```

By convention, the `view()` function is a spec function for the abstraction of an exec-mode object. For example, `Vec` implements `View` so that `vec@` returns a `Seq` representing the vector's contents mathematically. This lets you write specifications about `Vec` in terms of `Seq` operations. See [vstd-library.md](./vstd-library.md) for the collection types and their views.

---

## Spec closures

Verus supports anonymous functions (known as "closures" in Rust) in ghost code. For example, the following code uses an anonymous function `|i: int| 10 * i` to initialize a sequence with the values 0, 10, 20, 30, 40 (from `examples/guide/lib_examples.rs`, anchor `new0`):

```rust
proof fn test_seq2() {
    let s: Seq<int> = Seq::new(5, |i: int| 10 * i);
    assert(s.len() == 5);
    assert(s[2] == 20);
    assert(s[3] == 30);
}
```

The anonymous function `|i: int| 10 * i` has type `spec_fn(int) -> int` and has mode `spec`. Because it has mode `spec`, the anonymous function is subject to the same restrictions as named `spec` functions (for example, it can call other `spec` functions but not `proof` functions or `exec` functions).

Note that in contrast to standard executable Rust closures, where `Fn`, `FnOnce`, and `FnMut` are traits, `spec_fn(int) -> int` is a **type**, not a trait. Therefore, ghost code can return a spec closure directly, using a return value of type `spec_fn(t1, ..., tn) -> tret`, without having to use `dyn` or `impl`. For example, the `spec` function `adder` can return an anonymous function that adds `x` to `y` (anchor `ret_spec_fn`):

```rust
spec fn adder(x: int) -> spec_fn(int) -> int {
    |y: int| x + y
}

proof fn test_adder() {
    let f = adder(10);
    assert(f(20) == 30);
    assert(f(60) == 70);
}
```

---

## Sources used

Tutorial chapters read (`.tmp/verus/source/docs/guide/src/`):

- `operators.md` — chained inequalities, implication, `&&&`/`|||`, field access (`->`, `is`, `matches`)
- `integers.md` — `int`/`nat` types, integer constants, `as` coercions, ghost vs exec arithmetic, Euclidean division
- `equality.md` — `==` in ghost vs exec code, what `==` means for each type
- `forall.md` — `forall` and triggers (the deep dive: valid/invalid triggers, good/bad triggers, auto selection)
- `multitriggers.md` — multiple variables, multiple triggers, matching loops
- `exists.md` — `exists` and `choose` (witnesses, Hilbert choice operator)
- `spec-expressions.md` — spec expression grammar
- `spec-operator-precedence.md` — operator precedence table
- `spec-arithmetic.md` — arithmetic in spec code (widening rules, Euclidean division/remainder)
- `spec-rust-subset.md` — the Rust subset of spec (function calls, let, if/match, `&&`/`||`, `==`, arithmetic, references, Box)
- `spec-equality.md` — spec equality (`==`) reference
- `ref-extensional-equality.md` — extensional equality (`=~=`, `=~~=`) reference
- `reference-implication.md` — implication (`==>`, `<==`, `<==>`) reference
- `prefix-and-or.md` — prefix and/or (`&&&`, `|||`) reference
- `spec-quantifiers.md` — `forall`/`exists` reference (syntax, typing, semantics, trigger table)
- `spec-choose.md` — `choose` reference (Hilbert choice, single vs multiple binders)
- `trigger-annotations.md` — trigger annotation syntax, trigger group selection, trigger logging
- `reference-at-sign.md` — the `@` view operator reference
- `reference-types.md` — mathematical interpretations of types (integer bounds, bool, char, Box, references, pointers, strings)
- `reference-as.md` — coercion with `as` (integer types, pointers, truncation)
- `reference-chained-op.md` — chained operators reference
- `reference-spec-index.md` — spec index operator `[]` reference
- `reference-has.md` — the `has` operator reference
- `reference-is.md` — the `is` operator reference
- `reference-matches.md` — the `matches` operator reference
- `spec-bit-ops.md` — bit operators reference
- `spec_closures.md` — spec closures (`spec_fn` type, anonymous functions in ghost code)
- `overview.md` — Verus overview

Example files read (verbatim code snippets sourced from):

- `examples/guide/integers.rs` — anchors: `test_u8`, `test_consts_infer`, `test_coerce`, `test_sum2`, `test_sum3`
- `examples/guide/equality.rs` — anchors: `eq1`, `eq3`
- `examples/guide/quants.rs` — anchors: `quants_use_forall`, `test_use_forall_succeeds1`, `test_use_forall_succeeds2`, `test_use_forall_succeeds4`, `test_distinct2`, `test_distinct3`, `test_distinct4`, `test_multitriggers`, `test_sorted_good`, `test_exists_succeeds`, `test_choose_succeeds`, `test_choose_same`, `test_choose_succeeds2`
- `examples/guide/lib_examples.rs` — anchors: `new0`, `ret_spec_fn`

Source signatures quoted (directly from `source/vstd/`):

- `source/vstd/view.rs` — `pub trait View { type V; spec fn view(&self) -> Self::V; }` (the `View` trait, desugared by the `@` operator)
- `source/vstd/seq.rs:46` — `pub uninterp spec fn len(self) -> nat` (Seq length)
- `source/vstd/seq.rs:53` — `pub uninterp spec fn index(self, i: int) -> A` (Seq indexing)
- `source/vstd/set.rs:166` — `pub open spec fn spec_has(self, a: A) -> bool` (Set membership, desugared by `has`)

Upstream URLs:

- https://verus-lang.github.io/verus/guide/operators.html — rendered operators chapter
- https://verus-lang.github.io/verus/guide/integers.html — rendered integers chapter
- https://verus-lang.github.io/verus/guide/equality.html — rendered equality chapter
- https://verus-lang.github.io/verus/guide/forall.html — rendered forall and triggers chapter
- https://verus-lang.github.io/verus/guide/multitriggers.html — rendered multi-triggers chapter
- https://verus-lang.github.io/verus/guide/exists.html — rendered exists and choose chapter
- https://verus-lang.github.io/verus/guide/spec_closures.html — rendered spec closures chapter
- https://verus-lang.github.io/verus/guide/spec-operator-precedence.html — rendered operator precedence reference
- https://verus-lang.github.io/verus/guide/spec-quantifiers.html — rendered quantifiers reference
- https://verus-lang.github.io/verus/guide/spec-choose.html — rendered choose reference
- https://verus-lang.github.io/verus/guide/trigger-annotations.html — rendered trigger annotations reference
- https://verus-lang.github.io/verus/guide/reference-at-sign.html — rendered `@` view operator reference
- https://verus-lang.github.io/verus/guide/reference-types.html — rendered type interpretations reference
- https://verus-lang.github.io/verus/guide/reference-as.html — rendered `as` coercion reference
- https://verus-lang.github.io/verus/verusdoc/vstd/view/trait.View.html — `View` trait verusdoc
- https://microsoft.github.io/z3guide/docs/logic/Quantifiers — Z3 guide on quantifiers and patterns (referenced from `forall.md`)
