# Standard Library Runtime APIs

## Purpose

This document provides practical, opinionated guidance for AI coding agents working with Rust's standard-library runtime APIs. It focuses on synchronous environment, filesystem, path, process, I/O, and time operations, and separates rules that must always be followed from conventions that depend on context or repo policy. It is intended as a stable reference for producing correct, portable, and reviewable Rust code.

For async counterparts, see [`docs/rust/async-tokio.md`](./async-tokio.md). For error-propagation patterns, `Result`/`?`, and the `anyhow`/`thiserror` ecosystem, see [`docs/rust/error-handling.md`](./error-handling.md).

## Sources used

- [https://doc.rust-lang.org/std/env/](https://doc.rust-lang.org/std/env/)
- [https://doc.rust-lang.org/std/env/fn.var.html](https://doc.rust-lang.org/std/env/fn.var.html)
- [https://doc.rust-lang.org/std/env/fn.var_os.html](https://doc.rust-lang.org/std/env/fn.var_os.html)
- [https://doc.rust-lang.org/std/env/fn.args.html](https://doc.rust-lang.org/std/env/fn.args.html)
- [https://doc.rust-lang.org/std/env/fn.args_os.html](https://doc.rust-lang.org/std/env/fn.args_os.html)
- [https://doc.rust-lang.org/std/env/fn.set_var.html](https://doc.rust-lang.org/std/env/fn.set_var.html)
- [https://doc.rust-lang.org/std/env/fn.remove_var.html](https://doc.rust-lang.org/std/env/fn.remove_var.html)
- [https://doc.rust-lang.org/std/env/fn.current_exe.html](https://doc.rust-lang.org/std/env/fn.current_exe.html)
- [https://doc.rust-lang.org/std/env/enum.VarError.html](https://doc.rust-lang.org/std/env/enum.VarError.html)
- [https://doc.rust-lang.org/std/env/consts/index.html](https://doc.rust-lang.org/std/env/consts/index.html)
- [https://doc.rust-lang.org/std/path/](https://doc.rust-lang.org/std/path/)
- [https://doc.rust-lang.org/std/path/struct.Path.html](https://doc.rust-lang.org/std/path/struct.Path.html)
- [https://doc.rust-lang.org/std/path/struct.PathBuf.html](https://doc.rust-lang.org/std/path/struct.PathBuf.html)
- [https://doc.rust-lang.org/std/path/struct.Components.html](https://doc.rust-lang.org/std/path/struct.Components.html)
- [https://doc.rust-lang.org/std/path/enum.Component.html](https://doc.rust-lang.org/std/path/enum.Component.html)
- [https://doc.rust-lang.org/std/path/enum.Prefix.html](https://doc.rust-lang.org/std/path/enum.Prefix.html)
- [https://doc.rust-lang.org/std/path/constant.MAIN_SEPARATOR.html](https://doc.rust-lang.org/std/path/constant.MAIN_SEPARATOR.html)
- [https://doc.rust-lang.org/std/fs/](https://doc.rust-lang.org/std/fs/)
- [https://doc.rust-lang.org/std/fs/struct.File.html](https://doc.rust-lang.org/std/fs/struct.File.html)
- [https://doc.rust-lang.org/std/fs/struct.OpenOptions.html](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html)
- [https://doc.rust-lang.org/std/fs/struct.ReadDir.html](https://doc.rust-lang.org/std/fs/struct.ReadDir.html)
- [https://doc.rust-lang.org/std/fs/struct.DirEntry.html](https://doc.rust-lang.org/std/fs/struct.DirEntry.html)
- [https://doc.rust-lang.org/std/fs/struct.Metadata.html](https://doc.rust-lang.org/std/fs/struct.Metadata.html)
- [https://doc.rust-lang.org/std/fs/struct.FileType.html](https://doc.rust-lang.org/std/fs/struct.FileType.html)
- [https://doc.rust-lang.org/std/fs/struct.Permissions.html](https://doc.rust-lang.org/std/fs/struct.Permissions.html)
- [https://doc.rust-lang.org/std/fs/struct.DirBuilder.html](https://doc.rust-lang.org/std/fs/struct.DirBuilder.html)
- [https://doc.rust-lang.org/std/fs/fn.read_dir.html](https://doc.rust-lang.org/std/fs/fn.read_dir.html)
- [https://doc.rust-lang.org/std/fs/fn.canonicalize.html](https://doc.rust-lang.org/std/fs/fn.canonicalize.html)
- [https://doc.rust-lang.org/std/fs/fn.create_dir.html](https://doc.rust-lang.org/std/fs/fn.create_dir.html)
- [https://doc.rust-lang.org/std/fs/fn.create_dir_all.html](https://doc.rust-lang.org/std/fs/fn.create_dir_all.html)
- [https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html](https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html)
- [https://doc.rust-lang.org/std/fs/fn.rename.html](https://doc.rust-lang.org/std/fs/fn.rename.html)
- [https://doc.rust-lang.org/std/fs/fn.copy.html](https://doc.rust-lang.org/std/fs/fn.copy.html)
- [https://doc.rust-lang.org/std/fs/fn.soft_link.html](https://doc.rust-lang.org/std/fs/fn.soft_link.html)
- [https://doc.rust-lang.org/std/io/](https://doc.rust-lang.org/std/io/)
- [https://doc.rust-lang.org/std/io/trait.Read.html](https://doc.rust-lang.org/std/io/trait.Read.html)
- [https://doc.rust-lang.org/std/io/trait.Write.html](https://doc.rust-lang.org/std/io/trait.Write.html)
- [https://doc.rust-lang.org/std/io/trait.BufRead.html](https://doc.rust-lang.org/std/io/trait.BufRead.html)
- [https://doc.rust-lang.org/std/io/trait.Seek.html](https://doc.rust-lang.org/std/io/trait.Seek.html)
- [https://doc.rust-lang.org/std/io/struct.BufReader.html](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [https://doc.rust-lang.org/std/io/struct.BufWriter.html](https://doc.rust-lang.org/std/io/struct.BufWriter.html)
- [https://doc.rust-lang.org/std/io/struct.Cursor.html](https://doc.rust-lang.org/std/io/struct.Cursor.html)
- [https://doc.rust-lang.org/std/io/struct.Lines.html](https://doc.rust-lang.org/std/io/struct.Lines.html)
- [https://doc.rust-lang.org/std/io/struct.Empty.html](https://doc.rust-lang.org/std/io/struct.Empty.html)
- [https://doc.rust-lang.org/std/io/struct.Sink.html](https://doc.rust-lang.org/std/io/struct.Sink.html)
- [https://doc.rust-lang.org/std/io/fn.copy.html](https://doc.rust-lang.org/std/io/fn.copy.html)
- [https://doc.rust-lang.org/std/io/fn.stdin.html](https://doc.rust-lang.org/std/io/fn.stdin.html)
- [https://doc.rust-lang.org/std/io/fn.stdout.html](https://doc.rust-lang.org/std/io/fn.stdout.html)
- [https://doc.rust-lang.org/std/io/fn.stderr.html](https://doc.rust-lang.org/std/io/fn.stderr.html)
- [https://doc.rust-lang.org/std/io/type.Result.html](https://doc.rust-lang.org/std/io/type.Result.html)
- [https://doc.rust-lang.org/std/io/struct.Error.html](https://doc.rust-lang.org/std/io/struct.Error.html)
- [https://doc.rust-lang.org/std/io/enum.ErrorKind.html](https://doc.rust-lang.org/std/io/enum.ErrorKind.html)
- [https://doc.rust-lang.org/std/process/](https://doc.rust-lang.org/std/process/)
- [https://doc.rust-lang.org/std/process/struct.Command.html](https://doc.rust-lang.org/std/process/struct.Command.html)
- [https://doc.rust-lang.org/std/process/struct.Child.html](https://doc.rust-lang.org/std/process/struct.Child.html)
- [https://doc.rust-lang.org/std/process/struct.ExitStatus.html](https://doc.rust-lang.org/std/process/struct.ExitStatus.html)
- [https://doc.rust-lang.org/std/process/struct.Output.html](https://doc.rust-lang.org/std/process/struct.Output.html)
- [https://doc.rust-lang.org/std/process/struct.Stdio.html](https://doc.rust-lang.org/std/process/struct.Stdio.html)
- [https://doc.rust-lang.org/std/process/fn.exit.html](https://doc.rust-lang.org/std/process/fn.exit.html)
- [https://doc.rust-lang.org/std/process/fn.abort.html](https://doc.rust-lang.org/std/process/fn.abort.html)
- [https://doc.rust-lang.org/std/process/struct.ExitCode.html](https://doc.rust-lang.org/std/process/struct.ExitCode.html)
- [https://doc.rust-lang.org/std/process/trait.Termination.html](https://doc.rust-lang.org/std/process/trait.Termination.html)
- [https://doc.rust-lang.org/std/time/](https://doc.rust-lang.org/std/time/)
- [https://doc.rust-lang.org/std/time/struct.Instant.html](https://doc.rust-lang.org/std/time/struct.Instant.html)
- [https://doc.rust-lang.org/std/time/struct.Duration.html](https://doc.rust-lang.org/std/time/struct.Duration.html)
- [https://doc.rust-lang.org/std/time/struct.SystemTime.html](https://doc.rust-lang.org/std/time/struct.SystemTime.html)
- [https://doc.rust-lang.org/std/time/struct.SystemTimeError.html](https://doc.rust-lang.org/std/time/struct.SystemTimeError.html)
- [https://doc.rust-lang.org/std/time/constant.UNIX_EPOCH.html](https://doc.rust-lang.org/std/time/constant.UNIX_EPOCH.html)

## Core guidance

1. **Use `std::path::Path` and `std::path::PathBuf` for every filesystem path.** Never use `String`, `&str`, or byte arrays as paths. String-based paths break on Windows and on any path containing invalid UTF-8 (see [`Path`](https://doc.rust-lang.org/std/path/struct.Path.html) and [`PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html)).
2. **Propagate `std::io::Error` with `?`.** Most runtime APIs return `io::Result<T> = Result<T, io::Error>`. Let callers decide how to report and recover. For error enrichment and library design, see [`docs/rust/error-handling.md`](./error-handling.md).
3. **Buffer all loop I/O.** `std::fs::File` implements `Read`/`Write` but performs no buffering. Use `BufReader`, `BufWriter`, or the whole-file helpers (`fs::read`, `fs::write`) to avoid a syscall per operation (see [`BufReader`](https://doc.rust-lang.org/std/io/struct.BufReader.html), [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html)).
4. **Flush `BufWriter` explicitly or via `into_inner()`.** Dropping a `BufWriter` attempts a flush but **ignores errors**, which is a silent data-loss footgun (see [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html)).
5. **Check process exit status before using captured output.** `Command::output` returning `Ok(Output)` only means the child was spawned; inspect `Output::status.success()` or `ExitStatus::success()` (see [`Output`](https://doc.rust-lang.org/std/process/struct.Output.html), [`ExitStatus`](https://doc.rust-lang.org/std/process/struct.ExitStatus.html)).
6. **Reap every `Child`.** [`Child`](https://doc.rust-lang.org/std/process/struct.Child.html) has no `Drop`; dropping it leaks a zombie process. Always call `wait`, `try_wait`, or `wait_with_output`.
7. **Use monotonic `Instant` for elapsed time and timeouts.** Use `SystemTime` only for wall-clock timestamps such as file mtimes or calendar values. `SystemTime` can jump backwards; `Instant` is monotonic (see [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), [`SystemTime`](https://doc.rust-lang.org/std/time/struct.SystemTime.html)).
8. **Prefer checked arithmetic on `Duration` and `Instant`.** The panicking operators (`+`, `-`, `*`, `/`) and `Duration::new` can abort the process on overflow or invalid input. Use `checked_*`, `saturating_*`, and `try_*` variants in production code.
9. **Route async work to Tokio equivalents.** In `tokio` code, use `tokio::fs`, `tokio::process::Command`, `tokio::time`, and `tokio::io`. Never call blocking `std::fs`/`std::process` on a runtime worker thread without `spawn_blocking` (see [`docs/rust/async-tokio.md`](./async-tokio.md), sections [`tokio::time` line 405](./async-tokio.md) and [`tokio::process` line 423](./async-tokio.md)).
10. **Treat `ErrorKind` as non-exhaustive.** `io::ErrorKind` is `#[non_exhaustive]`. Any `match` on it must include a `_ =>` wildcard arm, and code must treat `Interrupted` and `WouldBlock` as non-fatal.

## Practical rules

### `std::env`

[`std::env`](https://doc.rust-lang.org/std/env/) provides access to the process environment, command-line arguments, and related process metadata.

- **Read environment variables with the right function.** [`std::env::var`](https://doc.rust-lang.org/std/env/fn.var.html) returns `Result<String, VarError>`; it fails with `VarError::NotPresent` when the variable is unset or the key is invalid, and with `VarError::NotUnicode(OsString)` when the value is not valid UTF-8. Use [`std::env::var_os`](https://doc.rust-lang.org/std/env/fn.var_os.html) when the value may be non-UTF-8: it returns `Option<OsString>` and never errors on non-Unicode (see [`VarError`](https://doc.rust-lang.org/std/env/enum.VarError.html)).
- **Read arguments with `args_os`, not `args`, unless you are certain they are UTF-8.** [`std::env::args`](https://doc.rust-lang.org/std/env/fn.args.html) yields `String` and **panics during iteration** if any argument is non-UTF-8. [`std::env::args_os`](https://doc.rust-lang.org/std/env/fn.args_os.html) yields `OsString` and is safe for arbitrary argument bytes. `argv[0]` is attacker-controlled; never rely on it for security.
- **Mutating the environment is `unsafe` since Rust 1.66.** [`std::env::set_var`](https://doc.rust-lang.org/std/env/fn.set_var.html) and [`std::env::remove_var`](https://doc.rust-lang.org/std/env/fn.remove_var.html) are `unsafe fn` because on Unix other threads (including the standard library's DNS resolver) may read the environment without synchronization. They are safe only in single-threaded programs or on Windows. Both panic if the key is empty or contains `=` or `\0`. To influence child processes, prefer `Command::env`, `Command::env_remove`, or `Command::env_clear` instead.
- **Current directory and executable path.** [`std::env::current_dir`](https://doc.rust-lang.org/std/env/fn.current_dir.html) returns `io::Result<PathBuf>`. [`std::env::current_exe`](https://doc.rust-lang.org/std/env/fn.current_exe.html) also returns `io::Result<PathBuf>`, but it is platform-specific and may return a symlink path or pre-rename path. In setuid/setgid programs it is untrusted, and on Linux it can be attacker-controlled via `PATH` or hardlinks.
- **Temporary and home directories are shared and brittle.** [`std::env::temp_dir`](https://doc.rust-lang.org/std/env/fn.temp_dir.html) returns `TMPDIR` on Unix or `TMP`/`TEMP` on Windows; predictable names under this directory are insecure. Use the `tempfile` crate or random names. [`std::env::home_dir`](https://doc.rust-lang.org/std/env/fn.home_dir.html) is no longer deprecated (deprecated in 1.29.0, restored in 1.85.0) but is brittle and inconsistent across platforms; prefer the `dirs` crate or explicit XDG/APPDATA lookup.
- **Compile-time target constants.** [`std::env::consts`](https://doc.rust-lang.org/std/env/consts/index.html) exposes `OS`, `ARCH`, `FAMILY`, `EXE_SUFFIX`, `DLL_EXTENSION`, and related strings evaluated at compile time.
- **Sanitize child environments with `env_clear`.** When spawning a subprocess whose behavior must not depend on the parent's environment, call `Command::env_clear()` and then add only the variables the child needs with `env`. This is safer than mutating the global environment with `set_var`.

```rust,no_run
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // UTF-8 required: use var.
    let user: String = env::var("USER")?;
    println!("user={}", user);

    // May be non-UTF-8: use var_os.
    if let Some(tmp) = env::var_os("TMP") {
        println!("tmp={:?}", tmp);
    }

    // argv[0] is not security-relevant; args_os avoids panics.
    for arg in env::args_os() {
        println!("arg={:?}", arg);
    }

    let here: PathBuf = env::current_dir()?;
    let exe: PathBuf = env::current_exe()?;
    println!("cwd={:?} exe={:?}", here, exe);

    println!("target os={} arch={}", env::consts::OS, env::consts::ARCH);
    Ok(())
}
```

### `std::path`

[`std::path`](https://doc.rust-lang.org/std/path/) is the correct way to represent filesystem paths in Rust.

- **Use `Path` for borrowed paths and `PathBuf` for owned paths.** [`Path`](https://doc.rust-lang.org/std/path/struct.Path.html) is to `PathBuf` as `str` is to `String`: `PathBuf` derefs to `Path`. Use `&Path` in function signatures and `PathBuf` as owned storage (see [`PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html)).
- **Path operations are mostly syntactic.** Everything in `Path`/`PathBuf` is string/syntactic manipulation except the explicitly-filesystem methods: `metadata`, `symlink_metadata`, `canonicalize`, `read_link`, `read_dir`, `exists`, `try_exists`, `is_file`, `is_dir`, and `is_symlink`.
- **Case sensitivity is uniform.** [`Path::starts_with`](https://doc.rust-lang.org/std/path/struct.Path.html#method.starts_with), [`Path::ends_with`](https://doc.rust-lang.org/std/path/struct.Path.html#method.ends_with), and equality are case-sensitive on all platforms, except that Windows drive-letter prefixes may compare case-insensitively.
- **Normalization is limited.** [`Path::components`](https://doc.rust-lang.org/std/path/struct.Path.html#method.components) and comparisons collapse repeated separators, non-leading `.`, and trailing separators, but they **do not resolve `..` or symlinks**. Use [`std::fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html) for that.
- **String conversions are fallible.** [`Path::to_str`](https://doc.rust-lang.org/std/path/struct.Path.html#method.to_str) returns `Option<&str>` (None on non-UTF-8). [`Path::to_string_lossy`](https://doc.rust-lang.org/std/path/struct.Path.html#method.to_string_lossy) returns `Cow<str>` and replaces invalid sequences with `U+FFFD`; it is for display only and must never be used for filesystem operations. [`PathBuf::into_os_string`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.into_os_string) and the `From` impls are the correct round-trip conversions.
- **Conversions from strings and collections.** `PathBuf` implements `From<String>`, `From<OsString>`, `From<&str>`, `From<&OsStr>`, `FromIterator` over items implementing `AsRef<Path>`, and `From<PathBuf>` for `OsString` and `Box<Path>`. Prefer these over string concatenation.
- **Absolute paths differ across platforms.** [`Path::is_absolute`](https://doc.rust-lang.org/std/path/struct.Path.html#method.is_absolute): on Unix it requires a leading `/`; on Windows it requires both a prefix (e.g., `C:`) and a root (`\`). Therefore `c:temp` and `\temp` are **not** absolute on Windows.
- **Joining an absolute path replaces the receiver.** [`Path::join`](https://doc.rust-lang.org/std/path/struct.Path.html#method.join) and [`PathBuf::push`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.push) replace the existing path if the appended path is absolute. This is a common source of bugs when building paths from user input.
- **Parent, ancestors, and file components.** [`Path::parent`](https://doc.rust-lang.org/std/path/struct.Path.html#method.parent) returns `Option<&Path>`; it returns `None` for root, prefix, or empty paths, and `Some("")` for a single relative component. [`Path::ancestors`](https://doc.rust-lang.org/std/path/struct.Path.html#method.ancestors) iterates from self toward root. [`Path::file_name`](https://doc.rust-lang.org/std/path/struct.Path.html#method.file_name), [`Path::file_stem`](https://doc.rust-lang.org/std/path/struct.Path.html#method.file_stem), [`Path::extension`](https://doc.rust-lang.org/std/path/struct.Path.html#method.extension), and [`Path::file_prefix`](https://doc.rust-lang.org/std/path/struct.Path.html#method.file_prefix) (1.91) follow the rule that dotfiles such as `.bashrc` have no extension.
- **Prefix stripping is whole-component only.** [`Path::strip_prefix`](https://doc.rust-lang.org/std/path/struct.Path.html#method.strip_prefix) succeeds only on whole components. [`Path::starts_with`](https://doc.rust-lang.org/std/path/struct.Path.html#method.starts_with) and [`Path::ends_with`](https://doc.rust-lang.org/std/path/struct.Path.html#method.ends_with) also operate on whole components, not substrings.
- **Components and separators.** [`Path::components`](https://doc.rust-lang.org/std/path/struct.Path.html#method.components) returns a [`Components`](https://doc.rust-lang.org/std/path/struct.Components.html) iterator of [`Component`](https://doc.rust-lang.org/std/path/enum.Component.html) values: `Prefix(PrefixComponent)` (Windows only), `RootDir`, `CurDir` (`.`), `ParentDir` (`..`), and `Normal(&OsStr)`. Only `Normal` carries user data. Windows [`Prefix`](https://doc.rust-lang.org/std/path/enum.Prefix.html) includes verbatim `\\?\` paths, verbatim UNC, disk, device namespace, UNC, and disk prefixes; in verbatim paths, `/` is **not** a separator. Use [`MAIN_SEPARATOR`](https://doc.rust-lang.org/std/path/constant.MAIN_SEPARATOR.html) or [`MAIN_SEPARATOR_STR`](https://doc.rust-lang.org/std/path/constant.MAIN_SEPARATOR_STR.html) instead of hardcoding separators.
- **Existence checks.** [`Path::exists`](https://doc.rust-lang.org/std/path/struct.Path.html#method.exists) follows symlinks and coerces **all** errors to `false`, including permission errors, which makes it TOCTOU-prone. Prefer [`Path::try_exists`](https://doc.rust-lang.org/std/path/struct.Path.html#method.try_exists) (1.63), which returns `io::Result<bool>` and propagates errors.
- **Canonicalize requires existence.** [`std::fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html) resolves all symlinks and requires the path to exist. On Windows it returns an extended-length UNC path (`\\?\`).
- **Setting extensions can panic.** [`PathBuf::set_extension`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.set_extension) and [`PathBuf::add_extension`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.add_extension) (1.91) **panic** if the extension contains a path separator. `set_extension("")` removes the extension.

```rust
use std::path::{Path, PathBuf};

fn main() {
    let base = Path::new("/tmp");
    let mut log: PathBuf = base.join("logs").join("app.log");
    assert_eq!(log.file_name(), Some(std::ffi::OsStr::new("app.log")));
    assert_eq!(log.extension(), Some(std::ffi::OsStr::new("log")));

    // Join with absolute path replaces the receiver.
    let abs = Path::new("/etc");
    let joined = log.join(abs);
    assert_eq!(joined, Path::new("/etc"));

    // Set extension safely.
    log.set_extension("txt");
    assert_eq!(log.file_name(), Some(std::ffi::OsStr::new("app.txt")));

    // Try to exists to preserve permission errors.
    match log.try_exists() {
        Ok(true) => println!("exists"),
        Ok(false) => println!("missing"),
        Err(e) => println!("error: {}", e),
    }
}
```

### `std::fs`

[`std::fs`](https://doc.rust-lang.org/std/fs/) exposes synchronous filesystem operations. All public functions return `io::Result<T>` and the module documents TOCTOU risks, including symlink-swap attacks on [`std::fs::remove_dir_all`](https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html).

- **Whole-file helpers.** [`std::fs::read`](https://doc.rust-lang.org/std/fs/fn.read.html) returns `Result<Vec<u8>>`, [`std::fs::read_to_string`](https://doc.rust-lang.org/std/fs/fn.read_to_string.html) returns `Result<String>` and errors with `InvalidData` if the bytes are not valid UTF-8, and [`std::fs::write`](https://doc.rust-lang.org/std/fs/fn.write.html) creates or truncates and writes in one call. These helpers automatically retry `ErrorKind::Interrupted`. They are good for small whole-file operations.
- **Directory iteration.** [`std::fs::read_dir`](https://doc.rust-lang.org/std/fs/fn.read_dir.html) returns `Result<ReadDir>`; [`ReadDir`](https://doc.rust-lang.org/std/fs/struct.ReadDir.html) is an `Iterator<Item = io::Result<DirEntry>>` that skips `.` and `..`. The **order is not guaranteed and may change between calls**; sort entries if order matters. Mid-iteration errors are yielded as `Err` items, so handle both `Ok` and `Err`.
- **DirEntry methods.** [`DirEntry::path`](https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.path) returns a `PathBuf`; [`DirEntry::file_name`](https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_name) returns `OsString`; [`DirEntry::file_type`](https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_type) and [`DirEntry::metadata`](https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.metadata) return `io::Result`. Notably, `DirEntry::metadata` does **not** follow symlinks and is cheap on Windows.
- **File opening modes.** [`File::open`](https://doc.rust-lang.org/std/fs/struct.File.html#method.open) is **read-only**. [`File::create`](https://doc.rust-lang.org/std/fs/struct.File.html#method.create) is write-only and create-or-truncate. [`File::create_new`](https://doc.rust-lang.org/std/fs/struct.File.html#method.create_new) (1.77) is read+write, atomic, and returns `AlreadyExists` if the file exists; use it for "create-if-missing" to avoid TOCTOU. [`File::options`](https://doc.rust-lang.org/std/fs/struct.File.html#method.options) returns an `OpenOptions` builder accessor.
- **OpenOptions semantics.** [`std::fs::OpenOptions`](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html) provides `read`, `write`, `append`, `truncate`, `create`, and `create_new`. `write(true)` alone does **not** truncate. `create(true)` requires `write` or `append`, else `InvalidInput`. `create_new(true)` overrides `create`/`truncate`, is atomic, and requires `write` or `append`. `append(true)` makes each `write` atomic at the end of the file and is safe under concurrent appenders.
- **File I/O and metadata.** [`File`](https://doc.rust-lang.org/std/fs/struct.File.html) implements `Read`, `Write`, and `Seek` with **no buffering**. Wrap it in `BufReader`/`BufWriter` for repeated small operations. Use [`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all) to flush data and metadata, or [`File::sync_data`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data) to skip metadata. [`File::set_len`](https://doc.rust-lang.org/std/fs/struct.File.html#method.set_len) truncates or extends (zero-fills). Dropping a `File` ignores close errors. [`File::try_clone`](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_clone) duplicates the handle.
- **Timestamps and times.** [`File::set_times`](https://doc.rust-lang.org/std/fs/struct.File.html#method.set_times) and [`File::set_modified`](https://doc.rust-lang.org/std/fs/struct.File.html#method.set_modified) (1.75) mutate file times. [`File::lock_shared`](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock_shared), [`lock`](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock), and [`try_lock`](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) (1.89) provide advisory file locking on supported platforms.
- **Metadata and file types.** [`std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html) follows symlinks (`stat`); [`std::fs::symlink_metadata`](https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html) does not (`lstat`). [`Metadata`](https://doc.rust-lang.org/std/fs/struct.Metadata.html) provides `is_file`, `is_dir`, `is_symlink` (mutually exclusive), `len()`, and `permissions()`. The time methods `modified`, `accessed`, and `created` return `io::Result<SystemTime>` (not `Option`) and can return `Err(ErrorKind::Unsupported)` on older filesystems or Linux. `is_file` is not authoritative for readability: pipes, sockets, and devices exist.
- **Directory creation and removal.** [`std::fs::create_dir`](https://doc.rust-lang.org/std/fs/fn.create_dir.html) creates a single directory and fails if the parent is missing or the directory exists. [`std::fs::create_dir_all`](https://doc.rust-lang.org/std/fs/fn.create_dir_all.html) is recursive and race-safe against concurrent calls (an `EEXIST` race is treated as success), but it is not atomic and partial parents may remain on error. [`std::fs::remove_dir`](https://doc.rust-lang.org/std/fs/fn.remove_dir.html) requires an empty directory. [`std::fs::remove_dir_all`](https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html) is recursive, not idempotent (errors if missing), does not follow symlinks, and defends against symlink-swap TOCTOU on most Unix systems but not under Miri or Redox.
- **Rename and copy.** [`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html) is atomic on the same filesystem and replaces the destination, but it **fails across mount points** with `ErrorKind::CrossesDevices` (Unix `EXDEV`). Use copy+delete for cross-filesystem moves. [`std::fs::copy`](https://doc.rust-lang.org/std/fs/fn.copy.html) copies contents and permission bits, overwrites the destination, and is not atomic. Source must be a regular file or a symlink to one.
- **Links.** [`std::fs::hard_link`](https://doc.rust-lang.org/std/fs/fn.hard_link.html) usually requires the same filesystem. [`std::fs::soft_link`](https://doc.rust-lang.org/std/fs/fn.soft_link.html) is **deprecated since Rust 1.1.0**; use `std::os::unix::fs::symlink` or `std::os::windows::fs::{symlink_file, symlink_dir}` (Windows requires Developer Mode or admin privilege).
- **Permissions footgun.** [`std::fs::Permissions`](https://doc.rust-lang.org/std/fs/struct.Permissions.html) portable API is only `readonly()` and `set_readonly(bool)`. On Unix, **`set_readonly(false)` makes the file world-writable (`chmod a+w`)**. Use `std::os::unix::fs::PermissionsExt::set_mode` instead. `readonly` ignores ACLs, groups, and root; it is a heuristic, not authorization.
- **DirBuilder.** [`std::fs::DirBuilder`](https://doc.rust-lang.org/std/fs/struct.DirBuilder.html) supports recursive creation; Unix default mode is `0o777 & ~umask`. Use `std::os::unix::fs::DirBuilderExt::mode` to control it.
- **Cross-platform extensions.** `OpenOptionsExt` (Unix default mode `0o666`; Windows access/share/custom flags), `PermissionsExt`, `FileExt` (positional `read_at`/`write_at` on Unix), and `MetadataExt` (raw `stat` fields) expose platform details. `ErrorKind` is `#[non_exhaustive]`: always include a `_ =>` wildcard arm in `match`.

```rust,no_run
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufRead, Write};

fn main() -> Result<(), std::io::Error> {
    // Whole-file helpers for small data.
    let text = fs::read_to_string("config.toml")?;
    fs::write("out.txt", "hello")?;
    fs::create_dir_all("data/cache")?;

    // Streaming read with buffering.
    let file = File::open("large.log")?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.starts_with("ERROR") {
            println!("{}", line);
        }
    }

    // Append mode: each write_all is atomic at EOF.
    let mut append = OpenOptions::new()
        .append(true)
        .create(true)
        .open("log.txt")?;
    append.write_all(b"entry\n")?;
    append.flush()?;

    // Atomic create-if-missing (Rust 1.77+).
    let _new = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open("lock.json")?;

    Ok(())
}
```

### `std::io`

[`std::io`](https://doc.rust-lang.org/std/io/) defines the core byte-stream traits, buffered adapters, and utility types. `io::Result<T>` is `Result<T, io::Error>`.

- **`Read` semantics.** [`std::io::Read::read`](https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read) fills the provided buffer and returns `Ok(n)`. `n` may be less than `buf.len()`; that is legal and not an error. `Ok(0)` means EOF. Provided methods include `read_to_end`, `read_to_string` (validates UTF-8, returns `InvalidData` on failure), `read_exact` (returns `UnexpectedEof` if the stream ends early), `bytes()` (inefficient on unbuffered readers), `chain`, and `take`. High-level methods auto-retry `ErrorKind::Interrupted`.
- **`Write` semantics.** [`std::io::Write::write`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write) returns `Ok(n)` where partial writes are **legal and not errors**. It also retries `Interrupted`. Implementors must also provide `flush`. Provided methods include `write_all` (loops until everything is written) and `write_fmt` (used by the `write!` macro). **Never assume `write` wrote everything; use `write_all`.**
- **`BufRead` line and delimiter APIs.** [`std::io::BufRead`](https://doc.rust-lang.org/std/io/trait.BufRead.html) extends `Read` with `fill_buf` and `consume`. Provided methods include `read_until` (delimiter included in the returned buffer), `read_line` (newline included), `skip_until` (1.83), `split` (delimiter excluded from yielded `Vec<u8>`), and `lines` (newline stripped, item is `Result<String>`). `read_line` and `read_until` are blocking and DoS-able by never-terminating input; cap with `.take(limit)` on untrusted streams.
- **`Seek`.** [`std::io::Seek::seek`](https://doc.rust-lang.org/std/io/trait.Seek.html#tymethod.seek) accepts `SeekFrom::Start(u64)`, `SeekFrom::End(i64)`, or `SeekFrom::Current(i64)`. Negative offsets are errors; seeking beyond EOF is allowed. Use `rewind` (1.55), `stream_position` (1.51), and `seek_relative` (1.80, efficient on `BufReader`).
- **Buffering rule.** Unbuffered reads or writes in loops are roughly one syscall per byte. Always wrap looped I/O in `BufReader` or `BufWriter` (default 8 KiB; tune with `with_capacity`).
- **`BufReader` caveats.** [`BufReader::into_inner`](https://doc.rust-lang.org/std/io/struct.BufReader.html#method.into_inner) drops leftover buffered data. Any `seek` discards the buffer; use `seek_relative` to avoid that.
- **`BufWriter` data-loss footgun.** [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) buffers writes but its `Drop` attempts to flush and **ignores errors**. Always call `flush()` before dropping, or call `into_inner()` (returns `Result<W, IntoInnerError>`), or use `into_parts()` (1.56) for panic-free recovery. `BufWriter::seek` flushes the buffer first.
- **`Cursor`.** [`std::io::Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html) provides in-memory `Read`/`BufRead`/`Seek` over `&[u8]`, `Vec<u8>`, arrays, and `Box<[u8]>`. It also implements `Write` for `&mut [u8]`, `Vec<u8>`, and similar. **`Cursor::new(vec)` starts at position 0 and overwrites; it does not append.** Use `position()`/`set_position()` to control it.
- **`io::copy`.** [`std::io::copy`](https://doc.rust-lang.org/std/io/fn.copy.html) copies from a `Read` to a `Write`, returns the byte count, auto-retries `Interrupted`, and on Linux may use zero-copy syscalls (`copy_file_range`, `sendfile`, `splice`).
- **Stdin/stdout/stderr.** [`std::io::stdin`](https://doc.rust-lang.org/std/io/fn.stdin.html), [`stdout`](https://doc.rust-lang.org/std/io/fn.stdout.html), and [`stderr`](https://doc.rust-lang.org/std/io/fn.stderr.html) return handles that share the underlying fd and a global buffer plus lock. The `Read` impl on `Stdin` locks per call. For many small ops, call `lock()` once to get a `StdinLock<'static>` (implements `Read` + `BufRead`). `Stdin::read_line` locks internally and includes the newline; `Stdin::lines` (1.62) consumes the handle. `Stdout` is line-buffered to terminals and auto-flushed on drop.
- **`io::Error` and `io::ErrorKind`.** [`io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html) can be constructed with `Error::new`, `Error::other` (1.74), `last_os_error`, `from_raw_os_error`, and `From<ErrorKind>`. Inspect it with `kind()`, `raw_os_error()`, and `downcast` (1.79). [`io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html) is `#[non_exhaustive]`; always include a `_ =>` wildcard arm. Common variants include `NotFound`, `PermissionDenied`, `AlreadyExists`, `InvalidInput`, `InvalidData`, `TimedOut`, `Interrupted`, `UnexpectedEof`, `Unsupported`, `WouldBlock`, `BrokenPipe`, `IsADirectory`, `NotADirectory`, `DirectoryNotEmpty` (1.83), and `CrossesDevices` (1.85). Treat `Interrupted` and `WouldBlock` as non-fatal.
- **Test doubles.** [`io::empty`](https://doc.rust-lang.org/std/io/fn.empty.html) reads `Ok(0)` and ignores writes; [`io::sink`](https://doc.rust-lang.org/std/io/fn.sink.html) writes succeed with the full length; [`io::repeat`](https://doc.rust-lang.org/std/io/fn.repeat.html) yields a repeating byte.

```rust,no_run
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

fn main() -> Result<(), std::io::Error> {
    // Lock stdin for line-oriented processing.
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        println!("{}", line);
    }

    // Buffered writer: flush explicitly.
    let out = File::create("out.bin")?;
    let mut writer = BufWriter::new(out);
    writer.write_all(&[1, 2, 3])?;
    writer.flush()?;

    // Cursor overwrites; position controls where.
    let mut buf = b"hello world".to_vec();
    let mut cursor = io::Cursor::new(&mut buf);
    cursor.set_position(6);
    cursor.write_all(b"rust")?;
    assert_eq!(&buf[..], b"hello rustt");

    Ok(())
}
```

### `std::process`

[`std::process`](https://doc.rust-lang.org/std/process/) runs and controls external programs.

- **`Command` is not a shell.** [`std::process::Command::new`](https://doc.rust-lang.org/std/process/struct.Command.html#method.new) takes a program path and runs it directly. It does **not** interpret pipes, globs, `$VAR`, quotes, or spaces. Pass program and arguments separately with `arg`/`args`. If you need shell features, explicitly spawn `sh -c script` (Unix) or `cmd /C script` (Windows), and **never** pass untrusted input into the script body. Windows `.bat`/`.cmd` files use non-standard argument parsing and are vulnerable to injection.
- **Builder methods.** `arg`, `args`, `env` (overrides inherited; case-insensitive on Windows), `envs`, `env_remove`, `env_clear` (clears and blocks all inheritance — use for sanitizing), `current_dir`, and `stdin`/`stdout`/`stderr`. Getters are available since 1.57. A `Command` is reusable; cloning or reusing it is allowed.
- **Three execution modes.** [`spawn`](https://doc.rust-lang.org/std/process/struct.Command.html#method.spawn) returns `io::Result<Child>` and runs concurrently (stdio inherited by default). [`status`](https://doc.rust-lang.org/std/process/struct.Command.html#method.status) waits and returns `io::Result<ExitStatus>` (stdio inherited; no capture). [`output`](https://doc.rust-lang.org/std/process/struct.Command.html#method.output) waits and captures stdout/stderr via pipes; stdin is closed immediately. The `io::Result` wrapping these methods reflects only the **spawn stage**, not child success.
- **`Child` has no `Drop` implementation.** Dropping a [`Child`](https://doc.rust-lang.org/std/process/struct.Child.html) without calling `wait`, `try_wait`, or `wait_with_output` **leaks a zombie**. `Child::kill` sends `SIGKILL` on Unix but does not reap; you must still `wait` afterward. `wait` closes stdin first (avoiding deadlock) and reaps. `try_wait` is non-blocking and does not close stdin. `wait_with_output` closes stdin, drains stdout/stderr, and reaps.
- **Pipe-buffer deadlock.** If both stdin and stdout are piped, writing more than the pipe buffer to stdin while waiting to read stdout can deadlock: the child blocks writing stdout while the parent blocks writing stdin. Fixes: write stdin from a separate thread while main reads stdout; use `wait_with_output` (closes stdin first); or structure the child to read all stdin before writing all stdout.
- **`Stdio` variants.** [`Stdio::inherit`](https://doc.rust-lang.org/std/process/struct.Stdio.html) (default for spawn/status), [`Stdio::piped`](https://doc.rust-lang.org/std/process/struct.Stdio.html) (capture or connect), [`Stdio::null`](https://doc.rust-lang.org/std/process/struct.Stdio.html) (`/dev/null`). `Stdio` implements `From<File>`, `From<OwnedFd>`/`From<OwnedHandle>`, `From<ChildStd*>`, `From<Stdout>`/`From<Stderr>` (1.74), and unsafe `FromRawFd`/`FromRawHandle`. `ChildStdin` implements `Write`; `ChildStdout` and `ChildStderr` implement `Read`.
- **`ExitStatus` interpretation.** [`ExitStatus::success`](https://doc.rust-lang.org/std/process/struct.ExitStatus.html#method.success) is the portable success check. [`ExitStatus::code`](https://doc.rust-lang.org/std/process/struct.ExitStatus.html#method.code) returns `Option<i32>`; on Unix it is truncated to 8 bits and is `None` if the child was terminated by a signal. Values such as 255, 254, 127, or 126 may be invented by the runtime. Use `ExitStatusExt` for Unix signal details. `ExitStatus` is `Copy + Default` (default is success since 1.73).
- **`Output` and exit helpers.** [`Output`](https://doc.rust-lang.org/std/process/struct.Output.html) contains `status: ExitStatus`, `stdout: Vec<u8>`, and `stderr: Vec<u8>`. [`std::process::exit`](https://doc.rust-lang.org/std/process/fn.exit.html) terminates immediately, runs **no Rust destructors**, and on Unix truncates the code to 8 bits. [`std::process::abort`](https://doc.rust-lang.org/std/process/fn.abort.html) raises `SIGABRT` with no destructors and no flush. Prefer returning `Result` or [`ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html) from `main`; implement [`Termination`](https://doc.rust-lang.org/std/process/trait.Termination.html) (1.61) for custom exit behavior.
- **Cross-platform extensions.** Unix `CommandExt` provides uid/gid, `pre_exec` (unsafe), `exec`, `arg0`, and process-group control. Windows `CommandExt` provides `creation_flags` and `raw_arg`. Use `cfg` to gate signal- or UID-specific logic.

```rust,no_run
use std::process::{Command, Stdio};
use std::thread;

fn run_filtered(pattern: &str) -> Result<String, std::io::Error> {
    let mut child = Command::new("grep")
        .arg(pattern)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write stdin from a thread to avoid pipe-buffer deadlock.
    if let Some(mut stdin) = child.stdin.take() {
        let input = b"one\ntwo\nthree\n".to_vec();
        thread::spawn(move || {
            use std::io::Write;
            let _ = stdin.write_all(&input);
        });
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!("grep failed: {}", err)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### `std::time`

[`std::time`](https://doc.rust-lang.org/std/time/) provides types for measuring and representing time.

- **`Instant` is monotonic.** [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html) measures elapsed time and timeouts. It is backed by `CLOCK_MONOTONIC` on Unix, `CLOCK_UPTIME_RAW` on Darwin, and `QueryPerformanceCounter` on Windows. It is monotonic but not steady (rate may be adjusted by the OS). Use `now`, `duration_since(earlier)`, `elapsed`, `checked_duration_since` (1.39), and `saturating_duration_since` (1.39). **Current behavior: `duration_since`, `elapsed`, and `Instant - Instant` saturate to zero when `earlier > self`.** Pre-1.34 they panicked; future versions may reintroduce a panic. Use `checked_duration_since` to detect ordering. `Instant + Duration` and `Instant - Duration` panic on out-of-range results; use `checked_add`/`checked_sub` (1.34) for large or untrusted offsets. Durations up to ~100 years are generally safe, but ~31.6 billion seconds panics on macOS. `Instant` is opaque and not comparable across machines or reboots.
- **`Duration` is a span.** [`std::time::Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html) stores `u64` seconds and `u32` nanoseconds. Constructors `from_secs`, `from_millis`, `from_micros`, and `from_nanos` are infallible, const, and never panic. `from_nanos_u128` (1.93) panics if it exceeds `MAX`. **`Duration::new(secs, nanos)` panics if the nanos carry overflows `secs`.** `from_secs_f64`/`from_secs_f32` panic on negative, NaN, infinite, or overflowing input; use `try_from_secs_f64`/`try_from_secs_f32` (1.66) for untrusted floats. The `+ - * /` operators panic on overflow or divide-by-zero; prefer `checked_add`/`sub`/`mul(u32)`/`div(u32)` (1.16) or `saturating_add`/`sub`/`mul` (1.53). `Duration::ZERO` and `Duration::MAX` are available since 1.53. Accessors include `as_secs`, `as_millis`, `as_micros`, `as_nanos` (returns `u128`), `as_secs_f64`, `as_secs_f32`, and `is_zero`. **Duration does not implement `Display`; format it manually or use a crate.**
- **`SystemTime` is wall clock.** [`std::time::SystemTime`](https://doc.rust-lang.org/std/time/struct.SystemTime.html) is for timestamps, file mtimes, and network time. It is **not monotonic** and can jump backwards. It does not count leap seconds; behavior near a leap second is platform-dependent. Use `now`, `duration_since(earlier)` (returns `Result<Duration, SystemTimeError>` and **errors** if `earlier > self`, not saturates), `elapsed`, and `checked_add`/`checked_sub` (1.34). `SystemTime + Duration` and `SystemTime - Duration` panic on out-of-range. There is no `Sub` impl for `SystemTime - SystemTime`; use `duration_since`. Canonical Unix seconds: `now().duration_since(UNIX_EPOCH)?.as_secs()` (can fail if the clock is before 1970; see [`UNIX_EPOCH`](https://doc.rust-lang.org/std/time/constant.UNIX_EPOCH.html)). `SystemTimeError::duration` gives the positive discrepancy.
- **Rules of thumb.** Use `Instant` for elapsed/timeout logic. Use `SystemTime` only for wall-clock interfaces. Prefer checked/saturating/try arithmetic over panicking operators in production. Do not unwrap `duration_since(UNIX_EPOCH)` on systems whose clock may be before the epoch. Treat `Instant` subtraction as saturating, not a bug.

```rust
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        // Work...
        thread::sleep(Duration::from_millis(10));

        let elapsed = start.elapsed();
        if elapsed >= timeout {
            println!("timed out");
            break;
        }

        // Safe arithmetic for large/untrusted offsets.
        if let Some(deadline) = start.checked_add(timeout) {
            if Instant::now() >= deadline {
                println!("deadline reached");
            }
        }
    }

    Ok(())
}
```

## Review checklist

- [ ] All filesystem paths are `Path`/`PathBuf`, not `String`/`&str`.
- [ ] `std::env::var`/`var_os` results are handled; missing variables are not blindly unwrapped.
- [ ] Process exit status is checked with `success()` before captured output is used.
- [ ] `Child` processes are reaped with `wait`, `try_wait`, or `wait_with_output`.
- [ ] `BufWriter` is explicitly flushed or consumed with `into_inner()` before relying on file contents.
- [ ] I/O that reads or writes more than a few bytes uses `BufReader`/`BufWriter`.
- [ ] `stdin`/`stdout` are locked when performing many small operations.
- [ ] Elapsed time and timeouts use `Instant`; `SystemTime` is used only for wall-clock timestamps.
- [ ] Checked/saturating/try variants of `Duration`/`Instant` arithmetic are used when values may be large or untrusted.
- [ ] `io::Error` is propagated with `?` or enriched with context.
- [ ] `ErrorKind` matches include a `_ =>` wildcard arm because `ErrorKind` is `#[non_exhaustive]`.
- [ ] Async code uses `tokio::fs`/`tokio::process`/`tokio::time` instead of blocking std APIs.

## Implementation checklist

- [ ] Determine whether the code is synchronous or async; route async filesystem/process/time work to Tokio.
- [ ] Choose the right fs helper: whole-file helper for small files, `File`/`OpenOptions` for streaming or mode control.
- [ ] Build paths with `join`/`push`; avoid string concatenation and hardcoded separators.
- [ ] Decide the policy for missing environment variables: error, default, or `Option`.
- [ ] For subprocesses, capture output, check `ExitStatus::success()`, and reap the child to avoid zombies.
- [ ] Buffer reads/writes when looping or handling large data; flush writers explicitly.
- [ ] Use `Instant` for timing and `Duration` for timeouts; use checked arithmetic for robustness.
- [ ] Use `try_exists` instead of `exists` when permission errors must be distinguished.
- [ ] Use `create_new` (1.77) for atomic "create-if-missing" file creation.
- [ ] Use platform-specific symlink/permission APIs behind `cfg` when portability requires it.

## Validation hooks

After implementing code that uses these APIs, run the following checks:

- `cargo check` to catch type and ownership errors early.
- `cargo clippy` with the project's normal feature set to catch common footguns such as unbuffered writes and fallible path string conversions.
- `cargo test` covering both success and failure paths for filesystem, process, and I/O code.
- For process tests, include a case where the child returns a non-zero exit and verify the parent detects it.
- For path tests, exercise the primary target platform and, if cross-platform, add Windows path-prefix tests.
- For time tests, use `Instant` deltas with generous margins; never assert exact wall-clock timing. For deterministic async timing, use `tokio::time` with `test-util` (see [`docs/rust/async-tokio.md`](./async-tokio.md)).
- Run tests in CI on the supported target matrix; filesystem semantics (case sensitivity, permissions, symlink support) differ materially between Unix and Windows.
- For integration tests that touch the filesystem, use a temporary directory isolated under `target/` or the `tempfile` crate, and clean up on success and failure.
- For code that uses unsafe extension traits (`pre_exec`, raw handle/fd conversions), add extra review and targeted tests on each supported platform.
- See [`docs/rust/testing.md`](./testing.md) for general Rust testing patterns.

## Examples

### 1. Read a configuration file with a typed error and `NotFound` handling

```rust,no_run
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
enum ConfigError {
    Missing(io::Error),
    InvalidUtf8(io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing(e) => write!(f, "config not found: {}", e),
            ConfigError::InvalidUtf8(e) => write!(f, "config not valid UTF-8: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

fn load_config(path: PathBuf) -> Result<String, ConfigError> {
    match fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(ConfigError::Missing(e)),
        Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(ConfigError::InvalidUtf8(e)),
        Err(e) => Err(ConfigError::Missing(e)),
    }
}
```

### 2. Atomically write a file using a temporary file and rename

```rust,no_run
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    let tmp = path.with_extension("tmp");
    {
        let file = fs::File::create(&tmp)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(content)?;
        writer.flush()?;
        // into_inner flushes and returns the file; drop would ignore errors.
        let _ = writer.into_inner()?;
    }
    // Rename is atomic on the same filesystem.
    fs::rename(&tmp, path)?;
    Ok(())
}
```

### 3. Walk a directory and process sorted entries

```rust,no_run
use std::fs;
use std::io;
use std::path::Path;

fn list_text_files(dir: &Path) -> Result<Vec<String>, io::Error> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                == Some("txt")
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    // read_dir order is not guaranteed; sort explicitly.
    entries.sort();
    Ok(entries)
}
```

### 4. Spawn a subprocess, feed stdin from a thread, capture stdout, and reap

```rust,no_run
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;

fn run_sort(input: &[&str]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut child = Command::new("sort")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = input.join("\n") + "\n";
        thread::spawn(move || stdin.write_all(payload.as_bytes()));
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "sort failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )).into());
    }

    Ok(BufReader::new(&output.stdout[..])
        .lines()
        .map(|l| l.unwrap_or_default())
        .collect())
}
```

### 5. Buffered file copy with explicit flush

```rust,no_run
use std::fs::File;
use std::io::{self, BufReader, BufWriter, copy};

fn buffered_copy(src: &str, dst: &str) -> io::Result<u64> {
    let mut reader = BufReader::new(File::open(src)?);
    let mut writer = BufWriter::new(File::create(dst)?);
    let n = copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(n)
}
```

### 6. Monotonic timeout loop with checked arithmetic

```rust,no_run
use std::thread;
use std::time::{Duration, Instant};

fn poll_until_ready<F>(mut predicate: F, timeout: Duration) -> bool
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    loop {
        if predicate() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));

        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return false;
        }

        // Defensive: detect impossible clock ordering with checked arithmetic.
        if let Some(deadline) = start.checked_add(timeout) {
            if Instant::now() >= deadline {
                return false;
            }
        }
    }
}
```

### 7. Portable symlink creation gated by `cfg`

```rust,no_run
use std::io;
use std::path::Path;

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        std::os::windows::fs::symlink_dir(src, dst)
    } else {
        std::os::windows::fs::symlink_file(src, dst)
    }
}
```

### 8. Resolve a path inside a sandbox root and reject traversal

```rust,no_run
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_in_root(root: &Path, user_path: &str) -> Option<PathBuf> {
    let candidate = root.join(user_path);
    let canonical_root = fs::canonicalize(root).ok()?;
    let canonical_candidate = fs::canonicalize(&candidate).ok()?;
    if canonical_candidate.starts_with(&canonical_root) {
        Some(canonical_candidate)
    } else {
        None
    }
}
```

### 9. Environment variable with a typed default and sanitized child environment

```rust,no_run
use std::process::Command;

fn run_with_limited_env() -> Result<std::process::Output, std::io::Error> {
    Command::new("/usr/bin/git")
        .arg("status")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/safe/home")
        .current_dir("/safe/repo")
        .output()
}
```

## Common mistakes

- **Using `String` for paths.** Always use `PathBuf` for owned paths and `&Path` for borrowed paths. String paths break on Windows and on any path containing invalid UTF-8.
- **Assuming `File::open` can write.** [`File::open`](https://doc.rust-lang.org/std/fs/struct.File.html#method.open) is read-only. Use `File::create` for writing or `OpenOptions` for read/write.
- **Ignoring process exit codes.** `Command::output` returning `Ok` only means the spawn succeeded. Always check `ExitStatus::success()` before interpreting stdout.
- **Forgetting to reap `Child`.** [`Child`](https://doc.rust-lang.org/std/process/struct.Child.html) has no `Drop`; dropping it leaks a zombie process. Always call `wait`, `try_wait`, or `wait_with_output`.
- **Unbuffered I/O in loops.** Reading or writing one byte or one line at a time without `BufReader`/`BufWriter` is slow and noisy.
- **Dropping `BufWriter` without flushing.** Drop ignores flush errors, causing silent data loss. Call `flush()` or `into_inner()`.
- **Using `write` and assuming it wrote everything.** `Write::write` may return a partial count; use `write_all` for all-or-nothing writes.
- **Using `set_readonly(false)` on Unix.** It makes the file world-writable. Use `PermissionsExt::set_mode` instead.
- **Using `SystemTime` for benchmarking.** Wall clocks can jump backwards; use `Instant` for duration measurements.
- **Panicking `Duration`/`Instant` arithmetic.** `Duration::new` panics on carry overflow; `from_secs_f64` panics on NaN/infinity; `Instant + Duration` can panic on out-of-range. Use checked/try variants in production.
- **Treating `Cursor::new(vec)` as append mode.** It overwrites from position 0. Set the position or use `Vec::extend` for appending.
- **Assuming `read_dir` order is stable.** Order is unspecified; sort if it matters.
- **Calling `rename` across filesystems.** It fails with `CrossesDevices` (`EXDEV`). Use copy+delete for cross-FS moves.
- **Using `Command::new` with shell syntax.** `Command` does not interpret pipes, globs, `$VAR`, or quotes. Build args separately; use an explicit shell only when necessary and never with untrusted input.
- **`.bat` / `.cmd` injection on Windows.** These files run through `cmd /C` and use non-standard argument parsing; restrict untrusted args.
- **Pipe-buffer deadlock.** Writing to a piped stdin while reading piped stdout without threading or closing stdin first can deadlock.
- **Exhaustively matching `ErrorKind`.** It is `#[non_exhaustive]`; always include a `_ =>` arm.
- **Not locking `stdin`/`stdout` for many ops.** Repeated small reads/writes lock on every call; lock once with `stdin().lock()` or `stdout().lock()`.
- **Using `Path::exists` when errors matter.** `exists` coerces permission errors to `false`. Use `try_exists` to propagate errors.
- **Manual `\n` splitting instead of `BufRead::lines`.** Use the provided line iterator to handle platform newlines and CRLF stripping.
- **Confusing `read` (may be short) with `read_exact` (errors on short read).** Choose based on whether partial reads are acceptable.
- **Using `Instant - Instant` to detect ordering.** It saturates to zero; use `checked_duration_since` to detect backward clock bugs.
- **Passing untrusted input to `current_dir`.** `Command::current_dir` makes a relative program path ambiguous; canonicalize the program path before setting a working directory.
- **Trusting `argv[0]` or `current_exe`.** Both are attacker-controlled in adversarial contexts; do not use them for authorization.

## Strict vs contextual guidance

### Strict guidance

- Use `Path`/`PathBuf` for every filesystem path.
- Propagate `io::Error` with `?` rather than silently swallowing it.
- Check process exit status before using captured output.
- Reap every `Child` process to avoid zombies.
- Use `Instant` for elapsed-time and timeout logic.
- Buffer looped I/O with `BufReader`/`BufWriter`.
- Flush or `into_inner()` a `BufWriter` before relying on its output.
- Always include a `_ =>` wildcard arm when matching `io::ErrorKind`.
- Use `write_all` instead of assuming `write` wrote the full buffer.
- Use `Command::env_clear`/`env` rather than `set_var` to control a child's environment.

### Contextual guidance

- **Buffered vs unbuffered I/O.** For a single small read or write, `fs::read_to_string` or `fs::write` is fine. For loops or large data, buffer explicitly.
- **Sync vs async.** Use `std` blocking APIs in synchronous code. In `tokio` runtimes, switch to `tokio::fs`, `tokio::process`, and `tokio::io` traits to avoid blocking executor threads (see [`docs/rust/async-tokio.md`](./async-tokio.md)).
- **Environment variable defaults.** Returning an error is strict; supplying a sensible default or `Option` may be acceptable depending on the application.
- **Canonicalize.** Use `std::fs::canonicalize` only when an absolute, symlink-resolved path is required. It requires the path to exist and produces verbatim `\\?\` paths on Windows.
- **Atomic file creation.** Use `File::create_new` (1.77) for "create-if-missing" when avoiding TOCTOU is important; otherwise `File::create` is acceptable.
- **Permissions.** Use the portable `readonly` API for simple checks, but switch to platform-specific `PermissionsExt` when exact Unix modes are required.
- **Symlinks.** Gate symlink creation behind `cfg(unix)`/`cfg(windows)` using the platform-specific extension functions.
- **Error crates.** Use `anyhow`/`thiserror`/`miette` per repo policy; see [`docs/rust/error-handling.md`](./error-handling.md).
- **Path sandboxing.** Canonicalizing and checking `starts_with` is appropriate when user-supplied paths must stay inside a root directory, but consider race conditions and symlink behavior carefully.

## Policy decisions for individual repos

Each repository should document its choices for the following:

- **Async runtime.** Is the codebase synchronous, `tokio`, or another runtime? Process and filesystem code must follow that decision and avoid blocking executor threads.
- **Error handling library.** Decide whether to use plain `io::Error`, `thiserror`, `anyhow`, `miette`, or another crate, and apply it consistently. See [`docs/rust/error-handling.md`](./error-handling.md).
- **Path validation.** Are relative paths allowed? Must paths be inside a specific root directory? Must symlinks be resolved before access? Is path traversal outside a sandbox forbidden?
- **Subprocess policy.** Which external binaries may be spawned? Should `PATH` be sanitized? Should shell execution ever be used? Are `.bat`/`.cmd` files forbidden for untrusted args?
- **Temporary files.** Use `std::env::temp_dir`, the `tempfile` crate, or a project-specific scratch directory?
- **Test isolation.** Filesystem and process tests should avoid mutating the developer's home directory or real system state. Use isolated temp directories and clean up in `Drop` guards or `tempfile`.
- **MSRV.** Some APIs in this document require Rust 1.63+ (`try_exists`), 1.77+ (`create_new`), 1.80+ (`seek_relative`), or newer. Pin the MSRV and avoid newer APIs if stability demands it.
- **Unsafe extension traits.** Code using `pre_exec`, raw fd/handle conversions, or `set_var`/`remove_var` must be explicitly reviewed and documented.

## Related docs

- [`docs/rust/async-tokio.md`](./async-tokio.md) — async alternatives: `tokio::fs`, `tokio::process::Command`, `tokio::time`, `tokio::io`, and runtime/threading rules. Relevant sections: [`tokio::time` line 405](./async-tokio.md) and [`tokio::process` line 423](./async-tokio.md).
- [`docs/rust/error-handling.md`](./error-handling.md) — `Result`/`?` patterns, panic vs recoverable errors, and `anyhow`/`thiserror`/`miette` usage.
- [`docs/rust/testing.md`](./testing.md) — Rust testing patterns, temp-file isolation, and deterministic time tests.
- Standard library reference: [`std::env`](https://doc.rust-lang.org/std/env/), [`std::path`](https://doc.rust-lang.org/std/path/), [`std::fs`](https://doc.rust-lang.org/std/fs/), [`std::io`](https://doc.rust-lang.org/std/io/), [`std::process`](https://doc.rust-lang.org/std/process/), [`std::time`](https://doc.rust-lang.org/std/time/).

## Related skills

- `nix-usage` — build and toolchain context for this repository.
- There are no std-runtime-specific skills in the current registry. Consider creating one if a project accumulates repeated rules around sandboxed subprocesses, path canonicalization, or deterministic time tests.
