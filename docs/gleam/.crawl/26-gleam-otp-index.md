# Crawl: hexdocs.pm/gleam_otp/ (index)
- seed_url: https://hexdocs.pm/gleam_otp/
- canonical_url: https://gleam-otp.hexdocs.pm/
- family: Gleam core package (gleam_otp)
- fetch: 200
- gleam_otp_version: v1.2.0
- feeds_docs: otp-actors-and-supervision.md

## Purpose
Gleam OTP provides fault-tolerant multi-core programs with OTP, the BEAM actor
framework. It is the standard Gleam library for typed actors (gen_server-like
processes) and supervision trees, wrapping Erlang/OTP in a type-safe Gleam API.

Stated primary goals:
- Full type safety of actors and messages.
- Compatibility with Erlang's OTP actor framework.
- Fault tolerance and self-healing through supervisors.
- Equivalent performance to Erlang's OTP.

Install: `gleam add gleam_otp@1`

## Module list + one-line purpose each
- `gleam/otp/actor` — The most commonly used process type; a typed
  gen_server-like actor that handles OTP system messages automatically
  (debugging/tracing). Built on `gleam/erlang/process`.
- `gleam/otp/supervision` — Core supervision abstractions: the `child`/`Child`
  types and child spec builders used by the supervisor modules.
- `gleam/otp/static_supervisor` — A supervisor with a fixed/static set of
  children, started once at startup (one_for_one-style restart strategy).
- `gleam/otp/factory_supervisor` — A supervisor that can dynamically start
  children on demand (factory/dynamic supervisor pattern).
- `gleam/otp/system` — Helpers for interacting with the OTP system layer
  (system messages, suspend/resume, etc.).
- `gleam/otp/port` — Wraps Erlang ports for communication with external
  programs / byte-oriented I/O.

Note: the lowest-level `process` module (Subject, Selector, send, receive,
monitor) lives in the separate `gleam_erlang` package
(`gleam/erlang/process`), not in `gleam_otp` itself.

## Conceptual model (typed actors/subjects/selectors; typed supervisors)
- **Typed actors**: An `actor` is started from an initial state and a message
  handler `fn(state, message) -> Next(state, message)`. The message type is
  statically typed, so `actor.send(subject, msg)` and `actor.call(subject, msg)`
  are type-checked at compile time. This is the typed analogue of Erlang's
  `gen_server`.
- **Subjects**: A `Subject(msg)` (from `gleam/erlang/process`) is a typed
  reference to a process's mailbox — the typed replacement for a raw Erlang PID.
  Actors expose a `Subject` for sending messages; `actor.call` uses a fresh
  reply `Subject` to receive a typed response.
- **Selectors**: A `Selector(msg)` (from `gleam/erlang/process`) lets a process
  selectively receive from multiple typed subjects, the typed analogue of
  Erlang's selective `receive`.
- **Typed supervisors**: `static_supervisor` and `factory_supervisor` build
  supervision trees from typed `child`/`Child` specs (defined in
  `gleam/otp/supervision`). A child spec describes how to start, restart, and
  shut down a child process. Supervisors can start other supervisors, forming a
  hierarchical supervision tree for fault tolerance and self-healing.
- **Next / continue**: Actors progress via `actor.continue(state)` (or
  `actor.stop`) returned from the message handler, the typed equivalent of a
  gen_server loop iteration.

## Relationship to BEAM OTP (→ docs/beam/)
`gleam_otp` is a thin, type-safe Gleam veneer over the BEAM's native OTP
behaviours; the underlying semantics are documented in `docs/beam/`:

- `gleam/otp/actor` ↔ Erlang `gen_server` (see `docs/beam/` gen_server material):
  handles OTP system messages, enables `sys` debugging/tracing, same
  request/reply (`call`) and cast (`send`) semantics.
- `gleam/otp/supervision` + `static_supervisor` ↔ Erlang `supervisor` behaviour
  with `one_for_one` child specs (see `docs/beam/` supervision material): child
  specs, restart strategies, shutdown values, intensity/period.
- `gleam/otp/factory_supervisor` ↔ Erlang `supervisor` with `simple_one_for_one`
  / `dynamic_supervisor` semantics (dynamic child start on demand).
- `gleam/otp/system` ↔ OTP `sys` module and system messages (suspend, resume,
  get_state, replace_state) — see `docs/beam/` observability/sys material.
- `gleam/otp/port` ↔ Erlang ports (`erlang:open_port`).
- `gleam/erlang/process` (Subject/Selector/send/receive/monitor) ↔ BEAM process
  primitives, links, and monitors (see `docs/beam/` processes material).

Notable limitation (per README): actors do not yet support all OTP system
messages, so some OTP debugging APIs may be incomplete; unsupported system
messages are discarded. Not all Erlang/OTP functionality is included — only
what can be represented type-safely.

## Version notes
- Current crawled version: **v1.2.0** (from `<title>gleam_otp · v1.2.0</title>`
  and `#project-version`).
- Docs tooling: HexDocs with ExDoc-style layout; sidebar lists Modules + Pages
  (README) + external Links (Website, Sponsor, Repository, Hex).
- Install major-version pin: `gleam add gleam_otp@1`.
- ExDoc version referenced in asset URLs: `1.13.0-rc1`.

## Discovered links
### Relevant (crawl later)
- https://gleam-otp.hexdocs.pm/gleam/otp/actor.html
- https://gleam-otp.hexdocs.pm/gleam/otp/supervision.html
- https://gleam-otp.hexdocs.pm/gleam/otp/static_supervisor.html
- https://gleam-otp.hexdocs.pm/gleam/otp/factory_supervisor.html
- https://gleam-otp.hexdocs.pm/gleam/otp/system.html
- https://gleam-otp.hexdocs.pm/gleam/otp/port.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html (cross-package:
  Subject/Selector/process primitives — feeds gleam_erlang crawl, not gleam_otp)

### Skipped
- https://gleam.run/ (website, out of scope)
- https://github.com/sponsors/lpil (sponsor)
- https://github.com/gleam-lang/otp (repository source)
- https://hex.pm/packages/gleam_otp (Hex package page)
- ./index.html (README page, same content as crawled)
- CSS/JS/SVG asset URLs
