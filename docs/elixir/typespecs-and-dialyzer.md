# Typespecs and Dialyzer

## Purpose

Elixir typespecs are a lightweight, Erlang-derived notation for declaring the expected shapes of data and function contracts. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "Elixir comes with a notation for declaring types and specifications. Elixir is a dynamically typed language, and as such, type specifications are never used by the compiler to optimize or modify code. Still, using type specifications is useful because they provide documentation and are used by tools such as Dialyzer."

Agents writing or reviewing Elixir code MUST use typespecs to make contracts explicit, to help ExDoc produce useful reference material, and to let Dialyzer catch type inconsistencies. This document covers the typespec syntax and attributes that are authoritative today, plus a full section on Dialyxir and Dialyzer.

## Sources used

- https://hexdocs.pm/elixir/typespecs.html (PRIMARY — typespecs)
- https://hexdocs.pm/elixir/typespecs.html "Built-in types" section
- https://hexdocs.pm/elixir/typespecs.html "Literals" section
- https://hexdocs.pm/elixir/typespecs.html "Maps" section
- https://hexdocs.pm/elixir/typespecs.html "Behaviours" section
- https://hexdocs.pm/elixir/typespecs.html "Pitfalls — The string() type" section
- https://hexdocs.pm/elixir/1.20/typespecs.html (version-specific)
- https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html (Dialyxir task)
- https://github.com/jeremyjh/dialyxir (README)

This page reflects Elixir v1.20.2 docs and Dialyxir v1.4.7.

## Typespecs

### What typespecs are and what consumes them

Typespecs are annotations, not a runtime type system. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "Elixir is a dynamically typed language, and as such, type specifications are never used by the compiler to optimize or modify code. Still, using type specifications is useful because they provide documentation and are used by tools such as Dialyzer."

Two primary consumers matter for agents:

1. **ExDoc** — renders `@spec`, `@type`, `@callback`, and `@typedoc` entries in generated HTML reference docs.
2. **Dialyzer** — via the `dialyxir` Mix task, performs success-typing analysis using specs to find inconsistencies, unreachable code, and incorrect returns.

Elixir is actively developing a separate set-theoretic type system. Typespecs may be phased out or superseded by that system in the long term, but they remain the authoritative contract notation now. Agents MUST write typespecs for public APIs today and SHOULD be ready to migrate if the language introduces a new type syntax later.

### Module attributes for typespecs

| Attribute | Meaning |
|---|---|
| `@type` | Public user-defined type. Exported and shown in docs. |
| `@typep` | Private user-defined type. Not exported and not shown in docs. |
| `@opaque` | Public type whose internal structure is hidden from consumers. |
| `@spec` | Function or macro contract (arguments and return type). |
| `@callback` | Behaviour callback contract for a function. |
| `@macrocallback` | Behaviour callback contract for a macro. |
| `@typedoc` | Documentation string for the type defined on the next line. |
| `@impl` | Declares that the following function implements a behaviour callback. |

Example from [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

```elixir
defmodule StringHelpers do
  @typedoc "A word separated by whitespace."
  @type word() :: String.t()

  @spec long_word?(word()) :: boolean()
  def long_word?(word) do
    String.length(word) > 8
  end
end
```

Agents SHOULD place `@spec` immediately above the corresponding `def`/`defp`/`defmacro`. Types (`@type`, `@typep`, `@opaque`) SHOULD live near the top of the module or near the functions that consume them.

### `@spec` syntax

The basic form is:

```elixir
@spec name(type1, type2) :: return_type
```

Arguments may be named for documentation:

```elixir
@spec days_since_epoch(year :: integer(), month :: integer(), day :: integer()) :: integer()
```

Zero-arity specs use empty parentheses:

```elixir
@spec now() :: DateTime.t()
```

Multiple specs may be declared for the same name/arity:

```elixir
@spec add(integer(), integer()) :: integer()
@spec add(float(), float()) :: float()
def add(a, b), do: a + b
```

Type variables and constraints use `when var: constraint`. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "This guard notation only works with `@spec`, `@callback`, and `@macrocallback`."

```elixir
@spec function(arg) :: [arg] when arg: atom

@spec function(arg1, arg2) :: {arg1, arg2} when arg1: atom, arg2: integer

@spec identity(value) :: value when value: var
```

`when value: var` declares an unrestricted type variable. Place `@spec` above `defmacro` the same way as for `def`:

```elixir
@spec my_macro(expr :: Macro.t()) :: Macro.t()
defmacro my_macro(expr) do
  quote do: unquote(expr)
end
```

### `@type`, `@typep`, and `@opaque`

| Attribute | Visibility | Use when |
|---|---|---|
| `@type` | Public, exported | The shape is part of the module's public API. |
| `@typep` | Private to the module | The shape is only used internally. |
| `@opaque` | Public name, hidden structure | Consumers may refer to the type but must not pattern-match on its internals. |

```elixir
defmodule Queue do
  @opaque t() :: {list(), list()}

  @spec new() :: t()
  def new(), do: {[], []}

  @spec push(t(), any()) :: t()
  def push({in_q, out_q}, item), do: {[item | in_q], out_q}
end
```

Parameterized types are defined with parentheses:

```elixir
@type dict(key, value) :: [{key, value}]

@type container(a) when a: var :: %{value: a}
```

By convention, a single-parameter type for a data structure is named `t` (e.g. `MapSet.t()`), and the parameter is often named inside the parentheses: `@type t(a) when a: var :: list(a)`.

### `@typedoc`

`@typedoc` MUST immediately precede the `@type`, `@typep`, or `@opaque` it documents. Use `@typedoc false` to hide a type from ExDoc while still making it available to Dialyzer (analogous to `@doc false`).

```elixir
@typedoc "A validated email address."
@type email() :: String.t()

@typedoc false
@typep internal_id() :: integer()
```

### Basic types

| Type | Meaning |
|---|---|
| `any()` | Top type: any Elixir term. |
| `none()` | Bottom type: no term. |
| `atom()` | Any atom. |
| `map()` | Any map. |
| `pid()` | A process identifier. |
| `port()` | A port identifier. |
| `reference()` | A reference. |
| `tuple()` | A tuple of any size. |
| `float()` | A floating-point number. |
| `integer()` | An integer. |
| `neg_integer()` | A negative integer (`... -2, -1`). |
| `non_neg_integer()` | Zero or a positive integer (`0, 1, 2, ...`). |
| `pos_integer()` | A positive integer (`1, 2, ...`). |
| `list(type)` | A list whose elements are all `type`. |
| `nonempty_list(type)` | A non-empty list of `type`. |
| `maybe_improper_list(content, termination)` | A list whose final tail may be `termination`, not necessarily `[]`. |
| `nonempty_improper_list(content, termination)` | A non-empty improper list. |
| `nonempty_maybe_improper_list(content, termination)` | A non-empty list that may be improper. |

Source: [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Basic types".

### Built-in types

From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Built-in types":

| Built-in type | Defined as |
|---|---|
| `term()` | `any()` |
| `any()` | top type (set of all terms) |
| `arity()` | `0..255` |
| `as_boolean(t)` | `t` (signals truthiness) |
| `binary()` | `<<_::_*8>>` |
| `nonempty_binary()` | `<<_::8, _::_*8>>` |
| `bitstring()` | `<<_::_*1>>` |
| `nonempty_bitstring()` | `<<_::1, _::_*1>>` |
| `boolean()` | `true \| false` |
| `byte()` | `0..255` |
| `char()` | `0..0x10FFFF` |
| `charlist()` | `[char()]` |
| `nonempty_charlist()` | `[char(), ...]` |
| `fun()` | `(... -> any)` |
| `function()` | `fun()` (alias) |
| `identifier()` | `pid() \| port() \| reference()` |
| `iodata()` | `iolist() \| binary()` |
| `iolist()` | `maybe_improper_list(byte() \| binary() \| iolist(), binary() \| [])` |
| `keyword()` | `[{atom(), any()}]` |
| `keyword(t)` | `[{atom(), t}]` |
| `list()` | `[any()]` |
| `nonempty_list()` | `nonempty_list(any())` |
| `maybe_improper_list()` | `maybe_improper_list(any(), any())` |
| `nonempty_maybe_improper_list()` | `nonempty_maybe_improper_list(any(), any())` |
| `mfa()` | `{module(), atom(), arity()}` |
| `module()` | `atom()` |
| `no_return()` | `none()` |
| `node()` | `atom()` |
| `number()` | `integer() \| float()` |
| `struct()` | `%{:__struct__ => atom(), optional(atom()) => any()}` |
| `timeout()` | `:infinity \| non_neg_integer()` |

### Literal types

From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Literals":

| Literal | Type meaning |
|---|---|
| `:atom` | The atom `:atom`. |
| `true`, `false`, `nil` | The corresponding atoms. |
| `<<>>` | Empty bitstring. |
| `<<_::size>>` | Bitstring with `size` bits. |
| `<<_::_*unit>>` | Bitstring with a number of `unit`-bit segments. |
| `<<_::size, _::_*unit>>` | Bitstring with a fixed `size` plus `unit`-bit segments. |
| `(-> type)` | Zero-arity function returning `type`. |
| `(t1, t2 -> type)` | Function with arity 2 returning `type`. |
| `(... -> type)` | Function of any arity returning `type`. |
| `1` | The integer `1`. |
| `1..10` | Integers from `1` to `10` inclusive. |
| `[type]` | List of zero or more `type`. |
| `[]` | Empty list. |
| `[...]` | `nonempty_list(any())`. |
| `[type, ...]` | `nonempty_list(type)`. |
| `[key: value_type]` | Keyword list where `:key` is optional and maps to `value_type`. |
| `%{}` | Singleton type for the empty map only. |
| `%{key: value_type}` | Map with required atom key `:key`. |
| `%{key_type => value_type}` | Map with required key/value types. |
| `%{required(key_type) => value_type}` | Explicit required-key form. |
| `%{optional(key_type) => value_type}` | Key may be absent. |
| `%SomeStruct{}` | A struct of type `SomeStruct`. |
| `%SomeStruct{key: value_type}` | A struct with a required key. |
| `{}` | Empty tuple. |
| `{:ok, type}` | Tuple of arity 2 with the atom `:ok` and a `type`. |

### Union types

The pipe operator `|` creates a union:

```elixir
@type result :: {:ok, term()} | {:error, atom()}

@type my_union :: integer() | atom() | pid() | tuple()
```

Named labels in tuple elements improve documentation but do not change the type:

```elixir
@type color :: {red :: integer(), green :: integer(), blue :: integer()}
```

### Function types

Function literals in specs:

```elixir
@spec apply_fun((integer() -> boolean()), integer()) :: boolean()
def apply_fun(fun, value), do: fun.(value)

@spec compose((b -> c), (a -> b)) :: (a -> c)
def compose(g, f), do: fn x -> g.(f.(x)) end

@spec any_arity_handler((... -> any())) :: any()
def any_arity_handler(fun), do: fun.(1, 2, 3)
```

`fun()` and `function()` both mean `(... -> any())`. Prefer the more precise literal form when the arity and argument types are known.

### Map types

Maps support required and optional keys. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Maps":

```elixir
@type user_required :: %{name: String.t(), age: integer()}

@type user_required_explicit :: %{required(:id) => integer()}

@type user_optional :: %{optional(:email) => String.t()}

@type string_keyed :: %{String.t() => integer()}
```

Critical rules, quoted from [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "The key types in maps are allowed to overlap, and if they do, the leftmost key takes precedence. A map value does not belong to this type if it contains a key that is not in the allowed map keys."

> "For this reason, it is common to end a map type with `optional(any) => any` to signal that a map can have any number of keys besides the keys explicitly required by the type."

> "Note that the syntactic representation of `map()` is `%{optional(any) => any}`, not `%{}`. The notation `%{}` specifies the singleton type for the empty map."

Structs are maps with a `:__struct__` key:

```elixir
@type user_struct :: %User{}

@type user_named :: %User{name: String.t()}
```

Agents MUST use `map()` or `%{optional(any) => any}` for "any map", and MUST reserve `%{}` for the empty-map singleton.

### Tuples and keyword lists

Fixed-size tuples:

```elixir
@type ok_result :: {:ok, value :: term()}

@type pair :: {atom(), integer()}

@type empty :: {}
```

`tuple()` matches tuples of any size. For keyword lists, prefer explicit unions when the allowed keys are known. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "This makes it clear that only these options are allowed, none are required, and order does not matter."

```elixir
@type option ::
        {:name, String.t()}
        | {:max, pos_integer()}
        | {:min, pos_integer()}

@type options :: [option()]

@type server_options :: [GenServer.option() | option()]
```

Shorthand keyword-list syntax `[key: value_type]` is acceptable when any such key is optional and order does not matter.

### Remote types

From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "Any module is also able to define its own types, which can be accessed by other modules. For example, a range is `Range.t/0`, a string is `String.t/0`, and so on."

```elixir
@spec greet(String.t()) :: String.t()

@type range :: Range.t()

@spec sum(Enum.t()) :: number()

@type int_set :: MapSet.t(integer())

@spec start(GenServer.on_start()) :: pid()
```

Agents SHOULD prefer remote types over raw `binary()` when a module-specific type exists (e.g. `String.t()` for UTF-8 text).

### Behaviours: `@callback`, `@macrocallback`, `@impl`

A behaviour is a contract. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Behaviours":

```elixir
defmodule Parser do
  @callback parse(String.t()) :: {:ok, term()} | {:error, atom()}

  @callback extensions() :: [String.t()]
end
```

Implementations use `@behaviour` and `@impl`:

```elixir
defmodule JSONParser do
  @behaviour Parser

  @impl Parser
  def parse(str) do
    # ...
  end

  @impl Parser
  def extensions, do: [".json"]
end
```

`@impl Parser` is strongly recommended because it catches arity and name typos at compile time. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "Note that `@callback` and `@macrocallback` are replaced with the `@spec` and `@spec` syntax respectively, and they can also be used with guards and with multiple clauses."

Optional callbacks:

```elixir
defmodule Parser do
  @callback parse(String.t()) :: {:ok, term()} | {:error, atom()}
  @callback non_vital_fun() :: :ok

  @optional_callbacks non_vital_fun: 0
end
```

Inspect behaviour callbacks at runtime with `Parser.behaviour_info(:callbacks)` or in IEx with `b Parser`.

### `@doc false` / `@typedoc false`

Use `false` to hide a function, macro, callback, or type from ExDoc while keeping it available to the compiler and Dialyzer:

```elixir
@doc false
@spec internal_helper(term()) :: term()
defp internal_helper(x), do: x

@typedoc false
@typep hidden_id() :: integer()
```

### Pitfall: the `string()` type

This is the most common typespec mistake. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html) "Pitfalls — The string() type":

> "Elixir has a `string()` type, which is the Erlang `string`: a charlist. If you want to refer to the Elixir string, you must use `String.t/0` instead. You are likely to never or rarely use `string()`, and should use `charlist()` and `nonempty_charlist()` when you want to document a charlist. If you use `string()`, Elixir will emit a warning."

Agents MUST NOT use `string()`. Use:

- `String.t()` for UTF-8 binaries (human-readable strings).
- `binary()` or `nonempty_binary()` for raw binaries.
- `charlist()` or `nonempty_charlist()` for Erlang-style charlists.

`String.t()` and `binary()` are equivalent to Dialyzer; `String.t()` communicates intent to humans.

### `no_return()` correct usage

`no_return()` is `none()` and MUST be reserved for functions that can never return normally. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "For example, `no_return()` is used as the return type for functions that always throw an exception, such as `raise/2`. `no_return()` is also used by functions that loop infinitely and never return."

Correct:

```elixir
@spec loop_forever() :: no_return()
def loop_forever do
  receive do
    msg -> handle(msg)
  end

  loop_forever()
end

@spec always_raise(String.t()) :: no_return()
def always_raise(msg), do: raise(msg)
```

Incorrect:

```elixir
# WRONG: IO.puts/1 returns :ok
@spec print(String.t()) :: no_return()
def print(msg), do: IO.puts(msg)
```

Agents MUST NOT use `no_return()` for functions that merely perform side effects or that may raise under some inputs.

## Dialyxir and Dialyzer

### What Dialyzer and Dialyxir are

Dialyzer is a **DI**screpancy **A**na**LYZ**er for **ER**lang programs. From the [Erlang `dialyzer` man page](https://www.erlang.org/doc/man/dialyzer.html):

> "Dialyzer is a static analysis tool that identifies software discrepancies, such as definite type errors, code that has become dead or unreachable because of programming error, and unnecessary tests, in single Erlang modules or entire (sets of) applications."

Dialyzer bases its analysis on **success typings**, a conservative approximation that guarantees sound warnings without false positives. If Dialyzer reports a problem, the code really can fail; if it is silent, the code is type-safe under the success-typing model. The theoretical foundation is described in the success typings paper by Tobias Lindahl and Konstantinos Sagonas.

Dialyxir is the Elixir wrapper around Dialyzer. From the [Dialyxir README](https://hexdocs.pm/dialyxir/readme.html):

> "Mix tasks to simplify use of Dialyzer in Elixir projects."

Dialyxir adds PLT management, ignore-file filtering, Elixir-friendly formatting, and the `mix dialyzer.explain` task. It invokes Erlang's `:dialyzer` module through its API. From [Mix.Tasks.Dialyzer](https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html):

> "compiles the mix project, creates a PLT with dependencies if needed and runs dialyzer."

Running the task outside a Mix project "will build the core PLT files and exit." Agents SHOULD therefore always run `mix dialyzer` from a project directory that contains a `mix.exs`.

Dialyzer consumes the typespec attributes covered above in this document: `@spec`, `@type`, `@typep`, `@opaque`, `@callback`, and `@macrocallback`. Agents MUST keep those annotations accurate so Dialyzer's warnings are meaningful.

### Installation and first run

Add Dialyxir to `mix.exs`:

```elixir
defp deps do
  [
    {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}
  ]
end
```

`only: [:dev, :test]` keeps Dialyxir out of production releases. `runtime: false` prevents it from being loaded at runtime, which is important because Dialyxir is a build-time tool. Then fetch dependencies and run the task:

```bash
mix deps.get
mix dialyzer
```

The first run builds the core Erlang PLT, the core Elixir PLT, and the project PLT. This is slow; subsequent runs reuse the cached PLTs and are much faster. Agents MUST run `mix dialyzer --plt` once in CI or locally to warm the cache before gating on `mix dialyzer`.

### The `mix dialyzer` task and CLI flags

`mix dialyzer` compiles the project, checks the PLT, and runs the analysis. `mix dialyzer --plt` builds or updates the PLT only and exits 0.

From [Mix.Tasks.Dialyzer](https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html):

| Flag | Purpose |
|---|---|
| `--plt` | "only build the required PLT(s) and exit" |
| `--no-compile` | "do not compile even if needed" |
| `--no-check` | "do not perform (quick) check to see if PLT needs update" |
| `--force-check` | Force PLT check also when the lock file is unchanged; useful with local or path dependencies. |
| `--ignore-exit-status` | "display warnings but do not halt the VM or return an exit status code" |
| `--list-unused-filters` | "list unused ignore filters useful for CI. do not use with mix do." |
| `--format <name>` | Repeatable output formatter; default is `dialyxir`. See the formats subsection below. |
| `--quiet` | "suppress all informational messages" |
| `--quiet-with-result` | "suppress all informational messages except for the final result message" (added in Dialyxir 1.4.0). |

Any other `--flag` is forwarded to Dialyzer as a warning flag. Dialyxir strips the leading `-W` or `--`, atomizes the remainder, and passes it through. For example, `--unmatched_returns` becomes `-Wunmatched_returns`.

> **Historical removal:** `--halt-exit-status` was removed in Dialyxir 1.0.0-rc.7. The default behavior now halts the VM with a non-zero exit status when warnings are found. Use `--ignore-exit-status` when you want to display warnings without failing the command. Agents MUST NOT use `--halt-exit-status`.

### The PLT (Persistent Lookup Table)

The PLT caches the analysis of OTP, Elixir stdlib, and dependencies. From the [Dialyxir README](https://hexdocs.pm/dialyxir/readme.html):

> "The Persistent Lookup Table (PLT) is basically a cached output of the analysis. This is important because you'd probably stab yourself in the eye with a fork if you had to wait for Dialyzer to analyze all the standard library and OTP modules you are using every time you ran it."

Dialyxir maintains three PLT files:

1. **Core Erlang PLT** — stored at `$MIX_HOME/dialyxir_erlang-$OTP_VERSION.plt`.
2. **Core Elixir PLT** — stored at `$MIX_HOME/dialyxir_erlang-$OTP_VERSION-$ELIXIR_VERSION.plt`.
3. **Project PLT** — stored under `_build/$MIX_ENV/...` with a name that embeds the OTP and Elixir versions.

From the [Dialyxir README](https://hexdocs.pm/dialyxir/readme.html):

> "The core files are simply copied to your project folder when you run dialyxir for the first time with a given version of Erlang and Elixir."

> "By default, all the modules in the project PLT are checked against your dependencies to be sure they are up to date."

The default core PLT apps are:

```elixir
[:erts, :kernel, :stdlib, :crypto]
```

The PLT MUST be rebuilt when a new Erlang or Elixir version is introduced. To avoid unnecessary full rebuilds, Dialyxir stores a sidecar `.plt.hash` file that contains a SHA1 hash over the `mix.lock` content plus the resolved app list. If the hash matches the current state, the check short-circuits with "PLT is up to date!". `--force-check` re-checks the PLT even when the lock file is unchanged; `--no-check` skips the check entirely.

### Configuration keys (`dialyzer:` in `mix.exs project/0`)

Configure Dialyxir under the `:dialyzer` key returned from `project/0`. Accepted keys:

| Key | Purpose / accepted values |
|---|---|
| `:plt_add_deps` | Which OTP app deps to include: `:app_tree` (transitive runtime dep tree, the DEFAULT, like `mix app.tree`) or `:apps_direct` (only direct runtime deps). Legacy `:transitive` and `:project` are DEPRECATED since 1.0.0 — use `:app_tree` / `:apps_direct`. |
| `:plt_add_apps` | Extra OTP/apps to include beyond core apps plus deps, for example `:ex_unit` or `:wx`. |
| `:plt_ignore_apps` | Apps to ignore or exclude from core apps plus deps. |
| `:plt_apps` | List that REPLACES the default app set entirely. "Using this option will mean dependencies are not added automatically." |
| `:plt_local_path` | Directory for the project PLT (default `_build/$MIX_ENV/`). |
| `:plt_core_path` | Alternative to `MIX_HOME` for storing the Erlang and Elixir core PLTs. |
| `:plt_file` | DEPRECATED. Use `plt_local_path` / `plt_core_path` instead. In v0.4+ it produces a deprecation warning; silence with `plt_file: {:no_warn, "path"}`. The README notes it is "fine to use in CI." |
| `:ignore_warnings` | Path to the ignore file; defaults to `.dialyzer_ignore.exs`. |
| `:list_unused_filters` | Boolean (default `false`). Fail or report when ignore-file entries are unused. "When used without `--ignore-exit-status`, this option will result in an error status code." |
| `:flags` | Any Dialyzer command-line arguments or warning flags, e.g. `["-Wunmatched_returns", :error_handling, :underspecs]`. Accepts both `-Wwarning` strings and `WarnOpt` atoms. |
| `:remove_defaults` | List of default flags to drop. Only `:unknown` is on by default, so `remove_defaults: [:unknown]` disables it entirely. |
| `:no_umbrella` | Set `true` to treat a project whose lockfile is in a parent folder as a non-umbrella project. |
| `:paths` | List of locations to find BEAM files for analysis (default: the `_build` ebin for the current `MIX_ENV`). |

### Recommended minimal config

```elixir
defp dialyzer do
  [
    plt_local_path: "priv/plts",
    plt_core_path: "priv/plts",
    plt_add_apps: [:ex_unit],
    plt_add_deps: :app_tree,
    flags: [:unknown],
    ignore_warnings: ".dialyzer_ignore.exs",
    list_unused_filters: true
  ]
end
```

This stores PLTs in `priv/plts` for easy CI caching, includes `:ex_unit` for test helpers, tracks the full dependency tree, and fails on stale ignore-file entries.

### Warning categories and flags

From the [Dialyxir README](https://hexdocs.pm/dialyxir/readme.html):

> "As of 0.4, there are no longer any flags used by default except for `:unknown`."

The Dialyxir source confirms `@default_warnings [:unknown]`. Use `remove_defaults: [:unknown]` to disable the default `:unknown` category entirely.

The main `WarnOpt` flags, documented on the [Erlang `dialyzer` man page](https://www.erlang.org/doc/man/dialyzer.html), are:

| Flag | Meaning |
|---|---|
| `:unknown` / `-Wunknown` | Emit warnings about unknown functions and types. ON by default in Dialyxir; the recommended starting point. From the README: usually "a clue that the PLT is not complete and it may be best to leave it on." |
| `:error_handling` / `-Werror_handling` | "Include warnings for functions that only return by an exception." Off by default. |
| `:unmatched_returns` / `-Wunmatched_returns` | "Include warnings for function calls that ignore a structured return value or do not match against one of many possible return values. However, no warnings are included if the possible return values are a union of atoms or a union of numbers." Off by default. |
| `:underspecs` / `-Wunderspecs` | "Warn about underspecified functions (the specification is strictly more allowing than the success typing)." Off by default. |
| `:overspecs` / `-Woverspecs` | "Warn about overspecified functions." The Erlang docs mark this as "not recommended." Off by default. |
| `:specdiffs` / `-Wspecdiffs` | "Warn when the specification is different than the success typing." Also marked "not recommended." Off by default. |
| `:extra_return` / `:missing_return` | Report extra or missing return types compared to the spec. Off by default. |

Dialyzer also provides `no_*` suppressors such as `-Wno_return`, `-Wno_match`, `-Wno_unknown`, `-Wno_opaque`, `-Wno_unused`, and `-Wno_contracts`. These suppress specific warning classes without disabling the entire analysis.

> **Historical removal:** `:race_conditions` was removed in Dialyxir 1.3.0 because Erlang/OTP removed the underlying analysis (see erlang/otp PR #5502). Agents MUST NOT use `:race_conditions`.

### Common Dialyzer warnings catalog

Dialyxir emits the warning atoms below. Each atom can be passed to `mix dialyzer.explain` for a detailed description.

| Warning atom | Meaning / typical cause |
|---|---|
| `:no_return` | Function has no return (always raises, or no clause can ever match). Often cascades; fix the deepest call first. Sub-phrases include "has no local return", "only terminates with explicit exception", and "has no clauses that will ever match". |
| `:pattern_match` | "The pattern can never match the type {type}." A clause guard or pattern is unreachable given the input type. |
| `:guard_fail` | An impossible guard, or calls that will never satisfy the guard. |
| `:contract_subtype` | The `@spec` does not completely cover the types the function actually returns (spec too narrow). |
| `:contract_supertype` | The `@spec` is more general than what the function returns (loose, but not wrong). |
| `:contract_range` / `:contract_diff` / `:invalid_contract` | The contract conflicts with the success typing. |
| `:extra_range` | The `@spec` declares return types the function never returns. |
| `:missing_range` | Function returns a value outside the declared spec (only reported with `:overspecs`). |
| `:call` / `:apply` | A function call exists with correct arity but will not succeed because of a type mismatch in arguments. |
| `:call_to_missing` | Calls a missing or private function; often a typo or wrong arity (also a compiler warning). |
| `:unknown_function` | `{module}.{function}/{arity} does not exist.` Controlled by `:unknown`; usually means an incomplete PLT. Add the app via `plt_add_apps` or `plt_add_deps`. |
| `:unknown_type` | A spec references a missing `@type`. |
| `:call_with_opaque` / `:call_without_opaque` | Type mismatch involving an opaque type. |
| `:opaque_match` / `:opaque_equality` / `:opaque_guard` | Pattern-matching or comparing on the internals of an opaque type. |
| `:unused_fun` | Function recognized as used by the compiler but unreachable in the analysis. Usually resolves once a higher-up error is fixed. |
| `:unmatched_return` | A call ignores a structured or union return value it should match on (only with `:unmatched_returns`). |
| `:improper_list_construction` / `:binary_construction` / `:map_update` | Malformed list, binary, or map construction. |
| `:callback_*` family (`:callback_missing`, `:callback_type_mismatch`, `:callback_not_exported`, etc.) | Behaviour callback contract mismatches. |

This is a subset; the full registry contains roughly fifty warning atoms. Agents SHOULD run `mix dialyzer.explain` (with no arguments) to list all of them.

A few Erlang `-W` names map directly to Dialyxir atoms: `-Wno_match` suppresses `:pattern_match`, and `-Wno_return` suppresses `:no_return`.

### The ignore file (`.dialyzer_ignore.exs`)

Dialyxir looks for `.dialyzer_ignore.exs` by default. Override the path via `:ignore_warnings`. If the path ends in `.exs`, the file is evaluated as Elixir data.

Supported entry shapes, from the [Dialyxir README](https://hexdocs.pm/dialyxir/readme.html):

```elixir
[
  # {short_description}
  {":0:unknown_function Function :erl_types.t_to_string/1 does not exist."},
  # {short_description, warning_type}
  {":0:unknown_function Function :erl_types.t_to_string/1 does not exist.", :unknown_function},
  # {short_description, warning_type, line}
  {":0:unknown_function Function :erl_types.t_to_string/1 does not exist.", :unknown_function, 0},
  # {file, warning_type, line}
  {"lib/my_app/foo.ex", :no_return, 100},
  # {file, warning_description}
  {"lib/my_app/foo.ex", "Function :erl_types.t_to_string/1 does not exist."},
  # {file, warning_type}
  {"lib/my_app/foo.ex", :no_return},
  # {file}
  {"lib/my_app/foo.ex"},
  # regex — matched against the short-description format of Dialyzer output
  ~r/my_file\.ex.*my_function.*no local return/
]
```

`short_description` carries extra context that `warning_description` does not. Generate ignore-file entries by running the analysis with a formatter:

```bash
mix dialyzer --format ignore_file       # {file, warning_type}
mix dialyzer --format ignore_file_strict # {file, warning_description} — recommended
```

> **Historical rename:** `ignore_file_string` was renamed to `ignore_file_strict` in Dialyxir 1.4.5. Agents MUST use `ignore_file_strict`.

For per-module or per-function suppression, use the `@dialyzer` module attribute, the Elixir form of Erlang's `-dialyzer`. Example:

```elixir
@dialyzer {:nowarn_function, rollback: 1}
```

Enable `:list_unused_filters: true` in `mix.exs` or pass `--list-unused-filters` on the CLI to catch stale ignore entries. CI SHOULD fail on unused filters. From the README:

> "When used without `--ignore-exit-status`, this option will result in an error status code."

### `mix dialyzer.explain`

From [Mix.Tasks.Dialyzer.Explain](https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.Explain.html):

> "This task provides background information about Dialyzer warnings. If invoked without any arguments it will list all warning atoms. When invoked with the name of a particular warning, it will display information regarding it."

Usage example:

```bash
mix dialyzer.explain pattern_match
```

Run it with no arguments to discover the full warning registry.

### Output formats

The `--format` flag is repeatable. The default is `dialyxir`.

| Format | Use when |
|---|---|
| `dialyxir` | Default pretty-printed format; includes the `warning_name` for use with `explain`. |
| `short` | Compact format; suitable for the Elixir-term ignore file. |
| `raw` | The format before Dialyxir formatting. |
| `dialyzer` | Original Dialyzer format; suitable for simple string-match ignore files. |
| `github` | GitHub Actions message format (annotations in the PR files view). |
| `ignore_file` | `{file, warning_type}` tuples for the ignore file. |
| `ignore_file_strict` | `{file, warning_description}` tuples for the ignore file (recommended). |

CI commonly combines `--format github --format dialyxir` so the PR gets annotations and the logs keep the full text.

### CI integration

Run Dialyzer in CI as a gating step once the project has a clean baseline. For new projects, treat it as advisory, then promote it to a gate when the baseline is clean. (See the [Policy decisions](#policy-decisions-for-individual-repos) section below.)

Cache the PLT with a key that includes the OTP version, the Elixir version, and a hash of `mix.lock`. Warm the cache with `mix dialyzer --plt`, then run the gate with `mix dialyzer` (optionally `--format github --format dialyxir`). Cache the PLT directory, for example `priv/plts` or `_build`. Use `--no-compile` if a previous step already ran `mix compile`.

A concrete GitHub Actions snippet:

```yaml
- name: Restore PLT cache
  uses: actions/cache@v4
  with:
    path: |
      priv/plts
      _build
    key: plt-${{ runner.os }}-${{ steps.beam.outputs.otp-version }}-${{ steps.beam.outputs.elixir-version }}-${{ hashFiles('**/mix.lock') }}
    restore-keys: |
      plt-${{ runner.os }}-${{ steps.beam.outputs.otp-version }}-${{ steps.beam.outputs.elixir-version }}-
- name: Build PLT
  run: mix dialyzer --plt
- name: Run dialyzer
  run: mix dialyzer --format github --format dialyxir
```

Ignore committed PLT files:

```text
/priv/plts/*.plt
/priv/plts/*.plt.hash
```

For CI-oriented `mix.exs` configuration, prefer `plt_local_path` and `plt_core_path`:

```elixir
defp dialyzer do
  [
    plt_local_path: "priv/plts",
    plt_core_path: "priv/plts",
    plt_add_apps: [:ex_unit],
    plt_add_deps: :app_tree,
    flags: [:unknown],
    ignore_warnings: ".dialyzer_ignore.exs",
    list_unused_filters: true
  ]
end
```

If a legacy setup requires `plt_file`, silence the deprecation warning in CI with `plt_file: {:no_warn, "priv/plts/project.plt"}`.

By default, `mix dialyzer` halts the VM with a non-zero exit status when warnings are found. Gate CI on that exit status. Use `--ignore-exit-status` ONLY for advisory "show warnings" jobs. `mix dialyzer --plt` always exits 0.

### Common mistakes (Dialyxir-specific)

- Using removed flags: `:race_conditions` (removed in 1.3.0), `--halt-exit-status` (removed in 1.0.0-rc.7), and `plt_add_deps: :transitive` / `:project` (deprecated).
- Using the deprecated `plt_file` as a plain string instead of `plt_local_path` / `plt_core_path`.
- Ignoring `:unknown_function` with the ignore file instead of adding the missing app to `plt_add_apps` or `plt_add_deps`.
- Suppressing a `:no_return` cascade at the top instead of fixing the deepest call in the stack.
- Forgetting to cache the PLT in CI, which rebuilds core PLTs every run.
- Using `--format dialyzer` (string-match) ignore entries when the term-format `.dialyzer_ignore.exs` is more robust.
- Leaving stale ignore entries; not enabling `list_unused_filters: true`.

### Incremental adoption

Follow this order:

1. Add `{:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}` to `mix.exs`.
2. Run `mix dialyzer` once; the first run builds the PLTs.
3. Triage `:unknown_function` warnings by completing the PLT through `plt_add_apps` or `plt_add_deps`.
4. Bootstrap `.dialyzer_ignore.exs` with `mix dialyzer --format ignore_file_strict` for genuine false-positive-shaped findings.
5. Once the baseline is clean, promote Dialyzer to a CI gate.
6. Then enable stricter flags such as `:unmatched_returns`, `:error_handling`, and optionally `:underspecs`.

This phased approach keeps the tool useful instead of noisy.

## Review checklist

- [ ] Every public function has an accurate `@spec`.
- [ ] The `@spec` return type covers all actual return paths, including `:error`/`:ok` tuples.
- [ ] `string()` is never used; `String.t()`, `binary()`, `charlist()`, or `nonempty_charlist()` is used instead.
- [ ] `%{}` is used only for the empty-map singleton, never for "any map" (use `map()` or `%{optional(any) => any}`).
- [ ] Type-variable constraints use `when var: constraint` only on `@spec`, `@callback`, or `@macrocallback`.
- [ ] `@impl Module` is used on behaviour implementations to catch arity/name typos.
- [ ] `@typedoc` immediately precedes the `@type`/`@typep`/`@opaque` it documents.
- [ ] `no_return()` is used only for functions that never return, not for side-effect functions.
- [ ] `@type`/`@opaque` are used for shared domain types; `@typep` is used for purely internal shapes.
- [ ] Dialyzer ignore entries include a reason, and `--list-unused-filters` is clean.

## Implementation checklist

- [ ] Add a `@spec` above each new public `def`/`defmacro`.
- [ ] Define `@type` or `@opaque` for domain types reused across the module or public API.
- [ ] Document types with `@typedoc` when their meaning is not obvious.
- [ ] Add `{:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}` to `mix.exs`.
- [ ] Configure `dialyzer:` in `mix.exs` with `plt_local_path`, `plt_add_deps`, and `ignore_warnings`.
- [ ] Run `mix dialyzer --plt` to build the PLT, then `mix dialyzer` to analyse.
- [ ] Add a `.dialyzer_ignore.exs` only when necessary, with a comment or commit message explaining each entry.
- [ ] Wire Dialyzer into CI with PLT caching keyed on OTP+Elixir version and `mix.lock`.

## Validation hooks

- `mix dialyzer` — run the default analysis.
- `mix dialyzer --plt` — build or update the PLT only.
- `mix dialyzer --format short` — concise output for local iteration.
- `mix dialyzer.explain <warning>` — describe a warning type.
- `mix docs` — verify that ExDoc renders `@spec`, `@type`, and `@typedoc` correctly.
- `mix compile` — malformed typespecs often produce compiler warnings.

## Examples

### Full `StringHelpers` module

```elixir
defmodule StringHelpers do
  @typedoc "A word separated by whitespace."
  @type word() :: String.t()

  @spec long_word?(word()) :: boolean()
  def long_word?(word) do
    String.length(word) > 8
  end
end
```

Demonstrates `@typedoc`, `@type`, and `@spec` together.

### Result / union type

```elixir
@type result(error) :: {:ok, term()} | {:error, error}

@spec fetch_user(integer()) :: result(atom())
def fetch_user(id) do
  case Repo.get(User, id) do
    nil -> {:error, :not_found}
    user -> {:ok, user}
  end
end
```

A parameterized union makes the error type explicit.

### Keyword options typed as a union

```elixir
@type option ::
        {:name, String.t()}
        | {:max, pos_integer()}
        | {:min, pos_integer()}

@type options :: [option()]

@spec process(options()) :: :ok
def process(opts) do
  name = Keyword.get(opts, :name, "default")
  max = Keyword.get(opts, :max, 100)
  _min = Keyword.get(opts, :min, 0)
  IO.puts("#{name}: max #{max}")
end
```

This documents exactly which keys are allowed, that none are required, and that order does not matter.

### Map with required and optional fields

```elixir
@type user :: %{
        required(:id) => pos_integer(),
        required(:name) => String.t(),
        optional(:email) => String.t(),
        optional(any) => any
      }

@spec user_name(user()) :: String.t()
def user_name(user), do: user.name
```

`optional(any) => any` lets callers pass extra keys without breaking the type.

### Behaviour with `@callback` and `@impl`

```elixir
defmodule Parser do
  @callback parse(String.t()) :: {:ok, term()} | {:error, atom()}
  @callback extensions() :: [String.t()]
end

defmodule JSONParser do
  @behaviour Parser

  @impl Parser
  def parse(str), do: Jason.decode(str)

  @impl Parser
  def extensions, do: [".json"]
end
```

`@impl Parser` ensures the implementation matches the callback name and arity.

### Parameterized type

```elixir
defmodule Dict do
  @type dict(key, value) :: [{key, value}]

  @spec get(dict(key, value), key) :: {:ok, value} | :error when key: var, value: var
  def get(dict, key) do
    case List.keyfind(dict, key, 0) do
      {^key, value} -> {:ok, value}
      nil -> :error
    end
  end
end
```

Parameterized types let the same structure carry precise element types.

## Common mistakes

| Mistake | Why it is wrong | Correct form |
|---|---|---|
| Using `string()` | In Elixir, `string()` means a charlist (Erlang string). | Use `String.t()`, `binary()`, `charlist()`, or `nonempty_charlist()`. |
| Using `%{}` for "any map" | `%{}` is the singleton type for the empty map. | Use `map()` or `%{optional(any) => any}`. |
| `@typedoc` not immediately before `@type` | ExDoc will attach the doc to the wrong type or drop it. | Place `@typedoc` on the line directly above `@type`/`@typep`/`@opaque`. |
| `when` guards on `@type` | Guard syntax is valid only for `@spec`, `@callback`, and `@macrocallback`. | Use plain parameterized types or move the constraint to the spec. |
| `no_return()` for side-effect functions | `no_return()` means the function never returns. | For side-effect functions, return the actual value (`:ok`, `String.t()`, etc.). |
| Overusing `any()`/`term()` | Hides intent and weakens Dialyzer's analysis. | Use the most precise type that covers the real values. |
| Forgetting `@impl` | A typo in callback name or arity fails silently. | Add `@impl Behaviour` above every callback implementation. |
| Mismatched arity between `@spec` and `def` | This usually raises a compile error. | Ensure the `@spec` argument count matches the function head. |
| Required map keys that exclude keys the function uses | A map with extra keys does not belong to the type. | End the map type with `optional(any) => any` when extra keys are expected. |

## Strict vs contextual guidance

### Strict (enforce in review)

- Every public function has a precise `@spec`.
- Every `@type`/`@opaque` shared across the public API is documented with `@typedoc`.
- `@impl` is required on every behaviour callback implementation.
- `string()` is forbidden; use `String.t()`, `binary()`, or `charlist()`.
- `%{}` is reserved for the empty-map singleton.
- `no_return()` is used only for never-returning functions.
- Dialyzer runs in CI and gates merges.

### Contextual (start here, tighten over time)

- Add `@spec` to new public functions as you write them; backfill legacy code incrementally.
- Define `@type` for domain types that appear in three or more places.
- Allow `term()` for genuinely dynamic values (e.g. Ecto changeset errors, protocol inputs).
- Treat Dialyzer as advisory until the project has a clean baseline, then make it a gate.
- Use `@opaque` when a type is public but its internals should remain free to change.

## Policy decisions for individual repos

Each repo SHOULD record its chosen policy in `docs/elixir/static-analysis-credo.md` or a project-level `CONTRIBUTING.md`:

- Which Dialyzer flags are enabled (default `:unknown`, or additional flags such as `:error_handling`, `:race_conditions`, `:unmatched_returns`).
- Whether CI fails on Dialyzer warnings or reports them as advisory annotations.
- PLT caching strategy (key on OTP+Elixir version, `mix.lock` hash, and optionally `mix.exs` hash).
- Whether to fail on `:unknown_function` warnings or add the relevant apps to `plt_add_apps`.
- Pinned versions of Elixir, OTP, and Dialyxir for reproducible CI analysis.
- Whether typespecs are required on all public functions or only on library/module boundaries.
- Whether `@opaque` is used aggressively to hide internal data structures.

## Related docs

- `docs/elixir/naming-conventions.md` — identifier casing, including type names.
- `docs/elixir/language-fundamentals.md` — basic Elixir syntax and semantics.
- `docs/elixir/core-modules.md` — common remote types such as `String.t/0`, `Enum.t/0`, `MapSet.t/0`.
- `docs/elixir/mix-project-structure.md` — project layout and dependency configuration.
- `docs/elixir/otp-supervision.md` — behaviours and callbacks in OTP.
- `docs/elixir/testing-exunit.md` — testing conventions.
- `docs/elixir/configuration-and-runtime.md` — runtime and release concerns.
- `docs/elixir/static-analysis-credo.md` — static analysis with Credo.

Official references: [Elixir Typespecs](https://hexdocs.pm/elixir/typespecs.html) and [Dialyxir](https://hexdocs.pm/dialyxir/).

## Related skills

None defined yet.
