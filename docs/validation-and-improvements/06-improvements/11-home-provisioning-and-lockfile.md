# 11 — Home provisioning (`home init --from` + positional dest) + `workestrate.lock`

> **STATUS: EXECUTED (2026-07-30 — commits `172d5dd` (provisioning), `19ff272` (lockfile), `be356f7` (lock consumption + version evolution); `--home` flag in `d991252`)**
> **Effort:** M
> Prerequisites / see-also: [README.md](../README.md) ·
> [00-index.md](00-index.md) · [06-config-home-flag.md](06-config-home-flag.md) ·
> [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md) ·
> [03-dogfooding.md](03-dogfooding.md) ·
> [../07-execution-order.md](../07-execution-order.md) · ADR 0023, ADR 0024
> ([../../migration/50-decisions/0024-dotfiles-home-and-working-copy-config-repos.md](../../migration/50-decisions/0024-dotfiles-home-and-working-copy-config-repos.md)),
> ADR 0025
> ([../../migration/50-decisions/0025-home-provisioning-and-lockfile.md](../../migration/50-decisions/0025-home-provisioning-and-lockfile.md)).

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

All gates in this spec are cargo-linked → run via `nix develop` / HOST-NIX
devshell.

---

## Summary

Implements ADR 0025: extends `home init` with `--from <src>` + positional
`<dest>`; introduces the generated `workestrate.lock`. Bare `home init` is
unchanged (ADR 0024d). Compose with spec 06's `--home` flag — same
CLI/home-resolution surface, implement in the same wave.

---

## 1. CLI surface (exact clap shapes)

`HomeAction::Init` GAINS `from: Option<String>` (`#[arg(long)]`) and positional
`dest: Option<String>`; the existing `config: Option<String>` (`#[arg(long)]`)
and `name: String` (`#[arg(long), default_value = "personal"]`) fields are
unchanged. Current definition site: `control/agentctl/src/cli_actions.rs:185`;
dispatch at `commands/home.rs:57-61`; implementation at
`commands/home.rs:67` (`cmd_home_init`).

Argument semantics:

| Invocation | Meaning |
|---|---|
| `home init` | Scaffold resolved home (today's behavior). |
| `home init <dest>` | Scaffold NEW home at dest (dest must not exist or be empty — bail if non-empty). |
| `home init --from <src>` | Provision resolved home from src. |
| `home init --from <src> <dest>` | Provision dest from src. |

`--config`/`--name` remain valid on the bare path only — specifying them with
`--from` or `dest` is a usage error.

---

## 2. Provisioning algorithm (step-by-step)

1. **Resolve src:** if `--from` value passes `looks_like_git_url`
   (`config/registry.rs:126` — note: currently private; make it `pub(crate)` or
   move to a shared module) → clone to a temp dir and treat that as src; else
   resolve path (absolute as-is, relative against cwd).
2. **Pre-flight validation (FAIL BEFORE WRITING ANYTHING — no residue):** src
   `config.toml` exists and parses as `Registry` (`config/registry.rs:11`
   `load_registry` shape); `home_version` compat check (absent = oldest;
   newer-than-supported = hard error "home created by a newer workestrate");
   read src `workestrate.lock` if present (same version rules); build the
   reproducibility report (per config repo: reproducible-via-url /
   local-only-copy / unreproducible-on-remote-source). If src is a remote (git
   URL) AND any config repo is local-only → the report marks those entries
   "unreproducible, clone manually" and they are SKIPPED (never a hollow home:
   the summary must name them).
3. **Materialize dest home layout** (`config-repos/`, `sources/`, `state/`,
   `secrets/` created; `secrets/` EMPTY).
4. **Git-clone the registry layer:** clone the src home repo into dest (carries
   `config.toml`, `overrides.toml`, `.gitignore`, hook, committed `*.enc`, and
   the committed `workestrate.lock`). If src home is NOT a git repo, copy
   `config.toml` (+ `overrides.toml` if present) instead and run the standard
   init scaffolding (ADR 0024c path).
5. **Reproduce config repos:** for each `[configs.<name>]`: reproducible →
   `git clone <url>` into `dest/config-repos/<name>` then `git checkout
   <locked rev>` (lock rev wins; fall back to registry `rev`, then to `ref`
   default `main`, warning when no pin exists); local-only-and-src-is-local →
   copy the working copy from src home; wire each resulting clone's `origin`
   to its registered url (or leave origin absent for local-only copies).
6. **Rewrite registry urls** that pointed into the SRC home's tree to the
   dest-local `config-repos/<name>` paths; leave remote urls untouched (the
   migrate-home rewrite precedent, ADR 0023).
7. **Wire the dest home repo's `origin`** to the src home repo path/url
   (pull-based sync default); print the
   `receive.denyCurrentBranch=updateInstead` opt-in hint for push-back.
8. **Write `workestrate.lock`** in dest (fresh pins from the actual
   checked-out revs; `tool_version` = current `env!("CARGO_PKG_VERSION")`).
9. **Carry `[[trusted_projects]]`** with a LOUD per-entry warning
   (machine-specific absolute paths).
10. **Post-flight:** run `validate-config` against dest; print summary (what
    was cloned/copied/skipped-unreproducible, lock written, next steps).

---

## 3. Lockfile design

TOML example:

```toml
version = 1
home_version = 2
tool_version = "0.1.0"

[repos.personal]
url = "https://github.com/user/workestrate-personal"
ref = "main"
rev = "c41a707…"
```

Struct sketch:

```rust
struct HomeLock {
    version: u32,            // default = oldest
    home_version: u32,
    tool_version: String,
    repos: BTreeMap<String, LockedRepo>,
}

struct LockedRepo {
    url: String,
    r#ref: Option<String>,
    rev: Option<String>,
}
```

**Writers:** `cmd_config_add` (`config_cmd.rs:188`), `cmd_config_update`
(`:510` — already re-records rev; extend to lock), `cmd_config_remove` (`:42`
— drop entry), `cmd_home_init` (`home.rs:67`).

**Consumers:** `--from` (step 5 above); future `up --pin` and spawn provenance
([03-dogfooding.md](03-dogfooding.md) B3 §5.3 + the pin-at-spawn item
designated B4 — the lock is their mechanism; do not build a second one).

**Evolution rules:** absent `version` = oldest; newer-than-supported = hard
error; changes via explicit migrations (migrate-home precedent). Registry =
human-edited; lock = generated; lock is committed to the home repo (not
gitignored).

**Touchpoint table** (file:line → change):

| File:line | Change |
|---|---|
| `config/registry.rs:11` / `:26` | `load`/`save` — the lock gets its own `load_home_lock`/`save_home_lock` siblings; save must be atomic like `save_registry` (see `registry.rs:667` test). |
| `config/registry.rs:126` | `looks_like_git_url` visibility (make `pub(crate)` or move to shared module). |
| `cli_actions.rs:185` | New args (`from`, `dest`) on `HomeAction::Init`. |
| `commands/home.rs:67` | Provisioning implementation. |
| `config_cmd.rs:42` / `:188` / `:510` | Lock writers (remove/add/update). |

---

## 4. Test plan

All `cargo test`, run via `nix develop`:

- positional dest: `home init <dest>` scaffolds at dest; two-arg usage errors
  rejected (`--config` + `--from` etc.).
- `--from` local absolute path AND relative path (cwd-resolved) both provision.
- `--from` with a local-only config repo in src → repo copied, report names it.
- `--from` git URL with local-only config repo → entry skipped +
  "unreproducible, clone manually" in report/summary; dest is NOT hollow
  (everything else present).
- invalid src (missing/unparseable config.toml) → error and NO dest residue
  (assert dest path does not exist after failure).
- lock round-trip: add/update/remove mutate the lock; save/load is atomic
  (mirror `registry.rs:667` test).
- newer lock version → hard error message contains "home created by a newer
  workestrate".
- trusted_projects carried + per-entry warning printed.
- locked-rev checkout: src lock pins rev R; dest clone HEAD == R (not branch
  tip).

---

## 5. Acceptance criteria

- [x] ADR 0025 exists
  ([../../migration/50-decisions/0025-home-provisioning-and-lockfile.md](../../migration/50-decisions/0025-home-provisioning-and-lockfile.md)).
- [x] [00-index.md](00-index.md), [../README.md](../README.md),
  [../07-execution-order.md](../07-execution-order.md) updated (those land with
  this docs wave).
- [x] `HomeAction::Init` gains `from: Option<String>` + positional
  `dest: Option<String>` at `cli_actions.rs:185`.
- [x] Bare `home init` unchanged (ADR 0024d).
- [x] `--config`/`--name` with `--from` or `dest` → usage error.
- [x] Provisioning algorithm steps 1–10 implemented at `commands/home.rs:67`.
- [x] `looks_like_git_url` made `pub(crate)` or moved to shared module
  (`config/registry.rs:126`).
- [x] `workestrate.lock` generated by `config add`/`update`/`remove`/`home
  init`; save is atomic (mirror `registry.rs:667` test).
- [x] Lock committed to home repo (not gitignored); absent `version` = oldest;
  newer-than-supported = hard error.
- [x] Test plan §4 green via `nix develop`.

---

## 6. Effort: M

Multi-file: CLI (`cli_actions.rs`), provisioning (`commands/home.rs`), lock
writers (`config_cmd.rs`, `registry.rs`), tests.
