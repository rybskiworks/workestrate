# Error Handling

## Purpose

This is the error-handling section of a multi-part Elixir reference for AI coding agents who write, review, and debug Elixir. This file covers the `try/catch/rescue/after/else` special form, exception classes, `raise`/`reraise`/`defexception`, the Exception behaviour, error tuples versus exceptions, and the "let it crash" philosophy with process exits and links.

## Sources used

- https://hexdocs.pm/elixir/try-catch-and-rescue.html (PRIMARY)
- https://hexdocs.pm/elixir/Exception.html
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/Process.html
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#__STACKTRACE__/0
- https://www.erlang.org/doc/system/errors.html

This page reflects Elixir v1.20.2 docs and OTP 27–29 semantics.

## Related BEAM guidance

The Elixir `try/catch/rescue`, `raise`/`reraise`/`defexception`, and error-tuple material below is the value of this doc; the underlying BEAM exit-signal and failure semantics live in `docs/beam/`:

- [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md) — for exit-signal propagation, links/monitors, `trap_exit`, and `:kill`/`:shutdown`/`:normal` reason semantics.
- [../beam/common-mistakes.md](../beam/common-mistakes.md) — for BEAM-wide error-handling pitfalls and anti-patterns.
- [../beam/supervision.md](../beam/supervision.md) — for restart values (`:permanent`/`:transient`/`:temporary`) and the "let it crash" recovery model.

## try/catch/rescue

### The `try` special form

The `try` special form has one mandatory part, the `do` block, and four optional clauses: `rescue`, `catch`, `after`, and `else`. At least one of `rescue`, `catch`, or `after` must be present for `try` to be meaningful.

The full grammar, in the order Elixir accepts, is:

```elixir
try do
  expression
rescue
  pattern -> expr
catch
  kind, value -> expr
after
  expr
else
  pattern -> expr
end
```

Syntactically the clause order is `do`, `rescue`, `catch`, `after`, `else` (with `rescue` and `catch` each individually optional and interchangeable in presence). At runtime, `after` always executes last regardless of its syntactic position. Each of `rescue`, `catch`, and `else` accepts `pattern -> body` clauses, like `case`.

The `try` keyword may be omitted at the top of a function body when `after`, `rescue`, or `catch` is used. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "Elixir will automatically wrap the function body in a `try` whenever one of `after`, `rescue` or `catch` is specified."

```elixir
defmodule RunAfter do
  def without_even_trying do
    raise "oops"
  after
    IO.puts("cleaning up!")
  end
end
```

### The three exception classes

Elixir inherits Erlang's three exception classes. From [errors.html](https://www.erlang.org/doc/system/errors.html):

> "Exceptions are run-time errors or generated errors and are of three different classes, with different origins. The try expression can distinguish between the different classes, whereas the catch expression cannot."

| Class | Created by | Origin |
|---|---|---|
| `:error` | `raise` (and runtime errors like bad arithmetic, bad pattern match) | Run-time error / programmer error |
| `:exit` | `Kernel.exit/1` | The current process intends to terminate |
| `:throw` | `Kernel.throw/1` | Non-local return (rare; library interop) |

### `rescue` clause

`rescue` matches only the `:error` class. Exits and throws are **not** caught by `rescue`; use `catch` instead.

The matching forms are:

| Form | Meaning |
|---|---|
| `rescue RuntimeError -> ...` | Match by exception module name. |
| `rescue e in RuntimeError -> e` | Match and bind the exception struct to `e`. |
| `rescue [ArithmeticError, ArgumentError] -> ...` | Match any exception in the list. |
| `rescue e in [ArithmeticError] -> ...` | Match list and bind the struct to `e`. |
| `rescue e -> e` | Bind any exception struct. |
| `rescue _ -> ...` | Wildcard; discard the exception. |

```elixir
iex> try do
...>   raise "oops"
...> rescue
...>   e in RuntimeError -> e
...> end
%RuntimeError{message: "oops"}
```

### `catch` clause

`catch` distinguishes the three classes. It can also bind a stacktrace as a third positional variable.

```elixir
try do
  throw(:value)
catch
  :throw, value ->
    {:caught_throw, value}

  :exit, reason ->
    {:caught_exit, reason}

  :error, error ->
    {:caught_error, error}

  kind, value, stacktrace ->
    {kind, value, stacktrace}
end
```

Using `catch` for throws is already uncommon; catching exits is rarer still. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "However, using `try/catch` is already uncommon and using it to catch exits is even rarer."

### `after` clause

`after` runs whether the `do` block returns normally, raises, throws, or exits (when the exit is caught by `catch`). It is a *soft* guarantee. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "The `after` clause will be executed regardless of whether or not the tried block succeeds. Note, however, that if a linked process exits, this process will exit and the `after` clause will not get run. Thus `after` provides only a soft guarantee."

`after` does not change the return value of the `try`:

> "The `after` block handles side effects and does not change the return value from the clauses above it."

Resources owned by the process are often linked to it and clean themselves up:

> "Luckily, files in Elixir are also linked to the current processes and therefore they will always get closed if the current process crashes, independent of the `after` clause. You will find the same to be true for other resources like ETS tables, sockets, ports and more."

```elixir
try do
  file = File.open!("sample", [:write])
  raise "oops"
after
  File.close(file)
end
```

Do **not** rely on `after` for critical cleanup. It does not run when a linked process exits (unless the current process is trapping exits) or when the process is forcibly killed with `:kill` or `:brutal_kill`. Cross-reference `docs/elixir/otp-supervision.md` `terminate/2`, which is also not guaranteed.

### `else` clause

`else` matches the *success* return value of the `try` block when no exception occurred. Guards are allowed. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "If an `else` block is present, it will match on the results of the `try` block whenever the `try` block finishes without a throw or an error."

If no `else` pattern matches, the exception raised is `TryClauseError`, **not** `CaseClauseError`. This exception is not caught by the surrounding `try/catch/rescue/after` block:

> "Exceptions in the `else` block are not caught. If no pattern inside the `else` block matches, an exception will be raised; this exception is not caught by the current `try/catch/rescue/after` block."

```elixir
try do
  1 / x
rescue
  ArithmeticError ->
    :infinity
else
  y when y < 1 and y > -1 ->
    :small
  _ ->
    :large
end
```

### Stacktrace

`__STACKTRACE__/0` is a special form that returns the stacktrace for the currently handled exception. It is available **only** inside `rescue` or `catch` clauses of `try` (and in function-head rescue). It replaced `System.stacktrace/0`, which was hard-deprecated in Elixir v1.11. From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#__STACKTRACE__/0):

> "Returns the stacktrace for the currently handled exception. It is available only in the `catch` and `rescue` clauses of `try/1` expressions and function definitions."

```elixir
# in rescue
try do
  ...
rescue
  e ->
    Logger.error(Exception.format(:error, e, __STACKTRACE__))
    reraise e, __STACKTRACE__
end

# in catch (third positional binding)
try do
  ...
catch
  kind, value, stacktrace ->
    Logger.error(Exception.format(kind, value, stacktrace))
end
```

## Exceptions (raise, reraise, defexception)

### `raise` and `reraise`

There are exactly four `raise`/`reraise` forms in Elixir. All are macros returning `no_return()`. There is **no** `raise/3` and **no** `reraise/1`.

| Macro | Signature (prose) | Raises |
|---|---|---|
| `raise/1` | `raise(message)` where `message` is a string | `RuntimeError` with that message |
| `raise/2` | `raise(exception, attributes)` | The exception built via `exception.exception(attributes)`; `raise Module` is shorthand for `raise Module, []` |
| `reraise/2` | `reraise(message, stacktrace)` | `RuntimeError` with that message, preserving the given stacktrace |
| `reraise/3` | `reraise(exception, attributes, stacktrace)` | The exception preserving the given stacktrace |

The three ways to raise:

```elixir
iex> raise "oops"
** (RuntimeError) oops

iex> raise ArgumentError, message: "invalid argument foo"
** (ArgumentError) invalid argument foo

iex> raise %ArgumentError{message: "msg"}
** (ArgumentError) msg
```

- `raise "s"` is equivalent to `raise RuntimeError, message: "s"` and to `raise %RuntimeError{message: "s"}`.
- `raise Module, attrs` calls `Module.exception(attrs)`. The default implementation accepts a keyword list that is merged into the struct, or a string that becomes the `:message`.
- `raise %Struct{}` raises the literal struct directly, bypassing `exception/1`.

### `Kernel.exit/1` and `Kernel.throw/1`

`Kernel.exit/1` and `Kernel.throw/1` are regular functions with known specs:

```elixir
@spec exit(term) :: no_return
def exit(reason) do
  :erlang.exit(reason)
end
```

```elixir
@spec throw(term) :: no_return
def throw(term) do
  :erlang.throw(term)
end
```

`exit/1` creates an `:exit` class exception in the current process. `throw/1` creates a `:throw` class value that can be caught with `catch`.

### `reraise` and preserving the stacktrace

Use `reraise` inside `rescue` when you want to log or transform an exception and then re-raise it with the *original* stacktrace. `raise e` would attach the rescue-site stacktrace, hiding the real origin.

```elixir
try do
  # ... some code ...
rescue
  e ->
    Logger.error(Exception.format(:error, e, __STACKTRACE__))
    reraise e, __STACKTRACE__
end
```

From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "We use the `__STACKTRACE__` construct both when formatting the exception and when re-raising. This ensures we reraise the exception as is, without changing value or its origin."

### `defexception`

`defexception/1` defines a struct that implements the `Exception` behaviour. The `:message` field is the conventional human-readable string.

Simple form:

```elixir
defmodule MyError do
  defexception message: "default message"
end
```

```elixir
iex> raise MyError
** (MyError) default message
iex> raise MyError, message: "custom message"
** (MyError) custom message
```

Field-list form:

```elixir
defmodule MyError do
  defexception [:field1, :field2, message: "default message"]
end
```

Full customization with `@impl true` callbacks:

```elixir
defmodule MyApp.RequestError do
  defexception [:status, :body, message: "request failed"]

  @impl true
  def exception(opts) do
    status = Keyword.fetch!(opts, :status)
    body = Keyword.get(opts, :body, "")
    msg = "HTTP #{status}: #{body}"
    %__MODULE__{status: status, body: body, message: msg}
  end

  @impl true
  def message(%__MODULE__{status: status, message: msg}) do
    "[status=#{status}] #{msg}"
  end
end
```

By default `defexception/1` generates:

- a struct (via `defstruct`);
- a default `exception/1` callback (keyword list → merged into struct; string → `%Mod{message: string}`);
- a default `message/1` callback that returns the `:message` field;
- the `@behaviour Exception` declaration and `:__exception__ => true` on the struct.

### The `Exception` behaviour

The `Exception` behaviour defines the shape of exception structs and their required callbacks:

```elixir
# Exception.t()
@type t() :: %{
  :__struct__ => module(),
  :__exception__ => term(),
  optional(atom()) => any()
}

@callback exception(term()) :: t()
@callback message(t()) :: String.t()
@callback blame(t(), stacktrace()) :: {t(), stacktrace()}   # optional
```

From [Exception.html](https://hexdocs.pm/elixir/Exception.html), the required callbacks:

> "Receives the arguments given to `raise/2` and returns the exception struct. The default implementation accepts either a set of keyword arguments that is merged into the struct or a string to be used as the exception's message."

> "Receives the exception struct and must return its message. Many exceptions have a message field which by default is accessed by this function. However, if an exception does not have a message field, this function must be explicitly implemented."

Useful helpers in the `Exception` module:

- `Exception.message/1` — returns the formatted message for any exception.
- `Exception.format(:error | :exit | :throw, value, stacktrace)` — formats an exception for logging.
- `Exception.normalize(kind, value, stacktrace)` — wraps Erlang `:error` terms as `ErlangError`.
- `Exception.blame/3` — enriches exceptions with blame information (since 1.5).
- `Kernel.is_exception/1` and `is_exception/2` — guards to test exception structs (since 1.11).

`Exception.exception?/1` was hard-deprecated in Elixir v1.15 in favor of `Kernel.is_exception/1`.

### Built-in exceptions

| Exception | Key fields | Raised by |
|---|---|---|
| `RuntimeError` | `:message` | `raise "string"` |
| `ArgumentError` | `:message` | bad arguments |
| `ArithmeticError` | `:message` | `/`, `div`, `rem` errors |
| `KeyError` | `:key`, `:term` | missing map key (dot access), missing struct field |
| `MatchError` | `:term` | failed `=` match |
| `CaseClauseError` | `:term` | `case` with no matching clause |
| `TryClauseError` | `:term` | `try` `else` with no matching clause |
| `WithClauseError` | `:message` | `with` `else` with no matching clause |
| `BadMapError` | `:term` | non-map where map expected |
| `BadStructError` | `:struct`, `:name`, `:fields` | invalid struct operations |
| `BadBooleanError` | `:module`, `:function`, `:args`, `:kind` | non-boolean with `and`/`or`/`not` |
| `BadArityError` | `:fun`, `:arity`, `:args` | calling a fun with wrong arity |
| `FunctionClauseError` | `:module`, `:function`, `:arity`, `:args`, `:kind` | no function clause matches |
| `UndefinedFunctionError` | `:module`, `:function`, `:arity` | undefined function |
| `ErlangError` | `:original` | wraps Erlang `:error` terms in Elixir |
| `SystemLimitError` | `:message` | system limit reached |
| `File.Error` | `:action`, `:reason`, `:path` | `File.read!/1` etc. |
| `File.CopyError` | `:action`, `:reason`, `:source`, `:destination` | `File.cp!/2` etc. |
| `Enum.OutOfBoundsError` | `:index`, `:range` | out-of-range enum access (since 1.16) |

`File.Error` is the canonical exception used in the try/catch docs. `ErlangError` wraps any Erlang-raised `:error` that reaches Elixir (for example, a `:function_clause` from an Erlang library).

## Error Tuples vs Exceptions

### Three idioms

1. **Error tuples** — `{:ok, result}` / `{:error, reason}`. Use these for *expected, controllable* failure at API boundaries. The caller pattern-matches with `case` or `with`.
2. **Exceptions / bang functions** — use for *unexpected* failure: programmer error, invariant violation, or corrupt external state. The default response is to let the process crash and let a supervisor restart it.
3. **The bang convention** — a function `foo` returns `{:ok, result}` or `{:error, reason}`; `foo!` raises and returns the unwrapped result on success.

From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "Many functions in the standard library follow the pattern of having a counterpart that raises an exception instead of returning tuples to match against. The convention is to create a function (`foo`) which returns `{:ok, result}` or `{:error, reason}` tuples and another function (`foo!`, same name but with a trailing `!`) that takes the same arguments as `foo` but which raises an exception if there's an error. `foo!` should return the result (not wrapped in a tuple) if everything goes fine. The `File` module is a good example of this convention."

In practice, `try/rescue` is rare. Prefer returning tuples. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "In practice, Elixir developers rarely use the `try/rescue` construct. For example, many languages would force you to rescue an error when a file cannot be opened successfully. Elixir instead provides a `File.read/1` function which returns a tuple containing information about whether the file was opened successfully."

### Decision table

| Situation | Use |
|---|---|
| Expected, recoverable failure (file not found, validation, network timeout) | `{:ok, _}` / `{:error, _}` tuple; caller matches |
| The caller needs the value directly and failure is exceptional | bang function `foo!` (raises) |
| Unexpected / programmer error / invariant violation | `raise`; let the supervisor recover |
| Translating exceptions at a boundary | `rescue` specific exception → `{:error, reason}` |

### Examples

```elixir
case File.read("README.md") do
  {:ok, body} ->
    String.length(body)

  {:error, reason} ->
    {:error, reason}
end
```

```elixir
body = File.read!("README.md")   # raises File.Error on failure
String.length(body)
```

```elixir
with {:ok, user} <- fetch_user(id),
     {:ok, account} <- fetch_account(user.account_id),
     :ok <- verify_active(account) do
  {:ok, account}
else
  {:error, :not_found} -> {:error, :user_missing}
  {:error, reason} -> {:error, reason}
end
```

Cross-reference `docs/elixir/language-fundamentals.md` for tagged tuples and `docs/elixir/naming-conventions.md` for the `!`/`?` suffix conventions.

## Let It Crash Philosophy

"Let it crash" / "fail fast" means that when something *unexpected* happens, it is better to let the exception occur than to rescue every possible case blindly. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "One saying that is common in the Erlang community, as well as Elixir's, is 'fail fast' / 'let it crash'. The idea behind let it crash is that, in case something *unexpected* happens, it is best to let the exception happen, without rescuing it."

> "At the end of the day, 'fail fast' / 'let it crash' is a way of saying that, when *something unexpected* happens, it is best to start from scratch within a new process, freshly started by a supervisor, rather than blindly trying to rescue all possible error cases without the full context of when and how they can happen."

> "The second approach also works because, as discussed in the Processes chapter, all Elixir code runs inside processes that are isolated and don't share anything by default. Therefore, an unhandled exception in a process will never crash or corrupt the state of another process. This allows us to define supervisor processes, which are meant to observe when a process terminates unexpectedly, and start a new one in its place."

The philosophy rests on BEAM process isolation (a crash in one process cannot corrupt another's state), supervision (supervisors restart failed processes into a known-good state), and exit-reason semantics (`:normal`/`:shutdown`/`{:shutdown, term}` are non-error; anything else is abnormal and triggers restart per the child `:restart` policy). For the full exit-signal propagation and restart model, see [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md) and [../beam/supervision.md](../beam/supervision.md).

Cross-reference `docs/elixir/otp-supervision.md` for restart values (`:permanent`/`:transient`/`:temporary`) and exit-reason semantics.

### When NOT to let it crash

The word *unexpected* is the key. At user-facing or system boundaries — CLI input, HTTP request handlers, config parsers — a descriptive error reply is usually better than a process restart. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "It is important to emphasize the word *unexpected*. For example, imagine you are building a script to process files. Your script receives filenames as inputs. It is expected that users may make mistakes and provide unknown filenames. In this scenario, while you could use `File.read!/1` to read files and let it crash in case of invalid filenames, it probably makes more sense to use `File.read/1` and provide users of your script with a clear and precise feedback of what went wrong. Other times, you may fully expect a certain file to exist, and in case it does not, it means something terribly wrong has happened elsewhere. In such cases, `File.read!/1` is all you need."

### `Process.exit/2` vs `Kernel.exit/1`, links, monitors, and `trap_exit`

- `Kernel.exit/1` stops the **current** process. It produces an `:exit` class value that can be caught with `try/catch`.
- `Process.exit/2` sends an exit **signal** to **another** process. It is not caught by `try` in the target process; it is handled by trapping exits, and it is never trappable when the reason is `:kill`.

From [Process.html](https://hexdocs.pm/elixir/Process.html#exit/2):

> "The functions `Kernel.exit/1` and `Process.exit/2` are named similarly but provide very different functionalities. The `Kernel.exit/1` function should be used when the intent is to stop the current process while `Process.exit/2` should be used when the intent is to send an exit signal to another process."

| `reason` | Effect on target (not trapping / trapping) |
|---|---|
| `:normal` | No exit unless self; trapping → `{:EXIT, from, :normal}` message |
| `:kill` | Unconditionally terminates; **untrappable** (target reason becomes `:killed`) |
| other | Not trapping → terminates with reason; trapping → `{:EXIT, from, reason}` message |

`Process.flag(:trap_exit, true)` converts incoming exit signals into `{:EXIT, from, reason}` mailbox messages. `Process.link/1` makes exit propagation bidirectional. `Process.monitor/1` is one-way and delivers `{:DOWN, ref, :process, pid, reason}`.

For exit-signal reception rules, `:kill` untrappability, link/monitor propagation, and `trap_exit` conversion semantics, see [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md).

`catch :exit` and bare `try/catch` for process death are uncommon. The normal mechanisms are trapping exits inside a GenServer or using monitors. Cross-reference `docs/elixir/otp-supervision.md` for `handle_info :EXIT`, `terminate/2`, and trapping exits.

### `throw` guidance

Avoid `throw` for general control flow. From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "In Elixir, a value can be thrown and later be caught. `throw` and `catch` are reserved for situations where it is not possible to retrieve a value unless by using `throw` and `catch`. Those situations are quite uncommon in practice except when interfacing with libraries that do not provide a proper API."

Prefer `with`, `Enum.find_value/3`, or restructuring. A thrown value is caught by `catch :throw, value`, not by `rescue`.

### Rescue only what you can handle

Valid reasons to use `rescue`:

1. Log and `reraise` for observability or middleware.
2. Translate a specific exception to an `{:error, reason}` tuple at a system boundary.
3. Retry a specific transient exception.
4. Wrap code for telemetry or spans, then re-raise.

Anti-patterns:

- Broad `rescue _ ->` or `rescue e ->` without `reraise` or translation (swallows errors and breaks supervision/observability).
- Using exceptions for control flow.

From [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html):

> "Generally speaking, we take errors in Elixir literally: they are reserved for unexpected and/or exceptional situations, never for controlling the flow of our code."

## Review checklist

- [ ] Errors and exceptions are used only for unexpected conditions, not for control flow.
- [ ] `try/rescue` is rare and, when used, either logs + `reraise`s or translates to an error tuple.
- [ ] `reraise e, __STACKTRACE__` is used inside `rescue`, not `raise e`.
- [ ] `__STACKTRACE__` is used instead of the deprecated `System.stacktrace/0`.
- [ ] `after` is not relied on for critical cleanup.
- [ ] Custom exceptions are defined with `defexception` and include a `:message` field.
- [ ] `:error`, `:exit`, and `:throw` are distinguished correctly in `catch` clauses.
- [ ] Bang functions raise and return the unwrapped success value.
- [ ] `Process.exit/2` (signal to another process) is not confused with `Kernel.exit/1` (stop self).
- [ ] `raise/3` and `reraise/1` are not used — only `raise/1`, `raise/2`, `reraise/2`, and `reraise/3` exist.
- [ ] `else` no-match in `try` is expected to raise `TryClauseError`, not `CaseClauseError`.
- [ ] `throw` is avoided except for rare library interop.
- [ ] Custom `exception/1` and `message/1` callbacks are marked with `@impl true`.
- [ ] `rescue` is not used to catch `:exit` or `:throw` (use `catch`).
- [ ] `Process.exit(pid, :kill)` is treated as unconditionally fatal and untrappable.

## Implementation checklist

- [ ] Choose deliberately between error tuple, bang function, `raise`, and let-it-crash.
- [ ] Provide `foo`/`foo!` pairs for fallible public APIs where callers may need either style.
- [ ] `rescue` only specific exception types and either translate to `{:error, reason}` or `reraise`.
- [ ] Use `__STACKTRACE__` whenever formatting or re-raising inside `rescue`/`catch`.
- [ ] Define custom exceptions with `defexception` and a `:message` field.
- [ ] Trap exits only in the process that owns the resource being protected.
- [ ] Use monitors for one-way death detection of other processes.
- [ ] Keep the supervisor as the primary recovery mechanism.
- [ ] Avoid `System.stacktrace/0`; use `__STACKTRACE__`.
- [ ] Handle `else` no-match explicitly or document that `TryClauseError` is acceptable.
- [ ] Use `with` for chained fallible operations instead of nested `case` or exceptions.

## Validation hooks

- `mix compile` — catches malformed `try` clause shapes and undefined modules in `raise Module`.
- `mix compile --warnings-as-errors` — treats deprecation warnings (e.g. `System.stacktrace/0`) as failures.
- `mix credo` — flags broad `rescue _ ->` and other discouraged patterns.
- `mix dialyzer` — verifies `@spec` on custom exceptions and `{:ok, _}` / `{:error, _}` tuple contracts.
- `iex` / `mix run` — interactive verification of exception behavior.
- ExUnit assertions: `assert_raise Exception, fn -> ... end`, `assert catch_throw(...)`, and `assert catch_exit(...)` for testing exceptions.
- Note: markdown documentation cannot be compiled; validate code blocks against the official docs.

## Examples

### `after` for resource cleanup

```elixir
def read_with_cleanup(path) do
  file = File.open!(path, [:read])

  try do
    IO.read(file, :all)
  after
    File.close(file)
  end
end
```

### Rescue + `reraise` for observability

```elixir
try do
  ExternalApi.call(params)
rescue
  e in HTTPError ->
    Logger.warning("API call failed: #{Exception.message(e)}")
    reraise e, __STACKTRACE__
end
```

### Translate an exception to an error tuple at a boundary

```elixir
def safe_read(path) do
  {:ok, File.read!(path)}
rescue
  e in File.Error ->
    {:error, e.reason}
end
```

### Custom exception with custom `exception/1` and `message/1`

```elixir
defmodule MyApp.RequestError do
  defexception [:status, :body, message: "request failed"]

  @impl true
  def exception(opts) do
    status = Keyword.fetch!(opts, :status)
    body = Keyword.get(opts, :body, "")
    %__MODULE__{status: status, body: body, message: "HTTP #{status}: #{body}"}
  end

  @impl true
  def message(%__MODULE__{status: status, message: msg}) do
    "[status=#{status}] #{msg}"
  end
end
```

### `catch :exit` from `Kernel.exit/1`

```elixir
try do
  exit(:stop)
catch
  :exit, reason ->
    {:caught_exit, reason}
end
#=> {:caught_exit, :stop}
```

### `with` for chained fallible operations

```elixir
with {:ok, user} <- fetch_user(id),
     {:ok, account} <- fetch_account(user.account_id),
     :ok <- verify_active(account) do
  {:ok, account}
else
  {:error, :not_found} -> {:error, :missing}
  {:error, reason} -> {:error, reason}
end
```

### Bang function pair

```elixir
defmodule Parser do
  def parse(input) when is_binary(input) do
    case Integer.parse(input) do
      {n, ""} -> {:ok, n}
      _ -> {:error, :invalid_integer}
    end
  end

  def parse!(input) when is_binary(input) do
    case parse(input) do
      {:ok, n} -> n
      {:error, reason} -> raise ArgumentError, message: "cannot parse #{inspect(input)}: #{reason}"
    end
  end
end
```

## Common mistakes

- **Using `raise e` instead of `reraise e, __STACKTRACE__` inside `rescue`.** `raise e` records the rescue site as the origin, hiding the original stacktrace. Use `reraise` to preserve it.
- **Broad `rescue _ ->` that swallows errors.** This breaks supervision signals and hides failures. Either `reraise` or translate to a specific `{:error, reason}`.
- **Relying on `after` for critical cleanup.** `after` does not run when a linked process kills the current process or when the process is killed with `:kill`/`:brutal_kill`.
- **Confusing `Kernel.exit/1` with `Process.exit/2`.** `Kernel.exit/1` stops the current process; `Process.exit/2` sends a signal to another process and is not caught by `try`.
- **Using exceptions for expected or controllable failure.** Prefer `{:ok, _}` / `{:error, _}` tuples at API boundaries.
- **Believing `raise/3` or `reraise/1` exist.** Only `raise/1`, `raise/2`, `reraise/2`, and `reraise/3` exist in Elixir.
- **Using `rescue` to catch `:exit` or `:throw`.** `rescue` catches only `:error`; use `catch` for `:exit` and `:throw`.
- **Thinking `else` no-match in `try` raises `CaseClauseError`.** It raises `TryClauseError`.
- **Forgetting `@impl true` on custom `exception/1` and `message/1` callbacks.** This causes a compiler warning under `mix compile --warnings-as-errors`.
- **Using `System.stacktrace/0`.** It was deprecated in favor of `__STACKTRACE__/0`, which works only inside `rescue`/`catch`.
- **Throwing a value and expecting `rescue` to catch it.** Throws are caught by `catch :throw, value`, not by `rescue`.
- **Catching `:exit` when you meant to trap exits in a GenServer.** Use `Process.flag(:trap_exit, true)` and handle `{:EXIT, from, reason}` in `handle_info/2`.
- **Expecting `Process.exit(pid, :kill)` to be trappable.** It unconditionally terminates the target; the target observes reason `:killed`.
- **Defining an exception without `:message` and without implementing `message/1`.** The default `message/1` returns the `:message` field; without it, the exception message will be unhelpful.
- **Using `raise Module, attrs` with attrs that do not match the struct keys.** The default `exception/1` merges the keyword list into the struct, so unknown keys cause a compile or runtime error.

## Strict vs contextual guidance

### Strict

- Only four `raise`/`reraise` forms exist: `raise/1`, `raise/2`, `reraise/2`, and `reraise/3`. There is no `raise/3` and no `reraise/1`.
- `rescue` catches only the `:error` exception class; `:exit` and `:throw` require `catch`.
- An `else` clause in `try` that does not match raises `TryClauseError`, not `CaseClauseError`.
- `after` is a soft guarantee: it does not run when the process exits due to a linked process or `:kill`/`:brutal_kill`.
- `Process.exit(_, :kill)` is unconditionally fatal and untrappable.
- `__STACKTRACE__/0` is valid only inside `rescue` or `catch` clauses of `try`.
- `System.stacktrace/0` is deprecated; use `__STACKTRACE__/0`.
- `Kernel.exit/1` stops the current process; `Process.exit/2` sends a signal to another process.
- `throw` is class `:throw`; it is caught by `catch`, not by `rescue`.

### Conventions

- Use `{:ok, _}` / `{:error, _}` tuples for expected failure and `raise` for unexpected failure.
- Provide `foo`/`foo!` function pairs when callers may need either tuples or a raising variant.
- `rescue` only specific exception types and either translate to an error tuple or `reraise` with `__STACKTRACE__`.
- Let unexpected failures crash and let supervisors restart the process.
- Avoid `throw` except for rare library interop that offers no better API.
- Define custom exceptions with `defexception` and a `:message` field.
- Mark custom `exception/1`, `message/1`, and `blame/2` callbacks with `@impl true`.

### Contextual tradeoffs

- Tuple-vs-raise depends on whether failure is *expected* at that boundary. Use tuples at API boundaries where callers decide; use exceptions internally for invariants.
- Trap exits only in the process that owns the resource whose cleanup matters, and consider monitors as an alternative.
- Rescue and translate exceptions at the system edge (HTTP handlers, CLI entry points); let crashes propagate internally so supervisors can recover.
- `after` is acceptable for non-critical teardown of locally-owned resources, but not for safety-critical cleanup or distributed coordination.
- Bang functions are useful when the caller has already validated that failure is exceptional.

## Policy decisions for individual repos

- Enforce error-tuple vs exception style per layer (boundary vs internal).
- Require `foo`/`foo!` pairs for all fallible public APIs.
- Add a lint rule banning `rescue _ ->` or `rescue e ->` without `reraise` or explicit translation.
- Require `__STACKTRACE__` and forbid `System.stacktrace/0`.
- Decide whether workers are allowed to trap exits, and under what conditions.
- Decide whether custom exceptions must implement `message/1` explicitly.
- Decide whether bang functions are permitted in business logic or only at system boundaries.
- Run `mix format --check-formatted`, `mix credo`, and/or `mix dialyzer` in CI; decide whether `mix compile --warnings-as-errors` is required.

## Related docs

- `docs/elixir/otp-supervision.md` — supervisors, restart values (`:permanent`/`:transient`/`:temporary`), exit-reason semantics, trapping exits, and `terminate/2` (not guaranteed).
- `docs/elixir/language-fundamentals.md` — tagged tuples `{:ok, _}` / `{:error, _}`, atoms, and `:ok`/`:error` idioms.
- `docs/elixir/naming-conventions.md` — the `!` (raises) and `?` (boolean) suffix conventions.
- `docs/elixir/core-modules.md` — module-level patterns for core APIs.
- `docs/elixir/typespecs-and-dialyzer.md` — typing `{:ok, t}` / `{:error, reason}` and `no_return`.
- Official sources:
  - [try-catch-and-rescue.html](https://hexdocs.pm/elixir/try-catch-and-rescue.html)
  - [Exception.html](https://hexdocs.pm/elixir/Exception.html)
  - [Kernel.html](https://hexdocs.pm/elixir/Kernel.html)
  - [Process.html](https://hexdocs.pm/elixir/Process.html)
  - [Kernel.SpecialForms.html#__STACKTRACE__/0](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#__STACKTRACE__/0)
  - [Erlang errors.html](https://www.erlang.org/doc/system/errors.html)

## Related skills

- None defined yet.
