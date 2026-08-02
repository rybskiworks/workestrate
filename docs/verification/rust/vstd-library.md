# vstd — The Verus Verified Standard Library

> Source-fidelity topic doc. Every signature quoted below is copied verbatim from `.tmp/verus/source/vstd/*.rs`; every runnable snippet is copied verbatim from `.tmp/verus/examples/guide/*.rs`. This doc is a **map** (module → purpose → key types/lemmas), not a reproduction of the library.

## What vstd is

`vstd` is the verified standard library for [Verus](https://github.com/verus-lang/verus). It provides:

- **Mathematical (spec-only) data types** — `Seq`, `Set`, `ISet`, `Map`, `IMap`, `Multiset` — used to write specifications and proofs independent of any concrete runtime container.
- **Specifications for Rust's `std`** — `std_specs/` gives `requires`/`ensures` contracts for `Vec`, `Option`, `Result`, ranges, iterators, `HashMap`/`BTreeMap`, slices, atomics, etc., so verified `exec` code can use ordinary Rust constructs.
- **An arithmetic lemma library** — `vstd::arithmetic::*` (division/mod, multiplication, powers, logarithms). See [arithmetic-and-provers.md](./arithmetic-and-provers.md).
- **Verus-safe interior-mutability primitives** — `PCell`, `InvCell`, `PointsTo`, atomics — replacing `Cell`/`RefCell`/`UnsafeCell` with verification-friendly analogues.
- **The `View` trait and `@` operator** — the bridge from a concrete `exec` type to its mathematical abstraction (e.g. `Vec<T>` → `Seq<T>`). See [specifications.md](./specifications.md).

Most Verus programs begin with:

```rust
use vstd::prelude::*;
```

`vstd` is published on crates.io. The `Cargo.toml` for the library declares:

```toml
[package]
name = "vstd"
description = "Verus Standard Library: Useful specifications and lemmas for verifying Rust code"
```

To depend on it in a verified crate, add `vstd` (and the Verus toolchain) as a dependency; the `default` feature is `std` (which implies `alloc`). Feature flags include `alloc`, `std`, `allocator`, `allow_panic`, and `strict_provenance_atomic_ptr`.

## The broadcast-group axiom model

`vstd` types like `Seq` and `Set` are declared `#[verifier::external_body]` (their internal representation is hidden from the SMT solver). Their semantics therefore come from **broadcast axiom groups** — sets of `broadcast proof fn`/`broadcast axiom fn` lemmas that the solver instantiates automatically.

The top-level group that ties the library together is defined in `source/vstd/vstd.rs`:

```rust
#[cfg_attr(verus_keep_ghost, verifier::broadcast_use_by_default_when_this_crate_is_imported)]
pub broadcast group group_vstd_default {
    // basic Verus math, types, and features
    seq::group_seq_axioms,
    seq_lib::group_seq_lib_default,
    map::group_map_lemmas,
    set::group_set_lemmas,
    imap::group_imap_lemmas,
    iset::group_iset_lemmas,
    set_lib::group_set_lib_default,
    multiset::group_multiset_axioms,
    compute::all_spec_ensures,
    function::group_function_axioms,
    laws_eq::group_laws_eq,
    laws_cmp::group_laws_cmp,
    // Rust types
    slice::group_slice_axioms,
    array::group_array_axioms,
    #[cfg(not(verus_verify_core))]
    string::group_string_axioms,
    raw_ptr::group_raw_ptr_axioms,
    layout::group_layout_axioms,
    // core std_specs
    std_specs::range::group_range_axioms,
    std_specs::bits::group_bits_axioms,
    std_specs::control_flow::group_control_flow_axioms,
    std_specs::slice::group_slice_axioms,
    std_specs::manually_drop::group_manually_drop_axioms,
    std_specs::iter::group_iter_axioms,
    // std_specs for alloc (with or without std)
    #[cfg(feature = "alloc")]
    std_specs::vec::group_vec_axioms,
    #[cfg(feature = "alloc")]
    std_specs::vecdeque::group_vec_dequeue_axioms,
    // std_specs for alloc + std
    #[cfg(all(feature = "alloc", feature = "std"))]
    std_specs::hash::group_hash_axioms,
    #[cfg(feature = "alloc")]
    std_specs::btree::group_btree_axioms,
}
```

The `broadcast_use_by_default_when_this_crate_is_imported` attribute means: **any crate that depends on `vstd` automatically gets `group_vstd_default` in scope**, so the core axioms about `Seq`/`Set`/`Map`/`Vec`/etc. are available without an explicit `broadcast use`. The per-type groups named below (`group_seq_axioms`, `group_set_lemmas`, …) are the building blocks.

## prelude vs pervasive

- **`vstd::prelude`** (`source/vstd/prelude.rs`) is what users import with `use vstd::prelude::*;`. It re-exports the `verus_builtin` macros (`proof`, `verus`, `is_variant`, …), the math types (`Seq`, `Set`, `ISet`, `Map`, `IMap` and their macros `seq!`/`set!`/`map!`/`iset!`/`imap!`), the `View` trait, and a curated set of additional spec/exec helpers (`ArrayAdditionalSpecFns`, `SliceAdditionalSpecFns`, `OptionAdditionalFns`, `ResultAdditionalSpecFns`, `VecAdditionalSpecFns`, `VecAdditionalExecFns`, string exec fns, and the token types).
- **`vstd::pervasive`** (`source/vstd/pervasive.rs`) is lower-level infrastructure: the `assume`/`assert`/`affirm` proof builtins, the `arbitrary<A>()` spec function (every type is inhabited in spec), `unreached`, `runtime_assert`, the `FnWithRequiresEnsures` trait (the `f.requires(args)`/`f.ensures(args, output)` aliases for higher-order function specs), `cloned`/`strictly_cloned` predicates, `allow_panic`, and the `VecAdditionalExecFns` impl (`set`, `set_and_swap` — Verus replacements for `self[i] = v` indexing-assignment, which Verus does not support directly). Most users never name `pervasive` directly; the prelude surfaces what they need.

## Mathematical data types

These are the workhorse spec types. `Seq`, `Set`, `Map`, and `Multiset` are always finite and may appear in recursive types (`#[verifier::accept_recursive_types]`); `ISet` and `IMap` may be infinite and are spec-only (`#[verifier::reject_recursive_types]`). The finite types' `len()` returns an unbounded `nat`, unlike `std` collections which return a bounded `usize`.

### `Seq<A>` — sequences (`source/vstd/seq.rs`, `seq_lib.rs`)

A spec sequence. Constructed with `Seq::empty`, `Seq::new`, the `seq!` macro, or by transforming an existing sequence. Declared:

```rust
#[verifier::external_body]
#[verifier::ext_equal]
#[verifier::accept_recursive_types(A)]
pub tracked struct Seq<A> {
    dummy: marker::PhantomData<A>,
}
```

Key operations (verbatim signatures):

```rust
pub uninterp spec fn empty() -> Seq<A>;
pub uninterp spec fn new(len: nat, f: impl Fn(int) -> A) -> Seq<A>;
pub uninterp spec fn len(self) -> nat;
pub uninterp spec fn index(self, i: int) -> A
    recommends
        0 <= i < self.len(),
;
pub uninterp spec fn push(self, a: A) -> Seq<A>;
pub uninterp spec fn update(self, i: int, a: A) -> Seq<A>
    recommends
        0 <= i < self.len(),
;
pub uninterp spec fn subrange(self, start_inclusive: int, end_exclusive: int) -> Seq<A>
    recommends
        0 <= start_inclusive <= end_exclusive <= self.len(),
;
pub uninterp spec fn add(self, rhs: Seq<A>) -> Seq<A>;
```

`take`, `skip`, `first`, `last` are defined inline on top of `subrange`/`index`. The `[]` operator desugars to `index` via the inline `spec_index`. Concatenation is `add` (also spelled `+`).

The axioms live in `pub broadcast group group_seq_axioms` (in `seq.rs`): `axiom_seq_empty`, `axiom_seq_new_len`, `axiom_seq_new_index`, `axiom_seq_push_len`, `axiom_seq_push_index_same`, `axiom_seq_push_index_different`, `axiom_seq_update_len`, `axiom_seq_update_same`, `axiom_seq_update_different`, `axiom_seq_ext_equal` (extensional equality `=~=``), `axiom_seq_subrange_len`, `axiom_seq_subrange_index`, `axiom_seq_add_len`, `axiom_seq_add_index1`, `axiom_seq_add_index2`, etc. A separate `group_seq_lemmas_expensive` holds costlier variants that are **not** in the default group (to avoid slowing verification).

### `Set<A>` — finite sets (`source/vstd/set.rs`, `set_lib.rs`)

A finite set. Because it is finite it can be used in recursive types. Declared `#[verifier::external_body]`/`#[verifier::ext_equal]`/`#[verifier::accept_recursive_types(A)]`. Key operations (verbatim):

```rust
pub open spec fn contains(self, a: A) -> bool { ... }
pub closed spec fn insert(self, a: A) -> Set<A> { ... }
pub closed spec fn remove(self, a: A) -> Set<A> { ... }
pub closed spec fn union(self, s2: Set<A>) -> Set<A> { ... }
pub closed spec fn intersect(self, s2: Set<A>) -> Set<A> { ... }
pub closed spec fn difference(self, s2: Set<A>) -> Set<A> { ... }
pub closed spec fn len(self) -> nat { ... }
pub open spec fn choose(self) -> A { ... }
```

`Set::new(f)` returns `Option<Set<A>>` — `Some` only when the predicate describes a finite set. `Set::full()` likewise returns `Option` (infinite for infinite `A`). `Set::range(lo, hi)` (via the `FiniteRange` trait) and `Set::<T>::full().unwrap()` (via `FiniteFull`) are the common finite constructors for numeric types. Lemmas live in `pub broadcast group group_set_lemmas` (`lemma_set_empty`, `lemma_set_insert_same`/`_different`, `lemma_set_remove_same`/`_different`/`_insert`, `lemma_set_union`, `lemma_set_intersect`, `lemma_set_difference`, `lemma_set_complement`, `lemma_set_filter`, `lemma_set_empty_len`, `lemma_set_insert_len`, `lemma_set_remove_len`, `lemma_set_contains_len`, `lemma_set_choose_len`, `axiom_set_ext_equal`, …). `set_lib.rs` adds `group_set_lib_default`.

### `ISet<A>` — possibly-infinite sets (`source/vstd/iset.rs`, `iset_lib.rs`)

A set that may be infinite. Its representation is a predicate: `pub struct ISet<A> { set: spec_fn(A) -> bool }` (marked `#[verifier::reject_recursive_types(A)]`, so it cannot appear in recursive types). Key operations (verbatim):

```rust
pub closed spec fn new(f: spec_fn(A) -> bool) -> ISet<A> { ... }
pub open spec fn full() -> ISet<A> { ... }
pub closed spec fn contains(self, a: A) -> bool { ... }
pub closed spec fn finite(self) -> bool { ... }
```

`finite` is defined via the existence of an injective mapping into a bounded `nat` range. Lemmas: `pub broadcast group group_iset_lemmas` (`lemma_iset_empty`, `lemma_iset_new`, `lemma_iset_insert_same`/`_different`, `lemma_iset_remove_*`, `lemma_iset_union`/`_intersect`/`_difference`/`_complement`, `lemma_iset_ext_equal`, `lemma_iset_empty_finite`, `lemma_iset_insert_finite`, `lemma_iset_remove_finite`, `lemma_iset_union_finite`, `lemma_iset_empty_len`, `lemma_iset_insert_len`, …). The `iset::fold` submodule (ported from Isabelle/HOL `Finite_Set`) defines `ISet::fold` and proves `lemma_fold_insert`, `lemma_fold_empty`, and `lemma_finite_set_induct`.

### `Map<K, V>` — finite maps (`source/vstd/map.rs`, `map_lib.rs`)

A finite map. Declared `#[verifier::external_body]`/`#[verifier::ext_equal]`/`accept_recursive_types(K)`/`accept_recursive_types(V)`. Key operations (verbatim):

```rust
pub uninterp spec fn dom(self) -> Set<K>;
pub uninterp spec fn index(self, key: K) -> V
    recommends
        self.dom().contains(key),
;
pub uninterp spec fn new(s: Set<K>, fv: spec_fn(K) -> V) -> Map<K, V>;
pub closed spec fn insert(self, key: K, value: V) -> Map<K, V> { ... }
pub closed spec fn remove(self, key: K) -> Map<K, V> { ... }
```

`Map::new` takes a `Set<K>` domain and a value function (a map comprehension). The `map!` macro builds small maps: `map![k1 => v1, k2 => v2]`. Lemmas: `pub broadcast group group_map_lemmas` (`axiom_map_index_decreases`, `lemma_map_new_domain`, `lemma_map_new_index`, `lemma_map_empty`, `lemma_map_insert_domain`/`_same`, `axiom_map_insert_different`, `lemma_map_remove_domain`, `axiom_map_remove_different`, `axiom_map_ext_equal`, `axiom_map_ext_equal_deep`). The `assert_maps_equal!` macro proves map equality by extensionality per-key.

### `IMap<K, V>` — possibly-infinite maps (`source/vstd/imap.rs`, `imap_lib.rs`)

A map that may be infinite. Representation: `pub tracked struct IMap<K, V> { mapping: spec_fn(K) -> Option<V> }` (`reject_recursive_types(K)`, `accept_recursive_types(V)`). Key operations (verbatim):

```rust
pub open spec fn total(fv: spec_fn(K) -> V) -> IMap<K, V> { ... }
pub open spec fn new(fk: spec_fn(K) -> bool, fv: spec_fn(K) -> V) -> IMap<K, V> { ... }
pub closed spec fn dom(self) -> ISet<K> { ... }
pub closed spec fn index(self, key: K) -> V
    recommends
        self.dom().contains(key),
{ ... }
```

`IMap::new(fk, fv)` builds a map whose domain is `{k | fk(k)}` and whose values are `fv(k)`; `IMap::total(fv)` builds a total map over all of `K`. Lemmas: `pub broadcast group group_imap_lemmas` (`axiom_imap_index_decreases_finite`/`_infinite`, `lemma_imap_empty`, `lemma_imap_insert_domain`/`_same`/`_different`, `lemma_imap_remove_domain`/`_different`, `lemma_imap_ext_equal`, `lemma_imap_ext_equal_deep`). The `assert_imaps_equal!` macro is the infinite-map analogue of `assert_maps_equal!`.

### `Multiset<A>` — bags (`source/vstd/multiset.rs`, `multiset_lib.rs`)

A finite multiset (bag) — a total map from elements to multiplicities with finitely many non-zero entries. Declared `#[verifier::external_body]`/`#[verifier::ext_equal]`/`#[verifier::accept_recursive_types(V)]`. Key operations (verbatim):

```rust
pub uninterp spec fn count(self, value: V) -> nat;
pub uninterp spec fn len(self) -> nat;
pub uninterp spec fn empty() -> Self;
pub uninterp spec fn singleton(v: V) -> Self;
pub uninterp spec fn add(self, m2: Self) -> Self;
pub uninterp spec fn sub(self, m2: Self) -> Self;
```

`insert`/`remove` are inline sugar over `add`/`sub` with `singleton`. `intersection_with`/`difference_with` use `min`/`clip` over counts. Lemmas: `pub broadcast group group_multiset_axioms` (`axiom_multiset_empty`, `axiom_multiset_contained`, `axiom_multiset_singleton`/`_different`, `axiom_multiset_add`, `axiom_multiset_sub`, `axiom_multiset_ext_equal`, `axiom_len_empty`/`_singleton`/`_add`/`_sub`, `axiom_count_le_len`, `axiom_filter_count`, `axiom_choose_count`) plus `group_multiset_properties` (verified lemmas `lemma_update_same`/`_different`, `lemma_insert_*`, `lemma_intersection_count`, `lemma_difference_count`, …). The `assert_multisets_equal!` macro proves multiset equality per-element.

## std_specs — specifications for Rust's std

`vstd::std_specs` (`source/vstd/std_specs/`, gated `#[cfg(verus_keep_ghost)]`) provides `requires`/`ensures` contracts for ordinary Rust std types so verified `exec` code can use them. The submodules are: `alloc`, `atomic`, `bits`, `borrow`, `btree`, `clone`, `cmp`, `control_flow`, `convert`, `core`, `default`, `hash`, `iter`, `manually_drop`, `maybe_uninit`, `num`, `ops`, `option`, `range`, `result`, `slice`, `vec`, `vecdeque`, `smart_ptrs`.

The pattern is `#[verifier::external_type_specification]` (to wrap a std type) plus `pub assume_specification[ ... ]` (to give a std function a contract). For example, `std_specs/vec.rs` wraps `Vec` as `ExVec` and specifies its methods. A few verbatim entries:

```rust
pub assume_specification<T>[ Vec::<T>::new ]() -> (v: Vec<T>)
    ensures
        v@ == Seq::<T>::empty(),
;

pub assume_specification<T, A: Allocator>[ Vec::<T, A>::push ](vec: &mut Vec<T, A>, value: T)
    ensures
        final(vec)@ == old(vec)@.push(value),
;

pub assume_specification<T, A: Allocator>[ Vec::<T, A>::pop ](vec: &mut Vec<T, A>) -> (value:
    Option<T>)
    ensures
        old(vec)@.len() > 0 ==> value == Some(old(vec)@[old(vec)@.len() - 1])
            && final(vec)@ == old(vec)@.subrange(0, old(vec)@.len() - 1),
        old(vec)@.len() == 0 ==> value == None::<T> && final(vec)@ == old(vec)@,
;

pub assume_specification<T, A: Allocator>[ Vec::<T, A>::insert ](
    vec: &mut Vec<T, A>,
    i: usize,
    element: T,
)
    requires
        i <= old(vec).len(),
    ensures
        final(vec)@ == old(vec)@.insert(i as int, element),
;
```

Note the `@` view: `Vec`'s `View::V` is `Seq<T>`, so `vec@` is the mathematical sequence of its elements (see [specifications.md](./specifications.md)). The `VecAdditionalSpecFns` trait adds `spec_index`; `VecAdditionalExecFns` (in `pervasive.rs`) provides `set`/`set_and_swap` as verified replacements for indexing-assignment (`self[i] = v`), which Verus does not support directly. `group_vec_axioms` is the broadcast group (`axiom_spec_len`, `axiom_vec_index_decreases`, `vec_clone_deep_view_proof`, `axiom_spec_into_iter`, …).

> **Soundness note:** `assume_specification` is unchecked — it is trusted. `vstd`'s std specs are part of the trusted computing base; treat them as axioms about std behavior.

## arithmetic/ — lemma library

`vstd::arithmetic` (`source/vstd/arithmetic/`) is a port of the Dafny standard arithmetic library. Verus turns nonlinear arithmetic reasoning **off** by default (to keep SMT proofs stable); this library supplies the lemmas that justify nonlinear facts. Submodules: `div_mod`, `mul`, `power`, `power2`, `logarithm`, `overflow` (plus `internals/`). See [arithmetic-and-provers.md](./arithmetic-and-provers.md) for full coverage.

A representative spec function and lemma (verbatim from `div_mod.rs`):

```rust
pub open spec fn rust_div(a: int, b: int) -> int
    recommends
        b != 0,
{ ... }

pub open spec fn rust_rem(a: int, b: int) -> int
    recommends
        b != 0,
{ ... }

pub broadcast proof fn lemma_div_is_div_recursive(x: int, d: int)
    requires
        0 < d,
    ensures
        div_recursive(x, d) == #[trigger] (x / d),
{ ... }
```

## cell — Verus-safe interior mutability

Rust's `Cell`/`RefCell`/`UnsafeCell` are not directly verifiable because Verus assumes `&T` never changes. `vstd::cell` (`source/vstd/cell.rs` plus submodules `pcell`, `pcell_maybe_uninit`, `invcell`) provides verified analogues.

### `PCell<T>` — the primitive permissioned cell (`cell/pcell.rs`)

A wrapper around `ManuallyDrop<UnsafeCell<T>>` with no runtime checks (unlike `RefCell`). Access is gated by a **ghost permission token** `PointsTo<T>`. `PCell` is always `Send`+`Sync`; the marker traits are enforced on `PointsTo` instead. Declared (verbatim):

```rust
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
pub struct PCell<T: ?Sized> {
    ucell: ManuallyDrop<UnsafeCell<T>>,
}

#[verifier::external_body]
#[verifier::reject_recursive_types_in_ground_variants(T)]
pub tracked struct PointsTo<T: ?Sized> {
    phantom: PhantomData<T>,
    no_copy: NoCopy,
}
```

Construction returns the cell and its permission token together (verbatim):

```rust
pub const fn new(v: T) -> (pt: (PCell<T>, Tracked<PointsTo<T>>))
    where T: Sized
    ensures
        pt.1@.id() == pt.0.id() && pt.1@.value() == v
    opens_invariants none
    no_unwind
{ ... }
```

Access methods (`borrow`, `borrow_mut`, `into_inner`, `replace`, `write`, `read`) each take a `Tracked<&PointsTo<T>>` / `Tracked<&mut PointsTo<T>>` argument proving the caller holds the permission. `PCell` does **not** run `T`'s destructor on drop — call `into_inner` with the `PointsTo` to avoid leaking. (There is also a deprecated top-level `vstd::cell::PCell`/`PointsTo`/`InvCell` in `cell.rs`; prefer the submodule versions.)

### `InvCell<T, Pred>` — invariant-gated cell (`cell/invcell.rs`)

Verus's closest analogue of `std::cell::Cell`. It allows reading/writing through a shared `&` by constraining stored values to a predicate. Declared (verbatim):

```rust
#[verifier::accept_recursive_types(T)]
pub struct InvCell<T, Pred> {
    pcell: PCell<T>,
    perm_inv: Tracked<LocalInvariant<(Pred, CellId), PointsTo<T>, InvCellPred>>,
}
```

Construction takes the initial value and a ghost predicate (verbatim):

```rust
pub fn new(val: T, Ghost(pred): Ghost<Pred>) -> (cell: Self)
    requires
        pred.predicate(val),
    ensures
        cell.predicate() == pred,
{ ... }
```

`get`/`set`/`replace` work through a `LocalInvariant` opened with `open_local_invariant!`. The predicate implements the `vstd::predicate::Predicate` trait. This is the recommended approach when correctness does not depend on predicting *which* value the cell holds, only that it satisfies an invariant (e.g. memoization).

## The `@` view operator and the `View` trait

The bridge between a concrete `exec` type and its mathematical abstraction is the `View` trait (`source/vstd/view.rs`, verbatim):

```rust
pub trait View {
    type V;

    spec fn view(&self) -> Self::V;
}

pub trait DeepView {
    type V;

    spec fn deep_view(&self) -> Self::V;
}
```

`x@` is sugar for `x.view()`. `vstd` provides `View` impls: `Vec<T>` → `Seq<T>`, `HashMap<K,V>` → `Map<K,V>`, `HashSet<K>` → `Set<K>`, `Option<T>` → `Option<T>`, and identity views for primitives (`bool`, `u8`…`u128`, `i8`…`i128`, `usize`, `isize`, `char`, `()`) and tuples up to arity 12. `DeepView` recursively applies the view to nested elements. See [specifications.md](./specifications.md) for how to implement `View` for a custom type and use `@` in `requires`/`ensures`.

## Runnable example: Seq, Set, and Map

The following is copied verbatim from `.tmp/verus/examples/guide/lib_examples.rs` (the `macro` and `new` anchors). It shows constructing and indexing the math types with the `seq!`/`set!`/`map!` macros and `Seq::new`/`Set::range`/`ISet::new`/`Map::new`/`IMap::new`.

```rust
use vstd::{map::*, prelude::*, seq::*, set::*};

verus! {

proof fn test_seq1() {
    let s: Seq<int> = seq![0, 10, 20, 30, 40];
    assert(s.len() == 5);
    assert(s[2] == 20);
    assert(s[3] == 30);
}

proof fn test_set1() {
    let s: Set<int> = set![0, 10, 20, 30, 40];
    assert(s.contains(20));
    assert(s.contains(30));
    assert(!s.contains(60));

    let s: ISet<int> = iset![0, 10, 20, 30, 40];
    assert(s.finite());
    assert(s.contains(20));
    assert(s.contains(30));
    assert(!s.contains(60));
}

proof fn test_map1() {
    let m: Map<int, int> = map![0 => 0, 10 => 100, 20 => 200, 30 => 300, 40 => 400];
    assert(m.dom().contains(20));
    assert(m.dom().contains(30));
    assert(!m.dom().contains(60));
    assert(m[20] == 200);
    assert(m[30] == 300);
}

proof fn test_seq2() {
    let s: Seq<int> = Seq::new(5, |i: int| 10 * i);
    assert(s.len() == 5);
    assert(s[2] == 20);
    assert(s[3] == 30);
}

proof fn test_set2() {
    let s_finite: Set<int> = Set::range(0, 41).filter(|i: int| i % 10 == 0);
    assert(s_finite.contains(20));
    assert(s_finite.contains(30));
    assert(!s_finite.contains(60));

    let s: ISet<int> = ISet::new(|i: int| 0 <= i <= 40 && i % 10 == 0);
    assert(s.contains(20));
    assert(s.contains(30));
    assert(!s.contains(60));

    let s_infinite: ISet<int> = ISet::new(|i: int| i % 10 == 0);
    assert(s_infinite.contains(20));
    assert(s_infinite.contains(30));
    assert(!s_infinite.contains(35));
}

proof fn test_map2() {
    let m: IMap<int, int> = IMap::new(|i: int| 0 <= i <= 40 && i % 10 == 0, |i: int| 10 * i);
    assert(m[20] == 200);
    assert(m[30] == 300);

    let m_infinite: IMap<int, int> = IMap::new(|i: int| i % 10 == 0, |i: int| 10 * i);
    assert(m_infinite[20] == 200);
    assert(m_infinite[30] == 300);
    assert(m_infinite[90] == 900);

    let m_finite: Map<int, int> = Map::new(Set::range(0, 41), |i: int| 10 * i);
    assert(m_finite[20] == 200);
    assert(m_finite[30] == 300);
}

} // verus!
```

### Extensional equality

Two collections with the same elements are equal, but the SMT solver will not automatically recognize this if they were constructed differently. Use the extensional equality operator `=~=`` (deep: `=~~=``) to force an element-wise check. By default Verus auto-promotes `==` to `=~=`` inside `assert`/`ensures`/`invariant`. For maps, `assert_maps_equal!` / `assert_imaps_equal!` / `assert_multisets_equal!` prove equality by per-key/per-element reasoning.

## How to use vstd in a project

1. Add `vstd` (and the Verus toolchain) as a dependency. The crate is on crates.io; the `default` feature is `std` (implies `alloc`).
2. Start every verified module with `use vstd::prelude::*;` — this brings in the math types, the `View` trait, the `verus!`/`proof`/`is_variant` macros, and (via `group_vstd_default`) the core axioms.
3. Write `spec`/`proof` functions against the mathematical types (`Seq`, `Set`, `Map`, `int`, `nat`) and connect them to `exec` code with the `@` view operator and the `std_specs` contracts.
4. For interior mutability, use `PCell`/`InvCell` (and `vstd::atomic_ghost::*` / `AtomicInvariant` for concurrency) instead of `Cell`/`RefCell`.

## Sources used

- `.tmp/verus/source/vstd/vstd.rs` — module root, `group_vstd_default` broadcast group
- `.tmp/verus/source/vstd/prelude.rs` — prelude re-exports
- `.tmp/verus/source/vstd/pervasive.rs` — pervasive builtins, `FnWithRequiresEnsures`, `VecAdditionalExecFns`
- `.tmp/verus/source/vstd/view.rs` — `View` / `DeepView` traits and impls
- `.tmp/verus/source/vstd/seq.rs` — `Seq<A>`, `group_seq_axioms`
- `.tmp/verus/source/vstd/set.rs` — `Set<A>`, `group_set_lemmas`
- `.tmp/verus/source/vstd/iset.rs` — `ISet<A>`, `group_iset_lemmas`, `fold` submodule
- `.tmp/verus/source/vstd/map.rs` — `Map<K,V>`, `group_map_lemmas`, `assert_maps_equal!`
- `.tmp/verus/source/vstd/imap.rs` — `IMap<K,V>`, `group_imap_lemmas`, `assert_imaps_equal!`
- `.tmp/verus/source/vstd/multiset.rs` — `Multiset<V>`, `group_multiset_axioms`, `assert_multisets_equal!`
- `.tmp/verus/source/vstd/std_specs/mod.rs` and `.tmp/verus/source/vstd/std_specs/vec.rs` — std specifications
- `.tmp/verus/source/vstd/arithmetic/mod.rs`, `.tmp/verus/source/vstd/arithmetic/README.md`, `.tmp/verus/source/vstd/arithmetic/div_mod.rs` — arithmetic lemma library
- `.tmp/verus/source/vstd/cell.rs`, `.tmp/verus/source/vstd/cell/pcell.rs`, `.tmp/verus/source/vstd/cell/invcell.rs` — interior-mutability primitives
- `.tmp/verus/source/vstd/Cargo.toml` — crate metadata and feature flags
- `.tmp/verus/source/docs/guide/src/vstd.md`, `.tmp/verus/source/docs/guide/src/spec_lib.md`, `.tmp/verus/source/docs/guide/src/exec_lib.md` — tutorial chapters
- `.tmp/verus/examples/guide/lib_examples.rs` — runnable Seq/Set/Map examples
