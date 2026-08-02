# Crawl: tour.gleam.run/everything/
- seed_url: https://tour.gleam.run/everything/
- canonical_url: https://tour.gleam.run/everything/
- family: Gleam language tour
- fetch: 200
- gleam_version: not present on page (no version string in HTML)
- feeds_docs: language-fundamentals.md, types-records-and-patterns.md, functions-pipelines-and-use.md, result-option-and-errors.md

## Purpose
The "Everything!" page is the whole Gleam language tour rendered on a single
page. It is an interactive introduction and reference to the Gleam programming
language, organised as 63 sequential lessons grouped into chapters: `basics`,
`functions`, `flow-control`, `data-types`, `standard-library`,
`advanced-features`. Each lesson pairs prose explanation with a runnable code
snippet. This file extracts the LANGUAGE FUNDAMENTALS (syntax + key rules)
needed to author the language-fundamentals / types-records-and-patterns /
functions-pipelines-and-use / result-option-and-errors doc pages.

## Types & values

### Built-in types
- `Int` — whole numbers. On the BEAM ints have no max/min size; on JavaScript
  they are represented as JS 64-bit floating point numbers.
  - Operators: `+ - / * %` (arithmetic), `> < >= <=` (comparison).
  - Equality `==` / `!=` works on any type (both sides must be the same type).
  - Number formats: underscores for clarity (`1_000_000`); binary/octal/hex
    prefixes `0b` / `0o` / `0x` (`0b00001111`, `0o17`, `0xF`).
  - Stdlib module: `gleam/int`.
- `Float` — non-integer numbers. 64-bit floating point on both runtimes.
  - Operators are NOT overloaded: dedicated float operators
    `+. -. /. *.` (arithmetic) and `>. <. >=. <=.` (comparison).
  - JS runtime: overflow -> `Infinity` / `-Infinity`; dividing two infinities
    yields `NaN`. BEAM: overflow raises an error; no `NaN`/`Infinity` float.
  - Division by zero is NOT an error — defined to be zero.
  - Scientific notation: `7.0e7`, `3.0e-4`.
  - Stdlib module: `gleam/float`.
- `String` — double-quoted, may span multiple lines and contain unicode.
  - Concatenation operator: `<>`.
  - Escape sequences supported; `\u{1F600}` unicode escape; `\"` escaped quote.
  - Stdlib module: `gleam/string`.
- `Bool` — either `True` or `False`.
  - Operators: `||` (or), `&&` (and), `!` (not).
  - `||` and `&&` are short-circuiting (RHS not evaluated if LHS determines
    result).
  - Stdlib module: `gleam/bool`.
- `Nil` — Gleam's unit type. Returned by functions that have nothing else to
  return (all functions must return something). `Nil` is NOT a valid value of
  any other type -> values are not nullable. If a value's type is `Nil` it is
  the value `Nil`; if it is another type it is never `Nil`.
- `Result(value, error)` — built-in type for computations that can succeed or
  fail (see Result/Option philosophy below). Variants: `Ok(value)` and
  `Error(error)`. Generic with two type parameters.
- `Option(inner)` — defined in `gleam/option` (not built-in but standard).
  Variants: `Some(inner)` and `None`. Represents a present/absent value.

### Equality
- `==` and `!=` work with values of any type, but both sides must be the same
  type. Equality is STRUCTURAL (same structure, not same memory location).

### Variables, `let`, const
- `let` assigns a value to a variable. Variable and function names are
  `snake_case`.
- Values are IMMUTABLE. Variable names can be reused by later `let` bindings,
  but the values themselves are never mutated. Re-binding `let x = "New"`
  shadows; earlier bindings (e.g. `let y = x`) still refer to the original.
- Discard pattern: a variable assigned but unused emits a warning; prefix the
  name with `_` (e.g. `_score`) to silence it.
- Type annotations on `let`: `let _name: String = "Gleam"`. Annotations are
  documentation only — they do not change type checking beyond ensuring the
  annotation is correct. Typically Gleam code omits them for assignments.
- `const` — top-level module constants. Must be literal values (functions
  cannot be used in their definitions). May have a type annotation:
  `const ints: List(Int) = [1, 2, 3]`. Can be `pub`.

### Blocks
- Blocks group expressions with `{ }`. Each expression evaluated in order;
  value of the last expression is returned. Variables assigned in a block are
  scoped to the block. Blocks can change evaluation order of binary operators:
  `{ 1 + 2 } * 3` (like parentheses in other languages).

### `echo`
- `echo` is a debug-print keyword that prints a value of ANY type (unlike
  `io.println` which only accepts strings). Can be used mid-pipeline:
  `|> echo`.

## Functions (labelled args, defaults, external functions)

### Definition
- `fn` keyword defines functions. Body is expression-based: each expression
  evaluated in order, value of the last expression returned. NO `return`
  operator.
- Without `pub` a function is private (module-local).
- Type annotations on arguments and return values are optional but considered
  good practice: `fn double(a: Int) -> Int { ... }`.
- Functions are VALUES: can be assigned to variables, passed as arguments,
  returned. The `fn` keyword also describes function types:
  `fn(Int) -> Int`.

### Higher-order & anonymous functions
- Anonymous function literals: `fn(a) { a + 1 }`. Interchangeable with named
  functions. Anonymous functions capture in-scope variables -> closures.
- Function captures (shorthand): `some_function(..., _, ...)` is sugar for
  `fn(a) { some_function(..., a, ...) }`. The `_` is a placeholder for the
  single argument. e.g. `add(1, _)` == `fn(x) { add(1, x) }`.

### Generic functions
- Generics (parametric polymorphism) via type variables written with a
  lowercase name. A type variable stands for one specific type per call site
  (NOT an `any` type); it is replaced with a concrete type each call.
  ```gleam
  fn twice(argument: value, my_function: fn(value) -> value) -> value {
    my_function(my_function(argument))
  }
  ```

### Labelled arguments
- Arguments can be given an external label before their internal name:
  `fn calculate(value: Int, add addend: Int, multiply multiplier: Int) { ... }`
  — here `add`/`multiply` are labels, `addend`/`multiplier` internal names.
- When labelled args are used, ORDER DOES NOT MATTER, but all unlabelled
  arguments must come before labelled arguments.
- Labels are OPTIONAL when calling: `calculate(1, 2, 3)` or
  `calculate(1, add: 2, multiply: 3)` or
  `calculate(1, multiply: 3, add: 2)`.
- No performance cost (no runtime dict/allocation).
- Label shorthand syntax: when a local variable has the same name as a label,
  the variable name can be omitted: `calculate_total_cost(quantity:, unit_price:, discount:)`.
  Shorthand also works for record constructor arguments.
- NOTE: Gleam has NO default argument values. Optionality is expressed via
  `Option`/`Result`, not default parameters.

### Pipelines (`|>`)
- Pipe operator takes the left-hand expression result and passes it as an
  argument to the function on the right, enabling top-to-bottom reading.
- First tries to use the LHS as the FIRST argument: `a |> b(1, 2)` -> `b(a, 1, 2)`.
  If that doesn't typecheck, falls back to calling the RHS result as a
  function: `b(1, 2)(a)`.
- Convention: write the "subject" as the first argument to make piping natural.
  To pipe to a different position use a function capture:
  `|> string.append("3", _)`.
- Debug mid-pipeline with `|> echo`.

### Documentation, deprecations
- Comments: `//` line comments (go on the line BEFORE the item they describe).
- Doc comments: `///` for types/functions (immediately before the item);
  `////` for module docs (top of module).
- `@deprecated("Use new_function instead")` attribute emits a warning when the
  deprecated definition is referenced.

### External functions & types
- External TYPE: a custom type with NO constructors — Gleam only knows it
  exists, not its shape: `pub type DateTime`.
- External FUNCTION: `@external` attribute directs the compiler to use a
  specified module function as the implementation instead of Gleam code:
  ```gleam
  @external(javascript, "./my_package_ffi.mjs", "now")
  pub fn now() -> DateTime
  ```
- Type annotations are MANDATORY for external functions (compiler cannot infer
  foreign types). Gleam trusts the annotation — inaccurate types cause runtime
  crashes. Use sparingly; prefer Gleam code.
- Multi-target externals: multiple `@external` lines for different targets:
  ```gleam
  @external(erlang, "calendar", "local_time")
  @external(javascript, "./my_package_ffi.mjs", "now")
  pub fn now() -> DateTime
  ```
  If no implementation exists for the compiled target -> compiler error.
- External Gleam fallbacks: a function may have BOTH a Gleam body and an
  `@external` impl; the external is used when available for the target,
  otherwise the Gleam body:
  ```gleam
  @external(erlang, "lists", "reverse")
  pub fn reverse_list(items: List(e)) -> List(e) {
    tail_recursive_reverse(items, [])
  }
  ```

## Collections (lists, tuples)

### Lists
- `List` is a generic type: `List(Int)`, `List(String)`.
- IMMUTABLE singly-linked lists. Efficient to add/remove from the FRONT.
  Counting length / indexing other positions is expensive and rarely done.
- Literal: `[1, 2, 3]`. Immutably prepend with spread: `[-1, 0, ..ints]`.
  All elements must be the same type (`["zero", ..ints]` errors if `ints` is
  `List(Int)`). Original list is unchanged.

### Tuples
- Combine multiple values of different types. Generic: `#(1, "Hi!")` has type
  `#(Int, String)`.
- Access without pattern matching: `some_tuple.0` (first), `some_tuple.1`
  (second), etc.
- Pattern match / destructure: `let #(a, _, _) = triple`.
- Most commonly used to return 2-3 values from a function; a custom type is
  often clearer.

## Custom types, records, generics, opaque

### Custom types
- Defined with `type` keyword + name + a constructor per VARIANT. Both type
  name and constructor names start with uppercase letters.
  ```gleam
  pub type Season {
    Spring
    Summer
    Autumn
    Winter
  }
  ```
- Variants are pattern matched with `case`.

### Records
- A variant that holds data is a record. Fields can be given labels (optionally
  used when constructing, like function argument labels).
- Single-variant custom type = Gleam's equivalent of a struct/object. The
  variant is often named the same as the type (not required).
  ```gleam
  pub type Person {
    Person(name: String, age: Int, needs_glasses: Bool)
  }
  // construction (labels optional)
  let amy = Person("Amy", 26, True)
  let jared = Person(name: "Jared", age: 31, needs_glasses: True)
  ```
- Record ACCESSORS: `record.field_label` gets a contained value. Accessible
  without a `case` ONLY when the field has the same name, position, and type
  across ALL variants. Other fields require the compiler to know the variant
  (e.g. after a `case` match).
- Record PATTERN MATCHING: extract multiple fields into variables. `let` can
  only destructure single-variant types (or when the variant is known after a
  `case`). Use `_` or `..` to discard unneeded fields:
  ```gleam
  case fish {
    Starfish(_, favourite_colour) -> ...
    Jellyfish(name, ..) -> ...
  }
  let IceCream(flavour) = ice_cream   // single-variant: `let` ok
  ```
- Record UPDATES: create a new record from an existing one with some fields
  changed (immutable — original unchanged):
  ```gleam
  let teacher2 = Teacher(..teacher1, subject: "PE", room: 6)
  ```

### Generic custom types
- Custom types take type parameters:
  ```gleam
  pub type Option(inner) {
    Some(inner)
    None
  }
  ```
  `gleam/option` defines this for real.

### Type aliases
- `pub type Number = Int` — a different NAME for the same type (does NOT make a
  new type). Type names always start with a capital letter. `pub` makes it
  public. Use RARELY: aliasing obscures the type and gives none of the safety
  of a custom type.

### Opaque types
- `pub opaque type` — the type itself is public (usable by other modules) but
  its constructors are PRIVATE (only the defining module can construct or
  pattern match on it). Enables SMART CONSTRUCTORS that enforce invariants:
  ```gleam
  pub opaque type PositiveInt {
    PositiveInt(inner: Int)
  }
  pub fn new(i: Int) -> PositiveInt {
    case i >= 0 { True -> PositiveInt(i); False -> PositiveInt(0) }
  }
  ```

## Pattern matching & case

### `case` expressions
- The most common flow control. Like `switch` but more powerful. Performs
  PATTERN MATCHING ("if the data has this shape then run this code").
- EXHAUSTIVENESS CHECKING: patterns must cover all possible values (compiler
  enforces; missing or redundant patterns are errors).
  ```gleam
  case x {
    0 -> "Zero"
    1 -> "One"
    _ -> "Other"
  }
  ```
- Variable patterns: a variable name in a pattern binds the matched value for
  use in that clause's body: `other -> "It is " <> int.to_string(other)`.
- Multiple subjects: `case x, y { 0, 0 -> ...; 0, _ -> ...; _, _ -> ... }` —
  same number of patterns as subjects.
- Alternative patterns with `|`: `2 | 4 | 6 | 8 -> "even"`. If a pattern defines
  a variable, ALL alternatives for that clause must define a variable with the
  same name and type. Nested alternative patterns are NOT valid
  (`[1 | 2 | 3]` is invalid).
- Pattern aliases with `as`: `[_, ..] as first` matches any non-empty list and
  binds that whole list to `first`.

### String patterns
- `<>` matches strings with a specific prefix:
  `"Hello, " <> name` matches any string starting with `"Hello, "` and binds
  the rest to `name`.

### List patterns
- Match on specific lengths: `[]` (empty), `[_]` (exactly one element).
- Spread `..` matches the rest: `[1, ..]` (starts with 1), `[_, _, ..]` (at
  least two elements), `[first, ..rest]` (head + tail).

### Tuple patterns
- `let #(a, _, _) = triple` destructuring; also usable in `case`.

### Guards
- `if` adds a GUARD to a case pattern — an expression that must evaluate to
  `True` for the clause to match:
  ```gleam
  case numbers {
    [first, ..] if first > limit -> first
    [_, ..rest] -> get_first_larger(rest, limit)
    [] -> 0
  }
  ```
- Guard expressions CANNOT contain function calls, case expressions, or
  blocks.

### Recursion (related to case)
- Gleam has NO loops; iteration is via recursion (top-level functions calling
  themselves). A recursive function needs >= 1 base case and >= 1 recursive
  case.
- Tail call optimisation: if a function call is the last thing a function does,
  the stack frame is reused (no memory growth). Rewrite with an ACCUMULATOR to
  make recursion tail-recursive; hide the accumulator behind a public wrapper
  calling a private recursive helper.
- List recursion idiom: `[first, ..rest]` + `[]` patterns to walk a list.

## use expressions & pipelines

### `use`
- Gleam has NO exceptions, macros, type classes, or early returns — it relies
  on first-class functions + pattern matching. `use` calls a function that
  takes a CALLBACK without increasing indentation.
- Everything below the `use` becomes an anonymous function; the assigned
  variables become the callback's arguments.
- `use a, b <- my_function` expands to
  `my_function(fn(a, b) { ...rest... })`.
- RHS should ideally be a regular function call (not a complex expression) for
  readability. Excessive `use` can make code unclear, especially for beginners.
- Primary use case: chaining fallible `Result`-returning functions with
  `gleam/result`:
  ```gleam
  use username <- result.try(get_username())
  use password <- result.try(get_password())
  use greeting <- result.map(log_in(username, password))
  greeting <> ", " <> username
  ```
  (equivalent to nested `result.try(..., fn(...) { ... })` calls).

### `gleam/result` module (used with `use`/pipelines)
- `result.map(result, fn)` — updates the Ok value via a function; skipped on
  Error.
- `result.try(result, fn)` — runs a result-returning fn on the Ok value; chains
  fallible calls, stopping at first Error.
- `result.unwrap(result, default)` — extracts the success value or returns a
  default.
- Often pipelined: `int.parse("-1234") |> result.map(...) |> result.try(...)`.

## todo / panic / assert

### `todo`
- Marks unimplemented code. `todo as "message"` (message optional). Compiler
  prints a warning; running crashes with the message.

### `panic`
- Crashes the program when an unreachable point is reached. `panic as "msg"`.
  Should almost NEVER be used (prototypes/scripts only); in libraries it
  signals a design flaw — prefer types that make invalid states
  unrepresentable.

### `let assert`
- Like `let` but the pattern may be PARTIAL (need not cover all values). Crashes
  if the pattern fails to match. `as` supplies a custom message:
  `let assert [first, ..] = items as "List should not be empty"`. Use sparingly;
  avoid in libraries.

### Bool `assert`
- For test assertions: asserts a boolean evaluates to `True`. Crashes on
  `False`. `assert expr as "message"` (message optional, else generic
  "Assertion failed"). Designed for TEST code; avoid in apps/libraries.
  ```gleam
  assert add(1, 2) == 3
  assert add(6, 2) == add(2, 6) as "Addition should be commutative"
  ```

## Bit arrays
- Represent a sequence of 1s and 0s; convenient syntax for binary data.
- Literal: `<<3>>` (8-bit int; == `<<3:size(8)>>`). `<<6147:size(16)>>` (16-bit).
  UTF8 segment: `<<"Hello, Joe!":utf8>>`.
- Concatenation: `<<first:bits, second:bits>>`.
- Each segment can take options; multiple options separated by dashes:
  `x:unsigned-little-size(2)`.
- LIMITED support when compiling to JavaScript (not all options available; full
  support planned). See Erlang bit syntax docs for details.

## Modules / imports / annotations
- Code is organised into MODULES (a bunch of definitions that belong together).
  Module name comes from the file path: `gleam/io` is `io.gleam` in a `gleam`
  directory.
- `import gleam/io` — the imported module is referred to by its LAST path
  segment (`io`).
- `as` renames a module: `import gleam/string as text`.
- Qualified vs unqualified imports: normally use qualified (`io.println(...)`).
  Unqualified imports list functions: `import gleam/io.{println}` — then
  `println(...)` works. Qualified imports are preferred for clarity.
- Type imports: types can be qualified (`bytes_tree.BytesTree`) or unqualified
  via `import gleam/string_tree.{type StringTree}`. Types are commonly imported
  unqualified (unlike functions).
- Type annotations: optional on `let` and function args/returns; documentation
  only (do not change type checking beyond correctness of the annotation).
- `pub` makes functions/types/constants public (usable by other modules).

## Strict Rules (immutability; no nil; Result/Option philosophy)
- IMMUTABILITY: all values are immutable. `let` shadowing creates new bindings;
  record update / list prepend return new values; originals are unchanged.
- NO `null`: Gleam has no null, no implicit conversions, no exceptions, and
  always performs full type checking. "If the code compiles you can be
  reasonably confident it does not have any inconsistencies that may cause
  bugs or crashes."
- NO nil-punning: `Nil` is a unit type, not a valid value of any other type.
  Values are not nullable.
- NO exceptions: fallible computations return `Result(value, error)`. The type
  is generic with two parameters (success + error). Custom error types with a
  variant per problem make errors visible and compiler-checked. "No nasty
  surprises with unexpected exceptions!"
- Result vs Option philosophy: `Option` (Some/None) is similar to `Result` but
  has no error value. Some languages use `Option` when there is no extra error
  detail, BUT Gleam ALWAYS uses `Result` for fallible functions. This keeps all
  fallible functions consistent and removes boilerplate from mixing the two
  types. (`Option` is still used for genuinely optional data fields, e.g.
  `pet: Option(String)`.)
- NO loops: iteration is via recursion (and stdlib `gleam/list` etc.).
- NO early returns / macros / type classes: Gleam relies on first-class
  functions + pattern matching; `use` mitigates indentation.
- Exhaustiveness checking: `case` patterns must cover all variants/values.
- Naming: types/constructors start with an UPPERCASE letter; variables and
  functions start lowercase and use `snake_case`.
- Operators are NOT overloaded: ints use `+ - * /` and `> < >= <=`; floats use
  `+. -. *. /.` and `>. <. >=. <=.`. String concat uses `<>`.
- Division by zero (float) is defined to be zero, not an error.

## Verbatim quotes (key syntax rules)
- "Gleam has no `null`, no implicit conversions, no exceptions, and always
  performs full type checking. If the code compiles you can be reasonably
  confident it does not have any inconsistencies that may cause bugs or
  crashes."
- "Variable names can be reused by later let bindings, but the values they
  reference are immutable, so the values themselves are not changed or mutated
  in any way."
- "`Nil` is not a valid value of any other types. Therefore, values in Gleam
  are not nullable. If the type of a value is `Nil` then it is the value
  `Nil`. If it is some other type then the value is not `Nil`."
- "Gleam doesn't use exceptions, instead computations that can either succeed
  or fail return a value of the built-in `Result(value, error)` type."
- "Some languages have functions that return an option when there is no extra
  error detail to give, but Gleam always uses result. This makes all fallible
  functions consistent and removes any boilerplate that would be required when
  mixing functions that use each type."
- "Gleam's numerical operators are not overloaded, so there are dedicated
  operators for working with floats."
- "Gleam lacks exceptions, macros, type classes, early returns, and a variety
  of other features, instead going all-in with just first-class-functions and
  pattern matching."
- "Gleam performs exhaustiveness checking to ensure that the patterns in a case
  expression cover all possible values."
- "Guard expressions cannot contain function calls, case expressions, or
  blocks."
- "Currently it is not possible to have nested alternative patterns, so the
  pattern `[1 | 2 | 3]` is not valid."
- "When labelled arguments are used the order of the arguments does not matter,
  but all unlabelled arguments must come before labelled arguments."
- "Constants must be literal values, functions cannot be used in their
  definitions."
- "The pipe operator takes the result of the expression on its left and passes
  it as an argument to the function on its right."
- "Gleam is an expression based language so there is no `return` operator."
- "With well designed types the type system can typically be used to make
  these invalid states unrepresentable." (re: panic)

## Version notes
- No Gleam version string is present on the page (HTML contains no version
  marker). The tour describes current Gleam as of crawl date (2026-06-25).
- Bit arrays have "limited support when compiling to JavaScript, not all
  options can be used. Full bit array support will be implemented in the
  future."
- Nested alternative patterns are not yet valid ("Currently it is not
  possible...").
- Float behaviour differs by runtime: BEAM raises on overflow (no NaN /
  Infinity); JS yields Infinity / -Infinity / NaN.

## Discovered links

### Relevant (crawl later)
- https://gleam.run/documentation/externals/ — externals guide (deeper FFI
  detail than the tour's externals lessons).
- https://hexdocs.pm/gleam_stdlib/ — gleam_stdlib docs root (authoritative
  stdlib reference; feeds result-option-and-errors / stdlib docs).
- https://hexdocs.pm/gleam_stdlib/gleam/result.html — `gleam/result` module
  (Result type + map/try/unwrap; feeds result-option-and-errors.md).
- https://hexdocs.pm/gleam_stdlib/gleam/option.html — `gleam/option` module
  (Option type; feeds result-option-and-errors.md).
- https://hexdocs.pm/gleam_stdlib/gleam/list.html — `gleam/list` module
  (map/filter/fold/find; feeds collections docs).
- https://hexdocs.pm/gleam_stdlib/gleam/int.html — `gleam/int` module.
- https://hexdocs.pm/gleam_stdlib/gleam/float.html — `gleam/float` module.
- https://hexdocs.pm/gleam_stdlib/gleam/string.html — `gleam/string` module.
- https://hexdocs.pm/gleam_stdlib/gleam/bool.html — `gleam/bool` module.
- https://hexdocs.pm/gleam_stdlib/gleam/dict.html — `gleam/dict` module
  (Dict type; immutable, unordered).
- https://hexdocs.pm/gleam_stdlib/gleam/io.html — `gleam/io` module.
- https://www.erlang.org/doc/programming_examples/bit_syntax.html — Erlang
  bit syntax reference (bit array segment options).
- https://gleam.run — Gleam homepage (language docs / book; candidate for a
  language-overview crawl).
- https://hex.pm — Hex package repository (package publishing; candidate for
  tooling/ecosystem crawl).

### Skipped
- https://tour.gleam.run/everything/ — the page itself (already crawled).
- https://tour.gleam.run/share-preview.png — social preview image (binary,
  not text).
- https://gleam.run/images/lucy/lucy.svg — logo image (binary, not text).
- https://plausible.io/js/script.js — analytics script (not Gleam content).
- Individual tour lesson pages (e.g. /basics/hello-world, /functions/pipelines,
  /data-types/results, /advanced-features/use, etc.) — these are the 63
  sub-pages whose content is ALREADY included verbatim in the everything page;
  crawling them would duplicate this extraction.
- Tour CSS/JS assets (/common.css, /css/*, /js/*) — presentational, not
  language content.
