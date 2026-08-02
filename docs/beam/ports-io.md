# Ports and I/O

## Purpose

Ports are the BEAM's basic mechanism for communicating with the external world:
a byte-oriented interface to an external program running in a separate OS
process. This doc covers the port owner/connected-process model, the
message-based protocol to and from ports, the equivalent BIFs, the distinction
between plain ports, port drivers (linked-in C drivers), and the contrast with
`os:cmd/1`.

## Sources used

- Crawl `29-ports.md` — system `ports.html` — https://www.erlang.org/doc/system/ports.html
- Crawl `17-erlang-bifs.md` — erts `erlang.html` — https://www.erlang.org/doc/apps/erts/erlang.html

> **Coverage note:** The full `erlang:open_port/2` option catalogue
> (`{spawn_executable,_}`, `{fd,_}`, `use_stdio`, `stream`, `eof`, `hide`,
> `exit_status`, `{args,_}`) was NOT captured in crawl 17 (port-program BIFs
> were deliberately skipped) and is NOT on the system `ports.html` page (crawl
> 29). Only `{spawn,Command}`, `{packet,N}`, and `binary` are sourced here.
> Revisit: crawl the `erlang:open_port/2` reference for the complete option list.

## Core guidance

### Ports as processes

"Ports provide the basic mechanism for communication with the external world,
from Erlang's point of view. They provide a byte-oriented interface to an
external program." After creation, Erlang communicates by sending/receiving
lists of bytes (including binaries). Port identifiers can be `link/1`-ed and
`register/2`-ed like PIDs; messages can be sent to / received from a port
identifier just like a PID.

Port owner / connected process:
- "The Erlang process creating a port is said to be the port owner, or the
  connected process of the port. All communication to and from the port must go
  through the port owner. If the port owner terminates, so does the port (and the
  external program, if it is written correctly)."
- The external program "resides in another OS process. By default, it reads from
  standard input (file descriptor 0) and writes to standard output (file
  descriptor 1). The external program is to terminate when the port is closed."
- Any process can send messages to a port, but the owner `Pid` must be named in
  the message tuple.
- "Messages sent to ports are delivered asynchronously." (Before OTP 16 they were
  synchronous.)

### open_port forms and options

Creation: `open_port(PortName, PortSettings)` returns a port identifier `Port`.
- `PortName` is usually `{spawn,Command}` where `Command` is the external
  program name. "The external program runs outside the Erlang workspace, unless a
  port driver with the name `Command` is found. If `Command` is found, that
  driver is started."
- `PortSettings` is a list of options. The page mentions:
  - `{packet,N}` (N = 1, 2, or 4) — N-byte length indicator preceding data.
  - `binary` — use binaries instead of lists of bytes.
- `Data` sent to a port must be an I/O list: "a binary or a (possibly deep) list
  of binaries or integers in the range 0 through 255."

### Port signals

Messages sent TO a port (owner identified by `Pid`):
- `{Pid,{command,Data}}` — sends `Data` to the port.
- `{Pid,close}` — closes the port. Unless already closed, the port replies with
  `{Port,closed}` once all buffers are flushed.
- `{Pid,{connect,NewPid}}` — sets the port owner to `NewPid`. The port replies
  `{Port,connected}` to the old owner. "The old owner stays linked; the new owner
  is NOT linked."

Messages received FROM a port (sent to the owning process):
- `{Port,{data,Data}}` — `Data` received from the external program.
- `{Port,closed}` — reply to `{Pid,close}`.
- `{Port,connected}` — reply to `{Pid,{connect,NewPid}}`.
- `{'EXIT',Port,Reason}` — if the port has terminated.

> **Note:** The 3-tuple `{Port,closed,Reason}` and `{Port,exit_status,Status}`
> signal shapes are tied to options like `exit_status` and `eof` documented in
> the `erlang:open_port/2` reference (not crawled here; see coverage note).

### Equivalent BIFs

- `port_command(Port, Data)` — sends `Data` to the port (`port_command/3` also
  exists in the `erlang` module).
- `port_close(Port)` — closes the port.
- `port_connect(Port, NewPid)` — sets port owner to `NewPid`. Old owner `Pid`
  stays linked and must call `unlink(Port)` if not desired.
- `erlang:port_info(Port, Item)` / `port_info/1` — returns info as specified by
  `Item`.
- `erlang:ports()` — returns a list of all ports on the current node.
- Port-driver BIFs: `port_control/3`, `erlang:port_call/3`.

### Linked-in drivers vs port drivers vs plain ports

- **Plain port**: external program in a separate OS process, communicating over
  stdin/stdout byte streams; created via `open_port({spawn,Command}, ...)` when
  no driver named `Command` is found.
- **Port driver / linked-in driver**: "a driver written in C according to certain
  principles and dynamically link it to the Erlang runtime system. The linked-in
  driver looks like a port from the Erlang programmer's point of view and is
  called a port driver." Started when `open_port({spawn,Command}, ...)` finds a
  driver with that name.
- **Warning (verbatim):** "An erroneous port driver causes the entire Erlang
  runtime system to leak memory, hang, or crash." Linked-in drivers run in-process
  and are NOT fault-isolated, unlike plain external-program ports.
- Driver reference material: `erl_driver` (ERTS), `driver_entry` (ERTS),
  `erl_ddll` (Kernel).

### Contrast with os:cmd/1

The crawled `ports.html` page does NOT mention `os:cmd/1`. Conceptually,
`os:cmd/1` is a convenience wrapper that runs an OS command and returns its
stdout as a string, blocking the caller until completion; ports are the
lower-level, asynchronous, bidirectional, owner-linked mechanism. The contrast
must be sourced from the `os` module / `erlang` module docs (not crawled here).

## Practical rules

- The port owner is the single channel for all communication; if the owner
  terminates, the port (and external program, if correct) terminates.
- Any process may send to a port, but the owner `Pid` must be named in the
  message tuple.
- Messages to ports are asynchronous (since OTP 16).
- `Data` must be an I/O list (binary or deep list of binaries / 0..255 integers).
- On `{connect,NewPid}`: old owner stays linked, new owner is NOT linked — old
  owner must `unlink/1` to sever the link.
- Port identifiers support `link/1` and `register/2` like PIDs.
- Port drivers (linked-in C) are NOT fault-isolated: a buggy driver can leak
  memory, hang, or crash the whole runtime.

## Review checklist

- [ ] Is the port owner a long-lived, supervised process?
- [ ] Is `{packet,N}` used to frame messages (avoiding stream reassembly bugs)?
- [ ] Is the owner trapping exits if it must survive port termination?
- [ ] After `port_connect/2`, does the old owner `unlink(Port)` if unintended?
- [ ] Is `Data` always an I/O list (not an arbitrary term)?
- [ ] For linked-in drivers: is the driver audited for memory safety?

## Implementation checklist

- [ ] Create ports with `open_port({spawn,Command}, [{packet,N}, binary])`.
- [ ] Send via `port_command/2` (or `{Pid,{command,Data}}`).
- [ ] Handle `{Port,{data,Data}}`, `{Port,closed}`, `{'EXIT',Port,Reason}`.
- [ ] Close with `{Pid,close}` and wait for `{Port,closed}` before assuming done.
- [ ] Register the port if other processes need a stable name.
- [ ] Prefer plain ports over linked-in drivers unless latency demands it.

## Runtime / debugging checklist

- [ ] `erlang:ports()` to enumerate all ports on the node.
- [ ] `erlang:port_info(Port)` / `port_info(Port, Item)` to inspect a port.
- [ ] Check the owner is alive (`is_process_alive/1` on the connected pid).
- [ ] Look for `{'EXIT',Port,Reason}` in the owner's mailbox.
- [ ] `port_control/3` / `port_call/3` for driver-level diagnostics.

## Validation hooks

- After `open_port`, assert the returned term is a port (`is_port/1`).
- After `{Pid,close}`, assert `{Port,closed}` is received within a timeout.
- After `port_connect/2`, assert the new owner receives `{Port,connected}`.
- Assert `erlang:ports()` count is within expected bounds in tests.

## Examples

Open a port with length-prefixed binary framing:
```erlang
Port = open_port({spawn, "cat"}, [{packet, 2}, binary]),
Port ! {self(), {command, <<"hello">>}},
receive
    {Port, {data, Data}} -> ok = binary_to_list(Data)
end,
Port ! {self(), close},
receive
    {Port, closed} -> ok
end.
```

Transfer ownership (old owner stays linked):
```erlang
Port ! {self(), {connect, NewOwner}},
receive
    {Port, connected} -> unlink(Port)
end.
```

## Common mistakes

- Sending a non-I/O-list term to a port (badarg).
- Forgetting to `unlink/1` after `port_connect/2` (stale exit signals).
- Assuming `{Pid,close}` synchronously closes the port (it replies
  `{Port,closed}` only after buffers flush).
- Using a linked-in driver as if it were fault-isolated (it is not).
- Not trapping exits in the owner when the external program's death must be
  handled rather than propagated.

## Strict vs contextual guidance

Strict:
- All communication goes through the port owner.
- `Data` must be an I/O list.
- Old owner stays linked after `{connect,NewPid}`; new owner is not linked.
- Linked-in driver bugs crash/hang/leak the whole VM.

Contextual:
- `{packet,N}` vs `stream` framing — choose by protocol.
- `binary` vs list data — choose by size and consumer.
- Plain port vs linked-in driver — plain ports for safety, drivers for latency.

## Policy decisions for individual repos

- Whether linked-in drivers are permitted (and the review bar for them).
- Default framing (`{packet,4}` vs `stream` + custom framing).
- Whether port owners must be supervised and trap exits.
- `os:cmd/1` vs ports for one-shot shell commands (not crawled; decide locally).

## Related docs

- `processes-and-messages.md` — sending messages to ports.
- `links-monitors-and-exits.md` — port links and exit signals.
- `nifs.md` — NIFs vs port drivers trade-offs.
- `runtime-debugging.md` — `port_info`/`ports/0` introspection.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
