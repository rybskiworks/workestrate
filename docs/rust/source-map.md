# Rust Source Map

## Purpose

This source map traces every piece of guidance in the Rust corpus back to its authoritative source. It records seed URLs, links discovered during research, crawl depth, coverage status, and which guidance documents use each source family. Use it when you need to verify a claim, update guidance after a source changes, or decide whether a topic has been explored deeply enough.

## Source coverage summary

The corpus cites more than 250 distinct URLs across roughly 15 source families. Official rust-lang sources (Book, Reference, Edition Guide, Cargo Book, rustdoc, rustfmt, Clippy, Standard Library, Rustonomicon, API Guidelines) account for the majority. Ecosystem and tooling sources (Tokio docs, async-book, Rust Design Patterns book, RustSec, cargo-deny, cargo-vet, Miri, ecosystem crates) provide the remainder. Coverage is deepest for language mechanics, Cargo, linting, and unsafe Rust; it is broad but less exhaustive for supply-chain security tooling and async ecosystem details.

## Link expansion coverage

Four documents contain explicit crawl ledgers (`design-patterns.md`, `iterators-closures.md`, `cargo-dependencies.md`, `supply-chain-security.md`). The other fourteen documents were constructed from explicitly listed Sources used sections without recursive crawl ledgers.

- **Total seed URLs explored:** approximately 40 (across the four ledgers and the Sources used lists of all docs).
- **Total discovered links followed:** approximately 100.
- **Notable discovered high-value sources:**
  - Cargo reference pages for semver, rust-version, profiles, config, build-scripts, source-replacement, features-examples, and unstable features.
  - cargo-deny check configuration pages (advisories, licenses, bans, sources).
  - cargo-vet documentation and registry.
  - cargo-auditable, cargo-supply-chain, and cargo-hack repositories.
  - Rust error index pages for E0004, E0005, E0038, E0117, E0119, E0184, E0204, E0277, E0382, E0502, E0505, E0597, E0106, E0495.
  - Rust API Guidelines chapters beyond the checklist (naming, interoperability, macros, predictability, flexibility, type-safety, dependability, debuggability, future-proofing, necessities).
- **Sources skipped (with categories of reasons):**
  - Out of scope / tangential: Cargo `[patch]`/`[replace]` deep details, registry internals, target discovery, pkgid spec, conditional compilation, build cache internals, glossary.
  - 404 / superseded: `rustsec.org/policies/`, `github.com/CycloneDX/cargo-cyclonedx`, `github.com/rust-secure-code/cargo-vet`, `doc.rust-lang.org/cargo/reference/build-script.html` (correct URL is `build-scripts.html`).
  - Already covered by sibling docs: topics delegated to `modules-visibility.md`, `testing.md`, `unsafe-security.md`, etc.
- **Partially explored sources:**
  - rustdoc JSON schema (nightly-only, unstable).
  - Tokio ecosystem beyond core `tokio` crate (e.g., `tokio-util`, async-trait).
  - Rust Secure Code Working Group repositories.

## Sources by topic

### Rust API Guidelines

- **Source family name:** Rust API Guidelines
- **Seed URLs:**
  - https://rust-lang.github.io/api-guidelines/
  - https://rust-lang.github.io/api-guidelines/about.html
  - https://rust-lang.github.io/api-guidelines/checklist.html
- **Discovered URLs:**
  - https://rust-lang.github.io/api-guidelines/naming.html
  - https://rust-lang.github.io/api-guidelines/interoperability.html
  - https://rust-lang.github.io/api-guidelines/macros.html
  - https://rust-lang.github.io/api-guidelines/documentation.html
  - https://rust-lang.github.io/api-guidelines/predictability.html
  - https://rust-lang.github.io/api-guidelines/flexibility.html
  - https://rust-lang.github.io/api-guidelines/type-safety.html
  - https://rust-lang.github.io/api-guidelines/dependability.html
  - https://rust-lang.github.io/api-guidelines/debuggability.html
  - https://rust-lang.github.io/api-guidelines/future-proofing.html
  - https://rust-lang.github.io/api-guidelines/necessities.html
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** naming (C-CASE, C-CONV), common traits, conversions, builders, newtypes, sealed traits, `#[non_exhaustive]`, documentation checklist items (C-CRATE-DOC, C-EXAMPLE, C-QUESTION-MARK, C-FAILURE, C-LINK, C-METADATA, C-RELNOTES, C-HIDDEN), semver-sensitive API evolution.
- **Which docs use it:** `api-design.md`, `documentation-guidelines.md`, `types-traits-generics.md`, `design-patterns.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### The Rust Programming Language (Book)

- **Source family name:** The Rust Programming Language
- **Seed URLs (per chapter):**
  - https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
  - https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
  - https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html
  - https://doc.rust-lang.org/book/ch04-03-slices.html
  - https://doc.rust-lang.org/book/ch05-01-defining-structs.html
  - https://doc.rust-lang.org/book/ch05-02-example-structs.html
  - https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html
  - https://doc.rust-lang.org/book/ch06-02-match.html
  - https://doc.rust-lang.org/book/ch06-03-if-let.html
  - https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
  - https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html
  - https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html
  - https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html
  - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
  - https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html
  - https://doc.rust-lang.org/book/ch09-00-error-handling.html
  - https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html
  - https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html
  - https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html
  - https://doc.rust-lang.org/book/ch10-01-syntax.html
  - https://doc.rust-lang.org/book/ch10-02-traits.html
  - https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html
  - https://doc.rust-lang.org/book/ch11-00-testing.html
  - https://doc.rust-lang.org/book/ch11-01-writing-tests.html
  - https://doc.rust-lang.org/book/ch11-02-running-tests.html
  - https://doc.rust-lang.org/book/ch11-03-test-organization.html
  - https://doc.rust-lang.org/book/ch13-00-functional-features.html
  - https://doc.rust-lang.org/book/ch13-01-closures.html
  - https://doc.rust-lang.org/book/ch13-02-iterators.html
  - https://doc.rust-lang.org/book/ch13-03-improving-our-io-project.html
  - https://doc.rust-lang.org/book/ch13-04-performance.html
  - https://doc.rust-lang.org/book/ch15-00-smart-pointers.html
  - https://doc.rust-lang.org/book/ch15-01-box.html
  - https://doc.rust-lang.org/book/ch15-02-deref.html
  - https://doc.rust-lang.org/book/ch15-03-drop.html
  - https://doc.rust-lang.org/book/ch15-04-rc.html
  - https://doc.rust-lang.org/book/ch15-05-interior-mutability.html
  - https://doc.rust-lang.org/book/ch15-06-reference-cycles.html
  - https://doc.rust-lang.org/book/ch16-00-concurrency.html
  - https://doc.rust-lang.org/book/ch16-03-shared-state.html
  - https://doc.rust-lang.org/book/ch17-00-async-await.html
  - https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html
  - https://doc.rust-lang.org/book/ch17-03-more-futures.html
  - https://doc.rust-lang.org/book/ch17-05-traits-for-async.html
  - https://doc.rust-lang.org/book/ch18-02-trait-objects.html
  - https://doc.rust-lang.org/book/ch19-02-refutability.html
  - https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html
  - https://doc.rust-lang.org/book/ch20-02-advanced-traits.html
  - https://doc.rust-lang.org/book/ch20-03-advanced-types.html
  - https://doc.rust-lang.org/book/appendix-03-derivable-traits.html
- **Seed or discovered:** seed (used as primary source)
- **Key topics extracted:** ownership, borrowing, lifetimes, structs, enums, pattern matching, modules, error handling, testing, closures, iterators, smart pointers, concurrency, async/await, advanced traits and types.
- **Which docs use it:** `ownership-lifetimes.md`, `types-traits-generics.md`, `modules-visibility.md`, `error-handling.md`, `testing.md`, `iterators-closures.md`, `smart-pointers-memory.md`, `async-tokio.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### The Rust Reference

- **Source family name:** The Rust Reference
- **Seed URLs:**
  - https://doc.rust-lang.org/reference/
  - https://doc.rust-lang.org/reference/expressions/operator-expr.html#borrow-operators
  - https://doc.rust-lang.org/reference/types/reference.html
  - https://doc.rust-lang.org/reference/types/pointer.html
  - https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types
  - https://doc.rust-lang.org/reference/destructors.html
  - https://doc.rust-lang.org/reference/lifetime-elision.html
  - https://doc.rust-lang.org/reference/subtyping.html
  - https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds
  - https://doc.rust-lang.org/reference/items/structs.html
  - https://doc.rust-lang.org/reference/items/enumerations.html
  - https://doc.rust-lang.org/reference/items/traits.html
  - https://doc.rust-lang.org/reference/items/implementations.html
  - https://doc.rust-lang.org/reference/items/associated-items.html
  - https://doc.rust-lang.org/reference/items/generics.html
  - https://doc.rust-lang.org/reference/trait-bounds.html
  - https://doc.rust-lang.org/reference/patterns.html
  - https://doc.rust-lang.org/reference/type-layout.html
  - https://doc.rust-lang.org/reference/attributes/type_system.html
  - https://doc.rust-lang.org/reference/expressions/match-expr.html
  - https://doc.rust-lang.org/reference/expressions/if-expr.html
  - https://doc.rust-lang.org/reference/expressions/loop-expr.html
  - https://doc.rust-lang.org/reference/statements.html
  - https://doc.rust-lang.org/reference/types/impl-trait.html
  - https://doc.rust-lang.org/reference/types/trait-object.html
  - https://doc.rust-lang.org/reference/types/never.html
  - https://doc.rust-lang.org/reference/special-types-and-traits.html
  - https://doc.rust-lang.org/reference/expressions/struct-expr.html
  - https://doc.rust-lang.org/reference/types/closure.html
  - https://doc.rust-lang.org/reference/expressions/closure-expr.html
  - https://doc.rust-lang.org/reference/items/modules.html
  - https://doc.rust-lang.org/reference/visibility-and-privacy.html
  - https://doc.rust-lang.org/reference/items/use-declarations.html
  - https://doc.rust-lang.org/reference/items/extern-crates.html
  - https://doc.rust-lang.org/reference/paths.html
  - https://doc.rust-lang.org/reference/names/preludes.html
  - https://doc.rust-lang.org/reference/attributes/testing.html
  - https://doc.rust-lang.org/reference/conditional-compilation.html
  - https://doc.rust-lang.org/reference/unsafe-keyword.html
  - https://doc.rust-lang.org/reference/unsafe-functions.html
  - https://doc.rust-lang.org/reference/unsafe-blocks.html
  - https://doc.rust-lang.org/reference/behavior-considered-undefined.html
  - https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html
  - https://doc.rust-lang.org/reference/items/external-blocks.html
  - https://doc.rust-lang.org/reference/abi.html
  - https://doc.rust-lang.org/reference/attributes.html
  - https://doc.rust-lang.org/reference/unsafety.html
- **Seed or discovered:** seed
- **Key topics extracted:** formal grammar of visibility, paths, modules, use declarations, closures, patterns, reference types, move/copy semantics, destructors, lifetime elision, subtyping, HRTB, unsafe keyword and blocks, undefined behavior, ABI/FFI, test attributes.
- **Which docs use it:** `ownership-lifetimes.md`, `types-traits-generics.md`, `iterators-closures.md`, `modules-visibility.md`, `testing.md`, `unsafe-security.md`, `smart-pointers-memory.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Standard Library

- **Source family name:** Standard Library
- **Seed URLs:**
  - https://doc.rust-lang.org/std/
  - https://doc.rust-lang.org/std/marker/trait.Copy.html
  - https://doc.rust-lang.org/std/clone/trait.Clone.html
  - https://doc.rust-lang.org/std/marker/trait.Send.html
  - https://doc.rust-lang.org/std/marker/trait.Sync.html
  - https://doc.rust-lang.org/std/marker/trait.Unpin.html
  - https://doc.rust-lang.org/std/marker/struct.PhantomData.html
  - https://doc.rust-lang.org/std/pin/index.html
  - https://doc.rust-lang.org/std/convert/
  - https://doc.rust-lang.org/std/convert/trait.From.html
  - https://doc.rust-lang.org/std/convert/trait.Into.html
  - https://doc.rust-lang.org/std/convert/trait.TryFrom.html
  - https://doc.rust-lang.org/std/convert/trait.TryInto.html
  - https://doc.rust-lang.org/std/convert/enum.Infallible.html
  - https://doc.rust-lang.org/std/convert/trait.AsRef.html
  - https://doc.rust-lang.org/std/convert/trait.AsMut.html
  - https://doc.rust-lang.org/std/str/trait.FromStr.html
  - https://doc.rust-lang.org/std/default/trait.Default.html
  - https://doc.rust-lang.org/std/ops/trait.Deref.html
  - https://doc.rust-lang.org/std/ops/trait.DerefMut.html
  - https://doc.rust-lang.org/std/ops/trait.Drop.html
  - https://doc.rust-lang.org/std/ops/trait.Try.html
  - https://doc.rust-lang.org/std/ops/trait.Fn.html
  - https://doc.rust-lang.org/std/ops/trait.FnMut.html
  - https://doc.rust-lang.org/std/ops/trait.FnOnce.html
  - https://doc.rust-lang.org/std/keyword.dyn.html
  - https://doc.rust-lang.org/std/keyword.move.html
  - https://doc.rust-lang.org/std/primitive.fn.html
  - https://doc.rust-lang.org/std/primitive.pointer.html
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
  - https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html
  - https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
  - https://doc.rust-lang.org/std/panic/fn.set_hook.html
  - https://doc.rust-lang.org/std/panic/struct.PanicHookInfo.html
  - https://doc.rust-lang.org/std/process/fn.abort.html
  - https://doc.rust-lang.org/std/process/trait.Termination.html
  - https://doc.rust-lang.org/std/future/trait.Future.html
  - https://doc.rust-lang.org/std/task/struct.Waker.html
  - https://doc.rust-lang.org/std/iter/
  - https://doc.rust-lang.org/std/iter/trait.Iterator.html
  - https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
  - https://doc.rust-lang.org/std/iter/trait.FromIterator.html
  - https://doc.rust-lang.org/std/iter/trait.Extend.html
  - https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html
  - https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html
  - https://doc.rust-lang.org/std/iter/trait.FusedIterator.html
  - https://doc.rust-lang.org/std/iter/trait.Step.html
  - https://doc.rust-lang.org/std/boxed/struct.Box.html
  - https://doc.rust-lang.org/std/boxed/index.html
  - https://doc.rust-lang.org/std/rc/struct.Rc.html
  - https://doc.rust-lang.org/std/rc/index.html
  - https://doc.rust-lang.org/std/rc/struct.Weak.html
  - https://doc.rust-lang.org/std/sync/struct.Arc.html
  - https://doc.rust-lang.org/std/sync/struct.Weak.html
  - https://doc.rust-lang.org/std/sync/struct.Mutex.html
  - https://doc.rust-lang.org/std/sync/struct.MutexGuard.html
  - https://doc.rust-lang.org/std/sync/struct.RwLock.html
  - https://doc.rust-lang.org/std/sync/atomic/index.html
  - https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
  - https://doc.rust-lang.org/std/cell/struct.Cell.html
  - https://doc.rust-lang.org/std/cell/struct.RefCell.html
  - https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html
  - https://doc.rust-lang.org/std/cell/struct.OnceCell.html
  - https://doc.rust-lang.org/std/sync/struct.OnceLock.html
  - https://doc.rust-lang.org/std/mem/fn.drop.html
  - https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html
  - https://doc.rust-lang.org/std/mem/union.MaybeUninit.html
  - https://doc.rust-lang.org/std/mem/fn.transmute.html
  - https://doc.rust-lang.org/std/borrow/enum.Cow.html
  - https://doc.rust-lang.org/std/borrow/trait.Borrow.html
  - https://doc.rust-lang.org/std/borrow/trait.ToOwned.html
  - https://doc.rust-lang.org/std/ptr/index.html
  - https://doc.rust-lang.org/std/ptr/struct.NonNull.html
  - https://doc.rust-lang.org/std/env/
  - https://doc.rust-lang.org/std/env/fn.var.html
  - https://doc.rust-lang.org/std/env/fn.var_os.html
  - https://doc.rust-lang.org/std/env/fn.args.html
  - https://doc.rust-lang.org/std/env/fn.args_os.html
  - https://doc.rust-lang.org/std/env/fn.set_var.html
  - https://doc.rust-lang.org/std/env/fn.remove_var.html
  - https://doc.rust-lang.org/std/env/fn.current_exe.html
  - https://doc.rust-lang.org/std/env/enum.VarError.html
  - https://doc.rust-lang.org/std/env/consts/index.html
  - https://doc.rust-lang.org/std/path/
  - https://doc.rust-lang.org/std/path/struct.Path.html
  - https://doc.rust-lang.org/std/path/struct.PathBuf.html
  - https://doc.rust-lang.org/std/path/struct.Components.html
  - https://doc.rust-lang.org/std/path/enum.Component.html
  - https://doc.rust-lang.org/std/path/enum.Prefix.html
  - https://doc.rust-lang.org/std/path/constant.MAIN_SEPARATOR.html
  - https://doc.rust-lang.org/std/fs/
  - https://doc.rust-lang.org/std/fs/struct.File.html
  - https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
  - https://doc.rust-lang.org/std/fs/struct.ReadDir.html
  - https://doc.rust-lang.org/std/fs/struct.DirEntry.html
  - https://doc.rust-lang.org/std/fs/struct.Metadata.html
  - https://doc.rust-lang.org/std/fs/struct.FileType.html
  - https://doc.rust-lang.org/std/fs/struct.Permissions.html
  - https://doc.rust-lang.org/std/fs/struct.DirBuilder.html
  - https://doc.rust-lang.org/std/fs/fn.read_dir.html
  - https://doc.rust-lang.org/std/fs/fn.canonicalize.html
  - https://doc.rust-lang.org/std/fs/fn.create_dir.html
  - https://doc.rust-lang.org/std/fs/fn.create_dir_all.html
  - https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html
  - https://doc.rust-lang.org/std/fs/fn.rename.html
  - https://doc.rust-lang.org/std/fs/fn.copy.html
  - https://doc.rust-lang.org/std/fs/fn.soft_link.html
  - https://doc.rust-lang.org/std/io/
  - https://doc.rust-lang.org/std/io/trait.Read.html
  - https://doc.rust-lang.org/std/io/trait.Write.html
  - https://doc.rust-lang.org/std/io/trait.BufRead.html
  - https://doc.rust-lang.org/std/io/trait.Seek.html
  - https://doc.rust-lang.org/std/io/struct.BufReader.html
  - https://doc.rust-lang.org/std/io/struct.BufWriter.html
  - https://doc.rust-lang.org/std/io/struct.Cursor.html
  - https://doc.rust-lang.org/std/io/struct.Lines.html
  - https://doc.rust-lang.org/std/io/struct.Empty.html
  - https://doc.rust-lang.org/std/io/struct.Sink.html
  - https://doc.rust-lang.org/std/io/fn.copy.html
  - https://doc.rust-lang.org/std/io/fn.stdin.html
  - https://doc.rust-lang.org/std/io/fn.stdout.html
  - https://doc.rust-lang.org/std/io/fn.stderr.html
  - https://doc.rust-lang.org/std/io/type.Result.html
  - https://doc.rust-lang.org/std/io/struct.Error.html
  - https://doc.rust-lang.org/std/io/enum.ErrorKind.html
  - https://doc.rust-lang.org/std/process/
  - https://doc.rust-lang.org/std/process/struct.Command.html
  - https://doc.rust-lang.org/std/process/struct.Child.html
  - https://doc.rust-lang.org/std/process/struct.ExitStatus.html
  - https://doc.rust-lang.org/std/process/struct.Output.html
  - https://doc.rust-lang.org/std/process/struct.Stdio.html
  - https://doc.rust-lang.org/std/process/fn.exit.html
  - https://doc.rust-lang.org/std/process/fn.abort.html
  - https://doc.rust-lang.org/std/process/struct.ExitCode.html
  - https://doc.rust-lang.org/std/time/
  - https://doc.rust-lang.org/std/time/struct.Instant.html
  - https://doc.rust-lang.org/std/time/struct.Duration.html
  - https://doc.rust-lang.org/std/time/struct.SystemTime.html
  - https://doc.rust-lang.org/std/time/struct.SystemTimeError.html
  - https://doc.rust-lang.org/std/time/constant.UNIX_EPOCH.html
- **Seed or discovered:** seed
- **Key topics extracted:** `Option`/`Result`, panic macros, backtrace, `Termination`, conversion traits, iterator traits, smart pointers, interior mutability, atomics, `Pin`, `Send`/`Sync`, env/path/fs/io/process/time APIs.
- **Which docs use it:** `ownership-lifetimes.md`, `types-traits-generics.md`, `error-handling.md`, `iterators-closures.md`, `smart-pointers-memory.md`, `async-tokio.md`, `std-runtime-apis.md`, `unsafe-security.md`, `testing.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Cargo Book

- **Source family name:** Cargo Book
- **Seed URLs:**
  - https://doc.rust-lang.org/cargo/
  - https://doc.rust-lang.org/cargo/reference/manifest.html
  - https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
  - https://doc.rust-lang.org/cargo/reference/semver.html
  - https://doc.rust-lang.org/cargo/reference/features.html
  - https://doc.rust-lang.org/cargo/reference/resolver.html
  - https://doc.rust-lang.org/cargo/reference/workspaces.html
  - https://doc.rust-lang.org/cargo/reference/profiles.html
  - https://doc.rust-lang.org/cargo/reference/config.html
  - https://doc.rust-lang.org/cargo/reference/build-scripts.html
  - https://doc.rust-lang.org/cargo/reference/source-replacement.html
  - https://doc.rust-lang.org/cargo/reference/environment-variables.html
  - https://doc.rust-lang.org/cargo/reference/rust-version.html
  - https://doc.rust-lang.org/cargo/reference/unstable.html
  - https://doc.rust-lang.org/cargo/reference/features-examples.html
  - https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html
  - https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html
  - https://doc.rust-lang.org/cargo/faq.html
  - https://doc.rust-lang.org/cargo/guide/project-layout.html
  - https://doc.rust-lang.org/cargo/reference/cargo-targets.html
  - https://doc.rust-lang.org/cargo/guide/tests.html
- **Discovered URLs:**
  - https://doc.rust-lang.org/cargo/commands/cargo-check.html
  - https://doc.rust-lang.org/cargo/commands/cargo-build.html
  - https://doc.rust-lang.org/cargo/commands/cargo-test.html
  - https://doc.rust-lang.org/cargo/commands/cargo-clippy.html
  - https://doc.rust-lang.org/cargo/commands/cargo-fmt.html
  - https://doc.rust-lang.org/cargo/commands/cargo-tree.html
  - https://doc.rust-lang.org/cargo/commands/cargo-update.html
  - https://doc.rust-lang.org/cargo/commands/cargo-add.html
  - https://doc.rust-lang.org/cargo/commands/cargo-remove.html
  - https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
  - https://doc.rust-lang.org/cargo/commands/cargo-vendor.html
  - https://doc.rust-lang.org/cargo/commands/cargo-package.html
  - https://doc.rust-lang.org/cargo/commands/cargo-publish.html
  - https://doc.rust-lang.org/cargo/commands/cargo-doc.html
  - https://doc.rust-lang.org/cargo/commands/cargo-rustdoc.html
  - https://doc.rust-lang.org/cargo/commands/cargo-fix.html
  - https://doc.rust-lang.org/cargo/commands/cargo-yank.html
  - https://doc.rust-lang.org/cargo/reference/external-tools.html
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** manifest structure, dependency specification, semver, features, resolver, workspaces, profiles, config, build scripts, source replacement, environment variables, MSRV, commands, `Cargo.lock` policy.
- **Which docs use it:** `cargo-dependencies.md`, `modules-visibility.md`, `supply-chain-security.md`, `error-handling.md`, `testing.md`, `editions-tooling.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Rust Edition Guide

- **Source family name:** Rust Edition Guide
- **Seed URLs:**
  - https://doc.rust-lang.org/edition-guide/
  - https://doc.rust-lang.org/edition-guide/rust-2024/
  - https://doc.rust-lang.org/edition-guide/rust-2015/index.html
  - https://doc.rust-lang.org/edition-guide/rust-2018/index.html
  - https://doc.rust-lang.org/edition-guide/rust-2021/index.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/index.html
  - https://doc.rust-lang.org/edition-guide/editions/index.html
  - https://doc.rust-lang.org/edition-guide/editions/advanced-migrations.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-formatting-fixes.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-version-sorting.html
  - https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-raw-identifier-sorting.html
- **Seed or discovered:** seed
- **Key topics extracted:** edition semantics, migration with `cargo fix`, 2024 edition changes, `unsafe_op_in_unsafe_fn` default, `unsafe extern`, `#[unsafe(attr)]`, style edition relationship.
- **Which docs use it:** `editions-tooling.md`, `style-formatting.md`, `unsafe-security.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Rustdoc

- **Source family name:** Rustdoc
- **Seed URLs:**
  - https://doc.rust-lang.org/rustdoc/
  - https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/what-is-rustdoc.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html
  - https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html
  - https://doc.rust-lang.org/rustdoc/lints.html
  - https://doc.rust-lang.org/rustdoc/command-line-arguments.html
  - https://doc.rust-lang.org/rustdoc/unstable-features.html
  - https://doc.rust-lang.org/rustdoc/advanced-features.html
  - https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/
- **Seed or discovered:** seed
- **Key topics extracted:** doc comment structure, doctests, `#[doc]` attributes, intra-doc links, re-exports, rustdoc lints, command-line arguments, rustdoc JSON (unstable).
- **Which docs use it:** `documentation-guidelines.md`, `editions-tooling.md`, `testing.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored (rustdoc JSON is partially explored due to nightly instability)

### rustfmt

- **Source family name:** rustfmt
- **Seed URLs:**
  - https://rust-lang.github.io/rustfmt/
  - https://github.com/rust-lang/rustfmt
  - https://github.com/rust-lang/rustfmt/blob/master/Configurations.md
  - https://github.com/rust-lang/rustfmt/blob/master/src/config.rs
  - https://doc.rust-lang.org/cargo/commands/cargo-fmt.html
  - https://rust-analyzer.github.io/manual.html#rustfmt
- **Discovered URLs:**
  - https://rust-lang.github.io/rustfmt/?version=v1.8.0&search=#Configuration
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** rustfmt configuration, stable vs nightly options, `style_edition`, import grouping, `#[rustfmt::skip]`, format-on-save, CI integration.
- **Which docs use it:** `style-formatting.md`, `cargo-dependencies.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Clippy

- **Source family name:** Clippy
- **Seed URLs:**
  - https://doc.rust-lang.org/stable/clippy/
  - https://doc.rust-lang.org/stable/clippy/usage.html
  - https://doc.rust-lang.org/stable/clippy/lints.html
  - https://rust-lang.github.io/rust-clippy/master/index.html
  - https://rust-lang.github.io/rust-clippy/master/
  - https://rust-lang.github.io/rust-clippy/master/#configuration
  - https://rust-lang.github.io/rust-clippy/master/lint_configuration.html
  - https://github.com/rust-lang/rust-clippy
  - https://doc.rust-lang.org/clippy/
- **Seed or discovered:** seed
- **Key topics extracted:** Clippy groups, lint levels, individual lints, restriction lints, `clippy.toml`, MSRV interaction, `[lints]` table integration.
- **Which docs use it:** `lints-clippy.md`, `api-design.md`, `cargo-dependencies.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Rustc lints

- **Source family name:** Rustc lints
- **Seed URLs:**
  - https://doc.rust-lang.org/rustc/lints/index.html
  - https://doc.rust-lang.org/rustc/lints/levels.html
  - https://doc.rust-lang.org/rustc/lints/groups.html
  - https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html
  - https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html
- **Seed or discovered:** seed
- **Key topics extracted:** lint levels, lint groups, allowed-by-default and warn-by-default listings, `warnings` group behavior.
- **Which docs use it:** `lints-clippy.md`, `editions-tooling.md`, `unsafe-security.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Rustup and toolchain

- **Source family name:** rustup and toolchain
- **Seed URLs:**
  - https://rust-lang.github.io/rustup/
  - https://rust-lang.github.io/rustup/overrides.html
  - https://rust-lang.github.io/rustup/concepts/components.html
  - https://rust-lang.github.io/rustup/concepts/profiles.html
  - https://rust-lang.github.io/rustup/concepts/channels.html
  - https://rust-lang.github.io/rustup/concepts/toolchains.html
  - https://rust-lang.github.io/rustup/cross-compilation.html
  - https://rust-lang.github.io/rustup/environment-variables.html
  - https://doc.rust-lang.org/rustc/platform-support.html
- **Seed or discovered:** seed
- **Key topics extracted:** toolchain pinning, components, profiles, channels, cross-compilation targets, overrides.
- **Which docs use it:** `editions-tooling.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### The Rustonomicon

- **Source family name:** The Rustonomicon
- **Seed URLs:**
  - https://doc.rust-lang.org/nomicon/
  - https://doc.rust-lang.org/nomicon/hrtb.html
  - https://doc.rust-lang.org/nomicon/leaking.html
  - https://doc.rust-lang.org/nomicon/dropck.html
  - https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html
  - https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html
  - https://doc.rust-lang.org/nomicon/working-with-unsafe.html
  - https://doc.rust-lang.org/nomicon/ffi.html
  - https://doc.rust-lang.org/nomicon/send-and-sync.html
  - https://doc.rust-lang.org/nomicon/transmutes.html
  - https://doc.rust-lang.org/nomicon/subtyping.html
  - https://doc.rust-lang.org/nomicon/phantom-data.html
  - https://doc.rust-lang.org/nomicon/vec-push-pop.html
  - https://doc.rust-lang.org/nomicon/vec-into-iter.html
  - https://doc.rust-lang.org/nomicon/vec-drain.html
  - https://doc.rust-lang.org/nomicon/exception-safety.html
  - https://doc.rust-lang.org/nomicon/unwinding.html
  - https://doc.rust-lang.org/nomicon/unbounded-lifetimes.html
  - https://doc.rust-lang.org/nomicon/casts.html
- **Seed or discovered:** seed
- **Key topics extracted:** HRTB, leaking, drop check, `Arc`/`Mutex`, safe/unsafe meaning, FFI, `Send`/`Sync`, transmutes, subtyping, `PhantomData`, exception safety, unwinding, unbounded lifetimes, casts.
- **Which docs use it:** `ownership-lifetimes.md`, `smart-pointers-memory.md`, `types-traits-generics.md`, `unsafe-security.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Rust Design Patterns book

- **Source family name:** Rust Design Patterns book
- **Seed URLs:**
  - https://rust-unofficial.github.io/patterns/
  - https://rust-unofficial.github.io/patterns/patterns/index.html
  - https://rust-unofficial.github.io/patterns/idioms/index.html
  - https://rust-unofficial.github.io/patterns/anti_patterns/index.html
  - https://rust-unofficial.github.io/patterns/functional/index.html
- **Discovered URLs:**
  - Patterns: behavioural/intro, command, interpreter, newtype, RAII, strategy, visitor; creational/intro, builder, fold; structural/intro, compose-structs, small-crates, unsafe-mods, trait-for-bounds; ffi/intro, ffi/export, ffi/wrappers.
  - Idioms: coercion-arguments, concat-format, ctor, default, deref, dtor-finally, mem-replace, on-stack-dyn-dispatch; ffi/intro, ffi/errors, ffi/accepting-strings, ffi/passing-strings; option-iter, pass-var-to-closure, priv-extend, rustdoc-init, temporary-mutability, return-consumed-arg-on-error.
  - Anti-patterns: borrow_clone, deny-warnings, deref.
  - Functional: paradigms, generics-type-classes, optics.
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** newtype, builder, RAII, strategy, command, visitor, fold, compose-structs, FFI wrappers, idioms for `Default`, `Deref`, `mem::replace`, `Cow`, anti-patterns.
- **Which docs use it:** `design-patterns.md`
- **Authority level:** community
- **Coverage status:** fully explored

### Asynchronous Programming in Rust (async-book)

- **Source family name:** async-book
- **Seed URLs:**
  - https://rust-lang.github.io/async-book/
  - https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html
  - https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
  - https://rust-lang.github.io/async-book/02_execution/05_io.html
  - https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
  - https://rust-lang.github.io/async-book/04_pinning/01_chapter.html
  - https://rust-lang.github.io/async-book/06_multiple_futures/01_chapter.html
  - https://rust-lang.github.io/async-book/07_workarounds/03_send_approximation.html
- **Seed or discovered:** seed
- **Key topics extracted:** async/await mental model, wakers, I/O, pinning, executing multiple futures, `Send` approximation.
- **Which docs use it:** `async-tokio.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### Tokio docs

- **Source family name:** Tokio docs
- **Seed URLs:**
  - https://docs.rs/tokio/latest/tokio/
  - https://docs.rs/tokio/latest/tokio/#feature-flags
  - https://docs.rs/tokio/latest/tokio/attr.main.html
  - https://docs.rs/tokio/latest/tokio/runtime/
  - https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html
  - https://docs.rs/tokio/latest/tokio/task/
  - https://docs.rs/tokio/latest/tokio/task/fn.spawn.html
  - https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html
  - https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html
  - https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
  - https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html
  - https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html
  - https://docs.rs/tokio/latest/tokio/macro.join.html
  - https://docs.rs/tokio/latest/tokio/macro.try_join.html
  - https://docs.rs/tokio/latest/tokio/macro.select.html
  - https://docs.rs/tokio/latest/tokio/sync/index.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.Barrier.html
  - https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html
  - https://docs.rs/tokio/latest/tokio/time/
  - https://docs.rs/tokio/latest/tokio/process/
  - https://docs.rs/tokio/latest/tokio/signal/
  - https://docs.rs/tokio/latest/tokio/net/
  - https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html
  - https://docs.rs/async-trait/latest/async_trait/
  - https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/
- **Seed or discovered:** seed
- **Key topics extracted:** Tokio runtime, tasks, spawning, `JoinHandle`, `join!`/`select!`, async synchronization primitives, time, process, signal, net, cancellation tokens, async traits.
- **Which docs use it:** `async-tokio.md`
- **Authority level:** official crate docs + Rust blog
- **Coverage status:** fully explored

### RustSec and cargo-audit

- **Source family name:** RustSec / cargo-audit
- **Seed URLs:**
  - https://rustsec.org/
  - https://rustsec.org/advisories/
  - https://github.com/rustsec/rustsec
  - https://github.com/rustsec/rustsec/tree/main/cargo-audit
  - https://github.com/rustsec/advisory-db
  - https://github.com/rustsec/advisory-db/blob/main/CONTRIBUTING.md
  - https://rustsec.org/contributing.html
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** advisory database, `cargo audit` subcommands, `audit.toml`, yanked crates, binary scanning.
- **Which docs use it:** `supply-chain-security.md`
- **Authority level:** community (RustSec organization)
- **Coverage status:** fully explored

### cargo-deny

- **Source family name:** cargo-deny
- **Seed URLs:**
  - https://github.com/EmbarkStudios/cargo-deny
  - https://embarkstudios.github.io/cargo-deny/
  - https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html
  - https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html
  - https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html
  - https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html
  - https://embarkstudios.github.io/cargo-deny/cli/check.html
  - https://github.com/EmbarkStudios/cargo-deny/blob/main/deny.template.toml
  - https://github.com/EmbarkStudios/cargo-deny-action
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** advisories, licenses, bans, sources checks, `deny.toml`, CI action.
- **Which docs use it:** `supply-chain-security.md`
- **Authority level:** community (Embark Studios)
- **Coverage status:** fully explored

### cargo-vet

- **Source family name:** cargo-vet
- **Seed URLs:**
  - https://github.com/mozilla/cargo-vet
  - https://mozilla.github.io/cargo-vet/
  - https://raw.githubusercontent.com/mozilla/cargo-vet/main/registry.toml
- **Seed or discovered:** seed + discovered
- **Key topics extracted:** upstream dependency review, audit criteria, supply-chain audits, registry.
- **Which docs use it:** `supply-chain-security.md`
- **Authority level:** community (Mozilla)
- **Coverage status:** fully explored

### Additional supply-chain and tooling

- **Source family name:** Additional supply-chain and validation tooling
- **Seed URLs:**
  - https://github.com/rust-secure-code/cargo-auditable
  - https://github.com/CycloneDX/cyclonedx-rust-cargo
  - https://github.com/rust-secure-code/cargo-supply-chain
  - https://github.com/rust-secure-code/wg
  - https://github.com/taiki-e/cargo-hack
  - https://spdx.org/licenses/
  - https://blog.rust-lang.org/inside-rust/2023/09/01/crates-io-malware-postmortem/
  - https://blog.rust-lang.org/2022/05/10/malicious-crate-rustdecimal/
  - https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html
  - https://github.com/rust-lang/crates.io/issues/10247
  - https://github.com/rust-lang/rfcs/pull/3691
- **Seed or discovered:** seed
- **Key topics extracted:** binary provenance, SBOM generation, supply-chain attack surface, trusted publishing, license identifiers.
- **Which docs use it:** `supply-chain-security.md`
- **Authority level:** community / official rust-lang blog / RFCs
- **Coverage status:** partially explored (SBOM generation and trusted publishing are overview-level)

### Unsafe validation tooling

- **Source family name:** Unsafe validation tooling
- **Seed URLs:**
  - https://github.com/rust-lang/miri
  - https://github.com/RalfJung/cargo-careful
  - https://github.com/tokio-rs/loom
  - https://github.com/awslabs/shuttle
  - https://github.com/geiger-rs/cargo-geiger
  - https://docs.rs/bytemuck/latest/bytemuck/
  - https://docs.rs/zerocopy/latest/zerocopy/
- **Seed or discovered:** seed
- **Key topics extracted:** Miri, AddressSanitizer via cargo-careful, concurrency model checkers (Loom, Shuttle), unsafe dependency scanning, safe transmute crates.
- **Which docs use it:** `unsafe-security.md`
- **Authority level:** community
- **Coverage status:** partially explored (tooling is referenced; detailed usage is left to each tool's docs)

### Error-handling ecosystem crates

- **Source family name:** Error-handling ecosystem crates
- **Seed URLs:**
  - https://docs.rs/thiserror/latest/thiserror/
  - https://docs.rs/anyhow/latest/anyhow/
  - https://docs.rs/miette/latest/miette/
  - https://docs.rs/color-eyre/latest/color_eyre/
- **Seed or discovered:** seed
- **Key topics extracted:** structured errors, type-erased errors, diagnostic reporting.
- **Which docs use it:** `error-handling.md`
- **Authority level:** community
- **Coverage status:** fully explored at API-design level

### RFCs and evolution

- **Source family name:** RFCs and API evolution
- **Seed URLs:**
  - https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md
  - https://github.com/rust-lang/rfcs/blob/master/text/1574-more-api-documentation-conventions.md
  - https://github.com/rust-lang/rfcs/pull/1687
  - https://rust-lang.github.io/rfcs/3338-style-evolution.html
  - https://rust-lang.github.io/rfcs/3389-manifest-lint.html
  - https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html
- **Seed or discovered:** seed
- **Key topics extracted:** breaking changes, documentation conventions, style evolution, manifest lint configuration, trusted publishing.
- **Which docs use it:** `api-design.md`, `documentation-guidelines.md`, `style-formatting.md`, `lints-clippy.md`, `supply-chain-security.md`
- **Authority level:** official rust-lang RFCs
- **Coverage status:** fully explored

### Rust error index

- **Source family name:** Rust error index
- **Seed URLs:**
  - https://doc.rust-lang.org/error_codes/E0004.html
  - https://doc.rust-lang.org/error_codes/E0005.html
  - https://doc.rust-lang.org/error_codes/E0038.html
  - https://doc.rust-lang.org/error_codes/E0117.html
  - https://doc.rust-lang.org/error_codes/E0119.html
  - https://doc.rust-lang.org/error_codes/E0184.html
  - https://doc.rust-lang.org/error_codes/E0204.html
  - https://doc.rust-lang.org/error_codes/E0277.html
  - https://doc.rust-lang.org/error_codes/E0001.html
  - https://doc.rust-lang.org/error_codes/E0002.html
  - https://doc.rust-lang.org/error_codes/E0162.html
  - https://doc.rust-lang.org/error_codes/E0382.html
  - https://doc.rust-lang.org/error_codes/E0502.html
  - https://doc.rust-lang.org/error_codes/E0505.html
  - https://doc.rust-lang.org/error_codes/E0597.html
  - https://doc.rust-lang.org/error_codes/E0106.html
  - https://doc.rust-lang.org/error_codes/E0495.html
- **Seed or discovered:** discovered
- **Key topics extracted:** move errors, borrow errors, lifetime errors, pattern errors, trait bound errors.
- **Which docs use it:** `ownership-lifetimes.md`, `types-traits-generics.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored for the cited errors

### Rust Unstable Book

- **Source family name:** Rust Unstable Book
- **Seed URLs:**
  - https://doc.rust-lang.org/unstable-book/language-features/specialization.html
  - https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html
- **Seed or discovered:** seed
- **Key topics extracted:** specialization (unstable), sanitizer compiler flags.
- **Which docs use it:** `types-traits-generics.md`, `unsafe-security.md`
- **Authority level:** official rust-lang
- **Coverage status:** partially explored (intentionally shallow because features are unstable)

### Rust internals / implementation sources

- **Source family name:** Rust compiler / implementation sources
- **Seed URLs:**
  - https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/resolve.rs
  - https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/encode.rs
- **Seed or discovered:** discovered
- **Key topics extracted:** Cargo resolver implementation details.
- **Which docs use it:** `cargo-dependencies.md`
- **Authority level:** official rust-lang implementation
- **Coverage status:** partially explored (used to confirm resolver behavior)

### Rust blog

- **Source family name:** Rust blog
- **Seed URLs:**
  - https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/
  - https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html
- **Seed or discovered:** seed
- **Key topics extracted:** async fn in traits, let-else statements.
- **Which docs use it:** `async-tokio.md`, `types-traits-generics.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored

### rust-by-example

- **Source family name:** Rust by Example
- **Seed URLs:**
  - https://doc.rust-lang.org/rust-by-example/meta/doc.html
- **Seed or discovered:** seed
- **Key topics extracted:** doc comment examples.
- **Which docs use it:** `editions-tooling.md`
- **Authority level:** official rust-lang
- **Coverage status:** fully explored
