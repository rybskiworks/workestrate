# Gleam conventions, patterns, and anti-patterns

## Purpose

Official Gleam guidance on writing idiomatic, reviewable code. Conventions and
anti-patterns are always-rules; patterns are techniques to apply when they
benefit the code. Also covers syntactic migration pitfalls from Elixir, Erlang,
and Rust.

## Sources used

- `/home/node/Development/ai-workbench/docs/gleam/.crawl/04-conventions.md` —
  PRIMARY source.
  <https://gleam.run/documentation/conventions-patterns-and-anti-patterns/>
- `/home/node/Development/ai-workbench/docs/gleam/.crawl/14-cs-elixir.md` —
  syntactic migration pitfalls for Elixir users.
  <https://gleam.run/cheatsheets/gleam-for-elixir-users/>
- `/home/node/Development/ai-workbench/docs/gleam/.crawl/15-cs-erlang.md` —
  syntactic migration pitfalls for Erlang users.
  <https://gleam.run/cheatsheets/gleam-for-erlang-users/>
- `/home/node/Development/ai-workbench/docs/gleam/.crawl/16-cs-rust.md` —
  syntactic migration pitfalls for Rust users.
  <https://gleam.run/cheatsheets/gleam-for-rust-users/>

No Gleam version is stated on any page.

## Related BEAM guidance

See [`../beam/common-mistakes.md`](../beam/common-mistakes.md) for BEAM-side
runtime and ecosystem pitfalls that apply regardless of source language.

## Core guidance

- Gleam enforces `snake_case` for variables, constants, and functions, and
  `PascalCase` for types and variants.
- Conventions and anti-patterns are always-rules; patterns are contextual.
- Prefer explicit, domain-oriented code. Use the type system to make invalid
  states unrepresentable.

## Practical rules

### Imports, naming, and types

- **Avoid unqualified imports of functions and constants.** Use qualified syntax
  (`string.to_graphemes`). Types and constructors may be unqualified if readable.
- **Annotate all module functions** with argument and return types.
- **Module names are singular** in every segment: `app/user`,
  `app/payment/invoice`.
- **Acronyms are single words:** `json`, not `JSON` (otherwise BEAM emits
  `j_s_o_n`).
- **Conversion functions** use `x_to_y` (`json_to_string`). Drop the type prefix
  when the module names the type (`to_string` in `identifier.gleam`). Use
  encoding names (`date_to_rfc3339`) or descriptive names (`round`) when clearer.
- **Fallible functions** get domain names (`parse_json`, `enqueue`). Use `try_`
  only for a result-handling early-return variant; avoid abstract names like
  `monadic_bind`.
- **Do not abbreviate.** Write `capacity`, `continuation`, not `cap`, `cnt`.

### Results, errors, and panics

- **Use `Result` for fallible functions.** Gleam does not use `Option` for
  fallible functions; `Result` removes conversion boilerplate. The error type
  may be `Nil`.
- **Do not panic in fallible functions**, especially in libraries. Panicking may
  be acceptable at the top level of application code.
- **Design descriptive errors.** Variants describe failures in business-domain
  terms and carry useful fields; lower-level errors are fields of higher-level
  errors.

### Project structure

- **Use the core libraries:** `gleam_stdlib`, `gleam_time`, `gleam_json`,
  `gleam_http`, `gleam_erlang`, `gleam_otp`, `gleam_javascript`. Do not
  reimplement them (e.g. use `gleam_time.Timestamp`).
- **Keep tool config in `gleam.toml`** under `tools.$TOOL_NAME`. Avoid
  `my-tool.toml` or `config/my-tool.yaml`.
- **Use the correct source directory:** `src` for app/library code, `test` for
  tests, `dev` for generators/helper scripts.
- **Avoid global namespace pollution.** Gleam inherits the BEAM's global module
  namespace; place modules under a directory matching the package name
  (`src/lustre/` for package `lustre`).
- **Do not trespass** on another package's namespace directory.
- **Avoid fragmented modules.** Large modules are fine; split by business domain,
  not by kind or design pattern. See Evan Czaplicki, "The life of a file"
  (<https://www.youtube.com/watch?v=XpDsk374LDE>). Common in AI-generated code.

### Type modeling

- **Make invalid states impossible.** Use custom types to encode business rules.
- **Replace bools with custom types.** `Bool` lacks context, is easy to confuse,
  cannot be extended to three states, and composes poorly.

### Design patterns

- **Sans-io for API clients/SDKs.** Do not depend on a specific HTTP client.
  Provide request constructors and response parsers. Not the same as taking an
  HTTP-sending function argument.
- **Builder pattern.** Required fields go in the starter function; builder
  functions take and return the builder. Make the type opaque when validation
  or integrity requires it.
- **Avoid category-theory overuse.** Gleam lacks the ergonomics and compiler/
  runtime optimizations to make complex abstractions worthwhile. Solve specific
  problems with specific solutions.

### Matching and FFI

- **Avoid check-then-assert.** Use pattern matching or `result.try`/`result.map`.
- **Match all variants explicitly.** Catch-all patterns disable exhaustiveness
  refactoring assistance.
- **Never use `gleam/dynamic.Dynamic` for FFI types.** Create a precise custom
  type instead.
- **Comment liberally.** Explain what and why, but do not use comments to excuse
  unclear code.

## Review checklist

- [ ] Functions/constants from other modules use qualified syntax.
- [ ] All module functions have full type annotations.
- [ ] Fallible functions return `Result`, not `Option`, and do not panic in libraries.
- [ ] Module names are singular; acronyms are lowercase words.
- [ ] Error variants are business-domain oriented and carry useful data.
- [ ] No abbreviations; names are written in full.
- [ ] Modules grouped by business domain, not kind or design pattern.
- [ ] Package owns its top-level namespace directory.
- [ ] No catch-all patterns where explicit variants would be safer.
- [ ] No check-then-assert sequences.
- [ ] FFI boundaries use precise custom types, not `Dynamic`.
- [ ] `Bool` fields replaced with descriptive custom types where appropriate.
- [ ] Invalid states are unrepresentable.
- [ ] Category-theory abstractions avoided in favour of concrete solutions.

## Implementation checklist

- [ ] Code in `src/`, tests in `test/`, dev tools in `dev/`.
- [ ] Tool config in `gleam.toml` under `tools.$TOOL_NAME`.
- [ ] Depend on core Gleam libraries instead of reimplementing them.
- [ ] HTTP clients/SDKs expose request constructors + response parsers (sans-io).
- [ ] Records with many optional fields provide a builder pipeline.
- [ ] Run `gleam format`, `gleam build`, and `gleam test` before review.

## Validation hooks

- `gleam format --check`
- `gleam build`
- `gleam test`

Most conventions are enforced by review, not tooling. The compiler enforces
`snake_case`/`PascalCase`; the rest rely on human judgment.

## Examples

### Custom types over bools and optional fields

```gleam
// Bad: optional fields permit invalid combinations
pub type Visitor { Visitor(id: Option(Int), email: Option(String)) }

// Good: invalid states are impossible
pub type Visitor { LoggedInUser(id: Int, email: String) Guest }

// Bad: bool lacks context and cannot grow
pub type SchoolPerson { SchoolPerson(name: String, is_student: Bool) }

// Good
pub type SchoolPerson { SchoolPerson(name: String, role: Role) }
pub type Role { Student Teacher }
```

### Descriptive errors

```gleam
// Good
pub type NoteBookError {
  NoteAlreadyExists(path: String)
  NoteCouldNotBeCreated(path: String, reason: simplifile.FileError)
  NoteCouldNotBeRead(path: String, reason: simplifile.FileError)
  NoteInvalidFrontmatter(path: String, reason: tom.ParseError)
}

// Bad: not enough detail
pub type NotesError {
  NoteAlreadyExists NoteCouldNotBeCreated
  NoteCouldNotBeRead NoteInvalidFrontmatter
}

// Bad: designed around dependencies, not the domain
pub type NotesError {
  FileError(path: String, reason: simplifile.FileError)
  TomlError(path: String, reason: tom.ParseError)
}
```

### Builder pattern

```gleam
pub type Button { Button(text: String, colour: String, classes: Set(String)) }

pub fn new(text text: String) -> Button {
  Button(text:, colour: "pink", classes: set.new())
}

pub fn colour(button: Button, value: String) -> Button {
  Button(..button, colour: value)
}

pub fn large(button: Button) -> Button {
  Button(..button, classes: button.classes |> set.delete("small") |> set.insert("large"))
}

button.new(text: "Continue") |> button.colour("green") |> button.large |> button.to_html
```

### Sans-io API client

```gleam
pub fn create_user_request(name: String) -> Request(String) { ... }

pub fn create_user_response(response: Response(String)) -> Result(User, ApiError) { ... }
```

### Avoid check-then-assert

```gleam
// Bad
use <- bool.guard(when: result.is_error(data), return: data)
let assert Ok(value) = data
process(value)

// Good
use value <- result.try(data)
process(value)
```

### Match all variants explicitly

```gleam
// Bad
case role { Student -> handle_student() _ -> handle_teacher() }

// Good
case role { Student -> handle_student() Teacher -> handle_teacher() }
```

### Precise FFI types

```gleam
// Good
pub type Buffer
pub fn byte_size(data: Buffer) -> Int

// Bad
import gleam/dynamic.{type Dynamic}
pub fn byte_size(data: Dynamic) -> Int
```

### Concrete over abstract

```gleam
// Bad
pub fn sum(data: a, monoid: Monoid(a), catamorphism: Catamorphism(a, b)) -> b {
  catamorphism.apply(data, monoid.empty, monoid.append)
}

// Good
pub fn total_cost(costs: List(Int)) -> Int { int.sum(costs) }
```

## Common mistakes

### Anti-patterns

1. **Abbreviations** — ambiguous and hard to read. Fix: write names in full.
2. **Fragmented modules** — premature splitting produces boilerplate and a
   harder API. Keep related code together by business domain. Common in
   AI-generated code.
3. **Panicking in libraries** — `panic`/`let assert` take control from users.
   Fix: return `Result`. Exception: OTP libraries may panic with a suitable
   supervision tree.
4. **Global namespace pollution** — place modules under the package-named
   directory (`src/my_package/`).
5. **Namespace trespassing** — do not place modules in another package's
   directory (e.g. `src/lustre/` unless you maintain `lustre`).
6. **Grouping by design pattern** — avoid `controllers`, `services`, `functors`,
   `monads`. Group by business domain (`app/stock`, `app/billing`).
7. **Check-then-assert** — use pattern matching or `result.try`/`result.map`.
8. **Using `Dynamic` with FFI** — create a precise custom type instead.
9. **Match-all variants** — match each variant explicitly.
10. **Category theory overuse** — prefer concrete, specific solutions.

### Migration pitfalls

The Elixir, Erlang, and Rust cheatsheets are **syntactic-only**; deeper pitfalls
come from the conventions page above.

**From Elixir:** no `nil`; no macros/`use`; no multiple heads/overloading;
single value/function namespace; types enforced (not optional `@spec`); atoms
must be declared; `Result` over exceptions; operators type-split (`>` int,
`>.` float); strict equality; dicts are not maps (no literal, no pattern match);
one module per file; private by default; labelled args are not keyword lists;
cons is `[h, ..t]` not `[h | t]`; `?` not allowed in names.

**From Erlang:** types enforced (not optional `-spec`); no ad-hoc atoms; Gleam
custom types do not compile to Erlang records; no multiple heads; partial
patterns need `let assert`; lists are homogeneous; operators type-strict;
variables are lowercase and shadowing is legal; labelled args are zero-cost and
checked; `pub opaque` is enforced.

**From Rust:** no ownership/borrowing/lifetimes (GC'd, immutable, shared freely);
no `mut` (use shadowing); no traits/trait bounds; `Result`/`Option` are not
covered in the Rust cheatsheet (see
[`result-option-and-errors.md`](result-option-and-errors.md)); Int/Float
operators are distinct; `let assert` is a runtime crash; tuples need `#`; lists
are linked lists; one module per file; member access uses `.`; arithmetic
grouping uses braces; constructors are unqualified; module docs use `////`.

## Strict vs contextual guidance

- **Conventions and anti-patterns** are strict: follow them always.
- **Patterns** are contextual: apply them when they benefit the code.

Always use `Result` for fallible functions and never use `Dynamic` for FFI
types. Use builders, sans-io, or opaque types only when the situation calls for
them.

## Policy decisions for individual repos

Teams may decide:

- Whether `let assert` is permitted outside tests.
- Whether builder types are opaque or public.
- Whether sans-io is required for HTTP clients/SDKs.
- Whether to allow unqualified imports of record constructors.
- How detailed error types must be.

Document repo-specific decisions in a local style guide and reference this doc
as the baseline.

## Related docs

- [`language-fundamentals.md`](language-fundamentals.md)
- [`types-records-and-patterns.md`](types-records-and-patterns.md)
- [`functions-pipelines-and-use.md`](functions-pipelines-and-use.md)
- [`result-option-and-errors.md`](result-option-and-errors.md)
- [`project-structure-and-cli.md`](project-structure-and-cli.md)
- [`gleam-toml-and-targets.md`](gleam-toml-and-targets.md)
- [`externals-and-ffi.md`](externals-and-ffi.md)
- [`package-management-and-publishing.md`](package-management-and-publishing.md)

## Related skills

- `gleam-packages-ffi` — for the FFI/`Dynamic` anti-pattern.
- `gleam-otp-interop` — for the OTP panicking exception.
