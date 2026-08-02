---
name: elixir-error-handling
description: |
  Operational guide for Elixir error handling — error tuples vs exceptions,
  raise/reraise/rescue, defexception, let-it-crash, supervisor restart semantics,
  exit signals. Load when handling errors, exceptions, or debugging crash/restart
  behavior. Does NOT cover OTP callback contracts (see elixir-otp) or pure coding
  patterns (see elixir-coding).
---

# Error Handling and Crash Semantics

## Triggers

Load this skill when:

- Writing or reviewing error-handling code.
- Defining custom exceptions.
- Deciding error tuples vs exceptions.
- Debugging crashes, restarts, exit signals.
- Using `try/catch/rescue`.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/error-handling.md`
  - https://hexdocs.pm/elixir/try-catch-and-rescue.html
  - https://hexdocs.pm/elixir/Exception.html
  - https://hexdocs.pm/elixir/Kernel.html
  - https://hexdocs.pm/elixir/Process.html
  - https://hexdocs.pm/elixir/Kernel.SpecialForms.html#__STACKTRACE__/0
  - https://www.erlang.org/doc/system/errors.html
- `docs/elixir/beam-otp-internals.md`
  - https://www.erlang.org/doc/system/errors.html
  - https://www.erlang.org/doc/system/sup_princ.html
  - https://www.erlang.org/doc/system/ref_man_processes.html
  - https://www.erlang.org/doc/system/spec_proc.html
  - https://blog.stenmans.org/theBeamBook/
- `docs/beam/links-monitors-and-exits.md` — link/monitor/alias mechanics, exit-signal propagation, `trap_exit` reception rules.
- `docs/beam/common-mistakes.md` — BEAM/Erlang pitfalls and secure-coding caveats.

## Key Rules

- Three exception classes: `:error` (raise / runtime errors), `:exit`
  (`Kernel.exit/1`), `:throw` (`Kernel.throw/1`, rare/non-local return).
- `try` grammar: `do`/`rescue`/`catch`/`after`/`else`. `rescue` matches only
  `:error`; `catch` matches all three classes. `after` always runs (soft
  guarantee, even on raise/throw/exit). `else` matches the `do` block's return
  value.
- `rescue` forms: `rescue RuntimeError ->`,
  `rescue e in RuntimeError -> e`, `rescue [A, B] ->`, `rescue e -> e`,
  `rescue _ ->`.
- `raise/1` (message -> `RuntimeError`), `raise/2` (exception module + args).
  `reraise/2`/`reraise/3` preserves the original stacktrace (use inside
  `rescue` to re-throw without resetting trace).
- `defexception` for custom exceptions: defines `__exception__: true`,
  `message/1`, `exception/1`. Implement `message/1` explicitly for good error
  messages.
- `__STACKTRACE__/0` inside `rescue`/`catch` for the current stacktrace (NOT
  deprecated `System.stacktrace/0`).
- Error tuples vs exceptions:
  - `{:ok, value}` / `{:error, reason}` tagged tuples for expected/operational
    failures (file not found, validation, not-found). Handle via pattern
    matching / `with/1`.
  - Exceptions (`raise`) for programmer errors / invariant violations /
    invalid arguments.
  - Boundary layers (controllers, API edges): error tuples. Internal logic:
    may raise.
  - `foo`/`foo!` pairs: non-bang returns tuple/nil, bang raises.
- "Let it crash": processes are isolated; a crashing process is restarted by
  its supervisor. Don't defensively `rescue` everything — let supervisors
  handle restarts for transient failures.
- `rescue _` / `rescue e ->` without `reraise` or explicit translation is a
  smell (swallows errors).
- Exit signals: `:normal` (not an error, linked processes don't crash),
  `:kill` (untrappable, always terminates), other reasons (linked processes
  exit with same reason). `Process.exit/2` sends exit signals.
- `trap_exit` (`Process.flag(:trap_exit, true)`): converts exit signals from
  linked processes into `{:EXIT, pid, reason}` messages. Use in supervisors /
  resource-owning GenServers needing cleanup.
- Supervisor restart semantics: `:permanent` (always restart), `:transient`
  (restart only on abnormal exit, not `:normal`/`:shutdown`), `:temporary`
  (never restart). `max_restarts`/`max_seconds` -> supervisor gives up and
  terminates itself + its siblings.
- `:kill` exit reason bypasses `trap_exit` (cannot be caught).
- `after` is a soft guarantee — it runs on normal exit, raise, throw, exit,
  but NOT on `:kill` or VM crash.

## Quick Commands

```bash
iex -S mix                                # interactive debugging
:erlang.process_info(pid, :messages)      # inspect mailbox
:sys.get_state(pid)                       # GenServer state
:observer.start                           # GUI inspector
```

## Anti-patterns

- `rescue _ ->` swallowing errors without logging or reraising.
- Using `System.stacktrace/0` (deprecated; use `__STACKTRACE__/0`).
- Defensively rescuing in business logic instead of letting supervisors
  restart.
- `raise` for expected/operational failures (use error tuples).
- Error tuples for programmer errors / invariant violations (use `raise`).
- Custom exceptions without `message/1` implementation.
- Relying on `after` for critical cleanup under `:kill` (not guaranteed).
- `trap_exit` outside supervisors without clear reason (hidden global state).

## Related Skills

- elixir-otp
- elixir-coding
- elixir-config
