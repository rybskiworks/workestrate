# Elixir Guidance Index

## Purpose

This corpus is a project-independent reference for Elixir, Erlang/OTP, and the BEAM virtual machine. It is not tied to any specific repository, framework version, or team convention; instead it captures the language semantics, standard-library behavior, OTP theory, tooling workflows, and architectural patterns that any Elixir codebase shares. Every document is sourced from the official Elixir (v1.20.2), Erlang/OTP (27–29), Mix, ExUnit, Hex, Credo, Dialyxir, Ecto, and Phoenix documentation, with verbatim quotations so claims can be audited.

The intended audience is **future AI agents** — agents that will write, review, refactor, debug, validate, or scaffold Elixir code in this or any other repository. Human developers may also find it useful, but the tone, structure, and "when to read" guidance are optimized for agent consumption: each file states its purpose up front, lists its sources, explains the evaluation model and semantics, provides decision tables, and ends with a review checklist, common mistakes, and a "Policy decisions for individual repos" section.

Because the corpus is project-independent, it deliberately stops short of mandating repo-specific choices (e.g., whether Credo runs in `--strict` mode in CI, whether Dialyzer is a gate or advisory, what the test coverage threshold is). Those decisions are enumerated in each file's "Policy decisions for individual repos" section and summarized in the [Open policy decisions](#open-policy-decisions) section below; each consuming repo must record its own answers.

## Relationship to BEAM, Erlang, and Gleam

This Elixir corpus is a **sibling corpus** to `docs/gleam/`; both are BEAM-hosted languages and both reference `docs/beam/` for the shared BEAM/OTP runtime concepts that are not language-specific — processes and message passing, supervision trees, exit signals and links/monitors, applications and releases, and the OTP Logger. Elixir does **not** depend on `docs/gleam/`, and Gleam does not depend on `docs/elixir/`; the two language corpora are independent and meet only at the shared `docs/beam/` runtime layer.

Shared runtime material (supervision theory, exit-signal propagation, `:sys`/`:proc_lib` special-process semantics, release handling, OTP design principles) has been **de-duplicated** into `docs/beam/` links rather than re-derived in each language corpus. The Elixir topic docs retain the Elixir-specific API surface, callback contracts, and Elixir-flavoured guidance, and point to `docs/beam/*.md` for the underlying runtime semantics. See the [BEAM guidance map](#beam-guidance-map) below for the per-doc routing.

`beam-otp-internals.md` has been repurposed into a thin **Elixir ↔ BEAM mapping index**: its former supervision/exit-signal/`sys`/release/design-principles sections now defer to `docs/beam/`, while a trimmed "VM internals (community-sourced)" section (schedulers, reductions, GC, memory model — sourced from the BEAM Book) is retained because that material has no authoritative home in `docs/beam/` yet (a known coverage gap; see `docs/elixir/source-map.md` → "Revisit later").

## BEAM guidance map

The table below routes each Elixir topic doc that touches shared BEAM runtime to the authoritative `docs/beam/*.md` reference(s). Elixir-specific API and guidance stays in the Elixir doc; only the shared runtime semantics are delegated to `docs/beam/`.

| Elixir doc | Shared runtime delegated to `docs/beam/` |
|---|---|
| `otp-supervision.md` | `docs/beam/supervision.md` (supervisor flags, child specs, strategies, intensity/period, auto-shutdown), `docs/beam/gen-server.md` (GenServer callback contract, call/cast/reply, `:via`) |
| `concurrency-processes.md` | `docs/beam/processes-and-messages.md` (spawn/send/receive, process states, reductions, process dictionary), `docs/beam/links-monitors-and-exits.md` (links vs monitors, `trap_exit`, exit-signal propagation) |
| `error-handling.md` | `docs/beam/links-monitors-and-exits.md` (exit reasons, exit signals, let-it-crash), `docs/beam/common-mistakes.md` (process/exit pitfalls) |
| `configuration-and-runtime.md` | `docs/beam/logger-and-config.md` (OTP Logger handlers/filters/formatters), `docs/beam/applications.md` (application env, `.app` spec, start/stop) |
| `mix-project-structure.md` | `docs/beam/releases.md` (`mix release`, `Config.Provider`, `runtime.exs`, `appup`/`relup`), `docs/beam/applications.md` (`.app` generation) |
| `beam-otp-internals.md` | Now a thin mapping index into `docs/beam/` (supervision, gen-server, links-monitors-and-exits, proc-lib-and-sys, releases, applications, otp-behaviours, processes-and-messages, runtime-debugging, ports-io, distribution, nifs). Retains a trimmed community-sourced VM-internals section (schedulers/GC/memory) with no `docs/beam/` home. |

Docs with no BEAM cross-ref needed (purely Elixir tooling/language): `core-modules.md`, `language-fundamentals.md`, `naming-conventions.md`, `dependencies-and-packages.md`, `documentation-and-publishing.md`, `ecto-phoenix-patterns.md`, `static-analysis-credo.md`, `testing-exunit.md`, `typespecs-and-dialyzer.md`.

## How to use this corpus

### Reading paths

Agents should not read all 15 files linearly. Instead, consult the [Recommended reading paths](#recommended-reading-paths) section to find the 2–4 files most relevant to the current task, then read those files' relevant sections.

### When to consult which docs

- **Writing new Elixir code** → start with `naming-conventions.md` and `language-fundamentals.md`, then consult the domain-specific file (`otp-supervision.md`, `concurrency-processes.md`, `ecto-phoenix-patterns.md`, etc.).
- **Reviewing a PR or diff** → consult `naming-conventions.md`, `static-analysis-credo.md`, `typespecs-and-dialyzer.md`, and `documentation-and-publishing.md` for the style/quality bar, plus the domain file matching the changed code.
- **Debugging a crash or hang** → consult `error-handling.md`, `concurrency-processes.md`, `otp-supervision.md`, and `beam-otp-internals.md` (in that order of depth).
- **Setting up or restructuring a project** → consult `mix-project-structure.md`, `configuration-and-runtime.md`, `dependencies-and-packages.md`, and `testing-exunit.md`.
- **Working with OTP / concurrency** → consult `otp-supervision.md`, `concurrency-processes.md`, and `beam-otp-internals.md` (the deep runtime companion).
- **Working with Ecto / Phoenix** → consult `ecto-phoenix-patterns.md`, then `configuration-and-runtime.md` and `testing-exunit.md` for sandbox/test concerns.

### Conventions used in every file

Each topic file follows a consistent structure:

1. **Purpose** — what the file covers and who it is for.
2. **Sources used** — the official HexDocs / Erlang docs pages consulted, with version notes.
3. **Core guidance** — verbatim quotations from the official docs, with inline citations.
4. **Decision tables** — comparisons (e.g., `Enum` vs `Stream`, `call` vs `cast`, restart strategies).
5. **Review checklist** — a copy-pasteable checklist for code review.
6. **Common mistakes** — anti-patterns and traps.
7. **Policy decisions for individual repos** — the open questions each repo must answer.
8. **Related docs** — cross-references to sibling files in this corpus.

## Generated files

### naming-conventions.md

- **Path:** `docs/elixir/naming-conventions.md`
- **Purpose:** Defines the naming conventions for Elixir identifiers — modules, functions, variables, atoms, filenames, typespecs — so that code is consistent, predictable, and aligned with the official Elixir documentation.
- **Main topics:**
  - `snake_case` vs `CamelCase` casing rules (modules, variables, functions, atoms, filenames, types)
  - Module nesting, aliases, and `alias`/`require`/`import`/`use` directives
  - Trailing `?` (boolean predicates), `is_` prefix (guard-safe checks), trailing `!` (raising variants)
  - `size` (O(1)) vs `length` (O(n)) naming semantics
  - `get` / `fetch` / `fetch!` return-shape conventions
  - `compare/2` returning `:lt` / `:eq` / `:gt`
  - The `t/0` type convention for a module's primary data type
  - Pseudo-variables (`__MODULE__`, `__ENV__`, etc.) and `__foo__` metadata functions
  - Constants as functions vs module attributes
  - Module-to-file path mirroring conventions
- **When a future agent should read it:** Before writing or reviewing any Elixir code that introduces new identifiers, predicates, or error-handling function pairs. Also read it when deciding whether a function should be `foo`, `foo?`, `foo!`, or `is_foo`.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`

### language-fundamentals.md

- **Path:** `docs/elixir/language-fundamentals.md`
- **Purpose:** A multi-part language fundamentals reference covering basic types, operators, pattern matching, guards, control flow, modules/functions, structs, protocols, recursion, enumerables/streams, processes, comprehensions, and sigils — the foundation every agent needs before touching Elixir code.
- **Main topics:**
  - Basic types: integers, floats, booleans, `nil`, atoms, strings/binaries, charlists, lists, tuples, maps, keyword lists
  - Immutability and collection-choice decision guide
  - Basic operators: arithmetic, boolean (strict vs truthy), comparison, concatenation, pipe `|>`, special-form operators, precedence table
  - Pattern matching: `=`, pin `^`, matching on tuples/lists/maps/structs/strings/binaries
  - Guards: `when`, strict boolean semantics, allowed guard expressions, `defguard`/`defguardp`
  - Control flow: `case/2`, `cond/1`, `if/2`/`unless/2`, `with/1`, truthy/falsy rules
  - Modules and functions: `defmodule`, `def`/`defp`, function clauses, default args, capture `&`
  - Structs: `defstruct`, `@enforce_keys`, `struct!/2`, pattern matching with `%`
  - Protocols: `defprotocol`, `defimpl`, `@derive`, built-in protocols (`Enumerable`, `Collectable`, `Inspect`, `String.Chars`)
  - Recursion and tail-call optimization
  - Enumerables and streams: eager `Enum` vs lazy `Stream`, pipe operator
  - Processes: `spawn`, `send`/`receive`, links, `Task`, stateful processes
  - Comprehensions: `for/1`, generators, filters, `:into`, `:uniq`, `:reduce`
  - Sigils: `~r`/`~s`/`~c`/`~w`/`~D`/`~T`/`~N`/`~U`, custom sigils, heredocs
- **When a future agent should read it:** When writing or reviewing Elixir code that involves types, operators, pattern matching, control flow, structs, protocols, or comprehensions. This is the foundational reference — read the relevant section before consulting domain-specific files.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/debugging.md`

### core-modules.md

- **Path:** `docs/elixir/core-modules.md`
- **Purpose:** A multi-section reference for Elixir's core modules and the API-design patterns they follow, covering `Enum`, `Stream`, `Map`, `Keyword`, `List`, `String`, `Kernel.SpecialForms`, `URI`, `Path`, `File`, `Code`, and `Kernel` in depth.
- **Main topics:**
  - `Enum`: eager evaluation model, key functions (traversal, filtering, selecting, counting, sorting, grouping, slicing), complexity tables, `Enumerable` protocol
  - `Stream`: lazy evaluation, composition/fusion, infinite streams, `Stream.resource/3`, `File.stream!`/IO
  - `Enum` vs `Stream` decision guide
  - `Map`: immutability, update syntax `%{map | k => v}`, `map.key` vs `map[key]`, `Access` behaviour, key functions
  - `Keyword`: ordered key-value lists, `Access`, key functions
  - `List`: linked-list performance, key functions
  - `String`: UTF-8 binaries, graphemes vs codepoints, `String` vs `:binary`
  - `Kernel.SpecialForms`: `for/1`, `case/2`, `with/1`, `receive/1`, `%`/structs
  - API design / naming patterns: `?`, `!`, `_by`, `_while`, `_every`, `take`/`drop` symmetry, `into`, `count` vs `size` vs `length`
- **When a future agent should read it:** When writing or reviewing code that transforms collections, uses `Enum`/`Stream`/`Map`/`Keyword`/`List`/`String` APIs, or when deciding between eager and lazy evaluation. Also read it for the complexity characteristics of collection operations.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/refactoring.md`

### otp-supervision.md

- **Path:** `docs/elixir/otp-supervision.md`
- **Purpose:** Defines guidance for OTP abstractions in Elixir: `GenServer`, `Supervisor`, `DynamicSupervisor`, `Registry`, `Application`, child specs, and supervision trees, so that process behavior is consistent, predictable, and aligned with the official Elixir and Erlang documentation.
- **Main topics:**
  - `GenServer`: client-server theory, when to use/not use, callback contract (`init`, `handle_call`, `handle_cast`, `handle_info`, `handle_continue`, `terminate`, `code_change`, `format_status`), `call` vs `cast` vs `info`, reply patterns, timeouts, naming/registration, `start_link` vs `start`
  - `Supervisor`: supervision theory, `start_link/2` forms, strategies (`:one_for_one`, `:one_for_all`, `:rest_for_one`), restart values (`:permanent`/`:transient`/`:temporary`), shutdown values, restart intensity (`max_restarts`/`max_seconds`), child specs, `child_spec/1`, `auto_shutdown`
  - `DynamicSupervisor`: when to use, `:one_for_one` only, ignored `:id`, `start_child/2`, `terminate_child/2`, `which_children/1`, `count_children/1`, `:extra_arguments`, migrating from `:simple_one_for_one`
  - `Registry`: `:via` registration, `:unique`/`:duplicate` keys, partitioning, `dispatch/4`
  - `Application` and supervision tree construction
- **When a future agent should read it:** When writing, reviewing, or debugging `GenServer`/`Supervisor`/`DynamicSupervisor`/`Registry` code, designing supervision trees, or configuring child specs and restart strategies.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/debugging.md`

### concurrency-processes.md

- **Path:** `docs/elixir/concurrency-processes.md`
- **Purpose:** A project-independent reference for BEAM concurrency primitives centered on the `Process` module, covering spawning, messaging, links, monitors, timers, process inspection, process flags, registration, the process dictionary, `Task`/`Task.Supervisor`, `Agent`, and `Node`.
- **Main topics:**
  - BEAM concurrency model: preemptive scheduling, process isolation, message copied between heaps
  - Spawning: `spawn`/`spawn_link`/`spawn_monitor`, `Process.spawn/2,4` with options
  - `send`/`receive`: selective receive, mailbox scanning, `after` timeout
  - Timers: `Process.send_after/4`, `cancel_timer/2`, `read_timer/1`
  - Process inspection: `alive?/1`, `info/1,2`, `list/0`
  - Links vs monitors: bidirectional vs unidirectional, `:DOWN` messages, `demonitor/2` with `:flush`
  - `trap_exit` and exit signals: `:normal`/`:kill`/other reasons, `Process.exit/2`
  - Process flags: `:trap_exit`, `:priority`, `:save_calls`, `:sensitive`, `:max_heap_size`, `:message_queue_data`
  - Process dictionary: discouraged hidden global state, legitimate uses
  - Registration: `register`/`unregister`/`whereis`, dynamic names via `Registry`
  - Heap/stack/reductions/GC: generational GC, `hibernate/3`, `:off_heap`
  - `Task`/`Task.Supervisor`: `async`/`await`, `start`/`start_link`, `async_nolink`, `yield`/`shutdown`/`ignore`, `async_stream`
  - `Agent`: state container over `GenServer`, `get`/`update`/`get_and_update`/`cast`, when to use vs `GenServer`
  - `Node`: distributed Erlang, node names, `ping`/`connect`/`disconnect`, cookies/security, EPMD, hidden nodes
- **When a future agent should read it:** When writing or reviewing code that spawns processes, sends messages, uses links/monitors, sets process flags, uses `Task`/`Agent`, or works with distributed Erlang nodes.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/debugging.md`

### mix-project-structure.md

- **Path:** `docs/elixir/mix-project-structure.md`
- **Purpose:** Describes how Mix projects are organized, configured, and built, covering project definition, custom tasks, compilation, formatting, dependency management, releases, and the standard Mix task inventory.
- **Main topics:**
  - Mix overview: `Mix.Project`, `project/0`, `mix.exs`, `mix help`
  - Custom Mix tasks: `Mix.Task` behaviour, `run/1`, naming conventions
  - Compilation: `mix compile`, `elixirc_paths`, `--warnings-as-errors`
  - Formatting: `mix format`, `.formatter.exs`, `--check-formatted`, `mix format --mix-formatter`
  - Dependencies: `mix deps`, `deps.get`/`deps.compile`/`deps.clean`/`deps.tree`/`deps.unlock`/`deps.update`, `Mix.SCM`, version requirements
  - Releases: `mix release`, `mix release.init`, `Mix.Release`, `Config.Provider`, `runtime.exs`, overlays, steps, cookie, Erlang VM options
  - Testing: `mix test` integration, `mix test --cover`
  - Other tasks: `mix clean`, `mix run`, `mix cmd`, `mix do`, `mix xref`
  - Archives and escripts: `mix archive.*`, `mix escript.build`
  - Optional syntax and `OptionParser`
- **When a future agent should read it:** When creating a new Mix project, adding custom tasks, configuring compilation/formatting/deps, building releases, or understanding the standard Mix task inventory.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/validation.md`

### testing-exunit.md

- **Path:** `docs/elixir/testing-exunit.md`
- **Purpose:** Provides repo-independent guidance for configuring, organizing, and running Elixir tests with ExUnit, covering startup, configuration, async execution, formatters, seed/ordering, `mix test` integration, `test_helper.exs` patterns, assertions, callbacks, capture helpers, doctests, tags, and coverage.
- **Main topics:**
  - ExUnit overview and configuration: `ExUnit.start/1`, `ExUnit.configure/1`, `autorun`, `async_run/0`
  - Configuration options: `:assert_receive_timeout`, `:capture_log`, `:colors`, `:formatters`, `:max_cases`, `:seed`, `:timeout`, etc.
  - Async test execution: `use ExUnit.Case, async: true`, concurrency model, `:max_cases`
  - Formatters: `ExUnit.CLIFormatter`, custom formatters, `ExUnit.Formatter`
  - Seed and test ordering: `:rand`, `:sorted`, custom sorters
  - `mix test` integration: file/line filtering, `--seed`, `--max-failures`, `--cover`, `--only`/`--exclude` tags, `--warnings-as-errors`
  - `test_helper.exs` patterns
  - Assertions: `assert`, `assert_in_delta`, `assert_raise`, `assert_receive`/`assert_received`, `refute`, `match?/2`
  - Callbacks: `setup`/`setup_all`, `ExUnit.Callbacks`, `on_exit/1`
  - Capture helpers: `ExUnit.CaptureIO`, `ExUnit.CaptureLog`
  - Doctests: `ExUnit.DocTest`, `doctest Module`, `:except`/`:import`/`:async`
  - Tags: `@tag`, `@moduletag`, `:registered`, `describe` blocks
  - Coverage: `mix test --cover`, `Mix.Tasks.Test.Coverage`, coverage thresholds
- **When a future agent should read it:** When writing or reviewing tests, configuring ExUnit, using capture helpers, writing doctests, or setting up test coverage in CI.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/validation.md`, `docs/elixir/workflows/implementation.md`

### typespecs-and-dialyzer.md

- **Path:** `docs/elixir/typespecs-and-dialyzer.md`
- **Purpose:** Covers Elixir typespec syntax and attributes (`@type`, `@spec`, `@callback`, etc.) and the Dialyxir/Dialyzer success-typing analysis tool, so agents can make function contracts explicit and catch type inconsistencies.
- **Main topics:**
  - What typespecs are and what consumes them (ExDoc, Dialyzer)
  - Module attributes: `@type`, `@typep`, `@opaque`, `@spec`, `@callback`, `@macrocallback`, `@typedoc`, `@impl`
  - `@spec` syntax: named args, zero-arity, multiple specs, `when` constraints, type variables
  - `@type`/`@typep`/`@opaque` visibility and use cases
  - Built-in types: `term()`, `any()`, `atom()`, `binary()`, `bitstring()`, `boolean()`, `float()`, `fun()`, `integer()`, `list()`, `map()`, `nil`, `no_return()`, `pid()`, `port()`, `reference()`, `tuple()`, `keyword()`, etc.
  - Literal types in specs: integers, atoms, lists, tuples, maps, records
  - Map types in specs: `%{key => value}`, `%{key: value_type}`
  - Behaviours: `@behaviour`, `@callback`, `@macrocallback`, `@optional_callbacks`
  - Pitfalls: the `string()` type (use `String.t()` instead)
  - Dialyxir/Dialyzer: `mix dialyzer`, PLT management, `--plt`, `--format`, exit codes, CI integration, success typing vs gradual typing
- **When a future agent should read it:** When writing or reviewing `@spec`/`@type` annotations, configuring Dialyzer in CI, or debugging type-related Dialyzer warnings. Also read it before publishing a library (typespecs feed ExDoc).
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/validation.md`

### static-analysis-credo.md

- **Path:** `docs/elixir/static-analysis-credo.md`
- **Purpose:** Covers Credo, the static code analysis tool for Elixir, including installation, usage, mix tasks, categories, configuration, check parameters, exit statuses, inline disable directives, and custom checks — so agents can run and configure linting consistently.
- **Main topics:**
  - Installation and dependency configuration (`only: [:dev, :test]`, `runtime: false`)
  - Mix tasks: `mix credo`, `suggest`, `list`, `diff`, `explain`, `info`, `categories`, `version`, `credo.gen.config`, `credo.gen.check`
  - Running Credo: `--all`, `--strict`/`--all-priorities`, reading output (category tags, priority arrows, locations)
  - Categories: Consistency, Design, Readability, Refactor, Warning
  - Configuration file: `.credo.exs`, `checks:` section, enabling/disabling checks, `priority`/`weight`/`param` overrides
  - Check parameters and per-check configuration
  - Exit statuses: bitmask per category for CI gating
  - Config comments: `# credo:disable-for-next-line`, `# credo:disable-for-lines:`, `# credo:disable-for-file:`
  - Custom checks: `mix credo.gen.check`, `Credo.Check` behaviour
  - CLI switches: `--config-file`, `--strict`, `--all`, `--format`, `--min-priority`, `--only`/`--ignore`, `--enable-disabled-checks`
- **When a future agent should read it:** When configuring Credo in a project, interpreting Credo output during review, writing custom checks, or setting up CI linting gates.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/validation.md`

### documentation-and-publishing.md

- **Path:** `docs/elixir/documentation-and-publishing.md`
- **Purpose:** Provides repo-independent guidance for writing documentation in Elixir and publishing packages to Hex.pm, covering `@moduledoc`/`@doc`/`@typedoc`, doctests, ExDoc configuration, and Hex publishing/retirement workflows.
- **Main topics:**
  - Foundational philosophy: documentation as a first-class contract, documentation vs code comments
  - `@moduledoc`: string/heredoc, `false`, metadata (`since:`, `authors:`), `@moduledoc false` caveats
  - `@doc`: placement before first clause, string/heredoc, `false`, metadata (`since:`), deprecation
  - `@typedoc`: documenting `@type`/`@opaque`
  - Doctests: `ExUnit.DocTest`, `doctest Module`, `:except`/`:import`/`:async`, heredoc formatting
  - Heredocs and `@doc` with `## Examples` sections
  - ExDoc: `mix docs`, configuration (`:main`, `:extras`, `:source_ref`, sidebar grouping), `ExDoc` module
  - Hex publishing: `mix hex.publish`, `mix hex.build`, package metadata (`:licenses`, `:links`), `mix hex.retire`, retirement reasons
  - Hex user management: `mix hex.user`
- **When a future agent should read it:** When writing or reviewing module/function/type documentation, adding doctests, configuring ExDoc, or publishing/retiring a Hex package.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`

### configuration-and-runtime.md

- **Path:** `docs/elixir/configuration-and-runtime.md`
- **Purpose:** A multi-section guide covering configuration and runtime behavior in Elixir, from `Logger` and `Application` environment to the `Config`, `System`, and `IO` modules.
- **Main topics:**
  - `Logger`: log levels, compile-time vs runtime level configuration, `Logger.configure/1`, compile-time purging (`compile_time_purge_matching`), formatting (`Logger.Formatter`), metadata, migration from pre-1.15 `Logger.Backends.Console` to the `:default` handler
  - `Config` module: `config.exs` vs `runtime.exs`, `Config.Provider`, `Config.Reader`, build-time vs runtime configuration
  - `Application` environment: `Application.get_env/3`, `Application.compile_env/3`, `Application.get_all_env/1`, `Application.fetch_env!/2`, `System.EnvError`
  - `System` module: `System.get_env/1,2`, `System.fetch_env!/1`, `System.cmd/3`, `System.time/1`, `System.trap_signal/3`, `System.monotonic_time/1`
  - `IO` module: `IO.puts`/`IO.inspect`, `IO.binread`/`IO.binwrite`, chardata/iodata, `IO.ANSI`, `Inspect.Opts`
  - Build-time and runtime configuration with Mix and releases
- **When a future agent should read it:** When configuring `Logger`, writing `config/config.exs` or `config/runtime.exs`, reading application environment, using `System` for env vars or signals, or configuring releases with `Config.Provider`.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/validation.md`

### error-handling.md

- **Path:** `docs/elixir/error-handling.md`
- **Purpose:** Covers the `try/catch/rescue/after/else` special form, exception classes, `raise`/`reraise`/`defexception`, the `Exception` behaviour, error tuples versus exceptions, and the "let it crash" philosophy with process exits and links.
- **Main topics:**
  - `try` special form: `do`/`rescue`/`catch`/`after`/`else` grammar and clause order
  - Three exception classes: `:error` (raise), `:exit` (Kernel.exit/1), `:throw` (Kernel.throw/1)
  - `rescue` clause: matching by module, binding the exception struct, list matching
  - `catch` clause: distinguishing classes, binding stacktrace
  - `after` clause: soft guarantee, runs on normal/raise/throw/exit
  - `else` clause: matching the `do` block's return value
  - `raise`/`reraise`: `raise/1`/`raise/2`, `reraise/2`/`reraise/3`, preserving stacktrace
  - `defexception`: defining custom exceptions, `message/1`, `exception/1`, `message/1`
  - `Exception` behaviour: `message/1`, `exception/1`
  - Error tuples vs exceptions: `{:ok, _}` / `{:error, _}` convention, when to raise vs return tuples
  - "Let it crash" philosophy: process exits, links, supervisors, fault tolerance
  - `__STACKTRACE__/0` vs deprecated `System.stacktrace/0`
- **When a future agent should read it:** When writing or reviewing error-handling code, defining custom exceptions, deciding between error tuples and exceptions, or debugging crashes and exit signals.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/debugging.md`, `docs/elixir/workflows/code-review.md`

### dependencies-and-packages.md

- **Path:** `docs/elixir/dependencies-and-packages.md`
- **Purpose:** Provides repo-independent guidance for adding, updating, auditing, reviewing, and removing Elixir dependencies, treating `mix.exs`, `mix.lock`, and the `mix deps.*` / `mix hex.*` tasks consistently and reproducibly.
- **Main topics:**
  - Dependency auditing: `mix hex.audit` (retired packages), `mix deps.unlock --check-unused` (stale lockfile entries)
  - Clarification that `mix deps.audit` does NOT exist (common misconception)
  - Vulnerability and transitive auditing: `mix_audit`, Dependabot, Renovate, OSV.dev, Hex dependency policies
  - Updating dependencies: `mix deps.update`, `--all`, flags, transitive updates
  - Lockfile hygiene: `mix.lock`, `--check-locked`, `--check-unused`, `--unused`
  - Dependency tree: `mix deps.tree`, `--format`
  - Unlocking: `mix deps.unlock`, `--unused`, `--check-unused`
  - Hex publishing and retirement: `mix hex.publish`, `mix hex.retire`, retirement reasons
  - Hex dependency policies: policy-gated resolution, cooldown
  - Optional dependencies: `optional: true`, `mix compile --no-optional-deps`
- **When a future agent should read it:** When adding, updating, auditing, or removing dependencies, managing `mix.lock`, configuring vulnerability scanning in CI, or publishing/retiring Hex packages.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/validation.md`, `docs/elixir/workflows/code-review.md`

### ecto-phoenix-patterns.md

- **Path:** `docs/elixir/ecto-phoenix-patterns.md`
- **Purpose:** Provides repo-independent, architectural guidance for Elixir/Phoenix code that uses Ecto, describing the patterns that keep contexts, changesets, queries, transactions, and directory structure coherent across Phoenix codebases.
- **Main topics:**
  - Ecto overview: four components (`Repo`, `Schema`, `Query`, `Changeset`), data-flow stack
  - Changesets: `cast/4` vs `change/2`, validations vs constraints, `validate_*` helpers, `*_constraint` helpers
  - Queries: `Ecto.Query`, `from`, composable query syntax, `Ecto.Query.API`, dynamic queries
  - `Ecto.Multi`: composable transactions, `Ecto.Multi.new/0`, `insert`/`update`/`delete`/`run`, `Ecto.Multi.run/3`
  - `Ecto.Repo`: `all/2`, `one/2`, `get/3`, `get_by/3`, `insert/2`, `update/2`, `delete/2`, `transaction/2`, `rollback/1`
  - Phoenix contexts: context boundary pattern, public intent API, `your_first_context`
  - Directory structure: `lib/my_app/` (business logic) vs `lib/my_app_web/` (web layer), `lib/my_app/repo.ex`
  - Schemas: `schema` block, `field`, `belongs_to`/`has_many`/`many_to_many`, embedded schemas
- **When a future agent should read it:** When writing or reviewing Phoenix/Ecto code, deciding where a query belongs, validating external params, using `Ecto.Multi`, or splitting business logic from the web layer.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/implementation.md`, `docs/elixir/workflows/code-review.md`, `docs/elixir/workflows/refactoring.md`

### beam-otp-internals.md

- **Path:** `docs/elixir/beam-otp-internals.md`
- **Purpose:** A project-independent reference for the Erlang/OTP layer and BEAM VM internals that Elixir developers need to understand but that go deeper than Elixir module docs — the deep companion to `otp-supervision.md` and `error-handling.md`.
- **Main topics:**
  - BEAM VM: ERTS, node concept, startup banner tokens (`smp`, `ds`, `async-threads`, `jit`)
  - Schedulers: one per core, ready/waiting queues, preemptive at Erlang level, cooperative at C level
  - Dirty schedulers: normal dirty vs CPU-native dirty, NIFs that may block
  - Reductions and preemption: `CONTEXT_REDS = 4000`, `INPUT_REDUCTIONS`, fair scheduling
  - Process state machine: `free`/`runnable`/`waiting`/`running`/`exiting`/`garbing`/`suspended`
  - Priority queues: `max`/`high`/`normal`+`low`
  - OTP design principles: generic/specific split, behaviours, applications, releases
  - Supervision theory: supervisor flags, child specs, restart strategies (Erlang perspective)
  - `gen_server` concepts: client-server model (Erlang perspective)
  - Error handling: exception classes, exit reasons, exit signals, links, monitors (Erlang perspective)
  - Release handling: `appup`/`relup`, release instructions, hot code upgrade
  - Special processes: `:sys`/`:proc_lib` integration, system messages/events/callbacks
  - Process reference: states, signals, links, monitors
- **When a future agent should read it:** When touching supervision trees, exit-signal handling, `:sys`/`:proc_lib` debugging, release handling, scheduler/GC tuning, or hot code upgrade — any time Elixir-level docs are insufficient and the underlying Erlang/OTP runtime semantics are needed.
- **Related skill files:** Not yet generated
- **Related workflow files:** `docs/elixir/workflows/debugging.md`, `docs/elixir/workflows/implementation.md`

## Recommended reading paths

### New to Elixir

1. `naming-conventions.md` — learn the casing and suffix conventions first.
2. `language-fundamentals.md` — read the Basic Types, Basic Operators, Pattern Matching and Guards, and Control Flow sections.
3. `core-modules.md` — read the `Enum` and `Map` sections to understand the most-used collection APIs.
4. `mix-project-structure.md` — understand how a Mix project is structured and built.
5. `testing-exunit.md` — learn how to write and run tests.

### Writing Elixir code

1. `naming-conventions.md` — check identifier conventions before writing.
2. `language-fundamentals.md` — consult the relevant section (types, pattern matching, control flow, structs, protocols).
3. `core-modules.md` — consult the relevant module section (`Enum`, `Stream`, `Map`, `String`, etc.).
4. Domain file depending on the task:
   - OTP/processes → `otp-supervision.md` and/or `concurrency-processes.md`
   - Ecto/Phoenix → `ecto-phoenix-patterns.md`
   - Configuration → `configuration-and-runtime.md`
   - Error handling → `error-handling.md`
5. `typespecs-and-dialyzer.md` — add `@spec`/`@type` annotations.
6. `documentation-and-publishing.md` — add `@doc`/`@moduledoc` and doctests.

### Reviewing Elixir code

1. `naming-conventions.md` — verify casing, `?`/`!`/`is_` usage, `size`/`length` semantics.
2. `static-analysis-credo.md` — run `mix credo` and interpret findings.
3. `typespecs-and-dialyzer.md` — verify `@spec` presence and correctness; run `mix dialyzer`.
4. `documentation-and-publishing.md` — verify `@moduledoc`/`@doc` on public APIs; check doctests.
5. `error-handling.md` — verify error-tuple vs exception style is appropriate per layer.
6. Domain file matching the changed code (e.g., `otp-supervision.md` for GenServer changes).

### Debugging Elixir

1. `error-handling.md` — understand exception classes, `try/catch/rescue`, and "let it crash".
2. `concurrency-processes.md` — inspect processes, mailboxes, links, monitors, and process flags.
3. `otp-supervision.md` — understand supervision tree behavior, restart strategies, and `:sys` debugging.
4. `beam-otp-internals.md` — go deep into scheduler, reduction, GC, and exit-signal semantics when Elixir-level debugging is insufficient.
5. `configuration-and-runtime.md` — check `Logger` configuration and runtime config for environment-specific issues.

### Setting up a project

1. `mix-project-structure.md` — create the project, configure `mix.exs`, set up `elixirc_paths`.
2. `configuration-and-runtime.md` — set up `config/config.exs` and `config/runtime.exs`, configure `Logger`.
3. `dependencies-and-packages.md` — add dependencies, manage `mix.lock`, set up auditing.
4. `testing-exunit.md` — configure `test_helper.exs`, set up `mix test`, tags, and coverage.
5. `static-analysis-credo.md` — add Credo, generate `.credo.exs`, configure CI linting.
6. `typespecs-and-dialyzer.md` — add Dialyxir, configure PLT, decide on CI gating.
7. `documentation-and-publishing.md` — configure ExDoc, set up `mix docs`.

### OTP/concurrency work

1. `otp-supervision.md` — understand `GenServer`/`Supervisor`/`DynamicSupervisor`/`Registry` APIs and callback contracts.
2. `concurrency-processes.md` — understand `Process` module, spawn/send/receive, links/monitors, `Task`/`Agent`.
3. `beam-otp-internals.md` — understand scheduler, reduction, GC, and exit-signal internals for tuning and debugging.
4. `error-handling.md` — understand exit reasons, "let it crash", and `trap_exit` semantics.
5. `naming-conventions.md` — verify process naming conventions (`__MODULE__` vs `:via` Registry).

## Skill derivation map

Skills (opencode skill packages) will be derived from this corpus in a future phase. The docs are the source of truth; skills will encode the actionable, repeatable procedures that agents should follow. The mapping below is **preliminary** — skill names, boundaries, and granularity are subject to change during the skill-creation phase.

| Skill category (preliminary) | Source docs |
|---|---|
| Elixir naming and style | `naming-conventions.md`, `static-analysis-credo.md` |
| Elixir language fundamentals | `language-fundamentals.md`, `core-modules.md` |
| OTP and supervision | `otp-supervision.md`, `concurrency-processes.md`, `beam-otp-internals.md` |
| Error handling and debugging | `error-handling.md`, `concurrency-processes.md`, `beam-otp-internals.md` |
| Project setup and build | `mix-project-structure.md`, `configuration-and-runtime.md`, `dependencies-and-packages.md` |
| Testing and validation | `testing-exunit.md`, `typespecs-and-dialyzer.md`, `static-analysis-credo.md` |
| Documentation and publishing | `documentation-and-publishing.md`, `typespecs-and-dialyzer.md` |
| Ecto and Phoenix patterns | `ecto-phoenix-patterns.md`, `configuration-and-runtime.md`, `testing-exunit.md` |

Each skill will reference its source doc(s) and will not duplicate the reference material; instead it will encode the decision procedure, checklist, or workflow that an agent follows.

## Workflow map

Workflow docs will exist under `docs/elixir/workflows/`. These are step-by-step procedural guides that an agent follows for a specific task type, referencing the topic docs above for the underlying semantics. The following workflow files are planned:

| Workflow file | Purpose |
|---|---|
| `docs/elixir/workflows/implementation.md` | Step-by-step guide for implementing Elixir features: naming, types, specs, docs, tests. References `naming-conventions.md`, `language-fundamentals.md`, `typespecs-and-dialyzer.md`, `documentation-and-publishing.md`, `testing-exunit.md`. |
| `docs/elixir/workflows/code-review.md` | Step-by-step guide for reviewing Elixir PRs/diffs: style checks, Credo, Dialyzer, typespecs, docs, error-handling style. References `naming-conventions.md`, `static-analysis-credo.md`, `typespecs-and-dialyzer.md`, `documentation-and-publishing.md`, `error-handling.md`. |
| `docs/elixir/workflows/refactoring.md` | Step-by-step guide for refactoring Elixir code: collection choice, `Enum` vs `Stream`, context boundaries, supervision tree restructuring. References `core-modules.md`, `ecto-phoenix-patterns.md`, `otp-supervision.md`. |
| `docs/elixir/workflows/debugging.md` | Step-by-step guide for debugging Elixir crashes and hangs: exception classes, process inspection, `:sys` debugging, scheduler/GC analysis. References `error-handling.md`, `concurrency-processes.md`, `otp-supervision.md`, `beam-otp-internals.md`. |
| `docs/elixir/workflows/validation.md` | Step-by-step guide for validating Elixir code: `mix format --check-formatted`, `mix credo`, `mix dialyzer`, `mix test --cover`, dependency auditing. References `static-analysis-credo.md`, `typespecs-and-dialyzer.md`, `testing-exunit.md`, `dependencies-and-packages.md`, `mix-project-structure.md`. |

All workflow files listed above now exist on disk under `docs/elixir/workflows/`. See [`docs/elixir/workflows/index.md`](workflows/index.md) for the workflow directory index.

## Open policy decisions

Each topic file ends with a "Policy decisions for individual repos" section listing the questions that are deliberately left unanswered because they depend on the consuming repository's conventions, CI setup, and risk tolerance. Below is a consolidated summary of the key decision points that appear across the corpus. Each consuming repo should record its answers in a project-level `CONTRIBUTING.md` or in `docs/elixir/static-analysis-credo.md`.

### CI lint stack

- **`mix format --check-formatted`**: Should formatting be enforced in CI? (Referenced in: `naming-conventions.md`, `language-fundamentals.md`, `core-modules.md`, `otp-supervision.md`, `mix-project-structure.md`, `configuration-and-runtime.md`, `error-handling.md`, `ecto-phoenix-patterns.md`)
- **`mix credo --strict` vs default**: Should CI run Credo in strict mode (all priorities) or the default task? Which `--min-priority` level gates the build? Should the build fail on specific categories (exit-code bitmask) or any issue? (Referenced in: `naming-conventions.md`, `static-analysis-credo.md`, `mix-project-structure.md`, `documentation-and-publishing.md`, `error-handling.md`)
- **`mix dialyzer` in CI**: Should Dialyzer be a gating step or advisory? Which flags are enabled (`:unknown`, `:error_handling`, `:race_conditions`, `:unmatched_returns`)? Should CI fail on `:unknown_function` or add apps to `plt_add_apps`? PLT caching strategy? (Referenced in: `naming-conventions.md`, `typespecs-and-dialyzer.md`, `otp-supervision.md`, `error-handling.md`)
- **`mix compile --warnings-as-errors`**: Should compilation warnings be treated as errors in CI? (Referenced in: `language-fundamentals.md`, `mix-project-structure.md`, `documentation-and-publishing.md`, `error-handling.md`, `testing-exunit.md`)

### Typespec requirements

- **Typespecs on all public functions vs library boundaries only**: Should `@spec` be required on all public functions, or only on library/module boundaries? (Referenced in: `typespecs-and-dialyzer.md`, `otp-supervision.md`, `language-fundamentals.md`)
- **`@type t` requirement**: Should modules that own a primary data type be required to define `@type t`? (Referenced in: `naming-conventions.md`, `language-fundamentals.md`)
- **`@opaque` usage**: Should `@opaque` be used aggressively to hide internal data structures? (Referenced in: `typespecs-and-dialyzer.md`)
- **`string()` vs `String.t()`**: Should `string()` be forbidden in typespecs in favor of `String.t()`? (Referenced in: `language-fundamentals.md`)

### Test configuration

- **`async: true` default**: Should `async: true` be the default for new test modules, or require explicit justification? (Referenced in: `testing-exunit.md`)
- **Test coverage thresholds**: What coverage threshold (if any) should `mix test --cover` enforce? (Referenced in: `testing-exunit.md`)
- **`:seed` in CI**: Should the test seed be fixed in CI or always run randomly? (Referenced in: `testing-exunit.md`)
- **`:capture_log` default**: Should `capture_log: true` be the global default, and what is the policy for per-test overrides? (Referenced in: `testing-exunit.md`)
- **`:timeout` policy**: Should there be a global default timeout for slow tests, or per-test `@tag timeout:`? (Referenced in: `testing-exunit.md`)
- **Default excluded tags**: Which tags should be excluded by default (`:external`, `:integration`, `:flaky`)? (Referenced in: `testing-exunit.md`)
- **`--warnings-as-errors` for test compilation**: Should test compilation treat warnings as errors? (Referenced in: `testing-exunit.md`)

### Error-handling style

- **Error-tuple vs exception style per layer**: Should the boundary layer use error tuples and the internal layer use exceptions, or vice versa? (Referenced in: `error-handling.md`, `ecto-phoenix-patterns.md`)
- **`foo`/`foo!` pair requirement**: Should every fallible public API be required to have both a non-bang and bang variant? Should bang functions without a non-bang counterpart be allowed? (Referenced in: `naming-conventions.md`, `error-handling.md`, `mix-project-structure.md`)
- **Bang functions in business logic**: Should bang functions be permitted in business logic or only at system boundaries? (Referenced in: `error-handling.md`, `ecto-phoenix-patterns.md`)
- **`rescue _` banning**: Should a lint rule ban `rescue _ ->` or `rescue e ->` without `reraise` or explicit translation? (Referenced in: `error-handling.md`)
- **`__STACKTRACE__` requirement**: Should `__STACKTRACE__/0` be required and `System.stacktrace/0` forbidden? (Referenced in: `error-handling.md`)
- **Custom exception `message/1`**: Should custom exceptions be required to implement `message/1` explicitly? (Referenced in: `error-handling.md`)

### OTP and concurrency

- **`handle_info/2` defensive default**: Should `handle_info/2` be required even when no raw messages are expected? (Referenced in: `otp-supervision.md`)
- **Dynamic process naming**: Must dynamic process names use `Registry` or a specific `:via` module? (Referenced in: `otp-supervision.md`, `beam-otp-internals.md`)
- **`:sys` debug in production**: Should `:sys` debug options be permitted in production code at all? (Referenced in: `otp-supervision.md`, `beam-otp-internals.md`)
- **`trap_exit` policy**: Should `:trap_exit` be permitted outside supervisors? Should it be the default for resource-owning GenServers? (Referenced in: `concurrency-processes.md`, `error-handling.md`, `beam-otp-internals.md`)
- **Raw `spawn` policy**: Should raw `spawn*` be allowed at all, or only via `Task`/supervisor? (Referenced in: `concurrency-processes.md`)
- **Process dictionary policy**: Should the process dictionary be banned or scoped to logger/request metadata? (Referenced in: `concurrency-processes.md`)
- **Restart/shutdown policy**: What are the default restart (`:permanent`/`:transient`/`:temporary`) and shutdown values per worker class? (Referenced in: `otp-supervision.md`, `beam-otp-internals.md`)
- **Supervisor intensity/period**: Should the default `max_restarts`/`max_seconds` be deviated from per supervisor? (Referenced in: `beam-otp-internals.md`)
- **GC tuning flags**: Should `:fullsweep_after`, `:min_heap_size`, `:max_heap_size` be tuned or left at defaults? (Referenced in: `beam-otp-internals.md`, `concurrency-processes.md`)
- **`:off_heap` mailboxes**: Should `:off_heap` be used for high-throughput mailboxes, and what measurement justifies it? (Referenced in: `beam-otp-internals.md`)
- **Release strategy**: Immutable releases with rolling deploys, or hot code upgrade via `appup`/`relup`? Are `code_change/3` and `@vsn` required? (Referenced in: `beam-otp-internals.md`)

### Project structure and dependencies

- **`--sup` requirement**: Should all new Mix applications require `--sup`, or allow plain libraries? (Referenced in: `mix-project-structure.md`)
- **Umbrella projects**: Should umbrella projects be permitted, and how should shared config/deps be structured? (Referenced in: `mix-project-structure.md`)
- **`config/runtime.exs` requirement**: Should all applications include `config/runtime.exs` even when no runtime secrets are needed? (Referenced in: `mix-project-structure.md`)
- **Lockfile CI gates**: Which lockfile gates should run in CI (`--check-locked`, `--check-unused`, `mix hex.audit`)? (Referenced in: `dependencies-and-packages.md`)
- **Vulnerability scanner**: Which vulnerability scanner should be adopted (`mix_audit`, Dependabot, Renovate, Snyk)? (Referenced in: `dependencies-and-packages.md`)
- **Git/path dependencies**: Should git/path dependencies be allowed, and what review/pinning rules apply? (Referenced in: `dependencies-and-packages.md`)
- **Optional dependencies CI**: Should `optional:` deps be exercised with `mix compile --no-optional-deps` in CI? (Referenced in: `dependencies-and-packages.md`)

### Documentation

- **`@moduledoc`/`@doc` requirement**: Should `@moduledoc`/`@doc` be required on all public entities in CI (via Credo checks)? (Referenced in: `documentation-and-publishing.md`)
- **Doctests mandatory**: Should doctests be mandatory for all pure public functions? (Referenced in: `documentation-and-publishing.md`)
- **`:since` versioning policy**: Must `:since` annotations match `mix.exs` `:version`? Semver-only? (Referenced in: `documentation-and-publishing.md`)
- **`@doc false` policy**: Should `@doc false` be allowed, or must internal code move into `@moduledoc false` modules? (Referenced in: `documentation-and-publishing.md`)

### Configuration and logging

- **Minimum Elixir version**: What is the minimum Elixir version targeted (determines Logger handler config)? (Referenced in: `configuration-and-runtime.md`)
- **Structured JSON logging**: Should the project adopt structured JSON logging or plain text? (Referenced in: `configuration-and-runtime.md`)
- **Required metadata keys**: Which metadata keys are required (`request_id`, `user_id`, `trace_id`)? (Referenced in: `configuration-and-runtime.md`)
- **`compile_time_purge_matching`**: Should compile-time purging be used in release builds? (Referenced in: `configuration-and-runtime.md`)
- **Colors in production**: Should colors be enabled in production logs or restricted to dev/test? (Referenced in: `configuration-and-runtime.md`)

### Ecto and Phoenix

- **Context boundaries**: Which context boundaries and names apply to each domain area? (Referenced in: `ecto-phoenix-patterns.md`)
- **Cross-context calls**: Should cross-context calls be allowed directly, or require an anti-corruption layer? (Referenced in: `ecto-phoenix-patterns.md`)
- **Bang context functions in controllers**: May controllers use bang context functions (`get_product!/1`) or must they handle `nil` via `get_product/1`? (Referenced in: `ecto-phoenix-patterns.md`)
- **Testing sandbox strategy**: Shared vs ownership-based checkout, and what are the async test boundaries? (Referenced in: `ecto-phoenix-patterns.md`)
- **Telemetry naming**: What are the telemetry event naming and prefix conventions? (Referenced in: `ecto-phoenix-patterns.md`)


## Related indexes

- [Language Guidance Index](../languages.md) — top-level map across the BEAM/Elixir/Gleam corpora.
- [BEAM / OTP Guidance Index](../beam/index.md) — shared runtime source of truth referenced by this corpus.
