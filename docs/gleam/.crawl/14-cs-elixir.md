# Crawl: cheatsheets/gleam-for-elixir-users/
- seed_url: https://gleam.run/cheatsheets/gleam-for-elixir-users/
- canonical_url: https://gleam.run/cheatsheets/gleam-for-elixir-users/
- family: Gleam cheatsheets
- fetch: 200
- gleam_version: not specified on page
- feeds_docs: conventions-patterns-antipatterns.md, code-review workflow

## Purpose
A side-by-side cheat sheet for Elixir developers learning Gleam. Translates
Elixir syntax and semantics (comments, variables, matching, functions, modules,
operators, constants, blocks, data types, strings, tuples, lists, atoms, dicts,
custom types/records, patterns, unions) into the equivalent Gleam construct,
calling out where the two languages diverge in type strictness, namespacing,
overloading, and runtime representation.

## Translation tables (Elixir → Gleam)

### Comments
| Concept | Elixir | Gleam |
|---|---|---|
| Line comment | `#` | `//` |
| Doc-following item | — | `///` |
| Doc-current module | — | `////` |

### Variables & binding
| Concept | Elixir | Gleam |
|---|---|---|
| Assignment | `size = 50` | `let size = 50` |
| Reassign / shadow | `size = size + 100` | `let size = size + 100` |
| Type annotation | none (dynamic) | `let some_list: List(Int) = [1, 2, 3]` |

### Match operator
| Concept | Elixir | Gleam |
|---|---|---|
| Pattern bind | `[x, y] = [1, 2]` | `let [x, y] = [1, 2]` (compile-checked) |
| Assertion | `2 = y` | `let assert 2 = y` |
| Type mismatch | runtime error | compile error |
| Value mismatch | runtime error | runtime error (or compile via `let assert`) |

### Functions
| Concept | Elixir | Gleam |
|---|---|---|
| Named fn | `def sum(x, y) do ... end` | `pub fn sum(x, y) { ... }` |
| Anonymous fn | `fn(x, y) -> x * y end` | `fn(x, y) { x * y }` |
| Call anon fn | `mul.(1, 2)` | `mul(1, 2)` (no `.`) |
| Public by default | `def` is public | functions are **private** by default |
| Private | `defp` | omit `pub` |
| Type spec | `@spec sum(number, number) :: number` (docs only) | `fn add(x: Int, y: Int) -> Int` (enforced) |
| Multiple heads | `def zero?(0)... def zero?(x)...` | NOT supported — use `case` |
| Overloading | supported via arity/heads | NOT supported — one impl per name |
| Reference fn | `&identity/1` | `identity` (single namespace) |
| Labelled args | keyword list `replace(each: ",", with: " ", inside: ...)` | `fn replace(inside string, each pattern, with replacement)` |
| Labelled perf | runtime penalty, no compile checks | zero cost, fully type-checked |

### Operators
| Concept | Elixir | Gleam | Notes |
|---|---|---|---|
| Equal | `==` | `==` | same type required |
| Strictly equal | `===` | `==` | Gleam always strict |
| Not equal | `!=` | `!=` | same type required |
| Greater (int) | `>` | `>` | ints only |
| Greater (float) | `>` | `>.` | floats only |
| Greater eq (int) | `>=` | `>=` | ints only |
| Greater eq (float) | `>=` | `>=.` | floats only |
| Less (int) | `<` | `<` | ints only |
| Less (float) | `<` | `<.` | floats only |
| Less eq (int) | `<=` | `<=` | ints only |
| Less eq (float) | `<=` | `<=.` | floats only |
| Boolean and | `and` | `&&` | bools only |
| Logical and | `&&` | — | not available |
| Boolean or | `or` | `||` | bools only |
| Logical or | `||` | — | not available |
| Add (int) | `+` | `+` | ints only |
| Add (float) | `+` | `+.` | floats only |
| Subtract (int) | `-` | `-` | ints only |
| Subtract (float) | `-` | `-.` | floats only |
| Multiply (int) | `*` | `*` | ints only |
| Multiply (float) | `*` | `*.` | floats only |
| Int divide | `div` | `/` | ints only |
| Float divide | `/` | `/.` | floats only |
| Remainder | `rem` | `%` | ints only |
| Concatenate | `<>` | `<>` | strings only |
| Pipe | `|>` | `|>` | Gleam pipes into anon fns too |

### Constants
| Concept | Elixir | Gleam |
|---|---|---|
| Module attr literal | `@the_answer 42` | `const the_answer = 42` |
| Cross-module ref | not supported (module-scoped) | `pub const the_answer: Int = 42` then `other_module.the_answer` |

### Blocks
| Concept | Elixir | Gleam |
|---|---|---|
| Group expressions | `(...)` parens | `{ ... }` braces |
| Arithmetic reorder | `x * (x + 10)` | `x * { x + 10 }` |
| `do`/`end` | macro keyword arg only | not present |

### Data types
| Concept | Elixir | Gleam |
|---|---|---|
| String | `"Hellø, world!"` (UTF-8 binary) | `"Hellø, world!"` (UTF-8 binary) |
| Tuple literal | `{"username", "password", 10}` | `#("username", "password", 10)` |
| Tuple destructure | `{_, password, _} = t` | `let #(_, password, _) = t` |
| List literal | `[2, 3, 4]` | `[2, 3, 4]` |
| Cons / prepend | `[1 | list]` | `[1, ..list]` |
| List pattern tail | `[1, second | _]` | `[1, second, ..]` |
| Mixed-type list | allowed | compile error |
| Atom | `:my_new_var` | must be a custom type variant: `type MyNewType { MyNewVar }` |
| Bool atoms | `true`/`false` are atoms | `True`/`False` are built-in bools |
| Result tuples | `{:ok, v}` / `{:error, e}` | `Ok(v)` / `Error(e)` of `Result(_, _)` |
| Map literal | `%{"k" => "v"}` | none — `dict.from_list([#("k", "v")])` |
| Mixed map | allowed | compile error (uniform K/V types) |
| Dict pattern match | allowed on maps | NOT possible on dicts |

### Custom types / records
| Concept | Elixir | Gleam |
|---|---|---|
| Struct | `defmodule Person do defstruct name: "John", age: 35 end` | `type Person { Person(name: String, age: Int) }` |
| Construct | `%Person{name: "Jake"}` | `Person(name: "Jake", age: 35)` |
| Field access | `person.name` | `person.name` |
| Erlang Record | `Record.defrecord(...)` (rare) | custom types compile to tuple/record repr |
| Runtime repr | Map | tuple (Erlang-record compatible) |

### Modules
| Concept | Elixir | Gleam |
|---|---|---|
| Define module | `defmodule Wibble do ... end` | file = module (named by path) |
| Multiple modules/file | allowed | one module per file |
| Import | `alias Wibble` etc. | `import Wibble` (path-based, e.g. `lib/Wibble`) |

### Patterns
| Concept | Elixir | Gleam |
|---|---|---|
| Bind sub-pattern name | `1 = first` (in list) | `1 as first` |
| Record pattern | `%Person{name: "Jack", age: 20} = p` | `Person(name: "Jack" as name, age: 20 as age)` |
| Tuple pattern | `{1 = a, 2 = b} = t` | `#(1 as a, 2 as b)` |

## Migration pitfalls / mental-model traps

1. **No `nil`.** Elixir code that leans on `nil` (defaults, optionals, nil-checks)
   has no direct translation. Optionality must be modelled with `Result` or
   `Option`-style custom types; there is no implicit nil fallback like
   `opts[:k] || default`.

2. **No macros / no metaprogramming.** `defmacro`, `quote`/`unquote`, `use`,
   `@behaviour`-driven macro injection, and compile-time code generation do not
   exist. Anything built on macros (`Plug`, `Ecto` macros, `use Foo`) must be
   re-expressed as plain function calls or external (FFI) modules.

3. **No `use`.** There is no `use` directive; behaviour/contract sharing is via
   plain imports and explicit function definitions.

4. **No function overloading / no multiple heads.** Elixir's multi-clause
   `def f(0)... def f(x)...` pattern is invalid in Gleam. Each function has
   exactly one head; dispatch on argument values must use `case`. This is the
   single biggest refactor when porting Elixir functions.

5. **Single value/function namespace.** Gleam has one namespace per module for
   both values and functions. There is no `&module_fun/arity` capture syntax and
   no `anon_fn.()` call distinction — all calls use `f(args)`.

6. **Types are real and enforced.** Elixir `@spec` is documentation (Dialyzer
   is optional and permissive). Gleam annotations are checked by the compiler
   and a wrong annotation is a compile error. Lists, dicts, and tuples enforce
   uniform element types — `[1.0, ..int_list]` is a compile error, not a
   runtime surprise.

7. **Atoms must be declared.** You cannot mint `:foo` on the fly; every atom
   is a variant of a custom type (with built-in exceptions `Ok`, `Error`,
   `True`, `False`). Code that uses atoms as ad-hoc tags/enums must introduce a
   `type`.

8. **Result vs exceptions.** Gleam favours `Result(a, b)` values over
   raise/rescue. `let assert` is the escape hatch for assertions that crash on
   mismatch — it is not the default binding form. Porting `{:ok, _} = ...` to
   plain `let` will compile only when the type is statically known; otherwise
   use `let assert` or handle the `Result`.

9. **Operators are type-split.** `>` is int-only, `>.` is float-only; `+` int,
   `+.` float; `div`→`/`, `/`→`/.`, `rem`→`%`. There is no `&&`/`||` truthiness
   — `&&`/`||` in Gleam are strict bool `and`/`or`. Elixir's truthy logical
   operators have no equivalent.

10. **Strict equality only.** `===` collapses to `==`; there is no loose
    equality. Both operands must share a type.

11. **Dicts are not maps.** No map literal, no pattern matching on dicts, and
    keys/values must be uniformly typed. Custom types are preferred over dicts
    for structured data.

12. **Modules are files.** One module per file, named by path. Multiple
    `defmodule` blocks in one Elixir file must be split across files.

13. **Private by default.** Functions are private unless marked `pub`; Elixir's
    `def` is public by default. Porting will silently hide functions that were
    meant to be public.

14. **Labelled args are not keyword lists.** Gleam labelled args
    (`inside string`) are compile-checked and zero-cost; they are not the
    dynamically-typed keyword-list `opts` pattern. `opts \ []` defaults and
    `Keyword.get` have no direct analogue.

15. **Cons syntax differs.** `[head | tail]` (Elixir) → `[head, ..tail]`
    (Gleam); tail pattern `| rest` → `, ..rest` / `, ..`.

16. **`?` not allowed in names.** Elixir `zero?` must be renamed (e.g.
    `is_zero`); Gleam identifiers cannot contain `?`.

## What NOT to assume from Elixir

- Do not assume dynamic typing or permissive `@spec` — types are enforced at
  compile time and annotations are checked.
- Do not assume `nil` exists or that `||` provides nil-defaults.
- Do not assume macros, `use`, `quote`, or compile-time code generation exist.
- Do not assume function overloading or multiple function heads are permitted.
- Do not assume atoms can be created ad hoc — declare a custom type.
- Do not assume mixed-type lists/maps are valid.
- Do not assume `def` is private/public the same way — Gleam is private by
  default.
- Do not assume truthiness or `&&`/`||` short-circuit on non-bools.
- Do not assume one operator works for ints and floats (`>` vs `>.`, `+` vs
  `+.`, `div`→`/`).
- Do not assume map literals or pattern matching on maps/dicts.
- Do not assume multiple modules per file.
- Do not assume `&fun/arity` capture syntax or `anon.()` call syntax.
- Do not assume `do`/`end` blocks for grouping — use `{ }`.
- Do not assume `@moduledoc`/`@doc` — use `////`/`///` comments.
- Do not assume keyword-list `opts` with `\ []` defaults — use labelled args
  or explicit `Result`/custom-type config.

## Review concerns (code that looks imported from Elixir)

When reviewing Gleam code that may have been ported from Elixir, watch for:

- Multi-clause functions attempting Elixir-style heads → must collapse to one
  head + `case`.
- Use of `let assert` where a `Result` should be handled explicitly (silent
  crash import).
- `Ok`/`Error` treated as bare atoms rather than `Result` values.
- Ad-hoc atoms that should be a `type` with variants.
- Mixed-type lists or dicts that slipped past mental model (will be compile
  errors, but review intent).
- `&&`/`||` used expecting truthiness rather than strict bool.
- Int/float operator confusion (`>` vs `>.`, `+` vs `+.`, `/` vs `/.`).
- Functions missing `pub` that were public in Elixir (or vice versa).
- `use`-style or macro-style patterns that have no Gleam equivalent.
- `opts \ []` / keyword-list patterns that should be labelled args or config
  types.
- `[h | t]` cons syntax instead of `[h, ..t]`.
- Identifiers containing `?` or `!`.
- Multiple `defmodule`-equivalent blocks assumed in one file.
- `nil`-based defaults or nil-checks instead of `Option`/`Result`.
- Map literals or dict pattern matching attempts.

## Verbatim quotes

- "In Gleam, `let` and `=` can be used for pattern matching, but you'll get
  compile errors if there's a type mismatch, and a runtime error if there's a
  value mismatch. For assertions, the equivalent `let assert` keyword is
  preferred."
- "These type annotations will always be checked by the compiler and throw a
  compilation error if not valid."
- "Unlike Elixir, Gleam does not support function overloading, so there can
  only be 1 function with a given name, and the function can only have a single
  implementation for the types it accepts."
- "Gleam has a single namespace for value and functions within a module, so
  there is no need for a special syntax to assign a module function to a
  variable."
- "In Gleam all functions are called using the same syntax."
- "There is no performance cost to Gleam's labelled arguments as they are
  optimised to regular function calls at compile time, and all the arguments
  are fully type checked."
- "Lists in Elixir are allowed to be of mixed types, but not in Gleam."
- "In Elixir atoms can be created as needed, but in Gleam all atoms must be
  defined as values in a custom type before being used."
- "In Gleam, maps are called Dict (Dictionary) and provided by the standard
  library. Dicts can have keys and values of any type, but all keys must be of
  the same type in a given dict and all values must be of the same type in a
  given dict."
- "There is no dictionary literal syntax in Gleam, and you cannot pattern
  match on a dict. Dicts are generally not used much in Gleam, custom types are
  more common."
- "Gleam's file is a module and named by the file name (and its directory
  path). Since there is no special syntax to create a module, there can be only
  one module in a file."
- "In Gleam we use `as` to name the variable, same as using `=` in Elixir."
- "In Elixir functions defined by `def` are public by default, while ones
  defined by `defp` are private." / "In Gleam functions are private by default
  and need the `pub` keyword to be public."

## Version notes
- Page does not state a Gleam version. Syntax shown (`let assert`, labelled
  args, `pub`, `type`, `const`, `#(...)` tuple syntax, `[h, ..t]` cons) is
  consistent with modern Gleam (1.x).
- No build/version metadata embedded in the page HTML.

## Discovered links

### Relevant (crawl later)
- https://gleam.run/documentation/  (main docs hub — likely already crawled as 01-documentation)
- https://tour.gleam.run  (interactive tour — external host)
- https://packages.gleam.run/  (package index — external host)
- https://playground.gleam.run  (playground — external host)
- https://gleam.run/feed.xml  (news feed)

### Skipped
- In-page anchors: #comments #variables #match-operator #variables-type-annotations
  #functions #exporting-functions #function-type-annotations #function-heads
  #function-overloading #referencing-function #calling-anonymous-functions
  #labelled-arguments #modules #operators #constants #blocks #data-types
  #strings #tuples #lists #atoms #dicts #custom-types #records #patterns #unions
- Site nav: / /install /news /community /case-studies /roadmap /sponsor
  /styles/main.css?v=... /images/lucy/lucy.svg
- External community: https://discord.gg/Fm8Pwmy
  https://github.com/gleam-lang https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md
  https://gleamweekly.com/ https://shop.gleam.run/en-gbp
