# Erlang interop (`gleam_erlang`)

## Purpose

`gleam_erlang` is the typed Gleam surface over Erlang/OTP primitives. It is the
lowest-level core package for Erlang-targeted Gleam code and the foundation on
which `gleam_otp` is built. It wraps BEAM process identity, message passing,
links, monitors, exit signals, process naming, timers, and OTP application
metadata in Gleam types.

- Package: `gleam_erlang` **v1.3.0**
- Install: `gleam add gleam_erlang@1`
- Runtime requirement: **Erlang/OTP 27.0 or higher**

## Sources used

- `/docs/gleam/.crawl/29-gleam-erlang-index.md` — package index and module list  
  https://hexdocs.pm/gleam_erlang/
- `/docs/gleam/.crawl/30-gleam-erlang-process.md` — `gleam/erlang/process` API  
  https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html
- `/docs/gleam/.crawl/31-gleam-erlang-application.md` — `gleam/erlang/application` API  
  https://hexdocs.pm/gleam_erlang/gleam/erlang/application.html

All API names reflect `gleam_erlang` v1.3.0.

## Related BEAM guidance

- `../beam/processes-and-messages.md` — mailboxes, per-pair FIFO ordering, and
  send/receive semantics that `gleam/erlang/process` wraps.
- `../beam/links-monitors-and-exits.md` — links, monitors, exit signals, and
  trapping; the runtime model behind `link`, `monitor`, `trap_exits`, and `kill`.
- `../beam/applications.md` — OTP applications; `gleam/erlang/application` is a
  thin typed projection of `StartType` and the `priv` directory.
- `../beam/distribution.md` — distributed Erlang; `gleam/erlang/node` is the
  typed surface over node identity.

## Core guidance

### Package overview

`gleam_erlang` is a thin, type-safe membrane over BEAM runtime primitives.
Modules: `gleam/erlang/process` (message-passing core), `gleam/erlang/application`
(OTP metadata), and the wrapper modules `atom`, `charlist`, `node`, `port`, and
`reference`.

### `gleam/erlang/process`

**Types:** `Pid`, `Subject(message)`, `Selector(payload)`, `Name(message)`,
`Monitor`, `Timer`, `ExitReason` (`Normal`, `Killed`,
`Abnormal(reason: dynamic.Dynamic)`), `ExitMessage` (`ExitMessage(pid, reason)`),
`Down` (`ProcessDown`, `PortDown`), `Cancelled` (`TimerNotFound`,
`Cancelled(time_remaining: Int)`).

A `Subject` is a typed receive endpoint owned by the process that created it
with `new_subject()`. Any process can `send` to it; only the owner can
`selector_receive` on it.

**Key functions:**

```gleam
new_selector() -> Selector(payload)
select(selector, for subject: Subject(payload)) -> Selector(payload)
select_map(selector, for subject: Subject(message), mapping transform: fn(message) -> payload) -> Selector(payload)
select_record(selector, tag tag: tag, fields arity: Int, mapping transform: fn(dynamic.Dynamic) -> payload) -> Selector(payload)
select_other(selector, mapping handler: fn(dynamic.Dynamic) -> payload) -> Selector(payload)
select_monitors(selector, mapping: fn(Down) -> payload) -> Selector(payload)
select_trapped_exits(selector, handler: fn(ExitMessage) -> payload) -> Selector(payload)
deselect(selector, for subject: Subject(message)) -> Selector(payload)
selector_receive(from selector: Selector(payload), within timeout: Int) -> Result(payload, Nil)
selector_receive_forever(from selector: Selector(payload)) -> payload
send(subject: Subject(message), message: message) -> Nil
send_after(subject: Subject(message), delay: Int, message: message) -> Timer
cancel_timer(timer: Timer) -> Cancelled
call(subject: Subject(message), waiting timeout: Int, sending make_request: fn(Subject(reply)) -> message) -> reply
call_forever(Subject(message), make_request: fn(Subject(reply)) -> message) -> reply
monitor(pid: Pid) -> Monitor
demonitor_process(monitor: Monitor) -> Nil
link(pid: Pid) -> Bool
unlink(pid: Pid) -> Nil
trap_exits(a: Bool) -> Nil
send_exit(to pid: Pid) -> Nil
send_abnormal_exit(pid: Pid, reason: anything) -> Nil
kill(pid: Pid) -> Nil
spawn(running: fn() -> anything) -> Pid
spawn_unlinked(a: fn() -> anything) -> Pid
self() -> Pid
sleep(a: Int) -> Nil
new_name(prefix: String) -> Name(message)
register(pid: Pid, name: Name(message)) -> Result(Nil, Nil)
named(name: Name(message)) -> Result(Pid, Nil)
named_subject(name: Name(message)) -> Subject(message)
unregister(name: Name(message)) -> Result(Nil, Nil)
```

Also available: `select_specific_monitor`, `deselect_specific_monitor`,
`map_selector`, `merge_selector`, `is_alive`, `sleep_forever`, `subject_name`,
`subject_owner`, and `flush_messages`.

`send` is fire-and-forget and panics when sent to a named subject with no
registered process. `call` and `call_forever` panic on timeout, callee exit
before reply, or unregistered named subject. v1.3.0 has no result-returning
variants. `monitor` delivers exactly one `Down`; if the target is already dead
the reason is `Abnormal(atom.to_dynamic(atom.create("noproc")))` and delivery is
not guaranteed by return time. Linked processes crash together unless the caller
traps exits; `trap_exits(True)` converts those signals into `ExitMessage` mailbox
messages. `kill` is untrappable. `flush_messages` discards the whole mailbox;
use only in tests. `spawn` creates a **linked** child via `proc_lib`; use
`spawn_unlinked` for crash isolation. `new_name` generates a new atom; atoms are
not GC'd, so generate names a bounded number of times at startup and never call
it in a loop or supervision tree.

### Mapping to BEAM primitives

| `gleam/erlang/process` | BEAM primitive |
|---|---|
| `spawn` / `spawn_unlinked` | `proc_lib:spawn_link` / `proc_lib:spawn` |
| `self()` | `self()` BIF |
| `Pid` | `pid()` |
| `Subject` | tagged tuple wrapping `{pid, ref/unique_tag}` |
| `send` | `erlang:send(Pid, Msg)` / `!` |
| `send_after` | `erlang:send_after(Delay, Pid, Msg)` |
| `cancel_timer` | `erlang:cancel_timer(TimerRef)` |
| `selector_receive` / `selector_receive_forever` | selective `receive` with timeout / `infinity` |
| `Selector` | compiled receive pattern set |
| `link` / `unlink` | `erlang:link/1` / `erlang:unlink/1` |
| `monitor` / `demonitor_process` | `erlang:monitor(process, Pid)` / `erlang:demonitor(Ref, [flush, info])` |
| `Down` (`ProcessDown`) | `{'DOWN', Ref, process, Pid, Reason}` |
| `trap_exits(True)` | `process_flag(trap_exit, true)` |
| `ExitMessage` | `{'EXIT', FromPid, Reason}` |
| `send_exit` / `send_abnormal_exit` | `erlang:exit(Pid, Reason)` |
| `kill` | `erlang:exit(Pid, kill)` — untrappable |
| `is_alive` | `erlang:is_process_alive/1` |
| `register` / `named` / `unregister` | `erlang:register/2` / `erlang:whereis/1` / `erlang:unregister/1` |
| `flush_messages` | `receive _ -> ... after 0 -> ok end` loop |
| `sleep` | `timer:sleep/1` |

### `gleam/erlang/application`

The v1.3.0 surface is intentionally tiny:

```gleam
pub type StartType {
  Normal
  Takeover(previous: node.Node)
  Failover(previous: node.Node)
}

pub fn priv_directory(name: String) -> Result(String, Nil)
```

`StartType` mirrors the argument Erlang/OTP passes to an application's
`Module:start/2` callback:

- `Normal` ↔ `normal`
- `Takeover(previous)` ↔ `{takeover, Node}`
- `Failover(previous)` ↔ `{failover, Node}`

`priv_directory(name)` ↔ `code:priv_dir/1`; it returns the path to an
application's `priv` directory. Each Gleam package is an Erlang application.

`start`, `stop`, `ensure_started`, `get_env`, and `set_env` are **not** wrapped
in this module. They are raw Erlang `:application` BIFs; call them via
FFI/externals if needed.

## Practical rules

- Only the owner of a `Subject` may call `selector_receive` on it.
- Use `Selector` when a process must wait on multiple subjects or message sources.
- Prefer `send` for one-way messages; use `call` only when the caller needs a
  reply and can tolerate a panic on timeout or callee exit.
- Use `monitor` to observe failure without crash propagation; use `link` when
  crashes should propagate (or trap exits to handle them as messages).
- Generate `Name` values once at startup; never call `new_name` in a loop or
  supervision tree.
- `spawn` is linked by default; use `spawn_unlinked` for crash isolation.
- Locate bundled files with `application.priv_directory("my_app")`; do not
  hardcode `priv/` paths.
- Do not use `flush_messages` outside of tests.

## Review checklist

- [ ] Code uses v1.3.0 API names.
- [ ] Only the owner receives or selects on a `Subject`.
- [ ] `call` timeout/callee-exit behavior is acceptable as a panic.
- [ ] `new_name` is called a bounded number of times at startup.
- [ ] `kill` is not treated as trappable.
- [ ] `select_record` is not used to match subject-sent messages.
- [ ] `gleam/erlang/application` is not expected to wrap lifecycle or env.
- [ ] `priv_directory` errors are handled.

## Implementation checklist

- [ ] Add `gleam_erlang@1` and confirm Erlang/OTP 27.0+.
- [ ] Implement process loops around `selector_receive` / `selector_receive_forever`.
- [ ] Register process names at startup, not dynamically.
- [ ] Use FFI/externals for `:application` operations not in this module.
- [ ] Use `priv_directory` for runtime assets.

## Validation hooks

- `gleam add gleam_erlang@1`
- `gleam build --target erlang`
- `gleam test`
- Verify the Erlang/OTP runtime is `27.0` or newer.

## Examples

### Subject, selector, and spawn

```gleam
import gleam/erlang/process

pub type Work {
  Work(n: Int, reply_to: process.Subject(Int))
}

pub fn run() {
  let work = process.new_subject()
  process.spawn(fn() {
    let s = process.new_selector()
      |> process.select(for: work, mapping: fn(m) { m })
    let assert Ok(Work(n, reply)) =
      process.selector_receive(from: s, within: 5000)
    process.send(subject: reply, message: n * 2)
  })
  let reply = process.new_subject()
  process.send(subject: work, message: Work(21, reply))
  let s = process.new_selector()
    |> process.select(for: reply, mapping: fn(x) { x })
  let assert Ok(result) = process.selector_receive(from: s, within: 5000)
  result
}
```

### Monitor and trapped exits

```gleam
import gleam/erlang/process

pub type Msg {
  DownMsg(process.Down)
  ExitMsg(process.ExitMessage)
}

pub fn watch(target: process.Pid) {
  let mon = process.monitor(pid: target)
  process.trap_exits(True)
  let s = process.new_selector()
    |> process.select_monitors(mapping: DownMsg)
    |> process.select_trapped_exits(mapping: ExitMsg)
  let msg = process.selector_receive_forever(from: s)
  process.demonitor_process(mon)
  msg
}
```

## Common mistakes

- **Sending to an unregistered named subject or a non-owner receiving on a
  `Subject`.** `send` panics when the name is unregistered; a non-owner simply
  never receives the message.
- **Calling `new_name` dynamically.** Each call creates an atom; atoms are never
  garbage-collected and can exhaust the VM atom table.
- **Expecting result-returning send/call variants.** v1.3.0 provides only `send`,
  `call`, and `call_forever`; failures panic.
- **Expecting `gleam/erlang/application` to wrap lifecycle or env.** Use Erlang
  FFI for `application:start`, `stop`, `ensure_started`, `get_env`, and
  `set_env`.
- **Treating `kill` as trappable.** `kill` always terminates the target.
- **Using `select_record` to catch subject-sent messages.** Subjects use a
  unique internal tag, so `select_record` will not match them.
- **Forgetting that `spawn` is linked by default.** Use `spawn_unlinked` when
  crash isolation is required.

## Strict vs contextual guidance

Strict rules:

- Only the `Subject` owner may receive or select.
- `new_name` must be called a bounded number of times at startup.
- `kill` is untrappable.
- `gleam/erlang/application` does not wrap application lifecycle or env.

Contextual decisions:

- Use a `Selector` when multiplexing multiple message sources.
- Use `call` only when the caller needs a reply and can accept a panic on failure.
- Use `named_subject` only for stable, long-lived identities that can be
  re-registered after failure.

## Policy decisions for individual repos

- Pin the major version with `gleam add gleam_erlang@1`.
- Document whether child processes are linked (`spawn`) or unlinked
  (`spawn_unlinked`) by default and establish `new_name` prefix conventions.
- Centralize `:application` FFI operations in one module.
- Document any processes that call `trap_exits(True)`.

## Related docs

- `externals-and-ffi.md`
- `otp-actors-and-supervision.md`
- `conventions-patterns-antipatterns.md`
- `deployment-and-runtime.md`

## Related skills

- `gleam-otp-interop`
