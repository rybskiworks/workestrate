# JavaScript target

## Purpose

This document defines Gleam guidance for the JavaScript compilation target: JS
runtime config, the `gleam_javascript` package, TypeScript declarations, source
maps, JS externals (FFI), and target-specific review risks. Future agents who
build, review, or debug Gleam code compiled to JavaScript should follow these rules.

## Sources used

- `.crawl/36-gleam-javascript-index.md` — https://hexdocs.pm/gleam_javascript/ (PRIMARY — gleam_javascript package: array/promise/symbol modules)
- `.crawl/08-externals.md` — https://gleam.run/documentation/externals/ (PRIMARY — `@external(javascript, ...)`, JS module paths, prelude API v1.13+, review risks)
- `.crawl/05-gleam-toml.md` — https://gleam.run/writing-gleam/gleam-toml/ (PRIMARY — `[javascript]` config, Deno permissions)

## Related BEAM guidance

Gleam/JS-specific; no BEAM runtime on the JavaScript target. The Erlang-target
guidance in `docs/beam/*` (processes, supervisors, applications) does not apply
when compiling to JavaScript. On the JS target, concurrency is `gleam/javascript/promise`,
not BEAM processes/OTP.

## Core guidance

### Selecting the JS target

Set `target = "javascript"` in `gleam.toml` (default target) or pass
`--target javascript` per command (`build`, `check`, `run`, `test`, `dev`).
Scaffold a JS-target project with `gleam new --template javascript`.

### JS runtime config (`[javascript]`)

```toml
[javascript]
source_maps = true              # default false
typescript_declarations = true   # default false
runtime = "node"                # "node" | "deno" | "bun"; default "node"
```
- `runtime` selects the runtime for `gleam run`/`gleam test`/related commands.
- `source_maps` and `typescript_declarations` default to `false`.

### Deno permissions (`[javascript.deno]`)

Deno uses an explicit IO permission system. Configure least-privilege allow-lists:
```toml
[javascript.deno]
allow_net = ["api.example.com:443"]
allow_read = ["./config.json"]
allow_env = ["DATABASE_URL"]
```
Each `allow_*` field is bool OR list; `allow_all = true` grants everything (avoid in
production). See `gleam-toml-and-targets.md` for the full field list.

### The `gleam_javascript` package (v1.0.0)

`gleam_javascript` is the official core package providing typed wrappers over
JavaScript values for code compiled to the JS target. It is the typed-FFI layer
used alongside `external` declarations, exposing JS-native primitives as typed
Gleam modules so JS values can be handled safely without dropping to untyped
`Dynamic`.

Modules (v1.0.0 surface):
- `gleam/javascript/array` — typed wrapper around JS `Array` (mutable, index-based).
  Distinct from Gleam's immutable `List`; the bridge for FFI boundaries that
  produce/consume JS arrays.
- `gleam/javascript/promise` — typed wrapper around JS `Promise`; the JS-target
  async/concurrency primitive (where the Erlang target would use processes/OTP).
- `gleam/javascript/symbol` — typed wrapper around JS `Symbol` (unique keys /
  well-known symbols interop).

This is the "typed FFI" half of the JS-target story: `external` declarations
declare the untyped boundary; `gleam/javascript/*` provides the typed Gleam types
(`Promise`, JS `Array`, `Symbol`) that callers use on the Gleam side.

### JavaScript externals (`@external(javascript, ...)`)

External functions are bodiless functions with the `@external` attribute taking 3
arguments: target (`javascript`), the module the function is exported from, and
the function name. Type annotations are MANDATORY.

```gleam
@external(javascript, "./my_app/pokemon.mjs", "badge_count")
pub fn pokemon_badge_count() -> Int
```
- The module path is used with a JS `import` statement; typically a relative path
  to a `.mjs` file, relative to the Gleam file containing the external.
- Node modules are referenced by the same path used in JS code:
  `@external(javascript, "has-flag", "hasFlag")`.
- Gleam has NO special support for `npm`/JS package managers — configure the JS
  runtime to ensure packages are present (`package.json` + `npm install`).

### Multi-target externals and Gleam fallbacks

`@external` may be specified multiple times for both targets:
```gleam
@external(erlang, "lists", "reverse")
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```
A function may have BOTH a Gleam body and an `@external`; the external is used when
compiling to its target, otherwise the Gleam body is the fallback. A function with
an `@external` for only one target is usable only on that target — using it on the
other is a compile error.

### Gleam data in JavaScript (prelude API, v1.13+)

Since v1.13, Gleam compiled to JS exposes an API for constructing Gleam data types.
The prelude is importable as if at `src/gleam.mjs`.

| Gleam | JavaScript |
|---|---|
| `True`/`False` | `true`/`false` |
| `Int` | number (must be whole) |
| `Float` | number (avoid infinity/NaN — no Gleam syntax) |
| `String` | string |
| `Nil` | `undefined` |
| `BitArray` | `BitArray$BitArray(new Uint8Array([...]))` from prelude |
| `List` | `List$Empty()` / `List$NonEmpty(head, rest)` from prelude |
| `#(a, b)` | array `[a, b]` (immutable — never mutate) |
| `Ok(x)`/`Error(x)` | `Result$Ok(x)` / `Result$Error(x)` from prelude |
| Custom variants | generated `TypeName$VariantName(...)` functions |

Importing Gleam modules from JS: each Gleam module compiles to a `.mjs` ES module;
relative paths ascend past the package root and back down into the target package
directory.

### TypeScript declarations

Enable `typescript_declarations = true` in `[javascript]` to emit `.d.ts` files
alongside generated JS. (The externals page does not document `.d.ts` emission in
detail; treat TS declaration specifics as configured via this flag.)

## Practical rules

1. Use `gleam_javascript` typed wrappers (`array`/`promise`/`symbol`) instead of
   raw `Dynamic` for JS-native values.
2. Type annotations are MANDATORY on `@external` functions — never optional.
3. `@external` takes exactly 3 args: target, module, function name.
4. JS external module paths are typically relative `.mjs` paths.
5. To make a function always usable, provide `@external` for both targets OR a
   Gleam body fallback.
6. Never mutate a JS array used as a Gleam tuple — Gleam tuples are immutable.
7. Avoid passing `Infinity`/`NaN` to Gleam (no Gleam syntax for them).
8. JS `Int` boundary: numbers must be whole; floats-as-ints are unchecked.
9. Do NOT vendor JS dependencies into a published Hex package — document the node
   modules users must install instead.
10. Design Gleam APIs idiomatically; do not mirror the external JS API.
11. Use opaque external types (e.g. `ZipHandle`) over generic types (e.g. `Pid`) to
    make invalid states impossible at the type level.
12. Minimise externals; most projects should use none.

## Review checklist

- [ ] `@external` functions have mandatory type annotations.
- [ ] `@external` takes exactly 3 args; target is `javascript` (or `erlang`).
- [ ] JS module paths are correct for the runtime (`.mjs` for Node).
- [ ] Multi-target functions have an `@external` per target or a Gleam fallback.
- [ ] No mutation of arrays used as Gleam tuples.
- [ ] No `Infinity`/`NaN` passed across the boundary.
- [ ] No vendored JS deps in a publishable package.
- [ ] `gleam_javascript` typed wrappers used instead of untyped `Dynamic` where
      possible.
- [ ] `[javascript]` config (runtime, source_maps, typescript_declarations) set
      intentionally.
- [ ] Deno permissions are least-privilege (no `allow_all = true` in production).

## Implementation checklist

- [ ] Set `target = "javascript"` or use `--target javascript`.
- [ ] Add `gleam_javascript` as a dependency for JS-native interop.
- [ ] Configure `[javascript]` runtime (`node`/`deno`/`bun`).
- [ ] Enable `typescript_declarations`/`source_maps` as needed.
- [ ] Write `.mjs` FFI modules alongside Gleam sources.
- [ ] Declare `@external(javascript, ...)` with mandatory type annotations.
- [ ] Provide Gleam fallbacks or per-target externals for multi-target code.

## Validation hooks

- `gleam build --target javascript` — compiles to JS.
- `gleam check --target javascript` — type-checks for JS target.
- `gleam test --target javascript` — runs tests on the configured JS runtime.
- `gleam format --check` — formatting gate.
- Extra unit tests around `@external` boundaries (compiler cannot verify the JS
  side exists or returns annotated types).

## Examples

Typed JS Promise interop:
```gleam
import gleam/javascript/promise

@external(javascript, "./ffi/timer.mjs", "delay")
pub fn delay(ms: Int) -> promise.Promise(Nil)

pub fn main() -> promise.Promise(Nil) {
  delay(100)
}
```

Multi-target external with Gleam fallback:
```gleam
@external(erlang, "lists", "reverse")
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```

## Common mistakes

- Omitting type annotations on `@external` functions (mandatory).
- Mutating a JS array used as a Gleam tuple.
- Passing `Infinity`/`NaN` or non-whole numbers as `Int`.
- Vendoring JS deps into a Hex package (breaks de-duplication/security audits).
- Mirroring the external JS API in Gleam instead of designing idiomatically.
- Using untyped `Dynamic` where `gleam/javascript/*` typed wrappers exist.
- Expecting BEAM/OTP features (processes, supervisors) on the JS target.
- Forgetting that a single-target `@external` is a compile error on the other target.

## Strict vs contextual guidance

Strict: mandatory type annotations on externals; `@external` takes exactly 3 args;
no tuple-array mutation; no `Infinity`/`NaN`; no vendored JS deps in published
packages; minimise externals.

Contextual: choice of JS runtime; whether to emit TS declarations/source maps; Deno
permission granularity; whether to use `gleam/javascript/array` vs `List`.

## Policy decisions for individual repos

- Choose JS runtime (`node`/`deno`/`bun`).
- Decide whether to emit `typescript_declarations`/`source_maps`.
- Define Deno permission allow-lists (least privilege).
- Decide FFI module layout (`.mjs` alongside `src/`).
- Adopt a policy on multi-target fallbacks vs single-target-only externals.

## Related docs

- `gleam-toml-and-targets.md` — `[javascript]` and `[javascript.deno]` config.
- `externals-and-ffi.md` — full `@external` reference (Erlang + JS).
- `project-structure-and-cli.md` — `--target javascript`, `--template javascript`.
- `json-dynamic-and-api-boundaries.md` — `Dynamic` decoding at FFI boundaries.

## Related skills

- `gleam-packages-ffi`
- `gleam-language`
