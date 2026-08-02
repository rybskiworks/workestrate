# Crawl: cheatsheets/gleam-for-erlang-users/
- seed_url: https://gleam.run/cheatsheets/gleam-for-erlang-users/
- canonical_url: https://gleam.run/cheatsheets/gleam-for-erlang-users/
- family: Gleam cheatsheets
- fetch: 200
- gleam_version: not present on page
- feeds_docs: conventions-patterns-antipatterns.md, erlang-interop.md

## Purpose
Side-by-side translation reference for Erlang programmers learning Gleam. Covers
syntax and type-system differences for variables, functions, operators, data
types, and custom types. Ends with stub headers (Modules / Imports / Nested
modules / First class modules) that have NO body content on the page — these
sections are unimplemented stubs as of fetch.

## Translation tables (Erlang → Gleam)

### Variables
| Erlang | Gleam | Notes |
|---|---|---|
| `Size = 50` (capital, single-assignment) | `let size = 50` (lowercase, reassignable) | Erlang reassign of bound var is runtime error; Gleam shadowing is legal |
| `[Element] = SomeList` (partial pattern assert) | `let assert [element] = some_list` | Gleam requires explicit `let assert` for partial patterns; bare `let [element] = ...` is a compile error |
| no type annotations on vars | `let some_list: List(Int) = [1, 2, 3]` | Optional; always checked |

### Functions
| Erlang | Gleam |
|---|---|
| `my_function(X) -> X + 1.` | `fn my_function(x) { x + 1 }` |
| `-export([my_function/1]).` | `pub fn my_function(x) { ... }` (no export statement) |
| `-spec my_function(integer()) :: integer().` | `fn my_function(x: Int) -> Int { ... }` (always checked, not optional-dialyzer) |
| multiple function heads (`f(1)->...; f(2)->...`) | single head + `case x { 1 -> ... 2 -> ... }` (no multiple heads, like Core Erlang) |
| function overloading | NOT supported — one impl per name |
| `Func = fun identity/1` | `let func = identity` (single namespace for values & functions) |
| `(((f(0))(1))(2))(3)` | `f(0)(1)(2)(3)` |
| map-based labelled args `#{inside => S, each => P}` | `pub fn replace(inside string, each pattern, ...)` then `replace(each: ",", with: " ", inside: "...")` — zero-cost, fully type-checked |

### Comments
| Erlang `%` | Gleam `//` |
| `%%` n/a | `///` doc-comment for following item; `////` module doc-comment |

### Operators
| Op | Erlang | Gleam | Constraint |
|---|---|---|---|
| Equal | `=:=` | `==` | same type both sides |
| Not equal | `=/=` | `!=` | same type |
| (Erlang `==`/`/=` loose) | — | not present | Gleam has no loose equality |
| Greater than (int) | `>` | `>` | both Int |
| Greater than (float) | `>` | `>.` | both Float |
| Greater or equal (int) | `>=` | `>=` | both Int |
| Greater or equal (float) | `>=` | `>=.` | both Float |
| Less than (int) | `<` | `<` | both Int |
| Less than (float) | `<` | `<.` | both Float |
| Less or equal (int) | `=<` | `<=` | both Int |
| Less or equal (float) | `=<` | `<=.` | both Float |
| Boolean and | `andalso` / `and` | `&&` | both Bool |
| Boolean or | `orelse` / `or` | `\|\|` | both Bool |
| Add (int) | `+` | `+` | both Int |
| Add (float) | `+` | `+.` | both Float |
| Subtract (int/float) | `-` / `-` | `-` / `-.` | |
| Multiply (int/float) | `*` / `*` | `*` / `*.` | |
| Divide (int) | `div` | `/` | both Int |
| Remainder | `rem` | `%` | both Int |
| Concatenate | n/a | `<>` | both String |
| Pipe | n/a | `\|>` | see pipe section |

### Pipe
Erlang: `X1 = trim(Input), X2 = csv:parse(X1, <<",">>), ledger:from_list(X2).`
Gleam: `input |> trim |> csv.parse(",") |> ledger.from_list`

### Constants
Erlang `-define(the_answer, 42).` (module-local macros) → Gleam `const the_answer = 42`
Gleam constants ARE referenceable from other modules (`other_module.the_answer`).

### Blocks
Erlang `begin ... end` / parens → Gleam `{ ... }` braces.

### Data types
- **Strings**: Erlang `<<"..."/utf8>>` → Gleam `"..."`. All Gleam strings are UTF-8 binaries.
- **Tuples**: Erlang `{"a","b",10}` → Gleam `#("a","b",10)`. Pattern: `{_,P,_}` → `#(_, p, _)`. Tuples are the ONLY mixed-type collection in Gleam.
- **Lists**: same perf semantics; Erlang allows mixed types, Gleam does NOT. Cons: Erlang `[1 \| List0]` → Gleam `[1, ..list]`. Pattern `[1, Second \| _]` → `[1, second_element, ..]`. `[1.0, ..list]` is a type error in Gleam.
- **Atoms**: Erlang atoms created ad-hoc (`my_new_var`, `{ok, true}`, `{error, false}`). Gleam: atoms must be defined as variants in a custom type first. Exceptions: `ok`/`error` (built into Result) and booleans. `Ok(True)` / `Error(False)` are constructors. Atoms otherwise rarely used in Gleam.
- **Dicts** (Erlang maps): Erlang `#{k => v}` allows mixed key/value types. Gleam `gleam/dict` Dict requires uniform key type and uniform value type. NO dict literal syntax, NO pattern matching on dicts. `dict.from_list([#("k","v")])`. Dicts uncommon in Gleam; custom types preferred.

### Type aliases
Erlang `-type scores() :: list(integer()).` → Gleam `pub type Scores = List(Int)`

### Custom types
- **Records**: Erlang `-record(person, {age, name})` → Gleam `type Person { Person(age: Int, name: String) }`. Access: `Person#person.name` → `person.name`. Gleam custom types do NOT compile to Erlang records.
- **Unions**: Erlang functions can return int OR float dynamically. Gleam requires a single return type; unions must be wrapped: `type IntOrFloat { AnInt(Int) AFloat(Float) }`.
- **Opaque**: Erlang `-opaque` is documentation-only (other modules can still introspect). Gleam `pub opaque type` does NOT export constructors, enforcing the abstraction at compile time.

## Migration pitfalls / mental-model traps

1. **Types are everywhere and enforced** — Erlang's `-spec` is optional/dialyzer-soft; Gleam's annotations are always checked and must be correct or compilation fails. You cannot ship code with wrong/missing types.
2. **No atoms-as-tags** — Erlang's ad-hoc atoms (`my_new_var`, `{ok, val}`, `{error, reason}`) cannot be created inline in Gleam. You must declare a `type` with variants first. The only built-in atoms are `Ok`/`Error` (Result) and `True`/`False` (Bool).
3. **No Erlang records** — Gleam custom types do NOT compile to Erlang records despite similar syntax. Do not assume record-style tuple introspection from Erlang side when interoping.
4. **Result vs tuple-returns** — Erlang idiom `{ok, Value}` / `{error, Reason}` maps to Gleam `Result(Value, Reason)` with `Ok(value)` / `Error(reason)` constructors. The page shows `{ok, true}` → `Ok(True)` and `{error, false}` → `Error(False)`. NOTE: the page does NOT explicitly discuss Result as a type or its handling functions; only the atom→constructor mapping is shown.
5. **No multiple function heads** — Erlang multi-clause heads (`f(1)->...; f(2)->...`) must become a single function with a `case` expression. No function overloading either.
6. **No hot code reload assumptions** — NOT covered on this page. (Gap; see erlang-interop docs.)
7. **Pattern matching** — partial patterns require explicit `let assert` (compile error otherwise, unlike Erlang's silent runtime assert). No multiple-head pattern dispatch.
8. **List/tuple/map differences** — Lists are homogeneous (Erlang allows mixed). Tuples are the only mixed-type collection. Maps→Dict: homogeneous keys+values, no literal, no pattern match. Prefer custom types over dicts.
9. **Operator type strictness** — `==`/`!=` require same type (no Erlang loose `==`). Int vs Float ops are distinct operators (`+` vs `+.`, `>` vs `>.`, etc.). `div`→`/`, `rem`→`%`.
10. **Variable casing flipped** — Erlang capitals = variables; Gleam lowercase = variables. Capitals now denote type/variant constructors.
11. **Shadowing is legal** — Erlang single-assignment runtime error becomes a Gleam `let` rebind. Mental model of "bound once" no longer holds.
12. **Labelled args are zero-cost + checked** — Erlang's map-based labelled args have runtime cost and no compile checks; Gleam's are optimised to plain calls and fully type-checked. Do not assume the Erlang map-arg performance penalty.
13. **Opaque is enforced** — Erlang `-opaque` is advisory; Gleam `pub opaque` hides constructors. Cross-module introspection of opaque internals is not possible via the public API.

## What NOT to assume from Erlang
- Do NOT assume ad-hoc atoms work — they must be declared as type variants.
- Do NOT assume `-spec` is optional/soft — type annotations are mandatory-checked.
- Do NOT assume multiple function heads or overloading exist.
- Do NOT assume lists/maps can hold mixed types.
- Do NOT assume dict literal syntax or dict pattern matching.
- Do NOT assume custom types compile to Erlang records (they don't).
- Do NOT assume `-opaque` is advisory — Gleam enforces it.
- Do NOT assume loose equality (`==` comparing int to float) works.
- Do NOT assume `div`/`rem` operators — use `/` and `%`.
- Do NOT assume `begin...end` — use `{ }`.
- Do NOT assume macros (`-define`) — use `const`.
- Do NOT assume single-assignment — Gleam allows shadowing.

## OTP interop notes (gleam_otp/gleam_erlang)
**NOT covered on this page.** The cheatsheet is purely a syntax/type translation
reference and contains no content on:
- `gleam_otp` (Actor, Supervisor, OTP behaviours)
- `gleam_erlang` (Erlang stdlib interop)
- message passing / `receive` / process mailboxes
- hot code reloading
- `gen_server`/`supervisor` translation
- `Result` type and its handling functions (only the `Ok`/`Error` constructor mapping is shown)

The page ends with empty stub headers: "Modules", "Imports", "Nested modules",
"First class modules" — these have no body content as of fetch and likely
intended to cover module/interop topics but are unimplemented.

For OTP interop, consult `gleam_otp` and `gleam_erlang` package docs directly
(not crawled in this step).

## Review concerns
- Page is incomplete: trailing section headers (Modules/Imports/Nested modules/First class modules) have no content. Anyone relying on this cheatsheet for module or interop guidance will find nothing.
- No version pinning on the page; content may change. Re-fetch periodically.
- Result type is shown only via atom→constructor mapping; the `Result(a,b)` type itself, `result` module functions, and `try`/`case`-on-Result patterns are NOT explained here. Misleading to treat this page as a Result reference.
- Operator table is dense; verify int-vs-float operator selection (`>` vs `>.`) carefully when porting Erlang numeric code — silent type errors in Erlang become compile errors in Gleam.
- `let assert` is a footgun: it preserves Erlang's partial-pattern runtime-crash semantics but looks like safe Gleam code. Review for exhaustive `case` preference.

## Verbatim quotes
- "In Erlang variables are written with a capital letter, and can only be assigned once." / "In Gleam variables are written with a lowercase letter, and names can be reassigned."
- "In Gleam, the `let assert` keyword is used to make assertions using partial patterns." / "let [element] = some_list // Compile error! Partial pattern"
- "Unlike in Erlang these type annotations will always be checked by the compiler and have to be correct for compilation to succeed."
- "Unlike Erlang (but similar to Core Erlang) Gleam does not support multiple function heads, so to pattern match on an argument a case expression must be used."
- "Gleam does not support function overloading, so there can only be 1 function with a given name, and the function can only have a single implementation for the types it accepts."
- "Gleam has a single namespace for value and functions within a module, so there is no need for a special syntax to assign a module function to a variable."
- "There is no performance cost to Gleam's labelled arguments as they are optimised to regular function calls at compile time, and all the arguments are fully type checked."
- "All strings in Gleam are UTF-8 encoded binaries."
- "Tuples are very useful in Gleam as they're the only collection data type that allows for mixed types of elements in the collection."
- "Lists in Erlang are allowed to be of mixed types, but not in Gleam. They retain all of the same performance semantics."
- "In Erlang atoms can be created as needed, but in Gleam all atoms must be defined as values in a custom type before being used."
- "In general, atoms are not used much in Gleam, and are mostly used for booleans, `ok` and `error` result types, and defining custom types."
- "In Erlang, maps can have keys and values of any type... In Gleam, maps are called Dict (Dictionary) and provided by the standard library... all keys must be of the same type... all values must be of the same type." / "There is no dict literal syntax in Gleam, and you cannot pattern match on a dict. Dicts are generally not used much in Gleam, custom types are more common."
- "Gleam does not have anything called a `record`, but custom types can be used in Gleam in much the same way that records are used in Erlang, even though custom types don't actually define a `record` in Erlang when it is compiled."
- "In Erlang a function can take or receive values of multiple different types... In Gleam functions must always take and receive one type. To have a union of two different types they must be wrapped in a new custom type."
- "In Erlang the `opaque` attr... is purely for documentation purposes and other modules can introspect and manipulate opaque types any way they wish." / "In Gleam custom types can be defined as being opaque, which causes the constructors for the custom type not to be exported from the module."

## Version notes
- No Gleam version stated on page.
- Page copyright footer: "© 2026 Louis Pilfold".
- Fetched 2026-06-25; HTTP 200; no redirects (canonical == seed URL).

## Discovered links

### Relevant (crawl later)
- (none beyond site nav already covered) — page body contains no inline content links to other docs/cheatsheets. Trailing stub sections (Modules/Imports/Nested modules/First class modules) imply future content but provide no links.

### Skipped
- /news, /community, /documentation, /install, /roadmap, /sponsor, /case-studies (site nav, out of scope)
- https://tour.gleam.run , https://playground.gleam.run , https://packages.gleam.run , https://gleamweekly.com , https://discord.gg/Fm8Pwmy , https://shop.gleam.run , https://github.com/gleam-lang (external, out of scope)
- /styles/main.css, /images/lucy/lucy.svg, /javascript/main.js (assets)
