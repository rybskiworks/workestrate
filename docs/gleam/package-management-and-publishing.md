# Package management and publishing

## Purpose

This document defines Gleam guidance for dependency management, version-constraint
syntax, the lockfile, publishing to Hex, package metadata, and Software Bill of
Materials (SBoM) generation. Future agents who add/update/remove dependencies,
publish packages, or produce SBoMs should follow these rules.

## Sources used

- `.crawl/06-cli.md` — https://gleam.run/command-line-reference/ (PRIMARY — `add`/`remove`/`update`/`deps`/`publish`/`docs`/`hex` commands)
- `.crawl/09-sbom.md` — https://gleam.run/documentation/source-bill-of-materials/ (PRIMARY — ORT-based SBoM generation, CycloneDX/SPDX)
- `.crawl/03-writing-gleam.md` — https://gleam.run/writing-gleam/ (manifest.toml lockfile, version-constraint format, path/git deps)
- `.crawl/05-gleam-toml.md` — https://gleam.run/writing-gleam/gleam-toml/ (dependency value forms, dev_dependencies, licences/description/repository for Hex)

## Related BEAM guidance

- `docs/beam/releases.md` — release packaging parallels. `gleam export hex-tarball`
  and `gleam publish` produce a versioned artifact analogous to a BEAM release
  tarball; consult for release/packaging concepts.

## Core guidance

### Dependency commands

| Command | Purpose |
|---|---|
| `gleam add [OPTIONS] <PACKAGES>...` | Add new project dependencies. `--dev` adds as dev-only. |
| `gleam remove <PACKAGES>...` | Remove project dependencies. |
| `gleam update` | Update dependency packages to their latest versions within constraints. |
| `gleam deps download` | Download all dependency packages. |
| `gleam deps list` | List all dependency packages. |
| `gleam deps update` | Update dependency packages (subcommand form). |

`gleam update` (top-level) and `gleam deps update` are equivalent.

### Version-constraint syntax (Hex requirement format)

Dependency version requirements use the **Hex requirement format**
(hexdocs.pm/elixir/Version.html#module-requirements), e.g.:
```toml
[dependencies]
gleam_stdlib = ">= 0.44.0 and < 2.0.0"
envoy = ">= 1.0.1 and < 2.0.0"
argv = ">= 1.0.2 and < 2.0.0"
```
The name must match the dependency package's own configured name. Erlang rebar3
and Elixir mix packages can be added with the same syntax.

Dependency value forms:
```toml
# Version requirement (Hex format)
gleam_stdlib = ">= 1.0.0 and < 2.0.0"
# Path dep
my_other_project = { path = "../my_other_project" }
# Git dep — prefer a commit SHA over branch/tag
my_library = { git = "https://github.com/my-project/my-library", ref = "a8b3c5d82" }
```

### dev_dependencies

`--dev` places a dependency in `[dev_dependencies]` — used for building, developing,
and testing only; NOT included in final production builds or when publishing to Hex.
A package CANNOT appear in both `dependencies` and `dev_dependencies`.

### Lockfile (`manifest.toml`)

`manifest.toml` locks all dependency packages to specific versions. Recommendations:
- Check `manifest.toml` into VCS so anyone who downloads the project gets the same
  dependency versions.
- `manifest.toml` is NOT uploaded to Hex and is NOT used when other projects depend
  on your project.
- `gleam update` refreshes versions within the declared constraints.

### Publishing to Hex

| Command | Purpose |
|---|---|
| `gleam publish [OPTIONS]` | Publish the project to Hex. Flags: `--replace`, `-y`/`--yes`. |
| `gleam docs publish` | Publish HTML docs to HexDocs. |
| `gleam docs remove --package <P> --version <V>` | Remove HTML docs from HexDocs. |
| `gleam hex retire <P> <V> <REASON> [MSG]` | Retire a release from Hex. |
| `gleam hex unretire <P> <V>` | Un-retire a release. |
| `gleam export hex-tarball` | Bundle the package into a tarball suitable for publishing to Hex. |

All Hex-authenticated commands (`publish`, `docs publish`, `docs remove`,
`hex retire`, `hex unretire`) use the `HEXPM_USER` and `HEXPM_PASS` environment
variables (both optional).

### Package metadata (required for Hex publish)

These `gleam.toml` keys are optional in general but REQUIRED when publishing to Hex:
- `licences` — list of licences in SPDX format, e.g. `["Apache-2.0"]`.
- `description` — short project description.
- `[repository]` — source repo metadata (type/user/repo, plus `path`/`tag_prefix`
  for monorepos).

### SBoM generation (ORT; CycloneDX/SPDX)

Gleam has NO built-in `gleam export` SBOM command. SBoM generation is delegated to
the OSS Review Toolkit (ORT), which consumes `manifest.toml`. Run three stages as
Docker containers from the project root:

1. **Analyzer** — resolves direct + transitive deps:
```sh
docker run --rm -v "$(pwd)":/workspace -w /workspace \
  ghcr.io/oss-review-toolkit/ort-minimal:74.0.0 \
  analyze --input-dir /workspace --output-dir /workspace/ort-result
```
   Produces `ort-result/analyzer-result.yml`. Requires `manifest.toml` committed
   and reflecting current state.

2. **Scanner** — collects licence + copyright metadata:
```sh
docker run --rm -v "$(pwd)":/workspace -w /workspace \
  ghcr.io/oss-review-toolkit/ort-minimal:74.0.0 \
  scan --input-file /workspace/ort-result/analyzer-result.yml \
       --output-dir /workspace/ort-result
```
   Produces `ort-result/scan-result.yml`.

3. **Reporter** — emits SBoMs:
```sh
docker run --rm -v "$(pwd)":/workspace -w /workspace \
  ghcr.io/oss-review-toolkit/ort-minimal:74.0.0 \
  report --input-file /workspace/ort-result/scan-result.yml \
         --output-dir /workspace/ort-result \
         --report-formats CycloneDx,SpdxDocument,WebApp \
         --option CycloneDX=output.file.formats=json,xml \
         --option SpdxDocument=outputFileFormats=JSON,YAML
```
   Produces CycloneDX (JSON + XML), SPDX (JSON + YAML), and a WebApp (HTML viewer).

An SBoM typically includes: direct and transitive dependencies; package names and
versions; source locations; cryptographic hashes; licence and copyright info. An
SBoM does NOT claim software is secure, compliant, or vulnerability-free — it is a
factual foundation for other tools.

## Practical rules

1. Version requirements use the Hex requirement format (e.g. `>= 1.0.0 and < 2.0.0`).
2. Check `manifest.toml` into VCS; it is not uploaded to Hex.
3. `--dev` deps go in `[dev_dependencies]`; excluded from production builds and Hex.
4. No package in both `dependencies` and `dev_dependencies`.
5. Git deps: prefer a commit SHA for `ref` over branch/tag (reproducibility + supply-chain).
6. Publishing to Hex requires `licences`, `description`, and `[repository]`.
7. Hex-authenticated commands use `HEXPM_USER`/`HEXPM_PASS` env vars.
8. `gleam update` refreshes within declared constraints (does not relax them).
9. SBoM generation requires `manifest.toml` committed and current.
10. Use the `ort-minimal` image (sufficient for Gleam projects).
11. Generate SBoMs automatically in CI and keep them up to date.

## Review checklist

- [ ] Version constraints use Hex requirement format (not bare semver).
- [ ] `manifest.toml` committed and reflects current deps.
- [ ] No dep in both dep tables.
- [ ] Git deps use commit SHA, not branch/tag.
- [ ] `licences` (SPDX), `description`, `[repository]` present if publishing.
- [ ] Dev-only deps in `[dev_dependencies]`.
- [ ] No vendored dependencies in a publishable package.
- [ ] SBoM (if produced) generated from a current `manifest.toml`.

## Implementation checklist

- [ ] Add deps with `gleam add <pkg>` (`--dev` for dev-only).
- [ ] Remove deps with `gleam remove <pkg>`.
- [ ] Refresh with `gleam update`; commit updated `manifest.toml`.
- [ ] Verify with `gleam deps list`.
- [ ] Set `licences`/`description`/`[repository]` before publishing.
- [ ] Publish with `gleam publish`; publish docs with `gleam docs publish`.
- [ ] Wire ORT SBoM generation in CI (analyzer → scanner → reporter).

## Validation hooks

- `gleam deps list` — verify resolved dependency set.
- `gleam build` — fails if version constraints are unsatisfiable.
- `gleam publish` — validates Hex metadata (`licences`/`description`/`[repository]`).
- `gleam docs build` — validates documentation pages before `docs publish`.
- ORT analyzer — validates `manifest.toml` resolves cleanly.

## Examples

Add and lock dependencies:
```sh
gleam add envoy argv
gleam add --dev gleeunit gleescript
gleam update
gleam deps list
```

Publish to Hex:
```sh
export HEXPM_USER=...
export HEXPM_PASS=...
gleam publish -y
gleam docs publish
```

Retire a release:
```sh
gleam hex retire my_package 1.2.0 security "See CVE ..."
```

## Common mistakes

- Using bare semver (`"1.0.0"`) instead of the Hex requirement format
  (`">= 1.0.0 and < 2.0.0"`).
- Treating `manifest.toml` as uploaded to Hex (it is not).
- Putting a dep in both `dependencies` and `dev_dependencies`.
- Using a branch/tag for a git dep `ref`.
- Publishing without `licences`/`description`/`[repository]`.
- Expecting a native `gleam export sbom` command (none exists; use ORT).
- Running ORT against a stale or uncommitted `manifest.toml`.

## Strict vs contextual guidance

Strict: Hex requirement format for version constraints; `manifest.toml` committed;
no dep in both tables; git deps use commit SHA; Hex publish requires
`licences`/`description`/`[repository]`; SBoM from current `manifest.toml`.

Contextual: constraint bounds (e.g. `< 2.0.0` vs `< 1.1.0`); whether to produce
SBoMs; ORT report formats chosen; CI cadence for SBoM regeneration.

## Policy decisions for individual repos

- Adopt version-constraint conventions (e.g. always `>= x and < x+1.0.0`).
- Decide whether `manifest.toml` is committed (recommended yes).
- Require `licences`/`description`/`[repository]` for all publishable packages.
- Choose SBoM formats (CycloneDX and/or SPDX) and CI cadence.
- Pin ORT image version in CI.

## Related docs

- `gleam-toml-and-targets.md` — dependency value forms, `[repository]`, `licences`.
- `project-structure-and-cli.md` — `gleam add`/`new`/`deps` CLI, `manifest.toml`.
- `validation.md` — CI gates including `gleam deps list` and SBoM checks.
- `deployment-and-runtime.md` — `gleam export hex-tarball`/`erlang-shipment`.

## Related skills

- `gleam-packages-ffi`
- `gleam-language`
