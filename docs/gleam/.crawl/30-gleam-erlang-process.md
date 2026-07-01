# Crawl: gleam/erlang/process.html
- seed_url: https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html
- canonical_url: https://gleam-erlang.hexdocs.pm/gleam/erlang/process.html
- family: Gleam core package module
- fetch: 200
- gleam_erlang_version: v1.3.0
- feeds_docs: erlang-interop.md, otp-actors-and-supervision.md

## Purpose
The `gleam.erlang.process` module is the typed, safe wrapper over BEAM
inter-process communication primitives. It exposes typed message-passing
(`Subject`/`send`/`receive`/`call`), selective receive (`Selector`), process
spawning, links, monitors, exit signals, name registration, and timers. This is
the foundational layer beneath `gleam/otp`'s `Actor` and `Supervisor`. The
module has no top-level docstring; documentation lives on individual items.

## Types (Subject, Pid, Selector, Name, ExitReason)

- **`Pid`** (opaque, L12): A reference to an Erlang process. Lowest-level
  building block of IPC in Erlang/Gleam OTP.
- **`Subject(message)`** (opaque, L78): A typed channel/topic. Owned by the
  process that created it via `new_subject`. Any process can `send` a message
  of the correct type to the owner; only the owner can `receive`/`select` on
  it. Comparable to "channel" types in other languages or pub-sub "topics".
- **`Selector(payload)`** (opaque, L290): Enables a process to wait for
  messages from multiple `Subject`s simultaneously, returning whichever arrives
  first. Built with `new_selector` + `select`/`select_map`/`select_record`/
  `select_other`/`select_monitors`/`select_trapped_exits`, consumed by
  `selector_receive`/`selector_receive_forever`.
- **`Name(message)`** (opaque, L115): A registered identity a process can adopt
  via `register`, so messages can be sent to it via `named_subject` without
  passing Pids/subjects around. Globally unique (≤1 name per process). Enables
  takeover: a new process can adopt a failed process's name. Generated via
  `new_name(prefix)` which appends a unique suffix to avoid collisions.
- **`Monitor`** (opaque, L511): A handle returned by `monitor`, used to
  demonitor and to select specific monitor messages.
- **`Timer`** (opaque, L739): A handle returned by `send_after`, cancellable via
  `cancel_timer`.
- **`ExitReason`** (L349): `pub type ExitReason { Normal; Killed;
  Abnormal(reason: dynamic.Dynamic) }`.
- **`ExitMessage`** (L345): `pub type ExitMessage { ExitMessage(pid: Pid,
  reason: ExitReason) }` — the trapped-exit message shape delivered when
  `trap_exits(True)` is active and a linked process exits.
- **`Down`** (L515): `pub type Down { ProcessDown(monitor: Monitor, pid: Pid,
  reason: ExitReason); PortDown(monitor: Monitor, port: port.Port,
  reason: ExitReason) }` — message received when a monitored process/port exits.
- **`Cancelled`** (L772): `pub type Cancelled { TimerNotFound;
  Cancelled(time_remaining: Int) }` — return of `cancel_timer`.

## Selector API (new_selector, selecting, select, record)

- **`new_selector() -> Selector(payload)`** (L296): Create an empty selector.
- **`select(selector, for subject: Subject(payload)) -> Selector(payload)`**
  (L388): Add a `Subject` of the same payload type.
- **`select_map(selector, for subject: Subject(message), mapping transform:
  fn(message) -> payload) -> Selector(payload)`** (L406): Add a `Subject` of a
  differing message type with a transform. Use identity fn if no transform
  needed. Required to mix `Subject`s of different types in one selector.
- **`select_record(selector, tag tag: tag, fields arity: Int, mapping transform:
  fn(dynamic.Dynamic) -> payload) -> Selector(payload)`** (L442): Handler for
  raw tuple messages with a tag in first position + N fields. For interop with
  other BEAM languages not using `Subject`. Will NOT match subject-sent messages
  even if same tag (subjects use a unique per-subject tag).
- **`select_other(selector, mapping handler: fn(dynamic.Dynamic) -> payload)
  -> Selector(payload)`** (L462): Catch-all handler for messages no other
  selector handler matches. Useful for handling non-subject/non-record messages
  from other BEAM languages.
- **`select_monitors(selector, mapping: fn(Down) -> payload) ->
  Selector(payload)`** (L562): Select for any monitor's `Down` message.
  Preferred over per-monitor handlers when selecting multiple monitors.
- **`select_specific_monitor(selector, monitor: Monitor, mapping: fn(Down) ->
  payload) -> Selector(payload)`** (L548): Select for one specific monitor.
  Each added handler has a select performance cost.
- **`select_trapped_exits(selector, handler: fn(ExitMessage) -> a) ->
  Selector(a)`** (L359): Handler for trapped exit messages. Requires
  `trap_exits(True)` to have been called for messages to arrive.
- **`deselect(selector, for subject: Subject(message)) -> Selector(payload)`**
  (L421): Remove a `Subject` from a selector.
- **`deselect_specific_monitor(selector, monitor: Monitor) ->
  Selector(payload)`** (L596): Remove a monitor handler.
- **`map_selector(a: Selector(a), b: fn(a) -> b) -> Selector(b)`** (L334):
  Apply a transform to all messages a selector yields.
- **`merge_selector(a: Selector(a), b: Selector(a)) -> Selector(a)`** (L343):
  Merge two selectors; on subject overlap, second selector's handler wins.
- **`selector_receive(from selector, within timeout: Int) ->
  Result(payload, Nil)`** (L316): Receive first matching message or `Error(Nil)`
  on timeout. Only the subject owner can receive.
- **`selector_receive_forever(from selector) -> payload`** (L325): Wait forever.

## send / send_after / try_send

- **`send(subject: Subject(message), message: message) -> Nil`** (L209): Place a
  typed message in the owner's mailbox; returns immediately (does not wait for
  receiver). Panics when sending to a named subject with no registered process.
  Ordering: two messages from P1 to P2 arrive in send order.
- **`send_after(subject: Subject(msg), delay: Int, message: msg) -> Timer`**
  (L757): Schedule a message after N ms; process is free meanwhile. Cancel via
  `cancel_timer`.
- **`cancel_timer(timer: Timer) -> Cancelled`** (L787): Cancel a scheduled
  send; returns `TimerNotFound` or `Cancelled(time_remaining)`.
- Note: there is no `try_send` in v1.3.0 — `send` is fire-and-forget and only
  panics on unregistered named subjects. (The crawl brief mentioned `try_send`;
  it is not present in this version.)

## call / try_call

- **`call(subject: Subject(message), waiting timeout: Int, sending make_request:
  fn(Subject(reply)) -> message) -> reply`** (L693): Send a request carrying a
  freshly-created reply `Subject` and wait `timeout` ms for the reply. Panics
  if: callee exited before replying; no reply within timeout; named subject has
  no registered process. The `make_request` callback receives the reply subject
  to embed in the request message.
- **`call_forever(subject, make_request: fn(Subject(reply)) -> message) ->
  reply`** (L710): Same as `call` but waits forever (no timeout panic).
- Note: there is no `try_call` in v1.3.0 — failure modes panic rather than
  return `Result`. (The crawl brief mentioned `try_call`; not present.)

## monitor / link / trap_exit / Exited model

- **`monitor(pid: Pid) -> Monitor`** (L535): Start monitoring. The `Down`
  message is sent exactly once: when the target exits (if alive at call time),
  or immediately with reason `Abnormal(atom.to_dynamic(atom.create("noproc")))`
  if already dead (delivery NOT guaranteed by return time). Receive via
  `select_monitors`/`select_specific_monitor`.
- **`demonitor_process(monitor monitor: Monitor) -> Nil`** (L583): Remove
  monitor; if `Down` already sent it is purged from the mailbox.
- **`link(pid pid: Pid) -> Bool`** (L727): Link caller to target. Linked
  processes crash together. Returns `True` on success, `False` if target not
  alive.
- **`unlink(pid: Pid) -> Nil`** (L734): Remove a link.
- **`trap_exits(a: Bool) -> Nil`** (L855): Toggle exit-signal trapping. When
  NOT trapping (default), a linked process's crash propagates as an exit signal
  that crashes the caller. When trapping, the exit becomes an `ExitMessage`
  delivered to the mailbox, handleable via `select_trapped_exits`.
- **`send_exit(to pid: Pid) -> Nil`** (L827): Send a normal exit signal telling
  a process to shut down.
- **`send_abnormal_exit(pid: Pid, reason: anything) -> Nil`** (L839): Send an
  abnormal exit signal (failure-style shutdown).
- **`kill(pid: Pid) -> Nil`** (L811): Send an untrappable kill exit signal
  (cannot be caught even with `trap_exits`).
- **`is_alive(a: Pid) -> Bool`** (L502): Check liveness.
- **`flush_messages() -> Nil`** (L379): Discard entire mailbox. Warning: may
  crash processes waiting on a response from the current process. Useful in
  tests.

## spawn / start

- **`spawn(running: fn() -> anything) -> Pid`** (L36): Spawn a LINKED process
  (child linked to creator). Started via Erlang `proc_lib`, so it benefits from
  proc_lib's behaviour (proper exit reasons, supervisor integration). For
  unlinked, use `spawn_unlinked`.
- **`spawn_unlinked(a: fn() -> anything) -> Pid`** (L53): Spawn an UNLINKED
  process (also via `proc_lib`). Use when you don't want crash propagation.
- **`self() -> Pid`** (L17): Current process pid.
- **`sleep(a: Int) -> Nil`** (L486) / **`sleep_forever() -> Nil`** (L493):
  Suspend the process.
- Name registration: **`new_name(prefix: String) -> Name(message)`** (L136),
  **`register(pid, name) -> Result(Nil, Nil)`** (L866), **`named(name) ->
  Result(Pid, Nil)`** (L882), **`named_subject(name) -> Subject(message)`**
  (L143), **`subject_name(subject) -> Result(Name(message), Nil)`** (L149),
  **`subject_owner(subject) -> Result(Pid, Nil)`** (L168),
  **`unregister(name) -> Result(Nil, Nil)`** (L877).
- **Warning on `new_name`**: each call generates a new atom. Atoms are not
  garbage-collected; generating too many (e.g. in a loop or inside a
  supervision tree) can exhaust the atom table and crash the VM. Create all
  names once at program start.

## Mapping to BEAM (→ docs/beam/processes-and-messages.md, links-monitors-and-exits.md)

This module is a typed façade over the BEAM primitives documented in
`docs/beam/processes-and-messages.md` and `docs/beam/links-monitors-and-exits.md`:

| gleam.erlang.process | BEAM primitive |
|---|---|
| `spawn` / `spawn_unlinked` | `proc_lib:spawn_link` / `proc_lib:spawn` |
| `self()` | `self()` BIF |
| `Pid` | `pid()` |
| `Subject` | a tagged tuple wrapping `{pid, ref/unique_tag}`; `send` does a
  typed `Pid ! Msg`; `receive` does a selective `receive` on the unique tag |
| `send` | `erlang:send(Pid, Msg)` / `!` |
| `send_after` | `erlang:send_after(Delay, Pid, Msg)` → returns `TimerRef` |
| `cancel_timer` | `erlang:cancel_timer(TimerRef)` |
| `receive`/`selector_receive` | selective `receive` with a timeout (`after
  Timeout -> ...`); `receive_forever` uses `infinity` |
| `Selector` | a compiled receive pattern set; `select_record` matches tuple
  tag+arity; `select_other` is a catch-all clause |
| `link` / `unlink` | `erlang:link/1` / `erlang:unlink/1` |
| `monitor` / `demonitor_process` | `erlang:monitor(process, Pid)` /
  `erlang:demonitor(Ref, [flush, info])` |
| `Down` (`ProcessDown`) | `{'DOWN', Ref, process, Pid, Reason}` |
| `trap_exits(True)` | `process_flag(trap_exit, true)` |
| `ExitMessage` | `{'EXIT', FromPid, Reason}` delivered to mailbox when trapping |
| `send_exit` / `send_abnormal_exit` | `erlang:exit(Pid, Reason)` |
| `kill` | `erlang:exit(Pid, kill)` — untrappable |
| `is_alive` | `erlang:is_process_alive(Pid)` |
| `register`/`named`/`unregister` | `erlang:register/2` /
  `erlang:whereis/1` / `erlang:unregister/1` |
| `flush_messages` | `receive _ -> ... after 0 -> ok end` loop |
| `sleep` | `timer:sleep/1` / `receive after Delay -> ok end` |

Key semantics mirrored from BEAM:
- **Exit signal propagation** (links): without `trap_exits`, a linked crash
  propagates and crashes the receiver (see links-monitors-and-exits.md). With
  `trap_exits(True)`, the signal becomes a mailbox message (`ExitMessage`).
- **`kill` is untrappable**: even a trapping process dies on `exit(Pid, kill)`.
- **Monitor `Down` is exactly-once** and async; `noproc` reason if target
  already dead.
- **Message ordering**: per-pair FIFO (P1→P2 messages arrive in send order),
  matching BEAM guarantees (processes-and-messages.md).
- **`proc_lib` spawn** gives proper exit reasons and supervisor-friendly
  behaviour, which is why `gleam/otp`'s `Actor` builds on this.

## Strict rules

- Only the **owner** of a `Subject` may `receive`/`select` on it; non-owners
  will never receive (no error, just no message).
- `send` to a **named subject** with no registered process **panics**.
- `call` **panics** on timeout, callee exit, or unregistered named subject — no
  `Result` return. Use `call_forever` only when you can guarantee a reply.
- `new_name` must be called a **bounded number of times** (at startup), never
  in loops or inside supervision trees — atoms are never GC'd.
- `flush_messages` can crash callers waiting on a reply — tests only.
- `kill` cannot be trapped; `send_exit`/`send_abnormal_exit` CAN be trapped.
- `select_record` will not match subject-sent messages even with identical tag
  (subjects use a unique internal tag).
- `spawn` (not `spawn_unlinked`) is the default — children are linked, so a
  child crash propagates to the parent unless trapped.

## Verbatim quotes

- Subject: "A `Subject` is a value that processes can use to send and receive
  messages to and from each other in a well typed way. Each subject is 'owned'
  by the process that created it."
- Name: "A name is an identity that a process can adopt, after which they will
  receive messages sent to that name... A new process can adopt the name of one
  that previously failed, allowing it to transparently take-over and handle
  messages that are sent to that name."
- new_name: "Never call this function dynamically such as within a loop or
  within a process within a supervision tree. Each time this function is called
  a new atom will be generated. Generating too many atoms will result [in VM
  crash]."
- monitor: "The message is always sent exactly once... If the target process is
  not alive, the message will be sent with reason
  `Abnormal(atom.to_dynamic(atom.create("noproc")))`. In this case the message
  is NOT guaranteed to be delivered by the time this function call returns."
- trap_exits: "When not trapping exits if a linked process crashes the exit
  signal propagates to the process which will also crash... When trapping exits
  ... if a linked process crashes an exit message is sent to the process
  instead."
- spawn: "The child process is linked to the creator process. When a process
  terminates an exit signal is sent to all other processes that are linked to
  it... This function starts processes via the Erlang `proc_lib` module."
- send ordering: "If process P1 sends two messages to process P2 it is
  guaranteed that process P1 will receive the messages in the order they were
  sent."
- flush_messages: "This function may cause other processes to crash if they
  sent a message to the current process and are waiting for a response, so use
  with caution."

## Version notes

- Crawled version: gleam_erlang **v1.3.0**.
- The crawl brief referenced `try_send`, `try_call`, `selecting`, `record`,
  `select_forever`, `monitor_process`, `start`, and `Exited`. **None of these
  exact names exist in v1.3.0.** The actual equivalents are: `send` (no
  try_send), `call`/`call_forever` (no try_call), `select`/`select_map`
  (instead of `selecting`), `select_record` (instead of `record`),
  `selector_receive_forever` (instead of `select_forever`), `monitor` (instead
  of `monitor_process`), `spawn`/`spawn_unlinked` (instead of `start`), and
  `ExitMessage`/`Down` (instead of `Exited`). The trap-exit message model uses
  `ExitMessage` + `select_trapped_exits`, not a type literally named `Exited`.

## Discovered links

### Relevant (crawl later)
- ../../gleam/erlang/application.html — gleam/erlang/application (OTP app env)
- ../../gleam/erlang/atom.html — gleam/erlang/atom (atom creation, used by
  monitor's noproc reason and new_name)
- ../../gleam/erlang/port.html — gleam/erlang/port (PortDown references
  port.Port)
- ../../gleam/erlang/node.html — gleam/erlang/node
- ../../gleam/erlang/charlist.html — gleam/erlang/charlist
- ../../gleam/erlang/reference.html — gleam/erlang/reference

### Skipped
- https://github.com/gleam-lang/erlang/blob/v1.3.0/src/gleam/erlang/process.gleam
  (source, per-item line anchors already captured)
- https://gleam-erlang.hexdocs.pm/gleam/erlang/process.html (self)
- ../../gleam/erlang/process.html (self)
