# Static Analysis with Credo

## Purpose

Credo is a static code analysis tool for Elixir. From [overview.html](https://hexdocs.pm/credo/overview.html):

> "Credo is a static code analysis tool for Elixir with a focus on teaching and code consistency."

Use it during local development and review to surface refactoring opportunities, common mistakes, naming inconsistencies, and complex code fragments. It is comparable to ESLint, RuboCop, or Stylecop, but it emphasizes teaching over style enforcement. Not every finding is equal: each issue is assigned a priority, and the default output deliberately hides low-priority issues.

## Sources used

- https://hexdocs.pm/credo/overview.html
- https://hexdocs.pm/credo/installation.html
- https://hexdocs.pm/credo/basic_usage.html (PRIMARY)
- https://hexdocs.pm/credo/mix_tasks.html
- https://hexdocs.pm/credo/suggest_command.html
- https://hexdocs.pm/credo/explain_command.html
- https://hexdocs.pm/credo/cli_switches.html
- https://hexdocs.pm/credo/config_file.html
- https://hexdocs.pm/credo/check_params.html
- https://hexdocs.pm/credo/exit_statuses.html
- https://hexdocs.pm/credo/checks.html (returns 404 in v1.7.x; the catalog lives in config_file.html "Default checks" + per-check module pages)
- https://hexdocs.pm/credo/config_comments.html (inline disable directives)
- https://hexdocs.pm/credo/Credo.Check.html (check behaviour; category enum)
- https://hexdocs.pm/credo/Credo.Check.Readability.ModuleNames.html (representative per-check module page)
- https://github.com/rrrene/credo/blob/master/lib/credo/cli/command/categories/categories_command.ex (category titles/descriptions)
- https://github.com/rrrene/credo (README/source)
- https://hex.pm/packages/credo

This page reflects Credo v1.7.x docs (hex.pm shows 1.7.19; hexdocs mirror 1.7.18). Author: René Föhring (@rrrene); MIT license.

## Basic Usage

### Installation

From [installation.html](https://hexdocs.pm/credo/installation.html):

> "Add Credo to your dependencies."

```elixir
defp deps do
  [
    {:credo, "~> 1.7", only: [:dev, :test], runtime: false}
  ]
end
```

Then run:

```bash
mix deps.get
mix credo
```

`only: [:dev, :test]` keeps Credo out of production. `runtime: false` prevents it from being loaded in releases. Pin to `~> 1.7`.

### Mix tasks

From [mix_tasks.html](https://hexdocs.pm/credo/mix_tasks.html):

| Task | Description |
|---|---|
| `mix credo` | Default task; alias for `mix credo suggest`. Runs analysis and suggests edits. |
| `mix credo suggest` | Default command; issues grouped by category, capped to the top N per category. |
| `mix credo list` | Same issues grouped by file; no per-category cap. Supports `--format oneline`. |
| `mix credo diff <ref>` | Report only issues in code changed since a git ref or directory. |
| `mix credo explain <location>` | Explain a specific issue. |
| `mix credo info` | Show enabled checks. Add `--verbose` for more detail. |
| `mix credo categories` | Show available categories. |
| `mix credo version` | Print version. `-v` and `--version` also work. |
| `mix credo.gen.config` | Generate a default `.credo.exs`. |
| `mix credo.gen.check` | Generate a custom check module. |

### Running Credo

```bash
mix credo
```

Default output groups findings by category and caps the list to a limited number of issues per category.

To show every issue regardless of the per-category cap:

```bash
mix credo --all
```

To also include low-priority issues, run in strict mode:

```bash
mix credo --strict
```

`--strict` is an alias for `--all-priorities`.

### Reading the output

Sample output from [basic_usage.html](https://hexdocs.pm/credo/basic_usage.html):

```text
┃  Refactoring opportunities
┃
┃ [F] ↗ Avoid negated conditions in if-else blocks.
┃       lib/foo/bar.ex:306 #(Foo.Bar.deprecated_def_explanations)
```

Anatomy of a finding:

- Category header (e.g. `Refactoring opportunities`, `Warnings`, `Consistency`, `Readability`, `Design`).
- One-letter category tag in brackets: `[C]` Consistency, `[R]` Readability, `[F]` Refactor, `[W]` Warning, `[D]` Design.
- Priority arrow: `↑ ↗ → ↘ ↓` from higher to lower importance.
- Message text.
- Location: `file:line_no` or `file:line_no:column`.
- Optional scope: `#(Module.function)`.
- With `--verbose`, Credo also prints the check module name and the offending source line.

### Categories

From [exit_statuses.html](https://hexdocs.pm/credo/exit_statuses.html): Consistency, Design, Readability, Refactor, Warning. Plugins and custom checks can add more.

### Severity / priority

Credo assigns a priority to each issue. It is shown as an arrow in default output:

```text
↑  ↗  →  ↘  ↓
```

From [basic_usage.html](https://hexdocs.pm/credo/basic_usage.html):

> "By default, only the issues with a positive priority are shown (the arrows pointing up or right)."

By default Credo shows only positive-priority issues. The arrow order above is from highest to lowest importance. The exact mapping of each arrow glyph to the four configurable priority atoms (`:low`, `:normal`, `:high`, `:higher`) is inferred from doc examples rather than explicitly documented; do not assert a fixed glyph-to-atom mapping.

Configurable priority atoms are `:low`, `:normal`, `:high`, `:higher`. Use `--min-priority` with `higher|high|normal|low|ignore` or an integer to filter. JSON output exposes a numeric `priority` field combining the base priority with dynamic factors.

### Command-line options

From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html) and [suggest_command.html](https://hexdocs.pm/credo/suggest_command.html):

| Flag | Description |
|---|---|
| `-a, --all` | Show all issues; default caps the top ~N per category. |
| `-A, --all-priorities` | Show all issues, including low-priority ones. |
| `--strict` | Alias for `--all-priorities`. |
| `-c, --checks PATTERN` / `--only PATTERN` | Run only checks matching the comma-separated pattern(s); accepts regex. |
| `-i, --ignore-checks PATTERN` / `--ignore PATTERN` | Ignore checks matching the pattern(s). |
| `--checks-with-tag TAG` | Only run checks with this tag; repeatable. |
| `--checks-without-tag TAG` | Exclude checks with this tag; repeatable. |
| `--min-priority LEVEL` | Show issues at or above `higher|high|normal|low|ignore` or an integer. |
| `-f, --format FORMAT` | Output format: `json`, `flycheck`, `sarif`, `oneline`. |
| `--files-included GLOB` | Only include matching files; repeatable. |
| `--files-excluded GLOB` | Exclude matching files; repeatable. |
| `--config-file PATH` | Use the given config file; disables transitive config lookup. |
| `-C, --config-name NAME` | Use the named config (default `default`). |
| `--enable-disabled-checks [PATTERN]` | Re-enable checks disabled in `.credo.exs`; CLI-only. |
| `--mute-exit-status` | Exit 0 even when issues are found; CLI-only. |
| `--verbose` | Print the check module and the offending source line. |
| `--watch` | Re-run analysis when files change. |
| `--read-from-stdin` | Read source from stdin (useful for editor integrations). |
| `--[no-]color` | Enable/disable colored output. |
| `-v, --version` / `-h, --help` | Print version or help. |

`--enable-disabled-checks` and `--mute-exit-status` have no `.credo.exs` equivalent.

### The `explain` flag

From [explain_command.html](https://hexdocs.pm/credo/explain_command.html) and [basic_usage.html](https://hexdocs.pm/credo/basic_usage.html):

Copy the location string from a report and run:

```bash
mix credo explain lib/my_app/server.ex:10:24
```

The same can be invoked without the `explain` subcommand:

```bash
mix credo lib/my_app/server.ex:10:24
```

It prints an explanation of the issue, the check's configuration options, and how to disable the check. Use `--format json` for machine-readable explain output.

### The `strict` flag

From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html), [basic_usage.html](https://hexdocs.pm/credo/basic_usage.html), and [config_file.html](https://hexdocs.pm/credo/config_file.html):

```bash
mix credo --strict
```

`--strict` is an alias for `--all-priorities`, so it also shows low-priority issues. The config equivalent is `strict: true` in `.credo.exs`. Use this for CI gating and exhaustive review.

### Exit status

From [exit_statuses.html](https://hexdocs.pm/credo/exit_statuses.html):

- `0` — success / no issues.
- `1..127` — issues found; the code is a bitwise OR of per-category codes:
  - consistency = 1
  - design = 2
  - readability = 4
  - refactor = 8
  - warning = 16
  - custom = 32 (and 64, ...)
- Example: `12` = `4 + 8` = readability + refactor.
- `>= 128` — runtime error during analysis:
  - `128` generic runtime error
  - `129-131` config errors
  - `132-191` reserved
  - `192-255` custom

Use `--mute-exit-status` to force exit 0 even when issues are present.

### Output formats

From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html):

| Format | Use when |
|---|---|
| `json` | CI/tooling ingestion. |
| `sarif` | Static-analysis tool integration (e.g. GitHub Advanced Security). |
| `flycheck` | Editor integration. |
| `oneline` | Grep-friendly summary or quick local review. |

JSON issue fields include: `category`, `check`, `column`, `filename`, `line_no`, `message`, `priority` (int), `related_code`, `scope`, `trigger`, and `explanation_for_issue` from explain output.

## Configuration File

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Credo is configured via a file called `.credo.exs`."

The file is a plain Elixir script that evaluates to a single map. Generate a complete starting point with:

```bash
mix credo gen.config
```

### Location

The file may live in the project root or in `config/` — both are valid. Credo finds it in either place.

### Top-level structure

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Credo's config is a plain `.exs` file, no magic here. It contains a map with a single key (`:configs`), which contains a list of maps that represent the individual configs (most of the time, it's just one, named "default")."

Skeleton:

```elixir
# .credo.exs or config/.credo.exs
%{
  configs: [
    %{
      name: "default",
      files: %{
        included: ["lib/", "src/", "web/", "apps/"],
        excluded: []
      },
      plugins: [],
      requires: [],
      strict: false,
      parse_timeout: 5000,
      color: true,
      checks: %{
        enabled: [
          {Credo.Check.Design.AliasUsage, priority: :low}
          # ... other checks ...
        ]
      }
    }
  ]
}
```

Credo 1.7.x documents exactly eight keys inside each config map:

| Key | Type | Default | Purpose |
|---|---|---|---|
| `name` | string | `"default"` | Select with `--config-name`. |
| `files` | `%{included, excluded}` | — | File/path filtering for the whole config. |
| `requires` | list | `[]` | Source files to `require` before checks run. |
| `plugins` | list of `{Mod, params}` | `[]` | Plugin modules to load at startup. |
| `checks` | `%{enabled, disabled, extra}` | default set | Which checks run, with params. |
| `color` | boolean | `true` | Toggle colored output. |
| `parse_timeout` | integer (ms) | `5000` | Source-file parse timeout. |
| `strict` | boolean | `false` | Include low-priority issues. |

> Note: `:lint_at_compile`, `:parse`, `:parse_without_term`, and `:checks_without_term` are **not** part of the documented config surface in Credo 1.7.x. The documented parse-related key is `parse_timeout`; strictness is controlled by `strict`.

### `name` — multiple named configs

Each entry in `configs:` has a `name`. The default is `"default"`. Run a named config with `--config-name`:

```bash
mix credo --config-name spring-cleaning
```

```elixir
%{
  configs: [
    %{
      name: "default",
      files: %{included: ["lib/", "src/", "web/", "apps/"], excluded: ["lib/that_big_namespace"]}
    },
    %{
      name: "spring-cleaning",
      files: %{included: ["lib/that_big_namespace"], excluded: []}
    }
  ]
}
```

### `files` — includes and excludes

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "`:included` contains a list of files, directories and globs (as in `"**/*_test.exs"`); `:excluded` contains a list of files, directories, globs … and regular expressions (as in `~r"/_build/"`)."

```elixir
files: %{
  included: ["mix.exs", "lib/", "src/", "web/", "apps/"],
  excluded: ["test/", ~r"/_build/"]
}
```

- `:included` accepts files, directories, and globs.
- `:excluded` accepts files, directories, globs, **and** regexes.

### `checks` — activation, deactivation, and params

`checks:` is a map with three sub-lists. Every entry is a two-element tuple `{Credo.Check.Module, params}` where `params` is either `false` (disable) or a keyword list.

**`:enabled` — pin the project's check set.** Default checks are bypassed; only the listed checks run:

```elixir
checks: %{
  enabled: [
    {Credo.Check.Consistency.TabsOrSpaces, []}
  ]
}
```

**`:disabled` — drop specific checks from the defaults.** Everything else still runs, and these checks keep their params so they can be re-enabled:

```elixir
checks: %{
  disabled: [
    {Credo.Check.Consistency.TabsOrSpaces, []}
  ]
}
```

**`:extra` — add checks on top of the defaults.** Used by plugins and by umbrella child apps to override specific check params without losing the parent config:

```elixir
# apps/my_app2/.credo.exs
checks: %{
  extra: [
    {Credo.Check.Readability.LargeNumbers, only_greater_than: 99_999}
  ]
}
```

A **flat `checks:` list** is also valid shorthand and is treated as `extra` entries — combine disabling (`false`) with param customization:

```elixir
checks: [
  {Credo.Check.Consistency.TabsOrSpaces, false},
  {Credo.Check.Design.AliasUsage, if_nested_deeper_than: 2}
]
```

#### Configuring individual checks (params)

From [check_params.html](https://hexdocs.pm/credo/check_params.html):

> "All checks are configured using a two-element tuple: `{MyApp.CheckModule, params}`."

Beyond each check's own params, **every** check accepts these general params:

| Param | Effect |
|---|---|
| `:category` | Overwrite the check's category. |
| `:exit_status` | Overwrite the check's exit status. |
| `:files` | Per-check file filter (same syntax as the top-level `:files`). |
| `:priority` | Overwrite the check's priority. |
| `:tags` | Overwrite or append to the check's tags (`tags: [:__initial__, :my_tag]` appends). |

Per-check file filtering example:

```elixir
{Credo.Check.Consistency.ExceptionNames, files: %{included: ["lib/**/*.ex"]}},
{Credo.Check.Warning.IExPry,             files: %{excluded: ["lib/debug_server.ex"]}},
{Credo.Check.Warning.IoInspect,           files: %{included: ["**/*.exs"], excluded: ["**/*_test.exs"]}}
```

From [check_params.html](https://hexdocs.pm/credo/check_params.html):

> "Please note that these params do not 'override' the top-level config, but are applied to the result of the top-level config's resolution."

### Priority adjustment

From [check_params.html](https://hexdocs.pm/credo/check_params.html):

> "Available priorities are: `:low`, `:normal`, `:high` and `:higher`."

The full ordering (lowest to highest) is `:ignore | :low | :normal | :high | :higher`. Set priority per check:

```elixir
{Credo.Check.Warning.IoInspect, priority: :low}
```

- Default `mix credo` reports only positive-priority issues.
- `--strict` / `--all-priorities` includes low-priority issues.
- `--min-priority LEVEL` accepts `higher|high|normal|low|ignore` or an integer.

> The configured priority atoms are `:ignore`, `:low`, `:normal`, `:high`, `:higher`. There is no `:lower` priority in Credo.

### `requires` — loading custom checks

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Configures Elixir source files that Credo should require at start up (defaults to `[]`). This is needed to add local custom checks in the analysis."

`requires:` is a list of files or globs that Credo `require`s before running. It is the mechanism for loading local custom checks:

```elixir
%{
  configs: [
    %{
      name: "default",
      requires: ["./lib/my_project/checks/**/*.ex"],
      checks: [
        {MyProject.Checks.RejectModuleAttributes, []}
      ]
    }
  ]
}
```

### `plugins`

```elixir
plugins: [
  {CredoDemoPlugin, []}
]
```

`params` may be `false` (disable) or a keyword list. Default `[]`.

### `color`

```elixir
%{configs: [%{name: "default", color: false}]}
```

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Set to `false` to disable colored output (defaults to `true`). This is equivalent to using the `--no-color` CLI switch."

### `parse_timeout`

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Configures a timeout for parsing source files in milliseconds (defaults to 5000 milliseconds)."

```elixir
parse_timeout: 60_000
```

Increase it for very large files that exceed the default parse budget.

### `strict`

```elixir
%{configs: [%{name: "default", strict: true}]}
```

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Set to `true` to enable low priority checks (defaults to `false`). This is equivalent to using the `--strict` CLI switch."

See [Strict Mode](#strict-mode) below.

### How Credo selects a config

Three mechanisms:

1. **Explicit config file** — `--config-file PATH`. Only that file is loaded; transitive discovery is disabled.

   ```bash
   mix credo --config-file ./path/to/credo.exs
   ```

2. **Explicit config name** — `--config-name NAME`. Default is `"default"`.

   ```bash
   mix credo --config-name spring-cleaning
   ```

3. **Transitive config files** (default behavior) — Credo walks **up** the filesystem from the project root and merges every `.credo.exs` (or `config/.credo.exs`) it finds, layered over Credo's built-in defaults. This suits umbrella apps and monorepos: a `.credo.exs` in each child app, a higher-level one in the umbrella root or `$HOME`.

   ```text
   /home/rrrene/
     .credo.exs
     projects/
       foo/
         .credo.exs
   ```

   For `projects/foo/`, Credo merges `projects/foo/.credo.exs` over `/home/rrrene/.credo.exs` over the built-in defaults.

### Toggling checks inline

In addition to the config file, checks can be disabled directly in source. From [config_comments.html](https://hexdocs.pm/credo/config_comments.html), there are four directives:

| Comment | Scope |
|---|---|
| `# credo:disable-for-this-file` | The entire file. |
| `# credo:disable-for-next-line` | The next line. |
| `# credo:disable-for-previous-line` | The previous line. |
| `# credo:disable-for-lines:<count>` | The next `<count>` lines (negative counts target previous lines). |

Each may be followed by a check module name or a regex (e.g. `/\.Warning\./`) to target specific checks:

```elixir
defp my_fun() do
  # credo:disable-for-next-line Credo.Check.Warning.IoInspect
  IO.inspect {:we_want_this_inspect_in_production!}
end

defp other_fun() do
  # credo:disable-for-next-line /\.Warning\./
  IO.inspect {:disable_every_warning_check_here!}
end
```

### Re-enabling disabled checks via CLI

Checks listed under `:disabled` can be re-enabled ad hoc without editing config:

```bash
mix credo --enable-disabled-checks Credo.Check.Consistency.TabsOrSpaces
```

They keep their configured params when re-enabled.

### Minimal complete example

```elixir
# .credo.exs
%{
  configs: [
    %{
      name: "default",
      files: %{
        included: ["lib/", "test/"],
        excluded: ["test/integration/", ~r"/_build/"]
      },
      requires: [],
      plugins: [],
      color: true,
      parse_timeout: 5000,
      strict: false,
      checks: %{
        enabled: [
          {Credo.Check.Readability.MaxLineLength, max_line_length: 120},
          {Credo.Check.Warning.IoInspect,         priority: :low}
        ],
        disabled: [
          {Credo.Check.Readability.VariableNames, []}
        ]
      }
    }
  ]
}
```

## Checks Catalog

From the [checks.html](https://hexdocs.pm/credo/checks.html) entry point:

> Note: as of Credo v1.7.x, `https://hexdocs.pm/credo/checks.html` does not host a rendered catalog (it returns 404). The catalog is split across three live surfaces — use whichever fits the task:

1. **The default-enabled list** — enumerated under "Default checks" in [config_file.html](https://hexdocs.pm/credo/config_file.html).
2. **Per-check module pages** — one ExDoc page per check at `https://hexdocs.pm/credo/Credo.Check.<Category>.<Name>.html` (e.g. [Credo.Check.Readability.ModuleNames](https://hexdocs.pm/credo/Credo.Check.Readability.ModuleNames.html)).
3. **The runtime command** — `mix credo info --verbose` prints every check enabled by the resolved config.

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "To see which checks are enabled with your current config and/or command line switches, run `mix credo info --verbose`."

Append switches to preview their effect:

```bash
mix credo info --verbose --strict
```

### Categories

There are five built-in categories. The CLI tags map to them as `[C]` Consistency, `[D]` Design, `[R]` Readability, `[F]` Refactor, `[W]` Warning. The authoritative descriptions come from the `mix credo categories` command (see [categories_command.ex](https://github.com/rrrene/credo/blob/master/lib/credo/cli/command/categories/categories_command.ex)) and the `Credo.Check` behaviour ([Credo.Check.html](https://hexdocs.pm/credo/Credo.Check.html)):

| Atom | Title | What it covers (verbatim) |
|---|---|---|
| `:consistency` | Consistency | "These checks take a look at your code and ensure a consistent coding style. Using tabs or spaces? Both is fine, just don't mix them or Credo will tell you." |
| `:design` | Software Design | "While refactor checks show you possible problems, these checks try to highlight possibilities, like - potentially intended - duplicated code or TODO and FIXME comments." |
| `:readability` | Code Readability | "Readability checks do not concern themselves with the technical correctness of your code, but how easy it is to digest." |
| `:refactor` | Refactoring opportunities | "The Refactor checks show you opportunities to avoid future problems and technical debt." |
| `:warning` | Warnings - please take a look | "These checks warn you about things that are potentially dangerous, like a missed call to `IEx.pry` you put in during a debugging session or a call to String.downcase without using the result." |

> The atom is `:refactor` (singular), even though output headers read "Refactoring opportunities". Plugins and custom checks can add more categories.

### Notable checks

Each check is a module under `Credo.Check.<Category>.<Name>`. The selection below highlights the most frequently useful defaults; params shown are the defaults documented on each check's page.

**Consistency**

| Check | Summary | Notable param |
|---|---|---|
| `Credo.Check.Consistency.TabsOrSpaces` | Flags mixed tabs/spaces across files (not indentation depth). | `:force` → `:tabs` or `:spaces` (default `nil`). |
| `Credo.Check.Consistency.SpaceInParentheses` | Enforces consistent spacing inside parentheses. | — |
| `Credo.Check.Consistency.LineEndings` | Flags mixed Unix/Windows line endings. | `:force` → `:unix` or `:windows` (default `nil`). |

Both `TabsOrSpaces` and `LineEndings` are tagged `:formatter` (redundant if you run Elixir's formatter).

**Design**

| Check | Summary | Notable param |
|---|---|---|
| `Credo.Check.Design.AliasUsage` | Prefer aliasing non-top-level modules so dependencies are visible at a glance. | `:if_nested_deeper_than`, `:if_called_more_often_than`, `:excluded_namespaces`, `:excluded_lastnames`. |
| `Credo.Check.Design.TagTODO` | Reports `# TODO` comments. | `:include_doc` (default `true`). |
| `Credo.Check.Design.TagFIXME` | Reports `# FIXME` comments. | `:include_doc` (default `true`). |

`TagTODO` and `TagFIXME` have base priority `0`, so they only surface under `--strict`.

**Readability**

| Check | Summary | Notable param |
|---|---|---|
| `Credo.Check.Readability.ModuleNames` | Module names use PascalCase. | `:ignore` (strings/regexes, default `[]`). |
| `Credo.Check.Readability.VariableNames` | Variable names use snake_case. | — |
| `Credo.Check.Readability.ModuleAttributeNames` | Module attribute names use snake_case. | — |
| `Credo.Check.Readability.FunctionNames` | Function/macro/guard names use snake_case. | `:allow_acronyms` (default `false`). |
| `Credo.Check.Readability.PredicateFunctionNames` | Predicates end in `?`; guard-safe macros prefix `is_`. | — |
| `Credo.Check.Readability.ModuleDoc` | Requires a `@moduledoc`. | — |
| `Credo.Check.Readability.MaxLineLength` | Line length limit. | `:max_length` (default `120`); ignores defs/strings/URLs by default. |

**Refactor**

| Check | Summary | Notable param |
|---|---|---|
| `Credo.Check.Refactor.CyclomaticComplexity` | Flags high cyclomatic complexity. | `:max_complexity` (default `9`). |
| `Credo.Check.Refactor.Nesting` | Limits nesting depth inside functions. | `:max_nesting` (default `2`). |
| `Credo.Check.Refactor.FunctionArity` | Flags functions with many parameters. | `:max_arity` (default `8`), `:ignore_defp`. |
| `Credo.Check.Refactor.MapJoin` | Prefer `Enum.map_join/3` over `map |> join`. | — |
| `Credo.Check.Refactor.FilterCount` | Prefer `Enum.count/2` over `filter |> count`. | — |
| `Credo.Check.Refactor.NegatedConditionsWithElse` | An `if` with a negated condition should not carry an `else`. | — |
| `Credo.Check.Refactor.Apply` | Prefer direct calls over `apply/2`/`apply/3` when arity is known. | — |

`Credo.Check.Refactor.PipeChainStart` is **disabled by default** and tagged `:controversial`.

**Warning**

| Check | Summary | Notable param |
|---|---|---|
| `Credo.Check.Warning.IoInspect` | Warns about `IO.inspect` left over from debugging. | — |
| `Credo.Check.Warning.IExPry` | Warns about `IEx.pry` left over from debugging. | — |
| `Credo.Check.Warning.Dbg` | Warns about `dbg/0`/`dbg/2` (Elixir ≥ 1.14). | `:allow_captures` (default `false`). |
| `Credo.Check.Warning.UnsafeExec` | Warns about unsafe `System.cmd` input. | — |
| `Credo.Check.Warning.OperationOnSameValues` | Flags operations where both operands are identical. | — |
| `Credo.Check.Warning.OperationWithConstantResult` | Flags operations whose result is constant. | — |
| `Credo.Check.Warning.RaiseInsideRescue` | Flags `raise` inside `rescue`. | — |

From [IoInspect](https://hexdocs.pm/credo/Credo.Check.Warning.IoInspect.html):

> "While calls to IO.inspect might appear in some parts of production code, most calls to this function are added during debugging sessions. This check warns about those calls, because they might have been committed in error."

### Browsing all checks

```bash
mix credo categories        # list categories
mix credo info              # list enabled checks
mix credo info --verbose    # list enabled checks with detail
```

Target a slice at runtime with partial names. From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html):

> "`mix credo --only todo` will show all `# TODO` comments since `todo` will match Credo.Check.Design.TagTODO. `mix credo --only inspect` will show you all calls to `IO.inspect` since it matches Credo.Check.Warning.IoInspect."

## Strict Mode

### What `--strict` does

From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html):

> "Use the `--all-priorities` switch to include low priority issues in the output (aliased as `--strict`)."

```bash
mix credo --strict        # alias for --all-priorities
```

`--strict` is a pure alias for `-A` / `--all-priorities`. It does not enable extra checks or change which files are scanned; it only widens the **priority filter**.

### How priority is computed

From [basic_usage.html](https://hexdocs.pm/credo/basic_usage.html):

> "Each issue is assigned a priority, based on a base priority set by the config and a dynamic component based on violation severity and location in the source code. These priorities hint at the importance of each issue and are displayed in the command-line interface using arrows: ↑ ↗ → ↘ ↓
>
> By default, only issues with a positive priority are part of the report (↑ ↗ →)."

- **Default run:** only positive-priority issues (`↑ ↗ →`) are reported, and each category is capped to roughly its most important issues.
- **Strict run:** low/negative-priority issues (`↓`) are included as well, so the report grows.

### `--all` vs `--strict` vs `--min-priority`

These are independent axes and are easy to conflate:

| Switch | Axis | Effect |
|---|---|---|
| `-a, --all` | Per-category cap | Removes the "top N per category" cap; still positive-priority only. |
| `-A, --all-priorities` / `--strict` | Priority floor | Includes low-priority issues. |
| `--min-priority LEVEL` | Priority floor | Finer-grained: `higher\|high\|normal\|low\|ignore` or an integer. |

```bash
mix credo --all                       # show every positive-priority issue
mix credo --strict                    # also include low priority
mix credo --min-priority high         # only high-or-above
mix credo --all --strict              # exhaustive: no cap, all priorities
```

Many checks (e.g. `CyclomaticComplexity`, `Nesting`, `FunctionArity`, `MaxLineLength`, `TagTODO`, `TagFIXME`) have a base priority of `0` or `low`, so they are effectively silent unless you run strict.

### Config equivalent

From [config_file.html](https://hexdocs.pm/credo/config_file.html):

> "Set to `true` to enable low priority checks (defaults to `false`). This is equivalent to using the `--strict` CLI switch."

```elixir
%{configs: [%{name: "default", strict: true}]}
```

For a per-run strict policy without editing the default config, add a named config:

```elixir
%{
  configs: [
    %{name: "default", strict: false},
    %{name: "picky", strict: true}
  ]
}
```

```bash
mix credo --config-name picky
```

### When to use strict mode

- **CI gating / exhaustive review:** run `--strict` (or `strict: true` in CI's config) so low-priority findings are not silently dropped.
- **Quick local feedback:** default output is deliberately curated to surface the most important issues first; treat it as a signal, not an exhaustive report.
- **Diff-based PR checks:** combine with `mix credo diff <ref> --strict` to surface new low-priority issues only in changed code.

> Keep local and CI strictness aligned. A common mistake is running `--strict` locally while CI uses the default config (or vice versa), so the two disagree on what is "clean". See [Common mistakes](#common-mistakes).

For CI-specific wiring (exit codes, output formats), see [CI Integration](#ci-integration).

## Disabling and Inline Directives

There are two complementary ways to turn checks off: inline comments in source, and entries in `.credo.exs`. The inline directives are documented in [config_comments.html](https://hexdocs.pm/credo/config_comments.html).

### Inline directives

From [config_comments.html](https://hexdocs.pm/credo/config_comments.html):

> "There are four config comments:
>
> - `# credo:disable-for-this-file` - to disable for the entire file
> - `# credo:disable-for-next-line` - to disable for the next line
> - `# credo:disable-for-previous-line` - to disable for the previous line
> - `# credo:disable-for-lines:<count>` - to disable for the given number of lines (negative for previous lines)"

| Comment | Scope |
|---|---|
| `# credo:disable-for-this-file` | The entire file. |
| `# credo:disable-for-next-line` | The next line. |
| `# credo:disable-for-previous-line` | The previous line. |
| `# credo:disable-for-lines:<count>` | The next `<count>` lines (negative targets previous lines). |

Each directive may be followed by a **check module name** or a **regex** to target specific checks; omit both to disable every check on the targeted lines/file.

Targeting a single check:

```elixir
defp my_fun do
  # credo:disable-for-next-line Credo.Check.Warning.IoInspect
  IO.inspect(:we_want_this_inspect_in_production!)
end
```

Targeting many checks with a regex (matched against the check's module name):

```elixir
defp other_fun do
  # credo:disable-for-next-line /\.Warning\./
  IO.inspect(:disable_every_warning_check_here!)
end
```

`disable-for-lines` scopes a block (it covers the next `<count>` lines):

```elixir
# credo:disable-for-lines:3 Credo.Check.Warning.IoInspect
defp my_fun do
  IO.inspect(:debug)
end
```

`disable-for-this-file` covers a whole module:

```elixir
# credo:disable-for-this-file /\.Warning\./
defmodule MyApp.DebugServer do
  # ...
end
```

> Caveats:
> - `# credo:disable-for-previous-line` must be placed **after** the line it exempts (it is the odd one out).
> - Inline directives only affect the current file; they are not config-level.
> - The regex form is Credo's own `/pattern/` notation (illustrated as `/\.Warning\./` in the docs); it is not Elixir's `~r{}` sigil.

### Disabling in `.credo.exs`

Three mechanisms (covered in [Configuration File](#configuration-file)):

1. **`{Check, false}`** — fully disable a check. From [check_params.html](https://hexdocs.pm/credo/check_params.html):

   > "`params` can be either `false`, to disable the check …"

   ```elixir
   checks: [
     {Credo.Check.Consistency.TabsOrSpaces, false}
   ]
   ```

2. **`:disabled` list** — drop checks from the defaults while preserving their params for later re-enable:

   ```elixir
   checks: %{
     disabled: [{Credo.Check.Consistency.TabsOrSpaces, []}]
   }
   ```

3. **`:enabled` list** — pin an explicit allow-list; everything not listed stops running.

### Re-enabling without editing config

From [cli_switches.html](https://hexdocs.pm/credo/cli_switches.html):

> "Use `--enable-disabled-checks [pattern]` to re-enable checks that were disabled in the config using `{CheckModule, false}`."

```bash
mix credo --enable-disabled-checks Credo.Check.Readability.Specs,Credo.Check.Refactor.DoubleBooleanNegation
# partial names work too:
mix credo --enable-disabled-checks specs,double
```

### Guidance

- Investigate before suppressing: run `mix credo explain <location>` so the disable is intentional, not a reflex.
- Scope narrowly. Prefer a specific check name or regex over a bare directive that mutes everything.
- Prefer config (`:disabled`) for repo-wide policy and inline directives for genuine one-off exceptions. Avoid accumulating inline mutes; treat each as carrying a documented reason.
- Keep disabling auditable: a check disabled inline without a reason violates the [Review checklist](#review-checklist).

## CI Integration

TL;DR: pin `{:credo, "~> 1.7", only: [:dev, :test], runtime: false}`, use `mix credo --strict` or `mix credo --format json`, and interpret exit codes via the bitwise category map. Source: [exit_statuses.html](https://hexdocs.pm/credo/exit_statuses.html).

## Review checklist

- [ ] Credo runs clean on the changed files before requesting review.
- [ ] Exit status is interpreted as a bitwise category map, not as an issue count.
- [ ] `--strict` / `--all-priorities` is used for CI or exhaustive review, not assumed from default output.
- [ ] Findings were investigated with `mix credo explain <location>` before disabling or ignoring a check.
- [ ] The difference between `--all` (remove per-category cap) and `--strict` (include low priority) is respected.
- [ ] Output format matches the consumer (`json`/`sarif` for CI, `oneline` for local grep).
- [ ] `runtime: false` and `only: [:dev, :test]` are set in `mix.exs`.
- [ ] No check is disabled inline without a documented reason.

## Implementation checklist

- [ ] Add `{:credo, "~> 1.7", only: [:dev, :test], runtime: false}` to `mix.exs`.
- [ ] Run `mix deps.get`.
- [ ] Run `mix credo` and triage the default output.
- [ ] Use `mix credo explain <location>` to understand each finding before fixing or suppressing.
- [ ] Generate `.credo.exs` with `mix credo.gen.config` if project-level tuning is needed.
- [ ] Decide CI policy: `--strict`, `--min-priority`, output format, and whether to use `mix credo diff <ref>`.
- [ ] Wire Credo into the validation hooks / CI pipeline.
- [ ] Document repo-specific decisions (see Policy decisions below).

## Validation hooks

- `mix credo` — default analysis; quick local check.
- `mix credo --strict` — exhaustive analysis including low-priority issues.
- `mix credo diff <ref>` — validate only changed code against a git ref.
- `mix credo explain <location>` — inspect a specific finding.

These commands form the validation surface for the sections covered on this page.

## Examples

### Default run

```bash
mix credo
```

Expect a categorized summary capped per category; low-priority issues are hidden.

### List all issues by file, one line each

```bash
mix credo list --format oneline
```

Expect one issue per line; useful for grep or quick counts.

### Explain a finding

```bash
mix credo explain lib/my_app/server.ex:10:24
```

Expect a detailed explanation plus config and disable instructions.

### Strict run

```bash
mix credo --strict
```

Expect more issues than the default, including low-priority findings.

### Run only warning checks

```bash
mix credo --checks warning
```

Expect only `[W]` issues.

### JSON output for CI

```bash
mix credo --strict --format json
```

Expect machine-readable output with `category`, `check`, `priority`, etc.

### Diff against a git ref

```bash
mix credo diff main
```

Expect only issues in code changed since `main`.

### Mute exit status

```bash
mix credo --mute-exit-status
```

Expect analysis output but exit code `0` regardless of findings.

## Common mistakes

- Treating default `mix credo` output as exhaustive; it caps per category and hides non-positive-priority issues.
- Reading the exit code as an issue count rather than a bitwise OR of categories.
- Running `--strict` locally while CI uses the default config, or vice versa.
- Forgetting `runtime: false` in `mix.exs`, which can bloat releases.
- Disabling a check without first running `mix credo explain`.
- Confusing `--all` (show all issues, removing the per-category cap) with `--all-priorities`/`--strict` (include low-priority issues).
- Assuming Credo is a style-only tool; it also flags design, refactoring, and potential bugs.

## Strict vs contextual guidance

### Strict

- `mix credo --strict` equals `mix credo --all-priorities`.
- Default output hides non-positive-priority issues.
- Exit codes `1..127` are a bitwise OR of per-category codes, not counts.
- `runtime: false` and `only: [:dev, :test]` keep Credo out of releases and production.
- Credo is a teaching/consistency tool, not a language-enforced requirement.

### Conventions (contextual)

- Use `--strict` for CI gating and exhaustive review; use default output for quick local checks.
- Prefer `json` or `sarif` for CI ingestion; `oneline` for local grep.
- Use `--min-priority` to tune noise vs signal per repo.
- Use `mix credo diff <ref>` on PRs when the team only wants new/changed-code findings.
- Generate `.credo.exs` only when project-level customization is actually needed.

## Policy decisions for individual repos

- Which `--min-priority` level gates the build.
- Whether CI runs `mix credo --strict` or the default task.
- Whether to fail the build on specific categories (exit-code bitmask) or any issue.
- Which output format CI consumes (`json`, `sarif`, `oneline`).
- Whether PR checks use `mix credo diff main` or a full scan.
- Version pin for Credo (`~> 1.7` at time of writing).
- Whether custom checks are generated with `mix credo.gen.check`.
- Whether to enable disabled checks via `--enable-disabled-checks` for special runs.

## Related docs

- `docs/elixir/naming-conventions.md` — already references `mix credo` as a validation hook.
- Related Elixir corpus docs: `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/mix-project-structure.md`, `docs/elixir/testing-exunit.md`.

## Related skills

- None defined yet.
