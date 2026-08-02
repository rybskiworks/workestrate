# Crawl: design_principles.html

- seed_url: https://www.erlang.org/doc/system/design_principles.html
- canonical_url: https://www.erlang.org/doc/system/design_principles.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2
- feeds_docs: overview.md, otp-behaviours.md, supervision.md, applications.md, releases.md

## Purpose
The OTP Design Principles define how to structure Erlang code in terms of processes, modules, and directories. This overview page introduces the foundational concepts (supervision trees, behaviours, applications, releases, release handling) that the rest of the design-principles documentation expands upon. It is the entry point / index for the design_principles chapter.

## Key concepts
- Supervision tree: a process-structuring model based on workers and supervisors.
- Workers: processes that perform computations and other actual work.
- Supervisors: processes that monitor workers and can restart them if something goes wrong.
- Supervision tree is a hierarchical arrangement of code into supervisors and workers, enabling fault-tolerant software.
- Behaviours: formalizations of common process patterns, dividing code into a generic part (behaviour module, part of Erlang/OTP) and a specific part (callback module).
- Callback module: user-implemented module exporting a pre-defined set of callback functions.
- Applications: Erlang/OTP components implementing specific functionality (e.g. Mnesia, Debugger). Applies to both program structure (processes) and directory structure (modules).
- Library application: simplest application with no processes, just a collection of functional modules (e.g. STDLIB).
- Minimal Erlang/OTP system = Kernel + STDLIB.
- Release: a complete system made from a subset of Erlang/OTP applications plus a set of user-specific applications.
- Release handling: upgrading and downgrading between different versions of a release in a (possibly) running system.

## Strict rules / invariants
- A behaviour is split into a generic behaviour module (provided by OTP) and a specific callback module (user-supplied) that must export a pre-defined set of callback functions.
- The compiler understands the module attribute `-behaviour(Behaviour)` and issues warnings about missing callback functions.
- Code written without behaviours can be more efficient, but at the expense of generality; using behaviours enables consistent management of all applications in the system.
- Using behaviours makes code easier to read and understand across programmers; improvised structures are always harder to understand.
- An application with processes is easiest implemented as a supervision tree using the standard behaviours.
- The application concept applies both to program structure (processes) and directory structure (modules).

## Examples
- `ch1.erl` — a plain-Erlang (non-behaviour) server managing "channels" with `alloc/0` and `free/1`. Lesson: shows the raw server loop pattern (spawn, register, receive loop) that behaviours formalize.
- `server.erl` — the generic server part extracted from `ch1`, reusable for many servers via `call/2`, `cast/2`, and a callback module (`Mod:init/0`, `Mod:handle_call/2`, `Mod:handle_cast/2`). Lesson: demonstrates the generic/specific split that underpins behaviours; corresponds (greatly simplified) to `gen_server`.
- `ch2.erl` — the callback module for `server.erl`, exposing the same `alloc/0`/`free/1` interface but delegating to the generic `server`. Lesson: shows how the server name and message protocol are hidden from clients, and how `server` can be extended without changing callback modules.
- `channels/0`, `alloc/1`, `free/2` — a sample implementation of the channel store (lists-based). Lesson: illustrative only; a realistic implementation must handle edge cases like running out of channels.
- `chs3.erl` — minimal module declaring `-behaviour(gen_server).` to demonstrate the compiler warning for a missing callback (`handle_call/3`). Lesson: the `-behaviour` attribute triggers static checking of required callbacks.

## Behaviour / callback details
- Module attribute: `-behaviour(Behaviour).` (e.g. `-behaviour(gen_server).`). The compiler issues warnings for missing callback functions.
- Standard Erlang/OTP behaviours:
  - `gen_server` — for implementing the server of a client-server relation. (callbacks include `init/1`, `handle_call/3`, `handle_cast/2`, etc.; the page's toy `server` uses `init/0`, `handle_call/2`, `handle_cast/2` as a simplified analogue)
  - `gen_statem` — for implementing state machines.
  - `gen_event` — for implementing event handling functionality.
  - `supervisor` — for implementing a supervisor in a supervision tree.
- The example generic `server` module corresponds (greatly simplified) to `gen_server`; its callback arities (`init/0`, `handle_call/2`, `handle_cast/2`) are simplified and differ from the real `gen_server` arities (e.g. real `handle_call/3`).

## Verbatim quotes
1. "The OTP Design Principles define how to structure Erlang code in terms of processes, modules, and directories." — (Overview / intro)
2. "A basic concept in Erlang/OTP is the supervision tree. This is a process structuring model based on the idea of workers and supervisors." — Supervision Trees
3. "Workers are processes that perform computations and other actual work." — Supervision Trees
4. "Supervisors are processes that monitor workers. A supervisor can restart a worker if something goes wrong." — Supervision Trees
5. "The supervision tree is a hierarchical arrangement of code into supervisors and workers, which makes it possible to design and program fault-tolerant software." — Supervision Trees
6. "Behaviours are formalizations of these common patterns. The idea is to divide the code for a process in a generic part (a behaviour module) and a specific part (a callback module)." — Behaviours
7. "The behaviour module is part of Erlang/OTP. To implement a process such as a supervisor, the user only needs to implement the callback module, which is to export a pre-defined set of functions, the callback functions." — Behaviours
8. "The code in `server` can be reused to build many different servers." — Behaviours
9. "The server name, in this example the atom `ch2`, is hidden from the users of the client functions. This means that the name can be changed without affecting them." — Behaviours
10. "The protocol (messages sent to and received from the server) is also hidden. This is good programming practice and allows one to change the protocol without changing the code using the interface functions." — Behaviours
11. "The functionality of `server` can be extended without having to change `ch2` or any other callback module." — Behaviours
12. "Code written without using behaviours can be more efficient, but the increased efficiency is at the expense of generality. The ability to manage all applications in the system in a consistent manner is important." — Behaviours
13. "Using behaviours also makes it easier to read and understand code written by other programmers. Improvised programming structures, while possibly more efficient, are always more difficult to understand." — Behaviours
14. "The `server` module corresponds, greatly simplified, to the Erlang/OTP behaviour `gen_server`." — Behaviours
15. "The compiler understands the module attribute `-behaviour(Behaviour)` and issues warnings about missing callback functions." — Behaviours
16. "Components are with Erlang/OTP terminology called applications." — Applications
17. "The minimal system based on Erlang/OTP consists of the following two applications: Kernel - Functionality necessary to run Erlang; STDLIB - Erlang standard libraries." — Applications
18. "The application concept applies both to program structure (processes) and directory structure (modules)." — Applications
19. "The simplest applications do not have any processes, but consist of a collection of functional modules. Such an application is called a library application. An example of a library application is STDLIB." — Applications
20. "An application with processes is easiest implemented as a supervision tree using the standard behaviours." — Applications
21. "A release is a complete system made from a subset of Erlang/OTP applications and a set of user-specific applications." — Releases
22. "Release handling is upgrading and downgrading between different versions of a release, in a (possibly) running system." — Release Handling

## Version notes
- Page metadata: Erlang System Documentation v29.0.2; sidebar shows "OTP 29.0.2"; meta `major-vsn` = 29.
- Generated with ExDoc v0.40.3.
- Source link points to GitHub `erlang/otp` tag `OTP-29.0.2` at `system/doc/design_principles/design_principles.md`.
- Canonical URL: https://www.erlang.org/doc/system/design_principles.html (no redirect; final URL == seed URL).
- Copyright © 1996-2026 Ericsson AB.
- No deprecation/version-specific callouts in the body text; content is version-stable conceptual overview.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/system/gen_server_concepts.html — gen_server behaviour (standard behaviour; next page in chapter)
- https://www.erlang.org/doc/system/statem.html — gen_statem behaviour (standard behaviour; state machines)
- https://www.erlang.org/doc/system/events.html — gen_event behaviour (standard behaviour; event handling)
- https://www.erlang.org/doc/system/sup_princ.html — supervisor behaviour (standard behaviour; supervision principles)
- https://www.erlang.org/doc/system/applications.html — Applications chapter (how to program applications)
- https://www.erlang.org/doc/system/release_structure.html — Releases chapter (how to program releases)
- https://www.erlang.org/doc/system/release_handling.html — Release Handling chapter (upgrading/downgrading)
- https://www.erlang.org/doc/system/create_target.html — Creating and Upgrading a Target System (System Principles)

### Skipped
- https://www.erlang.org/doc/system/design_principles.md — raw markdown mirror of this same page (duplicate content)
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/design_principles.md#L1 — source view on GitHub (out of scope for corpus)
- https://www.erlang.org/doc/system/misc.html — "Support, Compatibility, Deprecations, and Removal" (previous page; off-topic for design principles corpus)
- https://www.erlang.org/doc/system/search.html — search page
- https://www.erlang.org/doc/system/llms.txt — llms.txt index (not a content page)
- https://www.erlang.org/doc/system/Erlang%20System%20Documentation.epub — epub download
- https://github.com/elixir-lang/ex_doc — ExDoc tooling
- https://www.erlang.org — Erlang homepage
- https://www.ericsson.com — Ericsson homepage
