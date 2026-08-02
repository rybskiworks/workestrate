# Elixir Source Map

## Purpose

This document maps every researched source URL to the Elixir documentation files that use it. It is the authoritative index of the corpus behind the `docs/elixir/` topic files. Future agents can use it to trace any claim back to its primary source, identify which sources are seed (originally provided) versus discovered (found by following links), and assess coverage gaps before extending the documentation.

## Source coverage summary

The corpus comprises **227 unique source URLs** across **11 documentation families**, collected from the "## Sources used" sections of 15 completed topic files under `docs/elixir/`.

| Family | Count | Authority |
|---|---|---|
| 1. Elixir official guides (`hexdocs.pm/elixir/*.html`) | 31 | Official primary |
| 2. Elixir core modules (`hexdocs.pm/elixir/Module.html`) | 56 | Official primary |
| 3. Mix docs (`hexdocs.pm/mix/*.html`) | 33 | Official primary |
| 4. ExUnit docs (`hexdocs.pm/ex_unit/*.html`) | 9 | Official primary |
| 5. Logger docs (`hexdocs.pm/logger/*.html`) | 6 | Official primary |
| 6. Erlang/OTP docs (`erlang.org` + BEAM Book) | 24 | Official primary / community |
| 7. Credo docs (`hexdocs.pm/credo/*.html`) | 14 | Official primary |
| 8. Dialyxir docs (`hexdocs.pm/dialyxir/*.html`) | 1 | Official primary |
| 9. ExDoc/Hex docs (`hexdocs.pm/ex_doc`, `hexdocs.pm/hex`, `hex.pm`) | 16 | Official primary / community |
| 10. Ecto/Phoenix docs (`hexdocs.pm/ecto`, `hexdocs.pm/phoenix`) | 9 | Official primary |
| 11. GitHub source code references (`github.com`) | 28 | Official secondary / community |
| **Total** | **227** | |

Overall coverage: the corpus spans the full Elixir v1.20.2 documentation surface — language guides, core module API reference, Mix build tool, ExUnit testing, Logger, Erlang/OTP runtime theory, Credo static analysis, Dialyxir/Dialyzer, ExDoc/Hex publishing, Ecto/Phoenix data layer, and the Elixir/Erlang source code on GitHub. Two dead links are documented (`basic-operators.html` and `credo/checks.html`, both 404) and retained for provenance.

## Link expansion coverage

The corpus was built iteratively across multiple research passes:

1. **Seed pass (depth 0):** For each of the 15 topic files, a set of seed URLs was provided. These are the sources marked `(PRIMARY)` in each file's "## Sources used" section, plus closely related pages in the same documentation family that were supplied together (e.g., all Credo doc pages for the Credo topic, all Ecto module pages for the Ecto topic).

2. **Depth-1 expansion:** From each seed page, the researcher followed inline cross-references and "See also" links to discover related pages within the same documentation site. For example, `Enum.html` links to `Enumerable.html`, `Collectable.html`, `Stream.html`, and the `enumerable-and-streams.html` guide — all of which were then explored and added to the corpus.

3. **Depth-2 expansion:** Pages discovered at depth 1 were themselves explored for further links. For example, the `gen_server_concepts.html` Erlang system doc links to `design_principles.html` and `sup_princ.html`, which in turn link to `spec_proc.html` and `ref_man_processes.html`. The Erlang/OTP family had the deepest link-following chain (depth 3 in places).

4. **Depth-3 expansion:** In a few cases, tertiary links were followed — e.g., from HexDocs module pages to the corresponding GitHub source files (the "Source" link on every HexDocs page), and from the Elixir CHANGELOG on GitHub to version-specific historical Logger docs.

**Families with meaningful discovered links beyond the original seed list:**

- **Elixir official guides:** Extensive cross-linking between guides. The `language-fundamentals.md` file alone cites 30+ guide pages, most discovered by following links from `basic-types.html` to `lists-and-tuples.html` to `binaries-strings-and-charlists.html` to `keywords-and-maps.html` to `structs.html` to `protocols.html` to `recursion.html` to `enumerable-and-streams.html` to `processes.html`, and so on. The guide family had the richest depth-1 and depth-2 discovery.
- **Elixir core modules:** Module pages cross-reference related modules heavily. `Enum.html` links to `Enumerable.html`/`Collectable.html`/`Stream.html`; `Kernel.html` links to `Kernel.SpecialForms.html`/`Function.html`; `String.html` links to `Regex.html`/`Inspect.html`. Many module pages were discovered at depth 1 from the PRIMARY module for each section.
- **Erlang/OTP docs:** The deepest discovery chain. System docs cross-reference each other extensively: `design_principles.html` to `sup_princ.html` to `gen_server_concepts.html` to `errors.html` to `spec_proc.html` to `ref_man_processes.html`. Application-level module docs (`proc_lib.html`, `sys.html`, `erlang.html`) were discovered at depth 2 from the system docs. The BEAM Book was discovered as a community resource referenced from Erlang documentation.
- **Mix docs:** Task pages cross-reference related tasks. `Mix.Tasks.Deps.html` to `Deps.Get`/`Deps.Update`/`Deps.Clean`/`Deps.Unlock`/`Deps.Tree`/`Deps.Compile`. The `Mix.Tasks.Release.html` page led to `Mix.Tasks.Release.Init.html` and `Mix.Release.html`.
- **Credo docs:** The Credo documentation site has extensive cross-linking between command pages (`suggest_command.html` to `explain_command.html` to `cli_switches.html`), configuration pages (`config_file.html` to `check_params.html` to `config_comments.html`), and check reference pages (`Credo.Check.html` to per-check module pages). The `checks.html` page was discovered to be a 404, with the catalog living in `config_file.html` instead.
- **Logger docs:** Version-specific historical pages (`1.17.2/Logger.html`, `1.15.0/Logger.html`, `1.10.0/Logger.html`) were discovered by checking version dropdowns on HexDocs to trace API evolution (e.g., the 1.15 handler migration, the `warn` to `warning` rename).
- **GitHub source code:** All 28 GitHub references were discovered by following "Source" links from HexDocs module/task pages to the corresponding `.ex`/`.exs` files in the `elixir-lang/elixir` repository. CHANGELOG files were discovered when tracing feature introduction versions.
- **ExDoc/Hex docs:** Cross-references between ExDoc and Hex documentation were followed (e.g., `writing-documentation.html` to `ExDoc.html` to `Mix.Tasks.Docs.html`; `Mix.Tasks.Hex.Publish.html` to `hex.pm/docs/publish`).
- **Ecto/Phoenix docs:** Moderate discovery. `Ecto.html` to `Ecto.Changeset.html`/`Ecto.Query.html`/`Ecto.Repo.html`; `contexts.html` to `your_first_context.html` to `directory_structure.html`.
- **Dialyxir docs:** Minimal discovery — only one page (`Mix.Tasks.Dialyzer.html`) was needed, plus the GitHub README.

## Sources by topic

Legend:
- **Origin:** seed = provided in the initial URL list; discovered = found by following links at depth 1-3.
- **Authority:** official primary = the tool's own documentation site; official secondary = source code or CHANGELOG from the official repo; community = third-party resource.
- **Coverage:** fully explored = extensively quoted and extracted; partially explored = specific sections/functions used; reference only = cited for confirmation or cross-reference, not deeply extracted.

### 1. Elixir official guides (`hexdocs.pm/elixir/*.html`)

Narrative documentation pages (hyphenated names), not module API reference.

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/elixir/basic-types.html | seed | Integers, floats, booleans, nil, atoms, strings, charlists; numeric literals, arithmetic, equality | language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/lists-and-tuples.html | seed | Lists, tuples, immutability, list operations, prepend/concat | language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/binaries-strings-and-charlists.html | seed | Binaries, strings (UTF-8), charlists, `<>` concatenation, `byte_size` vs `String.length` | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/keywords-and-maps.html | seed | Keyword lists, maps, map syntax, access patterns, `Map` vs keyword | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/structs.html | seed | `defstruct`, default fields, `__struct__`, `struct!/2`, `@enforce_keys`, pattern matching with `%` | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/sigils.html | seed | `~c`/`~s`/`~S` sigils, heredocs, interpolation, `~S"""` for docs | language-fundamentals, core-modules, documentation-and-publishing, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/elixir/operators.html | seed | Operator precedence table, operator inventory, arithmetic/comparison/boolean/pipe operators | language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/basic-operators.html | discovered | (DEAD — now 404; content folded into `operators.html` and `Kernel.html`) | language-fundamentals | official primary | reference only |
| https://hexdocs.pm/elixir/patterns-and-guards.html | seed | Pattern matching, guard expressions, guard-safe BIFs, pin operator `^` | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/pattern-matching.html | discovered | Pattern matching basics, `=` match operator, destructuring | core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/case-cond-and-if.html | seed | `case/2`, `cond/1`, `if/2`, `unless/2`, control flow narrative | language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/docs-tests-and-with.html | seed | `with/1` idiom, nested `case` to `with` refactor, doctests intro | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/modules-and-functions.html | seed | `defmodule`, `def`/`defp`, function clauses, default args, function head rule, capture operator `&` | language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/alias-require-and-import.html | seed | `alias`/`require`/`import`/`use` directives, lexical scope, `as:`, multi-alias | language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/module-attributes.html | seed | Module attributes as annotations, temporary storage, compile-time constants, `@behaviour`, `@impl` | language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/protocols.html | seed | `defprotocol`, `defimpl`, dispatch rules, `@derive`, `@fallback_to_any`, built-in protocols | language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/recursion.html | seed | Loops through recursion, reduce/map algorithms, tail-call optimization | language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/enumerable-and-streams.html | seed | Eager vs lazy, `Enum` vs `Stream`, pipe operator, streams | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/enum-cheat.html | discovered | Enum cheatsheet, common Enum patterns | language-fundamentals, core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/comprehensions.html | seed | `for/1`, generators, filters, `:into`, bitstring generators, `:reduce`, `:uniq` | language-fundamentals, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/processes.html | seed | `spawn`, `send`/`receive`, links, tasks, stateful processes, BEAM concurrency model | language-fundamentals, concurrency-processes | official primary | fully explored |
| https://hexdocs.pm/elixir/typespecs.html | seed | `@type`/`@typep`/`@opaque`, `@spec`, `@callback`, `@macrocallback`, built-in types, literals, maps, behaviours, `string()` pitfall | language-fundamentals, naming-conventions, typespecs-and-dialyzer | official primary | fully explored |
| https://hexdocs.pm/elixir/naming-conventions.html | seed | `snake_case`, `CamelCase`, `?` predicates, `!` raising variants, `is_` guard prefix, filenames, pseudo-variables | naming-conventions, core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/writing-documentation.html | seed | `@moduledoc`, `@doc`, `@typedoc`, `@deprecated`, `:since`/`:deprecated`/`:group` metadata, doctests, `~S` heredocs | documentation-and-publishing | official primary | fully explored |
| https://hexdocs.pm/elixir/introduction-to-mix.html | seed | Mix project structure, `mix.exs`, environments (`:dev`/`:test`/`:prod`), `MIX_ENV`, `Mix.env/0` | mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/elixir/introduction.html | discovered | (checked; no naming mentions found) | naming-conventions | official primary | reference only |
| https://hexdocs.pm/elixir/library-guidelines.html | discovered | Library structure guidelines, module/file naming conventions | naming-conventions | official primary | partially explored |
| https://hexdocs.pm/elixir/design-anti-patterns.html | discovered | Using application configuration for libraries (anti-pattern) | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/elixir/compatibility-and-deprecations.html | discovered | `Application.get_env` in module body deprecation, `warn` to `warning` rename, version compatibility | concurrency-processes, configuration-and-runtime, core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/optional-syntax.html | discovered | Optional syntax, code point `?c`, multi-line strings | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/elixir/1.20/typespecs.html | discovered | Version-specific typespecs page (v1.20) for cross-checking | typespecs-and-dialyzer | official primary | partially explored |

### 2. Elixir core modules (`hexdocs.pm/elixir/Module.html`)

API reference pages for Elixir's standard library modules.

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/elixir/Kernel.html | seed | `spawn`/`send`/`self`, `raise`/`reraise`/`exit`/`throw`, `def`/`defmodule`/`defstruct`/`struct!`/`defprotocol`/`defimpl`/`use`/`if`/`unless`/`match?`/pipe operator | concurrency-processes, configuration-and-runtime, core-modules, error-handling, language-fundamentals, naming-conventions, testing-exunit | official primary | fully explored |
| https://hexdocs.pm/elixir/Kernel.SpecialForms.html | seed | `receive`, `for`, `<<>>` bitstring constructor, `alias`/`import`/`require`, `%` struct, `__STACKTRACE__` | concurrency-processes, core-modules, error-handling, language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/Process.html | seed | `spawn`/`send`/`send_after`/`link`/`monitor`/`demonitor`/`info`/`alive?`/`list`/`register`/`flag` | concurrency-processes, error-handling, otp-supervision | official primary | fully explored |
| https://hexdocs.pm/elixir/System.html | seed | `env`/`cmd`/`time`/`trap_signal`/`fetch_env!`/`EnvError`, `schedulers_online` | concurrency-processes, configuration-and-runtime | official primary | fully explored |
| https://hexdocs.pm/elixir/System.EnvError.html | discovered | `System.EnvError` exception, `fetch_env!` error | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/elixir/Config.html | seed | Config DSL, `config`/`import_config`, `config.exs` vs `runtime.exs` | configuration-and-runtime, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/elixir/Config.Provider.html | discovered | Config providers, `init/1`/`load/2`, release config injection | configuration-and-runtime, mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/elixir/Config.Reader.html | discovered | `Config.Reader.read!/2`, config file reading | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/elixir/Application.html | seed | Application environment, `get_env`/`put_env`/`compile_env`/`get_application`, `start`/`stop` | configuration-and-runtime | official primary | fully explored |
| https://hexdocs.pm/elixir/IO.html | seed | `puts`/`inspect`/`binread`/`binwrite`, chardata/iodata, `iodata_to_binary`/`iodata_length` | configuration-and-runtime, core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/IO.ANSI.html | discovered | `enabled?`/`format`/`syntax_colors`, ANSI escape codes | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/elixir/IO.Stream.html | discovered | `IO.Stream` as enumerable, `IO.binstream` | core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/Inspect.Opts.html | discovered | `:limit`/`:syntax_colors`/`:base`, inspect options | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/elixir/Enum.html | seed | Eager enumeration: `map`/`reduce`/`filter`/`sort`/`group_by`/`chunk_every`/`take`/`drop`/`zip`/`into`/`member?`/`count` | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/Stream.html | seed | Lazy enumeration: `map`/`filter`/`cycle`/`resource`/`unfold`/`iterate`, lazy composition | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/File.html | discovered | `File.stream!/1` built on `Stream.resource/3` | core-modules, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/File.Stream.html | discovered | `File.Stream!` struct, file-backed lazy stream | core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/Enumerable.html | seed | `reduce/3` core, `count/1`/`member?/2`/`slice/1` optimizations, protocol dispatch | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/Collectable.html | seed | `into/1`, `Enum.into/2`, collectable protocol | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/Range.html | discovered | `Range.t`, `Enumerable.slice/1` for O(1) count/member/at | core-modules, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Map.html | seed | Map API: `get`/`put`/`delete`/`merge`/`drop`/`take`/`update!`/`get_and_update`/`has_key?`/`keys`/`values` | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/MapSet.html | discovered | `MapSet` set API, membership, `Enumerable`/`Collectable` | core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/Access.html | seed | Access behaviour, `get_in`/`put_in`/`update_in`/`pop_in`/`get_and_update_in`, key-based access | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/Keyword.html | seed | Keyword list API: `get`/`put`/`delete`/`merge`/`take`/`drop`/`keyword?`/`keys`/`values` | core-modules, language-fundamentals | official primary | fully explored |
| https://hexdocs.pm/elixir/Module.html | seed | `@behaviour`/`@impl`/`@callback`/`@before_compile`/`@after_compile`/`@after_verify`/`@on_definition`/`@on_load`/`@external_resource`/`@compile`/`@deprecated`/`register_attribute/3` | core-modules, documentation-and-publishing, language-fundamentals, naming-conventions, testing-exunit | official primary | fully explored |
| https://hexdocs.pm/elixir/List.html | seed | List API: `first`/`last`/`delete`/`insert_at`/`replace_at`/`wrap`/`flatten`/`foldl`/`foldr`/`duplicate`/`to_tuple`/`to_charlist` | core-modules | official primary | fully explored |
| https://hexdocs.pm/elixir/Tuple.html | discovered | Tuple API: `to_list`/`to_map`/`append`/`insert_at`/`delete_at`/`duplicate` | core-modules | official primary | partially explored |
| https://hexdocs.pm/elixir/String.html | seed | UTF-8 encoding, graphemes vs codepoints, escape table, `String` vs `:binary` guidance, `length`/`byte_size`/`split`/`replace`/`trim`/`contains?`/`upcase`/`downcase` | core-modules, language-fundamentals, naming-conventions | official primary | fully explored |
| https://hexdocs.pm/elixir/Regex.html | discovered | PCRE regex, modifiers, `match?`/`replace`/`split`/`scan`/`run`/`named_captures` | core-modules, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Function.html | discovered | `Function.capture/3`/`info/1,2`, local vs external functions | core-modules, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Exception.html | discovered | Exception module, `message/1`/`exception/1`, `format/2,3` | error-handling | official primary | partially explored |

| https://hexdocs.pm/elixir/Task.html | discovered | `start`/`start_link`/`async`/`await`/`async_stream`/`shutdown`/`yield` | concurrency-processes, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Task.Supervisor.html | discovered | `start_link`/`start_child`/`async`/`async_nolink`/`terminate_child` | concurrency-processes | official primary | partially explored |
| https://hexdocs.pm/elixir/GenServer.html | seed | Callback contract (`init`/`handle_call`/`handle_cast`/`handle_info`/`handle_continue`/`terminate`/`code_change`/`format_status`), `call`/`cast`/`reply`, `:via` registration | concurrency-processes, language-fundamentals, otp-supervision | official primary | fully explored |
| https://hexdocs.pm/elixir/Agent.html | discovered | `start_link`/`get`/`update`/`get_and_update`/`cast`/`stop`, stateful process abstraction | concurrency-processes, language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Node.html | discovered | `alive?`/`self`/`list`/`set_cookie`/`connect`/`disconnect`/`spawn`/`ping` | concurrency-processes | official primary | partially explored |
| https://hexdocs.pm/elixir/Port.html | discovered | `open`/`command`/`info`/`close`/`list`, port communication | concurrency-processes | official primary | partially explored |
| https://hexdocs.pm/elixir/Supervisor.html | discovered | Child specs, `start_link`/`init`/`start_child`/`terminate_child`/`which_children`/`count_children`, restart strategies | otp-supervision | official primary | partially explored |
| https://hexdocs.pm/elixir/Registry.html | seed | `:via` registration, `:unique`/`:duplicate` keys, partitioning, `dispatch/4`/`register`/`lookup`/`unregister` | otp-supervision | official primary | fully explored |
| https://hexdocs.pm/elixir/DynamicSupervisor.html | seed | `start_link`/`start_child`/`terminate_child`/`which_children`, dynamic child management | otp-supervision | official primary | fully explored |
| https://hexdocs.pm/elixir/PartitionSupervisor.html | discovered | Scaling/partitioning, `resize!/2`/`start_link` | otp-supervision | official primary | partially explored |
| https://hexdocs.pm/elixir/Code.html | discovered | `string_to_quoted`/`eval_string`/`compile_string`/`require_file`/`ensure_compiled`/`fetch_docs`/`put_compiler_option` | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/elixir/Version.html | discovered | `parse/1`/`compare/2`/`match?/2`, semver requirements, `~>` operator | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/elixir/OptionParser.html | discovered | `parse/2`/`parse!/2`/`split/1`, CLI option parsing | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/elixir/Protocol.html | discovered | `@protocol`/`@for`, consolidation, `@undefined_impl_description`, multiple impls | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Inspect.html | discovered | `@derive {Inspect, ...}`, `#User<...>` notation, inspect protocol | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/String.Chars.html | discovered | `to_string/1`, string conversion protocol | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/List.Chars.html | discovered | `to_charlist/1`, charlist conversion protocol | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Date.html | discovered | `~D` sigil struct, calendar date | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/Time.html | discovered | `~T` sigil struct, time of day | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/NaiveDateTime.html | discovered | `~N` sigil struct, naive datetime | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/DateTime.html | discovered | `~U` sigil struct, timezone-aware datetime | language-fundamentals | official primary | partially explored |
| https://hexdocs.pm/elixir/CaseClauseError.html | discovered | No-clause-match error in `case` | language-fundamentals | official primary | reference only |
| https://hexdocs.pm/elixir/CondClauseError.html | discovered | No-clause-match error in `cond` | language-fundamentals | official primary | reference only |
| https://hexdocs.pm/elixir/WithClauseError.html | discovered | No-clause-match error in `with` | language-fundamentals | official primary | reference only |
| https://hexdocs.pm/elixir/FunctionClauseError.html | discovered | No function clause matches | language-fundamentals | official primary | reference only |

**Function-level anchor references** (specific functions cited within the above module pages):

| Anchor URL | Parent module | Functions referenced | Used by |
|---|---|---|---|
| `Kernel.html#if/2` | Kernel | `if/2`, `unless/2`, `match?/2` | language-fundamentals, testing-exunit |
| `Kernel.html#def/2` | Kernel | `def/2` signature | language-fundamentals |
| `Kernel.html#defmodule/2` | Kernel | `defmodule/2` macro | language-fundamentals |
| `Kernel.html#use/2` | Kernel | `use/2` macro | language-fundamentals |
| `Kernel.html#defstruct/1` | Kernel | `defstruct/1` macro | language-fundamentals |
| `Kernel.html#struct!/2` | Kernel | `struct!/2` | language-fundamentals |
| `Kernel.html#defprotocol/2` | Kernel | `defprotocol/2` macro | language-fundamentals |
| `Kernel.html#defimpl/3` | Kernel | `defimpl/3` macro | language-fundamentals |
| `Kernel.html#%7C%3E/2` | Kernel | pipe operator `|>` | language-fundamentals |
| `Kernel.html#spawn/1` | Kernel | `spawn/1`, `spawn_link/1`, `send/2`, `self/0` | language-fundamentals |
| `Kernel.html#match?/2` | Kernel | `match?/2` macro | testing-exunit |
| `Kernel.SpecialForms.html#for/1` | Kernel.SpecialForms | `for/1` comprehension | language-fundamentals, core-modules |
| `Kernel.SpecialForms.html#__STACKTRACE__/0` | Kernel.SpecialForms | `__STACKTRACE__/0` | error-handling |
| `Kernel.SpecialForms.html#%3C%3C%3E%3E/1` | Kernel.SpecialForms | `<<>>` bitstring constructor | language-fundamentals |
| `Kernel.SpecialForms.html#alias/2` | Kernel.SpecialForms | `alias/2` | language-fundamentals |
| `Kernel.SpecialForms.html#import/2` | Kernel.SpecialForms | `import/2` | language-fundamentals |
| `Kernel.SpecialForms.html#require/2` | Kernel.SpecialForms | `require/2` | language-fundamentals |
| `Kernel.SpecialForms.html#receive/1` | Kernel.SpecialForms | `receive/1` | language-fundamentals |
| `File.html#stream!/1` | File | `File.stream!/1` | language-fundamentals |
| `Module.html#register_attribute/3` | Module | `Module.register_attribute/3` | testing-exunit |
| `ExUnit.html#configure/1` | ExUnit | `ExUnit.configure/1` | testing-exunit |

### 3. Mix docs (`hexdocs.pm/mix/*.html`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/mix/Mix.html | seed | Mix overview, project definition, environments (`:dev`/`:test`/`:prod`), targets, compilers, aliases, `Mix.env/0`/`Mix.target/0` | configuration-and-runtime, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Project.html | seed | `use Mix.Project`, `project/0`/`application/0`, project config keys, `compile`/`deps`/`start_permanent` | mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Task.html | seed | `Mix.Task` behaviour, `run/1` callback, `@shortdoc`/`@moduledoc`/`@recursive`/`@preferred_cli_env`, custom task creation | mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.SCM.html | discovered | SCM behaviour, `:git`/`:hex` strategies, `fetch`/`checkout`/`accepts_options` | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Release.html | discovered | Release internals, `Mix.Release` struct, steps, overlays, `:vm_args`/`:config_providers`/`:strip_beams` | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Help.html | discovered | `mix help` forms: `--search`/`--names`/`--aliases`/`TASK`/`MODULE`/`MODULE.FUN`/`app:APP`/`c:MODULE.NAME`/`t:MODULE.NAME` | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.New.html | discovered | `mix new` generator, `--sup`/`--umbrella`/`--app` flags | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html | discovered | `.app` file generation, `def application` | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Format.html | seed | `mix format`, `.formatter.exs`, `:inputs`/`:subdirectories`/`:plugins`/`:import_deps`/`:export`, code formatting | mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.html | seed | `mix deps`, dependency listing, paths, SCM info, `--tree` | dependencies-and-packages, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html | seed | `mix deps.get`, `--check-locked` flag, lockfile fetching | dependencies-and-packages, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html | seed | `mix deps.clean`, `--all`/`--unlock`/`--build`/`--unused`/`--only` flags | dependencies-and-packages, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Compile.html | discovered | `mix deps.compile`, compilation of dependencies | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Tree.html | discovered | `mix deps.tree`, dependency graph visualization | dependencies-and-packages, mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html | seed | `mix deps.unlock`, `--check-unused`/`--unused`/`--all` flags, lockfile management | dependencies-and-packages, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html | seed | `mix deps.update`, `--all`/`--only`/`--target`/`--no-archives-check` flags, constraint-respecting updates | dependencies-and-packages, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Release.html | seed | `mix release`, `config_providers`/`runtime.exs`/`:validate_compile_env`, release configuration | configuration-and-runtime, mix-project-structure | official primary | fully explored |
| https://hexdocs.pm/mix/Mix.Tasks.Release.Init.html | discovered | `mix release.init`, release config template generation | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Test.html | discovered | `mix test`, `--only`/`--exclude`/`--seed`/`--trace`/`--cover`/`--failed`/`--stale` flags, test file pattern | mix-project-structure, testing-exunit | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Test.Coverage.html | discovered | `mix test.coverage`, coverage reporting | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Clean.html | discovered | `mix clean`, `--deps`/`--deps --unlock` flags | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Run.html | discovered | `mix run`, `-e`/`-r`/`--no-halt` flags, script execution | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Cmd.html | discovered | `mix cmd`, command execution across umbrella children | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Do.html | discovered | `mix do`, multi-task composition | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Xref.html | discovered | `mix xref`, cross-reference analysis, `calls`/`graph`/`warnings` modes | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Archive.html | discovered | `mix archive`, archive management overview | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Archive.Install.html | discovered | `mix archive.install`, archive installation | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Archive.Uninstall.html | discovered | `mix archive.uninstall`, archive removal | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html | discovered | `mix archive.build`, archive creation | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Local.html | discovered | `mix local`, local tasks listing | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Local.Hex.html | discovered | `mix local.hex`, Hex archive installation | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Local.Rebar.html | discovered | `mix local.rebar`, Rebar3 installation | mix-project-structure | official primary | partially explored |
| https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html | discovered | `mix escript.build`, escript generation | mix-project-structure | official primary | partially explored |

### 4. ExUnit docs (`hexdocs.pm/ex_unit/*.html`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/ex_unit/ExUnit.html | seed | `ExUnit.start/0,1`/`configure/1`/`run/0`/`async_run/0`, configuration options, `configure_opts` type, application env defaults | testing-exunit | official primary | fully explored |
| https://hexdocs.pm/ex_unit/ExUnit.Case.html | seed | `use ExUnit.Case`, `test/1,3`/`describe/2`/`register_test/6`, `:async`/`:group`/`:parameterize`/`:register` options | testing-exunit | official primary | fully explored |
| https://hexdocs.pm/ex_unit/ExUnit.Assertions.html | discovered | `assert`/`refute`/`assert_in_delta`/`assert_raise`/`assert_receive`/`refute_receive`/`catch_error`/`catch_exit`/`catch_throw`/`flunk` | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/ex_unit/ExUnit.AssertionError.html | discovered | `ExUnit.AssertionError` struct, assertion error fields | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html | discovered | `setup`/`setup_all`/`on_exit`/`start_supervised`/`start_supervised!`/`start_link_supervised!`/`stop_supervised`/`stop_supervised!` | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/ex_unit/ExUnit.DocTest.html | seed | `doctest/2`/`doctest_file/2`, `:only`/`:except`/`:import`/`:tags`/`:inspect_opts`, multiline `...>` continuation | documentation-and-publishing, testing-exunit | official primary | fully explored |
| https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html | discovered | `capture_io/1,2,3`/`with_io/1,2,3`, IO capture for testing | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html | discovered | `capture_log/2`/`with_log/2`, log capture for testing, level filtering | testing-exunit | official primary | partially explored |
| https://hexdocs.pm/ex_unit/ExUnit.Formatter.html | discovered | Formatter behaviour, `ExUnit.CLIFormatter`, event callbacks | testing-exunit | official primary | partially explored |

### 5. Logger docs (`hexdocs.pm/logger/*.html`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/logger/Logger.html | seed | Log levels, `Logger.configure/1`, compile-time purging, `:compile_time_purge_matching`, 1.15 handler migration, `:default_handler`/`:default_formatter`, scoped levels, `Logger.Backends.Console` deprecation | configuration-and-runtime, testing-exunit | official primary | fully explored |
| https://hexdocs.pm/logger/Logger.Formatter.html | discovered | `Logger.Formatter`, format string tokens (`$time`/`$message`/`$metadata`/`$level`), `:format`/`:metadata`/`:colors`/`:truncate` options | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/logger/Logger.Backends.Console.html | discovered | Deprecated backend, migration to `:logger_backends` dependency | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/logger/1.17.2/Logger.html | discovered | Historical v1.17.2 Logger docs for cross-checking API evolution | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/logger/1.15.0/Logger.html | discovered | Historical v1.15.0 Logger docs — 1.15 handler migration reference | configuration-and-runtime | official primary | partially explored |
| https://hexdocs.pm/logger/1.10.0/Logger.html | discovered | Historical v1.10.0 Logger docs — pre-handler-migration reference | configuration-and-runtime | official primary | partially explored |

### 6. Erlang/OTP docs (`erlang.org`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://www.erlang.org/doc/system/design_principles.html | seed | OTP design principles, generic/specific split, applications, releases, supervision trees, behaviours | beam-otp-internals, otp-supervision | official primary | fully explored |
| https://www.erlang.org/doc/system/sup_princ.html | seed | Supervision theory, supervisor flags, child specs, restart strategies (`:one_for_one`/`:one_for_all`/`:rest_for_one`/`:simple_one_for_one`) | beam-otp-internals, otp-supervision | official primary | fully explored |
| https://www.erlang.org/doc/system/gen_server_concepts.html | seed | gen_server client-server model, message dispatch, sys support, code change | beam-otp-internals, otp-supervision | official primary | fully explored |
| https://www.erlang.org/doc/system/errors.html | seed | Exception classes (`:error`/`:exit`/`:throw`), exit reasons, exit signals, links, monitors | beam-otp-internals, error-handling | official primary | fully explored |
| https://www.erlang.org/doc/system/release_handling.html | seed | Release handling, `appup`/`relup`, instructions, hot code upgrade | beam-otp-internals | official primary | fully explored |
| https://www.erlang.org/doc/system/spec_proc.html | seed | Special processes, `:sys`/`:proc_lib` integration, system messages/events/callbacks | beam-otp-internals | official primary | fully explored |
| https://www.erlang.org/doc/system/ref_man_processes.html | seed | Process reference, states (`free`/`runnable`/`waiting`/`running`/`exiting`/`garbing`/`suspended`), signals, links, monitors, message passing | beam-otp-internals | official primary | fully explored |
| https://www.erlang.org/doc/system/distributed.html | discovered | Distributed Erlang, nodes, cookies, `net_kernel` | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/system/ports.html | discovered | Ports, port communication, port drivers | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/reference_manual/processes.html | discovered | Process reference manual, spawn, links, monitors, message passing semantics | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/apps/stdlib/proc_lib.html | discovered | `proc_lib` module, `spawn_opt`/`start_link`/`hibernate`/`stop` | beam-otp-internals | official primary | partially explored |
| https://www.erlang.org/doc/apps/stdlib/sys.html | discovered | `sys` module, `get_state`/`get_status`/`replace_state`/`suspend`/`resume`/`change_code` | beam-otp-internals | official primary | partially explored |
| https://www.erlang.org/doc/apps/stdlib/gen_server.html | discovered | Erlang `gen_server` module, callback typespecs, exit signal trapping, `throw` handling | otp-supervision | official primary | partially explored |
| https://www.erlang.org/doc/apps/stdlib/binary.html | discovered | `:binary` Erlang module, `split/2`/`copy/2`/`compile_pattern/1` | language-fundamentals | official primary | partially explored |
| https://www.erlang.org/doc/apps/erts/erlang.html | seed | `erlang` module BIFs: `spawn_opt`/`process_info`/`system_info`/`statistics`/`garbage_collect`/`hibernate`/`process_flag` | beam-otp-internals, concurrency-processes | official primary | fully explored |
| https://www.erlang.org/doc/apps/erts/epmd_cmd.html | discovered | EPMD (Erlang Port Mapper Daemon), node name resolution | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/apps/erts/erl_cmd.html | discovered | `erl` command-line flags, `--sname`/`--name`/`--cookie`/`MIX_ENV` boot | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/apps/kernel/net_kernel.html | discovered | `net_kernel` module, distributed node management, `start/2`/`stop/0`/`monitor_nodes/2` | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/apps/kernel/net_adm.html | discovered | `net_adm` module, `host_file`/`names`/`ping`/`world` | concurrency-processes | official primary | partially explored |
| https://www.erlang.org/doc/apps/kernel/logger.html | discovered | Erlang/OTP `:logger` module, handlers, filters, formatters | configuration-and-runtime | official primary | partially explored |
| https://www.erlang.org/doc/apps/kernel/global.html | discovered | `:global` module, cluster-wide name registration, `register_name`/`whereis_name`/`send` | otp-supervision | official primary | partially explored |
| https://www.erlang.org/doc/apps/kernel/code.html | discovered | `:code` module, code server, `load_file`/`ensure_loaded`/`purge`/`soft_purge` | mix-project-structure | official primary | partially explored |
| https://www.erlang.org/doc/man/binary.html | discovered | `:binary` man page, binary module reference | core-modules | official primary | partially explored |
| https://blog.stenmans.org/theBeamBook/ | discovered | The BEAM Book by Erik Stenman — schedulers, reductions, GC, message passing internals, `CONTEXT_REDS`/`TIW_SIZE`/`max_gen_gcs`. **Repurpose note:** `beam-otp-internals.md` is now a thin Elixir↔BEAM mapping index; this BEAM-Book material backs only the trimmed "VM internals (community-sourced)" section retained there because it has no authoritative `docs/beam/` home (community-sourced; flagged as a `docs/beam/` revisit-later gap). | beam-otp-internals | community | fully explored |

### 7. Credo docs (`hexdocs.pm/credo/*.html`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/credo/overview.html | seed | Credo overview, static analysis philosophy, teaching focus | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/installation.html | seed | Dependency installation, `only: [:dev, :test]`/`runtime: false`, `~> 1.7` pin | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/basic_usage.html | seed | `mix credo`/`--all`/`--strict`, output anatomy, categories, priority arrows, `--min-priority` | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/mix_tasks.html | seed | Task reference: `suggest`/`list`/`diff`/`explain`/`info`/`categories`/`version`/`gen.config`/`gen.check` | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/suggest_command.html | discovered | `mix credo suggest` command details, default behavior, per-category cap | static-analysis-credo | official primary | partially explored |
| https://hexdocs.pm/credo/explain_command.html | discovered | `mix credo explain` command, location string format, JSON explain output | static-analysis-credo | official primary | partially explored |
| https://hexdocs.pm/credo/cli_switches.html | discovered | CLI switches: `--all`/`--all-priorities`/`--strict`/`--only`/`--ignore`/`--min-priority`/`--format`/`--config-file`/`--verbose`/`--watch` | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/config_file.html | seed | `.credo.exs` structure, `:configs` key, named configs, default checks, check params, `strict: true` | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/check_params.html | discovered | Check parameter reference, priority/category/tags configuration | static-analysis-credo | official primary | partially explored |
| https://hexdocs.pm/credo/exit_statuses.html | discovered | Exit status codes, bitwise OR of category codes, `>= 128` runtime errors | static-analysis-credo | official primary | fully explored |
| https://hexdocs.pm/credo/checks.html | discovered | (DEAD — returns 404 in v1.7.x; catalog lives in `config_file.html` "Default checks" + per-check module pages) | static-analysis-credo | official primary | reference only |
| https://hexdocs.pm/credo/config_comments.html | discovered | Inline disable directives, `# credo:disable-for-next-line`/`# credo:disable-for-this-file` | static-analysis-credo | official primary | partially explored |
| https://hexdocs.pm/credo/Credo.Check.html | discovered | Check behaviour, category enum (`:consistency`/`:design`/`:readability`/`:refactor`/`:warning`), `run/2` callback | static-analysis-credo | official primary | partially explored |
| https://hexdocs.pm/credo/Credo.Check.Readability.ModuleNames.html | discovered | Representative per-check module page, check configuration options | static-analysis-credo | official primary | partially explored |

### 8. Dialyxir docs (`hexdocs.pm/dialyxir/*.html`)

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html | seed | `mix dialyzer` task, `--plt`/`--halt-exit-status`/`--format` flags, PLT management, success-typing analysis | typespecs-and-dialyzer | official primary | partially explored |

### 9. ExDoc/Hex docs

ExDoc documentation, Hex package manager docs, and Hex.pm site references.

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/ex_doc/readme.html | seed | ExDoc overview, documentation generation, configuration options | documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html | seed | `mix docs` task, ExDoc configuration, `:main`/`:api_reference`/`:extras`/`:groups_for_modules` options | documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/ex_doc/ExDoc.html | seed | `ExDoc` module, `generate_docs/2`, retrievers, formatters | documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Audit.html | seed | `mix hex.audit`, retired package detection, lockfile-based audit | dependencies-and-packages | official primary | fully explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Outdated.html | seed | `mix hex.outdated`, `--all`/`--pre`/`--sort`/`--only`/`--within-requirements` flags, status values | dependencies-and-packages | official primary | fully explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html | seed | `mix hex.publish`, package publishing, `--revert`/`--replace` flags | dependencies-and-packages, documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html | seed | `mix hex.retire`, package retirement, retirement reasons (`:security`/`:deprecated`/`:invalid`/`:renamed`) | dependencies-and-packages, documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Policy.html | discovered | `mix hex.policy`, dependency policies, policy-gated resolution | dependencies-and-packages | official primary | partially explored |
| https://hexdocs.pm/hex/Hex.Policy.html | discovered | `Hex.Policy` module, policy types, retired/CVE severity/cooldown rules | dependencies-and-packages | official primary | partially explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.User.html | discovered | `mix hex.user`, Hex account management | documentation-and-publishing | official primary | partially explored |
| https://hexdocs.pm/hex/Mix.Tasks.Hex.Build.html | discovered | `mix hex.build`, tarball creation | documentation-and-publishing | official primary | partially explored |
| https://hex.pm | discovered | Hex.pm homepage, package immutability policy | dependencies-and-packages | official primary | partially explored |
| https://hex.pm/docs/publish | discovered | Hex publishing guide, package publishing workflow | documentation-and-publishing | official primary | partially explored |
| https://hex.pm/packages/credo | discovered | Credo package page on hex.pm, version info | static-analysis-credo | official primary | reference only |
| https://osv.dev/list?ecosystem=Hex | discovered | OSV vulnerability database, Hex ecosystem advisories | dependencies-and-packages | community | reference only |
| https://hexdocs.pm/stream_data/ | discovered | StreamData library, property-based testing (separate from ExUnit) | testing-exunit | official primary | reference only |

### 10. Ecto/Phoenix docs

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://hexdocs.pm/ecto/Ecto.html | seed | Ecto overview, 4 components (Repo/Schema/Query/Changeset), data-flow stack | ecto-phoenix-patterns | official primary | fully explored |
| https://hexdocs.pm/ecto/Ecto.Changeset.html | seed | `cast/4` vs `change/2`, validations vs constraints, `validate_required`/`validate_format`/`unique_constraint`, embeds, schemaless changesets, multiple changeset functions | ecto-phoenix-patterns | official primary | fully explored |
| https://hexdocs.pm/ecto/Ecto.Query.html | discovered | Query language, keyword vs pipe syntax, pinning `^`, nil comparisons, composition, bindings | ecto-phoenix-patterns | official primary | fully explored |
| https://hexdocs.pm/ecto/Ecto.Query.API.html | discovered | Query API functions, operators, field access, `fragment/1` | ecto-phoenix-patterns | official primary | partially explored |
| https://hexdocs.pm/ecto/Ecto.Multi.html | discovered | `Ecto.Multi`, multi-step transactions, `insert`/`update`/`delete`/`run`/`append`/`rollback` | ecto-phoenix-patterns | official primary | partially explored |
| https://hexdocs.pm/ecto/Ecto.Repo.html | seed | `use Ecto.Repo`, `all`/`get`/`insert`/`update`/`delete`/`transaction`, adapter configuration | ecto-phoenix-patterns | official primary | fully explored |
| https://hexdocs.pm/phoenix/contexts.html | seed | Phoenix contexts, bounded contexts, web layer separation, public intent API | ecto-phoenix-patterns | official primary | fully explored |
| https://hexdocs.pm/phoenix/your_first_context.html | discovered | Context creation tutorial, schema + changeset + context pattern | ecto-phoenix-patterns | official primary | partially explored |
| https://hexdocs.pm/phoenix/directory_structure.html | seed | Phoenix directory structure, `lib/my_app_web/` vs `lib/my_app/`, context module placement | ecto-phoenix-patterns | official primary | fully explored |

### 11. GitHub source code references

Source code files from the Elixir, Credo, Dialyxir, and mix_audit repositories.

| URL | Origin | Topics extracted | Used by | Authority | Coverage |
|---|---|---|---|---|---|
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/agent.ex | discovered | `Agent` module source, client API implementation | concurrency-processes | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/agent/server.ex | discovered | `Agent.Server` source, GenServer-based server loop | concurrency-processes | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.11.0/CHANGELOG.md | discovered | `runtime.exs` introduction (v1.11.0) | configuration-and-runtime | official secondary | reference only |
| https://github.com/elixir-lang/elixir/blob/v1.7/CHANGELOG.md | discovered | `__STACKTRACE__` introduction (v1.7) | configuration-and-runtime | official secondary | reference only |
| https://github.com/elixir-lang/elixir/blob/v1.12/CHANGELOG.md | discovered | `System.trap_signal/3` introduction, `stacktrace` hard-deprecation (v1.12) | configuration-and-runtime | official secondary | reference only |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/string.ex | discovered | `String` module source, UTF-8 implementation | core-modules | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/kernel/special_forms.ex | discovered | `Kernel.SpecialForms` source, special form definitions | core-modules | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.ex | discovered | `Mix.Tasks.Deps` source, dependency listing implementation | dependencies-and-packages | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.get.ex | discovered | `Mix.Tasks.Deps.Get` source, lockfile fetching implementation | dependencies-and-packages | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.update.ex | discovered | `Mix.Tasks.Deps.Update` source, update implementation | dependencies-and-packages | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.unlock.ex | discovered | `Mix.Tasks.Deps.Unlock` source, unlock implementation | dependencies-and-packages | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/new.ex | discovered | `Mix.Tasks.New` source, project generator implementation | mix-project-structure | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/format.ex | discovered | `Mix.Tasks.Format` source, code formatter implementation | mix-project-structure | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix.ex | discovered | `Mix` module source, project/environment management | mix-project-structure | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex | discovered | `Mix.Task` module source, task behaviour | mix-project-structure | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/pages/references/naming-conventions.md | discovered | Markdown source for the naming-conventions guide page | naming-conventions | official secondary | reference only |
| https://github.com/elixir-lang/elixir/blob/main/CHANGELOG.md | discovered | Registry `:keys` tuple forms (v1.19.0), `{:duplicate, :key}` `ordered_set` layout (v1.20.0) | otp-supervision | official secondary | reference only |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/mix.exs | discovered | ExUnit application env defaults, `:ex_unit` OTP application config | testing-exunit | official secondary | fully explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/callbacks.ex | discovered | `ExUnit.Callbacks` source, `setup`/`setup_all`/`on_exit` implementation | testing-exunit | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_io.ex | discovered | `ExUnit.CaptureIO` source, IO capture implementation | testing-exunit | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_log.ex | discovered | `ExUnit.CaptureLog` source, log capture implementation | testing-exunit | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_server.ex | discovered | `ExUnit.CaptureServer` source, level-filtering implementation | testing-exunit | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/runner.ex | discovered | `ExUnit.Runner` source, capture_log tag wiring | testing-exunit | official secondary | partially explored |
| https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/doc_test.ex | discovered | `ExUnit.DocTest` source, doctest parsing/execution | testing-exunit | official secondary | partially explored |
| https://github.com/mirego/mix_audit | discovered | `mix_audit` community tool, OSV-based vulnerability checks against `mix.lock` | dependencies-and-packages | community | partially explored |
| https://github.com/rrrene/credo/blob/master/lib/credo/cli/command/categories/categories_command.ex | discovered | Category titles/descriptions source, category display | static-analysis-credo | official secondary | partially explored |
| https://github.com/rrrene/credo | discovered | Credo README/source, author/license info | static-analysis-credo | official secondary | reference only |
| https://github.com/jeremyjh/dialyxir | discovered | Dialyxir README, version info, usage | typespecs-and-dialyzer | official secondary | reference only |

## Cross-corpus references

This source-map predates the `docs/beam/` corpus. The Elixir and `docs/beam/` corpora are now aligned as follows:

- **Shared BEAM/OTP runtime material is delegated to `docs/beam/`**, not re-crawled from `erlang.org` in this corpus. The Erlang/OTP system-doc sources in family 6 above remain the historical provenance for the Elixir docs that previously inlined that material; the runtime semantics themselves now live authoritatively in `docs/beam/*.md` (see `docs/elixir/index.md` → "BEAM guidance map" for the per-doc routing). The Elixir docs retain only the Elixir-specific API surface and callback contracts.
- **9 Elixir OTP-wrapper pages were source-verified** against their canonical `elixir.hexdocs.pm` URLs in `docs/elixir/.crawl/E01`–`E09`. These back the Elixir-specific API claims in `otp-supervision.md`, `concurrency-processes.md`, and `configuration-and-runtime.md`, and each crawl records its Elixir↔BEAM mapping. The verified URLs and beam mappings:

| # | Module | Canonical URL | Crawl artifact | BEAM mapping |
|---|---|---|---|---|
| E01 | `GenServer` | https://elixir.hexdocs.pm/GenServer.html | `.crawl/E01-genserver.md` | `docs/beam/gen-server.md` |
| E02 | `Supervisor` | https://elixir.hexdocs.pm/Supervisor.html | `.crawl/E02-supervisor.md` | `docs/beam/supervision.md` |
| E03 | `DynamicSupervisor` | https://elixir.hexdocs.pm/DynamicSupervisor.html | `.crawl/E03-dynamic-supervisor.md` | `docs/beam/supervision.md` |
| E04 | `Registry` | https://elixir.hexdocs.pm/Registry.html | `.crawl/E04-registry.md` | `docs/beam/processes-and-messages.md` |
| E05 | `Task` | https://elixir.hexdocs.pm/Task.html | `.crawl/E05-task.md` | `docs/beam/processes-and-messages.md` + `docs/beam/links-monitors-and-exits.md` |
| E06 | `Process` | https://elixir.hexdocs.pm/Process.html | `.crawl/E06-process.md` | `docs/beam/processes-and-messages.md` + `docs/beam/links-monitors-and-exits.md` |
| E07 | `Application` | https://elixir.hexdocs.pm/Application.html | `.crawl/E07-application.md` | `docs/beam/applications.md` |
| E08 | `Task.Supervisor` | https://elixir.hexdocs.pm/Task.Supervisor.html | `.crawl/E08-task-supervisor.md` | `docs/beam/supervision.md` |
| E09 | `Agent` | https://elixir.hexdocs.pm/Agent.html | `.crawl/E09-agent.md` | `docs/beam/gen-server.md` |

All 9 crawls fetched HTTP 200 against Elixir v1.20.2 (ExDoc v0.40.3); no post-v1.20.2 drift was detected in the extracted API surface. The `hexdocs.pm/elixir/<Module>.html` seed URLs canonical-redirect to `elixir.hexdocs.pm/<Module>.html`.

## Revisit later

- **P1/P2 Elixir HexDocs verification (the broader ~60-URL set).** Only the 9 P0 OTP-wrapper crawls above are persisted in `.crawl/`. The remaining P1 (≈20 URLs: `Kernel`, `Kernel.SpecialForms`, `IO`, `typespecs.html`, `Mix.Tasks.Dialyzer`, `ExUnit.*`, `Mix.Tasks.Test`/`Release`, `Ecto.*`, `Phoenix contexts`/`directory_structure`, `Module`) and P2 (≈60 URLs: guide pages + peripheral modules + Credo per-check pages) HexDocs URLs from the inspection plan are not yet crawl-backed. Their claims rest on the existing 227-URL provenance above but are not yet re-verified against current HexDocs.
- **VM-internals community source.** The BEAM Book (`blog.stenmans.org/theBeamBook/`) backs the schedulers/reductions/GC/memory-model material retained in the repurposed `beam-otp-internals.md`. This material is community-sourced and has **no authoritative `docs/beam/` home**; it awaits a future `docs/beam/` "VM internals" doc to fold into. Until then it is kept trimmed and clearly flagged in `beam-otp-internals.md`.
- **`beam-otp-internals.md` repurpose.** The file is now a thin Elixir↔BEAM mapping index (inbound references from `workflows/debugging.md` and several "Related docs" sections are preserved by keeping the filename). Its former erlang.org system-doc sources (family 6) are now cross-referenced via `docs/beam/` rather than inlined.

