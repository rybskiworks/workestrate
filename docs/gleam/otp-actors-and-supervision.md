# Gleam OTP: typed actors and supervision

Synthesised reference for `gleam_otp` **v1.2.0**. `Subject`, `Selector`, `Name`,
and `Pid` live in `gleam/erlang/process`, not in `gleam_otp`.

## Purpose

`gleam_otp` wraps Erlang/OTP `gen_server`, `supervisor`, and dynamic supervisor
behaviours in a fully typed Gleam API for fault-tolerant, multi-core BEAM
programs.

```sh
gleam add gleam_otp@1
```

## Sources used

- [`26-gleam-otp-index.md`](.crawl/26-gleam-otp-index.md) — package overview:
  https://hexdocs.pm/gleam_otp/
- [`27-gleam-otp-actor.md`](.crawl/27-gleam-otp-actor.md) — actor API:
  https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
- [`28-gleam-otp-supervision.md`](.crawl/28-gleam-otp-supervision.md) — child
  specs: https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
- [`34-static-supervisor.md`](.crawl/34-static-supervisor.md) — static
  supervisor: https://hexdocs.pm/gleam_otp/gleam/otp/static_supervisor.html
- [`37-factory-supervisor.md`](.crawl/37-factory-supervisor.md) — dynamic
  factory supervisor: https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html

## Related BEAM guidance

- [`../beam/supervision.md`](../beam/supervision.md) — OTP `supervisor`,
  `sup_flags`, `child_spec`, MaxR/MaxT, `auto_shutdown`, significant children.
- [`../beam/gen-server.md`](../beam/gen-server.md) — `gen_server` callbacks and
  `sys` tracing that `gleam/otp/actor` wraps.
- [`../beam/gen-statem.md`](../beam/gen-statem.md) — state-machine processes;
  `gleam_otp` has no typed `gen_statem` analogue as of v1.2.0.
- [`../beam/proc-lib-and-sys.md`](../beam/proc-lib-and-sys.md) — OTP system
  messages; actors only partially support them.

## Core guidance

### `gleam/otp/actor`

Typed `gen_server` analogue. A single `on_message` callback handles all
messages; request/reply is modelled by embedding `process.Subject(reply)` in the
message.

```gleam
pub opaque type Builder(state, message, return)
pub opaque type Initialised(state, message, data)
pub opaque type Next(state, message)

pub type StartError {
  InitTimeout
  InitFailed(String)
  InitExited(process.ExitReason)
}

pub type StartResult(data) = Result(Started(data), StartError)
pub type Started(data) { Started(pid: process.Pid, data: data) }
```

Lifecycle: build with `actor.new(state)` or
`actor.new_with_initialiser(timeout, init)`, configure with `on_message` and
optionally `named`, then `actor.start(builder)` returns `StartResult(return)`.
While running, `on_message` returns `Next`: `actor.continue(new_state)` keeps
running, `actor.stop()` exits normally, `actor.stop_abnormal(reason)` exits
abnormally and propagates to links, and `actor.with_selector(next, selector)`
replaces the receive selector. Messages are handled sequentially; unselected
messages are discarded with a logged warning.

`actor.send` and `actor.call` re-export `gleam/erlang/process`. `call` is
synchronous and a timeout **crashes the caller**. There is no idle `on_timeout`
handler in v1.2.0.

### `gleam/otp/supervision`

Child-specification vocabulary only. `ChildSpecification(data)` has fields
`start` (`Result(actor.Started(data), actor.StartError)`), `restart`
(`Permanent`/`Transient`/`Temporary`, default `Permanent`), `significant`
(`Bool`, default `False`), and `child_type` (`Worker(shutdown_ms)` /
`Supervisor`).

Builders: `worker(start_fn)` (default shutdown 5000 ms), `supervisor(start_fn)`
(unlimited shutdown), `restart(spec, restart)`, `significant(spec, bool)`,
`timeout(spec, ms)` (ignored for supervisor children), `map_data(spec, fn)`.
The `start` field must return `actor.StartResult`, not a raw Erlang MFA.

### `gleam/otp/static_supervisor`

Fixed child list at construction time. Types: `Builder`, `Supervisor`,
`Strategy` (`OneForOne`, `OneForAll`, `RestForOne`), `AutoShutdown`
(`Never`, `AnySignificant`, `AllSignificant`).

Builder: `new(strategy)`, `restart_tolerance(intensity, period)`,
`auto_shutdown(value)`, `add(child)`, `start(builder)`, `supervised(builder)`.
`start` links to the caller; startup failure terminates all started children
and the supervisor itself. Prefer `supervised` for nesting.

### `gleam/otp/factory_supervisor`

Dynamic children started from a template at runtime. Types:
`Builder(child_argument, child_data)`, `Supervisor(child_argument, child_data)`,
`Message(child_argument, child_data)`.

Builder: `worker_child(template)`, `supervisor_child(template)`, `named(name)`,
`restart_strategy(restart)`, `restart_tolerance(intensity, period)`,
`timeout(ms)`, `start(builder)`, `supervised(builder)`. Runtime API:
`start_child(supervisor, argument)` and `get_by_name(name)`. `get_by_name`
**panics** if the name is unregistered; create `process.Name` once at startup
and thread it through.

### Mapping to BEAM OTP

| Gleam | BEAM OTP |
|-------|----------|
| `actor.new` / `new_with_initialiser` | `gen_server:init/1` |
| `actor.on_message` | `handle_info/2` (subsumes `handle_call/3` + `handle_cast/2`) |
| `actor.continue` | `{noreply, NewState}` |
| `actor.stop` | `{stop, normal, State}` |
| `actor.stop_abnormal(reason)` | `{stop, {shutdown, Reason}, State}` |
| `actor.call` | `gen_server:call/2,3` |
| `actor.send` | `gen_server:cast/2` / `erlang:send/2` |
| `actor.named` | `{local, Name}` registration |
| `Restart.Permanent` | `permanent` |
| `Restart.Transient` | `transient` |
| `Restart.Temporary` | `temporary` |
| `ChildType.Worker(ms)` | `shutdown => ms`, `type => worker` |
| `ChildType.Supervisor` | `shutdown => infinity`, `type => supervisor` |
| `static_supervisor.new(strategy)` | `sup_flags.strategy` |
| `static_supervisor.restart_tolerance(i, p)` | `intensity` (MaxR) + `period` (MaxT) |
| `static_supervisor.auto_shutdown` | `auto_shutdown` flag |
| `static_supervisor.start` | `supervisor:start_link/2,3` (linked) |
| `factory_supervisor.worker_child` / `supervisor_child` | single `child_spec` in `simple_one_for_one` |
| `factory_supervisor.start_child` | `supervisor:start_child/2` |

### Notable limitation

As of v1.2.0, actors do not yet support all OTP system messages; some debugging
APIs are incomplete and unsupported system messages are discarded.

## Practical rules

- Import `Subject` / `Selector` / `Name` from `gleam/erlang/process`.
- Use `actor.new` for simple state; use `actor.new_with_initialiser` for setup
  or custom selectors.
- Every `on_message` branch must return `Next`.
- Treat `call` timeouts as caller failures; supervise the caller.
- Use `supervision.worker` unless the child is itself a supervisor.
- Default restart is `Permanent`; use `Transient` for no-restart-on-normal-exit,
  `Temporary` for never-restart.
- `significant: True` only matters when parent `auto_shutdown` is not `Never`.
- Prefer `supervised` over direct `start`.
- Use `static_supervisor` for fixed topologies; `factory_supervisor` for dynamic
  children.
- Create `process.Name` once at startup.

## Review checklist

- [ ] `Subject` imported from `gleam/erlang/process`.
- [ ] `on_message` returns `Next` for every branch.
- [ ] `call` timeouts handled by caller supervision.
- [ ] `selecting` in an initialiser re-adds the default subject if still needed.
- [ ] Child spec `start` returns `actor.StartResult`.
- [ ] `supervision.supervisor` only used for supervisor children.
- [ ] `significant: True` only with non-`Never` `auto_shutdown`.
- [ ] Supervisors nested via `supervised`.
- [ ] Factory supervisors use one shared `process.Name`.
- [ ] `get_by_name` never called before registration is guaranteed.

## Implementation checklist

- [ ] Add `gleam_otp@1` and `gleam_erlang` dependencies.
- [ ] Define a single `Message` type per actor.
- [ ] Request/reply messages carry `process.Subject(reply)`.
- [ ] Build actor with `actor.new` / `on_message` / optional `named`.
- [ ] Start actor and extract `Started(pid, data)`.
- [ ] Build child specs with `supervision.worker` / `supervisor` and chain
      `restart`, `significant`, or `timeout`.
- [ ] Assemble `static_supervisor` with `new`, `restart_tolerance`,
      `auto_shutdown`, and `add`.
- [ ] For dynamic children: `worker_child(template) |> named(name) |> supervised`.
- [ ] Runtime: `get_by_name(name)` then `start_child(supervisor, argument)`.

## Validation hooks

```sh
gleam add gleam_otp@1
gleam build --target erlang
gleam test
```

## Examples

### Typed actor

```gleam
import gleam/erlang/process
import gleam/otp/actor

pub type Message {
  Increment
  Get(reply: process.Subject(Int))
}

pub fn start() {
  actor.new(0)
  |> actor.on_message(fn(state, msg) {
    case msg {
      Increment -> actor.continue(state + 1)
      Get(reply) -> {
        process.send(reply, state)
        actor.continue(state)
      }
    }
  })
  |> actor.start
}

let assert Ok(actor.Started(_pid, counter)) = start()
let value = actor.call(counter, 1000, Get)
```

### Actor with custom initialiser and selector

```gleam
import gleam/erlang/process
import gleam/otp/actor

pub type Message { Tick Stop }

pub fn start_ticker() {
  let subject = process.new_subject()
  actor.new_with_initialiser(1000, fn(default) {
    let selector =
      process.selecting(process.new_selector(), default, fn(_) { Tick })
      |> process.selecting(subject, fn(_) { Stop })
    actor.initialised(0)
    |> actor.selecting(selector)
    |> actor.returning(default)
  })
  |> actor.on_message(fn(state, msg) {
    case msg {
      Tick -> actor.continue(state + 1)
      Stop -> actor.stop()
    }
  })
  |> actor.start
}
```

### Supervisors

```gleam
import gleam/erlang/process
import gleam/otp/static_supervisor
import gleam/otp/factory_supervisor
import gleam/otp/supervision

pub fn top_level_supervisor() {
  static_supervisor.new(static_supervisor.OneForOne)
  |> static_supervisor.restart_tolerance(intensity: 3, period: 10)
  |> static_supervisor.add(supervision.worker(start_my_worker))
  |> static_supervisor.add(supervision.supervisor(start_my_child_supervisor))
  |> static_supervisor.add(start_connection_factory(connection_name))
  |> static_supervisor.supervised
}

pub type ConnectionId = String

pub fn start_connection_factory(name) {
  factory_supervisor.worker_child(fn(id: ConnectionId) {
    start_connection_handler(id)
  })
  |> factory_supervisor.named(name)
  |> factory_supervisor.restart_strategy(supervision.Transient)
  |> factory_supervisor.timeout(ms: 10_000)
  |> factory_supervisor.supervised
}

pub fn spawn_connection(supervisor, id) {
  factory_supervisor.start_child(supervisor, id)
}
```

## Common mistakes

- Using `with_message_handler` instead of `actor.on_message`.
- Assuming `Subject`, `init`, `Status`, `spec`, or `start_spec` exist in
  `gleam/otp/actor`.
- Assuming `actor.on_timeout` exists.
- Using `with_strategy`, `with_intensity`, `with_period`, `with_auto_shutdown`,
  or `start_link` on `static_supervisor`; use `new`, `restart_tolerance`,
  `auto_shutdown`, and `start`.
- Calling `factory_supervisor.new`; use `worker_child` or `supervisor_child`.
- Calling `process.new_name` repeatedly for the same logical name.
- Calling `get_by_name` before the supervisor is registered.
- Forgetting to re-add the default subject after `actor.selecting` overwrites
  the selector.
- Returning a raw Erlang start tuple from a child spec `start` field.

## Strict vs contextual guidance

Strict: use only v1.2.0 API names listed here; every `on_message` branch returns
`Next`; `call` timeouts crash the caller; child spec `start` returns
`actor.StartResult`; `get_by_name` panics on missing names.

Contextual: `actor.new` vs `new_with_initialiser` depends on setup needs;
`static_supervisor` vs `factory_supervisor` depends on fixed vs dynamic children;
`restart` and `significant` depend on fault model and `auto_shutdown`.

## Policy decisions for individual repos

- Decide where `process.Name` values are created and how they are threaded to
  callers.
- Decide factory supervisor `restart_strategy` (`Transient` typical, `Permanent`
  if every child must always restart).
- Decide `restart_tolerance` for each supervision layer rather than using
  defaults.
- Decide whether `significant` children and `auto_shutdown` are ever used; most
  projects should leave `significant: False` and `auto_shutdown: Never`.

## Related docs

- [`erlang-interop.md`](erlang-interop.md) — `process.Subject`, `Selector`,
  `Name`, `Pid`, FFI.
- [`externals-and-ffi.md`](externals-and-ffi.md) — calling BEAM code from Gleam.
- [`conventions-patterns-antipatterns.md`](conventions-patterns-antipatterns.md).
- [`deployment-and-runtime.md`](deployment-and-runtime.md) — running Gleam OTP
  in production.

## Related skills

- `gleam-otp-interop`
