# Gleam language fundamentals

## Purpose
The concrete syntax and primitive semantics every Gleam file relies on: built-in
types, variables and `let`, `const`, blocks, the `todo`/`panic`/`assert` family,
modules and imports, and type annotations. This is the bedrock for
`types-records-and-patterns` and `functions-pipelines-and-use`.

## Sources used
- Crawl: `docs/gleam/.crawl/02-tour-everything.md` — https://tour.gleam.run/everything/
  (sections: Types & values; Variables, let, const; Blocks; echo; todo/panic/assert;
  Modules / imports / annotations; Strict Rules)

## Related BEAM guidance
- `docs/beam/overview.md` — Int/Float/String/Bool/Nil map to BEAM primitives on the
  Erlang target (integers, floats, binaries, booleans, the `nil` atom). See
  `erlang-interop` for the full mapping table. The fundamentals here are
  Gleam-specific syntax/semantics; runtime behavior of those primitives is shared
  with the BEAM.

## Core guidance

### Built-in types
- `Int` — whole numbers. On the BEAM, ints have no max/min size; on JavaScript they
  are 64-bit floating point. Operators: `+ - / * %` and `> < >= <=`. Number formats
  use underscores (`1_000_000`) and prefixes `0b`/`0o`/`0x`. Stdlib: `gleam/int`.
- `Float` — 64-bit floating point on both runtimes. Operators are NOT overloaded:
  dedicated `+. -. /. *.` and `>. <. >=. <=.`. Scientific notation: `7.0e7`,
  `3.0e-4`. Stdlib: `gleam/float`.
  - Runtime divergence: on JS, overflow yields `Infinity`/`-Infinity` and dividing
    two infinities yields `NaN`; on BEAM, overflow raises an error and there is no
    `NaN`/`Infinity` float.
  - "Division by zero is NOT an error — defined to be zero." (tour)
- `String` — double-quoted, may span multiple lines and contain unicode.
  Concatenation: `<>`. Escapes supported, including `\u{1F600}`. Stdlib:
  `gleam/string`.
- `Bool` — `True` or `False`. Operators: `||` (or), `&&` (and), `!` (not). `||` and
  `&&` are short-circuiting. Stdlib: `gleam/bool`.
- `Nil` — Gleam's unit type. "Returned by functions that have nothing else to
  return (all functions must return something)." `Nil` is NOT a valid value of any
  other type, so values are not nullable. Stdlib: `gleam/nil`.

### Equality
- `==` and `!=` work on values of any type, but both sides must be the same type.
  Equality is STRUCTURAL (same structure, not same memory location).

### Variables, `let`, `const`
- `let` assigns a value to a variable. Variable and function names are `snake_case`.
- Values are IMMUTABLE. "Variable names can be reused by later let bindings, but
  the values they reference are immutable, so the values themselves are not changed
  or mutated in any way." Re-binding `let x = "New"` shadows; earlier bindings still
  refer to the original.
- Discard pattern: an unused variable emits a warning; prefix with `_` (e.g.
  `_score`) to silence it.
- Type annotations on `let`: `let _name: String = "Gleam"`. "Annotations are
  documentation only — they do not change type checking beyond ensuring the
  annotation is correct." Typically omitted for assignments.
- `const` — top-level module constants. "Constants must be literal values, functions
  cannot be used in their definitions." May have a type annotation
  (`const ints: List(Int) = [1, 2, 3]`) and may be `pub`.

### Blocks
- Blocks group expressions with `{ }`. Each expression evaluated in order; the value
  of the last expression is returned. Variables assigned in a block are scoped to
  the block. Blocks can change evaluation order of binary operators: `{ 1 + 2 } * 3`
  (like parentheses in other languages).

### `echo`
- `echo` is a debug-print keyword that prints a value of ANY type (unlike
  `io.println` which only accepts strings). Can be used mid-pipeline: `|> echo`.

### `todo` / `panic` / `assert`
- `todo` — marks unimplemented code. `todo as "message"` (message optional). Compiler
  prints a warning; running crashes with the message.
- `panic` — crashes the program when an unreachable point is reached.
  `panic as "msg"`. "Should almost NEVER be used (prototypes/scripts only); in
  libraries it signals a design flaw — prefer types that make invalid states
  unrepresentable."
- `let assert` — like `let` but the pattern may be PARTIAL (need not cover all
  values). Crashes if the pattern fails to match. `as` supplies a custom message:
  `let assert [first, ..] = items as "List should not be empty"`. Use sparingly;
  avoid in libraries.
- Bool `assert` — for test assertions: asserts a boolean evaluates to `True`.
  Crashes on `False`. `assert expr as "message"` (message optional). "Designed for
  TEST code; avoid in apps/libraries."

### Modules / imports / annotations
- Code is organised into MODULES. The module name comes from the file path:
  `gleam/io` is `io.gleam` in a `gleam` directory.
- `import gleam/io` — the imported module is referred to by its LAST path segment
  (`io`).
- `as` renames a module: `import gleam/string as text`.
- Qualified vs unqualified: normally use qualified (`io.println(...)`). Unqualified
  imports list functions: `import gleam/io.{println}`. Qualified imports are
  preferred for clarity.
- Type imports: types can be qualified (`bytes_tree.BytesTree`) or unqualified via
  `import gleam/string_tree.{type StringTree}`. Types are commonly imported
  unqualified (unlike functions).
- Type annotations: optional on `let` and function args/returns; documentation only.
- `pub` makes functions/types/constants public (usable by other modules).

## Practical rules
- Use `snake_case` for variables/constants/functions; `PascalCase` for
  types/constructors (enforced).
- Prefer qualified imports for functions; unqualified is acceptable for types and
  record constructors where it aids readability.
- Annotate all module functions' argument and return types (convention).
- Use `_`-prefixed names for intentionally unused bindings.
- Reserve `assert` (bool) for test code; reserve `panic`/`let assert` for
  prototypes or genuinely unreachable points — never for fallible logic in
  libraries.
- Use `echo` for ad-hoc debugging of any type; remove before shipping.

## Review checklist
- [ ] Are all module functions annotated (args + return)?
- [ ] Are imports qualified (functions) / sensibly unqualified (types)?
- [ ] Are unused bindings prefixed with `_`?
- [ ] Is `panic`/`let assert` absent from library code (except OTP-specific cases)?
- [ ] Are float operators (`+.` etc.) used for floats, int operators for ints?

## Implementation checklist
- [ ] `const` values are literals (no function calls in definitions).
- [ ] Block scoping is correct (no reliance on block-local vars outside the block).
- [ ] Target-specific float behavior accounted for (overflow/NaN divergence).

## Validation hooks
- `gleam build` — type checks and compiles; unused-variable and exhaustiveness
  warnings surface here.
- `gleam test` — runs `assert`-based tests.

## Examples
```gleam
import gleam/io
import gleam/int

const default_count: Int = 10

pub fn greet(name: String) -> Nil {
  io.println("Hello, " <> name <> "!")
}

pub fn parse_or_default(input: String) -> Int {
  case int.parse(input) {
    Ok(n) -> n
    Error(_) -> default_count
  }
}
```

## Common mistakes
- Using `+` on floats (use `+.`) — operators are not overloaded.
- Treating `Nil` as a nullable for other types — it is not.
- Using `let assert` to "handle" a fallible result in a library — return `Result`
  instead.
- Forgetting that float division by zero yields `0.0`, not an error.

## Strict vs contextual guidance
- Strict: `snake_case`/`PascalCase` naming, immutability, no `null`, exhaustiveness,
  mandatory annotations on external functions, literal-only `const`.
- Contextual: whether to annotate `let` bindings (usually omit), qualified vs
  unqualified type imports, when `echo` is appropriate.

## Policy decisions for individual repos
- Maximum `let assert` / `panic` tolerance (libraries: none; apps: top-level only)?
- Required annotations on private (non-`pub`) functions?

## Related docs
- `overview`
- `types-records-and-patterns`
- `functions-pipelines-and-use`
- `result-option-and-errors`
- `conventions-patterns-antipatterns`

## Related skills
- `gleam-language`
- `gleam-otp-interop`
- `gleam-packages-ffi`
