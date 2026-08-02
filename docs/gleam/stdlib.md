# Standard library (gleam_stdlib)

## Purpose

Gleam's standard library (`gleam_stdlib` v1.0.3). A single dependency every Gleam
project relies on for core data types and operations. Supports both compilation
targets: Erlang and JavaScript. Installed via `gleam add gleam_stdlib@1`. The
package is versioned independently of the Gleam compiler (compiler at time of
crawl: 1.16.0 per page metadata); the stdlib package itself is at v1.0.3.

The stdlib provides the `gleam/...` module namespace covering built-in types
(`Int`, `Float`, `String`, `List`, `Result`, `Option`, `Bool`, `Nil`), collection
types (`Dict`, `Set`), binary/byte handling (`BitArray`, `BytesTree`,
`StringTree`), dynamic/decode utilities for runtime-typed boundaries, and
miscellaneous helpers (`io`, `uri`, `function`, `pair`, `order`). Total: 19
modules.

## Sources used

- Crawl 17-stdlib-index.md — hexdocs `gleam_stdlib` index — https://gleam-stdlib.hexdocs.pm/
- Crawl 20-list.md — `gleam/list` — https://gleam-stdlib.hexdocs.pm/gleam/list.html
- Crawl 21-string.md — `gleam/string` — https://gleam-stdlib.hexdocs.pm/gleam/string.html
- Crawl 22-dict.md — `gleam/dict` — https://hexdocs.pm/gleam_stdlib/gleam/dict.html
- Crawl 18-result.md — `gleam/result` — https://gleam-stdlib.hexdocs.pm/gleam/result.html
- Crawl 19-option.md — `gleam/option` — https://gleam-stdlib.hexdocs.pm/gleam/option.html
- Crawl 23-dynamic.md — `gleam/dynamic` — https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
- Crawl 24-dynamic-decode.md — `gleam/dynamic/decode` — https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
- Crawl 25-bit-array.md — `gleam/bit_array` — https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html

## Related BEAM guidance

Gleam-specific; the `gleam/list` and `gleam/dict` modules parallel BEAM
collection concepts but the Gleam APIs differ from raw Erlang. Note the
divergence: a Gleam `List(a)` is an immutable singly-linked cons-list (not an
array); `Dict(k, v)` is HAMT-backed (Erlang map since OTP 18) but explicitly
**unordered** — never assume ordered iteration. `gleam/bit_array` maps directly
to the Erlang binary/bitstring model and inherits the `<<>>` segment qualifiers
(type, signedness, endianness, unit). Cross-ref: `docs/beam/binaries.md`.

## Core guidance

### gleam/list (crawl 20)

Singly-linked immutable persistent cons-list. Literal/prepend syntax:
`[head, ..tail]`. 64 public members (1 type `ContinueOrStop(a)` + 63 functions).

Performance model:
- `prepend` / `[x, ..xs]`, `first`, `rest`, `is_empty`, `wrap`, `new` — **O(1)**.
- `length`, `reverse`, `append`, `take`, `drop`, `flatten`, `last`, `sort`, `max`
  — **O(n)**; `reverse`/`length` are VM-native and highly optimised; `drop`
  traverses but does NOT copy.
- `fold`, `fold_until`, `try_fold`, `map`, `filter`, `scan`, `index_map` —
  tail-recursive / accumulator-based; safe for large lists.
- `fold_right` — **O(n)** time, **O(n)** stack, NOT tail-recursive; prefer `fold`.
- `group` — O(n), returns `dict.Dict`, does NOT preserve input order.
- `zip` truncates to the shorter list; `strict_zip` returns `Error(Nil)` on
  length mismatch.
- `combinations` / `permutations` / `combination_pairs` — exponential /
  factorial; avoid on large inputs.
- Random access by index is **O(n)** — there is no `list[i]`.

Key combinators: `fold` (default choice), `fold_until` (early-exit via
`ContinueOrStop(a)`), `try_fold` / `try_map` (short-circuit on `Error`),
`filter_map` (drops `Error`s silently), `try_map` (propagates first `Error`),
`map_fold` (fold + map in one pass). `find`/`first`/`rest`/`last`/`reduce`/`max`
return `Result(_, Nil)` to signal absence.

NOT present in v1.0.3: `at`, `slice`, `range`, `pop_map`. Compose `take`+`drop`
or `split` for slicing; build ranges via `gleam/iterator` or a fold.

### gleam/string (crawl 21)

Strings are UTF-8 binaries. 39 public members (38 functions + `UtfCodepoint`
accessors; the type itself is opaque).

- `length` counts **grapheme clusters** (linear; avoid in loops).
- `byte_size` counts **bytes** (constant on Erlang, linear on JavaScript).
- `compare` is lexicographic by **graphemes**, not by length or byte order.
- `slice` indexes by grapheme; negative index counts from the end.
- `replace` replaces **all** occurrences (no first-only variant).
- `trim` whitespace = Unicode `Pattern_White_Space` (UAX #31), not ASCII.
- `pop_grapheme` has "notable overhead"; prefer `to_graphemes` / `split`.
- `append` / `concat` typically **copy** both/all strings (linear); use
  `gleam/string_tree` for large or repeated joins.
- `repeat` is loglinear.
- All functions are total; failures return `Result(_, Nil)`.

NOT present: `equals` (use `==`), `to_int` / `to_float` (live in `gleam/int` and
`gleam/float` as `parse`), `from_string_tree` (see `gleam/string_tree`).

### gleam/dict (crawl 22)

`Dict(k, v)` associative map, immutable and persistent. Backed by Erlang's
`map` (HAMT since OTP 18) on Erlang; JavaScript `Map` on the JavaScript target.

- `get` returns `Result(v, Nil)` (NOT `Option`); `Error(Nil)` signals absence.
- `upsert`'s callback receives `option.Option(v)` (`Some(v)` if present, `None`
  if absent) and must return the new value.
- `merge`: second dict wins on key collisions. `combine`: invokes
  `fn(v, v) -> v` to resolve collisions.
- `size` is **O(1)** (constant time, no iteration).
- **No ordering guarantee** — `to_list`, `keys`, `values`, `fold`, `each` visit
  entries in an unspecified order that may change between Gleam/Erlang versions.
- Target-dependent key equality: Erlang `=:=` structural term equality (`1` and
  `1.0` are distinct keys); JavaScript `Map` SameValueZero (`NaN === NaN`,
  `-0`/`+0` same).

### gleam/result (crawl 18)

Combinators for `Result(a, e)`. Gleam has **no exceptions** — no `throw`,
`try`/`catch`, `raise`/`rescue`. Any operation that can fail returns a `Result`.

- `try` is the monadic bind (equivalent to `map` + `flatten`); use it to chain
  fallible computations.
- `all` short-circuits on the first `Error`.
- `partition` returns `#(List(a), List(e))` with values in **reverse order**
  relative to the input.
- `values` drops `Error`s (never fails).
- `lazy_or` / `lazy_unwrap` defer the fallback computation (use when the
  fallback is expensive or has side effects).
- `map` transforms the `Ok` value; `map_error` transforms the `Error`.
- `try_recover` attempts recovery from an `Error` while staying in `Result`.

Function set in v1.0.3: `all`, `flatten`, `is_error`, `is_ok`, `lazy_or`,
`lazy_unwrap`, `map`, `map_error`, `or`, `partition`, `replace`, `replace_error`,
`try`, `try_recover`, `unwrap`, `unwrap_error`, `values`.

NOT present in v1.0.3: `then`, `combine`, `nil_error`, `from`, `from_option`,
`to_option`, `get`, `recover`. (`try` = `then`; `all` = `combine`;
`try_recover` = `recover`.)

### gleam/option (crawl 19)

`Option(a) = Some(a) | None`. Gleam's alternative to a nullable value.

- Use `Option` **only** for optional function arguments and optional
  data-structure fields.
- Do NOT use `Option` as a return type for fallible operations — use `Result`
  (with `Nil` error if no extra detail). This consistency removes Option/Result
  conversion boilerplate.
- `then` is the bind (equivalent to `map` + `flatten`).
- `all` / `flatten` / `values` mirror the `result` combinators.
- `from_result` discards the error; `to_result` injects an explicit error value
  on `None`.

Function set in v1.0.3: `all`, `flatten`, `from_result`, `is_none`, `is_some`,
`lazy_or`, `lazy_unwrap`, `map`, `or`, `then`, `to_result`, `unwrap`, `values`.

NOT present: `some` / `none` (use the `Some` / `None` constructors), `to_list`,
`filter`, `take`, `zip`, `map2`, `contains`, `get_or_insert`.

### gleam/dynamic (crawl 23)

The **construction** side of Gleam's boundary with untyped runtime data. A
`Dynamic` value's type is unknown at compile time — typically from Erlang FFI
interop or IO with the outside world (e.g. JSON parsed into an untyped blob
before decoding).

Constructors: `array`, `bit_array`, `bool`, `classify`, `float`, `int`, `list`,
`nil`, `properties`, `string` (10 functions + the `Dynamic` type). `classify`
returns a `String` naming the runtime type — for diagnostics/error messages
only, not for typed conversion. Runtime representation is target-dependent
(`nil` is the atom `nil` on Erlang, `undefined` on JS; `string` is a binary on
Erlang, not a charlist; `properties` is a map on Erlang, a Gleam dict object on
JS).

The page states: "You will likely mostly use the other module [decode] in your
projects." NOTE: decoders (`from`, `field`, `DecodeError`, `unsafe_coerce`,
etc.) moved to `gleam/dynamic/decode` in modern versions; any docs referencing
`dynamic.from` / `dynamic.field` describe the legacy layout.

### gleam/dynamic/decode (crawl 24)

The **decode** side. `Decoder(t)` is opaque — constructible only via the
provided combinators.

Primitives (consts): `bit_array`, `bool`, `float`, `int`, `string`, `dynamic`
(`bit_array`/`dynamic` never error).

Combinators (use-callback style — each takes a `next` callback): `field`,
`subfield`, `optional_field`, `then`, `success`, `failure`. Plus `at` /
`optionally_at`, `list` (NOT `sequence`), `optional` (returns `Option(a)`),
`one_of`, `map`, `then`, `recursive`, `run`, `dict`, `collapse_errors`,
`map_errors`, `decode_error`, `new_primitive_decoder`.

- `int` does NOT coerce floats to ints (fails on `1.0` on Erlang).
- `float` does NOT coerce ints to floats (fails on Erlang for ints — relevant
  when decoding JSON where numbers may arrive as ints).
- Use `one_of` to decode values that may be either int or float.
- `run` returns `Result(t, List(DecodeError))` — collects **all** errors (the
  decoder continues after a field fails using placeholder values).
- `one_of` returns the errors of the FIRST decoder if none succeed; use
  `collapse_errors` / `map_errors` to customise.
- `field` errors if the field is absent; `optional_field` / `optionally_at`
  return a default.
- `optional` handles `nil`/`null`/`undefined` across runtimes.
- `at` / `field` / `subfield` index into dicts with any key type; int keys also
  index Erlang tuples, JS arrays, and the first 8 elements of Gleam lists.
- Test decoders on all supported platforms (Erlang + JavaScript) — runtime data
  structures differ.
- The LSP has a "generate dynamic decoder" code action that generates a decoder
  from a custom type definition.

NOT present in v1.0.3: `decode1`..`decodeN` arity combinators, `sequence`.
Record decoding uses the `use`-callback style of `field` / `subfield` /
`optional_field` / `then`.

### gleam/bit_array (crawl 25)

`BitArray` = Erlang binary/bitstring. `<<>>` literal syntax (e.g. `<<1, 2, 3>>`
or `<<100, 5:size(3)>>`); segment specifiers mirror Erlang's binary segment
syntax. 18 public functions.

- `append`, `concat`, `bit_size`, `byte_size`.
- `slice(from, at, take) -> Result(BitArray, Nil)` — constant time; negative
  `length` extracts bytes from the end; `Error(Nil)` on out-of-bounds.
- `compare(a, with: b) -> order.Order` — compares as byte sequences.
- `starts_with`, `from_string`, `to_string` (returns `Result(String, Nil)`,
  errors on invalid UTF-8), `is_utf8`, `inspect` (prints `<<...>>` array syntax
  regardless of UTF-8 validity), `pad_to_bytes` (zero-pads to a byte boundary).
- `base64_encode(input, padding: Bool)` / `base64_url_encode` /
  `base64_decode` / `base64_url_decode` / `base16_encode` / `base16_decode`.
  Encoders zero-pad non-byte-aligned input first; decoders return
  `Result(_, Nil)` on malformed input. `base64_url_*` use the URL-and-filename-
  safe alphabet.

NOT present in v1.0.3: `from_list`, `to_list`, `equal`, `hash`, `xor`,
`to_int`, `from_int`. Equality uses `compare(...) == order.Eq`. Cross-ref
`docs/beam/binaries.md`.

### Index-only modules (from crawl 17)

These modules are documented at index level only; consult hexdocs for full
signatures.

- `gleam/int` — Integer arithmetic, parsing, and bitwise operations on `Int`.
- `gleam/float` — Operations and math functions on `Float`.
- `gleam/bool` — Boolean logic helpers (`and`, `or`, `not`, `nand`, `nor`, `xor`, `guard`).
- `gleam/function` — Function combinators (`identity`, `compose`, `constant`, `tap`, `apply`).
- `gleam/io` — Stdout/stderr printing and debugging (`println`, `debug`).
- `gleam/set` — Operations on the `Set(a)` unique-element collection type.
- `gleam/order` — The `Order` type (`Lt`/`Eq`/`Gt`) and comparison combinators.
- `gleam/pair` — Helpers for 2-tuples (`first`, `second`, `swap`, `map`).
- `gleam/uri` — URI parsing, building, and percent-encoding.
- `gleam/bytes_tree` — Efficient append-only builder for `BitArray` (byte) data.
- `gleam/string_tree` — Efficient append-only builder for `String`.
- `gleam/iterator` — lazy collection (referenced by the dict crawl; the list
  crawl notes "build via `gleam/iterator` or a fold" for ranges).

## Practical rules

- Prefer `fold` over `fold_right` (tail-recursion; `fold_right` is not
  tail-recursive and uses more memory).
- Prefer `gleam/string_tree` for large or repeated string joins (avoid `append`
  copying).
- Never rely on `Dict` / `group` iteration order.
- Use `Result` (not `Option`) for fallible returns; `Option` only for optional
  args/fields.
- Decode all `Dynamic` at boundaries via `gleam/dynamic/decode`; never use
  `Dynamic` internally in typed code.
- `int` / `float` decoders do not coerce — use `one_of` for numeric flexibility.
- Avoid `combinations` / `permutations` on large lists (exponential/factorial).
- Random access by index is O(n) — there is no `list[i]`.
- Use `strict_zip` (not `zip`) when a length mismatch must be an error.
- Use `try_map` to propagate the first `Error`; `filter_map` to silently drop
  `Error`s — choose by intent.

## Review checklist

- [ ] No `fold_right` on unbounded/large lists.
- [ ] No reliance on `Dict` / `group` / `partition` iteration order.
- [ ] Fallible returns are `Result`, not `Option`.
- [ ] `Option` used only for optional args/fields.
- [ ] All `Dynamic` decoded at boundaries via `gleam/dynamic/decode`.
- [ ] No `dynamic.classify` used for typed conversion (diagnostics only).
- [ ] No `int` decoder assumed to coerce `1.0` (use `one_of`).
- [ ] No `list.at` / `list.slice` / `list.range` (absent in v1.0.3).
- [ ] No `result.then` / `combine` / `recover` (absent; use `try` / `all` /
      `try_recover`).
- [ ] No `decode.decode1..N` / `sequence` (absent; use `use`-callback `field`).
- [ ] No `bit_array.from_list` / `to_list` / `equal` / `xor` (absent).
- [ ] `to_string` on a `BitArray` is checked for `Error(Nil)` (invalid UTF-8).

## Implementation checklist

- [ ] Pin `gleam_stdlib@1` in `gleam add`.
- [ ] Use `[head, ..tail]` for prepend (O(1)); accumulate + `reverse` when
      building a list in order.
- [ ] Use `fold` / `fold_until` / `try_fold` for list reduction.
- [ ] Use `gleam/string_tree` for large joins.
- [ ] Use `dict.get` (returns `Result(v, Nil)`) and `dict.upsert` (callback
      receives `option.Option(v)`).
- [ ] Use `result.try` to chain fallible computations.
- [ ] Use `decode.field` + `use` + `decode.success` for record decoders.
- [ ] Use `decode.one_of` for int-or-float numeric fields.
- [ ] Use `bit_array.base64_encode(input, padding: Bool)` for base64.
- [ ] Test decoders on both Erlang and JavaScript targets.

## Validation hooks

- `gleam build` — compile check (catches absent functions like `list.at`,
  `result.then`, `decode.sequence`).
- `gleam test` — run the test suite.
- LSP "generate dynamic decoder" code action — scaffold decoders from custom
  types (crawl 24).
- Cross-target decoder tests — run on both Erlang and JavaScript because
  runtime data structures differ (crawl 24).

## Examples

```gleam
// list: fold + prepend
let sum = list.fold([1, 2, 3], 0, fn(acc, n) { acc + n })
let prepended = [0, ..[1, 2, 3]]   // [0, 1, 2, 3]

// dict: get / insert
let d = dict.from_list([#("a", 1)])
let v = dict.get(d, "a")           // Ok(1)
let d2 = dict.insert(d, "b", 2)

// result: try chain (monadic bind)
let parse = fn(s) { result.try(int.parse(s), fn(n) { Ok(n + 1) }) }

// option: then (bind)
let x = option.then(Some(1), fn(n) { Some(n + 1) })   // Some(2)

// decode: use-callback record decoder (crawl 24)
let player_decoder = {
  use name <- decode.field("name", decode.string)
  use score <- decode.field("score", decode.int)
  decode.success(Player(name:, score:))
}
// decode.run(data, player_decoder) -> Result(Player, List(DecodeError))

// int-or-float via one_of (crawl 24)
let number = decode.one_of(decode.int, [decode.float])

// bit_array: base64 + to_string
let encoded = bit_array.base64_encode(<<1, 2, 3>>, padding: True)
let text = case bit_array.to_string(<<72, 105>>) {
  Ok(s) -> s
  Error(Nil) -> "<invalid utf-8>"
}
```

## Common mistakes

- Using `fold_right` on large lists (not tail-recursive; uses more memory).
- Relying on `Dict` / `group` / `partition` iteration order (unspecified).
- Using `Option` as a fallible return type (should be `Result`).
- Reaching for `dynamic.classify` instead of `decode` for typed conversion.
- Assuming the `int` decoder coerces `1.0` (it fails on Erlang).
- Using `list.at` / `list.slice` / `list.range` (do not exist in v1.0.3).
- Using `result.then` / `combine` / `recover` (absent; use `try` / `all` /
  `try_recover`).
- Using `decode.decode1..N` / `sequence` (absent; use `use`-callback `field`).
- Using `bit_array.from_list` / `to_list` / `equal` / `xor` (absent).
- Calling `string.length` in a loop (linear; avoid).
- Using `string.append` for large joins (copies; use `gleam/string_tree`).
- Assuming `zip` errors on length mismatch (it truncates; use `strict_zip`).

## Strict vs contextual guidance

Strict:
- `Result` for fallible returns; `Option` only for optional args/fields.
- Decode all `Dynamic` at boundaries via `gleam/dynamic/decode`.
- No reliance on `Dict` / `group` / `partition` iteration order.
- `fold` over `fold_right`.
- No exceptions (Gleam has none).

Contextual:
- `gleam/string_tree` vs `string.append` — size-dependent (large/repeated joins
  favour the tree).
- `one_of` for int/float decoding — JSON-dependent (numbers may arrive as ints
  or floats).
- `filter_map` vs `try_map` — intent-dependent (drop vs propagate `Error`).
- `zip` vs `strict_zip` — whether length mismatch is an error.

## Policy decisions for individual repos

- Pin `gleam_stdlib` major version (`gleam add gleam_stdlib@1`).
- Decide whether to standardise on `Result(_, Nil)` for absence or to use
  `Option` only at field/arg boundaries (per the stdlib's own convention).
- Decide a decoder-testing policy: require both Erlang and JavaScript target
  tests for any `gleam/dynamic/decode` decoder (runtime data structures differ).
- Decide whether to forbid `fold_right` in lint/review (prefer `fold`).

## Related docs

- `docs/gleam/stdlib` (this doc)
- `docs/gleam/result-option-and-errors`
- `docs/gleam/json-dynamic-and-api-boundaries`
- `docs/gleam/externals-and-ffi`
- `docs/gleam/erlang-interop`
- `docs/gleam/javascript-target`
- `docs/gleam/validation`
- `docs/beam/binaries.md`

## Related skills

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop`
