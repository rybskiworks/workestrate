# Crawl: hexdocs.pm/elixir/Task.Supervisor.html
- seed_url: https://hexdocs.pm/elixir/Task.Supervisor.html
- canonical_url: https://elixir.hexdocs.pm/Task.Supervisor.html
- family: Elixir core module
- fetch: 200
- elixir_version: v1.20.2
- feeds_docs: otp-supervision.md, concurrency-processes.md

## Purpose
A task supervisor. This module defines a supervisor which can be used to
dynamically supervise tasks. A task supervisor is started with no children,
often under a supervisor and a name:

    children = [{Task.Supervisor, name: MyApp.TaskSupervisor}]
    Supervisor.start_link(children, strategy: :one_for_one)

Once started, you can start tasks directly under the supervisor:

    task = Task.Supervisor.async(MyApp.TaskSupervisor, fn -> :do_some_work end)

The Task.Supervisor is a single process responsible for starting other
processes. In some applications it may become a bottleneck; to address this,
start multiple instances via `PartitionSupervisor` and route through
`{:via, PartitionSupervisor, {name, key}}` (e.g. key = `self()`).

A Task.Supervisor is bound to the same name registration rules as a GenServer.

## Key functions
- `start_link(options \\ [])` — Starts a new supervisor. Options: `:name`
  (GenServer name registration), `:max_restarts`, `:max_seconds`,
  `:max_children` (as in DynamicSupervisor). The `:restart` and `:shutdown`
  options are deprecated here; pass them directly to `start_child`.
- `start_child(supervisor, fun, options \\ [])` — Starts a task as a child of
  the given supervisor. The spawned process is NOT linked to the caller, only
  to the supervisor. Useful for side-effect tasks where you don't care about
  the result. Options: `:restart` (`:temporary` default, `:transient`,
  `:permanent`), `:shutdown` (`:brutal_kill` or timeout, default 5000 ms).
- `start_child(supervisor, module, fun, args, options \\ [])` — MFA variant.
- `async(supervisor, fun, options \\ [])` — Starts a task that can be awaited.
  The task IS linked to the caller (see Task.async/1). Raises if supervisor has
  reached max children. Option `:shutdown` (`:brutal_kill` or timeout, default
  5000 ms; task must trap exits for timeout to take effect).
- `async(supervisor, module, fun, args, options \\ [])` — MFA variant of async.
- `async_nolink(supervisor, fun, options \\ [])` — Starts a task that can be
  awaited but is NOT linked to the caller. Requires the task supervisor to have
  `:temporary` as the `:restart` option (the default), since async_nolink keeps
  a direct reference to the task which is lost if the task is restarted.
  Reply format `{ref, result}`; caller always receives a `:DOWN` message with
  the same ref; `:normal` reason on normal termination.
- `async_nolink(supervisor, module, fun, args, options \\ [])` — MFA variant.
- `async_stream(supervisor, enumerable, fun, options \\ [])` (since 1.4.0) —
  Returns a stream running `fun` concurrently on each element. Tasks are spawned
  under the supervisor and LINKED to the caller (like async/3). Emits
  `{:ok, value}` or `{:exit, reason}` (if caller traps exits).
- `async_stream(supervisor, enumerable, module, function, args, options \\ [])`
  (since 1.4.0) — MFA variant.
- `async_stream_nolink(supervisor, enumerable, fun, options \\ [])` (since 1.4.0)
  — Non-linked variant. No dangling tasks left after stream halts; ongoing tasks
  shut down on halt.
- `async_stream_nolink(supervisor, enumerable, module, function, args, options \\ [])`
  (since 1.4.0) — MFA variant.
- `terminate_child(supervisor, pid)` — Terminates the child with the given pid.
  Returns `:ok | {:error, :not_found}`.
- `children(supervisor)` — Returns all children PIDs except those that are
  restarting. Note: calling this with a large number of children under low
  memory can bring the system down (OOM).

### async_stream options
- `:max_concurrency` — max tasks at the same time. Defaults to
  `System.schedulers_online/0`.
- `:ordered` — results in same order as input. Defaults to `true`.
- `:timeout` — max ms to wait without a task reply (across all running tasks).
  Defaults to 5000.
- `:on_timeout` — `:exit` (default), `:kill_task`, or `:zip_input_on_exit`
  (since v1.14.0).
- `:shutdown` — `:brutal_kill` or timeout. Defaults to 5000 ms.

## task restart/shutdown defaults
- `:restart` default for tasks: `:temporary` (task is never restarted).
  - `:transient` — restarted if exit is not `:normal`, `:shutdown`, or
    `{:shutdown, reason}`.
  - `:permanent` — always restarted.
- `:shutdown` default: `5000` milliseconds (5 seconds).
  - `:brutal_kill` kills the task directly on shutdown.
  - The task must trap exits for the timeout to have an effect.
- `async_nolink` requires `:restart` to be `:temporary` (the default), because
  it keeps a direct reference to the task which is lost if the task is restarted.
- Supervisor-level `:max_restarts` / `:max_seconds` / `:max_children` — as
  specified in `DynamicSupervisor`.
- The `:restart` and `:shutdown` options on `start_link` are DEPRECATED; pass
  them directly to `start_child` instead.

## Task.Supervisor ↔ BEAM supervisor mapping (→ docs/beam/supervision.md)
Task.Supervisor is built on top of `DynamicSupervisor` (Elixir's wrapper around
the OTP `supervisor` behaviour for dynamically added children). Mapping:

- `Task.Supervisor.start_link/1` ↔ `DynamicSupervisor.start_link/1` ↔
  `supervisor:start_link/2,3` with `:simple_one_for_one`-style dynamic strategy.
  The child spec is a task process; restart strategy `:temporary` by default.
- `start_child/3,5` ↔ `DynamicSupervisor.start_child/2` ↔
  `supervisor:start_child/2` — dynamically adds a child under the supervisor.
- `async`/`async_nolink` ↔ start a child via the supervisor AND link/monitor the
  caller (link for `async`, monitor for `async_nolink`). The task process is a
  child of the supervisor; the link/monitor is between task and caller, not
  between task and supervisor (the supervisor always owns the task).
- `terminate_child/2` ↔ `DynamicSupervisor.terminate_child/2` ↔
  `supervisor:terminate_child/2`.
- `children/1` ↔ `DynamicSupervisor.which_children/1` ↔
  `supervisor:which_children/1` (filtered to exclude restarting children).
- `:max_restarts` / `:max_seconds` ↔ supervisor `intensity`/`period` (the
  restart intensity limits of the OTP supervisor).
- `:max_children` ↔ DynamicSupervisor's cap on concurrent dynamic children.
- Default restart `:temporary` means a crashed task is NOT restarted by the
  supervisor — this is the key difference from a typical supervisor child and
  matches the "fire and forget" / "let it crash, caller handles" semantics of
  tasks. The supervisor's restart intensity is therefore mostly relevant when
  `:transient` or `:permanent` is chosen via `start_child`.

See docs/beam/supervision.md for the underlying OTP supervisor behaviour,
child specs, restart strategies (`:one_for_one`, `:simple_one_for_one`), and
intensity/period semantics.

## Strict rules
- `async_nolink/3,5` REQUIRES the task supervisor's `:restart` to be
  `:temporary` (the default). It keeps a direct reference to the task which is
  lost if the task is restarted.
- `start_child` spawns a process linked ONLY to the supervisor, NOT to the
  caller. Use it for side-effect tasks where you don't care about the result.
- `async` spawns a task LINKED to the caller; `async_nolink` does NOT link.
- For the `:shutdown` timeout to take effect, the task must trap exits.
- `:restart` and `:shutdown` on `start_link` are deprecated — pass them to
  `start_child` instead.
- When using `async_nolink` inside an OTP behaviour (e.g. GenServer), match on
  the `{ref, result}` message in `handle_info/2` and handle the `:DOWN`
  message (always sent, with `:normal` reason on normal termination).
- `children/1` can cause OOM under low memory with many children — use with
  care.
- Raises an error if the supervisor has reached the maximum number of children.

## Verbatim quotes
- "A task supervisor. This module defines a supervisor which can be used to
  dynamically supervise tasks."
- "The Task.Supervisor is a single process responsible for starting other
  processes. In some applications, the Task.Supervisor may become a bottleneck."
- "Note that the spawned process is not linked to the caller, but only to the
  supervisor." (start_child/3)
- ":temporary means the task is never restarted, :transient means it is
  restarted if the exit is not :normal , :shutdown or {:shutdown, reason} . A
  :permanent restart strategy means it is always restarted."
- ":shutdown - :brutal_kill if the task must be killed directly on shutdown or
  an integer indicating the timeout value, defaults to 5000 milliseconds. The
  task must trap exits for the timeout to have an effect."
- "Note this function requires the task supervisor to have :temporary as the
  :restart option (the default), as async_nolink/3 keeps a direct reference to
  the task which is lost if the task is restarted."
- "This function could also receive :restart and :shutdown as options but those
  two options have been deprecated and it is now preferred to give them directly
  to start_child ."
- "Note that calling this function when supervising a large number of children
  under low memory conditions can bring the system down due to an out of memory
  error." (children/1)
- "regardless of how the task created with async_nolink terminates, the
  caller's process will always receive a :DOWN message with the same ref value
  that is held by the task struct."

## Version notes
- Page built with ExDoc v0.40.3 for Elixir v1.20.2.
- `async_stream` / `async_stream_nolink` introduced in Elixir 1.4.0.
- `async_stream_option` type since 1.17.0.
- `:zip_input_on_exit` option since v1.14.0.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/elixir/Task.html (Task module — referenced repeatedly)
- https://hexdocs.pm/elixir/Supervisor.html (Supervisor)
- https://hexdocs.pm/elixir/DynamicSupervisor.html (DynamicSupervisor —
  underlying implementation)
- https://hexdocs.pm/elixir/PartitionSupervisor.html (partitioning for
  scalability)
- https://hexdocs.pm/elixir/GenServer.html (name registration rules)

### Skipped
- https://hexdocs.pm/ex_unit/ (ExUnit, unrelated)
- https://hexdocs.pm/mix/ (Mix, unrelated)
- https://hexdocs.pm/eex/ (EEx, unrelated)
- https://hexdocs.pm/logger/ (Logger, unrelated)
- https://hexdocs.pm/iex/ (IEx, unrelated)
- llms.txt, ePub download, package docs links
