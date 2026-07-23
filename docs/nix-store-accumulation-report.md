# Nix Store Accumulation — What Happened and What Needs Doing

## TL;DR

The Nix store kept refilling because opencode subagent sessions running inside this container repeatedly executed `nix eval --impure` and `nix develop` gates against the repo's flake. Each run copies the entire repo working tree (including multi-GB `agents/` and Rust `target/` directories) plus full Rust/LLVM/GCC toolchains into the append-only Nix store. We garbage-collected 7+ times across this session, freeing a cumulative ~76 GiB; each time the store refilled within hours. GC treats the symptom; the subagent-run gates are the cause.

## How the Nix store works (60-second version)

- **Append-only cache.** The Nix store lives at `/nix/store`. Every flake evaluation or build copies its inputs (nixpkgs, toolchains, your repo source) into the store as new, immutable paths. Nothing is ever overwritten — each new eval creates new paths.
- **`builtins.getFlake (toString ./.)` copies your repo.** When a flake is evaluated, Nix copies the repo working tree into the store as a "source path." If a `.git` directory exists, Nix respects `.gitignore` and excludes gitignored files from the copy. If `.git` is absent, it copies everything.
- **Garbage collection deletes unreachable paths.** `nix-collect-garbage -d` deletes everything not reachable from a "GC root" (a symlink in `/nix/var/nix/gcroots/` or a live process's "temproot"). It **cannot** delete anything a live process is actively using — this is why GC sometimes frees less than expected.
- **Baseline for this machine:** 63 paths / 114M. This is nix itself plus the flake registry. Everything above that is transient eval/build output.

## Timeline of what we observed

All times approximate (Jul 22–23, 2025). "Trigger" = what spawned the store paths.

| # | Store size before | Freed | Trigger identified |
|---|---|---|---|
| 1 | 14G / 5773 paths | 12.0 GiB / 5710 paths | Old accumulation (pre-session) |
| 2 | 38G / 5900 paths | 8.5 GiB / 5622 paths | opencode subagent running B1/B2 `nix eval --impure` gates from `docs/migration/80-remediation-plan.md` |
| 3 | 35G / 64 paths | 32.9 GiB / 1 path | Single repo-source snapshot (28G Rust `target/debug` + 6.8G `agents/`) held alive by a stale temproot from a dead PID |
| 4 | 9.7G / 5896 paths | 8.6 GiB / 5833 paths | Two flake evals with different nixpkgs revisions (rustc 1.95.0 + 1.96.1, duplicate LLVM 21.1.8) |
| 5 | 7.5G / 5527 paths | 6.2 GiB / 2194 paths, then blocked | Live `nix develop -c cargo test` and `nix develop -c cargo clippy` pinning 4264 paths |
| 6 | 4.9G / 4941 paths | 4.4 GiB / 4966 paths, then blocked | Live `nix eval --impure` gate + 2.0G build temp dir (`tmp-1794066-*`) |
| 7 | 2.9G / 4878 paths | 3.2 GiB / 4982 paths | Live `nix print-dev-env --json .` (PID 1795515) killed on user order; GC then reached baseline |

**Cumulative freed across all cycles: ~76 GiB.**

## Root causes (ranked)

### 1. Subagents run HOST-NIX-gated evals inside the container

The remediation plan (`docs/migration/80-remediation-plan.md`, lines 344–353) explicitly marks the B1, B2, and B14 regression tests as **HOST-NIX gates** — meaning they should run on the host machine, not inside this container. Despite this, opencode subagent sessions repeatedly executed them here.

Traced process chains:
- `high_agency_orchestrator` ("Workestrator config-driven split assessment") → `lead_short` ("Verify WP1-WP5 merge gate") → `worker_general` ("Run final-state cargo/nix gates") — ran B1/B2 `nix eval --impure` expressions verbatim from the remediation plan.
- A later chain ran `nix develop -c cargo test` and `nix develop -c cargo clippy` inside a dev shell, creating toolchain + crate paths.
- The latest was `nix print-dev-env --json .` (parent: opencode PID 1775978), a dev-shell probe.

Each of these copies toolchains (rustc, LLVM, GCC, Python — 5–7G) and the repo source into the store.

### 2. Unfiltered repo source copies

When `builtins.getFlake (toString ./.)` evaluates, it copies the repo working tree into the store. The current `.gitignore` **does** exclude `target/` (lines 25–26: `target/` and `**/target/`) and `agents/*/repo` + `agents/*/build` (lines 18–20). However:

- `agents/opencode/repo` (2.6G) is gitignored — good.
- `control/agentctl/target/` (16G) is gitignored via `control/agentctl/.gitignore` line 5 — good.
- But the `agents/` directory itself (the top-level `agents/opencode/`, `agents/pi/`, etc.) is **not** gitignored — only `agents/*/repo` and `agents/*/build` subpaths are. The non-repo, non-build contents of `agents/` (if any) would be copied. In practice, the large items are under `repo/` and `build/`, so the gitignore is mostly effective.

**Open check:** verify that `builtins.getFlake` is actually respecting `.gitignore` during eval. If `.git` is somehow absent or the eval uses `--impure` with a path that bypasses git filtering, the full 16G `target/` and 2.6G `agents/opencode/repo/` would be copied. The 35G snapshot in cycle 3 suggests this happened at least once.

### 3. Toolchain re-fetching per eval

Each `nix eval` or `nix develop` against a GC'd store re-downloads and instantiates the full Rust/LLVM/GCC/Python closure (~5–7G). Since GC runs between subagent sessions, every new session starts from a clean store and pays the full fetch cost again.

## What needs doing (action list)

Ranked by impact. Each item has an owner, effort estimate, and expected impact.

### A. Stop running HOST-NIX gates in the container [Prevention — biggest lever]

**Problem:** Subagents execute `nix eval --impure` / `nix develop` / `nix print-dev-env` gates that the remediation plan explicitly defers to the host.

**Fix:** Update the prompts, skills, or agent instructions that lead subagents to run these gates locally. Options:
- Add a guard that refuses `nix eval --impure` / `nix develop` / `nix print-dev-env` when running in the container environment (detect via an env var like `OPENCODE=1` or the absence of a host marker).
- Update the remediation plan's gate descriptions to explicitly say "DO NOT run in the opencode container — host only" in language a subagent will respect.
- Fix the specific subagent prompts that reference B1/B2/B14 regression recipes to mark them as host-only.

**Owner:** Agent/prompt configuration. **Effort:** 1–2 hours. **Impact:** Eliminates the root cause — store stops refilling.

### B. Ensure repo source filtering is effective [Prevention — secondary]

**Problem:** If `.gitignore` filtering is bypassed during `--impure` evals, the full repo (including 16G `target/` and 2.6G `agents/opencode/repo/`) gets copied into the store.

**Fix:**
- Verify `builtins.getFlake` respects `.gitignore` during `--impure` evals (it should, since `.git` exists).
- If not reliable, add an explicit `builtins.path { path = ./.; filter = ...; }` in the flake to bound the source copy — similar to the B14 fix in `nix/lib/config.nix` that filters to only `workestrate.toml`.
- Consider adding `control/agentctl/target/` to the root `.gitignore` (currently only in `control/agentctl/.gitignore`) for defense in depth.

**Owner:** Nix flake config. **Effort:** 1 hour. **Impact:** Per-eval source copy drops from ~3–35G to ~100M if filtering works; prevents the 35G snapshot scenario.

### C. Nix hygiene [Optional cleanup]

- **Dangling profile symlink:** `~/.local/state/nix/profiles/profile` points to a deleted store path. Not causing accumulation, but should be fixed (`nix profile remove` or recreate the symlink).
- **Zombie processes:** Dead `nix` processes linger as zombies (`[nix] <defunct>`) because their parent (opencode) doesn't reap them. Harmless for GC, but messy.
- **`nix store optimise`:** Hard-links identical files across store paths. Could save a few hundred MB if duplicate toolchain paths coexist. Not urgent.
- **Automatic GC:** Consider adding a `nix-collect-garbage` cron or a `nix.settings.auto-optimise-store` config if this pattern continues.

**Owner:** Environment config. **Effort:** 30 min. **Impact:** Minor hygiene; doesn't address the root cause.

### D. Non-Nix disk usage [Separate concern]

Disk is at ~96% full (416G used / 17G avail on a 456G overlay). The Nix store is now only 114M. The remaining ~416G is non-Nix data — likely repo working trees, build artifacts outside Nix (`~/.cache/ai-workbench/`), Docker images, or other large files. This warrants its own investigation separate from the Nix store issue.

## Current state

- **Store:** 63 paths / 114M (clean baseline). No active nix processes.
- **Opencode:** PID 1775978 is still running and may spawn more gates. If it does, the store will re-accumulate.
- **Consequence accepted:** One in-flight gate (`nix print-dev-env --json .`, PID 1795515) was killed on user order. Its parent session will see that step fail/abort.

## Evidence pointers

- **Remediation plan:** `docs/migration/80-remediation-plan.md` lines 344–353 (B1/B2/B14 regression recipes, marked HOST-NIX gate), line 65 (WP2 status: "nix eval regressions deferred to host per env-honesty policy"), lines 76–84 (disk-pressure note acknowledging the probe artifact).
- **Opencode log sessions:** `ses_0855e7a4effe2qdA5C0w2VgvDR` (lead_short "Verify WP1-WP5 merge gate"), `ses_0855e06aeffeLV1r71Xyjm1JuG` (worker_general "Run final-state cargo/nix gates"). Log at `/home/node/.local/share/opencode/log/opencode.log`.
- **Gitignore:** `.gitignore` lines 18–20 (`agents/*/repo`, `agents/*/build`), lines 25–26 (`target/`, `**/target/`), `control/agentctl/.gitignore` line 5 (`/target`).
- **B14 fix:** `nix/lib/config.nix` — `builtins.path { path = ./../config.reference; filter = ...; }` bounding the config copy.
- **All GC/df/du outputs:** Captured in this session's command history.

## Resolution note — nix-store GC remediation (2026-07-23, branch migration/tool-model)

Work Items 1 and 3 of `docs/migration/nix-store-gc-remediation-spec.md` executed with orchestrator corrections:

- **Impure gate recipes removed.** The one true impure gate (`80-remediation-plan.md` B1 regression, old line 349-351) was rewritten from `nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'` to the native git-filtered form `nix eval .#packages.x86_64-linux.pi-image.drvPath`. Verified running: returns `"/nix/store/caql1kmknlqdw4nbr1lcs627g3r7dsfh-workestrator-pi.tar.gz.drv"`. Store delta measured across the run: **0 MB** (5678 MB before/after) — the git-filtered `.#` ref no longer copies the raw working tree.
- **Stale `.#lib.config.*` references corrected.** `lib.x86_64-linux` exposed no `config` attr (attrs were `[buildImagesFromConfig, buildWorkloadImage, checks, recipes, vocabulary]`). Fixed by adding `config = referenceConfig;` to the `libForSystem` attrset in `flake.nix` (least-invasive: reuses the existing let-bound `referenceConfig`, no new top-level output). Gates `nix eval .#lib.x86_64-linux.config.workloadNames` / `.nixLayeredImages` / `.localBuilds` verified: `[ "example-agent" "example-offensive" "example-service" ]`, `[ "example-agent" "example-offensive" ]`, `[ "example-agent" "example-offensive" ]`. Doc references at `40-migration-process.md:42` and `80-remediation-plan.md:361` updated to the corrected path with a note that the flake export is required.
- **B6 design note (old line 723) annotated:** `builtins.getFlake` (impure) marked as considered-and-rejected in favor of flake inputs, per `docs/nix-purity.md` rule 6. Historical narrative at line 84 left untouched.
- **Devshell gcroot (Work Item 3): DONE.** `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` succeeded directly (no print-dev-env fallback needed). Pin closure = 2.9 GiB. GC-survival verified: `nix-collect-garbage -d` freed 1.9 GiB of unrooted paths, store settled at 3.3G with the toolchain retained, and post-GC `nix develop -c true` completed in 9.1s with **zero re-fetch**.

### Disk-pressure tradeoff of the devshell pin (Work Item 3)

Pinning `.#devShells.x86_64-linux.default` (or `nix print-dev-env` closure) keeps ~5-7G of Rust/LLVM/GCC toolchain permanently reachable so post-GC sessions do not re-fetch it. Tradeoff: the overlay is ~95-96% full with **non-nix** data (411G used / 22G avail of 456G at time of writing); a permanent 5-7G pin consumes roughly a quarter to a third of the currently-free space. Accepted because: (a) without the pin every post-GC session re-downloads the same 5-7G anyway (transient, but repeatedly); (b) the pin is the only mechanism that makes `nix develop` survivable across `nix-collect-garbage -d`. **Rollback:**

```sh
rm /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
nix-collect-garbage -d
```

**Refresh rule:** re-create the pin whenever `flake.lock` changes (fenix/nixpkgs bumps), else the pinned closure goes stale and agents fetch the new toolchain anyway (no breakage, just no benefit).
