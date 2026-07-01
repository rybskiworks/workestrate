# Rust design patterns, idioms, and anti-patterns

## Purpose

Provide a comprehensive, repo-independent catalog of design patterns, idioms, and anti-patterns for AI coding agents working in Rust. Use this file when writing, reviewing, or refactoring Rust code to make it idiomatic, composable, type-safe, and maintainable. It focuses on *pattern-level* guidance and decision frameworks; language mechanics and API-surface guidelines live in sibling documents listed under [Related docs](#related-docs).

## Sources used

- https://rust-unofficial.github.io/patterns/
- https://rust-unofficial.github.io/patterns/patterns/index.html
- https://rust-unofficial.github.io/patterns/idioms/index.html
- https://rust-unofficial.github.io/patterns/anti_patterns/index.html
- https://rust-unofficial.github.io/patterns/functional/index.html

> **Note on provenance.** The [Rust Design Patterns book](https://rust-unofficial.github.io/patterns/) documents a finite set of design patterns, idioms, anti-patterns, and functional patterns. Several widely-known patterns and anti-patterns below are community-recognized conventions from the Rust API Guidelines, Clippy, the standard library, and ecosystem practice; they are marked as such rather than being from the book.

### Crawl ledger

Patterns: [behavioural/intro](https://rust-unofficial.github.io/patterns/patterns/behavioural/intro.html), [command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html), [interpreter](https://rust-unofficial.github.io/patterns/patterns/behavioural/interpreter.html), [newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html), [RAII](https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html), [strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html), [visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html); [creational/intro](https://rust-unofficial.github.io/patterns/patterns/creational/intro.html), [builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html), [fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html); [structural/intro](https://rust-unofficial.github.io/patterns/patterns/structural/intro.html), [compose-structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html), [small-crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html), [unsafe-mods](https://rust-unofficial.github.io/patterns/patterns/structural/unsafe-mods.html), [trait-for-bounds](https://rust-unofficial.github.io/patterns/patterns/structural/trait-for-bounds.html); [ffi/intro](https://rust-unofficial.github.io/patterns/patterns/ffi/intro.html), [ffi/export](https://rust-unofficial.github.io/patterns/patterns/ffi/export.html), [ffi/wrappers](https://rust-unofficial.github.io/patterns/patterns/ffi/wrappers.html).

Idioms: [coercion-arguments](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html), [concat-format](https://rust-unofficial.github.io/patterns/idioms/concat-format.html), [ctor](https://rust-unofficial.github.io/patterns/idioms/ctor.html), [default](https://rust-unofficial.github.io/patterns/idioms/default.html), [deref](https://rust-unofficial.github.io/patterns/idioms/deref.html), [dtor-finally](https://rust-unofficial.github.io/patterns/idioms/dtor-finally.html), [mem-replace](https://rust-unofficial.github.io/patterns/idioms/mem-replace.html), [on-stack-dyn-dispatch](https://rust-unofficial.github.io/patterns/idioms/on-stack-dyn-dispatch.html); [ffi/intro](https://rust-unofficial.github.io/patterns/idioms/ffi/intro.html), [ffi/errors](https://rust-unofficial.github.io/patterns/idioms/ffi/errors.html), [ffi/accepting-strings](https://rust-unofficial.github.io/patterns/idioms/ffi/accepting-strings.html), [ffi/passing-strings](https://rust-unofficial.github.io/patterns/idioms/ffi/passing-strings.html); [option-iter](https://rust-unofficial.github.io/patterns/idioms/option-iter.html), [pass-var-to-closure](https://rust-unofficial.github.io/patterns/idioms/pass-var-to-closure.html), [priv-extend](https://rust-unofficial.github.io/patterns/idioms/priv-extend.html), [rustdoc-init](https://rust-unofficial.github.io/patterns/idioms/rustdoc-init.html), [temporary-mutability](https://rust-unofficial.github.io/patterns/idioms/temporary-mutability.html), [return-consumed-arg-on-error](https://rust-unofficial.github.io/patterns/idioms/return-consumed-arg-on-error.html).

Anti-patterns: [borrow_clone](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html), [deny-warnings](https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html), [deref](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html).

Functional: [paradigms](https://rust-unofficial.github.io/patterns/functional/paradigms.html), [generics-type-classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html), [optics](https://rust-unofficial.github.io/patterns/functional/optics.html).

Community sources: [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [Clippy lint list](https://rust-lang.github.io/rust-clippy/master/), Rust standard library documentation.

## Core guidance

Rust favors composition over inheritance, explicit ownership over shared mutable state, and types that encode invariants. Patterns should reduce complexity, not add indirection. Apply a pattern when it makes the code simpler, safer, or more evolvable; avoid it when it introduces abstraction without a clear benefit. Prefer the type system and the borrow checker over runtime checks, and prefer the standard library and compiler lints over custom machinery.

## Practical rules

### Design patterns

#### Newtype

Source: [Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) from the book.

Wrap a primitive or foreign type in a tuple struct for distinct identity, invariants, trait isolation, or restricted operations. Newtypes are zero-cost. Use cases: units of measure, trait-orbit isolation, domain IDs. Con: boilerplate; use `derive_more` when appropriate. Cross-reference `docs/rust/api-design.md` for C-NEWTYPE.

```rust
use std::fmt;
pub struct Miles(pub f64);
pub struct Kilometres(pub f64);
fn fuel_needed(km: Kilometres, efficiency: f64) -> f64 { km.0 / efficiency }

pub struct Password(String);
impl fmt::Display for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "********") }
}
```

#### Builder

Source: [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) from the book.

Use a builder when construction has many optional parameters, compound data, backward-compatible additions, or validation. Rust lacks default arguments and overloading, so builders shine. The non-consuming variant is preferred for multi-step setup; the consuming variant is convenient for one-liners. `derive_builder` can generate boilerplate. A full example is under [Examples](#examples). Cross-reference `docs/rust/api-design.md` for C-BUILDER.

```rust
use std::time::Duration;

#[derive(Debug, Default)]
pub struct ServerBuilder { host: String, port: u16, timeout: Option<Duration> }

impl ServerBuilder {
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout); self
    }
    pub fn build(&self) -> Server {
        Server { host: self.host.clone(), port: self.port,
                 timeout: self.timeout.unwrap_or(Duration::from_secs(30)) }
    }
}
pub struct Server { host: String, port: u16, timeout: Duration }
```

#### RAII guards

Source: [RAII](https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html) from the book.

Acquire a resource in a constructor and release it in `Drop`. The borrow checker ties the guard's lifetime to the protected resource, preventing use-after-free and leaks. `Drop` can also act as a `finally` block via scope guards. Destructors must never panic; if cleanup can fail, expose a separate `close()` method. Cross-reference `docs/rust/smart-pointers-memory.md`.

```rust
pub struct Connection;
impl Drop for Connection {
    fn drop(&mut self) { self.close(); }
}
impl Connection { fn close(&mut self) {} }
```

#### Strategy via traits

Source: [Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html) from the book.

Define interchangeable algorithms through traits. Rust traits often replace the formal Strategy pattern; the book warns against reaching for Strategy until simpler trait-based polymorphism is insufficient. Closures are lightweight strategies. A full example is under [Examples](#examples).

```rust
pub trait Formatter { fn format(&self, record: &Record) -> String; }
pub struct TextFormatter;
impl Formatter for TextFormatter {
    fn format(&self, r: &Record) -> String { format!("[{}] {}", r.level, r.message) }
}
pub struct Record { pub level: String, pub message: String }
```

#### State / type-state

Community-recognized; built on [generics as type classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html).

Encode valid state transitions in types so invalid transitions are compile-time errors. Use unit structs or empty enums as state markers. A full example is under [Examples](#examples). For data-driven runtime state, prefer an enum carrying data. Cross-reference `docs/rust/types-traits-generics.md`.

```rust
pub struct Idle; pub struct Running;
pub struct Machine<S> { label: String, _state: std::marker::PhantomData<S> }

impl Machine<Idle> {
    pub fn start(self) -> Machine<Running> {
        Machine { label: self.label, _state: std::marker::PhantomData }
    }
}
impl Machine<Running> {
    pub fn stop(self) -> Machine<Idle> {
        Machine { label: self.label, _state: std::marker::PhantomData }
    }
}
```

#### Command

Source: [Command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html) from the book.

Encapsulate an operation as an object so it can be queued, logged, undone, or replayed. Three forms: trait objects (`Box<dyn Command>`), function pointers (`fn()`), and `Fn` trait objects (`Box<dyn Fn()>`). The trait-object form uses dynamic dispatch.

```rust
pub trait Command { fn execute(&mut self); fn undo(&mut self); }
pub struct MoveCommand { target: i32, previous: i32 }
impl Command for MoveCommand {
    fn execute(&mut self) { self.previous = self.target; }
    fn undo(&mut self) { self.target = self.previous; }
}
pub struct History { commands: Vec<Box<dyn Command>> }
```

#### Visitor

Source: [Visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html) from the book.

Add operations to a heterogeneous recursive data structure without modifying its types. Rust has no double-dispatch problem because traits handle dispatch directly. For homogeneous data, prefer iterators. Cross-reference [Fold](#fold).

```rust
pub enum Expr { Num(i32), Add(Box<Expr>, Box<Expr>) }

pub trait ExprVisitor {
    type Output;
    fn visit_num(&mut self, n: i32) -> Self::Output;
    fn visit_add(&mut self, l: &Expr, r: &Expr) -> Self::Output;
}

pub struct Evaluator;
impl ExprVisitor for Evaluator {
    type Output = i32;
    fn visit_num(&mut self, n: i32) -> i32 { n }
    fn visit_add(&mut self, l: &Expr, r: &Expr) -> i32 { eval(self, l) + eval(self, r) }
}

fn eval<V: ExprVisitor>(visitor: &mut V, expr: &Expr) -> V::Output {
    match expr {
        Expr::Num(n) => visitor.visit_num(*n),
        Expr::Add(l, r) => visitor.visit_add(l, r),
    }
}
```

#### Fold

Source: [Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html) from the book.

Map a recursive structure into a *new* recursive structure, in contrast to Visitor which inspects or updates in place. Default no-op methods reduce boilerplate. The downside is that consuming folds take ownership of `Box`es and cannot reuse the original tree; borrowed folds often force cloning.

```rust
pub enum Expr { Num(i32), Add(Box<Expr>, Box<Expr>) }

pub trait ExprFolder {
    fn fold_num(&mut self, n: i32) -> Expr { Expr::Num(n) }
    fn fold_add(&mut self, l: Expr, r: Expr) -> Expr {
        Expr::Add(Box::new(l), Box::new(r))
    }
}

pub fn fold<F: ExprFolder>(folder: &mut F, expr: Expr) -> Expr {
    match expr {
        Expr::Num(n) => folder.fold_num(n),
        Expr::Add(l, r) => folder.fold_add(fold(folder, *l), fold(folder, *r)),
    }
}
```

#### Interpreter

Source: [Interpreter](https://rust-unofficial.github.io/patterns/patterns/behavioural/interpreter.html) from the book.

Build a small language (DSL) for a recurring problem domain. A recursive-descent interpreter over an AST is the classic form; `macro_rules!` can provide a compile-time DSL alternative. Use sparingly; DSLs add maintenance burden and the book's example relies on brittle panics for malformed input.

```rust
pub enum Expr { Num(i32), Add(Box<Expr>, Box<Expr>) }
impl Expr {
    pub fn eval(&self) -> i32 {
        match self {
            Expr::Num(n) => *n,
            Expr::Add(l, r) => l.eval() + r.eval(),
        }
    }
}
```

#### Abstract Factory / trait-based factories

Community-recognized; not a standalone page in the book.

Use a factory trait to create families of related objects when the concrete type is chosen at runtime or by configuration. Return trait objects or generic values depending on whether heterogeneity is required.

```rust
pub trait WidgetFactory {
    fn create_button(&self) -> Box<dyn Button>;
    fn create_input(&self) -> Box<dyn Input>;
}
pub struct GtkFactory;
impl WidgetFactory for GtkFactory {
    fn create_button(&self) -> Box<dyn Button> { Box::new(GtkButton) }
    fn create_input(&self) -> Box<dyn Input> { Box::new(GtkInput) }
}
pub trait Button {} pub trait Input {}
struct GtkButton; impl Button for GtkButton {}
struct GtkInput; impl Input for GtkInput {}
```

#### Compose structs for independent borrowing

Source: [Compose structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html) from the book.

Decompose a large struct into smaller sub-structs so fields can be borrowed independently. This often reveals a better design and avoids fighting the borrow checker.

```rust
pub struct User { profile: Profile, settings: Settings }
pub struct Profile { pub name: String }
pub struct Settings { pub theme: String }

pub fn update(user: &mut User, name: &str, theme: &str) {
    user.profile.name = name.to_string();
    user.settings.theme = theme.to_string();
}
```

#### Contain unsafe in small modules

Source: [Contain unsafe in small modules](https://rust-unofficial.github.io/patterns/patterns/structural/unsafe-mods.html) from the book.

Keep `unsafe` blocks inside the smallest possible module, document the invariants it upholds, and expose only safe public APIs. Cross-reference `docs/rust/unsafe-security.md`.

```rust
mod raw {
    // Invariant: ptr is non-null and points to a valid, initialized T.
    pub unsafe fn deref_ptr<T>(ptr: *const T) -> &'static T { &*ptr }
}
pub fn safe_accessor<T>(ptr: *const T) -> Option<&'static T> {
    if ptr.is_null() { return None; }
    // SAFETY: null check performed above; caller contract applies.
    Some(unsafe { raw::deref_ptr(ptr) })
}
```

#### Custom traits to avoid complex type bounds

Source: [Custom traits for bounds](https://rust-unofficial.github.io/patterns/patterns/structural/trait-for-bounds.html) from the book.

Name unwieldy `Fn` or compound trait bounds with a custom trait, then provide a blanket implementation. This makes signatures readable and lets you change bounds later.

```rust
pub trait Predicate<T>: Fn(&T) -> bool {}
impl<T, F: Fn(&T) -> bool> Predicate<T> for F {}

pub fn filter<T>(items: &[T], pred: impl Predicate<T>) -> Vec<&T> {
    items.iter().filter(|x| pred(x)).collect()
}
```

#### Prefer small crates

Source: [Prefer small crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html) from the book.

Small crates improve modularity, enable parallel compilation, and let consumers depend only on what they need. Trade-offs: dependency management overhead, no default whole-program LTO, and ecosystem fragmentation. Organize large codebases into focused crates.

#### Composition over inheritance

Community-recognized core principle; Rust has no class inheritance.

Define behavior with traits and combine capabilities by embedding structs. Do not emulate inheritance through `Deref`; see [Deref polymorphism](#deref-polymorphism).

```rust
pub trait Render { fn render(&self); }
pub trait Update { fn update(&mut self); }
pub struct Widget<R: Render, U: Update> { renderer: R, updater: U }
```

### Functional patterns

#### Imperative vs declarative

Source: [Functional paradigms](https://rust-unofficial.github.io/patterns/functional/paradigms.html) from the book.

The book contrasts imperative and declarative styles:

```rust
let mut sum = 0;
for i in 1..11 { sum += i; }

let sum: i32 = (1..11).fold(0, |a, b| a + b);
```

Prefer declarative style when it improves clarity; prefer `for` loops when control flow is complex.

#### Generics as type classes / type-state

Source: [Generics as type classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html) from the book.

Use generic parameters to split an API at compile time. This underpins `embedded-hal`, `hyper`, and many zero-cost abstraction crates. The type-state pattern is the same idea applied to state machines. Con: monomorphization can increase binary size; prefer trait objects when code size matters more.

```rust
pub trait ProtoKind {}
pub struct Http; pub struct Https;
impl ProtoKind for Http {}
impl ProtoKind for Https {}

pub struct DownloadRequest<P: ProtoKind> {
    url: String, _proto: std::marker::PhantomData<P>,
}
impl<P: ProtoKind> DownloadRequest<P> {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), _proto: std::marker::PhantomData }
    }
}
```

#### Optics (Iso / Poly Iso / Prism)

Source: [Optics](https://rust-unofficial.github.io/patterns/functional/optics.html) from the book.

Optics are advanced, conceptual patterns for composable data access and transformation. They explain designs such as `serde`'s `Deserializer`/`Visitor` separation. Treat as advanced reading; most day-to-day Rust does not require custom optics.

#### Iterator pipelines

Community-recognized / standard library; not in the Functional chapter.

Day-to-day "functional Rust" lives in the standard library iterator API and is covered in `docs/rust/iterators-closures.md`. Prefer chains of `map`, `filter`, `fold`, and `collect` when the pipeline is clear; choose `iter()`/`iter_mut()`/`into_iter()` based on ownership; use turbofish or type annotations when `collect()` inference is ambiguous. Do not re-teach these mechanics here.

### Idioms

#### Borrowed types for args

Source: [Coercion arguments](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html) from the book.

Accept borrowed views rather than owned types. `&str` instead of `&String`, `&[User]` instead of `&Vec<User>`. Deref coercion lets callers pass either owned or borrowed values.

```rust
pub fn greet(name: &str) { println!("Hello, {name}!"); }
greet("world");
greet(&String::from("world"));
```

#### Constructor `new` + `Default`

Source: [Constructor](https://rust-unofficial.github.io/patterns/idioms/ctor.html) and [Default](https://rust-unofficial.github.io/patterns/idioms/default.html) from the book.

Provide `new()` and `Default` when a sensible default exists. Use `#[derive(Default)]` and struct update syntax for partial initialization.

```rust
#[derive(Debug, Default)]
pub struct Config { retries: u32, timeout: Duration }

impl Config {
    pub fn new() -> Self { Self::default() }
    pub fn with_retries(retries: u32) -> Self { Self { retries, ..Default::default() } }
}
```

#### `mem::take` / `mem::replace`

Source: [Mem replace](https://rust-unofficial.github.io/patterns/idioms/mem-replace.html) from the book.

Mutate an enum variant in place without cloning by temporarily replacing it with a dummy value. `Option::take` is the common special case.

```rust
use std::mem;
enum State { A(String), B(String) }
impl State {
    fn a_to_b(&mut self) {
        let taken = mem::take(self);
        *self = match taken {
            State::A(s) => State::B(s),
            other => other,
        };
    }
}
```

#### Finalisation in destructors / `Drop` as `finally`

Source: [Dtor finally](https://rust-unofficial.github.io/patterns/idioms/dtor-finally.html) from the book.

Use a scope guard to run cleanup code when a value goes out of scope. This is not guaranteed in all circumstances (panic during unwinding, process abort, or `mem::forget`), and the guard must not be moved or shared via `Rc`. Use it for best-effort cleanup, not safety-critical finalisation.

```rust
struct OnExit<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for OnExit<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() { f(); }
    }
}
```

#### Iterating over an `Option`

Source: [Option iter](https://rust-unofficial.github.io/patterns/idioms/option-iter.html) from the book.

`Option` implements `IntoIterator`, so it can be chained with `extend` or `chain`. Prefer `if let` or `iter::once` when the intent is clearer.

```rust
let maybe: Option<i32> = Some(42);
let mut nums = vec![1, 2, 3];
nums.extend(maybe);
```

#### Temporary mutability

Source: [Temporary mutability](https://rust-unofficial.github.io/patterns/idioms/temporary-mutability.html) from the book.

Perform setup with a mutable binding, then rebind as immutable.

```rust
let mut data = vec![3, 1, 4, 1, 5];
data.sort();
let data = data; // now immutable
```

#### Privacy for extensibility

Source: [Priv extend](https://rust-unofficial.github.io/patterns/idioms/priv-extend.html) from the book.

Add a private field (or use `#[non_exhaustive]`) so the struct can gain fields without breaking downstream struct literals or exhaustive destructuring. The ergonomic cost is that downstream crates cannot construct the type with a struct literal. Cross-reference `docs/rust/api-design.md` for C-STRUCT-PRIVATE.

```rust
#[non_exhaustive]
#[derive(Debug)]
pub struct Options {
    pub timeout: Duration,
    _b: (),
}
```

#### On-stack dynamic dispatch

Source: [On-stack dyn dispatch](https://rust-unofficial.github.io/patterns/idioms/on-stack-dyn-dispatch.html) from the book.

Create a `&mut dyn Trait` from an `if` branch without heap allocation. Since Rust 1.79, temporary lifetimes are extended so this works cleanly.

```rust
pub trait Logger { fn log(&self, msg: &str); }
pub struct StderrLogger;
impl Logger for StderrLogger { fn log(&self, msg: &str) { eprintln!("{msg}"); } }
pub struct NullLogger;
impl Logger for NullLogger { fn log(&self, _msg: &str) {} }

pub fn run(verbose: bool) {
    let mut stderr = StderrLogger;
    let mut null = NullLogger;
    let logger: &mut dyn Logger = if verbose { &mut stderr } else { &mut null };
    logger.log("starting");
}
```

#### Collections as smart pointers / `Deref`

Source: [Deref](https://rust-unofficial.github.io/patterns/idioms/deref.html) from the book.

`Vec<T>` implements `Deref<Target = [T]>`, so a `&Vec<T>` coerces to `&[T]`. This is a smart-pointer use of `Deref`. Do not use `Deref` to emulate inheritance; use `Borrow`/`AsRef` for generic bounds that accept owned or borrowed values. Cross-reference the [Deref polymorphism](#deref-polymorphism) anti-pattern.

```rust
fn sum_slice(values: &[i32]) -> i32 { values.iter().sum() }
let v = vec![1, 2, 3];
let _ = sum_slice(&v);
```

#### `format!` for string concat

Source: [Concat format](https://rust-unofficial.github.io/patterns/idioms/concat-format.html) from the book.

Use `format!` for readability; it is not the fastest choice for hot paths. For known-small concatenations, `String::push_str` or a pre-allocated buffer may be faster: `let message = format!("Hello, {}!", "world");`.

#### Return consumed arg on error

Source: [Return consumed arg on error](https://rust-unofficial.github.io/patterns/idioms/return-consumed-arg-on-error.html) from the book.

When a fallible conversion consumes an owned value, return it back in the error so callers can retry without cloning. `String::from_utf8` is the canonical example; use `e.into_bytes()` to recover the original `Vec`.

#### Pass variables to closure

Source: [Pass var to closure](https://rust-unofficial.github.io/patterns/idioms/pass-var-to-closure.html) from the book.

When a `move` closure needs mixed capture modes, rebind variables to control what is moved and what is borrowed. For example, rebind a reference as `let data_ref = &data;` before the `move` closure to borrow rather than move `data`.

#### Easy doc init

Source: [Rustdoc init](https://rust-unofficial.github.io/patterns/idioms/rustdoc-init.html) from the book.

Hide setup boilerplate in doc tests with `# fn` lines so the example focuses on the API. Assertions inside hidden helpers will not run as part of the doc test. Cross-reference `docs/rust/documentation-guidelines.md`.

#### FFI idioms

Source: [FFI intro](https://rust-unofficial.github.io/patterns/idioms/ffi/intro.html), [errors](https://rust-unofficial.github.io/patterns/idioms/ffi/errors.html), [accepting-strings](https://rust-unofficial.github.io/patterns/idioms/ffi/accepting-strings.html), and [passing-strings](https://rust-unofficial.github.io/patterns/idioms/ffi/passing-strings.html) from the book.

- Accept borrowed `CStr` for incoming strings; convert to `CString` only when passing ownership to C.
- Keep `CString` alive for the duration of the pointer use.
- Map Rust errors to an integer error code (`c_int`) or an out-parameter for FFI boundaries.
- Expose safe wrappers around unsafe FFI declarations. Cross-reference `docs/rust/unsafe-security.md`.

#### `?` operator

Community-recognized / standard library; not a standalone book idiom page.

Propagate `Result` or `Option` errors with `?` instead of explicit `match` when the only action is returning the error. Cross-reference `docs/rust/error-handling.md`.

```rust
fn read_and_parse(path: &str) -> Result<i32, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    Ok(text.trim().parse()?)
}
```

#### `if let` / `let else`

Community-recognized; Rust 1.65+.

Use `if let` for a single variant and `let else` for early-return extraction. Cross-reference `docs/rust/types-traits-generics.md`.

```rust
if let Some(value) = optional { process(value); }

let Some(value) = optional else {
    return Err(Error::MissingValue);
};
```

#### `expect` / `unwrap` conventions

Community-recognized / standard library; not a standalone book idiom page.

Limit `unwrap()` and `expect()` to tests, prototypes, and documented invariants. Use `expect("invariant: ...")` to document why failure is impossible. Cross-reference `docs/rust/error-handling.md` and Clippy lints `unwrap_used`/`expect_used`/`panic`.

#### Use `Self`

Community-recognized / standard library; not a standalone book idiom page. Use `Self` inside `impl` blocks to reduce renaming churn: `impl Config { pub fn new() -> Self { Self { ... } } }`.

#### `matches!` macro

Community-recognized / standard library; not a standalone book idiom page. Use `matches!` for boolean pattern tests: `assert!(matches!(opt, Some(n) if n > 0));`.

#### Extension traits

Community-recognized; not a standalone book idiom page. Add methods to foreign types with a local trait; keep them scoped and documented.

#### Prefixed naming (`with_` / `into_` / `as_`)

Community-recognized / Rust API Guidelines; not a standalone book idiom page. Follow cost semantics: `as_` for free borrowed views, `to_` for expensive conversions, `into_` for consuming conversions, `with_` for secondary constructors. Cross-reference `docs/rust/api-design.md` for C-CONV.

### Anti-patterns

#### Clone to satisfy the borrow checker

Source: [Borrow clone](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html) from the book.

Adding `.clone()` to silence a borrow-checker error often hides a real ownership bug. The legitimate exception is `Rc`/`Arc::clone`, which clones a pointer rather than the data. Rule of thumb: if a `clone()` makes a borrow error disappear, it is suspect. Fix with `mem::take`/`mem::replace`, references, `Rc`/`Arc`, or restructuring ownership.

```rust
// BAD
fn process(data: &String) -> String { data.clone().to_uppercase() }

// GOOD
fn process(data: &str) -> String { data.to_uppercase() }
```

#### `#[deny(warnings)]`

Source: [Deny warnings](https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html) from the book.

Hard-coding `#![deny(warnings)]` breaks forward compatibility when new compiler versions introduce warnings. Treat warnings as errors in CI (`RUSTFLAGS="-D warnings"`) or maintain a curated allow-list, not in source. Cross-reference `docs/rust/lints-clippy.md`.

#### `Deref` polymorphism

Source: [Deref polymorphism](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html) from the book.

Do not emulate inheritance by implementing `Deref` on a struct so it gains methods from another type. It is harmful: no real subtyping, traits are not auto-implemented, behavior is surprising, and it only supports single inheritance. Fix with explicit facade methods, traits, or crates like `delegate`/`ambassador`.

```rust
// BAD
struct User { name: String }
impl std::ops::Deref for User {
    type Target = String;
    fn deref(&self) -> &String { &self.name }
}

// GOOD
struct User { name: String }
impl User {
    pub fn name(&self) -> &str { &self.name }
    pub fn into_name(self) -> String { self.name }
}
```

#### `unwrap()` / `expect()` / `panic!()` in production code

Community-recognized / Clippy; not in the book.

Avoid panicking in non-test library or application code. Use `?`, combinators, or explicit error handling. Enable `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` where appropriate. Cross-reference `docs/rust/error-handling.md`.

#### `Box<dyn Trait>` when generics would suffice

Community-recognized; not in the book.

Avoid unnecessary dynamic dispatch. Prefer generics when the type set is closed and known at compile time. Use `dyn Trait` for heterogeneous collections, plugin systems, or binary-size control. Cross-reference `docs/rust/types-traits-generics.md`.

#### Deeply nested `match`

Community-recognized; not in the book.

Flatten nested matches with `?`, `let else`, combinators, or helper functions. Deep nesting obscures the happy path.

#### `unsafe` without a `// SAFETY:` comment

Community-recognized; not in the book.

Every `unsafe` block must document the contract it upholds and why the surrounding code satisfies it. Cross-reference `docs/rust/unsafe-security.md`.

#### Stringly-typed data

Community-recognized; not in the book.

Use newtypes or enums for domain values instead of raw `String`. `UserId(u64)` is clearer and safer than `String`.

#### Boolean blindness

Community-recognized / Rust API Guidelines; not in the book.

Replace bare `bool` parameters with enums or newtypes so call sites are self-documenting. Cross-reference `docs/rust/api-design.md` for C-CUSTOM-TYPE.

#### Mutable global state

Community-recognized; not in the book.

Avoid `static mut`. Use `Atomic*`, `std::sync::OnceLock`, `std::sync::LazyLock`, or pass state explicitly. `static mut` requires `unsafe` for every access and is easy to misuse.

#### `catch_unwind` as error handling

Community-recognized; not in the book.

Use `catch_unwind` only at FFI boundaries, thread boundaries, or test harnesses where panic recovery is required. Normal error propagation should use `Result`.

#### Over-engineering / god structs / modules

Community-recognized; the book's [Patterns introduction](https://rust-unofficial.github.io/patterns/patterns/index.html) has a YAGNI note.

Avoid unnecessary abstraction layers and monolithic structs or modules. Apply patterns only when they solve a real problem.

#### `mem::forget` on owning types

Community-recognized; not in the book.

Calling `mem::forget` on a type that owns resources defeats RAII and can leak memory or file descriptors. Use it only for deliberate resource transfer where Drop must be suppressed, and document why.

### Pattern decisions

#### Trait objects vs generics

Choose based on the constraints below. Cross-reference `docs/rust/types-traits-generics.md` for object safety, monomorphization, and `dyn Trait` mechanics.

| Concern | Prefer generics | Prefer `dyn Trait` |
|---|---|---|
| Type heterogeneity | Closed, known set | Open, runtime-varying set |
| Performance | Hot path, need inlining | Accept vtable indirection |
| Binary size | Accept code duplication | Need to limit monomorphization |
| Compile time | Accept longer compiles | Need faster compiles |
| Trait requirements | Generic methods, `Self` return | Only object-safe methods |

#### Sealed traits

Use sealed traits to prevent downstream implementations, enabling you to add methods or change undocumented signatures without breaking external code. Cross-reference `docs/rust/api-design.md` for C-SEALED and `docs/rust/types-traits-generics.md` for mechanics.

```rust
mod private { pub trait Sealed {} }

pub trait Renderer: private::Sealed {
    fn render(&self, doc: &Document);
}

pub struct SvgRenderer;
impl private::Sealed for SvgRenderer {}
impl Renderer for SvgRenderer { fn render(&self, _doc: &Document) {} }

pub struct Document;
```

#### Interior mutability

Interior mutability moves Rust's aliasing rules from compile time to runtime. Use it as a family of patterns, not a single tool. Cross-reference `docs/rust/smart-pointers-memory.md` for the ownership-mutability matrix.

- `Cell<T>`: `Copy` types; zero runtime cost; replaces value wholesale.
- `RefCell<T>`: non-`Sync`, single-threaded; panics on aliasing violations.
- `Mutex<T>` / `RwLock<T>`: thread-safe shared mutation; blocks or panics on misuse.
- `Atomic*`: lock-free integer and flag operations; order carefully.

## Review checklist

- [ ] Newtypes, enums, and type-state are used to make invalid states unrepresentable.
- [ ] Builders are used for complex construction; simple types use `new`/`Default`.
- [ ] RAII guards and `Drop` implementations handle resource cleanup without panicking.
- [ ] Strategies are expressed through traits or closures; inheritance is not emulated.
- [ ] `Deref` is used only for smart-pointer semantics, not method sharing.
- [ ] Trait objects (`dyn Trait`) are used only where heterogeneity or binary size justifies dynamic dispatch.
- [ ] Cloning is justified; borrowing or restructuring ownership was considered first.
- [ ] `unwrap()` / `expect()` / `panic!()` are limited to tests and documented invariants.
- [ ] `unsafe` blocks have a `// SAFETY:` comment explaining the upheld contract.
- [ ] `mem::take` / `mem::replace` are preferred over clone-then-mutate for enum variants.
- [ ] Public types implement common standard traits eagerly (`Debug`, `Clone`, etc.).
- [ ] `#[deny(warnings)]` is not baked into source files; CI enforces warnings-as-errors.
- [ ] `Box<dyn Trait>` is not used when a generic parameter would suffice.
- [ ] Deeply nested `match` is flattened with `?`, `let else`, or combinators.
- [ ] Stringly-typed values and boolean blindness are replaced with newtypes or enums.
- [ ] Mutable global state is avoided; explicit state passing or `OnceLock`/`LazyLock` is used.

## Implementation checklist

- [ ] Identify the pattern that fits the problem before writing custom machinery.
- [ ] Add `Default` and `new()` to configuration-like types.
- [ ] Use builders for structs with many optional or validated fields.
- [ ] Document invariants for newtypes, type-state transitions, and `unsafe` blocks.
- [ ] Replace nested `match` with `?`, `let else`, or iterator combinators where possible.
- [ ] Choose generics vs `dyn Trait` deliberately and record the rationale for non-obvious choices.
- [ ] Seal traits that are implementation details of the public API.
- [ ] Implement `From`/`TryFrom`/`AsRef`/`AsMut` for canonical conversions.
- [ ] Use `Cell` for `Copy` interior mutability, `RefCell` for single-threaded non-`Copy`, and `Mutex`/`RwLock` for thread-safe shared mutation.
- [ ] Break `Rc`/`Arc` cycles with `Weak` where back-references exist.
- [ ] Add regression tests for type-state invalid transitions, visitor/fold traversal, and RAII cleanup.

## Validation hooks

Run these commands after changing pattern-heavy code:

```bash
cargo check --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

Optional targeted lints:

```bash
# Forbidding panic-prone production code
cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic

# Detecting suspicious clones and pointer types
cargo clippy -- -W clippy::clone_on_ref_ptr -W clippy::rc_mutex -W clippy::mutex_atomic
```

Run doc tests and documentation builds:

```bash
cargo test --doc
cargo doc --no-deps
```

For FFI or `unsafe` patterns, run under Miri:

```bash
cargo +nightly miri test
```

## Examples

### Type-state machine

```rust
pub struct Idle; pub struct Running;

pub struct Service<S> {
    name: String,
    _state: std::marker::PhantomData<S>,
}

impl Service<Idle> {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), _state: std::marker::PhantomData }
    }
    pub fn start(self) -> Service<Running> {
        Service { name: self.name, _state: std::marker::PhantomData }
    }
}

impl Service<Running> {
    pub fn stop(self) -> Service<Idle> {
        Service { name: self.name, _state: std::marker::PhantomData }
    }
}
```

### Non-consuming builder with validation

```rust
use std::time::Duration;

#[derive(Debug, Default)]
pub struct RequestBuilder {
    method: String, uri: String, timeout: Option<Duration>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError { #[error("missing URI")] MissingUri }

impl RequestBuilder {
    pub fn new(method: impl Into<String>, uri: impl Into<String>) -> Self {
        Self { method: method.into(), uri: uri.into(), ..Default::default() }
    }
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout); self
    }
    pub fn build(&self) -> Result<Request, BuildError> {
        if self.uri.is_empty() { return Err(BuildError::MissingUri); }
        Ok(Request {
            method: self.method.clone(),
            uri: self.uri.clone(),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}

pub struct Request { method: String, uri: String, timeout: Duration }
```

### Strategy with static and dynamic dispatch

```rust
pub trait Formatter { fn format(&self, record: &Record) -> String; }

pub struct TextFormatter;
impl Formatter for TextFormatter {
    fn format(&self, r: &Record) -> String { format!("[{}] {}", r.level, r.message) }
}

pub struct JsonFormatter;
impl Formatter for JsonFormatter {
    fn format(&self, r: &Record) -> String {
        format!("{{\"level\":\"{}\",\"message\":\"{}\"}}", r.level, r.message)
    }
}

pub struct Record { pub level: String, pub message: String }

fn emit_static<F: Formatter>(formatter: &F, record: &Record) {
    println!("{}", formatter.format(record));
}

fn emit_dynamic(formatter: &dyn Formatter, record: &Record) {
    println!("{}", formatter.format(record));
}
```

## Common mistakes

1. **Using `unwrap()` in production code.** Every `unwrap()` is a latent panic. Prefer `?`, `if let`, or explicit error handling. See `docs/rust/error-handling.md`.
2. **Cloning to silence borrow-checker errors.** If `.clone()` makes a borrow error disappear, investigate whether a reference, `mem::take`, `Rc`/`Arc`, or ownership restructuring would solve it more honestly.
3. **Implementing `Deref` to share methods.** Only smart pointers should implement `Deref`. Use facade methods, traits, or delegation crates instead.
4. **`Box<dyn Trait>` for every abstraction.** Dynamic dispatch has a runtime cost and prevents inlining. Use generics when the concrete type is known at compile time.
5. **Deeply nested `match` instead of `?` or `let else`.** Nested matches obscure the happy path and increase the chance of inconsistent error handling.
6. **`unsafe` without a safety comment.** Every `unsafe` block must explain the contract and why it is upheld. See `docs/rust/unsafe-security.md`.
7. **Stringly-typed domain values.** Using `String` for identifiers, statuses, or modes removes compile-time guarantees. Use newtypes and enums.
8. **Boolean blindness.** Bare `bool` parameters make call sites ambiguous. Replace them with enums or newtypes.
9. **Mutable global state with `static mut`.** Use `Atomic*`, `OnceLock`, `LazyLock`, or explicit state passing.
10. **`#[deny(warnings)]` in library source.** This breaks forward compatibility. Enforce warnings-as-errors in CI instead.
11. **Holding `Ref`/`RefMut` guards for too long.** Long-lived `RefCell` borrows increase the risk of runtime panics. Scope them tightly or clone the value out.
12. **Over-engineering with patterns.** Apply patterns when they solve a real problem. The book's Patterns introduction warns against premature abstraction.

## Strict vs contextual guidance

### Strict

- `unwrap()` / `expect()` / `panic!()` are forbidden outside tests and documented invariants.
- `unsafe` blocks require a `// SAFETY:` or `# Safety` comment.
- `Deref` is only for smart-pointer semantics.
- `Drop` implementations must never panic.
- `#[deny(warnings)]` must not appear in source; use CI for warnings-as-errors.
- Library error types implement `std::error::Error + Send + Sync + 'static`.
- `static mut` is avoided; explicit state passing or safe sync primitives are used instead.
- Newtypes and enums are preferred over stringly-typed data and boolean parameters.

### Contextual

- Builder pattern is recommended for complex construction, not mandatory.
- Type-state is useful when state transitions are part of the domain; skip it for trivial or data-driven state.
- `dyn Trait` is acceptable when heterogeneity, binary size, or compile time is a concern.
- `Rc` vs `Arc` depends on whether the value may cross thread boundaries.
- `RefCell` is acceptable for single-threaded interior mutability when exterior mutability is infeasible.
- Visitor and Fold are appropriate for recursive heterogeneous data; prefer iterators for homogeneous data.
- Small crates are a goal for large codebases, but micro-crate fragmentation can be counterproductive.

## Policy decisions for individual repos

Each repository should document its own policy for:

- Whether `unwrap()` / `expect()` / `panic!()` are banned in production code and which clippy lints enforce the ban.
- The required review process and safety-comment format for `unsafe` code.
- Whether `anyhow` is allowed in library crates or reserved for binaries.
- The default choice between generics and `dyn Trait` for internal abstractions.
- Whether `Rc` or `Arc` is the default for shared ownership.
- When `RefCell`, `Mutex`, `RwLock`, or atomics are permitted for interior mutability.
- The maximum recommended iterator chain length before extracting a helper or switching to a `for` loop.
- How FFI boundaries are wrapped and documented.

## Related docs

- `docs/rust/api-design.md` — naming, conversions, builders, newtypes, sealed traits, and Rust API Guidelines checklist items.
- `docs/rust/types-traits-generics.md` — structs, enums, traits, generics, object safety, conversions, and `PhantomData` mechanics.
- `docs/rust/iterators-closures.md` — closures, iterator chains, `collect` inference, `into_iter` ownership, and functional combinators.
- `docs/rust/error-handling.md` — `?` operator, error enums, `thiserror`/`anyhow`, and `unwrap`/`expect` conventions.
- `docs/rust/smart-pointers-memory.md` — `Box`/`Rc`/`Arc`/`RefCell`/`Cell`/`Mutex`, `Drop`, Deref coercion, and interior mutability.
- `docs/rust/ownership-lifetimes.md` — ownership, borrowing, lifetimes, and variance.
- `docs/rust/modules-visibility.md` — module structure, visibility, and crate organization.
- `docs/rust/documentation-guidelines.md` — doc comments, doc tests, and rustdoc conventions.
- `docs/rust/lints-clippy.md` — clippy lint configuration and warnings-as-errors policy.
- `docs/rust/unsafe-security.md` — `unsafe` blocks, safety comments, and FFI wrapping.
- `docs/rust/style-formatting.md` — formatting, naming style, and code style conventions.

## Related skills

- None defined yet.
