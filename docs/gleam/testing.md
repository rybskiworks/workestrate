# Testing

## Purpose

gleeunit (v1.11.0) is the standard Gleam test runner — per its README, "A simple
test runner for Gleam, using EUnit on Erlang and a custom runner on JS." It is
the runner invoked by `gleam test`. A project's generated `test/<app>_test.gleam`
imports `gleeunit` and calls `gleeunit.main()`, which discovers and runs all
test functions. Install as a dev dependency: `gleam add gleeunit@1 --dev`.

## Sources used

- Crawl 33-gleeunit.md — gleeunit index — https://gleeunit.hexdocs.pm/
- Crawl 06-cli.md — Gleam command-line reference — https://gleam.run/command-line-reference/

## Related BEAM guidance

On the Erlang target, gleeunit delegates to EUnit (Erlang's standard test
framework). This parallels runtime-validation concepts in
`docs/beam/validation.md`, but Gleam's testing is gleeunit-driven, not raw
EUnit: Gleam tests are `pub fn ..._test` functions discovered by gleeunit, not
EUnit test macros written directly. Cross-ref: `docs/beam/validation.md`.

## Core guidance

### Modules (from crawl 33)

- `gleeunit` — entry point; call `gleeunit.main()` from the test main function.
- `gleeunit/should` — assertion helpers (`should.equal`, `should.be_ok`,
  `should.be_error`) used inside `_test` functions.

### Test entrypoint pattern (verbatim from crawl 33 README)

```gleam
// In test/yourapp_test.gleam
import gleeunit

pub fn main() {
  gleeunit.main()
}
```

### Test naming/discovery (verbatim rule from crawl 33)

> "any public function with a name ending in `_test` in the `test` directory
> will be found and run as a test."

- Test files live under `test/` (conventional generated entrypoint is
  `test/<yourapp>_test.gleam`).
- A function is treated as a test IFF it is `pub` AND its name ends in `_test`.
- Example:

```gleam
pub fn some_function_test() {
  assert some_function() == "Hello!"
}
```

- Non-`pub` functions and functions without the `_test` suffix are NOT collected.

### gleam test invocation (from crawl 33 + 06)

- Run from the project root: `gleam test`.
- `gleam test` builds the test target and executes the test entrypoint, which
  calls `gleeunit.main()`; gleeunit scans `test/` for `pub fn ..._test`
  functions and runs them.
- From crawl 06, signature: `gleam test [OPTIONS] [ARGUMENTS]...`, with flags
  `--runtime <RUNTIME>` and `-t, --target <TARGET>` (select platform).

### Target-specific notes (from crawl 33)

- Erlang target: gleeunit runs on EUnit (Erlang's standard test framework).
- JavaScript target: gleeunit uses a custom JS runner (not EUnit).
- Deno: when using the Deno JavaScript runtime, file read permissions must be
  granted in `gleam.toml`:
```toml
[javascript.deno]
allow_read = [
  "gleam.toml",
  "test",
  "build",
]
```

## Practical rules

- A function is collected as a test ONLY if: `pub`, name ends in `_test`,
  defined in a module under `test/`.
- The test entrypoint MUST call `gleeunit.main()`; without it, `gleam test`
  will not discover/run tests.
- gleeunit is a dev-only dependency (`gleam add gleeunit@1 --dev`).
- Do not place test functions outside `test/` — they will not be discovered.
- On Deno/JS, missing `allow_read` entries cause runtime failures.
- Use `gleeunit/should` helpers for assertions.
- Use `gleam test -t <target>` to run tests against a specific target.

## Review checklist

- [ ] Test entrypoint calls `gleeunit.main()`.
- [ ] Every test function is `pub` and ends in `_test`.
- [ ] Tests live under `test/`.
- [ ] gleeunit is declared as a dev dependency.
- [ ] Assertions use `gleeunit/should` (or `assert`) inside `_test` functions.
- [ ] On Deno/JS, `gleam.toml` grants `allow_read` for `gleam.toml`, `test`, `build`.
- [ ] Target selected explicitly via `gleam test -t <target>` when it matters.

## Implementation checklist

- [ ] `gleam add gleeunit@1 --dev`.
- [ ] Keep `test/<yourapp>_test.gleam` with `gleeunit.main()` in `main`.
- [ ] Add `pub fn ..._test` functions under `test/`.
- [ ] Use `gleeunit/should` for assertions.
- [ ] If targeting Deno/JS, add the `[javascript.deno] allow_read` block.
- [ ] Run `gleam test` (or `gleam test -t <target>`) from the project root.
## Validation hooks

- `gleam test` — runs the full suite via gleeunit (Erlang target by default).
- `gleam test -t javascript` — run the suite on the JavaScript target.
- `gleam test --runtime <RUNTIME>` — select a JS runtime (e.g. Deno).
- `gleam build` / `gleam check` — compile/typecheck before running tests.

## Examples
```gleam
import gleeunit/should

pub fn add_test() {
  should.equal(1 + 1, 2)
}

pub fn ok_result_test() {
  should.be_ok(Ok(1))
}

pub fn error_result_test() {
  should.be_error(Error("x"))
}
```

## Common mistakes

- Forgetting `gleeunit.main()` in the test entrypoint (tests won't run).
- Naming a test without the `_test` suffix (not collected).
- Making a test function non-`pub` (not collected).
- Placing tests outside `test/` (not discovered).
- On Deno/JS, omitting `allow_read` in `gleam.toml` (runtime failures).
- Treating gleeunit as a non-dev dependency (it's dev-only).

## Strict vs contextual guidance

Strict: `pub` + `_test` suffix + `test/` directory for discovery;
`gleeunit.main()` required; gleeunit is a dev-only dependency.

Contextual: target selection (Erlang vs JavaScript via `gleam test -t <target>`);
Deno `allow_read` config (only required for the Deno JavaScript runtime).

## Policy decisions for individual repos

- Which target(s) must pass in CI: Erlang only, JavaScript only, or both.
- Whether to enforce `gleeunit/should` helpers over bare `assert` expressions.
- Whether Deno is a supported JS runtime (and thus whether `allow_read` is required).

## Related docs

- `docs/gleam/testing.md` (this doc)
- `docs/gleam/validation.md`
- `docs/gleam/project-structure-and-cli.md`
- `docs/gleam/gleam-toml-and-targets.md`
- `docs/gleam/javascript-target.md`
- `docs/gleam/erlang-interop.md`
- `docs/beam/validation.md`

## Related skills

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop`
