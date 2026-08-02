---
name: nix-store-gc
description: |
  Operational guide for Nix store hygiene and garbage collection — GC roots,
  nix-collect-garbage -d, nix store optimise, the three-vector store-growth
  model, anti-accumulation patterns, store-audit tooling, and the devshell
  gcroot pin. Load when diagnosing store growth, running GC, auditing the store,
  or preventing accumulation. Distilled from docs/nix/store-hygiene-and-gc.md
  and docs/nix/nix-store-and-paths.md; consult those docs for full detail.
---

# Nix Store Hygiene and Garbage Collection

Distilled from [`docs/nix/store-hygiene-and-gc.md`](../../../docs/nix/store-hygiene-and-gc.md).
That doc holds the canonical rules, the 29 GB incident timeline, and upstream
source links; this skill is the actionable subset.

## Triggers

Load this skill when:

- The Nix store grows unexpectedly or disk is filling.
- Running garbage collection (`nix-collect-garbage`, `nix store gc`).
- Auditing the store (`just store-audit`, `nix path-info`).
- Diagnosing accumulation (impure evals, stale roots, toolchain churn).
- Managing GC roots (result symlinks, profiles, devshell pin).
- After `nix flake update` (new nixpkgs revision pulls a multi-GB closure).
- Cleaning up `result*` symlinks and out-links.

## How the Store Works

- The Nix store lives at `/nix/store`. It is content-addressed, append-only,
  and immutable — every eval/build copies inputs as new paths; nothing is
  overwritten.
- Source-path copying: when a flake is evaluated, Nix copies the repo working
  tree into the store. If `.git` exists, it respects `.gitignore`; otherwise it
  copies everything.
- Project baseline: ~63 paths / 114 MB (nix itself + flake registry). Everything
  above that is transient eval/build output.

## GC Roots

GC roots are the anchors that keep store paths alive. A path is collectable
only if unreachable from all roots.

- `/nix/var/nix/gcroots/` — canonical gcroots directory.
- `/nix/var/nix/gcroots/per-user/<user>/` — per-user gcroots (the devshell pin lives here).
- `/nix/var/nix/profiles/` — profile generations (each generation is a root).
- `result*` symlinks in the working directory — created by `nix build` without `--no-link`; pin closures forever until removed.
- Live process temproots — a running nix process holds a temproot preventing collection of in-use paths.

Safe alternative to `result` symlinks: `--no-link --print-out-paths` (no symlink created).

## GC Commands

| Command | Purpose |
|---|---|
| `nix-collect-garbage -d` | Delete all old profile generations + collect garbage. |
| `nix store gc` | Modern CLI garbage collector. |
| `nix-store --gc` | Legacy direct GC invocation. |
| `nix-store --gc --print-dead` | Preview what would be deleted. |
| `nix-store --gc --print-live` | Show paths that won't be deleted. |
| `nix-env --delete-generations old` | Delete all old (non-current) generations. |
| `nix-env --delete-generations 14d` | Delete generations older than 14 days. |

Old generations must be deleted first for GC to be effective — they keep
packages alive and prevent rollback-free collection.

## Store Optimisation

- `nix store optimise` — hard-links identical files across store paths,
  deduplicating content-addressed copies (growth vector (c)).
- `auto-optimise-store` — advisory setting that deduplicates identical content
  but is NOT a substitute for the purity rules. It reduces vector (c) but does
  nothing for vectors (a) and (b).

## The Three Growth Vectors

| Vector | Mechanism | Dominant when |
|---|---|---|
| (a) Per-edit churn | Impure source filter produces a new store path each eval; dirty `target/` copied each time. | Active development with impure evals (the 29 GB incident). |
| (b) GC root accumulation | `result*` symlinks, dev-shell profiles, `nix build` outputs pin closures. | Stale symlinks + accumulated profiles. |
| (c) Content-addressed copies | Same content under different paths duplicated until `nix store optimise`. | Multiple evals producing identical content. |

## Anti-Accumulation Patterns

| # | Pattern | What it does | Safe alternative | Guard |
|---|---|---|---|---|
| a | Impure/path-mode eval (`nix eval --impure`, `builtins.getFlake (toString ./.)`, unfiltered `builtins.path`) | Copies raw working tree (16–35G) into the store per invocation | Native `.#` refs (git-filtered, ~12M) or filtered `builtins.path { filter = ...; }` | `just lint-nix`; `just store-audit` flags `*-source` paths |
| b | GC-root leaks (`result` symlinks, `/tmp/*.tar.gz` out-links, `nix profile install`) | Pins closures forever — unreachable paths survive GC | `--no-link --print-out-paths`; `just gc` | `just store-audit`; manual inspection |
| c | `nix flake update` on rolling nixpkgs | Multi-GB rebuild — new revision pulls new toolchain | Update deliberately (never in CI); run `just gc` after | Operational discipline |
| d | `buildLayeredImage` for new images | 0.5–1G tarballs materialized in the store | `streamLayeredImage` (streams to stdout) | Operational discipline |
| e | Per-edit churn (`nix develop` after edits) | ~50–100MB per unique source state | `auto-optimise-store` + `just gc` cadence | `just store-audit`; `just gc` |

## Store Audit Tooling

- `just store-audit` (`scripts/store-audit.py`) — reads `nix path-info --all
  --json`, prints the top-20 store paths by closure size, and fails (exit 1)
  if any `*ai-workbench*-source` path exceeds 50 MB closure size (the impure
  path-style copy probe). Non-blocking when nix/python3 unavailable.
- Wired into `just verify` as the final step (V2 landed).
- `just store-delta-check` (V3) exists as a recipe but is NOT yet wired into
  `verify` — it is a periodic host/CI check measuring `/nix/store` growth from
  one pure eval.

## Project Hygiene Recipes

- `just gc` — runs `nix-collect-garbage --delete-old` + `nix store optimise`
  (dedupe). Reclaims unreachable paths and deduplicates content-addressed copies.
- Devshell gcroot pin — the toolchain is pinned at
  `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` (~5–7 GB), keeping
  Rust/LLVM/GCC reachable so post-GC sessions don't re-fetch. Re-pin after
  `flake.lock` changes; rollback is `rm` the gcroot + `nix-collect-garbage -d`.
- Disk-pressure tradeoff — the pin consumes ~a quarter to a third of
  currently-free space; accepted because without it every post-GC session
  re-downloads the same 5–7G anyway.

## Store Inspection Commands

| Command | Purpose |
|---|---|
| `nix path-info --all --json` | Primary inspection; JSON with closureSize, references, etc. |
| `nix path-info --recursive <path>` | Print the closure (transitive references). |
| `nix path-info --closure-size <path>` | Print total closure disk usage. |
| `nix store ls <store-path>` | List contents of a store path. |
| `nix store cat <store-path>` | Print a file inside a store path to stdout. |
| `nix store verify` | Verify store path integrity against NAR hashes. |
| `nix store diff-closures <p1> <p2>` | Show closure difference between two paths. |
| `nix-store -qR <path>` | Legacy equivalent of `nix path-info --recursive`. |

## Practical Rules

1. Run `just gc` regularly — the primary hygiene cadence.
2. Run `just store-audit` when the store feels large — top-20 report + source-path gate.
3. Clean up `result*` symlinks and `/tmp/*.tar.gz` out-links when done — they pin closures forever.
4. Delete old profile generations (`nix-env --delete-generations old`) before GC — old generations keep packages alive.
5. Keep the devshell gcroot pin fresh — re-pin after `flake.lock` changes.
6. Never run `nix eval --impure` or `builtins.getFlake (toString ./.)` on this repo — use native `.#` refs.
7. Run `just gc` after `nix flake update` — a new nixpkgs revision pulls a multi-GB closure.
8. Use `streamLayeredImage` over `buildLayeredImage` — streams tarballs without materializing a store path.
9. Relocate `CARGO_TARGET_DIR` out of the source tree (canonical: `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`).
10. Preview with `nix-store --gc --print-dead` before collecting.

## Review Checklist

- [ ] `just gc` run regularly (or after `nix flake update`).
- [ ] `just store-audit` passes (no `*-source` path over 50 MB).
- [ ] No stale `result*` symlinks or `/tmp/*.tar.gz` out-links.
- [ ] Old profile generations deleted before GC.
- [ ] Devshell gcroot pin is fresh (re-pinned after `flake.lock` changes).
- [ ] No `nix eval --impure` / `builtins.getFlake (toString ./.)` in the repo.
- [ ] `CARGO_TARGET_DIR` relocated out of the flake-visible source tree.
- [ ] `auto-optimise-store` enabled or `nix store optimise` run periodically.

## Validation Commands

> **HOST-GATE:** This container has no nix. The commands below are documented
> Nix semantics, not runtime-verified in this environment.

```bash
just gc                                              # nix-collect-garbage --delete-old + nix store optimise
just store-audit                                     # top-20 + *-source gate (50 MB)
nix path-info --all --json | python3 scripts/store-audit.py
nix-store --gc --print-dead                          # preview what GC would delete
nix store optimise                                   # dedupe identical content
```

## Common Mistakes

1. Running `nix eval --impure` — copies the raw working tree (16–35G) into the store per eval.
2. Leaving `result*` symlinks around — they pin closures forever and survive GC.
3. Not deleting old profile generations before GC — packages stay alive via old generations.
4. Letting the devshell gcroot pin go stale after `flake.lock` changes — agents re-fetch the new toolchain anyway.
5. Using `buildLayeredImage` instead of `streamLayeredImage` — materializes 0.5–1G tarballs in the store.
6. Running `nix flake update` in CI — pulls a multi-GB toolchain closure; update deliberately only.
7. Expecting `auto-optimise-store` to substitute for purity — it only dedupes vector (c), not (a) or (b).
8. Not running `just gc` after `nix flake update` — the old closure lingers unreachable but uncollected.

## Related Docs

- Full reference: [`docs/nix/store-hygiene-and-gc.md`](../../../docs/nix/store-hygiene-and-gc.md) (canonical; 29 GB incident timeline there).
- [`docs/nix/nix-store-and-paths.md`](../../../docs/nix/nix-store-and-paths.md) — store layout, path format, closure semantics.
- [`docs/nix-purity.md`](../../../docs/nix-purity.md) — purity rules, `just lint-nix`, source filters, store-growth model.

## Related Skills

- [`nix-usage`](../nix-usage/SKILL.md) — project flake, dev shell, anti-accumulation agent rules.
- [`nix-ci-cd`](../nix-ci-cd/SKILL.md) — `just store-audit` / `store-delta-check` in CI, HOST-NIX gates.
