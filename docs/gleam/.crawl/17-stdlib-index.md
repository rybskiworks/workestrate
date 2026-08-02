# Crawl: hexdocs.pm/gleam_stdlib/ (index)
- seed_url: https://hexdocs.pm/gleam_stdlib/
- canonical_url: https://gleam-stdlib.hexdocs.pm/
- family: Gleam stdlib (hexdocs)
- fetch: 200
- gleam_stdlib_version: 1.0.3
- feeds_docs: stdlib.md

## Purpose
Gleam's standard library. A single dependency every Gleam project relies on for
core data types and operations. Supports both compilation targets: Erlang and
JavaScript. Installed via `gleam add gleam_stdlib@1`. The package is versioned
independently of the Gleam compiler (compiler at time of crawl: 1.16.0 per page
metadata); the stdlib package itself is at v1.0.3.

The stdlib provides the `gleam/...` module namespace covering built-in types
(`Int`, `Float`, `String`, `List`, `Result`, `Option`, `Bool`, `Nil`), collection
types (`Dict`, `Set`), binary/byte handling (`BitArray`, `BytesTree`,
`StringTree`), dynamic/decode utilities for runtime-typed boundaries, and
miscellaneous helpers (`io`, `uri`, `function`, `pair`, `order`).

## Module list (every module + one-line purpose)
| Module | Purpose |
| --- | --- |
| gleam/bit_array | Construction, slicing, and inspection of `BitArray` binary data. |
| gleam/bool | Boolean logic helpers (`and`, `or`, `not`, `nand`, `nor`, `xor`, `guard`). |
| gleam/bytes_tree | Efficient append-only builder for `BitArray` (byte) data. |
| gleam/dict | Operations on the `Dict(k, v)` key-value map type. |
| gleam/dynamic | Runtime-typed value inspection and casting for interop/decoding. |
| gleam/dynamic/decode | Declarative decoders from `dynamic` into typed Gleam values. |
| gleam/float | Operations and math functions on `Float`. |
| gleam/function | Function combinators (`identity`, `compose`, `constant`, `tap`, `apply`). |
| gleam/int | Integer arithmetic, parsing, and bitwise operations on `Int`. |
| gleam/io | Stdout/stderr printing and debugging (`println`, `debug`). |
| gleam/list | Operations on the `List(a)` singly-linked list type. |
| gleam/option | The `Option(a)` type and helpers (explicit nullable values). |
| gleam/order | The `Order` type (`Lt`/`Eq`/`Gt`) and comparison combinators. |
| gleam/pair | Helpers for 2-tuples (`first`, `second`, `swap`, `map`). |
| gleam/result | Operations on `Result(a, b)` (ok/error handling, combinators). |
| gleam/set | Operations on the `Set(a)` unique-element collection type. |
| gleam/string | String construction, splitting, inspection, and transformation. |
| gleam/string_tree | Efficient append-only builder for `String`. |
| gleam/uri | URI parsing, building, and percent-encoding. |

Total: 19 modules.

## Version notes
- gleam_stdlib package version: **1.0.3** (from page title `gleam_stdlib · v1.0.3`).
- Page also references compiler/tooling version `1.16.0` (appears 9× in page
  metadata); this is the Gleam compiler version, not the stdlib version.
- Install line pins the major version: `gleam add gleam_stdlib@1`.
- Targets: both Erlang and JavaScript are supported by the stdlib.
- The seed URL `https://hexdocs.pm/gleam_stdlib/` 301-redirects to the canonical
  host `https://gleam-stdlib.hexdocs.pm/` (per-package HexDocs subdomain).

## Discovered links

### Relevant (crawl later)
Per-module documentation pages (relative to canonical root
`https://gleam-stdlib.hexdocs.pm/`):
1. https://gleam-stdlib.hexdocs.pm/gleam/bit_array.html
2. https://gleam-stdlib.hexdocs.pm/gleam/bool.html
3. https://gleam-stdlib.hexdocs.pm/gleam/bytes_tree.html
4. https://gleam-stdlib.hexdocs.pm/gleam/dict.html
5. https://gleam-stdlib.hexdocs.pm/gleam/dynamic.html
6. https://gleam-stdlib.hexdocs.pm/gleam/dynamic/decode.html
7. https://gleam-stdlib.hexdocs.pm/gleam/float.html
8. https://gleam-stdlib.hexdocs.pm/gleam/function.html
9. https://gleam-stdlib.hexdocs.pm/gleam/int.html
10. https://gleam-stdlib.hexdocs.pm/gleam/io.html
11. https://gleam-stdlib.hexdocs.pm/gleam/list.html
12. https://gleam-stdlib.hexdocs.pm/gleam/option.html
13. https://gleam-stdlib.hexdocs.pm/gleam/order.html
14. https://gleam-stdlib.hexdocs.pm/gleam/pair.html
15. https://gleam-stdlib.hexdocs.pm/gleam/result.html
16. https://gleam-stdlib.hexdocs.pm/gleam/set.html
17. https://gleam-stdlib.hexdocs.pm/gleam/string.html
18. https://gleam-stdlib.hexdocs.pm/gleam/string_tree.html
19. https://gleam-stdlib.hexdocs.pm/gleam/uri.html

### Skipped
- https://hex.pm/packages/gleam_stdlib (Hex.pm package page; not a docs page)
- https://discord.gg/Fm8Pwmy (Discord invite; community, not docs)
- HexDocs badge/shield image URLs (img.shields.io)
- Pages / Links / Installation / Targets nav anchors on the same index page
