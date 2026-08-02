# Crawl: gleam.run/documentation/
- seed_url: https://gleam.run/documentation/
- canonical_url: https://gleam.run/documentation/
- family: Gleam official docs
- fetch: HTTP 200 (no redirect)
- gleam_version: not stated on page
- feeds_docs: overview.md, index.md

## Purpose
Top-level documentation hub / index for the Gleam programming language. It is a
curated directory page that maps the entire Gleam documentation surface
(learning, references, guides, security, cheatsheets, deployment, about,
community) into a single entry point. It does not itself teach Gleam; it routes
readers to the language tour, writing-gleam guide, stdlib, package index, and
related references.

## Doc structure (canonical top-level sections)
The page presents these H2 sections, in order:

1. **Learning Gleam**
   - Language tour (external: tour.gleam.run)
   - Writing Gleam (guide on creating/developing projects)
   - Installing Gleam
   - Unofficial courses (Exercism, CodeCrafters)
2. **Gleam references**
   - The Gleam Language overview (tour.gleam.run/everything/)
   - The command line reference (writing-gleam/command-line-reference)
   - The Gleam language server reference (/language-server)
   - The gleam.toml config file reference (writing-gleam/gleam-toml)
   - The Gleam package index (packages.gleam.run)
   - The standard library documentation (hexdocs.pm/gleam_stdlib)
   - The "Awesome Gleam" resource list (github)
3. **Guides**
   - Developing Gleam projects using the Gleam build tool
   - Conventions, patterns, and anti-patterns in Gleam code
   - Using code written in other languages from Gleam (externals)
4. **Security**
   - Generating a Source Bill of Materials (SBOM) for a Gleam project
5. **Cheatsheets**
   - Gleam for Elixir / Elm / Erlang / PHP / Python / Rust users
6. **Deployment**
   - Deploying to a Linux server
   - Deploying on Fly.io
   - Community: Deploying on Clever Cloud (github)
7. **About Gleam**
   - Frequently asked questions
   - Gleam's Branding
8. **Community Resources**
   - Exercism's Gleam track

## Key concepts
- Gleam is a programming language with its own mascot ("Lucy the star").
- Documentation is split between gleam.run (guides/references), tour.gleam.run
  (interactive language tour), packages.gleam.run (package index), and
  hexdocs.pm (stdlib API docs).
- The build tool, language server, and `gleam.toml` config are first-class
  referenced artifacts.
- Gleam supports FFI: "Using code written in other languages from Gleam"
  (externals).
- Supply-chain security is addressed via SBOM generation.
- Cross-language onboarding is supported via per-language cheatsheets.
- Deployment targets include Linux servers and Fly.io.

## Verbatim quotes
- Heading "Learning Gleam" — "An in-browser interactive introduction that teaches the whole language." (re: Language tour)
- Heading "Learning Gleam" — "A guide on creating and developing projects in Gleam." (re: Writing Gleam)
- Heading "Learning Gleam" — "How to get Gleam on your computer." (re: Installing Gleam)
- Hero — "Learn all about programming in Gleam!"
- Meta description — "All about programming in Gleam: find the docs you need."
- Heading "Security" — "Generating a Source Bill of Materials for a Gleam project"

## Version notes
- No Gleam compiler/runtime version is stated on this page.
- Footer copyright: "© 2026 Louis Pilfold" (indicates site maintainer / year,
  not a language version).

## Discovered links

### Relevant (crawl later)
- https://tour.gleam.run — Language tour; interactive whole-language introduction. (RELEVANT)
- https://tour.gleam.run/everything/ — "The Gleam Language overview"; single-page full-language reference. (RELEVANT)
- /writing-gleam — Writing Gleam guide; project creation and development. (RELEVANT)
- /getting-started/installing — Installing Gleam; toolchain setup. (RELEVANT)
- /writing-gleam/command-line-reference — CLI reference for the Gleam build tool. (RELEVANT)
- /language-server — Gleam language server (LSP) reference. (RELEVANT)
- /writing-gleam/gleam-toml — gleam.toml config file reference. (RELEVANT)
- https://packages.gleam.run — Gleam package index. (RELEVANT)
- https://hexdocs.pm/gleam_stdlib/ — Standard library API documentation. (RELEVANT)
- /documentation/conventions-patterns-and-anti-patterns — Conventions, patterns, anti-patterns. (RELEVANT)
- /documentation/externals — FFI: using code from other languages in Gleam. (RELEVANT)
- /documentation/source-bill-of-materials — SBOM generation for Gleam projects. (RELEVANT)
- /cheatsheets/gleam-for-elixir-users — Cheatsheet: Gleam for Elixir users. (RELEVANT)
- /cheatsheets/gleam-for-elm-users — Cheatsheet: Gleam for Elm users. (RELEVANT)
- /cheatsheets/gleam-for-erlang-users — Cheatsheet: Gleam for Erlang users. (RELEVANT)
- /cheatsheets/gleam-for-php-users — Cheatsheet: Gleam for PHP users. (RELEVANT)
- /cheatsheets/gleam-for-python-users — Cheatsheet: Gleam for Python users. (RELEVANT)
- /cheatsheets/gleam-for-rust-users — Cheatsheet: Gleam for Rust users. (RELEVANT)
- /deployment/linux-server — Deploying to a Linux server. (RELEVANT)
- /deployment/fly — Deploying on Fly.io. (RELEVANT)
- /frequently-asked-questions — FAQ about Gleam. (RELEVANT)
- https://github.com/gleam-lang/awesome-gleam — "Awesome Gleam" curated resource list. (RELEVANT — index of ecosystem packages/tools)

### Skipped
- https://exercism.org/tracks/gleam — Third-party learning platform; out of scope (vendor course). (SKIP)
- https://app.codecrafters.io/join?via=lpil — Third-party paid course; referral link. (SKIP)
- https://github.com/davlgd/gleam-demo — Community deployment guide for Clever Cloud; vendor-specific, community-maintained. (SKIP)
- /news — News/blog index; context-only, not reference docs. (SKIP)
- /community — Community page; non-reference. (SKIP)
- /sponsor — Sponsorship page. (SKIP)
- /install — Install landing (covered by /getting-started/installing; nav duplicate). (SKIP)
- /branding — Branding assets; non-technical. (SKIP)
- /roadmap — Roadmap; project planning, not reference. (SKIP)
- /case-studies — Case studies; context-only. (SKIP)
- https://github.com/gleam-lang — GitHub org homepage; source mirror. (SKIP)
- https://discord.gg/Fm8Pwmy — Discord community invite. (SKIP)
- https://shop.gleam.run/en-gbp — Merch store. (SKIP)
- https://playground.gleam.run — Playground; interactive tool, not docs. (SKIP)
- https://gleamweekly.com — Gleam Weekly newsletter; non-reference. (SKIP)
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md — Code of conduct; non-technical. (SKIP)
- https://plausible.io/js/plausible.js — Analytics script. (SKIP)
- /feed.xml — Atom feed; non-reference. (SKIP)
