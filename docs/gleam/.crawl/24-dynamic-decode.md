# Crawl: gleam/dynamic/decode.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: json-dynamic-and-api-boundaries.md

## Purpose

The `gleam/dynamic/decode` module provides a type-safe, composable way to convert
dynamically typed data (`Dynamic` — data whose precise type is unknown and must be
introspected at runtime) into a desired static type, or into a list of
`DecodeError`s if the data does not have the desired structure.

`Dynamic` data typically originates from:
- user input;
- untyped languages such as Erlang or JavaScript;
- external boundaries (JSON, Erlang interop, JS interop).

The decoding technique was inspired by Juraj Petráš' Toy, Go's `encoding/json`,
and Elm's `Json.Decode`.

The `Decoder` type is generic with one type parameter: the type it attempts to
decode. A `Decoder(String)` decodes strings; a `Decoder(Option(Int))` decodes
`Option(Int)`.

Decoders work using runtime reflection and the data structures of the target
platform. Differences between Erlang and JavaScript data structures may impact
decoders, so test decoders on all supported platforms.

The language server has a "generate dynamic decoder" code action that generates
a decoder function from a custom type definition; the generated function can be
edited to suit.

## Decoder(t) type

```gleam
pub opaque type Decoder(t)
```

`Decoder(t)` is opaque. A decoder is a value that can turn `Dynamic` data into
typed data via the `run` function. Smaller decoders are combined into larger
decoders using functions such as `list` and `field`.

The related error type:

```gleam
pub type DecodeError {
  DecodeError(
    expected: String,
    found: String,
    path: List(String),
  )
}
```

`Dynamic` is re-exported:

```gleam
pub type Dynamic = dynamic.Dynamic
```

## Combinators (full list, signatures)

Primitives (consts):

```gleam
pub const bit_array: Decoder(BitArray)   // never errors
pub const bool: Decoder(Bool)
pub const float: Decoder(Float)          // does NOT coerce int -> float (Erlang)
pub const int: Decoder(Int)              // does NOT coerce float -> int
pub const string: Decoder(String)
pub const dynamic: Decoder(dynamic.Dynamic)  // never errors
```

Combinators:

```gleam
pub fn at(path: List(segment), inner: Decoder(a)) -> Decoder(a)
// Index into nested key-value containers (dicts/maps/JS objects); int keys
// also index Erlang tuples, JS arrays, and first 8 elements of Gleam lists.

pub fn collapse_errors(decoder: Decoder(a), name: String) -> Decoder(a)
// Replace all errors with a single named expected-type error. Useful with one_of.

pub fn decode_error(expected expected: String, found found: dynamic.Dynamic) -> List(DecodeError)
// Construct a decode error for unexpected dynamic data.

pub fn dict(key: Decoder(key), value: Decoder(value)) -> Decoder(dict.Dict(key, value))

pub fn failure(placeholder: a, expected name: String) -> Decoder(a)
// Always-failing decoder. `placeholder` is an internal default never returned
// to the user (used so the rest of a larger decoder can continue and collect
// all errors). Pick any arbitrary value.

pub fn field(
  field_name: name,
  field_decoder: Decoder(t),
  next: fn(t) -> Decoder(final),
) -> Decoder(final)
// Run a decoder on a field of a Dynamic value. Errors if the field is absent.
// Designed for `use` callback style. Indexes dicts (any key type); int keys
// also index Erlang tuples, JS arrays, first 8 of Gleam lists.

pub fn list(of inner: Decoder(a)) -> Decoder(List(a))
// On Erlang decodes from lists; on JS from lists and JS arrays.

pub fn map(decoder: Decoder(a), transformer: fn(a) -> b) -> Decoder(b)
// Apply a transformation to the decoded value.

pub fn map_errors(
  decoder: Decoder(a),
  transformer: fn(List(DecodeError)) -> List(DecodeError),
) -> Decoder(a)

pub fn new_primitive_decoder(
  name: String,
  decoding_function: fn(dynamic.Dynamic) -> Result(t, t),
) -> Decoder(t)
// Create a decoder for a new primitive type. The decoding function returns
// Result(t, t) where the error branch carries a placeholder default (used so
// the rest of a larger decoder can continue collecting errors).

pub fn one_of(first: Decoder(a), or alternatives: List(Decoder(a))) -> Decoder(a)
// Run each inner decoder in turn; use the first that succeeds. If none
// succeed, the errors from the FIRST decoder are used (use collapse_errors /
// map_errors to change this).

pub fn optional(inner: Decoder(a)) -> Decoder(option.Option(a))
// Decode nullable values. Handles nil, null, undefined on Erlang; undefined
// and null on JavaScript.

pub fn optional_field(
  key: name,
  default: t,
  field_decoder: Decoder(t),
  next: fn(t) -> Decoder(final),
) -> Decoder(final)
// Like `field` but returns `default` if the field is absent (no error).

pub fn optionally_at(path: List(segment), default: a, inner: Decoder(a)) -> Decoder(a)
// Like `at` but returns `default` if the path is absent (no error).

pub fn recursive(inner: fn() -> Decoder(a)) -> Decoder(a)
// Create a decoder that can refer to itself (deferred via a thunk) to avoid
// infinite loops when decoding deeply nested/recursive data.

pub fn run(data: dynamic.Dynamic, decoder: Decoder(t)) -> Result(t, List(DecodeError))
// Run a decoder on Dynamic data.

pub fn subfield(
  field_path: List(name),
  field_decoder: Decoder(t),
  next: fn(t) -> Decoder(final),
) -> Decoder(final)
// Like `field` but takes a path to the value rather than a single field name.

pub fn success(data: t) -> Decoder(t)
// Finalise a decoder having successfully extracted a value.

pub fn then(decoder: Decoder(a), next: fn(a) -> Decoder(b)) -> Decoder(b)
// Run a second decoder after the first succeeds; useful for further decoding
// based on the first value (e.g. enum strings -> variants).
```

NOTE on `decode1..decodeN` / `sequence`: This version (gleam_stdlib v1.0.3) does
NOT provide `decode1`, `decode2`, ... `decodeN` arity combinators, nor a
`sequence` function. Record decoding is done via the `use`-callback style of
`field` / `subfield` / `optional_field` / `then` (each takes a `next` callback),
and list decoding is via `list`. Older Gleam versions used `decode1..decode6`
arity combinators; they have been superseded by the `use`-based API.

## Building decoders for custom types (pattern)

### Records (use-callback field chaining + success)

```gleam
let decoder = {
  use name <- decode.field("name", decode.string)
  use score <- decode.field("score", decode.int)
  use colour <- decode.field("colour", decode.string)
  use enrolled <- decode.field("enrolled", decode.bool)
  decode.success(Player(name:, score:, colour:, enrolled:))
}

let result = decode.run(data, decoder)
// Ok(Player("Mel Smith", 180, "Red", True))
```

Each `decode.field` runs a decoder on a field and threads the value into the
next callback via `use`; `decode.success` finalises the decoder with the
constructed value.

### Enum variants (then + success + failure)

For custom types whose variants carry no data, decode as a string then map to
variants:

```gleam
let decoder = {
  use decoded_string <- decode.then(decode.string)
  case decoded_string {
    "fire" -> decode.success(Fire)
    "water" -> decode.success(Water)
    "grass" -> decode.success(Grass)
    "electric" -> decode.success(Electric)
    _ -> decode.failure(Fire, expected: "PocketMonsterType")
  }
}

decode.run(dynamic.string("water"), decoder)   // Ok(Water)
decode.run(dynamic.string("wobble"), decoder)
// Error([DecodeError("PocketMonsterType", "String", [])])
```

### Record variants (tag field dispatch)

For variants that contain values, combine the enum and record patterns: decode a
discriminator field ("type"), then return the appropriate per-variant decoder.

```gleam
let trainer_decoder = {
  use name <- decode.field("name", decode.string)
  use badge_count <- decode.field("badge-count", decode.int)
  decode.success(Trainer(name, badge_count))
}

let gym_leader_decoder = {
  use name <- decode.field("name", decode.string)
  use speciality <- decode.field("speciality", pocket_monster_type_decoder)
  decode.success(GymLeader(name, speciality))
}

let decoder = {
  use tag <- decode.field("type", decode.string)
  case tag {
    "gym-leader" -> gym_leader_decoder
    _ -> trainer_decoder
  }
}

let result = decode.run(data, decoder)
```

### Recursive types

```gleam
type Nested {
  Nested(List(Nested))
  Value(String)
}

fn nested_decoder() -> decode.Decoder(Nested) {
  use <- decode.recursive
  decode.one_of(decode.string |> decode.map(Value), [
    decode.list(nested_decoder()) |> decode.map(Nested),
  ])
}
```

## run

```gleam
pub fn run(data: dynamic.Dynamic, decoder: Decoder(t)) -> Result(t, List(DecodeError))
```

Run a decoder on a `Dynamic` value. Returns `Ok(value)` on success or
`Error(List(DecodeError))` on failure. Multiple errors are collected (the
decoder continues after a field fails, using placeholder values, so that all
errors can be reported at once).

## Strict Rules

- `Decoder(t)` is **opaque** — it can only be constructed via the provided
  combinators (`success`, `failure`, `field`, `then`, `map`, `one_of`,
  `new_primitive_decoder`, etc.), never directly.
- `int` does NOT coerce floats to ints (fails on `1.0` even on Erlang).
- `float` does NOT coerce ints to floats (fails on Erlang for ints; relevant
  when decoding JSON where numbers may arrive as ints).
- Use `one_of` to decode values that may be either int or float.
- `field` errors if the field is absent; use `optional_field` for a default, or
  `optionally_at` for a default at a nested path.
- `optional` handles nil/null/undefined across runtimes.
- `at` / `field` / `subfield` / `optional_field` / `optionally_at` index into
  dicts with any key type; int keys also index Erlang tuples, JS arrays, and
  the first 8 elements of Gleam lists.
- `one_of` returns the errors of the FIRST decoder if none succeed; use
  `collapse_errors` or `map_errors` to customise error reporting.
- `failure`'s placeholder value is internal only — never returned to the user;
  pick any arbitrary value of the right type.
- `new_primitive_decoder`'s decoding function returns `Result(t, t)` where the
  error branch carries a placeholder default (same rationale as `failure`).
- Test decoders on all supported platforms (Erlang + JavaScript) because runtime
  data structures differ.
- "You should never be converting your well typed data to dynamic data." —
  `Dynamic` is for external untyped systems only.

## Verbatim quotes

- "The `Dynamic` type is used to represent dynamically typed data. That is, data
  that we don't know the precise type of yet, so we need to introspect the data to
  see if it is of the desired type before we can use it."
- "This module provides the `Decoder` type and associated functions, which
  provides a type-safe and composable way to convert dynamic data into some
  desired type, or into errors if the data doesn't have the desired structure."
- "A decoder is a value that can be used to turn dynamically typed `Dynamic`
  data into typed data using the `run` function. Several smaller decoders can be
  combined to make larger decoders using functions such as `list` and `field`."
- "Decoders work using runtime reflection and the data structures of the target
  platform. Differences between Erlang and JavaScript data structures may impact
  your decoders, so it is important to test your decoders on all supported
  platforms."
- "The language server has the 'generate dynamic decoder' code action, which
  will generate a decoder function when run on a custom type definition."
- "`Dynamic` data is data that we don't know the type of yet, originating from
  external untyped systems. You should never be converting your well typed data
  to dynamic data."
- (int) "This will not coerse float values into int values, so on platforms
  with distinct runtime int and float types (Erlang, not JavaScript) it will
  fail, even if the float is a whole number (e.g. 1.0)."
- (float) "This will not coerse int values into float values, so on platforms
  with distinct runtime int and float types (Erlang, not JavaScript) it will
  fail for ints. One time this may happen is when decoding JSON data."
- (failure) "The first parameter is a 'placeholder' value, which is some default
  value that the decoder uses internally in place of the value that would have
  been produced if the decoder was successful. It doesn't matter what this value
  is, it is never returned by the decoder or shown to the user."
- (one_of) "If no decoder succeeds then the errors from the first decoder are
  used."
- (optional) "This decoder knows how to handle multiple different runtime
  representations of absent values, including `Nil`, `None`, `null`, and
  `undefined`."
- (new_primitive_decoder) "When this decoder is used as part of a larger decoder
  this placeholder value is used so that the rest of the decoder can continue to
  run and collect all decoding errors."

## Version notes

- Page reports `gleam_stdlib v1.0.3`.
- In this version, record decoding uses the `use`-callback combinators
  (`field`, `subfield`, `optional_field`, `then`) rather than the older
  `decode1`..`decode6` arity combinators. `decode1..decodeN` and `sequence`
  are NOT present in v1.0.3.
- `list` (not `sequence`) is the list-decoding combinator.
- `optional` returns `Option(a)`; `optional_field` / `optionally_at` return a
  default value rather than an `Option`.
- `new_primitive_decoder` is the extension point for custom primitive decoders
  (e.g. Erlang pids) and is also how the built-in `int` decoder would be defined.

## Discovered links

### Relevant (crawl later)

None. This is a leaf module page; the only outbound links are to other stdlib
module pages already covered or out of scope for the JSON/dynamic crawl:
- `gleam/dynamic` (the underlying `Dynamic` type) — sibling module, not crawled
  here per the one-link-only rule.
- `gleam/dict`, `gleam/option`, `gleam/list`, `gleam/int`, `gleam/float`,
  `gleam/string`, `gleam/bit_array` — already covered or out of scope.

### Skipped

- Hexdocs sidebar module links (gleam/bit_array, gleam/bool, gleam/bytes_tree,
  gleam/dict, gleam/dynamic, gleam/float, gleam/int, gleam/list, gleam/option,
  gleam/order, gleam/pair, gleam/result, gleam/string, gleam/uri, etc.) —
  stdlib module index, not part of this crawl step.
- README, Website, Sponsor, Repository, Hex links — top-level project metadata.
