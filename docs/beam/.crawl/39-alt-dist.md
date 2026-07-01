# Crawl: erts/alt_dist.html
- seed_url: https://www.erlang.org/doc/apps/erts/alt_dist.html
- canonical_url: https://www.erlang.org/doc/apps/erts/alt_dist.html
- family: Erlang/OTP ERTS docs
- fetch: 200
- otp_version: OTP 29.0.2 (erts 17.0.2; major-vsn 29)
- feeds_docs: distribution.md

## Purpose
This page describes how to implement an alternative *carrier* protocol for the
Erlang distribution. The distribution is normally carried by TCP/IP; this guide
explains how to replace TCP/IP with another protocol (e.g. Unix domain sockets),
walking through the `uds_dist` example application. It covers writing an Erlang
driver (or, since ERTS 10.0, a distribution controller process), an Erlang
interface module, a distribution module with well-defined callbacks that
`net_kernel` invokes, and boot scripts to start the protocol at boot time.

## Distribution carrier/protocol model
The distribution is treated as a *carrier* layer: the protocol that transports
distribution traffic is pluggable. The model has three layers:

1. **Driver / transport** — A native Erlang driver (C) that implements a
   reliable, order-maintaining, variable-length packet-oriented protocol. All
   I/O must be non-blocking (sockets in non-blocking mode, `driver_select` for
   readiness). Since ERTS 10.0, a *distribution controller process* (Erlang
   process) can replace the port driver, so large parts of the logic can live in
   Erlang (e.g. `gen_tcp_dist`, `erl_uds_dist`); a driver may not even be needed.
2. **Erlang interface module** — Mimics `inet`/`inet_tcp` to wrap the driver,
   covering protocol details from `net_kernel`.
3. **Distribution module** (suffix `_dist`) — A callback module (like a
   `gen_server`) that `net_kernel` calls to manage connections. Mandatory
   callbacks: `listen/1,2`, `address/0`, `accept/1`, `accept_connection/5`,
   `setup/5`, `close/1`, `select/1`. Optional: `setopts/2`, `getopts/2`.

Key structural rules:
- There must be **exactly one distribution controller per connection** (a process
  or a port). A process/port can be distribution controller for only one
  connection. Registration as distribution controller **cannot be undone**; it
  sticks until the controller terminates. The controller should not ignore exit
  signals (may trap exits but must voluntarily terminate on exit).
- The distribution module creates a listening entity + acceptor process, and per
  connection a connection supervisor (does the handshake) + a distribution
  controller (dispatches traffic). Supervisor and controller should be linked.
- `dist_util` does the hard work: handshakes, cookies, timers, ticking. The
  `#hs_data{}` record carries funs like `f_send`, `f_recv`, `mf_tick`,
  `mf_getstat`, `f_setopts_pre_nodeup`, `f_setopts_post_nodeup`,
  `f_handshake_complete`.
- **Data delivery ordering**: by default, data must be delivered in exact order,
  no loss. Ordering can be relaxed by rejecting the flags from
  `dist_util:strict_order_flags/0` in `reject_flags` (only same sender/receiver
  pair order must be preserved) — but this may hurt performance/throughput/latency.
- Selection: `-proto_dist <name>` (without `_dist` suffix) selects the module;
  `-no_epmd` skips epmd. `net_kernel:start/1` starts distribution on a running
  system.

## The blocking-signaling-over-distribution caveat (verbatim)
The page does not use the exact phrase "blocking signaling over distribution",
but it contains the precise caveat that `ref_man_processes`/`erpc` defer to.
There are three verbatim statements:

**1. Tick opcode (`'T'`) note — the core caveat about a blocking tick stalling
the system's ability to take down a dead connection:**

> Note: It is important that the interface for sending ticks is not blocking.
> This implementation uses `erlang:port_control/3`, which does not block the
> caller. If `erlang:port_command` is used, use `erlang:port_command/3` and pass
> `[force]` as option list; otherwise the caller can be blocked indefinitely on
> a busy port and prevent the system from taking down a connection that is not
> functioning.

**2. `mf_tick` field note:**

> Note: It is of vital importance that this operation does not block the caller
> for a long time. This since it is called from the connection supervisor.

**3. `mf_getstat` field note:**

> Note: It is of vital importance that this operation does not block the caller
> for a long time. This since it is called from the connection supervisor.

**4. Driver callback non-blocking rule:**

> As the driver callback routines execute in the main thread of the Erlang
> machine, the callback functions can perform no blocking activity whatsoever.
> The callbacks are only to set up file descriptors for waiting and/or read/write
> available data. All I/O must be non-blocking.

## Guidance to avoid blocking
- **Never block in driver callbacks.** Callbacks run in the emulator's main
  thread; any blocking stalls the entire runtime. Use non-blocking sockets +
  `driver_select` for readiness; do read/write only when ready.
- **Send ticks via a non-blocking interface.** Use `erlang:port_control/3`
  (synchronous control interface, does not block the caller). If you must use
  `erlang:port_command`, use `erlang:port_command/3` with the `[force]` option —
  otherwise the caller can be blocked indefinitely on a busy port and the system
  cannot take down a non-functioning connection.
- **`mf_tick` and `mf_getstat` must not block the caller for a long time**,
  because they are called from the connection supervisor. A blocking tick/getstat
  stalls the connection supervisor, which is responsible for ticking and for
  tearing down dead connections.
- **Prefer distribution controller processes (ERTS >= 10.0)** over port drivers
  where possible, so logic lives in Erlang and you may avoid a custom driver
  entirely (e.g. distribution over UDP via `gen_udp`, with retransmissions in
  Erlang).
- **All driver operations must be non-blocking and account for every situation**;
  a non-stable driver affects/crashes the whole runtime.
- **Connection supervisor / acceptor should run at `max` priority** (use
  `dist_util:net_ticker_spawn_options()` → `[link, {priority, max}]` by default,
  configurable via the `net_ticker_spawn_options` kernel parameter).

## Relationship to net_kernel / distributed signals
- `net_kernel` is the owner of the distribution. It calls the distribution
  module's callbacks (`listen`, `accept`, `accept_connection`, `setup`, `close`,
  `select`) to manage connections to other nodes. The caller of `setup/5` and
  `accept_connection/5` is "a representative for `net_kernel`" (identified in the
  doc as `Kernel`).
- The distribution module **covers the protocol details from `net_kernel`** —
  `net_kernel` is agnostic to the carrier; it only sees the callback API.
- `net_kernel:start/1` brings up the distribution on a running node (useful for
  debugging); at boot, distribution starts when `-sname`/`-name` plus
  `-proto_dist` (and `-no_epmd` if needed) are on the command line / boot script.
- The acceptor informs `Kernel` of an accepted connection via
  `{accept, AcceptorPid, DistController, Family, Proto}`; `Kernel` replies with
  `{Kernel, controller, SupervisorPid}` (accepted) or
  `{Kernel, unsupported_protocol}` (fatal).
- The connection supervisor (spawned by `accept_connection/5` / `setup/5`)
  performs the handshake via `dist_util:handshake_other_started/1` /
  `dist_util:handshake_we_started/1`, then runs the connection supervisor loop
  for the life of the connection. It calls `mf_tick` and `mf_getstat` — hence
  the blocking caveat above directly affects `net_kernel`'s ticking/teardown.
- `epmd` (TCP port mapper daemon) is used for node discovery unless `-no_epmd` is
  given; `erl_epmd` registers the listen port and retrieves `Creation`.
- Distributed signals themselves (the actual message/signal format, capability
  flags `dflags`, atom cache, fragmentation) are defined in
  `erl_dist_protocol.html`, not here. This page is strictly about the *carrier*
  (transport) layer and the controller/module plumbing that `net_kernel` uses.

## Verbatim quotes
- "This section describes how to implement an alternative carrier protocol for
  the Erlang distribution. The distribution is normally carried by TCP/IP. Here
  is explained a method for replacing TCP/IP with another protocol."
- "As of ERTS version 10.0 support for distribution controller processes has
  been introduced. That is, the traffic over a distribution channel can be
  managed by a process instead of only by a port."
- "Note that there need to be exactly one distribution controller per connection.
  A process or port can only be distribution controller for one connection. The
  registration as distribution controller cannot be undone. It will stick until
  the distribution controller terminates."
- "The distribution module exposes an API that `net_kernel` calls in order to
  manage connections to other nodes."
- (Tick caveat) "It is important that the interface for sending ticks is not
  blocking. This implementation uses `erlang:port_control/3`, which does not
  block the caller. If `erlang:port_command` is used, use
  `erlang:port_command/3` and pass `[force]` as option list; otherwise the
  caller can be blocked indefinitely on a busy port and prevent the system from
  taking down a connection that is not functioning."
- (mf_tick) "It is of vital importance that this operation does not block the
  caller for a long time. This since it is called from the connection supervisor."
- (mf_getstat) "It is of vital importance that this operation does not block the
  caller for a long time. This since it is called from the connection supervisor."
- (Driver) "the callback functions can perform no blocking activity whatsoever.
  The callbacks are only to set up file descriptors for waiting and/or read/write
  available data. All I/O must be non-blocking."
- (Data delivery) "the data to pass over a connection needs to be delivered as
  is to the node on the receiving end in the exact same order, with no loss of
  data what so ever, as sent from the sending node."
- "The driver used for Erlang distribution is to implement a reliable, order
  maintaining, variable length packet-oriented protocol."

## Version notes
- Page meta: `erts v17.0.2`, OTP 29.0.2, major-vsn 29. ExDoc v0.40.3.
- Source ref: `github.com/erlang/otp/blob/OTP-29.0.2/erts/doc/guides/alt_dist.md`.
- Distribution controller processes introduced in ERTS 10.0 (OTP 21) — noted as
  the modern alternative to port drivers.
- The "Driver" section is flagged as old: "This section was written a long time
  ago. Most of it is still valid, but some things have changed since then."
- Copyright © 1996-2026 Ericsson AB.

## Discovered links
### Relevant (crawl later)
1. https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html — Erlang distribution protocol (dflags, signal format, atom cache, fragments); the signal-level counterpart to this carrier page.
2. https://www.erlang.org/doc/apps/kernel/net_kernel.html — net_kernel (start/1, distribution owner).
3. https://www.erlang.org/doc/apps/erts/alt_disco.html — How to Implement an Alternative Node Discovery (next page; companion to alt carrier).
4. https://www.erlang.org/doc/apps/erts/erl_driver.html — erl_driver API (driver writer reference).
5. https://www.erlang.org/doc/apps/erts/driver_entry.html — driver_entry (ErlDrvEntry struct).
6. https://www.erlang.org/doc/apps/kernel/kernel_app.html — kernel app params (net_ticker_spawn_options).
7. https://www.erlang.org/doc/apps/erts/erlang.html — erlang module (port_control/3, port_command/3, dist_ctrl_* BIFs, spawn_opt/4).

### Skipped
- https://www.erlang.org/doc/apps/erts/crash_dump.html — crash dump interpretation (previous page; off-topic).
- https://www.erlang.org/doc/apps/erts/epmd_cmd.html — epmd command (node discovery utility; tangential).
- assets/gen_tcp_dist.erl, assets/erl_uds_dist.erl — example source files (not crawlable doc pages).
- llms.txt, search.html, sidebar/CSS/JS assets.
