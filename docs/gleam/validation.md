# Validation

## Purpose

Gleam validation gates: the compile/type-check, format, test, and export commands that catch regressions before merge or release. Gleam is statically typed — the compiler type-checks (no Dialyzer needed). This doc maps the `gleam` CLI validation subcommands, the LSP diagnostics, per-target validation, and the package-interface export into a single checklist.

## Sources used

- Crawl 06-cli.md — Gleam command-line reference — https://gleam.run/command-line-reference/
- Crawl 33-gleeunit.md — gleeunit index — https://gleeunit.hexdocs.pm/

## Related BEAM guidance

`docs/beam/validation.md` covers shared runtime validation concepts (compile-time gates, runtime sanity checks). BUT Gleam uses static types (compiler type-checking via `gleam check`), NOT Dialyzer. Note the divergence: BEAM/Erlang relies on Dialyzer/XRef for static analysis; Gleam's compiler IS the type checker. There is no separate Dialyzer pass.

Cross-ref: `docs/beam/validation.md`.

## Core guidance

### `gleam check` (from crawl 06)

- `gleam check [OPTIONS]` — Type check the project.
- Flag: `-t, --target <TARGET>` — The platform to target.
- This is the type-checking gate. Gleam is statically typed; the compiler performs full type inference and checking. There is NO Dialyzer step — Gleam's type system is the static analysis.

### `gleam build` (from crawl 06)

- `gleam build [OPTIONS]` — Build the project.
- Flags: `-t, --target <TARGET>`, `--warnings-as-errors` (Emit compile time warnings as errors).
- `--warnings-as-errors` promotes compile-time warnings to errors (CI gate).

### `gleam format --check` (from crawl 06)

- `gleam format [OPTIONS] [FILES]...` — Format source code.
- Flags: `--check` (Check if inputs are formatted without changing them), `--stdin` (Read source from STDIN).
- `format --check` is the CI formatting gate — verifies without rewriting.

### `gleam test` (from crawl 06 + 33)

- `gleam test [OPTIONS] [ARGUMENTS]...` — Run the project tests.
- Flags: `--runtime <RUNTIME>`, `-t, --target <TARGET>`.
- Runs gleeunit (v1.11.0): discovers `pub fn ..._test` functions under `test/` via `gleeunit.main()`. On Erlang uses EUnit; on JS a custom runner.
- Test discovery rule (verbatim from crawl 33): "any public function with a name ending in `_test` in the `test` directory will be found and run as a test."

### LSP diagnostics (from crawl 06)

- `gleam lsp` — Run the language server, to be used by editors.
- Provides editor-inline diagnostics (type errors, warnings). From crawl 24, the LSP also has a "generate dynamic decoder" code action (generates a decoder from a custom type definition).
- LSP diagnostics surface the same type-checking that `gleam check` performs, inline in the editor.

### Per-target validation (from crawl 06)

- `--target <TARGET>` is valid for `build`, `check`, `run`, `test`, `dev`.
- Validate on BOTH targets (Erlang and JavaScript) — runtime data structures differ (e.g. decoders behave differently; test decoders on all supported platforms).
- `gleam new --template` accepts only `erlang` (default) or `javascript`.

### Package-interface export (from crawl 06)

- `gleam export package-interface --out <OUTPUT>` — "Information on the modules, functions, and types in the project in JSON format."
- `--out <OUTPUT>` — The path to write the JSON file to (required).
- Useful for auditing the public API surface / generating docs.

### Static typing — no Dialyzer (key point)

- Gleam is statically typed with full type inference; the compiler enforces types at compile time (`gleam check` / `gleam build`).
- Gleam does NOT use Dialyzer (Erlang's success-typing static analysis). The type system IS the static analysis — no separate Dialyzer pass. This is a deliberate divergence from Erlang/Elixir workflows.

## Practical rules

- Run `gleam check` (or `gleam build --warnings-as-errors`) as the type-check gate in CI.
- Run `gleam format --check` as the formatting gate in CI.
- Run `gleam test` (with `gleeunit.main()` entrypoint) as the test gate.
- Validate on BOTH targets: `gleam check -t erlang` and `gleam check -t javascript` (and `gleam test -t ...`).
- Use `gleam export package-interface --out <file>` to audit the public API surface.
- Use the LSP (`gleam lsp`) for editor-inline diagnostics.
- Do NOT expect a Dialyzer step — Gleam's compiler type-checks.

## Review checklist

- [ ] `gleam check` passes (type-check gate).
- [ ] `gleam format --check` passes (no formatting drift).
- [ ] `gleam test` passes via `gleeunit.main()` entrypoint.
- [ ] Validated on BOTH targets (`-t erlang` and `-t javascript`).
- [ ] `--warnings-as-errors` applied in CI where policy requires.
- [ ] No Dialyzer step expected or wired in.
- [ ] `export package-interface --out <file>` used to audit public API on surface changes.
- [ ] LSP diagnostics clean in editor.

## Implementation checklist

- [ ] CI runs `gleam format --check && gleam check && gleam test`.
- [ ] CI runs per-target gates (`-t erlang`, `-t javascript`).
- [ ] `--warnings-as-errors` set on `gleam build` in CI per repo policy.
- [ ] `test/<app>_test.gleam` calls `gleeunit.main()`.
- [ ] Test functions are `pub fn ..._test` under `test/`.
- [ ] `gleam export package-interface --out <OUTPUT>` wired for API audit.
- [ ] Editor configured to use `gleam lsp`.

## Validation hooks

- Type-check: `gleam check` / `gleam build` (crawl 06).
- Warnings-as-errors: `gleam build --warnings-as-errors` (crawl 06).
- Format: `gleam format --check` (crawl 06).
- Tests: `gleam test` via gleeunit (crawl 06, 33).
- Per-target: `-t, --target <TARGET>` on check/build/test (crawl 06).
- API surface audit: `gleam export package-interface --out <OUTPUT>` (crawl 06).
- Editor diagnostics: `gleam lsp` (crawl 06).

## Examples

Shell gates:

```sh
gleam format --check && gleam check && gleam test
gleam check -t erlang && gleam check -t javascript
gleam build --warnings-as-errors
gleam export package-interface --out interface.json
```

A test function (discovered by gleeunit):

```gleam
// In test/<app>_test.gleam
import gleeunit

pub fn main() {
  gleeunit.main()
}

pub fn add_test() {
  assert 1 + 1 == 2
}
```

## Common mistakes

- Expecting Dialyzer (Gleam has none — compiler type-checks).
- Running `gleam test` without `gleeunit.main()` in the entrypoint (no tests discovered).
- Only validating one target (Erlang and JS differ — test both).
- Using `gleam format` (rewrites) instead of `gleam format --check` in CI.
- Forgetting `--warnings-as-errors` in CI (warnings slip through).
- Omitting `--out` on `export package-interface` (required).
- Placing test functions outside `test/` or not marking them `pub` (not collected by gleeunit).

## Strict vs contextual guidance

Strict:

- `gleam check`/`build` type-check (no Dialyzer).
- `format --check` in CI.
- `gleeunit.main()` required for test discovery.
- `--out` required for `export package-interface`.

Contextual:

- `--warnings-as-errors` (repo CI policy).
- Per-target validation scope (both vs one).
- LSP usage (editor-dependent).

## Policy decisions for individual repos

- Whether `--warnings-as-errors` is mandatory in CI.
- Whether both targets are gated, or only the primary target.
- Whether `export package-interface` runs on every release or only on public-API changes.
- Whether LSP diagnostics are enforced via pre-commit hooks or left to the editor.

## Related docs

- `docs/gleam/testing.md`
- `docs/gleam/project-structure-and-cli.md`
- `docs/gleam/gleam-toml-and-targets.md`
- `docs/gleam/javascript-target.md`
- `docs/gleam/erlang-interop.md`
- `docs/gleam/package-management-and-publishing.md`
- `docs/beam/validation.md`

## Related skills

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop`
