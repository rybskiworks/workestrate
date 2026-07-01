# Structs, Enums, Traits, Generics, Conversions, and Pattern Matching

## Purpose

This document is generic, repo-independent guidance for AI coding agents working with Rust's type-system primitives: structs, enums, traits, generics, type conversions, and pattern matching. It establishes the mental models, language-level mechanics, and review criteria needed to write, review, or refactor Rust code that is idiomatic, type-safe, and maintainable. It focuses on the *language semantics* of these features; API-surface concerns such as naming, semver, and sealed-trait policy live in [`docs/rust/api-design.md`](api-design.md), while ownership, lifetimes, and variance live in [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md).

Use this doc when you need to decide: whether a domain concept should be a struct or enum; which derive traits are safe; whether to use generics or `dyn Trait`; how to implement conversions; which pattern-matching construct to use; or how to choose a `PhantomData` variance marker. Cross-reference the sibling docs for ownership/lifetime details, API-design checklists, iterator/closure internals, error-type design, and smart-pointer mechanics.

## Sources used

### The Rust Programming Language (Book)

- [The Rust Programming Language: Defining and Instantiating Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)
- [The Rust Programming Language: An Example Program Using Structs](https://doc.rust-lang.org/book/ch05-02-example-structs.html)
- [The Rust Programming Language: Defining an Enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)
- [The Rust Programming Language: The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html)
- [The Rust Programming Language: Concise Control Flow with if let and if let else](https://doc.rust-lang.org/book/ch06-03-if-let.html)
- [The Rust Programming Language: Generic Data Types](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [The Rust Programming Language: Traits: Defining Shared Behavior](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [The Rust Programming Language: Trait Objects That Allow for Values of Different Types](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)
- [The Rust Programming Language: Refutability: Whether a Pattern Might Fail to Match](https://doc.rust-lang.org/book/ch19-02-refutability.html)
- [The Rust Programming Language: All the Pattern Syntax](https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html)
- [The Rust Programming Language: Advanced Traits](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html)
- [The Rust Programming Language: Advanced Types](https://doc.rust-lang.org/book/ch20-03-advanced-types.html)
- [The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)

### The Rust Reference

- [The Rust Reference: Structs](https://doc.rust-lang.org/reference/items/structs.html)
- [The Rust Reference: Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html)
- [The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html)
- [The Rust Reference: Implementations](https://doc.rust-lang.org/reference/items/implementations.html)
- [The Rust Reference: Associated Items](https://doc.rust-lang.org/reference/items/associated-items.html)
- [The Rust Reference: Generics](https://doc.rust-lang.org/reference/items/generics.html)
- [The Rust Reference: Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html)
- [The Rust Reference: Patterns](https://doc.rust-lang.org/reference/patterns.html)
- [The Rust Reference: Type Layout](https://doc.rust-lang.org/reference/type-layout.html)
- [The Rust Reference: The `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html)
- [The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)
- [The Rust Reference: If expressions](https://doc.rust-lang.org/reference/expressions/if-expr.html)
- [The Rust Reference: Loop expressions](https://doc.rust-lang.org/reference/expressions/loop-expr.html)
- [The Rust Reference: Statements](https://doc.rust-lang.org/reference/statements.html)
- [The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)
- [The Rust Reference: Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html)
- [The Rust Reference: The never type](https://doc.rust-lang.org/reference/types/never.html)
- [The Rust Reference: Special types and traits](https://doc.rust-lang.org/reference/special-types-and-traits.html)
- [The Rust Reference: Struct expressions](https://doc.rust-lang.org/reference/expressions/struct-expr.html)
- [The Rust Unstable Book: Specialization](https://doc.rust-lang.org/unstable-book/language-features/specialization.html)

### Standard library

- [`std::convert`](https://doc.rust-lang.org/std/convert/)
- [`std::convert::From`](https://doc.rust-lang.org/std/convert/trait.From.html)
- [`std::convert::Into`](https://doc.rust-lang.org/std/convert/trait.Into.html)
- [`std::convert::TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [`std::convert::TryInto`](https://doc.rust-lang.org/std/convert/trait.TryInto.html)
- [`std::convert::Infallible`](https://doc.rust-lang.org/std/convert/enum.Infallible.html)
- [`std::convert::AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)
- [`std::convert::AsMut`](https://doc.rust-lang.org/std/convert/trait.AsMut.html)
- [`std::str::FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html)
- [`std::default::Default`](https://doc.rust-lang.org/std/default/trait.Default.html)
- [`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html)
- [`std::borrow::Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html)
- [`std::borrow::ToOwned`](https://doc.rust-lang.org/std/borrow/trait.ToOwned.html)
- [`std::borrow::Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
- [`std::iter::IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html)
- [`std::marker::PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)
- [`std::marker::Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [`std::clone::Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)
- [`std::cmp::PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html)
- [`std::cmp::Eq`](https://doc.rust-lang.org/std/cmp/trait.Eq.html)
- [`std::cmp::PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html)
- [`std::cmp::Ord`](https://doc.rust-lang.org/std/cmp/trait.Ord.html)
- [`std::hash::Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html)
- [`std::keyword::dyn`](https://doc.rust-lang.org/std/keyword.dyn.html)

### Rust error index

- [E0004](https://doc.rust-lang.org/error_codes/E0004.html)
- [E0005](https://doc.rust-lang.org/error_codes/E0005.html)
- [E0038](https://doc.rust-lang.org/error_codes/E0038.html)
- [E0117](https://doc.rust-lang.org/error_codes/E0117.html)
- [E0119](https://doc.rust-lang.org/error_codes/E0119.html)
- [E0184](https://doc.rust-lang.org/error_codes/E0184.html)
- [E0204](https://doc.rust-lang.org/error_codes/E0204.html)
- [E0277](https://doc.rust-lang.org/error_codes/E0277.html)
- [E0001](https://doc.rust-lang.org/error_codes/E0001.html) (retired → `unreachable_patterns` lint)
- [E0002](https://doc.rust-lang.org/error_codes/E0002.html) (retired)
- [E0162](https://doc.rust-lang.org/error_codes/E0162.html) (retired → `irrefutable_let_patterns` lint)

### Rustnomicon and Rust blog

- [The Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [Announcing Rust 1.65.0: let-else statements](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)

### Rust API Guidelines

- [Rust API Guidelines: Type safety (C-NEWTYPE)](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust API Guidelines: Future proofing (C-SEALED, C-NEWTYPE-HIDE, C-STRUCT-BOUNDS)](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Rust API Guidelines: Interoperability (C-CONV-TRAITS, C-COMMON-TRAITS)](https://rust-lang.github.io/api-guidelines/interoperability.html)
- [Rust API Guidelines: Predictability (C-DEREF, C-OBJECT, C-GENERIC)](https://rust-lang.github.io/api-guidelines/predictability.html)

Claims below are inline-cited to the specific URLs above.

## Core guidance

### Structs

A struct is the basic unit for grouping related data. The Book defines it as follows:

> To define a struct, we enter the keyword struct and name the entire struct... Then, inside curly brackets, we define the names and types of the pieces of data, which we call fields.

([The Rust Programming Language: Defining and Instantiating Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html))

Rust provides three struct flavors. Named-field structs are the default. Tuple structs carry positional fields:

> Tuple structs have the added meaning the struct name provides but don't have names associated with their fields...

([The Rust Programming Language: Defining and Instantiating Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html))

Unit-like structs have no fields and behave similarly to `()`:

> You can also define structs that don't have any fields! These are called unit-like structs because they behave similarly to (), the unit type...

([The Rust Programming Language: Defining and Instantiating Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html))

The Reference adds the implementation detail that tuple structs define a constructor of the same name, and unit-like structs implicitly define a constant of their type with the same name. It also reminds us that memory layout is unspecified unless `repr` is used:

> A tuple struct is a nominal tuple type... it also defines a constructor of the same name in the value namespace. A unit-like struct is a struct without any fields... Such a struct implicitly defines a constant of its type with the same name. The precise memory layout of a struct is not specified. One can specify a particular layout using the repr attribute.

([The Rust Reference: Structs](https://doc.rust-lang.org/reference/items/structs.html))

A struct instance is mutable as a whole, not field-by-field:

> Note that the entire instance must be mutable; Rust doesn't allow us to mark only certain fields as mutable.

([The Rust Programming Language: Defining and Instantiating Structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html))

Use struct update syntax (`..`) with care: it moves data out of the base instance, and the base cannot be used afterward if any non-`Copy` fields were moved. This is covered in detail in [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md); here we note only the type-system surface.

### Enums

An enum is a simultaneous definition of a nominal type and a set of constructors:

> An enumeration, also referred to as an enum, is a simultaneous definition of a nominal enumerated type as well as a set of constructors...

([The Rust Reference: Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html))

Each variant name is also a constructor function:

> The name of each enum variant that we define also becomes a function that constructs an instance of the enum.

([The Rust Programming Language: Defining an Enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html))

`Option<T>` is the canonical enum for optional values:

> Rust doesn't have nulls, but it does have an enum that can encode the concept of a value being present or absent. This enum is Option<T>.

([The Rust Programming Language: Defining an Enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html))

The compiler enforces exhaustive handling of enums:

> Matches in Rust are exhaustive: We must exhaust every last possibility.

([The Rust Programming Language: The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html))

Discriminants are integers logically associated with each variant; under the default Rust representation they are interpreted as `isize`, and duplicate discriminants are an error. Primitive `repr`s (`u8`, `u16`, etc.) are only permitted on enums, and `#[repr(C, u8)]` may be combined. See [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) for `Copy`/`Drop` interactions and [`docs/rust/api-design.md`](api-design.md) for `#[non_exhaustive]` policy.

### Pattern matching

Pattern matching is the control-flow mechanism tied to Rust's algebraic types. A `match` expression has a type equal to the least upper bound of its arms; with no arms it is diverging and has type `!` ([The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)). `match` is exhaustive, while `if let` and `while let` opt out:

> you can think of if let as syntax sugar for a match that runs code when the value matches one pattern and then ignores all other values.

([The Rust Programming Language: Concise Control Flow with if let and if let else](https://doc.rust-lang.org/book/ch06-03-if-let.html))

`let-else` (stable since Rust 1.65) provides a diverging fallback for refutable patterns:

> This introduces a new type of let statement with a refutable pattern and a diverging else block that executes when that pattern doesn't match.

([Announcing Rust 1.65.0: let-else statements](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html))

Patterns are either refutable or irrefutable:

> Patterns come in two forms: refutable and irrefutable. Function parameters, let statements, and for loops can only accept irrefutable patterns... The if let and while let expressions and the let...else statement accept refutable and irrefutable patterns, but the compiler warns against irrefutable patterns.

([The Rust Programming Language: Refutability](https://doc.rust-lang.org/book/ch19-02-refutability.html))

Match guards, `@` bindings, `ref`/`ref mut`, rest patterns, alternation, and range patterns provide fine-grained control. Method-resolution and ownership details of matching on references are owned by [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md); here we focus on choosing the right construct and writing correct patterns.

### Traits

A trait describes an abstract interface:

> A trait describes an abstract interface that types can implement. This interface consists of associated items, which come in three varieties: functions, types, constants. All traits define an implicit type parameter Self... Trait functions are not allowed to be const functions.

([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html))

Traits can have default methods:

> If the trait function defines a body, this definition acts a default for any implementation which does not override it.

([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html))

Supertraits constrain implementors:

> Supertraits are traits that are required to be implemented for a type to implement a specific trait... It is an error for a trait to be its own supertrait.

([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html))

Associated types model exactly one output type per implementor (e.g., `Iterator::Item`), while generic trait parameters allow multiple implementations per type (e.g., `From<T>`). Generic associated types (GATs) are stable since Rust 1.65.

Trait implementations are constrained by the orphan rule:

> The orphan rule states that a trait implementation is only allowed if either the trait or at least one of the types in the implementation is defined in the current crate. It prevents conflicting trait implementations across different crates and is key to ensuring coherence.

([The Rust Reference: Implementations](https://doc.rust-lang.org/reference/items/implementations.html))

Overlapping implementations are forbidden on stable Rust; specialization is nightly-only behind `feature(specialization)` ([The Rust Unstable Book: Specialization](https://doc.rust-lang.org/unstable-book/language-features/specialization.html)).

### Object safety

A trait is usable as a trait object only when it is *dyn compatible* (formerly called "object-safe"). The Reference gives the full rules:

> A dyn-compatible trait can be the base trait of a trait object. A trait is dyn compatible if it has the following qualities: All supertraits must also be dyn compatible. Sized must not be a supertrait... It must not have any associated constants. It must not have any associated types with generics. All associated functions must either be dispatchable... or be explicitly non-dispatchable. Dispatchable fns must: no type params; not use Self except in receiver; have receiver `&self`/`&mut self`/`Box<Self>`/`Rc<Self>`/`Arc<Self>`/`Pin<P>`; no opaque return (no async fn, no -> impl Trait); no `where Self: Sized`.

([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html))

The standard workaround for a method that would otherwise break object safety is to add `where Self: Sized`, making it non-dispatchable. Error [E0038](https://doc.rust-lang.org/error_codes/E0038.html) is emitted when a non-dyn-compatible trait is used as `dyn Trait`.

### Generics and dispatch

Generics parameterize items over types, lifetimes, and const values. The Book notes the naming convention:

> type parameter names in Rust are short, often just one letter, and Rust's type-naming convention is UpperCamelCase.

([The Rust Programming Language: Generic Data Types](https://doc.rust-lang.org/book/ch10-01-syntax.html))

Monomorphization means generics have no runtime cost:

> Rust accomplishes this by performing monomorphization... we pay no runtime cost for using generics.

([The Rust Programming Language: Generic Data Types](https://doc.rust-lang.org/book/ch10-01-syntax.html))

Trait bounds constrain generic parameters. Multiple bounds use `+`; `where` clauses improve readability and can bound types that are not themselves parameters:

> provide another way to specify bounds... as well as a way to specify bounds on types that aren't type parameters

([The Rust Reference: Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html))

`impl Trait` in argument position is syntactic sugar for an anonymous generic parameter; in return position it is an abstract return type. Return-position `impl Trait` in traits (RPITIT) is stable since Rust 1.75 and desugars to an anonymous associated type. Every concrete return path must resolve to the same type. Edition 2024 changed auto-capturing of lifetimes for return-position `impl Trait`; use `use<..>` bounds for precise capturing where needed.

`dyn Trait` enables dynamic dispatch. The Book notes the trade-off:

> When we use trait objects, Rust must use dynamic dispatch... at runtime, Rust uses the pointers inside the trait object to know which method to call. This lookup incurs a runtime cost that doesn't occur with static dispatch. Dynamic dispatch also prevents the compiler from choosing to inline a method's code.

([The Rust Programming Language: Trait Objects](https://doc.rust-lang.org/book/ch18-02-trait-objects.html))

A `dyn Trait` reference contains two pointers: one to the data and one to the vtable ([`std::keyword::dyn`](https://doc.rust-lang.org/std/keyword.dyn.html)). Not every trait can be made into a trait object; the full rules are in [Object safety](#object-safety) below.

### Conversions

The `std::convert` module defines a strict conversion hierarchy:

> Implement the AsRef trait for cheap reference-to-reference conversions. Implement the AsMut trait for cheap mutable-to-mutable conversions. Implement the From trait for consuming value-to-value conversions. Implement the Into trait for consuming value-to-value conversions to types outside the current crate. The TryFrom and TryInto traits behave like From and Into, but should be implemented when the conversion can fail.

([`std::convert`](https://doc.rust-lang.org/std/convert/))

`From` is infallible, lossless, value-preserving, and obvious:

> Note: This trait must not fail. The From trait is intended for perfect conversions. If the conversion can fail or is not perfect, use TryFrom.

([`std::convert::From`](https://doc.rust-lang.org/std/convert/trait.From.html))

The blanket impl means `From` is canonical:

> One should always prefer implementing From over Into because implementing From automatically provides one with an implementation of Into thanks to the blanket implementation.

([`std::convert::From`](https://doc.rust-lang.org/std/convert/trait.From.html))

`TryFrom`/`TryInto` (stable since Rust 1.34) return `Result`; the reflexive `TryFrom<T> for T` uses `Infallible` as its error type ([`std::convert::Infallible`](https://doc.rust-lang.org/std/convert/enum.Infallible.html)). `FromStr` is the standard fallible parsing trait. `AsRef`/`AsMut` are for cheap borrowed views only. `Deref` is not a conversion trait; it exists for smart pointers and inserts silent coercions. The full semantics of `Deref`, `Borrow`, `ToOwned`, and `Cow` are covered in [`docs/rust/smart-pointers-memory.md`](smart-pointers-memory.md); the error-chaining use of `From` is covered in [`docs/rust/error-handling.md`](error-handling.md).

## Practical rules

### Structs

1. **Use named-field structs by default.** Tuple structs are acceptable when the field order is obvious, when wrapping a single value in a newtype, or when the tuple struct constructor is part of a public API. Unit structs are for marker types or type-level state ([The Rust Reference: Structs](https://doc.rust-lang.org/reference/items/structs.html)).

2. **Derive `Debug` on every public struct** unless the type contains sensitive data. `Debug` is required by `assert_eq!` and is essential for diagnostics ([The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)). The Book is explicit that structs do not get a provided `Display` implementation, so derive or implement `Display` separately if needed ([The Rust Programming Language: An Example Program Using Structs](https://doc.rust-lang.org/book/ch05-02-example-structs.html)).

3. **Derive `Clone` when duplication is meaningful.** If cloning is expensive, consider whether callers actually need `Clone` or should take references instead ([The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)).

4. **Derive comparison and hashing traits consistently.** Derive `PartialEq` and `Eq` when equality is well-defined, and `PartialOrd`/`Ord` only when there is a total order consistent with equality. If you derive `Hash`, ensure `k1 == k2 -> hash(k1) == hash(k2)`; the compiler does not enforce the `Hash`/`Eq` pairing. Skip `Eq`/`Ord` for types where equality or ordering is meaningless (e.g., `f32`/`f64` implement `PartialEq` but not `Eq` because `NaN != NaN`) ([`std::cmp::Eq`](https://doc.rust-lang.org/std/cmp/trait.Eq.html), [`std::hash::Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html), [The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)).

5. **Never derive `Copy` for types that own heap data or implement `Drop`.** `Copy` implies bitwise duplication is safe and that no destructor runs. The Reference states it is an error to have `Copy` or `Clone` as a bound on a mutable reference, trait object, or slice ([The Rust Reference: Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html)); the std docs add that any type implementing `Drop` cannot be `Copy` ([`std::marker::Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html)). See [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) for the full `Copy`/`Drop` mechanics.

6. **Use field-init shorthand and `..Default::default()` appropriately.** Use `fieldname` as shorthand for `fieldname: fieldname` when parameter names match field names ([The Rust Reference: Struct expressions](https://doc.rust-lang.org/reference/expressions/struct-expr.html)). Combine struct update syntax with `Default::default()` to customize a few fields and default the rest, but only when `Default` is implemented and the default value is sensible ([The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)).

7. **Apply `#[non_exhaustive]` to public structs whose fields may change across minor versions.** Within the defining crate this has no effect; downstream crates cannot construct such a struct with a struct literal or rely on exhaustive destructuring ([The Rust Reference: The `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html)). See [`docs/rust/api-design.md`](api-design.md) for the policy framing.

### Enums

8. **Use enums to model mutually exclusive alternatives.** If a value can be one of several kinds, an enum is almost always the right tool. Do not use multiple `bool` or `Option` fields to encode what a single enum would express.

9. **Make invalid states unrepresentable.** Combine related booleans into an enum. For example, replace `is_connected: bool, is_error: bool` with `enum State { Connected, Disconnected(Error), Connecting }`.

10. **Handle enums exhaustively in `match`.** Never use `_` to silently ignore variants unless you have a documented reason (e.g., forward-compatibility with `#[non_exhaustive]`). A catch-all `_` that swallows a new variant is a bug waiting to happen ([The Rust Programming Language: The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html)).

11. **Apply `#[non_exhaustive]` to public enums that may gain variants.** This forces downstream consumers to handle unknown variants via `_` ([The Rust Reference: The `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html)).

12. **Control enum layout and discriminants only when the representation is part of the API.** Specify discriminants explicitly when the integer value matters; implicit discriminants increment by one and duplicate discriminants are an error. Use `#[repr(...)]` only when layout matters; primitive reprs are only allowed on enums, `#[repr(C, u8)]` may be combined, and `align`/`packed` cannot coexist ([The Rust Reference: Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html), [The Rust Reference: Type Layout](https://doc.rust-lang.org/reference/type-layout.html)).

### Pattern matching

13. **Prefer `match` over chained `if`/`else` when testing an enum or `Option`.** `match` is exhaustive by default; `if`/`else` chains are not ([The Rust Programming Language: The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html)).

14. **Use `if let` for single-variant extraction** when you only care about one pattern and the others are irrelevant or handled elsewhere. This avoids the visual noise of a full `match` with a `_ => {}` arm ([The Rust Programming Language: Concise Control Flow with if let and if let else](https://doc.rust-lang.org/book/ch06-03-if-let.html)).

15. **Use `let-else` for early-return extraction.** The pattern `let Some(x) = value else { return }` is clearer and more concise than the equivalent `match` or `if let` with an else branch. The else block must diverge ([The Rust Reference: Statements](https://doc.rust-lang.org/reference/statements.html)).

16. **Keep guards simple and use `@` bindings to avoid redundant tests.** Guards are available only in `match`, the compiler does not check exhaustiveness when guards are involved, and a guard may execute multiple times with `|` alternation. Use `n @ 0..=100` to bind a value while constraining its range, avoiding a separate guard ([The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html), [The Rust Programming Language: All the Pattern Syntax](https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html)).

17. **Use `ref` and `ref mut` explicitly when matching through references.** Match ergonomics can silently adjust binding modes; being explicit makes borrowing intent clear and avoids surprises during refactoring ([The Rust Reference: Patterns](https://doc.rust-lang.org/reference/patterns.html)).

18. **Remember that `_` does not bind; `_foo` does.** The wildcard `_` does not copy, move, or borrow the value it matches. A name starting with an underscore still binds the value and may trigger a move or copy ([The Rust Programming Language: All the Pattern Syntax](https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html)).

19. **Respect rest-pattern and alternation rules.** Use `..` at most once per tuple, tuple-struct, or slice pattern. In `|`-separated patterns, every binding name must appear in every alternative of the arm ([The Rust Reference: Patterns](https://doc.rust-lang.org/reference/patterns.html), [The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)).

### Traits

20. **Implement `From` rather than `Into`.** The blanket impl `impl<T, U> Into<U> for T where U: From<T>` means implementing `From` gives you `Into` for free. The reverse is not true ([`std::convert::Into`](https://doc.rust-lang.org/std/convert/trait.Into.html)).

21. **Use associated types when there is exactly one implementing type per implementor.** Use generic trait parameters when multiple implementations with different types are valid for the same implementing type (e.g., `From<T>`) ([The Rust Reference: Associated Items](https://doc.rust-lang.org/reference/items/associated-items.html)).

22. **Provide default method implementations when a sensible default exists.** A default definition acts as a default for any implementation that does not override it ([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html)). Note that you cannot call the default implementation from an overriding implementation ([The Rust Programming Language: Traits](https://doc.rust-lang.org/book/ch10-02-traits.html)).

23. **Respect the orphan rule.** A trait implementation is allowed only if the trait or at least one of the types is local to the current crate. Fundamental types such as `Box<T>` are treated specially: `Box<LocalType>` is considered local ([The Rust Reference: Implementations](https://doc.rust-lang.org/reference/items/implementations.html)). Violations produce [E0117](https://doc.rust-lang.org/error_codes/E0117.html).

24. **Do not rely on specialization.** Overlapping implementations are forbidden on stable Rust. Specialization is nightly-only behind `feature(specialization)` ([The Rust Unstable Book: Specialization](https://doc.rust-lang.org/unstable-book/language-features/specialization.html)); conflicting impls produce [E0119](https://doc.rust-lang.org/error_codes/E0119.html).

25. **Use sealed traits to prevent external implementations.** Add a private `Sealed` supertrait that downstream crates cannot name. Only the defining crate can implement the public trait, letting you evolve signatures without downstream breakage ([Rust API Guidelines: Future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)). See [`docs/rust/api-design.md`](api-design.md) for policy guidance.

26. **Decide object safety early.** Exclude non-dispatchable methods from trait objects with `where Self: Sized` when the trait may be used as `dyn Trait` ([Rust API Guidelines: Predictability](https://rust-lang.github.io/api-guidelines/predictability.html)).

### Generics and dispatch

27. **Use generics (monomorphization) by default.** Static dispatch has zero runtime overhead and enables inlining. Use `dyn Trait` only when you need dynamic dispatch: heterogeneous collections, plugin architectures, or when binary size from monomorphization is a measurable problem ([The Rust Programming Language: Trait Objects](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)).

28. **Use `impl Trait` in argument position as sugar for a generic parameter** when you do not need the caller to name the type. Note that changing a parameter between `<T: Trait>` and `impl Trait` is a breaking change because callers can explicitly specify generic arguments but not anonymous `impl Trait` arguments ([The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)).

29. **Use `impl Trait` in return position only when the concrete type is unnameable or intentionally hidden.** Do not use it to hide a named type that callers might need to store or pass around ([The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)).

30. **Use `where` clauses when bounds are complex or reference non-parameter types.** This keeps the function signature readable: `fn foo<T>(t: T) where T: Clone + Hash + Ord + Display` ([The Rust Reference: Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html)).

31. **Relax the implicit `Sized` bound with `?Sized` when you need to accept dynamically-sized types.** The `?` operator is only used to relax the implicit `Sized` trait bound for type parameters or associated types. Use `fn foo<T: ?Sized>(t: &T)` to accept `dyn Trait`, slices, and `str` ([The Rust Reference: Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html)).

32. **Respect generic-parameter ordering and ensure parameters constrain the implementation.** The order of generic parameters is restricted to lifetime parameters and then type and const parameters intermixed. An `impl<T> Struct { ... }` or `impl<const N: usize> Struct { ... }` is an error if the parameter does not appear in the trait or type being implemented ([The Rust Reference: Generics](https://doc.rust-lang.org/reference/items/generics.html), [The Rust Reference: Implementations](https://doc.rust-lang.org/reference/items/implementations.html)).

33. **Const generics are stable for basic array-like use since Rust 1.51.** Const parameters may be `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `char`, or `bool`. They may only appear as standalone arguments in types or array repeat expressions; combining them with other expressions (e.g., `[T; N + 1]`) requires the nightly `generic_const_exprs` feature ([The Rust Reference: Generics](https://doc.rust-lang.org/reference/items/generics.html)).

### Conversions

34. **Use the conversion-trait hierarchy correctly.** Implement `From` for infallible conversions and get `Into` free; never implement `Into` directly. Use `TryFrom`/`TryInto` for fallible conversions and never panic in a `From` impl ([`std::convert::From`](https://doc.rust-lang.org/std/convert/trait.From.html), [`std::convert::TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)).

35. **Use `AsRef`/`AsMut` for cheap views and `Borrow` only for equivalent keys.** `AsRef`/`AsMut` are for cheap borrowed views that must not fail. `Borrow` additionally requires `Hash`, `Eq`, and `Ord` equivalence between borrowed and owned values, so implement `Borrow` only when that invariant holds ([`std::convert::AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html), [`std::borrow::Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html)).

36. **Use `FromStr` for string parsing.** This is the standard trait for `str::parse()`. It is fallible and has no lifetime parameter, so you can only parse types that do not contain a lifetime parameter themselves ([`std::str::FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html)).

37. **Implement `Default` for types with a sensible zero value.** This enables `..Default::default()` struct update syntax and is required by many ecosystem APIs. Derive it when the derived value is correct; implement it manually when it is not. For enums, place `#[default]` on one unit variant (stable since Rust 1.62) ([`std::default::Default`](https://doc.rust-lang.org/std/default/trait.Default.html)).

38. **Do not use `Deref` for general conversion.** `Deref` exists for smart pointers and inserts silent coercions. Implement `Deref` only when the value transparently behaves like the target, `deref()` is cheap, and coercion is desirable ([`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html)). See [`docs/rust/smart-pointers-memory.md`](smart-pointers-memory.md) for the smart-pointer treatment.

### Newtypes and `PhantomData`

39. **Use the newtype pattern to create distinct domain types.** Wrapping a primitive in a tuple struct prevents mixing values such as `UserId` and `OrderId`. There is no runtime cost ([The Rust Programming Language: Advanced Traits](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html)). See [`docs/rust/api-design.md`](api-design.md) for C-NEWTYPE/C-NEWTYPE-HIDE policy.

40. **Type aliases are not newtypes.** A type alias does not create a distinct type and provides no type-checking benefits ([The Rust Programming Language: Advanced Types](https://doc.rust-lang.org/book/ch20-03-advanced-types.html)).

41. **Use `PhantomData` correctly for unused parameters.** Add `PhantomData` when a type parameter or lifetime is unused in fields. Choose the variant deliberately: `PhantomData<T>` is covariant and implies ownership of `T`; `PhantomData<fn() -> T>` is the safe default for a merely parameterized type because it is covariant without ownership or drop-check implications. Use contravariant or invariant forms only when the corresponding variance is required ([`std::marker::PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html), [The Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)). Cross-reference [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) for the full variance table.

### Derive macros

42. **Understand the derive macro whitelist and constraints.** Only `Clone`, `Copy`, `Debug`, `Default`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, and `Hash` can be derived. Derive `Copy` only when all fields are `Copy` and the type has no destructor; `Copy` types must also be `Clone`. Derive `Default` field-by-field; for enums, place `#[default]` on exactly one unit variant ([The Rust Programming Language: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html), [`std::marker::Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html), [`std::default::Default`](https://doc.rust-lang.org/std/default/trait.Default.html)).

43. **Do not duplicate derived trait bounds on generic data structures.** Prefer deriving without adding bounds to the struct definition. Adding bounds is breaking-prone and usually unnecessary ([Rust API Guidelines: Future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)).

## Review checklist

- [ ] Every public struct derives `Debug` (or has a documented reason not to).
- [ ] `Copy` is only derived for types where bitwise copy is safe and no `Drop` impl exists.
- [ ] Enums are used instead of multiple booleans or `Option` fields to represent mutually exclusive states.
- [ ] `match` arms are exhaustive; `_` catch-alls are documented if used.
- [ ] `From` is implemented instead of `Into` (the blanket impl provides `Into`).
- [ ] `Deref` is not used for conversion purposes.
- [ ] Fallible conversions use `TryFrom`/`TryInto`, not panicking `From` impls.
- [ ] Generic bounds are on `where` clauses when they are complex or reference non-parameter types.
- [ ] `dyn Trait` is used only when dynamic dispatch is justified; generics are the default.
- [ ] `PhantomData` is used for unused type/lifetime parameters with the correct variance marker.
- [ ] Newtypes wrap primitive types when domain semantics matter (IDs, quantities, etc.).
- [ ] `#[non_exhaustive]` is applied to public enums and structs that may evolve.
- [ ] Sealed traits are used for internal extension points that must not be implemented externally.
- [ ] `Default` is implemented or derived for types with a sensible zero value.
- [ ] Generic data structures do not duplicate derived trait bounds on parameters.
- [ ] Object-unsafe traits intended for use as `dyn Trait` have `where Self: Sized` on non-dispatchable methods.

## Implementation checklist

- [ ] Identify whether a named-field, tuple, or unit struct is appropriate for the data.
- [ ] Determine which `#[derive]` traits are correct: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default`, `PartialOrd`, `Ord`.
- [ ] Model domain alternatives as enum variants; carry data in variants rather than alongside the enum.
- [ ] Choose the right pattern-matching construct: `match` for exhaustiveness, `if let` for single-variant extraction, `let-else` for early return.
- [ ] Decide between generics and `dyn Trait` based on whether the set of types is closed and known at compile time.
- [ ] Implement `From` for infallible conversions; `TryFrom` for fallible ones; `AsRef`/`AsMut` for cheap borrowed views.
- [ ] Add `PhantomData` for any unused type or lifetime parameters, choosing the variant that matches the intended variance.
- [ ] Consider `#[non_exhaustive]` for any public type that may change across versions.
- [ ] Consider sealing traits that are implementation details of your public API.
- [ ] Verify that generic parameters constrain the implementation and appear in the trait or type being implemented.
- [ ] Use `where` clauses for complex bounds; keep simple bounds inline.
- [ ] Add tests for exhaustive variant coverage, `TryFrom` failure cases, and `Default` sensibility.

## Validation hooks

### Compile gates

- **`cargo check`**: catches missing trait implementations, non-object-safe `dyn Trait` usage, and exhaustive match violations.
- **`cargo clippy --all-features -- -D warnings`**: enables the default clippy lint set.

### Relevant clippy lints

Pay special attention to these lints, several of which correspond to rules in this document:

- `clippy::derivable_impls` — manual `Default`/`Clone`/`Debug` impls that match the derived one.
- `clippy::from_over_into` — implementing `Into` instead of `From`.
- `clippy::unreachable_patterns` — patterns that can never match (replaces retired error E0001).
- `clippy::irrefutable_let_patterns` — irrefutable patterns in `if let`/`while let` (replaces retired error E0162).
- `clippy::match_same_arms` — redundant match arms that can be merged.
- `clippy::single_match` — a `match` with a single non-trivial arm that should be `if let`.
- `clippy::manual_map` — a `match` that can be rewritten with `Option::map`/`Result::map`.
- `clippy::redundant_closure` — closures that simply forward to a function.
- `clippy::deref_addrof` — suspicious `Deref`/`&` interactions.

### Test patterns

- Exercise every enum variant in tests to detect silently-swallowed cases.
- Test `TryFrom` failure paths, not just success paths.
- Verify that `Default` values are sensible and match any `new()` constructor.
- Add compile-time assertions for `Send`/`Sync` when a type contains raw pointers or `PhantomData` with raw-pointer-like variance.

### API review gates

- Verify public types have the right `#[derive]` and `#[non_exhaustive]` attributes before stabilizing.
- Confirm traits intended for `dyn Trait` are object-safe.
- Run `cargo semver-checks` or `cargo public-api diff` before release (see [`docs/rust/api-design.md`](api-design.md)).

## Examples

### Example 1: Named-field struct with correct derives

```rust
/// A user record in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

// Do NOT derive Copy: String owns heap data and cannot be bitwise-copied.
// Do NOT derive Default: there is no meaningful "zero" user.
```

This struct derives the common comparison and hashing traits but avoids `Copy` because it owns heap data. `Default` is intentionally omitted because a zero-valued `User` has no domain meaning.

### Example 2: Enum making invalid states unrepresentable

```rust
// WRONG: booleans can represent impossible states.
struct Connection {
    is_connected: bool,
    has_error: bool, // what does is_connected=true, has_error=true mean?
}

// CORRECT: enum makes every state meaningful.
#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Connected,
    Connecting,
    Disconnected { reason: String },
}
```

Enums encode mutually exclusive states directly, eliminating invalid combinations at compile time.

### Example 3: Pattern matching with `let-else`, `if let`, and guards

```rust
fn process_config(value: Value) -> Result<Config, Error> {
    // let-else for early-return extraction.
    let Value::Object(map) = value else {
        return Err(Error::ExpectedObject);
    };

    // if let for single-variant extraction.
    if let Some(timeout) = map.get("timeout") {
        configure_timeout(timeout)?;
    }

    // match with guard for combined test and extraction.
    match map.get("mode") {
        Some(Value::String(s)) if s.starts_with("prod") => {
            enable_production_mode(s)?;
        }
        Some(Value::String(s)) => {
            enable_development_mode(s)?;
        }
        None => {} // default mode
        Some(other) => return Err(Error::InvalidMode(other.clone())),
    }

    Ok(Config::default())
}
```

`let-else` removes boilerplate for the error path; `if let` handles the optional case; the guard combines a pattern test with an extra condition.

### Example 4: `From` vs `Into` implementation

```rust
// CORRECT: implement From, get Into for free.
impl From<ParseError> for AppError {
    fn from(err: ParseError) -> Self {
        AppError::Parse(err)
    }
}

// WRONG: implementing Into directly (the blanket impl won't provide From).
// impl Into<AppError> for ParseError { ... }

// Usage: both work because of the blanket impl.
let err1: AppError = parse_err.into();            // via Into
let err2: AppError = AppError::from(parse_err);   // via From
```

The blanket impl means `From` is the canonical conversion direction.

### Example 5: `TryFrom` for fallible conversion

```rust
use std::convert::TryFrom;

struct PositiveI32(i32);

impl TryFrom<i32> for PositiveI32 {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value > 0 {
            Ok(PositiveI32(value))
        } else {
            Err("value must be positive")
        }
    }
}

// Usage forces the caller to handle the error.
let pos: Result<PositiveI32, _> = 42.try_into();
let neg: Result<PositiveI32, _> = (-1).try_into();
assert!(neg.is_err());
```

`TryFrom` makes the failure path explicit and type-safe.

### Example 6: Newtype pattern

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct UserId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OrderId(u64);

fn load_user(id: UserId) { /* ... */ }

// load_user(OrderId(42)); // compile error: expected UserId, found OrderId
```

Newtypes are zero-cost and prevent domain-value confusion. The API-guidelines framing is in [`docs/rust/api-design.md`](api-design.md).

### Example 7: `AsRef` generic API and `Borrow` vs `AsRef`

```rust
use std::borrow::Borrow;

// Accept any type that can provide a cheap &str view.
fn greet<S: AsRef<str>>(name: S) {
    println!("Hello, {}!", name.as_ref());
}

greet("world");                       // &str implements AsRef<str> in some contexts
let owned = String::from("world");
greet(&owned);                        // String implements AsRef<str>

// Borrow requires equivalence of Hash, Eq, and Ord.
fn lookup<K: Eq + std::hash::Hash, V>(map: &HashMap<K, V>, key: &K) -> Option<&V> {
    map.get(key.borrow()) // Borrow<K> for K
}
```

`AsRef` is for cheap views; `Borrow` is for borrowed keys that must compare and hash the same as owned keys.

### Example 8: `dyn Trait` and object-safety workaround

```rust
// This trait is NOT object-safe because of by-value Self and the constructor.
trait Processor {
    fn process(&self, data: Vec<u8>) -> Self;
    fn new() -> Self;
}

// Workaround: make non-dispatchable methods require Sized.
trait ObjectSafeProcessor {
    fn process(&self, data: Vec<u8>) -> Box<dyn ObjectSafeProcessor>;
    fn create() -> Box<dyn ObjectSafeProcessor>
    where
        Self: Sized;
}

fn run(p: &mut dyn ObjectSafeProcessor) {
    p.process(vec![1, 2, 3]);
}
```

Adding `where Self: Sized` to methods that return `Self` or take generic parameters restores object safety.

### Example 9: Sealed trait

```rust
mod private {
    pub trait Sealed {}
}

/// Public trait that cannot be implemented outside this crate.
pub trait Protocol: private::Sealed {
    fn handshake(&self) -> bool;
}

pub struct Tcp;
impl private::Sealed for Tcp {}
impl Protocol for Tcp {
    fn handshake(&self) -> bool { true }
}

// External crate CANNOT do:
// impl Protocol for MyType { ... }  // error: Sealed is private
```

The private supertrait makes downstream implementation impossible. Policy guidance is in [`docs/rust/api-design.md`](api-design.md).

### Example 10: Const generics (stable subset)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Buffer<T, const N: usize> {
    data: [T; N],
}

impl<T: Default + Copy, const N: usize> Default for Buffer<T, N> {
    fn default() -> Self {
        Self { data: [T::default(); N] }
    }
}

let buf = Buffer::<u8, 64>::default();
```

Basic const generics (`[T; N]`, const parameters in impl blocks) are stable since Rust 1.51. Expressions like `[T; N + 1]` require the nightly `generic_const_exprs` feature.

### Example 11: `PhantomData` variance choice

```rust
use std::marker::PhantomData;

// A raw pointer that logically owns a T: use PhantomData<T>.
struct OwningPtr<T> {
    ptr: *mut T,
    _marker: PhantomData<T>,
}

// A type-level marker that only mentions T: use PhantomData<fn() -> T>.
struct Validator<T> {
    rules: Vec<Rule>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Validator<T> {
    fn validate(&self, value: &T) -> bool { /* ... */ }
}
```

`PhantomData<T>` implies ownership and affects drop check; `PhantomData<fn() -> T>` is the safe default for a merely parameterized type.

### Example 12: `#[default]` on enum (stable since 1.62)

```rust
#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum Mode {
    #[default]
    Passive,
    Active,
    Verbose,
}

let mode: Mode = Default::default();
assert_eq!(mode, Mode::Passive);
```

Deriving `Default` on an enum requires `#[default]` on exactly one unit variant.

## Common mistakes

### 1. Deriving `Copy` on types with heap data

```rust
// WRONG
#[derive(Copy, Clone)]
struct Config {
    name: String, // String owns heap data — Copy is unsound.
}

// CORRECT
#[derive(Clone)]
struct Config {
    name: String,
}
```

**Why:** `Copy` means bitwise duplication is safe. `String` manages a heap allocation; bitwise copying would create a double-free. The compiler rejects this with [E0204](https://doc.rust-lang.org/error_codes/E0204.html) or, if a destructor is present, [E0184](https://doc.rust-lang.org/error_codes/E0184.html). See [`docs/rust/ownership-lifetimes.md`](ownership-lifetimes.md) for the full `Copy`/`Drop` mechanics.

### 2. Implementing `Into` instead of `From`

```rust
// WRONG: manual Into impl
impl Into<AppError> for IoError {
    fn into(self) -> AppError { AppError::Io(self) }
}
// This does NOT provide From<IoError> for AppError,
// so .map_err(AppError::from) won't work.

// CORRECT: implement From
impl From<IoError> for AppError {
    fn from(err: IoError) -> Self { AppError::Io(err) }
}
// Now both .into() and AppError::from() work.
```

**Why:** The blanket impl `impl<T, U: From<T>> Into<U> for T` means `From` is the canonical direction. Implementing `Into` directly breaks symmetry and prevents `From`-based APIs from working ([`std::convert::Into`](https://doc.rust-lang.org/std/convert/trait.Into.html)).

### 3. Using `Deref` for conversion

```rust
// WRONG: using Deref to make Wrapper act like Inner
impl std::ops::Deref for Wrapper {
    type Target = Inner;
    fn deref(&self) -> &Inner { &self.inner }
}
// This causes auto-deref coercion: Wrapper methods silently resolve to Inner methods.
// Adding a method to Inner can silently change behavior of Wrapper users.

// CORRECT: use AsRef or explicit methods
impl AsRef<Inner> for Wrapper {
    fn as_ref(&self) -> &Inner { &self.inner }
}
```

**Why:** `Deref` is for smart pointers such as `Box`, `Rc`, `Arc`, `String`, and `Cow`. Auto-deref coercion affects method resolution in surprising ways. Use `AsRef` for borrowed views and `From`/`Into` for owned conversions ([`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html)).

### 4. Panicking in `From` for fallible conversions

```rust
// WRONG: From that can panic
impl From<&str> for Port {
    fn from(s: &str) -> Self {
        Port(s.parse().expect("invalid port")) // panics on bad input!
    }
}

// CORRECT: use TryFrom for fallible conversions
impl TryFrom<&str> for Port {
    type Error = ParseIntError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse().map(Port)
    }
}
```

**Why:** `From` is infallible by contract. Callers trust that `.into()` will not panic. If conversion can fail, `TryFrom` forces the caller to handle the error ([`std::convert::From`](https://doc.rust-lang.org/std/convert/trait.From.html)).

### 5. Using `_` catch-all in `match` without documentation

```rust
// WRONG: silently ignores new variants
match event {
    Event::Click(_) => handle_click(),
    Event::Key(_) => handle_key(),
    _ => {} // what about Event::Scroll? Event::Touch?
}

// CORRECT: handle every variant explicitly
match event {
    Event::Click(c) => handle_click(c),
    Event::Key(k) => handle_key(k),
    Event::Scroll(s) => handle_scroll(s),
    Event::Touch(t) => handle_touch(t),
}
// Or, with #[non_exhaustive] on Event:
match event {
    Event::Click(c) => handle_click(c),
    Event::Key(k) => handle_key(k),
    _ => log::warn!("unhandled event variant"),
}
```

**Why:** A bare `_` swallows any new variant added to the enum, hiding bugs. Exhaustive matching ensures that adding a variant causes compile errors at every match site, forcing explicit handling ([The Rust Programming Language: The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html)).

### 6. Using `impl Trait` in return position when the type should be nameable

```rust
// WRONG: callers cannot store or name the return type
pub fn make_iterator() -> impl Iterator<Item = i32> {
    vec![1, 2, 3].into_iter().filter(|x| *x > 1)
}
// Caller cannot write: let iter: ??? = make_iterator();

// CORRECT: return a named type or Box<dyn Iterator> if dynamic dispatch is needed
pub fn make_iterator() -> Box<dyn Iterator<Item = i32>> {
    Box::new(vec![1, 2, 3].into_iter().filter(|x| *x > 1))
}
```

**Why:** `impl Trait` in return position hides the concrete type. This is fine for internal helpers or when the type is unnameable (e.g., closures), but problematic for public APIs where callers need to store, name, or compose the return value ([The Rust Reference: `impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html)).

### 7. Missing `PhantomData` for unused type parameters

```rust
// WRONG: unused type parameter
struct Container<T> {
    data: Vec<u8>, // T is not used — compiler error: parameter `T` is never used
}

// CORRECT: PhantomData marks the logical relationship
struct Container<T> {
    data: Vec<u8>,
    _marker: PhantomData<T>, // communicates that Container is parameterized by T
}
```

**Why:** The compiler requires all type parameters to be used. `PhantomData<T>` satisfies this while also communicating variance and drop-check behavior to the compiler ([`std::marker::PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)).

### 8. Confusing `ref` in patterns with `&` in expressions

```rust
let value = Some(String::from("hello"));

// WRONG: trying to match a reference without being explicit
match &value {
    Some(s) => {
        // s is &String here because we matched on &value,
        // but the binding mode is implicit.
    }
    None => {}
}

// CORRECT: be explicit with ref
match value {
    Some(ref s) => {
        // s is &String, explicitly borrowed.
    }
    None => {}
}
```

**Why:** Rust's match ergonomics automatically adjust binding modes when matching on references, but this can be subtle and error-prone. Using `ref` explicitly makes the borrowing intent clear and avoids surprises when refactoring ([The Rust Reference: Patterns](https://doc.rust-lang.org/reference/patterns.html)).

### 9. Using `dyn Trait` for object-unsafe traits

```rust
// WRONG: this trait is not object-safe
trait Processor {
    fn process(&self, data: Vec<u8>) -> Self; // returns Self — not object-safe
    fn new() -> Self;                         // no &self — not object-safe
}

// This will fail:
// let p: Box<dyn Processor> = ...;

// CORRECT: redesign the trait for object safety
trait Processor {
    fn process(&self, data: Vec<u8>) -> Box<dyn Processor>;
    fn new() -> Box<dyn Processor>;
}
```

**Why:** Object safety requires that dispatchable methods take `&self`/`&mut self` (or certain receivers), do not return `Self`, and have no type parameters. Violations prevent `dyn Trait` usage with [E0038](https://doc.rust-lang.org/error_codes/E0038.html) ([The Rust Reference: Traits](https://doc.rust-lang.org/reference/items/traits.html)).

### 10. Not implementing `Default` when `..Default::default()` would be useful

```rust
// WRONG: no Default, forcing callers to specify every field
struct Config {
    host: String,
    port: u16,
    timeout: Duration,
    max_retries: u32,
}

// CORRECT: implement Default
impl Default for Config {
    fn default() -> Self {
        Config {
            host: "localhost".into(),
            port: 8080,
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}
// Callers can write: Config { port: 9090, ..Default::default() }
```

**Why:** `Default` enables struct update syntax, is required by many ecosystem APIs, and provides a clear "zero value" for the type ([`std::default::Default`](https://doc.rust-lang.org/std/default/trait.Default.html)).

### 11. Implementing `Borrow` for a single-field view

```rust
// WRONG: Borrow implies Hash/Eq/Ord equivalence, which a single-field view does not satisfy.
impl Borrow<str> for Person {
    fn borrow(&self) -> &str { &self.name }
}

// CORRECT: use AsRef for a cheap view
impl AsRef<str> for Person {
    fn as_ref(&self) -> &str { &self.name }
}
```

**Why:** `Borrow` requires that borrowed and owned values produce the same `Hash`, `Eq`, and `Ord` results. A view of one field cannot satisfy that for the whole struct. Use `AsRef` for such views ([`std::convert::AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)).

### 12. Adding bounds to generic data structures

```rust
// WRONG: adding bounds here is unnecessary and breaking-prone
pub struct Wrapper<T: Clone + Debug + PartialEq> {
    inner: T,
}

// CORRECT: derive without duplicating bounds
#[derive(Clone, Debug, PartialEq)]
pub struct Wrapper<T> {
    inner: T,
}
```

**Why:** Bounds on the struct definition restrict callers and become part of the public contract. Derive the traits without adding bounds; the compiler infers the required bounds on the impls ([Rust API Guidelines: Future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)).

## Error-code table

| Code | Meaning | Current status | Typical fix |
|------|---------|----------------|-------------|
| [E0004](https://doc.rust-lang.org/error_codes/E0004.html) | Non-exhaustive patterns in `match`. | Active | Add arms for all variants or a documented `_` catch-all. |
| [E0005](https://doc.rust-lang.org/error_codes/E0005.html) | Refutable pattern in an `let` / `fn` arg / `for` loop. | Active | Use `if let`, `match`, or `let-else` for refutable patterns. |
| [E0038](https://doc.rust-lang.org/error_codes/E0038.html) | Trait is not dyn-compatible (formerly "object-safe"). | Active | Add `where Self: Sized` to non-dispatchable methods or redesign the trait. |
| [E0117](https://doc.rust-lang.org/error_codes/E0117.html) | Trait can't be implemented for a type defined in another crate (orphan rule). | Active | Implement the foreign trait for a local newtype, or implement a local trait. |
| [E0119](https://doc.rust-lang.org/error_codes/E0119.html) | Conflicting trait implementations for the same type. | Active | Remove the overlap; specialization is nightly-only. |
| [E0184](https://doc.rust-lang.org/error_codes/E0184.html) | `Copy` implemented on a type with a destructor. | Active | Remove `Copy` or remove the `Drop` impl. |
| [E0204](https://doc.rust-lang.org/error_codes/E0204.html) | `Copy` implemented on a type with a non-`Copy` field. | Active | Remove `Copy` or make all fields `Copy`. |
| [E0277](https://doc.rust-lang.org/error_codes/E0277.html) | Trait bound not satisfied. | Active | Add the required bound or implement the missing trait. |
| [E0001](https://doc.rust-lang.org/error_codes/E0001.html) | Unreachable pattern. | Retired → `unreachable_patterns` lint | Remove or fix the unreachable arm. |
| [E0002](https://doc.rust-lang.org/error_codes/E0002.html) | Empty match expression. | Retired | Add at least one arm or explicitly handle the empty case. |
| [E0162](https://doc.rust-lang.org/error_codes/E0162.html) | Irrefutable `if let` pattern. | Retired → `irrefutable_let_patterns` lint | Use a plain `let` for irrefutable patterns. |

## Strict vs contextual guidance

| Rule | Strictness | Rationale |
|------|-----------|-----------|
| `match` must be exhaustive | **Strict** | Compiler-enforced; violations produce [E0004](https://doc.rust-lang.org/error_codes/E0004.html). |
| `Copy` requires all fields `Copy` and no `Drop` | **Strict** | Compiler-enforced via [E0204](https://doc.rust-lang.org/error_codes/E0204.html) and [E0184](https://doc.rust-lang.org/error_codes/E0184.html). |
| `Eq`/`Ord` supertrait chains must be respected | **Strict** | `Ord: Eq + PartialOrd`; `Eq: PartialEq` are compiler-enforced. |
| Orphan rule | **Strict** | Compiler-enforced via [E0117](https://doc.rust-lang.org/error_codes/E0117.html). |
| No overlapping trait impls on stable | **Strict** | Compiler-enforced via [E0119](https://doc.rust-lang.org/error_codes/E0119.html); specialization is nightly-only. |
| Trait must be dyn-compatible to use as `dyn Trait` | **Strict** | Compiler-enforced via [E0038](https://doc.rust-lang.org/error_codes/E0038.html). |
| Implement `From`, not `Into` | **Strict** | The blanket impl makes `From` canonical; direct `Into` impls break the ecosystem convention. |
| `From` must not fail | **Strict** | Violating the infallibility contract causes panics in `.into()` callers. |
| `Deref` is only for smart-pointer-like coercion | **Strict** | Auto-deref silently changes method resolution; misuse is a source of surprising API breakage. |
| Generic parameters must constrain the impl | **Strict** | Compiler-enforced for type and const parameters. |
| Use generics by default, `dyn Trait` when justified | **Contextual** | Generics have zero cost but increase binary size and compile time. `dyn` is appropriate for heterogeneous collections or plugin architectures. |
| `#[non_exhaustive]` on public types | **Contextual** | Essential for library crates with semver guarantees; unnecessary for application crates or internal types. |
| Sealed traits | **Contextual** | Use when the trait is an implementation detail. Do not seal traits intended for external implementation. |
| `impl Trait` in return position | **Contextual** | Fine for internal helpers and unnameable types. Avoid in public APIs where callers need to name the type. |
| `where` clauses vs inline bounds | **Contextual** | Use `where` when bounds are complex or reference types not in the parameter list. Inline bounds are fine for simple cases. |
| Derive `Default` | **Contextual** | Derive when the derived value is correct. Implement manually when the default is non-trivial. Skip when no sensible default exists. |
| `PhantomData<fn() -> T>` vs `PhantomData<T>` | **Contextual** | `PhantomData<T>` implies ownership and affects drop-check. `PhantomData<fn() -> T>` is covariant without ownership implications. Choose based on semantics. |
| Newtype pattern | **Contextual** | Use when domain semantics matter (IDs, quantities, units). Skip for simple wrappers where the inner type's semantics are sufficient. |

## Policy decisions for individual repos

Each repository should make and document the following decisions:

1. **Which `#[derive]` traits are mandatory for all public types?** For example: "All public structs must derive `Debug` and `Clone` unless there is a documented reason not to."

2. **When is `#[non_exhaustive]` required?** For example: "All public enums and structs in library crates must be `#[non_exhaustive]`" or "Only types in the `api` module need `#[non_exhaustive]`."

3. **When to use `dyn Trait` vs generics?** For example: "Use generics for all internal code. Use `dyn Trait` only in the `plugins` module where heterogeneous dispatch is required."

4. **Error conversion strategy:** For example: "All error types implement `From` for the crate's top-level error type. Never implement `Into` directly. Fallible conversions use `TryFrom`." See [`docs/rust/error-handling.md`](error-handling.md) for error-type design.

5. **Newtype policy:** For example: "All database IDs must be newtypes. All units of measurement (timestamps, durations, byte counts) must be newtypes with explicit conversion methods."

6. **Sealed trait policy:** For example: "All traits in the `traits` module that are not intended for external implementation must be sealed."

7. **`impl Trait` in return position policy:** For example: "`impl Trait` return types are allowed in private functions and for iterator chains. Public functions must return named types or `Box<dyn Trait>`."

8. **Pattern matching style:** For example: "Prefer `let-else` over `match` with early return. Use `if let` only when the else branch is empty. Never use `_ => ()` without a comment explaining why."

9. **`Default` implementation policy:** For example: "All configuration types must implement `Default`. Types with no sensible default must not implement `Default`."

10. **`PhantomData` variance choice:** For example: "Use `PhantomData<fn() -> T>` by default. Use `PhantomData<T>` only when the type logically owns `T`."

11. **Const-generic policy:** For example: "Const generics are allowed for array-like containers. Nightly `generic_const_exprs` is prohibited in production code."

12. **Conversion trait policy:** For example: "Implement `From` for lossless conversions, `TryFrom` for fallible conversions, `AsRef` for cheap views, and `FromStr` for parsing."

## Related docs

- `docs/rust/ownership-lifetimes.md` — Ownership, borrowing, lifetime parameters, lifetime elision, variance, `Copy`/`Drop` mechanics, `Pin`, and self-referential types.
- `docs/rust/api-design.md` — Public API stability, naming conventions (`as_`/`to_`/`into_`), `#[non_exhaustive]`, sealed traits, C-NEWTYPE/C-NEWTYPE-HIDE/C-SEALED, C-STRUCT-BOUNDS, C-CONV-TRAITS, C-COMMON-TRAITS, C-DEREF, C-OBJECT, C-GENERIC, and C-BUILDER.
- `docs/rust/iterators-closures.md` — `Fn`/`FnMut`/`FnOnce` hierarchy, iterator adapters, and the `Iterator` trait.
- `docs/rust/error-handling.md` — Error type design, the `?` operator, `From`-based error chaining, `thiserror`, and `anyhow`.
- `docs/rust/smart-pointers-memory.md` — `Box`, `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`, `Cow`, `Deref` as a smart-pointer mechanism, and drop semantics.
- `docs/rust/documentation-guidelines.md` — Documentation conventions for public items.
- `docs/rust/lints-clippy.md` — Clippy lint configuration and policy.

## Related skills

- `nix-usage` — for the Rust toolchain and `nix develop` workflow used by this repository.
- There are no Rust-type-system-specific skills in the current registry. Repository-specific Rust validation skills may be added under `.agents/skills/` in the future and should reference this document.
