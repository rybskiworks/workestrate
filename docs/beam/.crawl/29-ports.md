# Crawl: system/ports.html
- seed_url: https://www.erlang.org/doc/system/ports.html
- canonical_url: https://www.erlang.org/doc/system/ports.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2; major-vsn 29)
- feeds_docs: ports-io.md

## Purpose
This page is the system-level introduction to **ports** and **port drivers**: the
basic Erlang mechanism for communicating with the external world (external OS
programs) through a byte-oriented interface. It defines the port owner / connected
process model, the message-based protocol to/from ports, and the BIFs that mirror
those messages. It is a *conceptual* page; the full option catalogue for
`erlang:open_port/2` (e.g. `{spawn_executable,_}`, `{fd,_}`, `use_stdio`, `stream`,
`eof`, `hide`, `exit_status`, `{args,_}`) lives in the `erlang` module reference
(ERTS), not here. The page also briefly introduces port drivers (linked-in C
drivers) and points to the ERTS driver docs.

## Key concepts (ports as processes, open_port forms, port options)
- **Ports** provide the basic mechanism for communication with the external world
  from Erlang's point of view: a byte-oriented interface to an external program.
  After creation, Erlang communicates by sending/receiving lists of bytes
  (including binaries).
- **Port owner / connected process**: the Erlang process that creates the port.
  All communication to and from the port must go through the port owner. If the
  port owner terminates, so does the port (and the external program, if written
  correctly).
- The external program resides in **another OS process**. By default it reads
  from stdin (fd 0) and writes to stdout (fd 1). The external program is
  expected to terminate when the port is closed.
- Creation: `open_port(PortName, PortSettings)` returns a port identifier `Port`.
  Messages can be sent to / received from a port identifier just like a PID.
  Port identifiers can be `link/1`-ed and `register/2`-ed like PIDs.
- `PortName` is usually `{spawn,Command}` where `Command` is the external program
  name. The external program runs **outside the Erlang workspace**, *unless* a
  port driver with the name `Command` is found — in which case that driver is
  started instead.
- `PortSettings` is a list of options. The list typically contains at least
  `{packet,N}` (N = 1, 2, or 4) specifying an N-byte length indicator preceding
  data exchanged with the external program. To use binaries instead of lists of
  bytes, include the option `binary`.
- Any process can send messages to a port, but the port owner must be identified
  in the message.
- Messages sent to ports are delivered **asynchronously**. (Before OTP 16 they
  were delivered synchronously — see Change note.)
- `Data` sent to a port must be an **I/O list**: a binary or a (possibly deep)
  list of binaries or integers in 0..255.

NOTE: This page does NOT enumerate `{spawn_executable,_}`, `{fd,_}`, `use_stdio`,
`stream`, `eof`, `hide`, `exit_status`, `{args,_}` — those are documented in the
`erlang:open_port/2` reference in ERTS, not on this system page. Only `{spawn,_}`,
`{packet,N}`, and `binary` are mentioned here.

## Port BIFs & signals
Messages that can be **sent to** a port (owner identified by `Pid`):
- `{Pid,{command,Data}}` — sends `Data` to the port.
- `{Pid,close}` — closes the port. Unless already closed, the port replies with
  `{Port,closed}` once all buffers are flushed and the port really closes.
- `{Pid,{connect,NewPid}}` — sets the port owner of `Port` to `NewPid`. Unless
  already closed, the port replies `{Port,connected}` to the old owner. The old
  owner stays linked; the new owner is NOT linked.

Messages **received from** a port (sent to the owning process):
- `{Port,{data,Data}}` — `Data` received from the external program.
- `{Port,closed}` — reply to `Port ! {Pid,close}`.
- `{Port,connected}` — reply to `Port ! {Pid,{connect,NewPid}}`.
- `{'EXIT',Port,Reason}` — if the port has terminated for some reason.

Equivalent BIFs (instead of message passing):
- `port_command(Port, Data)` — sends `Data` to the port. (Also `port_command/3`
  exists in the erlang module; not enumerated on this page.)
- `port_close(Port)` — closes the port.
- `port_connect(Port, NewPid)` — sets port owner to `NewPid`. Old owner `Pid`
  stays linked and must call `unlink(Port)` if not desired.
- `erlang:port_info(Port, Item)` — returns info as specified by `Item`.
  (`port_info/1` also exists; not separately listed here.)
- `erlang:ports()` — returns a list of all ports on the current node.

Additional BIFs applying to **port drivers**:
- `port_control/3`
- `erlang:port_call/3`

NOTE: This page does NOT describe `{Port,closed,Reason}` (3-tuple) or
`{Port,exit_status,Status}` signal shapes. The page only lists the 2-tuple
`{Port,closed}` reply and the `{'EXIT',Port,Reason}` signal. The
`exit_status` / 3-tuple closed signals are tied to options like `exit_status`
and `eof` documented in the `erlang:open_port/2` reference, not here.

## Linked-in drivers vs port drivers vs plain ports
- **Plain port**: external program running in a separate OS process,
  communicating over stdin/stdout byte streams; created via
  `open_port({spawn,Command}, ...)` when no driver named `Command` is found.
- **Port driver / linked-in driver**: a driver written in C according to certain
  principles and **dynamically linked into the Erlang runtime system**. From the
  Erlang programmer's point of view it looks like a port — hence "port driver".
  It is started when `open_port({spawn,Command}, ...)` finds a driver with that
  name.
- **Warning** (verbatim): an erroneous port driver causes the *entire* Erlang
  runtime system to leak memory, hang, or crash. (i.e. linked-in drivers run
  in-process and are not fault-isolated, unlike plain external-program ports.)
- Driver reference material: `erl_driver` (ERTS), `driver_entry` (ERTS),
  `erl_ddll` (Kernel).

## Contrast with os:cmd/1
This page does **not** mention `os:cmd/1`. (Conceptually: `os:cmd/1` is a
convenience wrapper that runs an OS command and returns its stdout as a string,
blocking the caller until completion; ports are the lower-level, asynchronous,
bidirectional, owner-linked mechanism. The contrast must be sourced from the
`os` module / `erlang` module docs, not this page.)

## Strict rules (ports count against limits; signal shapes)
- The port owner is the single channel for all communication to/from the port;
  if the owner terminates, the port (and external program, if correct) terminates.
- Any process may send to a port, but the owner `Pid` must be named in the
  message tuple.
- Messages to ports are asynchronous (since OTP 16).
- `Data` must be an I/O list (binary or deep list of binaries / 0..255 integers).
- On `{connect,NewPid}`: old owner stays linked, new owner is NOT linked — old
  owner must `unlink/1` if it wants to sever the link.
- Port identifiers support `link/1` and `register/2` like PIDs.
- Port drivers (linked-in C) are NOT fault-isolated: a buggy driver can leak
  memory, hang, or crash the whole runtime.
- NOTE: This page does not explicitly state that ports count against the
  process/ports limit or the `+P`/`+Q` flags; that detail lives in ERTS docs
  (erl, erts). The page does not enumerate port-limit accounting.

## Verbatim quotes
- "Ports provide the basic mechanism for communication with the external world,
  from Erlang's point of view. They provide a byte-oriented interface to an
  external program."
- "The Erlang process creating a port is said to be the port owner, or the
  connected process of the port. All communication to and from the port must go
  through the port owner. If the port owner terminates, so does the port (and the
  external program, if it is written correctly)."
- "The external program resides in another OS process. By default, it reads from
  standard input (file descriptor 0) and writes to standard output (file
  descriptor 1). The external program is to terminate when the port is closed."
- "It is possible to write a driver in C according to certain principles and
  dynamically link it to the Erlang runtime system. The linked-in driver looks
  like a port from the Erlang programmer's point of view and is called a port
  driver."
- "An erroneous port driver causes the entire Erlang runtime system to leak
  memory, hang, or crash."
- "PortName is usually a tuple {spawn,Command}, where the string Command is the
  name of the external program. The external program runs outside the Erlang
  workspace, unless a port driver with the name Command is found. If Command is
  found, that driver is started."
- "Messages sent to ports are delivered asynchronously." / "Before Erlang/OTP 16,
  messages to ports were delivered synchronously."
- "Data must be an I/O list. An I/O list is a binary or a (possibly deep) list of
  binaries or integers in the range 0 through 255."
- Send-to-port messages: `{Pid,{command,Data}}`, `{Pid,close}`,
  `{Pid,{connect,NewPid}}`.
- Receive-from-port messages: `{Port,{data,Data}}`, `{Port,closed}`,
  `{Port,connected}`, `{'EXIT',Port,Reason}`.

## Version notes
- Page metadata: Erlang System Documentation v29.0.2; major-vsn 29; OTP 29.0.2.
- Source: github.com/erlang/otp blob OTP-29.0.2 system/doc/reference_manual/ports.md.
- Built with ExDoc v0.40.3.
- Change note: before Erlang/OTP 16, messages to ports were delivered
  synchronously; now asynchronous.

## Discovered links
### Relevant (crawl later)
- ../system/tutorial.html — Interoperability Tutorial (port/port driver examples)
- ../apps/erts/erlang.html — `erlang` module (open_port/2, port BIFs full options)
- ../apps/erts/erl_driver.html — erl_driver (C driver API)
- ../apps/erts/driver_entry.html — driver_entry (driver entry struct)
- ../apps/kernel/erl_ddll.html — erl_ddll (dynamic driver loading)
- ../apps/erts/erlang.html#open_port/2 — open_port/2 anchor (full option catalogue)
- ../apps/erts/erlang.html#port_command/2 — port_command/2
- ../apps/erts/erlang.html#port_close/1 — port_close/1
- ../apps/erts/erlang.html#port_connect/2 — port_connect/2
- ../apps/erts/erlang.html#port_info/2 — port_info/2
- ../apps/erts/erlang.html#ports/0 — ports/0
- ../apps/erts/erlang.html#port_control/3 — port_control/3
- ../apps/erts/erlang.html#port_call/3 — port_call/3
- ../apps/erts/erlang.html#link/1 — link/1
- ../apps/erts/erlang.html#register/2 — register/2
- ../apps/erts/erlang.html#unlink/1 — unlink/1

### Skipped
- code_loading.html (prev page: Compilation and Code Loading — out of scope)
- efficiency_guide.html (next page: Introduction to efficiency guide)
- search.html, llms.txt, ePub download, index.html (site chrome)
- github.com/erlang/otp source link (already captured version)
- github.com/elixir-lang/ex_doc, erlang.org, ericsson.com (external chrome)
