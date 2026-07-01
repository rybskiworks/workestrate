---
name: gleam-packages-ffi
description: |
  Operational guide for Gleam project structure, CLI, `gleam.toml`/targets,
  dependency management and Hex publishing, gleeunit testing, validation gates,
  HTTP/JSON/Dynamic API boundaries, and deployment/runtime. Load when
  scaffolding, building, testing, publishing, validating, or deploying Gleam
  projects, or when handling JSON/`Dynamic`/decode at API boundaries. Does NOT
  cover pure language fundamentals/stdlib (see `gleam-language`), nor OTP
  actors/supervisors or `gleam_erlang`/`gleam_otp` process interop (see
  `gleam-otp-interop`).
---

# Gleam projects, packages, FFI boundaries, and deployment

## Triggers

Load this skill when:

- Scaffolding, building, running, formatting, or editing Gleam projects
  (`gleam new`/`build`/`run`/`test`/`format`/`check`/`add`/`deps`/`export`).
- Authoring or reviewing `gleam.toml` (metadata, deps, targets, `[erlang]`,
  `[javascript]`, `[javascript.deno]`, `internal_modules`, `[repository]`).
- Adding/removing/updating dependencies, managing `manifest.toml`, or
  publishing to Hex / retiring releases / generating SBoMs.
- Writing gleeunit tests or wiring CI validation gates.
- Building HTTP services (`gleam_http` sans-IO `Service`) or handling JSON/
  `Dynamic`/decode at API boundaries (`gleam_json` v3.1.0).
- Deploying Gleam (`gleam export erlang-shipment`, container images, Linux
  server / Fly.io).

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/gleam/project-structure-and-cli.md`
  - https://gleam.run/writing-gleam/
  - https://gleam.run/command-line-reference/
  - https://gleam.run/language-server/
  - https://gleam.run/install
- `docs/gleam/gleam-toml-and-targets.md`
  - https://gleam.run/writing-gleam/gleam-toml/
- `docs/gleam/package-management-and-publishing.md`
  - https://gleam.run/command-line-reference/
  - https://gleam.run/documentation/source-bill-of-materials/
  - https://gleam.run/writing-gleam/
  - https://gleam.run/writing-gleam/gleam-toml/
- `docs/gleam/testing.md`
  - https://gleeunit.hexdocs.pm/
  - https://gleam.run/command-line-reference/
- `docs/gleam/validation.md`
  - https://gleam.run/command-line-reference/
  - https://gleeunit.hexdocs.pm/
- `docs/gleam/http-and-services.md`
  - https://gleam-http.hexdocs.pm/
  - https://gleam-json.hexdocs.pm/gleam/json.html
- `docs/gleam/json-dynamic-and-api-boundaries.md`
  - https://gleam-json.hexdocs.pm/gleam/json.html
  - https://hexdocs.pm/gleam_stdlib/gleam/dynamic.html
  - https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
- `docs/gleam/deployment-and-runtime.md`
  - https://gleam.run/deployment/linux-server/
  - https://gleam.run/deployment/fly/
  - https://gleam.run/frequently-asked-questions/
- `docs/beam/applications.md` — OTP application structure
  (`erlang.application_start_module`, `extra_applications`).
- `docs/beam/releases.md` — release packaging parallels
  (`gleam export erlang-shipment`/`hex-tarball` vs traditional OTP releases).
- `docs/beam/validation.md` — shared runtime validation concepts (Gleam uses
  compiler type-checking, NOT Dialyzer).
- `docs/beam/binaries.md` — `BitArray`/binary model behind `json.parse_bits`.
- `docs/beam/common-mistakes.md` — deserialising untrusted data safely.

## Key Rules

- Every Gleam package requires a `gleam.toml` (TOML 1.1) with `name` and
  `version`. `gleam_stdlib` is included by default; `gleeunit` is the default
  test runner. The `gleam` binary IS the build tool (no separate build tool).
- Project layout: `src/` (one module per file; module name = file path),
  `test/` (test code), `gleam.toml`, `manifest.toml` (lockfile). Entrypoint is
  `main` in the package-named module; run another with `gleam run -m <module>`.
- `manifest.toml` is a lockfile: check into VCS; NOT uploaded to Hex; NOT used
  by downstream dependents. `gleam update` refreshes within declared
  constraints (does not relax them).
- Internal modules: `packagename/internal` and `packagename/internal/*` are
  not public API (configurable via `internal_modules` globs, default
  `["$PACKAGE_NAME/internal", "$PACKAGE_NAME/internal/*"]`).
- Dependency version requirements use the HEX REQUIREMENT FORMAT (e.g.
  `">= 1.0.0 and < 2.0.0"`), NOT bare semver. Path deps: `{ path = "../p" }`.
  Git deps: `{ git = "...", ref = "<commit-sha>" }` — prefer a commit SHA over
  branch/tag (reproducibility + supply-chain).
- `--dev` places a dep in `[dev_dependencies]` (excluded from production builds
  and Hex). A package CANNOT appear in both `dependencies` and
  `dev_dependencies`.
- Publishing to Hex REQUIRES `licences` (SPDX list, e.g. `["Apache-2.0"]`),
  `description`, and `[repository]`. Hex-authenticated commands (`publish`,
  `docs publish`, `docs remove`, `hex retire`, `hex unretire`) authenticate
  via `HEXPM_USER`/`HEXPM_PASS` env vars. `gleam publish` flags: `--replace`,
  `-y`/`--yes`.
- There is NO `gleam export sbom` command. SBoM generation is delegated to the
  OSS Review Toolkit (ORT), which consumes a committed, current
  `manifest.toml` (analyzer → scanner → reporter; CycloneDX/SPDX output).
- `gleam new` flags: `--name`, `--skip-git`, `--skip-github`, `--template
  erlang|javascript`. There is NO `--target` flag and NO `lib`/`bare` templates.
- `--target <TARGET>` is valid for `build`/`check`/`run`/`test`/`dev`.
  `--warnings-as-errors` is `build`-only. `gleam check` type-checks without
  codegen (faster than `build`). `gleam format --check` is the CI formatting
  gate (no rewrite). `gleam deps` has only `download`/`list`/`update` (NO
  `deps tree`). `gleam update` (top-level) == `gleam deps update`.
- `gleam export` subcommands: `erlang-shipment`, `escript`, `hex-tarball`,
  `javascript-prelude`, `typescript-prelude`, `package-interface --out
  <OUTPUT>` (required). `gleam docs` subcommands: `build [--open]`,
  `publish`, `remove --package <P> --version <V>`.
- `gleam.toml` `target` accepts only `"erlang"` (default) or `"javascript"`.
  There is NO plural `targets` key and NO `entrypoint` key. `[erlang]`:
  `application_start_module` (atom format `my_project@application`, must
  implement the OTP application behaviour), `extra_applications`. `[javascript]`:
  `source_maps` (default `false`), `typescript_declarations` (default
  `false`), `runtime` (`node`/`deno`/`bun`, default `node`). `[javascript.deno]`:
  least-privilege allow-lists; `allow_all = true` overrides all others.
- gleeunit (v1.11.0) is a DEV-only dependency (`gleam add gleeunit@1 --dev`).
  A function is collected as a test IFF it is `pub`, its name ends in `_test`,
  and it lives under `test/`. The test entrypoint MUST call `gleeunit.main()`.
  Use `gleeunit/should` (`should.equal`/`should.be_ok`/`should.be_error`) for
  assertions. On Erlang gleeunit runs on EUnit; on JS a custom runner. On
  Deno/JS, grant `[javascript.deno] allow_read` for `gleam.toml`/`test`/`build`.
- Validation: Gleam is statically typed with full type inference — the
  compiler IS the type checker (`gleam check`/`gleam build`). There is NO
  Dialyzer step. Validate on BOTH targets (`-t erlang` and `-t javascript`) —
  runtime data structures differ (e.g. decoders). CI gate set:
  `gleam format --check && gleam check && gleam test` (plus
  `--warnings-as-errors` per policy). Use `gleam export package-interface
  --out <file>` to audit the public API surface.
- `gleam_http` (v4.3.0) ships NO I/O — it defines `Request`/`Response`/
  `Headers`/`Method`/`Status` and the sans-IO `Service(conn, body)` type
  (`fn(Request(body)) -> Response(body)`). Adapters supply concrete
  `conn`/`body` and transport. Server adapters: Mist, cgi, gleam_cowboy,
  gleam_elli, gleam_plug. Client adapters: gleam_fetch (JS), gleam_hackney,
  gleam_httpc (Erlang). wisp is a framework on top of Mist, NOT a gleam_http
  adapter enumerated on the index.
- Write handlers as PURE `Service` functions; keep I/O/socket logic in
  adapters. A service is reusable across server and client roles — do not
  bake transport into handler logic.
- `gleam_json` v3.1.0: `Json` is OPAQUE — construct only via builders
  (`object`, `array(from:, of:)`, `preprocessed_array`, `string`, `int`,
  `float`, `bool`, `null`, `nullable(from:, of:)`, `dict`). Serialise with
  `to_string_tree` (PREFERRED for IO — BEAM VM optimised for `StringTree`) or
  `to_string`. Decode with `parse(from:, using:)` / `parse_bits(from:, using:)`
  → `Result(t, DecodeError)`. ALWAYS handle `Error` (untrusted input).
- `gleam_json` `DecodeError` variants: `UnexpectedEndOfInput`,
  `UnexpectedByte(String)`, `UnexpectedSequence(String)`,
  `UnableToDecode(List(decode.DecodeError))`. There is NO decoder-less `parse`
  returning raw `Dynamic` — a `decode.Decoder(t)` is REQUIRED.
- DO NOT call absent `gleam_json` names: `to_string_builder` (use
  `to_string_tree`), `to_dynamic`, `object_take`, `null_as`, `decode` (as a
  public fn).
- `gleam/dynamic` v1.0.3 is the CONSTRUCTION side (constructors + `classify`
  for diagnostics only). Decode via `gleam/dynamic/decode` v1.0.3. `Decoder(t)`
  is opaque; primitives `int`/`float`/`string`/`bool`/`bit_array`/`dynamic`.
  `int` does NOT coerce `1.0`; `float` does NOT coerce ints — use `one_of`.
  Records use the `use`-callback style (`field`/`subfield`/`optional_field`/
  `then` + `success`); `run` collects ALL errors. Absent: `decode1..N`,
  `sequence`. The LSP has a "generate dynamic decoder" code action.
- Keep `Dynamic` at boundaries; decode immediately into typed data. Never
  convert well-typed data back to `Dynamic`. Test decoders on BOTH targets.
- Deployment: both documented paths (Linux server, Fly.io) target the Erlang
  runtime. `gleam export erlang-shipment` produces `build/erlang-shipment/`
  with `entrypoint.sh` — this is Gleam's own shipment format, NOT a
  traditional OTP release (no `.rel`/`relup`/boot scripts/`sys.config`).
- Bind the app to `0.0.0.0` (not `127.0.0.1`); the guides use port `8000`.
  Pin `GLEAM_VERSION` and `ERLANG_VERSION` in the Dockerfile. Run the final
  container as a non-root `webapp` user. Config is injected as ENV VARS, not
  `sys.config`. Redeployment is a full container replacement (no in-place
  hot upgrades). HTTPS terminates outside the BEAM (Caddy / Fly proxy).

## Quick Commands

```bash
gleam new my_app                            # scaffold (Erlang target default)
gleam new --template javascript my_app      # scaffold JS-target project
gleam add gleam_stdlib@1 envoy argv         # add deps (Hex requirement format)
gleam add --dev gleeunit gleescript         # dev-only deps
gleam deps list                              # verify resolved dep set
gleam update                                 # refresh within constraints
gleam format --check && gleam check && gleam test   # CI gate set
gleam build --warnings-as-errors            # promote warnings to errors
gleam check -t erlang && gleam check -t javascript  # both targets
gleam test -t javascript                    # run suite on JS target
gleam export package-interface --out interface.json  # audit public API
gleam export erlang-shipment                # build/erlang-shipment/ for deploy
gleam publish -y                            # publish to Hex (HEXPM_USER/PASS)
gleam docs publish                          # publish HTML docs to HexDocs
gleam hex retire <pkg> <ver> security "..." # retire a release
```

## Anti-patterns

- Using bare semver (`"1.0.0"`) instead of the Hex requirement format
  (`">= 1.0.0 and < 2.0.0"`).
- Treating `manifest.toml` as uploaded to Hex (it is not) or used by
  downstream dependents.
- Putting a dep in both `dependencies` and `dev_dependencies`.
- Using a branch/tag for a git dep `ref` instead of a commit SHA.
- Publishing without `licences`/`description`/`[repository]`.
- Expecting a native `gleam export sbom` command (none — use ORT) or running
  ORT against a stale/uncommitted `manifest.toml`.
- Expecting `gleam new --target` or `lib`/`bare` templates (only
  `--template erlang|javascript`); expecting `deps tree` (use `deps list`).
- Inventing a plural `targets` key or an `entrypoint` key (neither exists);
  plain module names in `internal_modules` (must be globs);
  `erlang.application_start_module` in dotted form instead of atom format.
- Running `gleam test` without `gleeunit.main()` (no tests discovered); naming
  tests without `_test` or making them non-`pub` (not collected); placing tests
  outside `test/`; treating gleeunit as a non-dev dependency.
- Expecting a Dialyzer step (Gleam's compiler type-checks); only validating one
  target; using `gleam format` (rewrites) instead of `gleam format --check` in
  CI; omitting `--out` on `export package-interface`.
- Putting I/O/socket logic inside a `Service` function (breaks sans-IO purity);
  assuming wisp is a gleam_http adapter; using `to_string` instead of
  `to_string_tree` for IO-bound JSON bodies; not handling `parse` `Error`.
- Calling `to_string_builder`/`to_dynamic`/`object_take`/`null_as`/`decode` in
  `gleam_json` (absent in v3.1.0); using `decode1..N`/`sequence` in
  `gleam/dynamic/decode` (absent — use `field`/`then`/`list`).
- Letting `Dynamic` flow into typed internals; using `classify` for typed
  conversion; not testing decoders on both targets.
- Binding to `127.0.0.1` instead of `0.0.0.0`; treating `erlang-shipment` as a
  traditional OTP release; expecting hot-code upgrades across redeploys;
  running the container as root; not pinning toolchain versions; using a
  Node/Bun runtime for the documented Fly.io path (Erlang-only).

## Related Skills

- `gleam-language`
- `gleam-otp-interop`
- `beam-applications-releases`
- `beam-validation`
- `beam-binaries`
