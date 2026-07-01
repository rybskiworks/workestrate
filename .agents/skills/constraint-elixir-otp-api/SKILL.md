---
name: constraint-elixir-otp-api
description: |
  Enforces Elixir OTP callback discipline during code execution — GenServer and
  Supervisor callback contracts, handle_continue over blocking init, handle_info
  default, child_spec/1, naming/registration, and terminate not guaranteed. Load
  when writing or reviewing GenServer/Supervisor/DynamicSupervisor code. Does NOT
  cover pure language style (see constraint-elixir-style) or BEAM supervision
  semantics (see constraint-beam-supervision).
metadata:
  org.kind: constraint
---

# Constraint: Elixir OTP Callback Discipline

This constraint enforces the GenServer/Supervisor callback contracts and the
Elixir OTP conventions layered on top of them. Violations either terminate the
process (bad return tuples), block supervisor startup, or fill the mailbox with
unhandled messages.

## Triggers

Load this skill when:

- Writing or reviewing a `GenServer`, `Supervisor`, `DynamicSupervisor`, or
  `Registry`.
- Implementing `init/1`, `handle_call/3`, `handle_cast/2`, `handle_info/2`,
  `handle_continue/2`, `terminate/2`.
- Defining `child_spec/1` or choosing restart/shutdown values.

## Rules

1. Use `@impl true` on every callback; return tuples MUST match the exact shapes
   — a bad return terminates the process.
2. Do NOT perform slow/blocking work in `init/1` (it is synchronous and blocks
   `start_link/3`); return `{:ok, state, {:continue, term}}` and do the work in
   `handle_continue/2`.
3. Always implement `handle_info/2` with a defensive default for unexpected
   messages (the default logs and drops them); if you return `{:continue, _}` you
   MUST implement `handle_continue/2` or the process exits with `undef`.
4. `terminate/2` is NOT guaranteed to run (not on `:brutal_kill`, `:kill`, or
   when a linked process exits and the GenServer does not trap exits); do NOT rely
   on it for critical cleanup — use links/monitors instead.
5. `call` (synchronous, blocks caller, has timeout) vs `cast` (async, fire-and-
   forget, always returns `:ok`); prefer `call`; use `cast` only when no
   reply/ordering is needed.
6. Pair `{:noreply, state}` with an explicit `GenServer.reply(from, reply)` — if
   the reply path is skipped the caller blocks forever.
7. For an immediate unconditional follow-up, use `{:continue, term}`, NOT
   `timeout: 0` (a waiting message preempts a `0` timeout).
8. Define `child_spec/1` in each worker module (or rely on the `use GenServer`
   default: `id: __MODULE__`, `restart: :permanent`, `shutdown: 5000`);
   supervisors consume it.
9. Use `Registry` (or another `:via` module) for dynamic names — never
   dynamically generated atoms (atoms are never garbage-collected).
10. `DynamicSupervisor` supports only `:one_for_one` and ignores `:id`; identify
    children by PID. Do NOT use `:simple_one_for_one` in new Elixir code
    (deprecated in v1.10; use `DynamicSupervisor`).
11. GenServer is for modelling RUNTIME characteristics (mutable state, serialized
    access, failure containment) — NEVER for code organization ("calculator
    GenServer" is an anti-pattern).

## References

- Operational skill: `elixir-otp`.
- Docs: `docs/elixir/otp-supervision.md`.

## Out of scope

- Pure Elixir style/typespecs — see `constraint-elixir-style`.
- BEAM supervisor/child-spec semantics — see `constraint-beam-supervision`.
- Exit-signal propagation / `trap_exit` — see `constraint-beam-failure`.

## Violation examples

### Blocking work in `init/1`

```elixir
# FORBIDDEN: blocks supervisor startup until the connection succeeds
@impl true
def init(arg) do
  {:ok, conn} = SomeDatabase.connect(arg)   # slow, blocks start_link
  {:ok, %{conn: conn}}
end
```

Correct: return `{:ok, %{arg: arg, conn: nil}, {:continue, :connect}}` and connect
in `handle_continue/2`.

### Missing `handle_info/2` default

```elixir
# FORBIDDEN: unexpected messages accumulate in the mailbox
# (no handle_info/2 clause defined)
```

Correct: implement `handle_info/2` with a default that logs/drops unexpected
messages.

### Relying on `terminate/2` for critical cleanup

```elixir
# FORBIDDEN: terminate/2 is not guaranteed to run
@impl true
def terminate(_reason, state), do: close_socket(state.socket)
```

Correct: close the socket via a monitor/link on the owning process, or trap exits
deliberately; do not depend on `terminate/2`.

### `cast` when a reply is needed

```elixir
# FORBIDDEN: cast is fire-and-forget; no reply or ordering guarantee
GenServer.cast(server, {:query, self()})
```

Correct: use `GenServer.call/3` when you need a reply or ordering.

## How to check

```bash
mix compile --warnings-as-errors     # bad @impl / missing callbacks surface here
mix credo --strict                    # flags GenServer anti-patterns
mix dialyzer                          # return-tuple shape / callback contracts
```

Manual review:

- `@impl true` on every callback; return tuples match exact shapes.
- No slow work in `init/1` (use `handle_continue/2`).
- `handle_info/2` has a defensive default.
- No reliance on `terminate/2` for critical cleanup.
- `call` over `cast` when a reply is needed; `{:noreply, _}` paired with
  `GenServer.reply/2`.
- No "calculator GenServer"; no dynamic atom names; no `:simple_one_for_one` in
  new code.
