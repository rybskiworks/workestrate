# Crawl: gleam/string.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/string.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/string.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: stdlib.md

## Purpose
Strings in Gleam are UTF-8 binaries. They can be written in code as text
surrounded by `"double quotes"`. The `gleam/string` module provides the
core operations on `String` values: construction, inspection, splitting,
joining, casing, trimming, padding, slicing, grapheme handling, and
UTF-codepoint conversion. All functions are total (no exceptions); errors
are reported via `Result(..., Nil)`.

## Key functions (categories + signatures)

### Construction / concatenation
- `pub fn append(to first: String, suffix second: String) -> String`
  — joins two strings; typically copies both, linear time. For large joins
  consider `gleam/string_tree`.
- `pub fn concat(strings: List(String)) -> String`
  — joins many strings; copies all, linear time.
- `pub fn join(strings: List(String), with separator: String) -> String`
  — joins with a separator, linear time.
- `pub fn repeat(string: String, times times: Int) -> String`
  — repeats a string `times`; loglinear time.
- `pub fn from_utf_codepoints(utf_codepoints: List(UtfCodepoint)) -> String`
  — builds a String from a list of codepoints.

### Inspection / size
- `pub fn inspect(term: anything) -> String`
  — Gleam-syntax string representation of any term. "Quick-and-dirty"
  printing; prefer explicit construction for error reporting.
- `pub fn length(string: String) -> Int`
  — number of **grapheme clusters**; iterates whole string, linear time.
  Avoid in loops.
- `pub fn byte_size(string: String) -> Int`
  — number of **bytes**; constant time on Erlang, linear on JavaScript.
- `pub fn is_empty(str: String) -> Bool`
- `pub fn to_option(string: String) -> option.Option(String)`
  — empty string becomes `None`.

### Casing
- `pub fn uppercase(string: String) -> String`
  — all graphemes to uppercase.
- `pub fn lowercase(string: String) -> String`
  — all graphemes to lowercase. Useful for case-insensitive comparisons.
- `pub fn capitalise(string: String) -> String`
  — first grapheme uppercase, remaining graphemes lowercase.

### Comparison / matching
- `pub fn compare(a: String, b: String) -> order.Order`
  — lexicographic by **graphemes**, not by size/length.
- `pub fn contains(does haystack: String, contain needle: String) -> Bool`
- `pub fn starts_with(string: String, prefix: String) -> Bool`
- `pub fn ends_with(string: String, suffix: String) -> Bool`
- `pub fn equals` — not present as a dedicated function; use `==`.

### Splitting / slicing
- `pub fn split(x: String, on substring: String) -> List(String)`
- `pub fn split_once(string: String, on substring: String) -> Result(#(String, String), Nil)`
  — single split; `Error(Nil)` if substring absent.
- `pub fn slice(from string: String, at_index idx: Int, length len: Int) -> String`
  — substring by grapheme index + length; negative index counts from end.
- `pub fn to_graphemes(string: String) -> List(String)`
- `pub fn first(string: String) -> Result(String, Nil)`
  — first grapheme cluster; `Error(Nil)` if empty.
- `pub fn last(string: String) -> Result(String, Nil)`
  — last grapheme cluster; traverses whole string.
- `pub fn pop_grapheme(string: String) -> Result(#(String, String), Nil)`
  — splits into head + tail (list-like). Notable overhead; prefer
  `to_graphemes`/`split` for performance-sensitive code.
- `pub fn drop_start(from string: String, up_to num_graphemes: Int) -> String`
  — drops n graphemes from start; linear in number dropped.
- `pub fn drop_end(from string: String, up_to num_graphemes: Int) -> String`
  — drops n graphemes from end; traverses full string, linear. Avoid in loops.
- `pub fn crop(from string: String, before substring: String) -> String`
  — drops content before the substring; unchanged if substring absent.
- `pub fn reverse(string: String) -> String`
  — reverses; linear time. Avoid in loops.

### Replacement / prefix-suffix removal
- `pub fn replace(in string: String, each pattern: String, with substitute: String) -> String`
  — replaces **all** occurrences of a substring.
- `pub fn remove_prefix(from string: String, matching prefix: String) -> String`
  — unchanged if prefix not present.
- `pub fn remove_suffix(from string: String, matching suffix: String) -> String`
  — unchanged if suffix not present.

### Trimming / padding
- `pub fn trim(string: String) -> String`
  — removes whitespace both sides. Whitespace = nonbreakable whitespace
  codepoints, defined as `Pattern_White_Space` in Unicode Standard Annex #31.
- `pub fn trim_start(string: String) -> String`
- `pub fn trim_end(string: String) -> String`
- `pub fn pad_start(string: String, to desired_length: Int, with pad_string: String) -> String`
  — pads start until given (grapheme) length; no-op if already longer.
- `pub fn pad_end(string: String, to desired_length: Int, with pad_string: String) -> String`

### UTF codepoint conversion
- `pub fn to_utf_codepoints(string: String) -> List(UtfCodepoint)`
- `pub fn utf_codepoint(value: Int) -> Result(UtfCodepoint, Nil)`
  — `Error(Nil)` if integer is not a valid UTF codepoint.
- `pub fn utf_codepoint_to_int(cp: UtfCodepoint) -> Int`

### Parse helpers
- `to_int` / `to_float` — **not present** in `gleam/string`. Integer/float
  parsing lives in `gleam/int` (`parse`) and `gleam/float` (`parse`).
- `from_string_tree` — **not present**; see `gleam/string_tree` module
  (linked from `append` docs) for tree-based construction.

## UTF-8 / grapheme semantics
- Strings are **UTF-8 binaries** at the value level.
- `length` counts **grapheme clusters**, not bytes and not code points.
  Example: a grapheme cluster may comprise multiple code points (e.g.
  flag emoji, ZWJ sequences).
- `byte_size` counts **bytes** (constant time on Erlang, linear on JS).
  Example from docs: `byte_size("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻") == 58`.
- `to_graphemes` returns a `List(String)` of grapheme clusters.
- `first`/`last`/`pop_grapheme` operate on **grapheme clusters**, returning
  `Result(String, Nil)` (or `Result(#(String, String), Nil)` for pop).
- `slice`, `drop_start`, `drop_end`, `pad_*` index/measure by **graphemes**.
- `compare` orders by **graphemes**, not by byte length.
- `to_utf_codepoints` / `from_utf_codepoints` operate on **code points**
  (see Unicode code point definitions), distinct from grapheme clusters.
- `trim` whitespace set = `Pattern_White_Space` per Unicode Standard
  Annex #31 (nonbreakable whitespace codepoints).

## Strict rules / caveats
- All functions are **total**; failures return `Result(_, Nil)` rather than
  crashing. `utf_codepoint` returns `Error(Nil)` for invalid code points;
  `first`/`last`/`split_once`/`pop_grapheme` return `Error(Nil)` when the
  sought substring/element is absent.
- `length`, `last`, `drop_end`, `reverse` traverse the whole string →
  **linear time**; the docs explicitly warn "Avoid using this in a loop."
- `append`/`concat` typically **copy** both/all strings (linear time). For
  large or repeated joins, use `gleam/string_tree` to avoid copying.
- `repeat` is **loglinear** time.
- `pop_grapheme` has "notable overhead"; prefer `to_graphemes` or `split`
  in performance-sensitive code.
- `slice` negative indexes count from the **end** of the string.
- `replace` replaces **all** occurrences (no first-only variant here; use
  `split_once` + recombine if you need single replacement).
- `compare` is **lexicographic by grapheme**, not by length or byte order.
- `byte_size` runtime behaviour differs: constant on Erlang, linear on JS.
- `trim`'s whitespace is the Unicode `Pattern_White_Space` set, not the
  ASCII whitespace set.
- No `equals`/`to_int`/`to_float`/`from_string_tree` functions exist in
  this module (see `==`, `gleam/int`, `gleam/float`, `gleam/string_tree`).

## Verbatim quotes
- Module intro: "Strings in Gleam are UTF-8 binaries. They can be written
  in your code as text surrounded by `\"double quotes\"`."
- `append`: "This function typically copies both Strings and runs in
  linear time, but the exact behaviour will depend on how the runtime you
  are using optimises your code."
- `append`: "If you are joining together large string and want to avoid
  copying any data you may want to investigate using the `string_tree`
  module."
- `byte_size`: "Returns the number of bytes in a String. This function
  runs in constant time on Erlang and in linear time on JavaScript."
- `length`: "Gets the number of grapheme clusters in a given String. This
  function has to iterate across the whole string to count the number of
  graphemes, so it runs in linear time. Avoid using this in a loop."
- `compare`: "Compares two Strings to see which is 'larger' by comparing
  their graphemes. This does not compare the size or length of the given
  Strings."
- `trim`: "Whitespace in this function is the set of nonbreakable
  whitespace codepoints, defined as Pattern_White_Space in Unicode
  Standard Annex #31."
- `pop_grapheme`: "There is a notable overhead to using this function."
- `inspect`: "This may be occasionally useful for quick-and-dirty
  printing of values in scripts. For error reporting and other uses prefer
  constructing strings explicitly."
- `uppercase`: "Useful for case-insensitive comparisons and VIRTUAL
  YELLING."

## Version notes
- Page reports `gleam_stdlib · v1.0.3`.
- Source links point to `github.com/gleam-lang/stdlib/blob/v1.0.3/src/gleam/string.gleam`.
- 39 public members documented (38 functions + the `UtfCodepoint`-related
  constructors/accessors). No types beyond `UtfCodepoint` are defined in
  this module's public surface (the type itself is opaque, accessed via
  `utf_codepoint` / `to_utf_codepoints` / `utf_codepoint_to_int`).

## Discovered links

### Relevant (crawl later)
- ../gleam/string_tree.html — recommended by `append` for large joins.
- ../gleam/bytes_tree.html — sibling tree builder.
- ../gleam/order.html — return type of `compare`.
- ../gleam/option.html — return type of `to_option`.
- ../gleam/int.html — `parse` for string→int (not in `string`).
- ../gleam/float.html — `parse` for string→float (not in `string`).
- ../gleam/bit_array.html — underlying binary representation.
- ../gleam/result.html — `Result` type used throughout.
- ../gleam/list.html — return type of `split`/`to_graphemes`.

### Skipped
- ../gleam/bool.html
- ../gleam/dict.html
- ../gleam/dynamic.html
- ../gleam/dynamic/decode.html
- ../gleam/function.html
- ../gleam/io.html
- ../gleam/pair.html
- ../gleam/set.html
- ../gleam/uri.html
- ../index.html
