# 15 — TOML toolchain: tombi format/lint/schema-validation for config repos + homes

> **STATUS: IN-FLIGHT (implementation wave in progress)**
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [13-secret-env-shorthand.md](13-secret-env-shorthand.md) ·
> [14-env-map-form.md](14-env-map-form.md) ·
> [../../migration/50-decisions/0002-toml-config-format.md](../../migration/50-decisions/0002-toml-config-format.md) ·
> [../../migration/50-decisions/0022-config-repo-scaffolding.md](../../migration/50-decisions/0022-config-repo-scaffolding.md)

This document specifies the adoption of **tombi** as the TOML toolchain
(formatter + linter + schema validator) for this repo, for scaffolded config
repos, and for dotfiles-style homes. tombi keeps the TOML surface tidy and
schema-validated without ever dictating notation: the collapsed map form from
specs 13/14 is already schema-valid, so tombi normalizes layout only and never
forces array-of-tables vs map form.

### Environment markers

- `verifiable-here` — all cargo/script gates for this spec are runnable inside
  the container via `nix develop` (nix at
  `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH;
  prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
  then `nix develop -c bash -c '<cmd>'`). No KVM is needed.
- `HOST-NIX` — the `nix build .#tombi` package build runs on the user's host.

---

## 1. Decision + version strategy

**Decision: adopt tombi v1.2.5** (MIT) as the single TOML toolchain —
formatter, linter, and JSON-Schema validator — for:

1. this repo (`config.reference/workestrate.toml`, all `Cargo.toml`,
   `tests/fixtures`),
2. scaffolded config repos (`config new` output), and
3. dotfiles-style homes (`home init` output).

### 1.1 GATING FACT: the formatter normalizes layout only

tombi's formatter implements **19 lexical rules**
(`crates/tombi-config/src/format.rs:35-295` in the tombi source) — whitespace,
indentation, key alignment, trailing commas. It **cannot restructure**
array-of-tables to map form (or vice versa). The collapsed map notation from
specs [13](13-secret-env-shorthand.md)/[14](14-env-map-form.md) is already
schema-valid TOML, so tombi keeps it tidy + validated but **never forces a
form**. This is the gating fact that makes tombi safe to adopt on top of the
13/14 ergonomics: author-chosen notation stays author-chosen.

### 1.2 Version strategy: package v1.2.5 ourselves

- **Rejected: `cargo install tombi-cli`.** The crates.io name resolves to a
  **0.0.1 placeholder** — an install trap. Never use it.
- **Fallback: the nixpkgs pin.** Our nixpkgs pin (`a799d3e3`) ships only
  tombi **0.11.6**, whose config keys differ from 1.x (`file-match` vs
  `include`) — usable only with that caveat, and documented as the fallback
  path.
- **Decision: package v1.2.5 via a new `nix/packages/tombi.nix`** using
  `fetchurl` of the official musl binary with a pinned sha256, following the
  `nix/packages/microsandbox.nix` fetchurl-prebuilt pattern (`fetchurl` +
  `autoPatchelfHook` + `installPhase`). See §6.
- **Version guard:** hooks and scripts pin `TOMBI_REQUIRED=1.2.5` and fail
  loudly on a version mismatch (§4) — a stale 0.11.6 on PATH cannot silently
  lint with the wrong config keys.

---

## 2. `tombi.toml` designs

Three designs, one per deployment surface. All use `strict = true` (the
tombi default, stated explicitly for readability) and repo-relative paths.

### 2.1 Repo root (`tombi.toml` at the workbench root)

Include set: `config.reference/workestrate.toml` (format + schema-lint), all
`Cargo.toml` files (format only), `tests/fixtures` (format ONLY — hostile
fixtures may be schema-invalid). Excludes: `.tmp/`, `target/`, `vendor/`,
`agents/*/repo` + `agents/*/build`, `result*`, `node_modules`.

```toml
# tombi.toml — repo root
strict = true

[format]
enabled = true

[lint]
enabled = true

include = [
  "config.reference/workestrate.toml",
  "**/Cargo.toml",
  "tests/fixtures/**/*.toml",
]
exclude = [
  ".tmp/**",
  "target/**",
  "vendor/**",
  "agents/*/repo/**",
  "agents/*/build/**",
  "result*",
  "node_modules/**",
]

[[schemas]]
path = "./schemas/workestrate.schema.json"
include = ["config.reference/workestrate.toml"]
```

### 2.2 Scaffolded config repo (emitted by `config new`)

Include set: the repo's `workestrate.toml` (format + schema-lint against the
vendored schema copy the scaffold emits — §3).

```toml
# tombi.toml — scaffolded config repo
strict = true

[format]
enabled = true

[lint]
enabled = true

include = ["workestrate.toml"]

[[schemas]]
path = "./schemas/workestrate.schema.json"
include = ["workestrate.toml"]
```

### 2.3 Home (emitted by `home init`)

Include set: `config-repos/*/workestrate.toml` (format + schema-lint against
the vendored schema copy inside each config repo), `config.toml` +
`overrides.toml` (format-only). Excludes: `workestrate.lock`, `secrets/`,
`sources/`, `state/`. Posture: **warn-not-fail** when tombi is absent — a
home must stay usable on a machine without the toolchain (§4).

```toml
# tombi.toml — dotfiles-style home
strict = true

[format]
enabled = true

[lint]
enabled = true

include = [
  "config-repos/*/workestrate.toml",
  "config.toml",
  "overrides.toml",
]
exclude = [
  "workestrate.lock",
  "secrets/**",
  "sources/**",
  "state/**",
]

[[schemas]]
path = "./schemas/workestrate.schema.json"
include = ["config-repos/*/workestrate.toml"]
```

---

## 3. Schema strategy

- **Vendored into the binary via `include_str!`.** The JSON schema becomes a
  new const `include_str!("../../../schemas/workestrate.schema.json")`
  alongside the existing template consts at
  `control/agentctl/src/scaffold/mod.rs:48-54`, so **template and schema
  versions match at compile time**. Freshness is guarded by
  `control/agentctl/tests/schema_drift.rs` — a stale vendored copy fails CI.
- **Scaffold emits a static copy.** `config new` writes
  `schemas/workestrate.schema.json` into the config repo (via `render_all`,
  `scaffold/mod.rs:102`), so the schema directive below resolves offline
  against a file the repo owns.
- **`#:schema` directive switches to a repo-relative path** at all 3 sites
  (verified), replacing the floating `raw.githubusercontent.com` URL:
  - native template `control/agentctl/src/scaffold/template/workestrate.toml.tpl:1`,
  - copier jinja `templates/workestrate-config/workestrate.toml.jinja:1`,
  - the `--empty` inline string at `control/agentctl/src/commands/config_cmd.rs:390`
    (plus its assertion in `scaffold/mod.rs:295`).

  New value at each site: `#:schema ./schemas/workestrate.schema.json`.
- **`[[schemas]]` config entries** with repo-relative `path`/`include` in each
  `tombi.toml` (§2) back the directive for files that don't carry it
  (`config.reference/workestrate.toml` in this repo).
- **Sandbox purity:** `--offline` flag / `TOMBI_OFFLINE=1` env — validation
  never fetches a schema over the network (the floating-URL directive would
  have; the relative one cannot).
- **Draft-07 `#/definitions` refs resolve fine** — no schema rewrite needed.
- **Strictness:** `strict = true` is the default and our schema's
  `additionalProperties: false` turns unknown keys into errors — the desired
  posture (mirrors the Rust types' `deny_unknown_fields`). `tombi lint` runs
  the validation.
- **LSP bonus:** the Zed/VSCode tombi extensions pick up the `#:schema`
  directive for inline validation while editing.

---

## 4. Hooks

No upstream pre-commit hook exists for tombi; the idiom is a custom shell
hook. Both hooks are `command -v`-guarded.

### 4.1 Config-repo pre-commit hook (new const)

A **NEW const** in the scaffold — do **NOT** reuse `HOME_PRE_COMMIT_HOOK`:
its store-dir guard is the wrong domain (home hygiene, not TOML gating).
Installed right after the git-init block at `config_cmd.rs:437-456`.

Content sketch:

```sh
#!/bin/sh
# tombi TOML gate — version-pinned; fail closed if tombi present but wrong.
TOMBI_REQUIRED=1.2.5

if ! command -v tombi >/dev/null 2>&1; then
  echo "tombi not found; skipping TOML gate (install tombi $TOMBI_REQUIRED)" >&2
  exit 0
fi

version="$(tombi --version | awk '{print $NF}')"
if [ "$version" != "$TOMBI_REQUIRED" ]; then
  echo "tombi $version found; required $TOMBI_REQUIRED" >&2
  exit 1
fi

tombi format --check || exit 1
tombi lint --error-on-warnings || exit 1
```

### 4.2 Home-hook amendment (warn-not-fail)

Append a tombi gate to `HOME_PRE_COMMIT_HOOK`
(`control/agentctl/src/commands/home.rs:32`; installed by
`install_pre_commit_hook` at `home.rs:656-664`). Unlike the config-repo hook,
the home gate is **warn-not-fail** when tombi is absent: the home must stay
committable on a machine without the toolchain. When tombi IS present it runs
the same `format --check` + `lint --error-on-warnings` pair.

### 4.3 Hook tests

Tests follow the `control/agentctl/tests/cmd_home_init.rs` 3-layer pattern:

1. **exists + executable** (see `init_creates_structure_gitignore_and_hook`
   at `cmd_home_init.rs:45`);
2. **substring content assertions** (the const's text appears in the written
   hook — `TOMBI_REQUIRED=1.2.5`, `format --check`, `--error-on-warnings`);
3. **execute-the-hook** (see `hook_rejects_gitlink` at `cmd_home_init.rs:171`)
   — run the generated `sh` script against a staged tree with a malformed
   TOML and assert it fails.

---

## 5. Scaffolding changes

### 5.1 Native path (`config new`)

- New `include_str!` consts at `scaffold/mod.rs:48-54` for `tombi.toml` and
  the schema (`../../../schemas/workestrate.schema.json`).
- `pub fn render_all` (`scaffold/mod.rs:102`) emits the new artifacts:
  `tombi.toml`, `schemas/workestrate.schema.json`, `.git/hooks/pre-commit`.
- `#:schema` directive switch at the 3 sites (§3).
- Follow-through: update the `--empty` assertion at `scaffold/mod.rs:295`.

### 5.2 Copier template

`templates/workestrate-config/` gets **static copies** of `tombi.toml` +
`schemas/workestrate.schema.json` (the jinja `workestrate.toml.jinja:1`
directive switches as above), plus a **parity-overlap test extension** —
the existing native-vs-copier parity test extends to assert the new files
match.

---

## 6. Flake/devshell integration

- **`nix/packages/tombi.nix` (new):** `fetchurl` of the official musl binary
  for v1.2.5 with a pinned sha256, `autoPatchelfHook` +
  `installPhase` — mirroring the `nix/packages/microsandbox.nix`
  fetchurl-prebuilt pattern. Build gate is `HOST-NIX` (`nix build .#tombi`).
- **Devshell:** add the package to the devshell `packages` list so
  `nix develop` puts `tombi` on PATH.
- **`lib.checks.tombiCheck`:** the core flake has no `checks.${system}`
  output; the exported-function pattern `lib.${system}.checks.validateConfig`
  at `flake.nix:140-154` is the one to mirror for a new
  `lib.checks.tombiCheck`.
- **`scripts/check-toml.sh` (new):** version guard + `tombi format --check` +
  `tombi lint --error-on-warnings` over the repo include set (§2.1),
  `command -v`-guarded.
- **just recipe:** a `tombi-check` recipe wired into `just verify`
  (`justfile:71`), mirroring the `lint-nix` recipe (`justfile:337`).

---

## 7. Acceptance criteria

This spec is **IN-FLIGHT** — the checklist below is the acceptance bar for
the implementation wave, not a record of completed work.

- [ ] `nix build .#tombi` succeeds (`HOST-NIX`).
- [ ] `tombi format --check` clean on the repo include set (§2.1).
- [ ] `tombi lint` clean on the repo include set.
- [ ] Schema validation negative tests: a planted **unknown key** AND a
      **bad secret type** in `config.reference/workestrate.toml` are both
      caught by `tombi lint` (then reverted).
- [ ] `config new` emits `tombi.toml` + `schemas/workestrate.schema.json` +
      the pre-commit hook (3-layer hook tests green per §4.3).
- [ ] `home init` hook includes the tombi gate (warn-not-fail when tombi
      absent; substring + execute-sh tests green).
- [ ] `just verify` passes with `tombi-check` wired in (`justfile:71`).

---

## 8. Non-goals

- **No restructuring enforcement.** tombi cannot and will not force map form
  vs array-of-tables (§1.1) — the 13/14 notation stays author-chosen.
- **No cargo-install path.** The crates.io `tombi-cli` 0.0.1 placeholder is a
  rejected install vector (§1.2).
- **No format-rule forcing** of any particular layout beyond tombi defaults
  (the 19 lexical rules).
- **No `schema_version` change** — this is tooling, not a config-format
  change.

---

## 9. ADR note

Additive to [ADR 0002](../../migration/50-decisions/0002-toml-config-format.md)
(TOML config format) and [ADR 0022](../../migration/50-decisions/0022-config-repo-scaffolding.md)
(config-repo scaffolding) — a toolchain adoption, no vocabulary expansion, no
new ADR required. When the implementation wave lands, an **ADR 0022 addendum
line** will record the template additions (`tombi.toml` + `schemas/` +
pre-commit hook).

---

## 10. Effort & gate

| Item | Value |
|---|---|
| Effort | **M** (one new nix package, three `tombi.toml` designs, two hook surfaces, scaffold emission at 3 sites, flake/scripts/just wiring, hook tests). |
| Gate | Devshell in-container: cargo gates + `scripts/check-toml.sh` via `nix develop` (`verifiable-here`); `HOST-NIX` for the `nix build .#tombi` package build. No KVM. |
| Files touched | `nix/packages/tombi.nix` (new); devshell packages list; `flake.nix:140-154` (mirror for `lib.checks.tombiCheck`); `scripts/check-toml.sh` (new); `justfile:71` + `justfile:337` (recipe pattern); `control/agentctl/src/scaffold/mod.rs:48-54,102,295`; `control/agentctl/src/scaffold/template/workestrate.toml.tpl:1`; `templates/workestrate-config/workestrate.toml.jinja:1` (+ static copies + parity test); `control/agentctl/src/commands/config_cmd.rs:390,437-456`; `control/agentctl/src/commands/home.rs:32,656-664`; `control/agentctl/tests/cmd_home_init.rs` (3-layer pattern); `control/agentctl/tests/schema_drift.rs` (freshness guard). |
| Risk | Low–moderate. Additive toolchain; the only foreign input is the pinned-binary fetch (`fetchurl` + sha256). No config-format change, no schema_version bump, no downstream code-path change. |
