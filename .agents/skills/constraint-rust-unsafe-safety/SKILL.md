---
name: constraint-rust-unsafe-safety
description: |
  Enforces unsafe code safety rules during Rust code execution. Load when
  writing or reviewing any unsafe block, unsafe fn, Send/Sync impl, transmute,
  or MaybeUninit usage. Does NOT cover general ownership/borrowing (see
  constraint-rust-ownership).
metadata:
  org.kind: constraint
---

# Constraint: Rust Unsafe Code Safety

This constraint enforces the documentation and encapsulation rules for `unsafe`
code. `unsafe` is not a license to bypass the rules; it is a contract that the
author must state and uphold. Every unsafe site must justify its safety and be
wrapped behind a safe API.

## Triggers

Load this skill when:

- Writing or reviewing any `unsafe {}` block.
- Writing or reviewing any `unsafe fn`.
- Implementing `unsafe impl Send` / `unsafe impl Sync`.
- Using or reviewing `std::mem::transmute`.
- Using or reviewing `MaybeUninit<T>` or any uninitialized memory.
- Reviewing FFI boundaries, raw pointer dereferences, or union access.

## Rules

1. Every `unsafe {}` block MUST have a `// SAFETY:` comment explaining why the
   operation is safe given the invariants the block relies on.
2. Every `unsafe fn` MUST have a `# Safety` doc section explaining the
   caller's obligations.
3. `unsafe` code must be encapsulated behind a safe API (the "sandwich"
   pattern): a thin `unsafe` layer that upholds invariants internally and
   exposes only safe functions to the rest of the crate.
4. Minimize the `unsafe` surface area — push `unsafe` to the leaves and keep
   blocks as small as possible.
5. `Send`/`Sync` `unsafe impl`s must document why the type is safe to share
   across threads (or why the default is wrong and the override is correct).
6. No `transmute` unless there is a documented safety justification. Prefer
   safe alternatives (`From`/`TryFrom`, `as` casts where sound, `MaybeUninit`).
7. Use `MaybeUninit<T>` instead of `mem::uninitialized()` (the latter is
   deprecated and instantiating UB for non-`Copy` types).

## References

- Operational skill: `rust-unsafe-review`.
- Docs: `docs/rust/unsafe-security.md`.

## Out of scope

- General ownership/borrowing rules — see `constraint-rust-ownership`.
- Smart pointer selection — see `rust-ownership-borrowing`.

## Violation examples

### `unsafe` block without a `// SAFETY:` comment

```rust
let bytes: [u8; 4] = [0, 0, 0, 1];
let n = unsafe { u32::from_ne_bytes(bytes) };  // FORBIDDEN: no SAFETY comment
```

Correct:

```rust
let bytes: [u8; 4] = [0, 0, 0, 1];
// SAFETY: `bytes` is a valid [u8; 4] initialized above; from_ne_bytes is
// sound for any byte pattern.
let n = unsafe { u32::from_ne_bytes(bytes) };
```

### `unsafe fn` without a `# Safety` doc section

```rust
/// Reads a u32 from a raw pointer.
pub unsafe fn read_u32(ptr: *const u8) -> u32 {  // FORBIDDEN: no # Safety
    unsafe { *(ptr as *const u32) }
}
```

Correct:

```rust
/// Reads a u32 from a raw pointer.
///
/// # Safety
///
/// `ptr` must be non-null, properly aligned for `u32`, and point to at least
/// 4 initialized bytes that are valid for `u32`.
pub unsafe fn read_u32(ptr: *const u8) -> u32 {
    unsafe { *(ptr as *const u32) }
}
```

### Bare `transmute` without justification

```rust
let n = unsafe { std::mem::transmute::<[u8; 4], u32>(bytes) };  // FORBIDDEN
```

Correct: use `u32::from_ne_bytes(bytes)` (safe), or if `transmute` is truly
required, add a `// SAFETY:` comment justifying type layout and validity.

### `mem::uninitialized()` (UB for non-`Copy` types)

```rust
let s = unsafe { std::mem::uninitialized::<String>() };  // FORBIDDEN: UB
```

Correct:

```rust
let mut buf: MaybeUninit<String> = MaybeUninit::uninit();
// ... initialize buf ...
let s = unsafe { buf.assume_init() };  // with SAFETY comment
```

### `unsafe impl Send` without justification

```rust
struct Raw(*mut u8);
unsafe impl Send for Raw {}  // FORBIDDEN: no justification
```

Correct:

```rust
struct Raw(*mut u8);
// SAFETY: Raw is Send because the raw pointer is only dereferenced on the
// owning thread (see module-level invariant); no shared mutation occurs.
unsafe impl Send for Raw {}
```

## How to check

```bash
cargo check --all-targets                 # compile gate
cargo clippy --workspace --all-targets -- -D warnings
# relevant lints: invalid_null_ptr_usage, transmute_ptr_to_ptr,
# mem_forget, uninit_assumed_init, missing_safety_doc,
# missing_const_for_fn (contextual)
cargo miri                                 # REQUIRED for unsafe lifetime/raw-ptr code
```

Manual review checklist:

- Every `unsafe {}` block has a `// SAFETY:` comment.
- Every `unsafe fn` has a `# Safety` doc section.
- Every `unsafe impl Send/Sync` has a `// SAFETY:` comment.
- No `mem::uninitialized`; `MaybeUninit` used instead.
- Every `transmute` has a documented safety justification.
- `unsafe` is encapsulated behind a safe API (sandwich pattern).
