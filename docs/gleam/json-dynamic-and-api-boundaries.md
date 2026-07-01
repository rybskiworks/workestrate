# JSON, dynamic, and API boundaries

## Purpose

How Gleam handles untyped boundary data: JSON encode/decode (`gleam_json` v3.1.0),
the `Dynamic` type for runtime-typed values (`gleam/dynamic` v1.0.3), and the
type-safe decode combinators (`gleam/dynamic/decode` v1.0.3). Covers the
parse → dynamic → decode → typed pipeline and the discipline of keeping dynamic
data at boundaries and out of typed internals.

## Sources used

- Crawl 32-gleam-json.md — `gleam/json` — https://gleam-json.hexdocs.pm/gleam/json.html
- Crawl 23-dynamic.md — `gleam/dynamic` — https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
- Crawl 24-dynamic-decode.md — `gleam/dynamic/decode` — https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html

## Related BEAM guidance

- `docs/beam/common-mistakes.md` — parallels to `binary_to_term` on untrusted
  input: decoding untrusted runtime data safely, never trusting external shape.
  The same caution that applies to deserialising Erlang terms applies to JSON
  parsing — assume hostile/malformed input.
- `docs/beam/binaries.md` — `BitArray` and JSON `BitArray` input via
  `json.parse_bits`; relevant when JSON arrives from sockets/ports as binaries.

## Core guidance

### (1) gleam_json v3.1.0 — encode/decode API

The `Json` type is **opaque**. Construct it only via builders; never pattern
match on it.

Builders (each returns `Json`):

```gleam
pub fn object(entries: List(#(String, Json))) -> Json
pub fn array(from entries: List(a), of inner_type: fn(a) -> Json) -> Json
pub fn preprocessed_array(from: List(Json)) -> Json
pub fn string(input: String) -> Json
pub fn int(input: Int) -> Json
pub fn float(input: Float) -> Json
pub fn bool(input: Bool) -> Json
pub fn null() -> Json
pub fn nullable(from input: option.Option(a), of inner_type: fn(a) -> Json) -> Json
pub fn dict(dict: dict.Dict(k, v), keys: fn(k) -> String, values: fn(v) -> Json) -> Json
```

Serialise:

```gleam
pub fn to_string(json: Json) -> String
pub fn to_string_tree(json: Json) -> string_tree.StringTree
```

`to_string_tree` is **preferred for IO** — the BEAM VM is optimised for sending
`StringTree` data, and it is faster than `to_string`.

Decode (parse + decode in one call):

```gleam
pub fn parse(from json: String, using decoder: decode.Decoder(t)) -> Result(t, DecodeError)
pub fn parse_bits(from json: BitArray, using decoder: decode.Decoder(t)) -> Result(t, DecodeError)
```

`DecodeError` variants:

- `UnexpectedEndOfInput` — malformed/truncated JSON.
- `UnexpectedByte(String)` — invalid byte.
- `UnexpectedSequence(String)` — invalid token sequence.
- `UnableToDecode(List(decode.DecodeError))` — JSON parsed but the decoder
  rejected the shape (wraps `gleam_stdlib` decode errors).

**Encode model** is two-stage: build `Json` via typed builders, then serialise
with `to_string` / `to_string_tree`.

**Decode model** is parse-then-decode in one call: `parse` produces an internal
dynamic value, the supplied `Decoder(t)` produces typed `t`. The dynamic
intermediate is **not** publicly exposed.

**NOT present in v3.1.0** (do not document or call these as existing):
`to_string_builder` (it is `to_string_tree`), `to_dynamic`, `object_take`,
`null_as`, `decode` as a public fn. Decoding **requires** a
`decode.Decoder(t)` — there is no decoder-less `parse` returning raw
`dynamic.Dynamic`.

### (2) gleam/dynamic v1.0.3 — construction side

`Dynamic` is a value whose type is unknown at compile time — obtained from FFI,
IO, or JSON before decoding. This module exports **constructors** (not decoders):

```gleam
pub fn array(a: List(Dynamic)) -> Dynamic
pub fn bit_array(a: BitArray) -> Dynamic
pub fn bool(a: Bool) -> Dynamic
pub fn classify(data: Dynamic) -> String
pub fn float(a: Float) -> Dynamic
pub fn int(a: Int) -> Dynamic
pub fn list(a: List(Dynamic)) -> Dynamic
pub fn nil() -> Dynamic
pub fn properties(entries: List(#(Dynamic, Dynamic))) -> Dynamic
pub fn string(a: String) -> Dynamic
```

`classify` returns the runtime type name as a `String` — for diagnostics/error
messages **only**. Per the page: "If you want to turn dynamic data into well
typed data then you want the `gleam/dynamic/decode` module."

Runtime representation is **target-dependent** (Erlang vs JavaScript) — do not
pattern-match on raw shape (`string` is a binary on Erlang, not a charlist;
`nil` is atom `nil` on Erlang / `undefined` on JS; `properties` is a map on
Erlang / a Gleam dict object on JS).

**NOTE:** decoders (`from`, `field`, `DecodeError`, `unsafe_coerce`, etc.) moved
to `gleam/dynamic/decode` in v1.0.3. Legacy docs referencing `dynamic.from` /
`dynamic.field` describe the pre-~v0.33 layout and are outdated.

### (3) gleam/dynamic/decode v1.0.3 — decode side (use-callback combinators)

`Decoder(t)` is **opaque** — construct only via combinators.

Primitives (consts): `bit_array`, `bool`, `float`, `int`, `string`, `dynamic`
(all `Decoder(t)`).

- `int` does **not** coerce float→int (fails on `1.0` even on Erlang).
- `float` does **not** coerce int→float (fails on Erlang for ints — relevant for
  JSON where numbers may arrive as ints). Use `one_of` for int-or-float.

Combinators (signatures):

```gleam
pub fn at(path: List(segment), inner: Decoder(a)) -> Decoder(a)
pub fn collapse_errors(decoder: Decoder(a), name: String) -> Decoder(a)
pub fn decode_error(expected: String, found: dynamic.Dynamic) -> List(DecodeError)
pub fn dict(key: Decoder(key), value: Decoder(value)) -> Decoder(dict.Dict(key, value))
pub fn failure(placeholder: a, expected name: String) -> Decoder(a)
pub fn field(field_name: name, field_decoder: Decoder(t), next: fn(t) -> Decoder(final)) -> Decoder(final)
pub fn list(of inner: Decoder(a)) -> Decoder(List(a))
pub fn map(decoder: Decoder(a), transformer: fn(a) -> b) -> Decoder(b)
pub fn map_errors(decoder: Decoder(a), transformer: fn(List(DecodeError)) -> List(DecodeError)) -> Decoder(a)
pub fn new_primitive_decoder(name: String, decoding_function: fn(dynamic.Dynamic) -> Result(t, t)) -> Decoder(t)
pub fn one_of(first: Decoder(a), or alternatives: List(Decoder(a))) -> Decoder(a)
pub fn optional(inner: Decoder(a)) -> Decoder(option.Option(a))
pub fn optional_field(key: name, default: t, field_decoder: Decoder(t), next: fn(t) -> Decoder(final)) -> Decoder(final)
pub fn optionally_at(path: List(segment), default: a, inner: Decoder(a)) -> Decoder(a)
pub fn recursive(inner: fn() -> Decoder(a)) -> Decoder(a)
pub fn run(data: dynamic.Dynamic, decoder: Decoder(t)) -> Result(t, List(DecodeError))
pub fn subfield(field_path: List(name), field_decoder: Decoder(t), next: fn(t) -> Decoder(final)) -> Decoder(final)
pub fn success(data: t) -> Decoder(t)
pub fn then(decoder: Decoder(a), next: fn(a) -> Decoder(b)) -> Decoder(b)
```

Record decoding uses the **use-callback** style: `field` / `subfield` /
`optional_field` / `then` each take a `next: fn(t) -> Decoder(final)` callback,
threaded via `use`. `success` finalises. `run` returns
`Result(t, List(DecodeError))` and collects **all** errors (the decoder continues
after a field fails using placeholder values).

**NOT present in v1.0.3:** `decode1`, `decode2`, ... `decodeN` arity
combinators, and `sequence`. List decoding is via `list`. Older Gleam used
`decode1..decode6` — superseded by the use-based API.

The LSP has a "generate dynamic decoder" code action that generates a decoder
from a custom type definition.

"You should never be converting your well typed data to dynamic data."

### The parse → dynamic → decode → typed pipeline

1. JSON text → `json.parse(json_string, using: decoder)` (or `parse_bits`).
2. Internally: `parse` produces a dynamic value.
3. The supplied `decode.Decoder(t)` runs against the dynamic value.
4. Result: `Result(t, DecodeError)` — typed Gleam data or structured errors.

```gleam
parse("[1,2,3]", decode.list(of: decode.int))
// Ok([1, 2, 3])

parse("1", decode.string)
// Error(UnableToDecode([decode.DecodeError("String", "Int", [])]))
```

## Practical rules

- Construct `Json` only via builders (opaque); never pattern-match.
- Prefer `to_string_tree` over `to_string` for IO.
- Always pass a `decode.Decoder` to `parse` / `parse_bits` (no decoder-less
  `parse` in v3.1.0).
- Always handle `Error` from `parse` (untrusted input).
- Use `field` / `optional_field` / `then` (use-callback) for records; never
  `decode1..N` (removed).
- Use `one_of` for int-or-float JSON numbers.
- Keep `Dynamic` at boundaries; decode immediately into typed data.
- Test decoders on both Erlang and JavaScript targets.

## Review checklist

- [ ] `Json` constructed only via builders (no pattern match, no opaque bypass).
- [ ] `parse` / `parse_bits` always given a `decode.Decoder(t)`; `Error` handled.
- [ ] No `to_string_builder` / `to_dynamic` / `object_take` / `null_as` / `decode`
      calls (not in v3.1.0); no `decode1..N` / `sequence` (not in v1.0.3).
- [ ] Records decoded via `field`/`then`/`success` use-callback style.
- [ ] `int`/`float` non-coercion accounted for (use `one_of` where needed).
- [ ] `Dynamic` does not flow past the boundary into typed internals.
- [ ] Decoders tested on both Erlang and JS targets.

## Implementation checklist

- [ ] Decoders defined next to the types they decode; `recursive` for recursive types.
- [ ] `optional_field` / `optionally_at` for defaults; `optional` for nullable.
- [ ] `collapse_errors` / `map_errors` to clean up `one_of` error reporting.
- [ ] `to_string_tree` for IO-bound serialisation; `DecodeError` handled at boundary.

## Validation hooks

- Compile both targets: `gleam build` and `gleam build --target javascript`.
- Run decoder tests on both targets: `gleam test` and
  `gleam test --target javascript`.
- Property-test round-trip: encode a value, `parse` it back with its decoder,
  assert equality (catches encoder/decoder drift).
- Assert that `parse` of malformed input returns an `Error` (not a crash).
- LSP "generate dynamic decoder" action available on custom type definitions.

## Examples

Encode an object; decode a list; nullable encode:

```gleam
to_string(object([#("game", string("Pac-Man")), #("score", int(3333360))]))
// "{\"game\":\"Pac-Man\",\"score\":3333360}"

parse("[1,2,3]", decode.list(of: decode.int))
// Ok([1, 2, 3])

nullable(Some(x), of: string)   // -> JSON value of x
nullable(None, of: string)      // -> JSON null
```

Record decoder via use-callback:

```gleam
let decoder = {
  use name <- decode.field("name", decode.string)
  use score <- decode.field("score", decode.int)
  use colour <- decode.field("colour", decode.string)
  use enrolled <- decode.field("enrolled", decode.bool)
  decode.success(Player(name:, score:, colour:, enrolled:))
}
decode.run(data, decoder)
// Ok(Player("Mel Smith", 180, "Red", True))
```

Enum decoder via `then` + `success` + `failure`:

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
decode.run(dynamic.string("water"), decoder)
// Ok(Water)
```

## Common mistakes

- Calling `to_string_builder` / `to_dynamic` / `object_take` / `null_as` /
  `decode` as a public fn (NOT in v3.1.0).
- Using `decode1..decodeN` / `sequence` (NOT in v1.0.3 — use `field`/`then`/`list`).
- Assuming `int` decoder coerces `1.0` (fails on Erlang).
- Letting `Dynamic` flow into typed internals (decode at the boundary).
- Using `classify` instead of `decode` for typed conversion.
- Not handling `parse` `Error` (untrusted input).
- Not testing decoders on both Erlang and JS targets.
- Pattern-matching on the raw runtime shape of a `Dynamic` (not portable).

## Strict vs contextual guidance

**Strict:** `Json` opaque (builders only); decode all `Dynamic` at the boundary;
always handle `parse`/`parse_bits` `Error`; no `decode1..N`/`sequence` (removed
in v1.0.3); `int`/`float` perform no coercion; never convert well-typed data to
`Dynamic`.

**Contextual:** `to_string` vs `to_string_tree` (IO-size dependent — prefer
`to_string_tree` for IO); `one_of` for numeric flexibility (JSON-shape dependent
— use when a number may arrive as either int or float).

## Policy decisions for individual repos

- JSON pretty-printing (if any) at the boundary; default serialiser
  (`to_string` vs `to_string_tree`).
- Default numeric decoder: `one_of([int, float])` vs schema-guaranteed type.
- Error-reporting shape: raw `List(DecodeError)` vs `collapse_errors`/
  `map_errors`-normalised.
- Decoders next to types vs a dedicated `decoders` module.

## Related docs

- `docs/gleam/json-dynamic-and-api-boundaries.md` (this doc)
- `docs/gleam/stdlib.md`
- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/erlang-interop.md`
- `docs/gleam/javascript-target.md`
- `docs/gleam/validation.md`
- `docs/beam/common-mistakes.md`
- `docs/beam/binaries.md`

## Related skills

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop`
