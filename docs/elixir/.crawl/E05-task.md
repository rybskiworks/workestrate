# Crawl: hexdocs.pm/elixir/Task.html
- seed_url: https://hexdocs.pm/elixir/Task.html
- canonical_url: https://elixir.hexdocs.pm/Task.html
- family: Elixir core module
- fetch: HTTP 200
- elixir_version: v1.20.2
- feeds_docs: concurrency-processes.md, otp-supervision.md

## Purpose
Conveniences for spawning and awaiting tasks. Tasks are processes meant to execute one
particular action throughout their lifetime, often with little or no communication with
other processes. The most common use case is to convert sequential code into concurrent
code by computing a value asynchronously:

```elixir
task = Task.async(fn -> do_some_work() end)
res  = do_some_other_work()
res + Task.await(task)
```

Tasks spawned with `async` can be awaited on by their caller process (and only their
caller). They are implemented by spawning a process that sends a message to the caller
once the computation is performed. Compared to plain `spawn/1` processes, tasks include
monitoring metadata and logging in case of errors. Besides `async/1` and `await/2`, tasks
can be started as part of a supervision tree and dynamically spawned on remote nodes.

## async/await/yield/shutdown/ignore model

### async/1 (and async/3)
- Spawns a process that is **linked to AND monitored by** the caller.
- Returns a `%Task{}` struct (`:mfa`, `:owner`, `:pid`, `:ref`).
- If you start an `async`, you MUST `await` (or `yield` + `shutdown`).
- Linking is intentional: it aborts the task if the parent dies, and guarantees the
  code before `async/await` has the same failure properties after you add the async
  call. "an asynchronous task should be thought of as an extension of the caller
  process rather than a mechanism to isolate it from all errors."
- To avoid linking the caller, use `Task.Supervisor.async_nolink/2`.

### await/2 (default timeout 5000ms)
- Awaits a task reply and returns it.
- If the task process dies, the caller exits with the same reason.
- On timeout, the caller exits; if the task is linked (the `async` case) it also exits;
  if it is trapping exits or not linked, it continues to run.
- Can only be called once per task. For repeated checks use `yield/2`.
- Assumes the task's monitor is still active OR the `:DOWN` message is in the queue.
- Not recommended to `await` long-running tasks inside OTP behaviours (GenServer);
  instead match `{ref, result}` and `{:DOWN, ref, :process, pid, reason}` in
  `handle_info/2`. Prefer `Task.Supervisor.async_nolink/3` inside OTP behaviours.

### await_many/2 (since 1.11.0)
- Awaits replies from a list of tasks; returns results in the same order as input.
- Same death/timeout semantics as `await/2` (caller exits if any task dies).

### yield/2 (default timeout 5000ms)
- Temporarily blocks waiting for a task reply.
- Returns `{:ok, reply}` | `{:exit, reason}` | `nil` (no reply yet).
- `nil` leaves the monitor active, so `yield/2` can be called multiple times.
- Can return `{:exit, reason}` when: task exited `:normal`; task isn't linked to caller
  (started via `Task.Supervisor.async_nolink/2`/`4`); or caller is trapping exits.
- Idiom for "result or shutdown": `Task.yield(task, timeout) || Task.shutdown(task)`.
- Idiom for "check but leave running": `Task.yield(task, timeout) || Task.ignore(task)`.

### yield_many/2
- Yields to multiple tasks in a time interval; returns `[{task, result}]` in input order.
- Each result is `{:ok, term}` | `{:exit, reason}` | `nil`.
- Options: `:limit` (max tasks to wait for), `:timeout` (default 5000), `:on_timeout`
  (`:nothing` default | `:ignore` | `:kill_task`).
- Common pattern: `yield_many` then `Task.shutdown(task, :brutal_kill)` for non-repliers.

### shutdown/2 (default 5000ms)
- Unlinks and shuts down the task, then checks for a reply.
- Returns `{:ok, reply}` | `{:exit, reason}` | `nil`.
- Second arg is a timeout or `:brutal_kill`. With a timeout, a `:shutdown` exit signal is
  sent; if the task does not exit within the timeout it is killed. `:brutal_kill` kills
  straight away.
- If the task terminates abnormally (killed by another process), this function exits
  with the same reason.
- Not required when terminating the caller, unless exiting `:normal` or the task is
  trapping exits. The caller can exit `:shutdown` to shut down all linked processes
  (including tasks) that are not trapping exits, without log messages.
- For tasks with no linked process (e.g. `Task.completed/1`), checks for a response/error
  without shutting a process down.
- Returns `{:exit, :noproc}` if the monitor was already demonitored/received and no
  response is queued.

### ignore/1 (since 1.13.0)
- Ignores an existing task: it continues running, but is unlinked and can no longer be
  yielded/awaited/shut down.
- Returns `{:ok, reply}` | `{:exit, reason}` | `nil`.
- IMPORTANT: avoid `Task.async/1,3` then immediately `ignore/1`. For fire-and-forget
  tasks use `Task.Supervisor.start_child/2` instead.

### start / start_link
- `start/1,3` — fire-and-forget, side-effects only, no interest in results/success.
  Returns `{:ok, pid}`. Node shutdown terminates even if the task is not completed;
  prefer `Task.Supervisor.start_child/2` to control shutdown via `:shutdown`.
- `start_link/1,3` — starts a statically supervised task under a supervision tree.
  These tasks are supervised and not directly linked to the caller, so they cannot be
  awaited. The Supervisor will NOT wait for the task to finish before starting the next
  child or returning. For synchronous initialization use Agent or GenServer.

### completed/1 (since 1.13.0)
- Starts a task that immediately completes with the given result.
- Unlike `async/1`, does NOT spawn a linked process. Can be awaited/yielded like any task.
- Useful for mixed asynchrony (some inputs handled synchronously, some asynchronously).

## async_stream
`async_stream/3` and `async_stream/5` (since 1.4.0) return a stream that runs `fun`
concurrently on each element of `enumerable`. Each element is processed by its own task.

- `async_stream/3` — anonymous function (one-arity). Tasks linked to caller (like `async/1`).
- `async_stream/5` — module/function/args; tasks linked to an **intermediate process**
  that is then linked to the caller. A failure in a task terminates the caller, and a
  failure in the caller terminates all tasks.
- Each task emits `{:ok, value}` on success, or `{:exit, reason}` if the caller is
  trapping exits. `:zip_input_on_exit` (since v1.14.0) yields `{:exit, {input, reason}}`.

Options (`async_stream_option()` since 1.17.0):
- `:max_concurrency` — max tasks at once. Defaults to `System.schedulers_online/0`.
- `:ordered` — whether results return in input order (default `true`). `false` avoids
  buffering; useful for side-effect-only streams.
- `:timeout` — max ms per task (default 5000) or `:infinity`.
- `:on_timeout` — `:exit` (default, caller exits) | `:kill_task` (kill timed-out task,
  emit `{:exit, :timeout}`).
- `:zip_input_on_exit` — (since v1.14.0) default `false`.

Caveat — "unbound async + take": `1..100 |> Task.async_stream(...) |> Enum.take(10)` on an
8-core machine processes ~16 items (8 concurrently, then 8 more kicked off, only 2 used).
Limit upfront with `Stream.take/2` before `async_stream`, or tune `:max_concurrency`.

For supervised streaming use `Task.Supervisor.async_stream/6`; for non-linked streaming
use `Task.Supervisor.async_stream_nolink/6`.

Note: there is no function literally named `async_stream_with` in this module — the
"with" variants are the `Task.Supervisor.async_stream*` functions.

## :temporary restart; link/demonitor model

### :temporary default restart
`use Task` defines a `child_spec/1` so the module can be a supervision child. Opposite
to GenServer, Agent and Supervisor, a Task has a default `:restart` of `:temporary`:
the task will NOT be restarted even if it crashes.
- `use Task, restart: :temporary` (default) — never restarted.
- `use Task, restart: :transient` — restarted for non-successful exits.
- `use Task, restart: :permanent` — always restarted.
Customizable `child_spec/1` options: `:id` (default current module), `:restart`,
`:shutdown` (immediate or give time to shut down).

### Link/demonitor model
- `async/1,3` spawns a process **linked to AND monitored by** the caller. The `:ref`
  field is the opaque monitor reference.
- Linking aborts the task if the parent dies and preserves the failure semantics of
  the equivalent sequential code.
- `shutdown/2` **unlinks** the task first, then shuts it down, then checks for a reply.
- `ignore/1` **unlinks** the task and leaves it running; no further yield/await/shutdown.
- `await/2` / `yield/2` assume the monitor is still active OR the `:DOWN` message is in
  the queue; if demonitored or already received, they wait the full timeout.
- Anti-patterns explicitly called out:
  - Setting `:trap_exit` to true (makes the process immune to exits from all processes;
    even when trapping, `await` still exits if the task terminated without sending its
    result).
  - Unlinking a task process started with `async/await` (may leave dangling tasks if the
    caller dies and the task is not supervised).

### Ancestor and Caller Tracking
- New processes are annotated with `$ancestors` in the process dictionary (supervision
  hierarchy). For supervised tasks the actual ancestor is the supervisor.
- `$callers` key tracks the relationship between your code and the task:
  `[your code] --calls--> [supervisor] --spawns--> [task]`, so `$ancestors` points to the
  supervisor and `$callers` points to your code.
- `Process.get(:"$callers")` returns `nil` or `[pid_n, ..., pid2, pid1]`.
- On task crash, the callers field is included in log metadata under `:callers`.

## Task ↔ BEAM spawn/monitor mapping (→ docs/beam/processes-and-messages.md, links-monitors-and-exits.md)

Task is a thin Elixir wrapper over BEAM process primitives. The mapping:

| Task construct | BEAM primitive |
|---|---|
| `Task.async/1,3` process | `:erlang.spawn_link/1` (linked) + `Process.monitor/1` |
| `Task.start/1,3` process | `:erlang.spawn/1` (NOT linked, fire-and-forget) |
| `Task.start_link/1,3` process | `:erlang.spawn_link/1` (linked, for supervision tree) |
| `Task.completed/1` | no process spawned; pure struct |
| task `:ref` field | monitor reference returned by `Process.monitor/1` |
| reply message `{ref, result}` | the standard monitor-reply message format |
| `:DOWN` message handling | `{:DOWN, ref, :process, pid, reason}` from `Process.monitor/1` |
| `Task.shutdown/2` | `Process.unlink/1` + exit signal `:shutdown` (+ `:brutal_kill` → `Process.exit(pid, :kill)`) |
| `Task.ignore/1` | `Process.unlink/1` + `Process.demonitor/1` (flush) leaving the task running |
| caller dies → task dies | bidirectional link propagation (exit signals) |
| `async_stream/5` intermediate process | a collector process that links the worker tasks and is itself linked to the caller |

Key BEAM semantics reflected in Task:
- A link is bidirectional: if either end dies, the other receives an exit signal (unless
  trapping exits). This is why `async` tasks die when the caller dies and vice-versa.
- A monitor is unidirectional: the monitoring process receives a `:DOWN` message when the
  monitored process exits. Task uses a monitor so the caller can detect task
  completion/crash via `:DOWN` even when the link is removed (e.g. `async_nolink`).
- `:trap_exit` converts link exit signals into `{:EXIT, from, reason}` messages, which is
  why Task docs warn against it (it immunizes the process to ALL linked-process exits, not
  just the task's).
- `:shutdown` / `:normal` exit reasons have special propagation semantics: exiting the
  caller with `:shutdown` shuts down linked non-trapping processes (including tasks)
  without log messages.

See:
- docs/beam/processes-and-messages.md — spawn/spawn_link, message passing, process dictionary
  (`$ancestors`, `$callers`).
- docs/beam/links-monitors-and-exits.md — link vs monitor semantics, exit signal
  propagation, `:trap_exit`, `:shutdown`/`:normal`/`:kill` reasons, `Process.demonitor`.

## Strict rules
1. If you start an `async`, you MUST consume it: `await/2`, or `yield/2` + `shutdown/2`,
   or `ignore/1`. Never abandon an `async` task.
2. `await/2` and `await_many/2` can be called exactly ONCE per task. For repeated checks
   use `yield/2` / `yield_many/2`.
3. Do NOT set `:trap_exit` to true to isolate task failures. Use
   `Task.Supervisor.async_nolink/2` instead.
4. Do NOT unlink a task started with `async/await`. Use supervised / `async_nolink` tasks.
5. Do NOT `Task.async/1,3` then immediately `Task.ignore/1`. For fire-and-forget use
   `Task.Supervisor.start_child/2`.
6. Inside OTP behaviours (GenServer etc.), do NOT `await` long-running tasks; match
   `{ref, result}` and `{:DOWN, ref, :process, pid, reason}` in `handle_info/2`, and prefer
   `Task.Supervisor.async_nolink/3`.
7. For distributed tasks, use `Task.Supervisor.async/5` (explicit MFA), not the anonymous
   `async/3` (anonymous functions require the same module version on all nodes).
8. Statically supervised tasks (`start_link`, `{Task, fn -> ... end}` under Supervisor)
   cannot be awaited and are not restarted by default (`:temporary`).
9. `Task.start/1,3` is fire-and-forget only; node shutdown kills it even if unfinished —
   prefer `Task.Supervisor.start_child/2` with a `:shutdown` value.
10. Beware "unbound async + take": `async_stream |> Enum.take(n)` may over-process; limit
    the input first or cap `:max_concurrency`.

## Verbatim quotes
- "Tasks are processes meant to execute one particular action throughout their lifetime,
  often with little or no communication with other processes."
- "Compared to plain processes, started with spawn/1, tasks include monitoring metadata
  and logging in case of errors."
- "When invoked, a new process will be created, linked and monitored by the caller."
- "Async tasks link the caller and the spawned process. This means that, if the caller
  crashes, the task will crash too and vice-versa. This is on purpose: if the process meant
  to receive the result no longer exists, there is no purpose in completing the
  computation."
- "an asynchronous task should be thought of as an extension of the caller process rather
  than a mechanism to isolate it from all errors."
- "Opposite to GenServer, Agent and Supervisor, a Task has a default :restart of
  :temporary. This means the task will not be restarted even if it crashes."
- "Setting :trap_exit to true - trapping exits should be used only in special
  circumstances as it would make your process immune to not only exits from the task but
  from any other processes. Moreover, even when trapping exits, calling await will still
  exit if the task has terminated without sending its result back."
- "Unlinking the task process started with async/await. If you unlink the processes and
  the task does not belong to any supervisor, you may leave dangling tasks in case the
  caller process dies."
- "Important: avoid using Task.async/1,3 and then immediately ignoring the task. If you
  want to start tasks you don't care about their results, use Task.Supervisor.start_child/2
  instead."
- "It is not recommended to await a long-running task inside an OTP behaviour such as
  GenServer. Instead, you should match on the message coming from a task inside your
  GenServer.handle_info/2 callback."
- "The caller can exit with reason :shutdown to shut down all of its linked processes,
  including tasks, that are not trapping exits without generating any log messages."

## Version notes
- Page built with ExDoc v0.40.3 for Elixir v1.20.2.
- `async_stream/3,5` — since 1.4.0.
- `child_spec/1` — since 1.5.0.
- `await_many/2` — since 1.11.0.
- `completed/1`, `ignore/1` — since 1.13.0.
- `:zip_input_on_exit` option — since v1.14.0.
- `async_stream_option()` type — since 1.17.0.
- `async_stream_with` is NOT a function in this module; the "with supervisor / no-link"
  variants live in `Task.Supervisor` (`async_stream/6`, `async_stream_nolink/6`).

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/elixir/Task.Supervisor.html — supervised/distributed/nolink tasks
- https://hexdocs.pm/elixir/Supervisor.html — child_spec, restart/shutdown semantics
- https://hexdocs.pm/elixir/GenServer.html — handle_info/2 task integration pattern
- https://hexdocs.pm/elixir/Process.html — monitor/1, demonitor, unlink, trap_exit
- https://hexdocs.pm/elixir/Agent.html — distributed-process limitations note

### Skipped
- In-page anchors (#async_stream/5-first-async-tasks-to-complete, #module-distributed-tasks,
  #module-dynamically-supervised-tasks, #module-statically-supervised-tasks,
  #module-tasks-are-processes)
- GitHub source links (github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/task.ex#...)
- ExDoc/llms.txt/ePub download links
- Task.md (markdown mirror of this same page)
