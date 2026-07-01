---
name: constraint-gleam-result
description: |
  Enforces Gleam Result/Option and crash-boundary invariants during code
  execution — use Result (not Option, not exceptions) for fallible functions,
  no panic/todo in libraries, make-invalid-states-impossible, and exhaustive
  pattern matching. Load when writing or reviewing Gleam error-handling code.
  Does NOT cover naming/import conventions (see constraint-gleam-conventions).
metadata:
  org.kind: constraint
---

# Constraint: Gleam Result, Option, and Errors

This constraint enforces Gleam's no-exceptions error model: fallible functions
return `Result`, `Option` is only for optional data, and crashes are reserved for
genuinely unreachable states. Violations take control from callers or disable
exhaustiveness checking.

## Triggers

Load this skill when:

- Writing or reviewing a Gleam function that can fail.
- Choosing between `Result`, `Option`, `panic`, `todo`, or `let assert`.
- Chaining fallible computations (`result.try`/`use`).
- Designing error types or deciding crash-vs-return boundaries.

## Rules

1. ALL fallible functions MUST return `Result(a, e)`; use `Nil` as the error type
   when there is no extra detail. NEVER return `Option` from a fallible function.
2. `Option(a)` is ONLY for optional function arguments or data-structure fields —
   never as a fallible return type.
3. Libraries MUST NOT use `panic`/`let assert` for fallible logic (exception:
   libraries about OTP, with a supervision tree). `panic` may be acceptable only
   at the top level of application code.
4. `todo` marks unimplemented code (compiles with a warning, crashes at runtime);
   `assert` (bool) is for TEST code only.
5. Chain fallible calls with `result.try` (+ `use`); transform with
   `result.map`/`result.map_error`; recover with `result.try_recover`; aggregate
   with `result.all` (short-circuits on first error).
6. Do NOT call absent `gleam/result`/`gleam/option` functions: there is no
   `result.then`/`combine`/`recover`/`nil_error`/`from`/`from_option`/
   `to_option`/`get` (use `try`/`all`/`try_recover`); no standalone `some`/`none`
   functions (use `Some`/`None` constructors); no `option.filter`/`take`/
   `to_list`/`zip`/`map2`/`contains`/`get_or_insert`.
7. `case` enforces EXHAUSTIVENESS — match all variants explicitly; avoid
   catch-all `_` where explicit variants are safer (it disables exhaustiveness
   refactoring).
8. Make invalid states unrepresentable: use custom types to encode business rules;
   replace `Bool` fields with descriptive custom types.
9. Design descriptive errors: variants describe failures in business-domain terms
   and carry useful fields; lower-level errors are fields of higher-level errors.
10. Avoid check-then-assert — use pattern matching or `result.try`/`result.map`.

## References

- Operational skill: `gleam-language`.
- Docs: `docs/gleam/result-option-and-errors.md`,
  `docs/gleam/conventions-patterns-antipatterns.md`.

## Out of scope

- Gleam naming/import/module conventions — see `constraint-gleam-conventions`.
- OTP actors/supervision in Gleam — see `gleam-otp-interop`.
- FFI/`Dynamic` boundaries — see `gleam-packages-ffi`.

## Violation examples

### Returning `Option` from a fallible function

```gleam
// FORBIDDEN: Option is not for fallible returns
pub fn parse(input: String) -> Option(Int) { ... }
```

Correct: `pub fn parse(input: String) -> Result(Int, ParseError) { ... }` (use
`Nil` as the error if there is no detail).

### `let assert` to "handle" a result in a library

```gleam
// FORBIDDEN: takes control from callers; crashes on Error
let assert Ok(value) = data
```

Correct: `use value <- result.try(data)`.

### Catch-all disabling exhaustiveness

```gleam
// FORBIDDEN: a new variant is silently swallowed
case role { Student -> handle_student() _ -> handle_teacher() }
```

Correct: `case role { Student -> handle_student() Teacher -> handle_teacher() }`.

### Calling an absent `result` function

```gleam
// FORBIDDEN: result.then does not exist in gleam_stdlib v1.0.3
use n <- result.then(parse(input))
```

Correct: `use n <- result.try(parse(input))` (`try` is the monadic bind).

## How to check

```bash
gleam check      # type-check + exhaustiveness (enforces Result handling)
gleam build      # full type-check + compile
gleam test       # assert-based tests for Ok/Error branches
```

Manual review:

- Every fallible function returns `Result` (not `Option`, not panic).
- No `panic`/`let assert` in library code (except OTP libraries).
- Fallible chains use `result.try` + `use`, not nested `case`.
- No invented `result`/`option` function names (v1.0.3 surface only).
- No catch-all `_` where explicit variants are safer; invalid states
  unrepresentable.
