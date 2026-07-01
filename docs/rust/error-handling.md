# Error Handling in Rust

## Purpose

This document is a comprehensive, opinionated reference on Rust error handling for future AI coding agents. It covers the language's two primary carrier types (`Option<T>` and `Result<T, E>`), the `?` operator, panic semantics, error-type design, backtraces, and the ecosystem crates (`thiserror`, `anyhow`, `miette`, `color-eyre`) most commonly used to implement those designs. The goal is to produce code that is safe, idiomatic, and easy to review, while avoiding both the overuse of `unwrap` and the over-engineering of error hierarchies.

## Sources used

### The Rust Programming Language, Chapter 9

- https://doc.rust-lang.org/book/ch09-00-error-handling.html
- https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html
- https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html
- https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

### Standard library: `Option`, `Result`, formatting, and panic macros

- https://doc.rust-lang.org/std/option/
- https://doc.rust-lang.org/std/option/enum.Option.html
- https://doc.rust-lang.org/std/result/
- https://doc.rust-lang.org/std/result/enum.Result.html
- https://doc.rust-lang.org/std/error/trait.Error.html
- https://doc.rust-lang.org/std/fmt/trait.Display.html
- https://doc.rust-lang.org/std/fmt/trait.Debug.html
- https://doc.rust-lang.org/std/macro.panic.html
- https://doc.rust-lang.org/std/macro.todo.html
- https://doc.rust-lang.org/std/macro.unimplemented.html
- https://doc.rust-lang.org/std/macro.unreachable.html

### Standard library: conversion, backtrace, panic runtime, and process abort

- https://doc.rust-lang.org/std/convert/trait.From.html
- https://doc.rust-lang.org/std/ops/trait.Try.html
- https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html
- https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
- https://doc.rust-lang.org/std/panic/fn.set_hook.html
- https://doc.rust-lang.org/std/panic/struct.PanicHookInfo.html
- https://doc.rust-lang.org/std/process/fn.abort.html

### Cargo and project configuration

- https://doc.rust-lang.org/cargo/reference/profiles.html

### Ecosystem error crates

- https://docs.rs/thiserror/latest/thiserror/
- https://docs.rs/anyhow/latest/anyhow/
- https://docs.rs/miette/latest/miette/
- https://docs.rs/color-eyre/latest/color_eyre/

Claims below are inline-cited to the specific URLs above.

## Core guidance

### Option vs Result

Rust uses `Option<T>` when a value may be absent, and `Result<T, E>` when an operation can fail with a cause:

> Rust doesn't have exceptions. Instead, it has the type `Result<T, E>` for recoverable errors and the `panic!` macro that stops execution when the program encounters an unrecoverable error.
> — https://doc.rust-lang.org/book/ch09-00-error-handling.html

> `Option<T>` is an optional value: every `Option` is either `Some` and contains a value, or `None`.
> — https://doc.rust-lang.org/std/option/

Use `Option` for ordinary absence (a key not in a map, a missing optional field, the end of an iterator). Use `Result` when the caller needs to distinguish *why* something failed and may want to recover. If you start with `Option` but later need to explain the absence, convert explicitly with `ok_or` or `ok_or_else`.

```rust
fn find_user(id: u64) -> Option<User> {
    // `None` means "no user found" — expected and recoverable.
}

fn read_config(path: &str) -> Result<Config, io::Error> {
    // `Err` means "the read failed" — caller must decide what to do.
}
```

### Result is `#[must_use]`

The compiler warns when a `Result` is silently discarded:

> `Result` is annotated with the `#[must_use]` attribute, which will cause the compiler to issue a warning when a `Result` value is ignored.
> — https://doc.rust-lang.org/std/result/

This is a feature, not a nuisance. Always handle the `Result`, propagate it with `?`, or explicitly drop it with `let _ = ...` when discarding is intentional.

### Panic vs Result

`panic!` is for bugs and invariant violations, not for expected runtime failures. The Book gives a clear decision framework:

> A *bad state* is when some assumption, guarantee, contract, or invariant has been broken... plus one or more of the following: The bad state is something that is unexpected... Your code after this point needs to rely on not being in this bad state... There's not a good way to encode this information in the types you use.
> — https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

Return `Result` for expected failures (missing files, network timeouts, malformed user input, rate limits). Panic only when the caller has a bug, when continuing would be unsafe, or when the failure represents a violated contract that the type system cannot rule out. The Book summarizes the library-vs-application split:

> Library code should return an error if you can... panic only where continuing could be insecure or harmful.
> — https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

> When failure is expected, it's more appropriate to return a `Result`.
> — https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

### Error type design at a glance

| Context | Recommended approach |
|--------|----------------------|
| Stable public library | Structured `enum` error, derive with `thiserror`, implement `std::error::Error + Send + Sync + 'static`, mark `#[non_exhaustive]`; see `docs/rust/api-design.md` (C-GOOD-ERR) |
| Internal library / workspace crate | `thiserror` enum or `Box<dyn std::error::Error + Send + Sync>` depending on whether callers need to match |
| Application / CLI / binary | `anyhow::Result<T>` with `.context()` enrichment; consider `miette` or `color-eyre` for rich diagnostics |
| Prototype / quick script | `Box<dyn std::error::Error>` is acceptable but limits callers |

The rule of thumb: libraries expose structured errors so downstream code can match; applications usually just want to report failures to humans.

## Practical rules

### 1. Choose `Option` for absence, `Result` for failure-with-cause

If the only information is "there is no value," use `Option`. If the caller might need to react differently depending on *why* the operation failed, use `Result`. Do not smuggle error semantics into `Option` via side channels.

### 2. Panic only on contract violations and bugs

Panicking is appropriate when:

- An index is out of bounds.
- A division by zero occurs.
- A constructor invariant is violated by the caller.
- A branch is logically unreachable because all variants were handled.

Panicking is *not* appropriate for expected runtime conditions such as a missing file, a refused connection, or invalid user input.

### 3. Use combinators before explicit `match` for simple transformations

`Option` and `Result` provide a rich combinator API. Prefer combinators when the logic is a single transformation, filter, default, or chaining step. Use `match`, `if let`, or `let-else` when the branches are large or asymmetric.

The module docs describe `and_then` as the canonical way to chain fallible operations:

> `and_then` is often used to chain fallible operations that may return `None`/`Err`.
> — https://doc.rust-lang.org/std/option/
> — https://doc.rust-lang.org/std/result/

`map_err` is the standard way to convert or annotate an error type:

> Maps a `Result<T, E>` to `Result<T, F>` by applying a function to a contained `Err` value, leaving an `Ok` value untouched. This function can be used to pass through a successful result while handling an error.
> — https://doc.rust-lang.org/std/result/enum.Result.html#method.map_err

### 4. Eager-vs-lazy rule

Many methods come in eager/lazy pairs. The eager variant evaluates its fallback value immediately, even if it is never used. The lazy variant takes a closure and evaluates it only when needed:

> Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing the result of a function call, it is recommended to use `unwrap_or_else`, which is lazily evaluated.
> — https://doc.rust-lang.org/std/option/enum.Option.html

This rule applies to `unwrap_or`/`unwrap_or_else`, `map_or`/`map_or_else`, `ok_or`/`ok_or_else`, `and`/`and_then`, and `or`/`or_else`.

### 5. Prefer `?` over manual early returns

The `?` operator propagates `Err` or `None` early and keeps success-path code readable:

> If the value of the `Result` is an `Ok`, the value inside the `Ok` will get returned... If the value is an `Err`, the `Err` will be returned from the whole function as if we had used the `return` keyword.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

`?` also performs automatic error conversion:

> Error values that have the `?` operator called on them go through the `from` function, defined in the `From` trait... the error type received is converted into the error type defined in the return type of the current function.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

`?` is allowed only in functions whose return type supports it:

> The `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`).
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

You cannot mix `Result` and `Option` freely with `?`:

> You can't mix and match. The `?` operator won't automatically convert a `Result` to an `Option`... in those cases, you can use methods like the `ok` method on `Result` or the `ok_or` method on `Option`.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

#### Stable boundary of `?`

On stable Rust, `?` is hardcoded to work only on a fixed set of carrier types: `Option<T>`, `Result<T, E>`, and (inside combinators that accept them) `Poll<Option<Result<T, E>>>` / `Poll<Result<T, E>>`. You cannot make `?` work on your own custom control-flow type on stable Rust.

> `Try` (and its companion `FromResidual`) is the trait that `?` desugars through... This is a nightly-only experimental API (`try_trait_v2`).
> — https://doc.rust-lang.org/std/ops/trait.Try.html

Generalizing `?` to a custom type requires the nightly feature `try_trait_v2` (RFC 3058, tracking issue rust-lang/rust#84277, still open). Practical implications:

- Do not write `impl Try for MyType` expecting `?` to work; it will not compile on stable Rust.
- The `Residual` associated type "colors" each carrier (`Result`'s residual is `Result<Infallible, E>`, `Option`'s is `Option<Infallible>`) so that `?` cannot silently cross between `Result` and `Option` — which is why you must convert explicitly with `ok_or`/`ok_or_else`, `Result::ok`, or `transpose`.
- Treat custom `Try` impls as unstable; never ship them in a library targeting stable Rust.

### 6. `?` in `main`

`main` may return a `Result`:

> Change `main`'s return type to `Result<(), Box<dyn Error>>`; the executable will exit with a value of `0` if `main` returns `Ok(())` and will exit with a nonzero value if `main` returns an `Err` value.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

In practice, `fn main() -> anyhow::Result<()>` is idiomatic for applications. More generally, `main` may return any type implementing `std::process::Termination`.

### 7. `unwrap` and `expect` are for tests, prototypes, and documented invariants

Both `unwrap` and `expect` panic on failure:

> If the `Result` value is the `Ok` variant, `unwrap` will return the value... If the `Result` is the `Err` variant, `unwrap` will call the `panic!` macro.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

> The `expect` method lets us also choose the `panic!` error message.
> — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html

Because of the panic risk, their use is discouraged in production code:

> Because this function may panic, its use is generally discouraged... Instead, prefer to use the `?` (try) operator, or pattern matching... or call `unwrap_or`, `unwrap_or_else`, or `unwrap_or_default`.
> — https://doc.rust-lang.org/std/result/enum.Result.html#method.unwrap
> — https://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap

If you must use `expect`, document the invariant in the message:

> We recommend that `expect` messages are used to describe the reason you *expect* the `Result` should be `Ok`.
> — https://doc.rust-lang.org/std/result/enum.Result.html#method.expect

Good message style focuses on the "should": `"env variable PORT should be set by the deployment script"`.

### 8. `Option` combinator catalog

| Method | Signature shape | Consumes/Borrows | Lazy/Eager | One-line use |
|--------|-----------------|------------------|------------|--------------|
| `is_some` | `fn is_some(&self) -> bool` | borrows | eager | `if opt.is_some()` |
| `is_none` | `fn is_none(&self) -> bool` | borrows | eager | `if opt.is_none()` |
| `is_some_and` | `fn is_some_and(self, f: impl FnOnce(T) -> bool) -> bool` | consumes | eager (predicate lazy) | `opt.is_some_and(\|x\| x > 0)` |
| `is_none_or` | `fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool` | consumes | eager (predicate lazy) | `opt.is_none_or(\|x\| x > 0)` |
| `map` | `fn map<U, F>(self, f: F) -> Option<U>` | consumes | lazy (f if `Some`) | `opt.map(\|x\| x.len())` |
| `map_or` | `fn map_or<U, F>(self, default: U, f: F) -> U` | consumes | eager default, lazy `f` | `opt.map_or(0, \|x\| x.len())` |
| `map_or_else` | `fn map_or_else<U, D, F>(self, default: D, f: F) -> U` | consumes | lazy | `opt.map_or_else(Default::default, \|x\| x.len())` |
| `and` | `fn and<U>(self, opt: Option<U>) -> Option<U>` | consumes | eager | `a.and(b)` |
| `and_then` | `fn and_then<U, F>(self, f: F) -> Option<U>` | consumes | lazy | `opt.and_then(parse)` |
| `or` | `fn or(self, opt: Option<T>) -> Option<T>` | consumes | eager | `a.or(b)` |
| `or_else` | `fn or_else<F>(self, f: F) -> Option<T>` | consumes | lazy | `opt.or_else(alternative)` |
| `xor` | `fn xor(self, opt: Option<T>) -> Option<T>` | consumes | eager | `a.xor(b)` |
| `filter` | `fn filter<P>(self, predicate: P) -> Option<T>` | consumes | lazy (predicate if `Some`) | `opt.filter(\|x\| x > 0)` |
| `unwrap_or` | `fn unwrap_or(self, default: T) -> T` | consumes | eager | `opt.unwrap_or(default)` |
| `unwrap_or_else` | `fn unwrap_or_else<F>(self, f: F) -> T` | consumes | lazy | `opt.unwrap_or_else(default_fn)` |
| `unwrap_or_default` | `fn unwrap_or_default(self) -> T where T: Default` | consumes | lazy | `opt.unwrap_or_default()` |
| `unwrap` | `fn unwrap(self) -> T` | consumes | eager (panics) | avoid in production |
| `expect` | `fn expect(self, msg: &str) -> T` | consumes | eager (panics) | `opt.expect("invariant: ...")` |
| `take` | `fn take(&mut self) -> Option<T>` | borrows mut | eager | `opt.take()` |
| `replace` | `fn replace(&mut self, value: T) -> Option<T>` | borrows mut | eager | `opt.replace(v)` |
| `insert` | `fn insert(&mut self, value: T) -> &mut T` | borrows mut | eager | `opt.insert(v)` |
| `get_or_insert` | `fn get_or_insert(&mut self, v: T) -> &mut T` | borrows mut | eager | `opt.get_or_insert(v)` |
| `get_or_insert_with` | `fn get_or_insert_with<F>(&mut self, f: F) -> &mut T` | borrows mut | lazy | `opt.get_or_insert_with(default_fn)` |
| `ok_or` | `fn ok_or<E>(self, err: E) -> Result<T, E>` | consumes | eager | `opt.ok_or(Error::Missing)` |
| `ok_or_else` | `fn ok_or_else<E, F>(self, f: F) -> Result<T, E>` | consumes | lazy | `opt.ok_or_else(\|\| Error::Missing)` |
| `zip` | `fn zip<U>(self, other: Option<U>) -> Option<(T, U)>` | consumes | eager | `a.zip(b)` |
| `zip_with` | `fn zip_with<U, F, R>(self, other: Option<U>, f: F) -> Option<R>` | consumes | lazy | `a.zip_with(b, combine)` |
| `as_ref` | `fn as_ref(&self) -> Option<&T>` | borrows | eager | `opt.as_ref()` |
| `as_mut` | `fn as_mut(&mut self) -> Option<&mut T>` | borrows mut | eager | `opt.as_mut()` |
| `as_deref` | `fn as_deref(&self) -> Option<&T::Target>` | borrows | eager | `opt.as_deref()` |
| `as_slice` | `fn as_slice(&self) -> &[T]` | borrows | eager | `opt.as_slice()` |
| `cloned` | `fn cloned(self) -> Option<T> where T: Clone` | consumes | lazy (clone if `Some`) | `opt_of_ref.cloned()` |
| `copied` | `fn copied(self) -> Option<T> where T: Copy` | consumes | lazy (copy if `Some`) | `opt_of_ref.copied()` |
| `iter` | `fn iter(&self) -> Iter<'_, T>` | borrows | eager | `opt.iter()` |
| `into_iter` | `fn into_iter(self) -> IntoIter<T>` | consumes | eager | `opt.into_iter()` |
| `transpose` | `fn transpose(self) -> Result<Option<T>, E>` for `Option<Result<T, E>>` | consumes | eager | `opt_of_result.transpose()` |
| `flatten` | `fn flatten(self) -> Option<T>` for `Option<Option<T>>` | consumes | eager | `opt.flatten()` |
| `inspect` | `fn inspect<F>(self, f: F) -> Option<T>` | consumes | lazy (f if `Some`) | `opt.inspect(\|x\| log!(x))` |

`Option<T>` is `Copy` only when `T: Copy`. When `T` is not `Copy`, use `as_ref`, `as_mut`, `as_deref`, `cloned`, or `copied` to avoid moving the inner value.

Rust also guarantees the null-pointer optimization (NPO) for pointer-like `T`:

> Rust guarantees to optimize the following types `T` such that `Option<T>` has the same size as `T`.
> — https://doc.rust-lang.org/std/option/

This includes `Box<U>`, `&U`, `&mut U`, `fn`, `NonZero*`, `NonNull`, and `#[repr(transparent)]` wrappers.

### 9. `Result` combinator catalog

| Method | Signature shape | Consumes/Borrows | Lazy/Eager | One-line use |
|--------|-----------------|------------------|------------|--------------|
| `is_ok` | `fn is_ok(&self) -> bool` | borrows | eager | `if res.is_ok()` |
| `is_err` | `fn is_err(&self) -> bool` | borrows | eager | `if res.is_err()` |
| `is_ok_and` | `fn is_ok_and(self, f: impl FnOnce(T) -> bool) -> bool` | consumes | eager (predicate lazy) | `res.is_ok_and(\|x\| x > 0)` |
| `is_err_and` | `fn is_err_and(self, f: impl FnOnce(E) -> bool) -> bool` | consumes | eager (predicate lazy) | `res.is_err_and(\|e\| e.is_not_found())` |
| `map` | `fn map<U, F>(self, f: F) -> Result<U, E>` | consumes | lazy (f if `Ok`) | `res.map(\|x\| x.len())` |
| `map_err` | `fn map_err<F, O>(self, op: O) -> Result<T, F>` | consumes | lazy (op if `Err`) | `res.map_err(Error::from)` |
| `map_or` | `fn map_or<U, F>(self, default: U, f: F) -> U` | consumes | eager default, lazy `f` | `res.map_or(0, \|x\| x.len())` |
| `map_or_else` | `fn map_or_else<U, D, F>(self, default: D, f: F) -> U` | consumes | lazy | `res.map_or_else(handle_err, process)` |
| `and` | `fn and<U>(self, res: Result<U, E>) -> Result<U, E>` | consumes | eager | `a.and(b)` |
| `and_then` | `fn and_then<U, F>(self, f: F) -> Result<U, E>` | consumes | lazy | `res.and_then(parse)` |
| `or` | `fn or<F>(self, res: Result<T, F>) -> Result<T, F>` | consumes | eager | `a.or(b)` |
| `or_else` | `fn or_else<F, O>(self, op: O) -> Result<T, F>` | consumes | lazy | `res.or_else(recover)` |
| `ok` | `fn ok(self) -> Option<T>` | consumes | eager (loses `Err`) | `res.ok()` |
| `err` | `fn err(self) -> Option<E>` | consumes | eager (loses `Ok`) | `res.err()` |
| `unwrap` | `fn unwrap(self) -> T` | consumes | eager (panics on `Err`) | avoid in production |
| `expect` | `fn expect(self, msg: &str) -> T` | consumes | eager (panics on `Err`) | `res.expect("invariant: ...")` |
| `unwrap_err` | `fn unwrap_err(self) -> E` | consumes | eager (panics on `Ok`) | tests only |
| `expect_err` | `fn expect_err(self, msg: &str) -> E` | consumes | eager (panics on `Ok`) | tests only |
| `unwrap_or` | `fn unwrap_or(self, default: T) -> T` | consumes | eager | `res.unwrap_or(default)` |
| `unwrap_or_else` | `fn unwrap_or_else<F>(self, f: F) -> T` | consumes | lazy | `res.unwrap_or_else(recover)` |
| `unwrap_or_default` | `fn unwrap_or_default(self) -> T where T: Default` | consumes | lazy | `res.unwrap_or_default()` |
| `as_ref` | `fn as_ref(&self) -> Result<&T, &E>` | borrows | eager | `res.as_ref()` |
| `as_mut` | `fn as_mut(&mut self) -> Result<&mut T, &mut E>` | borrows mut | eager | `res.as_mut()` |
| `as_deref` | `fn as_deref(&self) -> Result<&T::Target, &E>` | borrows | eager | `res.as_deref()` |
| `cloned` | `fn cloned(self) -> Result<T, &E> where T: Clone` | consumes | lazy (clone if `Ok`) | `res_of_ref.cloned()` |
| `copied` | `fn copied(self) -> Result<T, &E> where T: Copy` | consumes | lazy (copy if `Ok`) | `res_of_ref.copied()` |
| `iter` | `fn iter(&self) -> Iter<'_, T>` | borrows | eager | `res.iter()` |
| `into_iter` | `fn into_iter(self) -> IntoIter<T>` | consumes | eager | `res.into_iter()` |
| `transpose` | `fn transpose(self) -> Option<Result<T, E>>` for `Result<Option<T>, E>` | consumes | eager | `res_of_option.transpose()` |
| `inspect` | `fn inspect<F>(self, f: F) -> Result<T, E>` | consumes | lazy (f if `Ok`) | `res.inspect(log_ok)` |
| `inspect_err` | `fn inspect_err<F>(self, f: F) -> Result<T, E>` | consumes | lazy (f if `Err`) | `res.inspect_err(log_err)` |

`Result<T, E>` is `Copy` only when both `T: Copy` and `E: Copy`. When the inner types are not `Copy`, use `as_ref`/`as_mut`/`as_deref`/`cloned`/`copied` to borrow instead of move.

### 10. Short-circuiting `collect`

Collecting an iterator of `Result` or `Option` values short-circuits on the first failure:

> An iterator over a collection of `Result` values can be collected into a `Result<Collection, E>`; if any item is `Err`, that error is returned.
> — https://doc.rust-lang.org/std/result/
> — https://doc.rust-lang.org/std/option/

```rust
let nums: Result<Vec<u32>, ParseIntError> = strings.iter().map(|s| s.parse()).collect();
```

This is one of the most powerful combinators for bulk validation.

### 11. Implement `std::error::Error` correctly

`std::error::Error` is the contract for error values:

> `Error` is a trait representing the basic expectations for error values, i.e., values of type `E` in `Result<T, E>`.
> — https://doc.rust-lang.org/std/error/trait.Error.html

Its supertraits are `Debug + Display`, and it has no required methods on stable Rust. A minimal manual implementation looks like:

```rust
#[derive(Debug)]
pub struct FooError { ... }

impl fmt::Display for FooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { ... }
}

impl Error for FooError {}
```

Override `source()` only when wrapping an inner error. When you do wrap, either return the inner error from `source()` *or* render it in `Display`, but not both:

> In error types that wrap an underlying error, the underlying error should be either returned by the outer error's `Error::source()`, or rendered by the outer error's `Display` implementation, but not both.
> — https://doc.rust-lang.org/std/error/trait.Error.html

Error messages should be concise:

> Error messages are typically concise lowercase sentences without trailing punctuation.
> — https://doc.rust-lang.org/std/error/trait.Error.html

### 12. `Display` is for humans, `Debug` is for programmers

`Display` cannot be derived and is intended for user-facing output:

> `Display` is for user-facing output, and so cannot be derived.
> — https://doc.rust-lang.org/std/fmt/trait.Display.html

`Debug` should usually be derived and is meant for diagnostics:

> `Debug` should format the output in a programmer-facing, debugging context.
> — https://doc.rust-lang.org/std/fmt/trait.Debug.html

User-facing error output must come from `Display`. Logs, crash reports, and developer traces may use `Debug`. Derived `Debug` output is not stable across compiler versions, so do not parse it.

### 13. Use `From` for infallible error conversion

`From` must never fail:

> This trait must not fail. The `From` trait is intended for perfect conversions. If the conversion can fail or is not perfect, use `TryFrom`.
> — https://doc.rust-lang.org/std/convert/trait.From.html

For error handling, `From` is what makes `?` ergonomic:

> The `From` trait is also very useful when performing error handling... `From` simplifies error handling by allowing a function to return a single error type that encapsulates multiple error types... The `?` operator automatically converts the underlying error type to our custom error type with `From::from`.
> — https://doc.rust-lang.org/std/convert/trait.From.html

The blanket `impl<T, U> Into<U> for T where U: From<T>` means you always implement `From` and get `Into` for free; see `docs/rust/types-traits-generics.md` for the full conversion-trait mechanics.

### 14. Use `thiserror` for library error enums

`thiserror` reduces boilerplate without leaking into your public API:

> This library provides a convenient derive macro for the standard library's `std::error::Error` trait.
> — https://docs.rs/thiserror/latest/thiserror/

Key attributes:

- `#[error("...")]` — `Display` format.
- `#[from]` — generates `From` impl; always implies `#[source]`.
- `#[source]` — returned by `Error::source()`.
- `#[error(transparent)]` — forwards `Display` and `source()` straight through.
- `#[backtrace]` — captures a backtrace on nightly Rust 1.73+.

`#[from]` has a restriction:

> A `From` impl is generated for each variant that contains a `#[from]` attribute... always implies that the same field is `#[source]`. The variant must contain no other fields (except a backtrace).
> — https://docs.rs/thiserror/latest/thiserror/

`#[source]` selects the cause:

> The Error trait's `source()` method is implemented to return whichever field has a `#[source]` attribute or is named `source`, if any.
> — https://docs.rs/thiserror/latest/thiserror/

Transparent errors are useful for catch-all variants:

> `#[error(transparent)]` forwards source and Display straight through to an underlying error without adding a message.
> — https://docs.rs/thiserror/latest/thiserror/

And `thiserror` does not appear in your public API:

> Thiserror deliberately does not appear in your public API... switching from handwritten impls to thiserror or vice versa is not a breaking change.
> — https://docs.rs/thiserror/latest/thiserror/

### 15. Use `anyhow` for application errors

`anyhow` is the idiomatic choice when an application just needs to report failures to a human:

> This library provides `anyhow::Error`, a trait object based error type for easy idiomatic error handling in Rust applications.
> — https://docs.rs/anyhow/latest/anyhow/

`anyhow::Result<T>` is `Result<T, anyhow::Error>`. It is like `Box<dyn std::error::Error>` but with stricter bounds and a guaranteed backtrace:

> `Error` works a lot like `Box<dyn std::error::Error>`, but with these differences: ... requires that the error is `Send`, `Sync`, and `'static`. ... guarantees that a backtrace is available ... is represented as a narrow pointer — exactly one word in size instead of two.
> — https://docs.rs/anyhow/latest/anyhow/struct.Error.html

Attach context at domain boundaries:

> `.context(C)` wraps the error with additional context; `.with_context(|| ...)` is lazy (evaluated only on error).
> — https://docs.rs/anyhow/latest/anyhow/trait.Context.html

Use the macros for early returns:

- `anyhow!(...)` — construct an ad-hoc error.
- `bail!(...)` — `return Err(anyhow!(...))`.
- `ensure!(cond, ...)` — `if !cond { return Err(anyhow!(...)); }`.

Display rules for anyhow:

- `{}` / `to_string()` prints only the outermost layer.
- `{:#}` (alternate) prints the full cause chain.
- `{:?}` (Debug) prints the chain plus the captured backtrace.

Do **not** expose `anyhow::Error` in a public library API. It erases structured information that downstream callers need. See `docs/rust/api-design.md` (C-STABLE, C-GOOD-ERR).

### 16. Downcast errors only when `T: Error + 'static`

On `dyn Error`, the downcast methods require a `'static` target:

```rust
fn is_not_found(err: &dyn std::error::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}
```

`Box<dyn Error>::downcast::<T>()` returns `Result<Box<T>, Box<dyn Error>>`. The `'static` bound is mandatory for all downcasting because the type must be recoverable from the trait object vtable.

`anyhow::Error::downcast_ref` is stricter still: the target must be `Display + Debug + Send + Sync + 'static`.

### 17. Choose reporting crates deliberately

| Crate | Best for |
|-------|----------|
| `anyhow` | Applications that need easy propagation and human-readable chains |
| `thiserror` | Libraries that need structured, matchable error enums |
| `miette` | CLIs, parsers, compilers, and anywhere you want source-code snippets, error codes, and ANSI/Unicode diagnostics |
| `color-eyre` | Applications built on `eyre` that want colored reports, custom sections, and tracing integration |

`miette` provides a diagnostic protocol compatible with `std::error::Error`, fancy output, source-code snippets with labels, error codes, and cause-chain printing. `color-eyre` is a colored error-report handler for `eyre` with verbosity levels for `SpanTrace`/`Backtrace`, custom sections, multi-error aggregation, and `tracing-error` integration.

### 18. Panic strategy: unwind vs abort

By default a panic unwinds the stack, running destructors:

> By default, when a panic occurs, the program starts *unwinding*... Rust therefore allows you to choose the alternative of immediately *aborting*, which ends the program without cleaning up.
> — https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html

Set `panic = 'abort'` in `[profile.release]` to reduce binary size and avoid unwind tables in release binaries:

```toml
[profile.release]
panic = "abort"
```

> Set `panic = 'abort'` in `[profile.release]` in Cargo.toml.
> — https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html

Unwind is still forced for tests, benchmarks, build scripts, proc macros, and some targets regardless of the profile setting:

> Tests/benches/build-scripts/proc-macros IGNORE the setting and force unwind on deps; some targets (NVPTX) always abort.
> — https://doc.rust-lang.org/cargo/reference/profiles.html

`std::process::abort()` is harsher than `panic!` with `panic = "abort"`:

> `abort()` runs NO destructors, flushes no buffers, does NOT call the panic hook; SIGABRT on Unix.
> — https://doc.rust-lang.org/std/process/fn.abort.html

### 19. Backtraces

`std::backtrace::Backtrace` is stable since Rust 1.65. `Backtrace::capture()` respects the environment:

> `RUST_LIB_BACKTRACE` consulted first; if unset, `RUST_BACKTRACE`; if both unset, capture is disabled. State cached after first backtrace.
> — https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html

`Backtrace::force_capture()` always captures, ignoring the environment; `Backtrace::disabled()` constructs an empty `Backtrace` without capturing.

For panics, set `RUST_BACKTRACE` to any value except `0`:

> Set the `RUST_BACKTRACE` environment variable to any value except `0` to get a backtrace; debug symbols must be enabled (on by default without `--release`).
> — https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html

On stable Rust, if you want a backtrace inside your own error type, store a `Backtrace` field at construction and print it in `Display`/`Debug`. `std::error::Error::backtrace()` does not exist on stable; backtrace access through `Error::provide()` is nightly-only (`error_generic_member_access`).

### 20. Panic hooks and catching panics

`panic!` is the macro for bugs:

> Panics the current thread.
> — https://doc.rust-lang.org/std/macro.panic.html

> The `panic!` macro is used to construct errors that represent a bug that has been detected in your program.
> — https://doc.rust-lang.org/std/macro.panic.html

The default hook prints the payload plus file/line/column to stderr. A main-thread panic exits with code 101.

Register a custom hook with `std::panic::set_hook`. The hook runs before the panic runtime and works for both unwind and abort:

> `set_hook` / `take_hook`: registers a global `Fn(&PanicHookInfo<'_>) + Sync + Send + 'static` that runs BEFORE the panic runtime (works for both unwind and abort).
> — https://doc.rust-lang.org/std/panic/fn.set_hook.html

`PanicHookInfo` (stable since Rust 1.81) provides `payload()`, `payload_as_str()` (1.91), and `location()`.

`catch_unwind` is not a general try/catch mechanism:

> It is **not** recommended to use this function for a general try/catch mechanism. The `Result` type is more appropriate... It only catches unwinding panics, not those that abort the process.
> — https://doc.rust-lang.org/std/panic/fn.catch_unwind.html

Use it only at FFI boundaries, in test frameworks, and in other situations where stopping an unwinding panic is required. The closure must be `UnwindSafe`; use `AssertUnwindSafe` to opt in. Dropping the `Err` payload may itself panic.

### 21. `todo!`, `unimplemented!`, and `unreachable!`

All three are `panic!` shorthands, but they signal different intent:

- `todo!()` — unfinished code you intend to implement later.
- `unimplemented!()` — functionality intentionally not implemented (possibly never will be).
- `unreachable!()` — code path that should be impossible if invariants hold.

> `todo!` indicates unfinished code... just a shorthand for `panic!` with a fixed, specific message.
> — https://doc.rust-lang.org/std/macro.todo.html

> `unimplemented!` indicates unimplemented code by panicking with a message of 'not implemented'.
> — https://doc.rust-lang.org/std/macro.unimplemented.html

> `unreachable!` indicates unreachable code. This is useful any time that the compiler can't determine that some code is unreachable.
> — https://doc.rust-lang.org/std/macro.unreachable.html

The unsafe counterpart of `unreachable!()` is `unreachable_unchecked()`. If the latter is ever reached, the program has undefined behavior.

`todo!` in particular is easy to leave in shipped code. Treat it as a release blocker.

### 22. `assert!` vs `debug_assert!`

`assert!` is always evaluated, even in release builds. `debug_assert!` is compiled out in release (gated by `-C debug-assertions`), though it is still type-checked regardless. Use `assert!` for invariants that must hold in production; use `debug_assert!` for expensive checks that are acceptable to skip in release.

## Review checklist

- [ ] Expected failures return `Result`, not `panic!`.
- [ ] `Option` is used for ordinary absence; `Result` is used for failure-with-cause.
- [ ] `unwrap` / `expect` in production code are justified with a comment explaining the invariant.
- [ ] `unwrap` / `expect` messages describe why the value *should* be `Ok`/`Some`.
- [ ] Error types implement `Display` and `std::error::Error`.
- [ ] Public library errors are `Send + Sync + 'static`; see `docs/rust/api-design.md` (C-GOOD-ERR).
- [ ] `From` conversions are provided for every `?` propagation path.
- [ ] `source()` chains underlying errors, and the same inner error is not also rendered by `Display`.
- [ ] User-facing messages come from `Display`, not `Debug`.
- [ ] Domain boundaries attach context (`.context()`, `map_err`, or a custom wrapper).
- [ ] `anyhow::Error` is not exposed in a public library API.
- [ ] `todo!` / `unimplemented!` / `unreachable!` are not used in shipped code without a documented reason.
- [ ] `catch_unwind` is used only at FFI boundaries or in test frameworks, never as general error handling.
- [ ] Public error enums are marked `#[non_exhaustive]`; see `docs/rust/api-design.md`.
- [ ] `panic = "abort"` is a deliberate release-profile choice, not the default.

## Implementation checklist

- [ ] Identify whether the crate is a library or application and choose `thiserror` or `anyhow` accordingly.
- [ ] Choose the error representation: structured enum, `Box<dyn Error>`, `anyhow::Error`, or a reporting crate.
- [ ] Define the error enum and derive or implement `Debug`, `Display`, and `std::error::Error`.
- [ ] Add `From` impls or `#[from]` attributes for every wrapped error type.
- [ ] Mark public error enums `#[non_exhaustive]` if the crate is published.
- [ ] Replace ad-hoc `unwrap` calls with `?`, combinators, or explicit error handling.
- [ ] Convert `Option` to `Result` with `ok_or_else` when the caller needs an error.
- [ ] Use `let-else` or `if let` for single-variant extraction instead of verbose `match`.
- [ ] Add `unreachable!` only where the invariant is truly guaranteed.
- [ ] Ensure `Debug` output is useful for logs; `Display` output is useful for humans.
- [ ] Decide the panic strategy (unwind vs abort) and document it in the crate README or `Cargo.toml` comments.
- [ ] Decide whether backtraces are captured and which environment variable activates them.

## Validation hooks

Run these checks after changing error-handling code:

```bash
# Fast feedback
cargo check
cargo test

# If clippy is available
cargo clippy -- -D warnings

# If formatting is enforced
cargo fmt --check

# Documentation examples
cargo test --doc
```

Pay attention to clippy lints such as:

- `clippy::unwrap_used` / `clippy::expect_used` — if your repo enables them.
- `clippy::panic` — if public APIs are required to avoid panics.
- `clippy::result_unit_err` — `Result<T, ()>` is usually a poor API.
- `clippy::option_option` — `Option<Option<T>>` often indicates a design issue.

For libraries, also run:

```bash
cargo doc --no-deps
cargo semver-checks
```

## Examples

### Example 1: Converting `Option` to `Result` with `ok_or_else`

Use `ok_or_else` when the error message is expensive to build or depends on local data:

```rust
fn user_by_id(id: u64) -> Result<User, Error> {
    cache
        .get(&id)
        .ok_or_else(|| Error::UserNotFound { id })
}
```

### Example 2: `?` propagation with `From`

```rust
use std::fs;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
enum LoadError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
}

fn load_number(path: &str) -> Result<i32, LoadError> {
    let contents = fs::read_to_string(path)?; // From<io::Error>
    let n = contents.trim().parse()?;         // From<ParseIntError>
    Ok(n)
}
```

### Example 3: Library error enum with `thiserror`

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
    #[error(transparent)]
    Other(#[from] ExternalError),
}
```

### Example 4: Application error handling with `anyhow`

```rust
use anyhow::{bail, ensure, Context, Result};

fn parse_port(s: &str) -> Result<u16> {
    ensure!(!s.is_empty(), "empty port string");
    let n: u16 = s
        .parse()
        .with_context(|| format!("cannot parse port from {s}"))?;
    if n == 0 {
        bail!("port cannot be zero");
    }
    Ok(n)
}
```

### Example 5: Manual `std::error::Error` implementation with `source()`

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Missing(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "io error"),
            LoadError::Missing(key) => write!(f, "missing {key}"),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadError::Io(e) => Some(e),
            LoadError::Missing(_) => None,
        }
    }
}
```

Note that the `Display` impl for `Io` does *not* also print the inner error; it delegates to `source()`.

### Example 6: Downcasting `dyn Error`

```rust
fn root_is_not_found(err: &dyn std::error::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}
```

### Example 7: Newtype validation pattern (the `Guess` pattern)

Use a private field and a constructor that panics on invalid input. Document the panic in `# Panics`.

```rust
/// A number between 1 and 100.
pub struct Guess {
    value: i32,
}

impl Guess {
    /// Creates a new `Guess`.
    ///
    /// # Panics
    ///
    /// Panics if `value` is not between 1 and 100.
    pub fn new(value: i32) -> Guess {
        if value < 1 || value > 100 {
            panic!("Guess value must be between 1 and 100, got {value}.");
        }
        Guess { value }
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}
```

> Panicking when the contract is violated makes sense because a contract violation always indicates a caller-side bug... Contracts... should be explained in the API documentation.
> — https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

### Example 8: `catch_unwind` at an FFI boundary

Do not use `catch_unwind` as a general error-handling mechanism. Use it only where an unwinding panic must not cross a language boundary:

```rust
use std::panic::{catch_unwind, AssertUnwindSafe};

extern "C" fn rust_entry() {
    if catch_unwind(AssertUnwindSafe(|| do_rust_work())).is_err() {
        // The FFI caller cannot safely handle a Rust panic.
        // Log and abort, or translate to an error code as appropriate.
    }
}
```

> It is **not** recommended to use this function for a general try/catch mechanism. The `Result` type is more appropriate...
> — https://doc.rust-lang.org/std/panic/fn.catch_unwind.html

### Example 9: Short-circuiting `collect` into `Result<Vec<_>, _>`

```rust
let nums: Result<Vec<u32>, _> = ["1", "2", "oops"]
    .iter()
    .map(|s| s.parse::<u32>())
    .collect();

assert!(nums.is_err());
```

## Common mistakes

- **Using `unwrap` / `expect` in production without a documented invariant.** Every `unwrap` is a latent panic. Prefer `?`, combinators, or explicit `match`.
- **Using `catch_unwind` as general error handling.** It only catches unwinding panics and should be reserved for FFI boundaries and test harnesses.
- **Losing error context.** Returning `Err(())`, a bare string, or an opaque error without `source()` or `.context()` makes debugging hard.
- **Using `String` or `()` as the error type in a public API.** Neither gives callers enough information; `()` does not even implement `std::error::Error`.
- **Exposing `anyhow::Error` in a library's public API.** Type-erased errors prevent callers from matching on variants and couple your public API to anyhow's stability.
- **Panicking on expected I/O / parse / network errors.** Missing files, malformed input, refused connections, and rate limits should return `Result`.
- **Leaving `todo!()` in shipped code.** It is a panic with a friendly message; treat it as a release blocker.
- **Mixing `?` on `Result` and `Option` without explicit conversion.** Use `ok_or`/`ok_or_else`, `Result::ok`, or `Option::transpose` as needed.
- **Using `Box<dyn Error>` where downstream needs to match.** Trait-object errors are fine for prototypes and some binaries, but they prevent structured handling.
- **Forgetting `Display` for custom errors.** An empty or missing `Display` impl produces empty user-facing messages.
- **Delegating `provide()` to source errors.** On nightly, `Error::provide()` must not simply forward to the source; doing so causes duplicated context.
- **Rendering the inner error in both `Display` and `source()`.** Pick one: either return it from `source()` or print it in `Display`, not both.
- **Using `unreachable!()` for conditions that are only *probably* false.** If the branch can be reached through buggy caller input, return `Result` or use `debug_assert!`.
- **Assuming `Debug` output is stable.** Derived `Debug` formats may change across compiler versions; never parse them.
- **Forgetting that `Result` is `#[must_use]`.** Silently discarding a `Result` is a warning for a reason.

## Strict vs contextual guidance

### Strict guidance (follow unless there is an explicit, documented reason)

- Do not use `panic!` for expected runtime errors.
- Do not use `unwrap` / `expect` in production code without documenting the invariant.
- Library error types must implement `std::error::Error + Send + Sync + 'static`; see `docs/rust/api-design.md` (C-GOOD-ERR).
- User-facing output must come from `Display`, not `Debug`.
- `From` impls must be infallible.
- Do not expose `anyhow::Error` from a public library API.
- Do not leave `todo!` in shipped code.
- Public error enums should be `#[non_exhaustive]`; see `docs/rust/api-design.md`.

### Contextual guidance (depends on the repo)

- Whether to enable `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` as `deny`.
- Whether to use `anyhow`, `eyre`, or a reporting crate such as `miette` / `color-eyre`.
- Whether to hand-write error enums or use `thiserror`.
- Whether `Box<dyn Error>` is allowed in internal tools or only in prototypes.
- Whether to localize user-facing error messages.
- Whether release builds use `panic = "abort"`.
- Whether to capture backtraces by default and which environment variable activates them.

## Policy decisions for individual repos

Each repository should document its answers to:

1. Which error crates are approved (`thiserror`, `anyhow`, `eyre`, `miette`, `color-eyre`, `snafu`)?
2. Is `unwrap` banned by clippy in production code?
3. Is `expect` allowed only with a documented invariant?
4. Is `Box<dyn Error>` allowed in binaries, only in prototypes, or never?
5. What is the convention for error messages (e.g., lowercase, no trailing period)?
6. How are errors reported to users (CLI stderr, structured logs, HTTP response bodies)?
7. Is `panic = "abort"` enabled for release builds?
8. Which environment variable activates backtraces (`RUST_BACKTRACE`, `RUST_LIB_BACKTRACE`, or both)?
9. Are public error enums required to be `#[non_exhaustive]`?
10. Is `anyhow::Error` permitted in any public API, and if so, under what conditions?

## Related docs

- `docs/rust/api-design.md` — C-GOOD-ERR, `#[non_exhaustive]`, and the rule that public APIs contain no `unwrap`/`expect`/`panic!`.
- `docs/rust/types-traits-generics.md` — `From`/`Into` blanket impls, `Display`/`Debug` derive generalities, enum exhaustiveness, and newtypes.
- `docs/rust/ownership-lifetimes.md` — ownership, borrowing, and `Copy`/`Drop` interactions that affect whether combinators consume or borrow values.
- `docs/rust/testing.md` — testing error paths and the idiomatic use of `unwrap` / `expect` in tests.
- `docs/rust/async-tokio.md` — error handling in async code, `JoinSet` errors, and `?` in async functions.
- `docs/rust/lints-clippy.md` — clippy configuration for `unwrap_used`, `expect_used`, `panic`, and related lints.
- `docs/rust/style-formatting.md` — code formatting and documentation style.
- The Rust Book, Chapter 9: https://doc.rust-lang.org/book/ch09-00-error-handling.html
- Standard library docs for `Option`, `Result`, `Error`, and panic macros (cited above).

## Related skills

- `nix-usage` — for Rust toolchain and `nix develop` workflows in this repository.
