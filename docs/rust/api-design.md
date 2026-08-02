# Rust API design guidelines

## Purpose

This document is generic, repo-independent guidance for designing public Rust APIs. A public API is any `pub` item a downstream crate can import. The rules are derived from the official Rust API Guidelines (https://rust-lang.github.io/api-guidelines/) and help future AI coding agents write idiomatic, composable libraries.

The Rust API Guidelines are advisory, authored largely by the Rust library team: "These are only guidelines, some more firm than others … should not in any way be considered a mandate." They contain 54 recommendations across 11 categories, each prefixed with `C-` for "Checklist". This document maps those official recommendations to practical rules, examples, and checklists.

## Sources used

- https://rust-lang.github.io/api-guidelines/
- https://rust-lang.github.io/api-guidelines/about.html
- https://rust-lang.github.io/api-guidelines/checklist.html
- https://rust-lang.github.io/api-guidelines/naming.html
- https://rust-lang.github.io/api-guidelines/interoperability.html
- https://rust-lang.github.io/api-guidelines/macros.html
- https://rust-lang.github.io/api-guidelines/documentation.html (cross-reference; detailed in `docs/rust/documentation-guidelines.md`)
- https://rust-lang.github.io/api-guidelines/predictability.html
- https://rust-lang.github.io/api-guidelines/flexibility.html
- https://rust-lang.github.io/api-guidelines/type-safety.html
- https://rust-lang.github.io/api-guidelines/dependability.html
- https://rust-lang.github.io/api-guidelines/debuggability.html
- https://rust-lang.github.io/api-guidelines/future-proofing.html
- https://rust-lang.github.io/api-guidelines/necessities.html

Additional sources for corrections and ecosystem conventions:

- https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md (RFC 1105, canonical definition of breaking changes)
- https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive_attribute (the `#[non_exhaustive]` language feature)
- Clippy lints `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` (for the "no unwrap/expect/panic in public APIs" rule, which is not a Rust API Guidelines checklist item)

## Core guidance

A public API is any `pub` item a downstream crate can import. The authoritative summary is the Rust API Guidelines checklist at https://rust-lang.github.io/api-guidelines/checklist.html; run through it before any crate release.

The API surface should communicate intent through types: make invalid states unrepresentable, expose conversions through predictable names, and derive or implement the standard traits that enable generic code. Treat mechanical items (naming, common traits, additive features, `Debug`) as strict defaults and architectural items (builders, sealed traits, newtype hiding) as contextual choices.

## Practical rules

### Naming

Source: https://rust-lang.github.io/api-guidelines/naming.html

#### C-CASE: Casing conforms to RFC 430

| Item | Convention |
|------|------------|
| Modules | `snake_case` |
| Types, traits, enum variants | `UpperCamelCase` |
| Functions, methods, locals, fields | `snake_case` |
| Statics, constants | `SCREAMING_SNAKE_CASE` |
| Macros | `snake_case!` |
| General constructors | `new` or `with_more_details` |
| Conversion constructors | `from_some_other_type` |
| Type parameters | concise `UpperCamelCase`, usually `T` |
| Lifetimes | short lowercase, usually `'a`, `'de`, `'src` |

Rules:

- In `UpperCamelCase`, acronyms count as one word: `Uuid`, not `UUID`; `Usize`, not `USize`; `Stdin`, not `StdIn`.
- A "word" should never be a single letter unless it is the last word: `btree_map`, not `b_tree_map`, but `PI_2`, not `PI2`.
- Do not use `-rs` or `-rust` as a crate-name prefix or suffix.

```rust
pub mod request_builder;

pub struct HttpRequest;
pub enum Method { Get, Post }

pub fn send_message() {}
pub const MAX_RETRIES: usize = 3;

macro_rules! log_info { () => {} }
```

#### C-CONV: Ad-hoc conversions follow `as_` / `to_` / `into_`

| Prefix | Cost | Pattern |
|--------|------|---------|
| `as_` | Free | `borrowed -> borrowed` |
| `to_` | Expensive | `borrowed -> borrowed`; `borrowed -> owned` (non-`Copy`); `owned -> owned` (`Copy`) |
| `into_` | Variable | `owned -> owned` (non-`Copy`) |

`as_` and `into_` decrease abstraction; `to_` stays at the same abstraction but does work. Wrappers around a single value expose `into_inner()`. If `mut` is part of the return type, it appears as in the type: `as_mut_slice(&mut self) -> &mut [T]`, not `as_slice_mut`.

Examples: `str::as_bytes`, `Path::to_str`, `String::into_bytes`, `BufReader::into_inner`, `Result::as_ref`, `slice::to_vec`.

```rust
impl Container {
    pub fn as_bytes(&self) -> &[u8] { ... }
    pub fn to_vec(&self) -> Vec<u8> { ... }
    pub fn into_inner(self) -> Vec<u8> { ... }
}
```

#### C-GETTER: Getter names follow Rust convention

Do not use the `get_` prefix. Name the getter after the field; mutable accessors take `_mut`.

```rust
impl Pair {
    pub fn first(&self) -> &str { &self.first }
    pub fn first_mut(&mut self) -> &mut String { &mut self.first }
}
```

Reserve `get_` for a single obvious thing that requires runtime validation:

```rust
fn get(&self, index: K) -> Option<&V>;
fn get_mut(&mut self, index: K) -> Option<&mut V>;
unsafe fn get_unchecked(&self, index: K) -> &V;
```

A getter that returns a view (e.g. `TempDir::path`) is distinct from a conversion that transfers responsibility (e.g. `TempDir::into_path`).

#### C-ITER: Collections expose `iter`, `iter_mut`, and `into_iter`

Methods on conceptually homogeneous collections follow RFC 199:

```rust
fn iter(&self) -> Iter<'_>;
fn iter_mut(&mut self) -> IterMut<'_>;
fn into_iter(self) -> IntoIter;
```

This applies to methods, not free functions.

#### C-ITER-TY: Iterator type names match the methods

`iter()` returns `Iter`, `iter_mut()` returns `IterMut`, `into_iter()` returns `IntoIter`. Prefix the type with the owning module in public signatures: `vec::IntoIter<T>`.

```rust
impl<T> Queue<T> {
    pub fn iter(&self) -> QueueIter<'_, T> { ... }
    pub fn iter_mut(&mut self) -> QueueIterMut<'_, T> { ... }
}

impl<T> IntoIterator for Queue<T> {
    type Item = T;
    type IntoIter = QueueIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { ... }
}
```

#### C-FEATURE: Feature names are free of placeholder words

Name a feature `abc`, never `use-abc` or `with-abc`. For optional std support, use `default = ["std"]` and `std = []`, not `use-std`. Cargo requires features to be additive, so `no-abc` is practically never correct.

```toml
[features]
default = ["std"]
std = []
serde = ["dep:serde"]
```

#### C-WORD-ORDER: Names use a consistent word order

Be consistent within the crate and with similar standard library types. The standard library uses verb-object-error (`ParseIntError`, `ParseBoolError`, `JoinPathsError`), so prefer `ParseAddrError` over `AddrParseError`.

### Interoperability

Source: https://rust-lang.github.io/api-guidelines/interoperability.html

#### C-COMMON-TRAITS: Types eagerly implement common traits

The orphan rule prevents downstream crates from implementing standard traits for your types, so implement them eagerly: `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`. It is common for `new` and `Default` to behave identically.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Point { x: f64, y: f64 }
```

#### C-CONV-TRAITS: Conversions use `From`, `TryFrom`, `AsRef`, `AsMut`

Implement `From`, `TryFrom`, `AsRef`, and `AsMut`. Never implement `Into` or `TryInto` directly; they have blanket implementations. `From` must not fail; use `TryFrom` when it can fail.

```rust
impl From<u8> for u16 { ... }              // lossless
impl TryFrom<u32> for u16 { ... }         // fallible
```

#### C-COLLECT: Collections implement `FromIterator` and `Extend`

This lets consumers use `collect()`, `partition()`, and `unzip()`.

```rust
impl<T> FromIterator<T> for Queue<T> { ... }
impl<T> Extend<T> for Queue<T> { ... }
```

#### C-SERDE: Data structures implement `Serialize`/`Deserialize`, feature-gated

Place serde support behind a feature named exactly `serde`. Do not use `serde_impls` or `serde_serialization`.

```toml
[dependencies]
serde = { version = "1", optional = true }

[features]
serde = ["dep:serde"]
```

```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config { ... }
```

#### C-SEND-SYNC: Types are `Send` and `Sync` where possible

These are auto-derived in most cases. Vigilance is needed for raw-pointer types. Assert with compile-time test functions:

```rust
#[test]
fn assert_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<MyType>();
    assert_sync::<MyType>();
}
```

#### C-GOOD-ERR: Error types are meaningful and well-behaved

Any `E` in a public `Result<T, E>` must implement `std::error::Error` and be `Send + Sync`. Conventionally `Error + Send + Sync + 'static` is most useful because `'static` enables `downcast_ref`. Never use `()` as an error type.

`Display` messages should be lowercase, concise, and without trailing punctuation. Do not implement `Error::description()` (deprecated in 1.42) or `Error::cause()` (deprecated in 1.33); use `Display` and `Error::source()`. See the `thiserror` example below.

#### C-NUM-FMT: Binary number types provide `UpperHex`/`LowerHex`/`Octal`/`Binary`

Bitflag-like types should implement the relevant formatting traits.

```rust
impl fmt::Binary for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.bits, f)
    }
}
```

#### C-RW-VALUE: Generic reader/writer functions take `R: Read` / `W: Write` by value

The standard library provides `impl Read for &mut R`, so callers pass `&mut f` to retain ownership.

```rust
pub fn copy<R: std::io::Read, W: std::io::Write>(reader: R, writer: W) -> std::io::Result<()> { ... }
```

### Macros

Source: https://rust-lang.github.io/api-guidelines/macros.html

There is no hygiene or naming-collision checklist item in the official guidelines. Apply the five macro recommendations below.

#### C-EVOCATIVE: Input syntax is evocative of the output

Use keywords and punctuation in macro input that resemble the expanded code.

#### C-MACRO-ATTR: Item macros compose well with attributes

Support adding attributes to any output item so the result works with derive macros.

#### C-ANYWHERE: Item macros work anywhere items are allowed

Macros that emit items must work at module scope and inside function scope. Test both.

#### C-MACRO-VIS: Item macros support visibility specifiers

Emit items as private by default and `pub` when specified.

#### C-MACRO-TY: Type fragments are flexible

A `$t:ty` fragment must accept primitives, relative/absolute/upward paths, and generics.

### Predictability

Source: https://rust-lang.github.io/api-guidelines/predictability.html

#### C-SMART-PTR: Smart pointers do not add inherent methods

Place such functions as associated functions, not inherent `self` methods, to avoid ambiguity with the deref target.

```rust
let raw = Box::into_raw(b); // good (C-SMART-PTR)
```

#### C-CONV-SPECIFIC: Conversions live on the most specific type involved

For example, `str` has both `as_bytes()` for `&[u8]` and `from_utf8()` for `&[u8]` because `str` is the more specific type.

#### C-METHOD: Functions with a clear receiver are methods

Methods need no import, benefit from autoborrowing, and are easier to discover.

```rust
impl Document {
    pub fn title(&self) -> &str { ... }
}
```

#### C-NO-OUT: Functions do not take out-parameters

Return tuples or structs. The only common exception is reusing a caller-owned buffer such as `read(&mut self, buf: &mut [u8])`.

```rust
pub fn split_at(&self, mid: usize) -> (&[T], &[T]); // good (C-NO-OUT)
```

#### C-OVERLOAD: Operator overloads are unsurprising

Implement `std::ops` traits only when the operation resembles the operator's algebraic properties.

#### C-DEREF: Only smart pointers implement `Deref` and `DerefMut`

The standard library treats `Box`, `String -> str`, `Rc`, `Arc`, and `Cow` as smart pointers. Implement `Deref` only when the value transparently behaves as the target, `deref()` is cheap, and coercion is unsurprising. Do not implement `Deref` merely to add methods.

#### C-CTOR: Constructors are static, inherent methods

Primary constructor: `new`. I/O resource types use domain verbs (`File::open`, `TcpStream::connect`, `UdpSocket::bind`). Secondary constructors are suffixed `_with_foo`. For many options, use a builder (C-BUILDER).

Distinctions between `from_` constructors and `From<T>`:

1. `from_` may be unsafe (`Box::from_raw`).
2. `from_` may take extra arguments (`u64::from_str_radix`).
3. `From<T>` only when the input type fully determines the output encoding.

Types commonly implement both `Default` and `new` with the same behavior.

```rust
impl Config {
    pub fn new() -> Self { ... }
    pub fn with_timeout(timeout: Duration) -> Self { ... }
    pub fn from_env() -> Result<Self, Error> { ... }
}
```

### Flexibility

Source: https://rust-lang.github.io/api-guidelines/flexibility.html

#### C-INTERMEDIATE: Functions expose intermediate results to avoid duplicate work

Return information callers would otherwise recompute.

```rust
match vec.binary_search(&key) {
    Ok(idx) => ...,
    Err(insert_idx) => vec.insert(insert_idx, key),
}
```

#### C-CALLER-CONTROL: Caller decides where to copy/place data

If ownership is needed, take ownership; if not, take a borrow. Use a `Copy` bound only when absolutely needed.

#### C-GENERIC: Functions minimize assumptions via generics

Prefer trait-bounded generics over concrete collection types. Trade-offs: reuse, static dispatch, and precise types vs. code bloat and verbose signatures.

```rust
pub fn sum<I: IntoIterator<Item = i64>>(iter: I) -> i64 { ... }
```

`std::fs::File::open` takes `impl AsRef<Path>` for the same reason.

#### C-OBJECT: Traits are object-safe if they may be useful as trait objects

Decide early whether a trait will be used as an object or as a generic bound. Exclude generic methods from object safety with `where Self: Sized`. `io::Read`/`io::Write` are commonly objects; `Iterator` marks generic methods with `where Self: Sized`.

```rust
pub trait Renderer {
    fn render(&self, doc: &Document);
    fn render_many<I: IntoIterator<Item = Document>>(items: I) -> String
    where
        Self: Sized;
}
```

### Type safety

Source: https://rust-lang.github.io/api-guidelines/type-safety.html

#### C-NEWTYPE: Newtypes provide static distinctions

Wrap a primitive in a tuple struct to give it a distinct identity and prevent confusion.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Miles(pub f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kilometers(pub f64);

fn distance_in_km(d: Kilometers) { ... }
// distance_in_km(Miles(5.0)); // compile error (C-NEWTYPE)
```

#### C-CUSTOM-TYPE: Arguments convey meaning through types, not `bool` or `Option`

Use enums or structs so call sites are self-documenting and easier to extend.

```rust
pub enum Size { Small, Large }
pub enum Shape { Round, Square }

impl Widget {
    pub fn new(size: Size, shape: Shape) -> Self { ... } // good (C-CUSTOM-TYPE)
}
```

#### C-BITFLAG: Types for a set of flags use `bitflags`, not enums

An enum is exactly-one-of-N; a set of flags is presence/absence of multiple bits.

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
    }
}
```

#### C-BUILDER: Builders enable construction of complex values

Use a builder when there are many inputs, compound data, optional config, or several construction flavors. The constructor takes only required data; setters return `Self` for chaining; terminal methods build `T`.

Variant 1: non-consuming (preferred). Configuration methods take `&mut self`; terminal method takes `&self`. This mirrors `std::process::Command`.

```rust
impl RequestBuilder {
    pub fn new(method: Method, uri: Uri) -> Self { ... }
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self { self.timeout = Some(timeout); self }
    pub fn build(&self) -> Request { ... }
}
```

Variant 2: consuming. Configuration methods take and return owned `self` so one-liners keep working.

```rust
impl RequestBuilder {
    pub fn new(method: Method, uri: Uri) -> Self { ... }
    pub fn timeout(mut self, timeout: Duration) -> Self { self.timeout = Some(timeout); self }
    pub fn build(self) -> Request { ... }
}
```

### Dependability

Source: https://rust-lang.github.io/api-guidelines/dependability.html

There is no official checklist item in the API Guidelines for banning `unwrap()`/`expect()`/`panic!()`. That rule comes from clippy lints (`clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`) and ecosystem practice.

#### C-VALIDATE: Functions validate their arguments

Rust does not follow the "robustness principle"; enforce validity. Preference order:

1. Static enforcement: choose an argument type that rules out bad inputs (e.g. an `Ascii` newtype over raw `u8`).
2. Dynamic enforcement: validate as processed.
3. Dynamic enforcement with `debug_assert!`: for expensive checks that should not run in release.
4. Dynamic enforcement with opt-out: use an `_unchecked` suffix or a `raw` submodule.

```rust
pub struct Ascii(u8);

impl Ascii {
    pub fn new(byte: u8) -> Option<Self> {
        if byte <= 0x7f { Some(Ascii(byte)) } else { None }
    }
    pub unsafe fn from_byte_unchecked(byte: u8) -> Self {
        debug_assert!(byte <= 0x7f);
        Ascii(byte)
    }
}
```

#### C-DTOR-FAIL: Destructors never fail

`Drop` runs during unwinding; a failing destructor aborts the process. Expose a separate method such as `close() -> Result<...>` for clean teardown and make `Drop` ignore or log errors.

#### C-DTOR-BLOCK: Destructors that may block have alternatives

Do not invoke blocking operations inside `Drop`.

### Debuggability

Source: https://rust-lang.github.io/api-guidelines/debuggability.html

#### C-DEBUG: All public types implement `Debug`

If there are exceptions, they are rare. Derive `Debug` whenever possible.

```rust
#[derive(Debug)]
pub struct Connection { ... }
```

#### C-DEBUG-NONEMPTY: `Debug` representation is never empty

Even conceptually-empty values render distinctly: `""` for empty strings, `[]` for empty vectors.

### Future proofing

Source: https://rust-lang.github.io/api-guidelines/future-proofing.html

#### C-SEALED: Sealed traits protect against downstream implementations

Add a private `Sealed` supertrait that downstream crates cannot name. Only the defining crate can implement the trait, letting you add methods or change undocumented signatures without breaking. It is still breaking to remove a public method or change a public signature. Document that the trait is sealed.

```rust
mod private {
    pub trait Sealed {}
}

/// Sealed; cannot be implemented outside this crate (C-SEALED).
pub trait Renderer: private::Sealed {
    fn render(&self, doc: &Document);
}

impl private::Sealed for SvgRenderer {}
impl Renderer for SvgRenderer { ... }
```

#### C-STRUCT-PRIVATE: Structs have private fields

A public field pins down representation and prevents invariant validation because clients can mutate the field arbitrarily. Public fields are appropriate only for "C-spirit" passive data structs.

```rust
pub struct Good {
    name: String,
    age: u8,
}

impl Good {
    pub fn new(name: String, age: u8) -> Result<Self, Error> { ... }
    pub fn name(&self) -> &str { &self.name }
}
```

#### C-NEWTYPE-HIDE: Newtypes encapsulate implementation details

Use a newtype to hide compound return types such as `Enumerate<Skip<I>>`. `impl Trait` is more concise but limited; it cannot easily require `Debug + Clone` on the return type.

```rust
pub struct TokenStream {
    inner: std::vec::IntoIter<Token>,
}

impl Iterator for TokenStream {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
}
```

#### C-STRUCT-BOUNDS: Data structures do not duplicate derived trait bounds

Prefer deriving traits without adding bounds to the struct definition.

```rust
// Good (C-STRUCT-BOUNDS)
#[derive(Clone, Debug, PartialEq)]
pub struct Good<T> { value: T }

// Bad (C-STRUCT-BOUNDS)
// pub struct Bad<T: Clone + Debug + PartialEq> { value: T }
```

Adding a bound is breaking; deriving more traits is non-breaking. Traits that should never appear in data-structure bounds: `Clone`, `PartialEq`, `PartialOrd`, `Debug`, `Display`, `Default`, `Error`, `Serialize`, `Deserialize`, `DeserializeOwned`. Exceptions where a bound is required: it refers to an associated type on the trait; it is `?Sized`; the struct has a `Drop` impl requiring it.

### Necessities

Source: https://rust-lang.github.io/api-guidelines/necessities.html

#### C-STABLE: Public dependencies of a stable crate are stable

A crate cannot be at version `>= 1.0.0` unless all public dependencies (types used in the public API) are stable. Beware: `impl From<other_crate::Error>` for a public type puts that foreign type into your public API.

#### C-PERMISSIVE: Crate and its dependencies have a permissive license

The guidelines recommend dual `MIT OR Apache-2.0` to match Rust itself. Apache-only is not recommended. Require `LICENSE-APACHE` and `LICENSE-MIT` files at the repository root.

```text
LICENSE-APACHE
LICENSE-MIT
```

### Documentation

Source: https://rust-lang.github.io/api-guidelines/documentation.html

Detailed documentation rules are covered in `docs/rust/documentation-guidelines.md`. At a high level, the API Guidelines include C-CRATE-DOC (crate-level docs), C-EXAMPLE (examples for public items), and C-FAILURE (safety and error documentation).

## Review checklist

- [ ] Casing follows RFC 430 (C-CASE).
- [ ] Ad-hoc conversions use `as_`/`to_`/`into_` correctly (C-CONV).
- [ ] Getters are named after fields; `get_` is used only for validated lookup (C-GETTER).
- [ ] Collections expose `iter`, `iter_mut`, and `into_iter` with matching type names (C-ITER, C-ITER-TY).
- [ ] Feature names avoid placeholder words and are additive (C-FEATURE).
- [ ] Names use a consistent word order (C-WORD-ORDER).
- [ ] Public types implement common traits eagerly (C-COMMON-TRAITS).
- [ ] Conversions use `From`/`TryFrom`/`AsRef`/`AsMut`; `Into`/`TryInto` are never implemented directly (C-CONV-TRAITS).
- [ ] Collections implement `FromIterator` and `Extend` (C-COLLECT).
- [ ] Serde support is feature-gated behind the exact feature name `serde` (C-SERDE).
- [ ] Types are `Send` and `Sync` where possible (C-SEND-SYNC).
- [ ] Error types implement `std::error::Error + Send + Sync + 'static` and use `source()` (C-GOOD-ERR).
- [ ] Bitflag-like types implement `Binary`/`Octal`/`LowerHex`/`UpperHex` (C-NUM-FMT).
- [ ] Generic reader/writer functions take `R: Read` / `W: Write` by value (C-RW-VALUE).
- [ ] Smart pointers expose associated functions, not inherent methods (C-SMART-PTR).
- [ ] Conversions live on the most specific type involved (C-CONV-SPECIFIC).
- [ ] Functions with a clear receiver are methods (C-METHOD).
- [ ] Functions return values instead of taking out-parameters, except for reused buffers (C-NO-OUT).
- [ ] Operator overloads match the operator's algebraic properties (C-OVERLOAD).
- [ ] Only smart pointers implement `Deref`/`DerefMut` (C-DEREF).
- [ ] Constructors are static inherent methods; primary constructor is `new` (C-CTOR).
- [ ] Functions expose intermediate results to avoid duplicate work (C-INTERMEDIATE).
- [ ] Caller controls copy/place decisions (C-CALLER-CONTROL).
- [ ] Functions minimize assumptions via trait-bounded generics (C-GENERIC).
- [ ] Traits that may be useful as objects are object-safe (C-OBJECT).
- [ ] Newtypes provide static distinctions (C-NEWTYPE).
- [ ] Arguments use enums/structs instead of `bool` or raw numerics (C-CUSTOM-TYPE).
- [ ] Sets of flags use `bitflags`, not enums (C-BITFLAG).
- [ ] Builders are used for complex construction (C-BUILDER).
- [ ] Arguments are validated; prefer static enforcement (C-VALIDATE).
- [ ] Destructors never fail and never block without alternatives (C-DTOR-FAIL, C-DTOR-BLOCK).
- [ ] All public types implement `Debug` with a non-empty representation (C-DEBUG, C-DEBUG-NONEMPTY).
- [ ] Traits meant to be implementation-only are sealed (C-SEALED).
- [ ] Structs have private fields unless they are passive C-style data (C-STRUCT-PRIVATE).
- [ ] Data structures do not duplicate derived trait bounds (C-STRUCT-BOUNDS).
- [ ] Public dependencies of a stable crate are stable (C-STABLE).
- [ ] Crate and dependencies use a permissive license, preferably `MIT OR Apache-2.0` (C-PERMISSIVE).
- [ ] Public API contains no `unwrap()`, `expect()`, or `panic!()` (clippy `unwrap_used`/`expect_used`/`panic`; not a Rust API Guidelines checklist item).

## Implementation checklist

- [ ] Read the Rust API Guidelines checklist at https://rust-lang.github.io/api-guidelines/checklist.html.
- [ ] Define the public surface deliberately: every `pub` item is part of the semver contract.
- [ ] Choose names that follow the naming recommendations (C-CASE, C-CONV, C-GETTER, C-ITER, C-ITER-TY, C-FEATURE, C-WORD-ORDER).
- [ ] Derive or implement common standard traits for every public type (C-COMMON-TRAITS).
- [ ] Implement `From`/`TryFrom`/`AsRef`/`AsMut` for canonical conversions (C-CONV-TRAITS).
- [ ] Feature-gate optional dependencies with additive feature names (C-FEATURE, C-SERDE).
- [ ] Design error enums with `std::error::Error`, `Send + Sync + 'static`, and `#[non_exhaustive]` (C-GOOD-ERR; `#[non_exhaustive]` from the Rust Reference).
- [ ] Add `assert_send::<T>()` and `assert_sync::<T>()` tests for types containing raw pointers (C-SEND-SYNC).
- [ ] Return intermediate results instead of forcing callers to recompute (C-INTERMEDIATE).
- [ ] Use generics with trait bounds instead of concrete collection types where possible (C-GENERIC).
- [ ] Make traits object-safe when they may be used as trait objects (C-OBJECT).
- [ ] Replace `bool` and raw numeric arguments with enums or newtypes (C-CUSTOM-TYPE, C-NEWTYPE).
- [ ] Use `bitflags` for flag sets (C-BITFLAG).
- [ ] Validate inputs, preferring static enforcement (C-VALIDATE).
- [ ] Keep struct fields private and expose constructors/getters (C-STRUCT-PRIVATE, C-GETTER).
- [ ] Avoid duplicating trait bounds on generic data structures (C-STRUCT-BOUNDS).
- [ ] Seal traits that are not intended for downstream implementation (C-SEALED).
- [ ] Document every public item and include examples (C-CRATE-DOC, C-EXAMPLE, C-FAILURE; see `docs/rust/documentation-guidelines.md`).
- [ ] Ensure `LICENSE-MIT` and `LICENSE-APACHE` exist at the repository root (C-PERMISSIVE).

## Validation hooks

Run these commands to validate a crate against this guidance:

- `cargo check --all-features`
- `cargo test --all-features`
- `cargo test --doc`
- `cargo doc --no-deps`
- `cargo clippy --all-features -- -D warnings`

Additional clippy configuration for public API robustness (not official Rust API Guidelines checklist items):

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

Before releasing:

- `cargo semver-checks` or `cargo public-api diff` to detect breaking changes (RFC 1105 defines breaking changes).
- Manual review against the Rust API Guidelines checklist.
- Compile-time assertions for `Send`/`Sync` on raw-pointer-containing types (C-SEND-SYNC):

```rust
#[cfg(test)]
mod tests {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn connection_is_send_sync() {
        assert_send::<crate::Connection>();
        assert_sync::<crate::Connection>();
    }
}
```

## Examples

### Error enum with `thiserror`

Use `thiserror` to reduce boilerplate while keeping a structured public error type. Mark the enum `#[non_exhaustive]` so future variants are not breaking (this is a Rust language feature, not a Rust API Guidelines checklist item).

```rust
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid key: {key}")]
    InvalidKey { key: String },
    #[error("missing required field: {0}")]
    MissingField(&'static str),
}
```

### Sealed trait

```rust
use std::fmt;

mod private {
    pub trait Sealed {}
}

/// Sealed; only this crate can provide implementations (C-SEALED).
pub trait DisplayExt: private::Sealed + fmt::Display {
    fn greeting(&self) -> String {
        format!("Hello, {self}!")
    }
}

struct Name(String);

impl private::Sealed for Name {}
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl DisplayExt for Name {}
```

### Builder (non-consuming variant)

```rust
use std::time::Duration;

#[derive(Debug, Default)]
pub struct RequestBuilder {
    method: String,
    uri: String,
    timeout: Option<Duration>,
}

impl RequestBuilder {
    pub fn new(method: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            uri: uri.into(),
            ..Default::default()
        }
    }

    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(&self) -> Request {
        Request {
            method: self.method.clone(),
            uri: self.uri.clone(),
            timeout: self.timeout,
        }
    }
}
```

### Feature-gated serde

```toml
[dependencies]
serde = { version = "1", features = ["derive"], optional = true }

[features]
default = ["std"]
std = []
serde = ["dep:serde"]
```

```rust
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    pub timeout: Duration,
}
```

### Newtype for domain distinction

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(u64);

fn load_user(id: UserId) { ... }
// load_user(OrderId(42)); // compile error (C-NEWTYPE)
```

### Bitflags for flag sets

```rust
use bitflags::bitflags;
use std::fmt;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Mode: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
    }
}

impl fmt::Binary for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.bits, f)
    }
}
```

### Validation with static enforcement

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ascii(u8);

impl Ascii {
    pub fn new(byte: u8) -> Option<Self> {
        if byte <= 0x7f { Some(Ascii(byte)) } else { None }
    }

    pub unsafe fn from_byte_unchecked(byte: u8) -> Self {
        debug_assert!(byte <= 0x7f);
        Ascii(byte)
    }
}
```

### Struct bounds good vs bad

```rust
// Good (C-STRUCT-BOUNDS): derive without duplicating bounds on T.
#[derive(Clone, Debug, PartialEq)]
pub struct Wrapper<T> {
    inner: T,
}

// Bad (C-STRUCT-BOUNDS): adding bounds here is unnecessary and breaking-prone.
// pub struct Wrapper<T: Clone + Debug + PartialEq> {
//     inner: T,
// }
```

## Common mistakes

- Using `unwrap()` or `expect()` in public API code. This is enforced by clippy lints (`clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`), not by a Rust API Guidelines checklist item.
- Implementing `Deref` just to add methods to a type. Only smart pointers should implement `Deref`/`DerefMut` (C-DEREF).
- Implementing `Into` or `TryInto` directly. Implement `From`/`TryFrom` instead (C-CONV-TRAITS).
- Using the `get_` prefix on simple field getters. Use the field name directly; reserve `get_` for validated lookup (C-GETTER).
- Duplicating derived trait bounds on generic data structures (C-STRUCT-BOUNDS).
- Naming a feature `use-x`, `with-x`, or `no-x`. Use the plain name and keep features additive (C-FEATURE).
- Using `()` as an error type in a public `Result` (C-GOOD-ERR).
- Implementing `Error::description()` or `Error::cause()`. Both are deprecated; use `Display` and `Error::source()` (C-GOOD-ERR).
- Returning raw numeric types instead of newtypes for domain identifiers (C-NEWTYPE).
- Using `bool` or raw numeric arguments where an enum or newtype would be clearer (C-CUSTOM-TYPE).
- Forgetting `#[non_exhaustive]` on public error enums. This is a Rust language feature documented in the Rust Reference, not a Rust API Guidelines checklist item, but it is standard practice for published libraries.
- Naming a serde feature anything other than `serde` (C-SERDE).
- Adding inherent methods to smart pointers instead of associated functions (C-SMART-PTR).
- Treating the Rust API Guidelines as mandatory. They are advisory; apply them with judgment.

## Strict vs contextual guidance

### Strict

Treat these as defaults unless there is a strong, documented reason not to:

- All public types implement `Debug` with a non-empty representation (C-DEBUG, C-DEBUG-NONEMPTY).
- Cargo features are additive and avoid placeholder words (C-FEATURE).
- Naming follows RFC 430, including casing, conversion prefixes, getter rules, and word order (C-CASE, C-CONV, C-GETTER, C-ITER, C-ITER-TY, C-WORD-ORDER).
- Common standard traits are implemented where applicable (C-COMMON-TRAITS).
- Standard conversion traits (`From`, `TryFrom`, `AsRef`, `AsMut`) are implemented; `Into`/`TryInto` are never implemented directly (C-CONV-TRAITS).
- Struct fields are private unless the struct is passive C-style data (C-STRUCT-PRIVATE).
- Generic data structures do not duplicate derived trait bounds (C-STRUCT-BOUNDS).
- Public types are `Send` and `Sync` where possible (C-SEND-SYNC).
- Safety invariants are documented in `# Safety` sections (C-FAILURE).
- Error types implement `std::error::Error + Send + Sync + 'static` (C-GOOD-ERR).
- Public dependencies of a stable crate are stable (C-STABLE).
- Crate and dependencies use a permissive license, preferably `MIT OR Apache-2.0` (C-PERMISSIVE).

### Contextual

Apply these when the situation calls for them; do not force them everywhere:

- Builder pattern (C-BUILDER) is a "consider" when construction is complex; a plain `new()` constructor is fine for simple structs.
- Newtypes for hiding implementation details (C-NEWTYPE-HIDE) are useful when the concrete iterator or return type would leak internals.
- Sealed traits (C-SEALED) are appropriate when external implementations must be forbidden.
- `#[non_exhaustive]` (Rust Reference language feature) is required for published libraries but optional for internal binaries.
- Extension traits are helpful for integrating with foreign crates but should be scoped and documented.
- MSRV choice and CI enforcement are ecosystem conventions, not Rust API Guidelines checklist items.

## Policy decisions for individual repos

Each repository should document its own policy for:

- `anyhow` in library crates vs binaries. Libraries generally expose structured errors; applications may use type-erased errors.
- Minimum supported Rust version (MSRV) and CI enforcement. MSRV is an ecosystem convention, not a Rust API Guidelines checklist item.
- Whether `unsafe` is allowed and what review process it requires.
- Default feature set: opt-in to everything, or enable common defaults.
- Public API diffing tool: `cargo semver-checks`, `cargo public-api`, or manual review.
- Whether to deny missing docs (`#![deny(missing_docs)]`) and how strictly to enforce clippy lints.
- License choice. The guidelines recommend `MIT OR Apache-2.0` (C-PERMISSIVE).

## Related docs

- `docs/rust/documentation-guidelines.md`
- `docs/rust/design-patterns.md`
- `docs/rust/types-traits-generics.md`
- `docs/rust/error-handling.md`
- `docs/rust/lints-clippy.md`
- `docs/rust/style-formatting.md`

## Related skills

None defined yet.
