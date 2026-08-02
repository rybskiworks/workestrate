# Arithmetic and Alternate Provers in Verus

Verus's default solver (Z3) is excellent at *linear* integer arithmetic but
intentionally disables nonlinear theories for predictability and performance.
When a proof requires nonlinear reasoning, bitwise operations, or concrete
computation, Verus provides specialized *prover modes* that you invoke with the
`by(...)` keyword. This doc covers the linear/nonlinear distinction, each
alternate prover, overflow handling, and the `vstd` arithmetic lemma library.
For the general proof constructs (`assert`, `assume`, `assert(...) by { ... }`,
`calc!`, `broadcast`), see [proofs.md](./proofs.md).

> **Source fidelity:** Every code snippet below is copied verbatim from the
> Verus clone at `.tmp/verus/` (tutorial chapters under
> `source/docs/guide/src/`, example files under `examples/`, and the `vstd`
> arithmetic library under `source/vstd/arithmetic/`).

---

## Linear vs. nonlinear arithmetic

Z3's default prover mode in Verus is excellent at **linear** integer arithmetic:
equalities, inequalities, addition, subtraction, and multiplication/division by
*constants*. It handles expressions like `4 * x + 3 * y - z <= 20` with ease.

It is less capable with **nonlinear** expressions — `x * y` when neither `x`
nor `y` is a constant, or `x / y` when `y` is symbolic. Many common axioms are
inaccessible in the default mode:

- `x * y == y * x`
- `x * (y * z) == (x * y) * z`
- `x * (a + b) == x * a + x * b`
- `0 <= x <= y && 0 <= z <= w ==> x * z <= y * w`

Verus **intentionally** disables nonlinear theories in its default prover mode
for predictability. You opt in to nonlinear reasoning via specialized prover
modes:

| Mode | Backend | Decidable? | Scope |
|------|---------|-----------|-------|
| `nonlinear_arith` | Z3 nonlinear integer theory | No (heuristics) | General nonlinear |
| `integer_ring` | Singular (external CAS) | Yes | Equational ring properties (int only) |
| `bit_vector` | Z3 bit-vector theory (bit blasting) | Yes | Bitwise ops, bounded integers |
| `compute` / `compute_only` | Verus internal interpreter | Yes (if it terminates) | Concrete/symbolic evaluation |

If none of these work, you can manually invoke a lemma from the
[vstd arithmetic library](#the-vstd-arithmetic-lemma-library).

---

## `by(nonlinear_arith)` — Z3's nonlinear integer theory

`nonlinear_arith` enables [Z3's theory of nonlinear arithmetic for
integers](https://microsoft.github.io/z3guide/docs/theories/Arithmetic/#non-linear-arithmetic).
It is general-purpose but somewhat unpredictable, which is why it is off by
default.

### Inline proofs with `assert(...) by(nonlinear_arith)`

`assert(Q) by(nonlinear_arith)` creates a separate Z3 query just to prove `Q`,
with nonlinear heuristics enabled. The query does **not** include ambient facts
(e.g., from the surrounding function's `requires` or preceding assignments)
other than what is:

- inferred from a variable's type (e.g., the range of a `u64` or `nat`), or
- supplied explicitly via a `requires` clause.

```rust
proof fn bound_check(x: u32, y: u32, z: u32)
    requires
        x <= 8,
        y <= 8,
{
    assert(x * y <= 100) by (nonlinear_arith)
        requires
            x <= 10,
            y <= 10;

    assert(x * y <= 1000);
}
```

Step by step:

1. Verus uses its **normal solver** to prove the assert's `requires` clause
   (`x <= 10 && y <= 10`), which follows from the function precondition.
2. Verus uses Z3's **nonlinear solver** to prove
   `x <= 10 && y <= 10 ==> x * y <= 100`.
3. The fact `x * y <= 100` is now available for later asserts.
4. Verus uses its **normal solver** to prove `x * y <= 1000`, which follows from
   `x * y <= 100`.

### Reusable proofs with `proof fn ... by(nonlinear_arith)`

You can attach `by(nonlinear_arith)` to a proof function's signature. The
nonlinear solver verifies the lemma body; when the lemma is called from
elsewhere, Verus uses the **normal** solver to prove the precondition is met:

```rust
proof fn bound_check2(x: u32, y: u32, z: u32) by (nonlinear_arith)
    requires
        x <= 8,
        y <= 8,
    ensures
        x * y <= 64
{ }
```

---

## `by(integer_ring)` — decidable equational ring theory via Singular

`integer_ring` handles a specific class of nonlinear formulas: congruence
relations (equalities modulo some divisor `n`). For example,
`a % n == b % n ==> (a * c) % n == (b * c) % n`.

It is **decidable and efficient**, but only complete for properties true for all
[rings](https://en.wikipedia.org/wiki/Ring_(mathematics)). It is discharged by
the external computer algebra system [Singular](https://www.singular.uni-kl.de/).

> **Note:** At present, `integer_ring` can only be invoked in
> `proof fn ... by(integer_ring)` style — inline `assert(...) by(integer_ring)`
> is **not** supported.

### Installing Singular

Singular must be installed (version 4.3.2 recommended; 4.4.0 is known to be
incompatible). Install via your system package manager and set
`VERUS_SINGULAR_PATH`:

```bash
# Debian-based Linux:
apt-get install singular
export VERUS_SINGULAR_PATH=/usr/bin/Singular

# macOS:
brew install Singular
export VERUS_SINGULAR_PATH=/usr/local/bin/Singular
```

The `integer_ring` functionality is conditionally compiled when the `singular`
feature is set. Add `--features singular` when building Verus:

```bash
vargo build --features singular
```

### Details and limitations

- **`int` only** — can only be used with `int` parameters (not `nat` or
  fixed-width types directly).
- **Equalities only** — inequalities (`<`, `<=`, `>`, `>=`) are not supported.
- **No division** — `/` is not supported.
- **Function calls** are treated as uninterpreted functions. If a function
  definition matters, unfold it in the `requires` clause.
- **Modulus (`%`)** becomes a congruence relation: `a % b == x` is encoded as
  `a ≡ x (mod b)`, which does **not** imply `0 <= x < b`. You cannot ask
  `integer_ring` to prove `a % b == x` unless `x` is 0; it can prove
  `(a - x) % b == 0` (i.e., `a ≡ x (mod b)`), but not `a % b == x`.
- **Nonzero divisors** — Verus checks that `requires` clauses ensure all
  divisors are nonzero. If a divisor can be zero in the `ensures` clause, the
  facts will not be available at the call site.

### The `integer_ring` + `nonlinear_arith` helper pattern

Since `integer_ring` cannot handle inequalities or bounds, the common pattern is
to write a `by(nonlinear_arith)` lemma as the main proof, and use a
`by(integer_ring)` helper lemma to supply the equational facts that
`nonlinear_arith` struggles with:

```rust
pub proof fn lemma_mod_difference_equal_helper(x: int, y:int, d:int, small_x:int, small_y:int, tmp1:int, tmp2:int) by(integer_ring)
    requires
        small_x == x % d,
        small_y == y % d,
        tmp1 == (small_y - small_x) % d,
        tmp2 == (y - x) % d,
    ensures
        (tmp1 - tmp2) % d == 0
{}
pub proof fn lemma_mod_difference_equal(x: int, y: int, d: int) by(nonlinear_arith)
    requires
        d > 0,
        x <= y,
        x % d <= y % d,
        y - x < d
    ensures
        y % d - x % d == y - x
{
    let small_x = x % d;
    let small_y = y % d;
    let tmp1 = (small_y - small_x) % d;
    let tmp2 = (y - x) % d;
    lemma_mod_difference_equal_helper(x,y,d, small_x, small_y, tmp1, tmp2);
}
```

The helper lemma provides the congruence fact
`y % d - x % d ≡ y - x (mod d)`; the rest (inequalities, bounds) is handled by
`nonlinear_arith`.

### Bounded integers with `integer_ring`

Since `integer_ring` only supports `int`, you must include explicit bounds when
proving properties about fixed-width integers. Introduce additional variables to
work around division (e.g., introduce `c` where `b = a * c` instead of `b / a`):

```rust
proof fn lemma_mod_after_mul(x: int, y: int, z: int, m: int) by (integer_ring)
    requires (x-y) % m == 0
    ensures (x*z - y*z) % m == 0
{}

proof fn lemma_mod_after_mul_u32(x: u32, y: u32 , z: u32, m: u32)   
    requires
        m > 0,
        (x-y) % (m as int) == 0,
        x >= y,
        x <= 0xffff,
        y <= 0xffff,
        z <= 0xffff,
        m <= 0xffff,
    ensures (x*z - y*z) % (m as int) == 0
{ 
  lemma_mod_after_mul(x as int, y as int, z as int, m as int);
  // rest of proof body omitted for space
}
```

### A complete `integer_ring` example

From `examples/integer_ring/integer_ring.rs`, the `integer_ring` + `nonlinear_arith`
pairing for modular arithmetic:

```rust
proof fn mod_of_mul_int(a: int, b: int)
    by (integer_ring)
    requires b != 0,
    ensures (a * b) % b == 0,
{
}

proof fn mod_of_mul(a: nat, b: nat)
    by (nonlinear_arith)
    requires
        b > 0,
    ensures
        (a * b) % b == 0,
{
    mod_of_mul_int(a as int, b as int);
}
```

The `integer_ring` lemma proves the equational fact `(a * b) % b == 0` (which is
a ring identity); the `nonlinear_arith` lemma lifts it to `nat` by supplying the
bound `b > 0`.

### Examining the Singular encoding

Singular queries are logged to the directory specified with `--log-dir`
(defaults to `.verus-log`) in the `.air` file for the module. For example, a
query encoding looks like:

```
ring ring_R=integer, (a, b, c, d, x, y, tmp_0, tmp_1), dp;
    ideal ideal_I =
      a - c,
      b - d,
      (a - (b * tmp_0)) - x,
      (c - (d * tmp_1)) - y;
    ideal ideal_G = groebner(ideal_I);
    reduce(x - y, ideal_G);
    quit;
```

Here `a % b` is translated to `a - b * tmp_0` (a congruence), with no constraint
that the result is bounded — which is why `integer_ring` cannot prove
`a % b == x` in general.

---

## `by(bit_vector)` — bit-vector theory for bitwise/truncation proofs

In its default prover mode, Verus treats bitwise operations (`&`, `|`, `^`,
`<<`, `>>`) as uninterpreted functions. Even basic facts like
`x & y == y & x` are not available. The `bit_vector` prover mode encodes all
integers into the [Z3 `bv` type](https://microsoft.github.io/z3guide/docs/theories/Bitvectors/)
using "bit blasting," making bitwise properties decidable.

### Inline assertions

`assert(Q) by(bit_vector)` proves a short, context-free bit-manipulation
property. As with `nonlinear_arith`, ambient facts are not included unless
supplied via `requires`:

```rust
fn test_passes(b: u32) {
    assert(b & 7 == b % 8) by (bit_vector);
    assert(b & 0xff < 0x100) by (bit_vector);
}
```

Context must be imported explicitly:

```rust
fn test_success(x: u32, y: u32)
    requires
        x == y,
{
    assert(x & 3 == y & 3) by (bit_vector)
        requires
            x == y,
    ;  // now x == y is available for the bit_vector proof
}
```

### Proof functions with `by(bit_vector)`

```rust
proof fn de_morgan_auto()
    by (bit_vector)
    ensures
        forall|a: u32, b: u32| #[trigger] (!(a & b)) == !a | !b,
        forall|a: u32, b: u32| #[trigger] (!(a | b)) == !a & !b,
{
}
```

### Spec functions in bit-vector proofs

The `bit_vector` solver supports constants and `spec` functions (by inlining
them), restricted to bit-vector and arithmetic operations:

```rust
spec fn get_bit(val: u32, index: u32) -> bool {
    0x1u32 & (val >> index) == 1
}

fn test_get_bit() {
    assert(get_bit(128u32, 7)) by (bit_vector);
}
```

### Truncation and wrapping

The `bit_vector` solver is one of the easiest ways to reason about truncation:

```rust
proof fn test_truncation(a: u64) {
    assert(a as u32 == a & 0xffff_ffff) by(bit_vector);

    // You can write an identity with modulus as well:
    assert(a as u32 == a % 0x1_0000_0000) by(bit_vector);
}
```

The named functions `add(x, y)`, `sub(x, y)`, and `mul(x, y)` automatically
truncate (unlike `+`, `-`, `*` which widen to `int` in ghost code):

```rust
proof fn test_truncating_add(a: u64, b: u64) {
    assert(add(a, b) == (a + b) as u64) by(bit_vector);
}
```

### What it's good at

The `bit_vector` solver is ideal for bitwise operations (`&`, `|`, `^`, `<<`,
`>>`). It can also handle bounded integer arithmetic (`+`, `-`, `*`, `/`, `%`).
For symbolic `int`/`nat` values it cannot choose a bitwidth, but it *can* handle
`int`-typed results of operations on bounded types (e.g., `x + y` where `x, y`
are `u64` is representable in 65 bits).

For `usize`/`isize`, the solver generates two queries (32-bit and 64-bit) unless
a platform size is configured via a `global` directive.

---

## `by(compute)` / `compute_only` — Verus's built-in computational evaluator

Some proofs are "obvious" by simply computing on values. For example, given a
recursive `pow` function, proving `pow(2, 8) == 256` should be straightforward —
but Z3 may not unroll the definition enough or may refuse to simplify nonlinear
operations on constants. Verus's internal interpreter can do this deterministically.

### `assert(e) by (compute)`

Runs the interpreter to simplify `e` to an expression `e'`, then replaces the
statement with `assert(e')`. Even if simplification only partially succeeds, Z3
may complete the proof. The original expression is also assumed, so it can
trigger ambient knowledge:

```rust
// Naive definition of exponentiation
spec fn pow(base: nat, exp: nat) -> nat
    decreases exp,
{
    if exp == 0 {
        1
    } else {
        base * pow(base, (exp - 1) as nat)
    }
}

proof fn concrete_pow() {
    assert(pow(2, 8) == 256) by (compute);  // Assertion 1
    assert(pow(2, 9) == 512);  // Assertion 2
    assert(pow(2, 8) == 256) by (compute_only);  // Assertion 3
}
```

- **Assertion 1:** The interpreter reduces `pow(2, 8)` to `256`, simplifies to
  `true`, then assumes `pow(2, 8) == 256`.
- **Assertion 2:** Succeeds because Z3 unfolds `pow` once and uses the
  previously established fact.
- **Assertion 3:** `compute_only` fails unless the interpreter reduces the
  expression completely to `true` — useful for proof stability since it relies
  on no Z3 heuristics.

### No inherited context

Proofs by computation do **not** inherit context from their environment. Local
variables are treated symbolically. This fails:

```rust
let x = 2;
assert(pow(2, x) == 4) by (compute_only);
```

Move the `let` into the assertion:

```rust
proof fn let_passes() {
    assert({
        let x = 2;
        pow(2, x) == 4
    }) by (compute_only);
}
```

### Symbolic computation

The interpreter also supports symbolic values:

```rust
proof fn seq_example(a: Seq<int>, b: Seq<int>, c: Seq<int>, d: Seq<int>) {
    assert(seq![a, b, c, d] =~= seq![a, b].add(seq![c, d])) by (compute_only);
}
```

### Memoization with `#[verifier::memoize]`

By default, the interpreter does not cache function call results. For functions
that would be impractical to evaluate naively (e.g., naive Fibonacci), annotate
with `#[verifier::memoize]`:

```rust
#[verifier::memoize]
spec fn fibonacci(n: nat) -> nat
    decreases n
{
    if n == 0 {
        0
    } else if n == 1 {
        1
    } else {
        fibonacci((n - 2) as nat) + fibonacci((n - 1) as nat)
    }
}

proof fn test_fibonacci() {
    assert(fibonacci(63) == 6557470319842) by(compute_only);
}
```

### `all_spec` for bounded ranges

To reduce boilerplate for proofs over a concrete range of integers, use
[`all_spec`](https://verus-lang.github.io/verus/verusdoc/vstd/compute/trait.RangeAll.html#tymethod.all_spec):

```rust
use vstd::compute::RangeAll;

spec fn p(u: usize) -> bool {
    u >> 8 == 0
}

proof fn range_property(u: usize)
    requires 25 <= u < 100,
    ensures p(u),
{
    assert((25..100int).all_spec(|x| p(x as usize))) by (compute_only);
    let prop = |x| p(x as usize);
    assert(prop(u));
}
```

### Limitations

- The expression is interpreted in isolation (no surrounding context).
- The expression must be in spec mode (cannot be used on proof or exec mode
  functions).
- The interpreter is recursive; deeply nested expressions may exceed stack
  space.
- The time limit is the `--rlimit` value (in seconds).

---

## Overflow handling

Whenever Verus executable code performs a mathematical operation on concrete
(non-ghost) variables, Verus proves it doesn't overflow. This prevents a common
bug class and simplifies reasoning, but creates an obligation on the developer.

### Explicit bounds

Place explicit bounds on variables so the solver can infer no overflow:

```rust
fn compute_sum_limited(x: u64, y: u64) -> (result: u64)
    requires
        x < 1000000,
        y < 1000000,
    ensures
        result == x + y,
{
    x + y
}
```

### Runtime checks with `checked_add` / `checked_mul`

Use Rust standard library operations like `checked_add` and `checked_mul`, which
return an `Option` (`None` indicates overflow). Verus includes specifications for
these:

```rust
fn compute_sum_runtime_check(x: u64, y: u64) -> (result: Option<u64>)
    ensures
        match result {
            Some(z) => z == x + y,
            None => x + y > u64::MAX,
        },
{
    x.checked_add(y)
}
```

### `CheckedU8`/`CheckedU16`/.../`CheckedU64`

For chained operations, the vstd structs `CheckedU8`, `CheckedU16`, etc. can
continue operating even after overflow, maintaining the true non-overflowing
value in ghost state. This simplifies proofs — no need for monotonicity lemmas
to prove non-overflow:

```rust
fn fib_checked(n: u64) -> (result: u64)
    requires
        fib(n as nat) <= u64::MAX
    ensures
        result == fib(n as nat),
{
    if n == 0 {
        return 0;
    }
    let mut prev: CheckedU64 = CheckedU64::new(0);
    let mut cur: CheckedU64 = CheckedU64::new(1);
    let mut i: u64 = 1;
    while i < n
        invariant
            0 < i <= n,
            fib(n as nat) <= u64::MAX,
            cur@ == fib(i as nat),
            prev@ == fib((i - 1) as nat),
        decreases n - i,
    {
        i = i + 1;
        let new_cur = cur.add_checked(&prev);
        prev = cur;
        cur = new_cur;
    }
    cur.unwrap()
}
```

The small cost is runtime memory (a `CheckedU64` is an `Option<u64>` plus a
ghost `nat`), but the code is simpler and can handle overflow gracefully.

### `int` vs. fixed-width: ghost arithmetic never overflows

In **ghost** code (specifications, proof code), arithmetic operations (`+`,
`-`, `*`, `/`, `%`) never overflow or wrap — Verus widens results to `int`:

```rust
fn test_sum2(x: u8, y: u8) {
    assert({
        let sum2: int = x + y;  // in ghost code, + returns int and does not overflow
        0 <= sum2 < 511
    });
}
```

This makes it easy to write specifications *about* overflow — to ensure
executable `x + y` doesn't overflow, write `requires x + y < 256` (where `x + y`
is widened to `int`):

```rust
fn test_sum3(x: u8, y: u8)
    requires
        x + y < 256,  // make sure "let sum1: u8 = x + y" can't overflow
{
    let sum1: u8 = x + y;  // succeeds
}
```

The named functions `add(x, y)`, `sub(x, y)`, `mul(x, y)` do **not** widen and
thus truncate even in ghost code. In spec code, `/` and `%` use Euclidean
division (the remainder is always nonnegative), which differs from Rust's
truncating division for negative operands.

> See [verification-model.md](./verification-model.md) (sibling) for the full
> type-widening table and the `int`/`nat`/fixed-width type guidance.

---

## The vstd arithmetic lemma library

When prover modes are insufficient, the
[`vstd::arithmetic`](https://verus-lang.github.io/verus/verusdoc/vstd/arithmetic/index.html)
library supplies a large collection of verified facts about nonlinear operations.
These are `proof fn` lemmas you call explicitly (inside `proof { }` blocks or
`assert(...) by { ... }`).

### `vstd::arithmetic::mul` — multiplication lemmas

All are `pub broadcast proof fn` (can be used with `broadcast use` or called
directly). Confirmed in `source/vstd/arithmetic/mul.rs`:

| Lemma | Ensures | Requires |
|-------|---------|----------|
| `lemma_mul_is_commutative(x, y)` | `#[trigger] (x * y) == y * x` | — |
| `lemma_mul_is_associative(x, y, z)` | `x * (y * z) == (x * y) * z` | — |
| `lemma_mul_is_distributive_add(x, y, z)` | `x * (y + z) == x * y + x * z` | — |
| `lemma_mul_is_distributive_sub(x, y, z)` | `x * (y - z) == x * y - x * z` | — |
| `lemma_mul_upper_bound(x, xbound, y, ybound)` | `x * y <= xbound * ybound` | `x <= xbound`, `y <= ybound`, `0 <= x`, `0 <= y` |
| `lemma_mul_strict_upper_bound(x, xbound, y, ybound)` | `x * y <= (xbound-1) * (ybound-1)` | `x < xbound`, `y < ybound`, `0 < x`, `0 < y` |
| `lemma_mul_inequality(x, y, z)` | `x * z <= y * z` | `x <= y`, `z >= 0` |
| `lemma_mul_strict_inequality(x, y, z)` | `x * z < y * z` | `x < y`, `z > 0` |
| `lemma_mul_nonzero(x, y)` | `x * y != 0 <==> x != 0 && y != 0` | — |
| `lemma_mul_by_zero_is_zero(x)` | `x * 0 == 0 && 0 * x == 0` | — |
| `lemma_mul_ordering(x, y)` | `x * y >= x && x * y >= y` | `x != 0`, `y != 0`, `0 <= x * y` |
| `lemma_mul_strictly_positive(x, y)` | (product is positive) | — |
| `lemma_mul_nonnegative(x, y)` | (product is nonnegative) | — |
| `lemma_mul_increases(x, y)` | (monotonicity) | — |
| `lemma_mul_unary_negation(x, y)` | (negation properties) | — |

Broadcast groups: `group_mul_is_distributive`,
`group_mul_is_commutative_and_distributive`, `group_mul_basics`.

### `vstd::arithmetic::div_mod` — division and modulo lemmas

Confirmed in `source/vstd/arithmetic/div_mod.rs`:

| Lemma | Ensures | Requires |
|-------|---------|----------|
| `lemma_fundamental_div_mod(x, d)` | `x == d * (x / d) + (x % d)` | `d != 0` |
| `lemma_div_by_self(d)` | `d / d == 1` | — |
| `lemma_div_of0(d)` | `0 / d == 0` | — |
| `lemma_div_basics(x)` | basic division identities | — |
| `lemma_div_non_zero(x, d)` | `x / d == 0` when `0 <= x < d` | — |
| `lemma_small_mod(x, m)` | `x % m == x` when `x < m` | `x < m` (both `nat`) |
| `lemma_remainder(x, d)` | `0 <= x % d < d` | — |
| `lemma_remainder_upper(x, d)` | `x % d < d` | — |
| `lemma_remainder_lower(x, d)` | `0 <= x % d` | — |
| `lemma_div_by_multiple(b, d)` | `b / d` properties | — |
| `lemma_div_decreases(x, d)` | `x / d <= x` | — |
| `lemma_div_pos_is_pos(x, d)` | `0 <= x / d` | — |
| `lemma_mod_neg_neg(x, d)` | `x % d == (x * (1 - d)) % d` | — |
| `lemma_fundamental_div_mod_converse(x, d, q, r)` | converse | — |
| `lemma_truncate_middle(x, b, c)` | truncation property | — |

### `vstd::arithmetic::power` — exponentiation

`pub open spec fn pow(b: int, e: nat) -> int` (opaque, recursive). Confirmed in
`source/vstd/arithmetic/power.rs`:

| Lemma | Ensures |
|-------|---------|
| `lemma_pow0(b)` | `pow(b, 0) == 1` |
| `lemma_pow1(b)` | `pow(b, 1) == b` |
| `lemma0_pow(e)` | `pow(0, e) == 0` (requires `e > 0`) |
| `lemma1_pow(e)` | `pow(1, e) == 1` |
| `lemma_square_is_pow2(x)` | `pow(x, 2) == x * x` |
| `lemma_pow_positive(b, e)` | `0 < pow(b, e)` (requires `b > 0`) |
| `lemma_pow_adds(b, e1, e2)` | `pow(b, e1 + e2) == pow(b, e1) * pow(b, e2)` |
| `lemma_pow_subtracts(b, e1, e2)` | subtraction property |
| `lemma_pow_strictly_increases(b, e1, e2)` | monotonicity |
| `lemma_pow_division_inequality(x, b, e1, e2)` | division inequality |

### `vstd::arithmetic::power2` — powers of two

`pub open spec fn pow2(e: nat) -> nat` (defined as `pow(2, e) as nat`),
`pub open spec fn is_pow2(n: int) -> bool` (recursive),
`pub open spec fn is_pow2_exists(n: int) -> bool` (existential). Confirmed in
`source/vstd/arithmetic/power2.rs`:

| Lemma | Ensures |
|-------|---------|
| `is_pow2_equiv(n)` | `is_pow2(n) <==> is_pow2_exists(n)` |

### `vstd::arithmetic::overflow` — checked integer structs

Defines `CheckedU8`, `CheckedU16`, `CheckedU32`, `CheckedU64`, `CheckedU128`,
`CheckedUsize` (and signed variants). Each struct holds a ghost `nat` (the true
value) and an `Option<uN>` (the runtime value, `None` if overflowed). Confirmed
in `source/vstd/arithmetic/overflow.rs`. Key methods: `new`, `add_value`,
`mul_value`, `add_checked`, `is_overflowed`, `unwrap`, `to_option`, `view` (via
the `@` operator).

### `vstd::arithmetic::logarithm` — integer logarithm

`pub open spec fn log(base: int, pow: int) -> int` (opaque, recursive; meaningful
when `base > 1` and `pow >= 0`). Confirmed in
`source/vstd/arithmetic/logarithm.rs`:

| Lemma | Ensures | Requires |
|-------|---------|----------|
| `lemma_log0(base, pow)` | `log(base, pow) == 0` | `base > 1`, `0 <= pow < base` |
| `lemma_log_s(base, pow)` | `log(base, pow) == 1 + log(base, pow / base)` | `base > 1`, `pow >= base` |
| `lemma_log_nonnegative(base, pow)` | `log(base, pow) >= 0` | — |
| `lemma_log_is_ordered(base, pow1, pow2)` | ordering property | — |
| `lemma_log_pow(base, n)` | `log(base, pow(base, n)) == n` | — |

---

## Sources used

Tutorial chapters read (`.tmp/verus/source/docs/guide/src/`):

- `nonlinear.md` — integers: nonlinear arithmetic (`nonlinear_arith`,
  `integer_ring`, helper pattern, Singular encoding)
- `bitvec.md` — bit vectors and bitwise operations (`bit_vector`)
- `overflow.md` — proving absence of overflow
- `assert_by_compute.md` — proofs by computation (`compute`, `compute_only`)
- `integers.md` — integer types, constants, coercions, ghost vs. exec arithmetic
- `spec-arithmetic.md` — arithmetic in spec code (widening table, Euclidean
  division)
- `install-singular.md` — installing and configuring Singular
- `reference-prover-mode-nonlinear.md` — the nonlinear solver (reference)
- `reference-prover-mode-integer-ring.md` — the `integer_ring` solver (reference)
- `reference-prover-mode-bit-vector.md` — the `bit_vector` prover mode (reference)
- `reference-prover-mode-compute.md` — the compute mode (reference)
- `reference-assert-by-prover.md` — `assert ... by(...)` (reference)
- `overview.md` — Verus overview

Example files read (verbatim code snippets sourced from):

- `examples/guide/nonlinear_bitvec.rs` — anchors: `bound_checking`,
  `bound_checking_func`, `bitvector_easy`, `bitvector_success`, `de_morgan`,
  `bitvector_spec_fn`
- `examples/guide/assert_by_compute.rs` — anchors: `pow_concrete`, `let_passes`,
  `seq_example`, `fibonacci_memoize`, `all_spec`
- `examples/guide/overflow.rs` — anchors: `compute_sum_limited`,
  `compute_sum_runtime_check`
- `examples/guide/integers.rs` — anchors: `test_u8`, `test_sum2`, `test_sum3`
- `examples/guide/invariants.rs` — anchor: `fib_checked`
- `examples/integer_ring/integer_ring.rs` — `mod_of_mul_int`, `mod_of_mul`,
  `mod_add_zero_int`, `LemmaMulIsDistributive`, and more (full file)
- `examples/integer_ring/integer_ring_bound_check.rs` — `ModAfterMul`,
  `LemmaMulUpperBound`, bound-check pattern (full file)
- `examples/integer_ring/circular_by_d.rs` — `lemma_mod_difference_equal_helper`,
  `lemma_mod_difference_equal`, `lemma_mod_between` (full file)
- `examples/assert_by_compute.rs` — Fibonacci, exponentiation, recursive data
  structures, sequences (full file)

vstd arithmetic library confirmed (`.tmp/verus/source/vstd/arithmetic/`):

- `mul.rs` — `lemma_mul_is_commutative`, `lemma_mul_is_associative`,
  `lemma_mul_is_distributive_add`, `lemma_mul_upper_bound`,
  `lemma_mul_inequality`, `lemma_mul_strict_inequality`, `lemma_mul_nonzero`,
  `lemma_mul_by_zero_is_zero`, `lemma_mul_ordering`, broadcast groups
- `div_mod.rs` — `lemma_fundamental_div_mod`, `lemma_div_by_self`,
  `lemma_div_of0`, `lemma_div_basics`, `lemma_small_mod`, `lemma_remainder`,
  `lemma_div_by_multiple`, `lemma_mod_neg_neg`
- `power.rs` — `pow`, `lemma_pow0`, `lemma_pow1`, `lemma_pow_positive`,
  `lemma_pow_adds`, `lemma_pow_subtracts`, `lemma_pow_strictly_increases`,
  `lemma_pow_division_inequality`
- `power2.rs` — `pow2`, `is_pow2`, `is_pow2_exists`, `is_pow2_equiv`
- `overflow.rs` — `CheckedU8`/`CheckedU16`/.../`CheckedU64` structs
- `logarithm.rs` — `log`, `lemma_log0`, `lemma_log_s`,
  `lemma_log_nonnegative`, `lemma_log_is_ordered`, `lemma_log_pow`
- `mod.rs` — module structure (`div_mod`, `logarithm`, `mul`, `overflow`,
  `power`, `power2`, `internals`)
