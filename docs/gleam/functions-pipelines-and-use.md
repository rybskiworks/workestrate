# Gleam functions, pipelines, and `use`

## Purpose
Gleam's function model and the two constructs that define its idiomatic style:
first-class functions with labelled/default arguments, the pipe operator `|>`, and
`use` expressions for callback-based control flow. Includes external functions
(FFI declarations) at the function level; deeper FFI guidance is in
`externals-and-ffi`.

## Sources used
- Crawl: `docs/gleam/.crawl/02-tour-everything.md` — https://tour.gleam.run/everything/
  (sections: Functions; Labelled arguments; Pipelines; use expressions &
  pipelines; External functions & types; Documentation, deprecations; Strict Rules;
  Verbatim quotes)

## Related BEAM guidance
- `docs/beam/overview.md` — Gleam functions compile to Erlang functions on the BEAM
  target; tail-call optimisation is a BEAM property. The function model here is
  Gleam-specific; runtime calling semantics are shared with the BEAM.

## Core guidance

### Function definition
- `fn` defines functions. The body is expression-based: each expression evaluated in
  order, value of the last expression returned. "Gleam is an expression based
  language so there is no `return` operator."
- Without `pub` a function is private (module-local).
- Type annotations on arguments and return values are optional but good practice:
  `fn double(a: Int) -> Int { ... }`.
- Functions are VALUES: can be assigned to variables, passed as arguments, returned.
  The `fn` keyword also describes function types: `fn(Int) -> Int`.

### Higher-order & anonymous functions
- Anonymous function literals: `fn(a) { a + 1 }`. Interchangeable with named
  functions. Anonymous functions capture in-scope variables -> closures.
- Function captures (shorthand): `some_function(..., _, ...)` is sugar for
  `fn(a) { some_function(..., a, ...) }`. The `_` is a placeholder for the single
  argument. e.g. `add(1, _)` == `fn(x) { add(1, x) }`.

### Generic functions
- Generics via type variables written with a lowercase name. A type variable stands
  for one specific type per call site (NOT an `any` type); replaced with a concrete
  type each call.
```gleam
fn twice(argument: value, my_function: fn(value) -> value) -> value {
  my_function(my_function(argument))
}
```

### Labelled arguments
- Arguments can be given an external label before their internal name:
  `fn calculate(value: Int, add addend: Int, multiply multiplier: Int) { ... }` —
  here `add`/`multiply` are labels, `addend`/`multiplier` internal names.
- "When labelled arguments are used the order of the arguments does not matter, but
  all unlabelled arguments must come before labelled arguments."
- Labels are OPTIONAL when calling: `calculate(1, 2, 3)` or
  `calculate(1, add: 2, multiply: 3)` or `calculate(1, multiply: 3, add: 2)`.
- No performance cost (no runtime dict/allocation).
- Label shorthand syntax: when a local variable has the same name as a label, the
  variable name can be omitted: `calculate_total_cost(quantity:, unit_price:, discount:)`.
  Shorthand also works for record constructor arguments.
- NOTE: Gleam has NO default argument values. Optionality is expressed via
  `Option`/`Result`, not default parameters.

### Pipelines (`|>`)
- "The pipe operator takes the result of the expression on its left and passes it as
  an argument to the function on its right."
- First tries to use the LHS as the FIRST argument: `a |> b(1, 2)` -> `b(a, 1, 2)`.
  If that doesn't typecheck, falls back to calling the RHS result as a function:
  `b(1, 2)(a)`.
- Convention: write the "subject" as the first argument to make piping natural. To
  pipe to a different position use a function capture:
  `|> string.append("3", _)`.
- Debug mid-pipeline with `|> echo`.

### `use` expressions
- "Gleam lacks exceptions, macros, type classes, early returns, and a variety of
  other features, instead going all-in with just first-class-functions and pattern
  matching." `use` calls a function that takes a CALLBACK without increasing
  indentation.
- Everything below the `use` becomes an anonymous function; the assigned variables
  become the callback's arguments.
- `use a, b <- my_function` expands to `my_function(fn(a, b) { ...rest... })`.
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

### External functions & types
- External TYPE: a custom type with NO constructors — Gleam only knows it exists,
  not its shape: `pub type DateTime`.
- External FUNCTION: `@external` directs the compiler to use a specified module
  function as the implementation instead of Gleam code:
```gleam
@external(javascript, "./my_package_ffi.mjs", "now")
pub fn now() -> DateTime
```
- "Type annotations are MANDATORY for external functions (compiler cannot infer
  foreign types). Gleam trusts the annotation — inaccurate types cause runtime
  crashes. Use sparingly; prefer Gleam code."
- Multi-target externals: multiple `@external` lines for different targets. If no
  implementation exists for the compiled target -> compiler error.
- External Gleam fallbacks: a function may have BOTH a Gleam body and an `@external`
  impl; the external is used when available for the target, otherwise the Gleam
  body.

### Documentation, deprecations
- Comments: `//` line comments (go on the line BEFORE the item they describe).
- Doc comments: `///` for types/functions (immediately before the item); `////` for
  module docs (top of module).
- `@deprecated("Use new_function instead")` emits a warning when the deprecated
  definition is referenced.

## Practical rules
- Write the "subject" as the first argument so functions pipe naturally.
- Use `use` with `result.try`/`result.map` to chain fallible computations flatly
  instead of nesting `case`.
- Use labelled args to make call sites self-documenting; remember unlabelled args
  must precede labelled ones.
- Express optionality via `Option`/`Result`, never via default arguments (none
  exist).
- Keep `use` RHS a plain function call; avoid stacking many `use`s opaquely.
- For FFI, always annotate external functions fully; prefer Gleam fallbacks for
  multi-target safety.

## Review checklist
- [ ] Are functions annotated (args + return)?
- [ ] Do piped functions take the subject as the first argument?
- [ ] Is `use` used for `Result` chaining rather than nested `case`?
- [ ] Are external functions fully annotated with correct target(s)?
- [ ] Are deprecated functions marked with `@deprecated`?

## Implementation checklist
- [ ] Labelled args ordered after unlabelled args.
- [ ] Function captures used where a single arg is deferred (`fn(x){ f(a,x) }` -> `f(a, _)`).
- [ ] Multi-target externals provide an impl per target or a Gleam fallback.

## Validation hooks
- `gleam build` — type checks function signatures, labelled-arg ordering, and
  external annotations.
- `gleam test` — exercises `use` chains and pipelines.

## Examples
```gleam
import gleam/result
import gleam/int

pub fn parse_and_double(input: String) -> Result(Int, Nil) {
  use n <- result.try(int.parse(input))
  Ok(n * 2)
}

// Pipeline with subject-first function and a capture for non-first position.
pub fn process(s: String) -> String {
  s
  |> string.to_graphemes
  |> list.reverse
  |> string.concat
}
```

## Common mistakes
- Expecting default argument values — Gleam has none; use `Option`/`Result`.
- Putting labelled args before unlabelled args (not allowed).
- Making `use` RHS a complex expression instead of a plain call.
- Omitting type annotations on external functions (mandatory).
- Forgetting a Gleam fallback when an external is target-specific.

## Strict vs contextual guidance
- Strict: no `return`, no default args, mandatory external annotations, unlabelled
  args before labelled, `@external` takes exactly 3 args.
- Contextual: when to use `use` vs nested `case`, when to label args, when to use
  function captures, opaque vs transparent external types.

## Policy decisions for individual repos
- Require labels on all multi-arg public functions?
- Mandate Gleam fallbacks for every external function (multi-target safety)?

## Related docs
- `overview`
- `language-fundamentals`
- `types-records-and-patterns`
- `result-option-and-errors`
- `externals-and-ffi`
- `conventions-patterns-antipatterns`

## Related skills
- `gleam-language`
- `gleam-otp-interop`
- `gleam-packages-ffi`
