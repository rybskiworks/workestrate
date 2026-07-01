# Iterators, Closures, and Functional Features

## Purpose

This document provides detailed guidance for AI coding agents writing, reviewing, and refactoring Rust code that uses closures, iterators, and functional programming patterns. It establishes the mental model, concrete rules, and common pitfalls so that agents can produce idiomatic, performant, and correct Rust code without guessing.

Rust's iterator and closure system is one of its most distinctive features. Unlike functional languages where abstractions often carry runtime cost, Rust's iterators are zero-cost abstractions that compile down to the same machine code as hand-written loops ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)). Closures integrate with the ownership system in ways that are powerful but subtle: they capture variables from their environment and the compiler selects capture modes and `Fn` trait implementations automatically based on what the closure body does. Understanding both deeply is essential for writing idiomatic Rust.

Use this document together with the ownership, error-handling, type-system, design-pattern, and async docs listed under [Related docs](#related-docs). It deliberately does not duplicate:

- Higher-ranked trait bounds (`for<'a> F: Fn(...)`), `'static` lifetime semantics, slice-vs-`&String`/`&Vec` guidance, or the mechanics of modifying a collection while iterating — these live in `docs/rust/ownership-lifetimes.md`.
- Builder, newtype, RAII, strategy, and fold patterns — these live in `docs/rust/design-patterns.md`.
- Deep treatment of `?` inside iterator chains, collecting into `Result`, and `try_fold`/`try_for_each` short-circuit error propagation — these live in `docs/rust/error-handling.md`.

## Sources used

- [The Rust Programming Language: Functional Features](https://doc.rust-lang.org/book/ch13-00-functional-features.html)
- [The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)
- [The Rust Programming Language: Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)
- [The Rust Programming Language: Improving Our I/O Project](https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html)
- [The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)
- [`std::iter`](https://doc.rust-lang.org/std/iter/)
- [`std::iter::Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- [`std::iter::IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html)
- [`std::iter::FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)
- [`std::iter::Extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html)
- [`std::iter::DoubleEndedIterator`](https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html)
- [`std::iter::ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html)
- [`std::iter::FusedIterator`](https://doc.rust-lang.org/std/iter/trait.FusedIterator.html)
- [`std::iter::Step`](https://doc.rust-lang.org/std/iter/trait.Step.html)
- [The Rust Reference: Closure types](https://doc.rust-lang.org/reference/types/closure.html)
- [The Rust Reference: Closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html)
- [`std::ops::Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html)
- [`std::ops::FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html)
- [`std::ops::FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)
- [The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)
- [`std::primitive::fn`](https://doc.rust-lang.org/std/primitive.fn.html)
- [`move` keyword](https://doc.rust-lang.org/std/keyword.move.html)

### Crawl ledger

Book chapter 13 (functional language features): [ch13-00 intro](https://doc.rust-lang.org/book/ch13-00-functional-features.html), [ch13-01 closures](https://doc.rust-lang.org/book/ch13-01-closures.html), [ch13-02 iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html), [ch13-03 I/O refactor](https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html), [ch13-04 performance](https://doc.rust-lang.org/book/ch13-04-performance.html).

Standard-library iterator module and traits: [`std::iter`](https://doc.rust-lang.org/std/iter/), [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html), [`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html), [`FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html), [`Extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html), [`DoubleEndedIterator`](https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html), [`ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html), [`FusedIterator`](https://doc.rust-lang.org/std/iter/trait.FusedIterator.html), [`Step`](https://doc.rust-lang.org/std/iter/trait.Step.html).

Closures and callable types: [Reference: closure types](https://doc.rust-lang.org/reference/types/closure.html), [Reference: closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html), [`Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html), [`FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html), [`FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html), [`fn` primitive](https://doc.rust-lang.org/std/primitive.fn.html), [`impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html), [`move`](https://doc.rust-lang.org/std/keyword.move.html).

Sibling repo docs consulted to avoid duplication: `docs/rust/ownership-lifetimes.md`, `docs/rust/error-handling.md`, `docs/rust/design-patterns.md`, `docs/rust/types-traits-generics.md`, `docs/rust/async-tokio.md`, `docs/rust/smart-pointers-memory.md`, `docs/rust/lints-clippy.md`.

## Core guidance

### Closures are anonymous functions that capture their environment

A closure in Rust is an anonymous function that can capture variables from its enclosing scope. Unlike regular `fn` functions, closures have access to the environment where they are defined. The compiler infers how each captured variable is used and automatically selects the most permissive trait bound (`Fn`, `FnMut`, or `FnOnce`) that satisfies the closure's body.

The Rust Book lists four equivalent syntax forms for a simple increment closure ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)):

```rust
fn  add_one_v1 (x: u32) -> u32 { x + 1 }
let add_one_v2 = |x: u32| -> u32 { x + 1 };
let add_one_v3 = |x|             { x + 1 };
let add_one_v4 = |x|               x + 1 ;
```

Braces are optional only for a single-expression body; with a return-type annotation, the body must be a block ([The Rust Reference: Closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html)). Closures don't usually require you to annotate the types of the parameters or the return value like `fn` functions do ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). However, the compiler will infer one concrete type for each of their parameters and for their return value; once called with `String`, the closure cannot be called with `i32`.

### Capture modes

The compiler determines capturing mode based on what the closure body does with each captured value. The Book gives the rule: "The closure will decide which of these to use based on what the body of the function does with the captured values" ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)).

- **Borrowing immutably** (`&T`): the closure only reads the variable. The closure implements `Fn`.
- **Borrowing mutably** (`&mut T`): the closure writes to the variable. The closure implements `FnMut` (and therefore `FnOnce`).
- **Taking ownership** (`T`): the closure moves the variable into itself. The closure implements `FnOnce`.

The Reference names these modes `ImmBorrow`, `MutBorrow`, and `ByValue` ([The Rust Reference: Closure types](https://doc.rust-lang.org/reference/types/closure.html)). Since Rust 2021, captures are per-field and through deref projections, so a closure that only reads the contents of a `Box<T>` captures the box by mutable reference but the `T` by immutable reference; this is more precise than the pre-2021 behavior for `Rc`/`Arc`.

The `move` keyword forces the closure to take ownership of the values it uses in the environment even though the body of the closure doesn't strictly need ownership ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). `move` is mostly useful when passing a closure to a new thread. Crucially, `move` does **not** change which `Fn` trait is implemented — traits are determined by what the body does with the captures ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). A `move` closure that only reads its captures still implements `Fn`. Use `move` when returning a closure from a function or passing it to a spawned task, because the closure must outlive the borrowed environment.

### The `Fn` trait hierarchy

The three closure traits form an additive subtyping hierarchy. All closures implement at least `FnOnce`; closures that do not move captured values out of their body also implement `FnMut`; closures that neither move nor mutate captured values also implement `Fn` ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)).

- **`FnOnce`**: "applies to closures that can be called once. All closures implement at least this trait… A closure that moves captured values out of its body will only implement FnOnce." ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). Use `FnOnce` when the closure must take ownership, for example when transferring a `String` into a spawned thread.
- **`FnMut`**: "applies to closures that don't move captured values out of their body but might mutate the captured values. These closures can be called more than once." ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). Use `FnMut` when the closure needs to modify state across calls, such as the closure passed to `slice::sort_by_key`.
- **`Fn`**: "applies to closures that don't move captured values out of their body and don't mutate the captured values, as well as closures that capture nothing." ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). This is the most restrictive trait in terms of what the closure can do, and therefore the most permissive in terms of where it can be used.

The std hierarchy is explicit: `FnOnce` is the root (with `call_once`), `FnMut: FnOnce` (with `call_mut`), and `Fn: FnMut` (with `call`) ([`std::ops::Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html), [`std::ops::FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html), [`std::ops::FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)). "Closures will automatically implement one, two, or all three of these Fn traits, in an additive fashion" ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). Accept the most restrictive trait you can in an API — prefer `Fn` over `FnMut` over `FnOnce` — to give callers the most flexibility.

The Book gives two real std-library examples. `Option::unwrap_or_else` uses `F: FnOnce() -> T` because "will not call f more than once" ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). `slice::sort_by_key` uses `FnMut` because it "calls the closure multiple times: once for each item" ([The Rust Programming Language: Closures](https://doc.rust-lang.org/book/ch13-01-closures.html)). A common error is to move a captured `String` out of an `FnMut` closure passed to `sort_by_key`, producing E0507 "cannot move out of value, a captured variable in an FnMut closure"; the fix is to capture a `&mut` counter or to restructure so the closure does not move its captures.

### Closures as values

Closure types are anonymous and unique; the Reference describes them as "approximately equivalent to a struct which contains the captured values" ([The Rust Reference: Closure types](https://doc.rust-lang.org/reference/types/closure.html)). Closure types are `Sized` and auto-implement `Clone`, `Copy`, `Sync`, and `Send` when their captures allow. This means a closure that only captures `Copy` values is itself `Copy` and can be passed around freely.

A non-capturing closure can be coerced to a function pointer (`fn`). All safe `fn` pointers implement `Fn`, `FnMut`, and `FnOnce`, and `fn` is a primitive concrete type that is `Copy`/`Clone`/`Send`/`Sync`/`Hash`/`Eq` ([`std::primitive::fn`](https://doc.rust-lang.org/std/primitive.fn.html)). Use `fn` for FFI (`extern "C" fn`), `const`/`static` initializers, and any context that requires a `Copy` callable. Capturing closures **cannot** coerce to `fn`; use `impl Fn` or `Box<dyn Fn>` instead.

The function-item type (the zero-sized type denoted by a named function) coerces to `fn`, but `&function_name` is a reference to the ZST and is almost never what you want. Write `let f: fn(i32) -> i32 = add_one_v1;` not `let f = &add_one_v1;`.

### Returning and storing closures

There are two ways to return a closure from a function. Default to `impl Fn() -> T`: it is monomorphized and zero-cost, but every possible return value from the function must resolve to the same concrete type ([The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)). Use `Box<dyn Fn() -> T>` only when you need type erasure, such as when different return arms produce different closure types or when storing closures heterogeneously in a `Vec<Box<dyn Fn()>>`. `Box<dyn Fn>` allocates and uses virtual dispatch; prefer `impl Fn` whenever possible.

A returned closure that captures locals **must** use `move`, otherwise it borrows a stack frame that will no longer exist. The canonical form is:

```rust
fn make_printer(text: String) -> impl Fn() {
    move || println!("{text}")
}
```

The same tradeoff applies to closures as struct fields. Generic zero-cost storage is `struct Holder<F: Fn()> { f: F }`; this monomorphizes and allocates nothing, but each distinct `F` is a distinct `Holder<F>` type, making heterogeneous collections impossible. Type-erased storage is `struct Holder(Box<dyn Fn()>);` this is a single non-generic type and supports heterogeneous `Vec`s, but it heap-allocates and uses virtual dispatch. Default to the generic form; box only when erasure is required.

`impl Trait` cannot appear in a `let` binding, field type, or type alias; it is allowed only in function arguments and return types ([The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)). If you need to name a closure type in a field, use a generic parameter or `Box<dyn Fn>`.

### Iterators are lazy sequences driven by `next()`

The `Iterator` trait has a single required method:

```rust
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
    // methods with default implementations elided
}
```

"The Iterator trait only requires implementors to define one method: the next method" ([The Rust Programming Language: Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)). The iterator holds state and produces items one at a time. The iterator itself must be mutable to call `next`, because calling `next` consumes/uses up the iterator ([The Rust Programming Language: Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)). After returning `None`, "calling next() again may or may not eventually start returning Some(Item) again" ([`std::iter::Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html)); this is why `FusedIterator` and `fuse()` exist.

Iterators are lazy, meaning they have no effect until you call methods that consume the iterator ([The Rust Programming Language: Functional Features](https://doc.rust-lang.org/book/ch13-00-functional-features.html)). Iterator adaptors such as `map`, `filter`, `flat_map`, and `enumerate` return new iterators that wrap the original; they do no work until a consumer drives them. Consumers such as `collect`, `fold`, `any`, `sum`, and `for_each` repeatedly call `next` and force evaluation. For example, `sum` "takes ownership of the iterator and iterates through the items by repeatedly calling next" ([The Rust Programming Language: Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)). An unconsumed `map` emits the compiler warning "iterators are lazy and do nothing unless consumed" ([The Rust Programming Language: Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)).

### Owned vs borrowed iteration

Collections conventionally provide three iteration methods ([`std::iter`](https://doc.rust-lang.org/std/iter/)):

- **`iter()`**: yields `&T` (borrowed references). The collection is not consumed. Use when you only need to read elements.
- **`iter_mut()`**: yields `&mut T` (mutable references). The collection is not consumed. Use when you need to modify elements in place.
- **`into_iter()`**: yields owned `T`. The collection is consumed. Use when you need ownership of elements.

A `for` loop desugars to `IntoIterator::into_iter(xs)` plus a loop on `next`. There is a blanket `impl<I: Iterator> IntoIterator for I`, so any `Iterator` works in `for` ([`std::iter`](https://doc.rust-lang.org/std/iter/)). Collections conventionally implement `IntoIterator for &C` (→ `iter()`) and `for &mut C` (→ `iter_mut()`). Thus:

- `for x in &collection` calls `iter()`.
- `for x in &mut collection` calls `iter_mut()`.
- `for x in collection` calls `into_iter()` and consumes the collection.

Be explicit about which you intend. `Option` and `Result` also implement `IntoIterator`, yielding their contained value once; this is useful with `extend` and `chain`.

### Iterator constructor functions

The `std::iter` module provides small constructors for common iterator shapes ([`std::iter`](https://doc.rust-lang.org/std/iter/)):

- `empty::<T>() -> Empty<T>` (1.2.0): yields nothing; `Empty` is `ExactSizeIterator` + `FusedIterator`.
- `once<T>(value: T) -> Once<T>` (1.2.0): yields one value; great with `chain()`.
- `once_with<A, F: FnOnce() -> A>(make: F) -> OnceWith<F>` (1.43.0): lazy single value via closure.
- `repeat<T: Clone>(elt: T) -> Repeat<T>` (1.0.0): endless clone of `elt`.
- `repeat_with<A, F: FnMut() -> A>(repeater: F) -> RepeatWith<F>` (1.28.0): endless closure calls; not `DoubleEndedIterator`.
- `from_fn<T, F: FnMut() -> Option<T>>(f: F) -> FromFn<F>` (1.34.0): the closure **is** the `next` implementation. Not `FusedIterator`; default `size_hint` is `(0, None)`.
- `successors<T, F: FnMut(&T) -> Option<T>>(first: Option<T>, succ: F) -> Successors<T, F>` (1.34.0): stateful closure; **is** a `FusedIterator`.

Prefer `from_fn` and `successors` over hand-rolled iterator structs for simple stateful sequences. They are shorter, harder to get wrong, and compose with the rest of the iterator ecosystem.

### Combinator catalog

The `Iterator` trait provides dozens of provided methods. Group them into three buckets.

#### Adaptors (lazy, return a new `Iterator`)

| Adaptor | Semantics |
|---|---|
| `map` | Transform each item with a closure. |
| `filter` | Keep only items matching a predicate. |
| `filter_map` | Map and filter in one step; closure returns `Option<B>`. |
| `flat_map` | Map each item to an iterator, then flatten. |
| `flatten` | Flatten an iterator of iterators. |
| `enumerate` | Yield `(index, item)` tuples starting at 0. |
| `zip` | Pair items from two iterators; stops at the shorter. |
| `chain` | Append one iterator to another. |
| `take(n)` | Yield at most the first `n` items. |
| `skip(n)` | Skip the first `n` items. |
| `take_while` | Yield items while predicate is true. |
| `skip_while` | Skip items while predicate is true, then yield the rest. |
| `step_by(n)` | Yield every `n`th item; **panics** if `n == 0`. |
| `peekable` | Wrap so `peek()` can inspect the next item without consuming it. |
| `rev` | Reverse iteration; requires `DoubleEndedIterator`. |
| `scan` | Stateful map/fold hybrid with an accumulator. |
| `fuse` | Wrap so `None` is guaranteed forever after first `None`. |
| `cycle` | Repeat the iterator endlessly; requires `Clone`. |
| `inspect` | Observe each item via side effect; passes it through unchanged. |
| `cloned` | Clone each `&T` into `T`; requires `T: Clone`. |
| `copied` | Copy each `&T` into `T`; requires `T: Copy`. Prefer over `cloned` for `Copy` types. |
| `by_ref` | Borrow the iterator so adaptors do not consume it. |
| `map_while` | Map and take while `Some`; stabilized in 1.57. |

The following are **nightly-only** and must not be used on stable Rust: `intersperse`, `intersperse_with` (#79524), `map_windows`, `array_chunks`, `next_chunk` (#98326), `advance_by`, and `advance_back_by` (#77404).

**Important correction:** `windows`, `chunk_by`, and `group_by` are **not** methods on `Iterator`. They are methods on slices (`[T]`). You cannot write `iter.windows(...)`; write `slice.windows(...)` instead.

#### Consumers (eager, return a non-iterator value)

| Consumer | Semantics |
|---|---|
| `next` | Advance and return the next item, or `None`. |
| `count` | Count remaining items; panics only on `usize` overflow. |
| `last` | Return the last item, or `None` on empty; may not terminate on infinite iterators. |
| `nth(n)` | Return the `n`th item, or `None` if out of range; does **not** panic. |
| `collect` | Build a collection; use turbofish when the target type is ambiguous. |
| `try_collect` | Collect fallible items via `Try`; stable. |
| `collect_into` | Collect into an existing `Extend` implementor. |
| `partition` | Split items into two collections by predicate. |
| `partition_in_place` / `is_partitioned` | In-place partition and predicate checks. |
| `sum` | Sum items; panics on debug overflow for some numeric types. |
| `product` | Multiply items; panics on debug overflow for some numeric types. |
| `fold` | General accumulation with an explicit initial value. |
| `try_fold` | Short-circuiting `fold` for `Result`/`Option`. |
| `reduce` | Fold using the first element as the initial value; returns `None` on empty (does **not** panic). |
| `try_reduce` | Short-circuiting `reduce`. |
| `for_each` | Run a closure for each item; for side effects only. |
| `try_for_each` | Short-circuiting `for_each`. |
| `all` | True if predicate holds for all items; empty → `true`. |
| `any` | True if predicate holds for any item; empty → `false`. |
| `find` | First item matching predicate, or `None`. |
| `try_find` | Short-circuiting `find`. |
| `find_map` | `find` + `map` combined, returning `Option<B>`. |
| `position` | Index of first matching item, or `None`. |
| `rposition` | Index from the back; requires `ExactSizeIterator + DoubleEndedIterator`. |
| `max` / `min` | Return the max/min item, or `None` on empty. |
| `max_by` / `min_by` | Return max/min by custom comparator, or `None` on empty. |
| `max_by_key` / `min_by_key` | Return max/min by projected key, or `None` on empty. |
| `unzip` | Inverse of `zip`; splits an iterator of tuples into two collections. |
| `cmp` / `partial_cmp` / `eq` / `ne` / `lt` / `le` / `gt` / `ge` / `cmp_by` / `partial_cmp_by` / `eq_by` | Lexicographic comparison between iterators. |
| `is_sorted` / `is_sorted_by` / `is_sorted_by_key` | Check ordering. |

Panic-vs-`Option` rule: `min`, `max`, `min_by*`, `max_by*`, and `reduce` return `None` on an empty iterator — they do **not** panic. `sum`, `product`, and `fold` return their identity/default on empty. `count` panics only on `usize` overflow. `step_by(0)` panics immediately.

#### Capability methods on marker traits

- `DoubleEndedIterator` provides `next_back`, `nth_back`, `rfold`, `rfind`, and `try_rfold`.
- `ExactSizeIterator` provides `len` and (nightly, #35428) `is_empty`.

Do not implement `ExactSizeIterator` for adapters that can make the iterator longer than `usize::MAX`; that is why `Chain<A, B>` is not `ExactSizeIterator` even when both `A` and `B` are ([`std::iter::ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html)).

### Iterator-adjacent traits

- **`IntoIterator`** ([`std::iter::IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html)): required method `into_iter(self)`, associated types `Item` and `IntoIter: Iterator<Item = Self::Item>`. `Vec` → owned `T`; `&Vec` → `&T`; `&mut Vec` → `&mut T`; `Option` and `Result` → their contained value. The blanket `impl<I: Iterator> IntoIterator for I` lets any iterator be used in `for`.
- **`FromIterator`** ([`std::iter::FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)): required method `from_iter<T: IntoIterator<Item = A>>`. `collect()` is the user-facing API; `FromIterator::from_iter(iter)` is sometimes more readable than a turbofish. The `Result<V, E>: FromIterator<Result<A, E>>` impl short-circuits on the first `Err`. `FromIterator` is **not** dyn-compatible (object safe).
- **`Extend`** ([`std::iter::Extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html)): required method `extend<T: IntoIterator<Item = A>>(&mut self, iter)`. Powers `vec.extend(iter)` and is the basis for building collections. `Extend` is **not** dyn-compatible. Nightly-only helper methods `extend_one` and `extend_reserve` (#72631) should not be used on stable.
- **`DoubleEndedIterator`** ([`std::iter::DoubleEndedIterator`](https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html)): required method `next_back`. The contract is that `next` and `next_back` work over the same range and "do not cross" — they meet in the middle. Enables `rev`. `flatten` and `flat_map` are `DoubleEndedIterator` only when the inner iterator is.
- **`ExactSizeIterator`** ([`std::iter::ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html)): no required methods; `len` is provided. The contract is that `size_hint` must return the exact size. However, "this trait is a safe trait… does not and cannot guarantee that the returned length is correct. This means that unsafe code must not rely on the correctness of Iterator::size_hint" ([`std::iter::ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html)). The unsafe `TrustedLen` trait is what gives that stronger guarantee.
- **`FusedIterator`** ([`std::iter::FusedIterator`](https://doc.rust-lang.org/std/iter/trait.FusedIterator.html)): marker trait with no methods. "Calling next on a fused iterator that has returned None once is guaranteed to return None again." The module docs advise: "you should not use FusedIterator in generic bounds if you need a fused iterator. Instead, you should just call Iterator::fuse() on the iterator. If the iterator is already fused, the additional Fuse wrapper will be a no-op with no performance penalty" ([`std::iter`](https://doc.rust-lang.org/std/iter/)). Implement `FusedIterator` on custom iterators that behave this way; at use sites, call `.fuse()` rather than bounding on `FusedIterator`.
- **`Step`** ([`std::iter::Step`](https://doc.rust-lang.org/std/iter/trait.Step.html)): **nightly-only** (`step_trait` #42168). It backs `Range<A>` iteration. On stable Rust you **cannot** write `T: Step` bounds.

### `size_hint` and the iterator contract

`Iterator::size_hint` returns `(usize, Option<usize>)` representing `(lower, Option<upper>)`. The default is `(0, None)` ([`std::iter::Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html)). A `None` upper bound means unbounded. `unsafe` code must not rely on the correctness of `size_hint` ([`std::iter::ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html)); it is a hint, not a contract. Implement it accurately for custom finite iterators because many consumers use it to pre-allocate.

### Zero-cost abstraction

Iterator chains compile to the same machine code as hand-written `for` loops. The Book reports a benchmark searching "The Adventures of Sherlock Holmes" for the substring "the" ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)):

```text
bench_search_for  ... bench: 19,620,300 ns/iter (+/- 915,700)
bench_search_iter ... bench: 19,234,900 ns/iter (+/- 656,100)
```

The verdict is "The two implementations have similar performance!" ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)). "Iterators are one of Rust's zero-cost abstractions, by which we mean that using the abstraction imposes no additional runtime overhead" ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)). The Book quotes Stroustrup's zero-overhead principle from his 2012 ETAPS keynote: "In general, C++ implementations obey the zero-overhead principle: What you don't use, you don't pay for. And further: What you do use, you couldn't hand code any better" ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)). "Rust code using iterators compiles to the same assembly you'd write by hand. Optimizations such as loop unrolling and eliminating bounds checking on array access apply" ([The Rust Programming Language: Performance](https://doc.rust-lang.org/book/ch13-04-performance.html)).

You should not avoid iterators for performance reasons. Prefer them for clarity, and only fall back to explicit loops when the iterator chain becomes harder to read than the loop equivalent.

### Laziness caveats

Because iterators are lazy, a standalone adaptor chain does nothing. This code prints nothing:

```rust
v.iter().map(|x| println!("{x}"));
```

Use `for` or `for_each` when the goal is a side effect. Also remember that iterators need not be finite: `(0..)` is an infinite range, and `repeat(1)` is infinite. Combine them with `take(n)` to bound them. Calling `min()` on `repeat(1)` will infinite-loop ([`std::iter`](https://doc.rust-lang.org/std/iter/)).

### Refactoring I/O projects with iterators

The Book's I/O refactor chapter shows how iterators remove intermediate allocations and mutable state ([The Rust Programming Language: Improving Our I/O Project](https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html)). `env::args()` returns an iterator, so pass it directly instead of collecting to `Vec<String>` and indexing. The refactored signature uses `impl Iterator<Item = String>`; use `args.next()` to skip the program name and then match the next two arguments. This removes a `.clone()` because values can be moved out of the iterator.

The `search` function can be refactored from an explicit loop to:

```rust
contents.lines().filter(|line| line.contains(query)).collect()
```

This "lets us avoid having a mutable intermediate `results` vector. The functional programming style prefers to minimize the amount of mutable state" ([The Rust Programming Language: Improving Our I/O Project](https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html)). A further improvement returns `impl Iterator<Item = &'a str>` instead of `Vec` so results are printed lazily as each line matches. "Most Rust programmers prefer to use the iterator style" ([The Rust Programming Language: Improving Our I/O Project](https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html)).

## Practical rules

1. **Prefer `iter()` over `into_iter()` unless you need ownership of elements.** Borrowing is cheaper and preserves the collection for later use. Only consume a collection when you genuinely need to transfer ownership of its elements.

2. **Prefer iterator chains over explicit `for` loops when the chain is clear and linear.** A chain like `items.iter().filter(pred).map(transform).collect()` is more readable than a loop with conditional accumulation. When the logic becomes deeply nested or requires complex control flow, a `for` loop is acceptable.

3. **Use `filter_map` instead of `filter` followed by `map` when the filter and map are logically coupled.** `filter_map` combines both into a single pass and avoids an `Option` unwrap or intermediate step.

4. **Always specify the collection type with `collect()` when it is not inferable from context.** Write `collect::<Vec<_>>()` or `collect::<HashMap<_, _>>()` rather than relying on type inference that may surprise readers. Use turbofish when the return type alone is insufficient.

5. **Use `move` on closures that must outlive the current scope.** This includes closures passed to `thread::spawn`, closures returned from functions, and closures stored in structs that outlive the enclosing function. Without `move`, the closure borrows local variables that will be dropped.

6. **Accept `impl Fn(Arg) -> Ret` (or `FnMut`/`FnOnce`) in function signatures rather than concrete closure types.** This allows callers to pass closures, function pointers, or any callable that satisfies the trait.

7. **Do not collect into an intermediate collection only to iterate over it again.** Chain iterators directly. If you need to iterate multiple times, collect once into a named binding and reuse it — do not collect into a temporary and then immediately iterate.

8. **Use `peekable()` when you need to look ahead one element without consuming it.** This is the idiomatic way to implement lookahead parsers or conditional consumption.

9. **Implement `Iterator` for custom types when they represent a sequence.** Only implement `Iterator` — do not also implement `IntoIterator` on the iterator itself. Implement `IntoIterator` on the owning type to define how it converts to an iterator.

10. **Use `fold` for accumulation patterns that do not fit a named combinator.** `fold` is the general-purpose iterator consumer. Prefer specific combinators (`sum`, `count`, `any`, `all`) when they match the intent exactly, because they communicate purpose more clearly.

11. **Use `flat_map` (or `flatten`) when each input item maps to zero or more output items.** This replaces nested loops or `extend` calls with a single composable step.

12. **Prefer `for_each` only when the side effect is the entire purpose and there is no result to collect.** Do not use `for_each` as a substitute for `for` loops when the body is complex or when early returns are needed. `for_each` cannot use `break`, `continue`, or `return` from the enclosing function.

13. **Use `chain` to concatenate two iterators without allocating.** This replaces `extend` or manual pushing when you need a combined iterator rather than a combined collection.

14. **Use `zip` to iterate two sequences in lockstep.** The resulting iterator yields tuples and stops when the shorter iterator is exhausted.

15. **Use `enumerate` when you need the index alongside each item.** Do not manually track a counter variable — `enumerate` is clearer and less error-prone.

16. **Prefer `from_fn` and `successors` over hand-rolled iterator structs for stateful sequences.** They are concise, correct by construction, and compose with the rest of the ecosystem. Reach for a custom struct only when you need precise `size_hint`, `DoubleEndedIterator`, or other marker-trait implementations.

17. **Use fallible combinators (`try_fold`, `try_for_each`, `collect::<Result<..>>`) for fallible iteration.** These short-circuit on the first failure without explicit loops. See `docs/rust/error-handling.md` for the deep treatment of `?` and `collect` into `Result`.

18. **Use `.copied()` over `.cloned()` when `T: Copy`.** `copied` is more restrictive and signals that no allocation or deep clone is occurring; it is also marginally cheaper to compile and read.

19. **Do not bound generic APIs on `FusedIterator` or `ExactSizeIterator` for capability.** If you need a fused iterator, call `.fuse()` at the use site. If you need a length, rely on `size_hint` or the consumer's behavior. Bounding on marker traits restricts callers unnecessarily.

20. **Prefer `reduce` over `fold` when the first element is the natural initial accumulator.** `reduce` returns `Option<T>` and avoids inventing a default value. Use `fold` only when the initial value is genuinely different from the first item.

21. **Never use nightly-only iterator methods on stable Rust.** `intersperse`, `intersperse_with`, `map_windows`, `array_chunks`, `next_chunk`, `advance_by`, `advance_back_by`, and the `Step` trait are all unstable. Code that uses them will fail to compile on stable toolchains.

22. **Remember that `windows`, `chunk_by`, and `group_by` are slice methods, not iterator methods.** Write `slice.windows(n)`, not `iter.windows(n)`.

23. **Use non-capturing closures as `fn` pointers for FFI and `Copy` contexts.** A closure that captures nothing coerces to `fn`; capturing closures do not. Explicit `fn` types are `Copy` and safe to pass across FFI boundaries.

24. **Default to `impl Fn` for returned closures; use `Box<dyn Fn>` only for type erasure.** `impl Fn` is zero-cost; `Box<dyn Fn>` allocates and uses virtual dispatch. Heterogeneous storage is the main justification for boxing.

25. **Capture by reference deliberately when a closure must not own its environment.** A `move` closure that captures a `String` and only prints it will still own the `String`; if the original variable must remain usable, do not use `move`.

## Review checklist

- [ ] Does the closure capture mode match the intended lifetime? If the closure escapes the current scope, does it use `move`?
- [ ] Is the closure trait bound (`Fn`/`FnMut`/`FnOnce`) the most restrictive that works? Prefer `Fn` over `FnMut` over `FnOnce`.
- [ ] Is `into_iter()` used only when ownership of elements is needed? Is the collection intentionally consumed?
- [ ] Are there intermediate `collect()` calls that could be replaced by chaining iterators directly?
- [ ] Does `collect()` have an explicit type annotation or is the type unambiguous from context?
- [ ] Are iterator chains readable end-to-end, or would a `for` loop be clearer?
- [ ] Is `filter_map` used where `filter` + `map` are coupled?
- [ ] Are custom iterators correctly implementing only `Iterator` (with `next()`), not redundantly implementing other iterator methods?
- [ ] Is `for_each` used only for side effects, not as a replacement for `for` loops with control flow?
- [ ] Are string iteration methods (`chars()`, `bytes()`, `lines()`, `split()`) chosen correctly for the data type and intent?
- [ ] Is `.copied()` used for `Copy` types instead of `.cloned()`?
- [ ] Are fallible iterator operations using `try_fold`, `try_for_each`, or `collect::<Result<..>>` rather than manual error plumbing?
- [ ] Are `FusedIterator` / `ExactSizeIterator` used only as implementations, not as generic bounds for capability?
- [ ] Are nightly-only iterator methods absent from stable code?
- [ ] Are `Box<dyn Fn>` usages justified by type erasure or heterogeneous storage?

## Implementation checklist

- [ ] Identify whether the closure needs to capture by reference, mutable reference, or value.
- [ ] Add `move` if the closure must outlive the enclosing scope (threads, returned closures, stored closures).
- [ ] Choose `iter()`, `iter_mut()`, or `into_iter()` based on whether you need borrowed, mutably borrowed, or owned elements.
- [ ] Compose iterator adaptors lazily — do not insert `collect()` between steps unless you need an intermediate collection for reuse.
- [ ] Select the correct consumer for the intent: `collect` for building collections, `fold` for general accumulation, `reduce` when the first element is the natural seed, `any`/`all` for predicates, `find` for search.
- [ ] Specify the target type for `collect()` explicitly when inference is ambiguous.
- [ ] For custom iterators, implement `Iterator::next()` and derive `IntoIterator` on the owning type.
- [ ] Verify that iterator chains do not introduce unnecessary allocations (e.g., collecting to `Vec` then iterating again).
- [ ] Test edge cases: empty iterators, single-element iterators, iterators that return `None` on the first call.
- [ ] Use `from_fn`/`successors` for simple stateful sequences before writing a custom iterator struct.
- [ ] Use `try_*` combinators or `collect::<Result<..>>` for fallible iteration.
- [ ] Avoid nightly-only methods; pin a toolchain or use stable equivalents.

## Validation hooks

- **Compile check**: `cargo check` or `cargo clippy` will catch mismatched closure trait bounds, missing `move` keywords that cause lifetime errors, and incorrect `collect` types.
- **Clippy lints**: `clippy::needless_collect`, `clippy::unnecessary_filter_map`, `clippy::redundant_closure`, and `clippy::map_clone` catch common iterator and closure misuse patterns.
- **Benchmark comparison**: When performance is critical, compare iterator chains against explicit loops using `cargo bench`. The Book's Sherlock Holmes benchmark shows the two styles are within a few percent; if your results differ significantly, investigate whether the iterator chain introduces an unexpected allocation or missed optimization.
- **Custom iterator validation**: Ensure custom `Iterator` implementations return `None` when exhausted and do not panic. Test with `collect::<Vec<_>>()` to verify all elements are produced, call `next()` after exhaustion to confirm it returns `None` consistently, and test `DoubleEndedIterator` by consuming from both ends.
- **Stable-toolchain check**: Ensure no nightly-only iterator methods or `Step` trait bounds are used unless the repo has an explicit nightly policy.

## Examples

### Example 1: Closure capturing modes

```rust
// CORRECT: Compiler infers the minimal capture mode for each variable.
let list = vec![1, 2, 3];
let mut count = 0;

// This closure captures `list` by reference (Fn) and `count` by mutable reference (FnMut).
let mut print_and_count = || {
    println!("{:?}", list);
    count += 1;
};
print_and_count();
print_and_count(); // Can be called again — FnMut allows multiple calls.

// INCORRECT: Forcing `move` when the closure only needs a reference.
// This unnecessarily moves `list` into the closure.
let list = vec![1, 2, 3];
let consume = move || {
    println!("{:?}", list); // `list` is moved into the closure, original is gone.
};
// list is no longer accessible here.
```

### Example 2: `move` for closures that escape the scope

```rust
use std::thread;

let data = vec![1, 2, 3];

// CORRECT: `move` transfers ownership of `data` into the closure,
// so the closure can outlive the current stack frame.
thread::spawn(move || {
    println!("{:?}", data);
}).join().unwrap();

// INCORRECT: Without `move`, the closure borrows `data`, which does not
// live long enough for the spawned thread.
// thread::spawn(|| {
//     println!("{:?}", data); // ERROR: `data` does not live long enough.
// });
```

### Example 3: `into_iter()` vs `iter()` vs `iter_mut()`

```rust
let names = vec!["alice".to_string(), "bob".to_string(), "carol".to_string()];

// iter() — borrowed references, collection survives.
let lengths: Vec<usize> = names.iter().map(|s| s.len()).collect();
// names is still usable here.

// iter_mut() — mutable references, collection survives.
let mut scores = vec![10, 20, 30];
scores.iter_mut().for_each(|s| *s += 5);
// scores is now [15, 25, 35].

// into_iter() — owned values, collection is consumed.
let owned: Vec<String> = names.into_iter().map(|mut s| {
    s.push_str("!");
    s
}).collect();
// names is no longer usable — it was consumed.
```

### Example 4: Avoiding unnecessary `collect()`

```rust
// INCORRECT: Collecting into a Vec just to iterate again.
let items = vec![1, 2, 3, 4, 5];
let evens: Vec<&i32> = items.iter().filter(|x| *x % 2 == 0).collect();
let doubled: Vec<i32> = evens.iter().map(|x| **x * 2).collect();

// CORRECT: Chain iterators directly — no intermediate allocation.
let doubled: Vec<i32> = items.iter()
    .filter(|x| *x % 2 == 0)
    .map(|x| x * 2)
    .collect();
```

### Example 5: `filter_map` for coupled filter-and-map

```rust
// INCORRECT: Separate filter and map with an unwrap that could panic.
let results: Vec<i32> = strings.iter()
    .filter(|s| s.parse::<i32>().is_ok())
    .map(|s| s.parse::<i32>().unwrap())
    .collect();

// CORRECT: filter_map combines both — parse once, no unwrap.
let results: Vec<i32> = strings.iter()
    .filter_map(|s| s.parse::<i32>().ok())
    .collect();
```

### Example 6: Custom iterator implementation with `DoubleEndedIterator`

```rust
// A simple counter iterator that counts from `start` up to (but not including) `end`.
// This example is based on the std::iter trait docs, not the Book.
struct Counter {
    start: u32,
    end: u32,
}

impl Counter {
    fn new(start: u32, end: u32) -> Self {
        Counter { start, end }
    }
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let val = self.start;
            self.start += 1;
            Some(val)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end - self.start) as usize;
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Counter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            self.end -= 1;
            Some(self.end)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Counter {}
impl FusedIterator for Counter {}

// Usage:
let sum: u32 = Counter::new(1, 5).sum(); // 1 + 2 + 3 + 4 = 10
let reversed: Vec<u32> = Counter::new(1, 5).rev().collect(); // [4, 3, 2, 1]
```

### Example 7: `collect()` type inference with turbofish

```rust
let pairs = vec![(1, "a"), (2, "b"), (3, "c")];

// When the target type is ambiguous, use turbofish.
let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();
// Or equivalently:
let map = pairs.into_iter().collect::<std::collections::HashMap<_, _>>();

// Without the type annotation, the compiler cannot determine which
// collection type to build from the iterator.
```

### Example 8: String iteration methods

```rust
let text = "hello\nworld";

// chars() — iterate over Unicode scalar values.
let char_count = text.chars().count(); // 11 (including newline)

// bytes() — iterate over raw bytes. Use only when you need byte-level access.
let byte_count = text.bytes().count(); // 11 for ASCII, differs for multi-byte.

// lines() — iterate over lines, stripping newline characters.
let lines: Vec<&str> = text.lines().collect(); // ["hello", "world"]

// split() — iterate over substrings separated by a delimiter.
let parts: Vec<&str> = "a,b,c".split(',').collect(); // ["a", "b", "c"]

// IMPORTANT: chars() yields Unicode scalar values, NOT grapheme clusters.
// The string "e\u{0301}" (é as e + combining accent) yields two chars.
// For grapheme-aware iteration, use the `unicode-segmentation` crate.
```

### Example 9: `from_fn` and `successors` for stateful sequences

```rust
use std::iter;

// from_fn: the closure IS next().
let mut n = 0;
let powers_of_two = iter::from_fn(move || {
    let current = n;
    n += 1;
    Some(1u32 << current)
}).take(10);

// successors: stateful closure receives the previous value.
let fib = iter::successors(Some((0u32, 1u32)), |(a, b)| Some((*b, a + b)))
    .map(|(a, _)| a)
    .take(10);

// once is useful for chaining a single item.
let with_prefix = iter::once(0).chain(1..5).collect::<Vec<i32>>();
```

### Example 10: Fallible iteration with `try_fold` and `try_for_each`

```rust
// try_fold short-circuits on the first Err/None.
let nums = vec![1u32, 2, 3, u32::MAX];
let first_overflow = nums.iter().try_fold(0u32, |acc, &x| {
    acc.checked_add(x).ok_or("overflow")
});
assert_eq!(first_overflow, Err("overflow"));

// try_for_each short-circuits on the first error.
fn read_all(paths: &[&str]) -> Result<String, std::io::Error> {
    let mut out = String::new();
    paths.iter().try_for_each(|&path| {
        out.push_str(&std::fs::read_to_string(path)?);
        Ok(())
    })?;
    Ok(out)
}
```

### Example 11: `copied` vs `cloned`

```rust
let nums = vec![1, 2, 3];
let refs: Vec<&i32> = nums.iter().collect();

// CORRECT: T is Copy, so use copied().
let owned_nums: Vec<i32> = refs.iter().copied().collect();

// CORRECT: T is Clone but not Copy, so use cloned().
let strings = vec!["a".to_string(), "b".to_string()];
let refs: Vec<&String> = strings.iter().collect();
let owned_strings: Vec<String> = refs.iter().cloned().collect();
```

### Example 12: Returning closures

```rust
// Default: impl Fn is zero-cost.
fn make_printer(text: String) -> impl Fn() {
    move || println!("{text}")
}

// Type erasure: Box<dyn Fn> when heterogeneous storage is needed.
fn make_dynamic_printer(text: String) -> Box<dyn Fn()> {
    Box::new(move || println!("{text}"))
}

// Heterogeneous storage.
let greeters: Vec<Box<dyn Fn()>> = vec![
    make_dynamic_printer("alice".to_string()),
    make_dynamic_printer("bob".to_string()),
];
```

### Example 13: Closures as struct fields

```rust
// Generic zero-cost storage: one concrete Holder<F> per closure type.
struct GenericHolder<F: Fn()> {
    f: F,
}

impl<F: Fn()> GenericHolder<F> {
    fn call(&self) { (self.f)(); }
}

// Type-erased storage: single non-generic type, heap allocation + vtable.
struct BoxedHolder(Box<dyn Fn()>);

impl BoxedHolder {
    fn call(&self) { (self.0)(); }
}

let h1 = GenericHolder { f: || println!("generic") };
let h2 = BoxedHolder(Box::new(|| println!("boxed")));
h1.call();
h2.call();
```

### Example 14: Collecting into `HashMap` and `String`

```rust
// Collect (K, V) tuples into a HashMap.
let map: std::collections::HashMap<String, i32> = vec![("a", 1), ("b", 2)]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();

// Collect chars or &str into a String.
let s: String = ['h', 'e', 'l', 'l', 'o'].iter().collect();
let s2: String = ["h", "e", "l", "l", "o"].iter().copied().collect();
```

### Example 15: `partition` and `unzip`

```rust
// partition splits an iterator into two collections.
let (evens, odds): (Vec<i32>, Vec<i32>) = (0..10).partition(|x| x % 2 == 0);

// unzip is the inverse of zip.
let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
let (keys, values): (Vec<i32>, Vec<&str>) = pairs.into_iter().unzip();
```

### Example 16: `DoubleEndedIterator` with `rev`

```rust
let chars: Vec<char> = ['a', 'b', 'c'].iter().copied().rev().collect();
// chars == ['c', 'b', 'a']

// rev works because the array iterator implements DoubleEndedIterator.
let palindrome = "radar".chars().eq("radar".chars().rev());
```

### Example 17: Refactoring `env::args()` and `search`

```rust
use std::env;

fn parse_config(mut args: impl Iterator<Item = String>) -> (String, String) {
    args.next(); // skip program name
    let query = args.next().expect("expected query");
    let file_path = args.next().expect("expected file path");
    (query, file_path)
}

fn search<'a>(query: &str, contents: &'a str) -> impl Iterator<Item = &'a str> {
    contents.lines().filter(move |line| line.contains(query))
}

fn main() {
    let (query, file_path) = parse_config(env::args());
    let contents = std::fs::read_to_string(&file_path).unwrap();
    for line in search(&query, &contents) {
        println!("{line}");
    }
}
```

## Common mistakes

### 1. Collecting then iterating again unnecessarily

**Mistake**: Calling `collect()` to build an intermediate `Vec`, then immediately iterating over it with another adaptor or `for` loop.

**Why it is wrong**: The intermediate collection is an unnecessary allocation. Iterator chains are lazy and compose without allocating. Collecting between steps defeats this advantage and adds both memory overhead and indirection.

**Fix**: Chain the iterators directly. Only `collect()` when you need a concrete collection for reuse, for returning, or for an API that requires an owned type.

### 2. Using `into_iter()` when `iter()` suffices

**Mistake**: Calling `into_iter()` on a collection when you only need to read elements, thereby consuming the collection.

**Why it is wrong**: The collection is consumed and cannot be used afterward. This is a semantic error if the collection is needed later, and it forces unnecessary ownership transfer.

**Fix**: Use `iter()` for read-only access. Use `into_iter()` only when you need owned elements (e.g., to transform and return them, or to move them into another structure).

### 3. Forgetting `move` on closures that escape the scope

**Mistake**: Passing a closure to `thread::spawn` or returning it from a function without the `move` keyword.

**Why it is wrong**: Without `move`, the closure borrows local variables. When the closure outlives the scope where those variables are defined, the borrow checker rejects the code with a lifetime error.

**Fix**: Add `move` to force the closure to take ownership of captured variables. Verify that the variables being moved are `Send` (for threads) or have the required lifetimes.

### 4. Using `filter` + `map` with `unwrap` instead of `filter_map`

**Mistake**: Filtering on `Result::is_ok` or `Option::is_some`, then mapping with `unwrap()`.

**Why it is wrong**: The value is parsed or inspected twice (once in the filter, once in the map). The `unwrap()` is technically safe because the filter guarantees the value exists, but it is noisy and signals potential panic to readers.

**Fix**: Use `filter_map` with `.ok()` or `and_then()`. This parses once and avoids `unwrap`.

### 5. Using `for_each` when a `for` loop is clearer

**Mistake**: Using `for_each` for complex bodies with multiple statements, nested conditions, or early returns.

**Why it is wrong**: `for_each` cannot use `break`, `continue`, or `return` from the enclosing function. Complex closures inside `for_each` are harder to read and debug than equivalent `for` loops.

**Fix**: Use a `for` loop when the body is complex or requires control flow. Reserve `for_each` for simple side-effect-only operations at the end of a chain.

### 6. Implementing `IntoIterator` on the iterator type itself

**Mistake**: Implementing both `Iterator` and `IntoIterator` on the same struct, where `IntoIterator::into_iter()` returns `Self`.

**Why it is wrong**: `IntoIterator` should be implemented on the *collection* or *owning type*, not on the iterator. Implementing it on the iterator conflates the two roles and can cause confusing type inference errors.

**Fix**: Implement `Iterator` on the iterator struct. Implement `IntoIterator` on the owning type (e.g., the collection), returning the iterator struct.

### 7. Iterating over `String` with `bytes()` when `chars()` is intended

**Mistake**: Using `.bytes()` on a `String` when the intent is to process Unicode characters.

**Why it is wrong**: `bytes()` yields raw bytes, which do not correspond to characters in multi-byte UTF-8 encoding. This produces incorrect results for non-ASCII text and can lead to invalid string slicing.

**Fix**: Use `.chars()` for Unicode scalar values. Use `.bytes()` only when you explicitly need byte-level access (e.g., parsing binary protocols). For grapheme clusters, use the `unicode-segmentation` crate.

### 8. Confusing `Fn`/`FnMut`/`FnOnce` trait bounds

**Mistake**: Requiring `FnOnce` in a function signature when `Fn` would suffice, or vice versa.

**Why it is wrong**: Requiring `FnOnce` when `Fn` suffices unnecessarily restricts callers — they cannot pass closures that need to be called multiple times. Requiring `Fn` when the body consumes a value prevents the closure from compiling.

**Fix**: Use the most restrictive trait that satisfies your needs. If you call the closure once and consume its captures, use `FnOnce`. If you call it multiple times and mutate captures, use `FnMut`. If you call it multiple times and only read captures, use `Fn`.

### 9. Not specifying the collection type for `collect()`

**Mistake**: Relying on type inference for `collect()` when the return type is not obvious from the function signature or variable binding.

**Why it is wrong**: The compiler may infer an unexpected collection type, or the code may fail to compile with an ambiguous type error. Readers also cannot determine the output type without tracing through the context.

**Fix**: Use turbofish syntax (`collect::<Vec<_>>()`) or annotate the variable type explicitly (`let result: HashMap<_, _> = ...collect()`).

### 10. Assuming iterator chains always allocate

**Mistake**: Avoiding iterator chains and writing explicit loops under the assumption that adaptors like `map` and `filter` allocate intermediate collections.

**Why it is wrong**: Rust iterators are lazy. Each adaptor wraps the previous iterator and does no work until `next()` is called. The entire chain compiles to a single loop with no intermediate allocations. This is the zero-cost abstraction guarantee.

**Fix**: Use iterator chains freely for clarity. Only fall back to explicit loops when the chain is genuinely harder to read, not for performance reasons.

### 11. Treating `min`/`max`/`reduce` as panicking on empty

**Mistake**: Writing code that assumes `min`, `max`, `min_by*`, `max_by*`, or `reduce` panic when the iterator is empty.

**Why it is wrong**: These consumers return `None` on an empty iterator. Unwrapping them without checking for `None` will panic at runtime, but the combinator itself does not panic.

**Fix**: Handle the `Option` explicitly. Use `if let Some(m) = iter.min()` or provide a default with `unwrap_or`/`unwrap_or_else`.

### 12. Using `step_by(0)`

**Mistake**: Calling `iter.step_by(0)` or `(0..10).step_by(0)`.

**Why it is wrong**: A step of zero is invalid and panics immediately, even for lazy iterators.

**Fix**: Ensure the step is positive. Use `step_by(1)` when you genuinely want every element.

### 13. Calling `.windows()`/`.chunk_by()` on an `Iterator`

**Mistake**: Writing `iter.windows(n)` or `iter.chunk_by(f)`.

**Why it is wrong**: `windows` and `chunk_by` are methods on slices (`[T]`), not on `Iterator`. They do not exist on the `Iterator` trait.

**Fix**: Collect or borrow a slice first: `slice.windows(n)` or `vec.chunk_by(f)`. If you need windowed iteration over an arbitrary iterator, collect into a `Vec` or write a custom adaptor.

### 14. Bounding on `FusedIterator` instead of calling `.fuse()`

**Mistake**: Writing a generic bound like `fn foo<I: FusedIterator>(iter: I)` because you need the fused guarantee.

**Why it is wrong**: `FusedIterator` is a marker trait; most iterators do not implement it. Bounding on it rejects valid iterators. The std docs explicitly recommend calling `.fuse()` at the use site instead.

**Fix**: Call `iter.fuse()` where you need the guarantee. If the underlying iterator is already fused, the `Fuse` wrapper is a no-op.

### 15. Returning a non-`move` closure that captures locals

**Mistake**: Returning `|| println!("{x}")` from a function when `x` is a local variable.

**Why it is wrong**: The closure borrows `x`, but `x` is dropped when the function returns. The borrow checker will reject this, and even if it did not, the closure would be a dangling reference.

**Fix**: Use `move` so the closure takes ownership of the captured locals: `move || println!("{x}")`.

### 16. Expecting a `map` side-effect to run without a consumer

**Mistake**: Writing `v.iter().map(|x| println!("{x}"));` and expecting output.

**Why it is wrong**: `map` is lazy. Without a consumer such as `collect`, `for_each`, or `last`, the closure is never called.

**Fix**: Use `for x in &v { println!("{x}"); }` or `v.iter().for_each(|x| println!("{x}"));` when side effects are the goal.

### 17. Using `cloned` where `copied` would do

**Mistake**: Writing `.cloned()` on an iterator of `&T` where `T: Copy`.

**Why it is wrong**: `copied` is more restrictive and documents that only a bitwise copy is happening. It also avoids the mental overhead of wondering whether a heap allocation or deep clone is involved.

**Fix**: Use `.copied()` when `T: Copy`; reserve `.cloned()` for `Clone` but non-`Copy` types.

## Strict vs contextual guidance

| Rule | Strictness | Rationale |
|---|---|---|
| Use `move` on closures that escape the current scope | **Strict** | Without `move`, the code will not compile when the closure outlives borrowed variables. This is not a style choice — it is a correctness requirement. |
| Specify `collect()` type when not inferable from context | **Strict** | Ambiguous `collect()` calls either fail to compile or produce surprising types. Explicit annotation is always correct. |
| Do not collect then immediately iterate again | **Strict** | This is an unnecessary allocation with no benefit. It is always wrong unless the intermediate collection is reused. |
| Prefer `filter_map` over `filter` + `map` with `unwrap` | **Strict** | The `unwrap` is redundant and signals potential panic. `filter_map` is always the better choice for this pattern. |
| Choose `iter()`/`iter_mut()`/`into_iter()` based on ownership needs | **Strict** | Using `into_iter()` when `iter()` suffices consumes the collection unintentionally. The choice must match the intent. |
| Do not use nightly-only iterator methods on stable | **Strict** | Code using `intersperse`, `map_windows`, `array_chunks`, `next_chunk`, `advance_by`, `advance_back_by`, or `T: Step` bounds will not compile on stable Rust. |
| Use `copied` over `cloned` for `Copy` types | **Strict** | `copied` is more precise and documents intent. There is no downside for `Copy` types. |
| Do not bound generic APIs on `FusedIterator` | **Strict** | Use `.fuse()` at the use site. Bounding on the marker trait rejects valid iterators. |
| Prefer iterator chains over explicit loops | **Contextual** | Iterator chains are idiomatic and zero-cost, but deeply nested chains or chains with complex control flow can be harder to read than a `for` loop. Prefer whichever is clearer. |
| Use `for_each` vs `for` loop | **Contextual** | `for_each` is appropriate for simple side effects at the end of a chain. `for` loops are better for complex bodies with control flow. Choose based on readability. |
| Use `fold` vs specific combinators | **Contextual** | `fold` is general-purpose but less communicative than `sum`, `count`, `any`, etc. Use the specific combinator when it matches the intent exactly. Use `fold` for custom accumulation. |
| Custom iterator vs generator-style function | **Contextual** | For simple sequences, implementing `Iterator` is idiomatic. For one-off iteration patterns, `from_fn`, `successors`, or a function returning `impl Iterator` may be more concise. Choose based on reuse needs. |
| `peekable()` vs manual lookahead | **Contextual** | `peekable()` is idiomatic for single-element lookahead. For multi-element lookahead or complex parsing, a manual approach may be clearer. |
| `impl Fn` vs `Box<dyn Fn>` | **Contextual** | Default to `impl Fn` for zero cost. Use `Box<dyn Fn>` only when type erasure or heterogeneous storage is required. |
| `from_fn`/`successors` vs hand-rolled iterator struct | **Contextual** | Prefer the constructor functions for simple stateful sequences. Use a custom struct when you need precise `size_hint`, `DoubleEndedIterator`, `ExactSizeIterator`, or `FusedIterator` guarantees. |
| Iterator chain length before extraction | **Contextual** | A common threshold is 4-5 adaptors, but this depends on readability and domain conventions. |

## Policy decisions for individual repos

Each repository should make and document the following decisions:

1. **Maximum iterator chain length**: Define a guideline for how many adaptors to chain before extracting a helper method or switching to a `for` loop. A common threshold is 4-5 adaptors.

2. **`collect()` annotation style**: Decide whether to prefer turbofish (`collect::<Vec<_>>()`) or variable type annotation (`let result: Vec<_> = ...collect()`). Consistency improves readability.

3. **`for_each` policy**: Decide whether `for_each` is permitted for side-effect-only chains or whether `for` loops are always preferred for side effects.

4. **Custom iterator naming convention**: Define how custom iterator types are named (e.g., `FooIter`, `FooIterator`, `Iter<Foo>`). Document the convention.

5. **Error handling in iterator chains**: Decide how to handle `Result` and `Option` in chains. Common approaches: use `filter_map` with `.ok()`, use `try_fold`/`try_for_each` for short-circuiting, or collect into `Result<Vec<_>, _>` using `.collect()`. See `docs/rust/error-handling.md` for the deep treatment.

6. **String iteration default**: Decide whether `chars()` or `bytes()` is the default for string iteration in the codebase. For most application code, `chars()` should be the default. For performance-critical paths with ASCII-only data, `bytes()` may be acceptable.

7. **`into_iter()` in function bodies**: Decide whether `into_iter()` should be used freely in function bodies (where the collection is a local variable and consumption is intentional) or whether `iter()` should be preferred even for locals.

8. **Parallel iteration**: If the repo uses `rayon`, document when to use `par_iter()` vs `iter()`. Define the threshold (collection size or operation cost) above which parallel iteration is warranted. See `docs/rust/async-tokio.md` for task-spawning context and `Send`/`Sync` considerations.

9. **When to use `try_*` combinators vs collecting `Result`**: Decide whether the codebase prefers explicit `try_fold`/`try_for_each` for early exit, or `collect::<Result<Vec<_>, _>>()` followed by `?`. Both are valid; consistency matters.

10. **`copied` vs `cloned` default**: Document that `.copied()` is the default for iterators over `Copy` types and `.cloned()` is reserved for non-`Copy` `Clone` types. Enforce with `clippy::map_clone` if desired.

11. **`rayon` `par_iter` threshold**: If the project uses `rayon`, define the minimum collection size or per-item cost that justifies `par_iter`. Small collections often run slower in parallel due to overhead.

## Related docs

- `docs/rust/ownership-lifetimes.md` — captures, lifetimes of closures and iterators, HRTB `for<'a> F: Fn(...)`, slice-vs-`&String`/`&Vec` guidance, `'static` semantics, and the mechanics of modifying a collection while iterating.
- `docs/rust/error-handling.md` — `?` inside iterator chains, collecting into `Result<Vec<_>, _>`, `try_fold`/`try_for_each` short-circuit error propagation, and `Option`/`Result` combinators.
- `docs/rust/types-traits-generics.md` — trait mechanics, associated types (`Iterator::Item`), generic bounds, `impl Trait`, object safety, and `Send`/`Sync` bounds on closures.
- `docs/rust/design-patterns.md` — closures-as-strategy, builder, RAII guards, newtype, fold, and iterator-pipeline pattern-level guidance.
- `docs/rust/async-tokio.md` — `move` closures in spawned tasks, `Send`/`'static` bounds on closures passed to runtimes, and async iteration patterns.
- `docs/rust/smart-pointers-memory.md` — `Box<dyn Fn()>` allocation, `Rc`/`Arc` for shared closures, `Deref` coercion, and interior mutability when closures share state.
- `docs/rust/lints-clippy.md` — clippy lints for iterator and closure misuse (`needless_collect`, `unnecessary_filter_map`, `redundant_closure`, `map_clone`, `ptr_arg`).

## Related skills

- `constraint-typescript-function-shape` — Analogous function-shape guidance for TypeScript (cross-language reference for agents working in both).
- `validation-frontend-compile-check` — Compile-check validation pattern; adapt for `cargo check`, `cargo clippy`, and `cargo test`.
- `frontend-command-discovery` — Inspect repo package-manager scripts to determine the correct validation commands for Rust code in a mixed codebase.
- `nix-usage` — Reference for the ai-workbench Nix flake and dev shell when toolchain or `cargo` behavior depends on the Nix environment.
