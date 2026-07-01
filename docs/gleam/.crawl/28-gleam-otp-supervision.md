# Crawl: gleam/otp/supervision.html
- seed_url: https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
- canonical_url: https://gleam-otp.hexdocs.pm/gleam/otp/supervision.html
- family: Gleam core package module
- fetch: 200
- gleam_otp_version: v1.2.0
- feeds_docs: otp-actors-and-supervision.md

## Purpose

The `gleam/otp/supervision` module is the **child-specification vocabulary** for
Gleam OTP supervisors. It defines the `ChildSpecification(data)` record, the
`Restart` and `ChildType` enums, and a small set of builder/transformer functions
(`worker`, `supervisor`, `restart`, `significant`, `timeout`, `map_data`).

It does **not** itself start a supervisor — it only describes children. Starting
behaviour lives in `gleam/otp/static_supervisor` (fixed, compile-time child list)
and `gleam/otp/factory_supervisor` (dynamic, externally-added children). Those
modules consume `ChildSpecification` values produced here.

The `start` field of every `ChildSpecification` returns
`Result(actor.Started(data), actor.StartError)`, tying supervision directly to
the `gleam/otp/actor` start protocol rather than to raw `supervisor:start_child`.

## child/Child spec builder (child types + builder functions)

### `ChildSpecification(data)` — the spec record

```gleam
pub type ChildSpecification(data) {
  ChildSpecification(
    start: fn() -> Result(actor.Started(data), actor.StartError),
    restart: Restart,
    significant: Bool,
    child_type: ChildType,
  )
}
```

Fields:
- **`start`** — "A function to call to start the child process." Must return
  `actor.Started(data)` or an `actor.StartError`. This is the Gleam-actor start
  contract, not a raw Erlang `start_link` MFA.
- **`restart`** — when the child is restarted. See `Restart` below. Default
  `Permanent`.
- **`significant`** — whether the child counts toward supervisor auto-shutdown.
  "You most likely do not want to consider any children significant." Ignored
  when the supervisor's auto-shutdown is `Never` (the default). Default `False`.
- **`child_type`** — `Worker(shutdown_ms)` or `Supervisor`.

### `ChildType` — worker vs supervisor

```gleam
pub type ChildType {
  Worker(shutdown_ms: Int)
  Supervisor
}
```

- **`Worker(shutdown_ms)`** — "A worker child has to shut-down within a given
  amount of time." `shutdown_ms` is the milliseconds the child is given to shut
  down. The supervisor tells the child to terminate via
  `exit(Child, shutdown)` and waits for an exit signal with reason `shutdown`.
  If none arrives within `shutdown_ms`, the child is unconditionally terminated
  via `exit(Child, kill)`.
- **`Supervisor`** — a child that is itself a supervisor. Supervisor children
  have **unlimited** shutdown time (no timeout).

### `Restart` — restart semantics

```gleam
pub type Restart {
  Permanent
  Transient
  Temporary
}
```

- **`Permanent`** — "A permanent child process is always restarted." (default)
- **`Transient`** — "restarted only if it terminates abnormally, that is, with
  another exit reason than `normal`, `shutdown`, or `{shutdown,Term}`."
- **`Temporary`** — "never restarted (even when the supervisor's restart strategy
  is `RestForOne` or `OneForAll` and a sibling's death causes the temporary
  process to be terminated)."

### Builder functions

| Function | Signature | Role |
|---|---|---|
| `worker` | `(fn() -> Result(actor.Started(data), actor.StartError)) -> ChildSpecification(data)` | "A regular child process. You should use this unless your process is also a supervisor." Default shutdown timeout **5000ms** (changeable via `timeout`). |
| `supervisor` | `(fn() -> Result(actor.Started(data), actor.StartError)) -> ChildSpecification(data)` | "A special child that is a supervisor itself. Supervisor children have an unlimited shutdown time, there is no timeout." |
| `restart` | `(ChildSpecification(data), Restart) -> ChildSpecification(data)` | Set the `restart` field. Default is `Permanent`. |
| `significant` | `(ChildSpecification(data), Bool) -> ChildSpecification(data)` | Set the `significant` field. Default `False`. Ignored if supervisor auto-shutdown is `Never`. |
| `timeout` | `(ChildSpecification(data), Int) -> ChildSpecification(data)` | Set worker shutdown milliseconds. Default 5000ms. "This will be ignored if the child is a supervisor itself." |
| `map_data` | `(ChildSpecification(a), fn(a) -> b) -> ChildSpecification(b)` | "Transform the data of the started child process." Lets a parent supervisor hold a different `data` type than the child's `actor.Started` payload. |

Note: there is **no `id`/`start`/`prepare`/`wait_for`/`stop`** builder function
in this module. The `start` field is set by `worker`/`supervisor` (the only two
constructors that take a start function); `id` is not modelled here — child
identification is handled by the consuming supervisor modules
(`static_supervisor` assigns ids from start order; `factory_supervisor` accepts
an id at add-time). `prepare`/`wait_for`/`stop` are not part of this module's
API surface in v1.2.0.

## Restart strategy/intensity/period mapping

**Not present in this module.** The `gleam/otp/supervision` module exposes only
per-child `Restart` (permanent/transient/temporary) and per-child
`significant`/`shutdown_ms`. The supervisor-level **restart strategy**
(`one_for_one` / `one_for_all` / `rest_for_one`), **intensity** (MaxR), and
**period** (MaxT) are configured on the supervisor itself, in
`gleam/otp/static_supervisor` and `gleam/otp/factory_supervisor`, not here.

Mapping to BEAM (see `docs/beam/supervision.md`):

| Gleam `supervision` concept | BEAM `child_spec()` key | BEAM value |
|---|---|---|
| `Restart.Permanent` | `restart` | `permanent` |
| `Restart.Transient` | `restart` | `transient` |
| `Restart.Temporary` | `restart` | `temporary` |
| `ChildType.Worker(shutdown_ms)` | `shutdown` + `type` | `shutdown_ms`, `type => worker` |
| `ChildType.Supervisor` | `shutdown` + `type` | `infinity`, `type => supervisor` |
| `significant: Bool` | `significant` | `true`/`false` (OTP 24.0+) |
| `start` (Gleam actor start fn) | `start` / `modules` | wrapped: the Gleam start function is invoked, returning `actor.Started`; the underlying pid is linked under the supervisor. `modules => [gleam@otp@actor]`-style dynamic. |

The supervisor flags (`strategy`, `intensity`, `period`, `auto_shutdown`) are
set by `static_supervisor`/`factory_supervisor`, not by `ChildSpecification`.

## Relationship to static_supervisor / factory_supervisor

- **`gleam/otp/supervision`** — vocabulary only: `ChildSpecification`, `Restart`,
  `ChildType`, and the builder/transformer functions. Produces values; starts
  nothing.
- **`gleam/otp/static_supervisor`** — consumes a **fixed, ordered list** of
  `ChildSpecification` values known at supervisor-start time. Maps to a BEAM
  `supervisor` with a normal child spec list (strategy `one_for_one` /
  `one_for_all` / `rest_for_one`). Child ids are derived from start order.
- **`gleam/otp/factory_supervisor`** — consumes `ChildSpecification` values that
  are **added dynamically at runtime**. Maps to BEAM `simple_one_for_one` (all
  children are instances added via `start_child` after the supervisor starts)
  or to a dynamic-supervisor-style externally-added model. The id is supplied at
  add-time.

So the split is: `supervision` = "what a child is"; `static_supervisor` =
"fixed list of children, started together"; `factory_supervisor` = "children
added on demand after the supervisor is running".

## Mapping to BEAM supervisor + child_spec (→ docs/beam/supervision.md)

Cross-reference: `docs/beam/supervision.md` — sections "child_spec map",
"sup_flags()", "Restart values", "auto_shutdown() values", "intensity/period
semantics".

A Gleam `ChildSpecification(data)` corresponds to one BEAM `child_spec()` map
entry:

```erlang
#{
  id          => <derived by static_/factory_supervisor>,
  start       => {Mod, Fun, Args},   % wraps the Gleam start fn
  restart     => permanent | transient | temporary,  % from Restart
  shutdown    => integer() | infinity,                % from ChildType
  type        => worker | supervisor,                 % from ChildType
  significant => boolean()                             % from significant field
}
```

Key points (from `docs/beam/supervision.md`):
- `shutdown` for a `Worker(ms)` → `ms` (default 5000); for `Supervisor` →
  `infinity`. Matches the Gleam rule "Supervisor children have an unlimited
  shutdown time."
- `significant => true` is **invalid** when the supervisor's `auto_shutdown` is
  `never` (the default). This is why Gleam's doc says "You most likely do not
  want to consider any children significant" and "This will be ignored if the
  supervisor auto shutdown is set to `Never`."
- `temporary` children are never restarted even under `one_for_all` /
  `rest_for_one` sibling death — verbatim in both Gleam and BEAM docs.
- The supervisor flags (`strategy`, `intensity`, `period`, `auto_shutdown`,
  `hibernate_after`) are **not** modelled in `gleam/otp/supervision`; they live
  on the supervisor modules. See `docs/beam/supervision.md` "sup_flags()" for
  the BEAM side.

## Strict rules

- `worker` is the default choice; use `supervisor` only when the child is itself
  a supervisor.
- Supervisor children (`supervisor(...)`) have **unlimited** shutdown — `timeout`
  is ignored for them.
- Default worker shutdown is **5000ms**; override with `timeout`.
- Default `restart` is **`Permanent`**.
- Default `significant` is **`False`**; `significant=True` is only meaningful when
  the supervisor's `auto_shutdown` is not `Never`.
- `start` must return `Result(actor.Started(data), actor.StartError)` — the
  Gleam actor start contract, not a raw Erlang MFA tuple.
- `map_data` only transforms the carried `data` type; it does not change restart,
  significant, or child_type.

## Verbatim quotes

> "A description a how to start a new child process under an OTP supervisor."
> (ChildSpecification)

> "A function to call to start the child process." (start field)

> "This defines if a child is considered significant for automatic self-shutdown
> of the supervisor. You most likely do not want to consider any children
> significant. This will be ignored if the supervisor auto shutdown is set to
> Never, which is the default." (significant)

> "A worker child has to shut-down within a given amount of time." (Worker)

> "The supervisor tells the child process to terminate by calling
> exit(Child,shutdown) and then wait for an exit signal with reason shutdown
> back from the child process. If no exit signal is received within the
> specified number of milliseconds, the child process is unconditionally
> terminated using exit(Child,kill)." (Worker.shutdown_ms)

> "A permanent child process is always restarted." (Permanent)

> "A transient child process is restarted only if it terminates abnormally, that
> is, with another exit reason than normal, shutdown, or {shutdown,Term}."
> (Transient)

> "A temporary child process is never restarted (even when the supervisor's
> restart strategy is RestForOne or OneForAll and a sibling's death causes the
> temporary process to be terminated)." (Temporary)

> "A special child that is a supervisor itself. Supervisor children have an
> unlimited shutdown time, there is no timeout." (supervisor)

> "A regular child process. You should use this unless your process is also a
> supervisor. The default shutdown timeout is 5000ms. This can be changed with
> the timeout function." (worker)

> "The default value for restart is Permanent." (restart)

> "The default value for significance is False." (significant)

> "If not set the default for a child is 5000ms." (timeout)

> "This will be ignored if the child is a supervisor itself." (timeout)

> "Transform the data of the started child process." (map_data)

## Version notes

- gleam_otp **v1.2.0** (from page header "gleam_otp · v1.2.0").
- `significant` / auto-shutdown support implies OTP 24.0+ underneath (BEAM side).
- No `id`, `prepare`, `wait_for`, or `stop` builder in this module as of v1.2.0;
  `id` is the consuming supervisor's responsibility.
- No supervisor-flag (strategy/intensity/period) exposure here — see
  `static_supervisor` / `factory_supervisor`.

## Discovered links

### Relevant (crawl later)

- `gleam/otp/static_supervisor.html` — consumes `ChildSpecification`; holds
  strategy/intensity/period. (Crawl next for the supervisor-flags side.)
- `gleam/otp/factory_supervisor.html` — dynamic-add supervisor; consumes
  `ChildSpecification` at runtime.
- `gleam/otp/actor.html` — defines `actor.Started` / `actor.StartError` used by
  the `start` field.
- `gleam/otp/system.html` — system/supervisor-tree utilities (likely).

### Skipped

- README, Website, Sponsor, Repository, Hex (boilerplate nav).
- `gleam/otp/port.html` — unrelated to supervision.
