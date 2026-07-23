# Nix Store GC Remediation Spec

## Context

Opencode subagent sessions running inside the ai-workbench container repeatedly executed `nix eval --impure` regression-gate commands against the repo's flake. Each eval copied the raw working tree (including gitignored `target/` at 16-28G and `agents/*/repo/` at 2.6G) into the append-only Nix store, bypassing git source filtering. Over a single session, 7+ GC cycles freed ~76 GiB cumulative; the store refilled within hours each time. Full evidence, timeline, and root-cause analysis: `docs/nix-store-accumulation-report.md`.

**Root cause (one line):** regression-gate recipes using `nix eval --impure --expr 'builtins.getFlake (toString ./.)...'` copy the RAW working tree (git filtering bypassed) — observed snapshots up to 35G — while the git-tracked repo is only 735 files / ~12M.

**Note:** `nix.conf` `min-free`/`max-free` safety net already applied separately (not part of this spec).

## Work Item 1 — Rewrite impure getFlake gate recipes (REQUIRED)

### Locations

Verify with grep before editing (line numbers may drift):

- `docs/migration/80-remediation-plan.md` ~line 344-345 (B1): `nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'`
- `docs/migration/80-remediation-plan.md` ~lines 295-307 (B1/B2 variants incl. `.lib.config` type check)
- `docs/migration/40-migration-process.md` ~line 42 (`nix eval .#lib.config.workloadNames` — already pure form, verify only)

Also grep the entire repo for any other `getFlake.*toString` occurrences in docs, scripts, or agent instructions:
```sh
grep -rn 'getFlake.*toString\|toString \./\.' docs/ scripts/ .agents/ --include='*.md' --include='*.sh' --include='*.nix'
```

### Rewrite rule

`builtins.getFlake (toString ./.)` + `--impure` → native flake ref.

Example:
```
BEFORE: nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'
AFTER:  nix eval .#packages.x86_64-linux.pi-image.drvPath
```

### Constraints

1. **PRESERVE SEMANTICS:** each rewritten gate must evaluate the same attribute and return the same result type/value as the old form. Verify attribute paths against `flake.nix` outputs before rewriting (e.g. check whether `lib.config` exists at top level vs `lib.x86_64-linux.config` — do NOT blindly sed).
2. **Verify each rewritten command actually RUNS** and returns the expected value (run it). Paste output into the commit message.
3. Comply with `docs/nix-purity.md` (which already forbids this pattern at lines 29, 50, 64) and ensure `scripts/check-nix-paths.sh` passes. If `check-nix-paths.sh` does not currently scan `docs/`, note that as an optional hardening follow-up — do not block on it.
4. Add to each edited gate a one-line note: `# uses git-filtered flake ref; untracked files are invisible to eval — stage new files (git add / git add -N) before running`
5. Do not change what the gates TEST, only how the flake is referenced.

### Acceptance criteria

- All listed recipes rewritten and verified running.
- No remaining `getFlake (toString` occurrences in `docs/migration/` (grep proof):
  ```sh
  grep -rn 'getFlake.*toString' docs/migration/ && echo "FAIL: still present" || echo "PASS: clean"
  ```
- Store-delta test: run one rewritten B1 gate from a clean shell; measure `du -sh /nix/store` before/after; new source-path copies must be <50M (toolchain fetches excluded — note them separately if they occur).
- `docs/nix-purity.md` remains consistent (update it only if it references the old recipe form).

### Known tradeoffs

- `.#` refs on a git repo only see TRACKED files — newly created files must be staged before eval (classic flakes gotcha; error looks like missing file/attr).
- Semantics shift: gates now test the git-filtered view, which matches how remote consumers fetch the flake (git+file/github) — arguably more correct, but it is a behavior change vs the raw-tree view.
- If `.git` is ever absent (exported tarball), `.#` falls back to full-tree copy — acceptable in CI (small trees), noted for completeness.

## Work Item 3 — Pin the devshell toolchain closure (OPTIONAL, recommended)

### Problem

Nothing roots the Rust/LLVM/GCC closure (~5-7G), so every post-GC session re-fetches it. Each `nix develop` or `nix eval` against a GC'd store pays the full toolchain fetch cost again.

### Fix (exact steps)

1. Create a per-user gcroot for the devshell closure:
   ```sh
   nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
   ```
   If the devShell output is not directly buildable with nix 2.35.1, fall back to:
   ```sh
   nix print-dev-env . --profile /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
   ```
   If neither works, document why skipped.

2. Verify GC survival:
   ```sh
   nix-collect-garbage -d
   du -sh /nix/store  # should retain ~5-7G (the pinned toolchain closure)
   nix develop -c true  # should work without re-fetching
   ```

3. **Refresh rule:** re-run step 1 whenever `flake.lock` changes (fenix/nixpkgs bumps). Otherwise the pinned closure goes stale and agents fetch the new one anyway (no breakage, just no benefit).

4. **Rollback:**
   ```sh
   rm /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
   nix-collect-garbage -d
   ```

### Tradeoffs

- Permanent ~5-7G pinned in the store (acceptable — prevents repeated re-fetching).
- Staleness risk if not refreshed after `flake.lock` updates (no breakage, just no benefit).

## Out of scope

- `nix.conf` `min-free`/`max-free` (already applied).
- Blocking or limiting agent nix usage (considered and rejected — fixes must keep agents fully working).
- Non-nix disk usage investigation (separate concern; disk is ~96% full with non-nix data).

## Validation plan for the implementing agent

- Run every edited gate once, capture output, paste into PR/commit message.
- Grep proof: no `getFlake (toString` in `docs/`.
- `scripts/check-nix-paths.sh` output.
- Store-delta measurement for Work Item 1 acceptance criterion.
- For Work Item 3 (if done): GC-survival proof + `nix develop -c true` timing.
