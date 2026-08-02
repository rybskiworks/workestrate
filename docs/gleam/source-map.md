# Gleam Source Map

## Purpose

This document maps every crawled source URL to the Gleam documentation files that use it. It is the authoritative provenance index behind the `docs/gleam/` topic files. Future agents can trace any claim back to its primary source, identify coverage gaps, and assess which sources remain available for future expansion. All 37 sources were live-fetched via curl (HTTP 200 or verified redirect) from gleam.run, tour.gleam.run, and hexdocs.pm; the extracted content is persisted in `docs/gleam/.crawl/01-37`.

The intended audience is any future maintainer, reviewer, or agent who needs to verify a claim, add a topic, or audit coverage. It is a provenance layer between the generated guidance and the official sources, not a replacement for them. Every source is an official Gleam documentation page maintained by the Gleam team (gleam.run, tour.gleam.run) or the official hexdocs package reference (gleam_stdlib, gleam_otp, gleam_erlang, gleam_json, gleam_http, gleam_javascript, gleeunit).

## Source coverage summary

The corpus comprises 37 crawled source URLs across 10 documentation families, all official primary.

| Family | Count | Authority |
|---|---|---|
| gleam.run official docs (documentation, writing-gleam guides, deployment, install, faq) | 12 | Official primary |
| tour.gleam.run language tour | 1 | Official primary |
| gleam.run cheatsheets | 3 | Official primary |
| hexdocs gleam_stdlib (index + modules) | 9 | Official primary |
| hexdocs gleam_otp (index + modules) | 5 | Official primary |
| hexdocs gleam_erlang (index + modules) | 3 | Official primary |
| hexdocs gleam_json | 1 | Official primary |
| hexdocs gleam_http | 1 | Official primary |
| hexdocs gleam_javascript | 1 | Official primary |
| hexdocs gleeunit | 1 | Official primary |
| **Total** | **37** | |

The gleam.run official docs family is the largest (12 pages) and forms the conceptual and tooling backbone; the tour.gleam.run page (1) is the single-page language tour that anchors the language-mechanics docs; the cheatsheets (3) supply migration-pitfall translations from Elixir/Erlang/Rust; the hexdocs gleam_stdlib family (9) supplies the core data-type and collection module references; the hexdocs gleam_otp (5), gleam_erlang (3), gleam_json (1), gleam_http (1), gleam_javascript (1), and gleeunit (1) families supply the core-package API references. All 37 pages were crawled to completion and used to write the 19 canonical topic docs.

## Link expansion coverage

- **Depth-0 seed pass:** The crawl began with three entry points - `gleam.run/documentation/` (crawl 01, the top-level documentation hub), `tour.gleam.run/everything/` (crawl 02, the whole language tour), and `gleam.run/writing-gleam/` (crawl 03, the project lifecycle guide). These are the depth-0 seeds.
- **Depth-1 discovery:** From the documentation hub (01), discovered links were followed to the conventions page (04), externals (08), source-bill-of-materials (09), install (10), FAQ (11), deployment guides (12, 13), and cheatsheets (14, 15, 16). From the writing-gleam guide (03), discovered links were followed to the gleam.toml reference (05), command-line reference (06), and language-server reference (07).
- **Depth-2 discovery (hexdocs):** From the gleam.run pages and the stdlib references they cite, the hexdocs package indexes were reached - `gleam_stdlib` index (17), `gleam_otp` index (26), `gleam_erlang` index (29), `gleam_json` (32), `gleeunit` (33), `gleam_http` index (35), `gleam_javascript` index (36). From the stdlib index (17), the module pages were followed: `gleam/result` (18), `gleam/option` (19), `gleam/list` (20), `gleam/string` (21), `gleam/dict` (22), `gleam/dynamic` (23), `gleam/dynamic/decode` (24), `gleam/bit_array` (25). From the gleam_otp index (26), the module pages were followed: `gleam/otp/actor` (27), `gleam/otp/supervision` (28), `gleam/otp/static_supervisor` (34), `gleam/otp/factory_supervisor` (37). From the gleam_erlang index (29), the module pages were followed: `gleam/erlang/process` (30), `gleam/erlang/application` (31).
- **Families with rich discovered links:** The gleam.run documentation hub (01) had the richest cross-referencing - it links to every documentation sub-page, the writing-gleam guides, the cheatsheets, and the deployment guides. The stdlib index (17) was the hub for the 8 stdlib module pages. The gleam_otp index (26) was the hub for the 4 OTP module pages. The externals page (08) cross-references both the Erlang and JavaScript FFI surfaces and the gleam_javascript package.
- Note: the crawl ledger headers carry a `feeds_docs` field recording which generated docs each source feeds; origin (seed vs discovered) below is inferred from crawl order and link structure - pages 01, 02, 03 (the three entry points) are classified as seed; pages 04-37 (reached by following discovered links) are classified as discovered.

## Sources by topic

Legend:

- **Origin:** seed = depth-0 entry point (documentation hub, language tour, or project guide); discovered = reached by following links at depth 1-2.
- **Authority:** official primary = gleam.run / tour.gleam.run / hexdocs.pm documentation maintained by the Gleam team.
- **Coverage status:** fully explored = the page was crawled to completion and substantially used to write the topic docs. All 37 pages are fully explored.

### 1. gleam.run official docs (12 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 01 | https://gleam.run/documentation/ | - | seed | Gleam official docs | Top-level documentation hub/index; maps entire Gleam documentation surface (learning, references, guides, security, cheatsheets, deployment) | overview.md, index.md | gleam-language | impl, review | Official primary | Fully explored |
| 03 | https://gleam.run/writing-gleam/ | - | seed | Gleam official docs | End-to-end project lifecycle walkthrough; scaffolding, building, running, testing, formatting, dependencies | project-structure-and-cli.md, overview.md | gleam-language, gleam-packages-ffi | impl, review, val | Official primary | Fully explored |
| 04 | https://gleam.run/documentation/conventions-patterns-and-anti-patterns/ | - | discovered | Gleam official docs | Conventions, anti-patterns (always-rules), patterns (applied when beneficial); idiomatic Gleam code guidance | conventions-patterns-antipatterns.md, code-review workflow | gleam-language | review | Official primary | Fully explored |
| 05 | https://gleam.run/writing-gleam/gleam-toml/ | - | discovered | Gleam official docs | `gleam.toml` config reference; project metadata, dependencies, documentation, repository info, target/options, TOML 1.1 | gleam-toml-and-targets.md, project-structure-and-cli.md | gleam-packages-ffi | impl, review, val | Official primary | Fully explored |
| 06 | https://gleam.run/writing-gleam/command-line-reference/ | https://gleam.run/command-line-reference/ | discovered | Gleam official docs | `gleam` CLI reference; every subcommand, signature, flags | project-structure-and-cli.md, validation.md | gleam-packages-ffi | impl, review, val | Official primary | Fully explored |
| 07 | https://gleam.run/language-server-reference/ | https://gleam.run/language-server/ | discovered | Gleam official docs | Gleam Language Server (LSP); IDE features, editors, limitations | project-structure-and-cli.md, validation.md | gleam-packages-ffi | impl, review, val | Official primary | Fully explored |
| 08 | https://gleam.run/documentation/externals/ | - | discovered | Gleam official docs | External type/function features; `@external` annotation; FFI to Erlang/JS; v1.13 JS data API | externals-and-ffi.md, erlang-interop.md, javascript-target.md | gleam-otp-interop, gleam-packages-ffi | impl, review, debug | Official primary | Fully explored |
| 09 | https://gleam.run/documentation/source-bill-of-materials/ | - | discovered | Gleam official docs | SBoM guide; ORT (OSS Review Toolkit); CycloneDX/SPDX; not a native `gleam` command | package-management-and-publishing.md, validation.md | gleam-packages-ffi | val | Official primary | Fully explored |
| 10 | https://gleam.run/getting-started/installing/ | https://gleam.run/install | discovered | Gleam official docs | Install hub; OS chooser; legacy path redirect | project-structure-and-cli.md, overview.md | gleam-language, gleam-packages-ffi | impl, review, val | Official primary | Fully explored |
| 11 | https://gleam.run/documentation/frequently-asked-questions/ | https://gleam.run/frequently-asked-questions/ | discovered | Gleam official docs | FAQ; language design rationale, scope boundaries, deliberately excluded features | overview.md, conventions-patterns-antipatterns.md | gleam-language | impl, review | Official primary | Fully explored |
| 12 | https://gleam.run/deployment/linux-server/ | - | discovered | Gleam official docs | Deploy Gleam web app (Erlang target) to single Linux server; systemd, Podman, Caddy; Gleam v1.12.0, Erlang/OTP 28.0.2.0 | deployment-and-runtime.md | gleam-packages-ffi | val | Official primary | Fully explored |
| 13 | https://gleam.run/deployment/fly/ | - | discovered | Gleam official docs | Deploy Gleam web app to Fly.io; Dockerfile container build; `erlang-shipment` export; Gleam v1.8.0, Erlang/OTP 27.1.1.0 | deployment-and-runtime.md, javascript-target.md | gleam-packages-ffi | impl, review, val | Official primary | Fully explored |

### 2. tour.gleam.run language tour (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 02 | https://tour.gleam.run/everything/ | - | seed | Gleam language tour | Whole language tour on single page; 63 lessons; basics, types, functions, Result/Option, pattern matching, `use`, externals | language-fundamentals.md, types-records-and-patterns.md, functions-pipelines-and-use.md, result-option-and-errors.md | gleam-language | impl, review, debug | Official primary | Fully explored |

### 3. gleam.run cheatsheets (3 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 14 | https://gleam.run/cheatsheets/gleam-for-elixir-users/ | - | discovered | Gleam cheatsheets | Side-by-side cheat sheet for Elixir developers; syntax/semantics translation | conventions-patterns-antipatterns.md, code-review workflow | gleam-language | review | Official primary | Fully explored |
| 15 | https://gleam.run/cheatsheets/gleam-for-erlang-users/ | - | discovered | Gleam cheatsheets | Translation reference for Erlang programmers; syntax/type-system differences | conventions-patterns-antipatterns.md, erlang-interop.md | gleam-language, gleam-otp-interop | impl, review, debug | Official primary | Fully explored |
| 16 | https://gleam.run/cheatsheets/gleam-for-rust-users/ | - | discovered | Gleam cheatsheets | Syntactic translation reference for Rust developers | conventions-patterns-antipatterns.md, result-option-and-errors.md | gleam-language | impl, review, debug | Official primary | Fully explored |

### 4. hexdocs gleam_stdlib (9 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 17 | https://hexdocs.pm/gleam_stdlib/ | https://gleam-stdlib.hexdocs.pm/ | discovered | hexdocs gleam_stdlib | `gleam_stdlib` package index; core data types and operations; both targets; 19 modules; v1.0.3 | stdlib.md | gleam-language | impl, review | Official primary | Fully explored |
| 18 | https://hexdocs.pm/gleam_stdlib/gleam/result.html | https://gleam-stdlib.hexdocs.pm/gleam/result.html | discovered | hexdocs gleam_stdlib | `gleam/result` module; `Result(a,e)` combinators; explicit error handling; v1.0.3 | result-option-and-errors.md, stdlib.md | gleam-language | impl, review, debug | Official primary | Fully explored |
| 19 | https://hexdocs.pm/gleam_stdlib/gleam/option.html | https://gleam-stdlib.hexdocs.pm/gleam/option.html | discovered | hexdocs gleam_stdlib | `gleam/option` module; `Option` type; `Some`/`None`; v1.0.3 | result-option-and-errors.md, stdlib.md | gleam-language | impl, review, debug | Official primary | Fully explored |
| 20 | https://gleam-stdlib.hexdocs.pm/gleam/list.html | - | discovered | hexdocs gleam_stdlib | `gleam/list` module; `List(a)` singly-linked immutable persistent cons-list; v1.0.3 | stdlib.md, language-fundamentals.md | gleam-language | impl, review | Official primary | Fully explored |
| 21 | https://gleam-stdlib.hexdocs.pm/gleam/string.html | - | discovered | hexdocs gleam_stdlib | `gleam/string` module; UTF-8 binaries; construction, inspection, splitting; v1.0.3 | stdlib.md | gleam-language | impl, review | Official primary | Fully explored |
| 22 | https://gleam-stdlib.hexdocs.pm/gleam/dict.html | https://hexdocs.pm/gleam_stdlib/gleam/dict.html | discovered | hexdocs gleam_stdlib | `gleam/dict` module; `Dict(k,v)` associative key-value; immutable persistent; v1.0.3 | stdlib.md | gleam-language | impl, review | Official primary | Fully explored |
| 23 | https://gleam-stdlib.hexdocs.pm/gleam/dynamic.html | https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html | discovered | hexdocs gleam_stdlib | `gleam/dynamic` module; `Dynamic` type; construction side of untyped boundary; v1.0.3 | json-dynamic-and-api-boundaries.md, externals-and-ffi.md | gleam-language, gleam-packages-ffi | impl, review | Official primary | Fully explored |
| 24 | https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html | - | discovered | hexdocs gleam_stdlib | `gleam/dynamic/decode` module; type-safe composable decoders; v1.0.3 | json-dynamic-and-api-boundaries.md | gleam-language | impl, review | Official primary | Fully explored |
| 25 | https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html | - | discovered | hexdocs gleam_stdlib | `gleam/bit_array` module; `BitArray`; bit sequences; base16/base64; v1.0.3 | stdlib.md, docs/beam/binaries | gleam-language | impl, review | Official primary | Fully explored |

### 5. hexdocs gleam_otp (5 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 26 | https://hexdocs.pm/gleam_otp/ | https://gleam-otp.hexdocs.pm/ | discovered | hexdocs gleam_otp | `gleam_otp` package index; typed actors, supervision trees; wraps Erlang/OTP; v1.2.0 | otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 27 | https://hexdocs.pm/gleam_otp/gleam/otp/actor.html | - | discovered | hexdocs gleam_otp | `gleam/otp/actor` module; `Actor` abstraction; BEAM process, state, message passing; v1.2.0 | otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 28 | https://hexdocs.pm/gleam_otp/gleam/otp/supervision.html | https://gleam-otp.hexdocs.pm/gleam/otp/supervision.html | discovered | hexdocs gleam_otp | `gleam/otp/supervision` module; child-spec vocabulary; `ChildSpecification`; v1.2.0 | otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 34 | https://hexdocs.pm/gleam_otp/gleam/otp/static_supervisor.html | https://gleam-otp.hexdocs.pm/gleam/otp/static_supervisor.html | discovered | hexdocs gleam_otp | `gleam/otp/static_supervisor` module; static supervisor; children specified once; v1.2.0 | otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 37 | https://hexdocs.pm/gleam_otp/gleam/otp/factory_supervisor.html | https://gleam-otp.hexdocs.pm/gleam/otp/factory_supervisor.html | discovered | hexdocs gleam_otp | `gleam/otp/factory_supervisor` module; dynamic supervisor; template function; v1.2.0 | otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |

### 6. hexdocs gleam_erlang (3 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 29 | https://hexdocs.pm/gleam_erlang/ | https://gleam-erlang.hexdocs.pm/ | discovered | hexdocs gleam_erlang | `gleam_erlang` package index; typed wrappers over Erlang/OTP primitives; v1.3.0 | erlang-interop.md, otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 30 | https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html | https://gleam-erlang.hexdocs.pm/gleam/erlang/process.html | discovered | hexdocs gleam_erlang | `gleam/erlang/process` module; typed message-passing (`Subject`/`send`/`receive`/`call`), `Selector`; v1.3.0 | erlang-interop.md, otp-actors-and-supervision.md | gleam-otp-interop | impl, review, debug, refactor | Official primary | Fully explored |
| 31 | https://hexdocs.pm/gleam_erlang/gleam/erlang/application.html | https://gleam-erlang.hexdocs.pm/gleam/erlang/application.html | discovered | hexdocs gleam_erlang | `gleam/erlang/application` module; typed wrapper over OTP application concepts; v1.3.0 | erlang-interop.md | gleam-otp-interop | impl, review, debug | Official primary | Fully explored |

### 7. hexdocs gleam_json (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 32 | https://hexdocs.pm/gleam_json/gleam/json.html | https://gleam-json.hexdocs.pm/gleam/json.html | discovered | hexdocs gleam_json | `gleam/json` module; JSON encode/decode; `Json` opaque type; builders; parse to `Dynamic`; v3.1.0 | json-dynamic-and-api-boundaries.md | gleam-language | impl, review | Official primary | Fully explored |

### 8. hexdocs gleam_http (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 35 | https://hexdocs.pm/gleam_http/ | https://gleam-http.hexdocs.pm/ | discovered | hexdocs gleam_http | `gleam_http` package index; HTTP types/functions; `Request`/`Response`/`Headers`/`Method`/`Status`; sans-IO `Service`; v4.3.0 | http-and-services.md | gleam-packages-ffi | impl, review | Official primary | Fully explored |

### 9. hexdocs gleam_javascript (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 36 | https://hexdocs.pm/gleam_javascript/ | https://gleam-javascript.hexdocs.pm/ | discovered | hexdocs gleam_javascript | `gleam_javascript` package index; typed wrappers over JS values; array/promise/symbol; v1.0.0 | javascript-target.md, externals-and-ffi.md | gleam-packages-ffi | impl, review | Official primary | Fully explored |

### 10. hexdocs gleeunit (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 33 | https://hexdocs.pm/gleeunit/ | https://gleeunit.hexdocs.pm/ | discovered | hexdocs gleeunit | `gleeunit` index; standard Gleam test runner; EUnit on Erlang, custom on JS; v1.11.0 | testing.md, validation.md | gleam-packages-ffi | val | Official primary | Fully explored |

## Redirects

Several seed URLs were found to redirect or 404 during the crawl; the canonical destination was recorded in each case and the crawl succeeded against the canonical URL:

- `https://gleam.run/documentation/frequently-asked-questions/` returns **404** (the documentation-subpath was restructured). The canonical URL is `https://gleam.run/frequently-asked-questions/` (crawl file 11). The crawl succeeded against the canonical URL.
- `https://gleam.run/getting-started/installing/` **redirects** to `https://gleam.run/install` (the legacy getting-started path was replaced by the install hub) (crawl file 10). The crawl succeeded against the canonical URL.
- `https://gleam.run/writing-gleam/command-line-reference/` **redirects** to `https://gleam.run/command-line-reference/` (the writing-gleam subpath was promoted to a top-level path via a JS redirect) (crawl file 06). The crawl succeeded against the canonical URL.
- `https://gleam.run/language-server-reference/` returns **404**. The canonical URL is `https://gleam.run/language-server/` (crawl file 07). The crawl succeeded against the canonical URL.
- **hexdocs subdomain canonicalisation:** hexdocs.pm redirects package index and module pages to the `<package>.hexdocs.pm` subdomain canonical form. This applies to `gleam_stdlib` (-> `gleam-stdlib.hexdocs.pm/`), `gleam_otp` (-> `gleam-otp.hexdocs.pm/`), `gleam_erlang` (-> `gleam-erlang.hexdocs.pm/`), `gleam_json` (-> `gleam-json.hexdocs.pm/`), `gleam_http` (-> `gleam-http.hexdocs.pm/`), `gleam_javascript` (-> `gleam-javascript.hexdocs.pm/`), and `gleeunit` (-> `gleeunit.hexdocs.pm/`). The canonical URL was recorded in each crawl ledger header. Note: a few module pages (e.g. `gleam/dict.html`, `gleam/dynamic.html`) recorded the reverse direction (`gleam-stdlib.hexdocs.pm` -> `hexdocs.pm/gleam_stdlib`); both forms resolve to the same content.
- All other crawl files: seed URL == canonical URL, fetch 200, no redirects.

## Discovered but not crawled (revisit later)

The following pages were discovered as cross-references during the crawl but were not themselves crawled. They are the remaining genuinely-uncrawled candidates for a future expansion pass, grouped by family with the reason each was deferred.

### gleam_stdlib smaller modules

- `gleam/int` - integer operations. Discovered by the stdlib index (17).
- `gleam/float` - float operations. Discovered by the stdlib index (17).
- `gleam/io` - IO/console output. Discovered by the stdlib index (17).
- `gleam/set` - `Set` collection. Discovered by the stdlib index (17).
- `gleam/function` - function helpers (`identity`, `compose`, `tap`, `constant`). Discovered by the stdlib index (17).
- `gleam/bool` - boolean operations. Discovered by the stdlib index (17).
- `gleam/order` - `Order` comparison type. Discovered by the stdlib index (17).
- `gleam/pair` - pair/tuple-2 helpers. Discovered by the stdlib index (17).
- `gleam/uri` - URI parsing. Discovered by the stdlib index (17).
- `gleam/bytes_tree` - `BytesTree` builder. Discovered by the stdlib index (17).
- `gleam/string_tree` - `StringTree` builder. Discovered by the stdlib index (17).
- `gleam/iterator` - lazy iteration. Discovered by the stdlib index (17).

### gleam_erlang smaller modules

- `gleam/erlang/atom` - atom type and operations. Discovered by the gleam_erlang index (29).
- `gleam/erlang/charlist` - charlist type. Discovered by the gleam_erlang index (29).
- `gleam/erlang/node` - distributed node operations. Discovered by the gleam_erlang index (29).
- `gleam/erlang/port` - port operations. Discovered by the gleam_erlang index (29).
- `gleam/erlang/reference` - reference type. Discovered by the gleam_erlang index (29).

### gleam_http sub-modules

- `gleam/http/request` - request construction. Discovered by the gleam_http index (35).
- `gleam/http/response` - response construction. Discovered by the gleam_http index (35).
- `gleam/http/service` - `Service` type and helpers. Discovered by the gleam_http index (35).
- `gleam/http/cookie` - cookie parsing/serialization. Discovered by the gleam_http index (35).

### gleam_javascript modules

- `gleam/javascript/array` - typed JS array wrapper. Discovered by the gleam_javascript index (36).
- `gleam/javascript/promise` - typed JS promise wrapper. Discovered by the gleam_javascript index (36).
- `gleam/javascript/symbol` - typed JS symbol wrapper. Discovered by the gleam_javascript index (36).

### gleam_otp additional modules

- `gleam/otp/system` - system message handling. Discovered by the gleam_otp index (26).
- `gleam/otp/port` - port abstraction. Discovered by the gleam_otp index (26).

### gleam_time

- `gleam_time` package - time/clock utilities. Discovered via the Gleam package ecosystem; not yet crawled.

### gleeunit sub-module

- `gleeunit/should` - assertion helpers (`should.equal`, `should.be_ok`, `should.be_error`). Discovered by the gleeunit index (33); referenced in `testing.md` but the module page itself was not crawled.

### Ecosystem packages

- `mist` - high-performance HTTP server on the BEAM. Discovered by the gleam_http index (35) and deployment docs (12, 13).
- `wisp` - web framework built on mist. Discovered by the deployment docs (12, 13).
- `lustre` - frontend framework. Discovered via the Gleam ecosystem.
- `gleam_httpc` - HTTP client adapter using Erlang/OTP `inets` `httpc`. Discovered by the gleam_http index (35).
- `gleam_hackney` - HTTP client adapter using Hackney. Discovered by the gleam_http index (35).
- `gleam_fetch` - HTTP client adapter using the Fetch API (JS target). Discovered by the gleam_http index (35).

### Additional cheatsheets

- `gleam-for-elm-users` - Elm migration cheatsheet. Discovered by the cheatsheet family (14, 15, 16).
- `gleam-for-python-users` - Python migration cheatsheet. Discovered by the cheatsheet family (14, 15, 16).
- `gleam-for-php-users` - PHP migration cheatsheet. Discovered by the cheatsheet family (14, 15, 16).

## Skipped

The following categories of source were intentionally excluded from the crawl:

- **Playground** (`https://gleam.run/playground/`) - interactive playground; context-only, not reference documentation.
- **Feed/blog/news** (`https://gleam.run/feed.xml`, `https://gleam.run/blog/`, `https://gleam.run/news/`) - RSS feed, blog posts, and news; context-only, not reference documentation.
- **Case studies** (`https://gleam.run/case-studies/`) - marketing case studies; context-only.
- **GitHub source mirrors** (`https://github.com/gleam-lang/...`) - source code repositories; not primary documentation. The hexdocs ExDoc pages already capture the API surface.
- **YouTube links** - video tutorials and talks linked from gleam.run; not text reference documentation.
- **Vendor homepages** (Fly.io homepage, Hover homepage) - the deployment guides (12, 13) reference these vendors but the vendor homepages themselves are not Gleam documentation.

## Verification method

All 37 pages were fetched live via curl and returned HTTP 200 against the canonical URL (or a verified redirect). The extracted content (purpose, key concepts, strict rules, verbatim quotes, and discovered links) is persisted in `docs/gleam/.crawl/01-37`. The 19 canonical topic docs were written strictly from those extractions; no claim in the topic docs relies on memory or non-crawled sources. Four gleam.run seed URLs were found to redirect or 404 (`frequently-asked-questions`, `getting-started/installing`, `writing-gleam/command-line-reference`, `language-server-reference`) and their canonical replacements are recorded in the Redirects section above. The hexdocs package index and module pages redirect to the `<package>.hexdocs.pm` subdomain canonical form; the canonical URL was recorded in each crawl ledger header. Origin (seed vs discovered) is inferred from crawl order and link structure because the ledger headers do not carry an explicit origin field.
