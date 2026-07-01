# Unsafe Rust and Security Boundaries

## Purpose

This document provides practical, opinionated guidance on `unsafe` Rust for future AI coding agents. It covers what `unsafe` enables, the two roles of the `unsafe` keyword, the complete catalog of undefined behavior, safety invariants, FFI and ABI patterns, raw pointers and provenance, uninitialized memory, `Send`/`Sync` manual implementations, `transmute`, atomics, `Pin`, review and implementation checklists, and validation tooling. The goal is to help agents write, review, and refactor unsafe code that is sound, minimal, and auditable.

This doc records the verified default for `unsafe_op_in_unsafe_fn`: it is **allow-by-default in editions 2015/2018/2021** and **warn-by-default in the 2024 edition** (an edition-specific default via `edition_lint_opts`). Teams should consider enabling it explicitly (`warn` or `deny`) in all editions. See the "2024 edition changes" section and [The Rust Edition Guide — unsafe_op_in_unsafe_fn](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html).

## Sources used

Primary references from the Rust Reference and Nomicon:

- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [The Rust Reference: The `unsafe` keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html)
- [The Rust Reference: Unsafe functions](https://doc.rust-lang.org/reference/unsafe-functions.html) (redirects to `unsafe-keyword.html`)
- [The Rust Reference: Unsafe blocks](https://doc.rust-lang.org/reference/unsafe-blocks.html) (redirects to `unsafe-keyword.html`)
- [The Rust Reference: Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
- [The Rust Reference: Behavior not considered unsafe](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html)
- [The Rust Reference: External blocks](https://doc.rust-lang.org/reference/items/external-blocks.html)
- [The Rust Reference: ABI and FFI](https://doc.rust-lang.org/reference/abi.html)
- [The Rust Reference: Attributes](https://doc.rust-lang.org/reference/attributes.html)
- [The Rust Reference: Unsafety](https://doc.rust-lang.org/reference/unsafety.html)
- [The Rust Reference: Subtyping and variance](https://doc.rust-lang.org/reference/subtyping.html)
- [The Rust Reference: Moved and copied types](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)
- [The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [The Rust Unstable Book: Sanitizer](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html)
- [Rustc Lints: Allowed-by-default lints](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html)

Standard library docs:

- [`std::marker::Send`](https://doc.rust-lang.org/std/marker/trait.Send.html)
- [`std::marker::Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- [`std::primitive::pointer`](https://doc.rust-lang.org/std/primitive.pointer.html)
- [`std::ptr`](https://doc.rust-lang.org/std/ptr/index.html)
- [`std::ptr::NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html)
- [`std::mem::MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)
- [`std::mem::transmute`](https://doc.rust-lang.org/std/mem/fn.transmute.html)
- [`std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html)
- [`std::sync::atomic::Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)
- [`std::pin`](https://doc.rust-lang.org/std/pin/index.html)

Nomicon chapters:

- [The Rustonomicon: Safe and unsafe meaning](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)
- [The Rustonomicon: Working with unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)
- [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [The Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [The Rustonomicon: Transmutes](https://doc.rust-lang.org/nomicon/transmutes.html)
- [The Rustonomicon: Subtyping](https://doc.rust-lang.org/nomicon/subtyping.html)
- [The Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [The Rustonomicon: Vec push/pop](https://doc.rust-lang.org/nomicon/vec-push-pop.html)
- [The Rustonomicon: Vec into-iter](https://doc.rust-lang.org/nomicon/vec-into-iter.html)
- [The Rustonomicon: Vec drain](https://doc.rust-lang.org/nomicon/vec-drain.html)
- [The Rustonomicon: Arc and Mutex](https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html)
- [The Rustonomicon: Leaking](https://doc.rust-lang.org/nomicon/leaking.html)
- [The Rustonomicon: Drop check](https://doc.rust-lang.org/nomicon/dropck.html)
- [The Rustonomicon: Exception safety](https://doc.rust-lang.org/nomicon/exception-safety.html)
- [The Rustonomicon: Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html)
- [The Rustonomicon: Unbounded lifetimes](https://doc.rust-lang.org/nomicon/unbounded-lifetimes.html)
- [The Rustonomicon: Casts](https://doc.rust-lang.org/nomicon/casts.html)

Tooling and crate docs:

- [Miri (GitHub)](https://github.com/rust-lang/miri)
- [cargo-careful (GitHub)](https://github.com/RalfJung/cargo-careful)
- [Loom (GitHub)](https://github.com/tokio-rs/loom)
- [Shuttle (GitHub)](https://github.com/awslabs/shuttle)
- [cargo-geiger (GitHub)](https://github.com/geiger-rs/cargo-geiger)
- [bytemuck docs](https://docs.rs/bytemuck/latest/bytemuck/)
- [zerocopy docs](https://docs.rs/zerocopy/latest/zerocopy/)

Related internal docs are listed in [Related docs](#related-docs) below.

Claims below are inline-cited to the specific URLs above.

## Core guidance

### What `unsafe` permits

The `unsafe` keyword does not disable the borrow checker or turn off safety guarantees. It grants access to operations the compiler cannot verify on its own ([The Rust Reference: Unsafety](https://doc.rust-lang.org/reference/unsafety.html)):

1. **Dereferencing a raw pointer** — `*const T` and `*mut T`.
2. **Reading or writing a mutable or unsafe external static variable** — `static mut` and `extern static`.
3. **Accessing a field of a `union`**, other than to assign to it.
4. **Calling an unsafe function** — items declared `unsafe fn`.
5. **Calling a safe function marked with `target_feature`** from a function that does not have a matching `target_feature`.
6. **Implementing an unsafe trait** — traits declared `unsafe trait`.
7. **Declaring an `extern` block** — `unsafe extern "C" { ... }`.
8. **Applying an unsafe attribute to an item** — `#[unsafe(no_mangle)]`, `#[unsafe(export_name = "...")]`, `#[unsafe(link_section = "...")]`, `#[unsafe(naked)]`.

Note: inline assembly (`asm!`, `naked_asm!`, `global_asm!`) also requires `unsafe`. The complete list of unsafe attributes is `export_name`, `link_section`, `naked`, and `no_mangle` ([The Rust Reference: Attributes](https://doc.rust-lang.org/reference/attributes.html)).

Every other Rust feature — borrowing rules, type system invariants, overflow checks — remains in effect inside an `unsafe` block. The keyword is a contract: you, the programmer, assert that the enclosed operations are sound despite the compiler being unable to prove it.

### The two roles of `unsafe`

The `unsafe` keyword has two distinct roles ([The Rust Reference: The `unsafe` keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html)):

- **It defines extra safety conditions.** `unsafe fn`, `unsafe trait`, `unsafe extern` blocks, and unsafe extern statics impose proof obligations on the caller or implementer.
- **It asserts conditions are met.** `unsafe {}` blocks, `unsafe impl`, `unsafe extern` (asserting signatures are correct), and `#[unsafe(attr)]` discharge those obligations.

Duality: `unsafe fn`/`unsafe trait` = define proof obligation; `unsafe {}`/`unsafe impl` = discharge proof obligation.

By default, the body of an `unsafe fn` is also considered an unsafe block. This means unsafe operations inside an `unsafe fn` do not require an additional `unsafe {}` unless the `unsafe_op_in_unsafe_fn` lint is active. That lint is allow-by-default in editions 2015/2018/2021 and warn-by-default in the 2024 edition; see the "2024 edition changes" section below for details.

```rust
/// # Safety
///
/// `ptr` must point to a valid, aligned, initialized `T` that remains
/// valid for the duration of the returned reference.
unsafe fn deref_ptr<T>(ptr: *const T) -> &T {
    // Inside an unsafe fn, unsafe operations are allowed without
    // an additional unsafe block unless unsafe_op_in_unsafe_fn is enabled.
    &*ptr
}

fn caller() {
    let x: i32 = 42;
    let ptr: *const i32 = &raw const x;
    // SAFETY: ptr was just derived from a valid reference to x,
    // which is still alive for the duration of this scope.
    let val = unsafe { deref_ptr(ptr) };
    assert_eq!(val, &42);
}
```

`unsafe trait` declares that implementing the trait requires upholding a safety contract beyond what the type system can check. `unsafe impl` asserts the implementor has verified that contract ([The Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)). The canonical examples are `Send` and `Sync`.

### The soundness property

The foundational invariant of Rust is: "No matter what, Safe Rust can't cause Undefined Behavior" ([The Rustonomicon: Safe and unsafe meaning](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)). A function is **sound** if "safe code cannot cause Undefined Behavior through it" ([The Rustonomicon: Working with unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)).

Key consequences:

- **Soundness is non-local.** Editing only safe code can break an unsafe invariant. For example, changing `<` to `<=` in a bounds check can introduce UB in an unsafe abstraction that relied on that check.
- **Asymmetric trust.** Safe Rust must trust all unsafe code it touches. Unsafe Rust cannot blindly trust generic safe code — the classic example is a malicious `Ord` implementation fed to `BTreeMap` breaking an unsafe abstraction.
- **Privacy is the only bulletproof blast-radius limiter.** "The only bullet-proof way to limit the scope of unsafe code is at the module boundary with privacy" ([The Rustonomicon: Working with unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)). Public unsafe invariants can be violated by any downstream crate.

There are three stable unsafe traits in the standard library: `Send`, `Sync`, and `GlobalAlloc` ([The Rustonomicon: Safe and unsafe meaning](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)).

### Safety invariants

Every `unsafe` block must be accompanied by a `// SAFETY:` comment explaining why the operation is sound. This is not optional — it is the primary mechanism for auditing unsafe code.

The `// SAFETY:` comment must:

- State which preconditions are being satisfied.
- Explain aliasing, lifetime, and initialization assumptions.
- Reference any invariants maintained by the enclosing module or type.
- Be specific to the exact unsafe operation, not a vague assertion.

Format:

```rust
// SAFETY: <specific reasoning for why this is sound>
unsafe { ... }
```

For `unsafe impl`, use the same `// SAFETY:` convention in a doc comment or inline comment above the impl. Every `unsafe fn` must have a `# Safety` doc section describing the preconditions callers must uphold.

### Undefined behavior — complete catalog

Undefined behavior (UB) is any behavior the compiler is free to assume will not happen. When UB occurs, the compiler may miscompile, optimize away code, or produce silently wrong results. The authoritative list is at [The Rust Reference: Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).

> Rust code is incorrect if it exhibits any of the behaviors in the following list. This includes code within `unsafe` blocks and `unsafe` functions. `unsafe` only means that avoiding undefined behavior is on the programmer.

The list is non-exhaustive. Reproduced below with the Reference's anchor IDs:

- **r-undefined.race — Data races.** Two or more threads accessing the same memory location concurrently, where at least one access is a write, and no synchronization orders the accesses.
- **r-undefined.pointer-access — Accessing a place that is dangling or based on a misaligned pointer.** A dangling pointer is one where not all pointed-to bytes are in the same live allocation. Size-0 pointers are trivially never dangling, even if null. A dynamic size must never exceed `isize::MAX`.
- **r-undefined.place-projection — Performing a place projection** (field/tuple-index/array-slice index) that violates in-bounds pointer arithmetic.
- **r-undefined.alias — Breaking the pointer aliasing rules.** `&T` must not be mutated while live, except inside an `UnsafeCell<U>`; `&mut T` must not be read/written by any pointer not derived from it, and no other reference may point to it; `Box<T>` is treated like `&'static mut T`. The exact rules are not yet finalized, so stay conservative.
- **r-undefined.immutable — Mutating immutable bytes.** Immutable bytes include bytes reachable through shared references transitively, const-promoted values, immutable statics/bindings, unless inside an `UnsafeCell`. A mutation is any write of >0 bytes overlapping, even if the content is unchanged.
- **r-undefined.intrinsic — Invoking UB via compiler intrinsics.**
- **r-undefined.target-feature — Executing code compiled with platform features the current platform does not support**, except where documented safe.
- **r-undefined.call — Calling a function with the wrong call ABI**, or unwinding past a stack frame that does not allow unwinding (for example, calling a `"C-unwind"` function imported or transmuted as `"C"`).
- **r-undefined.invalid — Producing an invalid value** by assigning, reading, passing, or returning it. Sub-rules:
  - `bool` must be `0` or `1`.
  - Function pointers must be non-null.
  - `char` must not be a surrogate (not in `0xD800..=0xDFFF`) and must be `<= char::MAX`.
  - The never type `!` must never exist.
  - `i*`/`u*`/`f*`/raw pointers must be initialized.
  - `str` (like `[u8]`) must be initialized and valid UTF-8.
  - Enums must have a valid discriminant and valid fields for that variant.
  - Structs/tuples/arrays need all fields valid.
  - References and `Box` must be aligned, non-null, non-dangling, and point to a valid value.
  - Wide references/Boxes/pointers must have valid vtable metadata or valid `usize` length metadata, and total size must be `<= isize::MAX`.
  - Custom-validity types (`NonNull`, `NonZero*`) must be in range.
  - Padding and unions are the only places uninitialized memory may be read.
  - In const contexts, transmuting a pointer to a non-pointer type is UB if the pointer had provenance.
- **r-undefined.asm — Incorrect use of inline assembly.**
- **r-undefined.runtime — Violating Rust runtime assumptions.** For example, using `longjmp` to deallocate a Rust stack frame without running destructors.

UB affects the **entire program** and crosses FFI boundaries. A single UB site invalidates the entire execution.

### Behavior not considered unsafe

The compiler does **not** consider the following to be unsafe ([The Rust Reference: Behavior not considered unsafe](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html)):

- Deadlocks.
- Leaks of memory or other resources.
- Exiting without calling destructors.
- Exposing randomized base addresses via pointer leaks.
- Integer overflow: this is defined behavior (panic in debug, wrapping two's-complement in release) but is still erroneous.
- Logic errors such as violating trait contracts (for example, inconsistent `Hash`/`Eq` implementations, mutating `BTreeMap` keys): behavior is unspecified but not UB.

These can still be serious bugs, but they are not undefined behavior and the compiler will not exploit them for optimizations.

### 2024 edition changes

The 2024 edition made several important changes to `unsafe` syntax. The most important change concerns the default level of `unsafe_op_in_unsafe_fn`.

1. **`unsafe extern` is now required.** Write `unsafe extern "C" { ... }`. In editions before 2024, the `unsafe` keyword on extern blocks was optional. The `missing_unsafe_on_extern` lint fires in earlier editions (allow by default) and becomes a hard error in 2024 ([The Rust Reference: External blocks](https://doc.rust-lang.org/reference/items/external-blocks.html), [The Rust Reference: The `unsafe` keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html)).

2. **Unsafe attributes must be wrapped.** Use `#[unsafe(no_mangle)]`, `#[unsafe(export_name = "...")]`, `#[unsafe(link_section = "...")]`, `#[unsafe(naked)]`. Pre-2024 the bare forms were allowed ([The Rust Reference: The `unsafe` keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html), [The Rust Reference: Attributes](https://doc.rust-lang.org/reference/attributes.html)).

3. **`unsafe_op_in_unsafe_fn` is warn-by-default in the 2024 edition.** It is allow-by-default in editions 2015/2018/2021 and warn-by-default in the 2024 edition, set via an edition-specific default (`edition_lint_opts`, i.e. `@edition Edition2024 => Warn;`). When the lint is active (`warn` or `deny`), every unsafe operation inside an `unsafe fn` body must be wrapped in its own `unsafe {}`. Recommend enabling it explicitly (`warn` or `deny`) in all editions. Source: [The Rust Edition Guide — unsafe_op_in_unsafe_fn](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html).

   > Caveat on the listing page: the rustc "Allowed-by-default Lints" page (https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html) shows this lint as `Allow` because the docs generator reads only the first level keyword and ignores the `@edition Edition2024 => Warn;` override. Do **not** cite that page as evidence that the 2024 default is allow. See [`editions-tooling.md`](editions-tooling.md) and [`lints-clippy.md`](lints-clippy.md) for the same conclusion.

   ```rust
   #![warn(unsafe_op_in_unsafe_fn)]
   // or, for stricter enforcement:
   #![deny(unsafe_op_in_unsafe_fn)]
   ```

4. **`safe` and `unsafe` function qualifiers inside extern blocks.** Inside an `unsafe extern` block, you can now mark individual foreign functions as `safe fn` if you have verified they are safe to call, or leave them as implicit `unsafe fn` ([The Rust Reference: External blocks](https://doc.rust-lang.org/reference/items/external-blocks.html), [The Rust Reference: ABI and FFI](https://doc.rust-lang.org/reference/abi.html)).

### FFI, ABI, and `no_mangle`

FFI is the most common legitimate use of `unsafe` in application code. See [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html) for the authoritative guide.

Foreign functions in `unsafe extern "C" {}` are implicitly `unsafe` unless qualified `safe`. They are unsafe "because C libraries often expose interfaces that aren't thread-safe, and almost any function that takes a pointer argument isn't valid for all possible inputs" ([The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)).

The **safe-wrapper sandwich**: wrap every C call in a safe Rust function that has removed `unsafe` from its signature, with the inner `unsafe {}` block expressing that the wrapper upholds the C function's preconditions.

ABI strings ([The Rust Reference: ABI and FFI](https://doc.rust-lang.org/reference/abi.html)):

- `"C"` — default when omitted for extern blocks; stable lingua franca.
- `"Rust"` — default for plain `fn`; no stability guarantees.
- `"system"` — equal to `"C"` except on Windows x86 32-bit non-variadic, where it is `"stdcall"`.
- `"C-unwind"`, `"system-unwind"` — allow unwinding across the boundary.
- Platform ABIs: `cdecl`, `stdcall`, `win64`, `sysv64`, `aapcs`, `fastcall`, `thiscall`, `efiapi` (most with `-unwind` variants).
- Variadic functions are only supported with: `aapcs`, `C`, `cdecl`, `efiapi`, `system`, `sysv64`, `win64` (plus their `-unwind` variants).

`-unwind` ABIs: a Rust panic across a non-unwind ABI safely aborts. A foreign exception entering Rust across a non-unwind ABI is UB. Interaction between `catch_unwind` and foreign exceptions is UB. Always catch panics at the FFI boundary, or use the matching `-unwind` ABI ([The Rust Reference: ABI and FFI](https://doc.rust-lang.org/reference/abi.html)).

`#[unsafe(no_mangle)]` (formerly `#[no_mangle]`): disables name mangling; required to export to C. It is unsafe because unmangled symbols may collide, which is UB. Use `crate-type = ["cdylib"]` for shared libraries ([The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)).

Layout compatibility: use `repr(C)`, `repr(C, packed)`, and `repr(transparent)` for FFI. Empty enums must not be used as FFI types because the compiler relies on their uninhabitedness. Field-less Rust enums used to model C `enum` are often wrong: a C enum can hold any integer, while a Rust enum only permits valid discriminants. Use the opaque struct idiom when the layout must remain private:

```rust
use std::marker::PhantomData;
use std::pin::PhantomPinned;

#[repr(C)]
pub struct Foo {
    _data: (),
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}
```

Nullable pointer optimization (niche) is guaranteed for enums with exactly two variants where one is data-less and the other holds a non-nullable type (`&T`, `Box<T>`, `NonNull<T>`, `fn`) ([The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)).

### `Send` and `Sync`

`Send` and `Sync` are unsafe auto traits. Incorrect implementations are UB.

Definitions ([`std::marker::Send`](https://doc.rust-lang.org/std/marker/trait.Send.html), [`std::marker::Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html), [The Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)):

- `pub unsafe auto trait Send {}` — types transferable across thread boundaries.
- `pub unsafe auto trait Sync {}` — "a type `T` is `Sync` if and only if `&T` is `Send`."

Auto-derivation: a struct is `Send` iff all fields are `Send`; `Sync` iff all fields are `Sync`.

Common exceptions:

- Raw pointers `*const T`/`*mut T` are `!Send + !Sync`.
- `NonNull<T>` is `!Send` and `!Sync` because the data may be aliased.
- `UnsafeCell<T>`/`Cell<T>`/`RefCell<T>` are `!Sync`.
- `Rc<T>` is `!Send + !Sync`.
- `MutexGuard` is `!Send` (pthread) but `Sync`.
- `Arc<T>` is `Send + Sync` iff `T: Send + Sync`.

Reference rules:

- `&T` is `Send` iff `T` is `Sync`.
- `&mut T` is `Send` iff `T` is `Send`.
- `&T` and `&mut T` are `Sync` iff `T` is `Sync`.

Opt out with negative impls (`impl !Send for MyType {}`) requires `#![feature(negative_impls)]` for explicit syntax, but `PhantomData<*const T>` gives `!Send + !Sync` on stable. See the [Examples](#examples) section for a concrete `unsafe impl Send/Sync` pattern.

### `transmute` and friends

`mem::transmute` reinterprets the bits of a value as a different type ([`std::mem::transmute`](https://doc.rust-lang.org/std/mem/fn.transmute.html), [The Rustonomicon: Transmutes](https://doc.rust-lang.org/nomicon/transmutes.html)).

Signature: `pub const unsafe fn transmute<Src, Dst>(src: Src) -> Dst`. Same size is required — compilation fails if the sizes are not guaranteed equal. It performs a bitwise copy; padding is not preserved. Both `Src` and `Dst` must be valid at their type or the result is UB.

Always-UB patterns:

- Transmuting `&T` to `&mut T` is always UB ("No you're not special").
- Transmuting to a reference produces an unbounded lifetime.
- Do not transmute `3` to `bool`.
- Layout of non-`repr(C)`/`repr(transparent)` types is undefined (`Vec<i32>` vs `Vec<u32>` may differ).

Pointer/integer transmute:

- Pointer to integer in a const context is UB unless the pointer was originally created from an integer.
- Integer to pointer transmute is largely unspecified; memory access through such a pointer is currently UB.
- Round-trip via `as` casts or `MaybeUninit<usize>`, not `transmute`.

Related functions:

- `mem::transmute_copy<T, U>`: copies `size_of::<U>()` bytes; UB if `U` is larger than `T` (reads past the source).
- `core::intrinsics::transmute_unchecked` (nightly): like `transmute` but size mismatch is runtime UB instead of a compile error.

Prefer safe alternatives: `from_ne_bytes`/`from_le_bytes`; `ptr as *const T as usize`; `&mut *ptr`; or crates like `bytemuck` and `zerocopy`.

### Raw pointers, provenance, and `NonNull`

A pointer semantically carries an **address** and **provenance** (permission to access memory). A pointer with no provenance may not be dereferenced. Offsetting across allocation boundaries is UB; `offset_from` of two non-co-derived pointers is UB ([`std::primitive::pointer`](https://doc.rust-lang.org/std/primitive.pointer.html), [`std::ptr`](https://doc.rust-lang.org/std/ptr/index.html)).

Strict provenance APIs (stable) ([`std::ptr`](https://doc.rust-lang.org/std/ptr/index.html)):

- `ptr::addr()` — discards provenance, does not expose it.
- `ptr::with_addr(addr)` — replace address while preserving provenance.
- `ptr::map_addr(f)` — map the address.
- `ptr::expose_provenance() -> usize` — expose provenance as an integer.
- `ptr::with_exposed_provenance(addr)` — reconstitute provenance; UB if no provenance justifies the use.

Prefer strict provenance; avoid integer-to-pointer and pointer-to-integer casts when possible.

Macros for raw pointers:

- `ptr::addr_of!(place)` / `ptr::addr_of_mut!(place)` — create raw pointers without creating an intermediate reference (needed for packed structs and uninitialized memory).
- Raw borrow operators: `&raw const expr` / `&raw mut expr` (stable) — create raw pointers to places that may be unaligned or uninitialized without UB.

Four ways to create a raw pointer:

1. Coerce from `&T`/`&mut T`.
2. `Box::into_raw` (pair with `Box::from_raw`).
3. `&raw const` / `&raw mut`.
4. From C via FFI.

`NonNull<T>` ([`std::ptr::NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html)):

- A `*mut T` guaranteed non-null and covariant in `T`.
- `!Send + !Sync`.
- Enables niche optimization for `Option<NonNull<T>>`.
- `NonNull::dangling()` is non-null and aligned but must not be treated as an initialized sentinel semantically.

Offset arithmetic: `add`/`offset` require the result to remain within (or one-past) the same allocation. Allocations are capped at `isize::MAX` bytes.

### `MaybeUninit`

`MaybeUninit<T>` is a union with `#[repr(transparent)]` — same size, alignment, and ABI as `T`. Its `Drop` never calls `T`'s drop ([`std::mem::MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)).

Core rule: the compiler assumes variables are valid per their type **always**, even in unsafe code. Therefore `mem::zeroed()` or `MaybeUninit::uninit().assume_init()` on a reference type is instantaneous UB even if never dereferenced.

`assume_init()` safety: the caller must guarantee the `MaybeUninit<T>` really is initialized and valid for `T`; calling it on uninitialized memory is immediate UB. This applies even to `i32`.

You cannot construct `&T` or `&mut T` to uninitialized data. Use `as_mut_ptr()` plus `ptr::write`, or `&raw mut (*ptr).field`.

Useful APIs:

- `uninit()` — uninitialized.
- `new(val)` — safely initialized.
- `zeroed()` — validity depends on `T`.
- `write(val)` — overwrite without dropping the old value.
- `as_ptr` / `as_mut_ptr`.
- `assume_init()` / `assume_init_read` / `assume_init_ref` / `assume_init_mut` / `assume_init_drop`.
- `array_assume_init` for arrays.

Use `ptr::write` (move, no drop), `ptr::copy` (memmove, handles overlap), and `ptr::copy_nonoverlapping` (memcpy, UB if overlap) for uninit-safe writes.

Layout caveat: `size_of::<Option<bool>>() == 1` but `size_of::<Option<MaybeUninit<bool>>>() == 2`.

See the [Examples](#examples) section for a stack-array initialization pattern.

### Atomics and memory ordering

Rust atomics currently follow the same rules as C++20 atomics, **without** the "consume" memory ordering ([`std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html), [`std::sync::atomic::Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)).

`Ordering` is `#[non_exhaustive]`:

```rust
enum Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
}
```

There is **no** `Consume` variant.

Semantics:

- **Relaxed** — no ordering, only atomicity.
- **Release** (store) — all previous operations are ordered before any Acquire-or-stronger load of this value.
- **Acquire** (load) — if the loaded value was Release-stored, all subsequent operations are ordered after that store. Note: Acquire on a read-modify-write does a **Relaxed** store.
- **AcqRel** — Acquire load plus Release store; never Relaxed.
- **SeqCst** — Acquire/Release/AcqRel plus a single global total order of all SeqCst operations visible to all threads.

Data races are UB: conflicting non-synchronized accesses where at least one is non-atomic. "It is literally impossible to write correct synchronized code using only data accesses" ([The Rustonomicon](https://doc.rust-lang.org/nomicon/)).

All atomic accesses on read-only memory are UB, except sufficiently small Relaxed loads (<= 4 bytes on 32-bit targets, <= 8 bytes on 64-bit targets).

A failed `compare_exchange` is **not** a write.

The compiler may reorder or elide data accesses; weakly-ordered hardware can reorder even atomic-cooperating sequences. Use the right `Ordering`.

### `Pin` and self-referential types

A value is **pinned** when guaranteed to remain at the same memory address from pinning until drop ([`std::pin`](https://doc.rust-lang.org/std/pin/index.html)). See also `docs/rust/async-tokio.md`.

Drop guarantee: "From the moment a value is pinned… that value must remain, valid, at that same address in memory, until its drop handler is called."

`Unpin` is an auto-trait that cancels `Pin`'s restrictions. `Pin<Box<T>>` where `T: Unpin` behaves like a regular `Box<T>`. Most types are `Unpin`. Make a type `!Unpin` with `PhantomPinned`.

When unsafe is required:

- `Pin::new(&mut x)` is safe iff `T: Unpin`.
- For `!Unpin` types, use `Pin::new_unchecked` inside `unsafe {}`.
- `Box::pin(x)` and the `pin!` macro are always safe.

Incorrect unsafe code creating a `Pin` that does not satisfy the invariants leads to UB even in subsequent safe code.

Drop impls of pinned types should treat `&mut self` as `Pin<&mut Self>` via `Pin::new_unchecked`.

`ManuallyDrop` trap: cannot soundly build `Pin<&mut T>` inside `Pin<Box<ManuallyDrop<T>>>` because it inhibits the destructor and violates the drop guarantee.

### The Nomicon as authoritative reference

The [Rustonomicon](https://doc.rust-lang.org/nomicon/) is the authoritative reference for unsafe Rust. Consult it when writing new `unsafe` abstractions, reviewing unsafe code for soundness, understanding the memory model or aliasing rules, or debugging potential UB. Use it alongside stdlib docs: the Nomicon explains *why* things are sound or unsound, while stdlib docs describe *what* the API does. When in doubt, the Nomicon takes precedence for soundness questions.

## Practical rules

1. Every `unsafe` block must begin with a `// SAFETY:` comment that explains why the operation is sound.
2. Every `unsafe fn` must have a `# Safety` doc section describing the preconditions callers must uphold.
3. Never use `mem::uninitialized()` or `mem::zeroed()` as a general initialization tool — use `MaybeUninit<T>` instead.
4. Never use `mem::transmute()` without a written justification and a comment explaining why no safe alternative works.
5. Do not let panics cross FFI boundaries — use `catch_unwind` or the matching `-unwind` ABI; catch panics at every exported boundary.
6. Minimize the number of operations inside each `unsafe` block — one logical operation per block when practical.
7. Encapsulate all unsafe operations behind safe public APIs — unsafe should be an implementation detail, not part of the interface.
8. Use `ptr::NonNull<T>` instead of `*mut T` when the pointer is guaranteed non-null.
9. Enable `#![deny(unsafe_op_in_unsafe_fn)]` at the crate level to require explicit `unsafe` blocks inside `unsafe fn`.
10. Run `cargo +nightly miri test` on any module that contains `unsafe` before merging.
11. Never implement `unsafe impl Send` or `unsafe impl Sync` without a `// SAFETY:` comment explaining why the type is safe to send or share.
12. Use `CString` for FFI string arguments — never pass Rust `&str` or `String` directly to C.
13. Verify pointer alignment before dereferencing — use `read_unaligned`/`write_unaligned` or strict provenance APIs when alignment is uncertain.
14. Do not create a reference (`&T` or `&mut T`) to uninitialized memory — use `MaybeUninit<T>` and `ptr::write` instead.
15. Audit every raw pointer for its provenance — know where it came from, what allocation it refers to, and that the allocation remains live; prefer strict provenance APIs over raw integer-to-pointer casts.
16. Use `unsafe extern "C" { ... }` in Rust 2024 and wrap unsafe attributes as `#[unsafe(attr)]`.
17. Use the correct ABI string and `-unwind` variant; do not mix `"C"` and `"C-unwind"` references to the same symbol.

## Review checklist

When reviewing a PR that contains `unsafe`:

- [ ] Does every `unsafe` block have a `// SAFETY:` comment with specific, correct reasoning?
- [ ] Does every `unsafe fn` have a `# Safety` doc section listing all preconditions?
- [ ] Are the safety preconditions actually sufficient — do they cover aliasing, lifetimes, alignment, initialization, and thread safety?
- [ ] Is the unsafe surface area minimal — could any unsafe operations be replaced with safe alternatives (`bytemuck`, `zerocopy`, `crossbeam`, safe std APIs)?
- [ ] Are raw pointers derived from valid allocations that remain live for the entire duration of use?
- [ ] Could any code path create `&mut` and `&` references to the same data simultaneously?
- [ ] Are `MaybeUninit` values definitely initialized before `assume_init` is called?
- [ ] Do `unsafe impl Send` / `unsafe impl Sync` have `// SAFETY:` comments, and are the justifications correct?
- [ ] Could a panic propagate across an FFI boundary? Is `catch_unwind` or `extern "C-unwind"` used?
- [ ] Is `#![deny(unsafe_op_in_unsafe_fn)]` enabled, or is there a documented plan to enable it?
- [ ] Are extern blocks written as `unsafe extern "C" { ... }` (Rust 2024) or documented as pending migration?
- [ ] Are unsafe attributes written as `#[unsafe(no_mangle)]`, `#[unsafe(export_name)]`, `#[unsafe(link_section)]`, `#[unsafe(naked)]`?
- [ ] Has `cargo +nightly miri test` been run on the affected code?
- [ ] Are there tests that exercise the unsafe code paths, including edge cases (null, empty, max-size, zero-size)?
- [ ] Is `transmute` used? If so, is there a written justification for why no safe alternative works?
- [ ] Are pointer provenance and strict-provenance rules respected?
- [ ] Are atomics using the correct `Ordering` for the synchronization intent?
- [ ] Are `Pin` invariants upheld for `!Unpin` types?

## Implementation checklist

When writing new `unsafe` code:

- [ ] Identify the minimum set of operations that require `unsafe` — do not wrap safe code in unsafe blocks.
- [ ] Write the `// SAFETY:` comment before writing the unsafe block — if you cannot articulate why it is safe, it probably is not.
- [ ] Document all preconditions of `unsafe fn` in a `# Safety` doc section.
- [ ] Ensure raw pointers are derived from valid, live allocations with correct alignment.
- [ ] Verify that no aliasing violations can occur — no simultaneous `&mut` and `&` to the same data.
- [ ] Use `MaybeUninit<T>` for any uninitialized buffer; never call `assume_init` until every element is written.
- [ ] Wrap all unsafe operations in a safe public API — callers should not need to reason about unsafe.
- [ ] Add `// SAFETY:` comments to `unsafe impl Send/Sync` explaining why the type is safe to send or share.
- [ ] Use `catch_unwind` at every FFI boundary to prevent panic propagation, or use a `-unwind` ABI if appropriate.
- [ ] Use `CString` for C string arguments; never assume Rust strings are null-terminated.
- [ ] Run `cargo +nightly miri test` on the new code.
- [ ] Write tests that exercise the unsafe paths, including boundary conditions.
- [ ] If using `transmute`, document why `TryFrom`, `from_le_bytes`, `bytemuck`, or `zerocopy` cannot be used instead.
- [ ] Enable `#![deny(unsafe_op_in_unsafe_fn)]` at the crate level when possible.
- [ ] Prefer strict provenance APIs over integer-to-pointer casts.
- [ ] Verify `Ordering` choices for atomics.
- [ ] For `!Unpin` types, use `Box::pin` or the `pin!` macro; only use `Pin::new_unchecked` when you can prove the invariants.

## Validation hooks

Run these commands to detect unsafe-related issues:

```bash
# Miri — detects UB at the interpreter level (nightly only)
cargo +nightly miri test

# Miri with weak-memory data-race tracking
cargo +nightly miri test -- -Zmiri-track-weak-memory-loads

# Miri with Tree Borrows aliasing model instead of default Stacked Borrows
cargo +nightly miri test -- -Zmiri-tree-borrows

# Miri with deterministic concurrency (useful for reproducible CI)
cargo +nightly miri test -- -Zmiri-deterministic-concurrency

# Miri multi-seed testing
cargo +nightly miri test -- -Zmiri-many-seeds=0..256

# Clippy — catches common unsafe mistakes and anti-patterns
cargo clippy -- -W clippy::all -W clippy::pedantic

# AddressSanitizer — detects memory errors at runtime (nightly)
RUSTFLAGS="-Z sanitizer=address" cargo test --target x86_64-unknown-linux-gnu -Zbuild-std

# MemorySanitizer — detects uninitialized memory reads (nightly)
RUSTFLAGS="-Z sanitizer=memory" cargo test --target x86_64-unknown-linux-gnu -Zbuild-std

# ThreadSanitizer — detects data races (nightly)
RUSTFLAGS="-Z sanitizer=thread" cargo test --target x86_64-unknown-linux-gnu -Zbuild-std

# Valgrind — detects memory errors at the native level (no nightly required)
valgrind --leak-check=full --error-exitcode=1 target/debug/my_test_binary

# Check for unsafe_op_in_unsafe_fn violations
cargo rustc -- -D unsafe_op_in_unsafe_fn

# Audit all unsafe usage in the crate
cargo geiger

# cargo-careful: build std with extra UB checks and run tests
cargo +nightly careful test

# Loom: model-check concurrent data structures
cargo test --features loom

# Shuttle: randomized concurrency testing
cargo test --features shuttle
```

Miri notes ([Miri GitHub](https://github.com/rust-lang/miri)): detects OOB, use-after-free, invalid values, uninit reads, alignment, type invariants, data races, and aliasing (Stacked Borrows by default; opt into Tree Borrows with `-Zmiri-tree-borrows`). Key flags: `-Zmiri-disable-isolation`, `-Zmiri-ignore-leaks`, `-Zmiri-many-seeds=A..B` (default `0..64`), `-Zmiri-track-weak-memory-loads` (not `-refs`), `-Zmiri-tree-borrows`, `-Zmiri-strict-provenance`, `-Zmiri-deterministic-concurrency`. It is **unsound** to use `-Zmiri-disable-data-race-detector`, `-Zmiri-disable-stacked-borrows`, or `-Zmiri-disable-validation` for validation. Miri cannot run FFI/inline-asm/networking and does not prove absence of UB.

Sanitizer notes ([The Rust Unstable Book: Sanitizer](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html)): always pass `--target <triple>` **and** `-Zbuild-std` so std is instrumented. ThreadSanitizer does not support fence/inline-asm synchronization; MemorySanitizer requires all code to be instrumented.

cargo-careful ([GitHub](https://github.com/RalfJung/cargo-careful)): builds std with debug assertions, `-Zstrict-init-checks`, and `-Zextra-const-ub-checks`; supports FFI/inline asm and is faster than Miri. Use `#[cfg(careful)]` for custom paths.

Loom ([GitHub](https://github.com/tokio-rs/loom)): exhaustive C11-style concurrency model checking; treats `SeqCst` as `AcqRel` (possible false alarms) and is not sound (load-buffering gaps). Use `#[cfg(loom)]`.

Shuttle ([GitHub](https://github.com/awslabs/shuttle)): randomized concurrency testing; not sound but scales to larger tests; good complement to Loom.

cargo-geiger ([GitHub](https://github.com/geiger-rs/cargo-geiger)): counts unsafe in the dependency tree; heuristic (false positives/negatives via macros); statistical audit input, not a security verdict.

## Examples

### `MaybeUninit<T>` usage for stack allocation

```rust
use std::mem::MaybeUninit;

fn build_array() -> [String; 4] {
    let mut arr: [MaybeUninit<String>; 4] = MaybeUninit::uninit_array();
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(format!("item {i}"));
    }
    // SAFETY: Every element was initialized in the loop above.
    unsafe { MaybeUninit::array_assume_init(arr) }
}
```

### Safe wrapper around a raw pointer operation

```rust
use std::ptr::NonNull;

/// Pushes `value` into the `len`-th slot of a buffer that the caller
/// has allocated with capacity > len.
///
/// # Safety
/// `base` must be a non-null, aligned pointer to a live allocation of
/// at least `len + 1` initialized-or-uninitialized slots of type `T`.
unsafe fn push_unchecked<T>(base: NonNull<T>, len: usize, value: T) {
    // SAFETY: caller guarantees base + len is in-bounds and valid for writes.
    unsafe { base.as_ptr().add(len).write(value); }
}
```

### `unsafe impl Send/Sync` with justification

```rust
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Mutex;

struct SharedBuffer<T> {
    ptr: NonNull<T>,
    lock: Mutex<()>,
    _marker: PhantomData<T>,
}

// SAFETY: SharedBuffer synchronizes all access through its Mutex.
// The pointer is only dereferenced while the lock is held,
// so no data race is possible. T must be Send because the
// data can be accessed from any thread that acquires the lock.
unsafe impl<T: Send> Send for SharedBuffer<T> {}

// SAFETY: SharedBuffer synchronizes all access through its Mutex.
// &SharedBuffer allows obtaining &T only while the lock is held,
// so no mutable aliasing is possible. T must be Sync because
// &T can be shared across threads via &SharedBuffer.
unsafe impl<T: Send + Sync> Sync for SharedBuffer<T> {}
```

### FFI declaration with `unsafe extern "C"` and `#[unsafe(no_mangle)]`

```rust
use std::ffi::CStr;
use std::os::raw::c_char;

unsafe extern "C" {
    safe fn getenv(name: *const c_char) -> *const c_char;
}

fn get_env_var(name: &str) -> Option<String> {
    let c_name = std::ffi::CString::new(name).ok()?;
    // SAFETY: getenv is a standard C function. We pass a valid,
    // null-terminated C string. The returned pointer, if non-null,
    // points to a C string owned by the environment — we only read it.
    let ptr = unsafe { getenv(c_name.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: ptr is non-null and points to a valid C string
    // returned by getenv.
    let c_str = unsafe { CStr::from_ptr(ptr) };
    Some(c_str.to_string_lossy().into_owned())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_greeting(name: *const c_char) -> *const c_char {
    let result = std::panic::catch_unwind(|| {
        if name.is_null() {
            return std::ptr::null();
        }
        let c_str = unsafe { CStr::from_ptr(name) };
        let greeting = format!("Hello, {}!", c_str.to_string_lossy());
        std::ffi::CString::new(greeting).unwrap()
    });
    match result {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null(),
    }
}
```

### Anti-pattern: `mem::transmute` for endian swap

```rust
// WRONG — transmute for endian conversion
fn read_u32_le_wrong(bytes: [u8; 4]) -> u32 {
    // SAFETY: ??? — this is unsound if the platform has
    // different alignment requirements for u32 vs [u8; 4],
    // and it obscures the intent.
    unsafe { std::mem::transmute::<[u8; 4], u32>(bytes) }
}

// CORRECT — use the standard library
fn read_u32_le(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}
```

### Anti-pattern: panic propagating across FFI boundary

```rust
// WRONG — panic will unwind through extern "C", which is UB
#[no_mangle]
pub extern "C" fn bad_ffi_function(x: i32) -> i32 {
    let val = vec![1, 2, 3][x as usize]; // may panic if x >= 3
    val
}

// CORRECT — catch panics at the boundary
#[unsafe(no_mangle)]
pub extern "C" fn safe_ffi_function(x: i32) -> i32 {
    match std::panic::catch_unwind(|| {
        vec![1, 2, 3][x as usize]
    }) {
        Ok(val) => val,
        Err(_) => -1,
    }
}
```

### Anti-pattern: integer-to-pointer cast instead of strict provenance

```rust
use std::ptr;

// WRONG — loses provenance information
fn bad_roundtrip<T>(ptr: *const T) -> *const T {
    let addr = ptr as usize;
    addr as *const T
}

// CORRECT — strict provenance
fn good_roundtrip<T>(ptr: *const T) -> *const T {
    ptr::with_exposed_provenance(ptr::expose_provenance(ptr))
}
```

## Common mistakes

### 1. Missing `// SAFETY:` comment

```rust
// WRONG
unsafe { *ptr };

// CORRECT
// SAFETY: ptr was derived from a valid reference that is still in scope.
unsafe { *ptr };
```

### 2. Using `mem::uninitialized()`

```rust
// WRONG — deprecated and UB for any type with invalid bit patterns
let x: i32 = unsafe { std::mem::uninitialized() };

// CORRECT
let mut x: std::mem::MaybeUninit<i32> = std::mem::MaybeUninit::uninit();
x.write(42);
// SAFETY: x was initialized on the line above.
let x = unsafe { x.assume_init() };
```

### 3. Creating a reference to uninitialized memory

See the corresponding example in the [Examples](#examples) section. The fix is to use `ptr::write` into a `MaybeUninit<T>` and only call `assume_init()` after initialization. Creating `&mut T` or `&T` to uninit bytes is immediate UB.

### 4. Aliasing `&mut` and `&` to the same data

See the corresponding example in the [Examples](#examples) section. Simultaneous live `&mut T` and `&T` to overlapping data, or two `&mut T` to the same data, breaks the aliasing rules (`r-undefined.alias`). Restructure lifetimes so the borrows do not overlap.

### 5. Using `transmute` to extend a lifetime

Lifetime-escaping transmute is always UB. If a `'static` reference is genuinely needed, store the data in a static location or use `Box::leak` intentionally. See the corresponding example in the [Examples](#examples) section.

### 6. Forgetting to check alignment

See the corresponding example in the [Examples](#examples) section. Dereferencing a misaligned pointer is UB (`r-undefined.pointer-access`). Use `read_unaligned`/`write_unaligned` when alignment is not guaranteed.

### 7. Not catching panics at FFI boundaries

See the corresponding example in the [Examples](#examples) section. Unwinding through an `extern "C"` boundary is UB (`r-undefined.call`). Use `catch_unwind` or the matching `-unwind` ABI.

### 8. Implementing `Send`/`Sync` without justification

```rust
// WRONG — no safety comment
unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}

// CORRECT — explain why it is safe
// SAFETY: MyType contains a raw pointer to memory that is only
// accessed through a Mutex. The data is never moved or shared
// without holding the lock, so no data race is possible.
unsafe impl Send for MyType {}
```

### 9. Using `as` to cast between reference and pointer unsafely

Casting `&T` to `*mut T` and writing through it creates an aliasing violation. Be explicit about intent and ensure the source is the sole live reference:

```rust
let mut x: i32 = 42;
let ptr: *mut i32 = &raw mut x;
// SAFETY: ptr was derived from &mut x, which is the only
// reference to x. No other reference exists.
unsafe { *ptr = 43; }
```

### 10. Assuming `size_of::<T>() == size_of::<U>()` in transmute

```rust
// WRONG — sizes may differ across platforms or after refactoring
unsafe { std::mem::transmute::<u64, [u8; 8]>(x) }

// CORRECT — use to_ne_bytes / from_ne_bytes
let bytes: [u8; 8] = x.to_ne_bytes();
```

### 11. Wrong ABI or missing `unsafe extern`

```rust
// WRONG in Rust 2024 — missing unsafe on extern block
extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
}

// CORRECT
unsafe extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
}
```

### 12. Using bare unsafe attributes in Rust 2024

```rust
// WRONG in Rust 2024
#[no_mangle]
pub extern "C" fn old_style() {}

// CORRECT
#[unsafe(no_mangle)]
pub extern "C" fn new_style() {}
```

### 13. `Pin::new_unchecked` on a `!Unpin` type without proof

```rust
// WRONG — creates a Pin without guaranteeing the address will not change
struct SelfRef {
    data: String,
    ptr: *const String,
    _pin: std::marker::PhantomPinned,
}

let mut x = SelfRef { data: String::from("hello"), ptr: std::ptr::null(), _pin: std::marker::PhantomPinned };
// UB: x can still be moved; Pin::new_unchecked lies about the contract.
let _pinned = unsafe { std::pin::Pin::new_unchecked(&mut x) };

// CORRECT — use Box::pin or pin! macro for !Unpin types
let mut x = Box::pin(SelfRef { data: String::from("hello"), ptr: std::ptr::null(), _pin: std::marker::PhantomPinned });
```

## Strict vs contextual guidance

### Strict guidance (must-follow)

- Every `unsafe` block must have a `// SAFETY:` comment explaining why the operation is sound.
- Every `unsafe fn` must have a `# Safety` doc section listing all preconditions.
- Use `MaybeUninit<T>` instead of `mem::uninitialized()`. The latter is deprecated and UB for most types.
- Do not use `mem::transmute` without a written justification and a comment explaining why no safe alternative works.
- Mark `unsafe impl Send/Sync` with `// SAFETY:` rationale.
- Do not let panics cross FFI boundaries — use `catch_unwind` or the matching `-unwind` ABI.
- Never create a reference to uninitialized memory.
- Use `unsafe extern "C" { ... }` in Rust 2024 and newer code.
- Wrap unsafe attributes as `#[unsafe(attr)]` in Rust 2024 and newer code.
- Run `cargo +nightly miri test` on any module containing `unsafe` before merging.

### Common convention (default in most Rust codebases)

- One `// SAFETY:` line per unsafe block, placed immediately above the block.
- No `unsafe fn` without an explicit safety contract doc comment.
- Prefer safe wrappers over exposing unsafe in public APIs.
- Use `NonNull<T>` instead of `*mut T` when the pointer is guaranteed non-null.
- Enable `#![deny(unsafe_op_in_unsafe_fn)]` at the crate level.
- Use `CString` for FFI string arguments, never raw `*const c_char` from Rust strings.
- Prefer strict provenance APIs over raw integer-to-pointer casts.

### Contextual tradeoffs

- **FFI** — Manual `unsafe` is unavoidable when interfacing with C libraries. Minimize the surface area and wrap in safe APIs.
- **Hot paths with measurable benefit** — If profiling shows that removing `unsafe` causes a significant regression, document the measurement and the tradeoff. Re-measure after compiler updates.
- **OS-level primitives** — `epoll`, `io_uring`, `kqueue`, and similar APIs require `unsafe`. Use well-maintained crates (`mio`, `tokio`) when possible.
- **Lock-free data structures** — These require `unsafe` for atomic operations on raw pointers. Prefer `crossbeam` or `dashmap` over manual implementations.
- **Safety budget** — Some teams track the number of `unsafe` blocks as a metric. If the budget is exceeded, require a security review before adding more.

## Policy decisions for individual repos

Each repo should document the following in its `CONTRIBUTING.md` or equivalent:

1. **Unsafe policy** — Is `unsafe` allowed? Only for FFI? Only with sign-off from a specific reviewer?
2. **`// SAFETY:` comment format** — Require the exact `// SAFETY:` prefix? Allow `// Safety:`? Enforce via CI lint?
3. **Miri in CI** — Is `cargo +nightly miri test` required to pass? On which targets?
4. **`unsafe_op_in_unsafe_fn`** — Deny, warn, or allow? This affects how `unsafe fn` bodies are written.
5. **Review requirements** — Does every PR with `unsafe` require a second reviewer? A security-focused reviewer?
6. **Dependency unsafe budget** — Is there a limit on how much `unsafe` code is allowed in transitive dependencies?
7. **Tooling requirements** — Must `cargo geiger` be run? Must AddressSanitizer tests pass?
8. **Edition policy** — Is Rust 2024 required for new code? If not, document the migration timeline for `unsafe extern` and `#[unsafe(attr)]`.

## Related docs

- `docs/rust/async-tokio.md` — async futures, `Pin`, task spawning, and lifetime issues specific to async code.
- `docs/rust/lints-clippy.md` — Clippy lint guidance, including unsafe-aware lints like `clippy::not_unsafe_ptr_arg_deref`, `clippy::unsafe_removed_from_safe`, and `clippy::missing_safety_doc`.
- `docs/rust/cargo-dependencies.md` — Guidance on dependency management, including auditing dependencies for unsafe usage via `cargo geiger` and `cargo audit`.
- `docs/rust/supply-chain-security.md` — Supply chain security for Rust crates, including how to assess whether a dependency's use of `unsafe` is trustworthy.
- `docs/rust/editions-tooling.md` — Edition-specific behavior changes that affect unsafe code, such as the 2024 edition's `unsafe extern` requirement and `#[unsafe(attr)]` syntax.
- `docs/rust/ownership-lifetimes.md` — ownership, borrowing, aliasing, and variance rules that underpin unsafe soundness.
- `docs/rust/smart-pointers-memory.md` — `Box`, `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`, `Cow`, `Deref`, drop semantics, interior/exterior mutability, and `NonNull`.

## Related skills

No repo-specific skills for this topic. The general Rust guidance corpus (this file and its siblings) serves as the primary reference for unsafe Rust decisions.
