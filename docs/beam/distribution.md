# Distribution

## Purpose

Distributed Erlang is a set of Erlang runtime systems ("nodes") communicating
over TCP/IP sockets. This doc covers node naming, the magic-cookie authentication
model, EPMD, `net_kernel` (start/stop/monitor_nodes/net_ticktime), distributed
message passing, `global` name registration and resolver policies, `erpc` vs
`rpc`, hidden nodes, and TLS distribution. It also covers the distribution
carrier/protocol model (how the distribution transport is pluggable) and the
blocking-signaling caveat from the ERTS `alt_dist` chapter.

## Sources used

- Crawl `26-distributed.md` — system `distributed.html` — https://www.erlang.org/doc/system/distributed.html
- Crawl `27-global.md` — kernel `global.html` — https://www.erlang.org/doc/apps/kernel/global.html
- Crawl `28-net-kernel.md` — kernel `net_kernel.html` — https://www.erlang.org/doc/apps/kernel/net_kernel.html
- Crawl `30-erpc.md` — kernel `erpc.html` — https://www.erlang.org/doc/apps/kernel/erpc.html
- Crawl `39-alt-dist.md` — erts `alt_dist.html` — https://www.erlang.org/doc/apps/erts/alt_dist.html

> **Coverage gap (resolved):** The "blocking signaling over distribution" caveat
> (referenced by `erlang:send/2,3`, `exit/2`, `erpc`, etc.) is NOT in
> `distributed.html` (crawl 26) itself — it lives in the ERTS `alt_dist`
> carrier-protocol chapter. Crawl 39 (`alt_dist.html`) has now been crawled and
> integrated, filling this gap. The caveat now appears below in "## Core
> guidance" under "### The blocking-signaling-over-distribution caveat (crawl 39,
> verbatim)".

## Core guidance

### Nodes, names, EPMD

"A distributed Erlang system consists of a number of Erlang runtime systems
communicating with each other. Each such runtime system is called a node." A
node is given a name via `-name` (long names) or `-sname` (short names); the
name is the atom `name@host`. "A node with a long node name cannot communicate
with a node with a short node name." `node/0` returns the current node name
(`nonode@nohost` when non-distributed). Name can also be set at runtime via
`net_kernel:start/1`.

EPMD: "The Erlang Port Mapper Daemon epmd is automatically started at every host
where an Erlang node is started. It is responsible for mapping the symbolic node
names to machine addresses."

### Cookie security model

"Security" here means protection against accidental misuse, NOT cryptographic
security. "Furthermore, the communication between nodes is by default in clear
text." All nodes use a magic cookie (an Erlang atom). "The cookies themselves are
never transferred; instead, they are compared using hashed challenges, although
not in a cryptographically secure manner." The default random cookie is "not very
unpredictable." At startup the `auth` server searches for `.erlang.cookie` in the
home directory, then in `filename:basedir(user_config, "erlang")`; if none
exists it creates one (UNIX mode 0400). `erlang:set_cookie(Node, DiffCookie)`
sets a per-node cookie; symmetric cross-default-cookie configuration is broken.
`erlang:get_cookie/0` / `erlang:get_cookie(Node)` retrieve cookies.

### net_kernel

"The net kernel is a system process, registered as net_kernel, which must be
operational for distributed Erlang to work." Key functions:
- `start(Name, Options) -> {ok, pid()} | {error, Reason}` (OTP 24.3+; `start/1`
  list form is DEPRECATED). `Options` map keys: `name_domain => shortnames |
  longnames` (default `longnames`), `net_ticktime => pos_integer()`,
  `net_tickintensity => 4..1000`, `dist_listen => boolean()`, `hidden =>
  boolean()`. `Name = undefined` requests a dynamic node name (implies
  `dist_listen => false` and `hidden => true`).
- `stop() -> ok | {error, Reason}` — only works if started via `start/2`;
  otherwise `{error, not_allowed}`.
- `connect_node(Node) -> boolean() | ignored`.
- `allow(Nodes) -> ok | error | ignored` — before the first call, any node with
  the correct cookie can connect; after, only listed nodes. Subsequent calls ADD
  to the list; nodes cannot be removed. "Disallowing an already connected node
  will not cause it to be disconnected."
- `monitor_nodes(Flag) | monitor_nodes(Flag, Options)` — subscribe to
  `nodeup`/`nodedown`. `nodeup` delivered before any signals from the remote node;
  `nodedown` delivered after all signals over the connection. Options:
  `node_type => visible | hidden | all`, `connection_id => boolean()`,
  `nodedown_reason => boolean()`. Since OTP 23.0, a `nodedown` for a connection
  being taken down is delivered before a `nodeup` for a new connection to the
  same node.
- `get_net_ticktime()`, `set_net_ticktime(NetTicktime, TransitionPeriod)`.

`nodedown_reason` values (standard TCP distribution): `connection_setup_failed`,
`no_network`, `net_kernel_terminated`, `shutdown`, `connection_closed`,
`disconnect`, `net_tick_timeout`, `send_net_tick_failed`, `get_status_failed`.

### net_ticktime (failure detection)

The net tick mechanism is the failure-detection heart of distributed Erlang: if
ticks stop being exchanged within the tick window, the peer is presumed dead and
a `nodedown` with reason `net_tick_timeout` (or `send_net_tick_failed`) is
produced. MTTI = `minimum(NetTicktime, PreviousNetTicktime)*1000 div 4` ms.
"Warning: net_ticktime changes must be initiated on ALL nodes in the network
(with the same NetTicktime) before the end of any transition period on any node;
otherwise connections can erroneously be disconnected."

### Distributed message passing

"Message passing between processes at different nodes, as well as links and
monitors, are transparent when pids are used. Registered names, however, are
local to each node." Cross-node sends to registered names must include the node:
`{Name, Node} ! Msg`. `spawn/4`, `spawn_link/2,4`, `spawn_opt/3,5` accept a
`Node` argument. Nodes are loosely connected: the first time another node's name
is used, a connection attempt is made. "Connections are by default transitive."
Disable with `-connect_all false`. "If a node goes down, all connections to that
node are removed." `erlang:disconnect_node(Node)` forces disconnection.
`monitor_node(Node, Bool)` monitors node status; a `{nodedown, Node}` message is
received if the connection is lost.

### Hidden nodes and dynamic names

Hidden nodes (started with `-hidden`): "Connections between hidden nodes and
other nodes are not transitive; they must be set up explicitly. Also, hidden nodes
do not show up in the list of nodes returned by nodes/0. Instead, nodes(hidden) or
nodes(connected) must be used." Hidden nodes are excluded from the set `global`
tracks. Dynamic node name (OTP 23+, both peers): `Name = undefined` requests a
dynamic name from the first node it connects to; forces `dist_listen false
-hidden -kernel dist_auto_connect never`, so `net_kernel:connect_node/1` must be
called explicitly. Losing the granting connection drops all connections and the
name. C nodes: a C program acting as a hidden node via `Erl_Interface`.

### TLS distribution

"Starting a distributed node without also specifying -proto_dist inet_tls will
expose the node to attacks that may give the attacker complete access to the node
and by extension the cluster." Secure distribution uses `-proto_dist inet_tls` and
the SSL application's "Using TLS for Erlang Distribution" guide. The `inet_tls`
configuration detail is NOT on the `distributed.html` page; it points to the SSL
app guide and ERTS `alt_dist`.

### global name registration + resolver policies

`global` provides: (1) registration of global names (cluster-wide name->pid
aliases), (2) global locks, (3) maintenance of a fully connected network.
Registered names are stored in replica tables on every node — "the translation
of a name to a pid is fast, as it is always done locally." Changes propagate
automatically; names whose process or node goes down are unregistered.

Key functions:
- `register_name(Name, Pid) | register_name(Name, Pid, Resolve) -> yes | no` —
  fully synchronous: "when this function returns, the name is either registered
  on all nodes or none."
- `re_register_name(Name, Pid) | /3 -> yes`.
- `unregister_name(Name)`.
- `whereis_name(Name) -> pid() | undefined`.
- `send(Name, Msg) -> Pid` — "If `Name` is not a globally registered name, the
  calling function exits with reason `{badarg, {Name, Msg}}`."
- `registered_names() -> [Name]`.
- `set_lock(Id) | set_lock(Id, Nodes) | set_lock(Id, Nodes, Retries) -> boolean()`.
- `del_lock(Id) | del_lock(Id, Nodes) -> true`.
- `trans(Id, Fun) | trans(Id, Fun, Nodes) | trans(Id, Fun, Nodes, Retries) -> Res | aborted`.
- `sync() -> ok | {error, Reason}`.
- `disconnect() -> [node()]` (OTP 25.1) — preferred over manual disconnect.

Three predefined resolver policies (arity 3):
- `random_exit_name(Name, Pid1, Pid2) -> pid()` — randomly selects one pid and
  KILLS the other. (Default.)
- `random_notify_name(Name, Pid1, Pid2) -> pid()` — randomly selects one, sends
  `{global_name_conflict, Name}` to the other (no kill).
- `notify_all_name(Name, Pid1, Pid2) -> none` — unregisters BOTH and sends
  `{global_name_conflict, Name, OtherPid}` to both.

Overlapping partitions: as of OTP 25, `global` by default prevents overlapping
partitions by actively disconnecting. Controlled by
`prevent_overlapping_partitions` kernel parameter; must be enabled on ALL nodes.
"None of the above services will be reliably delivered unless both of the kernel
parameters `connect_all` and `prevent_overlapping_partitions` are enabled."

Reserved `ResourceId` values to avoid: `dist_ac`, `global`,
`mnesia_adjust_log_writes`, `mnesia_table_lock`.

### erpc vs rpc (prefer erpc)

`erpc` (Enhanced RPC, OTP 23+) is "an enhanced subset of the operations provided
by the rpc module. Enhanced in the sense that it makes it possible to distinguish
between returned value, raised exceptions, and other errors. erpc also has better
performance and scalability than the original rpc implementation." `rpc` is now
layered on `erpc`. `erpc` requires the remote node to support `erpc` (OTP 23+
ordinary nodes); `rpc` works against older nodes too.

Key functions: `call/2,3,4,5`, `cast/2,4`, `multicall/2,3,4,5`,
`multicast/2,4`, `send_request/2,4,6`, `receive_response/1,2,3`,
`wait_response/1,2,3`, `check_response/2,3`, `reqids_new/0`, `reqids_add/3`,
`reqids_size/1`, `reqids_to_list/1`.

Error semantics: failures of the erpc operation itself are `{erpc, Reason}`
(`badarg`, `noconnection`, `system_limit`, `timeout`, `notsup`), distinct from
exceptions raised by the applied function (`{throw, _}`, `{exit, {exception, _}}`,
`{error, {exception, _, _}}`, `{exit, {signal, _}}`). `multicall` returns
per-node `{ok, Value} | {Class, ExceptionReason}` rather than crashing the whole
call.

`call_options()` (OTP 28.0): `#{timeout => timeout_time(), always_spawn =>
boolean()}`. `timeout_time()`: `0..4294967295 | infinity | {abs, integer()}`
(absolute monotonic time in ms). `receive_response/2,3` abandons requests at
timeout (raises `{erpc, timeout}`); `wait_response/2,3` does NOT abandon
(returns `no_response`).

"Blocking signaling can, for example, cause timeouts in erpc to be significantly
delayed." (See coverage gap above.)

### The distribution carrier/protocol model (crawl 39)

The distribution is treated as a *carrier* layer: the protocol that transports
distribution traffic is pluggable. "This section describes how to implement an
alternative carrier protocol for the Erlang distribution. The distribution is
normally carried by TCP/IP. Here is explained a method for replacing TCP/IP with
another protocol."

The three layers:

1. **Driver / transport** — a native Erlang driver (C) implementing a reliable,
   order-maintaining, variable-length packet-oriented protocol. All I/O must be
   non-blocking. Since ERTS 10.0, a *distribution controller process* (Erlang
   process) can replace the port driver. "As of ERTS version 10.0 support for
   distribution controller processes has been introduced. That is, the traffic
   over a distribution channel can be managed by a process instead of only by a
   port."
2. **Erlang interface module** — mimics `inet`/`inet_tcp` to wrap the driver.
3. **Distribution module** (suffix `_dist`) — a callback module that `net_kernel`
   calls. Mandatory callbacks: `listen/1,2`, `address/0`, `accept/1`,
   `accept_connection/5`, `setup/5`, `close/1`, `select/1`. Optional:
   `setopts/2`, `getopts/2`.

Key structural rules:

- Exactly one distribution controller per connection (a process or a port); a
  process/port can be controller for only one connection; registration cannot be
  undone (sticks until the controller terminates). "Note that there need to be
  exactly one distribution controller per connection. A process or port can only
  be distribution controller for one connection. The registration as distribution
  controller cannot be undone. It will stick until the distribution controller
  terminates."
- `dist_util` does the hard work: handshakes, cookies, timers, ticking. The
  `#hs_data{}` record carries funs like `f_send`, `f_recv`, `mf_tick`,
  `mf_getstat`, `f_setopts_pre_nodeup`, `f_setopts_post_nodeup`,
  `f_handshake_complete`.
- Data delivery ordering: by default, data must be delivered in exact order, no
  loss. "the data to pass over a connection needs to be delivered as is to the
  node on the receiving end in the exact same order, with no loss of data what
  so ever, as sent from the sending node." Ordering can be relaxed by rejecting
  `dist_util:strict_order_flags/0` flags (only same sender/receiver pair order
  preserved) — but this may hurt performance.
- Selection: `-proto_dist <name>` (without `_dist` suffix) selects the module;
  `-no_epmd` skips epmd.

Relationship to net_kernel: `net_kernel` is the owner of the distribution; it
calls the distribution module's callbacks. The distribution module covers the
protocol details from `net_kernel` (net_kernel is agnostic to the carrier).
"The distribution module exposes an API that `net_kernel` calls in order to
manage connections to other nodes."

Distributed signals themselves (the actual message/signal format, capability
flags `dflags`, atom cache, fragmentation) are defined in
`erl_dist_protocol.html`, NOT this page. Reference
`https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html` as a follow-up
crawl target for signal-level detail.

### The blocking-signaling-over-distribution caveat (crawl 39, verbatim)

This is the caveat previously flagged as a gap. The `alt_dist` page does not use
the exact phrase "blocking signaling over distribution" but contains the
precise caveat that `ref_man_processes`/`erpc` defer to. Verbatim blockquotes
from crawl 39:

The tick opcode note:

> Note: It is important that the interface for sending ticks is not blocking.
> This implementation uses `erlang:port_control/3`, which does not block the
> caller. If `erlang:port_command` is used, use `erlang:port_command/3` and pass
> `[force]` as option list; otherwise the caller can be blocked indefinitely on
> a busy port and prevent the system from taking down a connection that is not
> functioning.

The `mf_tick` field note:

> Note: It is of vital importance that this operation does not block the caller
> for a long time. This since it is called from the connection supervisor.

The `mf_getstat` field note:

> Note: It is of vital importance that this operation does not block the caller
> for a long time. This since it is called from the connection supervisor.

The driver callback non-blocking rule:

> As the driver callback routines execute in the main thread of the Erlang
> machine, the callback functions can perform no blocking activity whatsoever.
> The callbacks are only to set up file descriptors for waiting and/or read/write
> available data. All I/O must be non-blocking.

Summary of the guidance to avoid blocking (from crawl 39 "Guidance to avoid
blocking"):

- Never block in driver callbacks (they run in the emulator's main thread; any
  blocking stalls the entire runtime).
- Send ticks via a non-blocking interface (`erlang:port_control/3`; or
  `erlang:port_command/3` with `[force]`).
- `mf_tick` and `mf_getstat` must not block the caller for a long time (called
  from the connection supervisor, which is responsible for ticking and tearing
  down dead connections).
- Prefer distribution controller processes (ERTS >= 10.0) over port drivers
  where possible.
- The connection supervisor / acceptor should run at `max` priority
  (`dist_util:net_ticker_spawn_options()` → `[link, {priority, max}]` by
  default; configurable via the `net_ticker_spawn_options` kernel parameter).

Connection to the existing erpc note (above): "Blocking signaling can, for
example, cause timeouts in erpc to be significantly delayed." — this is because
a blocking tick/getstat stalls the connection supervisor, which is responsible
for taking down non-functioning connections, so timeouts (including erpc
timeouts) can be significantly delayed.

## Practical rules

- A long-name node cannot communicate with a short-name node.
- Registered names are local per node; cross-node sends need `{Name, Node}`.
- Connections are transitive by default unless `-connect_all false`.
- Hidden nodes never appear in `nodes/0`; use `nodes(hidden)`/`nodes(connected)`.
- Plain distribution is cleartext; use `-proto_dist inet_tls` on untrusted
  networks.
- `net_ticktime` changes must be initiated on ALL nodes before any transition
  period ends.
- `global:register_name/3` is fully synchronous (all-or-nothing).
- Use external funs (`fun M:F/A`) for `Resolve` if hot-loading code.
- Avoid reserved `global` lock ResourceIds.
- Prefer `erpc` over `rpc` for new code (OTP 23+ peers).
- `erpc` `Args` list is NOT verified to be proper client-side.
- `monitor_nodes` `nodeup` precedes remote signals; `nodedown` follows them.
- Custom distribution carriers must use non-blocking I/O in all driver callbacks
  (blocking stalls the entire runtime).
- Send ticks via `erlang:port_control/3` (non-blocking); if using
  `erlang:port_command`, use `/3` with `[force]`.
- `mf_tick` and `mf_getstat` must not block (called from the connection
  supervisor; a block stalls ticking and teardown of dead connections — the root
  cause of delayed erpc timeouts).
- Exactly one distribution controller per connection; registration cannot be
  undone.
- Prefer distribution controller processes (ERTS >= 10.0) over custom port
  drivers.
- The connection supervisor should run at `max` priority
  (`net_ticker_spawn_options` kernel parameter).

## Review checklist

- [ ] Is `-proto_dist inet_tls` used on any untrusted network?
- [ ] Are `connect_all` and `prevent_overlapping_partitions` enabled on all nodes?
- [ ] Is `net_ticktime` consistent across the cluster?
- [ ] Are global names registered with an explicit resolver policy?
- [ ] Is `erpc` used (not `rpc`) where all peers are OTP 23+?
- [ ] Are `erpc` timeouts chosen with blocking-signaling delay in mind?
- [ ] If using a custom distribution carrier, are all driver callbacks
      non-blocking?
- [ ] Are ticks sent via a non-blocking interface (`port_control/3` or
      `port_command/3` with `[force]`)?
- [ ] Is the connection supervisor running at `max` priority?
- [ ] Are erpc timeouts chosen with blocking-signaling delay in mind (a blocking
      tick stalls teardown)?

## Implementation checklist

- [ ] Start nodes with `-name`/`-sname` and a strong cookie.
- [ ] Use `net_kernel:monitor_nodes(true, #{node_type => all, ...})` for
      node-status tracking.
- [ ] Register cluster-singleton processes via `global:register_name/3`.
- [ ] Use `erpc:multicall/5` for fan-out; handle per-node `{ok,_}|{Class,_}`.
- [ ] Use `erpc:send_request/4` + `receive_response/2` for async RPC.
- [ ] Prefer `global:disconnect/0` over manual disconnect on shutdown.
- [ ] Custom distribution module implements `listen/1,2`, `address/0`,
      `accept/1`, `accept_connection/5`, `setup/5`, `close/1`, `select/1`.
- [ ] Distribution controller process (ERTS >= 10.0) preferred over a custom
      port driver.
- [ ] `dist_util` used for handshakes/cookies/ticking via the `#hs_data{}` funs.

## Runtime / debugging checklist

- [ ] `node/0`, `nodes/0`, `nodes(hidden)`, `nodes(connected)` for topology.
- [ ] `net_kernel:get_state()` (OTP 25.0) for distribution state.
- [ ] `net_kernel:get_net_ticktime()` for tick configuration.
- [ ] `erlang:get_cookie/0` / `get_cookie(Node)` for cookie inspection.
- [ ] `global:registered_names/0` / `global:whereis_name/1`.
- [ ] `global:sync/0` to reconcile the name server.
- [ ] `net_adm:ping(Node)` to probe connectivity (not crawled in detail).
- [ ] Check connection supervisor priority (`net_ticker_spawn_options` kernel
      parameter).
- [ ] For custom carriers, verify `mf_tick`/`mf_getstat` are non-blocking (a
      block delays erpc timeouts).

## Validation hooks

- After `net_kernel:start/2`, assert `{ok, _}` and `is_alive() =:= true`.
- After `net_kernel:connect_node/1`, assert `true`.
- `monitor_nodes` assertions: `nodeup` before signals, `nodedown` after.
- `global:register_name/3` returns `yes`; `whereis_name/1` resolves the pid.
- `erpc:call/5` returns the value or raises a distinguishable `{erpc, _}`.
- `erpc:multicall/5` returns a per-node result list aligned with `Nodes`.
- After starting a custom-carrier node (`-proto_dist <name>`), assert
  `is_alive() =:= true` and `node/0` returns the expected name.
- Assert the connection supervisor is running at `max` priority.
- Assert erpc timeouts are not significantly delayed (indicating non-blocking
  tick/getstat).

## Examples

Register a global singleton with a notify resolver:
```erlang
case global:register_name(my_service, self(), fun global:random_notify_name/3) of
    yes -> ok;
    no  -> exit({name_already_registered, my_service})
end.
```

Async erpc with a deadline:
```erlang
Deadline = erlang:monotonic_time(millisecond) + 5000,
ReqId = erpc:send_request(Node, my_mod, do_work, [Arg]),
case erpc:receive_response(ReqId, {abs, Deadline}) of
    {ok, Result}        -> Result;
    {error, {erpc, _}}  -> fallback()
end.
```

Select an alternative distribution protocol:
```erlang
%% Start a node using a custom distribution carrier (e.g. Unix domain sockets)
%% erl -sname mynode -proto_dist inet6_tcp -no_epmd
%% Or at runtime:
net_kernel:start({mynode, shortnames}, #{proto_dist => inet6_tcp}).
```

## Common mistakes

- Mixing long and short node names (incompatible).
- Relying on cleartext distribution on untrusted networks.
- Changing `net_ticktime` on one node only (partitions the cluster).
- Using a local fun as `global` resolver (blocks hot code loading).
- Using `rpc:call` where `erpc` is available (worse error semantics).
- Assuming `erpc:call` timeout bounds wall-clock latency (blocking signaling can
  delay it — see gap).
- Forgetting that `erpc` `Args` properness is not checked client-side.
- Blocking in a distribution driver callback (stalls the entire runtime — the
  root cause of blocking-signaling delays).
- Sending ticks via `erlang:port_command/2` without `[force]` (caller can be
  blocked indefinitely on a busy port).
- Allowing `mf_tick`/`mf_getstat` to block (stalls the connection supervisor;
  delays erpc timeouts and dead-connection teardown).
- Assuming `erpc:call` timeout bounds wall-clock latency (blocking signaling can
  delay it — now sourced from crawl 39).
- Registering a process/port as distribution controller for more than one
  connection (not allowed; cannot be undone).

## Strict vs contextual guidance

Strict:
- Long vs short names are incompatible.
- `connect_all` + `prevent_overlapping_partitions` must be on for reliable
  `global`.
- `net_ticktime` changes must be cluster-wide and synchronized.
- `global:register_name/3` is all-or-nothing synchronous.
- `erpc` requires OTP 23+ peers.
- All distribution driver callbacks must be non-blocking (run in the emulator's
  main thread).
- Exactly one distribution controller per connection; registration cannot be
  undone.
- Ticks must be sent via a non-blocking interface (`port_control/3` or
  `port_command/3` with `[force]`).
- `mf_tick`/`mf_getstat` must not block the caller for a long time.

Contextual:
- `net_ticktime` value (default 60s) — tune by network characteristics.
- `erpc` `always_spawn` — `false` (default) is faster but process-identity is
  unspecified.
- `receive_response` (abandon) vs `wait_response` (keep waiting) at timeout.
- Distribution controller process (ERTS >= 10.0) vs port driver — prefer the
  process where possible.
- Connection supervisor priority (`max` by default; configurable via
  `net_ticker_spawn_options`).
- Relaxing strict data-delivery ordering (reject `strict_order_flags/0`) — may
  hurt performance.

## Policy decisions for individual repos

- Whether TLS distribution is mandatory in all environments.
- Default `net_ticktime` / `net_tickintensity` for the deployment.
- Required `erpc` timeout slack for blocking-signaling delay.
- Whether `global` or a custom registry is used for singletons.
- Whether hidden nodes are used for O&M/inspection.
- Whether a custom distribution carrier (e.g. Unix domain sockets) is used
  instead of TCP/IP.
- Required erpc timeout slack for blocking-signaling delay on custom carriers.
- Connection supervisor priority (`net_ticker_spawn_options`).

## Related docs

- `processes-and-messages.md` — distributed message passing.
- `links-monitors-and-exits.md` — distributed links/monitors.
- `runtime-debugging.md` — `net_kernel:get_state`, `global:sync`.
- `validation.md` — distribution validation hooks.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
