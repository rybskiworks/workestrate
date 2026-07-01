# Validation Workflow

## Purpose

Step-by-step procedure for validating BEAM/OTP code before merge or release. The agent runs the full validation suite — compile, Dialyzer, runtime checks, sys inspection, supervisor intensity sanity, and release sanity — and reports gate status.

## What happens first

1. Determine the validation context: pre-merge (code correctness) or pre-release (code + packaging + configuration).
2. Identify the language (Erlang, Elixir, Gleam) to select the correct tool commands.
3. Confirm the PLT (Persistent Lookup Table) for Dialyzer is up to date.

## Context gathering

- Read the diff or release manifest to identify what changed.
- Read the relevant docs for the validation checks (see "Docs consulted" below).
- Load the skills for the components being validated.

## Docs consulted

- `gen-server.md` — gen_server callback contract validation.
- `supervision.md` — supervisor flags and child spec validation.
- `proc-lib-and-sys.md` — sys inspection and code loading checks.
- `runtime-debugging.md` — runtime inspection and sys debugging.
- `applications.md` — application callback and .app file validation.
- `releases.md` — release sanity (sys.config, vm.args, embedded mode).
- `nifs.md` — dirty-scheduler/NIF sanity (NIF library version, dirty scheduler flags).
- `distribution.md` — cookie/cluster config sanity (cookie set, node name correct, EPMD reachable).
- `common-mistakes.md` — `binary_to_term` on untrusted input check (must use `[safe]`).

## Skills loaded

- `beam-supervision` — supervisor/child spec validation.
- `beam-gen-server` — gen_server callback validation.
- `beam-observability-debugging` — sys inspection and runtime checks.
- `beam-applications-releases` — application and release validation.

## Checks run

```sh
# Compile (with warnings as errors):
erlc +warn +report -o ebin src/*.erl     # Erlang
# rebar3 compile                          # Erlang (rebar3)
# mix compile --warnings-as-errors        # Elixir
# gleam build                             # Gleam

# Dialyzer:
dialyzer --src src/ --plt .plt             # BEAM-common
# rebar3 dialyzer                          # Erlang (rebar3)
# mix dialyzer                             # Elixir (Dialyxir)

# XRef:
# Erlang: xref:analyze/2 in shell
# Elixir: mix xref warnings

# Tests:
rebar3 ct                                  # Erlang (Common Test)
# mix test                                 # Elixir (ExUnit)
# gleam test                               # Gleam

# Release sanity (pre-release only):
erl -eval '{ok,[C]}=file:consult("config/sys.config"),io:format("~p~n",[C])' -s init stop -noshell
erl -eval '{ok,[{application,A,S}]}=file:consult("ebin/my_app.app"),io:format("~p~n",[S])' -s init stop -noshell
```

Runtime checks (in an Erlang shell after starting the application):
```erl
application:which_applications().      %% all expected apps started
supervisor:which_children(SupName).    %% supervision tree complete
supervisor:count_children(SupName).    %% child counts correct
sys:get_state(ServerName).             %% gen_server state accessible
sys:get_status(ServerName).            %% debug list empty
erlang:system_info(process_count).     %% process count stable
erlang:statistics(run_queue).          %% run-queue low
```

## Process

1. **Compile**: clean compile with warnings enabled; treat warnings as errors if repo policy requires it.
2. **Dialyzer**: run Dialyzer and verify no new warnings, especially callback return-type mismatches and undefined functions.
3. **XRef**: run cross-reference analysis and verify no undefined function calls or warnings.
4. **Tests**: run the full suite (Common Test / ExUnit / gleam test) and verify all tests pass.
5. **Runtime checks**: start the application and verify apps started, supervision tree complete, child counts correct, gen_server states accessible, no sys debug handlers, stable process count, and low run-queue. Binary safety: verify `binary_to_term/1` on untrusted input uses `[safe]` option — see `common-mistakes.md`.
6. **Supervisor intensity sanity**: verify intensity/period are not identical at every supervision level (multi-level product warning).
7. **Release sanity** (pre-release only): verify `sys.config` is a valid Erlang term, `.app` is valid and complete, `vm.args` flags are correct, and (if hot upgrades) `appup`/`relup` exist and `code_change/3` is implemented.
8. **Report**: summarize gate status (pass/fail per check).

## Evidence reported

- Compile: clean / warnings listed.
- Dialyzer: clean / warnings listed.
- XRef: clean / warnings listed.
- Tests: N passed, 0 failed (or failures listed).
- Runtime checks: all passed / issues listed.
- Supervisor intensity: verified / issues listed.
- Release sanity (if pre-release): all checks passed / issues listed.
- Overall verdict: pass / fail.

## When human judgment is needed

- **Dialyzer warnings**: some are false positives (especially with success typing); confirm whether to fix or suppress.
- **Release mode**: hot code upgrades make validation significantly more complex (appup/relup verification) — confirm scope.
- **Performance / coverage gates**: if the repo has benchmarks or coverage thresholds, confirm whether they gate the build.

## Related docs

- `../gen-server.md`, `../supervision.md`, `../proc-lib-and-sys.md`, `../runtime-debugging.md`, `../applications.md`, `../releases.md`

## Related workflows

- `code-review.md` — validation complements code review; run both before merge.
- `implementation.md` — if validation finds issues requiring reimplementation.
- `debugging.md` — if validation reveals a defect.
