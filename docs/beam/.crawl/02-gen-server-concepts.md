# Crawl: gen_server_concepts.html

- seed_url: https://www.erlang.org/doc/system/gen_server_concepts.html
- canonical_url: https://www.erlang.org/doc/system/gen_server_concepts.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2; ExDoc v0.40.3)
- feeds_docs: gen-server.md, otp-behaviours.md

## Purpose
Conceptual introduction to the `gen_server` behaviour. Walks through a minimal
client-server callback module (`ch3`) reimplementing the plain-Erlang server
from the Overview (`design_principles.html#ch1`) using `gen_server`. Explains
starting, synchronous `call`, asynchronous `cast`, stopping (in a supervision
tree vs standalone), `handle_info`, and `code_change/3`.

The page explicitly recommends reading it alongside the STDLIB reference
`gen_server` module (`../apps/stdlib/gen_server.html`), which holds the full
callback signatures and return-tuple enumerations. This concepts page only
shows the subset of return tuples used by the `ch3` example.

## Key concepts
- Client-server model: central server + arbitrary clients; used for shared
  resource management. Server owns/manages the resource.
- `gen_server` is a behaviour: callback module implements `init/1`,
  `handle_call/3`, `handle_cast/2` (and optionally `handle_info/2`,
  `terminate/2`, `code_change/3`).
- Interface functions (`start_link/0`, `alloc/0`, `free/1`) live in the same
  module as the callbacks — recommended practice: one process's code in one
  module.
- `start_link` spawns + links a new process; synchronous (does not return until
  `init` has run and the server is ready).
- Requests become messages; the gen_server dispatches to the matching callback.
- `call` = synchronous, blocking, returns `Reply`.
- `cast` = asynchronous, returns `ok` immediately, no reply.
- Stopping: supervised servers are terminated by their supervisor per a
  shutdown strategy; standalone servers can stop themselves by returning
  `{stop, normal, State1}` from a callback.

## Strict rules / invariants
- `start_link/4` MUST be used when the gen_server is part of a supervision tree
  (started by a supervisor). `start/4` is for standalone gen_servers not in a
  supervision tree.
- `start_link`/`start` is synchronous: it does not return until the gen_server
  has been initialized and is ready to receive requests.
- The server name passed to `call`/`cast` must agree with the name used to
  start it.
- To clean up on supervised shutdown: the shutdown strategy must be a timeout
  value AND the gen_server must trap exit signals (`process_flag(trap_exit, true)`
  in `init`). Then `terminate(shutdown, State)` is called.
- `init` is expected to return `{ok, State}`.
- `handle_call/3` is expected to return `{reply, Reply, State1}`.
- `handle_cast/2` is expected to return `{noreply, State1}` (or `{stop, Reason, State1}`
  to stop).
- `handle_info/2` must be implemented to receive non-request messages (e.g.
  `'EXIT'` messages from linked processes when trapping exits).
- Returning `{stop, normal, State1}` causes the gen_server to call
  `terminate(normal, State1)` and then terminate gracefully.

## Examples
The page presents a single running example, `ch3`, a channel allocator. It is
the gen_server reimplementation of the plain-Erlang server from the Overview
(`design_principles.html#ch1`). The ch3/ch4/server3 progression itself lives in
the Overview page (`design_principles.html`), NOT on this page — this page only
shows `ch3`.

### ch3 (gen_server callback module) — what it teaches
- `-behaviour(gen_server).` declaration.
- Three exports: interface (`start_link/0`, `alloc/0`, `free/1`) and callbacks
  (`init/1`, `handle_call/3`, `handle_cast/2`).
- `start_link/0` calls `gen_server:start_link({local, ch3}, ch3, [], [])`.
- `alloc/0` -> `gen_server:call(ch3, alloc)`.
- `free/1` -> `gen_server:cast(ch3, {free, Ch})`.
- `init(_Args)` -> `{ok, channels()}`.
- `handle_call(alloc, _From, Chs)` -> `{reply, Ch, Chs2}`.
- `handle_cast({free, Ch}, Chs)` -> `{noreply, Chs2}`.

### Starting gen_server — argument walkthrough
`gen_server:start_link({local, ch3}, ch3, [], []) => {ok, Pid}`
1. `{local, ch3}` — name; locally registered as `ch3`. Omit name => not
   registered (use pid). `{global, Name}` => registered via
   `global:register_name/2`.
2. `ch3` — callback module name (where callbacks live).
3. `[]` — term passed as-is to `init`.
4. `[]` — options list (see STDLIB `gen_server`).

### Supervised shutdown with cleanup
```
init(Args) ->
    ...,
    process_flag(trap_exit, true),
    ...,
    {ok, State}.
...
terminate(shutdown, State) ->
    %% Code for cleaning up here
    ...
    ok.
```

### Standalone stop
```
stop() -> gen_server:cast(ch3, stop).
handle_cast(stop, State) -> {stop, normal, State};
handle_cast({free, Ch}, State) -> ...
terminate(normal, State) -> ok.
```

### handle_info (exit messages)
```
handle_info({'EXIT', Pid, Reason}, State) ->
    %% Code to handle exits here.
    ...
    {noreply, State1}.
```

### code_change/3
```
code_change(OldVsn, State, Extra) ->
    %% Code to convert state (and more) during code change.
    ...
    {ok, NewState}.
```

## Behaviour / callback details
Callbacks shown on this page (signatures as written here; full enumeration is
in STDLIB `gen_server`, not this page):

- `init(Args)` — returns `{ok, State}`.
- `handle_call(Request, From, State)` — expected to return
  `{reply, Reply, State1}`. (`Reply` sent back to client; `State1` new state.)
- `handle_cast(Request, State)` — expected to return `{noreply, State1}`.
  Returning `{stop, normal, State1}` stops the server (standalone stop pattern).
- `handle_info(Info, State)` — for non-request messages (e.g. `'EXIT'`);
  returns `{noreply, State1}`.
- `terminate(Reason, State)` — `terminate(shutdown, State)` for supervised
  shutdown (when trapping exits); `terminate(normal, State)` for standalone
  stop. Returns `ok`.
- `code_change(OldVsn, State, Extra)` — returns `{ok, NewState}`.

### Return-tuple shapes captured verbatim from the page
- `init`: `{ok, State}` (page: "`init` is expected to return `{ok, State}`")
- `handle_call`: `{reply,Reply,State1}` (page writes it without spaces:
  "expected to return a tuple `{reply,Reply,State1}`")
- `handle_cast`: `{noreply,State1}` ("expected to return a tuple
  `{noreply,State1}`")
- `handle_cast` (stop): `{stop,normal,State1}` ("returns a tuple
  `{stop,normal,State1}`")
- `handle_info`: `{noreply, State1}` (shown in code as `{noreply, State1}`)
- `code_change`: `{ok, NewState}` (shown in code as `{ok, NewState}`)

### call vs cast vs reply
- `gen_server:call(ServerName, Request)` — synchronous; the request is made
  into a message and sent; when received, `handle_call(Request, From, State)`
  is invoked; the client blocks until `Reply` is returned via
  `{reply, Reply, State1}`.
- `gen_server:cast(ServerName, Request)` — asynchronous; `cast` (and thus the
  wrapper function) returns `ok` immediately; on receipt `handle_cast(Request,
  State)` is invoked returning `{noreply, State1}` (or `{stop, ...}`).
- "reply" is the `Reply` value carried in `{reply, Reply, State1}` sent back to
  the caller of `call`.

### Timeouts / hibernate / continue / format_status
NOT covered on this concepts page. The page only mentions the four options
argument and defers to STDLIB `gen_server` for available options and the full
callback return-tuple enumeration (which includes timeout, hibernate,
`{continue, Continue}`, and `format_status`). See
`../apps/stdlib/gen_server.html`.

## Verbatim quotes
1. (Client-Server Principles)
   "The client-server model is characterized by a central server and an
   arbitrary number of clients. The client-server model is used for resource
   management operations, where several different clients want to share a
   common resource. The server is responsible for managing this resource."

2. (Example)
   "An example of a simple server written in plain Erlang is provided in
   Overview. The server can be reimplemented using gen_server, resulting in
   this callback module:"

3. (Starting a Gen_Server)
   "start_link/0 calls function gen_server:start_link/4. This function spawns
   and links to a new process, a gen_server."

4. (Starting a Gen_Server — registration)
   "If the name is omitted, the gen_server is not registered. Instead its pid
   must be used. The name can also be given as {global, Name}, in which case
   the gen_server is registered using global:register_name/2."

5. (Starting a Gen_Server — colocation)
   "The interface functions (start_link/0, alloc/0, and free/1) are located in
   the same module as the callback functions (init/1, handle_call/3, and
   handle_cast/2). It is usually good programming practice to have the code
   corresponding to one process contained in a single module."

6. (Starting a Gen_Server — init return)
   "If name registration succeeds, the new gen_server process calls the
   callback function ch3:init([]). init is expected to return {ok, State},
   where State is the internal state of the gen_server."

7. (Starting a Gen_Server — synchronous)
   "gen_server:start_link/4 is synchronous. It does not return until the
   gen_server has been initialized and is ready to receive requests."

8. (Starting a Gen_Server — supervision tree)
   "gen_server:start_link/4 must be used if the gen_server is part of a
   supervision tree, meaning that it was started by a supervisor. There is
   another function, gen_server:start/4, to start a standalone gen_server that
   is not part of a supervision tree."

9. (Synchronous Requests - Call)
   "When the request is received, the gen_server calls
   handle_call(Request, From, State), which is expected to return a tuple
   {reply,Reply,State1}. Reply is the reply that is to be sent back to the
   client, and State1 is a new value for the state of the gen_server."

10. (Asynchronous Requests - Cast)
    "cast, and thus free, then returns ok."

11. (Asynchronous Requests - Cast)
    "When the request is received, the gen_server calls handle_cast(Request,
    State), which is expected to return a tuple {noreply,State1}. State1 is a
    new value for the state of the gen_server."

12. (Stopping — In a Supervision Tree)
    "If the gen_server is part of a supervision tree, no stop function is
    needed. The gen_server is automatically terminated by its supervisor.
    Exactly how this is done is defined by a shutdown strategy set in the
    supervisor."

13. (Stopping — In a Supervision Tree)
    "If it is necessary to clean up before termination, the shutdown strategy
    must be a time-out value and the gen_server must be set to trap exit
    signals in function init. When ordered to shut down, the gen_server then
    calls the callback function terminate(shutdown, State):"

14. (Stopping — Standalone Gen_Servers)
    "The callback function handling the stop request returns a tuple
    {stop,normal,State1}, where normal specifies that it is a normal
    termination and State1 is a new value for the state of the gen_server. This
    causes the gen_server to call terminate(normal, State1) and then it
    terminates gracefully."

15. (Handling Other Messages)
    "If the gen_server is to be able to receive other messages than requests,
    the callback function handle_info(Info, State) must be implemented to
    handle them. Examples of other messages are exit messages, if the
    gen_server is linked to other processes than the supervisor and it is
    trapping exit signals."

## Version notes
- OTP 29.0.2 (major version 29). Page meta: `Erlang System Documentation
  v29.0.2`, `<meta name="major-vsn" content="29">`.
- Built with ExDoc v0.40.3.
- Source: github.com/erlang/otp blob at tag `OTP-29.0.2`,
  `system/doc/design_principles/gen_server_concepts.md`.
- Copyright © 1996-2026 Ericsson AB.
- No version-specific change notes on this page; it is a stable concepts page.

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/system/design_principles.html — Overview (prev page;
  contains the ch1 plain-Erlang server, ch4, and server3 progression referenced
  here; sup_princ shutdown anchor).
- https://www.erlang.org/doc/system/design_principles.html#ch1 — Overview / ch1
  plain-Erlang server that ch3 reimplements.
- https://www.erlang.org/doc/system/sup_princ.html#shutdown — Supervisor
  shutdown strategy (referenced for supervised termination).
- https://www.erlang.org/doc/system/statem.html — gen_statem Behaviour (next
  page; sibling behaviour).
- https://www.erlang.org/doc/apps/stdlib/gen_server.html — STDLIB gen_server
  module reference; holds full callback signatures, return-tuple enumeration
  (timeouts, hibernate, continue, format_status), and start options.
- https://www.erlang.org/doc/apps/stdlib/gen_server.html#start_link/4
- https://www.erlang.org/doc/apps/stdlib/gen_server.html#start/4
- https://www.erlang.org/doc/apps/stdlib/gen_server.html#call/2
- https://www.erlang.org/doc/apps/stdlib/gen_server.html#cast/2
- https://www.erlang.org/doc/apps/kernel/global.html#register_name/2 — global
  name registration.

### Skipped
- https://www.erlang.org/doc/system/gen_server_concepts.md (copy-markdown link,
  same content).
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/gen_server_concepts.md#L1
  (view-source mirror).
- https://www.erlang.org/doc/system/llms.txt
- https://www.erlang.org/doc/system/Erlang%20System%20Documentation.epub
- https://github.com/elixir-lang/ex_doc (ExDoc tooling).
- https://www.erlang.org (Erlang home).
- https://www.ericsson.com (Ericsson).
- Relative asset/script links (CSS/JS/sidebar items) — not documentation.
