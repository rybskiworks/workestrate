# Ownership, Borrowing, and Lifetimes

## Purpose

This document is the canonical reference for Rust's ownership system, borrowing rules, lifetimes, and the surrounding mechanics that AI coding agents must internalize when writing, reviewing, or refactoring Rust code. It focuses on the compile-time aliasing discipline that makes Rust memory-safe without garbage collection ([The Rust Programming Language: Understanding Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)). Use it alongside the smart-pointers, type-system, async, and unsafe docs for the full memory-model picture.

## Sources used

- [The Rust Programming Language: Understanding Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)
- [The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)
- [The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)
- [The Rust Programming Language: Box](https://doc.rust-lang.org/book/ch15-01-box.html)
- [The Rust Reference: Borrow Operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#borrow-operators)
- [The Rust Reference: Reference Types](https://doc.rust-lang.org/reference/types/reference.html) (redirected/superseded by [Pointer Types](https://doc.rust-lang.org/reference/types/pointer.html))
- [The Rust Reference: Pointer Types](https://doc.rust-lang.org/reference/types/pointer.html)
- [The Rust Reference: Moved and Copied Types](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)
- [The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [The Rust Reference: Lifetime Elision](https://doc.rust-lang.org/reference/lifetime-elision.html#lifetime-elision-in-functions)
- [The Rust Reference: Subtyping and Variance](https://doc.rust-lang.org/reference/subtyping.html)
- [The Rust Reference: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds)
- [The Rustonomicon: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/nomicon/hrtb.html)
- [`std::marker::Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [`std::clone::Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)
- [`std::pin`](https://doc.rust-lang.org/std/pin/index.html)
- [`std::marker::Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html)
- [Rust API Guidelines: Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)
- [Rust Error Index E0382](https://doc.rust-lang.org/error_codes/E0382.html)
- [Rust Error Index E0502](https://doc.rust-lang.org/error_codes/E0502.html)
- [Rust Error Index E0505](https://doc.rust-lang.org/error_codes/E0505.html)
- [Rust Error Index E0597](https://doc.rust-lang.org/error_codes/E0597.html)
- [Rust Error Index E0106](https://doc.rust-lang.org/error_codes/E0106.html)
- [Rust Error Index E0204](https://doc.rust-lang.org/error_codes/E0204.html)
- [Rust Error Index E0495](https://doc.rust-lang.org/error_codes/E0495.html)

Claims below are inline-cited to the specific URLs above.

## Core guidance

### Ownership rules

The Rust Book states three rules ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)):

> 1. Each value in Rust has an owner.
> 2. There can only be one owner at a time.
> 3. When the owner goes out of scope, the value will be dropped.

These rules are not stylistic: the borrow checker enforces them as hard compile-time constraints. Single ownership guarantees no double-free and no use-after-free, because the single owner is responsible for dropping the value exactly once.

### Move semantics and the `Copy` trait

Assignment, argument passing, and returning a non-`Copy` value *moves* it. The source is invalidated. As the Book explains, for `String` this means "we copy the pointer, the length, and the capacity that are on the stack. We do not copy the data on the heap" and "Rust calls this a *move*" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)). The Book also notes that "Rust will never automatically create 'deep' copies" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)).

Attempting to use a moved value produces the classic error:

```text
error[E0382]: borrow of moved value: s1
  move occurs because s1 has type String, which does not implement the Copy trait
```

([Rust Error Index E0382](https://doc.rust-lang.org/error_codes/E0382.html)).

`Copy` is the exception. The Book says: "Rust has a special annotation called the Copy trait that we can place on types that are stored on the stack... If a type implements the Copy trait, variables that use it do not move, but rather are trivially copied" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)). The std docs add: "The behavior of Copy is not overloadable; it is always a simple bit-wise copy" and "Shared references can be copied, but mutable references cannot!" ([`std::marker::Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html)).

`Copy` and `Drop` are mutually exclusive. The Book is explicit: "Rust won't let us annotate a type with Copy if the type, or any of its parts, has implemented the Drop trait" and "any group of simple scalar values can implement Copy, and nothing that requires allocation or is some form of resource can implement Copy" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)). `Clone` is the explicit deep-copy operation. The Book calls `.clone()` "a visual indicator that arbitrary code is being executed" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)). For `Copy` types, `clone()` is equivalent to a bitwise copy ([`std::clone::Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)).

RAII means that when an owner goes out of scope, `Drop::drop` runs and memory is freed. Reassigning an existing variable "will call drop and free the original value's memory immediately" ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)). `Box<T>` extends this to the heap: "Just like any owned value, when a box goes out of scope... it will be deallocated. The deallocation happens both for the box (stored on the stack) and the data it points to (stored on the heap)" ([The Rust Programming Language: Box](https://doc.rust-lang.org/book/ch15-01-box.html)).

### Place expressions vs. value expressions

The Reference divides expressions into *place expressions* (memory locations) and *value expressions* (computed values). Locals, statics, dereferences, indexing, and field accesses are place expressions ([The Rust Reference: Moved and Copied Types](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)).

> When a place expression is evaluated in a value expression context, or is bound by value in a pattern, it denotes the value held in that memory location. If the type of that value implements Copy, then the value will be copied. In the remaining situations, if that type is Sized, then it may be possible to move the value.

Movable places include variables not currently borrowed, temporaries, fields of a movable place that do not implement `Drop`, and the deref of `Box<T>` ([The Rust Reference: Moved and Copied Types](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)). After moving out of a local variable, "the location is deinitialized and cannot be read from again until it is reinitialized" ([The Rust Reference: Moved and Copied Types](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)).

This underpins *partial moves*: you can move individual non-`Drop` fields out of a struct, leaving the remaining fields still usable and droppable.

### References and borrowing

> A reference is like a pointer... Unlike a pointer, a reference is guaranteed to point to a valid value of a particular type for the life of that reference.

([The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).

The two rules of references, verbatim:

> 1. At any given time, you can have either one mutable reference or any number of immutable references.
> 2. References must always be valid.

([The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).

The first rule prevents data races. A data race requires "two or more pointers access the same data at the same time," "at least one of the pointers is being used to write to the data," and "there's no mechanism being used to synchronize access to the data" ([The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)). The second rule means "the compiler guarantees that references will never be dangling references" ([The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).

The borrow operators are unary prefix `&` (shared) and `&mut` (mutable); they cannot be overloaded. "If the & or &mut operators are applied to a value expression, then a temporary value is created." For a shared borrow, "the place may not be mutated, but it may be read or shared again." For a mutable borrow, "the place may not be accessed in any way until the borrow expires" ([The Rust Reference: Borrow Operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#borrow-operators)). `&&` desugars as two borrows. The reference also defines raw borrows `&raw const` and `&raw mut` for unsafe code.

Shared references are `Copy`; mutable references are not. "A mutable reference (that hasn't been borrowed) is the only way to access the value it points to, so is not Copy" ([The Rust Reference: Pointer Types](https://doc.rust-lang.org/reference/types/pointer.html)).

### Non-Lexical Lifetimes (NLL)

The Book describes the modern borrow-scope behavior without using the name NLL:

> a reference's scope starts from where it is introduced and continues through the last time that reference is used.

([The Rust Programming Language: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)). "NLL" is the community/compiler-internal name for this behavior; the Book does not use the term. Rely on this rule to keep borrows minimal.

### Slices

> Slices let you reference a contiguous sequence of elements in a collection. A slice is a kind of reference, so it does not have ownership.

([The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)). A slice range has the form `[starting_index..ending_index]`, where `starting_index` is the first position included and `ending_index` is one more than the last position included ([The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)). Shorthands: `&s[..2]`, `&s[3..]`, `&s[..]`.

String slice indices "must occur at valid UTF-8 character boundaries" ([The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)). Prefer `fn first_word(s: &str) -> &str` over taking `&String`; "String literals are slices" and "also why string literals are immutable; &str is an immutable reference" ([The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)). Array slices have type `&[i32]` ([The Rust Programming Language: Slices](https://doc.rust-lang.org/book/ch04-03-slices.html)).

### Lifetimes

> every reference in Rust has a lifetime, which is the scope for which that reference is valid. Most of the time, lifetimes are implicit and inferred.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). The borrow checker "compares scopes to determine whether all borrows are valid" ([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).

Lifetime annotation syntax uses names starting with an apostrophe, usually `'a`, placed after the `&`:

```rust
&i32        // a reference
&'a i32     // a reference with an explicit lifetime
&'a mut i32 // a mutable reference with an explicit lifetime
```

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).

The canonical multi-input example ties the output to both inputs:

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

> Lifetime annotations don't change how long any of the references live. Rather, they describe the relationships of the lifetimes of multiple references to each other.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). Missing annotations produce:

```text
error[E0106]: missing lifetime specifier ... expected named lifetime parameter
```

([Rust Error Index E0106](https://doc.rust-lang.org/error_codes/E0106.html)).

Structs holding references must declare the lifetime:

```rust
struct ImportantExcerpt<'a> {
    part: &'a str,
}
```

> an instance of ImportantExcerpt can't outlive the reference it holds in its part field.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). Lifetime names on `impl` blocks are also part of the type:

> Lifetime names for struct fields always need to be declared after the impl keyword and then used after the struct's name because those lifetimes are part of the struct's type.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).

### Lifetime elision

The compiler applies three elision rules, verbatim from the Book:

> 1. each [reference] parameter that's a reference gets its own lifetime parameter.
> 2. if there is exactly one input lifetime parameter, that lifetime is assigned to all output lifetime parameters.
> 3. if there are multiple input lifetime parameters, but one of them is &self or &mut self because this is a method, the lifetime of self is assigned to all output lifetime parameters.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). The Reference restates them as:

> - Each elided lifetime in the parameters becomes a distinct lifetime parameter.
> - If there is exactly one lifetime used in the parameters (elided or not), that lifetime is assigned to all elided output lifetimes.
> - In method signatures... If the receiver has type &Self or &mut Self, then the lifetime of that reference to Self is assigned to all elided output lifetime parameters.

([The Rust Reference: Lifetime Elision](https://doc.rust-lang.org/reference/lifetime-elision.html#lifetime-elision-in-functions)).

> The elision rules don't provide full inference. If there is still ambiguity... the compiler won't guess.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). These signatures are illegal to elide:

```rust
fn get_str() -> &str;                   // ERROR: no input lifetime to infer from
fn frob(s: &str, t: &str) -> &str;      // ERROR: ambiguous which input owns the output
```

### The `'static` lifetime

> One special lifetime we need to discuss is 'static, which denotes that the affected reference can live for the entire duration of the program. All string literals have the 'static lifetime.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). But the Book adds a critical caveat:

> You might see suggestions in error messages to use the 'static lifetime. But before specifying 'static as the lifetime for a reference, think about whether or not the reference you have actually lives the entire lifetime of your program... Most of the time, an error message suggesting the 'static lifetime results from attempting to create a dangling reference or a mismatch of the available lifetimes.

([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)). Leaking a `Box` (`Box::leak`) is another legitimate source of `'static` references; adding `'static` to silence the compiler is not.

### Variance and subtyping

> Subtyping is restricted to two cases: variance with respect to lifetimes and between types with higher ranked lifetimes.

([The Rust Reference: Subtyping and Variance](https://doc.rust-lang.org/reference/subtyping.html)). For lifetimes, "'static outlives the lifetime parameter 'a, & 'static str is a subtype of &'a str" ([The Rust Reference: Subtyping and Variance](https://doc.rust-lang.org/reference/subtyping.html)).

| Type | Variance in `'a` | Variance in `T` |
|---|---|---|
| `&'a T` | covariant | covariant |
| `&'a mut T` | covariant | invariant |
| `*const T` | — | covariant |
| `*mut T` | — | invariant |
| `[T]`, `[T; n]` | — | covariant |
| `fn() -> T` | — | covariant |
| `fn(T) -> ()` | — | contravariant |
| `UnsafeCell<T>` | — | invariant |
| `PhantomData<T>` | — | covariant |
| `dyn Trait<T> + 'a` | covariant | invariant |

`&mut T` is invariant in `T` because allowing a shorter-lived value to be written through a mutable reference would let a longer-lived reference observe a dangling value. `&'a T` is covariant in both because shared, immutable access cannot invalidate other references. The composite rule is: "If the parameter is used in positions with different variances then the parameter is invariant" ([The Rust Reference: Subtyping and Variance](https://doc.rust-lang.org/reference/subtyping.html)).

### Higher-Ranked Trait Bounds (`for<'a>`)

> Trait bounds may be higher ranked over lifetimes. These bounds specify a bound that is true for all lifetimes.

([The Rust Reference: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds)). The canonical example:

```rust
fn call_on_ref_zero<F>(f: F)
where
    for<'a> F: Fn(&'a i32),
{
    let zero = 0;
    f(&zero);
}
```

([The Rustonomicon: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/nomicon/hrtb.html)). The two spellings are equivalent: `for<'a> F: Fn(&'a i32)` and `F: for<'a> Fn(&'a i32)` ([The Rustonomicon: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/nomicon/hrtb.html)). Function-pointer and trait-object types with borrowed arguments have implicit higher-ranked lifetimes: `fn(&str) -> &str` desugars to `for<'a> fn(&'a str) -> &'a str`, and `dyn Fn(&str) -> &str` desugars to `dyn for<'a> Fn(&'a str) -> &'a str` ([The Rustonomicon: Higher-Ranked Trait Bounds](https://doc.rust-lang.org/nomicon/hrtb.html)).

You must write `for<'a>` explicitly when the borrowed lifetime is shorter than any function-level lifetime parameter, when returning closures that borrow their arguments, or when storing such function pointers / trait objects in structs. A generic lifetime parameter `<'a>` means the caller picks one fixed lifetime; `for<'a>` means the implementation must work for *all* lifetimes.

### `Pin` and self-referential structs

A self-referential struct contains a pointer to another piece of its own data. "if that value is moved, the pointer will still point to the old address... thus becoming invalid" ([`std::pin`](https://doc.rust-lang.org/std/pin/index.html)). `Pin<P>` is the contract that "the pointee will not be moved and remains valid at that address" ([`std::pin`](https://doc.rust-lang.org/std/pin/index.html)).

> Pinning does not require nor make use of any compiler 'magic' ... only a specific contract between the unsafe parts of a library API and its users.

([`std::pin`](https://doc.rust-lang.org/std/pin/index.html)). `Unpin` is an auto-trait; almost every type is `Unpin`, and for `T: Unpin`, `Pin<Box<T>>` behaves like `Box<T>` ([`std::marker::Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html)). A `PhantomPinned` field opts a type out of `Unpin`. "self-referential types and intrusive data structures [cannot currently] be modeled in fully safe Rust using only borrow-checked references" ([`std::pin`](https://doc.rust-lang.org/std/pin/index.html)). The Drop guarantee requires a pinned value to remain valid at the same address until its drop handler runs.

For async-generated futures, `Pin` is pervasive; see `docs/rust/async-tokio.md`. For ordinary ownership problems, prefer indices into a `Vec`, owned copies, or arena allocation over self-referential references.

## Practical rules

1. **Trace the ownership chain first.** Every value has one owner from creation to drop. If you cannot name the owner, redesign.
2. **Assume moves by default.** Assignment, argument passing, and returning move non-`Copy` values. Use `Copy` types only for simple scalar values whose fields are all `Copy` and which do not implement `Drop` ([The Rust Programming Language: What Is Ownership?](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)).
3. **Pass `&T` to read, `&mut T` to mutate, and `T` to consume.** If the function needs an owned result, do not take a borrow and `clone()` internally unless the caller explicitly requests a borrowed API (see rule 13).
4. **Keep borrow scopes minimal.** Under NLL, a borrow ends at its last use. Restructure code so conflicting borrows do not overlap.
5. **Accept slices, not `&String` or `&Vec<T>`.** `fn foo(s: &str)` and `fn bar(v: &[T])` work with owned containers, literals, and subslices. `clippy::ptr_arg` flags `&String` and `&Vec<T>`.
6. **Let elision handle simple lifetime cases.** Do not write `fn foo<'a>(x: &'a str) -> &'a str` when `fn foo(x: &str) -> &str` suffices. Add annotations only when the compiler requires them or when multiple input lifetimes create ambiguity.
7. **Use `'static` only when the data truly lives forever.** String literals and `Box::leak` are legitimate; patching compiler errors with `'static` is not ([The Rust Programming Language: Lifetime Syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).
8. **Return ownership when the caller needs to store or transform the value.** Returning a reference is fine only when it borrows from an input the caller still owns.
9. **Remember drop order.** Locals and temporaries drop in reverse order of declaration/creation; struct fields drop in declaration order; tuple fields drop in order; array/slice elements drop from first to last; function parameters drop last; partially initialized values drop only their initialized fields ([The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)). Use `core::mem::ManuallyDrop` or `core::mem::forget` only when you are intentionally suppressing drop.
10. **Partial moves are allowed but fragile.** Moving one field out of a struct leaves the remaining fields usable. Do not use a moved field again; the compiler will report E0382/E0505.
11. **Reserve `&raw const` / `&raw mut` for unsafe code.** They create raw pointers without the borrow checker's aliasing guarantees; ordinary code should use `&` and `&mut`.
12. **Respect variance.** Do not transmute or coerce references in ways that would let a longer-lived reference observe a shorter-lived value. When designing generic containers, consult the variance table and cross-reference `docs/rust/types-traits-generics.md` for `PhantomData` choices.
13. **Choose borrow, clone, or move deliberately.** See the decision guide in the Examples section and the API guidelines ([Rust API Guidelines: Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)).
14. **Use `for<'a>` for callbacks that borrow arguments.** If a generic lifetime parameter cannot express "works for any lifetime the caller provides," use a higher-ranked trait bound.
15. **Do not build self-referential structs with ordinary references.** Use indices, owned copies, arena allocation, or `Pin<!Unpin>`; for async, see `docs/rust/async-tokio.md`.

## Review checklist

- [ ] Does every value have a single, traceable owner from creation to drop?
- [ ] Are moves intentional? Is the source used after the move only when the type is `Copy`?
- [ ] Are borrow scopes as narrow as NLL allows?
- [ ] Are there no simultaneous mutable and immutable borrows of the same data?
- [ ] Do all references outlive the data they borrow?
- [ ] Are lifetime annotations present only where required or genuinely clarifying?
- [ ] Is `'static` used only for string literals, leaked heap values, or other truly static data?
- [ ] Do function signatures accept `&str` / `&[T]` instead of `&String` / `&Vec<T>` when they only read?
- [ ] Are `.clone()` calls justified by the borrow-vs-clone-vs-move decision guide?
- [ ] Does the code respect drop order, especially for fields that must outlive one another?
- [ ] Are partial moves handled correctly (only remaining fields used afterward)?
- [ ] Are self-referential patterns either restructured or correctly pinned?
- [ ] Does any `unsafe` code rely on lifetime invariants that the borrow checker cannot verify?

## Implementation checklist

- [ ] Identify the owner for every piece of data before writing the implementation.
- [ ] Use `&T` for read-only parameters, `&mut T` for mutation, and `T` for consumption.
- [ ] Prefer slices (`&str`, `&[T]`) over references to `String` or `Vec<T>`.
- [ ] Apply lifetime elision; add explicit annotations only when the compiler requires them.
- [ ] If a struct holds a reference, declare a lifetime and verify the struct cannot outlive the borrowed data.
- [ ] Before adding `.clone()`, try restructuring the code to narrow borrow scopes or return ownership.
- [ ] Check variance when adding generic lifetime or type parameters to collections.
- [ ] Use `for<'a>` when a callback or stored function pointer must accept references of any lifetime.
- [ ] Verify drop order for types with inter-field dependencies or custom `Drop` logic.
- [ ] Run `cargo check` and `cargo clippy` before runtime tests.

## Validation hooks

- **`cargo check`**: The primary gate. Borrow-checker acceptance means the ownership and lifetime constraints are satisfied.
- **`cargo clippy`**: Catches anti-patterns such as `clippy::ptr_arg`, `clippy::clone_on_copy`, `clippy::redundant_clone`, `clippy::needless_borrow`, and `clippy::assigning_clones`.
- **`cargo test`**: Runtime validation. The compiler cannot catch logic errors in `unsafe` lifetime management.
- **`cargo miri`**: Essential for `unsafe` code that manipulates lifetimes or raw pointers directly.
- **Code review**: For lifetime-heavy APIs, manually verify that annotations express the intended semantics, not just whatever compiles.

## Examples

### Example 1: Move semantics and `Copy`

```rust
fn move_and_copy() {
    let s1 = String::from("hello");
    let s2 = s1;                       // MOVE: s1 is invalidated
    // println!("{}", s1);             // ERROR: borrow of moved value
    println!("{}", s2);

    let x1 = 42;
    let x2 = x1;                       // i32 is Copy
    println!("{} {}", x1, x2);         // OK
}
```

### Example 2: Borrowing rules and NLL

```rust
fn borrowing() {
    let mut data = vec![1, 2, 3];

    let r1 = &data;
    let r2 = &data;
    println!("{:?} {:?}", r1, r2);     // shared borrows end after this use

    let r3 = &mut data;                // OK under NLL
    r3.push(4);
}
```

### Example 3: Lifetime annotations

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

fn use_longest() {
    let s1 = String::from("long");
    let result;
    {
        let s2 = String::from("short");
        result = longest(s1.as_str(), s2.as_str());
        println!("{}", result);        // OK while s1 and s2 are alive
    }
    // println!("{}", result);         // ERROR: s2 dropped
}
```

### Example 4: Illegal lifetime elision

```rust
// ERROR: no input lifetime to infer from
// fn get_str() -> &str {
//     "hello"
// }

// ERROR: ambiguous which input lifetime owns the output
// fn frob(s: &str, t: &str) -> &str {
//     s
// }

// OK: one input lifetime, elision applies
fn first(s: &str) -> &str {
    &s[0..1]
}
```

### Example 5: Slices

```rust
fn first_word(s: &str) -> &str {
    &s[..s.find(' ').unwrap_or(s.len())]
}

fn sum(values: &[i32]) -> i32 {
    values.iter().sum()
}

fn slice_usage() {
    let s = String::from("hello world");
    let _ = first_word(&s);

    let v = vec![1, 2, 3];
    let _ = sum(&v);
    let _ = sum(&v[1..]);
}
```

### Example 6: Partial moves

```rust
struct Config {
    name: String,
    version: u32,
}

fn partial_move() {
    let config = Config {
        name: String::from("app"),
        version: 1,
    };

    let name = config.name;            // partial move of `name`
    println!("version: {}", config.version); // OK: version not moved
    // println!("name: {}", config.name);      // ERROR: use of partially moved value
    drop(name);
}
```

### Example 7: Drop order

```rust
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

struct Pair {
    first: Loud,
    second: Loud,
}

fn drop_order() {
    let a = Loud("a");
    let b = Loud("b");
    let pair = Pair {
        first: Loud("first"),
        second: Loud("second"),
    };
    // Drop order:
    // 1. pair.second, pair.first   (declaration order)
    // 2. b, a                      (reverse of local declaration)
}
```

### Example 8: Variance and `&mut T` invariance

```rust
fn invariant_mut() {
    let mut s = String::from("static");
    let r: &'static mut String = unsafe {
        std::mem::transmute(&mut s) // DO NOT DO THIS
    };
    // If &mut T were covariant in T, this would let a 'static
    // mutable reference outlive the local `s`.
    drop(r);
}
```

In safe code, the invariance of `&mut T` prevents this class of unsoundness.

### Example 9: Higher-ranked trait bound

```rust
fn call_on_ref_zero<F>(f: F)
where
    for<'a> F: Fn(&'a i32),
{
    let zero = 0;
    f(&zero);
}
```

A plain generic `<'a>` would not work here because the lifetime of `&zero` is shorter than any lifetime the caller could name.

### Example 10: Self-referential struct workarounds

```rust
// Cannot be expressed safely with borrow-checked references:
// struct Parser {
//     input: String,
//     cursor: &str, // would point into input
// }

// Workaround 1: indices
struct ParserIndexed {
    input: String,
    cursor: std::ops::Range<usize>,
}

impl ParserIndexed {
    fn cursor_str(&self) -> &str {
        &self.input[self.cursor.clone()]
    }
}

// Workaround 2: owned copy
struct ParserOwned {
    input: String,
    cursor: String,
}
```

### Example 11: Borrow vs. clone vs. move decision guide

```rust
// MOVE: function consumes the value
fn take_ownership(s: String) -> String {
    s.to_uppercase()
}

// BORROW: function only reads
fn inspect(s: &str) {
    println!("{}", s);
}

// CLONE: caller needs an owned copy that outlives the borrow
fn cached(config: &Config) -> String {
    config.name.clone()
}

struct Config { name: String }
```

When designing APIs, prefer the guidance from the Rust API Guidelines: "If a function requires ownership of an argument, it should take ownership of the argument rather than borrowing and cloning the argument." Conversely, "If a function does not require ownership of an argument, it should take a shared or exclusive borrow of the argument rather than taking ownership and dropping the argument" ([Rust API Guidelines: Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)). When mutating an existing owned value from a borrow, prefer `dest.clone_from(&source)` over `*dest = source.clone()` (`clippy::assigning_clones`). For conditional ownership, consider `Cow<'_, T>` (see `docs/rust/smart-pointers-memory.md`).

## Common mistakes

### 1. Using a value after it has been moved

```rust
let s = String::from("hello");
let t = s;
// println!("{}", s);              // ERROR E0382: use of moved value
```

**Fix:** Borrow (`&s`) if you only need to read, or clone if you need two owned copies.

### 2. Holding a mutable borrow across a conflicting read

```rust
let mut v = vec![1, 2, 3];
let r = &mut v;
// println!("{:?}", v);            // ERROR E0502
r.push(4);
```

**Fix:** Restructure so the mutable borrow ends before the read, or read first.

### 3. Returning a reference to a local value

```rust
fn dangling() -> &'static str {
    let s = String::from("hello");
    // &s                              // ERROR E0597
    "static"                          // OK: string literal
}
```

**Fix:** Return an owned value or a `'static` string literal.

### 4. Adding unnecessary lifetime annotations

```rust
// Noisy
fn echo<'a>(s: &'a str) -> &'a str { s }

// Preferred
fn echo(s: &str) -> &str { s }
```

**Fix:** Rely on elision unless the compiler requires annotations or ambiguity exists.

### 5. Using `'static` to silence lifetime errors

```rust
fn bad<'a>(s: &'a str) -> &'static str {
    // s as &'static str              // UNSOUND
    todo!()
}
```

**Fix:** Tie the output lifetime to the correct input, return an owned value, or use `Box::leak` only when the leak is intentional.

### 6. Cloning to silence the borrow checker without understanding why

```rust
fn bad(data: &Vec<i32>) -> i32 {
    let cloned = data.clone();
    cloned[0]
}

fn good(data: &[i32]) -> i32 {
    data[0]
}
```

**Fix:** Take a slice and avoid the clone. If an owned copy is genuinely needed, make that choice explicit.

### 7. Storing references in structs without considering lifetime impact

```rust
struct Cache<'a> {
    entries: Vec<&'a str>,
}
```

**Fix:** Use `Vec<String>` if the cache must outlive its inputs. Reserve borrowed structs for transient, stack-local views.

### 8. Ignoring NLL and adding artificial scopes

```rust
// Unnecessary
let mut v = vec![1, 2, 3];
{
    let r = &v[0];
    println!("{}", r);
}
v.push(4);

// Equivalent under NLL
let mut v = vec![1, 2, 3];
let r = &v[0];
println!("{}", r);                    // borrow ends here
v.push(4);
```

**Fix:** Trust NLL; add scopes only when they clarify intent.

### 9. Confusing `Copy` with `Clone`

```rust
#[derive(Clone)]
struct Wrapper { data: String }

let a = Wrapper { data: String::from("x") };
let b = a;                                // MOVE, not copy
// let c = a.data;                        // ERROR
let d = b.clone();                        // explicit clone
```

**Fix:** `Copy` is implicit and bitwise; `Clone` is explicit. Deriving `Clone` does not make a type `Copy`.

### 10. Modifying a collection while iterating over it

```rust
let mut v = vec![1, 2, 3];
for item in &v {
    // v.push(*item);                  // ERROR E0502
    println!("{}", item);
}
```

**Fix:** Collect first, or iterate by index.

### Common borrow-checker errors

| Error | Meaning | Fix |
|---|---|---|
| E0382 | Use of moved value. | Borrow, clone, or derive `Copy` if eligible. |
| E0502 | Conflicting mutable/immutable borrow. | Reorder so the first borrow ends before the second begins. |
| E0505 | Cannot move out of borrowed content. | Release the borrow before the move, avoid the move, or implement `Copy`. |
| E0597 | Borrowed value does not live long enough. | Extend the borrower's scope or return ownership. |
| E0106 | Missing lifetime specifier. | Add the annotation or restructure so elision applies. |
| E0204 | `Copy` on a type with a non-`Copy` field. | Remove `Copy` or fix the field. |

Note: E0495 is no longer emitted by the compiler ([Rust Error Index E0495](https://doc.rust-lang.org/error_codes/E0495.html)). Do not treat it as a current error.

## Strict vs contextual guidance

| Rule | Strictness | Rationale |
|---|---|---|
| Each value has exactly one owner at a time | Strict | Compiler-enforced; violations are compile errors. |
| No simultaneous mutable and immutable borrows | Strict | Compiler-enforced; prevents data races. |
| References must be valid | Strict | Compiler-enforced; prevents dangling references. |
| `Copy` and `Drop` are mutually exclusive | Strict | Compiler-enforced; `Copy` requires bitwise duplication safety. |
| Do not build self-referential structs with ordinary references | Strict | Unsound without `Pin`. |
| Accept `&str` / `&[T]` instead of `&String` / `&Vec<T>` when reading | Contextual | Idiomatic default; ownership transfer is valid when consuming. |
| Prefer moves over clones | Contextual | Avoid unnecessary clones, but clone when the alternative is complex. |
| Use `'static` only for truly static data | Strict | Misuse creates unsound APIs. |
| Rely on NLL instead of artificial scopes | Contextual | NLL handles most cases; explicit scopes may clarify intent. |
| Add lifetime annotations only when required | Contextual | Elision reduces noise; explicit annotations can clarify ambiguous APIs. |

## Policy decisions for individual repos

1. **Clone threshold.** When is a clone acceptable in hot paths? Document whether code review should flag clones in performance-critical sections.
2. **Owned vs. borrowed struct data.** Default to owned data (`String`, `Vec<T>`) unless lifetime constraints are manageable and profiling justifies borrowing.
3. **Lifetime annotation style.** Rely on elision by default; decide whether public APIs may use explicit annotations for documentation.
4. **`Copy` derivation policy.** Limit `Copy` to small, trivially duplicable types with no custom drop semantics.
5. **Error ownership.** Decide whether error types own messages (`String`) or borrow them (`&'static str` / `&'a str`).
6. **Async ownership.** Default to owned data in futures; see `docs/rust/async-tokio.md`.
7. **Shared ownership threshold.** Document when `Rc`/`Arc` are acceptable; see `docs/rust/smart-pointers-memory.md`.
8. **`unsafe` lifetime invariants.** Every `unsafe` block must have a safety comment explaining lifetime and aliasing assumptions; see `docs/rust/unsafe-security.md`.

## Related docs

- `docs/rust/smart-pointers-memory.md` — `Box`, `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`, `Cow`, `Deref`, drop semantics, and interior/exterior mutability.
- `docs/rust/types-traits-generics.md` — structs, enums, traits, generics, `From`/`Into`/`AsRef`, `PhantomData` variance choices.
- `docs/rust/async-tokio.md` — async futures, `Pin`, task spawning, and lifetime issues specific to async code.
- `docs/rust/unsafe-security.md` — safety invariants for `unsafe` blocks that manipulate lifetimes or raw pointers.
- `docs/rust/error-handling.md` — `Result`, error conversion, and ownership of error payloads.
- `docs/rust/api-design.md` — public API stability, parameter ownership conventions, and flexibility guidelines.
- [The Rust Reference](https://doc.rust-lang.org/reference/) — formal semantics for moves, drops, borrows, lifetimes, and variance.
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — advanced unsafe Rust, including HRTB and self-referential types.
- [`std::pin`](https://doc.rust-lang.org/std/pin/index.html) — pinning contract and self-referential data structures.

## Related skills

- `nix-usage` — for the Rust toolchain and `just shell` devshell workflow used by this repository.
- There are no ownership-specific skills in the current registry. Repository-specific Rust validation skills may be added under `.agents/skills/` in the future and should reference this document.
