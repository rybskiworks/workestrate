# Getting Started with Verus

## Purpose

Install Verus, run the verifier on a first program, understand the two-phase verify/compile workflow, and set up the surrounding tooling (`cargo-verus`, `verusdoc`, IDE support, `verusfmt`). This is the entry point for the corpus; for the verification model behind the tooling, see [verification-model.md](./verification-model.md), and for the specification language, see [specifications.md](./specifications.md).

> **Source fidelity:** Every code snippet and command below is copied verbatim from the Verus clone at `.tmp/verus/` (tutorial chapters under `source/docs/guide/src/`, top-level `INSTALL.md`/`BUILD.md`, and example files under `examples/guide/`). Anchor names reference the `// ANCHOR:` markers in those example files.

---

## What Verus is (one paragraph)

Verus is a tool for verifying the correctness of code written in Rust. Developers write specifications of what their code should do, and Verus statically checks that the executable Rust code will always satisfy the specifications for all possible executions of the code. Rather than adding run-time checks, Verus relies on automated theorem proving (an SMT solver, Z3) to discharge the generated verification conditions. Verification is static: Verus adds no run-time checks. See [index.md](./index.md) for the full pipeline and trusted computing base.

---

## Install

Verus provides first-tier support for relatively recent OS distributions (MacOS, Windows, and Ubuntu). The general rule of thumb is that Verus lags one version behind the latest offered on GitHub. Prebuilt release artifacts exist for:

- MacOS 14 (arm)
- MacOS 15 (x86_64)
- Windows 2022 (x86_64)
- Ubuntu 24.04 (x86_64)

Otherwise, build `verus` from source (see [From source](#from-source) below).

### Binary release (recommended for most users)

These instructions are for installing Verus using the binary releases, intended for most people who plan to use Verus from the command line. If you would rather use the VSCode IDE, see [IDE support](#ide-support) below.

1. **Download the binary release.** Select from either:
   - **(Recommended)** The weekly point releases, available at [https://github.com/verus-lang/verus/releases/latest](https://github.com/verus-lang/verus/releases/latest).
   - The rolling release, which tracks the latest commit on `main`. This can be found on the [release page](https://github.com/verus-lang/verus/releases), marked "pre-release".

   After selecting the desired release, open the "Assets" drawer from the releases page and download the file appropriate to your platform.

2. **Unzip the file** and navigate into the resulting directory. In bash, for example:

   ```bash
   unzip verus-0.2025.06.24.77d5bbe-x86-macos.zip
   mv verus-x86-macos verus
   cd verus
   ```

   The remainder of these instructions assume this is your working directory.

3. **(MacOS only) Remove Gatekeeper quarantine.** If you are on MacOS, the binaries and libraries will be quarantined by Gatekeeper. A script is provided to fix this automatically:

   ```bash
   bash macos_allow_gatekeeper.sh
   ```

4. **Install the correct Rust toolchain.** To check if this step is necessary, run `./verus`. It may print an error telling you to install `rustup`, or to install the necessary toolchain. Follow the instructions it prints. For example, if `rustup` is not available, you will see something like:

   ```bash
   $ ./verus
   verus: rustup not found, or not executable
   verus needs a rustup installation
   run the following command (in a bash-compatible shell) to install rustup:
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.86.0-x86_64-unknown-linux-gnu
   or visit https://rustup.rs/ for more information
   ```

   If `rustup` is installed but the necessary toolchain is not, `./verus` will instead print:

   ```bash
   $ ./verus
   verus: required rust toolchain 1.86.0-x86_64-unknown-linux-gnu not found
   run the following command (in a bash-compatible shell) to install the necessary toolchain:
     rustup install 1.86.0-x86_64-unknown-linux-gnu
   error: toolchain '1.86.0-x86_64-unknown-linux-gnu' is not installed
   ```

   Follow the instructions to install the necessary toolchain, then run `verus` again.

> **Toolchain pin (from source):** The repository's `rust-toolchain.toml` pins the toolchain to `1.96.0` with components `rustc`, `rust-std`, `cargo`, `rustfmt`, `rustc-dev`, and `llvm-tools`. The binary-release path references an older `1.86.0` toolchain for end-users; building from source uses the `rust-toolchain.toml` pin of `1.96.0`. See [index.md](./index.md) for the full version-pin table.

### From source

The main project source is in `source/`. `tools` contains scripts for setting up the development environment by building a `cargo` wrapper (`vargo`) that ensures artifacts are built correctly with the custom build process.

**Step 1: Setup Z3.** Change directory to `source` (`cd source`).

- On Windows: download the [Z3 binaries](https://github.com/Z3Prover/z3/releases). Make sure you get Z3 4.12.5. Set the `VERUS_Z3_PATH` environment variable to the path of the Z3 executable file.
- On Unix/macOS/Windows: from `source`, use the script `./tools/get-z3.sh` (on Unix/macOS) or `./tools/get-z3.ps1` (on Windows) to download Z3. On Unix/macOS the cargo wrapper will correctly set the `VERUS_Z3_PATH` environment variable for the verifier to find Z3. If you run the verifier binary manually, set `VERUS_Z3_PATH` to `source/z3` or `source/z3.exe`.

> **Z3 version:** Z3 4.12.5 is the pinned version (from `BUILD.md`: "Make sure you get Z3 4.12.5").

**Step 2: Ensure you have a recent rustup installed.** Obtain rustup from [https://rustup.rs](https://rustup.rs) if you do not have it.

**Step 3: Build Verus.** You should be in the `source` subdirectory. First, activate the development environment:

```bash
source ../tools/activate       # for bash and zsh
source ../tools/activate.fish  # for fish
..\tools\activate.bat          # for Windows
..\tools\activate.ps1          # for Windows (Power Shell)
```

If you do not have the necessary rust toolchain installed, you will get a message like:

```bash
error: toolchain '1.82.0-aarch64-unknown-linux-gnu' is not installed
help: run `rustup toolchain install 1.82.0-aarch64-unknown-linux-gnu` to install it
```

Do not run the command as indicated. Run `rustup toolchain install` so that rustup installs the toolchain according to the requirements of the project (specified in `rust-toolchain.toml` at the project root).

This command builds (or re-builds) `vargo`, the cargo wrapper, and adds it to the `PATH` for the current shell. Now, simply run:

```bash
vargo build --release
```

(Omit `--release` for a debug build.) This builds everything you need to use Verus:

- The `rust_verify` binary, which verifies Verus code.
- Additional libraries that Verus code will need to include (`builtin`, `builtin_macros`, and `state_machines_macros`).
- The [Verus standard library, `vstd`](https://verus-lang.github.io/verus/verusdoc/vstd/), which is written in Verus. The build system builds **and verifies** the `vstd` crate.

If everything is successful, you should see output indicating that various modules in `vstd` are being verified.

---

## First run

### Verify a sample program

Create a file called `getting_started.rs`, and paste in the following contents (this is the verbatim file `examples/guide/getting_started.rs`):

```rust
use vstd::prelude::*;

verus! {

spec fn min(x: int, y: int) -> int {
    if x <= y {
        x
    } else {
        y
    }
}

fn main() {
    assert(min(10, 20) == 10);
    assert(min(-10, -20) == -20);
    assert(forall|i: int, j: int| min(i, j) <= i && min(i, j) <= j);
    assert(forall|i: int, j: int| min(i, j) == i || min(i, j) == j);
    assert(forall|i: int, j: int| min(i, j) == min(j, i));
}

} // verus!
```

To run Verus on the file, on macOS, Linux, or similar systems, run:

```bash
/path/to/verus getting_started.rs
```

On Windows, run:

```bash
.\path\to\verus.exe getting_started.rs
```

You should see the following output:

```
note: verifying root module

verification results:: 1 verified, 0 errors
```

This indicates that Verus successfully verified 1 function (the `main` function). (When verifying a larger file, the line takes the form `verification results:: verified: N errors: 0`, e.g. `verification results:: verified: 7 errors: 0` for the `examples/vectors.rs` example.)

### Try it on code that will not verify

If you want, you can try editing the `getting_started.rs` file to see a verification failure. For example, if you add the following line to `main`:

```rust
    assert(forall|i: int, j: int| min(i, j) == min(i, i));
```

you will see an error message:

```
note: verifying root module

error: assertion failed
  --> getting_started.rs:19:12
   |
19 |     assert(forall|i: int, j: int| min(i, j) == min(i, i));
   |            ^^^^^^ assertion failed

error: aborting due to previous error

verification results:: 0 verified, 1 errors
```

### Compile the program

The command above only verifies the code, but does not compile it. If you also want to compile it to a binary, run `verus` with the `--compile` flag:

```bash
/path/to/verus getting_started.rs --compile
```

Either will create a binary `getting_started`. However, in this example, the binary will not do anything interesting because the `main` function contains no executable code — it contains only statically-checked assertions, which are erased before compilation.

To verify an entire crate, point Verus at your `src/main.rs` file for an executable project, or `src/lib.rs` for a library project. You will need to add `--crate-type=lib` for the latter.

---

## The `verus!` macro

Each file in a crate will typically take the following form:

```rust
use vstd::prelude::*;

verus! {
    // ...
}
```

The `vstd::prelude` exports the `verus!` macro along with some other Verus utilities. The `verus!` macro extends Rust's syntax with verification-related features such as preconditions, postconditions, assertions, `forall`, `exists`, etc. Besides extending Rust's syntax, the `verus!` macro also *tells Verus to verify the functions contained within*. By default, Verus verifies everything inside the `verus!` macro and ignores anything defined outside the `verus!` macro.

> See [verification-model.md](./verification-model.md) for the three modes (`spec`/`proof`/`exec`), `requires`/`ensures`, ghost code, and erasure. See [specifications.md](./specifications.md) for the specification language (`forall`, `exists`, `int`/`nat`, operators, triggers).

---

## `cargo verus` — Cargo integration

For projects with multiple crates, Verus provides a `cargo verus` subcommand that integrates verification into the normal Rust Cargo workflow. The `cargo-verus` binary must be in your `PATH`; it is included in the official [Verus release packages](https://github.com/verus-lang/verus/releases) alongside the `verus` binary. Check that it is available by running:

```bash
cargo verus --help
```

### Starting a new project

The fastest way to create a new Verus project is:

```bash
# Binary project
cargo verus new --bin my_project

# Library project
cargo verus new --lib my_library
```

This creates a new directory with a correctly-configured `Cargo.toml`, an initial `src/main.rs` (or `src/lib.rs`), and a `.gitignore`. The generated project already has `vstd` as a dependency and the `[package.metadata.verus]` section described below.

### Cargo.toml configuration

Crates that should be verified must opt in by adding a `[package.metadata.verus]` section to their `Cargo.toml`:

```toml
[package.metadata.verus]
verify = true
```

Since Verus enables a `verus_only` cfg flag during verification (so that `use` statements for ghost items can be guarded with `#[cfg(verus_only)]`), and this flag is not declared in `Cargo.toml`, Rust 1.80+ will emit a warning unless you suppress it:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(verus_only)'] }
```

`cargo verus new` adds this automatically. See [Ghost erasure](#ghost-erasure) below for a full explanation of `verus_only` and when to use it.

### Subcommands

| Subcommand | Effect |
|---|---|
| `cargo verus verify` | Runs verification on all crates that have `verify = true`, including dependencies. Does **not** produce a compiled binary. Use for day-to-day proof development. Incremental builds mean only changed crates are re-verified. |
| `cargo verus focus` | Like `verify`, but skips re-verification of dependencies. Useful when iterating on root crates whose dependencies have not changed. Stores artifacts under `target/verus-partial`. |
| `cargo verus build` | Verifies all opted-in crates **and** compiles them to native artifacts. Use when you want to ship a verified binary. |

```bash
cargo verus verify              # Verify all of the crates
cargo verus verify -p my_crate  # Verify only my_crate and its dependencies
cargo verus focus               # Verify all of the root crates, but not their dependencies
cargo verus build --release     # Verify and compile a release binary
```

### Passing arguments

Arguments are split around `--`: before `--` is forwarded to `cargo`; after `--` is forwarded to every `verus` invocation.

```bash
cargo verus verify -p my_crate --release -- --rlimit 60 --expand-errors
#                  ^^^^^^^^^^^^^^^^^^^^^    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                  Cargo arguments            Verus arguments
```

Common Verus arguments that can be passed this way include `--rlimit`, `--expand-errors`, and `--log-all`. By default, the Verus arguments after `--` are forwarded to **all** verified crates; use `--fwd-verus-args-to <target>` (`all`/`roots`/`deps`) to narrow or widen the target.

---

## Ghost erasure (the two-phase verify/compile)

Verus performs ghost erasure: ghost code that exists for verification purposes is removed when building the executable artifacts, ensuring they are minimally disturbed. This is the two-phase workflow: phase 1 verifies (the `verus!` macro rewrites spec/proof syntax and the verifier discharges VCs with Z3); phase 2 compiles (a second `rustc` pass erases ghost code and produces a normal executable via MIR → LLVM → linking).

One byproduct of erasure is that certain identifiers that exist at verification time will not exist at compile time. The code below would fail to *compile*:

```rust
use vstd::prelude::*;

verus! {

pub mod ghost_mod {
    pub closed spec fn ghost_fn() -> bool { true }
}

pub mod test_mod {
    use crate::ghost_mod::ghost_fn;
//  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//  FAILS: During compilation, ghost_fn is erased, causing rustc
//         to complain about a missing definition

    pub fn exec_fn() -> u64 {
        1
    }
}
```

To remedy this, Verus provides the `verus_only` flag, which is turned on during verification, but is otherwise off. This allows us to guard `use` statements like the one above:

```rust
pub mod test_mod {
    #[cfg(verus_only)]
    use crate::ghost_mod::ghost_fn;
//  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//  OK: During compilation, the `use` statement is removed
}
```

Another use-case for the `verus_only` flag is setting attributes that only make sense during verification:

```rust
#![cfg_attr(verus_only, verus::loop_isolation(false))]
```

> **CAUTION:** This should only be used to guard `use` statements and setting config attributes. Using this feature flag for conditional compilation of code can introduce **unsoundness**, breaking the verification guarantees. As an example, the code below will verify successfully, even though `f()` returns `42` when running the executable:
>
> ```rust
> fn f() -> (u: u32)
>   ensures u != 0
> {
>   #[cfg(verus_only)]
>   { return 0; }
>   #[cfg(not(verus_only))]
>   { return 42; }
> }
> ```

---

## Documentation with `verusdoc`

Verus provides a tool to help make Verus specifications look nice in rustdoc. To do this, you first run `rustdoc` on a crate and then run an HTML postprocessor called Verusdoc.

First, make sure `verusdoc` is built by running `vargo build -p verusdoc` in the `verus/source` directory. Then run `rustdoc` with the appropriate dependencies and flags (note the `VERUSDOC` environment variable), followed by the post-processor:

```bash
VERUS=/path/to/verus/source

if [ `uname` == "Darwin" ]; then
    DYN_LIB_EXT=dylib
elif [ `uname` == "Linux" ]; then
    DYN_LIB_EXT=so
fi

# Run rustdoc.
# Note the VERUSDOC environment variable.

RUSTC_BOOTSTRAP=1 VERUSDOC=1 rustdoc \
  --extern builtin=$VERUS/target-verus/debug/libbuiltin.rlib \
  --extern builtin_macros=$VERUS/target-verus/debug/libbuiltin_macros.$DYN_LIB_EXT \
  --extern state_machines_macros=$VERUS/target-verus/debug/libstate_machines_macros.$DYN_LIB_EXT \
  --extern vstd=$VERUS/target-verus/debug/libvstd.rlib \
  --edition=2021 \
  --cfg verus_keep_ghost \
  --cfg verus_keep_ghost_body \
  --cfg 'feature="std"' \
  --cfg 'feature="alloc"' \
  '-Zcrate-attr=feature(register_tool)' \
  '-Zcrate-attr=register_tool(verus)' \
  '-Zcrate-attr=register_tool(verifier)' \
  '-Zcrate-attr=register_tool(verusfmt)' \
  --crate-type=lib \
  ./lib.rs

# Run the post-processor.

$VERUS/target/debug/verusdoc
```

If you run it with a file `lib.rs` like this:

```rust
#![allow(unused_imports)]

use builtin::*;
use builtin_macros::*;
use vstd::prelude::*;

verus!{

/// Computes the max
pub fn compute_max(x: u32, y: u32) -> (max: u32)
    ensures max == (if x > y { x } else { y }),
{
    if x < y {
        y
    } else {
        x
    }
}

}
```

it will generate rustdoc that renders the `ensures` clause alongside the function signature.

---

## IDE support

Verus currently has IDE support for VS Code and Emacs.

### VS Code (`verus-analyzer`)

For VS Code, Verus requires `verus-analyzer`, a Verus-specific fork of `rust-analyzer`. Note that `verus-analyzer` is **very experimental**. To set it up:

1. Create a Rust crate: `cargo init verus_test`.
2. Install `verus-analyzer` via the VS Code marketplace.
3. Open the `verus_test` directory in VS Code.
4. Disable `rust-analyzer` (it is redundant and will produce unwanted errors): go to the extensions panel, find rust-analyzer, click the gear icon, and select **"Disable (Workspace)"**, then click **"Restart Extensions"**.
5. Paste the sample program into `src/main.rs` and save to trigger `verus-analyzer`.

### Emacs (`verus-mode`)

For Emacs, Verus is supported through [verus-mode.el](https://github.com/verus-lang/verus-mode.el), a major mode that supports syntax highlighting, verification-on-save, jump-to-definition, and more. To use `verus-mode`, configure `.emacs` to set `verus-home` to the path to Verus, then load `verus-mode`. For example, with `use-package`:

```bash
(use-package verus-mode
  :init
  (setq verus-home "PATH_TO_VERUS_DIR"))   ; Path to where you've cloned https://github.com/verus-lang/verus
```

### Formatter (`verusfmt`)

Verus supports an auto-formatter, [verusfmt](https://github.com/verus-lang/verusfmt), for `verus! { }` code. It is the Verus analogue of `rustfmt`.

---

## The playground

If you do not want to install Verus yet, but just want to experiment with it or follow along the tutorial, you can run Verus through [the Verus playground](https://play.verus-lang.org/) in your browser.

---

## Installing Singular (optional)

Singular must be installed in order to use the `integer_ring` solver mode. Install Singular version 4.3.2 (other versions are untested, and 4.4.0 is known to be incompatible with Verus). For example:

- Mac: `brew install Singular` and set `VERUS_SINGULAR_PATH` (e.g. `VERUS_SINGULAR_PATH=/usr/local/bin/Singular`).
- Debian-based Linux: `apt-get install singular` and set `VERUS_SINGULAR_PATH` (e.g. `VERUS_SINGULAR_PATH=/usr/bin/Singular`).

The `integer_ring` functionality is conditionally compiled when the `singular` feature is set. To add this feature, add the `--features singular` flag when you invoke `vargo build` to compile Verus. See [arithmetic-and-provers.md](./arithmetic-and-provers.md) for the prover modes.

---

## Sources used

Tutorial chapters read (`.tmp/verus/source/docs/guide/src/`):

- `getting_started.md` — getting started chapter intro (command line vs VSCode vs playground)
- `getting_started_cmd_line.md` — install + verify sample program + `--compile` (verbatim commands and output)
- `getting_started_vscode.md` — VSCode setup with `verus-analyzer`
- `verus_macro_intro.md` — the `verus!` macro, `use vstd::prelude::*`
- `ide_support.md` — VS Code (`verus-analyzer`) and Emacs (`verus-mode`) support
- `install-singular.md` — installing Singular 4.3.2 for `integer_ring`
- `projects.md` — project setup chapter intro
- `cargo_verus.md` — `cargo verus` subcommands, `Cargo.toml` config, argument passing, multi-crate workspaces, incremental verification
- `verusdoc.md` — rustdoc post-processor for Verus specifications
- `erasure.md` — ghost erasure, `verus_only` cfg flag, `Cargo.toml` lint config
- `overview.md` — Verus overview (what Verus is, SMT/Z3, goals)

Top-level files read (`.tmp/verus/`):

- `INSTALL.md` — binary release install, platform support, toolchain bootstrap
- `BUILD.md` — building from source, Z3 setup (`get-z3.sh`, `VERUS_Z3_PATH`), `vargo build`, running the verifier, `--compile`
- `README.md` — project overview, playground link, documentation links, `verusfmt`
- `rust-toolchain.toml` — toolchain pin: `1.96.0`, components

Example files read (verbatim code snippets sourced from):

- `examples/guide/getting_started.rs` — full file (the `min` spec fn + `main` with `assert`/`forall`)

Upstream URLs:

- https://github.com/verus-lang/verus/blob/main/INSTALL.md — binary release install instructions
- https://github.com/verus-lang/verus/blob/main/BUILD.md — build-from-source instructions
- https://github.com/verus-lang/verus/releases — Verus releases (weekly point + rolling pre-release)
- https://github.com/verus-lang/verus/blob/main/rust-toolchain.toml — toolchain pin
- https://verus-lang.github.io/verus/guide/getting_started.html — rendered getting-started guide
- https://verus-lang.github.io/verus/guide/ide_support.html — rendered IDE support guide
- https://verus-lang.github.io/verus/guide/cargo_verus.html — rendered cargo-verus guide
- https://verus-lang.github.io/verus/guide/erasure.html — rendered ghost erasure guide
- https://play.verus-lang.org/ — Verus playground (in-browser)
- https://github.com/verus-lang/verus-analyzer — `verus-analyzer` (VS Code, experimental)
- https://github.com/verus-lang/verus-mode.el — `verus-mode` (Emacs)
- https://github.com/verus-lang/verusfmt — `verusfmt` auto-formatter
- https://github.com/Z3Prover/z3/releases — Z3 releases (pinned 4.12.5)
- https://rustup.rs — rustup installer
