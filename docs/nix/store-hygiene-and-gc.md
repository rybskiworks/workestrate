---
type: Reference
resource: https://nix.dev/manual/nix/stable/glossary.html
title: Store Hygiene and Garbage Collection
description: Nix store hygiene — garbage collection, GC roots, accumulation patterns, auditing, optimisation, and the 29 GB incident remediation for the ai-workbench flake.
tags: [nix, store, gc, garbage-collection, hygiene, audit]
timestamp: 2026-07-24T01:20:00Z
---

## Overview

This document is the canonical reference for keeping the Nix store bounded in the ai-workbench project. It covers how the store works (append-only, content-addressed, immutable), garbage collection mechanics and commands, GC roots and their role in keeping paths alive, the three-vector store-growth model, the 29 GB-per-eval incident (including the full 7-cycle GC timeline table), root causes and remediation, the five anti-accumulation patterns, the store-audit tooling (`just store-audit`, `scripts/store-audit.py`), project hygiene recipes (`just gc`, the devshell gcroot pin, disk-pressure tradeoffs), store inspection commands, and best practices. It elevates and expands the incident report at [Nix Store Accumulation Report](../nix-store-accumulation-report.md) into the corpus structure. It is the operational companion to [Nix Purity](../nix-purity.md), [Nix Store and Paths](nix-store-and-paths.md), and the [Nix Usage skill](../../.agents/skills/nix-usage/SKILL.md).

## How the Nix store works

- The Nix store lives at `/nix/store`. It is content-addressed and append-only: every flake evaluation or build copies its inputs (nixpkgs, toolchains, repo source) into the store as new, immutable paths. Nothing is ever overwritten — each new eval creates new paths. [acc]
- Verbatim from the accumulation report:
  > "**Append-only cache.** The Nix store lives at `/nix/store`. Every flake evaluation or build copies its inputs (nixpkgs, toolchains, your repo source) into the store as new, immutable paths. Nothing is ever overwritten — each new eval creates new paths." [acc]
- Source-path copying: "When a flake is evaluated, Nix copies the repo working tree into the store as a 'source path.' If a `.git` directory exists, Nix respects `.gitignore` and excludes gitignored files from the copy. If `.git` is absent, it copies everything." [acc]
- Project baseline: "Baseline for this machine: 63 paths / 114M. This is nix itself plus the flake registry. Everything above that is transient eval/build output." [acc]
- The store-growth model from nix-purity.md explains why impurity balloons the store: "a dirty source closure changes hash on every edit, so Nix produces a new store path each time, and those paths are kept alive by live GC roots (dev-shell profiles, `result*` symlinks, `nix build` outputs)." [purity]

## GC roots

GC roots are the anchors that keep store paths alive. A path is collectable only if it is unreachable from all roots.

- **`/nix/var/nix/gcroots/`** — the canonical gcroots directory. Symlinks here pin closures. [store]
- **`/nix/var/nix/gcroots/per-user/<user>/`** — per-user gcroots. The project pins the devshell at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. [usage]
- **`/nix/var/nix/profiles/`** — profile generations. Each profile generation is a root. [store]
- **`result*` symlinks** in the working directory — created by `nix build` (without `--no-link`). These pin closures forever until removed. [usage]
- **Live process temproots** — a running `nix` process holds a "temproot" that prevents its in-use paths from being collected. This is why GC sometimes frees less than expected. [acc]

Verbatim from nix-purity.md on stale roots as a growth vector:
> "Live `result*` symlinks, dev-shell profiles, and `nix build` outputs keep paths alive. A path that is unreachable is collectable; a path held by a live root is not. Stale `result*` symlinks pointing at long-gone paths, and accumulated dev-shell profiles, are the second growth vector." [purity]

The devshell gcroot pin (Work Item 3, commit a315f24):
> "`nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` succeeded directly (no print-dev-env fallback needed). Pin closure = 2.9 GiB. GC-survival verified: `nix-collect-garbage -d` freed 1.9 GiB of unrooted paths, store settled at 3.3G with the toolchain retained, and post-GC `nix develop -c true` completed in 9.1s with **zero re-fetch**." [acc]

Safe alternative to `result` symlinks: `--no-link --print-out-paths` (no symlink created). [usage]

## Garbage collection commands

- Verbatim from crawl 61 (the Nix manual):
  > "`nix-env` operations such as upgrades (`-u`) and uninstall (`-e`) never actually delete packages from the system. All they do (as shown above) is to create a new user environment that no longer contains symlinks to the 'deleted' packages." [61]
  > "Of course, since disk space is not infinite, unused packages should be removed at some point. You can do this by running the Nix garbage collector. It will remove from the Nix store any package not used (directly or indirectly) by any generation of any profile." [61]
- GC cannot delete anything a live process is actively using — this is why GC sometimes frees less than expected. [acc]
- Old generations must be deleted first for GC to be effective:
  > "Note however that as long as old generations reference a package, it will not be deleted. After all, we wouldn't be able to do a rollback otherwise. So in order for garbage collection to be effective, you should also delete (some) old generations." [61]
- Commands:
  - `nix-collect-garbage -d` (or `--delete-old`) — deletes all old generations of all profiles in `/nix/var/nix/profiles` and collects garbage. Verbatim: "is a quick and easy way to clean up your system." [61]
  - `nix store gc` — modern CLI equivalent for running the garbage collector. [store]
  - `nix-store --gc` — legacy direct GC invocation. [61]
  - `nix-store --gc --print-dead` — preview what would be deleted. [61]
  - `nix-store --gc --print-live` — show paths that won't be deleted. [61]
  - `nix-env --delete-generations old` — delete all old (non-current) generations of the current profile. [61]
  - `nix-env --delete-generations 14d` — delete all generations older than 14 days (except current). [61]
  - `nix-env --delete-generations 10 11 14` — delete specific generations by number. [61]
- GC behavior is affected by configuration options:
  > "The behaviour of the garbage collector is affected by the `keep-derivations` (default: true) and `keep-outputs` (default: false) options in the Nix configuration file. The defaults will ensure that all derivations that are build-time dependencies of garbage collector roots will be kept and that all output paths that are runtime dependencies will be kept as well." [61]

## Store optimisation

- **`nix store optimise`** — hard-links identical files across store paths. Deduplicates content-addressed copies (vector (c) of the store-growth model). [purity]
- **`auto-optimise-store`** — advisory setting that deduplicates identical content but is NOT a substitute for the purity rules. Verbatim from nix-purity.md:
  > "`auto-optimise-store` is advisory: it deduplicates identical content but is **not** a substitute for the purity rules. It reduces vector (c) but does nothing for vectors (a) and (b)." [purity]
- The same content under different store paths is duplicated until `nix store optimise` (or `auto-optimise-store`) deduplicates it. This is the third growth vector: even when content is identical, distinct paths consume space until deduplication runs. [purity]
- `just gc` runs `nix-collect-garbage --delete-old` followed by `nix store optimise` (dedupe), reclaiming unreachable paths AND deduplicating content-addressed copies in one recipe. [usage]

## The store-growth model

The store grows along three vectors. Understanding all three is necessary to keep the store bounded.

> "The store grows along three vectors. Understanding all three is necessary to keep the store bounded." [purity]

**(a) Per-edit churn**
> "Each evaluation with an impure source filter produces a new store path. With a dirty `target/` or `agents/*/build/` in the closure, every `nix develop` or `nix build` copies the whole closure again. This is the dominant growth vector during active development — it is what produced the 29 GB incident." [purity]

The mechanism: a dirty source closure changes hash on every edit, so Nix produces a new store path each time, and those paths are kept alive by live GC roots. [purity]

**(b) GC root accumulation**
> "Live `result*` symlinks, dev-shell profiles, and `nix build` outputs keep paths alive. ... Stale `result*` symlinks pointing at long-gone paths, and accumulated dev-shell profiles, are the second growth vector." [purity]

**(c) Content-addressed copies**
> "The same content under different store paths is duplicated until `nix store optimise` (or `auto-optimise-store`) deduplicates it. This is the third vector: even when content is identical, distinct paths consume space until deduplication runs." [purity]

## The 29 GB incident timeline

From nix-purity.md:
> "An impure eval path previously caused Nix to copy `target/` and `agents/*/build/` into the store on every evaluation. Because those directories were large and changed on every edit, each evaluation produced a new content-addressed store path containing the full dirty source closure. The store grew at roughly 1 GB/min and reached approximately 29 GB before the impurity was caught and the source filter was fixed." [purity]

From the accumulation report TL;DR:
> "The Nix store kept refilling because opencode subagent sessions running inside this container repeatedly executed `nix eval --impure` and `nix develop` gates against the repo's flake. Each run copies the entire repo working tree (including multi-GB `agents/` and Rust `target/` directories) plus full Rust/LLVM/GCC toolchains into the append-only Nix store. We garbage-collected 7+ times across this session, freeing a cumulative ~76 GiB; each time the store refilled within hours. GC treats the symptom; the subagent-run gates are the cause." [acc]

Full 7-cycle GC timeline table (verbatim from the accumulation report, Jul 22–23, 2025):

| # | Store size before | Freed | Trigger identified |
|---|---|---|---|
| 1 | 14G / 5773 paths | 12.0 GiB / 5710 paths | Old accumulation (pre-session) |
| 2 | 38G / 5900 paths | 8.5 GiB / 5622 paths | opencode subagent running B1/B2 `nix eval --impure` gates from `docs/migration/80-remediation-plan.md` |
| 3 | 35G / 64 paths | 32.9 GiB / 1 path | Single repo-source snapshot (28G Rust `target/debug` + 6.8G `agents/`) held alive by a stale temproot from a dead PID |
| 4 | 9.7G / 5896 paths | 8.6 GiB / 5833 paths | Two flake evals with different nixpkgs revisions (rustc 1.95.0 + 1.96.1, duplicate LLVM 21.1.8) |
| 5 | 7.5G / 5527 paths | 6.2 GiB / 2194 paths, then blocked | Live `nix develop -c cargo test` and `nix develop -c cargo clippy` pinning 4264 paths |
| 6 | 4.9G / 4941 paths | 4.4 GiB / 4966 paths, then blocked | Live `nix eval --impure` gate + 2.0G build temp dir (`tmp-1794066-*`) |
| 7 | 2.9G / 4878 paths | 3.2 GiB / 4982 paths | Live `nix print-dev-env --json .` (PID 1795515) killed on user order; GC then reached baseline |

Cumulative freed across all cycles: ~76 GiB. [acc]

## Root causes

Ranked by impact, from the accumulation report:

### 1. Subagents ran HOST-NIX-gated evals inside the container

> "The remediation plan (`docs/migration/80-remediation-plan.md`, lines 344–353) explicitly marks the B1, B2, and B14 regression tests as **HOST-NIX gates** — meaning they should run on the host machine, not inside this container. Despite this, opencode subagent sessions repeatedly executed them here." [acc]

Traced process chains:
- `high_agency_orchestrator` → `lead_short` ("Verify WP1-WP5 merge gate") → `worker_general` ("Run final-state cargo/nix gates") — ran B1/B2 `nix eval --impure` expressions verbatim from the remediation plan. [acc]
- A later chain ran `nix develop -c cargo test` and `nix develop -c cargo clippy` inside a dev shell, creating toolchain + crate paths. [acc]
- The latest was `nix print-dev-env --json .` (parent: opencode PID 1775978), a dev-shell probe. [acc]

Each of these copies toolchains (rustc, LLVM, GCC, Python — 5–7G) and the repo source into the store. [acc]

### 2. Unfiltered repo source copies

> "When `builtins.getFlake (toString ./.)` evaluates, it copies the repo working tree into the store. The current `.gitignore` **does** exclude `target/` (lines 25–26: `target/` and `**/target/`) and `agents/*/repo` + `agents/*/build` (lines 18–20)." [acc]

However, the 35G snapshot in cycle 3 suggests git filtering was bypassed at least once:
> "**Open check:** verify that `builtins.getFlake` is actually respecting `.gitignore` during eval. If `.git` is somehow absent or the eval uses `--impure` with a path that bypasses git filtering, the full 16G `target/` and 2.6G `agents/opencode/repo/` would be copied. The 35G snapshot in cycle 3 suggests this happened at least once." [acc]

### 3. Toolchain re-fetching per eval

> "Each `nix eval` or `nix develop` against a GC'd store re-downloads and instantiates the full Rust/LLVM/GCC/Python closure (~5–7G). Since GC runs between subagent sessions, every new session starts from a clean store and pays the full fetch cost again." [acc]

## Remediation

From the accumulation report's resolution note (2026-07-23, branch migration/tool-model):

### Impure gate rewrites

> "The one true impure gate (`80-remediation-plan.md` B1 regression, old line 349-351) was rewritten from `nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'` to the native git-filtered form `nix eval .#packages.x86_64-linux.pi-image.drvPath`. Verified running: returns `"/nix/store/caql1kmknlqdw4nbr1lcs627g3r7dsfh-workestrator-pi.tar.gz.drv"`. Store delta measured across the run: **0 MB** (5678 MB before/after) — the git-filtered `.#` ref no longer copies the raw working tree." [acc]

### Devshell gcroot pinning (Work Item 3)

> "`nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` succeeded directly (no print-dev-env fallback needed). Pin closure = 2.9 GiB. GC-survival verified: `nix-collect-garbage -d` freed 1.9 GiB of unrooted paths, store settled at 3.3G with the toolchain retained, and post-GC `nix develop -c true` completed in 9.1s with **zero re-fetch**." [acc]

### Disk-pressure tradeoff of the devshell pin

> "Pinning `.#devShells.x86_64-linux.default` (or `nix print-dev-env` closure) keeps ~5-7G of Rust/LLVM/GCC toolchain permanently reachable so post-GC sessions do not re-fetch it. Tradeoff: the overlay is ~95-96% full with **non-nix** data (411G used / 22G avail of 456G at time of writing); a permanent 5-7G pin consumes roughly a quarter to a third of the currently-free space. Accepted because: (a) without the pin every post-GC session re-downloads the same 5-7G anyway (transient, but repeatedly); (b) the pin is the only mechanism that makes `nix develop` survivable across `nix-collect-garbage -d`." [acc]

Rollback:
```sh
rm /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
nix-collect-garbage -d
```

Refresh rule: re-create the pin whenever `flake.lock` changes (fenix/nixpkgs bumps), else the pinned closure goes stale and agents fetch the new toolchain anyway (no breakage, just no benefit). [acc]

## Anti-accumulation patterns

This is the operational quick-reference for agents running nix tasks on this repo. The incident class it prevents: agents running impure evaluation gates that copy the raw working tree into the store on every invocation, accumulating 16–35G per eval until the disk fills. The git-tracked repo is ~735 files / ~12M; an impure eval can copy 16–28G of gitignored `target/` alone. [usage]

| # | Pattern | What it does | Safe alternative | Guard |
|---|---|---|---|---|
| a | Impure/path-mode flake eval: `nix eval --impure`, `builtins.getFlake (toString ./.)`, unfiltered `builtins.path { path = ./.; }` / `src = ./.` | Copies the raw working tree (gitignored `target/` 16–28G, `agents/*/build`, `.workestrate/`) into the store — up to ~35G per invocation; content-addressed so every dirty edit produces a new path | Native flake refs `nix eval .#attr` (git-filtered, ~12M) or filtered `builtins.path { path = ./subdir; filter = ...; }` | `just lint-nix` (checks 1–4); `just store-audit` flags `*-source` paths |
| b | GC-root leaks: `result` symlinks, `/tmp/*.tar.gz` out-links, `nix profile install` | Pins closures forever — unreachable paths survive GC | `--no-link --print-out-paths` (no `result` symlink); `just gc` to collect | `just store-audit` (flags `*-source` paths); manual inspection |
| c | `nix flake update` on rolling nixpkgs | Multi-GB rebuild — new nixpkgs revision pulls new toolchain/closure | Update deliberately (never in CI); run `just gc` after | Operational discipline (no static guard) |
| d | `buildLayeredImage` for new images | 0.5–1G tarballs materialized in the store | `streamLayeredImage` (streams to stdout, no store path) — documented default in `docs/nix-purity.md` | Operational discipline (`docs/nix-purity.md` recipe patterns) |
| e | Per-edit churn: `nix develop` after edits | ~50–100MB per unique source state (content-addressed copies accumulate) | `auto-optimise-store` + `just gc` cadence | `just store-audit` (top-20 report); `just gc` |

[usage]

Agent rules (verbatim from the nix-usage skill):

1. **Regression/eval gates use ONLY native `.#` refs.** NEVER `--impure`. NEVER `toString ./.` or `getFlake` on this repo. NEVER `builtins.path` on the repo root without a `filter =` field.
2. **Stage new files (`git add -N`) before eval** — untracked files are invisible to `.#` refs (classic flakes gotcha; the error looks like a missing file/attr).
3. **Clean up out-links** — remove `result*` symlinks and `/tmp/*.tar.gz` out-links when done; they pin closures forever.
4. **Run `just lint-nix` before committing** nix-adjacent changes.
5. **After `nix flake update`, run `just gc`** — a new nixpkgs revision pulls a multi-GB toolchain closure.
6. **The devshell toolchain is gcroot-pinned** (Work Item 3, commit a315f24) at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. If it is stale after a `flake.lock` change, re-pin:
   ```bash
   nix build .#devShells.x86_64-linux.default --out-link \
     /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
   ```
   Rollback: `rm` the gcroot + `nix-collect-garbage -d`.

[usage]

## Store audit tooling

The project ships `scripts/store-audit.py`, invoked via `just store-audit`.

What it does (verbatim from the script docstring):
> "Reads `nix path-info --all --json` output from stdin and prints the top-20 store paths by closure size." [audit]

With `--fail-if-source-over <MB>` (verbatim from the script docstring):
> "the script additionally scans every path whose name contains `ai-workbench` AND ends with `-source` (the impure path-style copy probe). If any such path's closure size exceeds the given threshold (in MiB, 1 MB = 1_000_000 bytes), the oversized paths are printed to stderr and the script exits 1 — turning the previously passive probe into a blocking gate." [audit]

Non-blocking on input problems (verbatim):
> "on any read/parse failure it prints an informational note and exits 0 — the gate fails only on actual oversized source paths, never on missing/malformed input." [audit]

The `just store-audit` recipe (from the justfile):
- Reports the top-20 store paths by closure size via `nix path-info --all --json` piped to `scripts/store-audit.py --fail-if-source-over 50`.
- Fails (exit 1) if any `*ai-workbench*-source` path exceeds 50 MB closure size — the impure path-style copy probe.
- Non-blocking when nix or python3 is unavailable, or when `nix path-info` itself fails (daemon/DB errors degrade to a note, exit 0).
- Wired into `just verify` as the FINAL step (V2 landed). [usage]

`just store-delta-check` (V3) exists as a recipe but is NOT yet wired into `verify` — it is a periodic host/CI check that measures `/nix/store` growth from one pure eval (`nix eval .#packages.x86_64-linux.pi-image.drvPath`) and fails on new source-path copies exceeding the 50M criterion. [usage]

Manual usage:
```bash
# Top-20 report only (non-blocking)
nix path-info --all --json | python3 scripts/store-audit.py

# Top-20 report + blocking source-path gate (50 MB threshold)
nix path-info --all --json | python3 scripts/store-audit.py --fail-if-source-over 50
```

The `*-source` probe: non-empty `*-source` output referencing `ai-workbench` indicates an unbounded source copy that should be bounded by a `cleanSourceWith` filter. [purity]

## Project hygiene recipes

### `just gc`

Runs `nix-collect-garbage --delete-old` followed by `nix store optimise` (dedupe). Reclaims unreachable store paths and deduplicates content-addressed copies (vectors (b) and (c) of the store-growth model). [usage]

Run regularly to collect unreachable store paths. [purity]

### Devshell gcroot pin

The devshell toolchain is gcroot-pinned at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` (Work Item 3, commit a315f24). This keeps ~5–7 GB of Rust/LLVM/GCC toolchain permanently reachable so post-GC sessions do not re-fetch it. [usage]

Re-pin after `flake.lock` changes:
```bash
nix build .#devShells.x86_64-linux.default --out-link \
  /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
```

Rollback:
```sh
rm /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
nix-collect-garbage -d
```

### Git hooks vs GC (pre-commit shims)

A previous `prek install` wrote `.git/hooks/pre-commit` shims that exec'd a hardcoded store-path prek binary against a generated `.pre-commit-config.yaml`. GC deleted both, so every commit failed unless passed `--no-verify`. The fix: `.git/hooks/pre-commit` is now a pure-sh shim (canonical tracked copy `scripts/git-hooks/pre-commit.sh`; reinstall with `cp scripts/git-hooks/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`) that never references store paths — secret-material greps always run, while tombi/prek gates skip with a message when the toolchain is absent. The generated `.pre-commit-config.yaml` is gitignored and never committed. Inside a devenv shell the declared `git-hooks.hooks.*` (via the nix-tooling devenv modules) enforce with the pinned toolchain; hook exactness otherwise comes from `nix flake check` / CI, not the commit gate.

Canonical note (all repos): a fresh clone ships no hooks, so plain commits work until one is installed. Entering the devenv shell runs git-hooks.nix's installer on every entry: it moves any foreign `.git/hooks/pre-commit` to `.git/hooks/pre-commit.legacy` and installs a generated hook that execs a store-path'd pre-commit against the generated `.pre-commit-config.yaml` — the pure-sh fallback is clobbered, its secret gate is lost in-shell, and the generated hook dangles after GC. Outside the shell (or after GC/shell entry) reinstall the fallback (`cp scripts/git-hooks/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`); its skip messages are audible (stderr) and the secret-material gate is unconditional. Tradeoff accepted until a config follow-up sets `git-hooks.install.enable = false` (or devenv's `git-hooks.skip`) in the shared devenv modules.

### Disk-pressure tradeoffs

The overlay is ~95-96% full with non-nix data (411G used / 22G avail of 456G). A permanent 5-7G devshell pin consumes roughly a quarter to a third of the currently-free space. Accepted because: (a) without the pin every post-GC session re-downloads the same 5-7G anyway; (b) the pin is the only mechanism that makes `nix develop` survivable across `nix-collect-garbage -d`. [acc]

Non-Nix disk usage is a separate concern: "Disk is at ~96% full (416G used / 17G avail on a 456G overlay). The Nix store is now only 114M. The remaining ~416G is non-Nix data — likely repo working trees, build artifacts outside Nix (`~/.cache/ai-workbench/`), Docker images, or other large files." [acc]

### Additional hygiene items

From the accumulation report:
- **Dangling profile symlink:** `~/.local/state/nix/profiles/profile` points to a deleted store path. Not causing accumulation, but should be fixed (`nix profile remove` or recreate the symlink). [acc]
- **Zombie processes:** Dead `nix` processes linger as zombies (`[nix] <defunct>`) because their parent (opencode) doesn't reap them. Harmless for GC, but messy. [acc]
- **`nix store optimise`:** Hard-links identical files across store paths. Could save a few hundred MB if duplicate toolchain paths coexist. Not urgent. [acc]
- **Automatic GC:** Consider adding a `nix-collect-garbage` cron or a `nix.settings.auto-optimise-store` config if accumulation continues. [acc]

## Store inspection commands

- **`nix path-info --all --json`** — the primary inspection command. Produces JSON with `closureSize`, `size`, `references`, `narSize`, and other fields per path. This is the data source for `scripts/store-audit.py`. [audit]
- **`nix path-info --recursive <path>`** — prints the closure of a store path (the transitive set of all paths it references). [store]
- **`nix path-info --closure-size <path>`** — prints the closure size (total disk usage of all paths in the closure). [store]
- **`nix store ls <store-path>`** — lists the contents of a store path (like `ls` but operates on store paths, including remote/substitutable paths). Supports `--recursive` / `-R` for recursive listing and `--long` / `-l` for metadata. [store]
- **`nix store cat <store-path>`** — prints the contents of a file inside a store path to stdout. Useful for inspecting a built file without linking it. [store]
- **`nix store verify`** — verifies store path integrity against their registered NAR hashes. [store]
- **`nix store diff-closures <path1> <path2>`** — shows the closure difference between two store paths (what was added/removed/up/downgraded). [store]
- **`nix-store -qR <path>`** — legacy equivalent of `nix path-info --recursive`. [store]

## Best practices

1. **Run `just gc` regularly.** Collects unreachable paths and deduplicates content-addressed copies. This is the primary hygiene cadence. [purity]
2. **Run `just store-audit` when the store feels large.** Reports the top-20 paths by closure size and flags unbounded `*-source` copies. [purity]
3. **Clean up stale `result*` symlinks.** Remove `result*` symlinks and `/tmp/*.tar.gz` out-links when done with a build — they pin closures forever. Use `--no-link --print-out-paths` instead of bare `nix build` to avoid creating `result` symlinks. [usage]
4. **Delete old profile generations.** Run `nix-env --delete-generations old` (or `nix-collect-garbage -d`) before GC — old generations keep packages alive and prevent collection. [61]
5. **Keep the devshell gcroot pin fresh.** Re-pin after `flake.lock` changes (fenix/nixpkgs bumps), else the pinned closure goes stale. [acc]
6. **Never run `nix eval --impure` or `builtins.getFlake (toString ./.)` on this repo.** Use native `.#` refs (git-filtered, ~12M) instead. Run `just lint-nix` before committing nix-adjacent changes. [usage]
7. **Run `just gc` after `nix flake update`.** A new nixpkgs revision pulls a multi-GB toolchain closure that the old one did not reference. [usage]
8. **Use `streamLayeredImage` over `buildLayeredImage`.** Streams tarballs to stdout without materializing a large store path. [purity]
9. **Relocate `CARGO_TARGET_DIR` out of the source tree.** The canonical path is `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`, set by the devshell shellHook and the top-level justfile. [purity]
10. **Preview before collecting.** Use `nix-store --gc --print-dead` to preview what GC would delete before running it. [61]

## Citations

[61] [Nix Manual — Garbage Collection](https://nix.dev/manual/nix/2.34/package-management/garbage-collection) — crawl source: `.crawl/61-nix-garbage-collection.md`
[purity] [Nix Purity](../nix-purity.md) — project doc
[acc] [Nix Store Accumulation Report](../nix-store-accumulation-report.md) — project doc
[usage] [Nix Usage Skill](../../.agents/skills/nix-usage/SKILL.md) — project skill
[audit] [store-audit.py](../../scripts/store-audit.py) — project script
[store] [Nix Store and Paths](nix-store-and-paths.md) — project doc
