# Crawl: hexdocs.pm/gleeunit/ (index)
- seed_url: https://hexdocs.pm/gleeunit/
- canonical_url: https://gleeunit.hexdocs.pm/
- family: Gleam package (gleeunit)
- fetch: 200
- gleam_stdlib/gleeunit_version: gleeunit v1.11.0 (hexdocs built with ExDoc v1.17.0)
- feeds_docs: testing.md, validation.md

## Purpose
gleeunit is the standard Gleam test runner. Per the index README it is
"A simple test runner for Gleam, using EUnit on Erlang and a custom runner on JS."

It is the runner invoked by `gleam test`. A Gleam project's generated
`test/<app>_test.gleam` file imports `gleeunit` and calls `gleeunit.main()`,
which discovers and runs all test functions.

Install as a dev dependency:
```
gleam add gleeunit@1 --dev
```

## gleeunit module + should helpers
Two modules are exposed (listed in the index "Modules" sidebar):
- `gleeunit` — entry point; call `gleeunit.main()` from the test main function.
- `gleeunit/should` — assertion helpers (e.g. `should.equal`, `should.be_ok`,
  `should.be_error`) used inside `_test` functions.

The canonical test entrypoint pattern from the README:
```gleam
// In test/yourapp_test.gleam
import gleeunit

pub fn main() {
  gleeunit.main()
}
```

## Test naming/discovery conventions (test/ dir, _test suffix)
Discovery rule (verbatim from README): "any public function with a name ending
in `_test` in the `test` directory will be found and run as a test."

Conventions:
- Test files live under `test/` (the conventional generated entrypoint is
  `test/<yourapp>_test.gleam`).
- A function is treated as a test iff it is `pub` and its name ends in `_test`.
- Example:
```gleam
pub fn some_function_test() {
  assert some_function() == "Hello!"
}
```
- Non-`pub` functions and functions without the `_test` suffix are not
  collected.

## gleam test invocation
Run tests from the project root with:
```
gleam test
```
`gleam test` builds the test target and executes the test entrypoint, which
calls `gleeunit.main()`; gleeunit then scans `test/` for `pub fn ..._test`
functions and runs them. On Erlang it delegates to EUnit; on JavaScript it uses
a custom runner.

## Target-specific notes (Erlang/JavaScript)
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

## Strict rules
- A function is collected as a test ONLY if: it is `pub`, its name ends in
  `_test`, and it is defined in a module under the `test/` directory.
- The test entrypoint must call `gleeunit.main()`; without it, `gleam test`
  will not discover/run tests.
- gleeunit is a dev-only dependency (`gleam add gleeunit@1 --dev`).
- Do not place test functions outside `test/`; they will not be discovered.
- On Deno/JS, missing `allow_read` entries cause runtime failures.

## Verbatim quotes
- "A simple test runner for Gleam, using EUnit on Erlang and a custom runner on JS."
- "gleam add gleeunit@1 --dev"
- "// In test/yourapp_test.gleam"
- "import gleeunit"
- "pub fn main() { gleeunit.main() }"
- "Now any public function with a name ending in `_test` in the `test` directory will be found and run as a test."
- "Run the tests by entering `gleam test` in the command line."
- "If using the Deno JavaScript runtime, you will need to add the following to your `gleam.toml`."

## Version notes
- gleeunit v1.11.0 (current per hexdocs index at fetch time).
- Hexdocs pages built with ExDoc v1.17.0.
- Install line pins major version: `gleam add gleeunit@1 --dev`.

## Discovered links
### Relevant (crawl later)
- https://gleeunit.hexdocs.pm/gleeunit.html — `gleeunit` module API (main/1, etc.)
- https://gleeunit.hexdocs.pm/gleeunit/should.html — `gleeunit/should` assertion helpers API
- https://hex.pm/packages/gleeunit — Hex package page (version/Changelog)
- https://github.com/lpil/gleeunit — source repository (README, CHANGELOG, test examples)

### Skipped
- ./css/index.css, ./css/atom-one-light.min.css — ExDoc stylesheets
- #icon-menu, #icon-x-circle, #icon-chevrons-down, #icon-svg-search — SVG icon anchors
- https://github.com/sponsors/lpil — sponsorship link (non-doc)
- ./ (self)
- ./index.html (self)
