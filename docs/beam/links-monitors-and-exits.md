# Links, Monitors, and Exits

## Purpose
Define link/monitor/alias mechanics, exit-signal propagation, `trap_exit`
reception rules, `DOWN` messages, and the distinction between `exit/1`,
`exit/2`, and `exit_signal/2,3`. Failure semantics for OTP supervision trees
depend on these primitives.

> **Source note:** the exit-reception rules and link/monitor propagation live
> in the Processes reference chapter (`ref_man_processes.html`, crawl 06), NOT
> in the Errors chapter (`errors.html`, crawl 07). Crawl 07 covers exception
> classes, `raise/3`, and the stacktrace only.

## Sources used
- Crawl file: `crawl/06-ref-man-processes.md` — canonical URL: `https://www.erlang.org/doc/system/ref_man_processes.html`
- Crawl file: `crawl/07-errors.md` — canonical URL: `https://www.erlang.org/doc/system/errors.html`
- Crawl file: `crawl/17-erlang-bifs.md` — canonical URL: `https://www.erlang.org/doc/apps/erts/erlang.html`
- Documented OTP version: OTP 29.0.2

## Core guidance

### Links
"Links are bidirectional and there can only be one link between two processes.
Repeated calls to `link()` have no effect. Either one of the involved processes
may create or remove a link." (06) Links can be created by `link/1`,
`spawn_link/4`, `spawn_opt/5`, and `spawn_request/5`; removed by `unlink/1`.
(06) "If one participant terminates, an exit signal is sent to the other with
the terminated participant's reason." (06)

`link/1` has a historical quirk: "For historical reasons, `link/1` has a strange
semi-synchronous behavior when it is 'cheap' to check if the linkee exists or
not, and the caller does not trap exits. If ... the linkee does not exist,
`link/1` will raise a `noproc` error exception." (17) This `noproc` *exception*
is not to be confused with an exit signal with reason `noproc`. (17)

`unlink/1`: "Once `unlink(Id)` has returned, it is guaranteed that the link
between the caller and the unlinkee has no effect on the caller in the future."
(17) But if trapping exits, an `{'EXIT', Id, _}` message may already be in the
queue — flush it explicitly. (17)

### Monitors
"`erlang:monitor(process, Pid2)` returns a reference `Ref`. If `Pid2` terminates
with reason `Reason`, the monitor delivers `{'DOWN', Ref, process, Pid2,
Reason}`." (06) "If `Pid2` does not exist, the message is sent immediately with
`Reason = noproc`." (06) "Monitors are unidirectional. Repeated calls to
`erlang:monitor(process, Pid)` create several independent monitors, and each
one sends a 'DOWN' message when `Pid` terminates." (06) A monitor is removed
with `demonitor/1`. (06)

`demonitor/1`: "Once `demonitor(MonitorRef)` has returned, it is guaranteed that
no `{'DOWN', MonitorRef, _, _, _}` message ... will be placed in the caller
message queue in the future. However, a `{'DOWN', ...}` message can have been
placed ... before the call." (17) Use `demonitor(MonitorRef, [flush])` to remove
a queued `DOWN`. (17) `demonitor/2` with `[info]` returns whether the monitor
was found and removed. (17)

`monitor/3` (OTP 24+) adds the `{alias, UnaliasOpt}` option: the returned
reference also becomes a process alias. `UnaliasOpt` is `explicit_unalias`,
`demonitor`, or `reply_demonitor`. (17)

### Process aliases
"A process alias is a reference created by `alias/0,1` or together with a
monitor using the `{alias, _}` option of `monitor/3`, `spawn_opt/5`, or
`spawn_request/5`." (06) "As long as the alias is active, messages sent to it
are delivered as if sent to the creator's pid. When deactivated ... new messages
sent to the alias are dropped before entering the message queue; messages
already in the queue remain." (06)

"It is *not* possible to: create an alias identifying a process other than the
caller; deactivate an alias unless it identifies the caller; look up an alias;
look up the process identified by an alias; check if an alias is active or not;
check if a reference is an alias. These are all intentional design decisions
relating to performance, scalability, and distribution transparency." (06)

`alias/1` options: `explicit_unalias` (default), `reply`, `priority` (OTP 28).
(17) `unalias/1` always deactivates regardless of creation options. (17)

### Exit signals and the link flag
"When a process or port terminates, it sends exit signals to all linked
processes/ports." (06) The signal carries: sender identifier, receiver
identifier, the `link` flag, and the exit reason (or `noproc`/`noconnection`
for link-setup failures). (06)

"`exit_signal(PidOrPort, Reason)` sends an exit signal with the `link` flag
**not** set." (06) "If `Reason` is `kill`, the receiver cannot trap it and
terminates unconditionally with reason `killed`." (06)

### Receiving exit signals (the three cases — from 06)
The action depends on trap-exit state, exit reason, sender, the `link` flag,
and whether the link is still active.

**1. Silently dropped if:**
- "the `link` flag of the exit signal is set and the corresponding link has been
  deactivated"; **or**
- "the exit reason of the exit signal is the atom `normal`, and the receiver is
  not trapping exits."

**2. Receiving process is terminated if:**
- "the `link` flag of the exit signal is not set, and the exit reason of the
  exit signal is the atom `kill`. The receiving process will terminate with the
  atom `killed` as exit reason."; **or**
- "the receiver is not trapping exits, and the exit reason is something other
  than the atom `normal`. Also, if the `link` flag ... is set, the link also
  needs to be active; otherwise, the exit signal will be dropped. The exit
  reason of the receiving process will equal the exit reason of the exit signal.
  Note that if the `link` flag is set, an exit reason of `kill` will *not* be
  converted to `killed`."

**3. Converted to a message `{'EXIT', SenderID, Reason}` and appended to the
message queue if:**
- "the receiver is trapping exits, the `link` flag ... is: not set, and the exit
  reason ... is not the atom `kill`; set, and corresponding link is active.
  Note that an exit reason of `kill` will *not* terminate the process in this
  case and it will not be converted to `killed`."

### `trap_exit`
Setting `process_flag(trap_exit, true)` converts exit signals into ordinary
`{'EXIT', From, Reason}` messages instead of terminating the process. (06, 17)
"`normal` exit signals are silently dropped unless the receiver is trapping
exits." (06)

### `kill` from a link vs explicit `exit_signal/2`
"When an `exit` signal with exit reason `kill` is received, the action taken is
different depending on whether the signal was sent due to a linked process
terminating, or the signal was explicitly sent using the `exit_signal/2` BIF.
When sent using the `exit_signal/2` BIF, the signal cannot be trapped, while it
can be trapped if the signal was sent due to a link." (06)

### Directly visible Erlang resources
"`exit` signals due to links, `down` signals, and `alive_request` replies are
not sent until all *directly visible Erlang resources* held by the terminating
process are released." (06) These are "all resources made available by the
language excluding resources held by heap data, dirty native code execution, and
the process identifier of the terminating process. Examples ... are registered
name and ETS tables." (06) "The PID cannot be reused until everything about the
process is released." (06)

### Exception classes (from 07 — in-process, not inter-process)
Erlang has three exception classes: "`error` — Run-time error, for example,
`1+a`, or the process called `error/1`"; "`exit` — The process called
`exit/1`"; "`throw` — The process called `throw/1`". (07) "All of the above
exceptions can also be generated by calling `erlang:raise/3`." (07) "`try` can
distinguish between the different classes, whereas the `catch` expression
cannot." (07)

> Crawl 07 explicitly defers inter-process exit-signal propagation, `trap_exit`,
> `DOWN`, and link/monitor semantics to `ref_man_processes.html`. The reasons
> `normal`, `kill`, `shutdown`, `{shutdown,Term}` belong to process-termination
> semantics, not the run-time-error reason atoms documented in 07.

## Practical rules

### `exit/1` vs `exit/2` vs `exit_signal/2,3`
- **`exit/1`** — "Raises an exception of class `exit` with exit reason `Reason`."
  (17) It stops the *current* process. "If a process calls `exit(kill)` and does
  not catch the exception, it will terminate with exit reason `kill` and also
  emit exit signals with exit reason `kill` (not `killed`) to all linked
  processes. Such exit signals with exit reason `kill` can be trapped by the
  linked processes." (17)
- **`exit/2`** — "Old form of `exit_signal/2`, with a quirk when sender and
  receiver are the same." (17) `exit(Dest, Reason)` sends an exit signal to
  `Dest`. The self-quirk: `exit(self(), normal)` when trapping exits delivers
  `{'EXIT', From, normal}` to the queue; when not trapping, the process exits
  with `normal`. (17) "Use `exit_signal/2` for new code." (17)
- **`exit_signal/2`** — sends an exit signal to the process/port identified by
  `Pid`/`Dest` (or an active alias). (17) `kill` is untrappable → `killed`;
  `normal` has no effect if not trapping, else becomes `{'EXIT', From, normal}`;
  any other reason: not trapping → exits with that reason, trapping →
  `{'EXIT', From, Reason}`. (17)
- **`exit_signal/3`** — adds an option list; currently `priority` (OTP 28).
  (17)

> **Version discrepancy (flagged):** crawl 17 annotates `exit_signal/2,3` as
> "(since OTP 29.0)" at the function signature, while its own Version notes
> state "exit_signal/2,3: introduced in OTP 24 (split of exit/2 into
> send-signal vs raise)." The OTP-24 introduction is consistent with the
> `exit/2` → `exit_signal/2` split. Treat OTP 24 as the introduction version;
> verify against the live `erlang.html` page if the exact version matters.

### `raise/3`
"`erlang:raise/3` can raise any of the three classes (`error`, `exit`,
`throw`)." (07, 17) "The stacktrace is used as the exception stacktrace for the
calling process; it is truncated to the current maximum stacktrace depth." (17)

### Stacktrace
"The stack backtrace (stacktrace) is a list that contains `{Module, Function,
Arity, ExtraInfo}` and/or `{Fun, Arity, ExtraInfo}` tuples." (07) "Developers
should rely on stacktrace entries only for debugging purposes." (07) "The only
exception to this rule is the class `error` with the reason `undef` which is
guaranteed to include the `Module`, `Function`, and `Arity` of the attempted
function as the first stacktrace entry." (07)

## Review checklist
- [ ] Is the link bidirectional/uniqueness invariant respected (no duplicate
  links)?
- [ ] Are monitors used (not links) where unidirectional observation is needed?
- [ ] Is `demonitor/2` with `[flush]` used to avoid stale `DOWN` messages?
- [ ] Is `trap_exit` set only where exit signals must be handled as messages?
- [ ] Are the three exit-reception cases correctly applied in design?
- [ ] Is `exit/1` used to stop the current process and `exit_signal/2` to signal
  another?
- [ ] Is the stacktrace treated as debug-only (except the `undef` guarantee)?

## Implementation checklist
- [ ] Use `spawn_link`/`spawn_monitor` for atomic spawn+link/monitor.
- [ ] Use `monitor/3` with `{alias, reply_demonitor}` for race-free client/server
  replies.
- [ ] Flush `{'EXIT', Id, _}` after `unlink/1` when trapping exits.
- [ ] Use `exit_signal/2` (not `exit/2`) for new code that sends exit signals.
- [ ] Use `init_fail/2,3` (not `init_ack`) to signal failed proc_lib starts.

## Runtime / debugging checklist
- [ ] `process_info(Pid, links)` / `process_info(Pid, monitors)` /
  `process_info(Pid, monitored_by)` / `process_info(Pid, trap_exit)`. (17)
- [ ] `is_process_alive/1` to wait for a `kill`ed process to actually die. (17)
- [ ] Inspect `{'EXIT', From, Reason}` messages when `trap_exit` is on.

## Validation hooks
- A `normal` exit signal never kills a linked process (dropped unless trapping).
- An explicit `exit_signal(Pid, kill)` is untrappable → `killed`.
- A link-originated `kill` signal *can* be trapped and is *not* converted to
  `killed`.
- `demonitor/1` guarantees no future `DOWN` from that monitor.

## Examples
```erlang
%% Monitor + alias for a race-free call/reply (OTP 24+)
client(ServerPid, Request) ->
    AliasMonReqId = monitor(process, ServerPid, [{alias, reply_demonitor}]),
    ServerPid ! {request, AliasMonReqId, Request},
    receive
        {reply, AliasMonReqId, Result} -> Result;   %% alias+monitor auto-removed
        {'DOWN', AliasMonReqId, process, ServerPid, Reason} -> {error, Reason}
    after 5000 ->
        unalias(AliasMonReqId),
        receive {reply, AliasMonReqId, Result} -> Result
        after 0 -> exit(timeout) end
    end.

%% Trapping exits: flush after unlink
unlink(Id),
receive {'EXIT', Id, _} -> ok after 0 -> ok end.

%% exit/1 stops the current process; exit_signal/2 signals another
exit(shutdown),                       %% class exit, current process
exit_signal(WorkerPid, shutdown),    %% signal to WorkerPid (OTP 24+)
```

## Common mistakes
- Confusing `exit/1` (stop self) with `exit/2`/`exit_signal/2` (signal another).
- Expecting a link-originated `kill` to be untrappable — only explicit
  `exit_signal/2` `kill` is untrappable.
- Forgetting to flush a queued `{'EXIT', Id, _}` after `unlink/1` when trapping.
- Forgetting to flush a queued `DOWN` after `demonitor/1` (use `[flush]`).
- Relying on stacktrace content/order beyond the `undef` first-entry guarantee.
- Using `exit/2`'s `exit(self(), normal)` self-quirk intentionally — prefer
  `exit_signal/2`.

## Strict vs contextual guidance
- **Strict:** Use `exit_signal/2` (not `exit/2`) for new code. Use
  `demonitor/2,[flush]` to avoid stale `DOWN`. Treat stacktrace as debug-only.
  Use `init_fail` (not `init_ack`) for failed starts.
- **Contextual:** `trap_exit` is appropriate for supervisors and bridge
  processes; prefer monitors for general observation. `link/2` with `priority`
  (OTP 28+) only for very specific priority-EXIT needs.

## Policy decisions for individual repos
- Whether `trap_exit` is permitted outside supervisors/bridges.
- Whether `exit/2` is banned in favor of `exit_signal/2` (lint rule).
- Default monitoring strategy (monitor vs link) for client/server calls.

## Related docs
- [processes-and-messages.md](processes-and-messages.md)
- [supervision.md](supervision.md)
- [proc-lib-and-sys.md](proc-lib-and-sys.md)
- [runtime-debugging.md](runtime-debugging.md)
- [common-mistakes.md](common-mistakes.md)

## Related skills
- `beam-processes`
- `beam-errors-failures`
- `beam-supervision`
