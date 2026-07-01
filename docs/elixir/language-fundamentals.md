# Language Fundamentals

## Purpose

This is the Elixir language fundamentals reference for AI coding agents who write, review, and debug Elixir. It covers the full language fundamentals: basic types, operators, pattern matching and guards, control flow, modules and functions, structs, protocols, enumerables and streams, processes, alias/require/import, module attributes, comprehensions, sigils, and more. All Elixir values are immutable, and basic types are the foundation for pattern matching, control flow, and data modeling.

## Sources used

- https://hexdocs.pm/elixir/basic-types.html (PRIMARY for Basic Types)
- https://hexdocs.pm/elixir/lists-and-tuples.html
- https://hexdocs.pm/elixir/binaries-strings-and-charlists.html (PRIMARY for Strings, Binaries, and Charlists)
- https://hexdocs.pm/elixir/String.html (UTF-8 encoding, graphemes vs codepoints, escape table, String vs :binary guidance)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#%3C%3C%3E%3E/1 (<<>> bitstring/binary constructor: type/size/unit/signed/endian modifiers)
- https://hexdocs.pm/elixir/sigils.html (~c / ~s / ~S sigils, heredocs, interpolation)
- https://hexdocs.pm/elixir/IO.html (IO data / chardata, iodata_to_binary/1, iodata_length/1)
- https://www.erlang.org/doc/apps/stdlib/binary.html (:binary Erlang module: split/2, copy/2, compile_pattern/1)
- https://hexdocs.pm/elixir/keywords-and-maps.html (PRIMARY for Keywords and Maps)
- https://hexdocs.pm/elixir/Map.html (Map module API)
- https://hexdocs.pm/elixir/Keyword.html (Keyword module API)
- https://hexdocs.pm/elixir/Access.html (Access behaviour; get_in/put_in/update_in/pop_in/get_and_update_in)
- https://hexdocs.pm/elixir/structs.html (PRIMARY for Structs: defstruct, default fields, __struct__ field, struct!/2, @enforce_keys, pattern matching with %)
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/typespecs.html (@type, @spec, @callback, @macrocallback, @optional_callbacks; referenced by Module Attributes)
- https://hexdocs.pm/elixir/operators.html (PRIMARY for Basic Operators: precedence table + operator inventory)
- https://hexdocs.pm/elixir/basic-operators.html (DEAD — now 404; content folded into operators.html and Kernel.html)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html (=, ^, ., &, ::; PRIMARY for Control Flow: case/2, cond/1, with/1)
- https://hexdocs.pm/elixir/patterns-and-guards.html (PRIMARY for Pattern Matching and Guards)
- https://hexdocs.pm/elixir/case-cond-and-if.html (PRIMARY for Control Flow: case/cond/if narrative)
- https://hexdocs.pm/elixir/docs-tests-and-with.html (with/1 idiom: nested case → with refactor)
- https://hexdocs.pm/elixir/Kernel.html#if/2, unless/2, match?/2 (Control Flow macros + soft-deprecation note)
- https://hexdocs.pm/elixir/CaseClauseError.html, https://hexdocs.pm/elixir/CondClauseError.html, https://hexdocs.pm/elixir/WithClauseError.html (no-clause-match errors)
- https://hexdocs.pm/elixir/modules-and-functions.html (PRIMARY for Modules and Functions: defmodule, def/defp, function clauses, default args, function head rule, capture operator)
- https://hexdocs.pm/elixir/Kernel.html#def/2 (def/2 signature: def(call, expr \\ nil))
- https://hexdocs.pm/elixir/Kernel.html#defmodule/2 (defmodule/2 macro: defines a module given by name)
- https://hexdocs.pm/elixir/Function.html (Function module: capture/3, info/1,2; local vs external functions)
- https://hexdocs.pm/elixir/alias-require-and-import.html (PRIMARY for Alias, Require, and Import: alias/require/import/use directives)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#alias/2 (alias/2 special form: lexical scope, as:, multi-alias)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#import/2 (import/2 special form: only:/except:, :functions/:macros/:sigils)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#require/2 (require/2 special form: enabling macro calls)
- https://hexdocs.pm/elixir/Kernel.html#use/2 (use/2 macro: expands to require + __using__/1)
- https://hexdocs.pm/elixir/module-attributes.html (PRIMARY for Module Attributes: annotations, temporary storage, compile-time constants)
- https://hexdocs.pm/elixir/Module.html (Module reference: @behaviour, @impl, @callback, @before_compile, @after_compile, @after_verify, @on_definition, @on_load, @external_resource, @compile, @deprecated, @file, @vsn, @nifs, register_attribute/3)
- https://hexdocs.pm/elixir/Kernel.html#defstruct/1 (defstruct/1 macro)
- https://hexdocs.pm/elixir/Kernel.html#struct!/2 (struct!/2: raises on invalid keys)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#%25/2 (%/2 special form: struct creation/pattern matching, %struct_name{}, %_{})
- https://hexdocs.pm/elixir/protocols.html (PRIMARY for Protocols: defprotocol, defimpl, dispatch rules, @derive, @fallback_to_any, built-in protocols)
- https://hexdocs.pm/elixir/Protocol.html (Protocol module: @protocol/@for, consolidation, @undefined_impl_description, multiple impls)
- https://hexdocs.pm/elixir/Kernel.html#defprotocol/2 (defprotocol/2 macro)
- https://hexdocs.pm/elixir/Kernel.html#defimpl/3 (defimpl/3 macro: defimpl(name, opts, do_block \\ []))
- https://hexdocs.pm/elixir/Enumerable.html (Enumerable protocol: reduce/3 core + count/1/member?/2/slice/1 optimizations)
- https://hexdocs.pm/elixir/Collectable.html (Collectable protocol: into/1, Enum.into/2)
- https://hexdocs.pm/elixir/Inspect.html (Inspect protocol: @derive {Inspect, only:/except:/optional:}, #User<...> notation)
- https://hexdocs.pm/elixir/String.Chars.html (String.Chars protocol: to_string/1)
- https://hexdocs.pm/elixir/List.Chars.html (List.Chars protocol: to_charlist/1)
- https://hexdocs.pm/elixir/recursion.html (PRIMARY for Recursion: loops through recursion, reduce/map algorithms, tail-call optimization)
- https://hexdocs.pm/elixir/FunctionClauseError.html (raised when no function clause matches in a recursive call)
- https://hexdocs.pm/elixir/Enum.html (Enum module: eager map/2, reduce/3, filter/2, polymorphic over Enumerable)
- https://hexdocs.pm/elixir/Stream.html (Stream module: lazy map/2, filter/2, cycle/1, resource/3)
- https://hexdocs.pm/elixir/enumerable-and-streams.html (PRIMARY for Enumerables and Streams: eager vs lazy, pipe operator, streams; NOTE: URL is singular "enumerable-and-streams")
- https://hexdocs.pm/elixir/Kernel.html#%7C%3E/2 (pipe operator |> special form)
- https://hexdocs.pm/elixir/enum-cheat.html (Enum cheatsheet)
- https://hexdocs.pm/elixir/File.html#stream!/1 (File.stream!/1 built on Stream.resource/3)
- https://hexdocs.pm/elixir/processes.html (PRIMARY for Processes: spawn, send/receive, links, tasks, stateful processes)
- https://hexdocs.pm/elixir/Kernel.html#spawn/1, #spawn_link/1, #send/2, #self/0 (process primitives)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#receive/1 (receive/1: mailbox pattern matching, after timeout)
- https://hexdocs.pm/elixir/Process.html (Process module: alive?/1, link/1, register/2, monitor/1)
- https://hexdocs.pm/elixir/Task.html (Task module: start/1, start_link/1, async/1, await/1)
- https://hexdocs.pm/elixir/Agent.html, https://hexdocs.pm/elixir/GenServer.html (stateful process abstractions)
- https://hexdocs.pm/elixir/comprehensions.html (PRIMARY for Comprehensions: for/1, generators, filters, :into, bitstring generators)
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#for/1 (for/1 special form: full reference incl. :reduce, :uniq)
- https://hexdocs.pm/elixir/Collectable.html (Collectable protocol: required by :into)
- https://hexdocs.pm/elixir/sigils.html (PRIMARY for Sigils: ~r/~s/~c/~w/~D/~T/~N/~U, custom sigils, heredocs)
- https://hexdocs.pm/elixir/Regex.html (Regex module: PCRE, modifiers)
- https://hexdocs.pm/elixir/Date.html, https://hexdocs.pm/elixir/Time.html, https://hexdocs.pm/elixir/NaiveDateTime.html, https://hexdocs.pm/elixir/DateTime.html (calendar sigil structs)

This page reflects Elixir v1.20.2 docs.

## Basic Types

### Immutability

From [lists-and-tuples.html](https://hexdocs.pm/elixir/lists-and-tuples.html):

> "Elixir data structures are immutable."

Every "mutating" operation returns a new value; the original is unchanged. `put_elem/3`, `%{map | k => v}`, `list ++ list`, and similar operations create new data rather than altering the input. This eliminates a large class of shared-state bugs and makes pattern matching and concurrent code safer.

### Integers

Literal syntax: `1`; hex `0x1F`; binary `0b1010`; octal `0o777`. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "Elixir also supports shortcut notations for entering binary, octal, and hexadecimal numbers."

Numeric literals also allow underscores as digit separators (`1_000_000`), a feature inherited from Erlang. Elixir integers are arbitrary precision (no fixed width, no overflow), which is an inherent property of the BEAM integer representation.

Arithmetic: `+`, `-`, `*` return an integer given integers; `/` ALWAYS returns a float. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "In Elixir, the operator / always returns a float."

Use `div/2` (truncated toward zero) and `rem/2` for integer division and modulus. `rem/2` raises `ArithmeticError` when the divisor is 0.

Typespec name: `integer()`. Subtypes: `pos_integer()`, `neg_integer()`, `non_neg_integer()`. Predicates: `is_integer/1`, `is_number/1`.

```elixir
iex> 1 + 2
3
iex> 10 / 2
5.0
iex> div(10, 2)
5
iex> rem(10, 3)
1
iex> 0x1F
31
iex> 0o777
511
iex> 0b1010
10
iex> is_integer(1)
true
```

### Floats

Floats require a dot followed by at least one digit: `1.0`, `1.0e-10`. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "Floats in Elixir are 64-bit precision."

Useful functions: `round/1` (nearest integer), `trunc/1` (toward zero), `Float.ceil/2`, `Float.floor/2`, `abs/1`.

Typespec name: `float()`. Predicates: `is_float/1`, `is_number/1`.

Equality gotcha: `1 == 1.0` is `true` (structural equality), but `1 === 1.0` is `false` (strict equality, different types). When comparing numbers of different types, the one with greater precision wins (float > integer) unless the magnitude exceeds ±9007199254740992.0.

```elixir
iex> round(3.58)
4
iex> trunc(3.58)
3
iex> is_float(2.15)
true
iex> 1 == 1.0
true
iex> 1 === 1.0
false
```

### Booleans

Literals are `true` and `false`. They are atoms. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "true, false and nil are atoms as well."

`true == :true` is `true`; `is_atom(false)` is `true`; `is_boolean(:false)` is `true`.

Typespec name: `boolean()` (defined as `true | false`). Predicate: `is_boolean/1`.

Strict boolean operators `and/2`, `or/2`, `not/1` require boolean operands and raise `BadBooleanError` otherwise; they short-circuit. Truthy/falsy operators `&&/2`, `||/2`, `!/1` treat only `false` and `nil` as falsy. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "use and, or and not when you are expecting booleans. If any of the arguments are non-boolean, use &&, || and !."

The full operator detail belongs to the "Basic Operators" section; this is introduced here for type correctness.

```elixir
iex> true and false
false
iex> false or is_boolean(true)
true
iex> 1 and true
** (BadBooleanError) expected a boolean on left-side of "and", got: 1
iex> 1 || true
1
iex> !nil
true
iex> !1
false
```

### `nil`

Represents absence of a value. It is an atom and one of only two falsy values in Elixir (the other is `false`). Predicate: `is_nil/1`. `nil == :nil` is `true`.

```elixir
iex> is_nil(nil)
true
iex> nil == :nil
true
iex> is_atom(nil)
true
```

### Atoms

Literal syntax `:foo`. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "An atom is a constant whose value is its own name. Some other languages call these symbols."

Equality is by name: `:foo == :foo` is `true`. `true`, `false`, and `nil` are atoms whose colon may be omitted. Module names are atoms: `is_atom(String)` is `true`, and `to_string(String)` returns `"Elixir.String"`.

Typespec name: `atom()`; `module()` is defined as `atom()`. Predicate: `is_atom/1`.

Idiomatic use: `:ok` / `:error` tagged tuples for operation results (e.g. `File.read/1` returns `{:ok, contents}` or `{:error, reason}`).

```elixir
iex> :apple == :apple
true
iex> true == :true
true
iex> is_atom(false)
true
iex> is_atom(String)
true
iex> to_string(String)
"Elixir.String"
```

### Strings (binaries)

Double-quoted `"hello"`, UTF-8 encoded. From [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html):

> "Strings in Elixir are represented internally by contiguous sequences of bytes known as binaries."

A string is a UTF-8 encoded binary. Concatenate with `<>`; interpolate with `"hello #{expr}"`.

Typespec name: use `String.t()` for UTF-8 strings; `binary()` for raw binaries. **Important:** the typespec `string()` is discouraged — it refers to Erlang charlists, not Elixir strings, and the compiler warns. Use `String.t()` for Elixir strings.

Predicates: `is_binary/1` (and `is_bitstring/1`, since binaries are bitstrings). Length: `byte_size/1` (bytes, O(1)), `String.length/1` (graphemes), `bit_size/1`. Character literal `?c` returns the integer code point; `\uXXXX` escapes for Unicode.

```elixir
iex> "hellö"
"hellö"
iex> "hello " <> "world!"
"hello world!"
iex> name = "world"
iex> "hello #{name}!"
"hello world!"
iex> byte_size("hellö")
6
iex> String.length("hellö")
5
iex> ?a
97
iex> "\u0061" == "a"
true
```

### Charlists (intro only)

A charlist is a list of integers where all integers are valid code points. From [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html):

> "A charlist is a list of integers where all the integers are valid code points."

Preferred sigil: `~c"hello"`. Single-quoted `'hello'` is soft-deprecated since Elixir v1.15 and emits warnings in future versions. Charlists are lists, so concatenate with `++`, not `<>` (which is for binaries and raises on lists).

Use case: Erlang interop — older Erlang libraries accept lists of code points. Conversion: `to_string/1` (charlist to string), `to_charlist/1` (string to charlist), both polymorphic.

Note: the dedicated "Strings, Binaries, and Charlists" section expands this topic in depth.

```elixir
iex> ~c"hello"
~c"hello"
iex> [?h, ?e, ?l, ?l, ?o]
~c"hello"
iex> is_list(~c"hello")
true
iex> ~c"this " ++ ~c"works"
~c"this works"
```

### Lists

Square-bracket `[1, 2, true, 3]`; heterogeneous. They are linked lists. From [lists-and-tuples.html](https://hexdocs.pm/elixir/lists-and-tuples.html):

> "Elixir data structures are immutable."
> "List operators never modify the existing list."

Operators: `++/2` (concatenate), `--/2` (subtract — removes the first occurrence of each RHS element from LHS). Head/tail: `hd/1`, `tl/1` (both raise `ArgumentError` on an empty list). Cons: `[head | tail]`.

Performance: `length/1` is linear. Prepending `[0 | list]` is cheap; appending `list ++ [4]` is slow because it traverses the whole left list. From [lists-and-tuples.html](https://hexdocs.pm/elixir/lists-and-tuples.html):

> "accessing the length of a list is a linear operation."

Typespec name: `list(type)`; shorthand `[]`; also `nonempty_list(type)`. Predicate: `is_list/1`.

Cross-reference `docs/elixir/naming-conventions.md` for the `size` = O(1) vs `length` = O(n) naming rule.

```elixir
iex> [1, 2, true, 3]
[1, 2, true, 3]
iex> length([1, 2, 3])
3
iex> [1, 2, 3] ++ [4, 5, 6]
[1, 2, 3, 4, 5, 6]
iex> [1, true, 2, false, 3, true] -- [true, false]
[1, 2, 3, true]
iex> list = [1, 2, 3]
iex> hd(list)
1
iex> tl(list)
[2, 3]
```

### Tuples

Curly-brace `{:ok, "hello"}`; heterogeneous; stored contiguously; fixed size. Immutable: `put_elem/3` returns a new tuple. Operations: `elem/2` (get by 0-based index), `tuple_size/1` (O(1)), `put_elem/3` (set by index).

Typespec name: `tuple()`, or literal `{a, b, c}`. Predicate: `is_tuple/1`.

Common idiom: tagged tuples `{:ok, value}` / `{:error, reason}` for success/failure returns. From [lists-and-tuples.html](https://hexdocs.pm/elixir/lists-and-tuples.html):

> "Lists are used when the number of elements returned may vary. Tuples have a fixed size."

Example: `String.split/1` returns a list, while `String.split_at/2` returns a 2-tuple.

```elixir
iex> tuple = {:ok, "hello"}
iex> tuple_size(tuple)
2
iex> elem(tuple, 1)
"hello"
iex> put_elem(tuple, 1, "world")
{:ok, "world"}
iex> tuple
{:ok, "hello"}
```

### Maps (intro only)

Literal `%{key => value}`; atom-key shorthand `%{name: "John", age: 23}` (equivalent to `%{:name => "John", :age => 23}`). Any value may be a key. Internal ordering is not guaranteed and not stable.

Access: `map[:key]` returns `nil` if missing; `map.key` (dot syntax, atom keys only) raises `KeyError` if missing. Update: `%{map | key: value}` raises `KeyError` if the key is not already present.

Predicate: `is_map/1`; `map_size/1` is O(1). Pattern matching: `%{}` matches any map; `%{:a => x}` matches on a subset (key `:a` must exist).

Typespec name: `map()`, or literal `%{key_type => value_type}` / `%{key: value_type}`.

Note: the dedicated "Keywords and Maps" section expands this topic in depth.

```elixir
iex> map = %{:a => 1, 2 => :b}
%{2 => :b, :a => 1}
iex> map[:a]
1
iex> map[:c]
nil
iex> %{map | :a => 99}
%{2 => :b, :a => 99}
iex> %{:a => x} = %{:a => 1, :b => 2}
iex> x
1
```

### Keyword lists (intro only)

A keyword list is a list of 2-tuples whose keys are atoms: `[parts: 3, trim: true]` is equivalent to `[{:parts, 3}, {:trim, true}]`. Brackets may be omitted when it is the last argument: `String.split(s, " ", parts: 3)`.

From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Elixir provides a special syntax for listing keyword lists. The three characteristics of keyword lists highlighted above are what led Elixir to implement keyword lists:
> 1. Keys must be atoms.
> 2. Keys are ordered, as specified by the developer.
> 3. Keys can be given more than once."

Access via `list[:key]` (returns the first matching key's value). Typespec name: `keyword()` ≡ `[{atom(), any()}]`; `keyword(t)` ≡ `[{atom(), t}]`.

Do not pattern match on keyword lists: order and key count are not fixed. They are lists, so all list operations apply and performance is linear.

Note: the dedicated "Keywords and Maps" section expands this topic in depth.

```elixir
iex> [{:parts, 3}, {:trim, true}] == [parts: 3, trim: true]
true
iex> list = [a: 1, b: 2]
iex> list[:a]
1
iex> is_list(list)
true
```

### Choosing a collection (summary)

Use this decision guide, sourced from the official docs:

- Variable number of elements → list; fixed size → tuple.
- Optional function arguments → keyword list; general key-value data → map; known-shape data with atom keys → map with dot access (and, later, structs).
- Success/failure returns → tagged tuples `{:ok, _}` / `{:error, _}`.

| Need | Choose |
|---|---|
| Ordered collection of unknown size | List |
| Fixed-size record | Tuple |
| Optional arguments / options | Keyword list |
| Arbitrary key-value lookup | Map |
| Structured data with typed keys | Struct (built on maps) |

### Built-in types & predicates quick-reference

From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html) and [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

| Type | Example literal | Typespec name | Guard predicate |
|---|---|---|---|
| Integer | `42` | `integer()` | `is_integer/1` |
| Float | `3.14` | `float()` | `is_float/1` |
| Boolean | `true`, `false` | `boolean()` | `is_boolean/1` |
| Atom | `:ok` | `atom()` | `is_atom/1` |
| `nil` | `nil` | `nil` | `is_nil/1` |
| String (binary) | `"hello"` | `String.t()` | `is_binary/1` |
| Bitstring / binary | `"raw"`, `<<1::3>>` | `binary()` / `bitstring()` | `is_bitstring/1` |
| List | `[1, 2, 3]` | `list()`, `[type]` | `is_list/1` |
| Tuple | `{:ok, 1}` | `tuple()` | `is_tuple/1` |
| Map | `%{a: 1}` | `map()` | `is_map/1` |
| Keyword list | `[a: 1]` | `keyword()` | `is_list/1` |
| Charlist | `~c"abc"` | `charlist()` | `is_list/1` |
| Function | `fn -> :ok end` | `function()` | `is_function/1` |
| Port | (runtime value) | `port()` | `is_port/1` |
| PID | (runtime value) | `pid()` | `is_pid/1` |
| Reference | (runtime value) | `reference()` | `is_reference/1` |
| Number | `1` or `1.0` | `number()` | `is_number/1` |

## Basic Operators

Most Elixir operators are functions or macros defined in `Kernel` and inlined at compile time; a smaller set lives in `Kernel.SpecialForms` and cannot be overridden. For the complete inventory see [operators.html](https://hexdocs.pm/elixir/operators.html); for the semantics of each operator see [Kernel.html](https://hexdocs.pm/elixir/Kernel.html). The old `basic-operators.html` page was removed in v1.20.x and its content folded into those two pages.

### Arithmetic operators

Unary `+x` returns `x`; unary `-x` negates. Binary `+`, `-`, `*` return an `integer()` when both operands are integers and a `float()` when any operand is a float. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "In Elixir, the operator / always returns a float."

Use `div/2` and `rem/2` for integer division and remainder. `div/2` truncates toward zero, not toward negative infinity; `rem/2` follows the sign of the dividend. `Integer.floor_div/2` is available when floored division is required. `abs/1`, `round/1`, `trunc/1`, `floor/1`, and `ceil/1` are guard-safe. `x ** y` raises to a power; an integer base with a non-negative integer exponent stays integral, otherwise the result is a float.

```elixir
iex> 10 / 2
5.0
iex> div(6, -4)
-1
iex> rem(6, -4)
2
iex> 2 ** 2
4
iex> 2 ** -4
0.0625
iex> round(2.5)
3
```

### Boolean operators: strict vs truthy

Elixir has two short-circuiting boolean operator families. The strict family (`and`, `or`, `not`) requires boolean operands and is guard-legal. The truthy family (`&&`, `||`, `!`) treats only `false` and `nil` as falsy and is **not** guard-legal. From [basic-types.html](https://hexdocs.pm/elixir/basic-types.html):

> "use and, or and not when you are expecting booleans. If any of the arguments are non-boolean, use &&, || and !."

From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> "a value is truthy when it is neither false nor nil. A value is falsy when it is either false or nil."

Rule of thumb: use `and`/`or`/`not` for booleans and inside guards; use `&&`/`||`/`!` for truthy control flow.

```elixir
iex> true and false
false
iex> false or is_boolean(true)
true
iex> "yay!" and true
** (BadBooleanError) expected a boolean on left-side of "and", got: "yay!"
iex> 1 || true
1
iex> List.first([]) && true
nil
iex> !nil
true
iex> !1
false
```

Cross-reference the `Booleans` subsection in `## Basic Types` above for the type-level framing.

### Comparison operators

`==` and `!=` are structural; `===` and `!==` are strict (same type and value). All comparison operators are inlined and guard-legal. From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> "number < atom < reference < function < port < pid < tuple < map < list < bitstring"

This ordering applies even to values of different types. Tuples compare by size then element-wise; maps by size then keys ascending then values; lists element-wise; bitstrings byte-by-byte; atoms by UTF-8 code-point order (byte-wise, not alphabetic). Struct comparison is structural by field-declaration order, not semantic — for example dates must use `Date.compare/2`.

```elixir
iex> 1 == 1.0
true
iex> 1 === 1.0
false
iex> 1 < :a
true
iex> [1, 2] < [1, 3]
true
iex> "álien" > "office"
true
iex> ~D[2017-03-31] > ~D[2017-04-01]
true
iex> Date.compare(~D[2017-03-31], ~D[2017-04-01])
:lt
```

### Concatenation and membership

`<>` concatenates binaries (strings). It can be used in pattern matching when the left side is a literal binary: `"foo" <> x = "foobar"` binds `x` to `"bar"`. `++` and `--` operate on lists; both are right-associative. `in` and `not in` test membership and expand differently in guards (RHS must be a list literal or range). `=~` performs a text match against a string or regex. String interpolation `"#{expr}"` converts via `String.Chars`.

```elixir
iex> "foo" <> "bar"
"foobar"
iex> "foo" <> x = "foobar"
iex> x
"bar"
iex> [1] ++ [2, 3]
[1, 2, 3]
iex> [1, 2, 3, 2, 1] -- [1, 2, 2]
[3, 1]
iex> 1 in [1, 2, 3]
true
iex> 5 in 1..1000
true
iex> "abcd" =~ ~r/c(d)/
true
iex> "abcd" =~ "bc"
true
iex> name = "world"
iex> "hello #{name}!"
"hello world!"
```

Avoid `not x in list`; it currently parses as `not(x in list)` but emits a deprecation warning. Always write `x not in list`. Cross-reference the `Strings (binaries)` and `Lists` subsections in `## Basic Types`.

### The pipe operator `|>`

`left |> right` passes the result of `left` as the first argument of `right`. It is left-associative and purely syntactic sugar for nested calls, so there is no runtime cost. Use it to express left-to-right data flow.

```elixir
iex> "  hello  " |> String.trim() |> String.upcase()
"HELLO"
```

### Special-form operators

These operators are defined in `Kernel.SpecialForms` and cannot be overridden:

| Operator | Meaning | Notes |
|---|---|---|
| `=` | Match operator | Pattern-matches RHS to LHS; binds variables. See `## Pattern Matching and Guards` for full treatment. |
| `^` | Pin operator | Matches an existing binding instead of rebinding. See `## Pattern Matching and Guards`. |
| `.` | Remote call / anonymous call / alias chain | `Module.fun(args)`, `fun.(args)`, `Foo.Bar`. |
| `&` | Capture operator | `&Mod.fun/arity` or `&(&1 * 2)` with placeholders `&1`, `&2`, ... |
| `::` | Type / bitstring operator | Typespecs (`@spec f() :: :ok`) and bitstring segments (`<<x::integer-little>>`). |
| `\|` | Cons / map update | `[head | tail]`, `%{map | key: value}`. Right-associative; updating a missing key raises `KeyError`. |
| `..` | Range | `1..10`, `first..last//step` (step since 1.12.0), bare `..` full-slice since 1.14.0. |
| `@` | Module attribute | Compile-time read/write; see the `## Module Attributes` section. |

Bitwise operators (`&&&`, `|||`, `^^^`, `<<<`, `>>>`, `~~~`) are provided by the `Bitwise` module and require `import Bitwise` or a fully-qualified call.

### Operator precedence and associativity

From [operators.html](https://hexdocs.pm/elixir/operators.html), highest to lowest precedence:

| Operators | Associativity |
|---|---|
| `@` | Unary |
| `.` | Left |
| `+` `-` `!` `^` `not` (unary) | Unary |
| `**` | Left |
| `*` `/` | Left |
| `+` `-` (binary) | Left |
| `++` `--` `+++` `---` `..` `<>` | Right |
| `in` `not in` | Left |
| `\|>` `<<<` `>>>` `<<~` `~>>` `<~` `~>` `<~>` | Left |
| `<` `>` `<=` `>=` | Left |
| `==` `!=` `=~` `===` `!==` | Left |
| `&&` `&&&` `and` | Left |
| `\|\|` `\|\|\|` `or` | Left |
| `=` | Right |
| `&` | Unary |
| `\|` | Right |
| `::` | Right |
| `when` | Right |
| `<-` `\\` | Left |

Practical notes: `++` and `--` are right-associative, so `[1,2,3] -- [1] ++ [2]` evaluates as `[1,2,3] -- ([1] ++ [2])` and yields `[3]`. `=` is right-associative, so `a = b = 1` binds both. `\|` is right-associative, which is why list cons chains and map updates compose predictably.

## Pattern Matching and Guards

In Elixir v1.20.x the older pattern-matching and guards material was consolidated into a single reference page, [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html). From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "Patterns in Elixir are made of variables, literals, and data structure specific syntax."

Pattern matching is the primary mechanism for destructuring values and driving control flow; guards augment patterns with a restricted, side-effect-free set of boolean checks. Cross-reference the `=`, `^`, and `|` rows in the `## Basic Operators` → `Special-form operators` table.

### The match operator `=`

The `=` operator matches the right-hand value against the left-hand pattern; it is not assignment. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "One of the most used constructs to perform pattern matching is the match operator (=):"

```elixir
iex> x = 1
1
iex> 1 = x
1
```

Patterns appear ONLY on the left of `=`; the right side follows normal evaluation rules. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "patterns are allowed only on the left side of =. The right side of = follows the regular evaluation semantics of the language."

`=` returns the right-hand value on success, so it is right-associative and chainable:

```elixir
iex> a = b = 1
1
iex> a
1
iex> b
1
```

A failed match raises `MatchError`. Because the right side is evaluated normally, an unbound variable on the right of `=` is a compile-time error:

```elixir
iex> 1 = y
** (CompileError) undefined variable "y"
```

### Variable binding and rebinding

Variables in patterns always bind (Elixir supports rebinding). From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "Variables in patterns are always assigned to:"

```elixir
iex> x = 1
1
iex> x = 2
2
iex> x
2
```

The same variable appearing twice in ONE pattern must bind to the same value:

```elixir
iex> {x, x} = {1, 1}
{1, 1}
iex> {x, x} = {1, 2}
** (MatchError) no match of right hand side value: {1, 2}
```

A variable cannot be defined through itself in the same pattern — cyclic definitions are rejected at compile time. For example, `{:ok, x} = {x, :ok}` is rejected: match once, then use a guard or a separate comparison if you need to relate values.

### The pin operator `^`

The pin operator `^` matches against an existing binding instead of rebinding it. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "In case you don't want the value of a variable to change, you can use the pin operator (^):"

```elixir
iex> x = 1
1
iex> ^x = 2
** (MatchError) no match of right hand side value: 2
```

A pinned value is compared for equality, whereas a bare pattern such as `%{}` matches any map. Contrast:

```elixir
iex> x = %{}
%{}
iex> {:ok, %{}} = {:ok, %{a: 13}}      # %{} is a wildcard map pattern — matches
{:ok, %{a: 13}}
iex> {:ok, ^x} = {:ok, %{a: 13}}       # ^x requires structural equality with %{} — fails
** (MatchError) no match of right hand side value: {:ok, %{a: 13}}
```

The pin works wherever patterns are allowed: function heads, `case`, `fn`, `with`, `receive`, and comprehensions. You cannot both pin and bind the same variable in one pattern.

### Pattern matching on tuples

Tuples use `{}` and require the same size with each element matching. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "A tuple in a pattern will match only tuples of the same size, where each individual tuple element must also match:"

```elixir
iex> {:ok, integer} = {:ok, 13}
{:ok, 13}
iex> {:ok, integer} = {:ok, 11, 13}     # wrong size
** (MatchError) no match of right hand side value: {:ok, 11, 13}
iex> {:ok, binary} = {:error, :enoent}  # wrong first element
** (MatchError) no match of right hand side value: {:error, :enoent}
```

### Pattern matching on lists

Lists use `[]` and require the same size. Head/tail decomposition uses `[head | tail]`; multiple elements may prefix the `|`. `[head | tail]` does NOT match an empty list.

```elixir
iex> [a, b, c] = [1, 2, 3]
[1, 2, 3]
iex> [head | tail] = [1, 2, 3]
[1, 2, 3]
iex> head
1
iex> tail
[2, 3]
iex> [first, second | tail] = [1, 2, 3]
[1, 2, 3]
iex> tail
[3]
iex> [head | tail] = []
** (MatchError) no match of right hand side value: []
```

Charlists are lists of integers, so prefix matches use `++`:

```elixir
iex> ~c"hello " ++ world = ~c"hello world"
~c"hello world"
iex> world
~c"world"
```

Cross-reference the `Lists` subsection in `## Basic Types` for the cons/`|` operator and prepend performance.

### Pattern matching on maps

Maps perform a SUBSET match — a map pattern matches any map that has AT LEAST the keys in the pattern. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "maps perform a subset match. This means a map pattern will match any other map that has at least all of the keys in the pattern."

```elixir
iex> %{name: name} = %{name: "meg"}
%{name: "meg"}
iex> %{name: name} = %{name: "meg", age: 23}   # extra keys are fine
%{age: 23, name: "meg"}
iex> %{name: name, age: age} = %{name: "meg"}  # missing key fails
** (MatchError) no match of right hand side value: %{name: "meg"}
```

The empty map `%{}` matches ANY map, unlike `[]` and `{}`:

```elixir
iex> %{} = %{name: "meg"}
%{name: "meg"}
```

Key shorthands: `%{key: v}` is equivalent to `%{:key => v}` (atom keys only); `%{key => v}` matches any key. Map keys in patterns must be literals or pinned variables:

```elixir
iex> key = :name
:name
iex> %{^key => value} = %{name: "meg"}
%{name: "meg"}
iex> value
"meg"
```

Cross-reference the `Maps (intro only)` subsection in `## Basic Types` and the `## Structs` section.

### Pattern matching on structs

Structs use `%ModuleName{field: pattern}` and match by struct type AND keys. An unknown key is a compile error. The struct module name itself can be captured:

```elixir
iex> defmodule User do
...>   defstruct [:name]
...> end
iex> %User{name: name} = %User{name: "meg"}
%User{name: "meg"}
iex> %User{type: type} = %User{name: "meg"}
** (CompileError) unknown key :type for struct User
iex> %struct_name{} = %User{name: "meg"}
%User{name: "meg"}
iex> struct_name
User
```

### Pattern matching on strings and binaries

Strings are UTF-8 binaries. Prefix match with `<>`; the LEFT of `<>` in a pattern must be a literal binary, the right may be any expression. Suffix matches are invalid.

```elixir
iex> "hello " <> world = "hello world"
"hello world"
iex> world
"world"
```

Richer bitstring segments use `<<>>` with type/size/unit modifiers:

```elixir
iex> <<val::unit(8)-size(2)-integer>> = <<123, 56>>
"{8"
iex> val
31544
```

Binary matching works on BYTES; use the `utf8` modifier for multibyte codepoint patterns. Cross-reference the `## Strings, Binaries, and Charlists` section and the `::` row in the `## Basic Operators` → `Special-form operators` table.

### Where patterns and guards can be used

| Construct | Patterns | Guards |
|---|---|---|
| `=` match operator | yes | NO |
| `match?/2` first arg | yes | yes |
| function clauses (`def`/`defp`) | yes | yes |
| anonymous function clauses (`fn`) | yes | yes |
| `case` clauses | yes | yes |
| `cond` clauses | NO (truthy expressions, not patterns) | n/a |
| `receive` clauses | yes | yes |
| `with` `<-` and `else` | yes | yes |
| `for`/comprehension `<-` | yes | yes |
| `try` `catch`/`else` | yes | yes |

In function heads, multiple clauses are tried in order; the FIRST clause whose patterns match AND whose guard is true wins. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "If a function has several clauses, Elixir will try each clause until it finds one that matches."

If none match, `FunctionClauseError` is raised.

`case` example with a guard and a `_` fallback:

```elixir
iex> case {1, 2, 3} do
...>   {1, x, 3} when x > 0 -> "matches, x is 2"
...>   _ -> "fallback"
...> end
"matches, x is 2"
```

A `case` with no matching clause raises `CaseClauseError`.

`cond` is NOT pattern matching — its clause heads are truthy boolean expressions evaluated top to bottom (only `false` and `nil` are falsy). If all are falsy, `CondClauseError` is raised.

```elixir
iex> cond do
...>   2 + 2 == 5 -> "no"
...>   1 + 1 == 2 -> "yes"
...> end
"yes"
```

The match operator `=` itself does NOT support guards; you cannot write `{:ok, b} when is_binary(b) = File.read(...)`.

### The underscore `_` and leading-underscore variables

Bare `_` is a wildcard that never binds and cannot be read. A name starting with `_` (e.g. `_height`) is a real variable the compiler will not warn about as unused, and it CAN be inspected. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "The underscore variable (_) has a special meaning as it can never be bound to any value."

```elixir
iex> {_, integer} = {:not_important, 1}
{:not_important, 1}
iex> integer
1
iex> _
** (CompileError) invalid use of _
```

Cross-reference the "Unused variables → `_foo` or `_`" row in `docs/elixir/naming-conventions.md`.

Guards augment patterns with a restricted set of side-effect-free boolean checks. They are optional and follow a `when` keyword.

### Guards: the `when` keyword

From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "Guards are a way to augment pattern matching with more complex checks."

> "The clause will be executed if and only if the guard expression returns true."

```elixir
defmodule M do
  def type(term) when is_integer(term), do: :integer
  def type(term) when is_float(term), do: :float
end
```

### Guards are strictly boolean (no truthy/falsy)

A guard must evaluate to literally `true`; any other value skips the clause. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "guards have no concept of 'truthy' or 'falsy'."

```elixir
defmodule Wrong do
  # WRONG: head is "some_value" (truthy but not true) → clause skipped
  def not_nil_head?([head | _]) when head, do: true
  def not_nil_head?(_), do: false
end

defmodule Right do
  # RIGHT:
  def not_nil_head?([head | _]) when head != nil, do: true
  def not_nil_head?(_), do: false
end

Wrong.not_nil_head?(["some_value"])
#=> false

Right.not_nil_head?(["some_value"])
#=> true
```

This strictness is why `and`/`or`/`not` (which require boolean operands) are guard-legal, while `&&`/`||`/`!` (truthy) are NOT. Cross-reference the `Boolean operators: strict vs truthy` subsection in `## Basic Operators`.

### Combining guards: `and`, `or`, and multiple `when`

Combine guards within one clause with `and`/`or`:

```elixir
def positive_int?(x) when is_integer(x) and x > 0, do: true
```

A chain of `or` can be written as MULTIPLE `when` clauses on the same head — this is guard OR. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "Elixir supports writing 'multiple guards' in the same clause."

```elixir
def categorize_number(term) when is_integer(term) or is_float(term) or is_nil(term),
  do: :maybe_number

# is equivalent to:
def categorize_number(term)
    when is_integer(term)
    when is_float(term)
    when is_nil(term) do
  :maybe_number
end
```

Rule: comma/`and` means ALL must pass (AND); repeated `when` means ANY may pass (OR). The multiple-`when` form also isolates raising sub-expressions (see below). In multi-clause function definitions, `;` separates WHOLE clauses (each with its own head + optional `when`), not guard alternatives.

### Errors in guards become failures, not exceptions

If a guard expression raises, the whole guard FAILS (treated as false) rather than propagating the exception. From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "In guards, when functions would normally raise exceptions, they cause the guard to fail instead."

```elixir
iex> case "hello" do
...>   something when tuple_size(something) == 2 -> :worked
...>   _ -> :failed
...> end
:failed
```

THE GOTCHA: combining alternatives with `or` lets one raising sub-expression short-circuit the rest:

```elixir
defmodule Check do
  # For {}, map_size/1 raises, short-circuiting the `or`, so tuple_size is never tried:
  def empty?(val) when map_size(val) == 0 or tuple_size(val) == 0, do: true
  def empty?(_val), do: false
end

Check.empty?(%{})
#=> true
Check.empty?({})
#=> false   # true was expected!
```

Fix with separate `when` clauses so each alternative is isolated:

```elixir
defmodule Check do
  def empty?(val)
      when map_size(val) == 0
      when tuple_size(val) == 0,
      do: true
  def empty?(_val), do: false
end

Check.empty?(%{})
#=> true
Check.empty?({})
#=> true
```

### Allowed guard expressions (and why they are restricted)

From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "Not all expressions are allowed in guard clauses, but only a handful of them. This is a deliberate choice. This way, Elixir (through Erlang) ensures that all guards are predictable (no mutations or other side-effects) and they can be optimized and performed efficiently."

Consequence: arbitrary user functions CANNOT be called in guards.

Allowed categories:

- Comparison operators: `==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=`, plus `max/2`, `min/2`.
- Strictly boolean operators: `and`, `or`, `not` (NOT `&&`/`||`/`!`).
- Arithmetic: unary `+`/`-`, binary `+` `-` `*` `/`.
- Membership: `in`, `not in` (RHS must be a Range or a literal list).
- The `map.field` syntax.
- Type-check predicates and the guard-safe BIFs below.

Guard-safe BIF quick-reference (from [Kernel.html](https://hexdocs.pm/elixir/Kernel.html)):

| Category | Guard-safe functions |
|---|---|
| Type checks | `is_atom/1`, `is_binary/1`, `is_bitstring/1`, `is_boolean/1`, `is_float/1`, `is_function/1`, `is_function/2`, `is_integer/1`, `is_list/1`, `is_map/1`, `is_map_key/2`, `is_nil/1`, `is_non_struct_map/1`, `is_number/1`, `is_pid/1`, `is_port/1`, `is_reference/1`, `is_struct/1`, `is_struct/2`, `is_tuple/1`, `is_exception/1`, `is_exception/2` |
| Sizes / access | `byte_size/1`, `bit_size/1`, `length/1`, `map_size/1`, `tuple_size/1`, `element/2`, `hd/1`, `tl/1`, `binary_part/3` |
| Numeric | `abs/1`, `ceil/1`, `floor/1`, `round/1`, `trunc/1`, `div/2`, `rem/2` |
| Other | `self/0`, `node/0`, `node/1` |

`ceil/1`/`floor/1` are guard-safe since v1.8; `is_exception/1,2` since v1.11. The `Bitwise` module additionally exposes guard-safe bitwise operators when imported.

### `in` and `not in` in guards

`when x in 1..1000` expands to a bounded comparison (effectively `x >= 1 and x <= 1000`) and is efficient.

`when x in [1, 2, 3]` expands to `x === 1 or x === 2 or x === 3` and is only efficient for short lists.

Cross-reference the `Concatenation and membership` subsection in `## Basic Operators`.

### Custom guards: `defguard` / `defguardp`

`defguard` defines a PUBLIC, importable guard; `defguardp` defines a PRIVATE guard (analogous to `def`/`defp`). From [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html):

> "it's recommended to define them using defguard/1 and defguardp/1 which perform additional compile-time checks."

```elixir
defmodule MyInteger do
  defguard is_even(term) when is_integer(term) and rem(term, 2) == 0
end

import MyInteger, only: [is_even: 1]

defmodule Label do
  def label(number) when is_even(number), do: :even
end
```

The body must itself be a valid guard expression; otherwise it is a compile error. Macros composed entirely of allowed guard expressions (e.g. `Integer.is_even/1`) are also valid in guards.

For deeper coverage of `case`, `cond`, `with`, and `receive`, see the `## Control Flow` section; for multi-clause functions and default arguments, see the `## Modules and Functions` section.

## Control Flow

Elixir is expression-oriented: every construct returns a value, so there are no "statements." Control flow is built on pattern matching first (`case/2`, `with/1`, function clauses), with truthy/falsy dispatch (`if/2`, `unless/2`, `cond/1`) for logic that cannot be expressed as patterns or guards. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "In Elixir, there are only expressions, no statements. Everything you write in Elixir language returns some value."

`case/2`, `cond/1`, and `with/1` are special forms in `Kernel.SpecialForms`; `if/2`, `unless/2`, and `match?/2` are macros in `Kernel`. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "`if` is implemented as a macro in the language: it isn't a special language construct as it would be in many languages."

Cross-reference the `## Pattern Matching and Guards` → `Where patterns and guards can be used` table, which already lists which of these constructs accept patterns and guards.

### Truthy and falsy values

`cond/1`, `if/2`, and `unless/2` all branch on truthiness rather than on `true`/`false` strictly. From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> "a value is truthy when it is neither false nor nil. A value is falsy when it is either false or nil."

Only `false` and `nil` are falsy; everything else — including `0`, `0.0`, `""`, `[]`, and atoms like `:error` — is truthy. This is the single most common trap for developers coming from languages where `0` or `""` are falsy.

```elixir
iex> !!0
true
iex> !!""
true
iex> !![]
true
iex> !!nil
false
iex> !!false
false
```

Cross-reference the `Booleans` and `nil` subsections in `## Basic Types` and the `Boolean operators: strict vs truthy` subsection in `## Basic Operators`. Contrast with guards, which have no truthy/falsy concept — a guard runs only when it evaluates to literally `true` (see `## Pattern Matching and Guards` → `Guards are strictly boolean`).

### `case/2` — matching a value against clauses

`case/2` matches a single expression against a series of `pattern -> body` clauses, taking the first clause whose pattern (and optional guard) matches. From [Kernel.SpecialForms.html#case/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#case/2):

> "Matches the given expression against the given clauses."

> "`case/2` relies on pattern matching and guards to choose which clause to execute. If your logic cannot be expressed within patterns and guards, consider using `if/2` or `cond/1` instead."

```elixir
iex> case {1, 2, 3} do
...>   {4, 5, 6} -> "won't match"
...>   {1, x, 3} -> "matches and binds x to 2"
...>   _ -> "matches any value"
...> end
"matches and binds x to 2"
```

To match a clause against the value of an EXISTING variable rather than bind a new one, use the pin operator `^`. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "If you want to pattern match against an existing variable, you need to use the `^/1` operator"

Cross-reference `## Pattern Matching and Guards` → `The pin operator ^`.

Guards augment each clause with a `when` check:

```elixir
iex> case {1, 2, 3} do
...>   {1, x, 3} when x > 0 -> "x is positive"
...>   {1, x, 3} -> "x is non-positive"
...> end
"x is positive"
```

Errors raised inside a guard do NOT propagate — they make that clause fail and the next clause is tried. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "Keep in mind errors in guards do not leak but simply make the guard fail"

```elixir
iex> case 1 do
...>   x when hd(x) -> "won't match: hd/1 raises on an integer, so the guard fails"
...>   x -> "Got #{x}"
...> end
"Got 1"
```

Cross-reference `## Pattern Matching and Guards` → `Errors in guards become failures, not exceptions`.

**Variable scoping:** bindings introduced inside a clause are local to that clause and do NOT leak to the outer scope. From [Kernel.SpecialForms.html#case/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#case/2):

> "variables bound in a clause do not leak to the outer context"

```elixir
iex> x = 1
1
iex> case true do
...>   true -> x = x + 1
...> end
2
iex> x
1
```

To carry a computed value out, capture the return value of the whole `case` expression: `x = case ... do ... end`.

**No matching clause raises `CaseClauseError`.** From [CaseClauseError.html](https://hexdocs.pm/elixir/CaseClauseError.html):

> "An exception raised when a term in a `case/2` expression does not match any of the defined `->` clauses."

A `_ ->` fallback clause is the idiomatic way to guarantee full coverage.

**Nested `case` is a smell.** Two or more levels of nested `case` usually signal that the inner cases should collapse into a `with/1` chain. From [Kernel.SpecialForms.html#with/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#with/1):

> "If you find yourself nesting `case` expressions inside `case` expressions, consider using `with/1`."

### `cond/1` — first truthy condition

`cond/1` evaluates condition expressions top to bottom and runs the body of the FIRST clause whose condition is truthy. It is NOT pattern matching — clause heads are arbitrary boolean expressions. From [Kernel.SpecialForms.html#cond/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#cond/1):

> "Evaluates the expression corresponding to the first clause that evaluates to a truthy value."

```elixir
iex> cond do
...>   2 + 2 == 5 -> "no"
...>   2 * 2 == 3 -> "nor this"
...>   1 + 1 == 2 -> "yes"
...> end
"yes"
```

Because only `false` and `nil` are falsy, conditions like `hd([1, 2, 3])` or `[]` are truthy and WILL match. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "Similar to `if/2`, `cond/1` considers any value besides `nil` and `false` to be true"

If every condition is falsy, `cond/1` raises `CondClauseError`. The idiom is to end with a final `true ->` "else" clause:

```elixir
iex> cond do
...>   1 + 1 == 1 -> "never"
...>   2 * 2 != 4 -> "never"
...>   true -> "fallback"
...> end
"fallback"
```

From [CondClauseError.html](https://hexdocs.pm/elixir/CondClauseError.html):

> "An exception raised when no clauses in a `cond/1` expression evaluate to a truthy value."

`cond/1` is equivalent to an `else if` chain in imperative languages, but is used sparingly in Elixir. From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "If your `cond` has two clauses, and the last one falls back to `true`, you may consider using `if/2` instead."

### `if/2` and `unless/2`

`if/2` runs its `do` block when the condition is truthy; otherwise it returns `nil` (or runs the `else` block if present). From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "If the condition given to `if/2` returns `false` or `nil`, the body given between `do`-`end` is not executed and instead it returns `nil`."

```elixir
iex> if true do
...>   "works"
...> end
"works"
iex> if nil do
...>   "no"
...> else
...>   "yes"
...> end
"yes"
iex> if nil, do: "no", else: "yes"
"yes"
```

`if(condition, do: x)` / `if(condition, do: x, else: y)` is the inline keyword-list form — `do:` and `else:` are keyword-list keys, which is why `if` is a macro taking a keyword list rather than a special statement syntax. Because `if` is a regular macro in `Kernel`, a module can replace it via `import Kernel, except: [if: 2]`.

Like `case`, bindings inside an `if` block do not leak; capture the return value to carry a result out:

```elixir
iex> x = 1
1
iex> x = if true, do: x + 1, else: x
2
iex> x
2
```

`unless/2` is the negation form: `unless cond` is equivalent to `if !cond`. It is conventionally used WITHOUT an `else` block for single-branch "negative guard" cases (e.g. `unless logged_in?(user), do: redirect(conn)`). As of Elixir v1.20 the official hexdocs mark `Kernel.unless/2` as **soft-deprecated** (a documentation-level `@doc deprecated:` marker), steering new code toward `if !cond`; it is NOT hard-deprecated — there is no compiler warning and existing call sites remain fully supported. If a two-branch form is needed, prefer `if/2` over `unless ... do ... else ...`, whose negated condition is hard to read.

### `with/1` — chaining pattern matches

`with/1` sequences `pattern <- expression` steps, short-circuiting at the first step whose pattern does not match. It replaces deeply nested `case` expressions. From [Kernel.SpecialForms.html#with/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#with/1):

> "Combine matching clauses."

> "Consider `<-` as a sibling to `=`, except that, while `=` raises in case of not matches, `<-` will simply abort the `with` chain and return the non-matched value."

```elixir
iex> opts = %{width: 10, height: 5}
%{height: 5, width: 10}
iex> with {:ok, width} <- Map.fetch(opts, :width),
...>      {:ok, height} <- Map.fetch(opts, :height) do
...>   {:ok, width * height}
...> end
{:ok, 50}
```

If every step matches, the `do` block runs and its value is returned. If any step fails, the chain ABORTS and the non-matching value is returned unchanged (no exception):

```elixir
iex> opts = %{width: 10}
%{width: 10}
iex> with {:ok, width} <- Map.fetch(opts, :width),
...>      {:ok, height} <- Map.fetch(opts, :height) do
...>   {:ok, width * height}
...> end
:error
```

(`Map.fetch(%{width: 10}, :height)` returned `:error`, which failed the `{:ok, height}` pattern, so the chain aborts with `:error`.)

**The `<-` vs `=` distinction is the central `with` gotcha.** A bare `=` inside `with` is a normal match: on failure it RAISES `MatchError`; it does NOT short-circuit. From [Kernel.SpecialForms.html#with/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#with/1):

> "The behavior of any expression in a clause is the same as if it was written outside of `with`. For example, `=` will raise a `MatchError` instead of returning the non-matched value."

```elixir
iex> with :foo = :bar, do: :ok
** (MatchError) no match of right hand side value: :bar
```

Bare `=` is useful for binding an intermediate value that is GUARANTEED to match (a plain assignment within the chain), e.g. `double = width * 2`. Use `<-` whenever a mismatch should short-circuit the chain.

**Bindings are local to `with`.** Variables bound inside a `with` (such as `width` above) are not accessible outside it, and are NOT available inside the `else` block. From [Kernel.SpecialForms.html#with/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#with/1):

> "As in `for/1`, variables bound inside `with/1` won't be accessible outside of `with/1`."

**The `else` clause** transforms the value returned on a failed match. It behaves like a `case` over the non-matching value, with its own pattern/guard clauses:

```elixir
iex> with {:ok, width} <- Map.fetch(opts, :width),
...>      {:ok, height} <- Map.fetch(opts, :height) do
...>   {:ok, width * height}
...> else
...>   :error -> {:error, :missing_field}
...>   other -> other
...> end
```

Without an `else`, the failed value is returned AS-IS. With an `else` in which NO clause matches, `WithClauseError` is raised. From [WithClauseError.html](https://hexdocs.pm/elixir/WithClauseError.html):

> "An exception raised when a term in a `with/1` expression does not match any of the defined `->` clauses in its `else`."

**Canonical refactor — nested `case` → `with`.** A sequence of operations that each return `{:ok, _} | {:error, _}` is the canonical `with` use case:

```elixir
# nested case (avoid):
case read_line(socket) do
  {:ok, data} ->
    case parse(data) do
      {:ok, command} -> run(command)
      {:error, _} = err -> err
    end
  {:error, _} = err -> err
end

# with (prefer):
with {:ok, data} <- read_line(socket),
     {:ok, command} <- parse(data) do
  run(command)
end
```

**The "flattened failures" pitfall.** When different steps can fail with different non-matching values, all failures land in a single `else` block, and the `else` must distinguish them from the value alone (it has no access to which step ran). From [Kernel.SpecialForms.html#with/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#with/1):

> "one of potential drawback of `with` is that all failure clauses are flattened into a single `else` block"

```elixir
with ".ex" <- Path.extname(path),
     true <- File.exists?(path) do
  copy(path)
  {:ok, path}
else
  ext when is_binary(ext) -> {:error, :invalid_extension}
  false -> {:error, :missing_file}
end
```

When failures are heterogeneous, prefer tagging them upstream (e.g. always `{:error, reason}`) so the `else` block stays simple and unambiguous.

### When to use which

From [case-cond-and-if.html](https://hexdocs.pm/elixir/case-cond-and-if.html):

> "Elixir developers prefer pattern matching and guards, using `case/2` and function definitions [...] as they are succinct and precise. When your logic cannot be outlined within patterns and guards, you may consider `if/2`, falling back to `cond/1` when there are several conditions to check."

| Need | Choose | Notes |
|---|---|---|
| Dispatch on function arguments | Function-head pattern matching | Preferred over `case` inside a function body. See `## Modules and Functions`. |
| Destructure / branch on ONE value's shape | `case/2` | Tuples, maps, structs, with optional guards. |
| Chain 2+ matching steps with early failure | `with/1` | Replaces nested `case`; each step returns `{:ok, _}` / `{:error, _}` or similar. |
| Several unrelated boolean conditions | `cond/1` | First truthy wins; end with a `true ->` fallback. |
| A single boolean condition | `if/2` | `unless/2` for a single negative guard (soft-deprecated; prefer `if !cond`). |

Rule of thumb: reach for pattern matching (`case`, `with`, function clauses) first; use truthy/falsy forms (`if`, `cond`) only when the logic genuinely cannot be expressed as patterns and guards. Match on the SHAPE of data with `case`/`with`; branch on computed booleans with `if`/`cond`.

### `match?/2` — a guard-less pattern predicate

`match?/2` tests whether an expression matches a pattern, returning `true` or `false` WITHOUT binding variables. From [Kernel.html#match?/2](https://hexdocs.pm/elixir/Kernel.html#match?/2):

> "A convenience macro that checks if the result of `expression` matches `pattern`."

```elixir
iex> match?({:ok, _}, File.read("README.md"))
true
iex> value = {:ok, 1}
{:ok, 1}
iex> match?({:ok, _}, value)
true
iex> value
{:ok, 1}
```

Because it is a macro rather than a guard, it accepts patterns richer than guards allow. Use it as a predicate in `if`/`cond` or in tests when only a boolean is needed, not destructured bindings. When the bound values are needed, use `case`/`with` instead.

For multi-clause functions, default arguments, and `&`/capture syntax, see the `## Modules and Functions` section.

## Strings, Binaries, and Charlists

The `### Strings (binaries)` and `### Charlists (intro only)` subsections of `## Basic Types` introduced the literals and predicates; this section goes deep on the underlying byte representation, the `<<>>` constructor, binary pattern matching, the `String` vs `:binary` module split, graphemes, and iodata performance. From [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html):

> "In this chapter, we will gain clarity on how Elixir handles strings and binaries, what a charlist is, and how they relate to one another."

### Strings are UTF-8 encoded binaries

A string is a binary whose bytes form valid UTF-8. A *code point* is **what** is stored (a Unicode scalar value, `0..0x10FFFF`); UTF-8 is **how** it is stored — a variable-width encoding using 1 to 4 bytes per code point. From [String.html](https://hexdocs.pm/elixir/String.html):

> "Elixir strings are UTF-8 encoded binaries."

The typespec alias `String.t()` and `binary()` are the SAME type at runtime (`String.t()` is literally `binary()`); `String.t()` exists only to *document* the intent that the bytes are valid UTF-8. From [String.html](https://hexdocs.pm/elixir/String.html):

> "The types String.t() and binary() are equivalent to analysis tools. Although, for those reading the documentation, String.t() implies it is a UTF-8 encoded binary."

The headline consequence: byte count ≠ character count. There are three distinct counters:

| Function | Counts | Complexity |
|---|---|---|
| `byte_size/1` | raw bytes | O(1) |
| `String.codepoints/1 \|> length/1` | Unicode code points | O(n) |
| `String.length/1` | grapheme clusters (perceived characters) | O(n) |

```elixir
iex> string = "héllo"
"héllo"
iex> String.length(string)      # graphemes
5
iex> byte_size(string)          # bytes ('é' is 2 bytes: 195, 169)
6
iex> String.codepoints(string)
["h", "é", "l", "l", "o"]
```

To see the raw bytes directly, append a sentinel or inspect as binaries:

```elixir
iex> "hełło" <> <<0>>
<<104, 101, 197, 130, 197, 130, 111, 0>>
```

`byte_size/1` is constant time; `String.length/1` is linear because it must walk the string honoring Unicode grapheme boundaries. A binary may NOT be a valid string:

```elixir
iex> is_binary(<<239, 191, 19>>)
true
iex> String.valid?(<<239, 191, 19>>)
false
```

### Bitstrings vs binaries

A **bitstring** is a contiguous sequence of bits, built with the `<<>>` special form. A **binary** is a bitstring whose total bit count is divisible by 8. Every binary is a bitstring; not every bitstring is a binary. From [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html):

> "A binary is a bitstring where the number of bits is divisible by 8."

Each segment inside `<<>>` defaults to an 8-bit integer, which is why `<<42>>` and `<<42::8>>` are identical:

```elixir
iex> <<1, 2, 3>>
<<1, 2, 3>>
iex> <<42>> == <<42::8>>
true
iex> is_bitstring(<<3::4>>)      # 4 bits → bitstring but NOT a binary
true
iex> is_binary(<<3::4>>)
false
iex> is_binary(<<0, 255, 42>>)   # 24 bits → a binary
true
```

A value that does not fit its segment size is **truncated**:

```elixir
iex> <<1>> == <<257>>
true
```

(257 in binary is `100000001`; only the low 8 bits `00000001` are kept.)

The **empty** `<<>>` has 0 bits, and 0 is divisible by 8, so it is both a binary and a bitstring:

```elixir
iex> is_binary(<<>>)
true
iex> is_bitstring(<<>>)
true
iex> byte_size(<<>>)
0
```

`bit_size/1` returns the exact bit count; `byte_size/1` rounds up to the next whole byte:

```elixir
iex> bit_size(<<433::16, 3::3>>)
19
iex> byte_size(<<433::16, 3::3>>)
3
```

### The `<<>>` constructor: types, size, and unit

A `<<>>` segment has a *type*, a *size*, a *unit*, plus modifiers (signedness, endianness). From [Kernel.SpecialForms.html#<</1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#%3C%3C%3E%3E/1):

> "Defines a new bitstring."

Allowed segment types: `integer` (default), `float`, `bits`/`bitstring`, `binary`/`bytes`, `utf8`, `utf16`, `utf32`. Defaults: integer segments are 8 bits; floats 64 bits; a `binary` segment consumes the whole remainder. A literal string inside `<<>>` expands to its bytes:

```elixir
iex> <<0, "foo">>
<<0, 102, 111, 111>>
```

`size(n)` (or the shorthand `n`) sets the number of *units*; `unit(k)` sets the unit width. `<<x::size(8)>>` is `<<x::8>>`; `<<x::8*4>>` is shorthand for `<<x::size(8)-unit(4)>>` (32 bits). Integer/float/utf16/utf32 segments also take `signed`/`unsigned` (default `unsigned`) and `big`/`little`/`native` endianness (default `big`). Default `unsigned-big` is why `<<0, 1>>` reads as `1` big-endian but `256` little-endian.

### Binary pattern matching

Binaries pattern-match anywhere patterns are legal (`=`, `case`, function heads, `with`, `receive`). Cross-reference `## Pattern Matching and Guards` → `Pattern matching on strings and binaries`. A bare segment matches one byte; `::binary` matches the open-ended remainder; `binary-size(n)` matches exactly `n` bytes:

```elixir
iex> <<0, 1, x>> = <<0, 1, 2>>
<<0, 1, 2>>
iex> x
2
iex> <<0, 1, x>> = <<0, 1, 2, 3>>
** (MatchError) no match of right hand side value: <<0, 1, 2, 3>>
iex> <<head::binary-size(2), rest::binary>> = <<0, 1, 2, 3>>
<<0, 1, 2, 3>>
iex> head
<<0, 1>>
iex> rest
<<2, 3>>
```

**Constraint:** only the LAST `::binary` segment in a pattern may omit its size; all others must use `binary-size(n)`, or the compiler raises:

```
** (CompileError) a binary field without size is only allowed at the end of a binary pattern
```

**The `::utf8` modifier is the canonical way to walk a string** — it decodes one UTF-8 code point into an integer and binds the remainder as a binary:

```elixir
iex> <<head::utf8, rest::binary>> = "banana"
"banana"
iex> head == ?b
true
iex> rest
"anana"
```

**Gotcha — byte patterns silently split multi-byte characters.** Without `::utf8`, a segment grabs a raw byte and may cut into the middle of a code point:

```elixir
iex> <<x, rest::binary>> = "über"     # WRONG for text: x is a byte, not a code point
"über"
iex> x == ?ü                          # ?ü is 252; x is the byte 195
false
iex> rest
<<188, 98, 101, 114>>                 # garbled bytes

iex> <<x::utf8, rest::binary>> = "über"   # RIGHT: x is the code point
"über"
iex> x == ?ü
true
iex> rest
"ber"
```

Byte-level matching is exactly right for parsing binary protocols. The classic example — dispatch on a file's magic bytes in a function head:

```elixir
defmodule ImageType do
  @png_signature <<137, 80, 78, 71, 13, 10, 26, 10>>
  @jpg_signature <<255, 216>>

  def type(<<@png_signature, _rest::binary>>), do: :png
  def type(<<@jpg_signature, _rest::binary>>), do: :jpg
  def type(_), do: :unknown
end

ImageType.type(<<137, 80, 78, 71, 13, 10, 26, 10, 0, 0>>)
#=> :png
```

### Charlists are lists of code points

A **charlist is a list of integers where all the integers are valid code points.** The preferred sigil is `~c"..."`; the `'...'` literal is soft-deprecated since v1.15. From [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html):

> "A charlist is a list of integers where all the integers are valid code points."

The `?` prefix returns the integer code point of a character literal:

```elixir
iex> ?a
97
iex> ?ł
322
iex> ~c"hello"
~c"hello"
iex> ~c"hello" == [?h, ?e, ?l, ?l, ?o]
true
iex> is_list(~c"hello")
true
```

When a code point is outside ASCII, IEx prints the list as raw integers rather than as a charlist:

```elixir
iex> ~c"hełło"
[104, 101, 322, 322, 111]
```

**IEx display gotcha:** IEx renders any list of integers in the ASCII range as a charlist, which can mislead you about the real type. Force a list view with `inspect/2`:

```elixir
iex> heartbeats_per_minute = [99, 97, 116]
~c"cat"
iex> inspect(heartbeats_per_minute, charlists: :as_list)
"[99, 97, 116]"
```

**Conversion** is polymorphic via the `List.Chars` and `String.Chars` protocols:

```elixir
iex> to_charlist("hełło")
[104, 101, 322, 322, 111]
iex> to_string(~c"hełło")
"hełło"
iex> to_string(:hello)
"hello"
iex> to_string(1)
"1"
```

`to_string/1` and `to_charlist/1` are the general boundaries; `String.to_charlist/1` and `String.to_string/1` are the string-specific helpers.

### String vs charlist: which to choose

| | `"..."` (string) | `'...'` / `~c"..."` (charlist) |
|---|---|---|
| Type | binary (UTF-8) | list of codepoint integers |
| Concatenation | `<>` | `++` |
| Byte cost | compact (1–4 bytes/char) | one cons cell per character |
| Length | `byte_size/1` / `String.length/1` | `length/1` |
| Typical use | all text in Elixir | Erlang interop |

The two are entirely different types and never compare equal:

```elixir
iex> 'hello' === "hello"
false
iex> 'hello' == "hello"
false
```

Using the wrong concatenator raises:

```elixir
iex> ~c"this " <> ~c"fails"
** (ArgumentError) expected binary argument in <> operator but got: ~c"this "
iex> "he" ++ "llo"
** (ArgumentError) argument error
```

**Guidance:** default to strings (`"..."`) for all text. Reach for charlists only at an Erlang/legacy API boundary (e.g. `:gen_tcp` options, `:logger` formatters, some `Regex` options) and convert at the edge with `to_charlist/1`. Cross-reference the `Charlists (intro only)` subsection in `## Basic Types`.

### Heredocs and string interpolation

**String heredocs** use triple double-quotes — the standard form for multi-line `@doc`/`@moduledoc`:

```elixir
@doc """
Returns a greeting.

## Examples

    iex> greet("world")
    "Hello, world!"
"""
def greet(name), do: "Hello, #{name}!"
```

**Charlist heredocs** use triple single-quotes, and the `~c`/`~s` sigils also accept the heredoc form:

```elixir
iex> ~c'''
...> this is
...> a heredoc charlist
'''
~c"this is\na heredoc charlist\n"
```

**String interpolation** uses `#{ ... }`; any expression is allowed and non-strings are converted via the `String.Chars` protocol:

```elixir
iex> name = "joe"
iex> "hello #{name}"
"hello joe"
iex> "2 + 2 = #{2 + 2}"
"2 + 2 = 4"
```

**Escape sequences** (from [String.html](https://hexdocs.pm/elixir/String.html)):

| Escape | Meaning |
|---|---|
| `\0` `\a` `\b` `\t` `\n` `\v` `\f` `\r` `\e` `\s` | control chars (null, bell, backspace, tab, newline, vtab, formfeed, CR, escape, space) |
| `\#` | literal `#` (suppresses interpolation) |
| `\\` `\"` `\'` | literal backslash / quote |
| `\uNNNN` | Unicode code point, 4 hex digits (preferred) |
| `\u{NNNNNN}` | Unicode code point, 1–6 hex digits |
| `\xNN` | single byte (hex) — AVOID; can produce invalid UTF-8 |

From [String.html](https://hexdocs.pm/elixir/String.html):

> "we recommend `\uNNNN` instead, as `\xNN` represents a single byte and it may result in an invalid string."

**Uppercase sigils** (`~S`, `~C`) disable both interpolation and escape processing; lowercase sigils (`~s`, `~c`) apply them. `~S"""..."""` is the idiomatic choice for documentation that contains backslashes or literal `#{` snippets:

```elixir
iex> ~S(String without escapes \x26 without #{interpolation})
"String without escapes \\x26 without \#{interpolation}"
```

Cross-reference the `## Sigils` section.

### The String module vs the `:binary` module

The `String` module works on **UTF-8** (code points and graphemes). Erlang's `:binary` module works on **raw bytes**. The String module's own docs steer byte-level work to `:binary` and a few guard-safe BIFs. From [String.html](https://hexdocs.pm/elixir/String.html):

> "For low-level operations that work directly with binaries, Elixir also provides the `:binary` module."

Common **`String`** functions:

| Function | Note |
|---|---|
| `String.length/1` | grapheme count (linear) |
| `String.codepoints/1` / `String.graphemes/1` | code points vs graphemes |
| `String.at/2`, `String.slice/2,3` | grapheme-position access |
| `String.split/1,3`, `String.replace/3,4` | split / replace |
| `String.upcase/2` / `downcase/2` / `capitalize/2` | modes `:default`/`:ascii`/`:greek`/`:turkic` |
| `String.trim/1,2` | whitespace or given graphemes |
| `String.starts_with?/2`, `ends_with?/2`, `contains?/2` | membership |
| `String.valid?/1` | true if valid UTF-8 |
| `String.normalize/2` | `:nfd`/`:nfc`/`:nfkd`/`:nfkc` |
| `String.next_grapheme/1`, `next_codepoint/1` | streaming decomposition |

Common **`:binary`** (Erlang) functions:

| Function | Note |
|---|---|
| `:binary.split/2` | split on a byte pattern |
| `:binary.copy/2` | repeat a binary N times |
| `:binary.compile_pattern/1` | pre-compile a search pattern |
| `:binary.part/3` | byte-range slice (≈ `binary_part/3`) |
| `:binary.at/2`, `:binary.first/1`, `:binary.last/1` | byte access (O(1)) |

For repeated searches on the same set of substrings, compile a pattern once:

```elixir
iex> pattern = :binary.compile_pattern([" ", "!"])
iex> String.split("foo bar!", pattern)
["foo", "bar", ""]
```

A compiled pattern is a runtime value and CANNOT be stored in a module attribute.

**Performance trade-off:** `:binary` is faster for byte-oriented work but ignores UTF-8 boundaries — using it on multi-byte content can split a code point and yield an invalid string. Default to `String.*` for text; drop to `:binary` / `binary_part/3` / `binary_slice/3` only for guaranteed byte-safe data. UTF-8 is self-synchronizing, so an invalid sequence corrupts at most one code point; `String.replace_invalid/2` substitutes `"�"` for malformed bytes.

### Graphemes vs code points

A **code point** is one Unicode scalar value. A **grapheme** is one or more code points that together form a single perceived character. The canonical example — `"é"` may be one code point (`\u00E9`) or two (`e` + combining acute `\u0301`):

```elixir
iex> string = "\u0065\u0301"   # e + combining acute
"é"
iex> byte_size(string)
3
iex> String.length(string)      # one grapheme
1
iex> String.codepoints(string)
["e", "́"]
iex> String.graphemes(string)
["é"]
```

Emoji can be several code points but one grapheme (woman + ZWJ + fire engine = woman firefighter):

```elixir
iex> String.codepoints("👩‍🚒")
["👩", "‍", "🚒"]
iex> String.graphemes("👩‍🚒")
["👩‍🚒"]
iex> String.length("👩‍🚒")
1
```

Grapheme segmentation follows [Unicode Standard Annex #29](https://www.unicode.org/reports/tr29/) (Extended Grapheme Cluster). It is NOT locale-aware, so a perceived unit like `"ch"` is two graphemes. Prefer `String.length/1` (graphemes) for "character count" shown to users; use `String.codepoints/1` only when you specifically need code points.

### Binary and iodata performance

Each `<>` copies BOTH operands into a new binary. Concatenating in a loop is O(n²) in total size. From [IO.html](https://hexdocs.pm/elixir/IO.html):

> "each concatenation operation will copy both binaries to a new one."

The fix is **IO data** (`iodata`): a possibly-nested list of bytes (`0..255`) and binaries, assembled with cheap cons cells (O(1) per append) and flattened once at the boundary. The recursive type is `iodata = binary | [byte | iodata | iolist]`.

```elixir
# Naive (quadratic) — each <> copies:
def email(username, domain), do: username <> "@" <> domain

# iodata (cheap) — cons cells, no copies:
def email(username, domain), do: [username, ?@, domain]
```

Materialize iodata to a binary at the edge, or hand it directly to an iodata-accepting function (`IO.write/2`, `:gen_tcp.send/2`, `File.write!/3`) that flattens in C:

```elixir
iex> IO.iodata_to_binary([<<"foo">>, "bar"])
"foobar"
iex> IO.iodata_length([<<"foo">>, "bar"])
6
```

`IO.iodata_to_binary/1` treats integers as raw bytes and is Unicode-UNSAFE; `IO.chardata_to_string/1` is the codepoint-safe counterpart for **chardata** (iodata whose integers are code points, not bytes). `Enum.join/2` already builds iodata internally, so it is the idiomatic way to join many pieces.

**Drawback:** iodata is opaque — you cannot `<<head, rest::binary>>` it. Convert to a binary first when you need to pattern match. To see the compiler's binary optimizations, compile with `ERL_COMPILER_OPTIONS=bin_opt_info mix compile`. Cross-reference the pipe operator `|>` in `## Basic Operators` — piping does not avoid the copies; only iodata does.

### Choosing the right tool (summary)

| Need | Use |
|---|---|
| Text you store / create | `"..."` string (binary) |
| Walk a string character by character | `<<cp::utf8, rest::binary>>` or `String.next_grapheme/1` |
| Parse a binary protocol / magic bytes | `<<>>` byte segments with `size`/`signed`/endian |
| "Character count" for users | `String.length/1` (graphemes) |
| Raw byte count | `byte_size/1` (O(1)) |
| Erlang API requiring charlists | `~c"..."` + `to_charlist/1` at the boundary |
| Assemble many pieces efficiently | iodata (`[...]`) + `IO.iodata_to_binary/1` at the edge |
| Byte-level slicing on known-safe data | `binary_part/3` / `binary_slice/3` / `:binary` |

## Keywords and Maps

The `### Maps (intro only)` and `### Keyword lists (intro only)` subsections of `## Basic Types` introduced the literals, access syntax, and predicates; this section goes deep on the three keyword-list characteristics, the dual nature of map access, the `Map`/`Keyword` modules, the `Access` behaviour for nested data, and struct maps. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "As the name implies, keyword lists are simply lists. In particular, they are lists consisting of 2-item tuples where the first element (the key) is an atom and the second element can be any value."

### Keyword lists

A keyword list is a list of 2-tuples whose first element (the key) is an atom. Elixir provides the `[key: value]` shorthand, which is identical to the explicit `[{:key, value}]` form:

```elixir
iex> [{:parts, 3}, {:trim, true}] == [parts: 3, trim: true]
true
```

A key may be any atom; if it needs characters outside letters, digits, `_`, or `@`, wrap it in quotes (it remains an atom, never a string):

```elixir
iex> ["exit on close": true]
[:"exit on close": true]
```

The three characteristics that distinguish keyword lists from maps, quoted verbatim from [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Keyword lists are important because they have three special characteristics:
> - Keys must be atoms.
> - Keys are ordered, as specified by the developer.
> - Keys can be given more than once."

Because they are lists, all list operations apply and the same linear performance characteristics hold. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "keyword lists are simply lists, and as such they provide the same linear performance characteristics: the longer the list, the longer it will take to find a key, to count the number of items, and so on."

Access uses bracket syntax (the `Access` behaviour), returning the value of the FIRST matching key:

```elixir
iex> list = [a: 1, b: 2]
[a: 1, b: 2]
iex> list[:a]
1
iex> list[:c]
nil
```

Predicate: `is_list/1` (there is no distinct `is_keyword/1`); `length/1` is linear. Cross-reference the `Lists` and `Atoms` subsections in `## Basic Types`.

### Keyword lists as function options

The idiomatic use of keyword lists is optional function arguments. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "keyword lists are mostly used as optional arguments to functions."

When a keyword list is the LAST argument, its brackets may be omitted:

```elixir
iex> String.split("1,2,3,4", ",", trim: true)
["1", "2", "3", "4"]
iex> String.split("1,2,3,4", ",", [trim: true])   # equivalent, explicit brackets
["1", "2", "3", "4"]
```

This same "trailing keyword list" convenience also applies inside tuples, lists, and maps:

```elixir
iex> {1, 2, foo: :bar}
{1, 2, [{:foo, :bar}]}
iex> [1, 2, foo: :bar]
[1, 2, {:foo, :bar}]
iex> %{1 => 2, foo: :bar}
%{1 => 2, :foo => :bar}
```

The `do`/`end` blocks of macros such as `if/2`, `case/2`, and `def` are themselves keyword lists in disguise — `if(true, do: "yes", else: "no")` is the keyword-list form of `if true do "yes" else "no" end`. Cross-reference the `if/2` and `unless/2` subsection in `## Control Flow`.

### Duplicate keys in keyword lists

Because keys may repeat, bracket access returns only the FIRST match. Use `Keyword.get_values/2` to retrieve all of them:

```elixir
iex> list = [a: 1, b: 2, a: 3]
[a: 1, b: 2, a: 3]
iex> list[:a]
1
iex> Keyword.get_values(list, :a)
[1, 3]
```

Duplicate keys are common in options that can be given repeatedly, such as multi-clause joins or repeated headers. `Keyword.put/3` REPLACES every existing entry for a key (it deduplicates), while `Keyword.delete/2` removes ALL of them — mirror the bracket-access behavior, which only ever surfaces the first value.

### The `Keyword` module

Key functions, all linear-time, from [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

| Function | Behavior |
|---|---|
| `Keyword.get/3` | First value for key, or default (`nil` if absent). |
| `Keyword.get_values/2` | All values for a duplicate key (list, possibly empty). |
| `Keyword.fetch/2` | `{:ok, value}` or `:error`. |
| `Keyword.fetch!/2` | Value, or raises `KeyError`. |
| `Keyword.put/3` | Replaces ALL entries for key (deduplicates). |
| `Keyword.put_new/3` | Puts only if the key is absent. |
| `Keyword.delete/2` | Removes ALL entries for key. |
| `Keyword.delete_first/2` | Removes only the first entry for key. |
| `Keyword.has_key?/2` | Presence test. |
| `Keyword.keys/1` / `values/1` | Lists of keys / values (duplicates kept). |
| `Keyword.merge/2` | Concatenates; the later list's keys win on conflict. |
| `Keyword.pop/3` | Removes ALL entries for key, returns `{first_value, rest}`. |
| `Keyword.pop_first/3` | Removes only the first entry for key. |
| `Keyword.split/2` / `take/2` / `drop/2` | Partition / keep / remove a set of keys. |
| `Keyword.validate/2` | `{:ok, kw}` or `{:error, invalid_keys}` (since v1.13). |

Typespec name: `keyword()` ≡ `[{atom(), any()}]`; `keyword(t)` ≡ `[{atom(), t}]`. Note that although list order is preserved, `Keyword` functions do not guarantee WHERE a newly-added key lands — `Keyword.put/3` may prepend, append, or insert — which is a second reason not to pattern match on keyword lists.

### Do not pattern match on keyword lists

List patterns require an exact count and order, but keyword lists have a variable key count and no guaranteed insertion point for new keys. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "do not pattern match on keyword lists."

Use `Keyword.get/3` or `Keyword.fetch/2` instead:

```elixir
# BAD — fragile, breaks if opts order/count changes:
[parts: parts, trim: trim] = opts

# GOOD:
parts = Keyword.get(opts, :parts, 10)
trim  = Keyword.get(opts, :trim, false)
```

Cross-reference `## Pattern Matching and Guards` → `Pattern matching on lists`.

### Maps

Maps are the general-purpose key-value structure. From [Map.html](https://hexdocs.pm/elixir/Map.html):

> "Maps are the \"go to\" key-value data structure in Elixir."

Creation uses `%{key => value}`; when the key is an atom the `%{key: value}` shorthand is available, and any term may be a key — not only atoms:

```elixir
iex> %{}
%{}
iex> map = %{:a => 1, 2 => :b}
%{2 => :b, :a => 1}
iex> %{"hello" => "world", a: 1, b: 2}   # shorthand must come last when mixed
%{:a => 1, :b => 2, "hello" => "world"}
```

Two properties distinguish maps from keyword lists, quoted from [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Maps allow any value as a key.
> Maps have their own internal ordering, which is not guaranteed to be the same across different maps, even if they have the same keys."

Keys are compared with strict equality (`===/2`); a map may not contain duplicate keys, and in a literal the LAST colliding key wins:

```elixir
iex> %{a: 1, a: 2}
%{a: 2}
```

Do NOT rely on map iteration order — it is an implementation detail and may differ between maps with identical keys. `map_size/1` is O(1) and guard-safe (inlined by the compiler):

```elixir
iex> map_size(%{a: "foo", b: "bar"})
2
```

Key lookup is logarithmic (O(log n)), unlike keyword lists' linear lookup — but `Map.keys/1` and `Map.values/1` are still linear because they visit every entry. Predicate: `is_map/1`.

### Map access: `map.key` vs `map[key]`

The two access syntaxes reflect the dual nature of maps. From [Map.html](https://hexdocs.pm/elixir/Map.html):

> "The `map[key]` syntax is used for dynamically created maps that may have any key, of any type. `map.key` is used with maps that hold a predetermined set of atoms keys, which are expected to always be present."

```elixir
iex> map = %{name: "John", age: 23}
%{age: 23, name: "John"}
iex> map.name      # dot syntax: atom keys only; raises KeyError if absent
"John"
iex> map[:name]    # bracket syntax: any key type; returns nil if absent
"John"
iex> map[:missing]
nil
iex> map.missing
** (KeyError) key :missing not found in: %{age: 23, name: "John"}
```

Two gotchas around the dot form:
- Never add parentheses — `map.key()` parses as a remote function call `key/0` on the atom `map`, not field access.
- The dot form works on atom keys only; for non-atom keys (e.g. `map[42]`) you must use brackets.

Cross-reference the `Boolean operators: strict vs truthy` subsection in `## Basic Operators` — the strict-vs-loose distinction recurs here: `map.key` is strict (raises), `map[key]` is loose (`nil`).

### Updating maps: `%{map | key => value}`

The update syntax `%{map | key => value}` modifies ONLY existing keys; a missing key raises `KeyError`, and the compiler may warn at compile time. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "There is also syntax for updating keys, which also raises if the key has not yet been defined … These operations have one large benefit in that they raise if the key does not exist in the map and the compiler may even detect and warn when possible."

```elixir
iex> map = %{name: "John", age: 23}
%{age: 23, name: "John"}
iex> %{map | age: 24}
%{age: 24, name: "John"}
iex> %{map | agee: 27}
** (KeyError) key :agee not found in: %{name: "John", age: 23}
```

To add a key that may not yet exist, use `Map.put/3` (no raise) or `Map.put_new/3` (no overwrite). The `|` operator is also the list-cons operator; cross-reference the `|` row in the `## Basic Operators` → `Special-form operators` table.

### The `Map` module

Key functions, from [Map.html](https://hexdocs.pm/elixir/Map.html):

| Function | Behavior |
|---|---|
| `Map.get/3` | Value for key, or default (`nil` if absent). |
| `Map.fetch/2` | `{:ok, value}` or `:error` — the idiomatic "present + value" check. |
| `Map.fetch!/2` | Value, or raises `KeyError`. |
| `Map.put/3` | Returns a new map with `key => value` (adds or overwrites). |
| `Map.put_new/3` | Puts only if the key is absent. |
| `Map.replace/3` | Puts only if the key already exists; else unchanged (since v1.11). |
| `Map.update/4` | Applies a function to the existing value; inserts a default if absent. |
| `Map.update!/3` | Applies a function to the existing value; raises if absent. |
| `Map.delete/2` / `drop/2` | Remove one key / a list of keys (missing keys ignored). |
| `Map.take/2` | New map with only the listed keys. |
| `Map.merge/2` | All keys of the second map override the first. |
| `Map.intersect/2` | Keeps keys present in both; the second map's value wins (since v1.15). |
| `Map.has_key?/2` | Presence test. |
| `Map.keys/1` / `values/1` | Lists of keys / values. |
| `Map.new/0,1` | Empty map, or built from `{key, value}` pairs. |
| `Map.to_list/1` | List of `{key, value}` tuples (order not guaranteed). |
| `Map.pop/3` | Removes key, returns `{value, new_map}`. |
| `Map.get_and_update/3` | Single-pass read + update; the function may return `:pop` to remove. |

Elixir developers prefer the `map.key` syntax and pattern matching over the `Map` module functions for known-shape maps, because they enforce presence and enable an assertive style. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Elixir developers typically prefer to use the `map.key` syntax and pattern matching instead of the functions in the `Map` module when working with maps because they lead to an assertive style of programming."

Typespec name: `map()`; literal `%{key_type => value_type}` for any-key maps; `%{atom_key: value_type}` for atom-key maps; `%{}` is the empty-map singleton type.

### Pattern matching on maps

Maps perform a SUBSET match: a map pattern matches any map that has AT LEAST the keys in the pattern, and the empty pattern `%{}` matches every map. This is covered in depth in `## Pattern Matching and Guards` → `Pattern matching on maps`; the headline facts:

```elixir
iex> %{name: name} = %{name: "meg", age: 23}   # extra keys are fine
%{age: 23, name: "meg"}
iex> name
"meg"
iex> %{} = %{name: "meg"}                       # empty pattern matches any map
%{name: "meg"}
```

Map keys in patterns must be literals or pinned variables: `%{^key => v} = map`. Because map patterns subset-match while `map.field` and `%{map | field: v}` require exact keys, pattern matching is the preferred way to destructure maps at function boundaries and inside `case`/`with`.

### Struct maps

A struct is a tagged map carrying a `__struct__` field, defined with `defstruct`. From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs are extensions built on top of maps that provide compile-time checks and default values."

```elixir
iex> defmodule User do
...>   defstruct name: "John", age: 27
...> end
iex> john = %User{name: "Jane"}
%User{age: 27, name: "Jane"}
iex> is_map(john)
true
iex> john.__struct__
User
```

Structs reject unknown fields at COMPILE time (`%User{oops: 1}` raises `KeyError` while expanding the struct) and allow ONLY the `struct.field` dot syntax — structs do NOT implement the `Access` behaviour, so `john[:name]` raises `UndefinedFunctionError` (`User.fetch/2` is undefined). The dedicated `## Structs` section covers `defstruct`, `@enforce_keys`, `struct!/2`, and `@derive` in depth. Cross-reference `## Pattern Matching and Guards` → `Pattern matching on structs`.

### Nested data structures and the `Access` behaviour

Maps inside maps, or keyword lists inside maps, are common. Elixir provides the `get_in`/`put_in`/`update_in`/`pop_in`/`get_and_update_in` macros to manipulate nested immutable data. From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Elixir provides conveniences for manipulating nested data structures via the `get_in/1`, `put_in/2`, `update_in/2`, and other macros giving the same conveniences you would find in imperative languages while keeping the immutable properties of the language."

```elixir
iex> users = [
...>   john: %{name: "John", age: 27, languages: ["Erlang", "Ruby", "Elixir"]},
...>   mary: %{name: "Mary", age: 29, languages: ["Elixir", "F#", "Clojure"]}
...> ]
iex> users[:john].age
27
iex> users = put_in(users[:john].age, 31)
[john: %{age: 31, languages: ["Erlang", "Ruby", "Elixir"], name: "John"},
 mary: %{age: 29, languages: ["Elixir", "F#", "Clojure"], name: "Mary"}]
iex> users = update_in(users[:mary].languages, &List.delete(&1, "Clojure"))
```

**Square-bracket vs dot segments in these macros.** The bracket `container[:key]` form is "loose" — it goes through the `Access` behaviour and returns `nil` for a missing key, so it chains safely. The dot `container.key` form is "strict" — it raises on a missing key or on `nil`. From [Access.html](https://hexdocs.pm/elixir/Access.html):

> "the `map[key]` syntax is loose, returning `nil` for missing keys, while the `map.key` syntax is strict, raising for both nil values and missing keys."

`get_in/1` composes a path of dot/bracket segments and is nil-safe for the bracket segments:

```elixir
iex> users = %{"john" => %{age: 27}, "meg" => %{age: 23}}
iex> get_in(users["john"].age)
27
iex> get_in(users["unknown"].age)
nil
```

`nil` itself answers to `[]` and returns `nil`, which is what makes arbitrary nesting safe:

```elixir
iex> nil[:a]
nil
```

**`get_in/2` with a list of accessors** traverses collections (e.g. a list of maps) via `Access` functions such as `Access.all/0`, `Access.at/1`, and `Access.key/1`:

```elixir
iex> user = %{name: "john", languages: [%{name: "elixir", type: :functional}, %{name: "c", type: :procedural}]}
iex> get_in(user, [:languages, Access.all(), :name])
["elixir", "c"]
iex> update_in(user, [:languages, Access.all(), :name], &String.upcase/1)
%{name: "john", languages: [%{name: "ELIXIR", type: :functional}, %{name: "C", type: :procedural}]}
```

The `Access` behaviour is implemented by `Map` and `Keyword` (and by any module declaring `@behaviour Access` and defining `fetch/2`, `get_and_update/3`, `pop/2`). Structs do NOT implement it. Key accessors: `Access.key/1,2` (with optional default) and `Access.key!/1` (raising), `Access.all/0`, `Access.at/1`, `Access.elem/1`, `Access.filter/1`, `Access.slice/1`.

### Maps vs keyword lists

Four axes distinguish them:

| Property | Keyword list | Map |
|---|---|---|
| Key type | atoms only | any term |
| Duplicate keys | allowed | not allowed (last wins in a literal) |
| Order | developer-specified, preserved | not guaranteed, internal |
| Lookup | linear O(n) | logarithmic O(log n) |

Decision guidance, quoted from the summary of [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Use keyword lists for passing optional values to functions.
> Use maps for general key-value data structures.
> Use maps when working with data that has a predefined set of keys."

### Choosing a collection (summary)

| Need | Choose | Notes |
|---|---|---|
| Optional function arguments / options | Keyword list | Last-arg bracket omission; duplicate keys allowed. |
| General key-value lookup | Map | Any key type; logarithmic lookup. |
| Known-shape data with atom keys | Map with `map.key` (or struct) | Dot access raises on absence; structs add compile-time checks. |
| Fixed-size ordered record | Tuple | Cross-reference `## Basic Types` → `Tuples`. |
| Heterogeneous ordered collection | List | Linear; cross-reference `## Basic Types` → `Lists`. |

Cross-reference the `Choosing a collection (summary)` table in `## Basic Types` for the type-level framing, and `docs/elixir/naming-conventions.md` for the `size` = O(1) vs `length` = O(n) rule.

## Modules and Functions

Modules are the unit of code organization in Elixir: a named collection of functions, macros, and module attributes. Functions are defined with `def/2` (public) and `defp/2` (private), and Elixir supports multiple clauses, default arguments, guards, and a capture operator for treating functions as first-class values. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "In order to create our own modules in Elixir, we use the `defmodule` macro. The first letter of a module name (an alias, as described further down) must be in uppercase. We use the `def` macro to define functions in that module. The first letter of every function must be in lowercase (or underscore)"

Cross-reference the `## Pattern Matching and Guards` → `Guards: the `when` keyword` subsection for guard semantics, and `## Keywords and Maps` → `Do blocks and keywords` for the `do:`/`do`-block equivalence.

### `defmodule` and module naming

`defmodule/2` is a Kernel macro. From [Kernel.html#defmodule/2](https://hexdocs.pm/elixir/Kernel.html#defmodule/2):

> "Defines a module given by name with the given contents."

Module names are aliases — capitalized identifiers that compile to atoms. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "An alias in Elixir is a capitalized identifier (like `String`, `Keyword`, etc) which is converted to an atom during compilation. For instance, the `String` alias translates by default to the atom `:"Elixir.String"`"

> "Aliases expand to atoms because in the Erlang Virtual Machine (and consequently Elixir) modules are always represented by atoms. By namespacing those atoms, Elixir modules avoid conflicting with existing Erlang modules."

```elixir
defmodule Math do
  def sum(a, b), do: a + b
end
```

Nested modules are independent and need not be defined in order. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "You don't have to define the `Foo` module before defining the `Foo.Bar` module, as they are effectively independent."

> "If, later, the `Bar` module is moved outside the `Foo` module definition, it must be referenced by its full name (`Foo.Bar`) or an alias must be set using the `alias` directive"

Elixir has two file extensions with identical treatment but different intent. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "Elixir has two file extensions `.ex` (Elixir) and `.exs` (Elixir scripts). Elixir treats both files exactly the same way, the only difference is in intention. `.ex` files are meant to be compiled while `.exs` files are used for scripting."

### `def` and `defp` — public vs private functions

From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "Inside a module, we can define functions with `def/2` and private functions with `defp/2`. A function defined with `def/2` can be invoked from other modules while a private function can only be invoked locally."

From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> `[def(call, expr \\ nil)](#def/2)` — "Defines a public function with the given name and body."
> `[defp(call, expr \\ nil)](#defp/2)` — "Defines a private function with the given name and body."

```elixir
defmodule Math do
  def sum(a, b), do: do_sum(a, b)
  defp do_sum(a, b), do: a + b
end
```

A private function called from outside raises `UndefinedFunctionError`:

```elixir
iex> Math.do_sum(1, 2)
#=> ** (UndefinedFunctionError)
```

### Function clauses and guards

Functions with the same name and arity can have multiple clauses; Elixir tries them in order and the first matching clause wins. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "Function declarations also support guards and multiple clauses. If a function has several clauses, Elixir will try each clause until it finds one that matches."

> "Giving an argument that does not match any of the clauses raises an error."

```elixir
defmodule Math do
  def zero?(0), do: true
  def zero?(x) when is_integer(x), do: false
end
```

A non-matching call raises `FunctionClauseError`:

```elixir
iex> Math.zero?([1, 2, 3])
#=> ** (FunctionClauseError)
iex> Math.zero?(0.0)
#=> ** (FunctionClauseError)
```

Cross-reference the `## Pattern Matching and Guards` → `Allowed guard expressions (and why they are restricted)` subsection for the full set of guard-legal expressions.

### `do:` vs `do`-block syntax

Both are equivalent for function bodies. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "Similar to constructs like `if`, function definitions support both `do:` and `do`-block syntax, as we learned in the previous chapter. […] You may use `do:` for one-liners but always use `do`-blocks for functions spanning multiple lines. If you prefer to be consistent, you can use `do`-blocks throughout your codebase."

```elixir
def zero?(0), do: true
def zero?(x) when is_integer(x), do: false

# equivalent to:
def zero?(0) do
  true
end
def zero?(x) when is_integer(x) do
  false
end
```

### Default arguments (`\\` syntax)

Default arguments use the `\\` separator. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "Any expression is allowed to serve as a default value, but it won't be evaluated during the function definition. Every time the function is invoked and any of its default values have to be used, the expression for that default value will be evaluated."

```elixir
defmodule Concat do
  def join(a, b, sep \\ " ") do
    a <> sep <> b
  end
end
```

**Gotcha:** Default expressions are NOT evaluated at definition time. They are evaluated every time the function is called and a default value is needed — so a default of `DateTime.utc_now()` re-evaluates on each call.

### The function head rule (defaults + multi-clause)

When a function with default values has multiple clauses, Elixir requires a **function head** — a definition without a body that declares the defaults. From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html):

> "If a function with default values has multiple clauses, it is required to create a function head (a function definition without a body) for declaring defaults"

> "Function heads cannot have patterns nor guards. They may only define the argument names and their default values."

```elixir
defmodule Concat do
  # Function head declaring defaults
  def join(a, b, sep \\ " ")

  # Body clauses follow — patterns/guards OK here
  def join(a, b, _sep) when b == "" do
    a
  end

  def join(a, b, sep) do
    a <> sep <> b
  end
end
```

This works because the function head is effectively "splatted" into multiple sub-functions with different arities (e.g. `join/2` and `join/3`).

### Function naming conventions

- Trailing `?` → returns a boolean (e.g. `zero?`, `Map.has_key?`). From [modules-and-functions.html](https://hexdocs.pm/elixir/modules-and-functions.html): "The trailing question mark in `zero?` means that this function returns a boolean."
- Trailing `!` → may raise an exception (e.g. `Map.fetch!`, `File.cwd!`).
- Module names: PascalCase aliases (`Math`, `String`).
- Function names: snake_case (`zero?`, `sum`, `do_sum`).

Cross-reference `docs/elixir/naming-conventions.md` for the full convention set.

### Anonymous functions (`fn`)

Created with the `fn ... end` special form. Multiple clauses are allowed but **all clauses must have the same arity**. Anonymous functions are invoked with the dot operator.

```elixir
iex> negate = fn
...>   true -> false
...>   false -> true
...> end
iex> negate.(true)
false
iex> add = fn a, b -> a + b end
iex> add.(1, 2)
3
```

### The capture operator (`&`)

The capture operator `&` has two distinct uses, both documented in `Kernel.SpecialForms.html`.

**1. Capturing named functions by module/name/arity:**

```elixir
iex> fun = &Kernel.is_atom/1
iex> fun.(:atom)
true
```

You can capture local/private functions without the module prefix (`&local_function/1`). To capture imports or other modules' functions, use the short form (no module prefix) for imports, or the full form for remote captures.

**2. Creating anonymous functions via placeholders (`&1`, `&2`, …):**

```elixir
iex> double = &(&1 * 2)
iex> double.(2)
4
iex> take_five = &Enum.take(&1, 5)
iex> take_five.(1..10)
[1, 2, 3, 4, 5]
iex> first_elem = &elem(&1, 0)
iex> first_elem.({0, 1})
0
iex> fun = &(&1 + &2 + &3)
iex> fun.(1, 2, 3)
6
```

**Restrictions when creating anonymous functions with placeholders:**

- At least one placeholder must be present (i.e. it must contain at least `&1`).
- Block expressions are not supported — `&(&1; &2)` fails.
- `&(:foo)` (no placeholder) fails to compile.

### Local vs external captures and hot-code reloading

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html) and [Function.html](https://hexdocs.pm/elixir/Function.html):

> "Functions that point to definitions residing in modules, such as `&String.length/1`, are **external** functions. All other functions are **local** and they are always bound to the file or module that defined them."

> "Note that `&local_function/1` creates a local capture, but `&__MODULE__.local_function/1` or `&imported_function/1` create a remote capture. […] Whether a capture is local or remote has implications when using hot code reloading: local captures dispatch to the version of the module that existed at the time they were created, while remote captures dispatch to the current version of the module."

### Function arity

Arity is the number of arguments a function accepts. `sum/2` is a function named `sum` taking 2 args; `zero?/1` is `zero?` taking 1 arg. Anonymous functions also have arity — `fn x -> x end` is arity 1. A function with default arguments effectively defines multiple arities (e.g. `join/2` and `join/3`).

### `is_function/1` and `is_function/2`

Both are Kernel guards (allowed in `when`) and inlined by the compiler. From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> `[is_function(term)](#is_function/1)` — "Returns `true` if `term` is a function, otherwise returns `false`."
> `[is_function(term, arity)](#is_function/2)` — "Returns `true` if `term` is a function that can be applied with `arity` number of arguments; otherwise returns `false`."

```elixir
iex> is_function(fn x -> x + x end)
true
iex> is_function(fn x -> x * 2 end, 1)
true
iex> is_function(fn x -> x * 2 end, 2)
false
```

### The `Function` module

From [Function.html](https://hexdocs.pm/elixir/Function.html): the `Function` module provides utilities for working with functions. `Function.capture/3` captures `module.function/arity` programmatically:

```elixir
iex> Function.capture(String, :length, 1)
&String.length/1
```

`Function.info/1` returns a keyword list with `:type`, `:module`, `:arity`, `:name`, `:env`, plus `:pid`, `:index`, `:new_index`, `:new_uniq`, `:uniq` for local anonymous functions. The `:type` field is `:local` (anonymous) or `:external` (named). Note that `is_function/1,2` lives in `Kernel`, not `Function`.

### When to use which function form

| Need | Choose | Notes |
|---|---|---|
| Named, reusable, callable from other modules | `def/2` | Public; supports clauses, guards, defaults. |
| Internal helper | `defp/2` | Private to the defining module. |
| Inline, short-lived, passed to `Enum`/`Stream` | `fn ... end` or `&(...)` | `&` placeholder form is most concise for simple expressions. |
| Reference an existing named function | `&Mod.fun/arity` | External capture; respects hot-code reloading. |
| Boolean predicate | name ending in `?` | Convention; cross-reference `docs/elixir/naming-conventions.md`. |
| Raising variant | name ending in `!` | Convention; pair with a tagged-tuple variant. |

### Gotchas

- **Functions in `.ex` vs `.exs` are treated identically** — only the intent differs (compiled vs scripting).
- **Default expressions re-evaluate on every call** that needs the default — they are not evaluated once at definition time.
- **Function heads cannot have patterns or guards** — only argument names and default values.
- **Local captures bind to the module version at definition time; remote captures bind dynamically** — this matters for hot code reloading.
- **`def/2`'s signature is `def(call, expr \\ nil)`** — `expr` defaults to `nil`, which some tools rely on when overriding `Kernel.def/1`.
- **Anonymous function clauses must share arity** — mixing arities in one `fn` is a compile error.

Cross-reference the `## Recursion` section for how multi-clause functions drive recursive loops, and `## Module Attributes` for `@doc`/`@spec` annotations on functions.

## Recursion

Elixir has no loop constructs (`for`/`while`); iteration is expressed through recursion and the higher-level `Enum`/`Stream` modules. From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "Elixir does not provide loop constructs. Instead we leverage recursion and high-level functions for working with collections."

> "Due to immutability, loops in Elixir (as in any functional programming language) are written differently from imperative languages."

> "functional languages rely on recursion: a function is called recursively until a condition is reached that stops the recursive action from continuing. No data is mutated in this process."

Recursion is driven by multi-clause functions: each clause has its own pattern (and optional guard), and the FIRST clause whose pattern matches and guard holds wins. From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "Similar to `case`, a function may have many clauses. A particular clause is executed when the arguments passed to the function match the clause's argument patterns and its guards evaluate to `true`."

Cross-reference `## Pattern Matching and Guards` → `Where patterns and guards can be used` and `## Modules and Functions` for multi-clause function semantics.

### Loops through recursion

A recursive loop has a recursive clause and a termination clause (base case). From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "This clause, also known as the termination clause, ignores the message argument by assigning it to the `_msg` variable and returns the atom `:ok`."

```elixir
defmodule Recursion do
  def print_multiple_times(msg, n) when n > 0 do
    IO.puts(msg)
    print_multiple_times(msg, n - 1)
  end

  def print_multiple_times(_msg, 0) do
    :ok
  end
end

Recursion.print_multiple_times("Hello!", 3)
# Hello!
# Hello!
# Hello!
:ok
```

If no clause matches (e.g. a negative `n`), `FunctionClauseError` is raised — the guard `n > 0` and the `0` pattern together form an exhaustive match only for non-negative integers:

```elixir
iex> Recursion.print_multiple_times("Hello!", -1)
** (FunctionClauseError) no function clause matching in Recursion.print_multiple_times/2
```

Cross-reference `## Modules and Functions` for `FunctionClauseError` on multi-clause dispatch.

### Reduce and map algorithms

Two recursive shapes dominate list processing. From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "The process of taking a list and *reducing* it down to one value is known as a *reduce algorithm* and is central to functional programming."

> "The process of taking a list and *mapping* over it is known as a *map algorithm*."

**Reduce** — collapse a list to a single value using an accumulator:

```elixir
defmodule Math do
  def sum_list([head | tail], accumulator) do
    sum_list(tail, head + accumulator)
  end

  def sum_list([], accumulator) do
    accumulator
  end
end

IO.puts Math.sum_list([1, 2, 3], 0) #=> 6
```

The recursive call `sum_list(tail, head + accumulator)` is in TAIL POSITION: nothing else runs after it returns. The trace shows the accumulator carrying state forward:

```
sum_list([1, 2, 3], 0)
sum_list([2, 3], 1)
sum_list([3], 3)
sum_list([], 6)
```

**Map** — build a new list by transforming each element:

```elixir
defmodule Math do
  def double_each([head | tail]) do
    [head * 2 | double_each(tail)]
  end

  def double_each([]) do
    []
  end
end

Math.double_each([1, 2, 3]) #=> [2, 4, 6]
```

Here `double_each/1` is NOT tail-recursive: the cons `[head * 2 | double_each(tail)]` must build its result AFTER the recursive call returns, so each frame stays on the stack.

### Tail-call optimization

When the recursive call is the LAST action of a clause (tail position), the BEAM reuses the current stack frame instead of allocating a new one — this is tail-call optimization (TCO). The `sum_list/2` reduce above is tail-recursive; the `double_each/1` map is not. From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "Recursion and [tail call](https://en.wikipedia.org/wiki/Tail_call) optimization are an important part of Elixir and are commonly used to create loops."

The practical consequence: a tail-recursive loop can iterate over millions of elements without stack overflow, because each step runs in constant stack space. To make a non-tail-recursive function tail-recursive, thread the partial result through an ACCUMULATOR argument (as `sum_list/2` does) rather than building the result on the way back up. A common idiom is a public head delegating to a private tail-recursive helper:

```elixir
defmodule Math do
  # Public head: no accumulator exposed to callers
  def sum_list(list), do: sum_list(list, 0)

  # Private tail-recursive helper with accumulator
  defp sum_list([head | tail], acc), do: sum_list(tail, head + acc)
  defp sum_list([], acc), do: acc
end
```

Note: TCO applies only to a call in genuine tail position. A call followed by additional work (cons, arithmetic, another function applied to its result) is NOT a tail call and will grow the stack.

### Recursion vs `Enum`

Manual recursion is rarely the right tool for collection processing. From [recursion.html](https://hexdocs.pm/elixir/recursion.html):

> "Recursion and tail call optimization are an important part of Elixir and are commonly used to create loops. However, when programming in Elixir you will rarely use recursion as above to manipulate lists."

The `Enum` module provides the same reduce/map operations as eager, polymorphic functions:

```elixir
iex> Enum.reduce([1, 2, 3], 0, fn x, acc -> x + acc end)
6
iex> Enum.map([1, 2, 3], fn x -> x * 2 end)
[2, 4, 6]
```

Both accept the capture operator (`&`) for concise anonymous functions:

```elixir
iex> Enum.reduce([1, 2, 3], 0, &+/2)
6
iex> Enum.map([1, 2, 3], &(&1 * 2))
[2, 4, 6]
```

Rule of thumb: reach for `Enum`/`Stream` (see `## Enumerables and Streams`) for collection traversal; reserve hand-written recursion for control-flow loops that do not map onto an existing `Enum` function, or for teaching/algorithmic clarity. Cross-reference the `&` row in `## Basic Operators` → `Special-form operators`.

### Mutual recursion (note)

Mutual recursion — two or more functions that call each other in a cycle — is supported naturally because Elixir functions dispatch by clause at runtime. It is not covered on the recursion getting-started page; the canonical patterns (e.g. a request/reply loop split across `handle_call`/`handle_cast` clauses) appear in `GenServer` and supervision code. See `## Processes` for stateful loops and the OTP guide for supervised mutual recursion.

## Enumerables and Streams

Most collection operations in Elixir go through the `Enum` and `Stream` modules rather than hand-written recursion. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "While Elixir allows us to write recursive code, most operations we perform on collections is done with the help of the `Enum` and `Stream` modules."

Cross-reference `## Recursion` for the recursive foundations these modules build on.

### Enumerables and the `Enumerable` protocol

An enumerable is any data type that implements the `Enumerable` protocol. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "Elixir provides the concept of enumerables and the `Enum` module to work with them. We have already learned two enumerables: lists and maps."

> "We say the functions in the `Enum` module are polymorphic because they can work with diverse data types. In particular, the functions in the `Enum` module can work with any data type that implements the `Enumerable` protocol."

Lists, maps, and ranges are all enumerables:

```elixir
iex> Enum.map([1, 2, 3], fn x -> x * 2 end)
[2, 4, 6]
iex> Enum.map(%{1 => 2, 3 => 4}, fn {k, v} -> k * v end)
[2, 12]
iex> Enum.map(1..3, fn x -> x * 2 end)
[2, 4, 6]
iex> Enum.reduce(1..3, 0, &+/2)
6
```

Cross-reference `## Protocols` for protocol dispatch, and the `Enumerable.html` entry in `## Sources used` for the protocol's `reduce/3` core plus `count/1`/`member?/2`/`slice/1` optimizations.

The `Enum` module is broad but collection-only. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "The functions in the `Enum` module are limited to, as the name says, enumerating values in data structures. For specific operations, like inserting and updating particular elements, you may need to reach for modules specific to the data type."

### Eager vs lazy

`Enum` functions are EAGER: each call fully traverses its input and produces a concrete list. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "All the functions in the `Enum` module are eager. Many functions expect an enumerable and return a list back"

> "This means that when performing multiple operations with `Enum`, each operation is going to generate an intermediate list until we reach the result"

A pipelined eager computation builds one intermediate list per step:

```elixir
iex> odd? = fn x -> rem(x, 2) != 0 end
#Function<6.80484245/1 in :erl_eval.expr/5>
iex> Enum.filter(1..3, odd?)
[1, 3]
iex> 1..100_000 |> Enum.map(&(&1 * 3)) |> Enum.filter(odd?) |> Enum.sum()
7500000000
```

Without the pipe operator the same call nests inside-out:

```elixir
iex> Enum.sum(Enum.filter(Enum.map(1..100_000, &(&1 * 3)), odd?))
7500000000
```

### The pipe operator

The `|>` operator passes the result of the left expression as the FIRST argument of the right call, making data-transformation pipelines read left-to-right. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "The `|>` symbol used in the snippet above is the **pipe operator**: it takes the output from the expression on its left side and passes it as the first argument to the function call on its right side. Its purpose is to highlight the data being transformed by a series of functions."

Cross-reference `## Basic Operators` → `The pipe operator |>` for precedence and associativity.

### Streams

`Stream` is the lazy counterpart of `Enum`. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "As an alternative to `Enum`, Elixir provides the `Stream` module which supports lazy operations"

> "**Streams are lazy, composable enumerables.**"

> "Instead of generating intermediate lists, streams build a series of computations that are invoked only when we pass the underlying stream to the `Enum` module."

The same pipeline expressed with `Stream` produces no intermediate lists; the chain is only realized when a terminal `Enum` function consumes it:

```elixir
iex> 1..100_000 |> Stream.map(&(&1 * 3)) |> Stream.filter(odd?) |> Enum.sum()
7500000000
```

A stream value is itself a computation description, not a materialized list — inspecting it shows the lazy structure:

```elixir
iex> 1..100_000 |> Stream.map(&(&1 * 3))
#Stream<[enum: 1..100000, funs: [#Function<34.16982430/1 in Stream.map/2>]]>
iex> 1..100_000 |> Stream.map(&(&1 * 3)) |> Stream.filter(odd?)
#Stream<[enum: 1..100000, funs: [...]]>
```

### When to use streams

Streams shine for large or potentially infinite collections. From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "Streams are useful when working with large, *possibly infinite*, collections."

`Stream.cycle/1` produces an infinite stream that must be consumed with a bounded operation like `Enum.take/2`:

```elixir
iex> stream = Stream.cycle([1, 2, 3])
#Function<15.16982430/2 in Stream.unfold/2>
iex> Enum.take(stream, 10)
[1, 2, 3, 1, 2, 3, 1, 2, 3, 1]
```

> "Be careful to not call a function like `Enum.map/2` on such streams, as they would cycle forever"

`Stream.resource/3` wraps external resources with guaranteed open/close semantics, and is the foundation of `File.stream!/1`:

```elixir
iex> "path/to/file" |> File.stream!() |> Enum.take(10)
```

> "Another interesting function is `Stream.resource/3` which can be used to wrap around resources, guaranteeing they are opened right before enumeration and closed afterwards, even in the case of failures."

### Choosing `Enum` vs `Stream`

From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "The `Enum` and `Stream` modules provide a wide range of functions, but you don't have to know all of them by heart. Familiarize yourself with `Enum.map/2`, `Enum.reduce/3` and other functions with either `map` or `reduce` in their names, and you will naturally build an intuition around the most important use cases. You may also focus on the `Enum` module first and only move to `Stream` for the particular scenarios where laziness is required, to either deal with slow resources or large, possibly infinite, collections."

| Need | Choose |
|---|---|
| Small/medium in-memory collection, simple pipeline | `Enum` (eager, simpler, intermediate lists are cheap) |
| Large collection, avoid intermediate lists | `Stream` (lazy, single traversal) |
| Possibly infinite source | `Stream` + bounded terminal (`Enum.take/2`) |
| External resource (file, socket) | `Stream.resource/3` / `File.stream!/1` |

Cross-reference `## Comprehensions` for an alternative concise form of map/filter over enumerables.

## Processes

All Elixir code runs inside BEAM processes — lightweight, isolated, concurrent units of execution that communicate solely by message passing. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "In Elixir, all code runs inside processes. Processes are isolated from each other, run concurrent to one another and communicate via message passing. Processes are not only the basis for concurrency in Elixir, but they also provide the means for building distributed and fault-tolerant programs."

> "Elixir's processes should not be confused with operating system processes. Processes in Elixir are extremely lightweight in terms of memory and CPU (even compared to threads as used in many other programming languages). Because of this, it is not uncommon to have tens or even hundreds of thousands of processes running simultaneously."

Cross-reference the `PID` and `Port` rows in `## Basic Types` → `Built-in types & predicates quick-reference`.

### Spawning processes

The auto-imported `spawn/1` runs a function in a new process and returns its PID. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "The basic mechanism for spawning new processes is the auto-imported `spawn/1` function"

> "[`spawn/1`] takes a function which it will execute in another process."

> "Notice `spawn/1` returns a PID (process identifier). At this point, the process you spawned is very likely dead. The spawned process will execute the given function and exit after the function is done"

```elixir
iex> spawn(fn -> 1 + 2 end)
#PID<0.43.0>
iex> pid = spawn(fn -> 1 + 2 end)
#PID<0.44.0>
iex> Process.alive?(pid)
false
iex> self()
#PID<0.41.0>
iex> Process.alive?(self())
true
```

`self/0` returns the current process's PID; `Process.alive?/1` tests liveness.

### Sending and receiving messages

`send/2` deposits a message in a process's mailbox (asynchronously); `receive/1` matches messages from the current process's mailbox. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "We can send messages to a process with `send/2` and receive them with `receive/1`"

> "When a message is sent to a process, the message is stored in the process mailbox. The `receive/1` block goes through the current process mailbox searching for a message that matches any of the given patterns. `receive/1` supports guards and many clauses, exactly as `case/2`."

> "The process that sends the message does not block on `send/2`, it puts the message in the recipient's mailbox and continues. In particular, a process can send messages to itself."

```elixir
iex> send(self(), {:hello, "world"})
{:hello, "world"}
iex> receive do
...>   {:hello, msg} -> msg
...>   {:world, _msg} -> "won't match"
...> end
"world"
```

`receive` blocks if no message matches. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "If there is no message in the mailbox matching any of the patterns, the current process will wait until a matching message arrives. A timeout can also be specified"

> "A timeout of 0 can be given when you already expect the message to be in the mailbox."

```elixir
iex> receive do
...>   {:hello, msg}  -> msg
...> after
...>   1_000 -> "nothing after 1s"
...> end
"nothing after 1s"
```

A parent can spawn a child that sends back a message, then receive it:

```elixir
iex> parent = self()
#PID<0.41.0>
iex> spawn(fn -> send(parent, {:hello, self()}) end)
#PID<0.48.0>
iex> receive do
...>   {:hello, pid} -> "Got hello from #{inspect pid}"
...> end
"Got hello from #PID<0.48.0>"
```

The shell helper `flush/0` prints and clears the mailbox:

```elixir
iex> send(self(), :hello)
:hello
iex> flush()
:hello
:ok
```

Cross-reference `## Pattern Matching and Guards` → `Where patterns and guards can be used` (the `receive` row) and `## Control Flow` → `case/2` for the shared clause semantics.

### Process isolation and links

A crashing process does NOT bring down unrelated processes — isolation is the default. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "It merely logged an error but the parent process is still running. That's because processes are isolated."

```elixir
iex> spawn(fn -> raise "oops" end)
#PID<0.58.0>

[error] Process #PID<0.58.0> raised an exception
** (RuntimeError) oops
```

To propagate failure between processes, LINK them with `spawn_link/1` (or `Process.link/1` after a plain spawn). From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "If we want the failure in one process to propagate to another one, we should link them. This can be done with `spawn_link/1`"

> "Because processes are linked, we now see a message saying the parent process, which is the shell process, has received an EXIT signal from another process causing the shell to terminate."

```elixir
iex> self()
#PID<0.41.0>
iex> spawn_link(fn -> raise "oops" end)
** (EXIT from #PID<0.41.0>) evaluator process exited with reason: an exception was raised:
    ** (RuntimeError) oops
```

> "Linking can also be done manually by calling `Process.link/1`."

Links are bidirectional and underpin supervision. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "Processes and links play an important role when building fault-tolerant systems. Elixir processes are isolated and don't share anything by default. Therefore, a failure in a process will never crash or corrupt the state of another process. Links, however, allow processes to establish a relationship in case of failure. We often link our processes to supervisors which will detect when a process dies and start a new process in its place."

> "While other languages would require us to catch/handle exceptions, in Elixir we are actually fine with letting processes fail because we expect supervisors to properly restart our systems. \"Failing fast\" (sometimes referred as \"let it crash\") is a common philosophy when writing Elixir software!"

Note: the getting-started page covers links but not monitors. `Process.monitor/1` and `spawn_monitor/1` provide one-way failure observation (a `{:DOWN, ref, ...}` message rather than an EXIT signal) and are documented in the `Process` module reference — reach for a monitor when you need to observe a process WITHOUT dying with it.

### Tasks

`Task` wraps `spawn` with better error reports and introspection, returning `{:ok, pid}` instead of a bare PID. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "Tasks build on top of the spawn functions to provide better error reports and introspection"

> "Instead of `spawn/1` and `spawn_link/1`, we use `Task.start/1` and `Task.start_link/1` which return `{:ok, pid}` rather than just the PID."

```elixir
iex> Task.start(fn -> raise "oops" end)
{:ok, #PID<0.55.0>}

15:22:33.046 [error] Task #PID<0.55.0> started from #PID<0.53.0> terminating
** (RuntimeError) oops
```

`Task.async/1` + `Task.await/1` provide a concurrent request/reply pair over a linked task.

### Stateful processes

A process can hold state by looping forever, matching messages, and recursing with an updated state argument. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "Processes are the most common answer to this question. We can write processes that loop infinitely, maintain state, and send and receive messages."

```elixir
defmodule KV do
  def start_link do
    Task.start_link(fn -> loop(%{}) end)
  end

  defp loop(map) do
    receive do
      {:get, key, caller} ->
        send(caller, Map.get(map, key))
        loop(map)
      {:put, key, value} ->
        loop(Map.put(map, key, value))
    end
  end
end
```

> "Note that the `start_link` function starts a new process that runs the `loop/1` function, starting with an empty map. The `loop/1` (private) function then waits for messages and performs the appropriate action for each message. We made `loop/1` private by using `defp` instead of `def`. In the case of a `:get` message, it sends a message back to the caller and calls `loop/1` again, to wait for a new message. While the `:put` message actually invokes `loop/1` with a new version of the map, with the given `key` and `value` stored."

The `loop/1` recursion is in tail position (see `## Recursion` → `Tail-call optimization`), so the loop runs forever in constant stack. Callers interact by sending messages and reading replies from their own mailbox:

```elixir
iex> {:ok, pid} = KV.start_link()
{:ok, #PID<0.62.0>}
iex> send(pid, {:get, :hello, self()})
{:get, :hello, #PID<0.41.0>}
iex> flush()
nil
:ok
iex> send(pid, {:put, :hello, :world})
{:put, :hello, :world}
iex> send(pid, {:get, :hello, self()})
{:get, :hello, #PID<0.41.0>}
iex> flush()
:world
:ok
```

> "Notice how the process is keeping a state and we can get and update this state by sending the process messages. In fact, any process that knows the `pid` above will be able to send it messages and manipulate the state."

A PID can be registered under a name so any process can address it by atom:

```elixir
iex> Process.register(pid, :kv)
true
iex> send(:kv, {:get, :hello, self()})
{:get, :hello, #PID<0.41.0>}
iex> flush()
:world
:ok
```

### Abstractions over raw processes

In practice the loop pattern above is rarely hand-written. From [processes.html](https://hexdocs.pm/elixir/processes.html):

> "Using processes to maintain state and name registration are very common patterns in Elixir applications. However, most of the time, we won't implement those patterns manually as above, but by using one of the many abstractions that ship with Elixir."

> "Besides agents, Elixir provides an API for building generic servers (called `GenServer`), registries, and more, all powered by processes underneath."

`Agent` is the thin state abstraction; `GenServer` is the generic server; both sit under supervision trees. The `Agent` equivalent of the `KV` example:

```elixir
iex> {:ok, pid} = Agent.start_link(fn -> %{} end)
{:ok, #PID<0.72.0>}
iex> Agent.update(pid, fn map -> Map.put(map, :hello, :world) end)
:ok
iex> Agent.get(pid, fn map -> Map.get(map, :hello) end)
:world
```

Cross-reference `## Modules and Functions` for `defp`/multi-clause dispatch, and `## Recursion` for tail-recursive loops.

## Alias, Require, and Import

Elixir provides three directives (`alias`, `require`, `import`) plus a macro (`use`) for software reuse. The first three are called directives because they have **lexical scope**; `use` is a Kernel macro that invokes a `__using__/1` callback to inject code. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "In order to facilitate software reuse, Elixir provides three directives (`alias`, `require`, and `import`) plus a macro called `use` summarized below:"

```elixir
# Alias the module so it can be called as Bar instead of Foo.Bar
alias Foo.Bar, as: Bar

# Require the module in order to use its macros
require Foo

# Import functions from Foo so they can be called without the `Foo.` prefix
import Foo

# Invokes the custom code defined in Foo as an extension point
use Foo
```

> "Keep in mind the first three are called directives because they have *lexical scope*, while `use` is a common extension point that allows the used module to inject code."

### `alias` — shortening module names

`alias/2` is a special form (see [Kernel.SpecialForms.html#alias/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#alias/2)) that sets a module-name shortcut. With no `:as` option, it defaults to the last part of the module name. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "Aliases are frequently used to define shortcuts. In fact, calling `alias` without an `:as` option sets the alias automatically to the last part of the module name, for example: `alias Math.List` is the same as `alias Math.List, as: List`."

> "Note that `alias` is *lexically scoped*, which allows you to set aliases inside specific functions:"

```elixir
defmodule Math do
  def plus(a, b) do
    alias Math.List
    # ...
  end
end
```

From [Kernel.SpecialForms.html#alias/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#alias/2):

> "`import/2`, `require/2` and `alias/2` are called directives and all have lexical scope. This means you can set up aliases inside specific functions and it won't affect the overall scope."

Multiple modules can be aliased in one line. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "We can also alias multiple modules in one line: `alias Foo.{Bar, Baz, Biz}` Is the same as: `alias Foo.Bar` `alias Foo.Baz` `alias Foo.Biz`"

> "If you alias a module and you don't use the alias, Elixir is going to issue a warning implying the alias is not being used."

```elixir
alias Math.List                 # → Math.List becomes List
alias Math.List, as: List       # explicit form
alias Foo.{Bar, Baz, Biz}       # → aliases for Foo.Bar, Foo.Baz, Foo.Biz
```

### `require` — enabling macro calls

`require/2` is a special form (see [Kernel.SpecialForms.html#require/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#require/2)) that must precede any macro invocation. Macros are expanded at compile time, so the compiler must know the module exists. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "Public functions in modules are globally available, but in order to use macros, you need to opt-in by requiring the module they are defined in."

```elixir
iex> Integer.is_odd(3)
** (UndefinedFunctionError) function Integer.is_odd/1 is undefined or private. However, there is a macro with the same name and arity. Be sure to require Integer if you intend to invoke this macro
    (elixir) Integer.is_odd(3)
iex> require Integer
Integer
iex> Integer.is_odd(3)
true
```

> "In Elixir, `Integer.is_odd/1` is defined as a macro so that it can be used as a guard. This means that, in order to invoke `Integer.is_odd/1`, we need to first require the `Integer` module."

> "Note that like the `alias` directive, `require` is also lexically scoped."

### `import` — bringing functions into scope

`import/2` is a special form (see [Kernel.SpecialForms.html#import/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#import/2)) that brings public functions/macros into local scope without the module prefix. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "We use `import` whenever we want to access functions or macros from other modules without using the fully-qualified name. Note we can only import public functions, as private functions are never accessible externally."

```elixir
iex> import List, only: [duplicate: 2]
List
iex> duplicate(:ok, 3)
[:ok, :ok, :ok]
```

> "We imported only the function `duplicate` (with arity 2) from `List`. Although `:only` is optional, its usage is recommended in order to avoid importing all the functions of a given module inside the current scope. `:except` could also be given as an option in order to import everything in a module except a list of functions."

> "Note that `import` is *lexically scoped* too. This means that we can import specific macros or functions inside function definitions"

> "While `import`s can be useful for frameworks and libraries to build abstractions, developers should generally prefer `alias` to `import` on their own codebases, as aliases make the origin of the function being invoked clearer."

From [Kernel.SpecialForms.html#import/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#import/2):

> "By default, Elixir imports functions and macros from the given module, except the ones starting with the underscore (which are usually callbacks)"

> "A developer can filter to import only functions, macros, or sigils (which can be functions or macros) via the `:only` option: `import List, only: :functions`, `import List, only: :macros`, `import Kernel, only: :sigils`"

> "Importing the same module again will erase the previous imports, except when the `except` option is used, which is always exclusive on a previously declared `import/2`."

> "By default functions starting with `_` are not imported. If you really want to import a function starting with `_` you must explicitly include it in the `:only` selector."

> "If two modules `A` and `B` are imported and they both contain a `foo` function with an arity of `1`, an error is only emitted if an ambiguous call to `foo/1` is actually made; that is, the errors are emitted lazily, not eagerly."

> "If you import a module and you don't use any of the imported functions or macros from this module, Elixir is going to issue a warning implying the import is not being used."

```elixir
import List, only: [duplicate: 2]
import List, only: :functions              # all functions
import List, only: :macros                # all macros
import Kernel, only: :sigils              # all sigils
import File.Stream, only: [__build__: 3] # underscore-prefixed require explicit opt-in
import String, except: [split: 2]         # exclude
```

### `use` — the extension point

`use/2` is a regular Kernel macro (see [Kernel.html#use/2](https://hexdocs.pm/elixir/Kernel.html#use/2)), NOT a special form. From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "The `use` macro is frequently used as an extension point. This means that, when you `use` a module `FooBar`, you allow that module to inject *any* code in the current module, such as importing itself or other modules, defining new functions, setting a module state, etc."

```elixir
defmodule AssertionTest do
  use ExUnit.Case, async: true

  test "always pass" do
    assert true
  end
end
```

> "Behind the scenes, `use` requires the given module and then calls the `__using__/1` callback on it allowing the module to inject some code into the current context. Some modules (for example, the above `ExUnit.Case`, but also `Supervisor` and `GenServer`) use this mechanism to populate your module with some basic behaviour, which your module is intended to override or complete."

> "Generally speaking, the following module: `defmodule Example do use Feature, option: :value end` is compiled into `defmodule Example do require Feature; Feature.__using__(option: :value) end`"

> "Since `use` allows any code to run, we can't really know the side-effects of using a module without reading its documentation. Therefore use this function with care and only if strictly required. Don't use `use` where an `import` or `alias` would do."

### Multi-alias/import/require/use

From [alias-require-and-import.html](https://hexdocs.pm/elixir/alias-require-and-import.html):

> "It is possible to `alias`, `import`, `require`, or `use` multiple modules at once. […] For example, imagine you have an application where all modules are nested under `MyApp`, you can alias the modules `MyApp.Foo`, `MyApp.Bar` and `MyApp.Baz` at once as follows: `alias MyApp.{Foo, Bar, Baz}`"

### Directive vs macro distinction

- **`alias`, `require`, `import`** are special forms with lexical scope — they affect only the scope in which they appear (a function, a module body, etc.).
- **`use`** is a Kernel macro that expands to `require Mod; Mod.__using__(opts)`. It is NOT lexically scoped in the same sense; its effects are whatever `__using__/1` injects at the call site.

### Combining directives

All four can appear in the same module:

```elixir
defmodule MyApp.Worker do
  alias MyApp.{Repo, Helpers}
  require SomeMacros
  import AnotherModule, only: [some_func: 1]
  use GenServer

  # ...
end
```

### When to use each

| Directive | Use when | Notes |
|---|---|---|
| `alias` | Shortening long module names (everyday code). | Most common; default-to-last-part; `Foo.{Bar,Baz}` multi-alias. |
| `require` | Before invoking a macro. | Most macros arrive via `use` (which calls `require` for you). |
| `import` | Rarely in application code; common in frameworks/libraries. | Prefer `alias`; always use `:only`/`:except`. |
| `use` | Known extension points (ExUnit, GenServer, Supervisor, Logger). | Read the used module's docs; arbitrary side effects. |

### Gotchas

- **`use` is a macro, not a special form** like the other three — it is defined in `Kernel` (`use/2`).
- **Unused `alias` and unused `import` produce warnings.** Aliases/imports generated by macros suppress the warning; `warn: true`/`warn: false` controls it explicitly.
- **Lexical scope** means an `alias` inside a function does not leak to the rest of the module.
- **A second `import` of the same module erases the previous** unless `:except` is used (which is cumulative).
- **Ambiguity errors are lazy** — two imports of the same `name/arity` only error when an ambiguous call is actually made.
- **`_`-prefixed functions are not imported by default** — opt in via `:only`.

Cross-reference the `## Module Attributes` section for `@behaviour`/`@impl` (which interact with `use`-injected callbacks), and the `## Processes` section for `use GenServer` / `use Supervisor` patterns.

## Module Attributes

Module attributes in Elixir serve three purposes: as annotations, as temporary storage during compilation, and as compile-time constants. From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

> "Module attributes in Elixir serve three purposes: 1. as module and function annotations; 2. as temporary module storage to be used during compilation; 3. as compile-time constants"

> "Elixir brings the concept of module attributes from Erlang."

Cross-reference the `## Modules and Functions` section for `def`/`defp` (which `@doc`/`@spec` annotate) and the `## Structs` section for `@enforce_keys`/`@derive`.

### Reserved attributes (annotations)

From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

> "Elixir has a handful of reserved attributes. Here are a few of them, the most commonly used ones:
> - `@moduledoc` — provides documentation for the current module.
> - `@doc` — provides documentation for the function or macro that follows the attribute.
> - `@spec` — provides a typespec for the function that follows the attribute.
> - `@behaviour` — (notice the British spelling) used for specifying an OTP or user-defined behaviour."

> "`@moduledoc` and `@doc` are by far the most used attributes, and we expect you to use them a lot."

> "Documentation is only accessible from compiled modules."

From [Module.html](https://hexdocs.pm/elixir/Module.html), the full set of documentation-related attributes includes `@moduledoc`, `@doc`, and `@typedoc` (for types). `@doc` accepts a string, `false`, or a keyword list; `@typedoc` is used with a public or opaque type.

### `@behaviour` (British spelling!)

From [Module.html](https://hexdocs.pm/elixir/Module.html):

> "**@behaviour**
> Note the British spelling!
> Behaviours can be referenced by modules to ensure they implement required specific function signatures defined by `@callback`."

A module that omits any required `@callback` (or implements one with the wrong arity) gets a compile-time warning.

### `@impl` — marking callbacks

From [Module.html](https://hexdocs.pm/elixir/Module.html):

> "**@impl (since v1.5.0)**
> To aid in the correct implementation of behaviours, you may optionally declare `@impl` for implemented callbacks of a behaviour. This makes callbacks explicit and can help you to catch errors in your code. The compiler will warn in these cases:
> - if you mark a function with `@impl` when that function is not a callback.
> - if you don't mark a function with `@impl` when other functions are marked with `@impl`. If you mark one function with `@impl`, you must mark all other callbacks for that behaviour as `@impl`."

> "`@impl` works on a per-context basis. If you generate a function through a macro and mark it with `@impl`, that won't affect the module where that function is generated in."

> "`@impl` also helps with maintainability by making it clear to other developers that the function is implementing a callback."

```elixir
defmodule URI.HTTP do
  @behaviour URI.Parser

  @impl true
  def default_port(), do: 80

  @impl true
  def parse(info), do: info
end
```

`@impl` accepts `false`, `true`, or a specific behaviour:

```elixir
@impl true   # any behaviour callback
@impl Baz    # callback from Baz specifically
```

> "`@impl true` automatically marks the function as `@doc false`, disabling documentation unless `@doc` is explicitly set."

**Critical rule:** "If you mark one function with `@impl`, you must mark all other callbacks for that behaviour as `@impl`." Mixed annotation triggers a warning.

### `@callback`, `@macrocallback`, `@optional_callbacks`

From [typespecs.html](https://hexdocs.pm/elixir/typespecs.html):

> "Modules adopting the `Parser` behaviour will have to implement all the functions defined with the `@callback` attribute. […] `@callback` expects a function name but also a function specification like the ones used with the `@spec` attribute we saw above."

```elixir
defmodule Parser do
  @callback parse(String.t) :: {:ok, term} | {:error, atom}
  @callback extensions() :: [String.t]
end
```

> "If a module adopting a given behaviour doesn't implement one of the callbacks required by that behaviour, a compile-time warning will be generated."

> "Optional callbacks are callbacks that callback modules may implement if they want to, but are not required to."

```elixir
defmodule MyBehaviour do
  @callback vital_fun() :: any
  @callback non_vital_fun() :: any
  @macrocallback non_vital_macro(arg :: any) :: Macro.t
  @optional_callbacks non_vital_fun: 0, non_vital_macro: 1
end
```

> "The `@callback` and `@optional_callbacks` attributes are used to create a `behaviour_info/1` function available on the defining module."

### As temporary storage during compilation

Attributes can accumulate values during compilation. Reading an undefined attribute emits a warning. From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

> "Trying to access an attribute that was not defined will print a warning:
> ```
> warning: undefined module attribute @unknown, please remove access to @unknown or explicitly set it before access
> ```"

> "Do not add a newline between the attribute and its value, otherwise Elixir will assume you are reading the value, rather than setting it."

> "The module attribute is defined at compilation time and its *return value*, not the function call itself, is what will be substituted in for the attribute."

> "You cannot invoke functions defined in the same module as part of the attribute itself, as those functions have not yet been defined."

#### The snapshot rule

> "Every time we read an attribute inside a function, Elixir takes a snapshot of its current value. Therefore if you read the same attribute multiple times inside multiple functions, you end-up increasing compilation times as Elixir now has to compile every snapshot. Generally speaking, you want to avoid reading the same attribute multiple times and instead move it to function. For example, instead of this:
> ```
> def some_function, do: do_something_with(@example)
> def another_function, do: do_something_else_with(@example)
> ```
> Prefer this:
> ```
> def some_function, do: do_something_with(example())
> def another_function, do: do_something_else_with(example())
> defp example, do: @example
> ```"

### As compile-time constants

From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

> "Module attributes may also be useful as compile-time constants. Generally speaking, functions themselves are sufficient for the role of constants in a codebase. For example, instead of defining:
> ```
> @hours_in_a_day 24
> ```
> You should prefer:
> ```
> defp hours_in_a_day(), do: 24
> ```
> You may even define a public function if it needs to be shared across modules."

> "You can even have composite data structures as constants, as long as they are made exclusively of other data types (no function calls, no operators, and no other expressions)."

> "Given data structures in Elixir are immutable, only a single instance of the data structure above is allocated and shared across all functions calls, as long as it doesn't have any executable expression."

#### Attributes in patterns and guards

The key use case for module attributes as constants is injection into patterns and guards — where function calls cannot go. From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

> "The use case for module attributes arise when you need to do some work at compile-time and then inject its results inside a function. A common scenario is module attributes inside patterns and guards (as an alternative to `defguard/1`), since they only support a limited set of expressions:
> ```
> # Inside pattern
> @default_timezone "Etc/UTC"
> def shift(@default_timezone), do: ...
>
> # Inside guards
> @time_periods [:am, :pm]
> def shift(time, period) when period in @time_periods, do: ...
> ```"

Cross-reference the `## Pattern Matching and Guards` → `Custom guards: `defguard` / `defguardp`` subsection for the alternative.

### Accumulation and registered attributes

ExUnit's `@tag` demonstrates accumulation. From [module-attributes.html](https://hexdocs.pm/elixir/module-attributes.html):

```elixir
defmodule MyTest do
  use ExUnit.Case, async: true

  @tag :external
  @tag os: :unix
  test "contacts external service" do
    # ...
  end
end
```

> "[ExUnit] stores the value of `async: true` in a module attribute to change how the module is compiled. Tags also work as annotations and they can be supplied multiple times, thanks to Elixir's ability to **accumulate attributes**."

From [Module.html#register_attribute/3](https://hexdocs.pm/elixir/Module.html):

> "When registering an attribute, two options can be given:
> - `:accumulate` - several calls to the same attribute will accumulate instead of overriding the previous one. New attributes are always added to the top of the accumulated list.
> - `:persist` - the attribute will be persisted in the Erlang Abstract Format. Useful when interfacing with Erlang libraries.
>
> By default, both options are `false`. Once an attribute has been set to accumulate or persist, the behaviour cannot be reverted."

```elixir
defmodule MyModule do
  Module.register_attribute(__MODULE__, :custom_threshold_for_lib, accumulate: true)

  @custom_threshold_for_lib 10
  @custom_threshold_for_lib 20
  @custom_threshold_for_lib #=> [20, 10]
end
```

### Custom attributes

From [Module.html](https://hexdocs.pm/elixir/Module.html):

> "In addition to the built-in attributes outlined above, custom attributes may also be added. Custom attributes are expressed using the `@/1` operator followed by a valid variable name. The value given to the custom attribute must be a valid Elixir value."

### Compile hooks and other reserved attributes

From [Module.html](https://hexdocs.pm/elixir/Module.html):

- **`@before_compile`** — "A hook that will be invoked before the module is compiled. Accepts a module or a `{module, function_or_macro_name}` tuple. […] When just a module is provided, the function/macro is assumed to be `__before_compile__/1`."
- **`@after_compile`** — "A hook that will be invoked with the bytecode of the current module. […] accepts a module or a `{module, function_name}` tuple. The function must take two arguments: the module environment and its bytecode."
- **`@after_verify` (since v1.14.0)** — "A hook that will be invoked right after the current module is verified for undefined functions, deprecations, etc."
- **`@on_definition`** — "A hook that will be invoked when each function or macro in the current module is defined. […] the function must take 6 arguments: the module environment, the kind (`:def`, `:defp`, `:defmacro`, or `:defmacrop`), the function/macro name, the list of quoted arguments, the list of quoted guards, the quoted function body. […] If the function/macro being defined has multiple clauses, the hook will be called for each clause."
- **`@on_load`** — "A hook that will be invoked whenever the module is loaded. Accepts the function name (as an atom) of a function in the current module. The function must have an arity of 0."
- **`@external_resource`** — "Specifies an external resource for the current module. […] Tools may use this information to ensure the module is recompiled in case any of the external resources change."
- **`@compile`** — "Defines options for module compilation. […] `@compile {:inline, my_fun: 1}` for example."
- **`@deprecated` (since v1.6.0)** — "Provides the deprecation reason for a function." Supports soft vs hard deprecations.
- **`@file`** — "Changes the filename used in stacktraces for the function or macro that follows the attribute."
- **`@vsn`** — "Specify the module version."
- **`@nifs` (since v1.16.0)** — "A list of functions and their arities which will be overridden by a native implementation (NIF)."

### Struct-related attributes

From [Module.html#module-struct-attributes](https://hexdocs.pm/elixir/Module.html):

> "@derive - derives an implementation for the given protocol for the struct defined in the current module"
> "@enforce_keys - ensures the given keys are always set when building the struct defined in the current module"

Cross-reference the `## Structs` section for usage, and the `## Protocols` section for `@derive` mechanics.

### When to use which attribute

| Need | Choose | Notes |
|---|---|---|
| Document a module | `@moduledoc` | String or `false`; only visible in compiled modules. |
| Document a function/macro | `@doc` | String, `false`, or keyword list. |
| Document a type | `@typedoc` | Pairs with `@type`/`@opaque`. |
| Typespec a function | `@spec` | Cross-reference `docs/elixir/typespecs.md`. |
| Declare a behaviour | `@behaviour` | British spelling. |
| Mark a callback impl | `@impl true` | Auto-sets `@doc false`; "mark one, mark all". |
| Declare a callback | `@callback`/`@macrocallback` | In the behaviour module. |
| Mark optional callbacks | `@optional_callbacks` | `name: arity` pairs. |
| Compile-time constant in a pattern/guard | `@const` | Where functions can't go; prefer `defp` otherwise. |
| Accumulate annotations | `Module.register_attribute/3` with `:accumulate` | New values pushed to top. |
| Persist for Erlang interop | `:persist` option | Stored in BEAM Abstract Format. |

### Gotchas

- **The read-vs-set newline gotcha:** `@attr` (read) vs `@attr value` (set) are distinguished by a newline — do not add a newline between the attribute and its value, or the parser treats it as a read.
- **Attributes cannot reference same-module functions** — functions don't exist yet at attribute-resolution time.
- **Reading an attribute in many functions = many snapshots = slower compilation** — use a `defp` helper that reads the attribute once.
- **`@impl true` auto-sets `@doc false`** — set `@doc` explicitly if you need docs on a callback.
- **`@behaviour` is British spelling** — `@behavior` (American) is not recognized.
- **`:accumulate` and `:persist` cannot be reverted** once set on a registered attribute.
- **Prefer functions over attributes for plain constants** — `defp hours_in_a_day, do: 24` is preferred over `@hours_in_a_day 24` unless you need pattern/guard injection.

Cross-reference the `## Alias, Require, and Import` section for `use` (which often sets `@behaviour` and `@impl` via `__using__/1`), and the `## Structs` section for `@enforce_keys`/`@derive`.

## Structs

Structs are extensions built on top of maps that provide compile-time checks and default values. They are the idiomatic way to model named, typed records in Elixir. From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs are extensions built on top of maps that provide compile-time checks and default values."

Cross-reference the `## Keywords and Maps` → `Struct maps` subsection for the earlier map-based introduction, and the `## Protocols` section for how structs interact with protocol dispatch.

### `defstruct` and default fields

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "To define a struct, the `defstruct/1` construct is used:
> ```
> iex> defmodule User do
> ...>   defstruct name: "John", age: 27
> ...> end
> ```"

> "The keyword list used with `defstruct` defines what fields the struct will have along with their default values. Structs take the name of the module they're defined in."

Three syntaxes for default fields:

1. **Keyword with defaults:** `defstruct name: "John", age: 27`
2. **Atom list, all default to `nil`:** `defstruct [:name, :age]` → both default to `nil`.
3. **Mixed:** `defstruct [:email, name: "John", age: 27]` → `email` defaults to `nil`, others have explicit defaults.

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "If you don't specify a default key value when defining a struct, `nil` will be assumed:
> ```
> iex> defmodule Product do
> ...>   defstruct [:name]
> ...> end
> iex> %Product{}
> %Product{name: nil}
> ```"

> "You can define a structure combining both fields with explicit default values, and implicit `nil` values. In this case you must first specify the fields which implicitly default to `nil`:
> ```
> iex> defmodule User do
> ...>   defstruct [:email, name: "John", age: 27]
> ...> end
> iex> %User{}
> %User{age: 27, email: nil, name: "John"}
> ```"

> "Doing it in reverse order will raise a syntax error:
> ```
> iex> defstruct [name: "John", age: 27, :email]
> ** (SyntaxError) iex:107: unexpected expression after keyword list. Keyword lists must always come last in lists and maps.
> ```"

**Rule:** keyword-style pairs must come last. `[name: "John", age: 27, :email]` raises `SyntaxError`.

### Compile-time field guarantees

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs provide *compile-time* guarantees that only the fields defined through `defstruct` will be allowed to exist in a struct:
> ```
> iex> %User{oops: :field}
> ** (KeyError) key :oops not found expanding struct: User.__struct__/1
> ```"

### Accessing and updating structs

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs share the same syntax for accessing and updating fields as maps of fixed keys:
> ```
> iex> john = %User{}
> %User{age: 27, name: "John"}
> iex> john.name
> "John"
> iex> jane = %{john | name: "Jane"}
> %User{age: 27, name: "Jane"}
> iex> %{jane | oops: :field}
> ** (KeyError) key :oops not found in: %User{age: 27, name: "Jane"}
> ```"

> "When using the update syntax (`|`), Elixir is aware that no new keys will be added to the struct, allowing the maps underneath to share their structure in memory. In the example above, both `john` and `jane` share the same key structure in memory."

The `%{struct | key: value}` update syntax does NOT allow new keys and enables memory sharing because the key structure is preserved.

### Pattern matching with structs

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs can also be used in pattern matching, both for matching on the value of specific keys as well as for ensuring that the matching value is a struct of the same type as the matched value.
> ```
> iex> %User{name: name} = john
> %User{age: 27, name: "John"}
> iex> name
> "John"
> iex> %User{} = %{}
> ** (MatchError) no match of right hand side value: %{}
> ```"

From [Kernel.SpecialForms.html#%/2](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#%25/2):

> "A struct is a tagged map that allows developers to provide default values for keys, tags to be used in polymorphic dispatches and compile time assertions."

> "Underneath a struct is a map with a `:__struct__` key pointing to the `User` module, where the keys are validated at compile-time:
> ```
> %User{} == %{__struct__: User, name: "john", age: 27}
> ```"

Structs also allow pattern matching on the struct name:

```elixir
iex> %struct_name{} = john
%User{age: 27, name: "John"}
iex> struct_name
User
```

You can assign the struct name to `_` when you want to check that something is a struct but don't care about its name:

```elixir
iex> %_{} = john
%User{age: 27, name: "John"}
```

**Gotcha:** `%User{} = %{}` fails with `MatchError` — struct pattern matching checks BOTH key existence and struct identity.

### `@enforce_keys`

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "You can also enforce that certain keys have to be specified when creating the struct via the `@enforce_keys` module attribute:
> ```
> iex> defmodule Car do
> ...>   @enforce_keys [:make]
> ...>   defstruct [:model, :make]
> ...> end
> iex> %Car{}
> ** (ArgumentError) the following keys must also be given when building struct Car: [:make]
>     expanding struct: Car.__struct__/1
> ```"

> "Enforcing keys provides a simple compile-time guarantee to aid developers when building structs. **It is not enforced on updates** and it does not provide any sort of value-validation."

**Gotchas:**

- `@enforce_keys` only fires on FULL construction (`%Car{...}`), NOT on the update syntax (`%{car | model: "Corolla"}`).
- It is NOT a validator — only checks presence, not value validity.
- The error happens at struct construction (expansion) time.

### Structs are bare maps underneath

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs are simply maps with a 'special' field named `__struct__` that holds the name of the struct:
> ```
> iex> is_map(john)
> true
> iex> john.__struct__
> User
> ```"

> "However, structs do not inherit any of the protocols that maps do. For example, you can neither enumerate nor access a struct:
> ```
> iex> john[:name]
> ** (UndefinedFunctionError) function User.fetch/2 is undefined (User does not implement the Access behaviour)
>              User.fetch(%User{age: 27, name: "John"}, :name)
> iex> Enum.each(john, fn {field, value} -> IO.puts(value) end)
> ** (Protocol.UndefinedError) protocol Enumerable not implemented for %User{age: 27, name: "John"} of type User (a struct)
> ```"

> "Structs alongside protocols provide one of the most important features for Elixir developers: data polymorphism."

### Structs vs maps

- `is_map(struct) == true` — a struct IS a map.
- `struct.__struct__` returns the module atom.
- `john[:name]` raises `UndefinedFunctionError` (User does not implement the Access behaviour).
- `Enum.each(john, ...)` raises `Protocol.UndefinedError` (User doesn't implement Enumerable).
- Structs **do not inherit** `Map`, `Enumerable`, `Collectable`, `Inspect`, or `Access` implementations. You must derive or implement them explicitly per struct.

### `struct!/2` vs `struct/2` for dynamic updates

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "When you need to update structs with data from keyword lists or maps, use `Kernel.struct!/2`:
> ```
> iex> john = %User{name: "John", age: 27}
> %User{age: 27, name: "John"}
> iex> updates = [name: "Jane", age: 30]
> [name: "Jane", age: 30]
> iex> struct!(john, updates)
> %User{age: 30, name: "Jane"}
> ```"

> "`struct!/2` will raise an error if you try to set invalid fields:
> ```
> iex> struct!(john, invalid: "field")
> ** (KeyError) key :invalid not found in: %User{name: "John", age: 27}
> ```"

> "Use the map update syntax (`%{john | name: "Jane"}`) when you know the exact fields at compile time. Always use `struct!/2` instead of `Map` functions to preserve struct integrity."

`Kernel.struct/2` (no bang) also exists but does NOT raise on invalid keys — it silently drops them. **Always prefer `struct!/2`** to preserve integrity.

### `@derive`

From [structs.html](https://hexdocs.pm/elixir/structs.html) and [Module.html](https://hexdocs.pm/elixir/Module.html): `@derive` auto-implements protocols for a struct based on the protocol's `Any` implementation (or a custom `__deriving__/2` macrocallback).

```elixir
defmodule User do
  @derive [Inspect]
  defstruct name: "John", age: 27
end
```

For `@derive` to work, the protocol must have a `for: Any` implementation OR a custom `__deriving__/2` macrocallback. Cross-reference the `## Protocols` section for the full mechanics.

### When to use structs vs maps

| Need | Choose | Notes |
|---|---|---|
| Named, typed record with compile-time field checks | Struct | `defstruct`; `%Struct{}` literal. |
| Ad-hoc key-value data | Map | Any key type; no field validation. |
| Known-shape data with atom keys | Map with `map.key` OR struct | Structs add compile-time checks + `__struct__` tag. |
| Dynamic update from external input | `struct!/2` | Raises on invalid keys; never `Map.put/3`. |
| Required fields at construction | `@enforce_keys` | Construction-time only; not a validator. |
| Custom protocol impls | `@derive` or `defimpl` | Requires `for: Any` or `__deriving__/2`. |

### Gotchas

- **`@enforce_keys` only fires on full construction**, not on `%{s | key: val}` updates.
- **`struct/2` (no bang) silently drops unknown keys** — always use `struct!/2` to preserve integrity.
- **`%User{} = %{}` fails** — struct pattern matching checks both key existence and struct identity.
- **Updates via `%{s | key: val}` share memory** because the key list is preserved (no key reorder).
- **`defstruct` keyword pairs MUST come after plain atom entries** in a mixed list — or you get `SyntaxError`.
- **Structs do NOT inherit Map/Enumerable/Access/Inspect** — derive or implement explicitly.
- **`is_map(struct) == true`** — a struct is a map, but the reverse is not true.

Cross-reference the `## Module Attributes` section for `@enforce_keys`/`@derive`, and the `## Protocols` section for implementing protocols for structs (including the `:for`-omission rule when `defimpl` is inside the struct module).

## Protocols

Protocols are Elixir's mechanism for data-type polymorphism: a protocol declares a set of functions, and any data type can provide an implementation. Unlike behaviours (which dispatch on the module name), protocols dispatch on the data type of the first argument. From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "Protocols are a mechanism to achieve polymorphism in Elixir where you want the behavior to vary depending on the data type. We are already familiar with one way of solving this type of problem: via pattern matching and guard clauses."

> "This is where protocols can help us: protocols allow us to extend the original behavior for as many data types as we need. That's because **dispatching on a protocol is available to any data type that has implemented the protocol** and a protocol can be implemented by anyone, at any time."

Cross-reference the `## Structs` section for how structs interact with protocols (structs do NOT inherit Map protocol implementations), and the `## Module Attributes` section for `@behaviour`/`@callback` (the compile-time counterpart).

### `defprotocol`

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "We define the protocol using `defprotocol/2` - its functions and specs may look similar to interfaces or abstract base classes in other languages. We can add as many implementations as we like using `defimpl/3`."

From [Kernel.html#defprotocol/2](https://hexdocs.pm/elixir/Kernel.html#defprotocol/2):

> `defprotocol(name, do_block)` — "Defines a protocol."

Inside a protocol, only bare `def` declarations (no body) are allowed:

```elixir
defprotocol Size do
  @doc "Calculates the size (and not the length!) of a data structure"
  def size(data)
end
```

From [Protocol.html](https://hexdocs.pm/elixir/Protocol.html):

> "Defining a protocol automatically defines a zero-arity type named `t`, which can be used as follows:
> ```
> @spec print_size(Size.t()) :: :ok
> ```"

### `defimpl`

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

```elixir
defimpl Size, for: BitString do
  def size(string), do: byte_size(string)
end

defimpl Size, for: Map do
  def size(map), do: map_size(map)
end

defimpl Size, for: Tuple do
  def size(tuple), do: tuple_size(tuple)
end
```

From [Kernel.html#defimpl/3](https://hexdocs.pm/elixir/Kernel.html#defimpl/3):

> `defimpl(name, opts, do_block \\ [])` — "Defines an implementation for the given protocol."

Options:

- `for: Type` (or `for: [Type1, Type2, ...]` for multi-impl) — required.
- `for:` can be omitted if `defimpl` is inside the struct's module.

From [Protocol.html](https://hexdocs.pm/elixir/Protocol.html):

> "Inside `defimpl/3`, you can use `@protocol` to access the protocol being implemented and `@for` to access the module it is being defined for."

### Dispatch is on the FIRST argument only

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "With protocols, however, we are no longer stuck having to continuously modify the same module to support more and more data types. […] Functions defined in a protocol may have more than one input, but the **dispatching will always be based on the data type of the first input**."

So `MyProto.do_something(%User{}, other_thing)` dispatches based on `%User{}` (the first argument).

### Missing implementations raise

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "Passing a data type that doesn't implement the protocol raises an error:
> ```
> iex> Size.size([1, 2, 3])
> ** (Protocol.UndefinedError) protocol Size not implemented for [1, 2, 3] of type List
> ```"

### Implementable types

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "It's possible to implement protocols for all Elixir data types:
> - `Atom`
> - `BitString`
> - `Float`
> - `Function`
> - `Integer`
> - `List`
> - `Map`
> - `PID`
> - `Port`
> - `Reference`
> - `Tuple`"

### Protocols and structs

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "The power of Elixir's extensibility comes when protocols and structs are used together."

> "In the previous chapter, we have learned that although structs are maps, they do not share protocol implementations with maps. For example, `MapSet`s (sets based on maps) are implemented as structs. Let's try to use the `Size` protocol with a `MapSet`:
> ```
> iex> Size.size(%{})
> 0
> iex> set = %MapSet{} = MapSet.new
> MapSet.new([])
> iex> Size.size(set)
> ** (Protocol.UndefinedError) protocol Size not implemented for MapSet.new([]) of type MapSet (a struct)
> ```"

> "Since a `MapSet` has its size precomputed and accessible through `MapSet.size/1`, we can define a `Size` implementation for it:
> ```
> defimpl Size, for: MapSet do
>   def size(set), do: MapSet.size(set)
> end
> ```"

From [Protocol.html#module-protocols-and-structs](https://hexdocs.pm/elixir/Protocol.html):

> "When implementing a protocol for a struct, the `:for` option can be omitted if the `defimpl/3` call is inside the module that defines the struct:
> ```
> defmodule User do
>   defstruct [:email, :name]
>
>   defimpl Size do
>     # two fields
>     def size(%User{}), do: 2
>   end
> end
> ```"

### Multiple types in one `defimpl`

From [Protocol.html#module-multiple-implementations](https://hexdocs.pm/elixir/Protocol.html):

> "Protocols can also be implemented for multiple types at once:
> ```
> defprotocol Reversible do
>   def reverse(term)
> end
>
> defimpl Reversible, for: [Map, List] do
>   def reverse(term), do: Enum.reverse(term)
> end
> ```"

### Deriving and `@fallback_to_any`

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "Manually implementing protocols for all types can quickly become repetitive and tedious. In such cases, Elixir provides two options: we can explicitly derive the protocol implementation for our types or automatically implement the protocol for all types. In both cases, we need to implement the protocol for `Any`."

#### Deriving

```elixir
defimpl Size, for: Any do
  def size(_), do: 0
end

defmodule OtherUser do
  @derive [Size]
  defstruct [:name, :age]
end
```

> "When deriving, Elixir will implement the `Size` protocol for `OtherUser` based on the implementation provided for `Any`."

For `@derive` to work, the protocol must have a `for: Any` implementation OR a custom `__deriving__/2` macrocallback (used by protocols like `Inspect` for custom deriving logic).

#### Fallback to Any

```elixir
defprotocol Size do
  @fallback_to_any true
  def size(data)
end
```

> "Another alternative to `@derive` is to explicitly tell the protocol to fallback to `Any` when an implementation cannot be found. This can be achieved by setting `@fallback_to_any` to `true` in the protocol definition"

> "As we said in the previous section, the implementation of `Size` for `Any` is not one that can apply to any data type. That's one of the reasons why `@fallback_to_any` is an opt-in behavior. For the majority of protocols, raising an error when a protocol is not implemented is the proper behavior."

> "Now all data types (including structs) that have not implemented the `Size` protocol will be considered to have a size of `0`."

> "Which technique is best between deriving and falling back to `Any` depends on the use case but, given Elixir developers prefer explicit over implicit, you may see many libraries pushing towards the `@derive` approach."

### Protocol configuration attributes

From [Protocol.html](https://hexdocs.pm/elixir/Protocol.html):

> "The following module attributes are available to configure a protocol:
> - `@fallback_to_any` - when true, enables protocol dispatch to fallback to any
> - `@undefined_impl_description` - a string with additional description to be used on `Protocol.UndefinedError` when looking up the implementation fails. This option is only applied if `@fallback_to_any` is not set to true"

### Built-in protocols

From [protocols.html](https://hexdocs.pm/elixir/protocols.html):

> "Elixir ships with some built-in protocols. In previous chapters, we have discussed the `Enum` module which provides many functions that work with any data structure that implements the `Enumerable` protocol:
> ```
> iex> Enum.map([1, 2, 3], fn x -> x * 2 end)
> [2, 4, 6]
> ```"

> "Another useful example is the `String.Chars` protocol, which specifies how to convert a data structure to its human representation as a string. It's exposed via the `to_string` function:
> ```
> iex> to_string(:hello)
> "hello"
> ```"

> "Notice that string interpolation in Elixir calls the `to_string` function:
> ```
> iex> "age: #{25}"
> "age: 25"
> ```"

> "When there is a need to 'print' a more complex data structure, one can use the `inspect` function, based on the `Inspect` protocol:
> ```
> iex> "tuple: #{inspect(tuple)}"
> "tuple: {1, 2, 3}"
> ```"

> "The `Inspect` protocol is the protocol used to transform any data structure into a readable textual representation. This is what tools like IEx use to print results"

> "Keep in mind that, by convention, whenever the inspected value starts with `#`, it is representing a data structure in non-valid Elixir syntax. This means the inspect protocol is not reversible as information may be lost along the way:
> ```
> iex> inspect &(&1+2)
> "#Function<6.71889879/1 in :erl_eval.expr/5>"
> ```"

The built-in protocols (from [Kernel.html](https://hexdocs.pm/elixir/Kernel.html)):

| Protocol | Purpose | Required function(s) |
|---|---|---|
| `Enumerable` | Handles collections (powers `Enum`/`Stream`). | `reduce/3` (core) + `count/1`, `member?/2`, `slice/1` (optimizations). |
| `Collectable` | Collects data into a data type (powers `Enum.into/2`). | `into/1`. |
| `Inspect` | Converts data to a readable programming representation. | `inspect/2`. |
| `String.Chars` | Converts data to its outside-world string representation. | `to_string/1`. |
| `List.Chars` | Converts data to its outside-world charlist representation. | `to_charlist/1`. |

From [Enumerable.html](https://hexdocs.pm/elixir/Enumerable.html):

> "This protocol requires four functions to be implemented, `reduce/3`, `count/1`, `member?/2`, and `slice/1`. The core of the protocol is the `reduce/3` function. All other functions exist as optimizations paths for data structures that can implement certain properties in better than linear time."

From [Collectable.html](https://hexdocs.pm/elixir/Collectable.html):

> "The `Enum.into/2` function uses this protocol to insert an enumerable into a collection."

From [Inspect.html](https://hexdocs.pm/elixir/Inspect.html):

> "The Inspect protocol can be derived to customize the order of fields (the default is alphabetical) and hide certain fields from structs, so they don't show up in logs, inspects and similar. The latter is especially useful for fields containing private information."

> "The supported options are: `:only`, `:except`, `:optional` (since v1.14.0). Since v1.19.0, the `:all` atom can be passed to mark all fields as optional."

> "Whenever `:only` or `:except` are used to restrict fields, the struct will be printed using the `#User<...>` notation, as the struct can no longer be copy and pasted as valid Elixir code."

```elixir
defmodule User do
  @derive {Inspect, only: [:name]}
  defstruct [:name, :email, :password]
end
```

### Protocol consolidation

From [Protocol.html#module-consolidation](https://hexdocs.pm/elixir/Protocol.html):

> "In order to speed up protocol dispatching, whenever all protocol implementations are known up-front, typically after all Elixir code in a project is compiled, Elixir provides a feature called *protocol consolidation*. Consolidation directly links protocols to their implementations in a way that invoking a function from a consolidated protocol is equivalent to invoking two remote functions - one to identify the correct implementation, and another to call the implementation."

> "Protocol consolidation is applied by default to all Mix projects during compilation. This may be an issue during test. For instance, if you want to implement a protocol during test, the implementation will have no effect, as the protocol has already been consolidated."

```elixir
def project do
  consolidate_protocols: Mix.env() != :test
end
```

> "Finally, note all protocols are compiled with `debug_info` set to `true`, regardless of the option set by the `elixirc` compiler. The debug info is used for consolidation and it is removed after consolidation unless globally set."

**Testing gotcha:** Implementations defined in `test/` do not take effect after consolidation. Two workarounds:

1. Set `consolidate_protocols: Mix.env() != :test` in `mix.exs` to disable consolidation in the test env.
2. Add `test/support` to `elixirc_paths(:test)` so test-time impls compile together with the rest.

For `Mix.install/2`:

```elixir
Mix.install(deps, consolidate_protocols: false)
```

### Protocols vs behaviours

| Aspect | Protocol | Behaviour |
|---|---|---|
| Dispatch basis | Data type of the FIRST argument. | Module name (you pass the module around). |
| Extensibility | Open — any data type can implement, at any time. | Closed at compile time — callback modules are known. |
| Contract location | `defprotocol` with bare `def` declarations. | `@callback`/`@macrocallback` in a module. |
| When to use | Cross-cutting polymorphic operations over data types. | Fixed set of implementations known at compile time. |

Use behaviours when you have a fixed set of implementations known at compile time. Use protocols when you want open-ended extensibility across data types.

### When to use protocols

| Need | Choose | Notes |
|---|---|---|
| Polymorphic operation over many data types | Protocol | Dispatch on first arg; open extensibility. |
| Compile-time contract for callback modules | Behaviour | `@behaviour` + `@callback`; cross-reference `## Module Attributes`. |
| Auto-implement a protocol for a struct | `@derive` | Requires `for: Any` or `__deriving__/2`. |
| Fallback for all unimplemented types | `@fallback_to_any true` | Opt-in; prefer `@derive` (explicit over implicit). |
| Custom struct inspection | `@derive {Inspect, only:/except:/optional:}` | Hides fields; uses `#User<...>` notation. |
| String conversion of a custom type | Implement `String.Chars` | Powers `to_string/1` and interpolation. |

### Gotchas

- **Missing implementation raises `Protocol.UndefinedError`** — the default is to raise, not silently return `nil`.
- **Structs do NOT inherit Map/Enumerable/etc implementations** — implement or `@derive` explicitly per struct.
- **No `@callback` inside `defprotocol`** — protocols are declared with bare `def name(args)` (no body).
- **Default arguments in `defprotocol` are deprecated** — the compiler emits a warning.
- **Consolidation requires `:debug_info`** — forced by `defprotocol` regardless of compiler settings.
- **`@derive` requires `for: Any` OR a custom `__deriving__/2` macrocallback** on the protocol.
- **`@fallback_to_any` is opt-in** — by default the framework prefers raising an error.
- **Dispatch is on the FIRST argument only** — multi-argument protocol functions still dispatch on the first.
- **Test-time implementations don't take effect after consolidation** — use `consolidate_protocols: Mix.env() != :test`.

Cross-reference the `## Structs` section for `@derive` and the `MapSet` example, and the `## Module Attributes` section for `@behaviour`/`@callback`/`@impl` (the compile-time counterpart to protocols).

## Comprehensions

Comprehensions are syntactic sugar for the common map/filter/collect pattern over enumerables, gathered into the `for` special form. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "In Elixir, it is common to loop over an `Enumerable`, often filtering out some results and mapping values into another list. Comprehensions are syntactic sugar for such constructs: they group those common tasks into the `for` special form."

> "A comprehension is made of three parts: generators, filters, and collectables."

Cross-reference `## Enumerables and Streams` (the `Enumerable` protocol) and `## Pattern Matching and Guards` (generators accept patterns and guards).

### Generators and filters

A generator is `pattern <- enumerable`. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "In the expression above, `n <- [1, 2, 3, 4]` is the **generator**. It is literally generating values to be used in the comprehension. Any enumerable can be passed on the right-hand side of the generator expression"

```elixir
iex> for n <- [1, 2, 3, 4], do: n * n
[1, 4, 9, 16]
iex> for n <- 1..4, do: n * n
[1, 4, 9, 16]
```

The left side of a generator is a PATTERN; non-matching elements are silently ignored. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Generator expressions also support pattern matching on their left-hand side; all non-matching patterns are *ignored*."

```elixir
iex> values = [good: 1, good: 2, bad: 3, good: 4]
iex> for {:good, n} <- values, do: n * n
[1, 4, 16]
```

Filters are boolean expressions that keep only truthy elements. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Alternatively to pattern matching, filters can be used to select some particular elements."

> "Comprehensions discard all elements for which the filter expression returns `false` or `nil`; all other values are selected."

```elixir
iex> for n <- 0..5, rem(n, 3) == 0, do: n * n
[0, 9]
```

### Multiple generators

A comprehension may have several generators and filters; later generators nest inside earlier ones. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Comprehensions generally provide a much more concise representation than using the equivalent functions from the `Enum` and `Stream` modules. Furthermore, comprehensions also allow multiple generators and filters to be given."

> "Multiple generators can also be used to calculate the Cartesian product of two lists"

```elixir
iex> for i <- [:a, :b, :c], j <- [1, 2], do:  {i, j}
[a: 1, a: 2, b: 1, b: 2, c: 1, c: 2]
```

A realistic multi-generator example walks directories, lists files, binds a path, and filters on `File.regular?/1`:

```elixir
dirs = ["/home/mikey", "/home/james"]

for dir <- dirs,
    file <- File.ls!(dir),
    path = Path.join(dir, file),
    File.regular?(path) do
  File.stat!(path).size
end
```

Bindings made inside a comprehension (in generators, filters, or the block) do NOT leak to the surrounding scope. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Finally, keep in mind that variable assignments inside the comprehension, be it in generators, filters or inside the block, are not reflected outside of the comprehension."

### Bitstring generators

Generators can destructure binaries with `<<pattern <- binary>>`. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Bitstring generators are also supported and are very useful when you need to comprehend over bitstring streams."

> "A bitstring generator can be mixed with \"regular\" enumerable generators, and supports filters as well."

```elixir
iex> pixels = <<213, 45, 132, 64, 76, 32, 76, 0, 0, 234, 32, 15>>
iex> for <<r::8, g::8, b::8 <- pixels>>, do: {r, g, b}
[{213, 45, 132}, {64, 76, 32}, {76, 0, 0}, {234, 32, 15}]
```

Cross-reference the `::` row in `## Basic Operators` → `Special-form operators` and `## Pattern Matching and Guards` → `Pattern matching on strings and binaries` for bitstring segment modifiers.

### The `:into` option

By default a comprehension returns a list; `:into` collects results into any `Collectable` instead. From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "In the examples above, all the comprehensions returned lists as their result. However, the result of a comprehension can be inserted into different data structures by passing the `:into` option to the comprehension."

> "Sets, maps, and other dictionaries can also be given to the `:into` option. In general, `:into` accepts any structure that implements the `Collectable` protocol."

**Into a binary** (strip spaces):

```elixir
iex> for <<c <- " hello world ">>, c != ?\s, into: "", do: <<c>>
"helloworld"
```

**Into a map** (transform values):

```elixir
iex> for {key, val} <- %{"a" => 1, "b" => 2}, into: %{}, do: {key, val * val}
%{"a" => 1, "b" => 4}
```

> "A common use case of `:into` can be transforming values in a map"

**Into a stream** (an echo terminal that uppercases each line — note this loops forever):

```elixir
iex> stream = IO.stream(:stdio, :line)
iex> for line <- stream, into: stream do
...>   String.upcase(line) <> "\n"
...> end
```

> "Unfortunately, this example also got your IEx shell stuck in the comprehension, so you will need to hit `Ctrl+C` twice to get out of it. :)"

Cross-reference `## Protocols` → the `Collectable` protocol entry in `## Sources used`.

### Other options: `:reduce` and `:uniq`

From [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html):

> "Comprehensions support other options, such as `:reduce` and `:uniq`."

The getting-started page lists these without examples; the full reference is [`Kernel.SpecialForms.html#for/1`](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#for/1). `:reduce` accumulates a result across all generated values (replacing the default list collection with a user-supplied reducer), and `:uniq` discards duplicate values. Reach for `:reduce` when a comprehension should fold into a single value rather than build a list.

## Sigils

Sigils are Elixir's extensible literal syntax: `~` followed by a single lowercase letter or one or more uppercase letters, a delimiter, and optional modifiers. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "One of Elixir's goals is extensibility: developers should be able to extend the language to fit any particular domain. Sigils provide the foundation for extending the language with custom textual representations. Sigils start with the tilde (`~`) character which is followed by either a single lower-case letter or one or more upper-case letters, and then a delimiter. Optional modifiers are added after the final delimiter."

Eight delimiters are supported: `/`, `|`, `"`, `'`, `(`, `[`, `{`, `<`. The choice exists so literals can avoid escaped delimiters — for example `~r(^https?://)` reads better than `~r/^https?:\/\//`. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "The reason behind supporting different delimiters is to provide a way to write literals without escaped delimiters."

Cross-reference the `Strings (binaries)` and `Charlists (intro only)` subsections in `## Basic Types`.

### Regular expressions: `~r`

`~r` builds a PCRE regex. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "The most common sigil in Elixir is `~r`, which is used to create regular expressions"

> "Elixir provides Perl-compatible regular expressions (regexes), as implemented by the PCRE library. Regexes also support modifiers. For example, the `i` modifier makes a regular expression case insensitive"

```elixir
iex> regex = ~r/foo|bar/
~r/foo|bar/
iex> "foo" =~ regex
true
iex> "bat" =~ regex
false
iex> "HELLO" =~ ~r/hello/
false
iex> "HELLO" =~ ~r/hello/i
true
```

The eight delimiter forms are equivalent:

```elixir
~r/hello/
~r|hello|
~r"hello"
~r'hello'
~r(hello)
~r[hello]
~r{hello}
~r<hello>
```

Cross-reference the `=~` row in `## Basic Operators` → `Concatenation and membership` and the `Regex.html` entry in `## Sources used`.

### Strings, charlists, and word lists: `~s`, `~c`, `~w`

Three textual sigils build string-like data with a chosen delimiter. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "Elixir ships with three sigils for building textual data structures. These allow you to choose an appropriate delimiter for your literal text such that you do not have to worry about escaping."

**`~s` — strings** (like double quotes, useful when the text contains double quotes):

```elixir
iex> ~s(this is a string with "double" quotes, not 'single' ones)
"this is a string with \"double\" quotes, not 'single' ones"
```

> "The `~s` sigil is used to generate strings, like double quotes are. The `~s` sigil is useful when a string contains double quotes"

**`~c` — charlists** (the regular way to represent charlists):

```elixir
iex> [?c, ?a, ?t]
~c"cat"
iex> ~c(this is a char list containing "double quotes")
~c"this is a char list containing \"double quotes\""
```

> "The `~c` sigil is the regular way to represent charlists."

**`~w` — word lists** (whitespace-separated words as strings):

```elixir
iex> ~w(foo bar bat)
["foo", "bar", "bat"]
```

> "The `~w` sigil is used to generate lists of words (*words* are just regular strings). Inside the `~w` sigil, words are separated by whitespace."

`~w` accepts the `c`, `s`, and `a` modifiers to set the element type (charlists, strings, atoms):

```elixir
iex> ~w(foo bar bat)a
[:foo, :bar, :bat]
```

> "The `~w` sigil also accepts the `c`, `s` and `a` modifiers (for charlists, strings, and atoms, respectively), which specify the data type of the elements of the resulting list"

### Interpolation and escaping: `~s` vs `~S`

Lowercase textual sigils perform interpolation and process escape codes; their UPPERCASE variants (`~S`, `~C`, `~W`) do neither — the content is taken literally. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "Sigils also help deal with escaping characters and interpolation. In particular, uppercase-letter textual sigils do not perform interpolation nor escaping, and most lowercase sigils have an uppercase variant. For example, although both `~s` and `~S` will return strings, the former allows escape codes and interpolation while the latter does not"

```elixir
iex> ~s(String with escape codes \x26 #{"inter" <> "polation"})
"String with escape codes & interpolation"
iex> ~S(String without escape codes \x26 without #{interpolation})
"String without escape codes \\x26 without \#{interpolation}"
```

Supported escape codes (from [sigils.html](https://hexdocs.pm/elixir/sigils.html)): `\\` (backslash), `\a` (bell), `\b` (backspace), `\d` (delete), `\e` (escape), `\f` (form feed), `\n` (newline), `\r` (carriage return), `\s` (space), `\t` (tab), `\v` (vertical tab), `\0` (null byte), `\xDD` (single byte in hex), `\uDDDD` and `\u{D...}` (Unicode codepoint in hex).

### Heredocs

Sigils support heredoc delimiters — three double-quotes or three single-quotes — for multi-line literals. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "Sigils also support heredocs, that is, three double-quotes or single-quotes as separators"

```elixir
iex> ~s"""
...> this is
...> a heredoc string
...> """
```

The dominant heredoc use case is docstrings, where `~S"""` avoids double-escaping backslashes and interpolation in example code. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "The most common use case for heredoc sigils is when writing documentation. For example, writing escape characters in the documentation would soon become error prone because of the need to double-escape some characters"

> "By using `~S`, this problem can be avoided altogether"

```elixir
@doc ~S"""
Converts double-quotes to single-quotes.

## Examples

    iex> convert("\"foo\"")
    "'foo'"

"""
def convert(...)
```

### Calendar sigils: `~D`, `~T`, `~N`, `~U`

Four sigils build calendar structs directly.

**`~D` — `Date`** (`year`, `month`, `day`, `calendar`):

```elixir
iex> d = ~D[2019-10-31]
~D[2019-10-31]
iex> d.day
31
```

> "A `Date` struct contains the fields `year`, `month`, `day`, and `calendar`. You can create one using the `~D` sigil"

**`~T` — `Time`** (`hour`, `minute`, `second`, `microsecond`, `calendar`):

```elixir
iex> t = ~T[23:00:07.0]
~T[23:00:07.0]
iex> t.second
7
```

> "The `Time` struct contains the fields `hour`, `minute`, `second`, `microsecond`, and `calendar`. You can create one using the `~T` sigil"

**`~N` — `NaiveDateTime`** (Date + Time fields, NO timezone):

```elixir
iex> ndt = ~N[2019-10-31 23:00:07]
~N[2019-10-31 23:00:07]
```

> "The `NaiveDateTime` struct contains fields from both `Date` and `Time`. You can create one using the `~N` sigil"

> "Why is it called naive? Because it does not contain timezone information. Therefore, the given datetime may not exist at all or it may exist twice in certain timezones - for example, when we move the clock back and forward for daylight saving time."

**`~U` — `DateTime` in UTC** (NaiveDateTime fields plus timezone fields):

```elixir
iex> dt = ~U[2019-10-31 19:59:03Z]
~U[2019-10-31 19:59:03Z]
iex> %DateTime{minute: minute, time_zone: time_zone} = dt
~U[2019-10-31 19:59:03Z]
iex> minute
59
iex> time_zone
"Etc/UTC"
```

> "A `DateTime` struct contains the same fields as a `NaiveDateTime` with the addition of fields to track timezones. The `~U` sigil allows developers to create a DateTime in the UTC timezone"

Cross-reference `## Structs` for struct pattern matching with `%DateTime{...}`.

### Custom sigils

Sigils desugar to a `sigil_{character}` function/macro call taking the content (as a binary or charlist) and a list of modifier charcodes. From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "As hinted at the beginning of this chapter, sigils in Elixir are extensible. In fact, using the sigil `~r/foo/i` is equivalent to calling `sigil_r` with a binary and a char list as the argument"

```elixir
iex> sigil_r(<<"foo">>, [?i])
~r"foo"i
```

> "We can also provide our own sigils by implementing functions that follow the `sigil_{character}` pattern."

A custom sigil defines `sigil_{char}/2` clauses; modifiers arrive as a charlist, so `?n` matches the `n` modifier:

```elixir
iex> defmodule MySigils do
...>   def sigil_i(string, []), do: String.to_integer(string)
...>   def sigil_i(string, [?n]), do: -String.to_integer(string)
...> end
iex> import MySigils
iex> ~i(13)
13
iex> ~i(42)n
-42
```

> "Custom sigils may be either a single lowercase character, or an uppercase character followed by more uppercase characters and digits. In practice, they are often used to embed templating languages or even represent regular languages within Elixir itself. If you're interested in learning more, check out how sigils are implemented in the `Kernel` module (where the `sigil_*` functions/macros are defined) for a starting point."

Cross-reference `## Modules and Functions` for `def`/multi-clause dispatch and `## Alias, Require, and Import` for `import` (custom sigils must be imported before use, like any function).

## Review checklist

- [ ] Division uses `/` for a float result and `div`/`rem` for integers — never assume `/` yields an integer.
- [ ] Strings are UTF-8 binaries — use `String.t()`, not the discouraged `string()` typespec.
- [ ] Charlists use `~c"..."`, not the deprecated `'...'` syntax.
- [ ] `map.key` is used only on known atom keys; `map[:key]` is used for optional keys.
- [ ] `%{map | key: value}` is only used when the key is known to already exist.
- [ ] `size` functions are used for O(1) sizing and `length` functions for O(n) traversal.
- [ ] Keyword lists are treated as lists — don't pattern match on them by exact shape.
- [ ] Lists are prepended; repeated `++` append in loops is avoided.
- [ ] Tagged tuples `{:ok, _}` / `{:error, _}` are used for fallible operations.
- [ ] `and`, `or`, `not` are used for booleans; `&&`, `||`, `!` are used for truthy values.
- [ ] `===` strict equality is used when integer vs float type distinction matters.
- [ ] All values are immutable; "mutating" operations return new data.
- [ ] `++`/`--` are right-associative — parenthesize when mixing with `--` to avoid surprises.
- [ ] `not x in list` (deprecated) is written as `x not in list`.
- [ ] Comparison of structs (e.g. dates) uses `Date.compare/2` / module-aware `Enum.sort/2`, not raw `<`/`>`.
- [ ] `<>` is used only on binaries (strings); charlists use `++`.
- [ ] `and`/`or`/`not` are used in guards (they're guard-legal); `&&`/`||`/`!` are NOT guard-legal.
- [ ] The pipe `|>` is used to express left-to-right data flow instead of deeply nested calls.
- [ ] The `=` operator is treated as MATCH (left = pattern, right = value), not assignment; patterns only appear on the left of `=`.
- [ ] The pin operator `^` is used to compare against an existing binding instead of rebinding it.
- [ ] Map patterns are understood as SUBSET matches; `%{}` matches any map, while tuple/list patterns require exact size.
- [ ] Guards must evaluate to literally `true`; `and`/`or`/`not` are guard-legal, `&&`/`||`/`!` are NOT.
- [ ] A guard that raises is treated as a failing (false) guard, not an exception; rely on multiple `when` clauses when a guard BIF may raise.
- [ ] Only guard-safe functions/operators are used in `when` clauses; arbitrary user functions are never called inside guards.
- [ ] Custom reusable guard predicates are defined with `defguard`/`defguardp` rather than ad-hoc inline expressions.
- [ ] In a binary/string pattern, the left side of `<>` is a literal prefix only; suffix matches are not valid patterns.
- [ ] `case` / `cond` / `with` / `if` are treated as EXPRESSIONS that return a value; a branch result the caller needs is captured, not discarded.
- [ ] `cond` ends with a final `true ->` fallback clause (or is replaced by `if/2` when there are only two branches) to avoid a runtime `CondClauseError`.
- [ ] Nested `case` expressions are collapsed into a single `with/1` chain wherever a step's failure should short-circuit.
- [ ] Inside `with`, the `<-` operator is used where a mismatch should short-circuit; a bare `=` is used only for guaranteed matches (it raises `MatchError` and does NOT short-circuit).
- [ ] A `with ... else` block matches on the FAILED value only (intermediate `<-` bindings are out of scope in `else`); failures are tagged upstream so the `else` stays simple.
- [ ] `case`/`with` clauses pin existing variables with `^` when matching against a bound value is intended (a bare name rebinds).
- [ ] `if`/`cond`/`unless` conditions rely only on truthiness; `0`, `""`, and `[]` are NOT treated as falsy (only `false` and `nil` are).
- [ ] `unless/2` is not written with an `else` block; a two-branch negative condition uses `if` (or `if !cond`), and new code treats `unless/2` as soft-deprecated.
- [ ] A function with default arguments (`\\`) AND multiple clauses has a function head (no body, no patterns, no guards) declaring the defaults.
- [ ] Default argument expressions are understood to re-evaluate on every call that needs them (not once at definition time).
- [ ] `def`/`defp` naming follows conventions: `?` suffix = boolean return, `!` suffix = may raise.
- [ ] `alias` is preferred over `import` in application code; `import` always uses `:only`/`:except`.
- [ ] `use` is reserved for known extension points (ExUnit, GenServer, Supervisor, Logger); its docs are read before relying on side effects.
- [ ] `@behaviour` uses the British spelling; `@impl true` is applied to ALL callbacks of a behaviour once one is marked ("mark one, mark all").
- [ ] Module attributes used as constants in patterns/guards are preferred over `defp` ONLY when injection into a pattern/guard is required; otherwise `defp` is preferred.
- [ ] Reading the same module attribute in many functions is avoided (snapshot rule); a `defp` helper reads it once.
- [ ] `struct!/2` (not `struct/2` and not `Map.put/3`) is used for dynamic struct updates to preserve integrity.
- [ ] `@enforce_keys` is understood to fire only at construction, not on `%{s | k: v}` updates, and is not a value validator.
- [ ] Structs are understood to NOT inherit Map/Enumerable/Access/Inspect — protocols are derived or implemented explicitly.
- [ ] Protocol dispatch is on the FIRST argument only; missing implementations raise `Protocol.UndefinedError`.
- [ ] Protocol implementations in `test/` are confirmed to take effect (consolidation disabled in test env via `consolidate_protocols: Mix.env() != :test`).

## Implementation checklist

- [ ] When an integer result is needed from division, use `div/2` (and `rem/2` for modulus).
- [ ] Prepend to lists with `[head | tail]`; avoid repeated `++` append in recursive loops.
- [ ] Pick the collection by cardinality: list = variable size, tuple = fixed size.
- [ ] For optional function options, accept a keyword list as the final argument.
- [ ] Return `{:ok, value}` / `{:error, reason}` tuples for fallible operations.
- [ ] Use the appropriate `is_*` predicate for type checks inside guards.
- [ ] Prefer `String.t()` in typespecs and avoid `string()` for Elixir strings.
- [ ] Prefer `~c"..."` over `'...'` for charlists.
- [ ] Use `map[:key]` for optional keys and `map.key` only when the key is guaranteed to exist.

## Validation hooks

- `iex` / `mix run` — validate type behavior interactively in the REPL or a script. Official Elixir tooling.
- `mix format --check-formatted` — official formatter; catches layout and some syntactic issues.
- `mix credo` — community linter; flags non-idiomatic patterns such as discouraged `string()` typespec usage or non-idiomatic collection choices.
- `mix dialyzer` — static analysis tool (community) for typespec checks; will flag `string()` misuse and type mismatches.
- `mix compile --warnings-as-errors` — promotes compiler warnings (e.g. the `string()` typespec warning, deprecated `'...'` charlist warnings on newer versions) to errors.
- Compiler warnings and errors — official; surfacing at compile time for syntax, type, and deprecation issues.

## Examples

### Arithmetic and division

```elixir
iex> 10 / 2
5.0
iex> div(10, 2)
5
iex> rem(10, 3)
1
```

### Tagged-tuple error handling

```elixir
iex> case File.read("README.md") do
...>   {:ok, body} -> String.length(body)
...>   {:error, reason} -> {:error, reason}
...> end
# returns an integer on success or {:error, reason} on failure
```

### Map access, bracket vs dot, and update

```elixir
iex> map = %{a: 1, b: 2}
iex> map[:a]    # safe, returns nil if missing
1
iex> map.a      # raises KeyError if :a were absent
1
iex> %{map | a: 99}
%{a: 99, b: 2}
```

### List prepend vs append

```elixir
iex> list = [2, 3]
iex> [1 | list]      # cheap prepend
[1, 2, 3]
iex> list ++ [4]     # slow: traverses entire list
[2, 3, 4]
```

### Choosing list vs tuple

```elixir
iex> String.split("a b c")    # unknown number of parts → list
["a", "b", "c"]
iex> String.split_at("abc", 1) # fixed-size result → tuple
{"a", "bc"}
```

## Common mistakes

- **Assuming `10 / 2` is an integer.** `/` always returns a float; use `div/2` for integer division.
- **Using `string()` as a typespec for Elixir strings.** It means Erlang charlists; use `String.t()`.
- **Using `'...'` for strings or modern charlists.** Prefer double quotes `"..."` for strings and `~c"..."` for charlists.
- **Mixing `<>` (binaries) and `++` (lists).** `<>` concatenates binaries; `++` concatenates lists. Using the wrong operator raises.
- **Using `map.key` on a possibly absent key.** Use `Map.get/3` or `map[:key]` for optional keys.
- **Pattern matching keyword lists by exact shape.** Keyword lists are lists; order and duplicate keys are allowed, so matching `[a: x]` is fragile.
- **Appending to a list in a loop.** Each `list ++ [x]` is O(n); build with cons and reverse once at the end.
- **Using `==` where `===` strictness matters.** `1 == 1.0` is `true`, but `1 === 1.0` is `false`.
- **Treating `0`, `""`, or `[]` as falsy.** Only `false` and `nil` are falsy in Elixir.
- **Expecting map ordering to be stable.** Maps do not guarantee iteration or insertion order.
- **Calling `hd/1` or `tl/1` on a possibly empty list.** Both raise `ArgumentError` on `[]`.
- **Updating a map with `%{map | key: value}` when the key may be absent.** This raises `KeyError`; use `Map.put/3` for unconditional insertion.
- **Confusing `div`/`rem` rounding.** `div/2` truncates toward zero (`div(6,-4) == -1`); `rem/2`'s sign follows the dividend (`rem(6,-4) == 2`). Use `Integer.floor_div/2` if you need floored division.
- **Expecting `not x in list` to negate membership.** It currently parses as `not(x in list)` with a deprecation warning; write `x not in list` instead.
- **Comparing structs with `<`/`>`.** Struct comparison is structural (field-declaration order), not semantic; `~D[2017-03-31] > ~D[2017-04-01]` is `true`. Use `Date.compare/2`.
- **Using `&&`/`||`/`!` inside a guard.** They are not guard-legal; use `and`/`or`/`not` (which require boolean operands).
- **Relying on `<`/`>` for alphabetic string ordering.** Comparison is byte-wise (UTF-8 code units), so `"álien" > "office"` is `true`.
- **Building lists with `++` in a loop.** `++` is right-associative and O(length(left)); build with cons `[h|t]` and reverse once.
- **Using `**` and expecting an integer.** `2 ** -4` returns a float `0.0625`; only non-negative integer exponents on integers stay integral.
- **Mixing `++`/`--` without parens.** Right-associativity makes `[1,2,3] -- [1] ++ [2]` equal `[3]`; add parens if you mean a different grouping.
- **Treating `=` as assignment.** It is a match: the left side is a pattern and the right side is the value. Writing `1 = x` against an unbound `x` is a compile error.
- **Forgetting `^` to compare rather than rebind.** `x = 1; x = 1` rebinds; to assert `x` already equals 1, write `^x = 1`.
- **Expecting tuple/list patterns to subset-match like maps.** Maps match on a key subset (`%{a: x}` matches `%{a: 1, b: 2}`); tuples and lists require an exact element count.
- **Assuming `%{}` matches only the empty map.** `%{}` matches ANY map (subset match with no required keys).
- **Using a non-literal on the left of `<>` in a pattern.** Only a literal binary prefix is allowed (`"foo" <> rest`); suffix matches (`rest <> "foo"`) are invalid.
- **Calling arbitrary functions in a guard.** Only guard-safe BIFs/operators are allowed; everything else fails to compile.
- **Relying on truthy values inside guards.** Guards have no truthy/falsy concept — a clause runs only when the guard is literally `true`; `when head` with a non-boolean `head` is skipped, not taken.
- **Assuming a raising guard throws.** A guard that raises (e.g. `tuple_size("x")`, `hd([])`) just fails the clause; combine alternatives with multiple `when` clauses so one raising BIF doesn't poison the rest.
- **Combining map/tuple size checks with `or`.** `when map_size(v) == 0 or tuple_size(v) == 0` fails for the non-matching type because the wrong BIF raises and short-circuits the `or`; use separate `when` clauses instead.
- **Confusing multiple `when` with `and`.** Repeated `when` on one clause means OR (any may pass); comma/`and` means AND (all must pass).
- **Defining defaults and guards on the same function head.** A function head declaring default arguments (`\\`) cannot have patterns or guards; only the body clauses can.
- **Treating `0`, `""`, or `[]` as falsy in `if`/`cond`.** Only `false` and `nil` are falsy; everything else (including `0`, `""`, `[]`) is truthy and WILL match a `cond` clause / run an `if` body.
- **Omitting the final `true ->` clause in `cond`.** If every condition is falsy, `cond` raises `CondClauseError` at runtime; always provide a fallback (or use `if/2` for a two-way branch).
- **Using a bare `=` inside `with` and expecting short-circuit.** `with :foo = :bar, do: :ok` raises `MatchError`; only `<-` short-circuits the chain and returns the non-matching value. Use `=` only for guaranteed bindings.
- **Nesting `case` inside `case` instead of using `with`.** Two or more levels of `case` that each handle `{:ok, _} | {:error, _}` should be a single `with` chain.
- **Letting a `with ... else` block grow into a fragile failure taxonomy.** When several steps can fail with different non-matching values, the `else` must distinguish them from the value alone; prefer tagging failures upstream (`{:error, reason}`) so `else` stays simple.
- **Expecting `with`'s `<-` bindings to be available in `else`.** They are not — `else` sees only the value that failed to match; bindings also do not leak outside `with`.
- **Using `unless` with an `else` branch.** The negated condition becomes hard to read; use `if` (or `if !cond`) instead. Note `Kernel.unless/2` is soft-deprecated (docs-only) as of Elixir v1.20.
- **Forgetting `^` when a `case`/`with` clause should compare against a bound variable.** A bare name in a pattern rebinds rather than compares; use `^x` to assert equality.
- **Dispatching on a value's shape with `cond` (`cond do x == {:ok, _} -> ...`).** Use `case x do {:ok, _} -> ...` — pattern matching is clearer and can destructure.
- **Putting patterns or guards on a function head that declares default arguments.** A function head (`def join(a, b, sep \\ " ")`) can only contain argument names and defaults; patterns and guards go on the body clauses.
- **Expecting default arguments to evaluate once.** Defaults re-evaluate on every call that needs them; `def f(x \\ DateTime.utc_now())` re-evaluates each call.
- **Using `import` where `alias` would do.** `import` obscures the origin of functions; prefer `alias` in application code and always pass `:only`/`:except` when importing.
- **Treating `use` as a no-op import.** `use Foo` expands to `require Foo; Foo.__using__(opts)` and can inject arbitrary code; read the used module's docs.
- **Writing `@behavior` (American spelling).** The attribute is `@behaviour` (British); the American spelling is not recognized.
- **Marking only some callbacks with `@impl`.** If you mark one callback with `@impl`, you must mark all callbacks for that behaviour; mixed annotation triggers a warning.
- **Adding a newline between a module attribute and its value.** The parser then treats it as a read, not a set.
- **Reading a module attribute in many functions.** Each read takes a snapshot and slows compilation; move it to a `defp` helper.
- **Using `struct/2` (no bang) for dynamic updates.** It silently drops unknown keys; use `struct!/2` to raise and preserve integrity.
- **Expecting `@enforce_keys` to validate on update.** It only fires at full construction (`%Struct{}`), not on `%{s | k: v}` updates, and does not validate values.
- **Expecting a struct to inherit Map/Enumerable/Access.** Structs are maps (`is_map/1` is `true`) but do NOT inherit protocol implementations; derive or implement explicitly.
- **Putting keyword pairs before plain atoms in `defstruct`.** `defstruct [name: "John", :email]` raises `SyntaxError`; atoms-with-nil-defaults must come first.
- **Implementing a protocol in `test/` and expecting it to take effect.** Protocol consolidation (default in Mix) already linked impls at compile time; disable in test env with `consolidate_protocols: Mix.env() != :test`.
- **Using `@derive` without an `Any` implementation.** `@derive` requires the protocol to have a `for: Any` impl or a custom `__deriving__/2` macrocallback.

## Strict vs contextual guidance

### Strict

- `/` always returns a float, even for integer operands.
- `and/2`, `or/2`, and `not/1` require boolean operands and raise otherwise.
- `true`, `false`, and `nil` are atoms.
- Module names are atoms.
- `map.key` raises `KeyError` when the atom key is missing.
- `%{map | key: value}` raises `KeyError` if the key is not already present.
- `<>` operates on binaries; using it on lists raises. `++` operates on lists; using it on binaries raises.
- Strings are UTF-8 encoded binaries.
- All values are immutable; "mutating" operations return new data.
- `hd/1` and `tl/1` raise on an empty list.
- The `string()` typespec triggers a compiler warning because it refers to charlists, not Elixir strings.

### Conventions

- Use `and`, `or`, `not` when expecting booleans; use `&&`, `||`, `!` for truthy values.
- Return `:ok` / `:error` tagged tuples for operation results.
- Prefer `~c"..."` over `'...'` for charlists.
- Use `map.key` / pattern matching for known-shape maps and `map[:key]` for optional keys.
- Use keyword lists for optional function arguments.
- Use `size` for O(1) and `length` for O(n).
- Use the smallest collection that fits the data shape.

### Contextual tradeoffs

- Charlists are correct for Erlang interop but should be converted to `String.t()` at the boundary for idiomatic Elixir code.
- Maps with atom keys and dot access are fine for structured data, but keyword lists are more flexible for options because they allow duplicate keys and preserve order.
- Tuples are efficient for fixed-size records, but structs (built on maps) are usually better for named, typed records.

## Policy decisions for individual repos

- Whether to enforce the `~c` sigil and reject `'...'` charlist syntax in CI or linting.
- Whether to require `String.t()` and forbid `string()` in typespecs (recommended).
- Lint stack: enable `mix format --check-formatted`, `mix credo --strict`, and/or `mix dialyzer`; whether to use `mix compile --warnings-as-errors`.
- Whether to standardize on bang (`!`) vs tagged-tuple vs exception-based error styles.
- Default collection choice policy: e.g. "variable size → list", "fixed size → tuple", "key-value → map", "options → keyword list".
- Whether to require `@type t` definitions for modules that own a primary data type.

## Related docs

- Existing sibling doc: `docs/elixir/naming-conventions.md` — especially for the `size`/`length` and `is_`/`?`/`!` conventions that interact with type predicates.
- Related sections in this document: `## Basic Operators`, `## Pattern Matching and Guards`, `## Control Flow`, `## Strings, Binaries, and Charlists`, `## Keywords and Maps`, `## Modules and Functions`. Related corpus docs: `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.
- Official sources:
  - [basic-types.html](https://hexdocs.pm/elixir/basic-types.html)
  - [lists-and-tuples.html](https://hexdocs.pm/elixir/lists-and-tuples.html)
  - [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html)
  - [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html)
  - [Kernel.html](https://hexdocs.pm/elixir/Kernel.html)
  - [typespecs.html](https://hexdocs.pm/elixir/typespecs.html)

## Related skills

- None defined yet.
