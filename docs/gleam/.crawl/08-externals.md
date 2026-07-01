# Crawl: documentation/externals/
- seed_url: https://gleam.run/documentation/externals/
- canonical_url: https://gleam.run/documentation/externals/
- family: Gleam official docs
- fetch: 200
- gleam_version: not present on page (v1.13 referenced for JS data construction API)
- feeds_docs: externals-and-ffi.md, erlang-interop.md, javascript-target.md

## Purpose
Gleam's external type and external function features let Gleam code use code
written in other languages, providing solutions when no pure-Gleam library is
readily available. The languages usable depend on the compilation target:
- Gleam on Erlang can use Erlang, Elixir, LFE, and other BEAM languages.
- Gleam on JavaScript can use JavaScript, and other compile-to-JS languages.

There is no additional performance cost to calling functions written in other
languages vs those written in Gleam.

Externals should be used sparingly. Always prefer Gleam-based solutions, using
externals only when there is no suitable alternative for your needs and
development constraints.

## external type / external fn
External functions are functions written in another language, imported into
Gleam code, callable like a normal Gleam function. They are written as a
bodiless function with the `@external` attribute taking 3 arguments:
1. The target — either `erlang` or `javascript`.
2. The module the function is exported from.
3. The name of the function.

Type annotations are NOT optional for external functions — they must always
be written. The Gleam compiler ensures all uses of the function are correct
for the annotated types, but it CANNOT verify that the function implemented
in the other language returns the specified types, or even that it exists.
Great care must be taken when defining external functions. You may wish to
write more unit tests than usual when using external functions.

External types are defined with syntax similar to custom types, except no
variants are specified:
```gleam
pub type ErlangReference
```
Gleam knows nothing about an external type other than its existence and the
name given in the definition, so it cannot be constructed or manipulated
directly in Gleam code — external functions must be used instead.

## @external attribute (Erlang form + JavaScript form)
Erlang form:
```gleam
@external(erlang, "lists", "reverse")
pub fn reverse_list(list: List(element)) -> List(element)
```
JavaScript form (module path is typically a relative path to a `.mjs` file,
relative to the location of the Gleam file containing the external):
```gleam
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```
The `@external` attribute can be specified multiple times on one function,
giving an implementation for both targets (multi-target externals), making
the function always usable:
```gleam
@external(erlang, "lists", "reverse")
@external(javascript, "./project_ffi.mjs", "reverse_list")
pub fn reverse_list(list: List(element)) -> List(element)
```

## Gleam fallbacks
A function may have BOTH a Gleam implementation and an external implementation.
If there is an external implementation for the current compilation target it
will be used; otherwise the Gleam implementation is used. Useful when a
function can be implemented in Gleam but an optimised version is possible on
one particular target:
```gleam
@external(erlang, "lists", "reverse")
pub fn reverse_list(list: List(element)) -> List(element) {
  reverse_and_prepend(list, [])
}
```

## Erlang externals + gleam_erlang
Erlang is the most straightforward language to declare external functions for.
Module and function names are written the same in Erlang code as in the Gleam
external definition. Any public function in any module can be used as an
external. Erlang macros are NOT usable outside of Erlang.

Example Erlang module + Gleam external:
```erlang
-module(pokemon).
-export([badge_count/0]).
badge_count() -> 8.
```
```gleam
@external(erlang, "pokemon", "badge_count")
pub fn pokemon_badge_count() -> Int
```

Elixir externals: same `@external(erlang, ...)` target is used (since the
Gleam code is using the Erlang/BEAM target), but Elixir modules have the
implicit `Elixir.` prefix which must be specified:
```gleam
@external(erlang, "Elixir.Pokemon", "badge_count")
pub fn pokemon_badge_count() -> Int
```
Elixir macros are NOT usable outside of Elixir.

### Gleam data in Erlang (mapping table)
| Gleam | Erlang |
|---|---|
| `True`/`False` | `true`/`false` (boolean atoms) |
| `Int` | integer |
| `Float` | float |
| `String` | UTF8 binary (`<<"..."/utf8>>`) — NOT Erlang `string()` (char list) |
| `Nil` | atom `nil` |
| `BitArray` | bit string |
| `List` | list (always proper; improper lists are incorrect) |
| `#(a, b)` | `{a, b}` tuple |
| `Ok(x)`/`Error(x)` | `{ok, x}`/`{error, x}` tagged tuples (bare atoms `ok`/`error` are NOT compatible) |
| Custom variant `Guest` | atom `guest` |
| Custom variant `User(id: 10)` | `{user, 10}` tagged tuple (PascalCase → snake_case) |
| `dict` | Erlang map `#{}` |

Erlang `string()` is a list of integers (char list), NOT compatible with Gleam
strings. Convert with `unicode:characters_to_binary/1`, or use
`gleam/erlang/charlist` from the `gleam_erlang` package.

The Gleam compiler generates an Erlang header file for each custom type
definition, containing an Erlang record definition for each variant —
includeable into Erlang modules for record syntax sugar.

### gleam_erlang usage
For real programs using Erlang-specific types like references, prefer the
`gleam_erlang` package modules (e.g. `gleam/erlang/reference`) over defining
raw external types, or define a more precise domain type (e.g. `TransactionId`,
`MessageTag`) rather than exposing a generic `Reference`.

## JavaScript externals + TS declarations
When defining JavaScript externals the module is internally used with a JS
`import` statement, so it typically should be a path to a JS file, relative to
the location of the Gleam file containing the external definition. The
function name is written the same as in the JS code; any public function in any
module can be used as an external.

```javascript
// In src/my_app/pokemon.mjs
export function badge_count() { return 8; }
```
```gleam
// In src/my_app.gleam
@external(javascript, "./my_app/pokemon.mjs", "badge_count")
pub fn pokemon_badge_count() -> Int
```
Each JS runtime has its own ES module import rules — ensure paths are correct
for the runtime. NodeJS may require a `.mjs` extension.

### Using node modules
Node modules are used by the same path you would use in JS code:
```gleam
@external(javascript, "has-flag", "hasFlag")
pub fn argv_has_flag(name: String) -> Bool
```
Gleam has no special support for `npm` or other JS package managers — configure
your JS runtime to ensure packages are present (e.g. `package.json` +
`npm install`). Vendoring (copy/pasting dependency code) is strongly
discouraged for published Hex packages: it prevents dependency de-duplication,
causes code bloat, and makes security audits/patches difficult. Instead
document the node modules users need to install.

### Importing Gleam modules from JavaScript
Each Gleam module compiles to a JS ES module importable via a relative path
with a `.mjs` extension. After compilation each Gleam package resides in the
same directory, so a relative path ascends up past the current package root
and back down into the desired dependency package directory.
- `src/code.mjs` → import `visitor` from same package: `import * as visitor from "./visitor.mjs";`
- `src/one/two/three.mjs` → import `do/ray/me`: `import * as me from "../../do/ray/me.mjs";`
- `src/app/web.mjs` → import `one/two` from package `wibble`: `import * as two from "../../wibble/one/one.mjs";`

### TypeScript declarations
NOTE: This page does NOT document TypeScript declaration (`.d.ts`) generation.
The page covers the JS data-construction/deconstruction API exposed by the
Gleam prelude (since v1.13) and per-custom-type generated functions, but does
not describe `.d.ts` emission. Treat TS declaration details as unverified
from this source.

### Gleam data in JavaScript (mapping + prelude API, v1.13+)
Since v1.13 Gleam code compiled to JS exposes an API for constructing Gleam
data types. The Gleam prelude module is importable as if located at
`src/gleam.mjs` in any Gleam project (internally provided by the build tool).

| Gleam | JavaScript |
|---|---|
| `True`/`False` | `true`/`false` (boolean) |
| `Int` | number (must be whole) |
| `Float` | number (no restriction; avoid infinity/NaN — Gleam has no syntax for them) |
| `String` | string |
| `Nil` | `undefined` |
| `BitArray` | `BitArray$BitArray(new Uint8Array([...]))` from prelude |
| `List` | `List$Empty()` / `List$NonEmpty(head, rest)` from prelude |
| `#(a, b)` | array `[a, b]` (immutable — never mutate) |
| `Ok(x)`/`Error(x)` | `Result$Ok(x)` / `Result$Error(x)` from prelude |
| Custom variants | generated `TypeName$VariantName(...)` functions per variant |

Prelude helpers:
- BitArray: `BitArray$isBitArray`, `BitArray$data` (returns `DataView`; raises if bit count not divisible by 8).
- List: `List$isEmpty`, `List$isNonEmpty`, `List$NonEmpty$first`, `List$NonEmpty$rest`.
- Result: `Result$isOk`, `Result$isError`, `Result$Ok$0`, `Result$Error$0`.

Custom types: for each variant, generated functions:
- `TypeName$VariantName(...args)` — constructor.
- `TypeName$isVariantName(value)` — variant check (value must be correct type).
- `TypeName$VariantName$index(value)` — positional field accessor.
- `TypeName$VariantName$label(value)` — labelled field accessor (if fields labelled).
- `TypeName$label(value)` — shared-label accessor across all variants (if same label in same position across all variants).

Import path for custom types from another package: path as if the other
package's `src` directory is at project root, with the package name as the
directory instead of `src`. E.g. from `src/wibble/wobble.js` to
`gleam/option` in `gleam_stdlib`: `../../gleam_stdlib/gleam/option.mjs`.

Dict (and other types with no literal syntax): no special JS API — import and
use the regular Gleam functions (e.g. `from_list` from `gleam/dict.mjs`).

## Target-specific modules (if erlang/javascript blocks)
NOTE: This page does NOT document the `if erlang { } / if javascript { }`
block pattern for target-specific code. That pattern belongs to the
"Conditional compilation" / "Target-specific code" guide, not the externals
guide. The externals guide covers target-specificity only via:
1. Multi-target `@external` attributes (one per target on a single bodiless fn).
2. Gleam fallbacks (Gleam body + `@external` for one target).
A function with an `@external` for only one target is usable only when
compiling to that target — using it on the other target is a compile error.

## Dynamic at FFI boundaries
NOTE: This page does NOT cover the `Dynamic` type or its use at FFI
boundaries. `Dynamic` (from `gleam/dynamic`) and `dynamic.decode` patterns
for safely handling untyped FFI data are documented elsewhere (likely the
"Dynamic" guide / `gleam_stdlib` docs). Treat Dynamic-at-FFI details as
unverified from this source.

## Review risks
- The Gleam compiler CANNOT verify that an external function actually exists
  in the target language or that it returns the annotated types. Type safety
  at the boundary is the programmer's responsibility — bugs and runtime
  errors are possible.
- The Gleam language server cannot work with non-Gleam languages, so editor
  assistance is reduced when working with external code.
- External code can make it challenging or impossible to run a Gleam project
  on both Erlang and JavaScript targets.
- Erlang `string()` (char list) is NOT compatible with Gleam `String` —
  silent corruption if passed through an external boundary unconverted.
- Bare Erlang atoms `ok`/`error` are NOT compatible with Gleam `Result`
  (`{ok, _}`/`{error, _}` tagged tuples required).
- Erlang improper lists passed as Gleam `List` are incorrect.
- JavaScript arrays used as Gleam tuples must never be mutated
  (`push`, index assignment) — Gleam tuples are immutable.
- JavaScript `Int` boundary: numbers must be whole; floats-as-ints are
  unchecked.
- Infinity/NaN have no Gleam syntax — avoid passing to Gleam code.
- Vendoring JS dependencies breaks Hex de-duplication and security audits.
- API design risk: imitating the external API in Gleam is usually a mistake.
  Use opaque external types (e.g. `ZipHandle`) rather than reusing generic
  types (e.g. `Pid`) to make invalid states impossible at the type level.
- Recommendation: write more unit tests than usual when using externals.

## Strict rules
- Type annotations are MANDATORY on external functions — never optional.
- `@external` takes exactly 3 args: target, module, function name.
- Target must be `erlang` or `javascript`.
- An external function with an `@external` for only one target is usable only
  on that target; using it on the other target is a compile error.
- To make a function always usable, provide an `@external` for each target,
  OR provide a Gleam body (fallback) plus target-specific `@external`s.
- External types have no variants and cannot be constructed/manipulated
  directly — only via external functions.
- Erlang macros are not usable outside Erlang; Elixir macros not usable
  outside Elixir.
- Prefer `gleam_erlang` package modules over raw external types for
  Erlang-specific primitives.
- Design Gleam APIs idiomatically regardless of internal implementation —
  do not mirror the external API.
- Minimise use of externals; most projects should use none.

## Verbatim quotes
- "Externals should be used sparingly. Always prefer Gleam based solutions,
  using externals only when there is no suitable alternative for your needs
  and development constraints."
- "Type annotations are not optional for external functions, they must always
  be written. The Gleam compiler will ensure that all uses of the function
  will be correct for the annotated types, but it cannot verify that the
  function implemented in the other language returns the specified types, or
  even that it exists."
- "You may wish to write more unit tests than usual when using external
  functions."
- "The `@external` attribute can be specified multiple times, giving an
  external function an implementation for both targets, making it always
  usable."
- "It is possible for a function to have a Gleam implementation and also an
  external implementation at the same time. If there is an external
  implementation for the current compilation target then it will be used, the
  Gleam implementation being used otherwise."
- "Gleam doesn't know anything about this type other than its existence and
  the name it has been given in the definition, so it cannot be constructed or
  manipulated directly in Gleam code, External functions must be used instead."
- "Erlang's `string()` type is a list of integers that represent characters,
  and it is not compatible with Gleam strings."
- "In Gleam lists are always proper lists. Passing an Erlang improper list to
  Gleam code as a list is incorrect."
- "Occasionally Erlang will use just the atoms `ok` or `error` in place of a
  tagged tuple. These are not compatible with Gleam's result type."
- "Gleam tuples are immutable, so you should never mutate an array that is
  being used as a Gleam tuple."
- "Gleam doesn't have any syntax for infinity and NaN. You should avoid
  passing these to Gleam code."
- "Vendoring (i.e. copy/pasting the dependency code into your project) is
  strongly discouraged if you intend to publish your package to the Hex
  package manager."
- "Always design your APIs with Gleam in mind, no matter how they are
  implemented internally."
- "Since v1.13 Gleam code compiled to JavaScript will expose an API for
  constructing Gleam data types."

## Version notes
- Page does not state a Gleam compiler version explicitly.
- v1.13 is referenced as the version since which JS-compiled Gleam exposes an
  API for constructing Gleam data types (prelude `BitArray$`, `List$`,
  `Result$`, and per-custom-type `TypeName$VariantName` functions).
- Elixir section references Elixir `Record` module docs at hexdocs.pm/elixir/1.12.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/reference.html — gleam_erlang reference module (Erlang interop)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/charlist.html#to_string — charlist interop (Erlang string conversion)
- https://hexdocs.pm/gleam_javascript/gleam/javascript/array.html — gleam_javascript Array module (JS interop, list alternative)

### Skipped
- #What-are-externals (in-page anchor)
- #When-to-use-externals (in-page anchor)
- #Defining-external-functions (in-page anchor)
- #Multi-target-externals (in-page anchor)
- #Gleam-fallbacks (in-page anchor)
- #Defining-external-types (in-page anchor)
- #Designing-APIs-using-externals (in-page anchor)
- #Erlang-externals (in-page anchor)
- #Gleam-data-in-Erlang (in-page anchor)
- #Elixir-externals (in-page anchor)
- #Gleam-data-in-Elixir (in-page anchor)
- #JavaScript-externals (in-page anchor)
- #Using-node-modules (in-page anchor)
- #Importing-Gleam-modules-from-JavaScript (in-page anchor)
- #Gleam-data-in-JavaScript (in-page anchor)
- https://www.erlang.org/doc/system/data_types.html#reference — Erlang reference type (external, not Gleam)
- https://www.erlang.org/doc/apps/stdlib/unicode.html#characters_to_binary/1 — Erlang unicode function (external, not Gleam)
- https://hexdocs.pm/elixir/1.12/Record.html — Elixir Record module (external, not Gleam)
