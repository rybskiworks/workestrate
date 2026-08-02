---
name: elixir-otp
description: |
  Operational guide for OTP and concurrent Elixir systems — GenServer, Supervisor,
  DynamicSupervisor, Registry, Application, Task, Agent. Load when building or debugging
  supervision trees, processes, or callback contracts. Does NOT cover pure language
  fundamentals (see elixir-coding) or deep BEAM internals (see docs/elixir/beam-otp-internals.md).
---

# OTP and Supervision in Elixir

## Triggers

Load this skill when:

- Writing, reviewing, or debugging `GenServer`/`Supervisor`/
  `DynamicSupervisor`/`Registry`/`Application`/`Task`/`Agent` code.
- Designing supervision trees.
- Configuring child specs, restart strategies, or registration.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/otp-supervision.md`
  - https://hexdocs.pm/elixir/GenServer.html
  - https://hexdocs.pm/elixir/Supervisor.html
  - https://hexdocs.pm/elixir/DynamicSupervisor.html
  - https://hexdocs.pm/elixir/Registry.html
  - https://hexdocs.pm/elixir/PartitionSupervisor.html
  - https://www.erlang.org/doc/system/gen_server_concepts.html
  - https://www.erlang.org/doc/system/design_principles.html
  - https://www.erlang.org/doc/system/sup_princ.html
- `docs/elixir/concurrency-processes.md`
  - https://hexdocs.pm/elixir/Process.html
  - https://hexdocs.pm/elixir/Task.html
  - https://hexdocs.pm/elixir/Task.Supervisor.html
  - https://hexdocs.pm/elixir/Agent.html
  - https://hexdocs.pm/elixir/Node.html
  - https://hexdocs.pm/elixir/Port.html
  - https://hexdocs.pm/elixir/processes.html
- `docs/elixir/beam-otp-internals.md`
  - https://www.erlang.org/doc/system/design_principles.html
  - https://www.erlang.org/doc/system/sup_princ.html
  - https://www.erlang.org/doc/system/gen_server_concepts.html
  - https://www.erlang.org/doc/system/errors.html
  - https://www.erlang.org/doc/system/spec_proc.html
  - https://www.erlang.org/doc/system/ref_man_processes.html
  - https://blog.stenmans.org/theBeamBook/
- `docs/beam/supervision.md` — supervisor flags, child specs, restart strategies (underlying runtime semantics).
- `docs/beam/gen-server.md` — `gen_server` callback contract and return-tuple shapes.
- `docs/beam/gen-statem.md` — `gen_statem` callback modes and timeout kinds.
- `docs/beam/proc-lib-and-sys.md` — `proc_lib`/`sys` contract for hand-written special processes.

## Key Rules

- GenServer is for modeling RUNTIME characteristics (mutable state, serialized
  access, concurrency isolation, failure containment) — NEVER for code
  organization. Pure functions belong as functions, not behind a process.
- Separate client API (runs in caller process) from server callbacks (run in
  GenServer process).
- Use `@impl true` on every callback. Return tuples MUST match exact shapes;
  bad returns terminate the process.
- `init/1` returns: `{:ok,state}`, `{:ok,state,timeout|:hibernate|{:continue,term}}`,
  `:ignore`, `{:stop,reason}`, `{:error,reason}`. `init/1` is synchronous and
  blocks `start_link/3`.
- `handle_call/3` returns: `{:reply,reply,new_state}`, `{:noreply,new_state}`,
  `{:stop,reason,reply,new_state}`, etc.
- `call` (synchronous, blocks caller, has timeout) vs `cast` (async,
  fire-and-forget, no guarantee). Prefer `call`; use `cast` only when no reply /
  reply-order is needed.
- `handle_continue/2` for post-init work without blocking `init/1`.
- Always implement `handle_info/2` for unexpected messages (defensive default)
  unless you intentionally crash on unknown messages.
- `terminate/2` is NOT guaranteed to run (only on graceful shutdown /
  supervised stop with shutdown timeout); do not rely on it for critical
  cleanup.
- `start_link/3` (linked, used by supervisors) vs `start/3` (unlinked,
  standalone).
- Registration: `name: __MODULE__` (atom), `{:global,name}`,
  `{:via,Registry,{MyApp.Registry,key}}`.
- Supervisor strategies: `:one_for_one` (independent), `:one_for_all` (restart
  all on any failure), `:rest_for_one` (restart failed + all started after it).
  Choose `:one_for_one` by default.
- Restart values: `:permanent` (always restarted, default for supervisors),
  `:transient` (restarted only on abnormal exit), `:temporary` (never
  restarted, default for Tasks).
- Shutdown: integer ms (graceful terminate timeout) or `:brutal_kill`;
  `:infinity` for processes that must clean up.
- `max_restarts`/`max_seconds` (default 3/5) — supervisor gives up if exceeded,
  terminates itself.
- Define `child_spec/1` in each worker module (or rely on `use GenServer`
  default); supervisors consume it.
- DynamicSupervisor: `:one_for_one` only, `:id` ignored, `start_child/2`,
  `terminate_child/2`, `which_children/1`, `count_children/1`. Use for dynamic
  process counts.
- Registry: `:unique` (one process per key) or `:duplicate` (many per key);
  partitioning for scalability; `:via` registration; `dispatch/4`.
- Task vs GenServer vs Agent decision:
  - Task: one-off async work, `async`/`await`, `Task.Supervisor` for supervised
    tasks.
  - Agent: simple shared state (get/update/get_and_update) — thin GenServer
    wrapper; prefer GenServer for anything beyond trivial state.
  - GenServer: long-lived stateful process with a request/response protocol.
- `spawn_link` only when you intend shared fate; prefer `spawn_monitor` or
  `Task` to observe failure.
- `trap_exit` only in supervisors or resource-owning GenServers that must clean
  up on linked-process exits.

## Quick Commands

```bash
mix run --no-halt                          # start app without halting
iex -S mix                                 # interactive shell with project loaded
:observer.start                            # (in iex) GUI process/supervisor inspector
:sys.get_state(pid)                        # inspect a GenServer's state
:sys.get_status(pid)                       # inspect status
```

## Anti-patterns

- GenServer for code organization / pure calculations ("calculator GenServer").
- Missing `@impl true` on callbacks.
- Missing `handle_info/2` default clause (mailbox fills with unexpected
  messages).
- Relying on `terminate/2` for critical cleanup (not guaranteed).
- `cast` when you need a reply or ordering guarantee.
- Heavy work in `init/1` blocking supervisor startup (use `handle_continue`).
- Raw `spawn` for supervised work (use `Task.Supervisor` or
  `DynamicSupervisor`).
- `:one_for_all` when children are independent (cascading restarts).

## Related Skills

- elixir-coding
- elixir-error-handling
- elixir-config
