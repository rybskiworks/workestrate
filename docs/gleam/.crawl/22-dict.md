# Crawl: gleam/dict.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/dict.html
- canonical_url: https://hexdocs.pm/gleam_stdlib/gleam/dict.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: stdlib.md

## Purpose
`gleam/dict` provides the `Dict(k, v)` associative key→value collection for Gleam.
A dictionary maps keys of one type to values of one type; each key can appear at
most once. Dicts are immutable and persistent — every "mutation" returns a new
`Dict`. The module is the standard way to model finite maps in Gleam and is the
backing type used pervasively across the stdlib (e.g. as the result of grouping,
counting, or config lookups).

## Type + key functions (signatures)
```gleam
pub type Dict(key, value)

pub fn new() -> Dict(k, v)
pub fn from_list(list: List(#(k, v))) -> Dict(k, v)
pub fn to_list(dict: Dict(k, v)) -> List(#(k, v))

pub fn get(from: Dict(k, v), get: k) -> Result(v, Nil)
pub fn insert(into dict: Dict(k, v), for key: k, insert value: v) -> Dict(k, v)
pub fn upsert(
  in dict: Dict(k, v), update key: k, with fun: fn(option.Option(v)) -> v,
) -> Dict(k, v)
pub fn delete(from dict: Dict(k, v), delete key: k) -> Dict(k, v)

pub fn merge(into dict: Dict(k, v), from new_entries: Dict(k, v)) -> Dict(k, v)
pub fn combine(
  dict: Dict(k, v), other: Dict(k, v), with fun: fn(v, v) -> v,
) -> Dict(k, v)

pub fn size(dict: Dict(k, v)) -> Int
pub fn is_empty(dict: Dict(k, v)) -> Bool
pub fn has_key(dict: Dict(k, v), key: k) -> Bool
pub fn keys(dict: Dict(k, v)) -> List(k)
pub fn values(dict: Dict(k, v)) -> List(v)

pub fn each(dict: Dict(k, v), fun: fn(k, v) -> a) -> Nil
pub fn map_values(in dict: Dict(k, v), with fun: fn(k, v) -> a) -> Dict(k, a)
pub fn fold(
  over dict: Dict(k, v), from initial: acc, with fun: fn(acc, k, v) -> acc,
) -> acc
pub fn filter(
  in dict: Dict(k, v), keeping predicate: fn(k, v) -> Bool,
) -> Dict(k, v)

pub fn take(from dict: Dict(k, v), keeping desired_keys: List(k)) -> Dict(k, v)
pub fn drop(
  from dict: Dict(k, v), drop disallowed_keys: List(k),
) -> Dict(k, v)
```

Notes on the API surface:
- `get` returns `Result(v, Nil)` (not `Option`) — `Error(Nil)` signals absence.
- `upsert`'s callback receives `option.Option(v)` (`Some(v)` if present, `None`
  if absent) and must return the new value.
- `combine` differs from `merge`: `merge` lets the second dict win on key
  collisions; `combine` invokes `fn(v, v) -> v` to resolve collisions.
- `map_values` is the value-transforming map (key also supplied to the fn);
  there is no key-transforming variant.
- `each` returns `Nil` and is for side effects; iteration order is unspecified.
- `size` is O(1) (constant time, no iteration).

## Implementation note (HAMT) + key equality
The page states the implementation is backed by Erlang's map and directs readers
to "the Erlang map module for more information." On the Erlang/OTP target,
`Dict(k, v)` is represented by Erlang's `map` type, which since OTP 18 is
implemented as a Hash Array Mapped Trie (HAMT) — a persistent, structurally
shared trie giving effectively O(log n) insert/lookup with good constant
factors. On the JavaScript target, Gleam compiles `Dict` to a JavaScript `Map`
(see `gleam_stdlib` JS prelude), preserving the immutable-by-convention API.

Key equality follows the underlying runtime's term equality:
- Erlang target: structural term equality (`=:=`), as used by Erlang maps.
  This means `1` and `1.0` are distinct keys (integer vs float), and `#(1, 2)`
  keys compare structurally.
- JavaScript target: JavaScript `Map` SameValueZero equality (like `===` but
  `NaN === NaN` and `-0`/`+0` treated as same).

Because equality is runtime-defined, the same Gleam source can exhibit
different key-deduplication behaviour for numeric keys across targets.

## Strict rules / caveats
- **No ordering guarantee.** Dicts are explicitly unordered. `to_list`,
  `keys`, `values`, `fold`, and `each` return/visit entries in an unspecified
  order that may change between Gleam or Erlang versions. Never write code that
  depends on iteration order.
- **Immutable / persistent.** All operations return new `Dict` values; none
  mutate in place.
- **Homogeneous keys and values.** All keys must share one type and all values
  must share one type (enforced by Gleam's type system).
- **Unique keys.** Each key present at most once. `insert` replaces an existing
  value for the key; `from_list` keeps the last tuple when keys collide.
- **`get` uses `Result(v, Nil)`, not `Option`.** Prefer `Result`-style handling
  rather than reaching for `Option` here.
- **`size` is O(1).** Documented as constant time; safe to call frequently.
- **Target-dependent equality.** Numeric key equality differs between Erlang
  and JavaScript targets (see above).

## Verbatim quotes
- "A dictionary of keys and values. Any type can be used for the keys and values
  of a dict, but all the keys must be of the same type and all the values must be
  of the same type. Each key can only be present in a dict once."
- "Dicts are not ordered in any way, and any unintentional ordering is not to be
  relied upon in your code as it may change in future versions of Erlang or
  Gleam. See the Erlang map module for more information."
- `from_list`: "If two tuples have the same key the last one in the list will be
  the one that is present in the dict."
- `get`: "The dict may not have a value for the key, so the value is wrapped in
  a Result."
- `insert`: "If the dict already has a value for the given key then the value is
  replaced with the new value."
- `merge`: "If there are entries with the same keys in both dicts the entry from
  the second dict takes precedence."
- `combine`: "If there are entries with the same keys in both dicts the given
  function is used to determine the new value to use in the resulting dict."
- `upsert`: "If there was not an entry in the dict for the given key then the
  function gets None as its argument, otherwise it gets Some(value)."
- `size`: "This function runs in constant time and does not need to iterate the
  dict."
- `fold`: "Dicts are not ordered so the values are not returned in any specific
  order. Do not write code that relies on the order entries are used by this
  function as it may change in later versions of Gleam or Erlang."
- `each`: "The order of elements in the iteration is an implementation detail
  that should not be relied upon."
- `to_list`: "The ordering of elements in the resulting list is an
  implementation detail that should not be relied upon."

## Version notes
- Page version: `gleam_stdlib` v1.0.3 (from `<title>`).
- HexDocs ExDoc version: 1.16.0 (from CSS/JS asset query strings).
- Canonical URL redirects to `https://hexdocs.pm/gleam_stdlib/gleam/dict.html`.
- The HAMT backing comes from Erlang/OTP 18+ maps; the page itself does not name
  "HAMT" but defers to "the Erlang map module."
- `upsert` references `option.Option(v)`, tying this module to `gleam/option`.

## Discovered links

### Relevant (crawl later)
- https://gleam-stdlib.hexdocs.pm/gleam/option.html  (referenced by `upsert`)
- https://gleam-stdlib.hexdocs.pm/gleam/list.html    (List, used by from_list/keys/values/take/drop)
- https://gleam-stdlib.hexdocs.pm/gleam/result.html   (Result, returned by `get`)
- https://gleam-stdlib.hexdocs.pm/gleam/set.html     (sibling collection module)
- https://gleam-stdlib.hexdocs.pm/gleam/iterator.html (lazy collection, often paired with dict)
- https://gleam-stdlib.hexdocs.pm/gleam/string.html  (used in fold example)
- https://www.erlang.org/doc/man/maps.html            (Erlang map module, referenced by page)

### Skipped
- Sidebar module links not directly relevant to dict semantics: bit_array,
  bool, float, int, io, pair, regex, string_builder, string_tree, uri.
- HexDocs infrastructure links (search, sidebar nav, theme toggle, plausible
  analytics, pride toggle script).
