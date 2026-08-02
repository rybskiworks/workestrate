---
name: elixir-coding
description: |
  Operational guide for writing idiomatic Elixir code — modules, functions, pattern
  matching, guards, types, structs, protocols, comprehensions, sigils, and core
  collection APIs. Load when writing or reviewing new Elixir code. Does NOT cover
  OTP/concurrency (see elixir-otp), testing (see elixir-testing), or Ecto/Phoenix.
---

# Elixir Coding Conventions and Core APIs

## Triggers

Load this skill when:

- Writing or reviewing Elixir modules, functions, pattern matching, guards, or
  control flow.
- Defining structs, protocols, comprehensions, or sigils.
- Using `Enum`/`Stream`/`Map`/`Keyword`/`List`/`String` core APIs.
- Making naming or typespec decisions for new code.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/language-fundamentals.md`
  - https://hexdocs.pm/elixir/basic-types.html
  - https://hexdocs.pm/elixir/lists-and-tuples.html
  - https://hexdocs.pm/elixir/binaries-strings-and-charlists.html
  - https://hexdocs.pm/elixir/keywords-and-maps.html
  - https://hexdocs.pm/elixir/structs.html
  - https://hexdocs.pm/elixir/sigils.html
  - https://hexdocs.pm/elixir/operators.html
  - https://hexdocs.pm/elixir/patterns-and-guards.html
  - https://hexdocs.pm/elixir/case-cond-and-if.html
  - https://hexdocs.pm/elixir/modules-and-functions.html
  - https://hexdocs.pm/elixir/alias-require-and-import.html
  - https://hexdocs.pm/elixir/module-attributes.html
  - https://hexdocs.pm/elixir/protocols.html
  - https://hexdocs.pm/elixir/enumerable-and-streams.html
  - https://hexdocs.pm/elixir/comprehensions.html
  - https://hexdocs.pm/elixir/typespecs.html
- `docs/elixir/core-modules.md`
  - https://hexdocs.pm/elixir/Enum.html
  - https://hexdocs.pm/elixir/Stream.html
  - https://hexdocs.pm/elixir/Map.html
  - https://hexdocs.pm/elixir/Keyword.html
  - https://hexdocs.pm/elixir/List.html
  - https://hexdocs.pm/elixir/String.html
  - https://hexdocs.pm/elixir/Enumerable.html
  - https://hexdocs.pm/elixir/Collectable.html
  - https://hexdocs.pm/elixir/Access.html
  - https://hexdocs.pm/elixir/Kernel.SpecialForms.html
- `docs/elixir/naming-conventions.md`
  - https://hexdocs.pm/elixir/naming-conventions.html
  - https://hexdocs.pm/elixir/modules-and-functions.html
  - https://hexdocs.pm/elixir/typespecs.html
  - https://hexdocs.pm/elixir/library-guidelines.html

## Key Rules

- `snake_case` for variables/functions/attributes/filenames/atoms; `CamelCase`
  for modules (acronyms keep capitals: `ExUnit.CaptureIO`).
- `?` suffix = boolean predicate (must return boolean); `is_` prefix = guard-safe
  check; never combine (`is_foo?`).
- `!` suffix = raising variant of a `{:ok,_}`/`{:error,_}` or `nil`-returning
  function; prefer non-bang + pattern match when handling outcomes.
- `size` = O(1) (`map_size/1`, `tuple_size/1`, `byte_size/1`); `length` = O(n)
  (`length/1`, `String.length/1`).
- `get` returns default/nil; `fetch` returns `{:ok,_}|:error`; `fetch!` raises.
- `compare/2` returns `:lt`/`:eq`/`:gt`.
- Module-to-file path mirrors dotted name (`MyApp.Foo.Bar` ->
  `lib/my_app/foo/bar.ex`); tests `*_test.exs`.
- Pattern matching is the primary control-flow mechanism; `=` is match not
  assignment; pin `^` to compare an existing binding; map patterns do subset
  match; `%{}` matches any map.
- Guards are strictly boolean (no truthy/falsy); errors in guards fail the
  clause silently; use `and`/`or`/`not` (not `&&`/`||`/`!`); `defguard`/
  `defguardp` for reusable guards.
- Truthy/falsy: only `false` and `nil` are falsy (`0`, `""`, `[]` are truthy).
- Prefer `with/1` over nested `case`; nested `case` is a smell.
- Enum vs Stream: Enum eager (default for small/finite/single-pass); Stream lazy
  (large/infinite/multi-stage pipelines/IO). Stream is not universally faster —
  per-element overhead.
- Map: `%{map | k => v}` updates existing keys (raises `KeyError` if absent);
  `Map.put/3` inserts new keys; `map.key` (atom keys, raises if missing) vs
  `map[key]` (Access, nil-safe).
- Keyword lists: ordered, atom keys, may repeat; do NOT pattern match on them
  (order/count not fixed).
- Structs: `defstruct`, `@enforce_keys`, `struct!/2` raises on invalid keys;
  pattern match with `%ModuleName{}`.
- Protocols: `defprotocol`/`defimpl`, `@derive`, built-ins (`Enumerable`,
  `Collectable`, `Inspect`, `String.Chars`).
- Comprehensions: `for/1` with generators, filters, `:into`, `:uniq`, `:reduce`.
- Typespecs: use `String.t()` not `string()` (`string()` = Erlang charlist);
  define `@type t` for the module's primary type.
- Prefer functions over module-attribute constants (`defp hours_in_a_day, do: 24`
  over `@hours_in_a_day 24`).
- Unused vars -> `_` or `_foo`.

## Quick Commands

```bash
mix compile --warnings-as-errors      # treat warnings as errors
mix format --check-formatted          # verify formatting
mix format                             # auto-format
```

## Anti-patterns

- "Calculator GenServer" / wrapping pure functions behind a process for code
  organization.
- `is_foo?` naming (combines both conventions).
- `!` function that silently returns instead of raising.
- O(n) function named `...size` or O(1) named `...length`.
- `string()` typespec (use `String.t()`).
- Pattern matching on keyword lists.
- `not x in list` (deprecated parse) — write `x not in list`.
- `ALL_CAPS` module-attribute constants.
- `__FILE__` (does not exist; use `__ENV__.file`).

## Related Skills

- elixir-otp
- elixir-testing
- elixir-static-analysis
- elixir-error-handling
