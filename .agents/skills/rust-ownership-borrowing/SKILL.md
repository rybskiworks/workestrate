---
name: rust-ownership-borrowing
description: |
  Operational guide for writing and debugging Rust code that hits the borrow
  checker: ownership moves, borrowing conflicts, lifetime errors, and smart
  pointer selection. Load when fixing compiler errors E0382, E0502, E0505,
  E0507, E0596, E0597, E0106, E0204, E0621; when choosing between clone,
  borrow, or restructure; when selecting Box/Rc/Arc/RefCell/Mutex/RwLock; or
  when annotating lifetimes. Does NOT cover async-specific ownership (see
  docs/rust/async-tokio.md), unsafe UB rules (see docs/rust/unsafe-security.md),
  or trait/generics design (see docs/rust/types-traits-generics.md).
---

# Rust Ownership, Borrowing, and Smart Pointer Selection

Distilled operational reference. Full theory and citations live in
`docs/rust/ownership-lifetimes.md` and `docs/rust/smart-pointers-memory.md`
(which themselves cite upstream Rust Book/Reference/Nomicon sources).

## Triggers

Load this skill when:

- Fixing borrow-checker errors (E0382, E0502, E0505, E0507, E0596, E0597,
  E0106, E0204, E0621, E0040).
- Deciding whether to `clone()`, borrow (`&` / `&mut`), move, or restructure.
- Annotating lifetimes or hitting "missing lifetime specifier".
- Choosing among `Box`, `Rc`, `Arc`, `Weak`, `Cell`, `RefCell`, `Mutex`,
  `RwLock`, `OnceCell`/`OnceLock`, `Cow`.
- Reviewing a PR for ownership/aliasing correctness.

## Core Borrow Rules (compiler-enforced)

1. Each value has **one owner**; owner drop frees the value.
2. At any time: **either** one `&mut T` **or** any number of `&T` — never both.
3. References must never outlive the data they borrow (no dangling refs).
4. Non-`Copy` values **move** on assignment / arg pass / return; source is dead.
5. `Copy` and `Drop` are mutually exclusive. `Copy` = bitwise, implicit;
   `Clone` = explicit, arbitrary code.

See `docs/rust/ownership-lifetimes.md` § "Ownership rules", "References and
borrowing", "Move semantics".

## Borrow-Checker Error → Fix Table

| Error | Meaning | First-line fix |
|---|---|---|
| E0382 | use of moved value | borrow (`&`) for reads; `clone()` if two owned copies needed; derive `Copy` only if eligible |
| E0502 | conflicting `&mut` + `&` of same data | reorder so first borrow ends (NLL) before the second begins; split into separate scopes |
| E0505 | move out of borrowed content | release borrow before move; avoid move; or impl `Copy` |
| E0507 | cannot move out of deref of `&` / behind shared ref | borrow, clone, or `Copy`; use `Cow` for conditional ownership |
| E0596 | cannot borrow `&` as `&mut` | take `&mut T` param; wrap in `Cell`/`RefCell`/`Mutex` for interior mutability |
| E0597 | borrowed value does not live long enough | extend borrower's scope; return owned value; tie output lifetime to input |
| E0106 | missing lifetime specifier | add annotation, or restructure so elision applies |
| E0204 | `Copy` on type with non-`Copy` field / `Drop` | drop `Copy`, or make all fields `Copy` and remove `Drop` |
| E0621 | explicit lifetime mismatched | align lifetimes; check variance (`&mut T` invariant in `T`) |
| E0040 | explicit call to `.drop()` | use `std::mem::drop(value)` for early drop |

Note: E0495 is no longer emitted — do not treat it as current.

## Clone vs. Borrow vs. Move Decision

| Function needs | Signature | Notes |
|---|---|---|
| Read only | `fn f(s: &str)`, `fn f(v: &[T])` | accept slices, not `&String`/`&Vec` (clippy::ptr_arg) |
| Mutate in place | `fn f(s: &mut T)` | caller keeps ownership |
| Consume | `fn f(s: String)` | take ownership; do not borrow+clone internally |
| Owned result from borrow | `fn f(c: &Config) -> String` | explicit `.clone()`; prefer `dest.clone_from(&src)` |
| Maybe-own | `Cow<'_, B>` | zero-copy read path, clone on first mutation |

Before reaching for `.clone()` to silence the borrow checker: try narrowing
borrow scope (NLL), returning ownership, or restructuring. If a clone is
genuinely needed, make it explicit and justified.

## Lifetime Elision Rules

Compiler applies automatically (do not annotate unless these fail):

1. Each reference param gets its own lifetime.
2. If exactly one input lifetime → assigned to all outputs.
3. Method with `&self`/`&mut self` → `self`'s lifetime assigned to outputs.

Illegal to elide (require explicit annotation):
- `fn get_str() -> &str` — no input lifetime to infer.
- `fn frob(s: &str, t: &str) -> &str` — ambiguous which input owns output.

Use `'static` **only** for string literals, `Box::leak`, or truly program-
lifetime data. Never slap `'static` on to silence the compiler — it signals a
dangling ref or lifetime mismatch. See `docs/rust/ownership-lifetimes.md`
§ "The `'static` lifetime".

## Smart Pointer Choice Matrix

| Need | Use | Avoid |
|---|---|---|
| Exclusive heap ownership / recursive type / `dyn Trait` | `Box<T>` | `Rc`/`Arc` (overkill) |
| Shared ownership, single thread | `Rc<T>` | `Arc` (atomic cost); never `Send` |
| Shared ownership, multi-thread | `Arc<T>` | `Rc` (`!Send`) |
| Break cycles / non-owning back-edge | `Weak<T>` | strong cycles (leak) |
| Interior mutability, `Copy` type, 1 thread | `Cell<T>` | `RefCell<u32>` (overhead) |
| Interior mutability, any type, 1 thread | `RefCell<T>` | `Cell` (needs `Copy`); `!Sync` |
| Lazy init, 1 thread | `OnceCell<T>` | `OnceLock` (unneeded sync cost) |
| Lazy init / static, multi-thread | `OnceLock<T>` | `OnceCell` (`!Sync`) |
| Interior mutability, multi-thread | `Arc<Mutex<T>>` | `Arc<RefCell<T>>` (`!Sync`); `Mutex<Arc<T>>` (locks pointer, not data) |
| Read-heavy multi-thread | `Arc<RwLock<T>>` | default to `Mutex` unless profiling justifies |
| Clone-on-write | `Cow<'a, B>` | unconditional `.clone()` |
| Suppress drop (FFI) | `ManuallyDrop<T>` (unsafe) | reordering fields first |
| Uninit memory | `MaybeUninit<T>` (unsafe) | `mem::uninitialized` (UB) |

Canonical patterns: `Rc<RefCell<T>>` (single-thread shared mutable),
`Arc<Mutex<T>>` (multi-thread shared mutable), strong parent→child + weak
child→parent for trees. See `docs/rust/smart-pointers-memory.md` § "Decision
matrix", "Practical rules".

## Implementation Checklist

- [ ] Name the single owner for every value from creation to drop.
- [ ] `&T` to read, `&mut T` to mutate, `T` to consume.
- [ ] Accept `&str` / `&[T]`, not `&String` / `&Vec<T>`.
- [ ] Rely on elision; annotate only when compiler requires or ambiguity exists.
- [ ] Struct holding a reference → declare lifetime; verify struct can't
      outlive borrowed data.
- [ ] Before `.clone()`: tried narrowing borrow scope / returning ownership?
- [ ] `Rc`/`Arc` cycle? Add `Weak` back-edges.
- [ ] `RefCell` borrows scoped tightly; not stored in fields or returned.
- [ ] `MutexGuard` dropped before re-lock or `.await`.
- [ ] `Drop` impl cannot panic (check `std::thread::panicking()`).
- [ ] `unsafe` lifetime/aliasing assumptions have `// SAFETY:` comments.

## Anti-patterns

- `Rc<T>` near `thread::spawn` or in `Send` types — use `Arc`.
- `Arc<RefCell<T>>` or `Mutex<Arc<T>>` — use `Arc<Mutex<T>>`.
- `Arc::try_unwrap(x).ok()` across threads (racy) — use `Arc::into_inner`.
- `RefCell<u32>`/`RefCell<bool>` — use `Cell`.
- `&String`/`&Vec<T>` params — use `&str`/`&[T]`.
- Implementing `Deref` for mere conversion — use `AsRef`/`Borrow`.
- `&mut T` by casting `&UnsafeCell<T>` directly (UB) — source via
  `UnsafeCell::get`/`raw_get`.
- `assume_init` on uninit `MaybeUninit` (UB even for `i32`).
- Self-referential structs with ordinary refs — use indices, owned copies,
  arena, or `Pin<!Unpin>`.
- `Box::from_raw` twice on same pointer (double-free).

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| E0382 use of moved value | non-`Copy` value used after move | borrow/clone/restructure |
| E0502 conflicting borrows | `&mut` overlaps `&` | reorder; NLL ends borrow at last use |
| E0597 lifetime too short | local ref returned / outlives data | return owned; extend scope; tie lifetime |
| E0106 missing lifetime | elision can't infer | add annotation or restructure |
| E0596 borrow as mut | `&` needs `&mut` | interior mutability (`Cell`/`RefCell`/`Mutex`) |
| `RefCell` panic at runtime | overlapping `borrow_mut` | scope borrows tightly; use `try_borrow_mut` |
| `Mutex` deadlock | guard held across re-lock / `.await` | drop guard before re-lock; use `tokio::sync::Mutex` across `.await` |
| `Rc` cycle leak | strong parent↔child | `Weak` back-edge |
| `Rc` in `Send` context | `!Send` | switch to `Arc` |

## Verification Commands

Before claiming a fix complete, run from the project root inside
`just shell` (see `nix-usage` skill):

```bash
cargo check                       # primary borrow-checker gate
cargo clippy --all-targets -- -D warnings   # ptr_arg, clone_on_copy,
                                             # redundant_clone, needless_borrow,
                                             # rc_mutex, arc_with_non_send_sync,
                                             # await_holding_lock
cargo test                        # runtime validation (RefCell panics, etc.)
cargo miri                        # REQUIRED for unsafe lifetime/raw-ptr code
```

For `Arc<Mutex<T>>` / `Arc<RwLock<T>>` patterns, also run under
ThreadSanitizer to detect data races and deadlocks.

## Related Docs

- `docs/rust/ownership-lifetimes.md` — full ownership/borrow/lifetime/variance
  theory with upstream citations.
- `docs/rust/smart-pointers-memory.md` — full smart-pointer reference, `Deref`,
  `Drop`, `Cow`, `Borrow`/`ToOwned`, drop order.
- `docs/rust/async-tokio.md` — async ownership, `Pin`, `tokio::sync::Mutex`.
- `docs/rust/unsafe-security.md` — UB rules for `UnsafeCell`/`ManuallyDrop`/
  `MaybeUninit` and raw-pointer lifetime invariants.
- `docs/rust/types-traits-generics.md` — `PhantomData` variance, `AsRef`/`Borrow`.
