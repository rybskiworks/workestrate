# Crawl: conventions-patterns-and-anti-patterns/
- seed_url: https://gleam.run/documentation/conventions-patterns-and-anti-patterns/
- canonical_url: https://gleam.run/documentation/conventions-patterns-and-anti-patterns/
- family: Gleam official docs
- fetch: 200
- gleam_version: not present on page
- feeds_docs: conventions-patterns-antipatterns.md, code-review workflow

## Purpose
Tips and guidance for writing good code in Gleam. The page distinguishes:
- **Conventions** and **anti-patterns**: rules that should be adhered to *always*.
- **Patterns**: to be applied whenever the programmer thinks it would benefit their code.

Gleam enforces `snake_case` for variables, constants, and functions, and
`PascalCase` for types and variants.

## Patterns (each: name + guidance, verbatim where possible)

### Avoid unqualified importing of functions and constants
Always use the qualified syntax for functions and constants defined in other
modules.
```gleam
// Good
import gleam/list
import gleam/string

pub fn reverse(input: String) -> String {
  input
  |> string.to_graphemes
  |> list.reverse
  |> string.concat
}

// Bad
import gleam/list.{reverse}
import gleam/string.{to_graphemes, concat}
```
Types and record constructors may be used with the unqualified syntax,
providing you think it does not make the code more difficult to read.

### Annotate all module functions
All module functions should have annotations for their argument types and for
their return type. Missing return annotation is also "Bad".
```gleam
// Good
fn calculate_total(amounts: List(Int), service_charge: Int) -> Int {
  int.sum(amounts) * service_charge
}

// Bad
fn calculate_total(amounts, service_charge) { ... }

// Bad: missing return annotation
fn calculate_total(amounts: List(Int), service_charge: Int) { ... }
```

### Use result for fallible functions
All functions that can succeed or fail may return a result in Gleam.
Some languages use both the result and the option type for fallible functions,
but Gleam does not. Using results always makes code consistent and removes the
boilerplate that would otherwise be required to convert between result and
option. If there is no extra information to return for failure then the result
error type can be `Nil`.
Panics are not used for fallible functions, especially within libraries.
Panicking may be appropriate at the top level of application code, handling the
result returned by fallible functions.
```gleam
// Good
pub fn first(list: List(a)) -> Result(a, Nil) {
  case list {
    [item, ..] -> Ok(item)
    _ -> Error(Nil)
  }
}

// Bad: returns an option
pub fn first(list: List(a)) -> option.Option(a) { ... }

// Bad: panics on failure
pub fn first(list: List(a)) -> a {
  case list {
    [item, ..] -> item
    _ -> panic as "cannot get first of empty list"
  }
}
```

### Use singular for module names
Module names are singular, not plural. This applies to all segments, not just
the final one.
```gleam
// Good
import app/user
import app/payment/invoice

// Bad
import app/users
import app/payments/invoice
```

### Treat acronyms as single words
Acronyms are always written as if they were a single word.
```gleam
// Good
let json: Json = build_json()

// Bad
let j_s_o_n: JSON = build_j_s_o_n()
```
It may be tempting to ignore this convention and use the name `JSON`, but this
will result in the BEAM code generated from the Gleam code using the name
`j_s_o_n`.

### Conventional conversion function naming
When naming a function that converts from one type to another, use the
convention `x_to_y`.
```gleam
// Good
pub fn json_to_string(data: Json) -> String

// Bad
pub fn json_into_string(data: Json) -> String
pub fn json_as_string(data: Json) -> String
pub fn string_of_json(data: Json) -> String
```
If the module name matches the type name then do not repeat the name of the
type at the start of the function (e.g. in `src/my_app/identifier.gleam` use
`to_string`, not `identifier_to_string`). Functions are used with a module
qualifier, so the name of the module clarifies what the input value is.
If there is a name for the encoding, format, or variant used in the conversion
function, then use that in the name of the function
(e.g. `date_to_rfc3339`, not `date_to_string`).
If there is a more descriptive name for the conversion operation then use that
instead (e.g. `round`, not `float_to_int`).

### Conventional fallible function naming
Functions that return results should be given a name that is appropriate for
the domain and the operation they perform.
```gleam
// Good
pub fn parse_json(input: String) -> Result(Json, ParseError)
pub fn enqueue(job: BackgroundJob) -> Result(Nil, EnqueueError)
```
If the function is a special result-handling version of an existing function
that returns-early when there is an error, then the `try_` prefix can be used,
so long as there is not a more appropriate domain-specific name. Names based on
design patterns or abstract concepts should be avoided.
```gleam
// Good
pub fn try_map(list: List(a), f: fn(a) -> Result(b, e)) -> Result(List(b), e)

// Bad
pub fn monadic_bind(list: List(a), f: fn(a) -> Result(b, e)) -> Result(List(b), e)
```

### Use the core libraries
The Gleam core team maintain several packages that are to be used as a shared
foundation for other Gleam libraries and applications:
- `gleam_stdlib`
- `gleam_time`
- `gleam_json`
- `gleam_http`
- `gleam_erlang`
- `gleam_otp`
- `gleam_javascript`
This shared foundation makes it easier for related Gleam packages to work
together, and helps avoid common problems that the design of the packages guard
against. Do not replicate functionality provided by these packages. e.g. Do not
create a new time type instead of using `gleam_time`'s `Timestamp`.

### Keep development tool config in `gleam.toml`
The `gleam` command line program provides most the functionality that we need
for Gleam development, but there may still be occasions where additional
tooling is desired (e.g. a security scanner, or a licence compliance checker).
If these tools are to be configured via a file, that file should be
`gleam.toml`, with configuration going under the `tools.$TOOL_NAME` key prefix.
```toml
name = "thingy"
version = "1.0.0"

[dependencies]
gleam_stdlib = "<= 1.0.0 and < 2.0.0"

[tools.lustre.dev]
host = "0.0.0.0"

[tools.lustre.build]
minify = true
outdir = "../server/priv/static"
```
Do not use dedicated configuration files such as `my-tool.toml`, or
`config/my-tool.yaml`. Dynamic configuration can be read from environment
variables or provided as command line arguments.

### Use the correct source code directory
Gleam's build tool offers 3 directories for source code: `src`, `dev`, `test`.
Each directory has a different purpose.
- `src` is for code to be included in application or library itself. Code in
  this directory can import modules from `dependencies` and `src/`, but not
  `dev_dependencies`, `dev/`, or `test/`.
- `test` is for code that tests the package, such as automated unit and
  integration tests. Code in this directory can import modules from any
  dependencies and any directory.
- `dev` is for any additional code used in development, such as code
  generators and helper scripts. Code in this directory can import modules from
  any dependencies and any directory.

### Design descriptive errors
When creating an error type, design the variants to describe what the error was
in terms of your business domain. Each variant should hold additional
information about the error instance, to aid debugging or with creation of
helpful error messages.
If the error was caused by a lower-level error, e.g. being unable to load
application data due to failing to read a file, then that lower error can be
one of the fields of the higher error.
```gleam
// Good
pub type NoteBookError {
  NoteAlreadyExists(path: String)
  NoteCouldNotBeCreated(path: String, reason: simplifile.FileError)
  NoteCouldNotBeRead(path: String, reason: simplifile.FileError)
  NoteInvalidFrontmatter(path: String, reason: tom.ParseError)
}

// Bad: Not enough detail
pub type NotesError {
  NoteAlreadyExists
  NoteCouldNotBeCreated
  NoteCouldNotBeRead
  NoteInvalidFrontmatter
}

// Bad: Designed around dependencies, not business domain
pub type NotesError {
  FileError(path: String, reason: simplifile.FileError)
  TomlError(path: String, reason: tom.ParseError)
}
```

### Comment liberally
Comments are a very effective way to make code easier to understand. This is
especially valuable for projects with more than one programmer, or projects
that are expected read and edited over longer periods of time.
Comments can explain both *what* the code does as well as *why* the code does
what it does. Often the reader could determine *what* without the aid of the
comment, but that may not be the case for unfamiliar readers or if the code is
later determined to have a bug, so what it does and what the writer intended it
to do do not match.
Adding comments does not mean the code itself can be written in an unclear way,
and having well written code doesn't mean that comments are not a valuable
addition.

### Make invalid states impossible
Gleam's type system and custom types enable Gleam programmers to precisely
model their domain in their code. Types definitions that sufficiently encode
the business rules can make it impossible to construct invalid data, removing
many types of bugs, and turning the type definitions into documentation for the
business logic.
Example: a website visitor can be a logged-in user (with id + email) or a guest.
```gleam
// Bad: permits invalid states (id set, email unset, etc.)
pub type Visitor {
  Visitor(id: Option(Int), email: Option(String))
}

// Good: invalid states impossible
pub type Visitor {
  LoggedInUser(id: Int, email: String)
  Guest
}
```
Richard Feldman has an excellent talk on this pattern:
https://www.youtube.com/watch?v=IcgmSRJHu_8

### Replace bools with custom types
The bool type can be useful for representing data with 2 possible states,
however, there are some drawbacks:
- `Bool`, `True`, and `False` have no meaning without context, making it easier
  to misunderstand what values represent.
- The bool type will be used for many unrelated pieces of data, so it is
  possible to mistake one bool value for another without a type error to
  prevent the mistake.
- If a third state is required in future then bool can no longer be used, and a
  larger refactoring will be needed. It may be tempting to use a second bool,
  but that results in 4 states, and a third bool results in 8 states.
  Combinations of bools can be especially unclear.
```gleam
// Consider replacing
pub type SchoolPerson {
  SchoolPerson(name: String, is_student: Bool)
}

// With descriptive custom types
pub type SchoolPerson {
  SchoolPerson(name: String, role: Role)
}

pub type Role {
  Student
  Teacher
}
```

### The sans-io pattern
The sans-io pattern is a way of designing API clients, SDKs, and similar
packages so that they do not depend on any particular HTTP client. Instead it
gives responsibility to the library user for sending HTTP requests, etc. With
this pattern the user has full control over HTTP sending, so they can use it on
any target, inside any structure or framework, they can add rate limiting or
retries, or anything else they might need.
To implement the pattern structure your code so that each API action has a pair
of functions: One that constructs a HTTP request, and another that takes a HTTP
response and returns the resulting data.
```gleam
/// Construct a request for the create-user endpoint.
pub fn create_user_request(name: String) -> Request(String) { ... }

/// Parse a response from the create-user endpoint.
pub fn create_user_response(response: Response(String)) -> Result(User, ApiError) { ... }
```
This is not the same as taking a HTTP-sending function as an argument. That
design means that only HTTP clients that conform to that type can be used,
which is very limiting. Most notably, it would mean that the library cannot be
used on both the Erlang and the JavaScript targets, as one will be using a
promise type, while the other will not.

### The builder pattern
The builder pattern is a flexible way to create records with multiple optional
fields, often used for configuration.
```gleam
// Usage
button.new(text: "Continue")
|> button.colour("green")
|> button.large
|> button.to_html
```
Any required fields can be taken as arguments by the function that starts the
builder pipeline, the remaining fields being set to default values.
```gleam
pub type Button {
  Button(text: String, colour: String, classes: Set(String))
}

pub fn new(text text: String) -> Button {
  Button(text:, colour: "pink", classes: set.new())
}
```
In this example the `Button` type is not opaque, so the record can be
constructed and manipulated directly. Other times you may wish to make it
opaque and force the builder functions to be used, which could be for
validation or data integrity reasons.
Builder functions take the builder value as an argument and return a new version
of it with one or more fields changed.
```gleam
pub fn colour(button: Button, value: String) -> Button {
  Button(..button, colour: value)
}
```
Builder functions do not need to only set a field to a value taken as an
argument, they can perform any logic to construct the data. Often it is helpful
to have convenience functions for common uses, and possibly have the users of
the code construct the builder record directly if they need full control.
```gleam
pub fn large(button: Button) -> Button {
  let classes = button.classes |> set.delete("small") |> set.insert("large")
  Button(..button, classes:)
}
```

## Anti-patterns (each: name + why it's bad + fix, verbatim where possible)

### Abbreviations
**Why bad:** Using shortened names can save a few keystrokes when typing, but
they greatly hinder code reading and understanding. Abbreviations are ambiguous,
so the reader has to guess what they are short for, and often they will get it
wrong. This is especially likely if you are working with people with different
backgrounds or from different cultures.
**Fix:** Always write names in full.
```gleam
// Bad
let cap = 5
let off = 0
let cnt = proc_dat(ss)

// Good
let capacity = 5
let offset = 0
let continuation = process_data(session)
```

### Fragmented modules
**Why bad:** Do not prematurely split up modules into multiple smaller modules,
and do not view large modules as a problem. Instead focus on the business
domain and making the best API for the users of the code. An API that is split
over many modules is harder to understand and requires more boilerplate to use
than one well designed module, and it becomes more challenging to hide internal
implementation details when they have to be exposed for other modules to use.
Multiple modules also encourages the creation of a much larger number of
functions and types, with overlapping functionality between modules. The larger
the API the more difficult it is to understand and to work with. The best APIs
are small and focused.
**Fix:** If you are having trouble with import cycles, or if multiple modules
need to be imported to perform a simple task with your code, then it may be a
sign that you have split up code that should be a single module.
```gleam
// Bad
import my_library/client
import my_library/config
import my_library/decode
import my_library/error
import my_library/parser
import my_library/types

// Good
import my_library
```
This anti-pattern is especially common with AI-generated code. If you are using
AI in your coding pay extra attention to this rule.
Reference: Evan Czaplicki's talk "The life of a file"
https://www.youtube.com/watch?v=XpDsk374LDE

### Panicking in libraries
**Why bad:** Libraries must not panic, so they should not use `panic` or
`let assert`. Panicking instead of returning a result takes control away from
the users of the library, preventing them from being able to handle errors. A
library does not know the context in which it is used, so it is impossible for
the author of a library to know if it acceptable to panic, so they never can.
**Fix:** Return a `Result` instead.
**Exception:** The one exception to this rule is for libraries *about* OTP, the
BEAM application framework. OTP has non-local handling through supervision
trees, so there may be some circumstances in which it is appropriate to panic,
providing they have a suitably designed supervision tree. The library author
would benefit from having a strong understanding of OTP system to identify
when this is a good option.

### Global namespace pollution
**Why bad:** Gleam has a global module namespace, a property inherited from the
BEAM ecosystem. If two packages each define a module with the same name then a
project adding both packages as dependencies will fail to compile.
**Fix:** To avoid this problem packages should define their own namespace by
placing their modules within a uniquely named directory. This name should match
the name of the package. For example, the package `lustre` places its modules
in `src/lustre/`.
```
# Good
src/
├── my_package.gleam
└── my_package/
    ├── distribution.gleam
    └── inventory.gleam

# Bad
src/
├── distribution.gleam
├── inventory.gleam
└── my_package.gleam
```

### Namespace trespassing
**Why bad:** Other packages should not place their modules within a top-level
directory that belongs to a different package. Trespassing in someone else's
module namespace can result in compilation errors due to module collisions, and
confusing code where it is unclear where modules come from.
**Fix:** Do not place your modules within another package's namespace directory
(e.g. `src/lustre/`), even if you are making a package intended to be used with
the `lustre` package. The maintainers of the `lustre` package may choose to
reuse the `src/lustre/` directory in other Lustre-related packages that they
also own.

### Grouping by design pattern
**Why bad:** When splitting your code into modules always design the boundaries
in your system around your business domain and what would be the best API for
the users of the code. This means never using design patterns or abstract code
constructs as the basis for boundaries within the project.
**Fix:** Group by business domain, not by kind/category theory/design pattern.
```gleam
// Bad: kind grouping
import app/constants
import app/functions
import app/types
import app/utilities

// Bad: category theory grouping
import app/functors
import app/monads
import app/monoids
import app/semigroups

// Bad: design pattern grouping
import app/controllers/user_controller
import app/decorator/user_decorator
import app/model/user_mode
import app/services/user_service
import app/views/user_view

// Good: business domain grouping
import app/stock
import app/billing
```

### Check-then-assert
**Why bad:** Check-then-assert is a pattern common in procedural languages
where one performs a check that a value is in some desired state, and then
performs some action afterwards with the knowledge that it is in the desired
state. This is an anti-pattern in functional languages like Gleam, and it
should never be done.
**Fix:** Instead use pattern matching or functions such as `result.try` and
`result.map` to handle data that could be in some other state. These patterns
take advantage of the type system to ensure that there are no mistakes coming
from a disconnect between the checking and the using, and they can have
performance benefits too.
```gleam
// Bad: check then assert
case result.is_ok(data) {
  True -> {
    let assert Ok(value) = data
    process(value)
  }
  False -> data
}

// Bad: check then assert with `use`
use <- bool.guard(when: result.is_error(data), return: data)
let assert Ok(value) = data
process(data)

// Good: pattern matching
case data {
  Ok(value) -> process(value)
  Error(e) -> Error(e)
}

// Good: combinators
data |> result.try(process)

// Good: combinators with `use`
use value <- result.try(value)
process(value)
```

### Using dynamic with FFI
**Why bad:** When using code written in other languages there will be some
arguments and return values that cannot be represented with the Gleam type
system. Never use the `gleam/dynamic` module's `Dynamic` type to represent
these types. The dynamic type represents *any* type of data, meaning it is
valid to pass any value at all to that function, which is not correct and will
cause runtime errors.
**Fix:** Instead create a new type that represents exactly the expected type.
```gleam
// Good
pub type Buffer

pub fn byte_size(data: Buffer) -> Int

// Bad
import gleam/dynamic.{type Dynamic}

pub fn byte_size(data: Dynamic) -> Int
```

### Match all variants
**Why bad:** Gleam's exhaustiveness checking of case expressions ensures that
when you change your data model all your code is updated appropriately before
it compiled again. If the final pattern of a case expression is one that
matches any value, then this refactoring assistance is effectively disabled for
new additions, and the compiler will not be able to help you update your code.
This makes it easy to introduce bugs due to old logic no longer being correct.
**Fix:** Avoid catch-all patterns where possible; match each variant
explicitly.
```gleam
// Bad: assumes all other variants are teachers
case role {
  Student -> handle_student()
  _ -> handle_teacher()
}

// Good: cannot silently become incorrect
case role {
  Student -> handle_student()
  Teacher -> handle_teacher()
}
```

### Category theory overuse
**Why bad:** Avoid creation of complex category theory based abstractions. Gleam
does not have the ergonomics to make these abstractions easy to work with, nor
the compiler and runtime optimisations required to erase the significant
runtime overhead they introduce. Complex abstractions typically introduce a
high cognitive overhead to the code, running contrary to Gleam's simple,
concrete, and approachable programming style.
**Fix:** Solve specific problems with specific solutions.
```gleam
// Bad: abstract style
pub fn sum(
  data: a,
  monoid: Monoid(a),
  catamorphism: Catamorphism(a, b),
) -> b {
  catamorphism.apply(data, monoid.empty, monoid.append)
}

// Good: concrete style
pub fn total_cost(costs: List(Int)) -> Int {
  int.sum(costs)
}
```

## Verbatim quotes (the key rules)
- "Conventions and anti-patterns are rules that should be adhered to always,
  while patterns are to applied whenever the programmer thinks it would benefit
  their code."
- "Gleam enforces `snake_case` for variables, constants, and functions, and
  `PascalCase` for types and variants."
- "Always used the qualified syntax for functions and constants defined in
  other modules."
- "All module functions should have annotations for their argument types and for
  their return type."
- "Using results always makes code consistent and removes the boilerplate that
  would otherwise be required to convert between result and option. If there is
  no extra information to return for failure then the result error type can be
  `Nil`."
- "Panics are not used for fallible functions, especially within libraries."
- "Module names are singular, not plural. This applies to all segments, not just
  the final one."
- "Acronyms are always written as if they were a single word."
- "When naming a function that converts from one type to another, use the
  convention `x_to_y`."
- "Names based on design patterns or abstract concepts should be avoided."
- "Do not replicate functionality provided by these packages."
- "If these tools are to be configured via a file, that file should be
  `gleam.toml`, with configuration going under the `tools.$TOOL_NAME` key
  prefix."
- "Do not use dedicated configuration files such as `my-tool.toml`, or
  `config/my-tool.yaml`."
- "Libraries must not panic, so they should not use `panic` or `let assert`."
- "Never use the `gleam/dynamic` module's `Dynamic` type to represent these
  types."
- "Avoid catch-all patterns where possible."
- "Avoid creation of complex category theory based abstractions."
- "Solve specific problems with specific solutions."
- "This anti-pattern is especially common with AI-generated code. If you are
  using AI in your coding pay extra attention to this rule." (fragmented modules)

## Version notes
- No Gleam version stated on the page.
- Page copyright footer: © 2026 Louis Pilfold.
- A commented-out "## OTP anti-patterns" section exists in the source HTML
  (Processes as state, Unsupervised processes, Large messages, Organising code
  with processes) — all marked TODO / not yet published.

## Discovered links

### Relevant (crawl later)
- https://gleam.run/documentation/ — parent documentation index
- https://hexdocs.pm/gleam_stdlib — core library
- https://hexdocs.pm/gleam_time — core library
- https://hexdocs.pm/gleam_json — core library
- https://hexdocs.pm/gleam_http — core library
- https://hexdocs.pm/gleam_erlang — core library
- https://hexdocs.pm/gleam_otp — core library
- https://hexdocs.pm/gleam_javascript — core library
- https://www.youtube.com/watch?v=IcgmSRJHu_8 — Richard Feldman: "Making Invalid States Unrepresentable"
- https://www.youtube.com/watch?v=XpDsk374LDE — Evan Czaplicki: "The life of a file"
- https://tour.gleam.run — language tour
- https://packages.gleam.run/ — package index

### Skipped
- https://gleam.run/news
- https://gleam.run/community
- https://gleam.run/sponsor
- https://gleam.run/install
- https://gleam.run/roadmap
- https://gleam.run/case-studies
- https://gleam.run/feed.xml (atom feed)
- https://playground.gleam.run
- https://gleamweekly.com/
- https://discord.gg/Fm8Pwmy
- https://shop.gleam.run/en-gbp
- https://github.com/gleam-lang
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md
- https://plausible.io/js/plausible.js (analytics)
