# Crawl: ref_man_processes.html

- seed_url: https://www.erlang.org/doc/system/ref_man_processes.html
- canonical_url: https://www.erlang.org/doc/system/ref_man_processes.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2
- feeds_docs: processes-and-messages.md, links-monitors-and-exits.md

## Purpose

This page is the Erlang/OTP Reference Manual chapter on *Processes*. It defines the process abstraction, creation, naming/aliasing, termination, the signal model, links, monitors, error handling, and the process dictionary. It is the authoritative source for the language-level semantics of Erlang processes.

## Key concepts

- **Erlang process**: a lightweight, isolated execution entity with its own memory, message queue, and process dictionary.
- **PID / registered name / process alias**: three ways to identify a receiver of a message.
- **Signals**: the universal asynchronous communication mechanism between processes and ports; includes messages, links, monitors, exit signals, and many internal request/reply signals.
- **Links**: bidirectional, one-per-pair relationships used for error propagation.
- **Monitors**: unidirectional subscriptions that deliver a `'DOWN'` message when the monitored entity terminates or is absent.
- **Trapping exits**: a process flag that converts exit signals into ordinary `'EXIT'` messages instead of terminating the process.
- **Directly visible Erlang resources**: resources held by a process that must be released before certain termination-related signals are sent.
- **Signal ordering guarantee**: per sender/destination pair, signals are not reordered.
- **Process dictionary**: per-process key/value store accessed via `put/2`, `get/1`, etc.

## Strict rules / invariants

1. A process always terminates with an *exit reason* (any term).
2. A process terminates *normally* only if its exit reason is the atom `normal`.
3. A registered name must be an atom and is automatically unregistered when the process terminates.
4. A process alias is a reference; only the creating process can create or deactivate it; aliases cannot be looked up or tested.
5. Links are bidirectional and at most one link can exist between two entities.
6. Monitors are unidirectional; repeated `monitor(process, Pid)` calls create independent monitors.
7. The only signal ordering guarantee: if entity `A` sends `S1` then `S2` to the same destination entity `B`, `S1` is guaranteed not to arrive after `S2` (but it may be lost).
8. `exit` signals due to links, `down` signals, and `alive_request` replies from an exiting process are delayed until all *directly visible Erlang resources* are released.
9. Directly visible Erlang resources exclude: heap data, dirty native code execution, and the PID of the terminating process.
10. `exit_signal(PidOrPort, kill)` (link flag not set) cannot be trapped and unconditionally terminates the receiver with reason `killed`.
11. An exit signal from a link with reason `kill` *can* be trapped and is *not* converted to `killed`.
12. If a receiver is not trapping exits and receives a non-`normal` exit signal, it terminates with the same reason (subject to link-active and kill rules above).
13. `normal` exit signals are silently dropped unless the receiver is trapping exits.
14. Signal reception is asynchronous and not tied to executing a `receive` expression.
15. A `receive` expression scans the message queue from the start and selects the first matching message.

## Examples

### Creating a process

```erlang
spawn(Module, Name, Args) -> pid()
Module = Name = atom()
Args = [Arg1,...,ArgN]
ArgI = term()
```

`spawn()` creates a new process and returns the pid. The new process starts executing in `Module:Name(Arg1,...,ArgN)`.

### Name registration BIFs

| BIF | Description |
| --- | --- |
| `register(Name, Pid)` | Associates the atom `Name` with process `Pid`. |
| `registered/0` | Returns a list of registered names. |
| `whereis(Name)` | Returns the pid registered under `Name`, or `undefined`. |

### Monitor DOWN message

```erlang
{'DOWN', Ref, process, Pid2, Reason}
```

Sent to the monitoring process when the monitored process `Pid2` terminates with reason `Reason`.

## Behaviour / callback details

### Process creation

A process is created via `spawn/3` (or `spawn/1,2,3,4`, `spawn_link/1,2,3,4`, `spawn_monitor/1,2,3,4`, `spawn_opt/2,3,4,5`, `spawn_request/1,2,3,4,5`). The new process begins execution in `Module:Name(Arg1,...,ArgN)`. With `spawn_link`, `spawn_opt`, and `spawn_request`, the spawn and link operations are performed atomically.

> Note: this page references `spawn_opt/5` but does not enumerate its option list (e.g., `message_queue_data`, `min_heap_size`, etc.); see the `erlang:spawn_opt/5` ERTS documentation for the full option set.

### Registration

Processes can be registered under an atom using `register/2`. The name is automatically unregistered on process termination. `whereis/1` resolves a name to a pid, and `registered/0` lists all registered names.

### Process aliases

A process alias is a reference created by `alias/0,1` or together with a monitor using the `{alias, _}` option of `monitor/3`, `spawn_opt/5`, or `spawn_request/5`. As long as the alias is active, messages sent to it are delivered as if sent to the creator's pid. When deactivated (by `unalias/1` or automatic events), new messages sent to the alias are dropped before entering the message queue; messages already in the queue remain.

A *priority alias* (created with the `priority` option to `alias/1`) enables priority message reception when used with `erlang:send/3` and the `priority` option. Priority message handling can also be enabled for monitor-triggered messages via `monitor/3` with `priority`, and for link exit signals via `link/2` with `priority`.

It is intentionally impossible to:

- create an alias identifying a process other than the caller.
- deactivate an alias unless it identifies the caller.
- look up an alias or the process it identifies.
- check whether an alias is active or whether a reference is an alias.

### Process termination

A process terminates with an exit reason. It terminates normally when the reason is `normal` (including running out of code). Runtime errors produce `{Reason, Stack}`. A process can terminate itself with `exit/1`, `error/1`, or `error/2`. A process can also be terminated by a non-`normal` exit signal (see Error Handling).

### Signals

All inter-process and process/port communication is asynchronous signaling. Common signals include: `message`, `link`, `unlink`, `exit`, `monitor`, `demonitor`, `down`, `change`, `group_leader`, `spawn_request`/`spawn_reply`, `alive_request`/`alive_reply`, various `*_request`/`*_reply` pairs, and port signals.

Signal reception is automatic and not tied to `receive`. Actions on reception:

- `message`: dropped if sent via an inactive alias; otherwise appended to the message queue.
- `link`/`unlink`: update process-local link state.
- `exit`: set exiting, drop, or convert to message depending on trap-exit state, reason, sender, and link flag.
- `monitor`/`demonitor`: update process-local monitor state.
- `down`/`change`: converted to message if the monitor is still active; otherwise dropped.
- `group_leader`: change the process's group leader.
- `spawn_reply`: converted to message or dropped depending on configuration.
- `alive_request` and info/gc/code requests: schedule execution; replies are sent when done.

### Message queue semantics

Without priority messages, every message is appended to the end of the message queue, preserving the order in which the corresponding signals were received. Because signal ordering is preserved per sender, messages from the same sender retain send order.

With priority messages enabled, accepted priority messages are inserted after the last accepted priority message, while ordinary messages remain at the end. Priority messages do **not** violate signal ordering; they only change how received signals are placed into the message queue. A `receive` still scans from the start and cannot distinguish priority from ordinary messages.

### Directly visible Erlang resources

`exit` signals due to links, `down` signals, and `alive_request` replies are not sent until all *directly visible Erlang resources* held by the terminating process are released. These are all resources made available by the language **except** resources held by heap data, dirty native code execution, and the terminating process's PID. Examples: registered names and ETS tables.

Excluded resources details:
- The PID cannot be reused until everything about the process is released.
- A process in dirty native code enters exiting state but the runtime cannot force the NIF to stop; `enif_send()` is disabled, and a well-behaved dirty NIF should check `enif_is_current_process_alive/0`.
- Heap data (including off-heap binaries and NIF resource objects on the heap) cannot be removed until all required signals have been sent.

### Signal ordering

The only guarantee: for multiple signals from the same sender to the same destination, order is preserved. There is no guarantee on delivery time, signals to a terminated receiver do not arrive (but may trigger other signals), and signals can be lost over distribution.

Runtime services (clock, name, timer, spawn) consist of multiple independent executors, so order between multiple signals from one service to one process is **not** preserved; this does not violate the per-sender ordering guarantee.

### Links

Two processes, or a process and a local port, can be linked. Links are bidirectional and unique per pair. `link/1`, `spawn_link/4`, `spawn_opt/5`, and `spawn_request/5` can create links. If one participant terminates, an exit signal is sent to the other with the terminated participant's reason. A link is removed with `unlink/1`.

### Error handling / exit signals

When a process or port terminates, it sends exit signals to all linked processes/ports. The signal carries:
- sender identifier,
- receiver identifier,
- `link` flag set,
- exit reason (or `noproc` / `noconnection` for certain link-setup failures).

`exit_signal(PidOrPort, Reason)` sends an exit signal with the `link` flag **not** set. If `Reason` is `kill`, the receiver cannot trap it and terminates unconditionally with reason `killed`.

#### Receiving exit signals

The action depends on trap-exit state, exit reason, sender, the `link` flag, and whether the link is still active.

- **Silently dropped if:**
  - the `link` flag is set and the corresponding link has been deactivated; **or**
  - the exit reason is `normal` and the receiver is not trapping exits.
- **Receiving process is terminated if:**
  - the `link` flag is **not** set and the exit reason is `kill` — the process terminates with reason `killed`; **or**
  - the receiver is not trapping exits and the exit reason is something other than `normal` (if the `link` flag is set, the link must also be active; otherwise the signal is dropped). The exit reason equals the signal's reason. If the `link` flag is set, an exit reason of `kill` is **not** converted to `killed`.
- **Converted to a message `{'EXIT', SenderID, Reason}` and appended to the message queue if:**
  - the receiver is trapping exits, the `link` flag is **not** set, and the reason is **not** `kill`; **or**
  - the receiver is trapping exits, the `link` flag is **set**, and the corresponding link is active. Again, `kill` in this case does **not** terminate the process and is **not** converted to `killed`.

### Monitors

`erlang:monitor(process, Pid2)` returns a reference `Ref`. If `Pid2` terminates with reason `Reason`, the monitor delivers:

```erlang
{'DOWN', Ref, process, Pid2, Reason}
```

If `Pid2` does not exist, the message is sent immediately with `Reason = noproc`. Monitors are unidirectional; repeated calls create independent monitors. A monitor is removed with `demonitor/1`. Monitors can target registered names, including on remote nodes.

### Process dictionary

Each process has a private process dictionary accessed via:

- `put(Key, Value)`
- `get(Key)`
- `get()`
- `get_keys(Value)`
- `erase(Key)`
- `erase()`

## Verbatim quotes

1. **Signals — introductory invariant**
   > "All communication between Erlang processes and Erlang ports is done by sending and receiving asynchronous signals."

2. **Signals — reception is automatic**
   > "Signals are received asynchronously and automatically. There is nothing a process must do to handle the reception of signals, or can do to prevent it. In particular, signal reception is *not* tied to the execution of a `receive` expression, but can happen anywhere in the execution flow of a process."

3. **Directly visible Erlang resources — definition**
   > "With *directly visible Erlang resources* we here mean all resources made available by the language excluding resources held by heap data, dirty native code execution, and the process identifier of the terminating process. Examples of *directly visible Erlang resources* are registered name and ETS tables."

4. **Signal ordering guarantee**
   > "The only signal ordering guarantee given is the following: if an entity sends multiple signals to the same destination entity, the order is preserved; that is, if `A` sends a signal `S1` to `B`, and later sends signal `S2` to `B`, `S1` is guaranteed not to arrive after `S2`. Note that `S1` may or may not have been lost."

5. **Signal ordering — runtime services caveat**
   > "Since each service consists of multiple independently executing entities, the order between multiple signals sent from one service to one process is *not* preserved. Note that this does *not* violate the signal ordering guarantee of the language."

6. **Trapping exits — kill from link vs explicit**
   > "When an `exit` signal with exit reason `kill` is received, the action taken is different depending on whether the signal was sent due to a linked process terminating, or the signal was explicitly sent using the `exit_signal/2` BIF. When sent using the `exit_signal/2` BIF, the signal cannot be trapped, while it can be trapped if the signal was sent due to a link."

7. **Links — bidirectional and unique**
   > "Links are bidirectional and there can only be one link between two processes. Repeated calls to `link()` have no effect. Either one of the involved processes may create or remove a link."

8. **Sending exit signals — link flag**
   > "The `link` flag - This flag will be set indicating that the exit signal was sent due to a link."

9. **Sending exit signals via `exit_signal/2` — kill semantics**
   > "If `Reason` is the atom `kill`, the receiver cannot trap the exit signal and will unconditionally terminate when it receives the signal."

10. **Receiving exit signals — silent drop rules**
    > "The exit signal is silently dropped if: the `link` flag of the exit signal is set and the corresponding link has been deactivated; the exit reason of the exit signal is the atom `normal`, and the receiver is not trapping exits."

11. **Receiving exit signals — termination rule for explicit kill**
    > "The receiving process is terminated if: the `link` flag of the exit signal is not set, and the exit reason of the exit signal is the atom `kill`. The receiving process will terminate with the atom `killed` as exit reason."

12. **Receiving exit signals — termination rule for non-normal reasons**
    > "The receiving process is terminated if: the receiver is not trapping exits, and the exit reason is something other than the atom `normal`. Also, if the `link` flag of the exit signal is set, the link also needs to be active; otherwise, the exit signal will be dropped. The exit reason of the receiving process will equal the exit reason of the exit signal. Note that if the `link` flag is set, an exit reason of `kill` will *not* be converted to `killed`."

13. **Receiving exit signals — conversion to message**
    > "The exit signal is converted to a message signal and added to the end of the message queue of the receiver, if the receiver is trapping exits, the `link` flag of the exit signal is: not set, and the exit reason of the signal is not the atom `kill`; set, and the corresponding link is active. Note that an exit reason of `kill` will *not* terminate the process in this case and it will not be converted to `killed`. The converted message will be of the form `{'EXIT', SenderID, Reason}` where `Reason` equals the exit reason of the exit signal and `SenderID` is the identifier of the process or port that sent the exit signal."

14. **Monitors — unidirectional and independent**
    > "Monitors are unidirectional. Repeated calls to `erlang:monitor(process, Pid)` create several independent monitors, and each one sends a 'DOWN' message when `Pid` terminates."

15. **Process aliases — intentional restrictions**
    > "It is *not* possible to: create an alias identifying a process other than the caller; deactivate an alias unless it identifies the caller; look up an alias; look up the process identified by an alias; check if an alias is active or not; check if a reference is an alias. These are all intentional design decisions relating to performance, scalability, and distribution transparency."

16. **Process termination — always an exit reason**
    > "When a process terminates, it always terminates with an *exit reason*. The reason can be any term."

17. **Priority messages — warning**
    > "Priority messages are intended to solve very specific problems where it previously was very hard to solve such problems efficiently using ordinary signaling. You *very seldom* need to resort to usage of priority messages. Receiving processes have *not* been optimized for handling large amounts of priority messages."

18. **Synchronous error checking**
    > "Some functionality that sends signals has synchronous error checking when sending locally on a node and fails if the receiver is not present at the time when the signal is sent: The send operator (`!`), `erlang:send/2,3`, BIFs and `erlang:send_nosuspend/2,3` BIFs when the receiver is identified by a name that is expected to be registered locally; `erlang:link/1`; `erlang:group_leader/2`."

19. **Blocking signaling over distribution**
    > "When sending a signal over a distribution channel, the sending process may be suspended even though the signal is supposed to be sent asynchronously. This is due to the built-in flow control over the channel that has been present more or less forever. When the size of the output buffer for the channel reaches the *distribution buffer busy limit*, processes sending on the channel will be suspended until the size of the buffer shrinks below the limit."

20. **Process dictionary BIFs**
    > "Each process has its own process dictionary, accessed by calling the following BIFs: `put(Key, Value)`, `get(Key)`, `get()`, `get_keys(Value)`, `erase(Key)`, `erase()`."

## Version notes

- Documentation corresponds to **Erlang/OTP 29.0.2** (per page `<meta name="project">` and sidebar version banner).
- Priority message reception is introduced as of **OTP 28.0**.

## Discovered links

### Relevant (crawl later)

- https://www.erlang.org/doc/index.html
- https://www.erlang.org/doc/system/data_types.html#pid
- https://www.erlang.org/doc/system/data_types.html#reference
- https://www.erlang.org/doc/system/expressions.html#receive
- https://www.erlang.org/doc/system/expressions.html#send
- https://www.erlang.org/doc/system/errors.html#exit_reasons
- https://www.erlang.org/doc/system/features.html
- https://www.erlang.org/doc/system/design_principles.html
- https://www.erlang.org/doc/system/distributed.html
- https://www.erlang.org/doc/apps/stdlib/ets.html
- https://www.erlang.org/doc/apps/kernel/erpc.html#call/5
- https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#link_protocol
- https://www.erlang.org/doc/apps/erts/erl_nif.html#enif_send
- https://www.erlang.org/doc/apps/erts/erl_nif.html#enif_is_current_process_alive
- https://www.erlang.org/doc/apps/erts/erl_nif.html#resource_objects
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn/3
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn/4
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn_link/4
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn_monitor/4
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn_opt/5
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn_request/5
- https://www.erlang.org/doc/apps/erts/erlang.html#register/2
- https://www.erlang.org/doc/apps/erts/erlang.html#registered/0
- https://www.erlang.org/doc/apps/erts/erlang.html#whereis/1
- https://www.erlang.org/doc/apps/erts/erlang.html#alias/0
- https://www.erlang.org/doc/apps/erts/erlang.html#alias/1
- https://www.erlang.org/doc/apps/erts/erlang.html#unalias/1
- https://www.erlang.org/doc/apps/erts/erlang.html#monitor/2
- https://www.erlang.org/doc/apps/erts/erlang.html#monitor/3
- https://www.erlang.org/doc/apps/erts/erlang.html#demonitor/1
- https://www.erlang.org/doc/apps/erts/erlang.html#link/1
- https://www.erlang.org/doc/apps/erts/erlang.html#link/2
- https://www.erlang.org/doc/apps/erts/erlang.html#unlink/1
- https://www.erlang.org/doc/apps/erts/erlang.html#exit/1
- https://www.erlang.org/doc/apps/erts/erlang.html#exit/3
- https://www.erlang.org/doc/apps/erts/erlang.html#exit_signal/2
- https://www.erlang.org/doc/apps/erts/erlang.html#process_flag_trap_exit
- https://www.erlang.org/doc/apps/erts/erlang.html#process_flag_async_dist
- https://www.erlang.org/doc/apps/erts/erlang.html#process_info/2
- https://www.erlang.org/doc/apps/erts/erlang.html#is_process_alive/1
- https://www.erlang.org/doc/apps/erts/erlang.html#send/2
- https://www.erlang.org/doc/apps/erts/erlang.html#send/3
- https://www.erlang.org/doc/apps/erts/erlang.html#send_nosuspend/2
- https://www.erlang.org/doc/apps/erts/erlang.html#send_after/3
- https://www.erlang.org/doc/apps/erts/erlang.html#start_timer/3
- https://www.erlang.org/doc/apps/erts/erlang.html#cancel_timer/1
- https://www.erlang.org/doc/apps/erts/erlang.html#group_leader/2
- https://www.erlang.org/doc/apps/erts/erlang.html#garbage_collect/1
- https://www.erlang.org/doc/apps/erts/erlang.html#check_process_code/2
- https://www.erlang.org/doc/apps/erts/erlang.html#open_port/2
- https://www.erlang.org/doc/apps/erts/erlang.html#port_command/2
- https://www.erlang.org/doc/apps/erts/erlang.html#port_connect/2
- https://www.erlang.org/doc/apps/erts/erlang.html#port_close/1
- https://www.erlang.org/doc/apps/erts/erlang.html#port_control/3
- https://www.erlang.org/doc/apps/erts/erlang.html#port_call/3
- https://www.erlang.org/doc/apps/erts/erlang.html#port_info/1
- https://www.erlang.org/doc/apps/erts/erlang.html#system_info_dist_buf_busy_limit
- https://www.erlang.org/doc/apps/erts/erlang.html#time_offset/0
- https://www.erlang.org/doc/apps/erts/erlang.html#unregister/1
- https://www.erlang.org/doc/apps/erts/erlang.html#error/1
- https://www.erlang.org/doc/apps/erts/erlang.html#error/2
- https://www.erlang.org/doc/apps/erts/erlang.html#put/2
- https://www.erlang.org/doc/apps/erts/erlang.html#get/1
- https://www.erlang.org/doc/apps/erts/erlang.html#get/0
- https://www.erlang.org/doc/apps/erts/erlang.html#get_keys/1
- https://www.erlang.org/doc/apps/erts/erlang.html#erase/1
- https://www.erlang.org/doc/apps/erts/erlang.html#erase/0

### Skipped

- https://www.erlang.org/doc/system/ref_man_processes.html (current page / self-reference)
- https://www.erlang.org/doc/system/ref_man_processes.html#processes
- https://www.erlang.org/doc/system/ref_man_processes.html#process-creation
- https://www.erlang.org/doc/system/ref_man_processes.html#registered-processes
- https://www.erlang.org/doc/system/ref_man_processes.html#process-aliases
- https://www.erlang.org/doc/system/ref_man_processes.html#process-termination
- https://www.erlang.org/doc/system/ref_man_processes.html#signals
- https://www.erlang.org/doc/system/ref_man_processes.html#sending-signals
- https://www.erlang.org/doc/system/ref_man_processes.html#receiving-signals
- https://www.erlang.org/doc/system/ref_man_processes.html#directly-visible-erlang-resources
- https://www.erlang.org/doc/system/ref_man_processes.html#delivery-of-signals
- https://www.erlang.org/doc/system/ref_man_processes.html#irregularities
- https://www.erlang.org/doc/system/ref_man_processes.html#links
- https://www.erlang.org/doc/system/ref_man_processes.html#error-handling
- https://www.erlang.org/doc/system/ref_man_processes.html#sending-exit-signals
- https://www.erlang.org/doc/system/ref_man_processes.html#receiving-exit-signals
- https://www.erlang.org/doc/system/ref_man_processes.html#monitors
- https://www.erlang.org/doc/system/ref_man_processes.html#process-dictionary
- https://www.erlang.org/doc/system/ref_man_processes.html#runtime-service
- https://www.erlang.org/doc/system/ref_man_processes.html#signal-delivery
- https://www.erlang.org/doc/system/ref_man_processes.html#visible-resources
- https://www.erlang.org/doc/system/ref_man_processes.html#enable-prio-msg-recv
- https://www.erlang.org/doc/system/ref_man_processes.html#errors
- https://www.erlang.org/doc/system/ref_man_processes.html#sending_exit_signals
- https://www.erlang.org/doc/system/ref_man_processes.html#receiving_exit_signals
- https://www.erlang.org/doc/system/ref_man_processes.html#link_exit_signal_reason
- https://www.erlang.org/doc/system/ref_man_processes.md (markdown copy source link)
- https://www.erlang.org/doc/system/llms.txt
- https://www.erlang.org/doc/system/Erlang System Documentation.epub
- https://www.erlang.org/doc/system/assets/logo.png
- https://www.erlang.org/doc/system/assets/prio-msg-recv.png
- https://www.erlang.org/doc/system/dist/html-erlang-KCHZLXSC.css
- https://www.erlang.org/doc/system/dist/html-Y2MUTVIN.js
- https://www.erlang.org/doc/system/dist/sidebar_items-D241B7A5.js
- https://www.erlang.org/doc/system/docs_config.js
- https://www.erlang.org/assets/css/algolia-typeahead.css
- https://www.erlang.org/assets/js/algolia-typeahead.bundle.js
- https://cdn.jsdelivr.net/npm/mermaid@11.14.0/dist/mermaid.min.js
- https://erlang.org
- https://github.com/elixir-lang/ex_doc
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/reference_manual/ref_man_processes.md#L1
- https://www.ericsson.com
