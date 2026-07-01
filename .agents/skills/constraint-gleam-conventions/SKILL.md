---
name: constraint-gleam-conventions
description: |
  Enforces Gleam conventions and anti-patterns during code execution — qualified
  imports, annotated functions, sans-IO, descriptive error names, no namespace
  trespassing, no fragmented modules, and concrete-over-abstract design. Load when
  writing or reviewing Gleam code. Does NOT cover the Result/Option error model
  (see constraint-gleam-result).
metadata:
  org.kind: constraint
---

# Constraint: Gleam Conventions and Anti-Patterns

This constraint enforces the official Gleam conventions and anti-patterns
(always-rules). The compiler enforces `snake_case`/`PascalCase`; the rest rely
on review. Violations produce boilerplate, harder APIs, or namespace collisions.

## Triggers

Load this skill when:

- Writing or reviewing Gleam imports, module structure, or naming.
- Designing API clients/SDKs, builders, or error types.
- Organizing modules or placing files in `src/`.

## Rules

1. Avoid unqualified imports of functions and constants — use qualified syntax
   (`string.to_graphemes`). Types and constructors may be unqualified if readable.
2. Annotate ALL module functions with argument and return types.
3. Module names are singular in every segment (`app/user`,
   `app/payment/invoice`); acronyms are single lowercase words (`json`, not
   `JSON`).
4. Conversion functions use `x_to_y` (`json_to_string`); drop the type prefix when
   the module names the type (`to_string` in `identifier.gleam`); fallible
   functions get domain names (`parse_json`); use `try_` only for a result-
   handling early-return variant.
5. Do NOT abbreviate — write `capacity`, `continuation`, not `cap`, `cnt`.
6. Group modules by business domain, NOT by kind or design pattern (no
   `controllers`/`services`/`functors`/`monads`).
7. Avoid global namespace pollution — place modules under a directory matching
   the package name (`src/lustre/` for package `lustre`); do NOT trespass on
   another package's namespace directory.
8. Avoid fragmented modules — large modules are fine; split by business domain,
   not by pattern (common in AI-generated code).
9. Use the core libraries (`gleam_stdlib`, `gleam_time`, `gleam_json`,
   `gleam_http`, `gleam_erlang`, `gleam_otp`, `gleam_javascript`); do not
   reimplement them.
10. Keep tool config in `gleam.toml` under `tools.$TOOL_NAME` (avoid
    `my-tool.toml`); code in `src/`, tests in `test/`, dev tools in `dev/`.
11. Avoid category-theory overuse — Gleam lacks the ergonomics for complex
    abstractions; solve specific problems with specific solutions.
12. Sans-IO for API clients/SDKs: provide request constructors and response
    parsers, not a dependency on a specific HTTP client.

## References

- Operational skill: `gleam-language`.
- Docs: `docs/gleam/conventions-patterns-antipatterns.md`.

## Out of scope

- The `Result`/`Option` error model and crash boundaries — see
  `constraint-gleam-result`.
- FFI/`Dynamic` boundaries — see `gleam-packages-ffi`.
- OTP actors/supervision — see `gleam-otp-interop`.

## Violation examples

### Unqualified import of a function

```gleam
// FORBIDDEN: unqualified function import
import gleam/string.{to_graphemes}
```

Correct: `import gleam/string` and use `string.to_graphemes(...)`.

### Abbreviated name

```gleam
// FORBIDDEN: ambiguous abbreviation
pub fn cap(list: List(Int)) -> Int { ... }
```

Correct: `pub fn capacity(list: List(Int)) -> Int { ... }`.

### Namespace trespassing

```gleam
// FORBIDDEN: placing modules in another package's directory
// src/lustre/element.gleam  (unless you maintain `lustre`)
```

Correct: place modules under your own package directory
(`src/my_app/element.gleam`).

### Grouping by design pattern

```gleam
// FORBIDDEN: grouping by kind, not domain
// src/controllers/, src/services/, src/functors/
```

Correct: group by business domain (`src/app/stock/`, `src/app/billing/`).

## How to check

```bash
gleam format --check   # formatting (compiler enforces snake_case/PascalCase)
gleam check            # type-check (enforces annotations are valid)
gleam build            # full compile
```

Manual review (most conventions are review-enforced, not tool-enforced):

- Functions/constants use qualified imports; all functions annotated.
- Module names singular; acronyms lowercase words; no abbreviations.
- Modules grouped by business domain; package owns its namespace directory.
- Core libraries used (not reimplemented); tool config in `gleam.toml`.
- No category-theory overuse; HTTP clients are sans-IO.
