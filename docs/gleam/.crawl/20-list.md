# Crawl: gleam/list.html
- seed_url: https://gleam-stdlib.hexdocs.pm/gleam/list.html
- canonical_url: https://gleam-stdlib.hexdocs.pm/gleam/list.html
- family: Gleam stdlib module
- fetch: 200
- gleam_stdlib_version: v1.0.3
- feeds_docs: stdlib.md, language-fundamentals.md

## Purpose
`gleam/list` is the stdlib module for the `List(a)` type — Gleam's singly-linked,
immutable, persistent cons-list. It provides traversal, transformation, filtering,
searching, slicing, grouping, zipping, sorting, and combinatorics over lists, plus
the `ContinueOrStop(a)` control-flow type used by `fold_until`.

Gleam lists are NOT arrays. They are head::tail cons cells. The module is the
canonical reference for every list operation in the language; the dedicated
`[head, ..tail]` syntax is the literal/prepend form.

## Function groups (categories + key signatures)

### Construction / shape
- `new() -> List(a)` — empty list.
- `wrap(item: a) -> List(a)` — single-element list.
- `repeat(item a: a, times times: Int) -> List(a)` — `repeat("a", times: 5)` → `["a","a","a","a","a"]`.
- `prepend(to list: List(a), this item: a) -> List(a)` — O(1); equivalent to `[item, ..list]`.
- `append(first: List(a), second: List(a)) -> List(a)` — O(n), copies/traverses `first`.
- `reverse(list: List(a)) -> List(a)` — O(n), VM-native, highly optimised.
- `flatten(lists: List(List(a))) -> List(a)`.
- `interleave(list: List(List(a))) -> List(a)`.

### Traversal / mapping
- `each(list: List(a), f: fn(a) -> b) -> Nil` — side-effect walk, discards return.
- `map(list: List(a), with fun: fn(a) -> b) -> List(b)`.
- `index_map(list: List(a), with fun: fn(a, Int) -> b) -> List(b)`.
- `flat_map(over list: List(a), with fun: fn(a) -> List(b)) -> List(b)`.
- `scan(over list: List(a), from initial: acc, with fun: fn(acc, a) -> acc) -> List(acc)` — running fold.

### Folding / reduction
- `fold(over list: List(a), from initial: acc, with fun: fn(acc, a) -> acc) -> acc` — left-to-right, tail-recursive, linear.
- `fold_right(over list: List(a), from initial: acc, with fun: fn(acc, a) -> acc) -> acc` — right-to-left, NOT tail-recursive; prefer `fold`.
- `fold_until(over list: List(a), from initial: acc, with fun: fn(acc, a) -> ContinueOrStop(acc)) -> acc` — early-exit fold.
- `index_fold(over list: List(a), from initial: acc, with fun: fn(acc, a, Int) -> acc) -> acc`.
- `reduce(over list: List(a), with fun: fn(a, a) -> a) -> Result(a, Nil)` — fold with no seed; `Error(Nil)` on empty.
- `map_fold(over list: List(a), from initial: acc, with fun: fn(acc, a) -> #(acc, b)) -> #(acc, List(b))` — fold + map in one pass.
- `try_fold(over list: List(a), from initial: acc, with fun: fn(acc, a) -> Result(acc, e)) -> Result(acc, e)` — short-circuits on `Error`.
- `try_each(over list: List(a), with fun: fn(a) -> Result(b, e)) -> Result(Nil, e)`.
- `try_map(over list: List(a), with fun: fn(a) -> Result(b, e)) -> Result(List(b), e)`.

### Filtering / partitioning
- `filter(list: List(a), keeping predicate: fn(a) -> Bool) -> List(a)`.
- `filter_map(list: List(a), with fun: fn(a) -> Result(b, e)) -> List(b)` — keep `Ok`s (drops `Error`s, unlike `try_map`).
- `partition(list: List(a), with categorise: fn(a) -> Bool) -> #(List(a), List(a))`.
- `drop_while(in list: List(a), satisfying predicate: fn(a) -> Bool) -> List(a)`.
- `take_while(in list: List(a), satisfying predicate: fn(a) -> Bool) -> List(a)`.
- `unique(list: List(a)) -> List(a)`.

### Searching / membership
- `find(in list: List(a), one_that is_desired: fn(a) -> Bool) -> Result(a, Nil)`.
- `find_map(in list: List(a), with fun: fn(a) -> Result(b, c)) -> Result(b, Nil)`.
- `contains(list: List(a), any elem: a) -> Bool`.
- `count(list: List(a), where predicate: fn(a) -> Bool) -> Int`.
- `all(in list: List(a), satisfying predicate: fn(a) -> Bool) -> Bool` — short-circuits on `False`.
- `any(in list: List(a), satisfying predicate: fn(a) -> Bool) -> Bool` — short-circuits on `True`.

### Head/tail / ends
- `first(list: List(a)) -> Result(a, Nil)` — `Error(Nil)` on empty.
- `rest(list: List(a)) -> Result(List(a), Nil)` — O(1), no copy.
- `last(list: List(a)) -> Result(a, Nil)` — O(n).
- `is_empty(list: List(a)) -> Bool`.
- `length(of list: List(a)) -> Int` — O(n), VM-native, highly optimised.

### Taking / dropping / splitting
- `take(from list: List(a), up_to n: Int) -> List(a)` — O(n); returns full list if `n > length`.
- `drop(from list: List(a), up_to n: Int) -> List(a)` — O(n) but does NOT copy.
- `split(list list: List(a), at index: Int) -> #(List(a), List(a))` — before/after index.
- `split_while(list list: List(a), satisfying predicate: fn(a) -> Bool) -> #(List(a), List(a))`.
- `sample(from list: List(a), up_to n: Int) -> List(a)`.

### Grouping / chunking / windows
- `group(list: List(v), by key: fn(v) -> k) -> dict.Dict(k, List(v))` — does NOT preserve input order.
- `chunk(in list: List(a), by f: fn(a) -> k) -> List(List(a))` — group adjacent equal-key runs.
- `sized_chunk(in list: List(a), into count: Int) -> List(List(a))` — fixed-size chunks; `count < 1` treated as `1`.
- `window(list: List(a), by n: Int) -> List(List(a))` — sliding windows.
- `transpose(list_of_lists: List(List(a))) -> List(List(a))`.

### Zipping / unzipping
- `zip(list: List(a), with other: List(b)) -> List(#(a, b))` — extra elements of longer list dropped.
- `strict_zip(list: List(a), with other: List(b)) -> Result(List(#(a, b)), Nil)` — `Error(Nil)` if lengths differ.
- `unzip(input: List(#(a, b))) -> #(List(a), List(b))`.

### Sorting / extrema
- `sort(list: List(a), by compare: fn(a, a) -> order.Order) -> List(a)`.
- `max(over list: List(a), with compare: fn(a, a) -> order.Order) -> Result(a, Nil)`.

### Combinatorics
- `combinations(items: List(a), by n: Int) -> List(List(a))`.
- `combination_pairs(items: List(a)) -> List(#(a, a))`.
- `permutations(list: List(a)) -> List(List(a))`.
- `intersperse(list: List(a), with elem: a) -> List(a)`.
- `shuffle(list: List(a)) -> List(a)`.

### Keyword lists (list of 2-tuples)
- `key_find(in keyword_list: List(#(k, v)), find desired_key: k) -> Result(v, Nil)`.
- `key_filter(in keyword_list: List(#(k, v)), find desired_key: k) -> List(v)`.
- `key_pop(list: List(#(k, v)), key: k) -> Result(#(v, List(#(k, v))), Nil)`.
- `key_set(list: List(#(k, v)), key: k, value: v) -> List(#(k, v))`.

### Control-flow type
- `pub type ContinueOrStop(a) { Continue(a) Stop(a) }` — consumed by `fold_until`.

### NOTE: functions NOT in this module
The crawl brief mentioned `at`, `pop_map`, `range`, and `slice`. These do NOT
exist in `gleam/list` v1.0.3:
- `at` — use `first`/`rest`/`take`/`drop`/`split` instead; there is no index accessor (random access is O(n) by nature).
- `slice` — compose `take` + `drop`, or `split`.
- `range` — no integer-range builder in `list`; build via `gleam/iterator` or a fold.
- `pop_map` — closest analogue is `key_pop` (on keyword lists) or `find_map` (on values).

## Performance model (linked list: prepend O(1), access O(n))

Gleam `List(a)` is a **singly-linked cons list** (head + tail), immutable and
persistent. The performance model follows directly:

| Operation | Complexity | Notes |
|---|---|---|
| `prepend` / `[x, ..xs]` | **O(1)** | shares the tail; no copy. |
| `first`, `rest`, `is_empty` | **O(1)** | `rest` does not copy. |
| `wrap`, `new` | **O(1)** | |
| `length`, `reverse`, `append`, `take`, `drop`, `flatten`, `last`, `sort`, `max` | **O(n)** | `reverse`/`length` are VM-native and highly optimised. `append` copies/traverses `first` only. `drop` traverses but does NOT copy. |
| `map`, `each`, `filter`, `fold`, `find`, `count`, `all`, `any`, `unique` | **O(n)** | single pass; `all`/`any`/`find`/`contains` short-circuit. |
| `fold_right` | **O(n)** time, **O(n)** stack — NOT tail-recursive. Prefer `fold`. |
| `fold`, `fold_until`, `try_fold`, `map`, `filter`, `scan`, `index_map` | tail-recursive / accumulator-based | safe for large lists. |
| `group` | **O(n)** | returns a `dict.Dict`; does not preserve input order. |
| `chunk`, `sized_chunk`, `window`, `split`, `split_while` | **O(n)** | |
| `zip`, `strict_zip`, `unzip` | **O(min(n,m))** / **O(n)** | `zip` truncates to shorter; `strict_zip` errors on mismatch. |
| `combinations`, `permutations`, `combination_pairs` | **exponential / factorial** | avoid on large inputs. |
| `shuffle`, `sample` | **O(n)** | |

**Random access by index is O(n)** — there is no `list[i]`. To reach index `i`,
traverse from the head (`take`/`drop`/`split`/`first`+`rest` loop). For
index-heavy work, prefer `gleam/array` (not in this module) or build via an
iterator + fold.

**Tail-call / accumulator pattern:** `fold` is the canonical accumulator form and
is tail-recursive — it is the default choice. `fold_right` is NOT tail-recursive
(it builds stack proportional to length); the docs explicitly say "Where
possible use fold instead as it will use less memory." `fold_until` and
`try_fold` are also tail-recursive and short-circuit. When hand-rolling
recursion over a list, accumulate into an extra `acc` parameter and recurse on
the tail so the BEAM can apply last-call optimisation; if you build a list in
reverse, finish with `reverse` (VM-native, cheap).

## Strict rules / caveats
- Lists are immutable: every "mutating" operation returns a new list; the original is unchanged (structural sharing).
- `fold_right` is not tail-recursive — do not use it on unbounded/large lists; use `fold`.
- `group` does NOT preserve input element order within groups.
- `zip` silently truncates to the shorter list; use `strict_zip` if length mismatch must be an error.
- `sized_chunk` with `count < 1` is treated as `1` (no error).
- `take`/`drop` with `n > length` return the full list / empty list respectively (no error).
- `first`/`rest`/`last`/`find`/`reduce`/`max` return `Result(_, Nil)` to signal emptiness — `Nil` here is the "no value" marker, not a real element.
- `filter_map` drops `Error` values silently; `try_map` propagates the first `Error`. Choose by intent.
- `combinations`/`permutations` are exponential/factorial — never on large inputs.
- No index access (`at`/`slice`/`range` are absent); random access is O(n) by traversal.
- `reverse` and `length` are VM-native and highly optimised but still O(n).

## Verbatim quotes
- `fold`: "Reduces a list of elements into a single value by calling a given function on each element, going from left to right. fold([1, 2, 3], 0, add) is the equivalent of add(add(add(0, 1), 2), 3). This function runs in linear time."
- `fold_right`: "Reduces a list of elements into a single value by calling a given function on each element, going from right to left. ... This function runs in linear time. Unlike fold this function is not tail recursive. Where possible use fold instead as it will use less memory."
- `fold_until`: "A variant of fold that allows to stop folding earlier. The folding function should return ContinueOrStop(accumulator). If the returned value is Continue(accumulator) fold_until will try the next value in the list. If the returned value is Stop(accumulator) fold_until will stop and return that accumulator."
- `reverse`: "Creates a new list from a given list containing the same elements but in the opposite order. This function has to traverse the list to create the new reversed list, so it runs in linear time. This function is natively implemented by the virtual machine and is highly optimised."
- `prepend`: "Prefixes an item to a list. This can also be done using the dedicated syntax instead. ... assert [1, ..existing_list] == [1, 2, 3, 4]"
- `append`: "Joins one list onto the end of another. This function runs in linear time, and it traverses and copies the first list."
- `rest`: "Returns the list minus the first element. If the list is empty, Error(Nil) is returned. This function runs in constant time and does not make a copy of the list."
- `length`: "Counts the number of elements in a given list. This function has to traverse the list to determine the number of elements, so it runs in linear time. This function is natively implemented by the virtual machine and is highly optimised."
- `drop`: "Returns a list that is the given list with up to the given number of elements removed from the front of the list. ... This function runs in linear time but does not copy the list."
- `take`: "Returns a list containing the first given number of elements from the given list. If the list has less than the number of elements then the full list is returned. This function runs in linear time."
- `group`: "Groups the elements from the given list by the given key function. Does not preserve the initial value order."
- `zip`: "Takes two lists and returns a single list of 2-element tuples. If one of the lists is longer than the other, the remaining elements from the longer list are not used."
- `strict_zip`: "Takes two lists and returns a single list of 2-element tuples. If one of the lists is longer than the other, an Error is returned."
- `each`: "Calls a function for each element in a list, discarding the return value. Useful for calling a side effect for every item of a list."
- `sized_chunk`: "Returns a list of chunks containing count elements each. If the last chunk does not have count elements, it is instead a partial chunk, with less than count elements. For any count less than 1 this function behaves as if it was set to 1."

## Version notes
- Page reports `gleam_stdlib · v1.0.3`.
- 64 public members parsed (1 type `ContinueOrStop` + 63 functions).
- `ContinueOrStop(a)` is the only custom type defined here; it exists solely to drive `fold_until`'s early-exit semantics.
- `sort`/`max` depend on `gleam/order` (`order.Order`).
- `group` returns `dict.Dict` from `gleam/dict`.
- `find`/`first`/`rest`/`last`/`reduce`/`max`/`strict_zip` use `Result(_, Nil)` as the absent-value signal (Gleam has no `null`/`undefined`); `gleam/option` (`Option(a)`) is the typed alternative used elsewhere in the stdlib.
- Source: `src/gleam/list.gleam` in `gleam-lang/stdlib` tag `v1.0.3`.

## Discovered links

### Relevant (crawl later)
- `../gleam/dict.html` — `group` returns `dict.Dict`; needed to document grouping output type.
- `../gleam/order.html` — `sort`/`max` take `fn(a, a) -> order.Order`; needed to document ordering contract.
- `../gleam/result.html` — `Result` is the return type of `find`/`first`/`rest`/`last`/`try_fold`/`strict_zip` etc.
- `../gleam/option.html` — `Option(a)` typed alternative to `Result(_, Nil)`; comparison relevant for absence modelling.
- `../gleam/pair.html` — 2-tuple helpers; `zip`/`unzip`/`combination_pairs` produce `#(a, b)`.
- `../gleam/function.html` — function-composition utilities often paired with `map`/`filter`.

### Skipped
- `../gleam/bit_array.html` — unrelated binary type.
- `../gleam/bool.html` — trivial boolean helpers.
- `../gleam/bytes_tree.html` — binary builder, not list-related.
- `../gleam/dynamic.html` — runtime dynamic typing.
- `../gleam/dynamic/decode.html` — dynamic decoders.
- `../gleam/float.html` — float arithmetic.
- `../gleam/int.html` — int arithmetic (used in examples but not core to list API).
- `../gleam/io.html` — I/O (used in `each` example only).
- `../gleam/set.html` — set type.
- `../gleam/string.html` — string module.
- `../gleam/string_tree.html` — string builder.
- `../gleam/uri.html` — URI handling.
- `../gleam/list.html` — self.
- GitHub source links (`github.com/gleam-lang/stdlib/blob/v1.0.3/...`) — source, not docs.
