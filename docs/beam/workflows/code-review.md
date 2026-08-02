# Code Review Workflow

## Purpose

Step-by-step procedure for reviewing a BEAM/OTP diff against the corpus checklists and tooling gates. The agent verifies callback contracts, child specs, restart strategies, error handling, and naming conventions.

## What happens first

1. Read the diff and identify which OTP components are touched (`gen_server`, `gen_statem`, `supervisor`, `application`, process spawning).
2. Load the relevant docs and skills for those components.
3. Run the tooling gates (compile, Dialyzer) before manual review.

## Context gathering

- Identify the behaviour type(s) in the diff.
- Note any changes to child specs, supervisor flags, or application env.
- Check for changes to error handling (`try/catch`, `exit/1`, `trap_exit`, links/monitors).

## Docs consulted

- `supervision.md` — verify supervisor flags, child specs, restart values, shutdown values, intensity/period.
- `gen-server.md` — verify callback return shapes, `handle_info` presence, naming, `start` vs `start_link`.
- `gen-statem.md` — verify `callback_mode`, event types, timeout kinds, action lists.
- `links-monitors-and-exits.md` — verify exit handling, `trap_exit` usage, error-tuple vs exception vs let-it-crash.
- `overview.md` — verify the change follows OTP design principles (generic/specific split).
- `ets-data.md` — verify ETS table ownership, protection, and match spec usage.
- `nifs.md` — verify NIF dirty scheduler flags and crash safety.
- `common-mistakes.md` — verify `binary_to_term` uses `[safe]` on untrusted input.
- `timers.md` — verify `erlang:send_after/3` (not `timer:send_after/3`) is used at scale.

## Skills loaded

- `beam-supervision` — for supervisor/child spec changes.
- `beam-gen-server` — for `gen_server` callback changes.
- `beam-gen-statem` — for `gen_statem` callback changes.
- `beam-errors-failures` — for error-handling changes.

## Checks run

```sh
erlc +warn +report -o ebin src/*.erl         # compile with warnings
dialyzer --src src/ --plt .plt               # Dialyzer
# Per-language: rebar3 dialyzer / mix dialyzer / gleam build
```

## Process

1. **Compile**: verify clean compile with warnings enabled.
2. **Dialyzer**: verify no new Dialyzer warnings (especially callback return-type mismatches).
3. **Callback contracts**: verify every callback return tuple matches the exact contract. Check `@impl` / `-callback` annotations.
4. **Child specs**: verify `id` (unique), `start` (correct MFA), `restart` (appropriate: permanent/transient/temporary), `shutdown` (infinity for supervisors, appropriate ms for workers), `type` (worker/supervisor).
5. **Supervisor flags**: verify strategy (one_for_one/one_for_all/rest_for_one), intensity/period (not same at every level), auto_shutdown (if used, not permanent child).
6. **Error handling**: verify `trap_exit` is set only where cleanup is needed; verify error tuples vs exceptions are appropriate per layer; verify no defensive catch-and-silence.
7. **Naming**: verify no dynamic atom names; verify `:via` module for dynamic names.
8. **handle_info**: verify `handle_info/2` is implemented if the process receives raw messages (monitors, timers, `EXIT` messages).
9. **terminate**: verify no reliance on `terminate/2` for critical cleanup.
10. **sys debug**: verify no sys debug/trace options left on in production code.

## Evidence reported

- Compile: clean or warnings listed.
- Dialyzer: clean or warnings listed.
- Callback contracts: verified or violations listed.
- Child specs: verified or issues listed.
- Supervisor flags: verified or issues listed.
- Error handling: verified or issues listed.
- Naming: verified or issues listed.
- Overall verdict: approve / request changes / block.

## When human judgment is needed

- **Supervision tree restructuring**: changes to tree shape or restart semantics need human confirmation.
- **trap_exit outside supervisors**: confirm intentional.
- **Custom special process**: confirm the complexity is justified.
- **Hot code upgrade**: if `code_change/3` or `@vsn` changes are involved.

## Related docs

- `../supervision.md`, `../gen-server.md`, `../gen-statem.md`, `../links-monitors-and-exits.md`, `../overview.md`

## Related workflows

- `implementation.md` — if the review finds issues requiring reimplementation.
- `validation.md` — the full gate suite.
- `refactoring.md` — if the review suggests refactoring.
