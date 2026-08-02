# Proof Techniques in Verus

Verus verifies executable Rust code against user-provided specifications by
generating verification conditions that an SMT solver (Z3) discharges. The
*proof* layer — `proof fn` lemmas, `proof { }` blocks, `assert`, `assume`,
`assert(...) by { ... }`, `calc!`, `broadcast`, and related constructs — is how
you guide the solver when automation alone is insufficient. This doc covers the
core proof techniques. For arithmetic-specific provers (`nonlinear_arith`,
`integer_ring`, `bit_vector`, `compute`), see
[arithmetic-and-provers.md](./arithmetic-and-provers.md).

> **Source fidelity:** Every code snippet below is copied verbatim from the
> Verus clone at `.tmp/verus/` (tutorial chapters under
> `source/docs/guide/src/` and example files under `examples/`). Anchor names
> reference the `// ANCHOR:` markers in those example files.

---

## `proof fn` — lemmas with `requires`/`ensures`

A `proof fn` (proof function) is a ghost function that is **not compiled** to
executable code. Its purpose is to reveal or prove properties about
specifications — particularly about `closed spec` functions whose bodies are
hidden from other modules.

Like `exec` functions, `proof` functions may have `requires` and `ensures`
clauses. Unlike `exec` functions, they are ghost and erased before compilation.

In the example below, module `M1` defines a `closed spec fn min` and a
`proof fn lemma_min` that reveals properties of `min` (it is no larger than
either argument, and equals one of them) without revealing `min`'s definition.
Module `M2` cannot see `min`'s body, but can call `lemma_min` to learn about
`min`:

```rust
mod M1 {
    use verus_builtin::*;

    pub closed spec fn min(x: int, y: int) -> int {
        if x <= y {
            x
        } else {
            y
        }
    }

    pub proof fn lemma_min(x: int, y: int)
        ensures
            min(x, y) <= x,
            min(x, y) <= y,
            min(x, y) == x || min(x, y) == y,
    {
    }
}

mod M2 {
    use verus_builtin::*;
    use crate::M1::*;

    proof fn test() {
        lemma_min(10, 20);
        assert(min(10, 20) == 10); // succeeds
        assert(min(100, 200) == 100); // FAILS
    }
}
```

The call `lemma_min(10, 20)` adds the `ensures` facts to the proof environment,
so the first assertion succeeds. The second assertion fails because
`lemma_min(100, 200)` was never called — each lemma invocation only proves the
property for the specific arguments passed.

> See [verification-model.md](./verification-model.md) (sibling) for `open`/`closed spec fn`
> and the spec-vs-proof-vs-exec mode table.

---

## Proof blocks `proof { }`

`exec` functions can contain pieces of proof code in *proof blocks*, written
with `proof { ... }`. A proof block can use all ghost-code features that
`proof` functions can, such as the `int` and `nat` types. The entire block
(including local variables) is erased before compilation.

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

Proof blocks can call `proof` functions. In fact, any call from an `exec`
function to a `proof` function must appear inside proof code such as a proof
block — this clarifies which code is executable and which is ghost:

```rust
mod M1 {
    use verus_builtin::*;

    pub closed spec fn min(x: int, y: int) -> int {
        if x <= y {
            x
        } else {
            y
        }
    }

    pub proof fn lemma_min(x: int, y: int)
        ensures
            min(x, y) <= x,
            min(x, y) <= y,
            min(x, y) == x || min(x, y) == y,
    {
    }

}

mod M2 {
    use verus_builtin::*;
    use crate::M1::*;

    fn test() {
        proof {
            lemma_min(10, 20);
            lemma_min(100, 200);
        }
        assert(min(10, 20) == 10);  // succeeds
        assert(min(100, 200) == 100);  // succeeds
    }

}
```

---

## `assert` and `assume`

### `assert` — SMT-checked

`assert(expr);` asks the SMT solver to **prove** the given expression using the
default solver, and then assumes it for subsequent code. If the solver cannot
prove it, verification fails.

### `assume` — the soundness escape

`assume(expr);` assumes the given predicate **without proof**. This is
unchecked and can subvert Verus's guarantees. Successful "verification"
provides **no guarantees** if it includes any `assume` statements, unless those
`assume` statements could in principle be replaced by a successful `assert`.

The `--no-cheating` flag disallows `assume` statements entirely.

`assume` is most useful during *intermediate* stages of development, within an
assert/assume-driven proof-development process.

### Developing proofs with `assert` and `assume`

The standard workflow is:

1. Start with a proof outline.
2. Run it; observe which postcondition fails.
3. `assume` the failing postcondition to check whether the rest of the proof
   would succeed if that step were proven.
4. Narrow in: move the `assume` into each branch, then convert `assume`→`assert`
   one branch at a time.
5. When an `assert` fails, add intermediate `assert`s (or `assume`s) to find
   the missing fact.
6. Replace every `assume` with a real `assert` (possibly via a lemma or
   extensional equality) until no `assume`s remain.

The final, cleaned-up result of this process for a lemma about set-intersection
cardinality looks like this (the `assume`s have all been eliminated):

```rust
pub proof fn lemma_len_intersect<A>(s1: Set<A>, s2: Set<A>)
    ensures
        s1.intersect(s2).len() <= s1.len(),
    decreases s1.len(),
{
    if s1.is_empty() {
        assert(s1.intersect(s2).len() == 0) by {
            assert(s1.intersect(s2) =~= s1);
        }
    } else {
        let a = s1.choose();
        lemma_len_intersect(s1.remove(a), s2);
        // by induction: s1.remove(a).intersect(s2).len() <= s1.remove(a).len()
        assert(s1.intersect(s2).remove(a).len() <= s1.remove(a).len()) by {
            assert(s1.intersect(s2).remove(a) =~= s1.remove(a).intersect(s2));
        }
        // simplifying ".remove(a).len()" yields s1.intersect(s2).len() <= s1.len())

    }
}
```

A key technique visible above: when the SMT solver cannot prove an equality
between collections, assert **extensional equality** (`=~=`) to give the solver
the fact it needs.

---

## `assert(...) by { ... }` — scoped proofs

In a long function, establishing a fact `F` with a proof `P` risks polluting the
solver's context: the facts `P` introduces can be used not only for `F` but for
the entire rest of the function, which can slow the solver or cause timeouts.

`assert(F) by { P }` restricts the context that `P` affects. After the closing
brace, all facts established by `P` **except** `F` are removed from the proof
context. Internally, the solver is given the facts from `P` as a premise when
proving `F`, but not for the rest of the proof.

```rust
mod M1 {
    use verus_builtin::*;

    pub closed spec fn min(x: int, y: int) -> int {
        if x <= y {
            x
        } else {
            y
        }
    }

    pub proof fn lemma_min(x: int, y: int)
        ensures
            min(x, y) <= x,
            min(x, y) <= y,
            min(x, y) == x || min(x, y) == y,
    {
    }
}

mod M2 {
    use verus_builtin::*;
    use crate::M1::*;

    fn test() {
        assert(min(10, 20) == 10) by {
            lemma_min(10, 20);
            lemma_min(100, 200);
        }
        assert(min(10, 20) == 10); // succeeds
        assert(min(100, 200) == 100); // FAILS
    }
}
```

The `lemma_min(100, 200)` call inside the `by` block helps prove
`min(10, 20) == 10`, but its facts do not propagate outside the block, so the
final assertion fails.

This is an alternative to extracting a separate lemma: `assert(F) by { P }`
obviates the need to figure out which context `P` needs as `requires` clauses,
making the proof more compact.

---

## `assert forall ... by` — proving universally-quantified goals

Proving a `forall` expression and using an `exists` expression are the two
cases that often "just work" without triggers. But when a `forall` proof
relies on a lemma proved by induction, the solver cannot prove it automatically
— and you cannot call the lemma because the quantified variable is not in
scope.

`assert forall |binders| P implies Q by { ... }` solves this: inside the body,
the variables of the `forall` are in scope and the left-hand side `P` of the
`==>` is assumed. The simplified form `assert forall |binders| Q by { ... }` is
equivalent to `assert forall |binders| true implies Q by { ... }`.

```rust
proof fn test_even_f()
    ensures
        forall|i: int| is_even(i) ==> f(i),
{
    assert forall|i: int| is_even(i) implies f(i) by {
        // First, i is in scope here
        // Second, we assume is_even(i) here
        lemma_even_f(i);
        // Finally, we have to prove f(i) here
    }
}
```

As with ordinary `assert ... by`, the proof inside the body does not enter the
context for the remainder.

For `exists` variables that are not in scope, use `choose` to extract a
witness:

```rust
proof fn test_g_proves_f(i: int)
    requires
        exists|j: int| g(i, j),
    ensures
        f(i),
{
    lemma_g_proves_f(i, choose|j: int| g(i, j));
}
```

> See [specifications.md](./specifications.md) (sibling) for triggers, matching
> loops, and the full forall/exists proving/using table.

---

## `broadcast` — ambient quantified lemmas

A `broadcast proof fn` introduces a quantified fact into the proof environment
automatically, without manually invoking it at every call site. This is useful
for facts so "obvious" that most programmers expect Verus to always know them
(e.g., an empty sequence's length is zero). Use with caution: every ambient
fact slows the solver.

### Defining a broadcast lemma

Adding the `broadcast` modifier to a proof fn introduces a quantified fact.
Because this introduces a quantifier, Verus typically requires an explicit
`#[trigger]` annotation:

```rust
pub broadcast proof fn seq_contains_orig_elems_after_push<A>(s:Seq<A>, v:A, x:A)
    requires s.contains(x)
    ensures #[trigger] s.push(v).contains(x)
{
    ...
}
```

This introduces the ambient fact:

```rust
forall |s: Seq<A>, v: A, x: A| s.contains(x) ==> s.push(v).contains(x)
```

### Bringing it into scope

```rust
broadcast use seq_contains_orig_elems_after_push;
```

This can be placed in a specific proof or at module level.

### Broadcast groups

Multiple broadcast lemmas can be combined into a named group, then brought into
scope with a single `broadcast use`:

```rust
      pub broadcast proof fn lemma_add_aligned(p: Self, v: Self)
        requires
          p.aligned(), v.aligned(), p.modulo == v.modulo,
        ensures
          (#[trigger] p.add(v)).aligned(),
          p.add(v).modulo == lib::same_or_arbitrary(p.modulo, v.modulo),
      {
        super::lib::mod_add_zero(p.i as int, v.i as int, p.modulo as int);
      }

      pub broadcast proof fn lemma_mul_aligned(p: Self, v: Self)
        requires
          p.aligned(), v.aligned(), p.modulo == v.modulo,
        ensures
          (#[trigger] p.mul(v)).aligned(),
          p.mul(v).modulo == lib::same_or_arbitrary(p.modulo, v.modulo),
      {
        // TODO
        admit();
      }

      pub broadcast group group_properties {
        Multiple::lemma_add_aligned,
        Multiple::lemma_mul_aligned,
      }
```

Consumers can then bring in the entire group:

```rust
    broadcast use Multiple::group_properties;
```

or a single lemma:

```rust
    broadcast use Multiple::lemma_add_aligned;
```

### vstd broadcast groups

The Verus standard library ships broadcast groups. For example,
`vstd::seq_lib::group_seq_properties` bundles common sequence facts. The
top-level group `vstd::group_vstd_default` is brought into scope by default
when the `vstd` crate is imported (it includes `seq::group_seq_axioms`,
`set::group_set_lemmas`, `map::group_map_lemmas`, `compute::all_spec_ensures`,
`laws_eq::group_laws_eq`, and more).

### Experimental usage info: `-V axiom-usage-info`

The `-V axiom-usage-info` experimental flag reports an overapproximation of the
broadcasted axioms and lemmas used in verifying each function. For large
projects, combine with `--verify-only-module` and `--verify-function` to limit
output. For example, on `examples/broadcast_proof.rs`, the `increase_twice`
function produces:

```
note: checking this function used these broadcasted lemmas and broadcast groups:
        - (group) broadcast_proof::multiple_broadcast_proof::Multiple::group_properties,
        - broadcast_proof::multiple_broadcast_proof::Multiple::lemma_add_aligned
   --> ../examples/broadcast_proof.rs:161:11
    |
161 |       proof fn increase_twice(
    |  ___________^
162 | |         p1: Multiple, v: Multiple, p2: Multiple)
    | |________________________________________________^
```

---

## `calc!` — structured calculation proofs

The `calc!` macro supports structured proofs through calculations: you show
`a_1 R a_n` for a transitive relation `R` by performing a series of steps
`a_1 R a_2`, `a_2 R a_3`, ... `a_{n-1} R a_n`. Each intermediate expression is
mentioned only once, and the proof for each step is localized to that step,
limiting proof-context pollution. The body where `calc!` is written only sees
`a_1 R a_n`, not the intermediate steps.

```rust
    let a: int = 2;
    calc! {
        (<=)
        a; {}
        a + 3; {}
        5;
    }
```

This is equivalent to proving `a <= 5` using `a <= a + 3 <= 5`. Each step's
proof block is `{}` (trivial) here, but can contain arbitrary proofs.

### Different relations for intermediate steps

Not every step needs the top-level relation; you can be more precise inline
(e.g., `a_1 <= a_2 == a_3 <= a_4 < a_5`). Intermediate relations are checked
for consistency with the top-level relation to maintain transitivity:

```rust
    let x: int = 2;
    let y: int = 5;
    calc! {
        (<=)
        x; (==) {}
        5 - 3; (<) {}
        5int; {}  // Notice that no intermediate relation
                  // is specified here, so `calc!` will
                  // use the top-level relation, here `<=`.
        y;
    }
```

This is equivalent to `x <= y` using `x == 5 - 3 < 5 <= y`.

Currently `calc!` supports common transitive relations for `R` (such as `==`,
`<=`, `<`, `==>`, `<==>`).

---

## `opaque` / `reveal` / `hide` — controlling spec fn body visibility

A common cause of verification timeouts is unfolding a function definition that
is complex or contains problematic quantifiers. `opaque` hides the body of a
spec function from the verifier; `reveal` selectively exposes it.

```rust
mod M1 {
    use verus_builtin::*;

    #[verifier::opaque]
    spec fn min(x: int, y: int) -> int {
        if x <= y {
            x
        } else {
            y
        }
    }

    fn test() {
        assert(min(10, 20) == min(10, 20)); // succeeds
        assert(min(10, 20) == 10); // FAILS
        reveal(min);
        assert(min(10, 20) <= 10); // succeeds
    }

}
```

`opaque` hides the body **even in the current module**, so you can `reveal` it
in specific proof blocks. This differs from `closed spec`, which hides the body
only from *other* modules but keeps it visible in the defining module. Use
`closed spec` for abstraction/modularity; use `opaque`/`reveal` for controlling
automation and verification performance.

### `reveal_with_fuel` and `hide`

- `reveal(f)` directs Verus to unfold the definition of `f` when it encounters
  a use of `f`.
- `hide(f)` directs Verus to treat `f` as an uninterpreted function without
  reasoning about its definition.
- `reveal_with_fuel(f, n)` is for recursive functions: the integer `n` indicates
  how many times Verus should unfold. Limiting fuel avoids trigger loops. The
  default fuel (absent any directive) is 1.

### `reveal_strlit`

`reveal_strlit("abc")` reveals the contents of a string literal to the prover
(its length and character sequence). Otherwise, string literals are treated as
opaque data to avoid polluting the prover's context:

```rust
fn test() {
    let s = "abc";
    proof { reveal_strlit("abc"); }
    assert(s@[0] == 'a');
}
```

---

## `recommends` — soft preconditions

`spec` functions cannot have `requires`/`ensures` clauses (this keeps the
specification language close to the SMT solver's mathematical language for
efficiency). However, `spec` functions may contain `recommends` clauses —
lightweight recommendations rather than hard requirements.

Callers are under no obligation to obey a recommendation. By default, Verus does
not check recommendations at all. However, *if* there is a verification error in
a function, Verus automatically reruns verification with recommendation checking
turned on, in hopes that recommendation failures help diagnose the error:

```rust
spec fn f(i: nat) -> nat
    recommends
        i > 0,
{
    (i - 1) as nat
}

proof fn test1() {
    assert(f(0) == f(0));  // succeeds
}
```

By default, Verus does not perform `recommends` checking on calls from `spec`
functions. Writing `spec(checked)` requests `recommends` checking, which
generates warnings for violations — useful for specifications in the trusted
computing base describing interfaces to external, unverified components:

```rust
spec(checked) fn caller2() -> nat {
    f(0)  // generates a warning because of "(checked)"

}
```

---

## Induction — recursive proof functions

A recursive `proof fn` whose `ensures` clause is the induction hypothesis
implements proof by induction. The base case is handled directly; the induction
step is a recursive call substituting a smaller value.

For example, to prove that `triangle` is monotonic (`i <= j ==> triangle(i) <=
triangle(j)`), we induct on `j`. The base case is `j == 0`; the induction step
calls `triangle_is_monotonic(i, (j - 1) as nat)`:

```rust
proof fn triangle_is_monotonic(i: nat, j: nat)
    ensures
        i <= j ==> triangle(i) <= triangle(j),
    decreases j,
{
    // We prove the statement `i <= j ==> triangle(i) <= triangle(j)`
    // by induction on `j`.

    if j == 0 {
        // The base case (`j == 0`) is trivial since it's only
        // necessary to reason about when `i` and `j` are both 0.
        // So no proof lines are needed for this case.
    }
    else {
        // In the induction step, we can assume the statement is true
        // for `j - 1`. In Verus, we can get that fact into scope with
        // a recursive call substituting `j - 1` for `j`.

        triangle_is_monotonic(i, (j - 1) as nat);

        // Once we know it's true for `j - 1`, the rest of the proof
        // is trivial.
    }
}
```

As with recursive `spec` functions, recursive `proof` functions must terminate
and need a `decreases` clause. Without it, you could prove `false` via
non-terminating "proofs" like:

```rust
proof fn circular_reasoning()
    ensures
        false,
{
    circular_reasoning(); // FAILS, does not terminate
}
```

The `triangle_is_monotonic` lemma can then be used to discharge overflow
obligations in tail-recursive or loop-based implementations:

```rust
fn tail_triangle(n: u32, idx: u32, sum: &mut u32)
    requires
        idx <= n,
        *old(sum) == triangle(idx as nat),
        triangle(n as nat) <= u32::MAX,
    ensures
        *final(sum) == triangle(n as nat),
    decreases n - idx,
{
    if idx < n {
        let idx = idx + 1;
        assert(*sum + idx <= u32::MAX) by {
            triangle_is_monotonic(idx as nat, n as nat);
        }
        *sum = *sum + idx;
        tail_triangle(n, idx, sum);
    }
}
```

> See [verification-model.md](./verification-model.md) (sibling) for `decreases`
> clauses, lexicographic ordering, and mutual recursion.

---

## Breaking proofs into smaller pieces

Solver response time increases nonlinearly as proof size increases: twice as
many facts gives far more than twice as many paths to search. Breaking a long
function into smaller pieces can make the difference between a timeout and a
fast success.

### Moving a subproof to a lemma

Look for a modest-size piece `P` of a long function that proves some locally
useful facts `S`. Replace `P` with a call to a lemma whose `ensures` are `S`,
then make `P` the body of that lemma. Put the necessary context as `requires`
clauses (which may involve local variables, passed as parameters):

```
proof fn my_long_function_helper(x: u64, y: int)
    requires
        f(x, y)
    ensures
        s1(x),
        s2(x, y)
{
    P1; // modest-size proof...
    P2; //   establishing...
    P3; //   facts s1 and s2...
    P4; //   about x and y
}

fn my_long_function(x: u64, ...)
{
    ... // first part of proof, establishing fact f(x, y)
    my_long_function_helper(x, y);
    ... // second part of proof, using facts s1 and s2
}
```

Once `P` is moved, you may find that significant portions of `P` can be removed
— a lemma dedicated solely to establishing `S` has a smaller context, so less
annotation may be needed.

### Dividing a proof into parts 1, 2, ..., n

Divide the proof into `n` consecutive lemmas. The first lemma's `requires`
match the function's `requires`; its `ensures` summarize what it establishes.
Each subsequent lemma's `requires` match the previous lemma's `ensures`. The
last lemma's `ensures` are the function's `ensures`. Replace the original proof
with a sequence of calls:

```
proof fn my_long_function_part1(x: u64) -> (y: int)
    requires
        r(x)
    ensures
        mid1(x, y)
{
    P1;
}

proof fn my_long_function_part2(x: u64, y: int)
    requires
        mid1(x, y)
    ensures
        mid2(x, y)
{
    P2;
}

proof fn my_long_function_part3(x: u64, y: int)
    requires
        mid2(x, y)
    ensures
        e(x)
{
    P3;
}

proof fn my_long_function(x: u64)
    requires r(x)
    ensures  e(x)
{
    let y = my_long_function_part1(x);
	my_long_function_part2(x, y);
	my_long_function_part3(x, y);
}
```

Since `r(x)`, `mid1(x, y)`, `mid2(x, y)`, and `e(x)` are each repeated twice,
it may help to factor each out as a `spec fn`.

---

## Sources used

Tutorial chapters read (`.tmp/verus/source/docs/guide/src/`):

- `proof_functions.md` — proof functions, proof blocks, assert-by
- `assert_assume.md` — using assert and assume to develop proofs (full deep dive)
- `assert_by.md` — hiding local proofs with `assert(...) by { ... }`
- `broadcast_proof.md` — adding ambient facts with `broadcast`
- `calc.md` — structured proofs by calculation
- `opaque.md` — modules, hiding, opaque, reveal
- `reference-reveal-hide.md` — `reveal`, `reveal_with_fuel`, `hide` (reference)
- `reference-reveal-strlit.md` — `reveal_strlit` (reference)
- `spec_vs_proof.md` — spec functions vs. proof functions; recommends
- `develop_proofs.md` — developing proofs (chapter intro)
- `induction.md` — recursive exec and proof functions, proofs by induction
- `quantproofs.md` — proofs about forall and exists
- `breaking_proofs_into_pieces.md` — breaking proofs into smaller pieces
- `reference-assert.md` — `assert` (reference)
- `reference-assert-by.md` — `assert ... by` (reference)
- `reference-assert-forall-by.md` — `assert forall ... by` (reference)
- `reference-assume.md` — `assume` (reference)
- `reference-recommends.md` — `recommends` (reference)
- `overview.md` — Verus overview

Example files read (verbatim code snippets sourced from):

- `examples/guide/modes.rs` — anchors: `spec_fun_proof`, `spec_fun_proof_block1`,
  `spec_fun_proof_block2`, `assert_by`, `recommends1`, `recommends4`
- `examples/guide/calc.rs` — anchors: `simple`, `transitive`
- `examples/guide/opaque.rs` — anchor: `opaque`
- `examples/guide/recursion.rs` — anchors: `mono`, `tail`, `circular`
- `examples/guide/quants.rs` — anchors: `test_even_f`, `test_g_proves_f`
- `examples/guide/lib_examples.rs` — anchor: `lemma_len_intersect_commented`
- `examples/broadcast_proof.rs` — broadcast lemma, group, and `broadcast use`
  (full file)
