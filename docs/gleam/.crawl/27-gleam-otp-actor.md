# Crawl: gleam/otp/actor.html
- seed_url: https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
- canonical_url: https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
- family: Gleam core package module
- fetch: 200
- gleam_otp_version: v1.2.0
- feeds_docs: otp-actors-and-supervision.md

## Purpose
The `gleam/otp/actor` module provides the **Actor** abstraction — one of the most
common building blocks of Gleam OTP programs. An Actor is a BEAM process that
holds state, executes code, and communicates with other processes by sending and
receiving messages. The advantage over a bare process is a single interface for
commonly needed functionality, including support for OTP
[tracing and debugging](https://www.erlang.org/doc/man/sys.html).

Gleam's Actor is similar to Erlang's `gen_server` and Elixir's `GenServer` but
differs in that it offers a **fully typed interface**. This different API is why
Gleam uses the name "Actor" rather than some variation of "generic-server".

The typed-message model: messages are values of a user-defined `Message` type
(often a custom type with variants carrying a `process.Subject(reply)` for
request/response messages). The actor's `on_message` handler is a pure function
`fn(state, message) -> Next(state, message)`. There is no separate `Subject(msg)`
type in this module — `Subject` is re-exported from `gleam/erlang/process`.

## Types (Subject, StartResult, init/Next/Status, spec)

> Note: This version (v1.2.0) does **not** have `Subject`, `init`, `Status`, or
> `spec`/`start_spec` types in the `actor` module. `Subject` comes from
> `gleam/erlang/process`. The init result type is `Initialised`; the per-message
> result type is `Next`. See "Version notes" for the mapping to the requested
> names.

### `Builder(state, message, return)` — opaque
Source: [actor.gleam#L294](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L294)
```gleam
pub opaque type Builder(state, message, return)
```
The builder value produced by `new` / `new_with_initialiser`, configured via
`on_message`, `named`, `with_selector`, then started with `start`.

### `Initialised(state, message, data)` — opaque
Source: [actor.gleam#L261](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L261)
```gleam
pub opaque type Initialised(state, message, data)
```
A type returned from an actor's initialiser, containing the actor state, a
selector to receive messages using, and data to return to the parent. Construct
it with `initialised`, `selecting`, and `returning`.

### `Next(state, message)` — opaque
Source: [actor.gleam#L169](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L169)
```gleam
pub opaque type Next(state, message)
```
The type used to indicate what to do after handling a message. Constructed via
`continue`, `stop`, `stop_abnormal`, or `with_selector`.

### `StartError`
Source: [actor.gleam#L580](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L580)
```gleam
pub type StartError {
  InitTimeout
  InitFailed(String)
  InitExited(process.ExitReason)
}
```
- `InitTimeout` — initialiser did not return within the timeout.
- `InitFailed(String)` — initialiser returned `Error(String)`.
- `InitExited(process.ExitReason)` — initialiser process exited.
  (`ExitReason` from `gleam/erlang/process`.)

### `StartResult(data)`
Source: [actor.gleam#L252-L253](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L252-L253)
```gleam
pub type StartResult(data) =
  Result(Started(data), StartError)
```
A convenience for the type returned when an actor process is started.

### `Started(data)`
Source: [actor.gleam#L239](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L239)
```gleam
pub type Started(data) {
  Started(pid: process.Pid, data: data)
}
```
A value returned to the parent when their child actor successfully starts.
- `pid` — the process identifier of the started actor. Can be used to monitor
  the actor, make it exit, etc.
- `data` — data returned by the actor after it initialised. Commonly a subject
  that it will receive messages from.

## Actor builder API (new, with_message_handler, on_timeout, selecting)

> Note: In v1.2.0 the message-handler setter is `on_message` (not
> `with_message_handler`), and there is **no** `on_timeout` builder method.
> Timeouts apply to `call` (per-call reply timeout) and to `new_with_initialiser`
> (init timeout). See "Version notes".

### `new`
Source: [actor.gleam#L329](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L329)
```gleam
pub fn new(state: state) -> Builder(state, message, process.Subject(message))
```
Create a builder for an actor **without** a custom initialiser. The actor returns
a subject to the parent that can be used to send messages to the actor. If the
actor has been given a name with `named` then the subject is a named subject.
For custom init logic see `new_with_initialiser`.

### `new_with_initialiser`
Source: [actor.gleam#L358-L362](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L358-L362)
```gleam
pub fn new_with_initialiser(
  timeout: Int,
  initialise: fn(process.Subject(message)) -> Result(
    Initialised(state, message, return),
    String,
  ),
) -> Builder(state, message, return)
```
Create a builder for an actor with a custom initialiser that runs before the
start function returns to the parent, and before the actor starts handling
messages. The first argument is a number of milliseconds that the initialiser
function is expected to return within; if it takes longer the initialiser is
considered to have failed and the actor will be killed and an error returned to
the parent. The actor's default subject is passed to the initialiser. You can
return it to the parent with `returning`, use it some other way, or ignore it.
If a custom selector is given using `selecting` then this overwrites the default
selector (which selects for the default subject), so you will need to add the
subject to the custom selector yourself.

### `on_message`
Source: [actor.gleam#L376-L379](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L376-L379)
```gleam
pub fn on_message(
  builder: Builder(state, message, return),
  handler: fn(state, message) -> Next(state, message),
) -> Builder(state, message, return)
```
Set the message handler for the actor. This callback function will be called
each time the actor receives a message. Actors handle messages sequentially,
later messages being handled after the previous one has been handled.

### `named`
Source: [actor.gleam#L395-L398](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L395-L398)
```gleam
pub fn named(
  builders: Builder(state, message, return),
  name: process.Name(message),
) -> Builder(state, message, return)
```
Provide a name for the actor to be registered with when started, enabling it to
receive messages via a named subject. Useful for making processes that can take
over from an older one that has exited due to a failure, or to avoid passing
subjects from receiver processes to sender processes. If the name is already
registered to another process then the actor will fail to start. When this
function is used the actor's default subject will be a named subject using this
name.

### `selecting` (initialiser-time selector)
Source: [actor.gleam#L277-L280](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L277-L280)
```gleam
pub fn selecting(
  initialised: Initialised(state, old_message, return),
  selector: process.Selector(message),
) -> Initialised(state, message, return)
```
Add a selector for the actor to receive messages with. If a message is received
by the actor but not selected for with the selector then the actor will discard
it and log a warning.

### `returning`
Source: [actor.gleam#L287-L290](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L287-L290)
```gleam
pub fn returning(
  initialised: Initialised(state, message, old_return),
  return: return,
) -> Initialised(state, message, return)
```
Add the data to return to the parent process. This might be a subject that the
actor will receive messages over.

### `initialised`
Source: [actor.gleam#L268](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L268)
```gleam
pub fn initialised(state: state) -> Initialised(state, message, Nil)
```
Takes the post-initialisation state of the actor. This state will be passed to
the `on_message` callback each time a message is received.

### `with_selector` (per-Next selector change)
Source: [actor.gleam#L210-L213](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L210-L213)
```gleam
pub fn with_selector(
  value: Next(state, message),
  selector: process.Selector(message),
) -> Next(state, message)
```
Provide a selector to change the messages that the actor is handling going
forward. This replaces any selector that was previously given in the actor's
`init` callback, or in any previous `Next` value.

## send / call / receive_message / register / named

> Note: v1.2.0 has **no** `receive_message`, `register`, or `unregister`
> functions in this module. `send` and `call` are re-exports of
> `process.send` / `process.call`. Registration is done via the `named` builder
> method (which uses `process.Name`). See "Version notes".

### `send`
Source: [actor.gleam#L642](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L642)
```gleam
pub fn send(subject: process.Subject(msg), msg: msg) -> Nil
```
Send a message over a given channel. This is a re-export of `process.send`, for
the sake of convenience.

### `call`
Source: [actor.gleam#L654-L658](https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam#L654-L658)
```gleam
pub fn call(
  subject: process.Subject(message),
  waiting timeout: Int,
  sending make_message: fn(process.Subject(reply)) -> message,
) -> reply
```
Send a synchronous message and wait for a response from the receiving process.
If a reply is not received within the given timeout then the sender process
crashes rather than leaving the processes in an invalid state. This is a
re-export of `process.call`, for the sake of convenience.

### `named` — see "Actor builder API" above (registration is via the builder).

## Actor lifecycle + timeouts

Lifecycle phases:

1. **Build** — `actor.new(state)` (or `actor.new_with_initialiser(timeout, fn)`).
   Produces an opaque `Builder(state, message, return)`.
2. **Configure** — `|> actor.on_message(handler)` and optionally `|> actor.named(name)`.
3. **Start** — `actor.start(builder) -> StartResult(return)` =
   `Result(Started(return), StartError)`.
   - On success the parent receives `Started(pid, data)`.
   - On failure: `InitTimeout` | `InitFailed(String)` | `InitExited(ExitReason)`.
4. **Running** — the actor's `on_message` callback is invoked for each selected
   message with the current state; it returns a `Next(state, message)`:
   - `actor.continue(new_state)` — keep running with new state.
   - `actor.with_selector(continue_state, selector)` — continue but swap the
     receive selector going forward (replaces init/previous selector).
   - `actor.stop()` — stop normally (exit reason `Normal`), handling no further
     messages.
   - `actor.stop_abnormal(reason)` — stop abnormally; linked processes also exit
     abnormally; the provided reason is propagated.
5. **Stopped** — once `stop`/`stop_abnormal` is returned (or the process exits),
   no further messages are handled.

Timeouts:
- **Init timeout** — `new_with_initialiser(timeout_ms, initialise)`. If the
  initialiser does not return within `timeout_ms`, the actor is killed and
  `StartError::InitTimeout` is returned to the parent.
- **Call timeout** — `call(subject, timeout, make_message)`. If no reply within
  `timeout` ms, the **caller** crashes (let-it-crash; avoids invalid state).
- There is **no** built-in idle/`on_timeout` handler in v1.2.0. Idle-timeout
  behaviour must be implemented by the user (e.g. via `process.select` with a
  timeout in the handler, or a separate timer process).

Messages not matched by the actor's selector are discarded and a warning is
logged (see `selecting`).

## Mapping to BEAM gen_server (→ docs/beam/gen-server.md)

Gleam's Actor is the typed analogue of Erlang/OTP `gen_server` (see
`docs/beam/gen-server.md`). Approximate mapping:

| gleam/otp/actor (v1.2.0)        | gen_server callback / concept                |
|--------------------------------|----------------------------------------------|
| `new(state)` / `new_with_initialiser` | `init/1` (init returns state)          |
| `Initialised` + `selecting` + `returning` | `init/1` returning `{ok, state, timeout \| hibernate}` plus a custom receive (selective receive) |
| `on_message(handler)`          | `handle_info/2` (and effectively `handle_cast/2` + `handle_call/3` since Gleam uses one typed message stream) |
| `Next` / `continue(state)`     | `{noreply, NewState}`                        |
| `with_selector(next, sel)`     | changing the selective receive / `{noreply, State, timeout}` |
| `stop()`                       | `{stop, normal, State}` → `terminate(normal, State)` |
| `stop_abnormal(reason)`        | `{stop, {shutdown, Reason}, State}` / abnormal exit; links propagate |
| `call(subject, timeout, make)` | `gen_server:call/2,3` (synchronous, with timeout; caller crashes on timeout) |
| `send(subject, msg)`           | `gen_server:cast/2` / `erlang:send/2` (fire-and-forget) |
| `named(name)`                  | `gen_server:start({local, Name}, ...)` registration |
| `StartError`                   | `init` failure (`ignore` / `{stop, Reason}`) |
| `Started(pid, data)`           | `{ok, Pid}` returned to the parent           |

Key difference: Gleam collapses `handle_call`/`handle_cast`/`handle_info` into a
single typed `on_message` callback, and request/reply is modelled by the caller
passing a `Subject(reply)` inside the message rather than by a `gen_server` call
reference. The actor still runs as a special BEAM process that supports the
[`sys`](https://www.erlang.org/doc/man/sys.html) tracing/debugging features.

## Strict rules
- The `on_message` handler must return `Next(state, message)` for every branch.
  Use `continue` to keep running, `stop`/`stop_abnormal` to terminate.
- Messages not selected by the actor's selector are **discarded** and a warning
  is logged — do not rely on unselected messages being queued.
- `call` crashes the **caller** on timeout; do not use `call` without a
  crash-recovery strategy for the caller.
- `new_with_initialiser`'s timeout is mandatory; an over-running initialiser
  kills the actor and returns `InitTimeout`.
- If you use `selecting` inside the initialiser you overwrite the default
  subject selector — re-add the subject to your custom selector if you still
  want to receive on it.
- Actors handle messages **sequentially, one at a time, in receive order** —
  do not assume concurrency within a single actor.
- `named` fails to start if the name is already registered.

## Verbatim quotes
- "An Actor is a process like any other BEAM process and can be used to hold
  state, execute code, and communicate with other processes by sending and
  receiving messages."
- "The advantage of using the actor abstraction over a bare process is that it
  provides a single interface for commonly needed functionality, including
  support for the tracing and debugging features in OTP."
- "Gleam's Actor is similar to Erlang's `gen_server` and Elixir's `GenServer`
  but differs in that it offers a fully typed interface. This different API is
  why Gleam uses the name "Actor" rather than some variation of
  "generic-server"."
- "Actors handle messages sequentially, later messages being handled after the
  previous one has been handled."
- "If a message is received by the actor but not selected for with the selector
  then the actor will discard it and log a warning." (`selecting`)
- "If a reply is not received within the given timeout then the sender process
  crashes rather than leaving the processes in an invalid state." (`call`)
- "Indicate the actor should stop and shut-down, handling no futher messages.
  The reason for exiting is `Normal`." (`stop`)
- "Indicate the actor is in a bad state and should shut down. It will not
  handle any new messages, and any linked processes will also exit abnormally.
  The provided reason will be given and propagated." (`stop_abnormal`)

## Version notes
- Page version: **gleam_otp v1.2.0** (HexDocs; ExDoc v1.13.0-rc1 renderer).
- Canonical URL declared in HTML:
  `https://hexdocs.pm/gleam_otp/gleam/otp/actor.html`. Final fetched URL after
  redirect: `https://gleam-otp.hexdocs.pm/gleam/otp/actor.html`.
- The crawl brief asked for several names that **do not exist** in v1.2.0:
  - `Subject(msg)` type — `Subject` lives in `gleam/erlang/process`, not here.
  - `init` / `Status` types — the init result type is `Initialised`; the
    per-message result type is `Next`. There is no `Status` type.
  - `spec` / `start_spec` — not present; the builder pattern (`Builder` +
    `start`) replaces spec-style start.
  - `with_message_handler` — the setter is `on_message`.
  - `on_timeout` — not present; no built-in idle-timeout handler.
  - `receive_message` — not present; receiving is driven by the actor loop
    using the configured `process.Selector`.
  - `register` / `unregister` — not present; registration is via the `named`
    builder method (uses `process.Name`). There is no explicit unregister.
  - `Selecting` type — not present; `selecting` is a function on `Initialised`,
    and the selector type is `process.Selector`.
- `send` and `call` are explicitly re-exports of `process.send` / `process.call`
  for convenience.
- Source file:
  https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/actor.gleam

## Discovered links

### Relevant (crawl later)
- `gleam/erlang/process` (v1.3.0) — `Subject`, `Selector`, `Name`, `Pid`,
  `ExitReason`, `send`, `call`, `receive`/selecting.
  https://hexdocs.pm/gleam_erlang/1.3.0/gleam/erlang/process.html
- `gleam/otp/supervision` — supervisor child specs (companion to actors).
  https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
- `gleam/otp/static_supervisor` — static supervision tree.
  https://hexdocs.pm/gleam_otp/gleam/otp/static_supervisor.html
- `gleam/otp/factory_supervisor` — dynamic factory supervisor.
  https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html
- `gleam/otp/system` — system-level helpers.
  https://hexdocs.pm/gleam_otp/gleam/otp/system.html
- Erlang `sys` module (tracing/debugging the actor supports).
  https://www.erlang.org/doc/man/sys.html

### Skipped
- `gleam/otp/port` — port driver, out of scope for the actor crawl.
  https://hexdocs.pm/gleam_otp/gleam/otp/port.html
- README / index, Website, Sponsor, Repository, Hex package links —
  non-API navigation.
