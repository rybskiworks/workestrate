# Crawl: hexdocs.pm/gleam_erlang/ (index)
- seed_url: https://hexdocs.pm/gleam_erlang/
- canonical_url: https://gleam-erlang.hexdocs.pm/
- family: Gleam core package (gleam_erlang)
- fetch: 200
- gleam_erlang_version: v1.3.0
- feeds_docs: erlang-interop.md, otp-actors-and-supervision.md

## Purpose
gleam_erlang provides types and functions for Gleam programs running on the
Erlang runtime (BEAM/OTP). It is the lowest-level Gleam core package for
Erlang-targeted code: typed wrappers over Erlang/OTP primitives that the
standard library and `gleam_otp` build upon.

Stated on the index page:
- "Types and functions for programs running on Erlang!"
- Install: `gleam add gleam_erlang@1`
- "This library requires OTP 27.0 or higher."

The package is the typed message-passing and runtime-identity layer: it
exposes processes, PIDs, messages, selectors, atoms, charlists, nodes, ports,
references, and application metadata as Gleam types rather than raw Erlang
terms. `gleam_otp`'s actors and supervisors are built on top of
`gleam/erlang/process` (subjects, selectors, PIDs).

## Module list + one-line purpose
- `gleam/erlang/application` — Read OTP application metadata (loaded
  applications, their environment, and start/permanent type) from Gleam.
- `gleam/erlang/atom` — Typed wrapper around Erlang atoms; creation and
  (where safe) conversion to/from strings. Atoms are the symbolic identity
  tokens of OTP (used in messages, refs, node names, supervisor child ids).
- `gleam/erlang/charlist` — Typed wrapper around Erlang charlists (lists of
  integers / IO-lists), the native string type of the BEAM before binaries.
- `gleam/erlang/node` — Distributed-node primitives: the local node name,
  remote nodes, and EPMD-style distributed Erlang identity.
- `gleam/erlang/port` — Typed wrapper around Erlang ports (channels to
  external/linked-in programs and drivers), the BEAM's external-process I/O
  primitive.
- `gleam/erlang/process` — The typed message-passing core: PIDs, `Subject`,
  `Selector`, `spawn`, `send`, `receive`/`select`, monitors, links, exits,
  timers, and process naming. Foundation for `gleam_otp` actors.
- `gleam/erlang/reference` — Typed wrapper around Erlang references
  (`make_ref`), unique tokens used for monitors, request correlation, and
  one-off message tags.

## Conceptual model (typed wrappers over OTP primitives)
gleam_erlang is a thin, type-safe membrane over Erlang/OTP's untyped runtime
primitives. Where Erlang exposes bare terms (PIDs as PIDs, atoms as atoms,
charlists as lists, refs as refs, ports as ports), gleam_erlang lifts each
into a distinct Gleam type so the type system tracks which value is which
and which operations are valid on it.

The conceptual layers, bottom-up:

1. **Identity primitives** — `atom`, `reference`, `port`, `node` wrap the
   symbolic/unique tokens of the BEAM runtime. These are the building blocks
   for naming, correlation, and distribution.
2. **String/data primitives** — `charlist` wraps the legacy list-of-integers
   string form still used by many Erlang/OTP APIs and NIFs.
3. **Process & message layer** — `process` is the heart of the package. It
   provides:
   - `Pid` — a typed process identifier (wraps Erlang pid).
   - `Subject(a)` — a typed receive endpoint: a PID plus a type tag for the
     message it accepts. This is the typed analogue of an Erlang mailbox.
   - `Selector(a)` — a typed selective receive: pattern-match on the next
     message of a given type from a subject, the typed analogue of Erlang's
     `receive` with guards.
   - `spawn` / `send` / `receive` / `select` / `try_call` / `monitor` /
     `link` / `exit` / `sleep` / timers — the full process lifecycle and
     synchronization toolkit, typed.
4. **Application metadata** — `application` exposes OTP application
   introspection (which apps are loaded, their env, start type), the
   top-level unit of OTP releases.

Subjects and selectors are the typed message-passing layer that
`gleam_otp`'s `actor` builds on: an OTP actor is a `process` that owns a
`Subject` and uses a `Selector` to dispatch incoming messages, with the
actor module supplying the typed message type. This is why gleam_erlang is
listed as feeding both `erlang-interop.md` (the primitive wrappers) and
`otp-actors-and-supervision.md` (the process/subject/selector substrate).

## Relationship to docs/beam/
docs/beam/ documents the underlying Erlang/OTP primitives in depth:
- `gen-server.md`, `gen-statem.md`, `gen-event.md` — the OTP behaviours that
  `gleam_otp/actor` (built on `gleam_erlang/process`) reimplements in a
  typed style.
- `links-monitors-and-exits.md` — the link/monitor/exit-signal semantics
  that `gleam/erlang/process` exposes via `link`, `monitor`,
  `monitor_process`, `exit`, and trap-exit patterns.
- `ets-data.md` — ETS, the term-table store; not wrapped by gleam_erlang
  (separate package), but uses atoms/refs from this package as keys.
- `distribution.md` — distributed Erlang; `gleam/erlang/node` is the typed
  surface over the node primitives documented there.
- `applications.md` — OTP applications; `gleam/erlang/application` is the
  typed wrapper over the application controller described there.
- `binaries.md`, `common-mistakes.md` — runtime term and pitfall context
  for the primitives wrapped here.

gleam_erlang is therefore the typed Gleam surface whose semantics are
documented in docs/beam/: each module here corresponds to a chapter there.
The crawl ledger treats docs/beam/ as the conceptual reference and
gleam_erlang as the typed API.

## Version notes
- Page title reports `gleam_erlang · v1.3.0`.
- Install line pins the major version: `gleam add gleam_erlang@1`.
- Requires OTP 27.0 or higher (stated on index). This is a notable floor:
  OTP 27 introduced `:persistent_term` improvements and other runtime
  features the typed wrappers rely on; older OTP is unsupported.
- The package is the typed-process foundation for `gleam_otp` (crawled at
  link 26, v1.2.0). gleam_otp's actors/supervisors import
  `gleam/erlang/process` for PIDs, subjects, selectors, monitors, and timers.

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html — process/subject/selector core (highest priority; feeds otp-actors-and-supervision.md)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/application.html — OTP application metadata (feeds erlang-interop.md)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/atom.html — atom wrapper
- https://hexdocs.pm/gleam_erlang/gleam/erlang/charlist.html — charlist wrapper
- https://hexdocs.pm/gleam_erlang/gleam/erlang/node.html — distributed node primitives
- https://hexdocs.pm/gleam_erlang/gleam/erlang/port.html — port wrapper
- https://hexdocs.pm/gleam_erlang/gleam/erlang/reference.html — reference wrapper
- https://hex.pm/packages/gleam_erlang — package page (version manifest)

### Skipped
- https://hexdocs.pm/gleam_erlang/ (self — this crawl)
- https://img.shields.io/hexpm/v/gleam_erlang (badge image)
- https://hexdocs.pm/gleam_erlang/ (badge link, same as seed)
- Pride-button / SVG icon assets (UI chrome)
