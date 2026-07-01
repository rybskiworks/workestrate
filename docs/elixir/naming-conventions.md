# Naming Conventions

## Purpose

Define naming conventions for Elixir code in this repo. Future agents who write, review, refactor, debug, or validate Elixir should follow these rules so identifiers are consistent, predictable, and aligned with the official Elixir documentation.

## Sources used

- https://hexdocs.pm/elixir/naming-conventions.html (PRIMARY)
- https://hexdocs.pm/elixir/modules-and-functions.html
- https://hexdocs.pm/elixir/typespecs.html
- https://hexdocs.pm/elixir/alias-require-and-import.html
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html
- https://hexdocs.pm/elixir/module-attributes.html
- https://hexdocs.pm/elixir/Module.html
- https://hexdocs.pm/elixir/basic-types.html
- https://hexdocs.pm/elixir/library-guidelines.html
- https://hexdocs.pm/elixir/String.html
- https://hexdocs.pm/elixir/introduction.html (checked; no naming mentions)
- Markdown source for the naming-conventions page: https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/pages/references/naming-conventions.md

This page reflects Elixir v1.20.2 docs.

## Core guidance

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html) "Casing":

> "Elixir developers must use `snake_case` when defining variables, function names, module attributes, and the like."
> "Aliases, commonly used as module names, are an exception as they must be capitalized and written in `CamelCase`, like `OptionParser`. For aliases, capital letters are kept in acronyms, like `ExUnit.CaptureIO` or `Mix.SCM`."
> "Atoms can be written either in `:snake_case` or `:CamelCase`, although the convention is to use the snake case version throughout Elixir."

Example from the page:

```elixir
some_map = %{this_is_a_key: "and a value"}
is_map(some_map)
```

Use `snake_case` everywhere except module/alias names, which use `CamelCase` and keep acronym capitals. Prefer `:snake_case` for atoms.

## Practical rules

### Case-conversion summary

| Identifier kind | Case | Example |
|---|---|---|
| Modules / aliases | CamelCase | `MyApp.Foo.Bar`, `OptionParser`, `ExUnit.CaptureIO` |
| Variables | snake_case | `some_map`, `this_is_a_key` |
| Function names | snake_case | `is_map/1`, `do_sum/2`, `zero?/1` |
| Module attributes (`@foo`) | snake_case | `@hours_in_a_day`, `@service`, `@custom_attr` |
| Atoms | snake_case (preferred) | `:ok`, `:error`, `:apple`, `:enoent` |
| Filenames | snake_case | `my_app.ex`, `math.exs`, `mix.exs` |
| Project/app names | snake_case | `hello_world`, `my_app` |
| Type names (typespecs) | snake_case (atoms) | `t()`, `dict(key, value)`, `option` |
| Pseudo-variables | `__UPPER_SNAKE__` | `__MODULE__`, `__ENV__`, `__DIR__`, `__CALLER__`, `__STACKTRACE__` |
| Compile-metadata functions | `__foo__` | `__info__/1`, `__using__/1`, `__before_compile__/1` |
| Unused variables | `_foo` or `_` | `_contents`, `_sep` |
| Underscore-prefixed functions | `_foo` (not auto-imported) | `_wont_be_imported/0` |

### Modules

Module/alias names MUST be capitalized `CamelCase`. The last segment is the "name". Capital letters are kept in acronyms: `ExUnit.CaptureIO`, `Mix.SCM`.

An alias is compiled to an atom:

```elixir
iex> is_atom(String)
true
iex> to_string(String)
"Elixir.String"
iex> :"Elixir.String" == String
true
```

Module nesting is lexical: define a nested module inside an outer module and the inner module is accessible by its short name within the same scope:

```elixir
defmodule Foo do
  defmodule Bar do
  end
end
```

Modules are isolated: you can define `Foo.Bar` without defining `Foo` first. If `Bar` is later moved outside `Foo`, reference it fully as `Foo.Bar` or add `alias Foo.Bar`.

`alias Math.List` is the same as `alias Math.List, as: List` (it defaults to the last segment). Use multi-alias to shorten several siblings:

```elixir
alias MyApp.{Foo, Bar, Baz}
```

Sources: [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html), [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html), [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html), [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html).

### Functions

Function names MUST start with a lowercase letter or underscore. Use `def/2` for public functions and `defp/2` for private functions:

```elixir
defmodule Math do
  def sum(a, b), do: do_sum(a, b)
  defp do_sum(a, b), do: a + b
end

Math.sum(1, 2)    #=> 3
Math.do_sum(1, 2) #=> ** (UndefinedFunctionError)
```

### Variables

Variables, module attributes, and function names are `snake_case`.

### Atoms

Atoms may be `:snake_case` or `:CamelCase`; prefer `snake_case`. `:ok` and `:error` are idiomatic for operation results. `true`, `false`, and `nil` are atoms and may omit the leading `:`:

```elixir
iex> true == :true
true
iex> is_atom(false)
true
```

Source: [basic-types.html](https://hexdocs.pm/elixir/basic-types.html).

### Filenames and module-to-file mapping

Filenames follow the `snake_case` form of the module they define: `MyApp` -> `my_app.ex`.

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "However, this is only a convention. At the end of the day any filename can be used as they do not affect the compiled code in any way."

The community convention (not enforced by the compiler) is that a dotted module name mirrors the directory path:

| Module | Conventional file |
|---|---|
| `MyApp` | `lib/my_app.ex` |
| `MyApp.Foo.Bar` | `lib/my_app/foo/bar.ex` |
| `MyApp.Foo.BarTest` | `test/my_app/foo/bar_test.exs` |

- `.ex` files are compiled.
- `.exs` files are scripts; they share the same semantics but are intended for scripting and tests.
- Mix `:elixirc_paths` defaults to `["lib"]`.
- Project/app names are `snake_case` (the application atom is `:my_app`).

Sources: [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html), [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html), [library-guidelines.html](https://hexdocs.pm/elixir/library-guidelines.html).

### Trailing `?` (boolean predicates)

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "Functions that return a boolean are named with a trailing question mark."

Examples: `Keyword.keyword?/1`, `Mix.debug?/0`, `String.contains?/2`.

A `?` function MUST return a boolean:

```elixir
defmodule Math do
  def zero?(0), do: true
  def zero?(x) when is_integer(x), do: false
end

Math.zero?(0)         #=> true
Math.zero?(1)         #=> false
Math.zero?([1, 2, 3]) #=> ** (FunctionClauseError)
```

### `is_` prefix (guard-safe boolean checks)

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "Type checks and other boolean checks that are allowed in guard clauses are named with an `is_` prefix."

> "Examples: `Integer.is_even/1`, `is_list/1`"

> "These functions and macros follow the Erlang convention of an `is_` prefix, instead of a trailing question mark, precisely to indicate that they are allowed in guard clauses. Type checks that are not valid in guard clauses do not follow this convention, such as `Keyword.keyword?/1`."

> "A trailing question mark should not be used in combination with the `is_` prefix."

Decision rule: guard-safe boolean check -> `is_foo` (e.g. `is_list/1`); non-guard boolean check -> `foo?`. Never `is_foo?`.

### Trailing `!` (raising variant)

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "A trailing bang (exclamation mark) signifies a function or macro where failure cases raise an exception. They most often exist as a 'raising variant' of a function that returns `:ok`/`:error` tuples (or `nil`)."

```elixir
File.read("file.txt")         #=> {:ok, "file contents"}
File.read("no_such_file.txt") #=> {:error, :enoent}
File.read!("file.txt")        #=> "file contents"
File.read!("no_such_file.txt")  #=> ** (File.Error) ...
```

Prefer the non-bang version when handling outcomes via pattern matching:

```elixir
case File.read(file) do
  {:ok, body} -> # use body
  {:error, reason} -> # handle reason
end
```

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "When thinking about failure cases, we are often thinking about semantic errors related to the operation being performed, such as failing to open a file or trying to fetch key from a map. Errors that come from invalid argument types, or similar, must always raise regardless if the function has a bang or not."

A bang variant can exist without a non-bang counterpart. This implies the operation can error and leaves room for a future non-raising variant:

```elixir
Protocol.assert_protocol!/1
PartitionSupervisor.resize!/2
```

Paired examples: `Base.decode16/2` & `Base.decode16!/2`, `File.cwd/0` & `File.cwd!/0`.

### Trailing `=`

The official [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html) page does NOT document a trailing `=` suffix. Do not treat `=` as an official naming convention.

### Underscores

Unused values MUST be assigned to `_` or a variable starting with `_`:

```elixir
{:ok, _contents} = File.read("README.md")
```

Function names starting with `_` are never auto-imported:

```elixir
defmodule Example do
  def _wont_be_imported, do: :oops
end

import Example
_wont_be_imported() #=> ** (CompileError) undefined function _wont_be_imported/0
```

To import a `_`-prefixed function you must use an explicit `:only` selector, for example:

```elixir
import File.Stream, only: [__build__: 3]
```

### Pseudo-variables and `__foo__` metadata functions

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "Elixir also includes five special forms that follow the double underscore format: `__CALLER__/0`, `__DIR__/0`, `__ENV__/0` and `__MODULE__/0` retrieve compile-time information about the current environment, while `__STACKTRACE__/0` retrieves the stacktrace for the current exception."

The authoritative list of these compile-environment accessors is documented in [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html) — exactly five:

- `__MODULE__/0` — the current module name as an atom, or `nil`.
- `__ENV__/0` — the current environment as a `Macro.Env` struct.
- `__DIR__/0` — the absolute directory of the current file (a shortcut for `Path.dirname(__ENV__.file)`).
- `__CALLER__/0` — the caller environment as a `Macro.Env` struct (only valid inside macros).
- `__STACKTRACE__/0` — the stacktrace for the currently handled exception.

> Note: there is **no** `__FILE__` special form in Elixir. To get the current file path at compile time, use `__ENV__.file`. Do not write `__FILE__` — it will not compile.

Compile-time metadata attached to modules uses the `__foo__` format, e.g. `__info__/1`:

```elixir
String.__info__(:functions) #=> [at: 2, capitalize: 1, chunk: 2, ...]
```

### Constants and module attributes

Module attributes are `snake_case`: `@custom_attr`, `@hours_in_a_day`.

Elixir does NOT use `ALL_CAPS` for constants. Do not write `@HOURS_IN_A_DAY`.

The docs recommend functions over module-attribute constants:

> "functions themselves are sufficient for the role of constants... instead of `@hours_in_a_day 24`, prefer `defp hours_in_a_day(), do: 24`."

```elixir
# Avoid:
@hours_in_a_day 24

# Prefer:
defp hours_in_a_day(), do: 24
```

Shared constants may live in a `MyApp.Constants` module as functions.

Sources: [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html), [Module.html](https://hexdocs.pm/elixir/Module.html).

### Type names (typespecs)

Define types with `@type`, `@typep` (private), and `@opaque` (public structure hidden):

```elixir
@type type_name :: type
@typep type_name :: type
@opaque type_name :: type
```

Type names are `snake_case` atoms. Parameterized types look like this:

```elixir
@type dict(key, value) :: [{key, value}]
```

Example:

```elixir
@type option :: {:name, String.t} | {:max, pos_integer} | {:min, pos_integer}
@type options :: [option()]
```

### The `t` type convention

A module's primary/public data type should be named `t/0` and referenced as `Module.t()` from elsewhere. From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "the `Range` module defines a `t/0` type... referred to as `Range.t/0`. In a similar fashion, a string is `String.t/0`."

```elixir
@type t() :: binary()   # String.t
```

### Special names

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html) "length and size":

> "When you see `size` in a function name, it means the operation runs in constant time (also written as \"O(1) time\") because the size is stored alongside the data structure. Examples: `map_size/1`, `tuple_size/1`"
> "When you see `length`, the operation runs in linear time (\"O(n) time\") because the entire data structure has to be traversed. Examples: `length/1`, `String.length/1`"

Rule summary: `size` = O(1); `length` = O(n). Name new functions accordingly. The official page cites `map_size/1`, `tuple_size/1`, `length/1`, and `String.length/1`. If you mention `byte_size/1` or `Enum.count/1`, attribute them to `Kernel`/`Enum`, not this naming-conventions section.

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html) "get, fetch, fetch!":

> "When you see the functions `get`, `fetch`, and `fetch!` for key-value data structures, you can expect the following behaviours:
> - `get` returns a default value (which itself defaults to `nil`) if the key is not present, or returns the requested value.
> - `fetch` returns `:error` if the key is not present, or returns `{:ok, value}` if it is.
> - `fetch!` raises if the key is not present, or returns the requested value.
> Examples: `Map.get/2`, `Map.fetch/2`, `Map.fetch!/2`, `Keyword.get/2`, `Keyword.fetch/2`, `Keyword.fetch!/2`"

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html) "compare":

> "The function `compare/2` should return `:lt` if the first term is less than the second, `:eq` if the two terms compare as equivalent, or `:gt` if the first term is greater than the second.
> Examples: `DateTime.compare/2`
> Note that this specific convention is important due to the expectations of `Enum.sort/2`"

## Review checklist

- [ ] Modules use `CamelCase`; acronyms keep capitals; variables/functions/attributes use `snake_case`.
- [ ] `?` functions return booleans; no `is_foo?` combinations.
- [ ] `is_` prefix is used only for guard-safe checks; non-guard predicates use `?`.
- [ ] `!` functions raise on semantic failure; a paired non-bang variant exists when outcomes should be handled.
- [ ] Unused variables are bound to `_`/`_foo`; no `ALL_CAPS` module-attribute constants.
- [ ] `size` vs `length` matches O(1)/O(n) semantics.
- [ ] `get`/`fetch`/`fetch!` semantics match the expected return shapes.
- [ ] `compare/2` returns `:lt`/`:eq`/`:gt`.
- [ ] Module-to-file path mirrors the dotted name under `lib/`; tests mirror under `test/` with `_test.exs`.
- [ ] Primary type is named `t/0`; typespecs use `snake_case` atoms.

## Implementation checklist

- [ ] Choose `snake_case` for all new identifiers except modules.
- [ ] When adding a boolean predicate, decide guard-safety to pick `is_` vs `?`.
- [ ] When adding an operation that can fail, add a non-bang `{:ok, _}` / `{:error, _}` (or `nil`) variant before/with the `!` variant unless you are intentionally future-proofing.
- [ ] Mirror module path in file path; add `_test.exs`.
- [ ] Define `@type t` for a module's primary data type.
- [ ] Prefer `defp`/functions for constants over `@attr` constants.

## Validation hooks

- `mix format --check-formatted` — catches formatting issues; primarily formatting but keeps code tidy.
- `mix credo` — community linter; flags non-idiomatic naming such as `is_`+`?`, missing `?` return-type, etc. This is a community tool, not an official Elixir tool.
- `mix dialyzer` — typespec checks; will often flag `?` functions whose specs do not return `boolean`.
- Compiler warnings and errors — redefinition, unused `_` handling, and module-name issues surface at compile time.
- Note explicitly: `snake_case`/`CamelCase` for non-module/module names is NOT compiler-enforced, except that module names MUST start with an uppercase letter and function names MUST start with a lowercase letter or underscore.

## Examples

### Modules, aliases, and nesting

```elixir
defmodule MyApp.Foo.Bar do
  def hello, do: :world
end

alias MyApp.Foo.Bar
alias MyApp.{Foo, Baz}

Bar.hello()
```

### Public and private functions

```elixir
defmodule Math do
  def sum(a, b), do: do_sum(a, b)
  defp do_sum(a, b), do: a + b
end

Math.sum(1, 2)    #=> 3
Math.do_sum(1, 2) #=> ** (UndefinedFunctionError)
```

### Boolean predicates

```elixir
defmodule Math do
  def zero?(0), do: true
  def zero?(x) when is_integer(x), do: false
end

Math.zero?(0)         #=> true
Math.zero?(1)         #=> false
Math.zero?([1, 2, 3]) #=> ** (FunctionClauseError)
```

### Bang pairs and error handling

```elixir
File.read("file.txt")         #=> {:ok, "file contents"}
File.read("no_such_file.txt") #=> {:error, :enoent}
File.read!("file.txt")        #=> "file contents"
File.read!("no_such_file.txt")  #=> ** (File.Error) ...

# Prefer pattern matching on the non-bang variant:
case File.read(file) do
  {:ok, body} -> # use body
  {:error, reason} -> # handle reason
end
```

### Special names

```elixir
map_size(%{a: 1, b: 2})   # O(1)
tuple_size({1, 2, 3})     # O(1)
length([1, 2, 3])         # O(n)
String.length("hello")    # O(n)

Map.get(map, :key, :default)
Map.fetch(map, :key)      #=> {:ok, value} | :error
Map.fetch!(map, :key)     # value | raises

DateTime.compare(a, b)    #=> :lt | :eq | :gt
```

### Typespecs

```elixir
defmodule MyApp.Options do
  @type option :: {:name, String.t()} | {:max, pos_integer()} | {:min, pos_integer()}
  @type options :: [option()]

  @spec validate(options()) :: :ok | {:error, String.t()}
  def validate(opts) do
    # ...
  end
end
```

### Constants as functions

```elixir
# Avoid:
defmodule Calendar do
  @hours_in_a_day 24
end

# Prefer:
defmodule Calendar do
  defp hours_in_a_day(), do: 24
end
```

### File layout

Community convention for a project named `my_app`:

```text
lib/
  my_app/
    foo/
      bar.ex          # defmodule MyApp.Foo.Bar
  my_app.ex           # defmodule MyApp
test/
  my_app/
    foo/
      bar_test.exs    # defmodule MyApp.Foo.BarTest
```

## Common mistakes

- Using `CamelCase` for function or variable names (`myFunction`, `doStuff`).
- Using `ALL_CAPS` constants (`@MAX_RETRIES`).
- Naming a boolean predicate `is_foo?` (combines both conventions) — pick one.
- A `?` function that returns a non-boolean (e.g. `nil` or a value).
- A `!` function that silently returns on failure instead of raising.
- An O(n) function named `...size...` or an O(1) function named `...length...`.
- A mismatched module-to-file path that confuses readers (allowed by the compiler, discouraged).
- Treating trailing `=` as an official convention (it is not documented on naming-conventions.html).
- Defining constants as module attributes when a function reads better.
- Forgetting `@type t` so a struct module's type isn't referable as `Module.t()`.

## Strict vs contextual guidance

### Strict

- Module/alias names MUST start uppercase (`CamelCase`). The parser treats capitalized identifiers as aliases.
- Function names MUST start lowercase or underscore.
- `?` predicate functions must return a boolean.
- Never combine the `is_` prefix with a trailing `?`.
- `_`-prefixed functions are not auto-imported; this is language behavior.
- Reserved module attributes and special forms cannot be overridden or reused.

### Conventions (not enforced by the compiler)

- `snake_case` for variables, functions, attributes, filenames, atoms, and types.
- `CamelCase` for module names (the compiler requires an uppercase start, but the full `CamelCase` shape is convention).
- Filename mirrors module path (`MyApp.Foo.Bar` -> `lib/my_app/foo/bar.ex`) — explicitly "only a convention".
- Trailing `!` for raising variants.
- `size` = O(1) vs `length` = O(n).
- `get`/`fetch`/`fetch!` return-shape conventions.
- `compare/2` returning `:lt`/`:eq`/`:gt`.
- `t/0` as a module's primary type.
- Preferring functions over module-attribute constants.

### Contextual tradeoffs

- Atoms may be `:CamelCase` if interop demands, but prefer `:snake_case`.
- Bang functions without a non-bang counterpart are acceptable when future-proofing.
- `.ex` vs `.exs` is intent-based, not semantics-based.

## Policy decisions for individual repos

- Whether to enforce module-to-file path mirroring in CI/credo.
- Whether to require a non-bang variant for every `!` function.
- Whether to allow bang functions without a non-bang counterpart.
- Whether to standardize a `MyApp.Constants` module or inline functions.
- Lint stack: enable `mix format --check-formatted`, `mix credo --strict`, and/or `mix dialyzer`.
- Atom casing policy for interop modules (`snake_case` default vs `CamelCase` for Erlang interop).

## Related docs

- Related Elixir corpus docs: `docs/elixir/language-fundamentals.md` (modules and functions), `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.

## Related skills

- None defined yet.
