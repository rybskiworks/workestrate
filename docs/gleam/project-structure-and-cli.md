# Project structure and CLI

## Purpose

This document defines Gleam guidance for project layout, the `gleam` command-line
tool, the language server, and installation. Future agents who scaffold, build, run,
test, format, or edit Gleam projects should follow these rules so project structure
is consistent and CLI usage matches the official reference.

This doc is Gleam-specific. It is not a language tutorial; it assumes the reader has
done the Gleam language tour.

## Sources used

- `.crawl/03-writing-gleam.md` — https://gleam.run/writing-gleam/ (PRIMARY — project lifecycle, `gleam new` tree, src/test dirs, internal modules, manifest.toml, escript)
- `.crawl/06-cli.md` — https://gleam.run/command-line-reference/ (PRIMARY — every subcommand, signature, flags verbatim from `--help`)
- `.crawl/07-language-server.md` — https://gleam.run/language-server/ (PRIMARY — LSP features, editors, limitations)
- `.crawl/10-install.md` — https://gleam.run/install (PRIMARY — install hub, OS chooser)

## Related BEAM guidance

- `docs/beam/applications.md` — OTP application structure. Gleam's `gleam new` scaffold
  and `erlang.application_start_module` map onto BEAM application concepts; consult this
  doc when wiring an OTP application start module or `extra_applications`.

## Core guidance

### Project layout (`gleam new`)

`gleam new <project_root>` scaffolds:

```
.
├── .github/workflows/test.yml   # GitHub Actions CI running tests on push
├── .gitignore
├── README.md                    # markdown intro docs
├── gleam.toml                   # project configuration
├── src/<package>.gleam         # program source; entrypoint = main fn in pkg-named module
└── test/<package>_test.gleam   # test code
```

- `src/` — program source. One module per file; the module name is the file path.
- `test/` — test code. Any public function in `test/` whose name ends in `_test` is
  run as a test by `gleeunit`.
- `gleam.toml` — required project config (see `gleam-toml-and-targets.md`).
- `manifest.toml` — lockfile; check into VCS; NOT uploaded to Hex; not used by
  downstream dependents.
- A "Gleam package" = the whole thing, whether library or runnable program.

### Entrypoint convention

The entrypoint for a program is the function called `main` in the module with the
same name as the package itself. Run a different module with `gleam run -m <module>`.

### Internal modules

Modules matching `packagename/internal` and `packagename/internal/*` are "internal":
public functions here are importable within the package but are NOT documented and
do NOT carry public-API stability guarantees. (Configured via `internal_modules` in
`gleam.toml`; see `gleam-toml-and-targets.md`.)

### Targets

Erlang is the default target. JavaScript is selected with `--target javascript`
(per-command) or `target = "javascript"` in `gleam.toml`.

### Build tool

The build tool is built into the `gleam` binary — there is no separate build tool.
`gleam_stdlib` is included by default in new projects; `gleeunit` is the default
test runner.

## CLI reference (subcommands, verbatim flags)

The `gleam` command is subcommand-based: `gleam <subcommand> [opts]`.

| Command | Signature | Key flags |
|---|---|---|
| `new` | `gleam new [OPTIONS] <PROJECT_ROOT>` | `--name <NAME>`, `--skip-git`, `--skip-github`, `--template <TEMPLATE>` (`erlang` default \| `javascript`) |
| `build` | `gleam build [OPTIONS]` | `-t, --target <TARGET>`, `--warnings-as-errors` |
| `check` | `gleam check [OPTIONS]` | `-t, --target <TARGET>` |
| `run` | `gleam run [OPTIONS] [ARGUMENTS]...` | `-m, --module <MODULE>`, `--runtime <RUNTIME>`, `-t, --target <TARGET>` |
| `test` | `gleam test [OPTIONS] [ARGUMENTS]...` | `--runtime <RUNTIME>`, `-t, --target <TARGET>` |
| `dev` | `gleam dev [OPTIONS] [ARGUMENTS]...` | `--runtime <RUNTIME>`, `-t, --target <TARGET>` |
| `add` | `gleam add [OPTIONS] <PACKAGES>...` | `--dev` |
| `remove` | `gleam remove <PACKAGES>...` | (none) |
| `update` | `gleam update` | (none) — updates deps to latest within constraints |
| `deps` | `gleam deps <SUBCOMMAND>` | `download`, `list`, `update` |
| `format` | `gleam format [OPTIONS] [FILES]...` | `--check`, `--stdin` |
| `clean` | `gleam clean` | (none) — clean build artifacts |
| `fix` | `gleam fix` | (none) — rewrite deprecated Gleam code |
| `shell` | `gleam shell` | (none) — start an Erlang shell |
| `lsp` | `gleam lsp` | (none) — run the language server |
| `help` | `gleam help [SUBCOMMAND]...` | print help |
| `docs` | `gleam docs <SUBCOMMAND>` | `build [--open]`, `publish`, `remove --package <P> --version <V>` |
| `publish` | `gleam publish [OPTIONS]` | `--replace`, `-y, --yes` |
| `export` | `gleam export <SUBCOMMAND>` | `erlang-shipment`, `escript`, `hex-tarball`, `javascript-prelude`, `typescript-prelude`, `package-interface --out <OUTPUT>` |
| `hex` | `gleam hex <SUBCOMMAND>` | `retire <P> <V> <REASON> [MSG]`, `unretire <P> <V>` |

Notes (corrections vs. common assumptions):
- `new` has NO `--target` flag and NO `lib`/`bare` templates — only `--template erlang|javascript`.
- `run`/`test` have NO `--rerun` flag.
- `deps` has NO `deps tree` subcommand — only `download`/`list`/`update`.
- `update` is a top-level command (alias-equivalent to `deps update`).
- `export package-interface` requires `--out <OUTPUT>` (JSON path).
- `--target <TARGET>` is valid for `build`, `check`, `run`, `test`, `dev`.
- `--warnings-as-errors` is `build`-only.
- `publish`, `docs publish`/`docs remove`, `hex retire`/`hex unretire` authenticate
  via `HEXPM_USER` / `HEXPM_PASS` env vars (both optional).

### Language server (LSP)

The Gleam Language Server is bundled inside the regular `gleam` binary; installing
Gleam installs the LSP. Editors run `gleam lsp` from the workspace root.

Capabilities: multiple project support; project compilation (analysis only — code
generation and Erlang compilation are NOT performed); error/warning diagnostics;
code formatting (configurable on save); hover; go-to definition; go-to type
definition; find references; code completion; rename; document symbols; signature
help; code folding; and a large set of code actions (quick fixes/refactors).

Editor support:
- **VS Code** — install the Gleam plugin; auto-starts on `.gleam` open. Ensure
  `gleam` is on VS Code's PATH.
- **Neovim** — via `nvim-lspconfig`. Nvim 0.11+ / nvim-lspconfig 2.1+:
  `vim.lsp.enable('gleam')`. Nvim <= 0.10: `require('lspconfig').gleam.setup({})`.
  Run `:TSInstall gleam` with `nvim-treesitter` for highlighting.
- **Helix** / **Gram** / **Zed** — support the LSP out of the box; auto-start.
- **Other editors (Emacs etc.)** — any LSP client configured to run `gleam lsp`
  from the workspace root.

Limitations:
- The LSP performs analysis compilation only — no code generation, no Erlang/Elixir
  compilation, so opening a file cannot execute code.
- For files outside a Gleam project, only code formatting is available.
- The target used is the one in `gleam.toml`; defaults to Erlang when unspecified.
- Unsaved edits are used when compiling in the LSP.

### Installation

The install page (`https://gleam.run/install`) is an OS chooser hub branching to
per-OS subpages: Linux, macOS, Windows, Android, FreeBSD, OpenBSD. Actual install
methods (version managers, package managers, build from source, GitHub releases)
and the Erlang/OTP runtime requirement live on those subpages, not the hub. Gleam
compiles to Erlang/OTP and needs an Erlang installation.

## Practical rules

1. Every Gleam package requires a `gleam.toml`.
2. Check `manifest.toml` into VCS; it is not uploaded to Hex.
3. Use `--dev` for build/test-only dependencies (excluded from production builds).
4. `gleam run` runs `main` in the package-named module; use `-m` for another module.
5. `gleam format --check` is the CI formatting gate (no rewrite).
6. `gleam build --warnings-as-errors` promotes compile warnings to errors.
7. `gleam check` type-checks without codegen (faster than `build`).
8. Internal modules (`packagename/internal/*`) are not public API.
9. `gleam new --template javascript` scaffolds a JS-target project.
10. `gleam export erlang-shipment` produces precompiled Erlang for deployment;
    `gleam export escript` bundles a CLI escript (via `gleescript` dev dep).

## Review checklist

- [ ] `gleam.toml` present with `name` and `version`.
- [ ] `manifest.toml` committed and reflects current deps.
- [ ] No dependency appears in both `dependencies` and `dev_dependencies`.
- [ ] Dev-only deps use `--dev` / `[dev_dependencies]`.
- [ ] CI runs `gleam format --check`, `gleam build --warnings-as-errors`, `gleam test`.
- [ ] Internal helper modules live under `packagename/internal/*`.
- [ ] Entrypoint is `main` in the package-named module (or `-m` documented).

## Implementation checklist

- [ ] Scaffold with `gleam new <name>` (or `--template javascript` for JS target).
- [ ] Add deps with `gleam add <pkg>` (`--dev` for dev-only).
- [ ] Run `gleam build` to compile; `gleam run` to execute.
- [ ] Write tests in `test/` as `*_test` public fns; run `gleam test`.
- [ ] Format with `gleam format`; gate with `gleam format --check`.
- [ ] Configure editor to run `gleam lsp`.

## Validation hooks

- `gleam format --check` — formatting gate (CI).
- `gleam build --warnings-as-errors` — compile + warnings-as-errors gate.
- `gleam check` — type-check gate (no codegen).
- `gleam test` — test runner (gleeunit).
- `gleam deps list` — verify resolved dependency set.

## Examples

Scaffold and run a CLI:
```sh
gleam new vars
cd vars
gleam add envoy argv
gleam run   # runs main in src/vars.gleam
gleam run -m vars.get USER   # run a different module
gleam test
gleam format --check
```

Bundle a CLI as an escript (Erlang target):
```sh
gleam add --dev gleescript
gleam build && gleam run -m gleescript
./vars   # runnable on any Erlang install within a few major versions
```

## Common mistakes

- Treating `manifest.toml` as publishable — it is NOT uploaded to Hex and not used
  by downstream dependents.
- Putting a dep in both `dependencies` and `dev_dependencies` (forbidden).
- Expecting `new` to accept `--target` or `lib`/`bare` templates (only
  `--template erlang|javascript`).
- Expecting `deps tree` (does not exist; use `deps list`).
- Expecting the LSP to compile/run code (analysis only; no codegen/execution).
- Opening a lone `.gleam` file outside a project and expecting full LSP features
  (only formatting works).

## Strict vs contextual guidance

Strict: `gleam.toml` required; `manifest.toml` committed; no dep in both dep
tables; `format --check` and `build --warnings-as-errors` in CI; `--template`
values limited to `erlang`/`javascript`.

Contextual: choice of `erlang` vs `javascript` target; whether to ship an escript;
editor choice; whether to use `gleam check` vs `gleam build` in a given CI step.

## Policy decisions for individual repos

- Pin a Gleam compiler version constraint (`gleam = ">= x.y.z"`) in `gleam.toml`.
- Choose default `target` (`erlang` or `javascript`).
- Decide whether `manifest.toml` is committed (recommended yes).
- Choose CI gate set (`format --check`, `build --warnings-as-errors`, `test`).
- Decide escript vs `erlang-shipment` for CLI/deployment packaging.

## Related docs

- `gleam-toml-and-targets.md` — full `gleam.toml` reference and target config.
- `javascript-target.md` — JavaScript target behavior and interop.
- `package-management-and-publishing.md` — deps, lockfile, publishing, SBOM.
- `testing.md` — gleeunit test patterns.
- `deployment-and-runtime.md` — `erlang-shipment` and deployment.

## Related skills

- `gleam-packages-ffi`
- `gleam-language`