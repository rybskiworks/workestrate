# Crawl: kernel/net_kernel.html
- seed_url: https://www.erlang.org/doc/apps/kernel/net_kernel.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/net_kernel.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel v11.0.2)
- feeds_docs: distribution.md

## Purpose
The net kernel is a system process, registered as `net_kernel`, which must be
operational for distributed Erlang to work. It implements parts of the BIFs
`spawn/4` and `spawn_link/4`, and provides monitoring of the network.

An Erlang node is started with command-line flag `-name` or `-sname`, or by
calling `net_kernel:start(Name, Options)` from a non-distributed shell. With
`-sname` the node name is `foobar@Host` (short host name); with `-name` it is
`foobar@Host` (fully qualified domain name).

Connections are normally established automatically when another node is
referenced. This can be disabled by Kernel config parameter `dist_auto_connect`
set to `never`, in which case connections must be established explicitly via
`connect_node/1`. Which nodes may communicate is governed by the magic cookie
system.

Security warning: starting a distributed node without `-proto_dist inet_tls`
exposes the node to attacks that may give complete access to the node and the
cluster. See the SSL for Erlang Distribution User's Guide.

## Key functions (exact arities)
- `allow(Nodes)` -> `ok | error | ignored` — permits access to specified set of nodes.
- `allowed()` -> `{ok, Nodes} | ignored` (since OTP 28.0) — returns explicitly allowed nodes.
- `connect_node(Node)` -> `boolean() | ignored` — establishes a connection to Node.
- `get_net_ticktime()` -> `NetTicktime | {ongoing_change_to, NetTicktime} | ignored`.
- `get_state()` -> map (since OTP 25.0) — current distribution state.
- `getopts(Node, Options)` (since OTP 19.1) — get distribution socket options.
- `monitor_nodes(Flag)` -> `ok | Error` — equivalent to `monitor_nodes(Flag, [])`.
- `monitor_nodes(Flag, Options)` -> `ok | Error` — subscribe to nodeup/nodedown.
- `set_net_ticktime(NetTicktime)` — equivalent to `set_net_ticktime(NetTicktime, 60)`.
- `set_net_ticktime(NetTicktime, TransitionPeriod)` — sets net_ticktime.
- `setopts(Node, Options)` (since OTP 19.1) — set distribution socket options.
- `start(Options)` -> `{ok, pid()} | {error, Reason}` — DEPRECATED, use `start/2`.
- `start(Name, Options)` -> `{ok, pid()} | {error, Reason}` (since OTP 24.3).
- `stop()` -> `ok | {error, Reason}` — turns a distributed node into non-distributed.

Types:
- `connection_state() :: check_pending | pending | up | up_pending` (not exported).
- `connection_type() :: normal | hidden` (not exported).

NOTE: This OTP 29 page does NOT document `disconnect/1`, `adjacent_nodes/1`,
or an `epmd_module` function. `disconnect` appears only as a `nodedown_reason`
value. `epmd_module` is a kernel(6) configuration parameter, not a
`net_kernel` API on this page. Distribution over TLS is referenced only via
the security warning linking to the SSL distribution guide and the
`-proto_dist inet_tls` erl flag.

## start/stop node
`start(Name, Options)` (OTP 24.3+) turns a non-distributed node into a
distributed node by starting net_kernel and other necessary processes.

If `Name` is `undefined`, the distribution requests a dynamic node name from
the first node it connects to (Dynamic Node Name). Setting `Name` to
`undefined` implies `dist_listen => false` and `hidden => true`.

Supported `Options` map keys:
- `name_domain => shortnames | longnames` — host name part of node name; `longnames` (fully qualified) is default.
- `net_ticktime => NetTickTime` (pos_integer, seconds) — defaults to `net_ticktime` kernel parameter.
- `net_tickintensity => NetTickIntensity` (4..1000) — defaults to `net_tickintensity` kernel parameter.
- `dist_listen => boolean()` — enable/disable listening for incoming connections; defaults to `-dist_listen` erl arg. `dist_listen => false` implies `hidden => true`. Overridden to `false` when `Name` is `undefined`.
- `hidden => boolean()` — enable/disable hidden node; defaults to true if `-hidden` erl arg passed, else false. Overridden to `true` when `Name` is `undefined` or `dist_listen` is `false`.

Deprecated `start(Options)` list form (order important):
- `[Name]` == `start([Name, longnames, 15000])`.
- `[Name, NameDomain]` == `start([Name, NameDomain, 15000])`.
- `[Name, NameDomain, TickTime]` == `start(Name, #{name_domain => NameDomain, net_ticktime => ((TickTime*4-1) div 1000) + 1, net_tickintensity => 4})`. Note `TickTime` is the time between ticks (ms) when intensity equals 4, NOT net tick time.

`stop()` turns a distributed node into a non-distributed node. For other
nodes this is the same as the node going down. Only possible when net_kernel
was started via `start/2`; otherwise returns `{error, not_allowed}`. Returns
`{error, not_found}` if the local node is not alive.

## monitor_nodes + nodedown items
`monitor_nodes(Flag, Options)` — the calling process subscribes (Flag=true) or
unsubscribes (Flag=false) to node status change messages. Two option lists are
the same if they contain the same set of options. `nodeup` delivered when a new
node connects; `nodedown` when disconnected.

Delivery guarantees:
- `nodeup` delivered before any signals from the remote node through the new connection.
- `nodedown` delivered after all signals from the remote node over the connection have been delivered.
- `nodeup` delivered after the node appears in `erlang:nodes()`.
- `nodedown` delivered after the node disappears from `erlang:nodes()`.
- Since OTP 23.0: a `nodedown` for a connection being taken down is delivered before a `nodeup` due to a new connection to the same node.

Message formats:
- Empty options (or `monitor_nodes/1`): `{nodeup, Node} | {nodedown, Node}`. Only visible nodes.
- Non-empty options: `{nodeup, Node, Info} | {nodedown, Node, Info}` where `Info` is a map (if Options is a map) or a list of 2-tuples (if Options is a list).

Map Options associations:
- `connection_id => boolean()` — if true, `connection_id => ConnectionId` included in Info. ConnectionId is the connection identifier (see `erlang:nodes/2`).
- `node_type => visible | hidden | all` — which node types to subscribe to. The `node_type` association is echoed into Info. If absent, subscribes to visible only (no echo).
- `nodedown_reason => boolean()` — if true, `nodedown_reason => Reason` included in Info for nodedown messages.

List Options:
- `connection_id` — `{connection_id, ConnectionId}` tuple in Info.
- `{node_type, NodeType}` — echoed into Info.
- `nodedown_reason` — `{nodedown_reason, Reason}` tuple in Info for nodedown.

`nodedown_reason` Reason values (standard TCP distribution module):
- `connection_setup_failed` — connection setup failed (after nodeup messages were sent).
- `no_network` — no network is available.
- `net_kernel_terminated` — the net_kernel process terminated.
- `shutdown` — unspecified connection shutdown.
- `connection_closed` — the connection was closed.
- `disconnect` — the connection was disconnected (forced from the current node).
- `net_tick_timeout` — net tick time-out.
- `send_net_tick_failed` — failed to send net tick over the connection.
- `get_status_failed` — status information retrieval from the Port holding the connection failed.

Example:
```
(a@localhost)1> net_kernel:monitor_nodes(true, #{connection_id=>true, node_type=>all, nodedown_reason=>true}).
ok
% {nodeup,b@localhost,#{connection_id => 3067552,node_type => visible}}
% {nodedown,b@localhost,#{connection_id => 3067552,node_type => visible,
%                       nodedown_reason => connection_closed}}
% {nodedown,c@localhost,#{connection_id => 13892107,node_type => hidden,
%                        nodedown_reason => net_tick_timeout}}
```

## net ticktime (failure detection)
`get_net_ticktime()` returns the currently used net tick time in seconds.
Return values:
- `NetTicktime` — net_ticktime is NetTicktime seconds.
- `{ongoing_change_to, NetTicktime}` — net_kernel is currently changing net_ticktime.
- `ignored` — local node is not alive.

`set_net_ticktime(NetTicktime, TransitionPeriod)` sets `net_ticktime` (see
kernel(6)) to `NetTicktime` seconds. `TransitionPeriod` defaults to 60.

Definitions:
- Minimum transition traffic interval (MTTI) = `minimum(NetTicktime, PreviousNetTicktime)*1000 div 4` milliseconds.
- Transition period = the least number of consecutive MTTIs to cover TransitionPeriod seconds = `((TransitionPeriod*1000 - 1) div MTTI + 1)*MTTI` ms.

If `NetTicktime < PreviousNetTicktime`, the change is done at the end of the
transition period; otherwise at the beginning. During the transition period,
net_kernel ensures outgoing traffic on all connections at least every MTTI ms.

WARNING: net_ticktime changes must be initiated on ALL nodes in the network
(with the same NetTicktime) before the end of any transition period on any
node; otherwise connections can erroneously be disconnected.

Returns:
- `unchanged` — net_ticktime already equals NetTicktime.
- `change_initiated` — net_kernel initiated the change.
- `{ongoing_change_to, NewNetTicktime}` — request ignored because a change is ongoing.

The net tick mechanism is the failure-detection heart of distributed Erlang:
if ticks stop being exchanged over a connection within the tick window, the
peer is presumed dead and a `nodedown` with reason `net_tick_timeout` (or
`send_net_tick_failed`) is produced.

## Strict rules
- `stop/0` only works if net_kernel was started via `start/2`; otherwise `{error, not_allowed}`.
- `allow/1`: before the first call, any node with the correct cookie can connect. After the first call, only listed nodes are allowed. Subsequent calls ADD to the list; nodes cannot be removed. Disallowing an already-connected node does NOT disconnect it, but prevents future reconnection. Empty list has no effect. Returns `error` if any element is not an atom, `ignored` if local node not alive.
- `connect_node/1` returns `true` if connection established/already established or Node is the local node; `false` if attempt failed; `ignored` if not alive.
- `setopts/2` with `Node = new` also adds Options to kernel config `inet_dist_listen_options` and `inet_dist_connect_options`.
- `dist_listen => false` implies `hidden => true`. `Name = undefined` forces `dist_listen => false` and `hidden => true`.
- `net_tickintensity` valid range is 4..1000.
- Two `monitor_nodes` option lists are equal iff they contain the same set of options.
- Security: never run distributed nodes without `-proto_dist inet_tls` on untrusted networks.

## Verbatim quotes
- "The net kernel is a system process, registered as net_kernel, which must be operational for distributed Erlang to work."
- "Starting a distributed node without also specifying -proto_dist inet_tls will expose the node to attacks that may give the attacker complete access to the node and in extension the cluster."
- "Normally, connections are established automatically when another node is referenced. This functionality can be disabled by setting Kernel configuration parameter dist_auto_connect to never ... connections must be established explicitly by calling connect_node/1."
- "nodeup messages are delivered before delivery of any signals from the remote node through the newly established connection."
- "nodedown messages are delivered after all the signals from the remote node over the connection have been delivered."
- "As of OTP 23.0, a nodedown message for a connection being taken down will be delivered before a nodeup message due to a new connection to the same node."
- "The net_ticktime changes must be initiated on all nodes in the network (with the same NetTicktime) before the end of any transition period on any node; otherwise connections can erroneously be disconnected."
- "If Name is set to undefined the distribution will be started to request a dynamic node name from the first node it connects to."
- "Disallowing an already connected node will not cause it to be disconnected. It will, however, prevent any future reconnection attempts."

## Version notes
- Page documents OTP 29.0.2 / kernel v11.0.2.
- `allowed/0` since OTP 28.0.
- `get_state()` since OTP 25.0.
- `getopts/2`, `setopts/2` since OTP 19.1.
- `start/2` since OTP 24.3.
- `start/1` (list form) is DEPRECATED in favor of `start/2`.
- OTP 23.0 changed nodedown-before-nodeup ordering guarantee for reconnections.
- `net_tickintensity` range 4..1000 (newer tick intensity model; legacy `start/1` TickTime assumed intensity 4).
- Built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/system/distributed.html — Distributed Erlang (Reference Manual); cookie system, dynamic node name.
- https://www.erlang.org/doc/system/distributed.html#dyn_node_name — Dynamic Node Name section.
- https://www.erlang.org/doc/apps/ssl/ssl_distribution.html — Using SSL for Erlang Distribution (TLS distribution / `-proto_dist inet_tls`).
- https://www.erlang.org/doc/apps/kernel/kernel.html — kernel(6) config: `net_ticktime`, `net_tickintensity`, `dist_auto_connect`, `inet_dist_listen_options`, `inet_dist_connect_options`, `epmd_module`.
- https://www.erlang.org/doc/apps/erts/erl_cmd.html — erl flags: `-name`, `-sname`, `-hidden`, `-dist_listen`, `-proto_dist`.
- https://www.erlang.org/doc/apps/erts/erlang.html#nodes/2 — `erlang:nodes/2`, connection identifiers.
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn/4 — `spawn/4` BIF implemented partly by net_kernel.

### Skipped
- In-page anchors (#allow/1, #start/2, etc.) — internal to this page.
- CSS/JS assets (algolia-typeahead.css, html-erlang-*.css).
- https://erlang.org — site root.
- https://github.com/elixir-lang/ex_doc — tooling.
- GitHub source links to lib/kernel/src/net_kernel.erl (OTP-29.0.2) — source, not docs.
- https://www.erlang.org/doc/index.html — index.
- llms.txt / ePub download links.
