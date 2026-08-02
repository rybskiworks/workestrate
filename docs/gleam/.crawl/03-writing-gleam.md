# Crawl: gleam.run/writing-gleam/
- seed_url: https://gleam.run/writing-gleam/
- canonical_url: https://gleam.run/writing-gleam/
- family: Gleam official guide
- fetch: 200
- gleam_version: not stated on page
- feeds_docs: project-structure-and-cli.md, overview.md

## Purpose
End-to-end walkthrough for creating and developing a Gleam project. Explicitly NOT a
language tutorial — it assumes the reader has done the language tour and has Gleam +
Erlang installed. It teaches the project lifecycle: scaffolding, building, running,
adding/using dependencies, testing, and sharing (escript bundling). Builds a small CLI
program `vars` that prints environment variables (`gleam run get USER`).

## Sections covered (project structure, directories, build/test/deps/targets)
- **The project** — goal: a CLI for printing env vars; sample usage `gleam run get USER`.
- **Creating a project** — `gleam new vars`; resulting tree; what each generated file is.
- **Running the project** — entrypoint is `main` fn in module named after the package;
  `gleam run`; `gleam run -m modulename` to run a different module; `--target javascript`
  to switch runtime (Erlang is the default used in the guide).
- **Adding dependencies** — `gleam add envoy argv`; deps land in `[dependencies]` of
  `gleam.toml`; `manifest.toml` locks versions (check into VCS; not uploaded to Hex);
  `gleam update` refreshes within constraints; path deps and git deps syntax shown.
- **Using dependencies** — importing `argv`/`envoy`; pattern matching on CLI args;
  helper `format_pair`.
- **Testing your code** — internal modules (`packagename/internal`, `packagename/internal/*`)
  for non-public-API code; `gleeunit` test runner; `gleam test` runs `main` in
  `vars_test`; any public fn in `test/` ending in `_test` is run as a test.
- **Sharing your program** — for web apps, defer to deployment docs; for CLI, bundle via
  escript using `gleescript` (added with `--dev`); `gleam build && gleam run -m gleescript`
  produces `./vars` escript runnable on any Erlang install with a compatible version.

## Project-structure conventions (src/test/dev dirs)
Generated `gleam new` tree:
```
.
├── .github/workflows/test.yml   # GitHub Actions CI running tests on push
├── .gitignore
├── README.md                    # markdown intro docs
├── gleam.toml                   # project configuration
├── src/vars.gleam               # program source; entrypoint = main fn in pkg-named module
└── test/vars_test.gleam         # test code
```
- `src/` — program source.
- `test/` — test code; public fns ending in `_test` are auto-run by gleeunit.
- `gleam.toml` — config; sections `[dependencies]` and `[dev_dependencies]`.
- `manifest.toml` — lockfile; check into VCS; not uploaded to Hex; not used by downstream
  dependents.
- **Internal modules**: `packagename/internal` and `packagename/internal/*` — public fns
  here are importable within the package but are NOT documented and do NOT carry public
  API stability guarantees.
- A "Gleam package" = the whole thing, whether library or runnable program.
- `--dev` flag places a dep in `[dev_dependencies]` (build/dev/test only, excluded from
  production builds).
- Version constraints use Hex semver range syntax, e.g. `>= 1.0.1 and < 2.0.0`.
- Path deps: `my_other_package = { path = "../my_other_package" }`.
- Git deps: `my_library = { git = "git@github.com:my-project/my-library", ref = "a8b3c5d82" }`.

## Key concepts
- Build tool is built into the `gleam` binary (no separate build tool).
- Targets: Erlang (default) and JavaScript (`--target javascript`).
- Entrypoint convention: `main` function in the module named after the package.
- Dependencies come from the Hex package manager; discovery via packages.gleam.run.
- `gleam_stdlib` is included by default in new projects.
- `gleeunit` is the default test runner in generated projects.
- escript (Erlang runtime feature) is the CLI bundling mechanism, accessed via the
  `gleescript` dev dependency.
- Internal modules namespace convention enables a public-but-internal API tier.

## Verbatim quotes
- "This guide shows you how to create and develop a Gleam project. It does not teach the
  Gleam language itself, so read through the language tour first if you have not already."
- "Altogether this is called a Gleam package, regardless of whether it's a library or a
  program that is run directly."
- "The entrypoint for the program is the function called main in the module with the same
  name as the package itself."
- "There is now also a manifest.toml file which locks all the dependency packages to
  specific versions. It's recommended to check this file into your version control system
  to ensure that anyone who downloads and runs your project will get the same versions of
  the dependencies. This manifest file isn't uploaded to Hex so it is not used when other
  projects depend on your project."
- "Public functions in these modules can be imported by other modules, but they're
  considered to be part of the package's internal implementation and as such are not
  documented or expected to give the same stability guarantees as functions in the public
  API."
- "The --dev flag is used to indicate that this package is only used for building,
  developing, and testing the project, and should not be included in the final production
  builds."
- "With it any public function in the test/ directory with a name ending in _test will
  be run as a test."

## Version notes
- No Gleam version is stated on the page itself.
- `gleam_stdlib` constraint shown: `>= 0.34.0 and < 2.0.0`.
- `envoy >= 1.0.1 and < 2.0.0`, `argv >= 1.0.2 and < 2.0.0`, `gleeunit >= 1.0.0 and < 2.0.0`.
- escript compatibility: "within a few major versions of the version of Erlang on the
  computer used to compile the escript."

## Discovered links

### Relevant (crawl later)
1. https://gleam.run/writing-gleam/gleam-toml — gleam.toml configuration reference (directly
   cited as "gleam.toml definition for more information"; core to project-structure docs).
2. https://gleam.run/documentation/#deployment — deployment section (cited for web app
   sharing; natural follow-up to "sharing your program").
3. https://gleam.run/documentation — Docs hub (parent index; may list further writing-gleam
   sub-pages and CLI reference).
4. https://gleam.run/getting-started/installing/ — install guide (prerequisite; useful for
   environment setup docs).
5. https://tour.gleam.run/ — language tour (prerequisite referenced by the page).
6. https://packages.gleam.run/ — Gleam Package Index (dependency discovery).
7. https://hexdocs.pm/gleeunit/ — gleeunit test runner docs (testing tooling).
8. https://hexdocs.pm/envoy/ — envoy package docs.
9. https://hexdocs.pm/argv/ — argv package docs.

### Skipped
- / (Gleam home), /news, /community, /sponsor, /install (top nav).
- https://hex.pm/ (Hex pm homepage — general).
- https://github.com/features/actions (GitHub Actions marketing).
- https://semver.org/ (semver spec — general reference).
- https://discord.gg/Fm8Pwmy (Discord invite).
- https://github.com/gleam-lang (GitHub org).
- https://shop.gleam.run/en-gbp (merch).
- https://playground.gleam.run (playground).
- https://gleamweekly.com/ (newsletter).
- /roadmap, /case-studies (non-guide content).
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md.
