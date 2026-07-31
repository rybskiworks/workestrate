---
type: Reference
resource: https://nix.dev/manual/nix/stable/store/store-path.html
title: Nix Store and Paths
description: The Nix store — store paths, closures, garbage collection, content addressing, and store hygiene for the ai-workbench flake.
tags: [nix, store, paths, garbage-collection, gc, hygiene]
timestamp: 2026-07-24T01:10:50Z
---

## Overview

The Nix store is the content-addressed, append-only filesystem tree at `/nix/store` that holds every build input and output. This document covers store path anatomy, closures, garbage collection, GC roots, the three-vector store-growth model, store hygiene, the project's store-audit tooling, the 29 GB-per-eval incident, anti-accumulation patterns, store inspection commands, and content-addressed derivations. It is the operational companion to [Nix Purity](../nix-purity.md) and the [Nix Usage skill](../../.agents/skills/nix-usage/SKILL.md).

## What is the Nix store?

- The Nix store lives at `/nix/store`. It is content-addressed and append-only: every flake evaluation or build copies its inputs (nixpkgs, toolchains, repo source) into the store as new, immutable paths. Nothing is ever overwritten — each new eval creates new paths.
- Verbatim quotation from crawl 60 (cite as `[60]`):
  > "Nix implements references to store objects as _store paths_. Think of a store path as an opaque, unique identifier: The only way to obtain store path is by adding or building store objects. A store path will always reference exactly one store object." [60]
- Store paths are pairs of a 20-byte digest for identification and a symbolic name for people to read. [60]
- The store directory defaults to `/nix/store` but is in principle arbitrary. [60]
- Referential integrity: "Nix can only guarantee referential integrity if store paths do not cross store boundaries." [60] One can only copy store objects to a different store if the source and target stores' directories match, or the store object has no references. [60]
- Project baseline (from the accumulation report): "Baseline for this machine: 63 paths / 114M. This is nix itself plus the flake registry. Everything above that is transient eval/build output." [acc]

## Store path structure

- A store path is rendered to a filesystem path as the concatenation of: store directory (typically `/nix/store`), path separator (`/`), digest rendered in Nix32 (a variant of base-32; 20 hash bytes become 32 ASCII characters), hyphen (`-`), and name. [60]
- Verbatim example from crawl 60:
  > ```
  > /nix/store/q06x3jll2yfzckz2bzqak089p43ixkkq-firefox-33.1
  > |--------| |------------------------------| |----------|
  > store directory            digest                 name
  > ```
  [60]
- Another example: `/nix/store/jf6gn2dzna4nmsfbdxsd7kwhsk6gnnlr-git-2.38.1` [60]
- The name typically encodes package-name-version (e.g., `hello-2.12.1`).
- "Exactly how the digest is calculated depends on the type of store path. Store path digests are _supposed_ to be opaque, and so for most operations, it is not necessary to know the details." [60]

## Store path types

- **Derivation paths** (`*.drv`): the derivation description itself, a store path ending in `.drv`. It encodes the build recipe (inputs, builder, environment, outputs) as an ATerm-format file.
- **Output paths**: the build results. A derivation produces one or more output paths (e.g., `out`, `dev`, `bin`). These are what `nix build` links as `result`.
- **Source paths**: paths copied into the store from the local filesystem (e.g., via `builtins.path` or `cleanSourceWith`). These end in `-source` in their name when the path is a directory. The project's store-audit specifically flags `*-source` paths referencing `ai-workbench` as a probe for unbounded source copies. [audit]
- **Fixed-output derivation (FOD) paths**: derivations whose output hash is declared in advance (`outputHashAlgo`, `outputHash`, `outputHashMode`). Because the output is verified by hash, the build sandbox permits network access during the fetch phase. FODs are how all dependency fetching works in a pure flake. [purity]

## Closures

- The closure of a store path is the transitive set of all store paths it (directly or indirectly) references. It is the complete set of paths needed to use that path.
- Commands:
  - `nix path-info --recursive <path>` — prints the closure.
  - `nix-store -qR <path>` — legacy equivalent.
  - `nix path-info --all --json` — JSON output including `closureSize` and `size` per path. This is what `scripts/store-audit.py` consumes. [audit]
- Closure size is the total disk usage of all paths in the closure. The store-audit script reads `closureSize` (falling back to `size`) from the JSON. [audit]
- Example: a devshell closure pins the full Rust/LLVM/GCC toolchain (~5–7 GB). The project pins this at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` so post-GC sessions do not re-fetch it. [usage]

## Garbage collection

- Verbatim from crawl 61:
  > "`nix-env` operations such as upgrades (`-u`) and uninstall (`-e`) never actually delete packages from the system. All they do (as shown above) is to create a new user environment that no longer contains symlinks to the 'deleted' packages." [61]
  > "Of course, since disk space is not infinite, unused packages should be removed at some point. You can do this by running the Nix garbage collector. It will remove from the Nix store any package not used (directly or indirectly) by any generation of any profile." [61]
- GC cannot delete anything a live process is actively using — this is why GC sometimes frees less than expected. [acc]
- Old generations must be deleted first for GC to be effective:
  > "Note however that as long as old generations reference a package, it will not be deleted. After all, we wouldn't be able to do a rollback otherwise. So in order for garbage collection to be effective, you should also delete (some) old generations." [61]
- Commands:
  - `nix-collect-garbage -d` — deletes all old generations of all profiles in `/nix/var/nix/profiles` and collects garbage. "is a quick and easy way to clean up your system." [61]
  - `nix-store --gc` — run the garbage collector directly. [61]
  - `nix store gc` — modern CLI equivalent.
  - `nix-store --gc --print-dead` — preview what would be deleted. [61]
  - `nix-store --gc --print-live` — show paths that won't be deleted. [61]
  - `nix-env --delete-generations old` / `14d` / `10 11 14` — delete specific old generations. [61]
- GC behavior is affected by `keep-derivations` (default: true) and `keep-outputs` (default: false). [61]
- Project command: `just gc` runs `nix-collect-garbage --delete-old` followed by `nix store optimise` (dedupe). [usage]

## GC roots

- GC roots are the anchors that keep store paths alive. A path is collectable only if it is unreachable from all roots.
- Root locations:
  - `/nix/var/nix/gcroots/` — the canonical gcroots directory. Symlinks here pin closures.
  - `/nix/var/nix/gcroots/per-user/<user>/` — per-user gcroots. The project pins the devshell at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. [usage]
  - `/nix/var/nix/profiles/` — profile generations. Each profile generation is a root.
  - `result*` symlinks in the working directory — created by `nix build` (without `--no-link`). These pin closures forever until removed. [usage]
  - Live process temproots — a running `nix` process holds a "temproot" that prevents its in-use paths from being collected. [acc]
- Stale roots are a growth vector:
  > "Live `result*` symlinks, dev-shell profiles, and `nix build` outputs keep paths alive. A path that is unreachable is collectable; a path held by a live root is not. Stale `result*` symlinks pointing at long-gone paths, and accumulated dev-shell profiles, are the second growth vector." [purity]
- The devshell gcroot pin (Work Item 3): `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` keeps ~5–7 GB of toolchain permanently reachable so post-GC sessions do not re-fetch it. Rollback: `rm` the gcroot + `nix-collect-garbage -d`. Refresh rule: re-create the pin whenever `flake.lock` changes. [acc]
- Safe alternative to `result` symlinks: `--no-link --print-out-paths` (no symlink created). [usage]

## Store growth model

The store grows along three vectors. Understanding all three is necessary to keep the store bounded.

> "The store grows along three vectors. Understanding all three is necessary to keep the store bounded." [purity]

**(a) Per-edit churn**
> "Each evaluation with an impure source filter produces a new store path. With a dirty `target/` or `agents/*/build/` in the closure, every `nix develop` or `nix build` copies the whole closure again. This is the dominant growth vector during active development — it is what produced the 29 GB incident." [purity]

The mechanism: a dirty source closure changes hash on every edit, so Nix produces a new store path each time, and those paths are kept alive by live GC roots. [purity]

**(b) GC root accumulation**
> "Live `result*` symlinks, dev-shell profiles, and `nix build` outputs keep paths alive. ... Stale `result*` symlinks pointing at long-gone paths, and accumulated dev-shell profiles, are the second growth vector." [purity]

**(c) Content-addressed copies**
> "The same content under different store paths is duplicated until `nix store optimise` (or `auto-optimise-store`) deduplicates it. This is the third vector: even when content is identical, distinct paths consume space until deduplication runs." [purity]

## Store hygiene

- **`nix store optimise`** — hard-links identical files across store paths. Deduplicates content-addressed copies (vector (c)). [purity]
- **`auto-optimise-store`** — advisory setting that deduplicates identical content but is NOT a substitute for the purity rules. "It reduces vector (c) but does nothing for vectors (a) and (b)." [purity]
- **`nix-collect-garbage`** — collects unreachable paths (vectors (a) and (b)).
- Project hygiene cadence (from nix-purity.md):
  - "Run `just gc` regularly to collect unreachable store paths." [purity]
  - "Run `just store-audit` when the store feels large, to audit what is consuming space and find stale roots." [purity]
- `just gc` confirmed: runs `nix-collect-garbage --delete-old` + `nix store optimise`. [usage]
- Additional hygiene items from the accumulation report: dangling profile symlink (`~/.local/state/nix/profiles/profile` pointing to a deleted store path — fix with `nix profile remove` or recreate the symlink); zombie nix processes (harmless for GC but messy); consider `auto-optimise-store` config or a GC cron if accumulation continues. [acc]

## Store audit

- The project ships `scripts/store-audit.py`, invoked via `just store-audit`.
- What it does (verbatim from the script docstring):
  > "Reads `nix path-info --all --json` output from stdin and prints the top-20 store paths by closure size." [audit]
- With `--fail-if-source-over <MB>`: "the script additionally scans every path whose name contains `ai-workbench` AND ends with `-source` (the impure path-style copy probe). If any such path's closure size exceeds the given threshold (in MiB, 1 MB = 1_000_000 bytes), the oversized paths are printed to stderr and the script exits 1 — turning the previously passive probe into a blocking gate." [audit]
- Non-blocking on input problems: "on any read/parse failure it prints an informational note and exits 0 — the gate fails only on actual oversized source paths, never on missing/malformed input." [audit]
- Wired into `just verify` with `--fail-if-source-over 50` (V2 landed). [usage]
- `just store-delta-check` (V3) exists as a recipe but is NOT yet wired into `verify` — it is a periodic host/CI check that fails on new source-path copies exceeding the <50M criterion. [usage]
- Manual usage:
  ```bash
  nix path-info --all --json | python3 scripts/store-audit.py
  nix path-info --all --json | python3 scripts/store-audit.py --fail-if-source-over 50
  ```
- The `*-source` probe: non-empty `*-source` output referencing `ai-workbench` indicates an unbounded source copy that should be bounded by a `cleanSourceWith` filter. [purity]

## The 29 GB incident

- From nix-purity.md:
  > "An impure eval path previously caused Nix to copy `target/` and `agents/*/build/` into the store on every evaluation. Because those directories were large and changed on every edit, each evaluation produced a new content-addressed store path containing the full dirty source closure. The store grew at roughly 1 GB/min and reached approximately 29 GB before the impurity was caught and the source filter was fixed." [purity]
- From the accumulation report TL;DR:
  > "The Nix store kept refilling because opencode subagent sessions running inside this container repeatedly executed `nix eval --impure` and `nix develop` gates against the repo's flake. Each run copies the entire repo working tree (including multi-GB `agents/` and Rust `target/` directories) plus full Rust/LLVM/GCC toolchains into the append-only Nix store. We garbage-collected 7+ times across this session, freeing a cumulative ~76 GiB; each time the store refilled within hours. GC treats the symptom; the subagent-run gates are the cause." [acc]
- Root causes (ranked, from the report):
  1. Subagents ran HOST-NIX-gated evals (`nix eval --impure`, `nix develop`, `nix print-dev-env`) inside the container instead of on the host. Each copies toolchains (rustc, LLVM, GCC, Python — 5–7G) and the repo source into the store. [acc]
  2. Unfiltered repo source copies — `builtins.getFlake (toString ./.)` copies the repo working tree into the store. If `.git` is absent or eval bypasses git filtering, the full 16G `target/` and 2.6G `agents/opencode/repo/` get copied. [acc]
  3. Toolchain re-fetching per eval — each `nix eval` or `nix develop` against a GC'd store re-downloads the full Rust/LLVM/GCC/Python closure (~5–7G). [acc]
- Remediation (from the report's resolution note):
  - Impure gate recipes removed: the B1 regression was rewritten from `nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'` to the native git-filtered form `nix eval .#packages.x86_64-linux.pi-image.drvPath`. Store delta measured: 0 MB. [acc]
  - Devshell gcroot pinned (Work Item 3): `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. Post-GC `nix develop -c true` completed in 9.1s with zero re-fetch. [acc]
- Timeline table (from the report) — reproduce this table verbatim:

  | # | Store size before | Freed | Trigger identified |
  |---|---|---|---|
  | 1 | 14G / 5773 paths | 12.0 GiB / 5710 paths | Old accumulation (pre-session) |
  | 2 | 38G / 5900 paths | 8.5 GiB / 5622 paths | opencode subagent running B1/B2 `nix eval --impure` gates |
  | 3 | 35G / 64 paths | 32.9 GiB / 1 path | Single repo-source snapshot (28G Rust `target/debug` + 6.8G `agents/`) held alive by a stale temproot |
  | 4 | 9.7G / 5896 paths | 8.6 GiB / 5833 paths | Two flake evals with different nixpkgs revisions (duplicate LLVM) |
  | 5 | 7.5G / 5527 paths | 6.2 GiB / 2194 paths, then blocked | Live `nix develop -c cargo test` and `cargo clippy` pinning 4264 paths |
  | 6 | 4.9G / 4941 paths | 4.4 GiB / 4966 paths, then blocked | Live `nix eval --impure` gate + 2.0G build temp dir |
  | 7 | 2.9G / 4878 paths | 3.2 GiB / 4982 paths | Live `nix print-dev-env --json .` killed; GC then reached baseline |

  Cumulative freed across all cycles: ~76 GiB. [acc]

## Anti-accumulation patterns

This is the operational quick-reference for agents running nix tasks on this repo. The incident class it prevents: agents running impure evaluation gates that copy the raw working tree into the store on every invocation, accumulating 16–35G per eval until the disk fills. The git-tracked repo is ~735 files / ~12M; an impure eval can copy 16–28G of gitignored `target/` alone. [usage]

| # | Pattern | What it does | Safe alternative | Guard |
|---|---|---|---|---|
| a | Impure/path-mode flake eval: `nix eval --impure`, `builtins.getFlake (toString ./.)`, unfiltered `builtins.path { path = ./.; }` / `src = ./.` | Copies the raw working tree (gitignored `target/` 16–28G, `agents/*/build`, `.workestrate/`) into the store — up to ~35G per invocation; content-addressed so every dirty edit produces a new path | Native flake refs `nix eval .#attr` (git-filtered, ~12M) or filtered `builtins.path { path = ./subdir; filter = ...; }` | `just lint-nix` (checks 1–4); `just store-audit` flags `*-source` paths |
| b | GC-root leaks: `result` symlinks, `/tmp/*.tar.gz` out-links, `nix profile install` | Pins closures forever — unreachable paths survive GC | `--no-link --print-out-paths` (no `result` symlink); `just gc` to collect | `just store-audit` (flags `*-source` paths); manual inspection |
| c | `nix flake update` on rolling nixpkgs | Multi-GB rebuild — new nixpkgs revision pulls new toolchain/closure | Update deliberately (never in CI); run `just gc` after | Operational discipline (no static guard) |
| d | `buildLayeredImage` for new images | 0.5–1G tarballs materialized in the store | `streamLayeredImage` (streams to stdout, no store path) | Operational discipline (`docs/nix-purity.md` recipe patterns) |
| e | Per-edit churn: `nix develop` after edits | ~50–100MB per unique source state (content-addressed copies accumulate) | `auto-optimise-store` + `just gc` cadence | `just store-audit` (top-20 report); `just gc` |

[usage]

Agent rules (verbatim from the nix-usage skill):

1. **Regression/eval gates use ONLY native `.#` refs.** NEVER `--impure`. NEVER `toString ./.` or `getFlake` on this repo. NEVER `builtins.path` on the repo root without a `filter =` field.
2. **Stage new files (`git add -N`) before eval** — untracked files are invisible to `.#` refs.
3. **Clean up out-links** — remove `result*` symlinks and `/tmp/*.tar.gz` out-links when done; they pin closures forever.
4. **Run `just lint-nix` before committing** nix-adjacent changes.
5. **After `nix flake update`, run `just gc`** — a new nixpkgs revision pulls a multi-GB toolchain closure.
6. **The devshell toolchain is gcroot-pinned** at `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. If stale after a `flake.lock` change, re-pin with `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. Rollback: `rm` the gcroot + `nix-collect-garbage -d`.

[usage]

## Store inspection commands

- **`nix store prefetch-file <url>`** — downloads a file into the store and prints its store path and hash. Useful for verifying a fixed-output hash before writing a FOD. The project's `just update-hashes` recipe uses `nix run nixpkgs#prefetch-npm-deps` for npm dependency hashes. [purity]
- **`nix store ls <store-path>`** — lists the contents of a store path (like `ls` but operates on store paths, including remote/substitutable paths). Supports `--recursive` / `-R` for recursive listing and `--long` / `-l` for metadata.
- **`nix store cat <store-path>`** — prints the contents of a file inside a store path to stdout. Useful for inspecting a built file without linking it.
- **`nix path-info`** — the primary inspection command. `nix path-info --all --json` produces JSON with `closureSize`, `size`, `references`, `narSize`, and other fields per path. This is the data source for `scripts/store-audit.py`. [audit]
- **`nix store verify`** — verifies store path integrity against their registered NAR hashes.
- **`nix store diff-closures <path1> <path2>`** — shows the closure difference between two store paths (what was added/removed/up/downgraded).

## Content-addressed derivations

- Content-addressed derivations (CA derivations) are an experimental feature where the output path hash is computed from the content of the outputs rather than from the derivation inputs. This means two derivations that produce identical outputs get the same store path, enabling cross-machine reproducibility and deduplication without `nix store optimise`.
- In the current (input-addressed) model, store path digests are computed from the derivation's inputs. "Exactly how the digest is calculated depends on the type of store path. Store path digests are _supposed_ to be opaque." [60]
- CA derivations change this: the digest is computed from the output content, so identical outputs collide to the same path regardless of how they were built.
- This directly addresses vector (c) of the store-growth model (content-addressed copies): with CA derivations, identical content naturally deduplicates at store-path allocation time rather than requiring a separate `nix store optimise` pass. [purity]
- Status: experimental. Requires `experimental-features = ca-derivations` in nix.conf. Not used in the ai-workbench flake currently.
- Fixed-output derivations (FODs) are a special, already-stable case of content addressing: the output hash is declared in advance and the build is verified against it. FODs are how all dependency fetching works in a pure flake. [purity] The project uses `outputHashAlgo`, `outputHash`, `outputHashMode` for FODs. [purity]

## Citations

[60] [Nix Manual — Store Path](https://nix.dev/manual/nix/2.34/store/store-path) — crawl source: `.crawl/60-nix-store-path.md`
[61] [Nix Manual — Garbage Collection](https://nix.dev/manual/nix/2.34/package-management/garbage-collection) — crawl source: `.crawl/61-nix-garbage-collection.md`
[31] [Nix Guides — Troubleshooting](https://nix.dev/guides/troubleshooting.html) — crawl source: `.crawl/31-troubleshooting.md`
[purity] [Nix Purity](../nix-purity.md) — project doc
[acc] [Nix Store Accumulation Report](../nix-store-accumulation-report.md) — project doc
[usage] [Nix Usage Skill](../../.agents/skills/nix-usage/SKILL.md) — project skill
[audit] [store-audit.py](../../scripts/store-audit.py) — project script
