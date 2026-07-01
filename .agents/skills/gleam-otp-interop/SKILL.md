---
name: gleam-otp-interop
description: |
  Operational guide for Gleam OTP and Erlang interop — `gleam_otp` (v1.2.0)
  actors/supervisors, `gleam_erlang` (v1.3.0) process/application interop,
  `@external` FFI/externals, and the JavaScript target. Load when building or
  debugging typed actors, supervision trees, `Subject`/`Selector`/`Name`
  message passing, links/monitors, or multi-target FFI. Does NOT cover pure
  language fundamentals or stdlib (see `gleam-language`), nor project/CLI/deps/
  publish/testing/HTTP/deploy (see `gleam-packages-ffi`).
---

# Gleam OTP, Erlang interop, and FFI

## Triggers

Load this skill when:

- Writing, reviewing, or debugging `gleam/otp/actor`, `gleam/otp/supervision`,
  `gleam/otp/static_supervisor`, or `gleam/otp/factory_supervisor` code.
- Using `gleam/erlang/process` (`Subject`, `Selector`, `Name`, `Pid`,
  `Monitor`, `Timer`, `ExitReason`) or `gleam/erlang/application`.
- Designing supervision trees, child specs, restart strategies, or
  `auto_shutdown`/significant children.
- Calling Erlang/Elixir/JS via `@external` externals, or bridging Gleam data
  across the FFI boundary.
- Compiling to the JavaScript target (`gleam_javascript`, `[javascript]`
  config, Deno permissions, prelude data-construction API).

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/gleam/otp-actors-and-supervision.md`
  - https://hexdocs.pm/gleam_otp/
  - https://hexdocs.pm/gleam_otp/gleam/otp/actor.html
  - https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html
  - https://hexdocs.pm/gleam_otp/gleam/otp/static_supervisor.html
  - https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html
- `docs/gleam/erlang-interop.md`
  - https://hexdocs.pm/gleam_erlang/
  - https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html
  - https://hexdocs.pm/gleam_erlang/gleam/erlang/application.html
- `docs/gleam/externals-and-ffi.md`
  - https://gleam.run/documentation/externals/
- `docs/gleam/javascript-target.md`
  - https://hexdocs.pm/gleam_javascript/
  - https://gleam.run/documentation/externals/
  - https://gleam.run/writing-gleam/gleam-toml/
- `docs/beam/supervision.md` — OTP `supervisor`, `sup_flags`, MaxR/MaxT,
  `auto_shutdown`, significant children (what `gleam_otp` wraps).
- `docs/beam/gen-server.md` — `gen_server` callbacks and `sys` tracing that
  `gleam/otp/actor` wraps.
- `docs/beam/processes-and-messages.md` — mailboxes, per-pair FIFO ordering,
  send/receive semantics behind `gleam/erlang/process`.
- `docs/beam/links-monitors-and-exits.md` — links, monitors, exit signals,
  trapping (the runtime model behind `link`/`monitor`/`trap_exits`/`kill`).
- `docs/beam/applications.md` — OTP applications (`gleam/erlang/application`
  is a thin typed projection).
- `docs/beam/nifs.md` / `docs/beam/ports-io.md` — BEAM-side equivalents of
  native FFI / external-process I/O.

## Key Rules

- `gleam_otp` v1.2.0 wraps `gen_server`/`supervisor`/dynamic supervisor in a
  typed Gleam API. `Subject`/`Selector`/`Name`/`Pid` live in
  `gleam/erlang/process`, NOT in `gleam_otp` — import them from there.
- Actor lifecycle: build with `actor.new(state)` or
  `actor.new_with_initialiser(timeout, init)`, configure with `on_message`
  (and optional `named`), then `actor.start(builder)` returns
  `StartResult(return)` (= `Result(Started(data), StartError)`).
- A single `on_message(state, msg) -> Next` callback handles ALL messages
  (subsumes `handle_call`/`handle_cast`/`handle_info`). Request/reply is
  modelled by embedding `process.Subject(reply)` in the message.
- `Next` variants: `actor.continue(new_state)`, `actor.stop()`,
  `actor.stop_abnormal(reason)`, `actor.with_selector(next, selector)`.
  Every `on_message` branch MUST return `Next`. Unselected messages are
  discarded with a logged warning.
- `actor.call` is synchronous and a timeout CRASHES the caller (no
  `on_timeout` handler in v1.2.0). `actor.send`/`actor.call` re-export
  `gleam/erlang/process`. Treat `call` timeouts as caller failures; supervise
  the caller.
- Child specs via `gleam/otp/supervision`: `worker(start_fn)` (default
  shutdown 5000 ms) / `supervisor(start_fn)` (unlimited shutdown), chained
  with `restart(spec, restart)`, `significant(spec, bool)`, `timeout(spec,
  ms)` (ignored for supervisor children), `map_data(spec, fn)`. The `start`
  field must return `actor.StartResult`, NOT a raw Erlang MFA.
- `Restart`: `Permanent` (default) / `Transient` (no restart on normal exit) /
  `Temporary` (never restart). `significant: True` only matters when the
  parent's `auto_shutdown` is not `Never`.
- `static_supervisor` (fixed child list): `new(strategy)` →
  `restart_tolerance(intensity, period)` → `auto_shutdown(value)` →
  `add(child)` → `start(builder)` or `supervised(builder)`. `Strategy`:
  `OneForOne`/`OneForAll`/`RestForOne`. `AutoShutdown`:
  `Never`/`AnySignificant`/`AllSignificant`. Prefer `supervised` for nesting.
- `factory_supervisor` (dynamic children from a template): build with
  `worker_child(template)` / `supervisor_child(template)`, then `named(name)`,
  `restart_strategy(restart)`, `restart_tolerance(i, p)`, `timeout(ms)`, and
  `start`/`supervised`. Runtime: `start_child(supervisor, argument)` and
  `get_by_name(name)` — `get_by_name` PANICS if the name is unregistered.
- DO NOT use absent names: `with_message_handler` (use `actor.on_message`);
  `actor.on_timeout` (does not exist); `actor.Subject`/`init`/`Status`/`spec`/
  `start_spec` (not in `gleam/otp/actor`); `static_supervisor.with_strategy`/
  `with_intensity`/`with_period`/`with_auto_shutdown`/`start_link` (use
  `new`/`restart_tolerance`/`auto_shutdown`/`start`); `factory_supervisor.new`
  (use `worker_child`/`supervisor_child`).
- `gleam/erlang/process` v1.3.0: `new_subject()`, `new_selector()`,
  `select`/`select_map`/`select_record`/`select_other`/`select_monitors`/
  `select_trapped_exits`/`deselect`, `selector_receive`/`selector_receive_forever`,
  `send`, `send_after`, `cancel_timer`, `call`/`call_forever`, `monitor`,
  `demonitor_process`, `link`/`unlink`, `trap_exits`, `send_exit`,
  `send_abnormal_exit`, `kill`, `spawn`/`spawn_unlinked`, `self`, `sleep`,
  `new_name`, `register`/`named`/`named_subject`/`unregister`, `is_alive`,
  `flush_messages`, etc.
- A `Subject` is a typed receive endpoint owned by its creator; only the owner
  may `selector_receive` on it. Any process may `send` to it. `send` panics
  when sent to an unregistered named subject.
- `call`/`call_forever` PANIC on timeout, callee exit before reply, or
  unregistered named subject — v1.3.0 has NO result-returning variants.
- `monitor` delivers exactly one `Down`; if the target is already dead the
  reason is `Abnormal(...)` and delivery is not guaranteed by return time.
  `kill` is untrappable. `spawn` is LINKED by default (via `proc_lib`); use
  `spawn_unlinked` for crash isolation. `trap_exits(True)` converts exit
  signals into `ExitMessage` mailbox messages.
- `new_name` generates an atom; atoms are NEVER GC'd — call it a bounded
  number of times at startup, never in a loop or supervision tree. Create one
  `process.Name` per logical identity and thread it through.
- `select_record` will NOT match subject-sent messages (subjects use a unique
  internal tag) — use `select`/`select_map` for subjects.
- `gleam/erlang/application` v1.3.0 surface is tiny: `StartType`
  (`Normal`/`Takeover(previous)`/`Failover(previous)`) and
  `priv_directory(name) -> Result(String, Nil)`. It does NOT wrap
  `start`/`stop`/`ensure_started`/`get_env`/`set_env` — call those via Erlang
  FFI. Use `priv_directory` for runtime assets (never hardcode `priv/`).
- `gleam_erlang` requires Erlang/OTP 27.0+. Add with `gleam add gleam_erlang@1`;
  `gleam_otp` with `gleam add gleam_otp@1`.
- FFI (`@external`): bodiless Gleam fn annotated with exactly 3 args — target
  (`erlang` or `javascript`), module, function name. Type annotations are
  MANDATORY; the compiler cannot verify the foreign fn exists or matches.
  Use sparingly; prefer Gleam code or `gleam_erlang`/`gleam_javascript`.
- Multi-target externals: multiple `@external` lines, OR a Gleam body fallback
  used when no external matches the target. A single-target external is a
  compile error on the other target.
- Erlang FFI data mapping: `True`/`False`→`true`/`false`; `Int`→integer;
  `Float`→float; `String`→UTF-8 binary (NOT Erlang `string()` charlist —
  convert with `unicode:characters_to_binary/1` or `gleam/erlang/charlist`);
  `Nil`→atom `nil`; `Ok(x)`/`Error(x)`→`{ok, x}`/`{error, x}` tagged tuples
  (bare `ok`/`error` atoms are NOT compatible); custom variant `User(id:10)`
  →`{user, 10}` (PascalCase→snake_case). Lists must be proper.
- Elixir externals use the `erlang` target with an `Elixir.` prefix. Erlang/
  Elixir macros are not usable outside their language.
- JS FFI: module path is a JS `import` (relative `.mjs` for Node, or an npm
  package path). Never mutate a JS array used as a Gleam tuple; never pass
  `Infinity`/`NaN` or non-whole numbers as `Int`. Do NOT vendor JS deps into a
  published Hex package — document the npm packages users must install.
- Since Gleam v1.13, JS-compiled code exposes a prelude data-construction API
  (importable as `src/gleam.mjs`): `Result$Ok`/`Result$Error`,
  `List$Empty`/`List$NonEmpty`, `BitArray$BitArray`, `TypeName$VariantName`,
  etc. `Nil`→`undefined`; `#(a,b)`→array `[a,b]` (immutable).
- `gleam_javascript` (v1.0.0) provides typed wrappers `gleam/javascript/array`,
  `gleam/javascript/promise`, `gleam/javascript/symbol` — use these instead of
  raw `Dynamic` for JS-native values. On the JS target, concurrency is
  `promise`, NOT BEAM processes/OTP.
- `[javascript]` config: `runtime` (`node`/`deno`/`bun`, default `node`),
  `source_maps` (default `false`), `typescript_declarations` (default
  `false`). Deno permissions under `[javascript.deno]` are least-privilege
  allow-lists (`allow_net`/`allow_read`/`allow_env`/`...; `allow_all = true`
  overrides all — avoid in production).
- Never use `gleam/dynamic.Dynamic` to represent an FFI type — define a
  precise opaque domain type (e.g. `ZipHandle`, not generic `Pid`).

## Quick Commands

```bash
gleam add gleam_otp@1 gleam_erlang@1     # OTP + Erlang interop deps
gleam add gleam_javascript               # JS-target typed FFI wrappers
gleam build --target erlang              # compile/check Erlang target
gleam build --target javascript         # compile/check JS target
gleam test -t erlang && gleam test -t javascript  # both targets
gleam shell                              # start an Erlang shell (Erlang target)
```

## Anti-patterns

- Importing `Subject`/`Selector`/`Name` from `gleam_otp` — they live in
  `gleam/erlang/process`.
- Using `with_message_handler`/`actor.on_timeout`/`actor.Subject`/`init`/
  `Status`/`spec`/`start_spec` — not in `gleam/otp/actor` v1.2.0.
- `static_supervisor.with_strategy`/`with_intensity`/`with_period`/
  `with_auto_shutdown`/`start_link` — use `new`/`restart_tolerance`/
  `auto_shutdown`/`start`.
- `factory_supervisor.new` — use `worker_child`/`supervisor_child`.
- Returning a raw Erlang start tuple from a child spec `start` field — must
  return `actor.StartResult`.
- Calling `new_name` in a loop or supervision tree (atom table exhaustion).
- Calling `get_by_name` before the supervisor is registered (panics).
- Forgetting to re-add the default subject after `actor.selecting` overwrites
  the selector.
- Expecting result-returning `send`/`call` variants in v1.3.0 (they panic).
- Treating `kill` as trappable; using `select_record` for subject messages;
  using `spawn` (linked) where `spawn_unlinked` is needed.
- Expecting `gleam/erlang/application` to wrap `start`/`stop`/`get_env`.
- Omitting type annotations on `@external` functions (mandatory); passing
  Erlang `string()` charlists as Gleam `String`; returning bare `ok`/`error`
  atoms instead of `{ok, _}`/`{error, _}` tagged tuples.
- Mutating JS arrays used as Gleam tuples; passing `Infinity`/`NaN`/non-whole
  numbers as `Int`; vendoring JS deps into a Hex package.
- Using `Dynamic` for FFI types; expecting BEAM/OTP features on the JS target.

## Related Skills

- `gleam-language`
- `gleam-packages-ffi`
- `beam-supervision`
- `beam-gen-server`
- `beam-processes`
- `beam-links-monitors-and-exits`
- `beam-applications-releases`
