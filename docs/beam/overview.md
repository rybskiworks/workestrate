# BEAM/OTP Overview

## Purpose
Establish the BEAM/OTP mental model and design principles: how Erlang/OTP
structures code into processes, modules, and directories via supervision
trees, behaviours, applications, and releases. This is the conceptual entry
point for the `docs/beam/` corpus.

## Sources used
- Crawl file: `crawl/01-design-principles.md`
- Canonical URL cited: `https://www.erlang.org/doc/system/design_principles.html`
- Documented OTP version: OTP 29.0.2 (ExDoc v0.40.3)

## Core guidance
The OTP Design Principles "define how to structure Erlang code in terms of
processes, modules, and directories." (Overview / intro)

The foundational concept is the **supervision tree**: "A basic concept in
Erlang/OTP is the supervision tree. This is a process structuring model based
on the idea of workers and supervisors." (Supervision Trees)

- **Workers** — "processes that perform computations and other actual work."
- **Supervisors** — "processes that monitor workers. A supervisor can restart
  a worker if something goes wrong."
- "The supervision tree is a hierarchical arrangement of code into supervisors
  and workers, which makes it possible to design and program fault-tolerant
  software."

**Behaviours** formalize common process patterns: "The idea is to divide the
code for a process in a generic part (a behaviour module) and a specific part
(a callback module)." (Behaviours) "The behaviour module is part of
Erlang/OTP. To implement a process such as a supervisor, the user only needs
to implement the callback module, which is to export a pre-defined set of
functions, the callback functions."

**Applications** — "Components are with Erlang/OTP terminology called
applications." (Applications) The application concept "applies both to program
structure (processes) and directory structure (modules)."

**Minimal system** — "The minimal system based on Erlang/OTP consists of the
following two applications: Kernel - Functionality necessary to run Erlang;
STDLIB - Erlang standard libraries."

**Library application** — "The simplest applications do not have any processes,
but consist of a collection of functional modules. Such an application is
called a library application. An example of a library application is STDLIB."

**Release** — "A release is a complete system made from a subset of Erlang/OTP
applications and a set of user-specific applications." (Releases)

**Release handling** — "Release handling is upgrading and downgrading between
different versions of a release, in a (possibly) running system." (Release
Handling)

## Practical rules
1. Structure every application with processes as a **supervision tree** using
   the standard behaviours: "An application with processes is easiest
   implemented as a supervision tree using the standard behaviours."
2. Prefer behaviours over hand-rolled process loops. "Code written without
   using behaviours can be more efficient, but the increased efficiency is at
   the expense of generality. The ability to manage all applications in the
   system in a consistent manner is important."
3. Use behaviours for readability across teams: "Using behaviours also makes it
   easier to read and understand code written by other programmers. Improvised
   programming structures, while possibly more efficient, are always more
   difficult to understand."
4. Declare `-behaviour(Behaviour).` so the compiler warns on missing callbacks:
   "The compiler understands the module attribute `-behaviour(Behaviour)` and
   issues warnings about missing callback functions."
5. Hide server names and message protocols behind client interface functions:
   "The server name ... is hidden from the users of the client functions" and
   "The protocol (messages sent to and received from the server) is also
   hidden. This ... allows one to change the protocol without changing the code
   using the interface functions."
6. Treat the application concept as both a process structure and a directory
   structure — they are the same notion.

## Review checklist
- [ ] Is the system organized as a supervision tree (workers under
  supervisors)?
- [ ] Are standard behaviours used instead of improvised server loops?
- [ ] Is `-behaviour(Behaviour).` declared in every callback module?
- [ ] Are server names and message protocols hidden behind interface functions?
- [ ] Is the application defined both as processes and as a directory layout?
- [ ] Is the release composed from a subset of OTP apps plus user apps?

## Implementation checklist
- [ ] Identify workers (do work) vs supervisors (monitor/restart workers).
- [ ] Pick the standard behaviour that matches each process's pattern.
- [ ] Write the callback module exporting the pre-defined callback set.
- [ ] Group callback modules into an application (.app resource).
- [ ] Compose applications into a release for deployment.
- [ ] Plan release handling (upgrade/downgrade) if running systems must evolve.

## Runtime / debugging checklist
- [ ] Confirm the minimal system boots: Kernel + STDLIB.
- [ ] Verify the supervision tree is hierarchical (supervisors restart workers).
- [ ] Check that behaviours give consistent management across all applications.

## Validation hooks
- Compiler warnings for missing callbacks (from `-behaviour/1`).
- Application start/stop via the application controller.
- Release creation and upgrade/downgrade via release-handling tooling.

## Examples
The design-principles chapter illustrates the generic/specific split with a toy
`server` module: "The `server` module corresponds, greatly simplified, to the
Erlang/OTP behaviour `gen_server`." A callback module (`ch2.erl`) delegates to
`server` while exposing the same `alloc/0`/`free/1` interface, hiding the
server name and protocol from clients.

```erlang
%% A callback module declaring its behaviour (compiler checks callbacks):
-module(chs3).
-behaviour(gen_server).
%% The compiler warns if a required callback (e.g. handle_call/3) is missing.
```

## Common mistakes
- Writing an improvised server loop instead of using a standard behaviour —
  loses consistent management and readability.
- Forgetting `-behaviour(Behaviour).` — no compiler check for missing
  callbacks.
- Exposing the server name or message protocol to clients — prevents changing
  them without touching callers.
- Treating "application" as only a directory layout or only a process group —
  it is both.

## Strict vs contextual guidance
- **Strict:** Use supervision trees + standard behaviours for any application
  with processes. Declare `-behaviour/1`. Hide names/protocols behind
  interfaces.
- **Contextual:** Code without behaviours "can be more efficient" — acceptable
  only when the generality/consistency trade-off is explicitly justified.

## Policy decisions for individual repos
- Whether to allow non-behaviour special processes (see `otp-behaviours.md` and
  `proc-lib-and-sys.md`) and under what review gate.
- Whether library applications (no processes) are permitted alongside
  process-bearing applications.
- Release-handling strategy: appup/relup vs. full restart.

## Related docs
- [otp-behaviours.md](otp-behaviours.md)
- [supervision.md](supervision.md)
- [applications.md](applications.md)
- [releases.md](releases.md)
- [gen-server.md](gen-server.md)
- [proc-lib-and-sys.md](proc-lib-and-sys.md)

## Related skills
- `beam-supervision`
- `beam-gen-server`
- `beam-applications-releases`
