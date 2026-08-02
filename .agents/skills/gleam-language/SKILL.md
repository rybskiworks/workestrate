---
name: gleam-language
description: |
  Operational guide for idiomatic Gleam language fundamentals — built-in types,
  custom types/records/pattern matching, functions/pipelines/`use`, the
  `Result`/`Option` error model, `gleam_stdlib` (v1.0.3) core modules, and
  official conventions/anti-patterns. Load when writing, reviewing, or
  debugging Gleam syntax, types, error handling, or stdlib usage. Does NOT
  cover OTP actors/supervisors or `gleam_erlang`/`gleam_otp` (see
  `gleam-otp-interop`), nor project/CLI/deps/publish/FFI/HTTP/deploy (see
  `gleam-packages-ffi`).
---

# Gleam language fundamentals

## Triggers

Load this skill when:

- Writing, reviewing, or debugging Gleam syntax: `let`/`const`, blocks, `echo`,
  `todo`/`panic`/`assert`, modules/imports, type annotations.
- Modeling domains with custom types, records, generics, opaque types, tuples,
  lists, bit arrays, and `case` pattern matching + guards.
- Designing functions, labelled args, pipelines (`|>`), `use` expressions, or
  external function declarations (function-level FFI).
- Choosing `Result` vs `Option`, chaining fallible computations, or deciding
  crash-vs-return boundaries.
- Using `gleam_stdlib` (v1.0.3) modules: `list`, `string`, `dict`, `result`,
  `option`, `dynamic`, `dynamic/decode`, `bit_array`.
- Enforcing Gleam naming, import, error-design, and type-modeling conventions.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/gleam/language-fundamentals.md`
  - https://tour.gleam.run/everything/
- `docs/gleam/types-records-and-patterns.md`
  - https://tour.gleam.run/everything/
- `docs/gleam/functions-pipelines-and-use.md`
  - https://tour.gleam.run/everything/
- `docs/gleam/result-option-and-errors.md`
  - https://hexdocs.pm/gleam_stdlib/gleam/result.html
  - https://hexdocs.pm/gleam_stdlib/gleam/option.html
  - https://tour.gleam.run/everything/
- `docs/gleam/stdlib.md`
  - https://gleam-stdlib.hexdocs.pm/
  - https://gleam-stdlib.hexdocs.pm/gleam/list.html
  - https://gleam-stdlib.hexdocs.pm/gleam/string.html
  - https://hexdocs.pm/gleam_stdlib/gleam/dict.html
  - https://gleam-stdlib.hexdocs.pm/gleam/result.html
  - https://gleam-stdlib.hexdocs.pm/gleam/option.html
  - https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
  - https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
  - https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html
- `docs/gleam/conventions-patterns-antipatterns.md`
  - https://gleam.run/documentation/conventions-patterns-and-anti-patterns/
  - https://gleam.run/cheatsheets/gleam-for-elixir-users/
  - https://gleam.run/cheatsheets/gleam-for-erlang-users/
  - https://gleam.run/cheatsheets/gleam-for-rust-users/
- `docs/beam/overview.md` — Gleam primitives map to BEAM runtime primitives
  on the Erlang target (ints, floats, binaries, booleans, the `nil` atom).
- `docs/beam/common-mistakes.md` — BEAM-side failure patterns complementing
  Gleam's "no exceptions, return `Result`" stance.

## Key Rules

- `snake_case` for variables/constants/functions; `PascalCase` for
  types/constructors (compiler-enforced). Acronyms are single lowercase words
  (`json`, not `JSON`).
- Values are IMMUTABLE; `let` re-binding shadows. `Nil` is the unit type and
  is NOT a valid value of any other type — values are not nullable.
- `Int`/`Float` operators are NOT overloaded: `+ - / * % > < >= <=` for ints;
  `+. -. /. *. >. <. >=. <=.` for floats. Float division by zero yields `0.0`
  (not an error). On JS, floats may be `Infinity`/`NaN`; on BEAM, overflow
  raises and there is no `NaN`/`Infinity`.
- `const` must be literal values (no function calls); may be `pub`.
- `todo`/`panic`/`let assert` crash at runtime; `assert` (bool) is for TEST
  code. Libraries MUST NOT use `panic`/`let assert` for fallible logic
  (exception: libraries *about* OTP). Return `Result` instead.
- `case` enforces EXHAUSTIVENESS; missing/redundant patterns are errors.
  Avoid catch-all `_` where explicit variants are safer. Guards (`if`) cannot
  contain function calls, `case`, or blocks.
- Records: immutable; update with `..record, field: value`. Accessors usable
  without `case` only when field name/position/type match across ALL variants.
- Opaque types (`pub opaque type`) enable smart constructors that enforce
  invariants — prefer them where invariants matter.
- Lists are immutable singly-linked; prepend `[x, ..xs]` is O(1); `length`/
  indexing is O(n) (no `list[i]`). Make recursive helpers tail-recursive via
  an accumulator.
- Functions are values; no `return`, no default argument values (express
  optionality via `Option`/`Result`). Labelled args follow unlabelled args;
  label shorthand (`name:`) works when a local var matches the label.
- Pipeline `|>` feeds LHS as FIRST argument (falls back to `rhs(lhs)`). Write
  the "subject" as the first argument so functions pipe naturally.
- `use a, b <- f` desugars to `f(fn(a, b) { ... })`; keep RHS a plain call.
  Primary use: chaining fallible `Result` functions with `result.try`.
- `Result(a, e)` for ALL fallible returns; use `Nil` as the error type when
  there is no extra detail. `Option(a)` ONLY for optional args/data-structure
  fields — NEVER as a fallible return type.
- `gleam/result` v1.0.3 surface: `all`, `flatten`, `is_error`, `is_ok`,
  `lazy_or`, `lazy_unwrap`, `map`, `map_error`, `or`, `partition`, `replace`,
  `replace_error`, `try`, `try_recover`, `unwrap`, `unwrap_error`, `values`.
  `try` is the monadic bind (= `map` + `flatten`); `all` short-circuits on
  first `Error`; `try_recover` recovers from `Error`.
- `gleam/option` v1.0.3 surface: `all`, `flatten`, `from_result`, `is_none`,
  `is_some`, `lazy_or`, `lazy_unwrap`, `map`, `or`, `then`, `to_result`,
  `unwrap`, `values`. Use `Some`/`None` constructors directly (no `some`/`none`
  fns).
- DO NOT call absent names: `result.then`/`combine`/`recover`/`nil_error`/
  `from`/`from_option`/`to_option`/`get` (use `try`/`all`/`try_recover`);
  `option.filter`/`take`/`to_list`/`zip`/`map2`/`contains`/`get_or_insert`.
- `gleam/list`: prefer `fold` over `fold_right` (tail-recursive). `zip`
  truncates to shorter list; use `strict_zip` to error on mismatch. Absent in
  v1.0.3: `at`, `slice`, `range`, `pop_map`.
- `gleam/dict`: `get` returns `Result(v, Nil)`; `upsert` callback receives
  `option.Option(v)`; `size` is O(1); NO ordering guarantee (never rely on
  iteration order of `Dict`/`group`/`partition`).
- `gleam/string`: `length` counts grapheme clusters (linear — avoid in loops);
  `replace` replaces ALL occurrences; use `gleam/string_tree` for large/repeated
  joins (`append` copies).
- `gleam/dynamic` is the CONSTRUCTION side (constructors + `classify` for
  diagnostics only). Decode via `gleam/dynamic/decode`. Never let `Dynamic`
  flow past a boundary into typed internals; never pattern-match on raw
  runtime shape (target-dependent).
- `gleam/dynamic/decode`: `Decoder(t)` opaque; primitives `int`/`float`/
  `string`/`bool`/`bit_array`/`dynamic`. `int` does NOT coerce `1.0`; `float`
  does NOT coerce ints — use `one_of` for int-or-float. Records use the
  `use`-callback style (`field`/`subfield`/`optional_field`/`then` +
  `success`). `run` collects ALL errors. Absent: `decode1..N`, `sequence`.
- `gleam/bit_array`: `base64_encode(input, padding: Bool)`, `to_string`
  returns `Result(String, Nil)` (errors on invalid UTF-8). Absent:
  `from_list`/`to_list`/`equal`/`xor`/`hash`/`to_int`/`from_int`.
- Conventions: qualified imports for functions; unqualified acceptable for
  types/constructors. Annotate all module functions (args + return). Module
  names singular; conversion fns `x_to_y`; fallible fns get domain names
  (`parse_json`). Do not abbreviate. Group modules by business domain, not by
  pattern. Make invalid states unrepresentable; replace bools with custom types.
- Never use `gleam/dynamic.Dynamic` for FFI types — define a precise custom
  type. Avoid check-then-assert; use `result.try`/`result.map`.

## Quick Commands

```bash
gleam format --check            # CI formatting gate (no rewrite)
gleam build                     # type-check + compile (enforces exhaustiveness)
gleam check                      # type-check without codegen (faster)
gleam test                       # gleeunit: pub fn ..._test under test/
gleam build --target javascript # validate JS target
gleam fix                        # rewrite deprecated Gleam code
```

## Anti-patterns

- Returning `Option` from a fallible function — use `Result` (with `Nil`
  error if no detail).
- Calling `result.then`/`combine`/`recover` or `option.filter`/`take`/
  `to_list` — absent in v1.0.3; use `try`/`all`/`try_recover` and
  `values`/`list.filter_map`/`then`.
- Using `let assert Ok(x) = res` to "handle" a result in a library — return
  `Result` or use `result.try`.
- Panicking on a fallible path; using `panic`/`let assert` in library code
  (except OTP libraries with a supervision tree).
- Catch-all `_` patterns that silently swallow new variants (disables
  exhaustiveness refactoring).
- Function calls / `case` / blocks inside a guard (not allowed).
- `fold_right` on large lists (not tail-recursive); `string.length` in loops
  (linear); `string.append` for large joins (copies — use `string_tree`).
- Relying on `Dict`/`group`/`partition` iteration order (unspecified).
- Using `Dynamic` for FFI types or letting `Dynamic` flow into typed internals.
- Abbreviations; fragmented modules; grouping by design pattern; category-
  theory overuse; bool fields where a custom type carries context.
- Expecting default argument values (none exist) or `return` (none exists).

## Related Skills

- `gleam-otp-interop`
- `gleam-packages-ffi`
- `beam-errors-failures`
- `beam-processes`
