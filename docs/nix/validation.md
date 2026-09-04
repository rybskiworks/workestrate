---
type: Reference
resource: https://nix.dev/reference/nix-manual.html
title: Validation
description: Nix validation gates — nix flake check, nix build, nix eval, nix flake show, the project's just verify / verify-full pipeline, lint-nix purity checks, store-audit, and CI integration.
tags: [nix, validation, nix-flake-check, linting, gates]
timestamp: 2026-07-24T02:00:00Z
---

# Validation

## Purpose

This document covers Nix validation gates: `nix flake check` (the primary
validation command), `nix build`, `nix eval`, `nix flake show`, and the
project's validation pipeline (`just verify`, `just verify-full`,
`just lint-nix`, `just store-audit`, `just gc`).

It is both a generic Nix reference and a project-specific (ai-workbench)
guide. Agents should use this as the authoritative reference when running or
wiring validation gates.

## Sources used

- https://nix.dev/concepts/flakes.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake-check.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-eval.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake-show.html
- https://nix.dev/guides/recipes/continuous-integration-github-actions.html
- Local: `/justfile` — `verify`, `verify-full`, `lint-nix`, `store-audit`, `store-delta-check`, `gc` recipes
- Local: `/scripts/check-nix-paths.sh` — the purity enforcement guard (6 checks)
- Local: `/scripts/store-audit.py` — store audit script (top-20 + blocking gate)
- Local: `/docs/nix-purity.md` — enforcement context, purity rules, store-growth model
- Local: `/flake.nix` — `checks.validateConfig`, `.#workestrate` output
- Local: `/.agents/skills/nix-usage/SKILL.md` — guard inventory, anti-accumulation patterns

## Core guidance

### The validation hierarchy

| Level | Mechanism | What it validates | Cost |
| --- | --- | --- | --- |
| Flake | `nix flake check` | Evaluates all outputs; builds `checks` derivations | Low–medium |
| Package | `nix build .#<name>` | Builds a single flake output | Medium |
| Project pipeline | `just verify` / `just verify-full` | lint-nix + store-audit + cargo gates + nix build | Medium–high |

### nix flake check

`nix flake check` is the primary validation command. It does two things:

1. Evaluates every output in the flake to confirm the attribute tree is
   well-formed.
2. Builds every derivation in `checks.<system>`.

From the flakes concept page [1]: "Nix checks `flake.nix`'s structure is
valid."

Core flake commands:

| command | purpose |
| --- | --- |
| `nix flake check` | validate flake structure and run `checks` |
| `nix flake show` | show all outputs (attribute tree) |

Usage:

```sh
nix flake check
nix flake check --no-build   # evaluate only, skip building checks
```

> **HOST-GATE:** `nix flake check` requires Nix on the host. This container
> has no nix; the command semantics are documented, not runtime-verified in
> this environment.

The project's `checks.validateConfig` from `flake.nix` (lines 140-154):

```nix
checks.validateConfig = { pkgs, config, workestrate }:
  let
    configFile = pkgs.writeText "workestrate.toml" config;
  in
  pkgs.runCommand "validate-config" {
    nativeBuildInputs = [ workestrate ];
    passAsFile = [ ];
  } ''
    mkdir -p $out
    cp ${configFile} workestrate.toml
    workestrate validate-config
    touch $out/ok
  '';
```

### nix build .#<name>

Build individual outputs. The project's primary build target is
`.#workestrate`:

```sh
nix build .#workestrate
./result/bin/workestrate --version
```

Use `--no-link` to avoid creating a `result` symlink (GC-root hygiene), and
`--print-out-paths` to print the store path:

```sh
nix build .#workestrate --no-link --print-out-paths
```

There is NO `packages.default` — use `.#workestrate`. From the nix-usage
SKILL.md: "There is NO `packages.default`, NO `nix fmt`, and NO
`nix run .#default`."

### nix build .#checks.x86_64-linux.<name>

Run individual checks:

```sh
nix build .#checks.x86_64-linux.validateConfig
```

This builds (runs) a single check derivation without running all checks.

### nix eval .#<attr>

Evaluate attributes without building:

```sh
nix eval .#workestrate.meta.description
nix eval .#packages.x86_64-linux.workestrate.drvPath
```

NEVER use `--impure` — the project's lint-nix guard forbids it. The
git-filtered `.#` ref form is pure and does not copy the working tree.

### nix flake show

List all flake outputs as an attribute tree:

```sh
nix flake show
nix flake show --json   # machine-readable
```

### The project validation pipeline

The justfile recipes compose the project's validation pipeline:

| Recipe | What it runs | Wired into verify? |
| --- | --- | --- |
| `just toolchain-check` | Verifies host rustc matches flake-pinned toolchain | Yes (first gate) |
| `just check` | cargo fmt --check + clippy -D warnings + cargo check | Yes |
| `just test` | cargo test | Yes |
| `just spec-examples` | TOML spec example parsing tests | Yes |
| `just litellm-check` | LiteLLM config.yaml schema validation | Yes |
| `just golden-check` | Golden plan file parity | Yes |
| `just schema-check` | JSON Schema drift guard | Yes |
| `just scaffold-check` | Scaffold template parity | Yes |
| `just lint-nix` | check-nix-paths.sh (6 purity checks) | Yes |
| `just store-audit` | store-audit.py (top-20 + blocking source-path gate) | Yes |
| `just verify` | All of the above + Cargo.lock stability | — |
| `just verify-full` | verify + `nix build .#workestrate` | — |
| `just gc` | nix-collect-garbage --delete-old + nix store optimise | No (maintenance) |
| `just store-delta-check` | /nix/store growth from one pure eval | No (periodic) |

From the justfile (lines 71-76):

```make
# Full pre-merge validation: format, lint, compile-check, test, spec-examples,
# config validation, golden-check, schema drift, lock-file stability, AND
# nix-purity lint.
verify: toolchain-check check test spec-examples litellm-check golden-check schema-check scaffold-check lint-nix store-audit
    git diff --exit-code HEAD -- control/agentctl/Cargo.lock

# Heaviest validation: verify plus Nix build
verify-full: verify
    nix build .#workestrate
```

### check-nix-paths.sh — the 6 purity checks

This is the active enforcement guard wired into `just lint-nix` /
`just verify`. Header comment from `scripts/check-nix-paths.sh` (lines 2-19):

```bash
# check-nix-paths.sh — active enforcement of the nix-purity invariants.
#
# Fails on:
#   (1) `nix ... --impure` invocations in shell scripts and nix code.
#       Comments and this linter's own source are skipped.
#   (2) `builtins.getFlake ... toString ...` — impure getFlake with a runtime
#       path string. Use a flake input instead.
#   (3) `builtins.path { ... }` without a `filter =` field — unbounded path
#       copy into the store (closes the B14 class of disk-exhausting evals).
#   (4) `cleanSourceWith { ... }` without a `filter =` field — same problem
#       via the lib helper.
#   (5) Bare repo-root path literals in nix code (outside `src =`/`lockFile =`
#       fields, which are the bounded escape hatches).
#   (6) Impure-pattern references in docs/**/*.md (including docs/migration/):
#       `getFlake ... toString`, `nix eval --impure <arg>`, `toString ./.`.
```

The 6 checks:

| Check | Pattern scanned | Files scanned | Safe alternative |
| --- | --- | --- | --- |
| 1 | `nix ... --impure`, `nix-shell ... --impure` | *.nix, *.sh, justfile | Use flake inputs or cleanSourceWith |
| 2 | `builtins.getFlake` + `toString` on same line | *.nix | Use a flake input |
| 3 | `builtins.path { ... }` without `filter =` in next 15 lines | *.nix | Add explicit `filter =` predicate |
| 4 | `cleanSourceWith { ... }` without `filter =` in next 15 lines | *.nix | Add explicit `filter =` predicate |
| 5 | `../` path literals in nix assignments outside `src =`/`lockFile =`/`path =` | *.nix | Use cleanSourceWith + src, or a flake input |
| 6 | `getFlake ... toString`, `nix eval --impure <arg>`, `toString ./.` in docs | docs/**/*.md | Use the git-filtered `.#` ref form |

Note: Check 5 does NOT catch bare `src = ./.` — rule 1 in `docs/nix-purity.md`
remains authoritative even when the guard passes. The guard is a static
heuristic, not a full eval-purity prover.

### Allowlists

Two allowlist mechanisms:

1. **Per-line**: lines matching `# allow: <reason>` are skipped in every
   scanned file type. From `check-nix-paths.sh` (lines 86-88):

```bash
is_allowlisted() {
    case "$1" in *"# allow:"*) return 0 ;; esac
    return 1
}
```

2. **File-level (docs only)**: `DOCS_ALLOWLIST` array names whole documents
   where impure patterns legitimately appear in narrative prose. From
   `check-nix-paths.sh` (lines 55-59):

```bash
DOCS_ALLOWLIST=(
    "docs/nix-purity.md"
    "docs/nix-store-accumulation-report.md"
    "docs/migration/nix-store-gc-remediation-spec.md"
)
```

Add a file here ONLY when the document's purpose is to discuss/forbid the
pattern, not to invoke it.

### store-audit.py — store growth detection

The script reads `nix path-info --all --json` from stdin and:

1. Prints the top-20 store paths by closure size (informational report).
2. With `--warn-if-source-over <MB>`, scans for attributable local
   path-style flake-input copies (basename `<hash>-(workestrate|personal|
   duelbits|nix-tooling)[-source]`). Any match over the threshold prints a
   WARN to stderr; the flag is informational and the script always exits 0.

Docstring from `scripts/store-audit.py` (lines 2-16):

```python
"""store-audit report — extracted from the justfile `store-audit` recipe.

Reads `nix path-info --all --json` output from stdin and prints the top-20
store paths by closure size.

With ``--warn-if-source-over <MB>`` the script additionally scans for
attributable LOCAL flake-input copies: paths whose basename is a 32-char
store hash plus one of the known local input names (workestrate, personal,
duelbits, nix-tooling), optionally suffixed ``-source`` — i.e. matching
``^[a-z0-9]{32}-(workestrate|personal|duelbits|nix-tooling)(-source)?$``.
Those names only appear when an input was declared ``path:``-style (nix
names such copies after the source basename), so a match IS attributable to
a local working copy. Any match whose closure size exceeds the threshold
(in MiB, 1 MB = 1_000_000 bytes) is printed to stderr as a WARN — the flag
is INFORMATIONAL and the script always exits 0.
"""
```

The justfile recipe invocation (line 291):

```bash
echo "$path_info" | python3 scripts/store-audit.py --warn-if-source-over 50
```

The threshold is 50 MB. The design is non-blocking: the source-path scan is
informational (always exits 0), and on any read/parse failure or when
nix/python3 is unavailable it prints a note and exits 0.

### just gc — garbage collection

`just gc` runs `nix-collect-garbage --delete-old` + `nix store optimise`
(dedupe). From the justfile (lines 253-255):

```make
gc:
    nix-collect-garbage --delete-old
    nix store optimise
```

It reclaims unreachable store paths (GC roots vector) and deduplicates
content-addressed copies (vector c). The store-growth model from
`docs/nix-purity.md` identifies three vectors: (a) per-edit churn, (b) GC
roots, (c) content-addressed copies.

### Format checking (nix fmt, nixpkgs-fmt, alejandra)

- `nix fmt --check` is experimental and NOT used in this project. From the
  nix-usage SKILL.md: "NO `nix fmt` — the project does not configure a
  formatter."
- `nixpkgs-fmt --check` and `alejandra --check` are Nix code formatters, also
  NOT used in this project.
- The project has no Nix code formatter configured. Nix formatting is enforced
  by code review, not tooling.

### CI integration

Run validation in GitHub Actions. The canonical pattern uses
`cachix/install-nix-action` to install Nix, then runs `nix flake check` as the
primary gate. Minimal workflow:

```yaml
name: "Nix"
on:
  pull_request:
  push:
jobs:
  nix-check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: cachix/install-nix-action@v25
      with:
        nix_path: nixpkgs=channel:nixos-unstable
    - run: nix flake check --no-build
    - run: nix build .#workestrate --no-link
```

CI should NOT run `nix flake update` (from the nix-usage SKILL.md). See
`docs/nix/ci-cd-integration.md` for full CI/CD guidance.

## Practical rules

1. Run `nix flake check` before merging any flake change — it validates the
   entire output tree.
2. Use `nix build .#<name> --no-link --print-out-paths` to avoid creating
   `result` symlinks that pin GC roots.
3. NEVER use `--impure` — the lint-nix guard forbids it and it copies the raw
   working tree into the store.
4. Stage new files (`git add -N`) before eval — untracked files are invisible
   to `.#` refs.
5. Run `just lint-nix` before committing nix-adjacent changes.
6. Run `just verify` as the full pre-merge gate; run `just verify-full` when
   the nix build itself must be confirmed.
7. Run `just gc` after `nix flake update` — a new nixpkgs revision pulls a
   multi-GB toolchain closure.
8. Run `just store-audit` when the store feels large — it reports top-20
   paths and blocks on oversized source copies.
9. CI should run `nix flake check` (or `nix flake check --no-build` for
   eval-only) as the primary nix gate.
10. CI should NOT run `nix flake update`.

## Review checklist

- [ ] `nix flake check` passes (or `--no-build` if builds are deferred to CI)
- [ ] `nix build .#<name>` succeeds for the target output
- [ ] No `--impure` invocations in any script, justfile, or nix code
- [ ] `just lint-nix` passes (0 purity violations)
- [ ] `just store-audit` passes (no `*-source` paths over 50 MB)
- [ ] New files are `git add`-ed before eval
- [ ] `result*` symlinks cleaned up after builds
- [ ] `just verify` passes before merge
- [ ] `just verify-full` passes when nix build confirmation is required
- [ ] No `nix fmt` / `nixpkgs-fmt` / `alejandra` invocations (not configured)

## Implementation checklist

- [ ] Decide whether the gate is a `checks` derivation (run by
      `nix flake check`) or a justfile recipe
- [ ] If a `checks` derivation: add to `checks.<system>.<name>` in flake.nix;
      use `pkgs.runCommand` or `testers`
- [ ] If a justfile recipe: add the recipe and wire it into `verify` (or
      `verify-full` for nix-build gates)
- [ ] Ensure the gate is non-blocking on missing prerequisites (nix, python3)
      unless it is a blocking gate by design
- [ ] Run `just lint-nix` to confirm no purity violations in the new code
- [ ] Document the gate in this doc and in the justfile recipe comment

## Validation hooks

```sh
# Primary nix validation (HOST-GATE: requires nix on host)
nix flake check
nix flake check --no-build

# Build a single output
nix build .#workestrate --no-link

# Run a single check
nix build .#checks.x86_64-linux.validateConfig

# Evaluate an attribute
nix eval .#workestrate.meta.description

# List all outputs
nix flake show

# Project pipeline
just lint-nix          # 6 purity checks
just store-audit       # top-20 + blocking source-path gate
just verify            # full pre-merge gate
just verify-full       # verify + nix build .#workestrate
just gc                # garbage collection + optimise
```

## Examples

### Running nix flake check

```sh
$ nix flake check
warning: creating lock file ...
checks.x86_64-linux.validateConfig> ...
```

### Running just verify

```sh
$ just verify
# Runs: toolchain-check, check, test, spec-examples, litellm-check,
#       golden-check, schema-check, scaffold-check, lint-nix, store-audit,
#       then: git diff --exit-code HEAD -- control/agentctl/Cargo.lock
```

### Running just store-audit

```sh
$ just store-audit
=== store-audit: top-20 store paths by closure size ===
   1,234,567,890  /nix/store/abc...-nixos-24.05
         123,456  /nix/store/def...-hello-2.12.1
OK: no ai-workbench *-source path exceeds 50 MB.
```

### A lint-nix violation and fix

Violation:

```nix
# BAD: unfiltered builtins.path — unbounded store copy
src = builtins.path { path = ./.; };
```

Fix:

```nix
# GOOD: filtered builtins.path
src = builtins.path {
  name = "source";
  path = ./subdir;
  filter = path: type:
    !(lib.hasSuffix ".md" path) && !(lib.hasPrefix "." (baseNameOf path));
};
```

## Common mistakes

| Mistake | Consequence | Fix |
| --- | --- | --- |
| Using `--impure` | Copies raw working tree into store (up to 35G); lint-nix fails | Use the git-filtered `.#` ref form |
| Forgetting `git add` on new files | `nix build` fails with "not tracked by Git" | `git add -N <file>` before eval |
| Leaving `result` symlinks | Pins closures forever; store grows | Use `--no-link` or remove `result*` |
| Running `nix flake update` in CI | Multi-GB rebuild from new nixpkgs | Update deliberately, never in CI |
| Using `nix fmt` | Not configured; command fails | Do not use; no formatter is configured |
| Using `.#default` | No such output; build fails | Use `.#workestrate` |
| Expecting `nix flake check` without nix | Command not found | HOST-GATE: requires nix on host |
| Skipping `just lint-nix` | Purity violations reach CI | Run `just lint-nix` before committing |

## Strict vs contextual guidance

**Strict (always):**

- NEVER use `--impure` in any nix invocation.
- NEVER use `builtins.getFlake` with `toString`.
- NEVER use `builtins.path` or `cleanSourceWith` without a `filter =`.
- NEVER run `nix flake update` in CI.
- ALWAYS stage new files before eval.
- ALWAYS run `just lint-nix` before committing nix changes.

**Contextual (depends on situation):**

- `nix flake check --no-build` vs `nix flake check` — use `--no-build` when
  build time is a concern and builds are deferred to CI.
- `just verify` vs `just verify-full` — use `verify-full` only when the nix
  build itself must be confirmed (it is slow).
- `just gc` cadence — run after `nix flake update`, after large builds, or
  when the store feels large.
- `just store-delta-check` — periodic host/CI check, NOT wired into `verify`
  (requires nix + is slow).

## Policy decisions for individual repos

- This project (ai-workbench) uses `just verify` as the pre-merge gate and
  `just verify-full` for nix-build confirmation.
- The project does NOT configure a Nix code formatter (`nix fmt`,
  `nixpkgs-fmt`, `alejandra` are all absent).
- The project's `checks` output contains only `validateConfig`; other
  validation is in justfile recipes (cargo gates, litellm-check, etc.).
- The `store-audit` blocking threshold is 50 MB for `*-source` paths.
- The `store-delta-check` threshold is 50 MB for `/nix/store` growth from one
  pure eval.
- Other repos may add more `checks` derivations or configure a formatter;
  this doc reflects the ai-workbench policy.

## Related docs

- [Nix Purity](../nix-purity.md) — purity rules, store-growth model, the 29GB incident, the lint-nix guard
- [Nix Store Hygiene and GC](store-hygiene-and-gc.md) — GC roots, store-growth model, store audit tooling, hygiene recipes
- [Testing](testing.md) — nix flake check, checks output, checkPhase, nixosTests
- [CI/CD Integration](ci-cd-integration.md) — GitHub Actions, Cachix, CI workflow patterns
- [Nix Commands](nix-commands.md) — nix build, nix eval, nix flake, nix store command reference
- [Flake Anatomy](flake-anatomy.md) — flake outputs, inputs, systems

## Related skills

- [nix-usage](../../.agents/skills/nix-usage/SKILL.md) — guard inventory, anti-accumulation patterns, agent rules
- [validation-rust-compile](../../.agents/skills/validation-rust-compile/SKILL.md) — Rust compile validation gate
- [validation-rust-clippy](../../.agents/skills/validation-rust-clippy/SKILL.md) — Rust clippy validation gate
- [validation-rust-test](../../.agents/skills/validation-rust-test/SKILL.md) — Rust test validation gate

## Citations

[1] [Nix Flakes — nix.dev](https://nix.dev/concepts/flakes.html)
[2] [nix flake check — Nix Manual](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake-check.html)
[3] [nix build — Nix Manual](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build.html)
[4] [nix eval — Nix Manual](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-eval.html)
[5] [nix flake show — Nix Manual](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake-show.html)
[6] [Continuous Integration with GitHub Actions — nix.dev](https://nix.dev/guides/recipes/continuous-integration-github-actions.html)
[7] [Nix Manual — reference](https://nix.dev/reference/nix-manual.html)
