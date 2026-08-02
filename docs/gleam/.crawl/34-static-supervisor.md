# Crawl: gleam/otp/static_supervisor.html
- seed_url: https://hexdocs.pm/gleam_otp/gleam/otp/static_supervisor.html
- canonical_url: https://gleam-otp.hexdocs.pm/gleam/otp/static_supervisor.html
- family: Gleam core package module
- fetch: 200
- gleam_otp_version: v1.2.0
- feeds_docs: otp-actors-and-supervision.md

## Purpose
A supervisor where the number and types of the children are specified once
(statically, at supervisor-construction time), and the supervisor manages them
using a configured restart strategy. It is a thin Gleam wrapper around
Erlang/OTP's `supervisor` module — it does NOT use Gleam subjects for message
passing. For further detail the page links to the Erlang docs:
https://www.erlang.org/doc/apps/stdlib/supervisor.html

## Builder API (signatures)
The builder is an opaque type; configuration is done by chaining functions that
each take and return a `Builder`. Actual function names (note: the crawl task's
hypothesized `with_strategy`/`with_intensity`/`with_period`/`with_auto_shutdown`
names do NOT exist — the real API uses `new`, `restart_tolerance`, `auto_shutdown`).

```gleam
pub opaque type Builder

// Create a new supervisor builder with a restart strategy.
pub fn new(strategy strategy: Strategy) -> Builder

// Set the restart intensity (MaxR) and period (MaxT, seconds).
// Defaults: intensity = 2, period = 5.
pub fn restart_tolerance(
  builder: Builder,
  intensity intensity: Int,
  period period: Int,
) -> Builder

// Configure automatic supervisor shutdown when significant children terminate.
pub fn auto_shutdown(
  builder: Builder,
  value: AutoShutdown,
) -> Builder

// Add a child to the supervisor. Child comes from supervision.ChildSpecification.
pub fn add(
  builder: Builder,
  child: supervision.ChildSpecification(data),
) -> Builder

// Start a new supervisor process from the builder. Links to caller.
pub fn start(
  builder: Builder,
) -> Result(actor.Started(Supervisor), actor.StartError)

// Wrap this supervisor as a ChildSpecification so it can be added to another
// supervisor (i.e. nested into the supervision tree).
pub fn supervised(
  builder: Builder,
) -> supervision.ChildSpecification(Supervisor)
```

Supporting public types:

```gleam
pub type Strategy {
  OneForOne   // default; only the terminated child is restarted
  OneForAll   // terminate all others, then restart all
  RestForOne  // terminate children after the failed one, then restart them + failed
}

pub type AutoShutdown {
  Never           // default; significant flag on children is rejected
  AnySignificant  // shutdown when any significant child terminates
  AllSignificant  // shutdown when the last active significant child terminates
}

pub opaque type Supervisor   // reference to the running supervisor process
```

## strategy / intensity / period / auto_shutdown mapping
- `Strategy` → Erlang `supervisor` `strategy` (`one_for_one` / `one_for_all` /
  `rest_for_one`). `OneForOne` is the default.
- `restart_tolerance(intensity, period)` → Erlang `intensity` (MaxR) and
  `period` (MaxT, seconds). If more than MaxR restarts occur within MaxT
  seconds, the supervisor terminates all children and then itself with reason
  `shutdown`. Defaults: intensity = 2, period = 5.
- `auto_shutdown(value)` → Erlang `auto_shutdown` child-spec key. Maps to
  `never` / `any_significant` / `all_significant`. With `Never`, child specs
  carrying the `significant` flag are rejected as invalid.

See docs/beam/supervision.md for the underlying BEAM supervisor semantics
(strategy, MaxR/MaxT intensity/period, auto_shutdown, significant children).

## start / start_link
Only `start` is exposed (there is NO `start_link` function in this module):

```gleam
pub fn start(builder: Builder) -> Result(actor.Started(Supervisor), actor.StartError)
```

- Starts a new supervisor process with the configuration and children in the
  builder.
- The supervisor is LINKED to the parent process that calls `start` (so it
  behaves like a `start_link` in Erlang terms — the linking is implicit).
- If any child fails to start, the supervisor first terminates all already
  started children with reason `shutdown`, then terminates itself, and returns
  an error.
- Typically you do NOT call `start` directly; instead use `supervised` to add
  the supervisor into a parent supervision tree.

## Static vs factory distinction
- `static_supervisor`: the child list is fixed at builder-construction time
  (compile-time / setup-time). Children are added via `add(...)` calls on the
  `Builder` before `start`/`supervised`. The set and order of children does not
  change at runtime. This mirrors Erlang's static `supervisor` child list.
- `factory_supervisor` (sibling module `gleam/otp/factory_supervisor`): the
  dynamic counterpart — children can be started on demand at runtime via a
  factory. Use `static_supervisor` when the children are known up front; use
  `factory_supervisor` when children must be created dynamically.

## Mapping to BEAM supervisor (→ docs/beam/supervision.md)
`static_supervisor` is a direct Gleam binding over the BEAM `supervisor`
behaviour (see docs/beam/supervision.md). Mapping table:

| Gleam                          | BEAM supervisor                       |
|--------------------------------|---------------------------------------|
| `new(OneForOne\|OneForAll\|RestForOne)` | `strategy` in `sup_flags`     |
| `restart_tolerance(intensity, period)`  | `intensity` (MaxR) + `period` (MaxT) |
| `auto_shutdown(Never\|AnySignificant\|AllSignificant)` | `auto_shutdown` flag |
| `add(child)`                   | child spec appended to `child_specs`  |
| `start(builder)`               | `supervisor:start_link/2,3` (linked) |
| `supervised(builder)`          | child spec wrapping this supervisor   |
| `Supervisor` (opaque)          | supervisor pid                        |

Note: Gleam's `start` links to the caller (Erlang `start_link` semantics); there
is no unlinked `start` variant exposed.

## Strict rules
- The child list is STATIC: it is fixed when the builder is turned into a
  supervisor via `start`/`supervised`. Do not use `static_supervisor` for
  runtime-spawned children — use `factory_supervisor` instead.
- Prefer `supervised` (nesting into a parent supervisor) over calling `start`
  directly, so the supervisor is part of the application's fault-tolerant
  supervision tree.
- With `auto_shutdown(Never)` (the default), child specs that set the
  `significant` flag are invalid and will be rejected.
- If any child fails to start, the supervisor terminates all already-started
  children with reason `shutdown` and then itself, returning an error.
- The supervisor process is linked to its parent; a parent crash will propagate
  per normal BEAM link semantics (see docs/beam/supervision.md and
  docs/beam/errors-failures.md).

## Verbatim quotes
- "A supervisor where the number and types of the children are specified once,
  and the supervisor manages them using a configured restart strategy."
- "This supervisor wrap Erlang/OTP's `supervisor` module, and as such it does
  not use subjects for message sending. If it was implemented in Gleam a
  subject might be used instead of this type."
- "If one child process terminates and is to be restarted, only that child
  process is affected. This is the default restart strategy." (OneForOne)
- "If one child process terminates and is to be restarted, all other child
  processes are terminated and then all child processes are restarted." (OneForAll)
- "If one child process terminates and is to be restarted, the 'rest' of the
  child processes (that is, the child processes after the terminated child
  process in the start order) are terminated. Then the terminated child
  process and all child processes after it are restarted." (RestForOne)
- "To prevent a supervisor from getting into an infinite loop of child process
  terminations and restarts, a maximum restart tolerance is defined using two
  integer values specified with keys intensity and period... if more than MaxR
  restarts occur within MaxT seconds, the supervisor terminates all child
  processes and then itself. The termination reason for the supervisor itself
  in that case will be shutdown. Intensity defaults to 2 and period defaults
  to 5."
- "Automic shutdown is disabled. This is the default setting. With auto_shutdown
  set to never, child specs with the significant flag set to true are
  considered invalid and will be rejected." (Never)
- "The supervisor will be linked to the parent process that calls this
  function." (start)
- "Typically you would use the `supervised` function to add your supervisor to
  a supervision tree instead of using this function directly." (start)
- "Create a `ChildSpecification` that adds this supervisor as the child of
  another, making it fault tolerant and part of the application's supervision
  tree. You should prefer to starting unsupervised supervisors with the `start`
  function." (supervised)

## Version notes
- Page reports gleam_otp v1.2.0 (source links point to
  https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/static_supervisor.gleam).
- ExDoc version: 1.13.0-rc1 (CSS/JS asset version).
- Canonical URL: https://gleam-otp.hexdocs.pm/gleam/otp/static_supervisor.html
  (hexdocs.pm URL redirects here).
- In v1.2.0 the builder API uses `restart_tolerance` (combined intensity+period)
  and `auto_shutdown` rather than separate `with_intensity`/`with_period`/
  `with_auto_shutdown` functions. There is no `start_link` — only `start`
  (which links to the caller).

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
  (gleam/otp/supervision — ChildSpecification, child spec model; referenced by
  `add` and `supervised` signatures)
- https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html
  (gleam/otp/factory_supervisor — the dynamic/factory counterpart to
  static_supervisor)
- https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
  (gleam/otp/actor — `Started`, `StartError`, `StartResult` used by `start`)
- https://www.erlang.org/doc/apps/stdlib/supervisor.html
  (Erlang/OTP supervisor module — underlying behaviour; cross-reference with
  docs/beam/supervision.md locally)

### Skipped
- https://gleam.run/ (website)
- https://github.com/sponsors/lpil (sponsor)
- https://github.com/gleam-lang/otp (repo)
- https://hex.pm/packages/gleam_otp (hex package)
- https://hexdocs.pm/gleam_otp/gleam/otp/port.html (port module — unrelated)
- https://hexdocs.pm/gleam_otp/gleam/otp/system.html (system module — unrelated)
- https://hexdocs.pm/gleam_otp/index.html (README)
- GitHub source blob links (per-function source anchors)
