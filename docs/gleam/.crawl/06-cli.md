# Crawl: command-line-reference/
- seed_url: https://gleam.run/writing-gleam/command-line-reference/
- canonical_url: https://gleam.run/command-line-reference/
- family: Gleam official guide
- fetch: 200 (seed URL 301-redirects to /command-line-reference/ via JS redirect `window.location = "/command-line-reference"`)
- gleam_version: not present on page
- feeds_docs: project-structure-and-cli.md, validation.md

## Purpose
Reference for the `gleam` command line program. The `gleam` command uses
**subcommands** to access different parts of the functionality. This page is a
generated CLI reference listing every subcommand, its signature, and its flags
verbatim from `--help` output.

## Commands (each: name, purpose, key flags — verbatim where possible)

### add
`gleam add [OPTIONS] <PACKAGES>...` — Add new project dependencies.
| Option | Description |
| --dev | Add the packages as dev-only dependencies |

### build
`gleam build [OPTIONS]` — Build the project.
| Option | Description |
| `-t, --target <TARGET>` | The platform to target |
| `--warnings-as-errors` | Emit compile time warnings as errors |

### check
`gleam check [OPTIONS]` — Type check the project.
| Option | Description |
| `-t, --target <TARGET>` | The platform to target |

### clean
`gleam clean` — Clean build artifacts. (No options.)

### deps
`gleam deps <SUBCOMMAND>` — Work with dependency packages.
- `gleam deps download` — Download all dependency packages.
- `gleam deps list` — List all dependency packages.
- `gleam deps update` — Update dependency packages to their latest versions.

### dev
`gleam dev [OPTIONS] [ARGUMENTS]...` — Run the project development entrypoint.
| Option | Description |
| `--runtime <RUNTIME>` | |
| `-t, --target <TARGET>` | The platform to target |

### docs
`gleam docs <SUBCOMMAND>` — Render HTML documentation.
- `gleam docs build [OPTIONS]` — Render HTML docs locally.
  - `--open` — Opens the docs in a browser after rendering.
- `gleam docs publish` — Publish HTML docs to HexDocs.
  - Env: `HEXPM_USER` (optional), `HEXPM_PASS` (optional).
- `gleam docs remove --package <PACKAGE> --version <VERSION>` — Remove HTML docs from HexDocs.
  - Env: `HEXPM_USER` (optional), `HEXPM_PASS` (optional).
  - `--package <PACKAGE>` — The name of the package.
  - `--version <VERSION>` — The version of the docs to remove.

### export
`gleam export <SUBCOMMAND>` — Export something useful from the Gleam project.
- `gleam export erlang-shipment` — Precompiled Erlang, suitable for deployment.
- `gleam export escript` — Escript file, for running in a script-like manner.
- `gleam export hex-tarball` — The package bundled into a tarball, suitable for publishing to Hex.
- `gleam export javascript-prelude` — The JavaScript prelude module.
- `gleam export package-interface --out <OUTPUT>` — Information on the modules, functions, and types in the project in JSON format.
  - `--out <OUTPUT>` — The path to write the JSON file to.
- `gleam export typescript-prelude` — The TypeScript prelude module.

### fix
`gleam fix` — Rewrite deprecated Gleam code. (No options.)

### format
`gleam format [OPTIONS] [FILES]...` — Format source code.
| Option | Description |
| `--check` | Check if inputs are formatted without changing them |
| `--stdin` | Read source from STDIN |

### help
`gleam help [SUBCOMMAND]...` — Print this message or the help of the given subcommand(s).

### hex
`gleam hex <SUBCOMMAND>` — Work with the Hex package manager.
- `gleam hex retire <PACKAGE> <VERSION> <REASON> [MESSAGE]` — Retire a release from Hex.
  - Env: `HEXPM_USER` (optional), `HEXPM_PASS` (optional).
- `gleam hex unretire <PACKAGE> <VERSION>` — Un-retire a release from Hex.
  - Env: `HEXPM_USER` (optional), `HEXPM_PASS` (optional).

### lsp
`gleam lsp` — Run the language server, to be used by editors. (No options.)

### new
`gleam new [OPTIONS] <PROJECT_ROOT>` — Create a new project.
| Option | Description |
| `--name <NAME>` | Name of the project |
| `--skip-git` | Skip git initialization and creation of `.gitignore`, `.git/*` and `.github/*` files |
| `--skip-github` | Skip creation of `.github/*` files |
| `--template <TEMPLATE>` | [default: erlang] [possible values: erlang, javascript] |

### publish
`gleam publish [OPTIONS]` — Publish the project to the Hex package manager.
Env: `HEXPM_USER` (optional), `HEXPM_PASS` (optional).
| Option | Description |
| `--replace` | |
| `-y, --yes` | |

### remove
`gleam remove <PACKAGES>...` — Remove project dependencies. (No options.)

### run
`gleam run [OPTIONS] [ARGUMENTS]...` — Run the project.
| Option | Description |
| `-m, --module <MODULE>` | The module to run |
| `--runtime <RUNTIME>` | |
| `-t, --target <TARGET>` | The platform to target |

### shell
`gleam shell` — Start an Erlang shell. (No options.)

### test
`gleam test [OPTIONS] [ARGUMENTS]...` — Run the project tests.
| Option | Description |
| `--runtime <RUNTIME>` | |
| `-t, --target <TARGET>` | The platform to target |

### update
`gleam update` — Update dependency packages to their latest versions. (No options.)

## Version-constraint syntax (gleam add)
Not documented on this page. The `gleam add` reference only lists the `--dev`
flag and a `<PACKAGES>...` positional; version-constraint syntax (e.g.
`package@>= 1.0.0`) is not described here. Likely covered in the
"dependencies" writing-gleam page or the manifest format docs — crawl later.

## Lockfile/manifest behavior
Not documented on this page. No mention of `gleam.toml` (manifest) or
`manifest.toml` (lockfile) on this reference page. Behavior (lockfile
regeneration, `gleam deps download` writing `manifest.toml`, etc.) must be
sourced from the dependencies / project-configuration pages — crawl later.

## Strict rules
- The `gleam` command is **subcommand-based**: `gleam <subcommand> [opts]`.
- `--target <TARGET>` selects platform; valid for `build`, `check`, `run`,
  `test`, `dev`.
- `--warnings-as-errors` (build only) promotes compile-time warnings to errors.
- `format --check` verifies formatting without rewriting (CI gate).
- `format --stdin` reads source from STDIN (editor integration).
- `publish` and `docs publish`/`docs remove`/`hex retire`/`hex unretire`
  authenticate via `HEXPM_USER` / `HEXPM_PASS` env vars.
- `new --template` accepts only `erlang` (default) or `javascript`.
- `new --skip-git` skips git init AND `.gitignore`/`.git/*`/`.github/*`.
- `new --skip-github` skips only `.github/*` files.
- `export package-interface` requires `--out <OUTPUT>` (JSON path).

## Verbatim quotes
- "The gleam command uses subcommands to access different parts of the
  functionality."
- `new --template <TEMPLATE>` — "[default: erlang] [possible values: erlang, javascript]"
- `build --warnings-as-errors` — "Emit compile time warnings as errors"
- `format --check` — "Check if inputs are formatted without changing them"
- `format --stdin` — "Read source from STDIN"
- `export package-interface` — "Information on the modules, functions, and types in the project in JSON format"
- `export erlang-shipment` — "Precompiled Erlang, suitable for deployment"
- `export hex-tarball` — "The package bundled into a tarball, suitable for publishing to Hex"
- `docs publish` / `docs remove` / `hex retire` / `hex unretire` / `publish`
  all share: "This command uses this environment variables: HEXPM_USER:
  (optional) The Hex username to authenticate with. HEXPM_PASS: (optional) The
  Hex password to authenticate with."

## Version notes
- No Gleam version number is printed on this page.
- Page is auto-generated from clap `--help` output (Rust CLI parser).
- Notable: the task brief's expected flags differ from the actual page:
  - `new` has NO `--target` and NO `lib`/`bare` templates (only `erlang`/`javascript`).
  - `run`/`test` have NO `--rerun` flag.
  - `deps` has NO `deps tree` subcommand (only `download`/`list`/`update`).
  - `export` uses `typescript-prelude` (not `typescript`) and
    `package-interface` (requires `--out`); also adds `erlang-shipment`,
    `escript`, `hex-tarball`, `javascript-prelude`.
  - Additional commands not in the brief: `dev`, `fix`, `help`, `hex retire`/
    `hex unretire`, `lsp`.
  - `update` is a top-level command (alias-equivalent to `deps update`).

## Discovered links
### Relevant (crawl later)
- https://gleam.run/documentation — Docs index; likely links to dependencies /
  project-configuration / manifest-format pages that document version-constraint
  syntax and lockfile behavior (gaps noted above).

### Skipped
- / (home), /news, /community, /sponsor, /install, /case-studies, /roadmap
  (nav/marketing, not CLI reference).
- https://packages.gleam.run , https://packages.gleam.run/ (package registry).
- https://playground.gleam.run , https://tour.gleam.run (interactive tools).
- https://discord.gg/Fm8Pwmy , https://gleamweekly.com ,
  https://shop.gleam.run/en-gbp , https://github.com/gleam-lang ,
  https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md ,
  https://gleam.run/feed.xml (community/social/feed).
- /images/lucy/lucy.svg , /styles/main.css (assets).
