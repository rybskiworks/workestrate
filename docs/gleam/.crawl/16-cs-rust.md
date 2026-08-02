# Crawl: cheatsheets/gleam-for-rust-users/
- seed_url: https://gleam.run/cheatsheets/gleam-for-rust-users/
- canonical_url: https://gleam.run/cheatsheets/gleam-for-rust-users/
- family: Gleam cheatsheets
- fetch: 200
- gleam_version: not stated on page
- feeds_docs: conventions-patterns-antipatterns.md, result-option-and-errors.md

## Purpose
A side-by-side syntactic translation reference for Rust developers learning Gleam.
Covers comments, variables, the match operator, type annotations, functions
(declaration, export, annotations, overloading, referencing, labelled args),
operators, constants, blocks, data types (strings, tuples, lists), patterns,
custom types (records + unions), flow control (`case` vs `match`), and modules.
The page is purely syntactic/lexical — it does NOT discuss runtime semantics,
ownership, GC, concurrency, error handling, or the BEAM.

## Translation tables (Rust → Gleam)

### Comments
| Concept | Rust | Gleam |
|---|---|---|
| Line comment | `//` | `//` |
| Item doc | `///` | `///` |
| Module doc | `//!` | `////` |

### Variables
| Concept | Rust | Gleam |
|---|---|---|
| Bind / shadow | `let size = 50;` | `let size = 50` |
| Mutability | `let mut` for mutable; default immutable | no `mut`; always immutable |
| Match in `let` | `let [x] = [1];` (compile error on type/value mismatch) | `let [x] = [1]` (compile error on type mismatch; runtime error on value mismatch) |
| Assertion binding | n/a (panics via `let ... else` / `assert!`) | `let assert 2 = x` (runtime error on value mismatch) |
| Type annotation | `let x: [u64; 3] = [1, 2, 3];` | `let x: List(Int) = [1, 2, 3]` |

### Functions
| Concept | Rust | Gleam |
|---|---|---|
| Declaration | `pub fn sum(x: u64, y: u64) -> u64 { x + y }` | `pub fn sum(x, y) { x + y }` (annotations optional) |
| Anonymous fn | `let mul = |x, y| x * y;` | `let mul = fn(x, y) { x * y }` |
| Export | `pub` keyword | `pub` keyword |
| Type annotations | always required | optional, always checked when present |
| Overloading | not supported | not supported |
| Referencing | `let func = identity;` | `let func = identity` |
| Labelled args | not supported | supported: `pub fn replace(inside string, each pattern, with replacement)`; call `replace(each: ",", with: " ", inside: "A,B,C")`; zero-cost, type-checked |

### Operators
| Operation | Rust | Gleam (Int) | Gleam (Float) | Notes |
|---|---|---|---|---|
| Equal | `==` | `==` | `==` | |
| Not equal | `!=` | `!=` | `!=` | |
| Greater than | `>` | `>` | `>.` | float variant has trailing `.` |
| Greater or equal | `>=` | `>=` | `>=.` | |
| Less than | `<` | `<` | `<.` | |
| Less or equal | `<=` | `<=` | `<=.` | |
| Bool and | `&&` | `&&` | — | both bool |
| Bool or | `||` | `||` | — | both bool |
| Add | `+` | `+` | `+.` | |
| Subtract | `-` | `-` | `-.` | |
| Multiply | `*` | `*` | `*.` | |
| Divide | `/` | `/` | `/.` | |
| Remainder | `%` | `%` | — | no float remainder |
| String concat | n/a (uses `+`/`format!`) | — | — | `<>` (both String) |
| Pipe | n/a | — | — | `|>` pipes into fn |

Key operator rule: Gleam separates Int and Float arithmetic into distinct
operators; floats require the trailing-dot form (`+.` `-.` `*.` `/.` `<.` `>.`
`<=.` `>=.`). Mixing is a compile error.

### Constants
| Concept | Rust | Gleam |
|---|---|---|
| Declare | `const the_answer: u64 = 42;` (type required) | `const the_answer = 42` (type optional) |
| Cross-module ref | `other_module::the_answer` | `other_module.the_answer` (after `import other_module`) |

### Blocks
| Concept | Rust | Gleam |
|---|---|---|
| Expression grouping | `{ ... }` | `{ ... }` |
| Arithmetic grouping | `( ... )` | `{ ... }` (braces, not parens) |

Note: Gleam uses braces for BOTH expression grouping and arithmetic precedence;
Rust uses parens for arithmetic.

### Data types
| Concept | Rust | Gleam |
|---|---|---|
| Strings | `"Hellø, world!"` (UTF-8) | `"Hellø, world!"` (UTF-8) |
| Tuples | `("username", "password", 10)` | `#("username", "password", 10)` (note `#` prefix) |
| Tuple destructuring | `let (_, password, _) = my_tuple;` | `let #(_, password, _) = my_tuple` |
| Arrays/Lists | `[1, 2, 3]` (fixed array) | `[1, 2, 3]` (linked list) |
| Cons / prepend | `[0, ..list]` — **compile error** in Rust arrays | `[0, ..list]` — works |
| List pattern head | `[0, second, ..]` — **compile error** in Rust | `let assert [0, second, ..] = list` — works |

### Custom types (records)
| Concept | Rust | Gleam |
|---|---|---|
| Struct decl | `struct Person { name: String, age: u64 }` | `type Person { Person(name: String, age: Int) }` |
| Construct | `Person { name: "Jake".to_string(), age: 35 }` | `Person(name: "Jake", age: 35)` |
| Field access | `person.name` | `person.name` |
| Runtime repr | struct | tuple (Erlang-record compatible) |

### Unions (enums)
| Concept | Rust | Gleam |
|---|---|---|
| Decl | `enum IpAddress { V4(u8,u8,u8,u8), V6(String) }` | `type IpAddress { V4(Int,Int,Int,Int) V6(String) }` |
| Construct variant | `IpAddress::V4(192,168,1,1)` | `V4(192,168,1,1)` (no path prefix; constructors are unqualified within scope) |

### Flow control
| Concept | Rust | Gleam |
|---|---|---|
| Match expr | `match x { A(n) => ..., _ => () }` | `case x { A(n) -> ... _ -> ... }` |
| Arm body | `=> expr` (single) / `{ stmts; expr }` (block) | `-> expr` (single) / `{ stmts expr }` (block, last expr is value) |
| String match | `match s { "abc" => ..., _ => () }` | `case s { "abc" -> ... _ -> ... }` |

### Modules
| Concept | Rust | Gleam |
|---|---|---|
| Define module | `mod wibble { ... }` (multiple per file) | one file = one module (named by file path); no `mod` keyword |
| Import | `use super::wibble;` | `import wibble` (or `import lib/wibble`) |
| Access member | `wibble::identity(1)` | `wibble.identity(1)` (dot, not `::`) |

## Migration pitfalls / mental-model traps (GC vs ownership; no lifetimes/traits)

> **Coverage caveat:** This cheatsheet is *syntactic only*. It does NOT discuss
> ownership, borrowing, lifetimes, traits, Result/Option, error handling, GC,
> or the BEAM concurrency model. The pitfalls below are inferred from the
> translation gaps a Rust developer will hit; they are NOT stated verbatim on
> the page. Cross-check with `result-option-and-errors.md` and the concurrency
> docs before relying on them.

1. **No ownership / no borrowing / no lifetimes.** Gleam is garbage-collected
   on the BEAM; values are immutable and shared freely. Do not look for
   `&`, `&mut`, `'a`, `Box`, `Rc`, `Arc`, `RefCell`, `Mutex` equivalents —
   none exist. Cloning is implicit and cheap (immutable data, no deep-copy
   semantics in the Rust sense).

2. **No `mut`.** All variables are immutable; "mutation" is done by shadowing
   (`let x = x + 1`). Rust `let mut` patterns must be rewritten as a series of
   shadowing binds or as recursion / a fold.

3. **No traits / no trait bounds / no generics-as-Rust-knows-them on this
   page.** The page never mentions traits. Gleam's polymorphism story
   (generics via type variables, no type classes) is not covered here — do
   not assume `impl Trait`, `where T:`, or `dyn Trait` exist.

4. **Result/Option are NOT covered here.** Despite the crawl brief asking for
   parallels, this page has zero mention of `Result`, `Option`, `?`, `panic!`,
   or `let assert`'s relationship to error handling. The only "assert" shown
   is `let assert` for pattern-match assertions (runtime error on mismatch) —
   this is NOT Gleam's `Result`/error-return story. See
   `result-option-and-errors.md` for the real model.

5. **Int vs Float operators are distinct and easy to get wrong.** Rust unifies
   numeric operators and infers integer/float from types; Gleam forces you to
   pick `+` vs `+.`, `<` vs `<.`, etc. A Rust dev writing `x + 1.0` will get a
   compile error, not silent coercion.

6. **`let assert` is a runtime crash, not a checked unwrap.** Rust's
   `unwrap()`/`expect()` panic at runtime; Gleam's `let assert` similarly
   crashes the process at runtime on value mismatch — but on the BEAM a
   process crash is isolated and supervised, not a process-wide abort. Do not
   assume `let assert` == `unwrap()` semantics in a concurrent system.

7. **Tuples need a `#` prefix.** `("a", 1)` is a Rust tuple; Gleam requires
   `#("a", 1)`. Forgetting the `#` is a syntax error.

8. **Lists are linked lists, not arrays/Vec.** Rust `[1,2,3]` is a fixed array
   (and `vec!` for growable); Gleam `[1,2,3]` is a singly-linked list. Prepends
   (`[0, ..list]`) and head-pattern matching that are *compile errors* in Rust
   arrays are *idiomatic* in Gleam. Do not assume O(1) indexing.

9. **One module per file; no nested `mod`.** Rust lets you nest `mod` blocks
   in a single file; Gleam does not — file path == module name. Restructuring
   Rust code with nested modules requires splitting into files/directories.

10. **Member access is `.`, not `::`.** Rust uses `::` for paths and `.` for
    fields; Gleam uses `.` for both module members and record fields. There is
    no `::` operator.

11. **Arithmetic grouping uses braces, not parens.** `x * (x + 10)` in Rust
    becomes `x * {x + 10}` in Gleam. Parens are not the grouping operator.

12. **Custom-type constructors are unqualified.** Rust writes
    `IpAddress::V4(...)`; Gleam writes `V4(...)` directly (qualified only via
    module import). Name collisions across imported types need aliasing.

13. **Module docs use `////`, not `//!`.** Easy to mistype.

## What NOT to assume from Rust
- Do NOT assume ownership, borrowing, or lifetimes exist — they don't.
- Do NOT assume `mut` exists — it doesn't; use shadowing.
- Do NOT assume traits, trait impls, or `dyn`/`impl Trait` exist — not on this
  page and not part of Gleam's model in the Rust sense.
- Do NOT assume `Result`/`Option`/`?` are covered here — they aren't.
- Do NOT assume numeric operators are type-polymorphic — Int and Float have
  separate operator spellings.
- Do NOT assume arrays/Vec exist — lists are immutable singly-linked lists.
- Do NOT assume `::` path syntax exists — use `.`.
- Do NOT assume `()` is the unit/never-return in the same way — Gleam's
  `Nil` type plays the unit role (not shown on this page).
- Do NOT assume `let assert` is checked error handling — it's a runtime crash.
- Do NOT assume a process crash aborts the program — on the BEAM, crashes are
  isolated and supervisor-restartable (not covered here).
- Do NOT assume you can nest multiple modules in one file — one file, one
  module.
- Do NOT assume labelled arguments exist in Rust — they're Gleam-only.

## Review concerns
- Page is syntactic only; any doc that promises "Rust→Gleam migration
  pitfalls including GC/ownership/Result/Option/concurrency" must NOT cite
  this page as the source for those topics — it doesn't cover them.
- The `let assert` example is the only error-adjacent content; it must not be
  presented as Gleam's error-handling story (that lives in
  `result-option-and-errors.md`).
- Operator table is the highest-value, most-verbatim section; safe to quote.
- "Gleam doesn't have a `mut` keyword" and "no function overloading" are the
  only explicit semantic claims — both safe.
- No version pinned; Gleam syntax here is stable/current as of fetch date.

## Verbatim quotes
- "Gleam doesn't have a `mut` keyword to mark variables as mutable, they're
  always immutable."
- "In Gleam, `let` and `=` can also be used for pattern matching, but you'll
  get compile errors if there's a type mismatch, and a runtime error if
  there's a value mismatch. For assertions, the equivalent `let assert`
  keyword is preferred."
- "Functions can optionally have their argument and return types annotated in
  Gleam. These type annotations will always be checked by the compiler and
  throw a compilation error if not valid. The compiler will still type check
  your program using type inference if annotations are omitted."
- "Like Rust, Gleam does not support function overloading, so there can only
  be 1 function with a given name, and the function can only have a single
  implementation for the types it accepts."
- "There is no performance cost to Gleam's labelled arguments as they are
  optimised to regular function calls at compile time, and all the arguments
  are fully type checked."
- "There is no equivalent feature [labelled arguments] in Rust."
- "Tuples are very useful in Gleam as they're the only collection data type
  that allows mixed types in the collection."
- "The `cons` operator works the same way both for pattern matching and for
  appending elements to the head of a list."
- "At runtime, they [custom types] have a tuple representation and are
  compatible with Erlang records."
- "In Gleam, each file is a module, named by the file name (and its directory
  path). Since there is no special syntax to create a module, there can be
  only one module in a file."
- "Gleam uses the `import` keyword to import modules, and the dot `.`
  operator to access properties and functions inside."

## Version notes
- No Gleam version stated on the page.
- No "last updated" / build-date metadata visible in the extracted content.
- Syntax shown is consistent with current Gleam (no deprecated forms
  observed).

## Discovered links

### Relevant (crawl later)
- (none new) The page is self-contained; its in-page anchor links
  (`#comments`, `#variables`, `#functions`, `#operators`, `#constants`,
  `#blocks`, `#data-types`, `#custom-types`, `#flow-control`, `#modules`)
  are intra-page navigation only, not separate crawl targets.
- Topics NOT covered here that should be fetched from elsewhere:
  - Result/Option/error handling → `result-option-and-errors.md` target
  - Concurrency / BEAM process model → concurrency docs
  - Generics / type variables → language tour
  - External functions (FFI) → externals page (already crawled as 08)

### Skipped
- In-page anchor TOC links (navigation only, not separate documents).
- Site chrome (header/nav/footer) — not extracted.
