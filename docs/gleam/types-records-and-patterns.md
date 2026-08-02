# Gleam types, records, and pattern matching

## Purpose
Gleam's data-modeling core: custom types, records, generics, opaque types, tuples,
lists, and the `case` expression with its pattern-matching forms and guards. This is
where domain modeling happens; pair with `result-option-and-errors` for the
`Result`/`Option` types specifically.

## Sources used
- Crawl: `docs/gleam/.crawl/02-tour-everything.md` — https://tour.gleam.run/everything/
  (sections: Collections; Custom types, records, generics, opaque; Pattern matching
  & case; Bit arrays; Strict Rules; Verbatim quotes)

## Related BEAM guidance
- `docs/beam/overview.md` — Gleam custom variants compile to tagged tuples on the
  BEAM; lists compile to Erlang lists; tuples to Erlang tuples. The modeling
  concepts here are Gleam-specific, but the runtime representations are shared BEAM
  primitives (see `erlang-interop` for the mapping table).

## Core guidance

### Custom types
- Defined with `type` + name + a constructor per VARIANT. Type name and constructor
  names start with uppercase.
```gleam
pub type Season {
  Spring
  Summer
  Autumn
  Winter
}
```
- Variants are pattern matched with `case`.

### Records
- A variant that holds data is a record. Fields can be given labels (optionally used
  when constructing, like function argument labels).
- A single-variant custom type is Gleam's equivalent of a struct/object; the variant
  is often named the same as the type (not required).
```gleam
pub type Person {
  Person(name: String, age: Int, needs_glasses: Bool)
}
let amy = Person("Amy", 26, True)
let jared = Person(name: "Jared", age: 31, needs_glasses: True)
```
- Record ACCESSORS: `record.field_label` gets a contained value. Accessible without a
  `case` ONLY when the field has the same name, position, and type across ALL
  variants. Other fields require the compiler to know the variant (e.g. after a
  `case` match).
- Record PATTERN MATCHING: extract multiple fields into variables. `let` can only
  destructure single-variant types (or when the variant is known after a `case`).
  Use `_` or `..` to discard unneeded fields.
```gleam
case fish {
  Starfish(_, favourite_colour) -> ...
  Jellyfish(name, ..) -> ...
}
let IceCream(flavour) = ice_cream   // single-variant: `let` ok
```
- Record UPDATES: create a new record from an existing one with some fields changed
  (immutable — original unchanged):
```gleam
let teacher2 = Teacher(..teacher1, subject: "PE", room: 6)
```

### Generic custom types
- Custom types take type parameters:
```gleam
pub type Option(inner) {
  Some(inner)
  None
}
```
  `gleam/option` defines this for real. A type variable stands for one specific
  type per call site (NOT an `any` type).

### Type aliases
- `pub type Number = Int` — a different NAME for the same type (does NOT make a new
  type). Type names always start with a capital letter. "Use RARELY: aliasing
  obscures the type and gives none of the safety of a custom type."

### Opaque types
- `pub opaque type` — the type itself is public but its constructors are PRIVATE
  (only the defining module can construct or pattern match on it). Enables SMART
  CONSTRUCTORS that enforce invariants:
```gleam
pub opaque type PositiveInt {
  PositiveInt(inner: Int)
}
pub fn new(i: Int) -> PositiveInt {
  case i >= 0 { True -> PositiveInt(i); False -> PositiveInt(0) }
}
```

### Tuples
- Combine multiple values of different types. Generic: `#(1, "Hi!")` has type
  `#(Int, String)`.
- Access without pattern matching: `some_tuple.0` (first), `some_tuple.1` (second).
- Pattern match / destructure: `let #(a, _, _) = triple`.
- Most commonly used to return 2-3 values from a function; a custom type is often
  clearer.

### Lists
- `List` is a generic type: `List(Int)`, `List(String)`.
- IMMUTABLE singly-linked lists. Efficient to add/remove from the FRONT. Counting
  length / indexing other positions is expensive and rarely done.
- Literal: `[1, 2, 3]`. Immutably prepend with spread: `[-1, 0, ..ints]`. All
  elements must be the same type. Original list is unchanged.

### `case` expressions
- The most common flow control. Performs PATTERN MATCHING. EXHAUSTIVENESS CHECKING:
  patterns must cover all possible values (compiler enforces; missing or redundant
  patterns are errors).
```gleam
case x {
  0 -> "Zero"
  1 -> "One"
  _ -> "Other"
}
```
- Variable patterns bind the matched value: `other -> "It is " <> int.to_string(other)`.
- Multiple subjects: `case x, y { 0, 0 -> ...; 0, _ -> ...; _, _ -> ... }` — same
  number of patterns as subjects.
- Alternative patterns with `|`: `2 | 4 | 6 | 8 -> "even"`. If a pattern defines a
  variable, ALL alternatives for that clause must define a variable with the same
  name and type. "Currently it is not possible to have nested alternative patterns,
  so the pattern `[1 | 2 | 3]` is not valid."
- Pattern aliases with `as`: `[_, ..] as first` matches any non-empty list and binds
  that whole list to `first`.

### String patterns
- `<>` matches strings with a specific prefix: `"Hello, " <> name` matches any
  string starting with `"Hello, "` and binds the rest to `name`.

### List patterns
- Match on specific lengths: `[]` (empty), `[_]` (exactly one element).
- Spread `..` matches the rest: `[1, ..]` (starts with 1), `[_, _, ..]` (at least
  two elements), `[first, ..rest]` (head + tail).

### Tuple patterns
- `let #(a, _, _) = triple` destructuring; also usable in `case`.

### Bit arrays
- Represent a sequence of 1s and 0s; convenient syntax for binary data.
- Literal: `<<3>>` (8-bit int; == `<<3:size(8)>>`). `<<6147:size(16)>>` (16-bit).
  UTF8 segment: `<<"Hello, Joe!":utf8>>`.
- Concatenation: `<<first:bits, second:bits>>`.
- Each segment can take options; multiple options separated by dashes:
  `x:unsigned-little-size(2)`.
- LIMITED support when compiling to JavaScript (not all options available; full
  support planned).

### Guards
- `if` adds a GUARD to a case pattern — an expression that must evaluate to `True`
  for the clause to match:
```gleam
case numbers {
  [first, ..] if first > limit -> first
  [_, ..rest] -> get_first_larger(rest, limit)
  [] -> 0
}
```
- "Guard expressions cannot contain function calls, case expressions, or blocks."

### Recursion (related to case)
- Gleam has NO loops; iteration is via recursion. A recursive function needs >= 1
  base case and >= 1 recursive case.
- Tail call optimisation: if a function call is the last thing a function does, the
  stack frame is reused. Rewrite with an ACCUMULATOR to make recursion
  tail-recursive; hide the accumulator behind a public wrapper calling a private
  recursive helper.
- List recursion idiom: `[first, ..rest]` + `[]` patterns to walk a list.

## Practical rules
- Model domains with custom types that make invalid states unrepresentable (e.g.
  separate variants instead of `Option` fields).
- Avoid catch-all `_` patterns where you can match each variant explicitly —
  exhaustiveness checking then helps when variants are added.
- Prefer opaque types with smart constructors when invariants must be enforced.
- Use record update (`..record, field: value`) for immutable changes; never mutate.
- Use tuples for 2-3 value return; prefer custom types for richer data.
- Make recursive helpers tail-recursive via an accumulator.

## Review checklist
- [ ] Are custom types modeling the domain (not just bags of `Option` fields)?
- [ ] Are `case` patterns exhaustive and free of unnecessary catch-alls?
- [ ] Are guards free of function calls / case / blocks?
- [ ] Are opaque types used where invariants matter?
- [ ] Is recursion tail-recursive where lists could be large?

## Implementation checklist
- [ ] All variants of a custom type are intentional; no dead variants.
- [ ] Record accessors relied upon are shared across all variants (same name/pos/type).
- [ ] Bit-array segment options are valid on the target (JS has limited support).

## Validation hooks
- `gleam build` — enforces exhaustiveness and type checking of all patterns.
- `gleam test` — exercises pattern branches.

## Examples
```gleam
pub type Visitor {
  LoggedInUser(id: Int, email: String)
  Guest
}

pub fn label(v: Visitor) -> String {
  case v {
    LoggedInUser(id:, ..) -> "user-" <> int.to_string(id)
    Guest -> "guest"
  }
}
```

## Common mistakes
- Using a catch-all `_` that silently swallows new variants.
- Putting function calls inside a guard (not allowed).
- Using nested alternative patterns like `[1 | 2 | 3]` (invalid).
- Mutating a record in place — use record update syntax instead.

## Strict vs contextual guidance
- Strict: exhaustiveness checking, immutability, uppercase type/constructor names,
  no nested alternative patterns, guards cannot call functions.
- Contextual: opaque vs transparent types, tuple vs custom type for returns,
  accumulator-based tail recursion vs direct recursion.

## Policy decisions for individual repos
- Prefer opaque types for all public data, or only where invariants exist?
- Maximum tuple arity permitted before requiring a custom type?

## Related docs
- `overview`
- `language-fundamentals`
- `functions-pipelines-and-use`
- `result-option-and-errors`
- `conventions-patterns-antipatterns`

## Related skills
- `gleam-language`
- `gleam-otp-interop`
- `gleam-packages-ffi`
