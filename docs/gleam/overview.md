# Gleam overview

## Purpose
Establish the Gleam mental model: a statically typed, immutable, expression-based
functional language that compiles to Erlang (BEAM) and JavaScript. This doc orients
readers to what Gleam is, what it deliberately is not, its compilation targets,
its type-system stance, and how it relates to the BEAM runtime guidance in
`docs/beam/`. It is the entry point for the Gleam doc set; language mechanics live
in the sibling fundamentals docs.

## Sources used
- Crawl: `docs/gleam/.crawl/01-documentation.md` — https://gleam.run/documentation/
- Crawl: `docs/gleam/.crawl/02-tour-everything.md` — https://tour.gleam.run/everything/
- Crawl: `docs/gleam/.crawl/03-writing-gleam.md` — https://gleam.run/writing-gleam/
- Crawl: `docs/gleam/.crawl/11-faq.md` — https://gleam.run/frequently-asked-questions/

## Related BEAM guidance
- `docs/beam/overview.md` — shared runtime model. Gleam on the Erlang target runs on
  the BEAM VM and preserves BEAM semantics (actor model, supervision, hot code
  reloading); the BEAM overview is the canonical reference for those runtime concepts.
  Gleam-specific concurrency/FFI details live in the Gleam doc set, not `docs/beam/`.

## Core guidance
Gleam is a "small and cohesive language with a minimal feature set" (FAQ). It is a
statically typed, expression-based functional language with no `null`, no implicit
conversions, no exceptions, and full type checking. From the tour: "Gleam has no
`null`, no implicit conversions, no exceptions, and always performs full type
checking. If the code compiles you can be reasonably confident it does not have any
inconsistencies that may cause bugs or crashes."

### What Gleam is
- A type-safe functional language. "Gleam is an impure functional language like
  OCaml or Erlang. Impure actions like writing to files and printing to the console
  are possible without special handling." (FAQ)
- Immutable by design. "All data structures in Gleam are immutable and are
  implemented using structural sharing, making them very efficient to update." (FAQ)
- Hindley-Milner derived, nominal type system. "Gleam's type system is based on the
  same Hindley-Milner type system that underpins OCaml, Haskell, F#, etc." (FAQ)
  Types are nominal (distinct-by-name), not structural.
- Production-ready. "Gleam is a production-ready programming language and the Erlang
  and JavaScript runtimes it runs on are extremely mature and battle-tested. Gleam
  is ready for mission critical workloads." (FAQ)

### Compilation targets
- "Gleam compiles to Erlang or JavaScript." (FAQ)
- Erlang is the default target used in the writing-gleam guide; JavaScript is
  selected via `--target javascript` (writing-gleam).
- On the Erlang target, Gleam compiles to Erlang *source* (not Core Erlang / abstract
  format). This affects tooling: stack traces and coverage/profiling are less
  accurate than native Erlang/Elixir tooling because Gleam goes via Erlang source
  (FAQ, Elixir comparison).
- BEAM semantics are preserved on the Erlang target: actor model, OTP, hot code
  reloading all work, though type-checking of hot upgrades is not possible (FAQ).

### Error model: no exceptions, no nil
- "Gleam doesn't use exceptions, instead computations that can either succeed or
  fail return a value of the built-in `Result(value, error)` type." (tour)
- `Nil` is a unit type, not a nullable. "`Nil` is not a valid value of any other
  types. Therefore, values in Gleam are not nullable." (tour)
- Fallible functions always return `Result`; `Option` is reserved for optional
  *data* (function args / data-structure fields), not for fallible returns. See
  `result-option-and-errors`.

### What Gleam deliberately excludes
From the FAQ ("What features are not planned for Gleam?"): exception-based error
handling, extensible variants, function and operator overloading, implicit
arguments, implicitly nullable values, linear/affine/relevant types, Lisp-style
macros, manual memory management, mutation, object orientation, optional arguments,
type classes or traits, untagged unions.
- No type classes: "Type classes ... are unfortunately not a good fit for Gleam and
  they are not planned." (FAQ)
- No metaprogramming today (open to a design that preserves readability and fast
  compilation).
- No loops / no early returns: iteration is via recursion; `use` mitigates
  indentation for callback-heavy code (tour).

### Concurrency and interop stance
- "Type safe message passing is implemented in Gleam in libraries, rather than being
  part of the core language itself." (FAQ) See `gleam_erlang` / `gleam_otp`.
- Gleam interops with Erlang, Elixir, and other BEAM languages on the Erlang target,
  and with JavaScript / compile-to-JS on the JavaScript target (externals guide).
- "Gleam code is designed to be usable from all BEAM languages." (FAQ)

### Compiler and tooling
- The Gleam compiler is written in Rust (FAQ). The build tool, language server, and
  `gleam.toml` config are first-class (documentation hub).
- Documentation is split: `gleam.run` (guides/references), `tour.gleam.run`
  (interactive tour), `packages.gleam.run` (package index), `hexdocs.pm` (stdlib +
  package API docs) (documentation hub).

## Practical rules
- Treat Gleam as a small, cohesive language: do not reach for features it lacks
  (type classes, macros, overloading, mutation). Solve problems with first-class
  functions, pattern matching, and custom types.
- Encode fallibility in the type system with `Result`, never with exceptions or
  `Nil`-punning.
- Model domains with custom types that make invalid states unrepresentable.
- Pick the target deliberately: Erlang for BEAM/OTP/concurrency; JavaScript for
  browser/Node. Multi-target code needs externals or Gleam fallbacks per target.
- Prefer the Gleam core libraries (`gleam_stdlib`, `gleam_time`, `gleam_json`,
  `gleam_http`, `gleam_erlang`, `gleam_otp`, `gleam_javascript`) over reimplementing
  (conventions doc).

## Review checklist
- [ ] Is fallibility expressed via `Result`, not exceptions or `Option` returns?
- [ ] Are custom types used to make invalid states unrepresentable?
- [ ] Is the target choice explicit (Erlang default vs `--target javascript`)?
- [ ] Are core libraries reused rather than reimplemented?
- [ ] Is code free of features Gleam deliberately excludes (type classes, macros,
      overloading, mutation, implicit null)?

## Implementation checklist
- [ ] Confirm the project compiles on the intended target(s).
- [ ] Use `gleam_stdlib` types (`Result`, `Option`, `List`, tuples) as the baseline.
- [ ] For BEAM-specific primitives, prefer `gleam_erlang` over raw externals.
- [ ] For OTP, prefer `gleam_otp` over hand-rolled processes.

## Validation hooks
- `gleam build` — compiles for the default (Erlang) target.
- `gleam build --target javascript` — compiles for the JavaScript target.
- `gleam test` — runs the gleeunit test suite (writing-gleam).
- Exhaustiveness checking and full type checking are compiler-enforced; a clean
  build is the primary correctness gate.

## Examples
```gleam
// Fallible computation returns Result, never throws.
pub fn parse_count(input: String) -> Result(Int, Nil) {
  int.parse(input)
}
```

## Common mistakes
- Reaching for `Option` as a fallible return type — use `Result` (with `Nil` error
  when there is no extra detail).
- Assuming `Nil` can stand in for "missing value" of another type — it cannot.
- Expecting type classes / traits / overloading — Gleam has none.
- Assuming JS-target float semantics match BEAM-target (overflow differs).

## Strict vs contextual guidance
- Strict: no exceptions, no `null`, no mutation, no type classes, full type checking,
  exhaustiveness checking. These are language-level and non-negotiable.
- Contextual: target choice, core-library vs custom implementation, opaque vs
  transparent custom types — decided per project/domain.

## Policy decisions for individual repos
- Which target(s) must the project support (Erlang only, JS only, both)?
- Which core libraries are approved dependencies?
- Is `gleam_otp` / `gleam_erlang` permitted (Erlang target only)?

## Related docs
- `language-fundamentals`
- `types-records-and-patterns`
- `functions-pipelines-and-use`
- `result-option-and-errors`
- `conventions-patterns-antipatterns`
- `erlang-interop`
- `javascript-target`
- `otp-actors-and-supervision`

## Related skills
- `gleam-language`
- `gleam-otp-interop`
- `gleam-packages-ffi`
