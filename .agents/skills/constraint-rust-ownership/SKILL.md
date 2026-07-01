---
name: constraint-rust-ownership
description: |
  Enforces Rust ownership and borrowing rules during code execution. Load when
  writing or reviewing Rust code that touches ownership, moves, borrows,
  lifetimes, or smart pointer selection. Does NOT cover unsafe memory model
  (see constraint-rust-unsafe-safety).
metadata:
  org.kind: constraint
---

# Constraint: Rust Ownership and Borrowing

This constraint enforces the ownership and borrowing rules that the Rust
compiler checks, plus project conventions on `clone()` discipline and smart
pointer selection. Violations either fail to compile, introduce undefined
behavior, or signal a design smell.

## Triggers

Load this skill when:

- Writing or reviewing Rust code that moves, borrows, or stores references.
- Selecting among `Box`, `Rc`, `Arc`, `Weak`, `Cell`, `RefCell`, `Mutex`,
  `RwLock`, `OnceCell`/`OnceLock`, `Cow`.
- Annotating lifetimes or resolving borrow-checker errors.
- Reviewing a PR for ownership/aliasing correctness.

## Rules

1. Each value has **one owner**; a moved value cannot be used afterward.
2. At any time: **either** one `&mut T` **or** any number of `&T` — never both.
3. References must not outlive the data they borrow (no dangling references).
4. Do NOT use `unsafe` to work around the borrow checker.
5. Prefer restructuring ownership over reflexive `clone()`. Reach for `clone()`
   only after narrowing borrow scope, returning ownership, or restructuring has
   been considered.
6. Non-`Copy` values move on assignment, argument pass, and return; the source
   is dead after the move.

## References

- Operational skill: `rust-ownership-borrowing` (error→fix table, smart pointer
  choice matrix, implementation checklist).
- Docs: `docs/rust/ownership-lifetimes.md`, `docs/rust/smart-pointers-memory.md`.

## Out of scope

- Unsafe memory model and UB rules for `UnsafeCell`/`ManuallyDrop`/
  `MaybeUninit`/raw pointers — see `constraint-rust-unsafe-safety`.
- Async-specific ownership (`Pin`, `tokio::sync::Mutex` across `.await`) — see
  `docs/rust/async-tokio.md`.

## Violation examples

### Using a moved value (E0382)

```rust
let s = String::from("hi");
let t = s;            // s moved
println!("{}", s);   // FORBIDDEN: use of moved value
```

### Simultaneous `&mut` and `&` (E0502)

```rust
let mut v = vec![1, 2, 3];
let r = &v[0];       // shared borrow starts
v.push(4);           // FORBIDDEN: mutable borrow while shared borrow alive
println!("{}", r);
```

### Using `unsafe` to bypass the borrow checker

```rust
let mut x = 1;
let r = &x as *const i32;
let m = &mut x;      // FORBIDDEN: aliasing &mut with existing raw ref
unsafe { println!("{}", *r); }
```

### Needless clone to silence the borrow checker

```rust
let v = vec![1, 2, 3];
let len = v.len();
let owned = v.clone();   // FORBIDDEN (needless): v not consumed by len()
consume(v);
```

## How to check

```bash
cargo check --all-targets                 # catches borrow/move/lifetime errors
cargo clippy --workspace --all-targets -- -D warnings
# relevant lints: needless_clone, redundant_clone, clone_on_copy,
# needless_borrow, ptr_arg, rc_mutex, arc_with_non_send_sync,
# await_holding_lock
```

For unsafe ownership/aliasing assumptions, also run `cargo miri` (see
`constraint-rust-unsafe-safety`).
