# 06 — `--home` global CLI flag (idiomatic config-home override)

> **STATUS: EXECUTED (2026-07-30 — commit `d991252` "feat(agentctl): global --home flag (spec 06)")**
> **Effort:** S (additive CLI front-end; no path-resolution change)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [03-dogfooding.md](03-dogfooding.md) ·
> [../03-sibling-config-setup.md](../03-sibling-config-setup.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md)

This document specifies a small, additive CLI convenience: a global `--home
<DIR>` flag on the `workestrate` binary that overrides the tool home
(`WORKESTRATE_HOME`) for a single invocation. It is **not** a new capability —
the env var already wins today — but a discoverability and ergonomics fix that
brings the tool in line with standard CLI idioms (`git --git-dir`, `cargo
--manifest-path`, `docker --config`). The flag reuses the existing env-var
precedence step in `resolve_home_with_kind()`, so **no path-resolution logic
changes**.

### Environment markers

- `verifiable-here` — the fix gate (`cargo test`) runs anywhere with a C
  toolchain; this container has no `cc` linker, so in practice it runs in the
  HOST-NIX devshell.
- `HOST-NIX` — the devshell where `cargo test` actually runs here.

---

## 1. Motivation

The tool currently selects the tool home ONLY via the `WORKESTRATE_HOME` env
var — precedence step 1 in `resolve_home_with_kind()`
(`control/agentctl/src/config/paths.rs:104-150`, env check at `:106-110`) —
then trusted-ancestor discovery (`:116-139`), then legacy XDG (`:141-145`),
then default `~/.workestrate` (`:147-149`). An env var is the least
discoverable and least idiomatic override mechanism.

A global `--home <DIR>` flag is the standard CLI idiom: `git --git-dir` /
`git -C`, `cargo --manifest-path`, `docker --config`, `kubectl --kubeconfig`,
`terraform -chdir`. Per [clig.dev](https://clig.dev) guidance, env-var-only
behavior is undiscoverable — flags surface in `--help`, env vars do not. An
operator who runs `workestrate --help` should see every way the tool can be
redirected, without reading the source or the ADR set.

The governing precedence-layering principle is: **flag > env >
project/config file > default** — explicit per-invocation intent beats
ambient environment. The flag does not introduce a new resolution layer; it
sets the env var, so it slots in *above* the existing env step without
touching `paths.rs`.

---

## 2. Why it matters more for workestrate

The tool juggles MULTIPLE homes concurrently:

- the real bundle `.workestrate/` at the repo root,
- the disposable experiment home `/tmp/workestrate-exp` (see
  [../03-sibling-config-setup.md](../03-sibling-config-setup.md)),
- the dogfood driver home `~/.workestrate-driver` (see
  [03-dogfooding.md](03-dogfooding.md) §Phase 0), and
- the future default `~/.workestrate`.

Contrast `--context` (`control/agentctl/src/main.rs:36-37`), which selects
config LAYERS *within* one home, vs `--home`, which selects WHICH home/brain
entirely (registry + state + secrets). The dogfood driver case genuinely
needs a separate home, so `--home` (not just `--context`) is the right fit:
`--context` cannot redirect the registry, the state dir, or the secrets
store — only `--home` (and therefore `WORKESTRATE_HOME`) can.

---

## 3. The change (small)

Two edits, no path-resolution change:

- Add to the `Cli` struct in `control/agentctl/src/main.rs` (alongside the
  existing globals `--show-source` `:33-34`, `--context` `:36-37`, `--json`
  `:42-43`):

  ```rust
  #[arg(long, global = true, value_name = "DIR", help = "Workestrate tool home")]
  home: Option<PathBuf>,
  ```

- In `async_main` (`main.rs:237-244`), mirror the existing `--context`
  pattern (`:242-244`):

  ```rust
  if let Some(ref h) = cli.home {
      std::env::set_var("WORKESTRATE_HOME", h);
  }
  ```

Because `resolve_home_with_kind()` checks `WORKESTRATE_HOME` FIRST
(`paths.rs:106`), the flag automatically becomes the highest-precedence
override with NO change to path-resolution logic.

> **Import note (verified):** bare `PathBuf` is **not** in scope in
> `main.rs` — the file uses the fully-qualified form `std::path::PathBuf`
> at `main.rs:89` and `:117` (no `use std::path::PathBuf;` at the top of
> the file). Either spell the field as `Option<std::path::PathBuf>` to
> match the existing style, or add `use std::path::PathBuf;` to the import
> block at `main.rs:1-23`. The latter is cleaner; the former is the smaller
> diff.

---

## 4. Precedence rule (explicit)

Once the flag exists, the full effective precedence is:

**flag > exported `WORKESTRATE_HOME` env > trusted-ancestor discovery >
legacy XDG > default `~/.workestrate`**

The flag sets the env var, so it wins over an ambient export AND implicitly
disables walk-up discovery for that invocation — discovery only runs when
the env var is unset (`paths.rs:116-120`). This is worth documenting so no
one is surprised that an explicit `--home` is never overridden by a stray
trusted `.workestrate/` in an ancestor directory. The flag is a hard
per-invocation pin: it cannot be silently demoted by filesystem state.

This matches [ADR 0023](../../migration/50-decisions/0023-single-tool-home.md)
§"Resolution precedence" (lines 84-94): the flag does not add a step to
that list, it populates step 1 (`WORKESTRATE_HOME`) from the CLI surface.

---

## 5. Ergonomics wins

- **Per-invocation** — no leaked `export` contaminating later commands. An
  `export WORKESTRATE_HOME=...` persists across the shell session and can
  silently redirect a later unrelated `workestrate` call; `--home` applies
  only to the invocation it is passed to.
- **Discoverable** — shows in `--help`, so an operator browsing subcommands
  sees the override without reading docs.
- **Shell-completion-friendly** — the clap `completions` subcommand already
  exists, so `--home` will appear in generated completions for free.
- **Greppable in shell history** — `history | grep -- '--home'` finds every
  redirected invocation; an env var is invisible in `workestrate ...`
  command lines.
- **Simplifies the experiment-home flow** ([../03-sibling-config-setup.md](../03-sibling-config-setup.md))
  and the dogfood-driver flow ([03-dogfooding.md](03-dogfooding.md)): e.g.

  ```sh
  alias workestrate-driver='workestrate --home ~/.workestrate-driver'
  ```

  replaces the env-pinning wrapper's `WORKESTRATE_HOME` line. (The wrapper
  still needs `WORKESTRATE_CONFIG_DIR` / `AGENTCTL_ROOT` /
  `WORKESTRATE_NO_PROJECT_CONFIG` for the full Phase 0 isolation — `--home`
  only replaces the home-selection line, not the whole wrapper.)

---

## 6. Scope honesty

This is an additive convenience front-end, NOT a new capability — the env
var already wins today. The env var stays for CI, wrappers, `.envrc`, and
the justfile. No ADR invariant is weakened: [ADR 0023](../../migration/50-decisions/0023-single-tool-home.md)'s
single-home model and trust-gated discovery are untouched. The flag is a
pure CLI-surface translation of an existing precedence step.

---

## 7. Tests

Enumerate:

- (a) flag sets home — plan/validate against `--home /tmp/x` resolves
  `/tmp/x` as the tool home;
- (b) flag beats an exported `WORKESTRATE_HOME` — with
  `WORKESTRATE_HOME=/a` in the environment, `--home /b` resolves `/b`;
- (c) flag disables discovery — run from inside a trusted-ancestor tree
  with `--home` elsewhere → flag wins (no walk-up);
- (d) works on a representative subcommand (e.g. `validate-config` or
  `context current`);
- (e) `--help` output shows `--home <DIR>`.

**Gate:** `cargo test` (HOST-NIX devshell — no `cc` linker in this
container).

---

## 8. ADR note

No new ADR is needed for the mechanism (it reuses the existing env-var
precedence step). To keep the docs honest, add a one-line note to
[ADR 0023](../../migration/50-decisions/0023-single-tool-home.md)'s
§"Resolution precedence" list (`docs/migration/50-decisions/0023-single-tool-home.md:84-94`)
stating that the `--home` global flag sets `WORKESTRATE_HOME` and therefore
slots in as precedence step 0 when passed (i.e. it populates step 1 from
the CLI surface, winning over any ambient export).

This ADR note was **applied at implementation time** (2026-07-30) — see ADR
0023 §"Resolution precedence".
