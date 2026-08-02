# Crawl: gleam/bit_array.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: stdlib.md, docs/beam/binaries

## Purpose
The `gleam/bit_array` module provides functions for working with `BitArray`
values — Gleam's type for arbitrary-length sequences of bits (Erlang binaries
and bitstrings). It covers construction helpers, slicing/sizing, base16/base64
encoding, UTF-8 conversion, comparison, inspection, and byte-padding. It is the
runtime library companion to Gleam's `<<>>` bit-array literal/match syntax;
the syntax creates the values, this module operates on them.

## Construction & matching
Gleam bit-arrays are written with the `<<>>` syntax, e.g. `<<1, 2, 3>>` or
`<<100, 5:size(3)>>`. Segments may carry a `:size(n)` specifier and other
type/endianness specifiers, mirroring Erlang's binary/bitstring segment syntax.
The `bit_array` module does **not** define the literal syntax (that is a
language-level construct); it provides functions that build, decompose, and
inspect values produced by that syntax. Construction-by-function is done via
`append`, `concat`, `from_string`, and the `base*_decode` family; matching is
done in `<<>>` patterns in `case` expressions, with `slice`/`starts_with`
offering non-pattern access to sub-sections.

## Key functions (signatures)
```gleam
pub fn append(to first: BitArray, suffix second: BitArray) -> BitArray
pub fn concat(bit_arrays: List(BitArray)) -> BitArray
pub fn bit_size(x: BitArray) -> Int
pub fn byte_size(x: BitArray) -> Int
pub fn slice(
  from string: BitArray,
  at position: Int,
  take length: Int,
) -> Result(BitArray, Nil)
pub fn compare(a: BitArray, with b: BitArray) -> order.Order
pub fn starts_with(bits: BitArray, prefix: BitArray) -> Bool
pub fn from_string(x: String) -> BitArray
pub fn to_string(bits: BitArray) -> Result(String, Nil)
pub fn is_utf8(bits: BitArray) -> Bool
pub fn inspect(input: BitArray) -> String
pub fn pad_to_bytes(x: BitArray) -> BitArray
```
Notes:
- No `from_list`/`to_list`/`equal` in this module (v1.0.3). Equality uses
  `compare(...) == order.Eq`; there is no list-of-ints round-trip helper here.
- `slice` returns `Result(BitArray, Nil)` — `Nil` on out-of-bounds. Negative
  `length` extracts bytes from the end. Documented as constant time.
- `compare` returns `order.Order` (`Lt`/`Eq`/`Gt`), comparing as byte sequences.

## base64 helpers
```gleam
pub fn base64_encode(input: BitArray, padding: Bool) -> String
pub fn base64_decode(encoded: String) -> Result(BitArray, Nil)
pub fn base64_url_encode(input: BitArray, padding: Bool) -> String
pub fn base64_url_decode(encoded: String) -> Result(BitArray, Nil)
pub fn base16_encode(input: BitArray) -> String
pub fn base16_decode(input: String) -> Result(BitArray, Nil)
```
- All encoders pad with zero bits first if the bit array is not a whole number
  of bytes.
- `base64_encode`/`base64_url_encode` take an explicit `padding: Bool` argument
  (whether to emit `=` padding).
- `base64_url_*` use the URL-and-filename-safe alphabet.
- Decoders return `Result(_, Nil)` — `Nil` on malformed input.

## Relationship to bit-array syntax + Erlang binaries
A Gleam `BitArray` is, at runtime, an Erlang binary/bitstring. The `<<>>`
literal syntax maps directly to Erlang's binary term syntax; segment specifiers
like `:size(n)` correspond to Erlang's `Size` segment qualifier. Functions in
this module are thin wrappers over Erlang BIFs / stdlib (`bit_size`/`byte_size`
mirror `bit_size/1` and `byte_size/1`; `slice` uses `binary.part/3` semantics;
base64/base16 wrap Erlang's `base64`/`base` modules). See
`docs/beam/binaries` for the underlying Erlang binary/bitstring model, segment
qualifiers (type, signedness, endianness, unit), and pattern-matching semantics
that the Gleam `<<>>` syntax inherits.

## Strict rules
- `to_string` returns `Error(Nil)` if the bit array is not valid UTF-8 — do not
  assume success; prefer `is_utf8` to pre-check or pattern-match on the result.
- `slice` is `Result`-returning; out-of-range positions yield `Error(Nil)`.
- Bit arrays that are not whole-byte multiples cannot be directly interpreted
  as UTF-8 or printed as a string; use `pad_to_bytes` to zero-pad to a byte
  boundary before byte-oriented operations.
- Base encoders silently zero-pad non-byte-aligned input; the decoded output
  may therefore include trailing zero bits that were not in the original.
- `inspect` prints the array syntax (`<<...>>`) regardless of UTF-8 validity;
  use it (not `string.inspect`) when the array form is desired even for valid
  UTF-8 content.

## Verbatim quotes
- `append`: "Creates a new bit array by joining two bit arrays."
- `concat`: "Creates a new bit array by joining multiple binaries."
- `bit_size`: "Returns an integer which is the number of bits in the bit array."
- `byte_size`: "Returns an integer which is the number of bytes in the bit array."
- `slice`: "Extracts a sub-section of a bit array. [...] A negative length can
  be used to extract bytes at the end of a bit array. This function runs in
  constant time."
- `compare`: "Compare two bit arrays as sequences of bytes."
- `from_string`: "Converts a UTF-8 String type into a BitArray."
- `to_string`: "Converts a bit array to a string. Returns an error if the bit
  array is invalid UTF-8 data."
- `is_utf8`: "Tests to see whether a bit array is valid UTF-8."
- `inspect`: "Converts a bit array to a string containing the decimal value of
  each byte. Use this over string.inspect when you have a bit array you want
  printed in the array syntax even if it is valid UTF-8."
- `pad_to_bytes`: "Pads a bit array with zeros so that it is a whole number of
  bytes."
- `base64_encode`: "Encodes a BitArray into a base 64 encoded string. If the
  bit array does not contain a whole number of bytes then it is padded with
  zero bits prior to being encoded."
- `base16_encode`: "Encodes a BitArray into a base 16 encoded string. If the bit
  array does not contain a whole number of bytes then it is padded with zero
  bits prior to being encoded."
- `inspect` example: `assert inspect(<<0, 20, 0x20, 255>>) == "<<0, 20, 32, 255>>"`
- `compare` example: `assert compare(<<1, 2:size(2)>>, with: <<1, 2:size(2)>>) == Eq`

## Version notes
- Page title: `gleam/bit_array · gleam_stdlib · v1.0.3`.
- Source links point to `github.com/gleam-lang/stdlib/blob/v1.0.3/src/gleam/bit_array.gleam`.
- Module surface (18 public fns): append, base16_decode, base16_encode,
  base64_decode, base64_encode, base64_url_decode, base64_url_encode, bit_size,
  byte_size, compare, concat, from_string, inspect, is_utf8, pad_to_bytes,
  slice, starts_with, to_string.
- Not present in v1.0.3: `from_list`, `to_list`, `equal`, `hash`, `xor`,
  `to_int`/`from_int`. (Older/newer versions may differ.)

## Discovered links
### Relevant (crawl later)
- ../gleam/order.html — `compare` returns `order.Order`; needed to document the
  `Lt`/`Eq`/`Gt` type used by `bit_array.compare`.
- ../gleam/string.html — `to_string`/`from_string` interplay with the `String`
  module; `string.inspect` referenced by `inspect` doc.
- ../gleam/bytes_tree.html — adjacent byte-buffer type (mutable-ish builder);
  useful contrast to immutable `BitArray`.
- ../gleam/string_tree.html — analogous "tree" builder; contrast for
  construction patterns.
- ../gleam/result.html — most decode/slice functions return `Result(_, Nil)`.
- docs/beam/binaries — Erlang binary/bitstring underpinnings (local doc target).

### Skipped
- ../index.html, ../, https://gleam.run/, GitHub sponsor/source/hex.pm links,
  CSS/asset URLs, intra-page `#` anchors, sibling module links not directly
  referenced by `bit_array` functions (bool, dict, dynamic, dynamic/decode,
  float, function, int, io, list, option, pair, set, uri).
