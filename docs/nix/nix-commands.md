---
type: Reference
resource: https://nix.dev/manual/nix/stable/command-ref/command-ref.html
title: Nix Commands
description: Reference for the core Nix CLI commands — build, develop, run, flake, profile, store, eval, fmt, and supporting utilities — with project-specific usage for the ai-workbench flake.
tags: [nix, cli, commands, nix-build, nix-develop, nix-run, nix-flake]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Commands

## Purpose

This document is a reference for the core Nix CLI commands, intended for AI
coding agents and maintainers working in the ai-workbench repository. It covers
the experimental `nix` (new CLI) commands — `build`, `develop`, `run`, `flake`,
`profile`, `store`, `eval`, `fmt`, and supporting utilities — with project-specific
usage for the ai-workbench flake.

The ai-workbench project enforces several constraints that override general Nix
usage: `--impure` is forbidden (it copies the raw working tree into the store),
`nix fmt` is not configured, and there is no `.#default` output (the only package
is `.#workestrate`). These constraints are enforced by the `just lint-nix` guard
and documented in `/docs/nix-purity.md`.

## Sources used

- https://nix.dev/manual/nix/stable/command-ref/command-ref.html (Nix command reference)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-build.html (nix build)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-develop.html (nix develop)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-run.html (nix run)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-flake.html (nix flake)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-profile.html (nix profile)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-store.html (nix store)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-eval.html (nix eval)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-fmt.html (nix fmt)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-log.html (nix log)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-path-info.html (nix path-info)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-copy.html (nix copy)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-registry.html (nix registry)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-search.html (nix search)
- https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-hash.html (nix hash)

### Crawl ledger

The crawl was extracted from a local clone of github.com/nixos/nix.dev at commit
`139034be5e14320c05f792872e6150bd981490d5` (2026-07-21). The command-specific
pages (62-64) were fetched from the Nix 2.34 manual at
https://nix.dev/manual/nix/2.34/command-ref/new-cli/.

**SEED** (directly crawled command pages):

- `docs/nix/.crawl/62-nix-command-build.md` — `nix build`
- `docs/nix/.crawl/63-nix-command-develop.md` — `nix develop`
- `docs/nix/.crawl/64-nix-command-run.md` — `nix run`

**DISCOVERED & VISITED** (the manual index that points to them):

- `docs/nix/.crawl/42-nix-reference-manual.md` — Nix reference manual index

## Related Nix guidance

- `/docs/nix/source-map.md` — provenance index for all crawled nix.dev sources
- `/docs/nix-purity.md` — Nix purity rules and anti-accumulation patterns (the authoritative source for the `--impure` prohibition)
- `/docs/nix-store-accumulation-report.md` — store growth incident report
- `/docs/migration/nix-store-gc-remediation-spec.md` — GC remediation spec

## Core guidance

### nix build

Build a derivation or fetch a store path.

> `nix build` builds the specified _installables_. Installables that resolve to derivations are built (or substituted if possible). Store path installables are substituted.
> (Nix 2.34 manual, nix3-build)

> Unless `--no-link` is specified, after a successful build, it creates symlinks to the store paths of the installables. These symlinks have the prefix `./result` by default; this can be overridden using the `--out-link` option.
> (Nix 2.34 manual, nix3-build)

| Flag | Description |
|---|---|
| `--no-link` | Do not create symlinks to result paths. |
| `--print-out-paths` | Print the resulting output paths on stdout. |
| `--out-link` / `-o <path>` | Use _path_ as the output link prefix instead of `./result`. |
| `--json` | Produce JSON output. |
| `--dry-run` | Show what would be built/downloaded without doing it. |
| `--rebuild` | Rebuild the derivation even if the output already exists. |
| `-L` / `--print-build-logs` | Print full build logs on standard error. |
| `-v` / `--verbose` | Increase logging verbosity. |
| `--refresh` | Consider all previously downloaded files out-of-date. |
| `--override-input <input-path> <flake-url>` | Override a specific flake input. |

**Project usage:**

```bash
nix build .#workestrate
```

`.#workestrate` is the only package output. There is **no** `.#default`. In
scripts, use `--no-link --print-out-paths` to avoid pinning closures with
`result` symlinks:

```bash
nix build .#workestrate --no-link --print-out-paths
```

### nix develop

Run a bash shell that provides the build environment of a derivation.

> `nix develop` starts a `bash` shell that provides an interactive build environment nearly identical to what Nix would use to build _installable_.
> (Nix 2.34 manual, nix3-develop)

**Flake output resolution:** tries `devShells.<system>.default` then
`packages.<system>.default`; with a name, tries `devShells.<system>.<name>`,
`packages.<system>.<name>`, `legacyPackages.<system>.<name>`.

| Flag | Description |
|---|---|
| `-c` / `--command <command> [args]` | Run _command_ in the devshell instead of an interactive shell. |
| `--phase <phase-name>` | Set the phase to run (e.g. `buildPhase`). |
| `--build` | Run the build phase. |
| `--configure` | Run the configure phase. |
| `--check` | Run the check phase. |
| `--install` | Run the install phase. |
| `--unpack` | Run the unpack phase. |
| `--profile <path>` | Use the specified profile. |
| `--redirect <installable> <outputs-dir>` | Redirect a dependency. |
| `-i` / `--ignore-env` | Ignore the user environment. |
| `-k` / `--keep-env-var <name>` | Keep an environment variable. |
| `--impure` | Allow access to mutable paths. **FORBIDDEN in this project.** |

**Project usage:**

```bash
just shell                         # enter the default devshell
just shell -c cargo --version      # run a command in the devshell
```

The devshell provides `cargo`, `clippy`, `gcc`, `just`, `pkg-config`, `rustc`,
`rustfmt`. **NEVER** use bare `cargo` or `rustc` — always `just shell -c ...`.

### nix run

Run a Nix application.

> `nix run` builds and runs _installable_, which must evaluate to an _app_ or a regular Nix derivation.
> (Nix 2.34 manual, nix3-run)

> If _installable_ evaluates to a derivation, it will try to execute the program `<out>/bin/<name>`, where _out_ is the primary output store path of the derivation, and _name_ is the first of the following that exists: The `meta.mainProgram` attribute... The `pname` attribute... The name part of the value of the `name` attribute of the derivation.
> (Nix 2.34 manual, nix3-run)

**Flake output resolution:** `apps.<system>.default`,
`packages.<system>.default`; with name: `apps.<system>.<name>`,
`packages.<system>.<name>`, `legacyPackages.<system>.<name>`.

| Flag | Description |
|---|---|
| `--impure` | Allow access to mutable paths. **FORBIDDEN in this project.** |
| `--refresh` | Consider all previously downloaded files out-of-date. |
| `-L` / `--print-build-logs` | Print full build logs on standard error. |

Arguments after `--` are passed to the program.

**Project usage:** There is **no** `nix run .#default` in this project (no
`apps` output). Use `nix run nixpkgs#<pkg>` to run packages from nixpkgs:

```bash
nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json
```

This is used by `just update-hashes`.

### nix flake

Subcommands for flake management.

| Subcommand | Purpose |
|---|---|
| `init` | Create a `flake.nix` from a template in the current directory. |
| `new` | Create a flake in a new directory. |
| `check` | Check that a flake evaluates correctly. |
| `show` | Show the outputs of a flake. |
| `update` | Update `flake.lock` to the latest input revisions. |
| `lock` | Create or update the lock file. |
| `archive` | Copy the flake and its closure to the Nix store. |
| `metadata` | Show flake metadata. |
| `clone` | Clone the flake source repository. |

**Project usage:**

```bash
nix flake check --no-build    # validate the flake without building
nix flake update              # update deliberately, NEVER in CI
```

`nix flake update` pulls a multi-GB nixpkgs closure. After running it, run
`just gc`.

### nix profile

Manage user profiles.

| Subcommand | Purpose |
|---|---|
| `install` | Install a package into the profile. |
| `remove` | Remove a package from the profile. |
| `list` | List installed packages. |
| `upgrade` | Upgrade packages in the profile. |
| `rollback` | Roll back to the previous profile generation. |
| `history` | Show profile history. |
| `wipe-history` | Delete non-current profile generations. |
| `diff-closure` | Show the closure difference between two generations. |

`nix profile install` pins closures forever (survives GC). Prefer `nix shell`
or `nix run` for ad-hoc use. In this project, the system profile is at
`/nix/var/nix/profiles/default` and `~/.nix-profile` symlinks to it.

### nix store

Store management subcommands.

| Subcommand | Purpose |
|---|---|
| `gc` | Garbage collect unreachable store paths. |
| `optimise` | Deduplicate identical store paths (hard-link). |
| `ls` | List the contents of a store path. |
| `cat` | Print a file inside a store path. |
| `path-info` | Show information about store paths. |
| `prefetch-file` | Prefetch a file into the store. |
| `add` | Add a path to the store. |
| `delete` | Delete a store path. |
| `repair` | Repair a store path. |

**Project usage:** `just gc` runs `nix-collect-garbage --delete-old` +
`nix store optimise`. `just store-audit` runs `nix path-info --all --json`
piped through `scripts/store-audit.py`.

### nix eval

Evaluate a Nix expression.

| Flag | Description |
|---|---|
| `.#<attr>` | Evaluate a flake output attribute. |
| `--json` | Produce JSON output. |
| `--impure` | Allow access to mutable paths. **FORBIDDEN in this project.** |
| `--raw` | Print raw string output (no quotes). |
| `--apply` | Apply a function to the result. |

**Project usage:**

```bash
nix eval .#packages.x86_64-linux.pi-image.drvPath
```

This is used by `just store-delta-check` to measure store growth from a pure
eval. **NEVER** use `--impure` — it copies the raw working tree into the store
(see `/docs/nix-purity.md`).

### nix fmt

Format Nix files (experimental).

This project does **not** configure a formatter. There is **no** `nix fmt`
usage. The `just lint-nix` guard checks for purity violations, not formatting.

### nix log

Show build logs.

```bash
nix log .#<name>          # build log for a flake output
nix log <store-path>      # log for a specific store path
```

### nix path-info

Show store path information.

| Flag | Description |
|---|---|
| `--recursive` | Include the closure of the paths. |
| `--json` | Produce JSON output. |
| `--closure-size` | Show the closure size. |
| `--all` | Show all store paths. |

**Project usage:** `nix path-info --all --json` is used by `just store-audit`
to audit store space consumption.

### nix copy

Copy closures between stores.

| Flag | Description |
|---|---|
| `--to <store-uri>` | Copy to the specified store. |
| `--from <store-uri>` | Copy from the specified store. |
| `--recursive` | Copy the closure. |
| `ssh://` | Remote store URI scheme. |

Not commonly used in this single-host project.

### nix registry

Manage the flake registry.

| Subcommand | Purpose |
|---|---|
| `list` | Show registry entries. |
| `add` | Add an entry. |
| `remove` | Remove an entry. |
| `pin` | Pin a flake input to its current revision. |

### nix search

Search packages.

```bash
nix search nixpkgs <query>
```

Searches nixpkgs for packages matching the query.

### nix hash

Compute/convert hashes.

| Subcommand | Purpose |
|---|---|
| `file` | Hash a file. |
| `to-base32` | Convert a hash to base32. |
| `to-sri` | Convert a hash to SRI format. |
| `path` | Hash a store path. |

**Project usage:** `nix run nixpkgs#prefetch-npm-deps` (used by `just
update-hashes` to compute FOD hashes for agent recipes).

### Common flags

Flags shared across commands:

| Flag | Description |
|---|---|
| `--override-input <input-path> <flake-url>` | Override a specific flake input. Implies `--no-write-lock-file`. |
| `--refresh` | Consider all previously downloaded files out-of-date. |
| `-v` / `--verbose` | Increase logging verbosity. |
| `--print-build-logs` / `-L` | Print full build logs on standard error. |
| `--debug` | Set logging verbosity to 'debug'. |
| `--quiet` | Decrease logging verbosity. |
| `--offline` | Disable substituters; consider all downloaded files up-to-date. |
| `--impure` | Allow access to mutable paths. **FORBIDDEN in this project.** |
| `--no-update-lock-file` | Do not allow updates to the flake's lock file. |
| `--no-write-lock-file` | Do not write the newly generated lock file. |
| `--json` | Produce JSON output. |
| `--dry-run` | Show what would happen without doing it. |

## Practical rules

1. **NEVER** use `--impure` — it copies the raw working tree (16-28G of
   gitignored `target/`) into the store. Use native `.#` refs.
2. **NEVER** use `nix fmt` — the project does not configure a formatter.
3. **NEVER** use `nix build .#default` — there is no such output. Use
   `.#workestrate`.
4. **NEVER** use bare `cargo` or `rustc` — always `just shell -c ...`.
5. **NEVER** run `nix flake update` in CI — it pulls a multi-GB nixpkgs closure.
   Update deliberately, then run `just gc`.
6. Stage new files (`git add -N`) before eval — untracked files are invisible to
   `.#` refs.
7. Clean up `result*` symlinks and `/tmp/*.tar.gz` out-links when done — they
   pin closures forever.
8. Run `just lint-nix` before committing nix-adjacent changes.
9. After `nix flake update`, run `just gc`.
10. Use `--no-link --print-out-paths` in scripts to avoid creating `result`
    symlinks that pin GC roots.
11. The devshell toolchain is gcroot-pinned at
    `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. Re-pin after
    `flake.lock` changes if stale.

## Review checklist

1. Is `--impure` absent from all nix invocations?
2. Are new files `git add`-ed before `nix build`/`nix eval`?
3. Are `result*` symlinks cleaned up after builds?
4. Is `nix flake update` run deliberately (never in CI)?
5. Is `just lint-nix` passing?
6. Was `just store-audit` reviewed (informational top-20 report; never gates)?
7. Are cargo/rustc invocations wrapped in `just shell -c`?
8. Is the correct output used (`.#workestrate`, not `.#default`)?

## Implementation checklist

1. Enter the devshell: `just shell` (or `just shell -c <cmd>` for one-shot;
   it wires the devenv-root override).
2. Build the package: `nix build .#workestrate` then
   `./result/bin/workestrate --version`.
3. Validate the flake: `nix flake check --no-build`.
4. Update FOD hashes: `just update-hashes` (runs `nix run nixpkgs#prefetch-npm-deps`
   + `nix build .#<name> --no-link`).
5. Run full verification: `just verify` (includes `lint-nix` + `store-audit`).
6. Collect garbage: `just gc` (`nix-collect-garbage --delete-old` +
   `nix store optimise`).
7. Audit the store: `just store-audit` (top-20 paths + source-path gate).

## Validation hooks

```bash
# Nix itself
nix --version
nix shell nixpkgs#hello -c hello

# Flake validation
nix flake check --no-build

# Dev shell tools
just shell -c cargo --version
just shell -c rustc --version
just shell -c gcc --version
just shell -c just --version

# Package build
nix build .#workestrate
./result/bin/workestrate --version

# Nix purity lint (active gate, wired into `just verify`)
just lint-nix

# Store audit (top-20 + blocking source-path gate, wired into `just verify`)
just store-audit

# Garbage collection + dedup
just gc

# Full pre-merge verification (includes lint-nix + store-audit)
just verify

# Verify + nix build
just verify-full

# Update FOD dependency hashes
just update-hashes

# Periodic store-delta check (not wired into verify; run on nix-capable host)
just store-delta-check
```

## Examples

```bash
# nix build — build the project package
nix build .#workestrate
./result/bin/workestrate --version

# nix build — script-safe (no result symlink, print path only)
nix build .#workestrate --no-link --print-out-paths

# nix build — build from nixpkgs
nix build nixpkgs#hello
./result/bin/hello

# just shell — enter the devshell (wraps `nix develop --override-input devenv-root ...`)
just shell

# just shell — run a single command
just shell -c cargo build

# nix run — run a nixpkgs package
nix run nixpkgs#vim -- --help

# nix run — prefetch npm deps (used by just update-hashes)
nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json

# nix flake — check the flake
nix flake check --no-build

# nix flake — show outputs
nix flake show

# nix flake — update lock file (DELIBERATELY, never in CI)
nix flake update

# nix eval — evaluate a flake attribute (pure)
nix eval .#packages.x86_64-linux.pi-image.drvPath

# nix eval — JSON output
nix eval .#packages.x86_64-linux.workestrate.meta.description --json

# nix path-info — all store paths as JSON (used by just store-audit)
nix path-info --all --json

# nix path-info — closure size of a specific path
nix path-info --recursive --closure-size /nix/store/...

# nix store — garbage collect + optimise (via just gc)
nix-collect-garbage --delete-old
nix store optimise

# nix search — find packages
nix search nixpkgs hello

# nix hash — convert to SRI
nix hash to-sri sha256-...
```

## Common mistakes

1. **Using `--impure`** — copies the raw working tree (16-28G) into the store.
   Use native `.#` refs instead.
2. **Using `.#default`** — there is no `packages.x86_64-linux.default`. Use
   `.#workestrate`.
3. **Running `nix flake update` in CI** — pulls a multi-GB nixpkgs closure.
   Update deliberately only.
4. **Using bare `cargo`/`rustc`** — outside the devshell, the toolchain is
   missing. Always use `just shell -c cargo ...`.
5. **Leaving `result*` symlinks** — they pin closures forever, surviving GC.
   Use `--no-link --print-out-paths` in scripts, or clean up after.
6. **Forgetting to `git add` new files** — untracked files are invisible to `.#`
   refs (classic flakes gotcha).
7. **Using `nix fmt`** — the project does not configure a formatter.
8. **Expecting `msb` in the Nix closure** — the Microsandbox SDK downloads `msb`
   at runtime; it is not bundled.

## Strict vs contextual guidance

- **STRICT (always enforced):** No `--impure`. No `nix fmt`. No
  `nix build .#default`. No bare `cargo`/`rustc`. No `nix flake update` in CI.
  `git add` new files before eval. Clean up `result*` symlinks.
- **CONTEXTUAL (judgment call):** `nix profile install` (pins closures; prefer
  `nix shell`/`nix run` for ad-hoc use). `nix copy` (not commonly needed in
  single-host setup). `nix registry pin` (useful for reproducibility but not
  required).

## Policy decisions for individual repos

- This project (ai-workbench) enforces: no `--impure` (active guard via
  `just lint-nix`), no `nix fmt`, `.#workestrate` as the only package output,
  `just shell` as the only devshell entry point (it wraps `nix develop` with
  the devenv-root override), `just gc` for store hygiene,
  `just store-audit` as a blocking gate in `just verify`.
- Other repos may: configure `nix fmt` with a formatter (e.g., `nixfmt` or
  `alejandra`), have a `.#default` output, use `nix profile install` for
  persistent tooling, or allow `--impure` in specific eval contexts (not
  recommended).

## Related docs

- `/docs/nix/source-map.md` — provenance index for all crawled nix.dev sources
- `/docs/nix-purity.md` — Nix purity rules and anti-accumulation patterns
- `/docs/nix-store-accumulation-report.md` — store growth incident report
- `/docs/migration/nix-store-gc-remediation-spec.md` — GC remediation spec

## Related skills

- `nix-usage` — project-specific Nix flake, dev shell, and toolchain reference
- `workflow-rust-implementation-*` — Rust implementation workflows (use `just shell -c cargo`)
- `workflow-rust-validation-*` — Rust validation workflows (use `just shell` for toolchain)
