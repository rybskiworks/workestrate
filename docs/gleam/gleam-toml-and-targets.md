# gleam.toml and targets

## Purpose

This document is the authoritative reference for the `gleam.toml` configuration
file required by every Gleam package, and for compilation target configuration
(Erlang vs JavaScript). Future agents who author, review, or validate `gleam.toml`
should follow these rules so config matches the official reference exactly.

## Sources used

- `.crawl/05-gleam-toml.md` — https://gleam.run/writing-gleam/gleam-toml/ (PRIMARY — every key, target config, Deno permissions, repository metadata, internal_modules)

## Related BEAM guidance

- `docs/beam/applications.md` — `erlang.application_start_module` and
  `extra_applications` map onto OTP application concepts; consult when wiring an
  OTP application callback module or startup dependencies.
- `docs/beam/releases.md` — release packaging parallels; `gleam export
  erlang-shipment` produces a deployable unit analogous to a release.

## Core guidance

Every Gleam package requires a `gleam.toml`, written in **TOML 1.1**. The default
scaffold:

```toml
name = "my_package"
version = "1.0.0"

[dependencies]
gleam_stdlib = ">= 0.44.0 and < 2.0.0"

[dev_dependencies]
gleeunit = ">= 1.0.0 and < 2.0.0"
```

### Project metadata

| key | required | meaning |
|-----|----------|---------|
| `name` | **required** | Project name. |
| `version` | **required** | Project version (semver string). |
| `licences` | optional (required for Hex publish) | List of licences in SPDX format, e.g. `["Apache-2.0"]`. |
| `description` | optional (required for Hex publish) | Short project description. |

### Dependencies

| key | meaning |
|-----|---------|
| `[dependencies]` | Packages the build tool must download/compile. Version requirement uses the **Hex requirement format** (hexdocs.pm/elixir/Version.html#module-requirements). Erlang rebar3 and Elixir mix packages use the same syntax. |
| `[dev_dependencies]` | Development-only deps, same syntax. NOT included when publishing to Hex. A package CANNOT appear in both `dependencies` and `dev_dependencies`. |

Dependency value forms:
```toml
# Version requirement (Hex format)
gleam_stdlib = ">= 1.0.0 and < 2.0.0"
# Path dep
my_other_project = { path = "../my_other_project" }
# Git dep — prefer a commit SHA over branch/tag
my_library = { git = "https://github.com/my-project/my-library", ref = "a8b3c5d82" }
```

### Compilation

| key | meaning |
|-----|---------|
| `gleam` | Required Gleam compiler version, e.g. `">= 1.15.0"`. Compiler raises a hard error if its version does not satisfy this. |
| `target` | Default compilation target. Accepted: `"erlang"`, `"javascript"`. Default `"erlang"`. Overridable per-command via `--target`. |
| `internal_modules` | List of **glob patterns** (Rust `glob` crate syntax) marking modules as internal (not public API). Default: `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]`. |

There is **no** plural `targets` key and **no** `entrypoint` key. The closest to
an entrypoint is `erlang.application_start_module`.

### Erlang compilation (`[erlang]`)

```toml
[erlang]
application_start_module = "my_application"   # atom format: my_project@application
extra_applications = ["inlets", "ssl"]
```
- `application_start_module` — OTP application module name (atom format, `@`
  separator). Optional; libraries generally omit. Module must implement the OTP
  application behaviour.
- `extra_applications` — OTP applications to start in addition to those from deps.

### JavaScript compilation (`[javascript]`)

```toml
[javascript]
source_maps = true              # default false
typescript_declarations = true   # default false
runtime = "node"                # "node" | "deno" | "bun"; default "node"
```

### Deno permissions (`[javascript.deno]`)

Deno uses an explicit IO permission system; any combination of fields may be set
or omitted:
```toml
[javascript.deno]
allow_all = true                 # equivalent to setting all others to true
allow_env = true                 # bool OR list of env-var names
allow_ffi = true                  # bool — FFI access
allow_hrtime = true               # bool — high-resolution time
allow_net = ["example.com:443"]   # bool OR list of hostnames/IPs (optionally :port)
allow_read = ["./database.sqlite"] # bool OR list of paths
allow_run = ["./bin/migrate.sh"]   # bool OR list of paths
allow_sys = true                  # bool — system info
allow_write = ["./database.sqlite"] # bool OR list of paths
```
`allow_all` overrides all others (equivalent to setting every field to `true`).

### Documentation (`[documentation]` / `[[documentation.pages]]`)

```toml
[[documentation.pages]]
title = "My Page"
path = "my-page.html"      # output HTML filename
source = "./path/to/my-page.md"
```
The `[documentation]` table is optional, and `pages` within it is also optional.
Each entry requires `title`, `path`, and `source`.

### Repository metadata (`[repository]`)

Used in generated docs and on Hex. Optional unless publishing to Hex.
```toml
[repository]
type = "github"
user = "torvalds"
repo = "linux"
```

Repository `type` values and required fields:

| type | required fields |
|------|-----------------|
| `bitbucket`, `codeberg`, `github`, `gitlab`, `sourcehut`, `tangled` | `user`, `repo` |
| `forgejo`, `gitea` | `host`, `user`, `repo` |
| `custom` | `url` (links to repo but NOT to source definitions of public types/values) |

**Monorepo support** — all types other than `custom` support `path` and `tag_prefix`:
```toml
[repository]
type = "github"
user = "my-user"
repo = "package_one"
path = "packages/one"      # path from repo root to the package directory
tag_prefix = "one-"        # prefix added to git tags to avoid collisions
```

### External tools (`[tools.<tool_name>]`)

Free-form config table for external tools; authors are encouraged to namespace
under `tools.<tool_name>`. Example fields: `enable`, `threads`.

## Practical rules

1. Every Gleam package requires a `gleam.toml`; format is TOML 1.1.
2. `name` and `version` are required.
3. `licences`, `description`, `[repository]` are optional unless publishing to Hex.
4. A package CANNOT appear in both `dependencies` and `dev_dependencies`.
5. `dev_dependencies` are NOT included when publishing to Hex.
6. `gleam` version constraint raises a hard error if unsatisfied.
7. `target` accepts only `"erlang"` or `"javascript"`; default `"erlang"`.
8. `internal_modules` uses glob patterns; default
   `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]`.
9. `erlang.application_start_module` must implement the OTP application behaviour;
   module name uses atom format (`@` separator).
10. Git deps: prefer a commit SHA over branch/tag (reproducibility + supply-chain).
11. `javascript.source_maps` and `javascript.typescript_declarations` default `false`.
12. `javascript.runtime` default is `"node"`; accepts `"node"`, `"deno"`, `"bun"`.
13. Deno permissions are opt-in per field; `allow_all` overrides all others.

## Review checklist

- [ ] `name` and `version` present.
- [ ] No dep in both `dependencies` and `dev_dependencies`.
- [ ] Git deps use a commit SHA for `ref`, not a branch/tag.
- [ ] `licences` uses SPDX format (e.g. `["Apache-2.0"]`) if publishing.
- [ ] `internal_modules` patterns are globs, not plain module names.
- [ ] `erlang.application_start_module` (if set) is atom-format and implements the
      application behaviour.
- [ ] `target` is `"erlang"` or `"javascript"` only.
- [ ] Deno `allow_*` fields use bool-or-list correctly.

## Implementation checklist

- [ ] Start from the scaffolded `gleam.toml`.
- [ ] Add `gleam = ">= x.y.z"` to pin the compiler version.
- [ ] Set `target` if the project is JS-first.
- [ ] Add `[repository]` with `path`/`tag_prefix` for monorepo packages.
- [ ] Enable `typescript_declarations`/`source_maps` for JS-target libraries.
- [ ] Configure `[javascript.deno]` permissions minimally (least privilege).

## Validation hooks

- `gleam build` — fails if `gleam` version constraint is unsatisfied.
- `gleam build -t javascript` / `-t erlang` — validates per-target config.
- `gleam check` — type-checks using the `gleam.toml` target.
- `gleam docs build` — validates `[documentation.pages]` entries.
- `gleam publish` (dry intent) — requires `licences`, `description`, `[repository]`.

## Examples

Multi-target library with JS declarations and Deno permissions:
```toml
name = "my_lib"
version = "1.2.0"
gleam = ">= 1.15.0"
target = "erlang"
licences = ["Apache-2.0"]
description = "A multi-target Gleam library."

[repository]
type = "github"
user = "my-org"
repo = "monorepo"
path = "libs/my_lib"
tag_prefix = "my_lib-"

[dependencies]
gleam_stdlib = ">= 0.44.0 and < 2.0.0"

[dev_dependencies]
gleeunit = ">= 1.0.0 and < 2.0.0"

[javascript]
typescript_declarations = true
source_maps = true
runtime = "deno"

[javascript.deno]
allow_net = ["api.example.com:443"]
allow_read = ["./config.json"]
```

## Common mistakes

- Inventing a plural `targets` key or an `entrypoint` key (neither exists).
- Using plain module names in `internal_modules` instead of glob patterns.
- Putting the same package in both dep tables.
- Using a branch/tag for a git dep `ref` instead of a commit SHA.
- Forgetting `licences`/`description`/`[repository]` when publishing to Hex.
- Writing `erlang.application_start_module` in dotted Gleam form instead of atom
  format (`my_project@application`).
- Setting `allow_all = true` when least-privilege was intended.

## Strict vs contextual guidance

Strict: TOML 1.1; `name`/`version` required; no duplicate dep across tables; git
deps use commit SHA; `target` values limited to `erlang`/`javascript`;
`internal_modules` is glob-based; `application_start_module` is atom-format and
implements the application behaviour.

Contextual: whether to pin `gleam` version; choice of default `target`; whether to
emit TS declarations/source maps; Deno permission granularity; monorepo
`path`/`tag_prefix` values.

## Policy decisions for individual repos

- Pin `gleam` compiler version constraint.
- Choose default `target`.
- Decide whether to emit `typescript_declarations`/`source_maps` for JS target.
- Choose `javascript.runtime` (`node`/`deno`/`bun`).
- Define Deno permission allow-lists (least privilege).
- Adopt monorepo `path`/`tag_prefix` conventions.

## Related docs

- `project-structure-and-cli.md` — scaffold, CLI, manifest.toml.
- `javascript-target.md` — JS target behavior and `gleam_javascript`.
- `package-management-and-publishing.md` — dependency value forms, Hex publish.
- `erlang-interop.md` — `erlang.application_start_module` and OTP interop.

## Related skills

- `gleam-packages-ffi`
- `gleam-language`
