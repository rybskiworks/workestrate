# ADR 0034: CLI-integrated secrets management (workestrate secrets) — replacing the standalone setup-secrets script

**Status:** Proposed
**Date:** 2026-09-01
**References:** `docs/secrets.md` (wrapper workflow); ADR 0018 (`docs/migration/50-decisions/0018-secrets-layering-and-per-repo-config.md` — per-repo layering + `secrets_file`/`age_key_file` overrides); ADR 0019 (`docs/migration/50-decisions/0019-contexts-and-user-global-overrides.md` — `--global` secrets layer); ADR 0023 (`docs/migration/50-decisions/0023-single-tool-home.md` — single `WORKESTRATE_HOME`); `control/agentctl/src/config/paths.rs` (`resolve_home_with_kind`, `resolve_home`); `control/agentctl/src/main.rs` (`Commands` enum, global `--home` flag); `control/agentctl/src/commands/secrets_target.rs` (`cmd_secrets_target`, `cmd_secrets_schema`, per-entry override resolution); `scripts/setup-secrets.sh` (standalone wrapper); `nix/devshells/default.nix` + `flake.nix` (`decrypt-env`/`write-env`/`setup-secrets` packages, devshell `setup-secrets` command)

## Context

Secrets are managed today via the standalone shell script `scripts/setup-secrets.sh`, exposed in the devshell as the `setup-secrets` package (`nix/devshells/default.nix` → `flake.nix` `setup-secrets`/`decrypt-env`/`write-env` outputs). The script encrypts/decrypts per-config `.env.enc` files and the user-global `secrets/.env.local.enc` layer with SOPS + age (`age-keygen`, `sops`), reusing `workestrate secrets-target --json` internally to resolve per-config overrides for `secrets_file` / `age_key_file` (Track B, `docs/secrets.md` “Landed”). The inversion is awkward: the external shell wrapper calls into the CLI to retrieve the directory/key that the CLI already knows how to resolve.

Current `workestrate` CLI surface (`control/agentctl/src/main.rs` `Commands`) exposes `check`, `init`, `new`, `completions`, `run`, `validate-config`, `secrets-schema`, `generate-env-example`, `ps`, `down`, `clean`, `images gc`, `context`, `generate-schema`, `config`, `home`, `schemas`, `secrets-target`, `doctor`, `source`, `migrate-home`, `workload`, `workloads`, `instances`, `policy`. There is **no** user-facing `secrets` subcommand; only plumbing helpers (`secrets-schema`, `secrets-target`, `generate-env-example`, `run`) that decrypt/inject but do not create or edit.

Existing ADRs: the index (`docs/migration/50-decisions/README.md`) lists 0001–0033; none covers CLI-integrated secrets. ADR 0018 established per-key value merge and per-repo `secrets_file`/`age_key_file` with “script alignment as follow-up”. ADR 0019 added `--global` secrets and `setup-secrets --global`. ADR 0023 collapsed the XDG three-home into single `WORKESTRATE_HOME`. `docs/secrets.md` documents the `setup-secrets` wrapper workflow. No ADR proposes `workestrate secrets {init,update,edit}`.

## Rationale — why the standalone script is painful

1. **Two env vars diverge.** The script honours `WORKESTRATE_CONFIG_DIR` and XDG-derived `TARGET_DIR` logic (`scripts/setup-secrets.sh:74-132` — `resolve_store_dir`/`resolve_config_dir` + `WORKESTRATE_CONFIG_DIR` override). The Rust CLI resolves the tool home via `control/agentctl/src/config/paths.rs:resolve_home_with_kind` (`WORKESTRATE_HOME` → legacy XDG compat → `~/.workestrate`) and a global `--home` flag in `main.rs:54-60` that populates `WORKESTRATE_HOME` for the invocation (ADR 0023 addendum, commit `d991252`). Operators must keep the two models in sync manually; they drift.

2. **`nix develop --keep` drops `WORKESTRATE_HOME`.** `setup-secrets` runs as `nix develop -c setup-secrets …`. Unless the caller adds `--keep WORKESTRATE_HOME` (or exports inside the shell), the outer `WORKESTRATE_HOME=/tmp/test-home` is absent inside the devshell. The failure mode is silent: the script targets the default `~/.workestrate` instead of the intended hermetic home — a hermetic-test footgun.

3. **Script reimplements home resolution.** `TARGET_DIR` detection in the script (`repos/$CONFIG_NAME`, `resolve_store_dir`/`resolve_config_dir`, `$PWD` fallbacks) duplicates — and diverges from — the registry + `resolve_home_with_kind` + context logic that already resolves store/config-repos paths in Rust (`config::resolve_store_dir`, `config::load_registry`, `cmd_secrets_target`). Store renames (`repos/` → `config-repos/`, ADR 0023 addendum spec 10) must be fixed in two places.

4. **No `--home` flag support.** `workestrate --home /tmp/test-home workload …` works (the CLI sets `WORKESTRATE_HOME` for the invocation). `setup-secrets --home …` does not exist; the caller must juggle env vars. Config-specific targeting requires the wrapper’s `--config <name>` to line up with the CLI’s registry view — again, two codepaths.

5. **Discoverability.** Users expect `workestrate --help` to surface secrets management. A separate devshell-only command (`setup-secrets`, `decrypt-env`, `write-env`) is invisible to `workestrate` help, completions, and `doctor` diagnostics, and is unavailable outside `nix develop` unless separately installed.

The script *does* call `workestrate secrets-target --json` for per-repo overrides, but the delegation is inverted: the shell is the entry point and the CLI is the helper. The ergonomic fix is to invert it — make the CLI the entry point.

## Options considered

1. **Keep standalone script (status quo). — REJECTED.** Retains all five pain points above. The dual-env-var drift and `nix develop --keep` hazard have been hit in hermetic tests and onboarding (“ awkward env handling ” in the request). `workestrate --help` remains silent about secrets.

2. **Thin wrapper: `workestrate secrets` delegates to `scripts/setup-secrets.sh`. — CONSIDERED, REJECTED as target.** A Rust subcommand that `exec`s the existing shell script behind the standard `--home` flag would fix (4) and (5) immediately with minimal code. However it retains dual codepaths (Rust home resolution just to `exec` a script that re-derives `TARGET_DIR`), two error models, and shell as source-of-truth for SOPS/age flow, editor loop, and validation. Maintenance cost stays split; hermetic tests still shell out; `secrets_target` inversion remains.

3. **Full CLI integration — `workestrate secrets {init,update,edit,show}` (SELECTED).** New top-level `workestrate secrets` verb, parallel to `home`/`config`/`workload`, integrated with the standard `--home`/`WORKESTRATE_HOME` resolution, `--config`/`--global` targeting, and per-entry `secrets_file`/`age_key_file` logic already in `control/agentctl/src/commands/secrets_target.rs` and `control/agentctl/src/config/loading.rs:resolve_secrets_layers`. SOPS/age invocation is ported to Rust (calling the `sops`/`age-keygen` binaries, not reimplementing crypto). Selected: unified CLI, correct home resolution, no `nix develop --keep`, discoverable in help/completions, testable with temp-home Rust tests, single source of truth.

   Argument for top-level `secrets` vs nested: secrets spans per-config repos, the user-global layer (`$WORKESTRATE_HOME/secrets/.env.local.enc`), and the home-level age key (`~/.config/sops/age/ai-workbench-secrets.txt` — never under the home), not merely “one config repo’s file”. A top-level verb mirrors `home`/`config`/`workload` in `main.rs:Commands` and is the expected discovery location. See option 4 for the rejected nesting.

4. **Alternative nesting: `workestrate config secrets …`. — CONSIDERED, REJECTED.** Keeps secrets under the config-repo verb. Plausible (secrets files live inside `config-repos/<name>/`), and ADR 0018’s `setup-secrets --config` framing suggests it. Rejected as narrower: it obscures the global layer and home-level key concerns, which are not “a config repo”. `config` already owns `add`/`list`/`update`/`trust`; overloading it with SOPS/editor flows mixes git-clone concerns with crypto/editor concerns. The top-level `secrets` verb is parallel to the existing top-level `home` verb (also home-scoped) and to `doctor` (also cross-cutting).

## Decision

Propose a new top-level `workestrate secrets` subcommand family that is the **user-facing** secrets editor. The existing plumbing (`workestrate run --`, `workestrate secrets-target --json`, `workestrate secrets-schema`, `workestrate generate-env-example`) remains as internal plumbing / backward-compat aliases; the new verb is the documented path.

### Proposed CLI shape

```
workestrate [--home <path>] [--context <name>] secrets <action> [options]

Actions (verbs, not flags):
  init    — create encrypted secrets for a target (refuses if exists; replaces setup-secrets init)
  update  — decrypt → edit → re-encrypt (replaces setup-secrets update)
  edit    — alias for update (familiar to sops users)
  show    — decrypt to stdout (replaces decrypt-env; alias: decrypt, cat)
  validate— check required keys present and values well-formed (uses secrets-schema)

Targeting (mutually exclusive, matching the script):
  --config <name>   per-config repo target (resolves via registry, like secrets-target)
  --global          user-global layer at $WORKESTRATE_HOME/secrets/.env.local.enc
  (no flag)         auto-detect single registered config; soft-deprecated, warns and suggests --config

Examples:
  workestrate --home /tmp/test-home secrets update --config personal
  workestrate secrets init --global
  workestrate --home ~/.workestrate secrets show --config personal
  workestrate secrets edit --config work
```

The top-level `secrets` verb is chosen over `workestrate config secrets` (see Options §4): secrets is cross-cutting (per-config + global + home-level key), not config-repo-only. A future `workestrate secrets schema` sub-subcommand may subsume `secrets-schema`/`generate-env-example`, but those top-level names are **kept for one release cycle** as aliases/hidden compat to avoid churn.

### Design constraints

- **Use global `--home` / `WORKESTRATE_HOME` resolution already used by `workestrate workload …`.** The subcommand must resolve the home via `control/agentctl/src/config/paths.rs:resolve_home_with_kind` and the `main.rs:54-60` `--home` flag that populates `WORKESTRATE_HOME` for the invocation. It must **not** introduce new `WORKESTRATE_CONFIG_DIR` handling; registry + context logic + `--config <name>` is the targeting model. The script’s `WORKESTRATE_CONFIG_DIR` / `XDG_*_HOME` / direct `TARGET_DIR` branches are retired (read-only shim only).

- **Per-config targeting via `--config <name>` and global via `--global`, mutually exclusive, matching the script’s flags.** Also support no-flag auto-detect (single registered config) for backward compat but emit a deprecation warning pointing to `--config`.

- **Correctly resolve `secrets_file` and `age_key_file` per entry overrides from the registry** (like `cmd_secrets_target` does). Reuse existing logic from `control/agentctl/src/commands/secrets_target.rs:49-91` and `control/agentctl/src/config/loading.rs:resolve_secrets_layers` — `entry.secrets_file` default `.env.enc`, `entry.age_key_file` tilde-expanded, fallback `SOPS_AGE_KEY_FILE` env else `~/.config/sops/age/ai-workbench-secrets.txt`. Do not duplicate derivation.

- **Reuse existing SOPS/age flow.** Joins the current shell flow: `ensure_key` (`age-keygen` → `~/.config/sops/age/ai-workbench-secrets.txt` mode 0600, derive recipient via `age-keygen -y`), `update_sops_config` (replace `age1PLACEHOLDER…` recipient in the target’s `.sops.yaml`), encrypt/decrypt via calling the `sops` binary, editor buffer with sentinel + 3-attempt validation via `workestrate secrets-schema` / `generate-env-example`. Initially implement by porting the shell editor loop to Rust *or* exec-ing `sops`/`age-keygen`; note that `sops encrypt`/`decrypt` is via the `sops` binary — not a Rust reimplementation of the crypto.

- **Age key stays at `~/.config/sops/age/ai-workbench-secrets.txt` (never under the home).** Same invariant as `docs/secrets.md` and ADR 0018: the key is host-scoped, never in repo/bundle/`.workestrate`/`$WORKESTRATE_HOME`, absent in containers (fail-closed).

- **Provide `--home <path>` without env var juggling.** The canonical hermetic example must work without `nix develop --keep`:

  ```bash
  # Before (script):
  WORKESTRATE_HOME=/tmp/test-home nix develop --keep WORKESTRATE_HOME -c setup-secrets --config personal update
  # After (CLI):
  workestrate --home /tmp/test-home secrets update --config personal
  ```

  Without the flag, `WORKESTRATE_HOME=/tmp/test-home workestrate secrets update --config personal` must also work — both set precedence step 1.

- **Keep `workestrate run --` and `workestrate secrets-target --json` as internal plumbing.** `run` remains the decrypted-exec path; `secrets-target --json` remains the machine-readable resolver for scripts/tooling. The new `secrets` subcommand is the human-facing editor. `secrets-schema` and `generate-env-example` remain for one cycle and may later become `workestrate secrets schema` / `workestrate secrets example`.

### Behavior details

- `init` refuses to overwrite an existing `secrets_file`; `update`/`edit` requires it exists.
- Non-interactive paths preserved: if all required env vars (from `secrets-schema`) are set and non-empty, use them directly; if stdin is not a TTY, read one line per `REQUIRED_KEYS` in order (empty line = preserve, non-empty = replace) — same contract as the script.
- Editor flow: write a mode-0600 temp buffer pre-filled from `generate-env-example` + existing decrypted values, include the `# setup-secrets: delete this line…` sentinel, launch `$EDITOR` (`nano`/`vi`/`vim` fallback), validate, re-prompt with `# ERROR:` annotation up to 3 attempts.
- `--global` ignores per-repo overrides; it always targets `$WORKESTRATE_HOME/secrets/.env.local.enc` with the home’s `.sops.yaml` equivalent handling.
- `--context` (global flag) does not change the edit target but influences validation context when resolving required keys from the active config.

## Consequences

**Positive:**

- **Unified CLI.** `workestrate --help` surfaces secrets management; shell completions cover `secrets {init,update,edit,show}`.
- **Correct home resolution.** Single source of truth via `paths.rs:resolve_home_with_kind` + global `--home` flag; no dual `WORKESTRATE_CONFIG_DIR` branch, no XDG re-derivation, no store-rename drift.
- **No `nix develop --keep` hazard.** `workestrate --home <path> secrets …` is the hermetic path; `WORKESTRATE_HOME` inside vs outside the devshell no longer matters.
- **Testable in Rust.** Temp-home tests can exercise `workestrate --home $TMPHOME secrets init/update/show` hermetically (parallel to `validate-secrets-workflow.sh` steps) without shell har`n`ess.
- **Discoverable diagnostics.** `workestrate doctor` can report `sops`/`age-keygen` presence and `age_key_file` existence alongside secrets targets.

**Negative / costs:**

- **Porting the editor loop + SOPS interaction.** The shell script’s sentinel, 3-attempt re-open, stdin/env-var precedence, and `sops`/`age-keygen` exec error handling must be ported and re-tested. Crypto itself is not reimplemented (still shelling to `sops`), but process spawning, temp-file mode 0600, and trap/cleanup semantics need care.
- **Flag parity surface.** `--config`/`--global` mutual exclusion, no-flag deprecation warning, and error messages must stay compatible with the wrapper’s phrasing for one cycle.

**Backward compatibility:**

- Keep `scripts/setup-secrets.sh` as a **deprecated shim** that prints a one-time warning and `exec`s `workestrate secrets …` (mapping `--config`/`--global`/`init`/`update` through) for one release cycle. Keep `flake.nix`/`nix/devshells/default.nix` `decrypt-env`/`write-env`/`setup-secrets` packages as aliases (or make them wrappers around `workestrate secrets show` / `workestrate run --` once the Rust verb lands). Remove only in phase 4 after docs/just recipes have migrated.

## Migration Path

1. **Phase 1 — Add Rust subcommand, keep script as wrapper.** Implement `Commands::Secrets { action: SecretsAction }` in `control/agentctl/src/main.rs` with `SecretsAction::{Init,Update,Edit,Show,Validate}` and the `--config`/`--global` flags (mutually exclusive, clap `conflicts_with`). Share `secrets_target.rs` resolution (extract `resolve_secrets_target(name)` helper used by both `cmd_secrets_target` and the new `cmd_secrets_*` handlers) and `config/loading.rs:resolve_secrets_layers` semantics. Make the shell script a thin shim: if `workestrate secrets` exists, delegate; otherwise run legacy path. No docs change yet except a “deprecated, use `workestrate secrets`” note in `docs/secrets.md` header.

2. **Phase 2 — Docs + recipes update.** Update `docs/secrets.md` “setup-secrets flows” → “`workestrate secrets` flows” (keep a “Legacy wrapper” subsection), update `justfile` `validate-secrets` recipe and `scripts/validate-secrets-workflow.sh` to exercise the Rust verb in its hermetic temp-home test (steps 3–7 now use `workestrate --home $TMPHOME secrets …`), and update `README.md`/agent onboarding that mentions `setup-secrets`. CI keeps a compat check that `nix develop -c setup-secrets --help` still works (via shim).

3. **Phase 3 — Remove flake/devshell packages from default.** Drop `setup-secrets`/`decrypt-env`/`write-env` from `nix/devshells/default.nix` `packages` (keep them as flake outputs for one more cycle behind `legacyPackages` or hidden). `workestrate` in the devshell now *is* the secrets entry point. Emit a shellHook note once per shell if the legacy wrapper is still on PATH.

4. **Phase 4 — Deprecate script file.** Delete `scripts/setup-secrets.sh` (or leave a 3-line stub that errors with the migration command), remove the flake `setup-secrets`/`decrypt-env`/`write-env` outputs, and delete the phase-1 shim branch. At this point `workestrate secrets-schema`/`generate-env-example` may be consolidated under `workestrate secrets schema`/`example` with hidden aliases retained.

### Testing plan (hermetic temp-home tests)

- Hermetic Rust integration test (new `control/agentctl/tests/secrets_cli.rs` or similar): create a temp `WORKESTRATE_HOME` + temp `HOME` for the age key, run `workestrate --home $TMPHOME config add …` (or init a minimal registry with one config repo), then `workestrate --home $TMPHOME secrets init --config <name>` with env-var-supplied values, `workestrate --home $TMPHOME secrets show --config <name>` asserting decrypted output, `workestrate --home $TMPHOME secrets update --config <name>` with stdin-supplied overrides, and `workestrate --home $TMPHOME run -- env` asserting injection. Also exercise `--global init/update/show` and the mutual-exclusion error (`--config`+`--global`).
- Port `scripts/validate-secrets-workflow.sh` to drive `workestrate --home` instead of `setup-secrets` for steps 3–7, keeping the `age-keygen` → `.sops.yaml` recipient rewrite → encrypt → decrypt → update → `write-env` (now `secrets show > .env`) → `workestrate run -- env` sequence.
- Negative tests: `init` refuses overwrite, `update` errors when no `.env.enc`, undecryptable layer warns, missing `sops`/`age-keygen` errors mention the binary, `--home` wins over ambient `WORKESTRATE_HOME`.

### Integration with existing helpers

`workestrate secrets-schema` and `workestrate generate-env-example` remain as top-level commands for backward compat. They are the schema/example source for the editor buffer and non-interactive validation. A future follow-up may namespace them as `workestrate secrets schema` / `workestrate secrets example` with the top-level names retained as hidden aliases (clap `visible_alias` / `alias`). `workestrate secrets-target --json` remains the machine-readable resolver; the new `secrets` verb reuses its resolution helper rather than duplicating it. `workestrate run --` remains the decrypted-exec plumbing; `workestrate secrets show` is the human-facing “print decrypted” counterpart to `decrypt-env`.

## Open Questions

- **Editor vs pure non-interactive in CI.** Should `workestrate secrets update` gain an explicit `--non-interactive` / `--from-env` flag to force env-var/stdin mode and never launch `$EDITOR` (fail if keys are missing), rather than inferring from “all env vars set” / “stdin not a TTY”? The script’s inference is convenient but surprising in CI; an explicit flag would make the contract testable. Default: keep inference for compat, add `--non-interactive` as an optional hardening flag.

- **Global `--context` interaction.** When `--context <name>` is passed alongside `secrets init/update --config <name>`, should validation use the context’s merged secrets schema (ADR 0019) or the single-repo schema? Proposal: validation uses the active context’s merged view when `--context` is given (so cross-layer required keys are checked), but the write target remains the single `--config <name>` file — same as `secrets-target`’s write-side vs read-side split (clone vs pinned archive, `secrets_target.rs:34-48`).

- **Age key rotation.** The age key at `~/.config/sops/age/ai-workbench-secrets.txt` is host-scoped and shared across all config repos. If a team rotates the team age key (different `age_key_file` per `[configs.<name>]`), the SOPS recipient update must be per-config. The Rust verb should handle this, but the global key’s rotation story (re-encrypting all per-config `.env.enc` files) is out of scope for this ADR — track as a follow-up to ADR 0018’s per-repo `age_key_file` design.

- **`write-env` / `decrypt-env` surface.** Should `write-env` (write `.env` mode 0600, refuse overwrite) become `workestrate secrets show --config <name> --write .env` or a dedicated `workestrate secrets export --write`? Proposal: `show` prints to stdout; `--write <path>` writes with 0600 and refuse-overwrite semantics — a single verb with an output flag, matching `generate-env-example --output`. Keep `write-env`/`decrypt-env` as flake aliases until phase 4.

- **Nesting decision finality.** If `config` verb ownership is later preferred (e.g., to group all registry-aware commands), `workestrate secrets` can be retained as a top-level alias to `workestrate config secrets` with no behavior change. The decision here is intentionally “top-level with alias-ability” — not a one-way door.

---

*Scope note:* This ADR does not change the secrets layering model (per-key value merge, per-repo `secrets_file`/`age_key_file`, user-global layer after context layers — ADRs 0018/0019) or the SOPS/age cryptography. It changes only the **management surface**: replacing the standalone `scripts/setup-secrets.sh` devshell wrapper with an integrated `workestrate secrets` CLI that reuses the existing home/context/registry resolution.
