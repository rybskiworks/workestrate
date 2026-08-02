# Crawl: gleam_stdlib/gleam/result.html
- seed_url: https://hexdocs.pm/gleam_stdlib/gleam/result.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/result.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: result-option-and-errors.md, stdlib.md

## Purpose
The `gleam/result` module provides combinators for working with Gleam's built-in
`Result(a, e)` type — the standard mechanism for explicit error handling. Gleam
has no exceptions and no `throw`/`try`/`catch`; any operation that can fail
returns a `Result`, and this module supplies the mapping, chaining, recovery,
and aggregation helpers used to compose such computations safely.

## Type definition (verbatim)
The `Result(a, e)` type is **not defined on this page** — it is part of the
Gleam prelude (built into every Gleam program). The page documents it only via
its module intro:

> Result represents the result of something that may succeed or not.
> `Ok` means it was successful, `Error` means it was not successful.

Canonical prelude definition (for reference, not from this page):

```gleam
pub type Result(a, e) {
  Ok(a)
  Error(e)
}
```

## Functions (full list, signature + purpose)
All signatures verbatim from the page (whitespace normalized).

- `pub fn all(results: List(Result(a, e))) -> Result(List(a), e)`
  Combines a list of results into a single result. If all elements are `Ok`
  returns an `Ok` holding the list of values; if any element is `Error` returns
  the first error.

- `pub fn flatten(result: Result(Result(a, e), e)) -> Result(a, e)`
  Merges a nested `Result` into a single layer.

- `pub fn is_error(result: Result(a, e)) -> Bool`
  Checks whether the result is an `Error` value.

- `pub fn is_ok(result: Result(a, e)) -> Bool`
  Checks whether the result is an `Ok` value.

- `pub fn lazy_or(first: Result(a, e), second: fn() -> Result(a, e)) -> Result(a, e)`
  Returns the first value if it is `Ok`, otherwise evaluates the given function
  for a fallback value. Use `try_recover` if you need access to the initial error.

- `pub fn lazy_unwrap(result: Result(a, e), or default: fn() -> a) -> a`
  Extracts the `Ok` value from a result, evaluating the default function if the
  result is an `Error`.

- `pub fn map(over result: Result(a, e), with fun: fn(a) -> b) -> Result(b, e)`
  Updates a value held within the `Ok` of a result by calling a function on it.
  If the result is an `Error` the function is not called and the result stays
  the same.

- `pub fn map_error(over result: Result(a, e), with fun: fn(e) -> f) -> Result(a, f)`
  Updates a value held within the `Error` of a result by calling a function on
  it. If the result is `Ok` the function is not called and the result stays the
  same.

- `pub fn or(first: Result(a, e), second: Result(a, e)) -> Result(a, e)`
  Returns the first value if it is `Ok`, otherwise returns the second value.

- `pub fn partition(results: List(Result(a, e))) -> #(List(a), List(e))`
  Given a list of results, returns a pair where the first element is a list of
  all values inside `Ok` and the second is a list of all values inside `Error`.
  Values in both lists appear in reverse order relative to the original list.

- `pub fn replace(result: Result(a, e), value: b) -> Result(b, e)`
  Replace the value within a result.

- `pub fn replace_error(result: Result(a, e), error: f) -> Result(a, f)`
  Replace the error within a result.

- `pub fn try(result: Result(a, e), apply fun: fn(a) -> Result(b, e)) -> Result(b, e)`
  "Updates" an `Ok` result by passing its value to a function that yields a
  result, returning the yielded result (this may "replace" the `Ok` with an
  `Error`). If the input is an `Error` the function is not called and the
  original `Error` is returned. Equivalent to `map` followed by `flatten`;
  useful for chaining functions that may fail. (This is Gleam's monadic bind.)

- `pub fn try_recover(result: Result(a, e), with fun: fn(e) -> Result(a, f)) -> Result(a, f)`
  Updates a value held within the `Error` of a result by calling a function on
  it, where the function also returns a result; the two results are merged. If
  the result is `Ok` the function is not called and the result stays the same.
  Useful for chaining computations that may fail and attempting recovery.

- `pub fn unwrap(result: Result(a, e), or default: a) -> a`
  Extracts the `Ok` value from a result, returning a default value if the result
  is an `Error`.

- `pub fn unwrap_error(result: Result(a, e), or default: e) -> e`
  Extracts the `Error` value from a result, returning a default value if the
  result is an `Ok`.

- `pub fn values(results: List(Result(a, e))) -> List(a)`
  Given a list of results, returns only the values inside `Ok`.

## Result philosophy
Gleam has **no exceptions**. There is no `try`/`catch`, no `throw`, no
`raise`/`rescue`. Any operation that can fail must return a `Result(a, e)` (or
`Option(a)` for the error-free variant). Error handling is therefore explicit
and type-checked: the compiler forces callers to deal with the `Error` case.
`Result` is the primary error channel for fallible Gleam code, and this module
provides the pure combinators (`map`, `try`, `try_recover`, `all`, `partition`,
etc.) used to compose fallible computations without pattern-matching boilerplate.

## Strict rules
- Never use exceptions for control flow — Gleam has none; return `Result`.
- A function that can fail must encode that in its return type via `Result`.
- Prefer `try` (bind) over nested `case` for chaining fallible computations.
- Use `map` to transform the `Ok` value; `map_error` to transform the `Error`.
- Use `try_recover` to attempt recovery from an `Error` while staying in
  `Result`; use `unwrap`/`lazy_unwrap` only at the boundary where a default is
  acceptable.
- `all` short-circuits on the first `Error`; `partition`/`values` never fail.
- `or` / `lazy_or` provide fallback `Result`s; `lazy_or` defers the fallback
  computation (use when the fallback is expensive).

## Verbatim quotes
- "Result represents the result of something that may succeed or not. `Ok`
  means it was successful, `Error` means it was not successful."
- `try`: "This function is the equivalent of calling `map` followed by
  `flatten`, and it is useful for chaining together multiple functions that may
  fail."
- `lazy_or`: "If you need access to the initial error value, use
  `result.try_recover`."
- `partition`: "The values in both lists appear in reverse order with respect to
  their position in the original list of results."

## Version notes
- Page version: gleam_stdlib **v1.0.3**.
- The `Result(a, e)` type itself is a prelude type (built-in); this module only
  documents the combinators, not the type definition.
- Function set on this page (v1.0.3): `all`, `flatten`, `is_error`, `is_ok`,
  `lazy_or`, `lazy_unwrap`, `map`, `map_error`, `or`, `partition`, `replace`,
  `replace_error`, `try`, `try_recover`, `unwrap`, `unwrap_error`, `values`.
- NOTE: functions sometimes referenced elsewhere (`then`, `combine`, `nil_error`,
  `from`, `from_option`, `to_option`, `get`, `recover`) are **NOT present** in
  gleam_stdlib v1.0.3 `gleam/result`. `try` is the bind/`then` equivalent;
  `all` is the `combine` equivalent; `try_recover` is the `recover` equivalent.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/gleam_stdlib/gleam/option.html  (sibling: Option type —
  error-free variant, feeds result-option-and-errors.md)
- https://hexdocs.pm/gleam_stdlib/gleam/list.html  (`all`/`partition`/`values`
  operate on List(Result); list combinators referenced)
- https://hexdocs.pm/gleam_stdlib/gleam/function.html  (function helpers)

### Skipped
- https://hexdocs.pm/gleam_stdlib/gleam/bit_array.html
- https://hexdocs.pm/gleam_stdlib/gleam/bool.html
- https://hexdocs.pm/gleam_stdlib/gleam/bytes_tree.html
- https://hexdocs.pm/gleam_stdlib/gleam/dict.html
- https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
- https://hexdocs.pm/gleam_stdlib/gleam/dynamic/decode.html
- https://hexdocs.pm/gleam_stdlib/gleam/float.html
- https://hexdocs.pm/gleam_stdlib/gleam/int.html
- https://hexdocs.pm/gleam_stdlib/gleam/io.html
- https://hexdocs.pm/gleam_stdlib/gleam/order.html
- https://hexdocs.pm/gleam_stdlib/gleam/pair.html
- https://hexdocs.pm/gleam_stdlib/gleam/set.html
- https://hexdocs.pm/gleam_stdlib/gleam/string.html
- https://hexdocs.pm/gleam_stdlib/gleam/string_tree.html
- https://hexdocs.pm/gleam_stdlib/gleam/uri.html
- https://gleam.run/
- https://github.com/gleam-lang/stdlib
- https://hex.pm/packages/gleam_stdlib
