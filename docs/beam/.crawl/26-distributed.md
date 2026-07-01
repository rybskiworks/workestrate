# Crawl: system/distributed.html
- seed_url: https://www.erlang.org/doc/system/distributed.html
- canonical_url: https://www.erlang.org/doc/system/distributed.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (page built with ExDoc v0.40.3; source tag OTP-29.0.2)
- feeds_docs: distribution.md (or processes-and-messages.md)

## Purpose
Describes the Distributed Erlang System: a set of Erlang runtime systems
("nodes") communicating over TCP/IP sockets. Covers node naming, the magic
cookie authentication model, EPMD, hidden/dynamic/C nodes, distribution BIFs,
command-line flags, and Kernel/STDLIB modules used for distribution. It is the
system-level (reference_manual/distributed) page, not the ERTS internals page.

## Key concepts (nodes, sname/name, cookie, EPMD, hidden nodes, TLS distribution)
- **Node**: an executing Erlang runtime system given a name via `-name` (long
  names) or `-sname` (short names). Node name format is the atom `name@host`.
  - `name`: alphanumerics, `-`, `_`, `\`.
  - `host`: full host name (long names) or first part of host name (short names).
  - `node/0` returns the current node name; a non-distributed runtime returns
    `nonode@nohost`.
  - A long-name node **cannot** communicate with a short-name node.
  - Name can also be set at runtime via `net_kernel:start/1`
    (e.g. `net_kernel:start([dilbert, shortnames])`).
- **EPMD** (Erlang Port Mapper Daemon): automatically started on every host
  where an Erlang node starts; maps symbolic node names to machine addresses.
  See `epmd` in ERTS.
- **Magic cookie**: an Erlang atom used as the shared-secret authentication
  token between nodes (see Security model below).
- **Hidden nodes**: started with `-hidden`. Connections to/from hidden nodes
  are **not transitive** (must be set up explicitly) and hidden nodes do **not**
  appear in `nodes/0`; use `nodes(hidden)` or `nodes(connected)`. Hidden nodes
  are also excluded from the set `global` tracks. Typical use: O&M/inspection
  without disturbing the cluster.
- **Dynamic node name**: if the node name is set to `undefined`, the node
  starts as a temporary client that requests a dynamic name from the first node
  it connects to. Forces `-dist_listen false -hidden -kernel dist_auto_connect
  never`, so `net_kernel:connect_node/1` must be called explicitly. If the
  connection that granted the dynamic name closes, all other connections close
  and the name is lost; a new `connect_node/1` yields a new dynamic name.
  - *Change*: dynamic node name supported from Erlang/OTP 23; both peers must be
    >= OTP 23.
- **C nodes**: a C program acting as a hidden node, via the `Erl_Interface`
  library.
- **TLS distribution**: the page repeatedly warns that plain distribution is
  cleartext and insecure. Secure distribution uses `-proto_dist inet_tls` and
  the SSL application's "Using TLS for Erlang Distribution" guide. The page
  does not detail `inet_tls` configuration itself; it points to the SSL app
  guide (`../apps/ssl/ssl_distribution.html`) and ERTS alt carrier doc
  (`../apps/erts/alt_dist.html`).

## Message passing across nodes
- Message passing, links, and monitors are **transparent across nodes when
  pids are used**.
- **Registered names are local to each node** — when sending to a registered
  name on another node the node must be specified, i.e. `{Name, Node} ! Msg`.
  (The page states this rule; the literal `Pid ! Msg` / `{Name,Node} ! Msg`
  syntax forms come from the processes reference page it links to.)
- `spawn/4`, `spawn_link/2,4`, `spawn_opt/3,5` accept a `Node` argument to
  create a process on a remote node.
- Node connections are **loosely connected**: the first time another node's
  name is used (e.g. `spawn(Node,...)` or `net_adm:ping(Node)`), a connection
  attempt is made.
- Connections are **transitive by default**: if A connects to B and B is
  connected to C, A also tries to connect to C. Disable with
  `-connect_all false`. If a node goes down, all connections to it are removed.
  `erlang:disconnect_node(Node)` forces disconnection.
- `nodes/0` returns the list of visible connected nodes.

## Security model (cookie as shared secret; risks)
- "Security" here means **protection against accidental misuse**, NOT
  cryptographic security.
- Inter-node communication is **clear text by default**.
- The default random cookie is **not very unpredictable**; `crypto` can
  generate a better one, but the **initial handshake is still not
  cryptographically secure** and traffic remains cleartext.
- Authentication is built in at the lowest level: all nodes use a magic cookie
  (an atom). During connection setup, after node names are exchanged, cookies
  are compared; if they differ the connection is rejected. Cookies are **never
  transferred** — compared via hashed challenges (not cryptographically secure).
- At startup a node gets a random default cookie and assumes other nodes'
  cookie is `nocookie`. The `auth` server searches for `.erlang.cookie` in the
  user's home directory, then in `filename:basedir(user_config, "erlang")`. If
  none exists, it creates `.erlang.cookie` in the home dir with UNIX mode
  `0400` containing a random string; an atom `Cookie` is created and set via
  `erlang:set_cookie(Cookie)` as the default for all nodes.
- Groups of users with identical cookie files get freely-communicating nodes.
- For a node `Node1` (cookie `Cookie`) to connect to / accept a connection from
  `Node2` with a different `DiffCookie`, call
  `erlang:set_cookie(Node2, DiffCookie)` on `Node1` first. Symmetric
  cross-default-cookie configuration is **broken** (each side uses the other's
  differing cookie).
- Because node names are exchanged before cookies are selected, connection
  setup works regardless of which side initiates.
- With many differing cookies, `-connect_all false` is required (fully-connected
  default mesh is impractical to configure pairwise).
- `erlang:get_cookie/0` returns the local node's cookie;
  `erlang:get_cookie(Node)` returns a specific node's cookie.

## Distributed signals / blocking signaling caveats
- This page does **not** describe distributed signals or the "blocking
  signaling over distribution" caveat. It covers only message passing, links,
  and monitors (which are transparent across nodes when pids are used) and
  `monitor_node/2` for node-status monitoring. The blocking-signaling caveat
  (e.g. `exit/2` propagation / `monitor` signal delivery blocking on a slow
  distribution channel) is documented elsewhere in ERTS — candidate follow-up:
  `../apps/erts/alt_dist.html` and the ERTS distribution internals.

## Failure detection
- `monitor_node(Node, Bool)`: monitors the status of `Node`; a `{nodedown, Node}`
  message is received if the connection is lost.
- If a node goes down, **all connections to that node are removed**
  (automatic cleanup).
- `erlang:disconnect_node(Node)` forces disconnection.
- `is_alive/0` returns `true` if the runtime is a node capable of connecting to
  other nodes.
- `nodes/0` reflects currently connected visible nodes (so a `nodedown` is
  observable as the node disappearing from `nodes/0`).

## Strict rules
- A **long-name node cannot communicate with a short-name node**.
- Registered names are **local per node**; cross-node sends to registered names
  must include the node: `{Name, Node} ! Msg`.
- Connections are **transitive by default** unless `-connect_all false`.
- Hidden nodes never appear in `nodes/0`; use `nodes(hidden)` / `nodes(connected)`.
- Dynamic-name nodes require `net_kernel:connect_node/1` (auto-connect is
  `never`); losing the granting connection drops all connections and the name.
- Plain distribution is **cleartext and not cryptographically secure**; use
  `-proto_dist inet_tls` (SSL distribution) for strong security.
- Cross-default-cookie configuration (each node using the other's differing
  default cookie) is broken.
- Dynamic node name requires both peers >= Erlang/OTP 23.

## Verbatim quotes
- "A distributed Erlang system consists of a number of Erlang runtime systems
  communicating with each other. Each such runtime system is called a node."
- "Message passing between processes at different nodes, as well as links and
  monitors, are transparent when pids are used. Registered names, however, are
  local to each node."
- "The distribution mechanism is implemented using TCP/IP sockets."
- "Starting a distributed node without also specifying -proto_dist inet_tls
  will expose the node to attacks that may give the attacker complete access
  to the node and by extension the cluster."
- "A node is an executing Erlang runtime system that has been given a name,
  using the command-line flag -name (long names) or -sname (short names)."
- "A node with a long node name cannot communicate with a node with a short
  node name."
- "The nodes in a distributed Erlang system are loosely connected. The first
  time the name of another node is used ... a connection attempt to that node
  is made."
- "Connections are by default transitive."
- "If a node goes down, all connections to that node are removed."
- "The Erlang Port Mapper Daemon epmd is automatically started at every host
  where an Erlang node is started. It is responsible for mapping the symbolic
  node names to machine addresses."
- "A hidden node is a node started with the command-line flag -hidden.
  Connections between hidden nodes and other nodes are not transitive; they
  must be set up explicitly. Also, hidden nodes do not show up in the list of
  nodes returned by nodes/0. Instead, nodes(hidden) or nodes(connected) must
  be used."
- "'Security' here does not mean cryptographically secure, but rather security
  against accidental misuse ... Furthermore, the communication between nodes
  is by default in clear text."
- "All nodes use a magic cookie, which is an Erlang atom, when connecting to
  another node."
- "The cookies themselves are never transferred; instead, they are compared
  using hashed challenges, although not in a cryptographically secure manner."
- "monitor_node(Node, Bool) - Monitors the status of Node. A {nodedown, Node}
  message is received if the connection to it is lost."

## Version notes
- Page metadata: Erlang System Documentation v29.0.2 / OTP 29.0.2.
- Source: `github.com/erlang/otp/blob/OTP-29.0.2/system/doc/reference_manual/distributed.md`.
- Built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- Dynamic node name feature: supported from Erlang/OTP 23 (both peers must be
  >= OTP 23).

## Discovered links

### Relevant (crawl later)
- ../apps/ssl/ssl_distribution.html — Using TLS for Erlang Distribution (the `inet_tls` / `-proto_dist inet_tls` detail)
- ../apps/erts/alt_dist.html — alternative distribution carrier / ERTS distribution internals (blocking-signaling caveat likely here)
- ../apps/kernel/net_kernel.html — net_kernel (start/1, connect_node/1)
- ../apps/kernel/global.html — global name registration
- ../apps/kernel/global_group.html — global_group
- ../apps/kernel/net_adm.html — net_adm (ping/1)
- ../apps/stdlib/peer.html — peer node control (STDLIB)
- ../apps/erts/erl_cmd.html — erl flags (-name/-sname/-hidden/-connect_all/-setcookie/-proto_dist)
- ../apps/erts/epmd_cmd.html — epmd
- ../apps/erl_interface/ei_users_guide.html — C nodes / Erl_Interface
- ../system/tutorial.html — Interoperability Tutorial (C nodes)
- code_loading.html — next page (Compilation and Code Loading)
- ref_man_processes.html — previous page (Processes; message-passing syntax)

### Skipped
- In-page anchors (#nodes, #security, #epmd, #hidden-nodes, #c-nodes,
  #dynamic-node-name, #node-connections, #distribution-bifs,
  #distribution-command-line-flags, #distribution-modules,
  #distributed-erlang-system)
- ../apps/erts/erlang.html#* BIF anchors (covered conceptually here; individual
  BIF pages deferred to a dedicated erlang.html crawl)
- ../apps/erts/init.html#home, ../apps/stdlib/filename.html#user_config
- ../index.html, llms.txt, distributed.md (markdown source mirror),
  "Erlang System Documentation.epub", CSS/JS assets,
  https://erlang.org, https://www.ericsson.com,
  https://github.com/erlang/otp/...source link,
  https://github.com/elixir-lang/ex_doc
