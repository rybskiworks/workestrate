# Crawl: gleam_stdlib/gleam/option.html
- seed_url: https://hexdocs.pm/gleam_stdlib/gleam/option.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/option.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: result-option-and-errors.md, stdlib.md

## Purpose
`Option` represents a value that may be present or not. `Some` means the value
is present, `None` means the value is not. This is Gleam's alternative to having
a value that could be `Null`, as is possible in some other languages.

## Type definition (verbatim)
```gleam
pub type Option(a) {
  Some(a)
  None
}
```
Constructors: `Some(a)`, `None`.

## Functions (full list, signature + purpose)
All public functions in the module (v1.0.3). Note: there are no standalone
`some`/`none` constructor functions — use the `Some`/`None` constructors
directly. There is no `to_list`, `filter`, `take`, or `zip` in this module.

- `pub fn all(list: List(Option(a))) -> Option(List(a))`
  Combines a list of Options into a single Option. If all elements are `Some`
  returns `Some` holding the list of values; if any element is `None` returns
  `None`.
  Examples: `all([Some(1), Some(2)]) == Some([1, 2])`; `all([Some(1), None]) == None`.

- `pub fn flatten(option: Option(Option(a))) -> Option(a)`
  Merges a nested Option into a single layer.
  Examples: `flatten(Some(Some(1))) == Some(1)`; `flatten(Some(None)) == None`;
  `flatten(None) == None`.

- `pub fn from_result(result: Result(a, e)) -> Option(a)`
  Converts a `Result` type to an `Option` type (discards the error).
  Examples: `from_result(Ok(1)) == Some(1)`; `from_result(Error("some_error")) == None`.

- `pub fn is_none(option: Option(a)) -> Bool`
  Checks whether the Option is a `None` value.
  Examples: `!is_none(Some(1))`; `is_none(None)`.

- `pub fn is_some(option: Option(a)) -> Bool`
  Checks whether the Option is a `Some` value.
  Examples: `is_some(Some(1))`; `!is_some(None)`.

- `pub fn lazy_or(first: Option(a), second: fn() -> Option(a)) -> Option(a)`
  Returns the first value if it is `Some`, otherwise evaluates the given
  function for a fallback value (lazy second operand).
  Examples: `lazy_or(Some(1), fn() { Some(2) }) == Some(1)`;
  `lazy_or(None, fn() { Some(2) }) == Some(2)`; `lazy_or(None, fn() { None }) == None`.

- `pub fn lazy_unwrap(option: Option(a), or default: fn() -> a) -> a`
  Extracts the value from an Option, evaluating the default function if the
  option is `None`.
  Examples: `lazy_unwrap(Some(1), fn() { 0 }) == 1`; `lazy_unwrap(None, fn() { 0 }) == 0`.

- `pub fn map(over option: Option(a), with fun: fn(a) -> b) -> Option(b)`
  Updates a value held within the `Some` of an Option by calling a function on
  it. If the Option is `None` the function is not called and the Option stays
  the same.
  Examples: `map(over: Some(1), with: fn(x) { x + 1 }) == Some(2)`;
  `map(over: None, with: fn(x) { x + 1 }) == None`.

- `pub fn or(first: Option(a), second: Option(a)) -> Option(a)`
  Returns the first value if it is `Some`, otherwise returns the second value
  (eager second operand).
  Examples: `or(Some(1), Some(2)) == Some(1)`; `or(None, Some(2)) == Some(2)`;
  `or(None, None) == None`.

- `pub fn then(option: Option(a), apply fun: fn(a) -> Option(b)) -> Option(b)`
  Updates a value held within the `Some` of an Option by calling a function on
  it, where the function also returns an Option. The two options are merged
  together. Equivalent to `map` followed by `flatten`; useful for chaining
  functions that return Option. If `None`, the function is not called.
  Examples: `then(Some(1), fn(x) { Some(x + 1) }) == Some(2)`;
  `then(Some(1), fn(_) { None }) == None`; `then(None, fn(x) { Some(x + 1) }) == None`.

- `pub fn to_result(option: Option(a), e: e) -> Result(a, e)`
  Converts an Option type to a Result type, using the supplied error value `e`
  when the option is `None`.
  Examples: `to_result(Some(1), "some_error") == Ok(1)`;
  `to_result(None, "some_error") == Error("some_error")`.

- `pub fn unwrap(option: Option(a), or default: a) -> a`
  Extracts the value from an Option, returning a default value if there is none.
  Examples: `unwrap(Some(1), 0) == 1`; `unwrap(None, 0) == 0`.

- `pub fn values(options: List(Option(a))) -> List(a)`
  Given a list of Options, returns only the values inside `Some` (drops `None`s).
  Examples: `values([Some(1), None, Some(3)]) == [1, 3]`.

## Option vs Result guidance
In other languages fallible functions may return either `Result` or `Option`
depending on whether there is more information to be given about the failure.
In Gleam ALL fallible functions return `Result`, and `Nil` is used as the error
if there is no extra detail to give. This consistency removes the boilerplate
that would otherwise be needed to convert between Option and Result types, and
makes APIs more predictable.

The `Option` type should ONLY be used for:
- taking optional values as function arguments;
- storing them in other data structures.

Do NOT use `Option` as a return type for fallible operations — use `Result`
instead (with `Nil` error if no detail is needed).

## Strict rules
- Use `Option(a)` only for optional values (function args / data-structure fields).
- Use `Result(a, e)` (never `Option`) for fallible operations; use `Nil` as the
  error type when there is no extra failure detail.
- Prefer `then` over manual `map` + `flatten` for chaining Option-returning
  functions.
- Use `lazy_or` / `lazy_unwrap` when the fallback is expensive or has side
  effects; use `or` / `unwrap` when the fallback is a plain value.
- `from_result` discards the error; `to_result` requires an explicit error
  value to inject on `None`.

## Verbatim quotes
- "Option represents a value that may be present or not. Some means the value
  is present, None means the value is not."
- "This is Gleam's alternative to having a value that could be Null, as is
  possible in some other languages."
- "In Gleam all fallible functions return Result, and Nil is used as the error
  if there is no extra detail to give. This consistency removes the boilerplate
  that would otherwise be needed to convert between Option and Result types, and
  makes APIs more predictable."
- "The Option type should only be used for taking optional values as function
  arguments, or for storing them in other data structures."
- (`then`) "This function is the equivalent of calling map followed by flatten,
  and it is useful for chaining together multiple functions that return Option."

## Version notes
- Page title: `gleam/option · gleam_stdlib · v1.0.3`.
- Module surface in v1.0.3: type `Option(a)` with constructors `Some(a)`/`None`,
  and 13 functions: `all`, `flatten`, `from_result`, `is_none`, `is_some`,
  `lazy_or`, `lazy_unwrap`, `map`, `or`, `then`, `to_result`, `unwrap`, `values`.
- NOT present in this module (despite being common in other stdlibs): `some`,
  `none` (use constructors), `to_list`, `filter`, `take`, `zip`, `map2`,
  `contains`, `get_or_insert`, etc. Use `values` to collect Some-values from a
  list; use `list.filter_map` / `then` for filtering-style chains.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/gleam_stdlib/gleam/result.html (Result module — the
  canonical fallible-return type; cross-referenced throughout this module)
- https://hexdocs.pm/gleam_stdlib/gleam/list.html (list module — `all`/`values`
  interact with `List(Option(a))`; `filter_map` is the list-side counterpart to
  `then` chains)
- https://hexdocs.pm/gleam_stdlib/gleam_stdlib.html (package README / overview)

### Skipped
- Website, Sponsor, Repository, Hex links (non-doc navigation)
- All other module links in the sidebar (bit_array, bool, bytes_tree, dict,
  dynamic, dynamic/decode, float, function, int, io, order, pair, set, string,
  string_tree, uri) — out of scope for this single-link crawl step
