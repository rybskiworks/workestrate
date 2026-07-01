# Smart Pointers and Memory Management

## Purpose

This document provides authoritative, repo-independent guidance on Rust smart pointers and memory-management patterns for AI coding agents writing, reviewing, or refactoring Rust code. It establishes the mental model for choosing among heap pointers, reference-counted pointers, interior-mutability primitives, synchronization types, clone-on-write wrappers, and custom destructors, and it documents the concrete rules that keep that code correct and idiomatic.

Smart pointers are the primary mechanism by which Rust programs manage heap allocation, shared ownership, interior mutability, lazy initialization, and custom destruction semantics. Choosing the wrong smart pointer is one of the most frequent sources of bugs, performance regressions, and compilation failures. This document aims to eliminate that class of errors. It focuses on the smart-pointer family itself; ownership rules, move/Copy semantics, borrowing, lifetimes, variance, and `Pin` are owned by [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md), and the full undefined-behavior rules for unsafe primitives are owned by [`docs/rust/unsafe-security.md`](unsafe-security.md).

## Sources used

### The Rust Programming Language (Book)

- [The Rust Programming Language: Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html) — seed source
- [The Rust Programming Language: Using `Box<T>` to Point to Data on the Heap](https://doc.rust-lang.org/book/ch15-01-box.html)
- [The Rust Programming Language: Treating Smart Pointers Like Regular References with the `Deref` Trait](https://doc.rust-lang.org/book/ch15-02-deref.html)
- [The Rust Programming Language: Running Code on Cleanup with the `Drop` Trait](https://doc.rust-lang.org/book/ch15-03-drop.html)
- [The Rust Programming Language: `Rc<T>`, the Reference Counted Smart Pointer](https://doc.rust-lang.org/book/ch15-04-rc.html)
- [The Rust Programming Language: `RefCell<T>` and the Interior Mutability Pattern](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [The Rust Programming Language: Reference Cycles Can Leak Memory](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)
- [The Rust Programming Language: Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [The Rust Programming Language: Shared-State Concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html)

### The Rust Reference

- [The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [The Rust Reference: Pointer Types](https://doc.rust-lang.org/reference/types/pointer.html)

### The Rustonomicon

- [The Rustonomicon: Leaking](https://doc.rust-lang.org/nomicon/leaking.html)
- [The Rustonomicon: Drop Check](https://doc.rust-lang.org/nomicon/dropck.html)
- [The Rustonomicon: Arc and Mutex](https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html)

### Standard library — heap pointers and reference counting

- [`std::boxed::Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html) — seed source
- [`std::boxed` module](https://doc.rust-lang.org/std/boxed/index.html)
- [`std::rc::Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html) — seed source
- [`std::rc` module](https://doc.rust-lang.org/std/rc/index.html)
- [`std::rc::Weak`](https://doc.rust-lang.org/std/rc/struct.Weak.html)
- [`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) — seed source
- [`std::sync::Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html)
- [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [`std::sync::MutexGuard`](https://doc.rust-lang.org/std/sync/struct.MutexGuard.html)
- [`std::sync::RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)

### Standard library — interior mutability and lazy initialization

- [`std::cell::Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html)
- [`std::cell::RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html) — seed source
- [`std::cell::UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)
- [`std::cell::OnceCell`](https://doc.rust-lang.org/std/cell/struct.OnceCell.html)
- [`std::sync::OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)

### Standard library — coercion, borrowing, conversion, and destruction

- [`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html)
- [`std::ops::DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html)
- [`std::ops::Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)
- [`std::mem::drop`](https://doc.rust-lang.org/std/mem/fn.drop.html)
- [`std::mem::ManuallyDrop`](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html)
- [`std::mem::MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)
- [`std::borrow::Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
- [`std::borrow::Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html)
- [`std::borrow::ToOwned`](https://doc.rust-lang.org/std/borrow/trait.ToOwned.html)

Claims below are inline-cited to the specific URLs above.

## Core guidance

### Smart pointers are ownership abstractions, not "nice to have" conveniences

A smart pointer is any type that behaves like a pointer but also owns metadata or cleanup logic. In Rust the canonical smart pointers are `Box<T>`, `Rc<T>`, `Arc<T>`, `Weak<T>`, `Cell<T>`, `RefCell<T>`, `OnceCell<T>`, `OnceLock<T>`, `Mutex<T>`, `RwLock<T>`, `Cow<'a, B>`, and the unsafe primitives `UnsafeCell<T>`, `ManuallyDrop<T>`, and `MaybeUninit<T>`. Each solves a specific ownership or mutability problem. Choose by need, not by habit.

### The ownership-mutability matrix

Every value sits at the intersection of an ownership axis (exclusive vs shared) and a mutability axis (exterior vs interior):

| | Exclusive ownership | Shared ownership |
|---|---|---|
| **Exterior mutability** | `T`, `Box<T>` | `Rc<T>`, `Arc<T>` |
| **Interior mutability** | `Cell<T>`, `RefCell<T>` | `Rc<RefCell<T>>`, `Arc<Mutex<T>>`, `Arc<RwLock<T>>` |

Exterior mutability means the compiler enforces `&mut` exclusivity at compile time. Interior mutability moves enforcement to runtime — `RefCell` panics, `Mutex` blocks, `Cell` avoids aliasing by moving values in and out. Prefer exterior mutability whenever possible; it has zero runtime cost and catches bugs at compile time. Interior mutability is a tool of necessity, not convenience.

### The heap-allocation principle

Rust values live on the stack by default. Move a value to the heap when you need one of these properties:

1. **Dynamically sized types** — trait objects (`dyn Trait`), recursive types, and slices require indirection because their size is unknown at compile time.
2. **Ownership transfer without copying** — moving a large value between owners is cheap when only the pointer is copied.
3. **Explicit lifetime management** — heap allocation decouples a value's lifetime from the stack frame that created it.

`Box<T>` is the default answer for heap allocation. It is exclusive, has no runtime overhead beyond the allocation, and the compiler can optimize through it. Reach for `Rc`, `Arc`, or interior-mutability wrappers only when shared ownership or interior mutability is required.

### `Box<T>` — exclusive heap ownership

`Box<T>` is "the default single-owner heap pointer" ([std `Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)). The Book notes that "Boxes don't have performance overhead, other than storing their data on the heap" ([Book ch15-01](https://doc.rust-lang.org/book/ch15-01-box.html)). Layout is also predictable: "So long as `T: Sized`, a `Box<T>` is guaranteed to be represented as a single pointer and is also ABI-compatible with C pointers (i.e. the C type `T*`)" ([std `boxed` module](https://doc.rust-lang.org/std/boxed/index.html)). `Box<T>` asserts uniqueness over its contents; aliasing rules are the same as for `&mut T`.

The three canonical Book use cases for `Box<T>` are ([Book ch15-01](https://doc.rust-lang.org/book/ch15-01-box.html)):

1. A type whose size cannot be known at compile time (recursive types, DSTs) — the compiler error even hints "insert some indirection (e.g., a `Box`, `Rc`, or `&`)".
2. Large data you want to transfer without copying.
3. Owning a value where you care only that it implements a trait — trait objects `Box<dyn Trait>`.

Stable API highlights:

- `Box::new(x: T) -> Box<T>` (does not allocate if `T` is a zero-sized type).
- `Box::pin(x: T) -> Pin<Box<T>>` (stable since 1.33.0) — "If `T` does not implement `Unpin`, then `x` will be pinned in memory and unable to be moved" ([std `Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)).
- `Box::into_pin(boxed)` (stable since 1.63.0) — converts an existing `Box` to `Pin<Box<T>>` in place, without reallocation.
- `Box::leak<'a>(b) -> &'a mut T` (stable since 1.26.0) — consumes the box and leaks it, returning a mutable reference. This is the canonical way to obtain a `&'static mut T`. "Dropping the returned reference will cause a memory leak" unless you recover ownership with `Box::from_raw` ([std `Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)).
- `Box::into_raw(b) -> *mut T` (stable since 1.4.0) — transfers ownership to the caller as a raw pointer. The pointer is aligned and non-null.
- `pub unsafe fn from_raw(raw: *mut T) -> Box<T>` (stable since 1.4.0) — "a double-free may occur if the function is called twice on the same raw pointer"; the pointer must come from the global allocator ([std `Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)).
- `Box::new_uninit() -> Box<MaybeUninit<T>>` (stable since 1.82.0) and `Box::new_zeroed()` (stable since 1.92.0) allocate without immediately producing a valid `T`.
- Allocator-API variants such as `try_new`, `new_in`, `try_new_uninit_in`, `from_non_null`, `map`, and `try_map` are nightly-only behind `feature(allocator_api)` (tracking issue [#32838](https://github.com/rust-lang/rust/issues/32838)).

### `Rc<T>` — single-threaded shared ownership

> A single-threaded reference-counting pointer.
> — [std `Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html)

`Rc<T>` provides shared ownership via a heap allocation. `clone` produces a new pointer to the same allocation, increments the strong reference count, and frees the inner value only when the last strong `Rc` is dropped. `Rc::clone` is cheap: it increments a count, not a deep copy. Rust's convention is to call `Rc::clone(&rc)` rather than `rc.clone()` so reviewers can distinguish reference-count bumps from deep clones when hunting performance bugs ([Book ch15-04](https://doc.rust-lang.org/book/ch15-04-rc.html)).

Key APIs:

- `Rc::new_cyclic<F: FnOnce(&Weak<T>) -> T>(data_fn) -> Rc<T>` (stable since 1.60.0) constructs an `Rc` that can hold a weak back-reference to itself before the `Rc` exists. "Calling `upgrade` on the weak reference inside your closure will fail and result in a `None` value" ([std `Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html)).
- `Rc::try_unwrap` (stable since 1.4.0) and `Rc::into_inner` (stable since 1.70.0) attempt to recover the inner value if the strong count is one.
- `Rc::get_mut(&mut rc) -> Option<&mut T>` returns `Some` only if the strong count is one and there are no outstanding `Weak` references.
- `Rc::make_mut(&mut rc)` provides clone-on-write semantics.
- `Rc::downgrade(&rc) -> Weak<T>` creates a non-owning reference.
- `Rc::pin` (stable since 1.33.0) pins an `Rc` on the heap.
- `Rc::strong_count(&rc)` and `Rc::weak_count(&rc)` expose the current counts.

`Rc<T>` is neither `Send` nor `Sync` — the compiler enforces this with explicit negative impls. "`Rc` uses non-atomic reference counting… cannot be sent between threads, and consequently `Rc<T>` does not implement `Send`" ([std `rc` module](https://doc.rust-lang.org/std/rc/index.html)). Do not work around this with unsafe code; use `Arc<T>` instead.

`Rc<T>` is not `Copy` but it is `Clone`. The canonical single-threaded shared-mutable pattern is `Rc<RefCell<T>>`.

### `Arc<T>` — thread-safe shared ownership

> A thread-safe reference-counting pointer.
> — [std `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)

`Arc<T>` uses atomic operations for its reference count. The docs warn that "atomic operations are more expensive than ordinary memory accesses. If you are not sharing… consider using `Rc<T>`" ([std `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)). `Arc<T>` is `Send + Sync` if and only if `T: Send + Sync`. A critical quote:

> `Arc<T>` makes it thread safe to have multiple ownership of the same data, but it doesn't add thread safety to its data. Consider `Arc<RefCell<T>>`. `RefCell<T>` isn't `Sync`… you may need to pair `Arc<T>` with some sort of `std::sync` type, usually `Mutex<T>`.
> — [std `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)

Three documented ways to mutate through an `Arc` are:

1. Interior mutability with `Mutex`, `RwLock`, or atomics.
2. `Arc::make_mut` for clone-on-write when you have `&mut Arc<T>`.
3. `Arc::get_mut` when the strong count is one.

API mirrors `Rc`: `Arc::new`, `Arc::new_cyclic` (stable since 1.60.0), `Arc::new_uninit` (stable since 1.82.0), `Arc::new_zeroed` (stable since 1.92.0), `Arc::pin` (stable since 1.33.0), `Arc::try_unwrap` (stable since 1.4.0), and `Arc::into_inner` (stable since 1.70.0).

`Arc::try_unwrap(this).ok()` is an important footgun: when called concurrently, two threads can both determine they are not the last owner, both discard the `Err`, and both drop the value, leading to a use-after-free or double-drop. The docs "strongly recommend" using `Arc::into_inner` instead ([std `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)).

`Arc` is only available on platforms with pointer-sized atomics: `#[cfg(target_has_atomic = "ptr")]`.

### `Weak<T>` — non-owning references

`rc::Weak<T>` and `sync::Weak<T>` are "a version of `Rc`/`Arc` that holds a non-owning reference" ([std `rc::Weak`](https://doc.rust-lang.org/std/rc/struct.Weak.html), [std `sync::Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html)). A `Weak` does **not** keep the inner value alive (the value is dropped when the strong count hits zero), but it **does** keep the backing allocation alive. `Weak::upgrade() -> Option<Rc<T>>` / `Option<Arc<T>>` returns `None` if all strong references have been dropped. Construct via `Rc::downgrade`/`Arc::downgrade`, or `Weak::new()` (stable since 1.10.0, `const` since 1.73.0), which always upgrades to `None`.

The standard tree pattern from the Book uses a strong parent→child edge and a weak child→parent back-edge ([Book ch15-06](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)):

```rust
struct Node {
    value: i32,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}
```

The Book's rule: "a parent node should own its children… a child should not own its parent… This is a case for weak references!" ([Book ch15-06](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)).

Reference cycles are possible even in safe Rust:

> Rust's memory safety guarantees make it difficult, but not impossible, to accidentally create memory that is never cleaned up (known as a memory leak). Preventing memory leaks entirely is not one of Rust's guarantees.
> — [Book ch15-06](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)

`rc::Weak` is `!Send` and `!Sync`, and `weak_count` is exact. `sync::Weak` is `Send + Sync` iff `T: Send + Sync + ?Sized` (plus allocator bounds), and `weak_count` is approximate: "can be off by 1 in either direction when other threads are manipulating any `Arc`s or `Weak`s" ([std `sync::Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html)). Both expose `into_raw`, `from_raw`, and `as_ptr` for FFI interop. `Weak::ptr_eq` ignores `dyn Trait` metadata, which is a common footgun when comparing trait-object weak pointers.

### `Cell<T>` — interior mutability for `Copy` types

> A mutable memory location.
> — [std `Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html)

`Cell<T>` has the same memory layout as `UnsafeCell<T>` (and therefore the same size/alignment as `T`). It works by moving values in and out, which means a `&T` to the inner value can never be obtained. `Cell::get(&self) -> T` therefore requires `T: Copy` by fundamental design. It is `!Sync` but `Send` iff `T: Send`, and it has zero runtime overhead.

API highlights:

- `new`, `set(&self, val)` (drops the replaced value), `swap`, `replace(&self, val) -> T`, `into_inner`.
- `get(&self) -> T where T: Copy`.
- `update(&self, f)` (stable since 1.88.0, requires `T: Copy`).
- `take(&self) -> T where T: Default`.
- `get_mut(&mut self) -> &mut T` for compile-time exclusive access.
- `as_ptr`, `from_mut`, `as_slice_of_cells` (stable since 1.37.0), `as_array_of_cells` (stable since 1.91.0).

The docs recommend `Cell` for "more simple types where copying or moving values isn't too resource intensive (e.g. numbers), and should usually be preferred over other cell types when possible" ([std `Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html)). A real-world example: `Rc`'s internal strong and weak counts are stored in `Cell<usize>`.

### `RefCell<T>` — runtime borrow-checked interior mutability

> A mutable memory location with dynamically checked borrow rules.
> — [std `RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html)

`RefCell<T>` enforces Rust's aliasing rules at runtime and pays a small runtime performance penalty for that tracking. It is the right choice when you need interior mutability for a non-`Copy` type in a single-threaded context.

API highlights:

- `borrow(&self) -> Ref<'_, T>` — panics if currently mutably borrowed.
- `borrow_mut(&self) -> RefMut<'_, T>` — panics if currently borrowed at all.
- `try_borrow() -> Result<Ref<'_, T>, BorrowError>` and `try_borrow_mut() -> Result<RefMut<'_, T>, BorrowMutError>` are non-panicking alternatives.
- `replace`, `replace_with`, `swap`, `take` (stable since 1.50.0, requires `T: Default`), `into_inner`, `get_mut`, `as_ptr`.

The panic message is `"already borrowed: BorrowMutError"` ([Book ch15-05](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)). `RefCell<T>` is `!Sync` — it cannot be shared across threads. The Book is explicit: "`RefCell<T>` does not work for multithreaded code! `Mutex<T>` is the thread-safe version" ([Book ch15-05](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)). `RefCell<T>: Send` iff `T: Send`. The canonical pattern is `Rc<RefCell<T>>`.

### `UnsafeCell<T>` — the core primitive for interior mutability

> The core primitive for interior mutability in Rust.
> — [std `UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)

All other interior-mutability types use `UnsafeCell` internally: "All other types that allow internal mutability, such as `Cell<T>` and `RefCell<T>`, internally use `UnsafeCell`" ([std `UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)). `UnsafeCell<T>` opts out of the immutability guarantee for `&T`: a shared reference `&UnsafeCell<T>` may point to data that is being mutated. However, "There is no legal way to obtain aliasing `&mut`, not even with `UnsafeCell<T>`" ([std `UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)). It has the same layout as `T` and is `!Sync`.

API highlights:

- `new`, `into_inner`, `get(&self) -> *mut T` (the canonical way to obtain `*mut T` from a shared reference).
- `get_mut(&mut self) -> &mut T`.
- `from_mut` (stable since 1.84.0), `raw_get` (stable since 1.56.0).

Critical rule: never construct `&mut T` by casting `&UnsafeCell<T>` directly. Always source the raw pointer from `UnsafeCell::get` or `UnsafeCell::raw_get`. Casting a shared reference to a mutable reference without going through the raw pointer is undefined behavior. The full UB rules live in [`docs/rust/unsafe-security.md`](unsafe-security.md).

### `OnceCell<T>` and `OnceLock<T>` — one-time initialization

`OnceCell<T>` (stable since 1.70.0, in `std::cell`) is "A cell which can nominally be written to only once" ([std `OnceCell`](https://doc.rust-lang.org/std/cell/struct.OnceCell.html)). It gives shared `&T` access without copying (unlike `Cell`) and without runtime borrow checks (unlike `RefCell`). Mutating access is only available through `&mut OnceCell<T>`.

`OnceCell` API highlights:

- `get() -> Option<&T>`, `get_mut`, `into_inner`, `take(&mut self)`.
- `set(&self, value) -> Result<(), T>` returns `Err(value)` if already initialized.
- `get_or_init<F: FnOnce() -> T>(&self, f) -> &T` initializes once and returns `&T`.

If `f()` panics, "the cell remains uninitialized". It is also an error to reentrantly initialize the cell from `f`; doing so panics ([std `OnceCell`](https://doc.rust-lang.org/std/cell/struct.OnceCell.html)). `OnceCell<T>` is `!Sync` and is intended for single-threaded lazy initialization.

`OnceLock<T>` (stable since 1.70.0, in `std::sync`) is the "thread-safe `OnceCell`, and can be used in statics" ([std `OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)). Its API is similar but thread-safe:

- `get() -> Option<&T>` never blocks.
- `set` may block if another thread is currently initializing.
- `get_or_init` guarantees "only one function will be executed" even when many threads call it concurrently.
- `wait()` (stable since 1.86.0) blocks until initialization completes.
- `get_mut`, `into_inner`, `take`.

`OnceLock<T>` is `Sync` when `T: Send + Sync`. Unlike `Mutex`, it is never poisoned on panic. Use `OnceLock` for lazy statics and one-time configuration that may be read from many threads.

### `Mutex<T>` and `RwLock<T>` — thread-safe interior mutability

`Mutex<T>` is "A mutual exclusion primitive useful for protecting shared data… block threads waiting for the lock… data only accessed through RAII guards" ([std `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)). `Mutex::new` is a `const fn` (const-stable since 1.63.0). `lock(&self) -> LockResult<MutexGuard<'_, T>>` blocks until the lock is acquired; `try_lock` returns `TryLockResult` and yields `WouldBlock` if already held. `LockResult<T> = Result<T, PoisonError<T>>`.

Poisoning: "a mutex becomes poisoned if it recognizes that the thread holding it has panicked" (the guard is dropped during unwind). The standard recovery is `Err(poisoned) => poisoned.into_inner()`. However, "Poisoning is only advisory… unsafe code cannot rely on poisoning for soundness, since the behavior of poisoning can depend on outside context" ([std `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)).

`MutexGuard` is RAII: it releases the lock on `Drop` and implements `Deref` and `DerefMut`. Critically, `MutexGuard` is `!Send`: "A MutexGuard is not Send to maximize platform portability. On pthreads platforms… requirement to release mutex locks on the same thread they were acquired" ([std `MutexGuard`](https://doc.rust-lang.org/std/sync/struct.MutexGuard.html)). `Mutex<T>` is `Send + Sync` when `T: Send` — it does **not** require `T: Sync` because "`&T` is only made available to one thread at a time" ([std `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)).

The standard library `Mutex` is not reentrant: "this function will not return on the second call (it might panic or deadlock)" if you re-lock the same mutex on the same thread ([std `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)).

`RwLock<T>` "allows a number of readers or at most one writer at any point in time" ([std `RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)). It provides `read()`, `try_read()`, `write()`, and `try_write()`. `RwLockReadGuard` implements `Deref`; `RwLockWriteGuard` implements `Deref` and `DerefMut`. `RwLockWriteGuard::downgrade(self) -> RwLockReadGuard` (stable since 1.92.0) atomically downgrades a writer lock to a reader lock and cannot fail. `RwLock<T>` is `Sync` when `T: Send + Sync`.

Poisoning for `RwLock` only happens on exclusive (write) panic: "an RwLock may only be poisoned if a panic occurs while it is locked exclusively (write mode). If a panic occurs in any reader, then the lock will not be poisoned" ([std `RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)).

Reader/writer priority is platform-dependent: the docs note that `RwLock` "does not guarantee that any particular policy will be used", and the standard library documentation gives a deadlock example caused by that non-determinism. `RwLock` is also not reentrant.

For async code, the choice between `std::sync::Mutex` and `tokio::sync::Mutex` is owned by [`docs/rust/async-tokio.md`](async-tokio.md). The short version: use `std::sync::Mutex` for short critical sections over plain data that do not cross `.await`; use `tokio::sync::Mutex` only when the guard must cross an `.await`. Strict rule: never hold a `std::sync::MutexGuard` across `.await`.

The Book's canonical shared-state pattern is `Arc<Mutex<T>>`:

> thanks to Rust's type system and ownership rules, you can't get locking and unlocking wrong.
> — [Book ch16-03](https://doc.rust-lang.org/book/ch16-03-shared-state.html)

### `Cow<'a, B>` — clone-on-write

`Cow<'a, B>` is "A clone-on-write smart pointer" ([std `Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)). Its definition is:

```rust
pub enum Cow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    Borrowed(&'a B),
    Owned(<B as ToOwned>::Owned),
}
```

It implements `Deref<Target = B>`, so you can call non-mutating methods directly. `to_mut(&mut self) -> &mut <B as ToOwned>::Owned` clones the borrowed value if it is not already owned. `into_owned(self) -> <B as ToOwned>::Owned` clones if borrowed, moves if owned. Many zero-copy `From` impls exist: `From<&str>`, `From<String>`, `From<&[T]>`, `From<Vec<T>>`, etc.

`is_borrowed` and `is_owned` are nightly-only behind `cow_is_borrowed` (tracking issue [#65143](https://github.com/rust-lang/rust/issues/65143)); on stable use `matches!(c, Cow::Borrowed(_))`.

Use `Cow` when a function may or may not need to mutate its input: the common read-only path stays zero-copy, and cloning happens only on the first mutation.

### `Borrow` and `ToOwned`

`Borrow<Borrowed>` provides `fn borrow(&self) -> &Borrowed` with an invariant: `Eq`, `Ord`, and `Hash` must be equivalent for borrowed and owned values. This enables heterogeneous lookup, e.g. `HashMap<K, V>::get<Q>(&self, k: &Q) where K: Borrow<Q>` ([std `Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html)). Key rule: if you want to borrow only a single field of a struct, implement `AsRef`, not `Borrow`, unless the owned and borrowed versions preserve Hash/Eq/Ord equivalence.

`ToOwned` generalizes `Clone`:

```rust
pub trait ToOwned {
    type Owned: Borrow<Self>;
    fn to_owned(&self) -> Self::Owned;
}
```

It is not dyn-compatible. `Cow<'a, B>` is the most common consumer of `ToOwned`.

### `Deref` and `DerefMut`

The signatures are ([std `Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html), [std `DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html)):

```rust
pub trait Deref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

pub trait DerefMut: Deref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}
```

The Book describes three coercion cases ([Book ch15-02](https://doc.rust-lang.org/book/ch15-02-deref.html)):

1. `&T` → `&U` when `T: Deref<Target = U>`.
2. `&mut T` → `&mut U` when `T: DerefMut<Target = U>`.
3. `&mut T` → `&U` when `T: Deref<Target = U>`.

"Immutable references will never coerce to mutable references." Coercion is resolved at compile time and has no runtime penalty; it can chain (e.g. `String` → `str`, `Vec<T>` → `[T]`, `PathBuf` → `Path`).

Critical guidance: do not implement `Deref` for mere conversion. The `Deref` docs advise implementing the deref traits only when (a) the value transparently behaves like the target, (b) the implementation is cheap, and (c) users will not be surprised. Do not implement `Deref` if the conversion could fail unexpectedly, method collision is likely, or you do not want to commit to deref coercion as part of your public API. "Deref coercion is a powerful language feature which has far-reaching implications." The docs also note that "The `AsRef` and `Borrow` traits have very similar signatures… It may be desirable to implement either or both of these, whether in addition to or rather than deref traits" ([std `Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html)).

Generic-target deref impls (such as `Box<T>`) should expose few or no inherent methods on the wrapper to reduce method-collision risk; specific-target impls (such as `String → str`) can expose many. For explicit conversions, prefer `AsRef` and `Borrow`. The common mistake of "using `Deref` for conversion" is covered in depth in [`docs/rust/types-traits-generics.md`](types-traits-generics.md); this document gives only the smart-pointer framing and a cross-reference.

Common `Deref`/`DerefMut` impls in the standard library include `Box<T> → T`, `Rc<T>/Arc<T> → T`, `String → str`, `Vec<T> → [T]`, `PathBuf → Path`, `OsString → OsStr`, `CString → CStr`, `Cow<'a, B> → B`, `Pin<Ptr> → Ptr::Target`, `ManuallyDrop<T> → T`, `MutexGuard<'_, T>/RwLock*Guard<'_, T> → T`, and `LazyCell<T>/LazyLock<T> → T`.

### `Drop` and drop order

`Drop` is defined as ([std `Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)):

```rust
pub trait Drop {
    fn drop(&mut self);
}
```

`drop` is called automatically at the end of a scope. It **cannot** be called manually; doing so produces error E0040 ("explicit use of destructor method"). To drop a value early, use `std::mem::drop(value)`, which is defined as `pub fn drop<T>(_x: T) {}` — it moves the value in and lets it drop on return, doing nothing for `Copy` types ([std `mem::drop`](https://doc.rust-lang.org/std/mem/fn.drop.html)).

A type cannot implement both `Copy` and `Drop` (errors E0184/E0204). The compiler explains that "Types that are `Copy` get implicitly duplicated… making it very hard to predict when destructors will be executed." If a field needs custom cleanup, the type cannot be `Copy`.

Drop order is specified by the Reference ([Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)):

- The destructor of `T` first calls `<T as Drop>::drop` if `T: Drop`.
- It then recursively runs destructors of all fields: struct fields in **declaration order**, enum-variant fields in declaration order, tuple fields in order, array elements first to last. So a type's own `Drop::drop` runs **before** its fields are dropped.
- Local variables in a scope are dropped in **reverse order of declaration** — this is "guaranteed by the language".
- Function parameters are dropped last (at the end of the function body's scope).
- In the 2024 edition, `if let` temporaries are dropped before `else`, and tail-expression temporaries are dropped immediately after evaluation.

The basic drop-order example is shown in [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) under "Example 7: Drop order"; this document cross-references it rather than duplicating the example.

Panicking inside `Drop` is dangerous: "Implementations should generally avoid `panic!`ing, because `drop()` may itself be called during unwinding due to a panic, and if the `drop()` panics in that situation (a 'double panic'), this will likely abort the program." If you must report a bug-state at drop, check `std::thread::panicking()` first. The interaction between panics and destructors is also discussed in [`docs/rust/error-handling.md`](error-handling.md).

Drop-check: if `T: Drop`, all generic lifetime and type parameters must be live when the value is dropped. `ManuallyDrop`, `PhantomData`, and zero-length arrays are considered to never have a destructor. Deep coverage of drop-check is in the Nomicon and [`docs/rust/unsafe-security.md`](unsafe-security.md).

Implement `Drop` only when a type directly owns a resource not represented by another field with its own `Drop` — file descriptors, sockets, FFI handles, OS resources, or raw pointers. "As Rust automatically calls the destructors of all contained fields, you don't have to implement `Drop` in most cases" ([std `Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)).

### `ManuallyDrop<T>` and `MaybeUninit<T>` — unsafe primitives (overview)

These types are unsafe primitives. This document gives only an overview and points to [`docs/rust/unsafe-security.md`](unsafe-security.md) for the full UB rules.

`ManuallyDrop<T>` (stable since 1.20.0) "is a wrapper to inhibit the compiler from automatically calling `T`'s destructor. This wrapper is 0-cost" ([std `ManuallyDrop`](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html)). It has the same layout and bit-validity as `T`, and implements `Deref`/`DerefMut`. APIs include `new`, `into_inner`, `unsafe fn take(&mut self) -> T`, and `unsafe fn drop(&mut self)`. A documented hazard: "if you have a `ManuallyDrop<T>`, where the type `T` is a `Box` or contains a `Box` inside, then dropping the `T` followed by moving the `ManuallyDrop<T>` is considered to be undefined behavior." Typical uses are FFI ownership transfer to C, custom drop ordering, and intrusive structures. Prefer reordering struct fields over `ManuallyDrop` when you only need to control drop order.

`MaybeUninit<T>` (stable since 1.36.0) is "A wrapper type to construct uninitialized instances of `T`" ([std `MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)). It is a union with the same size, alignment, and ABI as `T`, and `MaybeUninit<T>` itself has no validity requirement. APIs include `new`, `uninit`, `zeroed`, `write`, `as_ptr`/`as_mut_ptr`, `unsafe fn assume_init(self) -> T`, `unsafe fn assume_init_read`, `unsafe fn assume_init_drop`, and `unsafe fn assume_init_ref`/`assume_init_mut`. Critical rule: calling `assume_init` on uninitialized memory is immediate UB even for `i32` or `bool`. `MaybeUninit<T>` does not run `T`'s destructor; you must arrange cleanup yourself. Prefer `MaybeUninit` over `ManuallyDrop` for new code that needs uninitialized memory.

### Decision matrix

| Need | Pointer |
|---|---|
| Exclusive heap ownership | `Box<T>` |
| Trait object (dynamic dispatch) | `Box<dyn Trait>` |
| Shared ownership, single-threaded | `Rc<T>` |
| Shared ownership, multi-threaded | `Arc<T>` |
| Non-owning reference / break cycles | `Weak<T>` |
| Interior mutability, `Copy` type, single-thread | `Cell<T>` |
| Interior mutability, any type, single-thread | `RefCell<T>` |
| Lazy init, single-thread | `OnceCell<T>` |
| Lazy init, multi-thread / static | `OnceLock<T>` |
| Interior mutability, multi-thread | `Arc<Mutex<T>>` |
| Interior mutability, read-heavy multi-thread | `Arc<RwLock<T>>` |
| Clone-on-write | `Cow<'a, B>` |
| Building a primitive needing raw `*mut T` | `UnsafeCell<T>` (unsafe) |
| Suppress destructor (FFI) | `ManuallyDrop<T>` (unsafe) |
| Uninitialized memory | `MaybeUninit<T>` (unsafe) |

## Practical rules

1. **Use `Box<T>` as the default smart pointer.** When you need heap allocation and exclusive ownership, `Box` is always the right choice. It has no runtime overhead beyond the allocation and communicates intent clearly.

2. **Use `Rc<T>` only when you need shared ownership within a single thread.** `Rc::clone` increments a reference count, not a deep copy. Never send `Rc<T>` across a thread boundary; the compiler will stop you (`Rc<T>` is `!Send`).

3. **Use `Arc<T>` when shared ownership crosses thread boundaries.** `Arc` uses atomic reference counting. The cost is small compared to the safety guarantee; if you are unsure whether a value will be shared across threads, prefer `Arc`.

4. **Use `Weak<T>` to break reference cycles.** Strong parent→child ownership with weak child→parent back-edges is the canonical tree pattern. Never create strong cycles deliberately.

5. **Use `Cell<T>` for interior mutability of `Copy` types.** `Cell::set` and `Cell::get` are zero-cost and panic-free. Do not use `RefCell<u32>` or `RefCell<bool>`.

6. **Use `RefCell<T>` for interior mutability of non-`Copy` types in single-threaded code.** Keep borrows scoped tightly; do not store `Ref` or `RefMut` in struct fields or return them across function boundaries.

7. **Use `Mutex<T>` for thread-safe interior mutability.** Hold the guard for the minimum scope necessary. Never re-lock the same `Mutex` from the same thread. Never hold a `std::sync::MutexGuard` across `.await`.

8. **Use `RwLock<T>` only when reads vastly outnumber writes and profiling justifies it.** `RwLock` has higher overhead and platform-dependent reader/writer priority. Default to `Mutex`.

9. **Use `OnceLock<T>` for lazy statics and one-time configuration read from many threads.** Use `OnceCell<T>` only for single-threaded lazy initialization.

10. **Use `Cow<'a, B>` when mutation is rare.** Keep the borrowed path zero-copy; clone only on `to_mut` or `into_owned`.

11. **Prefer references over smart pointers when lifetimes are statically provable.** `&T` is always cheaper and clearer than `Rc<T>` or `Arc<T>`.

12. **Implement `Drop` only for direct ownership of non-Rust resources.** The compiler-generated destructor is correct for most types. Never panic in `Drop::drop`.

13. **Use `Box::leak` only when you genuinely need a `&'static mut T`.** Remember that leaking is forever unless you recover ownership with `Box::from_raw`.

14. **Never call `Box::from_raw` twice on the same pointer.** Doing so is a double-free.

15. **Use `Arc::into_inner` instead of `Arc::try_unwrap(...).ok()` across threads.** The latter is racy and can drop the inner value while another thread still uses it.

16. **Use `Rc::new_cyclic` when a value needs a weak reference to itself during construction.** Do not try to create self-references by hand with partially initialized `Rc`s.

17. **Always match the `Option` returned by `Weak::upgrade()`.** The strong references may have been dropped; `None` is a normal case.

18. **Do not implement `Deref` for mere conversion.** Use `AsRef`, `AsMut`, or `Borrow` for explicit conversions; reserve `Deref` for transparent pointer-like behavior.

19. **Never create `&mut T` by casting `&UnsafeCell<T>` directly.** Source the raw pointer from `UnsafeCell::get` or `UnsafeCell::raw_get`. Full UB rules are in [`docs/rust/unsafe-security.md`](unsafe-security.md).

20. **Never call `assume_init` on uninitialized `MaybeUninit<T>`.** This is immediate UB even for `i32` or `bool`. Full UB rules are in [`docs/rust/unsafe-security.md`](unsafe-security.md).

## Review checklist

- [ ] Is `Box<T>` used where exclusive heap ownership suffices? (No unnecessary `Rc`/`Arc`.)
- [ ] Is `Rc<T>` used only in single-threaded contexts? (No `Rc` in `Send` types or near `spawn`.)
- [ ] Is `Arc<T>` paired with `Mutex<T>`, `RwLock<T>`, or atomics when mutation is needed across threads? (Not `Arc<RefCell<T>>`.)
- [ ] Are reference cycles broken with `Weak<T>`? (Check parent-child, graph, and observer patterns.)
- [ ] Is `RefCell<T>` used only in single-threaded contexts? (It is `!Sync`.)
- [ ] Are `RefCell` borrows scoped as tightly as possible? (Long-lived `Ref`/`RefMut` values increase panic risk.)
- [ ] Is `Cell<T>` used for `Copy`-type interior mutability instead of `RefCell<T>`?
- [ ] Is `Mutex<T>` used instead of `RwLock<T>` unless read-heavy concurrency is proven?
- [ ] Are `MutexGuard` and `RwLock` guards dropped before any operation that might re-lock or `.await`?
- [ ] Is `OnceLock<T>` (not `OnceCell<T>`) used for statics and multi-threaded lazy initialization?
- [ ] Is `Cow<T>` used where clone-on-write semantics are appropriate? (Avoid unnecessary clones.)
- [ ] Are references (`&T`) preferred over smart pointers where lifetimes are statically provable?
- [ ] Does any custom `Drop` implementation avoid panicking? (Check `std::thread::panicking()` if it might.)
- [ ] Is the order `Arc<Mutex<T>>` (not `Mutex<Arc<T>>`) for shared mutable state across threads?
- [ ] Is `Deref` coercion relied on correctly without ambiguous cases?
- [ ] Are unsafe primitives (`UnsafeCell`, `ManuallyDrop`, `MaybeUninit`) accompanied by `// SAFETY:` comments and reviewed against [`docs/rust/unsafe-security.md`](unsafe-security.md)?

## Implementation checklist

- [ ] Identify whether the value needs heap allocation (dynamic size, ownership transfer, explicit lifetime).
- [ ] Determine ownership model: exclusive (`Box`), shared single-threaded (`Rc`), or shared multi-threaded (`Arc`).
- [ ] Determine mutability model: exterior (default), interior single-threaded (`Cell`/`RefCell`), or interior multi-threaded (`Mutex`/`RwLock`).
- [ ] If using `Rc` or `Arc`, check for reference cycles and add `Weak` back-references where needed.
- [ ] If using `RefCell`, minimize the scope of `borrow()` and `borrow_mut()` calls; prefer `try_borrow`/`try_borrow_mut` at API boundaries.
- [ ] If using `Mutex`, ensure lock guards are dropped before any potential re-lock or async yield point.
- [ ] If using `RwLock`, verify that writer starvation and platform-dependent priority are acceptable.
- [ ] If using `OnceLock`, confirm that `T: Send + Sync` and that the initialization closure does not panic in a way that leaves the system inconsistent.
- [ ] If using `Cow`, verify that the borrowed variant is used in the common path and cloning only occurs on mutation.
- [ ] If implementing `Drop`, ensure the destructor cannot panic and that it releases all owned resources.
- [ ] If using `Box::leak`, ensure there is a documented plan to either accept the leak or recover via `Box::from_raw`.
- [ ] Add documentation comments explaining why a particular smart pointer was chosen, especially for non-obvious choices like `RefCell`, `RwLock`, or `OnceLock`.

## Validation hooks

- **Compile-time `Send`/`Sync` enforcement.** The compiler rejects `Rc<T>`, `RefCell<T>`, and `Cell<T>` in contexts that require thread safety. Treat these errors as correctness signals, not obstacles to bypass with unsafe code.
- **Runtime borrow panics.** `RefCell::borrow_mut` panics on overlapping borrows. In tests, exercise all code paths that call `borrow_mut` to ensure no panic occurs under expected usage patterns.
- **Clippy lints.** Enable and heed:
  - `clippy::rc_mutex` — warns on `Rc<Mutex<T>>`, which is usually a mistake.
  - `clippy::mutex_atomic` — suggests atomics for simple flags under a `Mutex`.
  - `clippy::arc_with_non_send_sync` — warns when `Arc` wraps a non-`Send`/`Sync` type.
  - `clippy::rc_buffer` — warns on reference-counted `[T]` buffers that are often better as `Vec` or `Arc<str>`.
  - `clippy::await_holding_lock` — detects guards held across `.await` points.
- **Miri.** Run tests under Miri to detect undefined behavior in unsafe code that interacts with smart pointers (e.g., transmuting `Rc` to `Arc`, incorrect `Box::from_raw` usage, or unsound `UnsafeCell` access).
- **ThreadSanitizer.** For `Arc<Mutex<T>>` and `Arc<RwLock<T>>` patterns, run under ThreadSanitizer to detect data races and deadlocks.
- **Loom.** For low-level concurrent data structures built on `UnsafeCell` or atomics, consider model checking with `loom` to exhaustively explore thread interleavings.

## Examples

### Example 1: `Box<T>` for recursive types

```rust
// CORRECT: Box provides a known-size indirection, making the recursive enum representable.
enum List {
    Cons(i32, Box<List>),
    Nil,
}

let list = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));

// INCORRECT: the compiler cannot determine the size of List.
// enum List { Cons(i32, List), Nil }
```

### Example 2: `Box<dyn Trait>` for heterogeneous trait objects

```rust
// CORRECT: dyn Trait is unsized, so it must live behind a pointer.
trait Draw {
    fn draw(&self);
}

struct Screen {
    components: Vec<Box<dyn Draw>>,
}

// Each component can be a different concrete type. Box<dyn Draw> provides
// dynamic dispatch with a single vtable pointer. Prefer generics when the
// concrete type is known at compile time; see docs/rust/types-traits-generics.md.
```

### Example 3: `Rc<RefCell<T>>` tree with `Weak` back-references

```rust
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    value: i32,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,  // Weak breaks the reference cycle
}

let child = Rc::new(Node {
    value: 1,
    children: RefCell::new(vec![]),
    parent: RefCell::new(Weak::new()),
});

let parent = Rc::new(Node {
    value: 0,
    children: RefCell::new(vec![Rc::clone(&child)]),
    parent: RefCell::new(Weak::new()),
});

*child.parent.borrow_mut() = Rc::downgrade(&parent);

// Dropping parent drops children because the back-edge is Weak, not Rc.

// INCORRECT (would leak):
// parent: RefCell<Rc<Node>>  // strong cycle parent <-> child
```

### Example 4: `RefCell<T>` for interior mutability through a shared reference

```rust
use std::cell::RefCell;

struct Logger {
    entries: RefCell<Vec<String>>,
}

impl Logger {
    fn log(&self, message: &str) {
        // We need &mut access to entries, but log takes &self.
        // RefCell moves the borrow check to runtime.
        self.entries.borrow_mut().push(message.to_string());
    }

    fn entries(&self) -> Vec<String> {
        self.entries.borrow().clone()
    }
}

// CORRECT: RefCell allows mutation through a shared reference.
// borrow_mut() panics if entries() still holds a Ref<Vec<String>> —
// that is the runtime enforcement of Rust's aliasing rules.
```

### Example 5: `Arc<Mutex<T>>` for shared mutable state across threads

```rust
use std::sync::{Arc, Mutex};
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter_clone = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        let mut num = counter_clone.lock().unwrap();
        *num += 1;
        // MutexGuard is dropped here, releasing the lock.
    }));
}

for handle in handles {
    handle.join().unwrap();
}

assert_eq!(*counter.lock().unwrap(), 10);

// CORRECT: Arc provides shared ownership across threads; Mutex provides
// synchronized interior mutability. The order Arc<Mutex<T>> is essential.

// INCORRECT:
// Mutex<Arc<T>> only locks the pointer, not the data.
```

### Example 6: `Cell<T>` for zero-cost interior mutability on `Copy` types

```rust
use std::cell::Cell;

struct Counter {
    value: Cell<u32>,
}

impl Counter {
    fn increment(&self) {
        let current = self.value.get();
        self.value.set(current + 1);
    }
}

// CORRECT: Cell<T> is zero-cost interior mutability for Copy types.
// No runtime borrow checking, no panics, no locks.

// INCORRECT: RefCell<u32> adds unnecessary overhead and panic risk.
```

### Example 7: `Cow<'a, str>` for clone-on-write string normalization

```rust
use std::borrow::Cow;

fn normalize(input: Cow<'_, str>) -> Cow<'_, str> {
    if input.contains(char::is_uppercase) {
        // Mutation needed: Cow clones and returns the owned variant.
        Cow::Owned(input.into_owned().to_lowercase())
    } else {
        // No mutation needed: return the borrowed variant as-is.
        input
    }
}

let borrowed = Cow::Borrowed("hello");
let owned = Cow::Borrowed("Hello");

assert!(matches!(normalize(borrowed), Cow::Borrowed(_)));
assert!(matches!(normalize(owned), Cow::Owned(_)));

// CORRECT: Cow defers allocation until mutation is required.
```

### Example 8: `Deref` coercion chain

```rust
use std::ops::Deref;

struct MyBox<T>(T);

impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        MyBox(x)
    }
}

impl<T> Deref for MyBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn hello(name: &str) {
    println!("Hello, {name}!");
}

let x = MyBox::new(String::from("Rust"));
hello(&x); // &MyBox<String> -> &String -> &str via Deref coercion.

// CORRECT: Deref coercion chains automatically at compile time.
// Explicit dereferencing like &*x is needed only when the target type is ambiguous.
```

### Example 9: `OnceLock<T>` for a lazy static

```rust
use std::sync::OnceLock;

static CONFIG: OnceLock<String> = OnceLock::new();

fn config() -> &'static String {
    CONFIG.get_or_init(|| {
        // This closure runs at most once, even across many threads.
        "production".to_string()
    })
}

assert_eq!(config(), "production");

// CORRECT: OnceLock is Sync for Send + Sync T and is safe in statics.
```

### Example 10: `Rc::new_cyclic` for a self-referential weak pointer

```rust
use std::rc::{Rc, Weak};

struct Handle {
    self_ref: Weak<Handle>,
    value: i32,
}

let handle = Rc::new_cyclic(|weak| Handle {
    self_ref: weak.clone(),
    value: 42,
});

assert_eq!(handle.value, 42);
assert!(std::rc::Weak::ptr_eq(&handle.self_ref, &Rc::downgrade(&handle)));

// CORRECT: new_cyclic gives the closure a Weak<T> before the Rc<T> exists,
// solving the chicken-and-egg problem of self-referential shared ownership.
```

### Example 11: `Arc::make_mut` for clone-on-write across threads

```rust
use std::sync::Arc;

let mut data = Arc::new(vec![1, 2, 3]);

// If strong count is 1, no clone occurs; if shared, Arc clones for us.
Arc::make_mut(&mut data).push(4);

assert_eq!(data.as_ref(), &[1, 2, 3, 4]);

// CORRECT: Arc::make_mut gives &mut T when you hold &mut Arc<T>, cloning
// only when necessary.
```

### Example 12: `std::mem::drop` for early destruction

```rust
use std::mem::drop;

let s = String::from("clean up early");
drop(s);
// s is no longer valid; its destructor ran immediately.

// CORRECT: mem::drop moves the value in and lets it fall out of scope.
// You cannot call s.drop() directly — that is E0040.
```

## Common mistakes

### 1. Using `Rc<T>` where `Arc<T>` is needed for threads

`Rc<T>` does not implement `Send`. If you try to send an `Rc<T>` across a thread boundary, the compiler rejects it. Do not work around this with unsafe code or by wrapping `Rc` in `Mutex`. Use `Arc<T>` instead.

### 2. Creating reference cycles with `Rc` or `Arc`

When two strong pointers point at each other, both reference counts stay above zero and the values leak. Use `Weak<T>` for back-references, typically child→parent. `Weak::upgrade()` returns `Option<_>`; handle `None`.

### 3. Using `Arc<RefCell<T>>` or `Mutex<Arc<T>>`

`Arc<RefCell<T>>` does not compile into useful shared-mutable state across threads because `RefCell<T>` is `!Sync`. `Mutex<Arc<T>>` locks only the pointer; after cloning the inner `Arc` and releasing the lock, the data is shared unsynchronized. The correct ordering is `Arc<Mutex<T>>`.

### 4. Using `Arc::try_unwrap(this).ok()` concurrently

Two threads can both decide they are not the last owner, both discard the `Err`, and both drop the value. This can cause use-after-free. Use `Arc::into_inner` instead.

### 5. Holding `RefCell` borrows across function boundaries

Storing `Ref<T>` or `RefMut<T>` in a struct field or returning it from a function extends the borrow lifetime and increases the risk of a panic. Scope borrows tightly; clone the value out if you need it longer.

### 6. Using `RefCell<T>` for `Copy` types

`RefCell<u32>`, `RefCell<bool>`, and similar patterns add unnecessary runtime borrow checking. Use `Cell<T>` instead; it is zero-cost and panic-free for `Copy` types.

### 7. Holding a `std::sync::MutexGuard` across `.await` or re-locking the same mutex

In async code, holding a guard across `.await` can deadlock because the runtime may switch tasks. In sync code, calling `lock()` on a mutex already held by the same thread is not reentrant and may deadlock. Drop the guard before yielding or re-locking. See [`docs/rust/async-tokio.md`](async-tokio.md).

### 8. Using `RwLock<T>` prematurely

`RwLock<T>` has higher per-operation overhead than `Mutex<T>` and platform-dependent priority that can starve writers. Use `Mutex` by default; switch to `RwLock` only when profiling shows read concurrency is a bottleneck.

### 9. Being surprised that `MutexGuard` is `!Send`

`MutexGuard` intentionally does not implement `Send` to maximize portability: on pthreads platforms the lock must be released on the thread that acquired it. Do not attempt to move a guard to another thread.

### 10. Relying on `Mutex` poisoning for soundness in unsafe code

Poisoning is advisory. A poisoned mutex may still contain a valid value, and safe code can recover it with `poisoned.into_inner()`. Unsafe code cannot rely on poisoning to guarantee any invariant.

### 11. Implementing `Deref` for mere conversion

`Deref` is for transparent pointer-like behavior, not for exposing wrapped methods or performing conversions. Use `AsRef`, `AsMut`, or `Borrow` for explicit conversions. See [`docs/rust/types-traits-generics.md`](types-traits-generics.md).

### 12. Panicking in `Drop::drop`

If `Drop::drop` panics while the thread is already unwinding from another panic, the process aborts. If cleanup might fail, provide a separate `close()` or `shutdown()` method and call it explicitly. If you must log at drop, check `std::thread::panicking()` first.

### 13. Implementing both `Copy` and `Drop`

Rust forbids this combination (E0184/E0204) because it makes destructor timing unpredictable. If a type needs custom cleanup, it cannot be `Copy`.

### 14. Believing `Arc<T>` makes the inner `T` thread-safe

`Arc<T>` only makes *multiple ownership* of `T` thread-safe. It does not make `T` itself thread-safe. Pair `Arc` with `Mutex`, `RwLock`, or atomics when mutation is needed across threads.

### 15. Assuming `Weak::upgrade()` returns `Some`

`Weak` does not keep the value alive. The strong references may be gone by the time you call `upgrade`. Always match the `Option` and handle `None` gracefully.

### 16. Using `Box::leak` casually

`Box::leak` intentionally leaks memory forever. Use it only when you genuinely need a `&'static mut T`. You can recover ownership with `Box::from_raw`, but that is unsafe and easy to get wrong.

### 17. Calling `Box::from_raw` twice on the same pointer

`Box::from_raw` takes ownership of the pointer. Calling it twice on the same address is a double-free. The pointer must also come from the global allocator.

### 18. Constructing `&mut T` by casting `&UnsafeCell<T>`

Always obtain the raw pointer through `UnsafeCell::get` or `UnsafeCell::raw_get`. Casting a shared reference directly to `&mut T` is undefined behavior. See [`docs/rust/unsafe-security.md`](unsafe-security.md).

### 19. Calling `assume_init` on uninitialized `MaybeUninit<T>`

`MaybeUninit::assume_init` on uninitialized memory is immediate UB even for plain scalar types like `i32` or `bool`. Only call it after the memory has been properly initialized, e.g. via `write` or a safe constructor. See [`docs/rust/unsafe-security.md`](unsafe-security.md).

### 20. Dropping a `ManuallyDrop<T>` containing a `Box` and then moving the wrapper

The docs explicitly call this undefined behavior: if `T` is or contains a `Box`, dropping the inner value and then moving the `ManuallyDrop<T>` is UB. If you need to suppress destructors, prefer `MaybeUninit` for new code and carefully document the ownership transfer. See [`docs/rust/unsafe-security.md`](unsafe-security.md).

## Strict vs contextual guidance

| Guidance | Strictness | Rationale |
|---|---|---|
| Never use `Rc<T>` where `Arc<T>` is needed for thread safety | **Strict** | `Rc<T>` is `!Send`; using it across threads is a compile error or, with unsafe, undefined behavior. |
| Always use `Weak<T>` to break `Rc`/`Arc` cycles | **Strict** | Reference cycles are memory leaks; there is no valid use case for a deliberate strong cycle. |
| Always use `Arc<Mutex<T>>` (not `Mutex<Arc<T>>`) for shared mutable state | **Strict** | The reverse ordering does not synchronize access to the data. |
| Never panic in `Drop::drop` | **Strict** | A panic during unwinding causes a double-panic abort and violates exception safety. |
| Prefer `Cell<T>` over `RefCell<T>` for `Copy` types | **Strict** | `Cell` is zero-cost and panic-free; `RefCell` adds overhead and panic risk with no benefit for `Copy` types. |
| Prefer `Box<T>` as the default smart pointer | **Strict** | `Box` is the simplest, cheapest heap pointer. Using `Rc`/`Arc` without shared ownership is misleading and wasteful. |
| Prefer references over smart pointers when lifetimes are clear | **Strict** | References are zero-cost and communicate borrowing intent; smart pointers add allocation and indirection. |
| Scope `RefCell` borrows tightly | **Strict** | Long-lived borrows increase panic risk and make APIs harder to reason about. |
| Drop `MutexGuard`/`RwLock` guards before re-locking or yielding | **Strict** | Holding a guard across a re-lock or `.await` causes deadlock. |
| Use `OnceLock<T>` (not `OnceCell<T>`) for statics and shared multi-threaded lazy init | **Strict** | `OnceCell` is `!Sync` and cannot be used safely in statics or across threads. |
| Never create `&mut T` by casting `&UnsafeCell<T>` directly | **Strict** | This is immediate undefined behavior; always use `UnsafeCell::get`/`raw_get`. |
| Prefer generics over `Box<dyn Trait>` when the type is known | **Contextual** | Dynamic dispatch is sometimes the right trade-off for code size, compile time, API boundaries, or heterogeneous collections. |
| Use `RwLock<T>` instead of `Mutex<T>` for read-heavy workloads | **Contextual** | `RwLock` has higher per-operation overhead and platform-dependent priority. Use only when profiling justifies it. |
| Use `Cow<'a, B>` for clone-on-write patterns | **Contextual** | `Cow` adds complexity. Use it when clone avoidance is clear; otherwise, just clone. |
| Use `ManuallyDrop<T>` or `MaybeUninit<T>` only when necessary | **Contextual** | Both are unsafe primitives. Prefer safe ordering or `MaybeUninit` over `ManuallyDrop` for new code. |

## Policy decisions for individual repos

Each repository should make and document the following decisions in its contributing guidelines or architecture documentation:

1. **`Arc<T>` vs `Rc<T>` default.** If the codebase is entirely single-threaded, `Rc` may be the default. If any module uses threads or async runtimes, `Arc` should be the default for any type that might be shared. Document the threshold — for example, "Use `Arc` for all types in the `api` module because it may be used in a multi-threaded server context."

2. **Interior-mutability policy.** Decide which interior-mutability primitives are acceptable. Some repos ban `RefCell` in favor of `Cell` and thread-safe primitives; others allow `RefCell` but require a comment explaining why exterior mutability is insufficient.

3. **`Box<dyn Trait>` vs generics threshold.** At what point does the codebase switch from generics to trait objects? A common rule is "Use generics for all internal types; use `Box<dyn Trait>` only at API boundaries." Document this to prevent inconsistent choices.

4. **`RwLock<T>` vs `Mutex<T>` policy.** Some repos standardize on `Mutex` unless a benchmark justifies `RwLock`. Others default to `RwLock` for read-heavy caches. Document the default and the exception process.

5. **`Cow<'a, B>` usage policy.** Is `Cow` encouraged for string and slice processing? Some repos find it adds more complexity than it saves; others use it extensively in parsing and transformation code.

6. **Custom `Drop` policy.** What cleanup patterns are acceptable? Some repos require that all resource-holding types implement `Drop` and that `Drop` never fail. Others use a `close()` pattern with a `Drop` implementation that logs a warning if `close()` was not called. See [`docs/rust/error-handling.md`](error-handling.md) for panic interaction guidance.

7. **Smart-pointer documentation requirement.** Must every use of `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`, `OnceLock`, or `Cow` include a comment explaining why it was chosen? Some repos require this to prevent cargo-cult copying of patterns.

## Related docs

- [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) — ownership rules, move/Copy, borrow rules, NLL, lifetimes, variance, `Pin`/self-referential structs, and Drop-order basics (it includes "Example 7: Drop order").
- [`docs/rust/types-traits-generics.md`](types-traits-generics.md) — `dyn Trait`/object safety, `From`/`Into`/`AsRef`/`TryFrom`, and the common mistake of using `Deref` for conversion.
- [`docs/rust/async-tokio.md`](async-tokio.md) — std `Mutex` vs `tokio::sync::Mutex`, and the rule against holding guards across `.await`.
- [`docs/rust/unsafe-security.md`](unsafe-security.md) — full UB rules for `MaybeUninit<T>`, `ManuallyDrop<T>`, `UnsafeCell<T>`, `transmute`, `mem::zeroed`, and raw pointer aliasing.
- [`docs/rust/error-handling.md`](error-handling.md) — panic semantics and the interaction between panics and `Drop` destructors.
- [`docs/rust/std-runtime-apis.md`](std-runtime-apis.md) — synchronous runtime APIs that smart pointers often protect or own.
- [The Rust Book, Chapter 15: Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html)
- [The Rust Book, Chapter 16: Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)

## Related skills

No specific Rust smart-pointer skills are currently defined. Agents should apply the rules, checklists, and decision matrix in this document directly when writing or reviewing Rust code. If a Rust-specific skill is created in the future (for example, `rust-memory-patterns`), it should reference this document as its authoritative source.
