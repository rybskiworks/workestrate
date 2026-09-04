---
type: Reference
resource: https://nix.dev/guides/troubleshooting.html
title: Error Handling and Debugging
description: Nix error handling — common eval errors, build failure diagnosis, debugging derivations, build log inspection, nix repl, builtins.trace, lib.debug tracing, and project-specific failure modes and guards.
tags: [nix, errors, debugging, build-failures, eval-errors]
timestamp: 2026-07-24T01:21:00Z
---

# Error Handling and Debugging

## Purpose

This document is the canonical corpus reference for diagnosing Nix errors —
eval errors, build failures, and debugging derivations. It covers the error
classes most commonly hit when working on the `ai-workbench` flake, the
commands for inspecting build logs and derivations, the tracing primitives
(`builtins.trace`, `builtins.tryEval`, `lib.debug.*`), and the project-specific
failure modes and guards wired into the `just` task runner.

Intended audience: agents and contributors debugging `nix build`, `nix eval`,
or devshell-entry (`just shell`) failures on this flake. Every contributor
who hits a Nix error is expected to consult this document before guessing at
a fix.

Cross-references:

- [Purity and Sandboxing](purity-and-sandboxing.md) — sandbox restrictions,
  FOD hash discipline, and the impurity errors that surface as build errors.
- [Store Hygiene and GC](store-hygiene-and-gc.md) — store-growth diagnosis
  and garbage collection.
- [Nix Commands](nix-commands.md) — `nix log`, `nix eval`, `nix flake check`.
- [Language Fundamentals](language-fundamentals.md) — `rec`, `let`, and
  undefined-variable semantics that underlie most eval errors.

## Sources used

- <https://nix.dev/guides/troubleshooting.html> — Nix troubleshooting guide
- <https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build> — `nix build`
  command (flags: `-L`, `--keep-failed`, `--debugger`)
- <https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop> —
  `nix develop` command
- <https://nix.dev/manual/nix/2.34/language/builtins> — `builtins.trace`,
  `builtins.tryEval`, `builtins.break`
- <https://nix.dev/manual/nix/2.34/language/derivations> — derivation
  semantics
- Local crawl files (under `/docs/nix/.crawl/`):
  - `31-troubleshooting.md`
  - `30-guides-faq.md`
  - `45-concepts-faq.md`
  - `62-nix-command-build.md`
  - `63-nix-command-develop.md`
  - `58-nix-language-builtins.md`
  - `59-nix-language-derivations.md`
  - `66-nixpkgs-stdenv-mkDerivation.md`
  - `10-nix-language-basics.md`
- Local: `/docs/nix-purity.md` — purity errors and HOST-GATE convention
- Local: `/.agents/skills/nix-usage/SKILL.md` — failure modes table
  (9 symptoms), anti-accumulation patterns, agent rules
- Local: `/docs/nix/nix-commands.md` — `nix log`, `nix eval`,
  `nix flake check`
- Local: `/docs/nix/nixpkgs-library.md` — `lib.debug` functions
- Local: `/docs/nix/store-hygiene-and-gc.md` — store audit
- Local: `/docs/nix/language-fundamentals.md` — `rec`, `let`, undefined
  variable

### Crawl ledger

SEED:

- <https://nix.dev/guides/troubleshooting.html>
- <https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build>
- <https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop>
- <https://nix.dev/manual/nix/2.34/language/builtins>
- <https://nix.dev/manual/nix/2.34/language/derivations>

DISCOVERED & VISITED:

- `/docs/nix/.crawl/31-troubleshooting.md` — troubleshooting guide crawl
- `/docs/nix/.crawl/30-guides-faq.md` — guides FAQ crawl
- `/docs/nix/.crawl/45-concepts-faq.md` — concepts FAQ crawl
- `/docs/nix/.crawl/62-nix-command-build.md` — `nix build` flags
- `/docs/nix/.crawl/63-nix-command-develop.md` — `nix develop` semantics
- `/docs/nix/.crawl/58-nix-language-builtins.md` — `trace`, `tryEval`,
  `break`
- `/docs/nix/.crawl/59-nix-language-derivations.md` — derivation semantics
- `/docs/nix/.crawl/66-nixpkgs-stdenv-mkDerivation.md` — `buildInputs`,
  `--keep-failed`
- `/docs/nix/.crawl/10-nix-language-basics.md` — `nix repl`, coercion
  errors
- `/docs/nix-purity.md` — purity rules, HOST-GATE convention, FOD hashes
- `/.agents/skills/nix-usage/SKILL.md` — failure modes table (9 rows)
- `/docs/nix/nix-commands.md` — `nix log`, `nix eval`, `nix flake check`
- `/docs/nix/nixpkgs-library.md` — `lib.debug.traceVal` and friends
- `/docs/nix/store-hygiene-and-gc.md` — store audit
- `/docs/nix/language-fundamentals.md` — `rec`, `let`, undefined variable

SKIPPED:

- Upstream Nix manual chapters not directly cited above (e.g. `nix3-registry`,
  `nix3-profile`) — out of scope for error diagnosis.
- `nix-collect-garbage` man page — covered via `/docs/nix/store-hygiene-and-gc.md`
  and the `just gc` wrapper; not re-crawled here.

## Core guidance

### Common eval errors

Eval errors occur during Nix expression evaluation — before any build runs.
They are typically caused by scope issues, type mismatches, or missing
attributes.

| Error message | Cause | Fix |
|---|---|---|
| `error: undefined variable 'X'` | Missing import, typo, or scope issue (referencing an attribute in a non-`rec` set). | Use `rec { }`, `let ... in`, or `with`. |
| `error: infinite recursion encountered` | Circular reference in `rec` or `let` (e.g. `rec { a = a; }`). | Break the cycle; restructure the binding. |
| `error: value is a function while a set was expected` | A function was used where an attribute set was expected — typically a missing argument (e.g. `pkgs.lib.foo` where `pkgs` is a function not yet applied). | Apply the missing argument. |
| `error: cannot coerce an integer to a string` | Type mismatch in string interpolation — interpolating a non-string (int, list, set) into `${...}`. | Use `builtins.toString` to convert. |
| `error: attribute 'X' missing` | Wrong attribute path in an attribute set (e.g. `pkgs.nonExistentPackage`). | Check the attribute path with `nix repl` or `nix eval`. |

#### `error: undefined variable 'X'`

Verbatim from crawl 10:

> `error: undefined variable 'one'` at `«string»:3:9`

Cause: a missing import, a typo, or a scope issue — most commonly
referencing another attribute in the same attribute set without `rec`.

> Without `rec`, referencing another attribute in the same set is an error:
> `two = one + 1;  # error: undefined variable 'one'` [language-basics]

Fix: wrap the set in `rec { }`, bind via `let ... in`, or bring names into
scope with `with`.

#### `error: infinite recursion encountered`

A circular reference in `rec` or `let` — e.g. `rec { a = a; }` evaluates `a`
by evaluating `a`, ad infinitum. Fix: break the cycle by restructuring the
binding so each attribute is grounded in a non-self-referential value.

#### `error: value is a function while a set was expected`

A function was used where an attribute set was expected. The classic case is
calling `pkgs.lib.foo` where `pkgs` is itself a function (e.g.
`import nixpkgs {}`) that has not yet been applied. Fix: apply the missing
argument so the function reduces to a set before attribute selection.

#### `error: cannot coerce an integer to a string`

Verbatim from crawl 10:

> `error: cannot coerce an integer to a string` at `«string»:4:2`

Cause: a type mismatch in string interpolation — interpolating a non-string
(int, list, set) into `${...}`. The canonical failing example:

> `"${x} + ${x} = ${x + x}"` where `x = 1` fails [language-basics]

Fix: convert with `builtins.toString`:

```nix
let x = 1; in "${builtins.toString x} + ${builtins.toString x} = ${builtins.toString (x + x)}"
```

#### `error: attribute 'X' missing`

A wrong attribute path in an attribute set — e.g. `pkgs.nonExistentPackage`.
Fix: inspect the attribute path with `nix repl` (see
[Interactive debugging](#interactive-debugging)) or `nix eval .#<attr>` to
confirm the path exists before selecting into it.

### Common build errors

Build errors occur during the execution of a derivation's builder — after
evaluation succeeds. They are typically caused by missing dependencies,
sandbox violations, or wrong FOD hashes.

| Error message | Cause | Fix |
|---|---|---|
| `hash mismatch in fixed-output derivation` | FOD hash is wrong. | Use `just update-hashes` to prefetch the real hash. |
| `error: builder for '/nix/store/...drv' failed with exit code N` | The build script returned non-zero. | View the log with `nix log .#<name>` or `nix log /nix/store/<hash>-<name>.drv`. |
| `error: no such file or directory: 'X'` | Missing buildInput (a tool or library not in `nativeBuildInputs` or `buildInputs`). | Add the missing dependency. |
| `error: permission denied` | Sandbox restriction (writing outside `$out`, accessing `/home`, network). | Write only to `$out`/`$TMPDIR`; no network in `buildPhase`. |
| `error: network is unreachable` | Network access attempted in `buildPhase` (forbidden in sandbox). | Fetch data via FOD before build, or commit data. |

#### `hash mismatch in fixed-output derivation`

The declared FOD hash does not match the hash Nix computed from the fetched
output. Fix: use `just update-hashes` to prefetch the real hash.

> Running `just update-hashes` prefetches the real hash and prints it; the
> operator manually inlines the `got:` value into the matching
> `nix/packages/*.nix` file [purity]

Never guess the hash manually — see [Common mistakes](#common-mistakes).

#### `error: builder for '/nix/store/...drv' failed with exit code N`

The build script returned a non-zero exit code. The fix is always: read the
log first.

```bash
nix log .#<name>
# or, for a specific derivation:
nix log /nix/store/<hash>-<name>-<version>.drv
```

#### `error: no such file or directory: 'X'`

A tool or library is missing from the build environment — it was not
declared in `nativeBuildInputs` or `buildInputs`.

> Many packages have dependencies that are not provided in the standard
> environment. It's usually sufficient to specify those dependencies in the
> `buildInputs` attribute [stdenv]

Fix: add the missing dependency to the appropriate `*BuildInputs` list.

#### `error: permission denied`

A sandbox restriction fired — the build tried to write outside `$out`,
access `/home`, or reach the network. Fix: write only to `$out`/`$TMPDIR`
and perform no network access in `buildPhase`.

#### `error: network is unreachable`

Network access was attempted in `buildPhase`, which is forbidden in the
sandbox.

> No network in `buildPhase`/`installPhase`. If a build needs data, it must
> be committed (e.g. pi's model catalogs) or fetched via a FOD before the
> build runs. [purity]

Fix: fetch the data via a fixed-output derivation before the build runs, or
commit the data into the repo.

### Viewing build logs

The first command to reach for when a build fails:

```bash
nix log .#<name>
```

> `nix log .#<name>`  # build log for a flake output [nix-commands]

For a specific derivation path:

```bash
nix log /nix/store/<hash>-<name>-<version>.drv
```

To see logs inline during a build:

```bash
nix build .#<name> -L
```

> `--print-build-logs` / `-L`  Print full build logs on standard error.
> [nix-build]

To keep the failed build directory for inspection:

```bash
nix build .#<name> --keep-failed
```

> The `--keep-failed` option for `nix-build` may also be useful to examine
> the build directory of a failed build. [stdenv]

### Interactive debugging

#### `nix develop .#<name>`

Enter a shell with the derivation's build inputs, then run phase commands
manually to reproduce the failure.

> `nix develop` starts a `bash` shell that provides an interactive build
> environment nearly identical to what Nix would use to build [nix-develop]

Inside the shell, the standard `stdenv` phase commands are available:

```bash
nix develop .#<name>
# Inside the shell:
configurePhase
buildPhase
installPhase
```

#### `nix repl`

Interactive evaluation — the fastest way to inspect attribute paths and
test expressions.

> Use `nix repl` to evaluate Nix expressions interactively (by typing them
> on the command line) [language-basics]

Useful commands inside the repl:

- `:l flake.nix` — load a flake.
- `:p expr` — force-evaluate (deep) an expression.
- `:q` — quit.

> The Nix language uses lazy evaluation, and `nix repl` by default only
> computes values when needed... try prepending `:p` to the input expression
> [language-basics]

#### `nix eval .#<attr>`

Evaluate a flake attribute from the command line:

```bash
nix eval .#<attr>
```

For machine-readable output:

```bash
nix eval --json .#<attr>
```

> `nix eval .#<attr>` — evaluate a flake attribute [nix-commands]

#### `nix flake check`

Catch common flake issues before building:

```bash
nix flake check --no-build  # validate the flake without building
```

> `nix flake check --no-build`  # validate the flake without building
> [nix-commands]

#### `nix show-derivation .#<name>`

Inspect the derivation JSON — inputs, builder, env, and outputs:

```bash
nix show-derivation .#<name>
```

#### `--debugger` flag

> `--debugger`  Start an interactive environment if evaluation fails.
> [nix-build]

When `debugger-on-trace` is set to `true` and `--debugger` is given, the
interactive debugger is started when `trace` is called (see
[Tracing with builtins and lib.debug](#tracing-with-builtins-and-libdebug)).

### Tracing with builtins and lib.debug

#### `builtins.trace`

> `trace e1 e2`  Evaluate e1 and print its abstract syntax representation
> on standard error. Then return e2. This function is useful for debugging.
> [builtins]

> If the `debugger-on-trace` option is set to `true` and the `--debugger`
> flag is given, the interactive debugger is started when `trace` is called
> [builtins]

#### `builtins.tryEval`

> `tryEval e`  Try to shallowly evaluate e. Return a set containing the
> attributes `success`... and `value` [builtins]

Caveat:

> `tryEval` only prevents errors created by `throw` or `assert` from being
> thrown. Errors `tryEval` doesn't catch are, for example, those created by
> `abort` and type errors [builtins]

#### `lib.debug.traceVal`

> Trace a value (print to stderr, return the value). Uses `builtins.trace`
> under the hood. [nixpkgs-lib]

Signature: `traceVal :: a -> a`.

#### `lib.debug.traceSeq`

> Force evaluation of the first argument before returning the second.
> [nixpkgs-lib]

Signature: `traceSeq :: a -> b -> b`.

#### `lib.debug.traceValFn`

> Trace a value after applying a function to it. [nixpkgs-lib]

Signature: `traceValFn :: (a -> b) -> a -> a`.

#### `lib.debug.traceIf`

> Trace only if the condition is true. [nixpkgs-lib]

Signature: `traceIf :: Bool -> a -> a`.

### Project failure modes table

The following table is reproduced verbatim from the nix-usage skill [usage]:

| Symptom | Cause | Fix |
|---|---|---|
| `nix: command not found` | PATH missing Nix profile | `export PATH=$HOME/.nix-profile/bin:$PATH` |
| `~/.nix-profile/bin/...` not found | Dangling profile symlink | Run the profile repair (above) |
| `Path 'X' is not tracked by Git` | Untracked source file | `git add X` |
| `linking with '.../.toolchain/...'` failed | Ran cargo outside `just shell` | Use `just shell -c cargo ...` |
| `error: 'packages.x86_64-linux.default' is not a flake output` | Used `.#default` | Use `.#workestrate`; there is no default |
| `cargo: command not found` / `gcc: command not found` | Outside dev shell | Run inside `just shell` |
| `msb: command not found` at runtime | SDK has not downloaded it yet | First use downloads it; check network or pre-stage |
| Build error mentioning `$HOME/.microsandbox/bin` | build.rs writing outside sandbox | Set `HOME=$TMPDIR` (derivation already does this) |
| Store grows ~1GB/min during edits | Impure source filter copying target/ or agents/*/build | Run `just gc` + `just store-audit`; fix the source filter per docs/nix-purity.md |

### Project debugging tools

#### `just lint-nix`

> `just lint-nix` (`scripts/check-nix-paths.sh`) — ACTIVE gate, wired into
> `just verify`. [usage]

Catches purity violations (impure path references) before they reach the
store.

#### `just store-audit`

> `just store-audit` (`scripts/store-audit.py`) — reports the top-20 store
> paths by closure size and flags any `*-source` paths referencing
> `ai-workbench` [usage]

Diagnoses store growth — the single most common operational symptom on this
flake.

#### `just gc`

> `just gc` — `nix-collect-garbage --delete-old` + `nix store optimise`
> (dedupe). [usage]

Garbage collection + deduplication. Run after `just store-audit` identifies
bloat.

### HOST-GATE note

> **HOST-GATE.** Most debugging commands (`nix build`, `nix log`, `nix repl`,
> `nix develop`, `nix eval`, `nix show-derivation`, `nix flake check`)
> require Nix installed on the host. This container has no Nix; any `nix`
> command claim here is based on documented Nix semantics, not runtime
> verification.

> Add a HOST-GATE note if the build can only be verified on a host with nix.
> This container has no nix; any `nix build` claim here is based on
> documented Nix semantics, not runtime verification. [purity]

## Practical rules

1. Read the full error message — Nix errors include file, line, column,
   and the offending expression.
2. View the build log first: `nix log .#<name>` before guessing.
3. Use `nix build -L` to see logs inline during build.
4. Use `nix develop .#<name>` to reproduce build failures interactively.
5. Use `nix repl` to evaluate expressions and inspect attribute paths.
6. Use `builtins.trace` / `lib.debug.traceVal` to instrument evaluation.
7. Stage new files (`git add -N`) before eval — untracked files are
   invisible to `.#` refs.

   > Stage new files (`git add -N`) before eval — untracked files are
   > invisible to `.#` refs (classic flakes gotcha; the error looks like a
   > missing file/attr). [usage]

8. Run `just lint-nix` before committing nix-adjacent changes. [usage]
9. Run `just store-audit` when the store feels large. [usage]
10. Never use `--impure` for debugging — use `nix eval .#attr`
    (git-filtered). [usage]

## Review checklist

- [ ] Is the full error message captured (file, line, column)?
- [ ] Was `nix log` used to view the build log?
- [ ] Was `nix develop .#<name>` used to reproduce interactively?
- [ ] Are new files `git add`-ed before eval?
- [ ] Is `--impure` avoided?
- [ ] Was `just lint-nix` run?
- [ ] Was `just store-audit` run if store growth is suspected?
- [ ] Are FOD hashes updated via `just update-hashes` (not manually guessed)?
- [ ] Is a HOST-GATE note present if the fix was not verified in this
      container?

## Implementation checklist

- [ ] Reproduce the error with the exact command
- [ ] Capture the full error output (stderr + stdout)
- [ ] Identify error class: eval error vs build error vs runtime error
- [ ] For eval errors: use `nix repl` or `nix eval .#<attr>` to isolate
- [ ] For build errors: use `nix log .#<name>` or `nix build -L`
- [ ] For interactive debugging: `nix develop .#<name>` then run phase
      commands
- [ ] For hash mismatches: `just update-hashes`
- [ ] For store growth: `just store-audit` then `just gc`
- [ ] Verify the fix with `nix build .#<name>`
- [ ] Run `just lint-nix` after any nix file changes
- [ ] Add HOST-GATE note if not verified on host

## Validation hooks

- `nix flake check --no-build` — validate flake structure.

  > `nix flake check --no-build`  # validate the flake without building
  > [nix-commands]

- `nix build .#<name>` — verify the build succeeds.
- `nix eval .#<attr>` — verify evaluation succeeds.
- `just lint-nix` — purity violation guard. [usage]
- `just store-audit` — store growth audit. [usage]
- `just verify` — the composite gate that includes `lint-nix` and
  `store-audit`.
- `nix build --keep-failed` — keep failed build dirs for inspection.

## Examples

### 1. Diagnosing an undefined variable

A non-`rec` attrset that fails, then the fix with `rec`:

```nix
# Fails: error: undefined variable 'one'
{
  one = 1;
  two = one + 1;
}

# Fixed:
rec {
  one = 1;
  two = one + 1;
}
```

### 2. Diagnosing a coercion error

Int interpolation failing, then fixed with `builtins.toString`:

```nix
let x = 1; in "${x}"  # error: cannot coerce an integer to a string

# Fixed:
let x = 1; in "${builtins.toString x}"
```

### 3. Viewing a build log

```bash
nix log .#workestrate
nix log /nix/store/abc123-workestrate-0.1.0.drv
nix build .#workestrate -L
```

### 4. Interactive debugging with nix develop

```bash
nix develop .#workestrate
# Inside the shell:
configurePhase
buildPhase
installPhase
```

### 5. Using nix repl

```shell-session
nix repl
# Inside repl:
:l flake.nix
:p packages.x86_64-linux.workestrate.meta.description
:q
```

### 6. Using builtins.trace

```nix
builtins.trace "debug: value of x is ${builtins.toString x}" x
```

### 7. Using lib.debug.traceVal

```nix
pkgs.lib.debug.traceVal someAttrSet
```

### 8. Fixing a hash mismatch

```bash
just update-hashes
# Follow the printed instructions to inline the got: hash
```

### 9. Diagnosing store growth

```bash
just store-audit
just gc
```

## Common mistakes

- Guessing the cause without reading the build log (`nix log`).
- Using `--impure` to "work around" eval errors (masks the real issue,
  balloons the store). [usage]
- Forgetting to `git add` new files before `nix build`/`nix eval`. [usage]
- Manually guessing FOD hashes instead of using `just update-hashes`. [purity]
- Running `cargo`/`gcc` outside the devshell (`just shell`). [usage]
- Using `nix build .#default` (there is no default output). [usage]
- Ignoring store growth during debugging (each impure eval copies the
  working tree). [purity]
- Not cleaning up `result*` symlinks after debugging. [usage]

## Strict vs contextual guidance

**Strict (always):**

- Never use `--impure`.
- Always read the full error message.
- Always `git add` new files before eval.
- Always use `just update-hashes` for FOD hashes.

**Contextual:**

- `nix develop` for interactive debugging is acceptable (sanctioned impure
  zone per [purity-and-sandboxing](purity-and-sandboxing.md)).
- `builtins.trace` in production code should be removed before committing.
- `--keep-failed` is a debugging aid, not for CI.

## Policy decisions for individual repos

- This repo (ai-workbench): single-user Nix, no daemon, no sudo. Debugging
  requires host Nix (HOST-GATE). [usage]
- The flake has NO `packages.default` — always use `.#workestrate`. [usage]
- `just lint-nix` is the active purity guard; `just store-audit` is the
  active store-growth guard. Both are wired into `just verify`. [usage]
- FOD hash updates go through `just update-hashes` (prints hashes, operator
  manually inlines). [purity]

## Related docs

- [Purity and Sandboxing](purity-and-sandboxing.md)
- [Store Hygiene and GC](store-hygiene-and-gc.md)
- [Nix Commands](nix-commands.md)
- [Language Fundamentals](language-fundamentals.md)
- [Derivations and Builds](derivations-and-builds.md)
- [Devshells](devshells.md)
- [Nixpkgs Library](nixpkgs-library.md)
- [Nix Store and Paths](nix-store-and-paths.md)
- [Nix Purity (repo-root quick reference)](../nix-purity.md)

## Related skills

- [Nix Usage Skill](../../.agents/skills/nix-usage/SKILL.md) — failure
  modes table, anti-accumulation patterns, agent rules

## Citations

[1] [Nix Troubleshooting Guide](https://nix.dev/guides/troubleshooting.html)
[2] [Nix build command reference](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build)
[3] [Nix develop command reference](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop)
[4] [Nix language builtins](https://nix.dev/manual/nix/2.34/language/builtins)
[5] [Nix language derivations](https://nix.dev/manual/nix/2.34/language/derivations)
[6] [Nixpkgs stdenv mkDerivation](https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv)
[7] [Nix language basics tutorial](https://nix.dev/tutorials/nix-language.html)
[8] [Nixpkgs library functions](https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)
[9] [Nix FAQ (guides)](https://nix.dev/guides/faq.html)
[10] [Nix FAQ (concepts)](https://nix.dev/concepts/faq.html)
