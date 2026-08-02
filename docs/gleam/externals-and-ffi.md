# Externals and FFI

Gleam's external type and external function features let Gleam code call code
written in other languages. On the Erlang target this includes Erlang, Elixir,
LFE, and other BEAM languages; on the JavaScript target it includes JavaScript
and other compile-to-JS languages.

Externals should be used sparingly. Always prefer Gleam-based solutions, using
externals only when there is no suitable alternative for your needs and
development constraints.

## Sources used

- Crawl file: `docs/gleam/.crawl/08-externals.md`
- Canonical URL: <https://gleam.run/documentation/externals/>
- `gleam_version`: not present on the page. The JavaScript data-construction API
  is referenced as available since Gleam v1.13.

## Related BEAM guidance

- `../beam/nifs.md` — BEAM NIFs are the Erlang-side equivalent of native FFIs.
- `../beam/ports-io.md` — BEAM ports are the canonical external-process I/O
  mechanism.

## Core guidance

External functions are bodiless Gleam functions annotated with `@external`,
which takes exactly three arguments: target (`erlang` or `javascript`), module,
and function name.

```gleam
@external(erlang, "lists", "reverse")
pub fn reverse_list(list: List(element)) -> List(element)
```

```gleam
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```

Type annotations are **mandatory** on external functions. The compiler checks
that all Gleam uses match the annotated types, but it **cannot** verify that the
foreign function exists or returns those types.

External types are defined with no variants and can only be manipulated via
external functions:

```gleam
pub type ErlangReference
```

Multiple `@external` attributes may be placed on one function (multi-target
externals), making it usable on both targets:

```gleam
@external(erlang, "lists", "reverse")
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```

A function may have both a Gleam body and one or more `@external` attributes.
If an external exists for the current target it is used; otherwise the Gleam
body is used as a fallback:

```gleam
@external(erlang, "lists", "reverse")
pub fn reverse_list(list: List(element)) -> List(element) {
  do_reverse(list, [])
}

fn do_reverse(list: List(a), acc: List(a)) -> List(a) {
  case list {
    [] -> acc
    [head, ..tail] -> do_reverse(tail, [head, ..acc])
  }
}
```

A function with `@external` for only one target is usable only on that target;
using it on the other target is a compile error.

## Practical rules

### Erlang externals

Module and function names are written the same as in Erlang. Any public function
in any module can be used. Erlang macros are not usable outside Erlang.

```gleam
@external(erlang, "pokemon", "badge_count")
pub fn pokemon_badge_count() -> Int
```

Elixir externals use the same `erlang` target but require the implicit
`Elixir.` prefix. Elixir macros are not usable outside Elixir.

```gleam
@external(erlang, "Elixir.Pokemon", "badge_count")
pub fn pokemon_badge_count() -> Int
```

#### Gleam data in Erlang

| Gleam | Erlang |
|---|---|
| `True` / `False` | `true` / `false` atoms |
| `Int` | integer |
| `Float` | float |
| `String` | UTF-8 binary; **not** Erlang `string()` (char list) |
| `Nil` | atom `nil` |
| `BitArray` | bit string |
| `List` | proper list; improper lists are incorrect |
| `#(a, b)` | `{a, b}` tuple |
| `Ok(x)` / `Error(x)` | `{ok, x}` / `{error, x}` tagged tuples; bare `ok`/`error` atoms are **not** compatible |
| Custom variant `Guest` | atom `guest` |
| Custom variant `User(id: 10)` | `{user, 10}` tagged tuple (PascalCase → snake_case) |
| `dict` | Erlang map `#{}` |

Erlang `string()` is a char list and is not compatible with Gleam `String`.
Convert with `unicode:characters_to_binary/1` or use
`gleam/erlang/charlist` from the `gleam_erlang` package. The Gleam compiler
generates an Erlang header file per custom type definition containing record
definitions for each variant.

#### Prefer `gleam_erlang`

For Erlang-specific primitives such as references, prefer the `gleam_erlang`
package (e.g. `gleam/erlang/reference`) over defining raw external types. When
no library type fits, define a precise domain type such as `TransactionId` or
`MessageTag` rather than exposing a generic `Reference`.

### JavaScript externals

The module argument is used with a JavaScript `import`, typically a relative
path to a `.mjs` file from the Gleam file's location. Node.js may require a
`.mjs` extension. Node modules are imported with the same path used in JS.

```gleam
@external(javascript, "./my_app/pokemon.mjs", "badge_count")
pub fn pokemon_badge_count() -> Int
```

```gleam
@external(javascript, "has-flag", "hasFlag")
pub fn argv_has_flag(name: String) -> Bool
```

Gleam has no special npm support; configure the JS runtime yourself. Vendoring
is strongly discouraged for published Hex packages because it prevents
de-duplication, bloats code, and complicates security audits. Document the node
modules users must install instead.

#### Importing Gleam modules from JavaScript

Each Gleam module compiles to a JavaScript ES module importable via a relative
path with a `.mjs` extension. After compilation all packages live in the same
directory, so imports ascend past the current package root and descend into the
dependency package:

```javascript
import * as visitor from "./visitor.mjs";
import * as me from "../../do/ray/me.mjs";
import * as two from "../../wibble/one/one.mjs";
```

#### Gleam data in JavaScript (v1.13+)

Since v1.13, Gleam code compiled to JavaScript exposes a data-construction API.
The prelude is importable as if it were at `src/gleam.mjs`:

| Gleam | JavaScript |
|---|---|
| `True` / `False` | `true` / `false` |
| `Int` | number (must be whole) |
| `Float` | number (avoid `Infinity` and `NaN`; Gleam has no syntax for them) |
| `String` | string |
| `Nil` | `undefined` |
| `BitArray` | `BitArray$BitArray(new Uint8Array([...]))` |
| `List` | `List$Empty()` / `List$NonEmpty(head, rest)` |
| `#(a, b)` | array `[a, b]` (must never be mutated) |
| `Ok(x)` / `Error(x)` | `Result$Ok(x)` / `Result$Error(x)` |
| Custom variants | `TypeName$VariantName(...)` generated functions |

Prelude helpers: `BitArray$isBitArray`, `BitArray$data` (returns `DataView`;
raises if bit count not divisible by 8); `List$isEmpty`, `List$isNonEmpty`,
`List$NonEmpty$first`, `List$NonEmpty$rest`; `Result$isOk`, `Result$isError`,
`Result$Ok$0`, `Result$Error$0`.

For each custom-type variant the compiler generates: `TypeName$VariantName`
constructor, `TypeName$isVariantName` check, `TypeName$VariantName$index`
positional accessor, `TypeName$VariantName$label` labelled accessor, and
`TypeName$label` shared-label accessor when every variant has the same label in
the same position.

Dict and other types without literal syntax have no special JS API; import and
use the regular Gleam functions (e.g. `from_list` from `gleam/dict.mjs`).
TypeScript `.d.ts` generation is **not** documented on this page; treat it as
unverified from this source.

### Target-specific modules

The `if erlang { } / if javascript { }` block pattern for target-specific code
belongs to the conditional-compilation guide, not this externals guide. The
externals guide covers target-specificity only through multi-target `@external`
attributes and Gleam fallbacks.

### Dynamic at FFI boundaries

The `Dynamic` type and `dynamic.decode` patterns for untyped FFI data are
documented elsewhere. As a cross-cutting convention, do not use `Dynamic` to
represent FFI types; define a new precise type instead.

## Review and implementation checklist

- [ ] Is there a pure-Gleam alternative? Externals should be a last resort.
- [ ] Are type annotations present on every external function?
- [ ] Does each `@external` have exactly three arguments with target `erlang` or
      `javascript`?
- [ ] For dual-target packages, is every external multi-target or backed by a
      Gleam fallback?
- [ ] Are Erlang `string()` values converted and results passed as `{ok, _}` /
      `{error, _}` tagged tuples?
- [ ] Are Erlang lists always proper lists?
- [ ] Are JavaScript tuple arrays never mutated, and are `Int` values whole with
      no `Infinity`/`NaN`?
- [ ] Are opaque domain types used instead of generic types (e.g. `Pid`) in
      public APIs?
- [ ] Is vendoring avoided for published Hex packages?
- [ ] Are additional unit tests written at every FFI boundary?

## Validation hooks

```sh
# Verify each target compiles and run the test suite
gleam build --target erlang
gleam build --target javascript
gleam test
```

Increase test coverage at FFI boundaries; the compiler cannot verify the
foreign implementation.

## Examples

See Core guidance and Practical rules above for Erlang, JavaScript, Elixir,
multi-target, and fallback externals.

## Common mistakes

Omitting type annotations; assuming the compiler checks that the foreign
function exists; passing Erlang `string()` (char list) as a Gleam `String`;
returning bare `ok` / `error` atoms instead of `{ok, _}` / `{error, _}`;
using improper Erlang lists as Gleam `List` values; mutating JavaScript arrays
used as Gleam tuples; passing non-whole numbers, `Infinity`, or `NaN` as `Int`;
exposing generic external types (`Pid`, `Reference`) in public APIs instead of
opaque domain types (`ZipHandle`); vendoring JS dependencies into Hex packages;
and writing too few tests at FFI boundaries.

## Strict vs contextual guidance

Strict (non-negotiable):

- Type annotations are mandatory on external functions.
- `@external` takes exactly three arguments.
- Target must be `erlang` or `javascript`.
- External types have no variants and cannot be constructed directly.
- Erlang macros are not usable outside Erlang; Elixir macros not usable outside
  Elixir.
- A single-target external is usable only on that target.

Contextual: whether to allow externals at all; whether to require dual-target
support; whether to permit vendoring for internal-only packages; and how
idiomatic to make the Gleam API versus mirroring the external library.

## Policy decisions for individual repos

- Allow externals? Most projects should use none; require justification in
  review.
- Dual-target requirement: must every public external work on both Erlang and
  JavaScript, either through multi-target attributes or Gleam fallbacks?
- Vendoring policy: prohibit vendored JS dependencies in published Hex packages;
  internal projects may allow it with documented risk acceptance.
- FFI type discipline: require precise domain types and forbid `Dynamic` for
  representing FFI shapes.

## Related docs

- `javascript-target.md` — JavaScript target details.
- `erlang-interop.md` — Erlang/Elixir interop details.
- `json-dynamic-and-api-boundaries.md` — using `Dynamic` and decoders at
  boundaries.
- `conventions-patterns-antipatterns.md` — cross-cutting conventions including
  the anti-pattern of using `Dynamic` with FFI.
- `package-management-and-publishing.md` — Hex publishing, dependency
  de-duplication, and vendoring concerns.

## Related skills

- `gleam-packages-ffi`
- `gleam-otp-interop` (when externals touch Erlang/OTP primitives)
