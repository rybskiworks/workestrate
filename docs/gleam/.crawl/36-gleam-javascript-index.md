# Crawl: hexdocs.pm/gleam_javascript/ (index)
- seed_url: https://hexdocs.pm/gleam_javascript/
- canonical_url: https://gleam-javascript.hexdocs.pm/ (page declares canonical https://hexdocs.pm/gleam_javascript/index.html; curl final URL redirects to https://gleam-javascript.hexdocs.pm/)
- family: Gleam core package (gleam_javascript)
- fetch: HTTP 200
- gleam_javascript_version: v1.0.0
- feeds_docs: javascript-target.md, externals-and-ffi.md

## Purpose
Per the index overview: "Work with JavaScript types and values in Gleam, including promises!"
gleam_javascript is the official Gleam core package that provides typed wrappers
over JavaScript values for code compiled to the JavaScript target. It is the
typed-FFI / interop layer used alongside Gleam's `external` declarations when
targeting JavaScript, exposing JS-native primitives (arrays, promises, symbols)
as typed Gleam modules so JS values can be handled safely without resorting to
untyped `Dynamic`.

## Module list + one-line each
- `gleam/javascript/array` — typed wrapper around JS `Array` values (mutable,
  index-based, JS-native array operations exposed as Gleam functions).
- `gleam/javascript/promise` — typed wrapper around JS `Promise` values; the
  async/concurrency primitive on the JS target (Gleam's async story on JS).
- `gleam/javascript/symbol` — typed wrapper around JS `Symbol` values (unique
  keys / well-known symbols interop).

(No `javascript/types`, `javascript/document`, or `javascript` root module
listed on the v1.0.0 index — only the three submodules above. The package is
intentionally narrow: typed wrappers for the JS primitives that need Gleam
types; everything else is handled via `external` FFI + `Dynamic`.)

## JS-target model (typed wrappers; Promise/array; typed FFI)
The package embodies Gleam's "typed wrappers over JS values" interop model on
the JavaScript target:

- JS values that have no direct Gleam equivalent (mutable arrays, promises,
  symbols) are represented as opaque-ish typed Gleam types in dedicated modules
  (`gleam/javascript/array`, `gleam/javascript/promise`, `gleam/javascript/symbol`).
- Operations on those JS values are exposed as ordinary typed Gleam functions,
  so callers stay in Gleam's type system rather than dropping to `Dynamic`.
- `gleam/javascript/promise` is the JS-target async primitive: it is the typed
  face of JS `Promise`, used where the Erlang target would use processes/
  OTP supervisors. This is the typed-FFI counterpart to writing
  `external fn ... -> Promise(a)` declarations.
- `gleam/javascript/array` is the typed face of JS `Array` (distinct from
  Gleam's immutable `List`); it is the bridge for FFI boundaries that produce
  or consume JS arrays.
- `gleam/javascript/symbol` covers JS `Symbol` interop (well-known symbols,
  unique identity) for typed access where a raw `Dynamic` would be unsafe.

This is the "typed FFI" half of the JS-target story: `external` declarations
(link 08, externals-and-ffi) declare the untyped boundary, and
`gleam/javascript/*` provides the typed wrappers that callers use on the Gleam
side of that boundary.

## Relationship to externals page (08) + gleam-toml JS config (05)
- Link 08 (externals-and-ffi.md): gleam_javascript is the typed-wrapper layer
  that sits on the Gleam side of `external` FFI declarations. Where
  externals-and-ffi describes how to declare `external fn`/`external type` to
  cross into JS, gleam_javascript provides the typed Gleam types (`Promise`,
  JS `Array`, `Symbol`) that those externals produce/consume, so the boundary
  stays typed instead of leaking `Dynamic`.
- Link 05 (gleam-toml / JS-target config): gleam_javascript is the package you
  depend on when `gleam.toml` declares `target = "javascript"` (or
  target-specific deps). It is the canonical JS-target interop dependency;
  the JS target config in gleam.toml is what makes this package's modules
  available and relevant (they are JS-only and have no Erlang equivalent).

## Version notes
- Version: v1.0.0 (per page `<title>gleam_javascript · v1.0.0</title>`).
- v1.0.0 indicates the stable, post-1.0 Gleam package line; the module set
  (array, promise, symbol) is the current stable surface.
- Repository: https://github.com/gleam-lang/javascript
- Hex: https://hex.pm/packages/gleam_javascript
- Docs: https://hexdocs.pm/gleam_javascript (canonical index)

## Discovered links
### Relevant (crawl later)
- https://gleam-javascript.hexdocs.pm/gleam/javascript/array.html
  (gleam/javascript/array — JS Array typed wrapper)
- https://gleam-javascript.hexdocs.pm/gleam/javascript/promise.html
  (gleam/javascript/promise — JS Promise typed wrapper; JS-target async)
- https://gleam-javascript.hexdocs.pm/gleam/javascript/symbol.html
  (gleam/javascript/symbol — JS Symbol typed wrapper)

### Skipped
- https://github.com/gleam-lang/javascript (source repo, not docs)
- https://hex.pm/packages/gleam_javascript (package metadata, not docs)
- https://hexdocs.pm/gleam_javascript/index.html (self / canonical)
