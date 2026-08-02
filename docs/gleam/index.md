# Gleam Guidance Index

## Purpose

This corpus is a project-independent, Gleam-specific reference for the Gleam programming language: its syntax, type system, standard library, OTP/actor layer, FFI, package management, testing, and deployment. Every document is sourced from the official Gleam documentation (gleam.run, tour.gleam.run) and the hexdocs package references for the core Gleam packages — `gleam_stdlib` v1.0.3, `gleam_erlang` v1.3.0, `gleam_otp` v1.2.0, `gleam_json` v3.1.0, `gleam_http` v4.3.0, `gleam_javascript` v1.0.0, and `gleeunit` v1.11.0 — with verbatim quotations so claims can be audited. The 19 topic docs were written strictly from 37 crawled source files persisted in `docs/gleam/.crawl/01-37`.

The intended audience is future AI agents that will write, review, refactor, debug, validate, or scaffold Gleam code. Each file states its purpose, lists its crawl sources, provides core guidance with inline citations, decision tables/checklists, common mistakes, strict-vs-contextual guidance, and a "Policy decisions for individual repos" section. Because the corpus is project-independent, it stops short of mandating repo-specific choices; those are enumerated per file and consolidated in the Open policy decisions section below.

This corpus references `docs/beam/` for shared BEAM/OTP runtime concepts (processes, signals, supervision, distribution, ETS, NIFs, timers, ports, binaries, applications, releases). It does NOT depend on `docs/elixir/`; Gleam and Elixir are sibling corpora that both defer to `docs/beam/` for the underlying Erlang/OTP runtime.

## How to use this corpus

### Reading paths

Agents should not read all 19 files linearly; consult the Recommended reading paths section to find the 2-4 files most relevant to the current task.

### When to consult which docs

- **New to Gleam** -> start with `overview.md`, then `language-fundamentals.md` and `types-records-and-patterns.md`.
- **Writing Gleam code** -> `language-fundamentals.md`, `functions-pipelines-and-use.md`, `types-records-and-patterns.md`, then the domain file (`result-option-and-errors.md`, `stdlib.md`, `otp-actors-and-supervision.md`, etc.).
- **Reviewing a diff** -> `conventions-patterns-antipatterns.md`, the domain file matching the change, and `validation.md`.
- **Debugging a crash or hang** -> `result-option-and-errors.md`, `erlang-interop.md`, `otp-actors-and-supervision.md`.
- **Setting up or restructuring a project** -> `project-structure-and-cli.md`, `gleam-toml-and-targets.md`, `package-management-and-publishing.md`, `testing.md`.
- **Working with OTP / concurrency** -> `otp-actors-and-supervision.md`, `erlang-interop.md`, then `docs/beam/` for the underlying runtime.
- **Working with JSON / API boundaries** -> `json-dynamic-and-api-boundaries.md`, `http-and-services.md`.
- **FFI / externals** -> `externals-and-ffi.md`, `javascript-target.md` (JS target), `erlang-interop.md` (Erlang target).
- **Deploying** -> `deployment-and-runtime.md`, `gleam-toml-and-targets.md`.

### Conventions used in every file

1. **Purpose** - what the file covers and who it is for.
2. **Sources used** - the crawl files and canonical URLs consulted, with version notes.
3. **Related BEAM guidance** - cross-references to `docs/beam/` for shared runtime concepts.
4. **Core guidance** - verbatim quotations from the official docs, with inline citations.
5. **Practical rules** - concrete decision rules and API contracts.
6. **Review / Implementation checklists** - copy-pasteable checklists.
7. **Common mistakes** - anti-patterns and traps.
8. **Strict vs contextual guidance** - what is mandatory vs what depends on context.
9. **Policy decisions for individual repos** - open questions each repo must answer.

## Relationship to Erlang, Elixir, and BEAM

Gleam is a sibling corpus to Elixir; both reference `docs/beam/` for shared runtime concepts. The three corpora are independent: a Gleam agent consults `docs/beam/` when language-level docs are insufficient and the underlying Erlang/OTP runtime semantics are needed; it does not consult `docs/elixir/`.

On the **Erlang target**, Gleam compiles to BEAM bytecode and runs on the Erlang/OTP runtime. BEAM semantics apply: the actor model, supervision trees, process links/monitors, exit-signal propagation, hot code reloading, ETS, NIFs, ports, and distribution. Those runtime concepts are documented in `docs/beam/` and are NOT duplicated here. The Gleam-specific typed wrappers over those primitives live in `gleam_erlang` (-> `erlang-interop.md`) and `gleam_otp` (-> `otp-actors-and-supervision.md`).

On the **JavaScript target**, Gleam compiles to JavaScript and does NOT run on the BEAM. The `docs/beam/` runtime guidance (processes, supervisors, applications, distribution) does not apply; concurrency is `gleam/javascript/promise`, not BEAM processes. See `javascript-target.md`.

## Generated files

### overview.md

- **Path:** `docs/gleam/overview.md`
- **Purpose:** Establish the Gleam mental model: a statically typed, immutable, expression-based functional language that compiles to Erlang (BEAM) and JavaScript. Orients readers to what Gleam is, what it deliberately is not, its compilation targets, its type-system stance, and how it relates to the BEAM runtime guidance in `docs/beam/`.
- **Main topics:**
  - Language stance: no `null`, no implicit conversions, no exceptions, full type checking
  - Compilation targets (Erlang/BEAM, JavaScript)
  - Type-system philosophy and "small and cohesive language" design
  - Production-readiness
  - Relationship to `docs/beam/` runtime guidance
- **When a future agent should read it:** First, as the conceptual entry point before any Gleam work.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### language-fundamentals.md

- **Path:** `docs/gleam/language-fundamentals.md`
- **Purpose:** The concrete syntax and primitive semantics every Gleam file relies on: built-in types, variables and `let`, `const`, blocks, the `todo`/`panic`/`assert` family, modules and imports, and type annotations.
- **Main topics:**
  - Built-in types (`Int`, `Float`, `String`, `Bool`, `Nil`) and operators
  - Variables, `let`, `const`, blocks
  - `echo`, `todo`/`panic`/`assert`
  - Modules, imports, type annotations
  - Strict rules; runtime divergence (JS `Int` overflow, etc.)
- **When a future agent should read it:** Before writing or reviewing any Gleam code; the bedrock for `types-records-and-patterns` and `functions-pipelines-and-use`.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### types-records-and-patterns.md

- **Path:** `docs/gleam/types-records-and-patterns.md`
- **Purpose:** Gleam's data-modeling core: custom types, records, generics, opaque types, tuples, lists, and the `case` expression with its pattern-matching forms and guards.
- **Main topics:**
  - Custom types and variants
  - Records and accessors
  - Generics and opaque types
  - Tuples, lists, bit arrays
  - `case` expression and pattern-matching forms
  - Guards
- **When a future agent should read it:** When modeling domain data, defining custom types/records, or writing `case`/pattern-matching.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### functions-pipelines-and-use.md

- **Path:** `docs/gleam/functions-pipelines-and-use.md`
- **Purpose:** Gleam's function model and the two constructs that define its idiomatic style: first-class functions with labelled/default arguments, the pipe operator `|>`, and `use` expressions for callback-based control flow. Includes external functions (FFI declarations) at the function level.
- **Main topics:**
  - Function definition and first-class functions
  - Labelled and default arguments
  - Pipe operator `|>`
  - `use` expressions and pipelines
  - External functions and types (function-level FFI)
  - Documentation and deprecations
- **When a future agent should read it:** When writing or reviewing Gleam functions, pipelines, or `use`-based control flow.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### result-option-and-errors.md

- **Path:** `docs/gleam/result-option-and-errors.md`
- **Purpose:** Gleam's error model in full: the `Result(a, e)` and `Option(a)` types, the `gleam/result` and `gleam/option` module APIs (gleam_stdlib v1.0.3), and the `panic`/`todo`/`assert` boundary between recoverable errors and crashes.
- **Main topics:**
  - No-exceptions philosophy and no nil-punning
  - `Result` type and `gleam/result` combinators
  - `Option` type and `gleam/option` API
  - `panic`/`todo`/`let assert` crash boundary
  - Crash-vs-`Result` decision
- **When a future agent should read it:** When handling errors, deciding crash-vs-`Result`, or using `Result`/`Option` combinators.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`, `docs/gleam/workflows/debugging.md`

### project-structure-and-cli.md

- **Path:** `docs/gleam/project-structure-and-cli.md`
- **Purpose:** Gleam guidance for project layout, the `gleam` command-line tool, the language server, and installation.
- **Main topics:**
  - `gleam new` scaffold and project tree (`src`/`test` dirs, internal modules)
  - `manifest.toml` lockfile and escript
  - Every CLI subcommand, signature, and flags
  - Language server (LSP) features, editors, limitations
  - Installation
- **When a future agent should read it:** When scaffolding, building, running, testing, formatting, or editing Gleam projects.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`, `docs/gleam/workflows/validation.md`

### gleam-toml-and-targets.md

- **Path:** `docs/gleam/gleam-toml-and-targets.md`
- **Purpose:** Authoritative reference for the `gleam.toml` configuration file required by every Gleam package, and for compilation target configuration (Erlang vs JavaScript).
- **Main topics:**
  - Every `gleam.toml` key (name, version, dependencies, etc.)
  - Target config and options
  - Deno permissions (`[javascript]`)
  - `internal_modules`
  - Repository metadata for Hex
  - TOML 1.1 format
- **When a future agent should read it:** When authoring, reviewing, or validating `gleam.toml`.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`, `docs/gleam/workflows/validation.md`

### stdlib.md

- **Path:** `docs/gleam/stdlib.md`
- **Purpose:** `gleam_stdlib` v1.0.3 - the single dependency every Gleam project relies on for core data types and operations. Covers the 19-module `gleam/...` namespace.
- **Main topics:**
  - Package overview and target support
  - `gleam/list`, `gleam/string`, `gleam/dict`, `gleam/result`, `gleam/option`
  - `gleam/dynamic`, `gleam/dynamic/decode`, `gleam/bit_array`
  - Collection types (`Dict`, `Set`), binary/byte handling (`BitArray`, `BytesTree`, `StringTree`)
- **When a future agent should read it:** When using stdlib modules or choosing collection types.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### json-dynamic-and-api-boundaries.md

- **Path:** `docs/gleam/json-dynamic-and-api-boundaries.md`
- **Purpose:** How Gleam handles untyped boundary data: JSON encode/decode (`gleam_json` v3.1.0), the `Dynamic` type (`gleam/dynamic` v1.0.3), and type-safe decode combinators (`gleam/dynamic/decode` v1.0.3).
- **Main topics:**
  - `gleam_json` encode/decode API and `Json` opaque type
  - `Dynamic` type and construction
  - `gleam/dynamic/decode` combinators
  - parse -> dynamic -> decode -> typed pipeline
  - Boundary discipline (keep dynamic data out of typed internals)
- **When a future agent should read it:** When parsing/producing JSON or handling untyped runtime data at boundaries.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### http-and-services.md

- **Path:** `docs/gleam/http-and-services.md`
- **Purpose:** `gleam_http` v4.3.0 - HTTP types and functions for HTTP clients and servers, and the sans-IO `Service` abstraction.
- **Main topics:**
  - Core types: `Request`, `Response`, `Headers`, `Method`, `Status`
  - Sans-IO `Service` abstraction
  - Module list (`gleam/http`, `gleam/http/cookie`, etc.)
  - Adapter packages (server and client); no I/O in core
- **When a future agent should read it:** When building HTTP clients/servers or reviewing service handlers.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### externals-and-ffi.md

- **Path:** `docs/gleam/externals-and-ffi.md`
- **Purpose:** Gleam's external type and external function features for FFI to Erlang and JavaScript; the `@external` annotation and target-specific usage.
- **Main topics:**
  - `@external(target, module, function)` annotation
  - External types
  - Erlang-target FFI (Erlang, Elixir, LFE)
  - JavaScript-target FFI
  - When to use externals (sparingly)
  - Review risks; v1.13 JS data-construction API
- **When a future agent should read it:** When writing or reviewing FFI/external function declarations.
- **Related skills:** `gleam-otp-interop`, `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### erlang-interop.md

- **Path:** `docs/gleam/erlang-interop.md`
- **Purpose:** `gleam_erlang` v1.3.0 - the typed Gleam surface over Erlang/OTP primitives; the lowest-level core package for Erlang-targeted Gleam code and the foundation on which `gleam_otp` is built.
- **Main topics:**
  - Package overview and Erlang/OTP 27.0+ requirement
  - `gleam/erlang/process` (typed `Subject`/`send`/`receive`/`call`, `Selector`, process)
  - `gleam/erlang/application` (OTP application concepts)
  - BEAM primitive mappings
- **When a future agent should read it:** When writing Erlang-targeted Gleam that uses processes, message passing, or OTP applications.
- **Related skills:** `gleam-otp-interop`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`, `docs/gleam/workflows/debugging.md`

### otp-actors-and-supervision.md

- **Path:** `docs/gleam/otp-actors-and-supervision.md`
- **Purpose:** `gleam_otp` v1.2.0 - typed actors and supervision trees wrapping Erlang/OTP `gen_server`, `supervisor`, and dynamic supervisor behaviours in a fully typed Gleam API.
- **Main topics:**
  - `Actor` abstraction (`gleam/otp/actor`)
  - `Subject`/`Selector`/`Name`/`Pid` (in `gleam/erlang/process`)
  - Child specs (`ChildSpecification`, `gleam/otp/supervision`)
  - Static supervisor (`gleam/otp/static_supervisor`)
  - Factory supervisor / dynamic (`gleam/otp/factory_supervisor`)
  - Restart strategies
- **When a future agent should read it:** When writing, reviewing, or debugging Gleam OTP actors or supervision trees.
- **Related skills:** `gleam-otp-interop`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`, `docs/gleam/workflows/debugging.md`, `docs/gleam/workflows/refactoring.md`

### javascript-target.md

- **Path:** `docs/gleam/javascript-target.md`
- **Purpose:** Gleam guidance for the JavaScript compilation target: JS runtime config, the `gleam_javascript` package, TypeScript declarations, source maps, JS externals (FFI), and target-specific review risks.
- **Main topics:**
  - Target selection (`target = "javascript"`, `--target javascript`)
  - `gleam_javascript` (array/promise/symbol)
  - TypeScript declarations and source maps
  - `@external(javascript, ...)` and prelude API v1.13+
  - Deno permissions (`[javascript]` config)
  - Review risks
- **When a future agent should read it:** When building, reviewing, or debugging Gleam code compiled to JavaScript.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/implementation.md`, `docs/gleam/workflows/code-review.md`

### testing.md

- **Path:** `docs/gleam/testing.md`
- **Purpose:** `gleeunit` v1.11.0 - the standard Gleam test runner; EUnit on Erlang, custom runner on JS.
- **Main topics:**
  - `gleeunit.main()` and test discovery
  - `gleeunit/should` assertion helpers
  - `gleam test` command
  - Per-target testing (Erlang vs JS)
- **When a future agent should read it:** When writing or running Gleam tests.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/validation.md`

### package-management-and-publishing.md

- **Path:** `docs/gleam/package-management-and-publishing.md`
- **Purpose:** Gleam guidance for dependency management, version-constraint syntax, the lockfile, publishing to Hex, package metadata, and Software Bill of Materials (SBoM) generation.
- **Main topics:**
  - `add`/`remove`/`update`/`deps`/`publish`/`docs`/`hex` commands
  - Version-constraint syntax and `manifest.toml` lockfile
  - Path/git dependencies and `dev_dependencies`
  - Hex package metadata (licences, description, repository)
  - SBoM generation via ORT (CycloneDX/SPDX)
- **When a future agent should read it:** When adding/updating/removing dependencies, publishing packages, or producing SBoMs.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/validation.md`

### conventions-patterns-antipatterns.md

- **Path:** `docs/gleam/conventions-patterns-antipatterns.md`
- **Purpose:** Official Gleam guidance on writing idiomatic, reviewable code. Conventions and anti-patterns are always-rules; patterns are techniques to apply when they benefit the code. Also covers syntactic migration pitfalls from Elixir, Erlang, and Rust.
- **Main topics:**
  - Conventions (always-rules)
  - Anti-patterns (always-avoid)
  - Patterns (apply when beneficial)
  - Elixir/Erlang/Rust migration pitfalls
- **When a future agent should read it:** When reviewing Gleam code or migrating from Elixir/Erlang/Rust.
- **Related skills:** `gleam-language`
- **Related workflow files:** `docs/gleam/workflows/code-review.md`

### deployment-and-runtime.md

- **Path:** `docs/gleam/deployment-and-runtime.md`
- **Purpose:** Synthesise Gleam's official deployment and runtime guidance: how Gleam compiles, what it runs on, and the two documented container-first deployment paths (single Linux server with Podman/systemd/Caddy, and Fly.io). Contrasts Gleam's `erlang-shipment` format with traditional BEAM OTP releases.
- **Main topics:**
  - Compilation to Erlang/JS and production-readiness
  - `erlang-shipment` format vs traditional BEAM releases
  - Linux server deployment (systemd, Podman, Caddy)
  - Fly.io deployment (Dockerfile, multi-stage)
  - Hot code reloading; immutability/ETS; division-by-zero
- **When a future agent should read it:** When deploying Gleam applications or understanding runtime behavior.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/validation.md`

### validation.md

- **Path:** `docs/gleam/validation.md`
- **Purpose:** Gleam validation gates: the compile/type-check, format, test, and export commands that catch regressions before merge or release. Maps the `gleam` CLI validation subcommands, LSP diagnostics, per-target validation, and package-interface export into a single checklist.
- **Main topics:**
  - `gleam check` (type-checking; compiler IS the type checker, no Dialyzer)
  - `gleam build` (`--warnings-as-errors`)
  - `gleam format` / `gleam format --check`
  - `gleam test`
  - `gleam export` (package interface)
  - LSP diagnostics
- **When a future agent should read it:** When building or running a validation suite before merge/release.
- **Related skills:** `gleam-packages-ffi`
- **Related workflow files:** `docs/gleam/workflows/validation.md`

## Recommended reading paths

### New to Gleam

1. `overview.md` - language stance, targets, type-system philosophy.
2. `language-fundamentals.md` - syntax, built-in types, modules.
3. `types-records-and-patterns.md` - custom types, records, `case`/pattern matching.
4. `functions-pipelines-and-use.md` - functions, pipe, `use`.
5. `conventions-patterns-antipatterns.md` - idiomatic code and traps.

### Writing Gleam code

1. `language-fundamentals.md` - syntax bedrock.
2. `types-records-and-patterns.md` - data modeling.
3. `functions-pipelines-and-use.md` - function model and idioms.
4. `result-option-and-errors.md` - error handling.
5. The domain file: `stdlib.md`, `json-dynamic-and-api-boundaries.md`, `otp-actors-and-supervision.md`, `http-and-services.md`, or `externals-and-ffi.md`.

### Reviewing Gleam code

1. `conventions-patterns-antipatterns.md` - idioms and anti-patterns.
2. The domain file matching the change.
3. `result-option-and-errors.md` - crash-vs-`Result` discipline.
4. `validation.md` - gates to run before merge.

### Debugging a crash or hang

1. `result-option-and-errors.md` - `panic`/`todo`/`assert` and crash boundaries.
2. `erlang-interop.md` - process/message-passing primitives (Erlang target).
3. `otp-actors-and-supervision.md` - actor/supervisor behavior.
4. `docs/beam/` for underlying runtime (`links-monitors-and-exits`, `processes-and-messages`, `runtime-debugging`).

### Setting up a project

1. `project-structure-and-cli.md` - scaffold, CLI, LSP.
2. `gleam-toml-and-targets.md` - config and target.
3. `package-management-and-publishing.md` - dependencies and publishing.
4. `testing.md` - test runner setup.

### OTP / concurrency work (Erlang target)

1. `otp-actors-and-supervision.md` - actors and supervision.
2. `erlang-interop.md` - process/message-passing primitives.
3. `docs/beam/` - `supervision`, `processes-and-messages`, `links-monitors-and-exits`.

### JSON / API boundaries

1. `json-dynamic-and-api-boundaries.md` - JSON and `Dynamic`/decode.
2. `http-and-services.md` - HTTP types and service abstraction.

### Deploying

1. `deployment-and-runtime.md` - deployment paths and runtime.
2. `gleam-toml-and-targets.md` - target config.

## Skill derivation map

Skills (opencode skill packages) will be derived from this corpus in a future phase. The docs are the source of truth; skills will encode the actionable, repeatable procedures that agents should follow. The mapping below is preliminary - skill names, boundaries, and granularity are subject to change during the skill-creation phase.

| Skill | Source docs |
|---|---|
| `gleam-language` | `overview.md`, `language-fundamentals.md`, `types-records-and-patterns.md`, `functions-pipelines-and-use.md`, `result-option-and-errors.md`, `stdlib.md`, `json-dynamic-and-api-boundaries.md`, `conventions-patterns-antipatterns.md` |
| `gleam-otp-interop` | `otp-actors-and-supervision.md`, `erlang-interop.md` |
| `gleam-packages-ffi` | `project-structure-and-cli.md`, `gleam-toml-and-targets.md`, `externals-and-ffi.md`, `javascript-target.md`, `http-and-services.md`, `testing.md`, `package-management-and-publishing.md`, `deployment-and-runtime.md`, `validation.md` |

Each skill will reference its source doc(s) and will not duplicate the reference material; instead it will encode the decision procedure, checklist, or workflow that an agent follows.

## Workflow map

Workflow docs will exist under `docs/gleam/workflows/`. These are step-by-step procedural guides that an agent follows for a specific task type, referencing the topic docs above for the underlying semantics. The following workflow files are planned:

| Workflow file | Purpose |
|---|---|
| `docs/gleam/workflows/index.md` | Index for the workflow directory; maps task types to workflows, docs, and skills. |
| `docs/gleam/workflows/implementation.md` | Step-by-step procedure for implementing Gleam code: types, functions, error handling, stdlib usage, FFI. References `language-fundamentals.md`, `types-records-and-patterns.md`, `functions-pipelines-and-use.md`, `result-option-and-errors.md`, `stdlib.md`, `externals-and-ffi.md`. |
| `docs/gleam/workflows/code-review.md` | Step-by-step procedure for reviewing a Gleam diff: conventions, anti-patterns, error-handling discipline, target-specific risks. References `conventions-patterns-antipatterns.md`, `result-option-and-errors.md`, `javascript-target.md`, `validation.md`. |
| `docs/gleam/workflows/debugging.md` | Step-by-step procedure for debugging Gleam crashes and hangs: `panic`/`todo`/`assert` boundaries, actor/supervisor behavior, process/message-passing. References `result-option-and-errors.md`, `erlang-interop.md`, `otp-actors-and-supervision.md`. |
| `docs/gleam/workflows/refactoring.md` | Step-by-step procedure for refactoring Gleam code: type-safe changes, supervision tree restructuring, collection choice. References `types-records-and-patterns.md`, `stdlib.md`, `otp-actors-and-supervision.md`. |
| `docs/gleam/workflows/validation.md` | Step-by-step procedure for validating Gleam code: `gleam check`, `gleam format --check`, `gleam test`, `--warnings-as-errors`, per-target validation. References `validation.md`, `testing.md`, `project-structure-and-cli.md`. |

## Open policy decisions

Each topic file ends with a "Policy decisions for individual repos" section listing the questions that are deliberately left unanswered because they depend on the consuming repository's conventions, CI setup, and risk tolerance. Below is a consolidated summary of the key decision points that appear across the corpus. Each consuming repo should record its answers in a project-level `CONTRIBUTING.md`.

### Compilation target

- **Default target (Erlang vs JavaScript)** per project. (Referenced in: `gleam-toml-and-targets.md`, `javascript-target.md`)
- **Whether both targets must be validated in CI.** (Referenced in: `validation.md`, `javascript-target.md`)
- **Minimum Gleam compiler version baseline.** (Referenced in: `overview.md`, `project-structure-and-cli.md`)
- **Minimum Erlang/OTP version** (`gleam_erlang` requires OTP 27.0+). (Referenced in: `erlang-interop.md`)

### Error handling

- **Crash-vs-`Result` policy per layer.** (Referenced in: `result-option-and-errors.md`)
- **Whether `panic`/`todo` are permitted in production code.** (Referenced in: `result-option-and-errors.md`)
- **`let assert` policy** (banned outside tests?). (Referenced in: `result-option-and-errors.md`)

### Project structure and dependencies

- **Whether `internal_modules` is used and its scope.** (Referenced in: `gleam-toml-and-targets.md`)
- **Version-constraint policy** (caret vs exact). (Referenced in: `package-management-and-publishing.md`)
- **Whether git/path dependencies are allowed** and their review/pinning rules. (Referenced in: `package-management-and-publishing.md`)
- **SBoM generation policy** (ORT/CycloneDX/SPDX). (Referenced in: `package-management-and-publishing.md`)

### OTP and concurrency

- **Whether `gleam_otp` actors are required over raw `gleam_erlang` processes.** (Referenced in: `otp-actors-and-supervision.md`, `erlang-interop.md`)
- **Default supervisor restart strategy.** (Referenced in: `otp-actors-and-supervision.md`)
- **Static vs factory supervisor policy.** (Referenced in: `otp-actors-and-supervision.md`)

### FFI / externals

- **Whether externals are permitted and the review bar.** (Referenced in: `externals-and-ffi.md`)
- **JS externals review gate.** (Referenced in: `javascript-target.md`, `externals-and-ffi.md`)

### Testing and validation

- **Whether `gleam test` is a CI gate.** (Referenced in: `testing.md`, `validation.md`)
- **`--warnings-as-errors` policy.** (Referenced in: `validation.md`)
- **Whether `gleam format --check` is enforced.** (Referenced in: `validation.md`)
- **Per-target test policy** (Erlang-only vs both targets). (Referenced in: `testing.md`)

### Deployment

- **Deployment target** (Linux server vs Fly.io vs other). (Referenced in: `deployment-and-runtime.md`)
- **Whether `erlang-shipment` is the only release format.** (Referenced in: `deployment-and-runtime.md`)
- **Config injection strategy** (env vars vs other). (Referenced in: `deployment-and-runtime.md`)

### HTTP and services

- **Which HTTP adapter is standard** (mist/wisp/gleam_httpc/gleam_hackney/gleam_fetch). (Referenced in: `http-and-services.md`)
- **Whether the sans-IO `Service` abstraction is mandatory for handlers.** (Referenced in: `http-and-services.md`)


## Related indexes

- [Language Guidance Index](../languages.md) — top-level map across the BEAM/Elixir/Gleam corpora.
- [BEAM / OTP Guidance Index](../beam/index.md) — shared runtime source of truth referenced by this corpus.
