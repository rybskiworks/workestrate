# Elixir Corpus Inspection — Repair & Alignment Plan

> **Pass type:** INSPECTION (research/analysis only). No corpus files were modified.
> **Scope:** `docs/elixir/*.md` (16 topic docs + `source-map.md` + 6 workflows) aligned against `docs/beam/` (22 docs).
> **Date:** 2026-06-25.
> **Inspector:** `lead_long`.

## 0. Headline findings

1. **Zero `docs/beam` references exist anywhere in the Elixir corpus** (verified by grep across `*.md` and `workflows/*.md`). The Elixir corpus predates the BEAM corpus (Elixir files mtime Jun 24 16:10–20:20; BEAM files Jun 24 22:56–00:10). The entire alignment (de-dup + cross-link) is greenfield work.
2. **No `docs/elixir/.crawl/` directory exists** — unlike `docs/beam/.crawl/` which persists 47 crawled source files with verbatim quotations. The Elixir docs list "Sources used" but claims are not yet backed by persisted crawl artifacts. "Repair from source" therefore means *establishing* a `.crawl/` with fetched HexDocs pages + verbatim quotations, mirroring the BEAM repair model.
3. **`beam-otp-internals.md` (1110 lines) is the prime de-dup candidate** — ~60% of it (supervision theory, exit signals, sys/proc_lib, releases, design principles) is now duplicated authoritatively in `docs/beam/`; ~40% (schedulers/GC/memory-model/timing-wheel) is BEAM-Book *community*-sourced and has **no authoritative home in `docs/beam/`** (a coverage gap).
4. **Stale template artifact:** 4 Elixir docs cross-reference `docs/rust/style-formatting.md` "for formatting conventions and document tone" (`otp-supervision`, `naming-conventions`, `mix-project-structure`, `static-analysis-credo`). The file exists, but the reference is a leftover from a Rust-derived template and is semantically wrong for an Elixir corpus.
5. **Source-map is internally consistent** (227 URLs, 11 families, 15 sourced topic files, 2 documented dead links retained for provenance) but predates `docs/beam/` and records no BEAM cross-reference relationship.

---

## 1. Per-file classification table

Classification key: `KEEP` (good Elixir-specific, leave) · `LINK_TO_BEAM` (good but should link docs/beam for shared runtime) · `DE_DUP_BEAM` (long duplicated BEAM explanation → trim to summary + docs/beam link) · `FIX_BROKEN_REFERENCE` · `REPAIR_FROM_SOURCE` (shallow/stale/unsourced Elixir-specific, needs HexDocs verify) · `REVIEW_LATER`.

| # | File | Primary classification | Secondary | Rationale (1–2 lines) |
|---|---|---|---|---|
| 1 | `beam-otp-internals.md` | **DE_DUP_BEAM** | REPAIR_FROM_SOURCE | 1110 lines, ~60% duplicates docs/beam (supervision/links/sys/releases). ~40% VM-internals from BEAM Book (community) with no docs/beam home. Prime repurpose target. |
| 2 | `concurrency-processes.md` | **DE_DUP_BEAM** | LINK_TO_BEAM | "BEAM concurrency model", "Links vs monitors (deep comparison)", "trap_exit and exit signals", "Heap/stack/reductions/GC" duplicate docs/beam. Task/Agent/Node/Port API is Elixir-specific KEEP. |
| 3 | `configuration-and-runtime.md` | **LINK_TO_BEAM** | — | Logger/Config/Application/System are Elixir-specific KEEP (well-sourced). Add "Related BEAM guidance" → logger-and-config.md + applications.md. |
| 4 | `core-modules.md` | **KEEP** | — | Enum/Stream/Map/Keyword/IO/Access — purely Elixir-specific, well-sourced, no BEAM overlap. |
| 5 | `dependencies-and-packages.md` | **KEEP** | — | Mix.Tasks.Deps.* / Hex.* — Elixir-specific, well-sourced (v1.20.2 / Hex 2.4.2). |
| 6 | `documentation-and-publishing.md` | **KEEP** | — | @moduledoc/@doc/ExDoc/Hex — Elixir-specific, well-sourced. |
| 7 | `ecto-phoenix-patterns.md` | **KEEP** | — | Ecto/Phoenix contexts — Elixir-specific, well-sourced (Ecto 3.14 / Phoenix 1.8.8). |
| 8 | `error-handling.md` | **DE_DUP_BEAM** | REPAIR_FROM_SOURCE, LINK_TO_BEAM | "Let It Crash" + "Process.exit vs Kernel.exit/links/monitors/trap_exit" duplicate docs/beam/links-monitors-and-exits.md. Thin sourcing (only 6 sources; exit-signal theory is unsourced, inherited from beam-otp-internals). |
| 9 | `index.md` | **FIX_BROKEN_REFERENCE** | LINK_TO_BEAM | Must reflect beam-otp-internals repurpose + add docs/beam relationship note. (No Sources section — meta index.) |
| 10 | `language-fundamentals.md` | **KEEP** | — | Types/operators/pattern-matching/guards/control-flow — Elixir-specific, very well-sourced (30+ guide pages). |
| 11 | `mix-project-structure.md` | **LINK_TO_BEAM** | FIX_BROKEN_REFERENCE | Releases section → docs/beam/releases.md; .app generation → docs/beam/applications.md. Also has stale `docs/rust/style-formatting.md` ref. |
| 12 | `naming-conventions.md` | **FIX_BROKEN_REFERENCE** | KEEP | Stale `docs/rust/style-formatting.md` ref. Otherwise Elixir-specific, well-sourced. |
| 13 | `otp-supervision.md` | **DE_DUP_BEAM** | LINK_TO_BEAM | GenServer/Supervisor *theory* intros duplicate docs/beam/gen-server.md + supervision.md. Elixir API (child_spec/1, DynamicSupervisor, Registry, PartitionSupervisor, :via) is KEEP. |
| 14 | `static-analysis-credo.md` | **FIX_BROKEN_REFERENCE** | KEEP | Stale `docs/rust/` ref. Otherwise Elixir-specific, well-sourced (Credo v1.7.x). |
| 15 | `testing-exunit.md` | **KEEP** | — | ExUnit — Elixir-specific, well-sourced (incl. source files). |
| 16 | `typespecs-and-dialyzer.md` | **REPAIR_FROM_SOURCE** | — | Thin Dialyxir sourcing (1 HexDocs page + README). Verify PLT/flags/ignore-file/warning-categories against HexDocs. Typespec half is well-sourced. |

**Tally:** KEEP = 6 · LINK_TO_BEAM = 1 (pure) · DE_DUP_BEAM = 4 · REPAIR_FROM_SOURCE = 1 (pure) + 2 secondary · FIX_BROKEN_REFERENCE = 4. (Several docs carry a primary + secondary classification.)

---

## 2. BEAM-overlap map

### 2.1 `beam-otp-internals.md` — section-by-section de-dup map

| beam-otp-internals section (line range) | Duplicates / relates to | Action |
|---|---|---|
| ERTS and the node concept (29–47) | `docs/beam/overview.md`, `docs/beam/distribution.md` | Trim to 2-line summary + link. |
| Schedulers (49–65) | **No docs/beam home** (BEAM Book, community) | KEEP trimmed; flag as community-sourced + coverage gap. |
| Dirty schedulers (67–76) | `docs/beam/nifs.md` (dirty schedulers) | Trim to 1-line + link nifs.md. |
| Reductions and preemption (78–93) | `docs/beam/processes-and-messages.md` §"Process states and reductions" (high-level only) | KEEP trimmed (docs/beam is shallow here); note gap. |
| Process state machine (95–109) | `docs/beam/processes-and-messages.md` | Trim to summary table + link. |
| Priority queues / Load balancing / Timing wheel (111–141) | **No docs/beam home** (BEAM Book) | KEEP trimmed; flag community-sourced + gap. |
| Ports (142–148) | `docs/beam/ports-io.md` | Trim to 1-line + link. |
| Process memory model / Garbage collection (149–187) | **No docs/beam home** (BEAM Book) | KEEP trimmed; flag community-sourced + gap. |
| Message passing / Process dictionary (189–213) | `docs/beam/processes-and-messages.md` | Trim to summary + link. |
| Key `:erlang` functions / Connect-to-Elixir tables (214–236) | `docs/beam/runtime-debugging.md` (introspection BIFs) | KEEP the Erlang→Elixir mapping tables (genuinely Elixir-specific, useful). |
| OTP Design Principles (237–314) | `docs/beam/overview.md` + `docs/beam/otp-behaviours.md` + `docs/beam/applications.md` | DE_DUP heavily → 1-line per concept + link. |
| Supervision Theory (316–435) | `docs/beam/supervision.md` (near-total duplicate) | DE_DUP → remove; replace with "See docs/beam/supervision.md" + Elixir API one-liner. |
| Error Handling and Exit Signals (437–583) | `docs/beam/links-monitors-and-exits.md` (near-total duplicate) | DE_DUP → remove; replace with link + Elixir API one-liner. |
| Debugging with sys and proc_lib (585–747) | `docs/beam/proc-lib-and-sys.md` + `docs/beam/runtime-debugging.md` | DE_DUP → remove; replace with link. |
| Release Handling and Hot Code Upgrade (749–856) | `docs/beam/releases.md` (near-total duplicate) | DE_DUP → remove; replace with link + Elixir `mix release` note. |
| Checklists / Examples / Common mistakes / Strict-vs-contextual / Policy (858–1097) | Mixed; some Elixir-specific, some BEAM-duplicate | KEEP Elixir-flavored items; drop BEAM-theory duplicates (now in docs/beam checklists). |
| Related docs / Related skills (1099–1110) | — | Rewrite to point at docs/beam; fix "No opencode skills" note (elixir-* skills now exist). |

**Recommendation: REPURPOSE as a thin "Elixir ↔ BEAM mapping index".**

Rationale:
- The Erlang→Elixir API mapping tables ("Connect to Elixir") are genuinely useful and Elixir-specific (docs/beam is Erlang-native and has no such mapping).
- The VM-internals summary (schedulers/reductions/GC/memory) has no home in docs/beam and is useful for Elixir devs — keep a *trimmed* version, clearly marked BEAM-Book/community-sourced, with a note that docs/beam/processes-and-messages.md covers process-level semantics.
- Folding into `index.md` would bloat the index; a dedicated thin mapping doc is cleaner and preserves the existing cross-references from `workflows/debugging.md` (which consults `beam-otp-internals.md`).
- Do **not** delete-and-fold: the debugging workflow and several "Related docs" entries point here; a repurpose keeps those references valid.

Target shape for the repurposed file: one section per BEAM topic → one-line Elixir API + link to `docs/beam/<file>.md`; plus a short "VM internals (community-sourced)" section retained for scheduler/GC/memory; plus the Erlang→Elixir mapping tables.

### 2.2 BEAM-heavy sections in the other three docs

**`otp-supervision.md`** (127 KB, 2379+ lines):
| Section | Overlaps | Action |
|---|---|---|
| GenServer → "What GenServer is / OTP client-server theory" (28–46) | `docs/beam/gen-server.md` | Trim theory to 2 lines + link; KEEP Elixir callback contract (79–232), call/cast/reply, handle_continue, timeouts, naming, client API, module layout. |
| Supervisor → "What a Supervisor is / OTP supervision theory" (441–452) | `docs/beam/supervision.md` | Trim theory + link. |
| "Supervision strategies" / "Restart values" / "Shutdown values" / "Restart intensity" / "Automatic shutdown" (506–717) | `docs/beam/supervision.md` | Trim BEAM-theory prose; KEEP Elixir `Supervisor.init/2` option mapping + child_spec/1 + Supervisor.child_spec/2. |
| DynamicSupervisor / Registry / PartitionSupervisor (776+) | — | Elixir-specific KEEP (link PartitionSupervisor↔docs/beam/supervision.md only lightly). |

**`concurrency-processes.md`** (70 KB):
| Section | Overlaps | Action |
|---|---|---|
| "The BEAM concurrency model" (36–53) | `docs/beam/processes-and-messages.md` | Trim to summary + link. |
| "Links vs monitors (deep comparison)" (255–310) | `docs/beam/links-monitors-and-exits.md` | Trim deep-comparison to Elixir API + link. |
| "trap_exit and exit signals" (311–346) | `docs/beam/links-monitors-and-exits.md` | Trim to Elixir `Process.flag` + link. |
| "Heap, stack, reductions, and garbage collection" (420–456) | `docs/beam/processes-and-messages.md` (shallow) + beam-otp-internals VM section | Trim; link both. |
| Node (911+) / Port (1131+) | `docs/beam/distribution.md` / `docs/beam/ports-io.md` | Add "Related BEAM guidance" link; KEEP Elixir API. |
| Task / Task.Supervisor / Agent | — | Elixir-specific KEEP. |

**`error-handling.md`** (34 KB):
| Section | Overlaps | Action |
|---|---|---|
| "Let It Crash Philosophy" (429–452) | `docs/beam/links-monitors-and-exits.md` + (conceptually) `docs/beam/supervision.md` | Trim to Elixir-flavored summary + link. |
| "Process.exit/2 vs Kernel.exit/1, links, monitors, and trap_exit" (453–471) | `docs/beam/links-monitors-and-exits.md` | Trim to Elixir API distinction + link. |
| try/catch/rescue, raise/reraise/defexception, Exception behaviour, error tuples, with, bang | — | Elixir-specific KEEP. |

---

## 3. Elixir ↔ BEAM cross-reference list

For each Elixir doc touching shared runtime, the `docs/beam/*.md` to cite in a "Related BEAM guidance" note:

| Elixir doc | Add "Related BEAM guidance" links |
|---|---|
| `beam-otp-internals.md` (repurposed) | `overview.md`, `processes-and-messages.md`, `links-monitors-and-exits.md`, `supervision.md`, `gen-server.md`, `proc-lib-and-sys.md`, `runtime-debugging.md`, `releases.md`, `applications.md`, `otp-behaviours.md`, `ports-io.md`, `distribution.md`, `nifs.md` |
| `otp-supervision.md` | `supervision.md`, `gen-server.md`, `otp-behaviours.md`, `proc-lib-and-sys.md`, `links-monitors-and-exits.md` |
| `concurrency-processes.md` | `processes-and-messages.md`, `links-monitors-and-exits.md`, `distribution.md`, `ports-io.md`, `timers.md`, `runtime-debugging.md` |
| `error-handling.md` | `links-monitors-and-exits.md`, `common-mistakes.md`, `supervision.md` |
| `configuration-and-runtime.md` | `logger-and-config.md`, `applications.md`, `releases.md`, `runtime-environment.md` |
| `mix-project-structure.md` | `releases.md`, `applications.md`, `runtime-environment.md` |
| `testing-exunit.md` | `validation.md` (light — ExUnit is Elixir-native; only the validation-gate parallel) |
| `typespecs-and-dialyzer.md` | (none strong — Dialyzer is Elixir-tooling; optionally `validation.md`) |
| `index.md` | `docs/beam/index.md` (corpus-relationship note) |

Docs with **no** BEAM cross-ref needed: `core-modules`, `language-fundamentals`, `naming-conventions`, `dependencies-and-packages`, `documentation-and-publishing`, `ecto-phoenix-patterns`, `static-analysis-credo` (purely Elixir tooling).

---

## 4. Prioritized HexDocs crawl list (verify/repair targets)

Goal: back Elixir-specific claims with persisted crawl artifacts (mirror `docs/beam/.crawl/`). BEAM-overlap material is **not** re-crawled — it is *linked* to `docs/beam/` (already crawl-sourced from erlang.org). The queue below is Elixir HexDocs only.

### P0 — must-verify (core OTP wrapper modules; several "partially explored" in source-map)
1. `https://hexdocs.pm/elixir/GenServer.html` — callback contract, handle_continue, timeouts, format_status, :via (otp-supervision backbone)
2. `https://hexdocs.pm/elixir/Supervisor.html` — child specs, strategies, auto_shutdown, child_spec/2
3. `https://hexdocs.pm/elixir/DynamicSupervisor.html` — start_link forms, init opts, start_child/terminate_child
4. `https://hexdocs.pm/elixir/Registry.html` — :via, :unique/:duplicate, partitioning, dispatch/4, keys tuple forms (v1.19/1.20)
5. `https://hexdocs.pm/elixir/PartitionSupervisor.html` — resize!/2, start_link (partially explored)
6. `https://hexdocs.pm/elixir/Task.html` — async/await, start/start_link, async_stream, yield/shutdown (partially explored)
7. `https://hexdocs.pm/elixir/Task.Supervisor.html` — start_child, async_nolink, terminate_child (partially explored)
8. `https://hexdocs.pm/elixir/Agent.html` — start_link, get/update/get_and_update/cast, use Agent (partially explored)
9. `https://hexdocs.pm/elixir/Application.html` — get_env/put_env/compile_env, start/stop, get_application
10. `https://hexdocs.pm/elixir/Process.html` — spawn/send/link/monitor/demonitor/info/alive?/register/flag/send_after
11. `https://hexdocs.pm/elixir/Node.html` — alive?/self/list/set_cookie/connect/disconnect/spawn/ping (partially explored)
12. `https://hexdocs.pm/elixir/Port.html` — open/command/info/close/list (partially explored)
13. `https://hexdocs.pm/logger/Logger.html` — levels, configure, compile-time purging, 1.15 handler migration, default handler/formatter
14. `https://hexdocs.pm/elixir/Config.html` — config/2,3, import_config, config_env/config_target, read_config (1.18+)
15. `https://hexdocs.pm/elixir/Config.Provider.html` — init/1, load/2, release config injection
16. `https://hexdocs.pm/elixir/Config.Reader.html` — read!/2
17. `https://hexdocs.pm/elixir/System.html` — env/cmd/time/trap_signal/fetch_env!/EnvError/schedulers_online
18. `https://hexdocs.pm/elixir/Exception.html` — message/1, exception/1, format/2,3 (error-handling is thin here)

### P1 — should-verify (partially-explored supporting modules + shallow-topic sources)
1. `https://hexdocs.pm/elixir/Kernel.html` — verify specific anchors (if/2, def/2, defmodule/2, use/2, defstruct/1, struct!/2, defprotocol/2, defimpl/3, |> /2, spawn/1, match?/2)
2. `https://hexdocs.pm/elixir/Kernel.SpecialForms.html` — for/1, <<>>/1, __STACKTRACE__/0, alias/import/require/receive
3. `https://hexdocs.pm/elixir/IO.html` — puts/inspect/binread/binwrite, iodata
4. `https://hexdocs.pm/elixir/typespecs.html` — built-in types, literals, maps, behaviours, string() pitfall (typespecs-and-dialyzer heavy reliance)
5. `https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html` — PLT, --plt/--halt-exit-status/--format, ignore file (ONLY source for Dialyxir — must verify)
6. `https://hexdocs.pm/ex_unit/ExUnit.Assertions.html` — assert/refute/assert_in_delta/assert_raise/assert_receive/catch_*
7. `https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html` — setup/setup_all/on_exit/start_supervised!/stop_supervised!
8. `https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html` — capture_io/with_io
9. `https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html` — capture_log/with_log, level filtering
10. `https://hexdocs.pm/ex_unit/ExUnit.DocTest.html` — doctest/2, :only/:except, `...>` continuation
11. `https://hexdocs.pm/mix/Mix.Tasks.Test.html` — --only/--exclude/--seed/--trace/--cover/--failed/--stale
12. `https://hexdocs.pm/mix/Mix.Tasks.Release.html` — config_providers, runtime.exs, :validate_compile_env
13. `https://hexdocs.pm/mix/Mix.Release.html` — struct, steps, overlays, :vm_args/:config_providers/:strip_beams
14. `https://hexdocs.pm/ecto/Ecto.Changeset.html` — cast/4 vs change/2, validations vs constraints, embeds, schemaless
15. `https://hexdocs.pm/ecto/Ecto.Query.html` — keyword vs pipe, pinning ^, nil comparisons, composition, bindings
16. `https://hexdocs.pm/ecto/Ecto.Multi.html` — insert/update/delete/run/append/rollback, success/failure shapes
17. `https://hexdocs.pm/ecto/Ecto.Repo.html` — all/get/insert/update/delete/transaction, transact/2
18. `https://hexdocs.pm/phoenix/contexts.html` — bounded contexts, web-layer separation
19. `https://hexdocs.pm/phoenix/directory_structure.html` — lib/my_app_web vs lib/my_app
20. `https://hexdocs.pm/elixir/Module.html` — register_attribute/3, @behaviour/@impl/@callback/@deprecated attributes

### P2 — nice-to-have (guide pages + peripheral modules; verify if time permits)
Guide pages: `basic-types.html`, `patterns-and-guards.html`, `case-cond-and-if.html`, `modules-and-functions.html`, `alias-require-and-import.html`, `module-attributes.html`, `protocols.html`, `comprehensions.html`, `enumerable-and-streams.html`, `writing-documentation.html`, `naming-conventions.html` (guide), `introduction-to-mix.html`, `structs.html`, `keywords-and-maps.html`, `binaries-strings-and-charlists.html`, `sigils.html`, `operators.html`, `docs-tests-and-with.html`, `try-catch-and-rescue.html`, `compatibility-and-deprecations.html`, `design-anti-patterns.html`, `library-guidelines.html`.

Modules: `String.html`, `Enum.html`, `Stream.html`, `Map.html`, `Keyword.html`, `Access.html`, `Enumerable.html`, `Collectable.html`, `MapSet.html`, `List.html`, `Tuple.html`, `Range.html`, `File.html`, `File.Stream.html`, `Regex.html`, `Function.html`, `Version.html`, `OptionParser.html`, `Code.html`, `IO.ANSI.html`, `IO.Stream.html`, `Inspect.Opts.html`, `Inspect.html`, `Protocol.html`, `String.Chars.html`, `List.Chars.html`, `Date.html`/`Time.html`/`NaiveDateTime.html`/`DateTime.html`, `System.EnvError.html`, `Mix.SCM.html`, `Mix.Task.html`, `Mix.Project.html`, `Mix.Tasks.Format.html`, `Mix.Tasks.Deps.*` family, `Mix.Tasks.Help.html`, `Mix.Tasks.New.html`, `Mix.Tasks.Compile.App.html`.

Credo per-check pages: `Credo.Check.html`, `Credo.Check.Readability.ModuleNames.html` (+ representative checks).

**Queue size: P0 = 18, P1 = 20, P2 ≈ 60 (guides + modules). Total ≈ 98 URLs (P0+P1 = 38 are the repair-critical set).**

---

## 5. Source-map status

`docs/elixir/source-map.md` (376 lines) assessment:

- **Internally consistent.** Claims 227 unique URLs across 11 families; "15 completed topic files" matches the 15 content docs that carry a "## Sources used" section (index.md and source-map.md are meta). The 2 dead links (`basic-operators.html`, `credo/checks.html`, both 404) are documented and retained for provenance — consistent with the "Sources used" notes in `language-fundamentals.md` and `static-analysis-credo.md`.
- **Claimed sources are actually cited.** Spot-checks confirm the "Sources used" sections of each doc match the source-map's "Used by" columns. No phantom sources detected.
- **Predates `docs/beam/`.** The source-map records no relationship to `docs/beam/` and no BEAM cross-references (because none exist yet). After alignment, it needs: (a) a note that BEAM-overlap material is cross-referenced to `docs/beam/` rather than re-crawled from erlang.org; (b) the `beam-otp-internals.md` entry updated to reflect its repurpose (the erlang.org system-doc sources move to "cross-referenced via docs/beam").
- **Version pinning.** Corpus pins Elixir v1.20.2 / OTP 29. The crawl queue should fetch current HexDocs to detect any drift since v1.20.2 (minor risk; note any deltas).
- **Coverage gap to record.** The BEAM-Book community source (`blog.stenmans.org/theBeamBook/`) backs the VM-internals material in `beam-otp-internals.md` that has no `docs/beam/` authoritative home. Source-map should flag this as a known community-sourced + coverage-gap area.

---

## 6. Repair / alignment execution plan

Ordered checklist for the repair phase (each step independently verifiable):

### Phase A — BEAM de-duplication (highest leverage)
1. **Repurpose `beam-otp-internals.md`** into a thin "Elixir ↔ BEAM mapping index": replace the supervision-theory / exit-signals / sys-proc_lib / releases / design-principles sections with one-line Elixir API + `docs/beam/<file>.md` links (per §2.1 map). Retain a *trimmed* "VM internals (community-sourced from the BEAM Book)" section for schedulers/reductions/GC/memory/timing-wheel, clearly flagged as community + coverage-gap. Retain the Erlang→Elixir mapping tables. Rewrite "Related docs/skills" to point at `docs/beam/` and fix the stale "No opencode skills" note (elixir-* skills now exist).
2. **Trim `otp-supervision.md`** BEAM-theory subsections (per §2.2) to Elixir API + `docs/beam/supervision.md` / `gen-server.md` links. KEEP all Elixir callback-contract / child_spec / DynamicSupervisor / Registry / PartitionSupervisor material.
3. **Trim `concurrency-processes.md`** BEAM-overlap sections (BEAM concurrency model, links-vs-monitors deep comparison, trap_exit, heap/reductions/GC) to Elixir API + `docs/beam/` links. KEEP Task/Agent/Node/Port API.
4. **Trim `error-handling.md`** let-it-crash + exit-signal sections to Elixir API + `docs/beam/links-monitors-and-exits.md` link. KEEP try/rescue/raise/defexception/error-tuples material.

### Phase B — BEAM cross-linking (all shared-runtime docs)
5. Add a "Related BEAM guidance" note to each doc in §3 (otp-supervision, concurrency-processes, error-handling, configuration-and-runtime, mix-project-structure, testing-exunit, the repurposed beam-otp-internals, and index.md corpus-relationship note).

### Phase C — Broken-reference fixes
6. Remove the stale `docs/rust/style-formatting.md` (and `docs/rust/`) cross-references from `otp-supervision.md`, `naming-conventions.md`, `mix-project-structure.md`, `static-analysis-credo.md`. Replace with the appropriate Elixir-internal ref (`naming-conventions.md` / `mix-project-structure.md` formatting) or drop.

### Phase D — Source verification / repair (crawl-backed)
7. **Establish `docs/elixir/.crawl/`** (mirror `docs/beam/.crawl/`): fetch + persist the P0 HexDocs pages (§4) with verbatim quotations; back the claims in otp-supervision / concurrency-processes / configuration-and-runtime / error-handling.
8. **Repair `typespecs-and-dialyzer.md`** Dialyxir half against `Mix.Tasks.Dialyzer.html` (PLT, flags, ignore file, warning categories) — currently only 1 HexDocs source.
9. **Repair `error-handling.md`** sourcing (only 6 sources; verify Exception.html + exit-signal material, or link to docs/beam for the latter).
10. Fetch P1 pages to back partially-explored modules (Task, Task.Supervisor, Agent, Node, Port, Supervisor, PartitionSupervisor, Exception, ExUnit.*, Ecto.*, Phoenix).

### Phase E — Index / source-map / workflow / skill updates
11. **Update `index.md`**: reflect beam-otp-internals repurpose; add docs/beam relationship note; refresh the beam-otp-internals entry in "Generated files".
12. **Update `source-map.md`**: add docs/beam cross-reference relationship; update beam-otp-internals entry; flag BEAM-Book community source + coverage gap; record new .crawl/ artifacts.
13. **Update `workflows/debugging.md`**: it consults `beam-otp-internals.md` — add docs/beam pointers (links-monitors-and-exits.md, runtime-debugging.md, processes-and-messages.md) alongside the repurposed doc.
14. **Review elixir-* skills** (`.agents/skills/elixir-*`): add `docs/beam/` refs where the skill touches shared runtime (elixir-otp → supervision.md/gen-server.md; elixir-error-handling → links-monitors-and-exits.md; elixir-config → logger-and-config.md/applications.md). Confirm skill "source doc" pointers still resolve after beam-otp-internals repurpose.
15. **Final consistency sweep**: re-grep for `docs/beam` presence (should now be non-zero across shared-runtime docs); re-grep for `docs/rust` (should be zero); verify all "Related docs" links resolve.

### Risks / watch-items
- **beam-otp-internals VM-internals have no docs/beam home** — do not delete that material; trim + flag. If a future docs/beam "VM internals" doc is created, fold it there.
- **Repurpose must preserve inbound references** from `workflows/debugging.md` and several "Related docs" sections — keep the filename.
- **Crawl must fetch current HexDocs** to catch post-v1.20.2 drift; record any deltas in source-map.
- **Scope discipline:** do not re-crawl erlang.org for BEAM-overlap material — link to docs/beam (already crawl-sourced). Crawl budget is Elixir HexDocs only.

