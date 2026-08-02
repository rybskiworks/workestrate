# Crawl: writing-gleam/gleam-toml/
- seed_url: https://gleam.run/writing-gleam/gleam-toml/
- canonical_url: https://gleam.run/writing-gleam/gleam-toml/
- family: Gleam official guide
- fetch: 200
- gleam_version: >= 1.15.0 (example; key is project-specific)
- feeds_docs: gleam-toml-and-targets.md, project-structure-and-cli.md

## Purpose
Authoritative reference for the `gleam.toml` configuration file required by **every** Gleam package. Defines project metadata, dependencies, documentation, repository info, compilation target/options, and external-tool config. Written in TOML 1.1 format (see toml.io). The default scaffolded config for a new package is:

```toml
name = "my_package"
version = "1.0.0"

[dependencies]
gleam_stdlib = ">= 0.44.0 and < 2.0.0"

[dev_dependencies]
gleeunit = ">= 1.0.0 and < 2.0.0"
```

## Config keys (full list + meaning)

### Project metadata
| key | required | meaning |
|-----|----------|---------|
| `name` | **required** | Project name. |
| `version` | **required** | Project version (semver string). |
| `licences` | optional (required for Hex publish) | List of licences in SPDX format, e.g. `["Apache-2.0"]`. |
| `description` | optional (required for Hex publish) | Short project description. |

### Dependency management
| key | meaning |
|-----|---------|
| `[dependencies]` | Packages the build tool must download/compile for this package to use. Version requirement uses the Hex requirement format (docs: hexdocs.pm/elixir/Version.html#module-requirements). Erlang rebar3 and Elixir mix packages can be added with the same syntax. |
| `[dev_dependencies]` | Development-only deps, same syntax as `dependencies`. Not included when publishing to Hex. A package **cannot** appear in both `dependencies` and `dev_dependencies`. |

Dependency value forms:
- Version requirement: `gleam_stdlib = ">= 1.0.0 and < 2.0.0"`
- Path dep: `my_other_project = { path = "../my_other_project" }`
- Git dep: `my_library = { git = "https://github.com/my-project/my-library", ref = "a8b3c5d82" }` — **prefer a commit SHA over branch/tag** to prevent supply-chain attacks and ensure reproducibility.

### Documentation
| key | meaning |
|-----|---------|
| `[repository]` | Source repo; used in generated docs and on Hex. Optional unless publishing to Hex. |
| `links` | List of `{ title, href }` related-website links shown in docs and on Hex. |
| `[[documentation.pages]]` | Extra Markdown pages for generated HTML docs. Each entry: `title`, `path` (output html filename), `source` (path to `.md`). The `[documentation]` table and its `pages` key are both optional. |

### Compilation
| key | meaning |
|-----|---------|
| `gleam` | Required Gleam compiler version, e.g. `">= 1.15.0"`. Compiler raises an error if its version does not satisfy this. |
| `target` | Default compilation target. Accepted: `"erlang"`, `"javascript"`. Default `"erlang"`. Overridable per-command via `--target` CLI flag. |
| `internal_modules` | List of glob patterns (docs.rs/glob) marking modules as internal (not public API; no stability/behaviour guarantees). Default: `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]`. |

### Erlang compilation (`[erlang]`)
| key | meaning |
|-----|---------|
| `erlang.application_start_module` | OTP application module name (atom format, e.g. `my_project@application` for Gleam module `my_project/application`). Optional; libraries generally omit. Module must implement the OTP application behaviour (erlang.org/doc/man/application.html). |
| `erlang.extra_applications` | OTP applications to start in addition to those from project deps, e.g. `["inets", "ssl"]`. |

### JavaScript compilation (`[javascript]`)
| key | meaning |
|-----|---------|
| `javascript.source_maps` | Generate source map files. Optional, default `false`. |
| `javascript.typescript_declarations` | Generate TypeScript `.d.ts` files. Optional, default `false`. |
| `javascript.runtime` | JS runtime for `gleam run`/`gleam test`/related. Accepted: `"node"`, `"deno"`, `"bun"`. Default `"node"`. |
| `javascript.deno.allow_*` | Deno permissions config (see below). |

### External tools
| key | meaning |
|-----|---------|
| `[tools.<tool_name>]` | Free-form config table for external tools. Authors encouraged to namespace under `tools.<tool_name>`. Example fields shown: `enable`, `threads`. |

## target / targets config (Erlang + JavaScript; Deno permissions)

**`target`** (singular, top-level) sets the default compilation target:
```toml
target = "erlang"   # or "javascript"; default "erlang"
```
Overridable per CLI invocation with `--target`. There is **no** plural `targets` key in the current reference — a single default target is declared and the CLI flag selects per-run.

**Per-target options** live in their own tables:

Erlang (`[erlang]`):
```toml
[erlang]
application_start_module = "my_application"   # atom format: my_project@application
extra_applications = ["inets", "ssl"]
```

JavaScript (`[javascript]`):
```toml
[javascript]
source_maps = true
typescript_declarations = true
runtime = "node"   # "node" | "deno" | "bun"
```

**Deno permissions** under `[javascript.deno]` — Deno uses an explicit IO permission system; any combination of these fields may be provided or omitted:
```toml
[javascript.deno]
allow_hrtime = true
allow_net = ["example.com:443"]
allow_run = ["./bin/migrate.sh"]
allow_read = ["./database.sqlite"]
allow_write = ["./database.sqlite"]
```
| field | type / meaning |
|-------|---------------|
| `allow_all` | bool — grant all Deno permissions (equivalent to setting all others to `true`). |
| `allow_env` | bool **or** list of env-var names — environment variable access. |
| `allow_ffi` | bool — foreign function interface access. |
| `allow_hrtime` | bool — high-resolution time APIs. |
| `allow_net` | bool **or** list of IP addresses / hostnames (optionally with ports). |
| `allow_read` | bool **or** list of paths — filesystem read access. |
| `allow_run` | bool **or** list of paths — subprocess execution access. |
| `allow_sys` | bool — system information access. |
| `allow_write` | bool **or** list of paths — filesystem write access. |

## dependencies vs dev-dependencies
- `[dependencies]` — packages needed at build/runtime; downloaded and compiled by the build tool. Version requirement uses the **Hex requirement format**. Erlang rebar3 and Elixir mix packages use the same syntax. Supports `path =` (local) and `{ git =, ref = }` (git) inline tables.
- `[dev_dependencies]` — development-only deps, identical syntax. **Excluded** when publishing to Hex. A package **cannot** appear in both tables simultaneously.
- Git deps: always prefer a **commit SHA** for `ref` over a branch/tag to ensure reproducibility and resist malicious-actor attacks.

## [documentation] pages
```toml
[[documentation.pages]]
title = "My Page"
path = "my-page.html"
source = "./path/to/my-page.md"
```
Additional Markdown pages included in generated HTML documentation. The `[documentation]` table is optional, and `pages` within it is also optional. Each entry requires `title`, `path` (output HTML filename), and `source` (path to the source `.md` file).

## internal_modules
```toml
internal_modules = ["my_app/internal", "my_app/internal/*"]
```
A list of **glob patterns** (Rust `glob` crate syntax — docs.rs/glob/latest/glob/struct.Pattern.html). Any module whose name matches a pattern is marked internal: technically importable but **not** part of the public API — no stability or behavioural guarantees. Useful for helper modules used by public modules.

Default when omitted: `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]` — e.g. for a package `wibble`, modules `wibble/internal` and `wibble/internal/wobble` are internal by default.

## repository metadata (monorepo: type/path/tag_prefix)
```toml
[repository]
type = "github"
user = "torvalds"
repo = "linux"
```
Used in generated documentation and displayed on Hex. Optional unless publishing to Hex.

Repository `type` values and required fields:
| type | required fields |
|------|-----------------|
| `bitbucket`, `codeberg`, `github`, `gitlab`, `sourcehut`, `tangled` | `user`, `repo` |
| `forgejo`, `gitea` | `host`, `user`, `repo` |
| `custom` | `url` (generated HTML docs link to repo but **not** to source definitions of public types/values) |

Examples:
```toml
# forgejo / gitea
[repository]
type = "gitea"
host = "https://example.com"
user = "my-username"
repo = "my-repo"

# custom
[repository]
type = "custom"
url = "https://example.com/my-project"
```

**Monorepo support** — all types **other than `custom`** support `path` and `tag_prefix`:
- `path` — path from repo root to the directory containing the package.
- `tag_prefix` — prefix added to git tags for package releases, to avoid collisions between multiple packages in one repo.

```toml
[repository]
type = "github"
user = "my-user"
repo = "package_one"
path = "packages/one"
tag_prefix = "one-"
```

## entrypoint
There is **no** `entrypoint` key in the `gleam.toml` reference. The closest concept is `erlang.application_start_module` (the OTP application start module, atom format `my_project@application`), which is optional and only relevant for applications (not libraries). For JavaScript, the runtime entrypoint is governed by `javascript.runtime` and the generated module's exports, not a config key.

## Strict rules
1. **Every** Gleam package requires a `gleam.toml`.
2. File format is **TOML 1.1**.
3. `name` and `version` are **required**.
4. `licences` and `description` are optional **unless** publishing to Hex.
5. `[repository]` is optional **unless** publishing to Hex.
6. A package **cannot** appear in both `dependencies` and `dev_dependencies`.
7. `dev_dependencies` are **not** included when publishing to Hex.
8. `gleam` version requirement raises a hard error if the compiler version does not satisfy it.
9. `target` accepts only `"erlang"` or `"javascript"`; default `"erlang"`.
10. `internal_modules` default is `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]`.
11. `erlang.application_start_module` must implement the OTP application behaviour; module name is in atom format (`@` separator).
12. Git deps: **prefer commit SHA** over branch/tag refs (reproducibility + supply-chain safety).
13. `javascript.source_maps` and `javascript.typescript_declarations` default to `false`.
14. `javascript.runtime` default is `"node"`; accepts `"node"`, `"deno"`, `"bun"`.
15. Deno permissions are opt-in per field; `allow_all` overrides all others.

## Verbatim quotes
- "All Gleam packages require a gleam.toml configuration file. They are written in the toml 1.1 format, documentation for which can be found at toml.io."
- "The version requirement string uses the Hex requirement format, and the name must match the name the dependency package has in their own package configuration file."
- "Erlang rebar3 packages and Elixir mix packages can also be added as dependencies without any change in syntax."
- "These dependencies are not included if the package is published to Hex. A package cannot appear in both dependencies and dev_dependencies."
- "Always prefer a commit sha reference over a branch or tag reference to ensure you get the expected version of the code, and to prevent attacks from malicious actors."
- "An error is raised if the compiler version used to compile the package does not satisfy this requirement, informing the programmer that they will not be able to successfully compile the project."
- "This value can be overridden by the --target flag for the CLI commands that accept it."
- "An internal module is a module that is not part of the public API of the package. It is technically possible to import and use internal modules, but as they are not public there is no stability or behavioural guarantees for them."
- "If not given then the default value for this field is ["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]"
- "The module is specified in the Erlang atom format, so the Gleam module my_project/application would be written as my_project@application."
- "All types other than custom support the fields path and tag_prefix."
- "path contains the path to the directory that contains the package from the root of the repository. tag_prefix contains a prefix that is to be added to git tags for package releases, to avoid collisions between the multiple packages."
- "The Deno runtime uses a permissions system for IO, with the program having to be explicitly given permission to perform these actions."
- "allow_all : Whether to grant all Deno permissions. This is the equivalent of setting all the other fields to true."
- "Here you can place data or configuration for external tools. We encourage authors of such tools to place configuration under tools.<tool_name>."

## Version notes
- `gleam` compiler-version constraint example uses `">= 1.15.0"`, implying the constraint key/feature is present in Gleam >= 1.15.0 era docs.
- Default scaffold pins `gleam_stdlib = ">= 0.44.0 and < 2.0.0"` and `gleeunit = ">= 1.0.0 and < 2.0.0"`.
- TOML 1.1 is the configured format version.
- No plural `targets` key exists in this reference (single `target` default + `--target` CLI override).
- No `ericbb` or other monorepo-specific metadata keys beyond `repository.path` / `repository.tag_prefix`.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/elixir/Version.html#module-requirements — Hex requirement format (dependency version strings).
- https://docs.rs/glob/latest/glob/struct.Pattern.html — glob pattern syntax for `internal_modules`.
- https://www.erlang.org/doc/man/application.html — OTP application behaviour for `erlang.application_start_module`.
- https://toml.io/ — TOML 1.1 format reference.

### Skipped
- In-page anchor links (`#Compilation`, `#Project-metadata`, `#dependencies`, `#description`, `#dev_dependencies`, `#documentation.pages`, `#erlang.application_start_module`, `#erlang.extra_applications`, `#gleam`, `#internal_modules`, `#javascript.deno.allow_*`, `#javascript.runtime`, `#javascript.source_maps`, `#javascript.typescript_declarations`, `#licences`, `#links`, `#name`, `#repository`, `#target`, `#tools`, `#version`, `#Dependency-management`, `#Documentation`, `#Erlang-compilation`, `#External-tools`, `#JavaScript-compilation`) — same-page navigation.
