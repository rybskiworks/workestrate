# OTP Behaviours

## Purpose
Explain the generic/specific behaviour split, the `-behaviour` and `-callback`
attributes, the four standard behaviours, and the special-process contract
(`proc_lib` + `sys`) that lets a hand-written process comply with OTP design
principles without a standard behaviour — and why standard behaviours are
preferred.

## Sources used
- Crawl file: `crawl/01-design-principles.md` — canonical URL: `https://www.erlang.org/doc/system/design_principles.html`
- Crawl file: `crawl/10-spec-proc.md` — canonical URL: `https://www.erlang.org/doc/system/spec_proc.html`
- Crawl file: `crawl/14-proc-lib.md` — canonical URL: `https://www.erlang.org/doc/apps/stdlib/proc_lib.html`
- Crawl file: `crawl/15-sys.md` — canonical URL: `https://www.erlang.org/doc/apps/stdlib/sys.html`
- Documented OTP version: OTP 29.0.2 (stdlib v8.0.1)

## Core guidance
**The generic/specific split.** "Behaviours are formalizations of these common
patterns. The idea is to divide the code for a process in a generic part (a
behaviour module) and a specific part (a callback module)." (design_principles)
"The behaviour module is part of Erlang/OTP. To implement a process such as a
supervisor, the user only needs to implement the callback module, which is to
export a pre-defined set of functions, the callback functions."

**The `-behaviour` attribute.** "The compiler understands the module attribute
`-behaviour(Behaviour)` and issues warnings about missing callback functions."
(design_principles)

**The four standard behaviours** (design_principles):
- `gen_server` — "for implementing the server of a client-server relation."
- `gen_statem` — "for implementing state machines."
- `gen_event` — "for implementing event handling functionality."
- `supervisor` — "for implementing a supervisor in a supervision tree."

**Special processes.** "The `sys` module has functions for simple debugging of
processes implemented using behaviours. It also has functions that, together
with functions in the `proc_lib` module, can be used to implement a special
process that complies to the OTP design principles without using a standard
behaviour." (spec_proc) "Both `sys` and `proc_lib` belong to the STDLIB
application."

A special process must: "Be started in a way that makes the process fit into a
supervision tree; Support the sys debug facilities; Take care of system
messages." System messages are "messages with a special meaning, used in the
supervision tree. Typical system messages are requests for trace output, and
requests to suspend or resume process execution (used during release handling).
Processes implemented using standard behaviours automatically understand these
messages."

**Why prefer standard behaviours.** Standard behaviours "automatically
understand system messages, provide debug facilities, crash reporting, code
change, and supervision-tree integration out of the box." Writing a special
process via `sys`+`proc_lib` is "only warranted when a standard behaviour does
not fit; otherwise the manual contract (system messages, debug threading,
parent-exit handling) is error-prone boilerplate." (spec_proc)

## Practical rules
1. Declare `-behaviour(Behaviour).` in every callback module so the compiler
   checks required callbacks.
2. For user-defined behaviours, prefer `-callback` over `behaviour_info/1`:
   "We recommend using the -callback attribute rather than the behaviour_info()
   function. The reason is that the extra type information can be used by tools
   to produce documentation or find discrepancies." (spec_proc)
3. "Each `-spec` contract in a callback module MUST be a subtype of the
   respective `-callback` contract." (spec_proc)
4. Use `-optional_callbacks` together with `-callback` (never with
   `behaviour_info/1`): "`-optional_callbacks` MUST be used together with
   `-callback`; it cannot be combined with `behaviour_info/1`." (spec_proc)
5. Start special processes via `proc_lib` (not plain `spawn`): "A function in
   the proc_lib module is to be used to start the process ... for example,
   `proc_lib:spawn_link/3,4` for asynchronous start and
   `proc_lib:start_link/3,4,5` for synchronous start." (spec_proc)
6. Do all initialization (including name registration) in `init/1`. (spec_proc)
7. Acknowledge startup: the child must call `proc_lib:init_ack/1,2` on success
   or `proc_lib:init_fail/2,3` (OTP 26+) on failure. "proc_lib:start_link/3 is
   synchronous and does not return until proc_lib:init_ack/1,2 or
   proc_lib:init_fail/2,3 has been called, or the process has exited."
8. Never interpret `{system, From, Request}` — delegate to
   `sys:handle_system_msg/6`: "The content and meaning of these messages are
   not to be interpreted by the process." (spec_proc)
9. Thread the debug structure: call `sys:handle_debug/4` for each system event
   and thread the returned `Deb` back into the loop.
10. If a special process traps exits, watch for `{'EXIT', Parent, Reason}` and
    terminate with the same reason: "A process in a supervision tree is
    expected to terminate with the same reason as its parent." (spec_proc)
11. Hibernate via `proc_lib:hibernate/3`, never the raw `hibernate/3` BIF:
    "Always use this function instead of the BIF for processes started using
    proc_lib functions." (proc_lib)

## Review checklist
- [ ] Is `-behaviour/1` declared and are all required callbacks exported?
- [ ] For user-defined behaviours, are callbacks declared with `-callback`?
- [ ] Is each `-spec` a subtype of its `-callback` contract?
- [ ] If a special process is used, is it started via `proc_lib`?
- [ ] Does the special process delegate `{system, From, Request}` to
  `sys:handle_system_msg/6`?
- [ ] Are the five `system_*` callbacks exported (`system_continue/3`,
  `system_terminate/4`, `system_code_change/4`, `system_get_state/1`,
  `system_replace_state/2`)?
- [ ] Is `sys:handle_debug/4` called per system event with `Deb` threaded back?
- [ ] Does a trapping special process terminate with the parent's reason?

## Implementation checklist
- [ ] Choose a standard behaviour first; only fall back to a special process if
  none fits.
- [ ] For a special process: `proc_lib:start_link(Mod, init, [Parent])`.
- [ ] In `init/1`: register name, build state, `Deb = sys:debug_options([])`,
  `proc_lib:init_ack(Parent, {ok, self()})`, then enter the loop.
- [ ] Loop: `receive` app messages (wrap in/out with `sys:handle_debug/4`) and a
  `{system, From, Request}` clause delegating to `sys:handle_system_msg/6`.
- [ ] Implement `system_continue/3`, `system_terminate/4`,
  `system_get_state/1`, `system_replace_state/2`, (optional)
  `system_code_change/4`, plus a `write_debug/3` format fun.
- [ ] For a user-defined behaviour: write the generic part like a special
  process, declare `-callback` (and `-optional_callbacks`), delegate specific
  tasks to the callback module.

## Runtime / debugging checklist
- [ ] `sys:get_state/1,2` and `sys:replace_state/2,3` work (debugging only).
- [ ] `sys:suspend/1` / `sys:resume/1` work (release handling).
- [ ] `sys:log/2`, `sys:trace/2`, `sys:statistics/2` produce output.
- [ ] `proc_lib:stop/3` terminates the process (requires correct system-message
  handling; "based on the terminate system message").
- [ ] Crash reports appear for abnormal termination (reason other than `normal`,
  `shutdown`, `{shutdown,Term}`).

## Validation hooks
- Compiler warnings from `-behaviour/1` for missing callbacks.
- `sys:get_status/1` returns the `{status, Pid, {module, Module}, [SItem]}`
  shape with `Misc` varying by process type.
- Crash reports via `proc_lib` + Kernel Logger.

## Examples
The canonical special-process example is `ch4` (spec_proc):

```erlang
-module(ch4).
-export([start_link/0, alloc/0, free/1, init/1]).
-export([system_continue/3, system_terminate/4, write_debug/3,
         system_get_state/1, system_replace_state/2]).

start_link() ->
    proc_lib:start_link(ch4, init, [self()]).

init(Parent) ->
    register(ch4, self()),
    Chs = channels(),
    Deb = sys:debug_options([]),
    proc_lib:init_ack(Parent, {ok, self()}),
    loop(Chs, Parent, Deb).

loop(Chs, Parent, Deb) ->
    receive
        {From, alloc} ->
            Deb2 = sys:handle_debug(Deb, fun ch4:write_debug/3, ch4,
                                    {in, alloc, From}),
            {Ch, Chs2} = alloc(Chs),
            From ! {ch4, Ch},
            Deb3 = sys:handle_debug(Deb2, fun ch4:write_debug/3, ch4,
                                    {out, {ch4, Ch}, From}),
            loop(Chs2, Parent, Deb3);
        {system, From, Request} ->
            sys:handle_system_msg(Request, From, Parent, ch4, Deb, Chs)
    end.

system_continue(Parent, Deb, Chs) -> loop(Chs, Parent, Deb).
system_terminate(Reason, _Parent, _Deb, _Chs) -> exit(Reason).
system_get_state(Chs) -> {ok, Chs}.
system_replace_state(StateFun, Chs) -> {ok, NChs, NChs} where NChs = StateFun(Chs).
```

User-defined behaviour skeleton (spec_proc):

```erlang
-module(simple_server).
-callback init(State :: term()) -> 'ok'.
-callback handle_req(Req :: term(), State :: term()) -> {'ok', Reply :: term()}.
-callback terminate() -> 'ok'.
-callback format_state(State :: term()) -> term().
-optional_callbacks([format_state/1]).
```

## Common mistakes
- Using plain `spawn` for a supervision-tree process — loses proc_lib metadata
  (parent, ancestors, initial call), crash reports, and broadened normal
  termination (`shutdown`/`{shutdown,Term}`).
- Interpreting `{system, From, Request}` content instead of delegating to
  `sys:handle_system_msg/6`.
- Forgetting to thread the updated `Deb` back into the loop.
- Using `init_ack` to signal a failed start — "the start function can return
  before the failing process has exited, which may block VM resources." Use
  `init_fail/2,3`. (proc_lib)
- Calling the raw `hibernate/3` BIF instead of `proc_lib:hibernate/3` — breaks
  exception handling/logging on wake-up.
- Using `behaviour_info/1` with `-optional_callbacks` (not allowed).

## Strict vs contextual guidance
- **Strict:** Always declare `-behaviour/1`. Always start special processes via
  `proc_lib`. Always delegate system messages to `sys:handle_system_msg/6`.
  Always use `proc_lib:hibernate/3`. Use `init_fail` (not `init_ack`) for
  start failures.
- **Contextual:** Writing a special process instead of using a standard
  behaviour is justified only when no standard behaviour fits. "Code written
  without using behaviours can be more efficient, but ... at the expense of
  generality." (design_principles)

## Policy decisions for individual repos
- Whether user-defined (non-standard) behaviours are permitted and their
  review gate.
- Whether special processes may be used at all, or whether all processes must
  use `gen_server`/`gen_statem`/`gen_event`/`supervisor`.
- Required `system_*` callback coverage (e.g. mandating `system_code_change/4`
  for release-handling support).

## Related docs
- [overview.md](overview.md)
- [gen-server.md](gen-server.md)
- [gen-statem.md](gen-statem.md)
- [gen-event.md](gen-event.md)
- [supervision.md](supervision.md)
- [proc-lib-and-sys.md](proc-lib-and-sys.md)
- [applications.md](applications.md)

## Related skills
- `beam-gen-server`
- `beam-gen-statem`
- `beam-supervision`
- `beam-processes`
