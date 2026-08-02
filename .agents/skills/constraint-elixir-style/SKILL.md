---
name: constraint-elixir-style
description: |
  Enforces Elixir style and typespec conventions during code execution — mix
  format, naming conventions, @impl true, @spec/@type, and @moduledoc/@doc. Load
  when writing or reviewing Elixir code. Does NOT cover OTP callback discipline
  (see constraint-elixir-otp-api) or runtime debugging.
metadata:
  org.kind: constraint
---

# Constraint: Elixir Style and Typespecs

This constraint enforces Elixir naming conventions, formatting, behaviour-
implementation annotations, and typespec coverage. Violations either fail to
compile, produce Credo/Dialyzer warnings, or hide contracts from documentation
and analysis.

## Triggers

Load this skill when:

- Writing or reviewing Elixir modules, functions, guards, or typespecs.
- Implementing a behaviour callback (`@impl true`).
- Adding `@spec`/`@type`/`@typedoc`/`@moduledoc`/`@doc`.
- Running `mix format`, `mix credo`, or `mix dialyzer`.

## Rules

1. `snake_case` for variables/functions/attributes/filenames/atoms; `CamelCase`
   for modules (acronyms keep capitals: `ExUnit.CaptureIO`). Module-to-file path
   mirrors the dotted name (`MyApp.Foo.Bar` -> `lib/my_app/foo/bar.ex`); tests are
   `*_test.exs`.
2. `?` suffix = boolean predicate (MUST return a boolean); `is_` prefix = guard-
   safe check; never combine (`is_foo?`).
3. `!` suffix = raising variant of a `{:ok,_}`/`{:error,_}` or `nil`-returning
   function; prefer the non-bang variant + pattern match when handling outcomes.
4. `size` = O(1) (`map_size/1`, `tuple_size/1`); `length` = O(n) (`length/1`,
   `String.length/1`). Name new functions accordingly.
5. `@impl true` (or `@impl Module`) on EVERY behaviour callback implementation —
   it catches arity/name typos at compile time.
6. Every public function SHOULD have an accurate `@spec` placed immediately above
   the `def`/`defp`/`defmacro`; define `@type t` for a module's primary data type.
7. NEVER use `string()` (it is the Erlang charlist) — use `String.t()` for UTF-8
   text, `binary()` for raw binaries, `charlist()`/`nonempty_charlist()` for
   charlists.
8. Reserve `%{}` for the empty-map singleton; use `map()` or
   `%{optional(any) => any}` for "any map".
9. `no_return()` only for functions that never return (infinite loops,
   always-raise); never for side-effect functions that return `:ok`.
10. `@moduledoc`/`@doc` on public modules/functions; `@typedoc` immediately
    precedes the `@type`/`@typep`/`@opaque` it documents.
11. Prefer functions over module-attribute constants (`defp hours_in_a_day, do:
    24` over `@hours_in_a_day 24`); no `ALL_CAPS` constants.

## References

- Operational skills: `elixir-coding`, `elixir-static-analysis`.
- Docs: `docs/elixir/naming-conventions.md`,
  `docs/elixir/typespecs-and-dialyzer.md`.

## Out of scope

- GenServer/Supervisor callback discipline — see `constraint-elixir-otp-api`.
- Credo/Dialyzer tool configuration — see `elixir-static-analysis`.
- Runtime debugging — see `beam-observability-debugging`.

## Violation examples

### `is_foo?` (combines both conventions)

```elixir
# FORBIDDEN: never combine is_ prefix with ? suffix
def is_empty?(list), do: list == []
```

Correct: `def empty?(list)` (non-guard predicate) or `def is_empty(list)`
(guard-safe).

### `string()` typespec

```elixir
# FORBIDDEN: string() is the Erlang charlist, not an Elixir string
@spec greet(String.t()) :: string()
```

Correct: `@spec greet(String.t()) :: String.t()`.

### Missing `@impl true` on a callback

```elixir
# FORBIDDEN: typo in callback name/arity fails silently without @impl
def handle_call(req, from, state), do: ...
```

Correct: `@impl true` above the callback so a name/arity mismatch is a compile
error.

### `no_return()` for a side-effect function

```elixir
# FORBIDDEN: IO.puts/1 returns :ok
@spec print(String.t()) :: no_return()
```

Correct: `@spec print(String.t()) :: :ok`.

## How to check

```bash
mix format --check-formatted          # formatting gate
mix compile --warnings-as-errors     # warnings as errors
mix credo --strict                    # naming/consistency lints
mix dialyzer                          # typespec / success-typing checks
```

Manual review:

- `snake_case`/`CamelCase` correct; no `is_foo?`; `?` returns boolean; `!` raises
  on failure.
- `@impl true` on every callback; `@spec` above every public `def`.
- No `string()`; `%{}` only for empty map; `no_return()` only for never-returning
  functions.
- `@moduledoc`/`@doc`/`@typedoc` present and correctly placed.
