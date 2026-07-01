# Crawl: gleam/otp/factory_supervisor.html
- seed_url: https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html
- canonical_url: https://gleam-otp.hexdocs.pm/gleam/otp/factory_supervisor.html
- family: Gleam core package module
- fetch: 200
- gleam_otp_version: v1.2.0
- feeds_docs: otp-actors-and-supervision.md

## Purpose
A supervisor where children are started dynamically at runtime from a single
"template function", rather than being enumerated statically at construction
time. The template is a `fn(child_argument) -> Result(actor.Started(child_data),
actor.StartError)` that starts a linked child process; the supervisor calls it
each time a new child is requested via `start_child`.

Like `static_supervisor`, this module is a thin Gleam wrapper over
Erlang/OTP's `supervisor` module — it does NOT use Gleam subjects for message
passing. The page links to the Erlang docs:
https://www.erlang.org/doc/apps/stdlib/supervisor.html

The factory supervisor is parameterised by two type parameters:
- `child_argument` — the argument passed to the template function for each
  child (e.g. a request path, a connection id);
- `child_data` — the data type returned by the child's start function (the
  `actor.Started(child_data)` payload).

## Builder API (signatures)
The builder is an opaque type parameterised by `(child_argument, child_data)`.
Configuration is done by chaining functions that each take and return a
`Builder`. The template function is supplied via `worker_child` (or
`supervisor_child` for supervisor children).

```gleam
pub opaque type Builder(child_argument, child_data)
pub opaque type Supervisor(child_argument, child_data)
pub type Message(child_argument, child_data)   // message type of the name

// Create a builder from a template that starts WORKER children.
pub fn worker_child(
  template: fn(child_argument) -> Result(actor.Started(child_data), actor.StartError),
) -> Builder(child_argument, child_data)

// Create a builder from a template that starts SUPERVISOR children.
// Use only when children are themselves supervisors; supervisor children have
// an unlimited shutdown timeout (no timeout).
pub fn supervisor_child(
  template: fn(child_argument) -> Result(actor.Started(child_data), actor.StartError),
) -> Builder(child_argument, child_data)

// Register the supervisor under a name when started, so other processes can
// contact it via get_by_name. If the name is already registered the supervisor
// fails to start.
pub fn named(
  builder: Builder(child_argument, child_data),
  name: process.Name(Message(child_argument, child_data)),
) -> Builder(child_argument, child_data)

// Restart strategy for children (supervision.Restart). Default: Transient
// (restart only on abnormal termination).
pub fn restart_strategy(
  builder: Builder(argument, data),
  restart_strategy: supervision.Restart,
) -> Builder(argument, data)

// MaxR/MaxT restart tolerance. Defaults: intensity = 2, period = 5.
pub fn restart_tolerance(
  builder: Builder(child_argument, child_data),
  intensity intensity: Int,
  period period: Int,
) -> Builder(child_argument, child_data)

// Shutdown timeout in ms for worker children (default 5000). Ignored for
// supervisor children.
pub fn timeout(
  builder: Builder(argument, data),
  ms ms: Int,
) -> Builder(argument, data)

// Start a new supervisor process from the builder. LINKED to caller.
pub fn start(
  builder: Builder(child_argument, child_data),
) -> Result(actor.Started(Supervisor(child_argument, child_data)), actor.StartError)

// Wrap this supervisor as a ChildSpecification for nesting into a parent
// supervision tree (preferred over calling start directly).
pub fn supervised(
  builder: Builder(child_argument, child_data),
) -> supervision.ChildSpecification(Supervisor(child_argument, child_data))
```

Runtime child-start and name-lookup API (these operate on a running
`Supervisor`, not on a `Builder`):

```gleam
// Start a new child under the supervisor using its template + the given
// argument. Returns the child's start result.
pub fn start_child(
  supervisor: Supervisor(child_argument, child_data),
  argument: child_argument,
) -> Result(actor.Started(child_data), actor.StartError)

// Get a reference to a named supervisor. PANICS if no supervisor is
// registered under the name at call time.
pub fn get_by_name(
  name: process.Name(Message(child_argument, child_data)),
) -> Supervisor(child_argument, child_data)
```

## Dynamic child add at runtime
Unlike `static_supervisor`, children are NOT declared in the builder. The
builder only carries:
- the template function (`worker_child` / `supervisor_child`);
- restart strategy, restart tolerance, shutdown timeout;
- an optional registered name.

New children are spawned on demand by calling `start_child(supervisor,
argument)`. The supervisor invokes the template function with `argument`,
links the resulting process, and returns its `actor.Started(child_data)`.
The `child_argument` type lets each child be parameterised at start time
(e.g. per-HTTP-request data).

Typical usage pattern (from the page's Usage section):
1. At program start, create a `process.Name` once and pass it down to any
   process that will need to start children.
2. Build the factory supervisor with `worker_child(template) |> named(name)
   |> supervised`, and add it to a top-level `static_supervisor`.
3. At runtime, other processes call `get_by_name(name)` to obtain a
   `Supervisor` reference, then `start_child(supervisor, argument)` to spawn
   a new child.

The page stresses: each `process.new_name(...)` value is unique — calling it
twice yields different names even with the same string argument. Create the
name once at startup and thread it through.

## factory vs static distinction
- `static_supervisor` (`gleam/otp/static_supervisor`): the child list is fixed
  at builder-construction time via `add(child_spec)` calls. The set and order
  of children does not change at runtime. Mirrors Erlang's static supervisor
  child list.
- `factory_supervisor` (`gleam/otp/factory_supervisor`): the dynamic
  counterpart. Children are NOT enumerated up front; instead a single template
  function is registered, and an unbounded number of children can be started
  at runtime via `start_child`. Use `static_supervisor` when children are
  known up front; use `factory_supervisor` when children must be created
  dynamically (e.g. one per incoming request).

Both share the same `supervision.ChildSpecification` model (via `supervised`),
the same `supervision.Restart` restart-strategy type, and the same
MaxR/MaxT `restart_tolerance(intensity, period)` semantics.

## Mapping to BEAM DynamicSupervisor / simple_one_for_one (→ docs/beam/supervision.md)
`factory_supervisor` is the Gleam binding over the BEAM dynamic-supervisor
pattern. In classic Erlang/OTP this is `supervisor:start_child/2` with a
`simple_one_for_one` strategy (a single child spec reused for all dynamically
started children); in modern Elixir it is `DynamicSupervisor`. The Gleam
module exposes the same capability: one template (child spec) + on-demand
`start_child`.

Mapping table (cross-reference docs/beam/supervision.md for the underlying
BEAM semantics):

| Gleam `factory_supervisor`            | BEAM (Erlang/OTP supervisor)                |
|----------------------------------------|----------------------------------------------|
| `worker_child(template)` / `supervisor_child(template)` | the single `child_spec` in a `simple_one_for_one` supervisor (the "template") |
| `start_child(supervisor, argument)`    | `supervisor:start_child/2` (dynamic start)   |
| `restart_strategy(supervision.Restart)` | child `restart` field (`permanent`/`temporary`/`transient`) — note: this is the per-child restart type, NOT the supervisor strategy (a factory supervisor is effectively `simple_one_for_one`) |
| `restart_tolerance(intensity, period)` | `intensity` (MaxR) + `period` (MaxT) in `sup_flags` |
| `timeout(ms)`                          | child `shutdown` value (brutal_kill after timeout) |
| `named(name)`                          | `register(Name, Pid)` of the supervisor      |
| `get_by_name(name)`                    | name lookup of the registered supervisor pid |
| `start(builder)`                       | `supervisor:start_link/2,3` (linked)         |
| `supervised(builder)`                  | child spec wrapping this supervisor          |
| `Supervisor` (opaque)                  | supervisor pid                               |

Note: Gleam's `start` links to the caller (Erlang `start_link` semantics);
there is no unlinked `start` variant and no separate `start_link` function
exposed. See docs/beam/supervision.md for `simple_one_for_one` /
`DynamicSupervisor` semantics, MaxR/MaxT, and shutdown behaviour.

## Strict rules
- Children are NOT declared in the builder — only the template function and
  supervisor-level config. Use `static_supervisor` for a fixed child list.
- The template function must start a LINKED child process and return
  `Result(actor.Started(child_data), actor.StartError)`.
- Use `supervisor_child` ONLY when children are themselves supervisors;
  supervisor children have an unlimited shutdown timeout (no timeout). Use
  `worker_child` otherwise.
- `timeout(ms)` only applies to worker children; it is ignored for supervisor
  children. Default worker shutdown timeout is 5000ms.
- `get_by_name` PANICS if no factory supervisor is registered under the name
  at call time — always ensure supervisors are themselves supervised and
  started before callers use the name.
- `process.new_name(...)` values are unique per call; create the name once at
  program start and thread it through to all callers. Two names created with
  the same string argument are different names.
- If the name is already registered to another process, the factory
  supervisor fails to start.
- Prefer `supervised` (nesting into a parent supervisor) over calling `start`
  directly, so the supervisor is part of the application's fault-tolerant
  supervision tree.
- If any child fails to start, the supervisor terminates all already-started
  children with reason `shutdown` and then itself, returning an error.
- The supervisor process is linked to its parent; a parent crash propagates
  per normal BEAM link semantics (see docs/beam/supervision.md and
  docs/beam/errors-failures.md).

## Verbatim quotes
- "A builder for configuring and starting a supervisor. See each of the
  functions that take this type for details of the configuration possible."
  (Builder)
- "The message type of a factory supervisor. This message type is not used
  directly, but if you are using a name with a factory supervisor then this
  will be the message type of the name." (Message)
- "A reference to the running supervisor. This supervisor wrap Erlang/OTP's
  `supervisor` module, and as such it does not use subjects for message
  sending. If it was implemented in Gleam a subject might be used instead of
  this type." (Supervisor)
- "Add the factory supervisor to your supervision tree using the supervised
  function and a name created at the start of the program. The new function
  takes a 'template function', which is a function that takes one argument
  and starts a linked child process." (Usage)
- "You most likely want to give the factory supervisor a name, and to pass
  that name to any other processes that will want to cause new child
  processes to be started under the factory supervisor." (Usage)
- "Any process with the name of the factory supervisor can use the
  get_by_name function to get a reference to the supervisor, and then use the
  start_child function to have it start new child processes." (Usage)
- "Remember! Each process name created with process.new_name is unique. Two
  names created by calling the function twice are different names, even if
  the same string is given as an argument. You must create the name value at
  the start of your program and then pass it down into application code and
  library code that uses names." (Usage)
- "Get a reference to a supervisor using its registered name. If no supervisor
  has been started using this name then functions using this reference will
  fail." (get_by_name)
- "Functions using the Supervisor reference returned by this function will
  panic if there is no factory supervisor registered with the name when they
  are called. Always make sure your supervisors are themselves supervised."
  (get_by_name Panics)
- "Provide a name for the supervisor to be registered with when started,
  enabling it be more easily contacted by other processes. This is useful for
  enabling processes that can take over from an older one that has exited due
  to a failure. If the name is already registered to another process then the
  factory supervisor will fail to start." (named)
- "Configure the strategy for restarting children when they exit. See the
  documentation for the supervision.Restart for details. If not set the
  default strategy is supervision.Transient, so children will be restarted if
  they terminate abnormally." (restart_strategy)
- "To prevent a supervisor from getting into an infinite loop of child
  process terminations and restarts, a maximum restart tolerance is defined
  using two integer values specified with keys intensity and period...
  if more than MaxR restarts occur within MaxT seconds, the supervisor
  terminates all child processes and then itself. The termination reason for
  the supervisor itself in that case will be shutdown. Intensity defaults to
  2 and period defaults to 5." (restart_tolerance)
- "Start a new supervisor process with the configuration and child template
  specified within the builder. Typically you would use the supervised
  function to add your supervisor to a supervision tree instead of using
  this function directly. The supervisor will be linked to the parent process
  that calls this function." (start)
- "Start a new child using the supervisor's child template and the given
  argument. The start result of the child is returned." (start_child)
- "Create a ChildSpecification that adds this supervisor as the child of
  another, making it fault tolerant and part of the application's supervision
  tree. You should prefer to starting unsupervised supervisors with the start
  function. If any child fails to start the supevisor first terminates all
  already started child processes with reason shutdown and then terminate
  itself and returns an error." (supervised)
- "Configure a supervisor with a template that will start children that are
  also supervisors. You should only use this if the child processes are also
  supervisors. Supervisor children have an unlimited amount of time to
  shutdown, there is no timeout." (supervisor_child)
- "Configure the amount of milliseconds a child has to shut down before being
  brutal killed by the supervisor. If not set the default for a child is
  5000ms. This will be ignored if the child is a supervisor itself." (timeout)
- "Configure a supervisor with a child-starting template function. You should
  use this unless the child processes are also supervisors. The default
  shutdown timeout is 5000ms. This can be changed with the timeout function."
  (worker_child)

## Version notes
- Page reports gleam_otp v1.2.0 (title: "gleam/otp/factory_supervisor ·
  gleam_otp · v1.2.0"; source links point to
  https://github.com/gleam-lang/otp/blob/v1.2.0/src/gleam/otp/factory_supervisor.gleam).
- The builder API in v1.2.0 uses `worker_child`/`supervisor_child` (template
  injection) rather than a `new(template)` function — the page's prose says
  "The new function takes a 'template function'" but the actual v1.2.0 API
  has no `new` function; the template is supplied via `worker_child` or
  `supervisor_child`. (The prose likely predates a rename.)
- `restart_strategy` takes a `supervision.Restart` (per-child restart type:
  Permanent/Temporary/Transient), NOT a supervisor `Strategy`
  (OneForOne/OneForAll/RestForOne) as in `static_supervisor`. This reflects
  the `simple_one_for_one` model where only one strategy exists.
- `restart_tolerance(intensity, period)` and `timeout(ms)` mirror the
  `static_supervisor` API.
- Only `start` is exposed (links to caller); there is no `start_link`.
- Canonical URL: https://gleam-otp.hexdocs.pm/gleam/otp/factory_supervisor.html
  (hexdocs.pm URL redirects here).

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
  (gleam/otp/supervision — `ChildSpecification`, `Restart` (Permanent/
  Temporary/Transient); referenced by `restart_strategy`, `supervised`)
- https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
  (gleam/otp/actor — `Started`, `StartError`, `StartResult` used by `start`,
  `start_child`, `worker_child`/`supervisor_child` template signature)
- https://hexdocs.pm/gleam_erlang/1.3.0/gleam/erlang/process.html#Name
  (gleam/erlang/process — `Name` type used by `named`/`get_by_name`;
  `new_name` for unique name creation)
- https://www.erlang.org/doc/apps/stdlib/supervisor.html
  (Erlang/OTP supervisor module — underlying behaviour; cross-reference with
  docs/beam/supervision.md locally for `simple_one_for_one` /
  `DynamicSupervisor` semantics)

### Skipped
- https://gleam.run/ (website)
- https://github.com/sponsors/lpil (sponsor)
- https://github.com/gleam-lang/otp (repo)
- https://hex.pm/packages/gleam_otp (hex package)
- https://hexdocs.pm/gleam_otp/index.html (README)
- https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html (self)
- GitHub source blob links (per-function source anchors under
  /blob/v1.2.0/src/gleam/otp/factory_supervisor.gleam#L...)
