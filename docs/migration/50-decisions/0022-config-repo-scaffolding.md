# ADR 0022: Config-repo scaffolding — native `config new` + copier interop

**Status:** Accepted
**Date:** 2026-07-21
**References:** ADR 0008 (config repos via uniform `workestrate config add`);
`control/agentctl/src/scaffold/` (embedded skeleton + renderer);
`control/agentctl/tests/scaffold_template.rs` (CI guards);
`secrets_loader.rs` (`AGE_KEY_DEFAULT_PATH`).

## Context

Config repos (the `workestrate.toml` + secrets + infra tree) need a creation
interface. Prior to this ADR, the only path was the copier template at
`templates/workestrator-config/` (`copier copy ...`). This required users to
install copier (a Python tool), offered no `--json` path for scripted setup,
and could not auto-derive the age recipient or auto-register the new repo.

The copier template also suffered from drift: its README referenced the age
key at `~/.config/sops/age/workestrate.txt` while the code
(`secrets_loader.rs`) uses `~/.config/sops/age/ai-workbench-secrets.txt` —
a live drift bug of the dual-source class this ADR's CI guards are designed
to prevent.

ADR 0008 established that config repos are consumed via uniform
`workestrate config add` (git clone into managed store). This ADR addresses
the creation side.

## Options considered

1. **Copier only (status quo)** — `copier copy templates/workestrator-config/`.
   Rejected as primary: external Python dependency; no `--json`; no
   auto-derive/auto-register; template drift undetected by CI.

2. **`cargo-generate` as embedded library** — use the `cargo-generate` Rust
   crate as a library inside workestrate for Liquid-templated scaffolding.
   Rejected: pulls ~50 transitive crates (libgit2/git2 stack) and has
   panicking entry points; overkill for a 7-file skeleton. No feature flag
   to drop the git dependency.

3. **Drop copier entirely; Rust-only scaffold** — `workestrate config new`
   with no copier retention. Rejected: loses `copier update` (ongoing
   template sync), which is copier's unique value for users who want to pull
   template improvements into existing repos.

4. **Rust-canonical scaffold + copier retained-and-narrowed (selected)** —
   a native `workestrate config new` (Rust, embedded skeleton, `str::replace`
   renderer, zero template-engine deps) PLUS copier retained for its unique
   features (team-key interactive prompts, `copier update` sync). The native
   scaffold writes a `.copier-answers.yml` sidecar so `copier update` works
   from native-scaffolded repos. CI guards enforce byte-parity between the
   two renderers for the minimal-personal overlap set.

## Decision

**Native scaffold (`workestrate config new`)** is the primary creation path:

- The skeleton lives in `control/agentctl/src/scaffold/template/` as plain
  files with bare `{{ var }}` placeholders, embedded via `include_str!` at
  compile time. No template engine dependency.
- The renderer uses `str::replace` for each `{{ key }}` → value pair, then
  asserts no `{{ ` remains (the unsubstituted-token guard). This is a
  deliberate maintainability forcing-function: if a feature needs jinja
  conditionals (`{% if %}`), it goes in the copier template, not the native
  skeleton.
- The command auto-derives the age recipient via `age-keygen -y` (falls back
  to `age1PLACEHOLDER` + loud warning), auto-runs `git init` (no commit,
  mirroring `cargo new`), and auto-registers in the registry via the shared
  `register_config()` helper (also used by `config add`).
- `--json` emits a result envelope for scripted use (the `gh repo create`
  precedent).
- `.copier-answers.yml` is written with `_src_path`/`_vcs_ref`/answers so
  `copier update` works afterward without copier having generated the repo.

**Copier template (`templates/workestrator-config/`) is retained and narrowed:**

- Its role shifts from "the only way to create a config repo" to "the
  advanced/team + ongoing-sync path".
- The minimal-personal render (default answers: `team_age_recipient=""`,
  `include_flake=false`) is byte-aligned with the native skeleton for the
  overlap files (`workestrate.toml`, `.sops.yaml`, `.env.example`,
  `.gitignore`).
- The team-key conditional and the flake inclusion remain copier-only
  features (copier's unique value).
- A `_tasks` post-copy hook regenerates `.env.example` via
  `workestrate generate-env-example` when workestrate is on PATH (kills the
  static-file drift class at the source; falls back to the committed static
  file when workestrate is absent).

**Four CI guards** (in `control/agentctl/tests/scaffold_template.rs`):

1. `native_skeleton_passes_validate_config` (always runs) — the rendered
   skeleton must pass `workestrate validate-config`. Closes the
   `rw`/`read_only`-class schema/policy drift for the skeleton.
2. `native_render_leaves_no_unsubstituted_tokens` (always runs) — no `{{ `
   may remain post-render. Catches skeleton/var drift.
3. `copier_template_byte_matches_native_render` (HOST-NIX-gated; SKIP if
   copier absent) — the copier default-answer render must byte-match the
   native render for the overlap set. Closes the dual-source drift class.
4. `copier_update_works_from_native_scaffold` (HOST-NIX-gated) — verifies
   the `.copier-answers.yml` interop contract: `copier update` succeeds on
   a native-scaffolded repo.

**Shared helper:** `register_config()` in `config.rs` is called by both
`cmd_config_add` (clone + register) and `cmd_config_new` (scaffold +
register) — single registry-write code path, no drift.

**Local-path config-update guard:** `cmd_config_update` skips repos with
`rev == None` (local-path repos created by `config new` that haven't been
pushed to a remote) with a forward-looking hint, rather than failing on
`git_pull` without a remote.

**Age-key-path drift fix:** the native skeleton's `.sops.yaml` and README
use `~/.config/sops/age/ai-workbench-secrets.txt` (matching
`secrets_loader.rs:118` and the `AGE_KEY_DEFAULT_PATH` const). The copier
template's README and `.sops.yaml.jinja` are aligned to the same path.

## Consequences

- First-run users no longer need copier; `workestrate config new personal`
  produces a valid, registered, git-initialized config repo in one command.
- The Rust binary is the source of truth for the minimal skeleton; bumping
  workestrate bumps the skeleton (compiled together).
- Copier remains available for team workflows and template sync, but is no
  longer required.
- Every drift class (skeleton↔schema, skeleton↔policy, native↔copier,
  `.env.example`↔`[secrets.*]`, age-key-path) has a CI guard or single-source
  elimination.
- The `--with-flake`, `--from-reference`, and `--empty` flags cover the
  advanced creation cases without copier.
- Future template additions that need conditionals go in the copier template;
  the native skeleton stays conditional-free by design.

## Rejected why

- **Copier only:** external dep, no scripting path, drift undetected.
- **cargo-generate lib:** heavyweight git2 stack for a 7-file skeleton.
- **Copier drop:** loses ongoing-sync value.

## Amendments

**WP-C (store-default destination):** `workestrate config new <name>`
now defaults to the managed store (`<store>/repos/<name>`) instead of
`./<name>`. This ensures the scaffolded repo is immediately active for
layer resolution once registered (the registry's bare `layers` list
points at `<store>/repos/<name>`, and `load_config` resolves layers from
that path). An explicit `--path` still overrides the default; when
`--path` places the repo outside the store AND registration is enabled
(not `--no-register`), a warning is printed:

    warning: '<dest>' is outside the config store ('<store>/repos/<name>');
    the repo won't be active for layer resolution until moved into the
    store or re-added via `workestrate config add` after pushing to a remote.

The non-empty destination refusal is unchanged.
