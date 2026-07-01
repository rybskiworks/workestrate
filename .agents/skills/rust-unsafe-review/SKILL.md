---
name: rust-unsafe-review
description: |
  Review and write unsafe Rust: FFI, raw pointers, Send/Sync impls, transmute,
  MaybeUninit, atomics, Pin, and 2024-edition unsafe syntax. Load when reviewing
  or writing `unsafe` blocks, `unsafe fn`, `unsafe extern`, `#[unsafe(attr)]`,
  manual `Send`/`Sync`, or FFI boundaries. Provides UB checklists, review
  checklists, Miri/clippy validation hooks, and decision tables. Cites
  `docs/rust/unsafe-security.md` and `docs/rust/types-traits-generics.md`.
---

# Rust Unsafe Review

Operational guidance for reviewing/writing `unsafe` Rust. Distilled from
[`docs/rust/unsafe-security.md`](../../../../docs/rust/unsafe-security.md) (UB
catalog, FFI, Send/Sync, transmute, Miri) and
[`docs/rust/types-traits-generics.md`](../../../../docs/rust/types-traits-generics.md)
(PhantomData variance, derives). Upstream source URLs live inline in those docs
— link to the docs, not upstream.

## Triggers

Load when reviewing a diff with `unsafe {}`, `unsafe fn`, `unsafe trait`,
`unsafe impl`, `unsafe extern`, or `#[unsafe(...)]`; writing raw-pointer, FFI,
`MaybeUninit`, `transmute`, atomic, or `Pin` code; auditing manual `unsafe impl
Send`/`Sync`; migrating pre-2024 `unsafe` syntax to 2024; or diagnosing UB. Not
for pure safe-Rust type-system questions.

## Core Principle

`unsafe` does not disable the borrow checker. It asserts a **soundness
contract**: safe code must never cause UB through this code. Soundness is
non-local — editing safe code can break an unsafe invariant. Privacy is the
only bulletproof blast-radius limiter.

## Safety Comment Requirement (non-negotiable)

Every `unsafe {}` block and `unsafe impl` MUST begin with a `// SAFETY:`
comment stating which precondition is upheld (aliasing/lifetime/init/alignment
reasoning). Every `unsafe fn` MUST have a `# Safety` doc section listing all
caller preconditions. Comments must be specific to the exact operation.

```rust
// SAFETY: ptr derived from a live &T in scope for this borrow.
unsafe { *ptr }
```

## UB Checklist

Confirm none of these (full catalog in `docs/rust/unsafe-security.md` →
"Undefined behavior"):

| Category | Check |
|---|---|
| Data races | No unsynchronized concurrent access where one is a write. |
| Dangling/misaligned ptr / projection | All pointed-to bytes in one live allocation; aligned; size ≤ `isize::MAX`; field/index access in-bounds. |
| Aliasing / immutable bytes | No live `&mut T` aliased by another ref/ptr; `&T` not mutated outside `UnsafeCell`; no write through `&T`, const-promoted, immutable statics. |
| Invalid values | `bool` 0/1; `char` not surrogate; enums valid discriminant+fields; refs/`Box` non-null aligned; `NonNull`/`NonZero*` in range; no uninit read except padding/unions. |
| Wrong call ABI | Don't mix `"C"` and `"C-unwind"` for one symbol; no unwind past non-unwind frame. |
| Features/asm/runtime | Don't execute unsupported features; correct asm constraints; no `longjmp` past Rust destructors. |

UB affects the **entire program** and crosses FFI boundaries. Not UB (still
bugs): deadlocks, leaks, exit-without-drop, integer overflow (defined:
panic/wrap), trait-contract violations (e.g. inconsistent `Hash`/`Eq`).

## Review Checklist (unsafe blocks)

- [ ] `// SAFETY:` comment present, specific, correct? `unsafe fn` has `# Safety` doc?
- [ ] Preconditions cover aliasing, lifetimes, alignment, init, thread-safety?
- [ ] Minimal surface — could safe alternatives (`bytemuck`, `zerocopy`,
  `crossbeam`, std APIs) replace it?
- [ ] Raw pointers from valid, live allocations for the full duration of use?
- [ ] No simultaneous `&mut` and `&` to the same data; no ref to uninit memory
  (use `ptr::write`); `MaybeUninit` fully initialized before `assume_init`?
- [ ] Pointer provenance known; strict-provenance APIs preferred over int↔ptr casts?
- [ ] Alignment verified, or `read_unaligned`/`write_unaligned` used?
- [ ] `transmute` has written justification + no safe alternative works?
- [ ] Atomics use correct `Ordering`; `Pin` invariants upheld for `!Unpin`
  (no `Pin::new_unchecked` without proof)?
- [ ] Panics cannot cross FFI boundary (`catch_unwind` or matching `-unwind` ABI)?
- [ ] `#![deny(unsafe_op_in_unsafe_fn)]` enabled or migration planned?
- [ ] `cargo +nightly miri test` run on the affected code?

## Manual Send/Sync Review

`Send`/`Sync` are `unsafe auto trait`; incorrect impls are UB. `Send` =
transferable across threads; `Sync` iff `&T` is `Send`. Auto-derived: struct is
`Send`/`Sync` iff all fields are. `*const T`/`*mut T`/`NonNull<T>` are
`!Send + !Sync` (may be aliased); `UnsafeCell`/`Cell`/`RefCell` are `!Sync`;
`Rc` is `!Send + !Sync`. `&T` is `Send` iff `T: Sync`; `&mut T` is `Send` iff
`T: Send`. Every `unsafe impl Send/Sync` MUST have a `// SAFETY:` comment
explaining the synchronization (e.g. all access behind a `Mutex`) and why
`T: Send`/`T: Sync` bounds are correct. Opt out on stable with
`PhantomData<*const T>` (gives `!Send + !Sync`); explicit negative impls need
`feature(negative_impls)`.

## FFI Review

- **`unsafe extern`** required in 2024 (`unsafe extern "C" { ... }`);
  `missing_unsafe_on_extern` is a hard error in 2024. Foreign fns are implicitly
  `unsafe` unless marked `safe fn` (only when verified). Wrap each C call in a
  safe fn (safe-wrapper sandwich); inner `unsafe {}` upholds the C precondition.
- **`#[unsafe(no_mangle)]`** (2024) / `#[no_mangle]` (pre-2024): disables
  mangling; collisions are UB. Use `crate-type = ["cdylib"]` for shared libs.
- **ABI strings**: `"C"` (default), `"system"` (= `"C"` except win32 → `stdcall`),
  `"C-unwind"`/`"system-unwind"` (allow unwind). Variadic only with `aapcs`,
  `C`, `cdecl`, `efiapi`, `system`, `sysv64`, `win64` (+ `-unwind`).
- **Panic boundary**: panic across non-unwind ABI safely aborts; foreign
  exception entering Rust across non-unwind is UB. `catch_unwind` at every
  exported boundary, or use the matching `-unwind` ABI.
- **Layout**: `repr(C)`, `repr(C, packed)`, `repr(transparent)` for FFI. Never
  use empty enums as FFI types. Field-less Rust enums modeling C `enum` are
  often wrong (C holds any int). Use the opaque struct idiom for private layout.
  Use `CString` for C string args; never pass Rust `&str`/`String` raw.

## 2024 Edition Unsafe Changes

| Change | Pre-2024 | 2024 |
|---|---|---|
| `extern` blocks | `unsafe` optional | `unsafe extern` required |
| Unsafe attributes | `#[no_mangle]` | `#[unsafe(no_mangle)]` |
| `unsafe_op_in_unsafe_fn` lint | allow | **warn-by-default** |
| `safe`/`unsafe` fn qualifiers in extern | n/a | `safe fn` allowed in `unsafe extern` |

Unsafe attributes (full set): `no_mangle`, `export_name`, `link_section`,
`naked` — all must be `#[unsafe(...)]` in 2024. Recommend
`#![deny(unsafe_op_in_unsafe_fn)]` in all editions so each unsafe op inside an
`unsafe fn` body gets its own `unsafe {}`.

> Caveat: the rustc "Allowed-by-default Lints" page shows
> `unsafe_op_in_unsafe_fn` as `Allow` because it ignores the
> `@edition Edition2024 => Warn;` override — the 2024 default IS warn. See
> `docs/rust/unsafe-security.md` → "2024 edition changes".

## transmute Rules

`mem::transmute<Src, Dst>`: bitwise reinterpret; same size required (compile
error otherwise); padding not preserved; both types must be valid or UB.
**Always UB / forbidden:** `&T` → `&mut T` ("No you're not special");
transmuting to a reference (unbounded lifetime); `3u8` → `bool`;
non-`repr(C)`/`repr(transparent)` layout assumptions (`Vec<i32>` vs `Vec<u32>`);
pointer→integer in const context (unless ptr came from an integer);
lifetime-escaping transmute. Related: `transmute_copy` UB if `U` larger than
`T`; `transmute_unchecked` (nightly) makes size mismatch runtime UB. Prefer
`from_ne_bytes`, `ptr as *const T as usize`, `bytemuck`, `zerocopy`.

## Validation Hooks

```bash
# Miri — interpreter-level UB detection (nightly). Run on every unsafe module.
cargo +nightly miri test
cargo +nightly miri test -- -Zmiri-many-seeds=0..256      # concurrency robustness
cargo +nightly miri test -- -Zmiri-tree-borrows          # alt aliasing model
cargo +nightly miri test -- -Zmiri-strict-provenance

# Clippy + enforce explicit unsafe blocks inside unsafe fn
cargo clippy -- -W clippy::all -W clippy::pedantic
cargo rustc -- -D unsafe_op_in_unsafe_fn

# Sanitizers (nightly, need --target + -Zbuild-std): address/memory/thread
RUSTFLAGS="-Z sanitizer=address" cargo test --target x86_64-unknown-linux-gnu -Zbuild-std
# Audit unsafe surface (cargo geiger, heuristic) / std with extra UB checks (cargo-careful)
cargo geiger && cargo +nightly careful test
```

Miri detects OOB, use-after-free, invalid values, uninit reads, alignment,
data races, aliasing (Stacked Borrows default). It CANNOT run FFI/inline-asm/
networking and does NOT prove absence of UB. It is **unsound** to validate with
`-Zmiri-disable-data-race-detector`, `-Zmiri-disable-stacked-borrows`, or
`-Zmiri-disable-validation`.

## Anti-patterns

- `unsafe { *ptr }` with no `// SAFETY:` comment.
- `mem::uninitialized()` / `mem::zeroed()` as init, or creating `&T`/`&mut T`
  to uninit memory (use `MaybeUninit<T>` + `ptr::write`).
- `transmute` for endian swap (use `from_le_bytes`/`from_ne_bytes`).
- `unsafe impl Send for MyType {}` with no `// SAFETY:` rationale.
- Panic unwinding through `extern "C"` (use `catch_unwind`); `Pin::new_unchecked`
  on `!Unpin` without proof (use `Box::pin` / `pin!`).
- Integer↔pointer `as` casts losing provenance; `as` cast `&T` → `*mut T` then
  writing (aliasing violation). Use strict provenance APIs.
- Bare `#[no_mangle]` / `extern "C" {}` in 2024 edition.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Miri: "dereferenced after allocation freed" | use-after-free / dangling ptr | keep alloc live; pair `Box::into_raw`/`from_raw` |
| Miri: "borrow tag conflict" | `&mut` aliased with another ref | restructure borrows; `UnsafeCell` for interior mutability |
| Miri: "invalid value of type" | uninit/invalid bit pattern read | `MaybeUninit` + `ptr::write`; never `mem::zeroed()` on refs |
| Miri: "pointer out of bounds" / "data race" | `add` past alloc / unsynchronized access | stay in alloc (one-past ok); atomics/`Mutex` + correct `Ordering` |
| `error: requires unsafe` / `missing_unsafe_on_extern` / `unsafe attribute used without unsafe` | 2024 `unsafe_op_in_unsafe_fn` warn / pre-2024 `extern` / bare `#[no_mangle]` | wrap in `unsafe {}` / `unsafe extern "C" { ... }` / `#[unsafe(no_mangle)]` |
| Link error: symbol collision | `#[no_mangle]` name clash | rename/scope; collisions are UB |
| Panic abort / UB across FFI | unwound through `extern "C"` | `catch_unwind` or `"C-unwind"` ABI |

## References

- [`docs/rust/unsafe-security.md`](../../../../docs/rust/unsafe-security.md) — full UB catalog, FFI/ABI, Send/Sync, transmute, MaybeUninit, atomics, Pin, Miri/sanitizer details, examples, upstream source URLs.
- [`docs/rust/types-traits-generics.md`](../../../../docs/rust/types-traits-generics.md) — `PhantomData` variance markers (covariant `PhantomData<T>` vs safe default `PhantomData<fn() -> T>`), derive whitelist, `Copy`/`Drop` constraints.
