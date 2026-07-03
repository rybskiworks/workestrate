# How Verus handles Rust language features

> Source-fidelity topic doc. Every code snippet below is copied verbatim from `.tmp/verus/examples/guide/*.rs` or `.tmp/verus/source/docs/guide/src/*.md`. The supported/unsupported feature list is summarized from `.tmp/verus/source/docs/guide/src/features.md` (the authoritative 435-line reference, "Last Updated: 2026-05-13").

Verus verifies a large, carefully-chosen subset of Rust. This doc maps how Verus treats each major language feature: what works directly, what needs Verus-specific syntax, and what is unsupported. For the spec/proof side of these features, see [specifications.md](./specifications.md); for the math types that specs build on, see [vstd-library.md](./vstd-library.md).

## Mutable references and borrowing

Verus fully supports Rust's ownership, borrowing, and lifetime system. Immutable borrows are the easiest case: Verus treats `&T` the same as a non-reference `T` (a `&u32` is the same as a `u32`), so there is rarely any need to reason about pointer addresses. Lifetime variables have essentially no impact on verification — besides Rust's own lifetime checks, Verus ignores lifetimes for theorem-proving.

### Mutable borrows as function arguments

The common pattern is a function taking `&mut T`. The precondition refers to the input value with `*x` (or `*old(x)`); the postcondition relates the input to the output with `old(x)` (value at function entry) and `final(x)` (value at function exit). Verbatim from `examples/guide/references.rs`:

```rust
fn increment(a: &mut u32)
    requires *old(a) < u32::MAX,
    ensures *final(a) == *old(a) + 1,
{
    *a = *a + 1;
}

fn caller()
{
    let mut z: u32 = 0;
    increment(&mut z);
    assert(z == 1);
}
```

> **`*x` in postconditions.** Strictly, `*x` in a spec always refers to the value at the *beginning* of the function (because `x` is an input parameter). Verus requires you to disambiguate by writing `old(x)` for the input value and `final(x)` for the output value in postconditions.

### Assertions about mutable references

Inside a function, `*old(a)` is the dereferenced value at the *beginning* of the call and `*a` is the *current* value. Verbatim from `examples/guide/references.rs`:

```rust
fn check_and_assert(a: &mut u32)
    requires *old(a) == 0
{
    assert(*old(a) == 0);
    *a = *a + 1;
    assert(*a == 1);
    *a = *a + 1;
    assert(*a == 2);
    assert(*old(a) == 0);
}
```

### Returning mutable borrows

A function may return a `&mut T`, but the final value of the borrowed-from location is not known concretely at function exit (the caller may mutate through the returned reference). The postcondition must express `final(pair)` *in terms of* `final(ret)`. Verbatim from `examples/guide/mutable-references.md`:

```rust
fn get_mut_fst<A, B>(pair: &mut (A, B)) -> (ret: &mut A)
    ensures
        *ret == old(pair).0,
        *final(pair) == (*final(ret), old(pair).1),
{
    &mut pair.0
}
```

### `after_borrow`, `has_resolved`, and loops

While a local is mutably borrowed, Verus forbids reading it even in `spec` code (treated like a borrow error). The builtin `after_borrow(x)` reads the value `x` *will have* after its outstanding borrows expire — usable in loop invariants to relate a local to its mutable reference. `has_resolved(r)` states that a mutable reference `r` will never be modified again after the current point; `has_resolved(x_ref) ==> *x_ref == *final(x_ref)`. This matters for containers of mutable references (e.g. `(&mut T, &mut T)`, `Vec<&mut T>`).

## Datatypes: structs, enums, `is`, `matches`

### Structs

Verbatim from `examples/guide/datatypes.rs`:

```rust
struct Point {
    x: int,
    y: int,
}

impl Point {
    spec fn len2(&self) -> int {
        self.x * self.x + self.y * self.y
    }
}

fn rotate_90(p: Point) -> (o: Point)
    ensures o.len2() == p.len2()
{
    let o = Point { x: -p.y, y: p.x };
    assert((-p.y) * (-p.y) == p.y * p.y) by(nonlinear_arith);
    o
}
```

### Enums and the `is` operator

Enums work as in Rust. In spec contexts, the `is` operator queries which variant a value holds; `!is` is shorthand for `!(.. is ..)`. Verbatim from `examples/guide/datatypes.rs`:

```rust
enum Beverage {
    Coffee { creamers: nat, sugar: bool },
    Soda { flavor: Syrup },
    Water { ice: bool },
}

enum Syrup {
    Cola,
    RootBeer,
    Orange,
    LemonLime,
}

fn make_float(bev: Beverage) -> Dessert
    requires bev is Soda
{
    assert(bev !is Coffee);
    Dessert::new(/*...*/)
}
```

### Field access with `->` and `matches`

If all enum fields have distinct names, the arrow `->` accesses a field without a full `match`. For tuple-like variants declared with `()`, `->1` accesses the first tuple field. `match` works as in Rust. Verbatim from `examples/guide/datatypes.rs`:

```rust
enum Life {
    Mammal { legs: int, has_pocket: bool },
    Arthropod { legs: int, wings: int },
    Plant { leaves: int },
}

spec fn is_insect(l: Life) -> bool
{
    l is Arthropod && l->Arthropod_legs == 6
}

enum Shape {
    Circle(int),
    Rect(int, int),
}

spec fn area_2(s: Shape) -> int {
    match s {
        Shape::Circle(radius) => { radius * radius * 3 },
        Shape::Rect(width, height) => { width * height }
    }
}

spec fn rect_height(s: Shape) -> int
    recommends s is Rect
{
    s->1
}
```

The `matches` syntax binds fields while considering only one or two variants, and composes with `&&`, `==>`, and `&&&` (the bound names are in scope for the remainder of the expression after `&&`). Verbatim from `examples/guide/datatypes.rs`:

```rust
use Life::*;
spec fn cuddly(l: Life) -> bool {
    ||| l matches Mammal { legs, .. } && legs == 4
    ||| l matches Arthropod { legs, wings } && legs == 8 && wings == 0
}

spec fn is_kangaroo(l: Life) -> bool {
    &&& l matches Life::Mammal { legs, has_pocket }
    &&& legs == 2
    &&& has_pocket
}
```

### Type invariants

A struct/enum may carry a *type invariant* — a `#[verifier::type_invariant]` spec predicate of type `(X) -> bool` or `(&X) -> bool`. Verus enforces it holds for any exec or tracked-mode ghost object (on construction, after field assignment, after a `&mut` call) but **not** for spec objects. The invariant is not provided automatically; call the builtin `use_type_invariant(&x)` (in a `proof` block if in exec code) to learn it holds. The datatype must be declared in the same crate with no fields public outside the crate.

## Traits

Verus supports `requires`/`ensures` on trait functions. A trait function's spec is a *contract* every implementation must satisfy and callers may rely on through a trait bound.

### Trait function specifications and extending in impls

An `impl` **can** add stronger `ensures` but **cannot** add new `requires` (a caller using `C: Trait` has no obligation to satisfy requirements the trait doesn't mention). Verbatim from `examples/guide/traits.rs`:

```rust
trait Compressor {
    fn compress(&self, input: u64) -> (output: u64)
        ensures output <= input;
}

struct HalfCompressor;

impl Compressor for HalfCompressor {
    // The trait's `ensures output <= input` is automatically inherited.
    // We additionally specify the exact return value.
    fn compress(&self, input: u64) -> (output: u64)
        ensures output == input / 2,
    {
        input / 2
    }
}
```

### Generic vs. concrete dispatch

When Verus can statically determine the concrete type, it uses the (possibly stronger) `impl` spec; through a generic bound `T: Trait`, only the trait-level spec is known. Verbatim from `examples/guide/traits.rs`:

```rust
fn compress_generic<C: Compressor>(c: &C, x: u64) {
    let r = c.compress(x);
    assert(r <= x); // OK: trait-level ensures holds for every C
}

fn compress_concrete(c: &HalfCompressor, x: u64) {
    let r = c.compress(x);
    assert(r <= x);      // From the trait ensures
    assert(r == x / 2);  // From HalfCompressor's stronger ensures (statically resolved)
}
```

### Spec and proof functions in traits

A trait may declare `spec` and `proof` functions. Verbatim from `examples/guide/traits.rs` (a `Distance` trait requiring a spec `dist`, an exec `distance` proving it, and a `proof fn valid_distance_metric` establishing metric axioms):

```rust
trait Distance {
    spec fn dist(&self, other: &Self) -> nat;

    fn distance(&self, other: &Self) -> (d: u64)
        ensures
            d as nat == self.dist(other),
    ;

    proof fn valid_distance_metric()
        ensures
            forall |x: &Self, y| x.dist(y) == y.dist(x),
            forall |x: &Self, y| x.dist(y) == 0 <==> x == y,
            forall |x: &Self, y, z| x.dist(y) <= x.dist(z) + z.dist(y),
        ;
}
```

### The `View` trait and `@`

The most commonly used `vstd` trait. It gives an exec type a mathematical abstraction `type V` via `spec fn view`, with `x@` as sugar for `x.view()`. `vstd` provides `View` for `Vec<T>`→`Seq<T>`, `HashMap<K,V>`→`Map<K,V>`, `HashSet<K>`→`Set<K>`, and primitives→themselves. Implementing `View` for a custom type (verbatim from `examples/guide/traits.rs`):

```rust
struct Stack {
    data: Vec<u64>,
}

impl View for Stack {
    type V = Seq<u64>;

    closed spec fn view(&self) -> Seq<u64> {
        self.data@
    }
}

impl Stack {
    fn push(&mut self, val: u64)
        ensures final(self)@ == old(self)@.push(val),
    {
        self.data.push(val);
    }

    fn is_empty(&self) -> (result: bool)
        ensures result <==> self@.len() == 0,
    {
        self.data.len() == 0
    }
}
```

Use `closed spec fn view` when the underlying field is private (callers reason about effects, not the definition); use `open spec fn view` when callers should unfold `@` to its definition. `DeepView` recursively applies the view to nested elements. See [specifications.md](./specifications.md) and [vstd-library.md](./vstd-library.md).

### `default_ensures`

For trait functions with a default body, `default_ensures` gives an additional guarantee that holds *only* when a type uses the default implementation without overriding. Callers that statically know the type inherit both `ensures` and `default_ensures`; callers using a generic bound learn only `ensures`.

## Iterators

Verus verifies `for` loops over any type implementing Rust's `Iterator` trait, provided the type has Verus specifications (via `vstd::std_specs::iter`). You may name the iterator in a `for` loop to refer to ghost state: `iter.index()` (iterations so far) and `iter.seq()` (the prophetic complete sequence of items the iterator will yield). In a `for` without `break`, after the loop `iter.index() == iter.seq().len()`. Verbatim from `examples/guide/iterators.rs`:

```rust
fn all_positive(v: &Vec<u8>) -> (b: bool)
    ensures
        b <==> (forall|i: int| 0 <= i < v.len() ==> v[i] > 0),
{
    let mut b: bool = true;

    for x in iter: vec_iter(v)
        invariant
            b <==> (forall|i: int| 0 <= i < iter.index() ==> v[i] > 0),
    {
        b = b && *x > 0;
    }
    b
}
```

Range iterators work the same way (verbatim from `examples/guide/iterators.rs`):

```rust
fn build_range(n: u32) -> (v: Vec<u32>)
    ensures
        v.len() == n,
        forall|i: int| 0 <= i < n ==> v[i] == i,
{
    let mut v: Vec<u32> = Vec::new();
    for i in r_iter: 0..n
        invariant
            v.len() == r_iter.index(),
            forall|j: int| 0 <= j < v.len() ==> v[j] == r_iter.seq()[j],
    {
        v.push(i);
    }
    v
}
```

If you don't need ghost state, omit the binding: `for x in 0..10`. For `DoubleEndedIterator` types, `.rev()` works and `seq()`/`index()` refer to the reversed sequence.

### Specifying a custom iterator

To iterate over your own type: (1) define the iterator struct (with a `#[verifier::type_invariant]`); (2) implement Rust's `Iterator::next`; (3) implement `vstd::std_specs::iter::IteratorSpecImpl` providing `obeys_prophetic_iter_laws`, `remaining` (the prophetic sequence of remaining items), `will_return_none`, `decrease`, `initial_value_relation`, and `peek`; (4) write a constructor annotated `#[verifier::when_used_as_spec(spec_fn)]` so `for`-loop invariants can relate the iterator to the collection it came from. For backward iteration, implement `DoubleEndedIterator` and `DoubleEndedIteratorSpecImpl` (`peek_back`).

> **Limitation:** Verus currently only reasons about iterators that eventually return `None` and then continue returning `None`.

## Higher-order exec functions and closures

### Functions as values: `call_requires` / `call_ensures`

Verus reasons about the pre/postconditions of function values via two builtins: `call_requires(f, args)` (the precondition; `args` is a tuple) and `call_ensures(f, args, output)` (the postcondition). `vstd` aliases these as `f.requires(args)` and `f.ensures(args, output)` via the `FnWithRequiresEnsures` trait. Verbatim from `examples/guide/higher_order_fns.rs`:

```rust
fn double(x: u8) -> (res: u8)
    requires
        0 <= x < 128,
    ensures
        res == 2 * x,
{
    2 * x
}

fn higher_order_fn(f: impl Fn(u8) -> u8) -> (res: u8)
    requires
        call_requires(f, (50,)),
        forall|x, y| call_ensures(f, x, y) ==> y % 2 == 0,
    ensures
        res % 2 == 0,
{
    let ret = f(50);
    return ret;
}
```

> **Pitfall:** `call_ensures(f, args, expected)` only says `expected` is *a possible* return value (functions may be nondeterministic). To pin down a unique result, write `requires forall |ret| call_ensures(f, args, ret) ==> ret == expected_return_value`.

### Closures

Closures may have `requires`/`ensures` like any function. Verbatim from `examples/guide/higher_order_fns.rs`:

```rust
fn test_vec_map_with_closure() {
    let double = |x: u8| -> (res: u8)
        requires 0 <= x < 128
        ensures res == 2 * x
    {
        2 * x
    };

    assert(forall |x| 0 <= x < 128 ==> call_requires(double, (x,)));
    assert(forall |x, y| call_ensures(double, (x,), y) ==> y == 2 * x);

    let mut v = Vec::new();
    v.push(0);
    v.push(10);
    v.push(20);
    let w = vec_map(&v, double);
    assert(w[2] == 40);
}
```

**Capturing limitation:** Verus supports move-captures and immutable-reference captures easily, but does **not** yet support borrowing mutably from the context. It therefore has better support for `Fn` and `FnOnce` than for `FnMut` (no mutable captures).

## Strings

Verus reasons about Rust `String` and `&str`. A string literal is a `&str` whose view is a `Seq<char>`; the view's value is unknown until the literal is *revealed* with `reveal_strlit`. Equality comparison of literals does not require revealing. Verbatim from `examples/guide/strings.rs`:

```rust
fn get_char() {
    let x = "hello world";
    proof {
        reveal_strlit("hello world");
    }
    assert(x@.len() == 11);
    let val = x.get_char(0);
    assert('h' == val);
}

fn literal_eq() {
    let x = "hello world";
    let y = "hello world";
    assert(x@ == y@);
}

fn str_view() {
    let x = "hello world";
    let ghost y: Seq<char> = x@;
}
```

String operations currently require the functions in `vstd::string`. The `is_ascii` predicate enables efficient ASCII slicing (e.g. `as_str().substring_ascii(2, 3)`).

## Ownership complexity: interior mutability and pointers

### Interior mutability

Verus's SMT encoding assumes `&T` never changes, so the SMT "value" of a `Cell`-like type is just a unique identifier (not its contents). `===` (pure equality) on cells compares identity, not contents. `vstd` provides verified alternatives (see [vstd-library.md](./vstd-library.md)):

- **`InvCell<T, Pred>`** — invariant-gated cell; the client supplies a `Predicate<T>` constraining allowed values. Reads return *some* value satisfying the invariant; writes must satisfy it. Best when correctness doesn't depend on predicting the exact value (e.g. memoization).
- **`PCell<T>` + `PointsTo<T>`** — the primitive permissioned cell; access is gated by a tracked ghost `PointsTo` token that tracks the exact contents. Best when you need to track the precise value.
- **Atomics** — `vstd::atomic_ghost::*` (`AtomicU64`, `AtomicBool`, …) and `AtomicInvariant` for concurrent interior mutability.
- **`GhostCell`** — see the vstd docs.

Rust's `Cell`/`RefCell` are replaced by these vstd alternatives; `UnsafeCell` is replaced by `PCell`.

### Pointers

- **`PPtr<T>`** (`vstd::simple_pptr`) — a pointer to a fixed-size heap allocation, paired with a `PointsTo<T>` permission token (like `PCell` but with a fixed address).
- **`*mut T` / `*const T`** — partially supported via `vstd::raw_ptr`.
- **`transmute`** — not supported. **`Pin`** — not supported. **Unions** — supported (field access is checked well-formed; spec uses `is_variant(u, "field")` and `get_union_field`). **`UnsafeCell`** — use the `PCell` vstd alternative.

## Interacting with unverified code

### `external_body` and `assume_specification`

`#[verifier::external_body]` marks a function whose body Verus should not inspect — only its `requires`/`ensures` are trusted. `pub assume_specification[ StdFn ](...) -> ...` gives an external (std or third-party) function a Verus contract; this is **unchecked** (trusted) and is how `vstd::std_specs` specifies Rust's std.

### External trait specifications

For traits defined in external crates (including `std`), Verus provides two attributes:

- `#[verifier::external_trait_specification]` — adds `requires`/`ensures` to a trait's methods. The spec trait must contain an associated type `ExternalTraitSpecificationFor: ExternalTrait` naming the external trait; method signatures must match; the spec trait need not include all members (omitted members are inaccessible to verified code).
- `#[verifier::external_trait_extension(SpecTrait via SpecImplTrait)]` — additionally defines spec helper functions on the trait; `SpecTrait` becomes usable in bounds and `SpecImplTrait` in `impl` blocks.

> **Soundness warning:** all implementations of the trait — including those in unverified code, even code not yet written — are assumed to uphold the specification. This is a contract on current and future unverified code.

### The `obeys_*` pattern

`vstd` mitigates the above risk for std traits (`PartialEq`, `Ord`, `Add`, `From`, …) with an `obeys_*` spec guard. The `ensures` is conditional: `Self::obeys_eq_spec() ==> r == self.eq_spec(other)`. For integer/`bool` types `vstd` proves `obeys_eq_spec()` is true; for other types Verus doesn't assume it. To opt an external type in, use an `assume_specification` asserting `obeys_eq_spec()` is true.

### exec↔spec bridges

- **`exec_spec_verified!` / `exec_spec_unverified!`** — compile spec-mode functions/structs/enums to exec counterparts, with (`verified`) or without (`unverified`) proven equivalence. `verified` generates `Exec*` types implementing `DeepView<V = SpecType>` and `exec_*` functions with `ensures res == spec_fn(...)`. `unverified` marks generated exec code `#[verifier::external_body]` (equivalence not proven). Supported spec fragment: basic arithmetic, logical ops, if/match/matches, field access, recursion, bounded quantifiers, `Seq`/`Map`/`Set`/`Multiset`/`Option`/`SpecString`, user-defined structs/enums, primitive integers/`bool`/`char` (no `int`/`nat`).
- **`#[verus_verify(dual_spec)]`** — go the other direction: generate a spec function from an exec function (removes ghost vars/proof blocks, applies `#[verifier::when_used_as_spec]`). Requires `#[verus_spec(...)]`; does not support mutable inputs.
- **`#[verus_spec(...)]` attribute macro** — add specs/proofs to exec code in native Rust syntax (for incremental adoption into existing projects), keeping the exec code readable and proof-erased for non-Verus builds. `#[verus_spec(with ...)]` + `proof_with!` passes tracked/ghost variables to callees (generates a verified and an unverified version of the function). `proof!` / `proof_decl!` introduce proof blocks (the latter allows function-scoped ghost/tracked variables across blocks).

## Supported and unsupported Rust features

The authoritative list is `source/docs/guide/src/features.md` ("Last Updated: 2026-05-13"). Summary of the key items:

**Supported (directly):** functions/methods/associated functions; structs; enums; macros; type aliases; type parameters; where clauses; lifetime parameters; custom discriminants; variables/assignment/`mut`; if/else; patterns/match/if-let/match guards; block expressions; `loop`/`while`; `?`; unsafe blocks; shared `&` and mutable `&mut` borrows; place expressions; `==`/`!=`; compound assignments (`+=` etc.); range expressions; tuple expressions; struct/enum constructors; field access; function/method calls; closures; labels/break/continue; return; implicit coercions/derefs/borrows; unsigned and signed integer arithmetic; bitwise ops; `usize`/`isize`; integer types; `bool`; strings; `Vec`; `Option`/`Result`; slices; arrays; shared/mutable references; never type; user-defined traits; default trait impls; trait bounds; traits with type args; associated types; higher-ranked trait bounds; marker traits (`Copy`/`Send`/`Sync`); `Sized`/`size_of`/`align_of`; `Deref`/`DerefMut`; modules; rustdoc; spawn and join; interior mutability; verified lock implementations; unions.

**Partially supported:** associated constants; const functions; const items; static items; nested items; `for` loops; type cast (`as`); array expressions (no fill expressions with `const` args); index expressions; public/private fields; const generics; floating point; pointers; trait objects (`dyn`); `impl` types; closure types (no mutable captures); iterators; `PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Clone`/`Default`/`From`/`TryFrom`/`Into`/arithmetic-operator/`Bit*`/`Shl`/`Shr` traits; generic associated types (only lifetimes); panic-unwinding; multi-crate projects; verified + unverified crates; raw pointers.

**Not supported:** async functions; async blocks; `await`; destructuring assignment; function pointer types; `Debug`/`serde::Serialize` traits; user-defined destructors (`Drop`); `Mutex`/`RwLock` from std (use verified lock implementations); `Pin`; hardware intrinsics; printing/I/O; `transmute`. `Cell`/`RefCell`/`UnsafeCell` are replaced by vstd alternatives.

## Runnable example: traits and references

The following is copied verbatim from `.tmp/verus/examples/guide/traits.rs` (the `view_impl` anchor) and `.tmp/verus/examples/guide/references.rs` (the `requires` anchor), illustrating the `View` trait, `@`, `old`/`final`, and `&mut` specifications together.

```rust
use vstd::prelude::*;

verus! {

// From examples/guide/references.rs
fn increment(a: &mut u32)
    requires *old(a) < u32::MAX,
    ensures *final(a) == *old(a) + 1,
{
    *a = *a + 1;
}

fn caller()
{
    let mut z: u32 = 0;
    increment(&mut z);
    assert(z == 1);
}

// From examples/guide/traits.rs
struct Stack {
    data: Vec<u64>,
}

impl View for Stack {
    type V = Seq<u64>;

    closed spec fn view(&self) -> Seq<u64> {
        self.data@
    }
}

impl Stack {
    fn push(&mut self, val: u64)
        ensures final(self)@ == old(self)@.push(val),
    {
        self.data.push(val);
    }

    fn is_empty(&self) -> (result: bool)
        ensures result <==> self@.len() == 0,
    {
        self.data.len() == 0
    }
}

} // verus!
```

## Sources used

- `.tmp/verus/source/docs/guide/src/features.md` — authoritative supported/unsupported Rust feature list (435 lines, "Last Updated: 2026-05-13")
- `.tmp/verus/source/docs/guide/src/mutation-references-borrowing.md` — borrowing overview
- `.tmp/verus/source/docs/guide/src/mutable-references.md` — `&mut`, `old`/`final`, returning mutable borrows, `after_borrow`, `has_resolved`
- `.tmp/verus/source/docs/guide/src/assert-mut-ref.md` — assertions about mutable references
- `.tmp/verus/source/docs/guide/src/datatypes_struct.md`, `.tmp/verus/source/docs/guide/src/datatypes_enum.md` — structs and enums
- `.tmp/verus/source/docs/guide/src/traits.md` — trait specs, `View`, `default_ensures`
- `.tmp/verus/source/docs/guide/src/reference-type-invariants.md` — type invariants
- `.tmp/verus/source/docs/guide/src/iterators.md`, `.tmp/verus/source/docs/guide/src/iterator-specs.md` — iterators and `for` loops
- `.tmp/verus/source/docs/guide/src/exec_funs_as_values.md`, `.tmp/verus/source/docs/guide/src/exec_closures.md`, `.tmp/verus/source/docs/guide/src/higher-order-fns.md` — higher-order functions and closures
- `.tmp/verus/source/docs/guide/src/strings.md` — `str`/`String` specs
- `.tmp/verus/source/docs/guide/src/interior_mutability.md`, `.tmp/verus/source/docs/guide/src/reference-pointers-cells.md`, `.tmp/verus/source/docs/guide/src/pointers.md` — interior mutability and pointers
- `.tmp/verus/source/docs/guide/src/reference-unions.md`, `.tmp/verus/source/docs/guide/src/static.md` — unions and static items
- `.tmp/verus/source/docs/guide/src/external_trait_specifications.md` — external trait specs and the `obeys_*` pattern
- `.tmp/verus/source/docs/guide/src/exec_spec.md`, `.tmp/verus/source/docs/guide/src/exec_attr.md`, `.tmp/verus/source/docs/guide/src/exec_to_spec.md` — exec↔spec bridges
- `.tmp/verus/examples/guide/references.rs`, `.tmp/verus/examples/guide/datatypes.rs`, `.tmp/verus/examples/guide/traits.rs`, `.tmp/verus/examples/guide/iterators.rs`, `.tmp/verus/examples/guide/higher_order_fns.rs`, `.tmp/verus/examples/guide/strings.rs`, `.tmp/verus/examples/guide/interior_mutability.rs` — verbatim runnable examples
