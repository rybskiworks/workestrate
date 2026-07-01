# Crawl: gleam/dynamic.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/dynamic.html
- canonical_url: https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: json-dynamic-and-api-boundaries.md, externals-and-ffi.md

## Purpose
`gleam/dynamic` is the **construction** side of Gleam's boundary with untyped
runtime data. A `Dynamic` value is a value whose type is not known to the
Gleam type system at compile time — typically obtained from Erlang FFI
interop or from IO with the outside world (e.g. JSON parsed into an
untyped blob before decoding).

This module provides functions to **build** `Dynamic` values from typed
Gleam values (so they can cross the FFI/IO boundary), plus `classify` for
debugging. The companion module `gleam/dynamic/decode` (separate page) is
where the type-safe **decoding** of dynamic data back into typed Gleam
lives. The page itself states: "You will likely mostly use the other
module in your projects."

The exact runtime representation of dynamic values depends on the
compilation target (Erlang vs JavaScript).

## Dynamic type + DecodeError variants
- `pub type Dynamic` — opaque-ish type for values of unknown shape at
  runtime. The page does not expose its internal representation; it is
  target-dependent.
- **`DecodeError` is NOT on this page.** `DecodeError`, its variants
  (unexpected type, missing field, etc.), `from`, the decoder functions
  (`string`/`int`/`float`/`bool`/`list`/`dict`/`field`/`optional_field`),
  `element`, `decode`, and `unsafe_coerce` all live in the sibling module
  `gleam/dynamic/decode` (see Discovered links). This crawl honoured the
  one-link rule and did not fetch that page.

## Decoder functions (signatures)
None on this page. This module exports **constructors**, not decoders:

- `pub fn array(a: List(Dynamic)) -> Dynamic` — list → sequential runtime
  format (tuple on Erlang, array on JS).
- `pub fn bit_array(a: BitArray) -> Dynamic`
- `pub fn bool(a: Bool) -> Dynamic`
- `pub fn classify(data: Dynamic) -> String` — returns a string naming the
  runtime type; useful for error messages/logs. Example:
  `classify(string("Hello")) == "String"`.
- `pub fn float(a: Float) -> Dynamic`
- `pub fn int(a: Int) -> Dynamic`
- `pub fn list(a: List(Dynamic)) -> Dynamic`
- `pub fn nil() -> Dynamic` — "nothing" value (atom `nil` on Erlang,
  `undefined` on JS).
- `pub fn properties(entries: List(#(Dynamic, Dynamic))) -> Dynamic` —
  unordered unique-key map (map on Erlang, Gleam dict object on JS).
- `pub fn string(a: String) -> Dynamic` — binary string on Erlang (not a
  charlist).

## FFI-boundary usage guidance
- `Dynamic` is the type to use at the FFI/IO boundary when the Gleam type
  system cannot statically describe the value (Erlang interop, external
  IO, JSON before decoding).
- Use the constructors here to **produce** dynamic values to hand to FFI
  functions that expect untyped Erlang/JS terms.
- Use `gleam/dynamic/decode` to **consume** dynamic values back into
  typed Gleam data with explicit, type-safe decoders.
- `classify` is the recommended helper for building diagnostic/error
  messages about dynamic values without attempting a full decode.
- Runtime representation is target-dependent: code that pattern-matches
  on the raw shape of a `Dynamic` (rather than going through
  `gleam/dynamic/decode`) is not portable across Erlang and JavaScript
  targets.

## Strict rules
- Do NOT treat `Dynamic` as a regular Gleam value — it must be decoded
  before use in typed code.
- Do NOT rely on the internal runtime representation of `Dynamic`; it
  differs between Erlang and JavaScript.
- Prefer `gleam/dynamic/decode` decoders over `classify` for turning
  untrusted data into typed data; `classify` is for diagnostics only.
- `string` produces a binary on Erlang, not a charlist — relevant when
  interoperating with Erlang functions that expect charlists.

## Verbatim quotes
- "`Dynamic` data is data that we don't know the type of yet. We likely
  get data like this from interop with Erlang, or from IO with the
  outside world."
- "This module contains code for forming dynamic data, and the
  `gleam/dynamic/decode` module contains code for turning dynamic data
  back into Gleam data with known types. You will likely mostly use the
  other module in your projects."
- "The exact runtime representation of dynamic values will depend on the
  compilation target used."
- (`nil`) "On Erlang this will be the atom `nil`, on JavaScript this will
  be `undefined`."
- (`string`) "On Erlang this will be a binary string rather than a
  character list."
- (`properties`) "On Erlang this will be a map, on JavaScript this will
  be a Gleam dict object."
- (`classify`) "This function may be useful for constructing error
  messages or logs. If you want to turn dynamic data into well typed data
  then you want the `gleam/dynamic/decode` module."

## Version notes
- Page title: `gleam/dynamic · gleam_stdlib · v1.0.3`.
- Source links point to `github.com/gleam-lang/stdlib/blob/v1.0.3/src/gleam/dynamic.gleam`.
- Module surface in v1.0.3: `Dynamic` type + 10 functions
  (`array`, `bit_array`, `bool`, `classify`, `float`, `int`, `list`,
  `nil`, `properties`, `string`).
- NOTE: In older gleam_stdlib versions (pre-~v0.33) the decoders
  (`from`, `string`, `int`, `field`, `decode`, `DecodeError`,
  `unsafe_coerce`, etc.) lived directly in `gleam/dynamic`. They have
  since been moved to the `gleam/dynamic/decode` submodule; this
  v1.0.3 page only contains the constructors. Any docs referencing
  `dynamic.from` / `dynamic.field` / `dynamic.DecodeError` are
  describing the legacy layout.

## Discovered links

### Relevant (crawl later)
- ../gleam/dynamic/decode.html  ← **HIGH PRIORITY**: the actual decoders,
  `DecodeError` variants, `from`, `field`, `optional_field`, `element`,
  `decode`, `unsafe_coerce`. Required to complete the
  json-dynamic-and-api-boundaries.md picture.
- ../gleam/dict.html  — referenced by `properties` (JS representation).
- ../gleam/bit_array.html — `bit_array` constructor input type.
- ../gleam/list.html — `array`/`list`/`properties` input type.
- https://github.com/gleam-lang/stdlib/blob/v1.0.3/src/gleam/dynamic.gleam — source.

### Skipped
- ../gleam/bool.html, ../gleam/bytes_tree.html, ../gleam/float.html,
  ../gleam/function.html, ../gleam/int.html, ../gleam/io.html,
  ../gleam/option.html, ../gleam/order.html, ../gleam/pair.html,
  ../gleam/result.html, ../gleam/set.html, ../gleam/string.html,
  ../gleam/string_tree.html, ../gleam/uri.html — core modules, not
  boundary-specific.
- https://gleam.run/, https://hex.pm/packages/gleam_stdlib — top-level
  project pages.
- #icon-gleam-chasse, #icon-gleam-chasse-2 — UI anchors.
