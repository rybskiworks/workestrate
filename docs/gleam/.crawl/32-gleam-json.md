# Crawl: gleam_json/gleam/json.html
- seed_url: https://hexdocs.pm/gleam_json/gleam/json.html
- canonical_url: https://gleam-json.hexdocs.pm/gleam/json.html
- family: Gleam core package (gleam_json)
- fetch: 200
- gleam_json_version: v3.1.0
- feeds_docs: json-dynamic-and-api-boundaries.md

## Purpose
`gleam/json` provides JSON encoding and decoding for Gleam. Encoding is done by
building a `Json` value from typed Gleam data via builder functions, then
serialising it to a `String` or `StringTree`. Decoding parses a JSON string into
dynamically typed data and then decodes it into typed Gleam data using the
`gleam/dynamic/decode` module (from gleam_stdlib).

## Json type + builders (signatures)
```gleam
pub type Json   // opaque encoded-JSON value
```

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

NOTE on spec vs. actual API (v3.1.0):
- `object_take` — NOT present. Use `object` with a `List(#(String, Json))`.
- `null_as` — NOT present. Use `null()` or `nullable(.., of: ..)`.
- `to_string_builder` — NOT present. The function is `to_string_tree` (returns
  `string_tree.StringTree`).
- `to_dynamic` — NOT present. Parsing returns typed-via-decoder directly; the
  intermediate dynamic step is internal.
- `decode` (as a public fn) — NOT present. Decoding is done by passing a
  `decode.Decoder(t)` to `parse`/`parse_bits`.

## to_string / to_string_builder / parse / decode
```gleam
pub fn to_string(json: Json) -> String
pub fn to_string_tree(json: Json) -> string_tree.StringTree
pub fn parse(from json: String, using decoder: decode.Decoder(t)) -> Result(t, DecodeError)
pub fn parse_bits(from json: BitArray, using decoder: decode.Decoder(t)) -> Result(t, DecodeError)
```
- `to_string` — serialise `Json` to a flat `String`.
- `to_string_tree` — serialise to `StringTree` (preferred; faster, BEAM IO
  optimised for StringTree).
- `parse` — parse a JSON `String` and decode it with a `decode.Decoder(t)` in
  one step, returning `Result(t, DecodeError)`.
- `parse_bits` — same as `parse` but accepts a `BitArray`.

## Encode model (builder → string/tree)
Encode is a two-stage, type-safe pipeline:
1. Build a `Json` value using typed builders (`string`, `int`, `float`, `bool`,
   `null`, `nullable`, `object`, `array`, `preprocessed_array`, `dict`). The
   `Json` type is opaque, so it can only be constructed via these builders.
2. Serialise with `to_string` (→ `String`) or `to_string_tree`
   (→ `string_tree.StringTree`, preferred for IO).

Example:
```gleam
to_string(object([
  #("game", string("Pac-Man")),
  #("score", int(3333360)),
]))
// "{\"game\":\"Pac-Man\",\"score\":3333360}"
```

## Decode model (parse → dynamic → decode combinators → typed)
Decoding is parse-then-decode in a single call, but conceptually:
1. `parse(json_string, using: decoder)` parses the JSON text into an internal
   dynamic representation.
2. The supplied `decode.Decoder(t)` (from `gleam/dynamic/decode`, gleam_stdlib)
   is applied to that dynamic value to produce typed Gleam data `t`.
3. On failure, returns `DecodeError`:
   - `UnexpectedEndOfInput` — malformed/truncated JSON.
   - `UnexpectedByte(String)` — invalid byte in JSON.
   - `UnexpectedSequence(String)` — invalid token sequence.
   - `UnableToDecode(List(decode.DecodeError))` — JSON parsed but the decoder
     rejected the shape (delegated to gleam_stdlib's dynamic/decode errors).

The dynamic intermediate is NOT exposed as a public `to_dynamic` function in
v3.1.0; the parser is coupled to a `decode.Decoder`. This means the
parse-then-decode pattern is the only documented entry point, and the dynamic
value type itself lives in gleam_stdlib (`gleam/dynamic`).

Example:
```gleam
parse("[1,2,3]", decode.list(of: decode.int))
// Ok([1, 2, 3])
parse("1", decode.string)
// Error(UnableToDecode([decode.DecodeError("String", "Int", [])]))
```

## Strict Rules
- `Json` is opaque: construct only via the builder functions; do not pattern
  match on it.
- Prefer `to_string_tree` over `to_string` for IO (BEAM VM optimised for
  StringTree).
- Decoding requires a `decode.Decoder(t)` from gleam_stdlib's
  `gleam/dynamic/decode`; there is no decoder-less `parse` returning raw
  `dynamic.Dynamic` in v3.1.0.
- `parse`/`parse_bits` return `Result(t, DecodeError)` — always handle the
  `Error` variant (JSON is untrusted input).
- Object keys must be `String`; use `dict` builder with a `keys` encoder for
  non-string keys.

## Verbatim quotes
- "`to_string` — Convert a JSON value into a string. Where possible prefer the
  to_string_tree function as it is faster than this function, and BEAM VM IO is
  optimised for sending StringTree data."
- "`to_string_tree` — Convert a JSON value into a string tree. Where possible
  prefer this function to the to_string function as it is slower than this
  function, and BEAM VM IO is optimised for sending StringTree data."
- "`parse` — Decode a JSON string into dynamically typed data which can be
  decoded into typed data with the gleam/dynamic module."
- "`null` — The JSON value null."
- "`nullable` — Encode an optional value into JSON, using null if it is the
  None variant."

## Version notes
- Page version: gleam_json v3.1.0 (title: `gleam/json · gleam_json · v3.1.0`).
- gleam_stdlib referenced at 0.60.0 (dynamic, dynamic/decode, dict, option,
  string_tree).
- API surface in v3.1.0 differs from older docs that may mention
  `to_string_builder`/`to_dynamic`/`object_take`/`null_as`/`decode` as a public
  fn — these are NOT present in v3.1.0. The current names are `to_string_tree`,
  `parse`/`parse_bits` (with a `decode.Decoder` argument), `object`,
  `nullable`/`null`.
- `DecodeError` has 4 variants; `UnableToDecode` wraps
  `List(decode.DecodeError)` from gleam_stdlib, confirming the parse→dynamic→
  decode pipeline boundary.

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/gleam_stdlib/0.60.0/gleam/dynamic/decode.html  (decode combinators: Decoder, DecodeError — core to the parse-then-decode pipeline)
- https://hexdocs.pm/gleam_stdlib/0.60.0/gleam/dynamic.html  (dynamic type — referenced by parse internals; not directly linked but implied)
- https://hexdocs.pm/gleam_stdlib/0.60.0/gleam/string_tree.html  (StringTree — return type of to_string_tree)
- https://hexdocs.pm/gleam_stdlib/0.60.0/gleam/dict.html  (Dict — used by dict builder)
- https://hexdocs.pm/gleam_stdlib/0.60.0/gleam/option.html  (Option — used by nullable builder)

### Skipped
- https://github.com/gleam-lang/json  (source repo, not hexdocs)
- https://github.com/gleam-lang/json/blob/v3.1.0/src/gleam/json.gleam#L...  (source line links)
- https://hex.pm/packages/gleam_json  (package page, not module docs)
- https://gleam.run/  (language site)
- ../gleam/json.html  (self)
