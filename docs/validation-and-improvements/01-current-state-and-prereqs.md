# 01 — Current State and Prerequisites

> **STATUS: READY-TO-EXECUTE**
> Prerequisites / see-also: [README.md](README.md) · [00-overview.md](00-overview.md) ·
> [03-sibling-config-setup.md](03-sibling-config-setup.md) · [04-baseline-validation.md](04-baseline-validation.md)

This document is the verified current-state snapshot and prerequisites checklist
for the validation-and-improvements effort. A contextless session reads this to
know exactly what exists today, what is broken or drifted, and what must be true
before executing anything else in this tree.

> Every claim below cites a file:line read during this session. The codebase is
> on branch `migration/tool-model` (verified: `git branch --show-current` →
> `migration/tool-model`).

## Environment honesty

This documentation was authored in a container with the following capabilities
and limits (mirroring the convention in
[`docs/migration/README.md`](../migration/README.md) §Environment honesty):

| Capability | Present here | Notes |
|---|---|---|
| `nix` on PATH | **No (but installed)** | `command -v nix` → not found on PATH, but nix IS installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`. Usable via `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c '<cmd>'` (first devshell build takes minutes; subsequent runs are fast). |
| KVM (`/dev/kvm`) | **No** | `ls /dev/kvm` → not found. No sandbox runtime can execute. |
| `sops` on PATH | **No** | `command -v sops` → not found. |
| SOPS age key | **No** | `~/.config/sops/age/` absent; `SOPS_AGE_KEY` unset. Secrets cannot be decrypted here. |
| `cargo` / `rustc` | Yes | Toolchain-check gate parses `RUST_TOOLCHAIN_VERSION` from `flake.nix` (`justfile:15`). |
| `python3` + PyYAML | Variable | `just litellm-check` falls back to `nix develop -c python3` when PyYAML is absent (`justfile:51-61`). |

### Environment markers

Reproduced from [`docs/migration/40-migration-process.md`](../migration/40-migration-process.md)
lines 8-14:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). INCLUDES cargo-linked gates run via `nix develop` (nix is installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH — prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c '<cmd>'`; verified 2026-07-29: `nix develop -c bash -c 'cc --version'` → gcc 15.2.0, `cargo check` compiles in ~27s). A bare shell (outside `nix develop`) has no `cc`. |
| `HOST-NIX` | Requires nix on the user's host: `nix build` image builds, `nix run nixpkgs#...` prefetch jobs, full `just verify-full`, and `just generate-schema` (devshell RUSTFLAGS/libcap-ng). Cargo-linked `just` gates are NOT here — they run in-container via `nix develop` (see `verifiable-here`). |
| `HOST-KVM` | Requires KVM on the user's host (this container has no KVM) |

### Devshell / vendor facts

The devshell (`nix/devshells/default.nix:60`) packages `libcap_ng` as a build
input — the `aws-lc-rs` / `parking_lot_core` native crates require it. The
`generate-schema` justfile recipe documents this dependency explicitly
(`justfile:95-96`):

> Run on a nix-capable host (the dev shell's RUSTFLAGS → libcap-ng OUT lib dir
> is required to build the aws-lc-rs / parking_lot_core native crates).

The `nix/packages/agentctl.nix` derivation (`buildInputs` at line 55-57) lists
`libcap_ng` and stages the microsandbox vendor symlink in `preBuild`
(`agentctl.nix:59-65`):

```
ln -sfn "${microsandbox-filesystem-patched}" vendor/microsandbox-filesystem-0.5.6
```

The `just vendor-unlock` recipe (`justfile:156-168`) replaces this Nix-managed
symlink at `control/agentctl/vendor/microsandbox-filesystem-0.5.6` with a
writable copy for local editing; `just vendor-lock` (`justfile:171-184`) reverts
it. This vendor symlink is the ADR 0011 open item (deferred; see
[`docs/migration/40-migration-process.md`](../migration/40-migration-process.md)
§Phase 0b Status, lines 50-56).

## Bundle inventory (verified)

The workestrate tool home is at `/home/node/Development/ai-workbench/.workestrate/`
(resolved via `WORKESTRATE_HOME` — see §WORKESTRATE_HOME resolution below).
Verified directory listing:

```
.workestrate/
├── config.toml          # registry: layers, settings, configs.personal, trusted_projects
├── repos/
│   └── personal/        # managed config-repo clone (the personal layer)
├── secrets/             # empty (no .env.local.enc here — HOST-only)
├── sources/             # empty (no agent source checkouts here — HOST-only)
├── state/
│   ├── var/             # runtime var
│   └── workspaces/      # per-workload state mounts
└── scratch/             # empty — NOT in ADR 0023 layout (see §Bundle fixes)
```

### `.workestrate/config.toml` (verified, 15 lines)

Cited from `.workestrate/config.toml`:

| Field | Value | Line |
|---|---|---|
| `layers` | `["personal"]` | 1 |
| `[settings] default_context` | `"personal"` | 4 |
| `[settings] home_version` | `2` | 5 |
| `[configs.personal] url` | `/home/node/Development/ai-workbench/.workestrate/repos/personal` | 8 |
| `[configs.personal] ref` | `"main"` | 9 |
| `[configs.personal] rev` | `d2cd0c3506b5641507883078d01b626368f9d163` | 10 |
| `[[trusted_projects]] path` | `/home/node/Development/ai-workbench` | 15 |

### Registry/clone branch mismatch (RESOLVED — fix a applied)

**RESOLVED (fix a applied):** the clone at `.workestrate/repos/personal` is now
on branch **`main`** (verified: `git -C .workestrate/repos/personal
branch --show-current` → `main`), HEAD `d2cd0c3` unchanged (matches the
registry `rev` at `config.toml:10`). The registry needed **no edit** —
`config.toml:9` already declared `ref = "main"`. The fix was a plain
`git branch -m master main` (see [06-improvements/02](06-improvements/02-main-standardization.md)).

**Was:** the registry declared `ref = "main"` (`config.toml:9`) but the clone
was on branch `master` — a registry/clone mismatch (see §Bundle fixes (a)
below).

**Still pending:** the clone has **NO origin remote** (verified:
`git -C .workestrate/repos/personal remote -v` → empty), so
`workestrate config update` fails for that reason until the config repo is
pushed to a remote.

## The 5 workloads

All workload definitions are in
`.workestrate/repos/personal/workestrate.toml` (359 lines, verified). The
table below summarizes each workload; line citations refer to that file.

### litellm

| Field | Value | Lines |
|---|---|---|
| `kind` | `service` | 51 |
| `image` | `recipe = "registry"`, `ref = "ghcr.io/berriai/litellm:v1.89.4"` | 52 |
| `ports` | host 4000 → guest 4000 | 83-85 |
| `mounts` | `${MSB_HOME}/sandboxes/litellm/logs` → `/var/log/litellm` (rw); `infra/litellm` → `/app/config` (ro) | 87-95 |
| `env` (secret) | `LITELLM_MASTER_KEY` (secret=) | 67-69 |
| `secret_env` | `OPENROUTER_API_KEY`, `KIMI_CODE_API_KEY`, `NEURALWATT_API_KEY`, `MINIMAX_CODING_API_KEY` | 71-81 |
| `network` | `default_deny = true`; egress `dns` + `https` to provider hosts; ingress tcp/4000 local | 97-110 |
| `local_build` | none | — |
| `seed_files` | none | — |

### pi

| Field | Value | Lines |
|---|---|---|
| `kind` | `agent` | 113 |
| `image` | `recipe = "nix-layered"`, `name = "workestrate-pi"`, `tag = "latest"`, `contents = ["cacert","busybox","fakeNss"]`, `binary = { recipe = "bun-compile", src = "flake://pi", ... }`, `features = ["create_tmp"]` | 114 |
| `ports` | none | — |
| `mounts` | `workspaces/pi-state` → `/data` (rw); `${CWD}` → `/work` (rw) | 136-144 |
| `env` (secret) | `LITELLM_MASTER_KEY` (secret=) | 129-131 |
| `secret_env` | `GITHUB_TOKEN` | 133-134 |
| `network` | `default_deny = true`; egress `agent_base`; deny `.pi.dev` | 146-153 |
| `local_build` | none (image is nix-built) | — |
| `seed_files` | `agents/pi/config/models.json` → `workspaces/pi-state/agent/models.json` (only_if_missing) | 155-158 |

### odysseus

| Field | Value | Lines |
|---|---|---|
| `kind` | `service` | 161 |
| `image` | `recipe = "registry"`, `ref = "python:3.12-slim"` | 162 |
| `ports` | host 7000 → guest 7000 | 207-209 |
| `mounts` | `${WORKESTRATE_ODYSSEUS_BUILD}` → `/app` (ro); `workspaces/odysseus-state` → `/data` (rw) | 211-219 |
| `env` (secret) | `ODYSSEUS_ADMIN_PASSWORD` (secret=) | 197-199 |
| `secret_env` | `LITELLM_AUTH`, `GITHUB_TOKEN` | 201-205 |
| `network` | `default_deny = true`; egress `agent_base` + `https` to huggingface.co + cdn-lfs; ingress tcp/7000 local | 221-234 |
| `local_build` | `recipe = "pip-install"`, `source = "flake://odysseus"`, `requirements_file = "requirements.txt"`, `target = ".deps"`, `env_override = "WORKESTRATE_ODYSSEUS_BUILD"`, `fallback = "agents/odysseus/build"` | 241-248 |
| `seed_files` | `agents/odysseus/config/settings.json` → `workspaces/odysseus-state/settings.json` (only_if_missing) | 236-239 |

### opencode

| Field | Value | Lines |
|---|---|---|
| `kind` | `agent` | 251 |
| `image` | `recipe = "registry"`, `ref = "node:24-bookworm-slim"` | 252 |
| `ports` | host 3000 → guest 3000 | 273-275 |
| `mounts` | `${WORKESTRATE_OPENCODE_BUILD}` → `/app` (ro); `${CWD}` → `/workspace` (rw); `agents/opencode/config/opencode.jsonc` → `/home/node/.config/opencode/opencode.jsonc` (ro); `${MSB_HOME}/sandboxes/opencode/state` → `/home/node/.local/share/opencode` (rw) | 277-295 |
| `env` (secret) | none (plain env only) | — |
| `secret_env` | `LITELLM_AUTH`, `GITHUB_TOKEN` | 267-271 |
| `network` | `default_deny = true`; egress `agent_base`; ingress tcp/3000 local | 297-306 |
| `local_build` | `recipe = "bun-install"`, `source = "flake://opencode"`, `gating_file = "bun.lock"`, `env_override = "WORKESTRATE_OPENCODE_BUILD"`, `fallback = "agents/opencode/build"` | 308-313 |
| `seed_files` | none | — |

### tempest

| Field | Value | Lines |
|---|---|---|
| `kind` | `agent` | 316 |
| `image` | `recipe = "nix-layered"`, `name = "tempest"`, `tag = "latest"`, `contents = ["cacert","busybox","fakeNss","nodejs_24","nmap","dnsutils"]`, `binary = { recipe = "npm-build", src = "flake://tempest", npm_deps_hash = "sha256-AAAA...AAA=" }` (the `install_layout = "app"` field previously in this inline table has been REMOVED — fix e applied; see §"Bundle fixes needed" item e), `baked_files = [...]`, `features = ["create_tmp"]` | 317 |
| `ports` | none | — |
| `mounts` | `workspaces/tempest-state` → `/data` (rw); `${CWD}` → `/work` (rw) | 340-348 |
| `env` (secret) | `TEMPEST_LOCAL_API_KEY` (secret=`LITELLM_MASTER_KEY`) | 332-334 |
| `secret_env` | none | — |
| `network` | `default_deny = false` (tempest holds the `DEFAULT_DENY_FALSE_ENTITLEMENT`) | 350-351 |
| `local_build` | `recipe = "npm-build"`, `source = "flake://tempest"`, `gating_file = "package-lock.json"`, `env_override = "WORKESTRATE_TEMPEST_BUILD"`, `fallback = "sources/tempest/build"` | 353-358 |
| `seed_files` | none | — |

## Bundle fixes needed

The known drift between the bundle and the spec/ADR layout. Each item is
flagged with severity.

| # | Drift | Severity | Detail / fix | Env |
|---|---|---|---|---|
| a | `ref = "main"` vs actual branch `master` | Medium | **APPLIED.** `config.toml:9` declares `ref = "main"`; the clone at `.workestrate/repos/personal` was on branch `master` and is now renamed to `main` (verified: `git branch --show-current` → `main`), HEAD `d2cd0c3` unchanged. The registry needed **no edit** — `ref` was already `"main"`. See [06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md). **Still pending:** the clone has NO origin remote (verified: `git remote -v` → empty), so `workestrate config update` fails for that reason until the config repo is pushed to a remote. | `verifiable-here` |
| b | `scratch/` directory not in ADR 0023 layout | Medium | `.workestrate/scratch/` exists (empty) but [ADR 0023](../migration/50-decisions/0023-single-tool-home.md) lines 73-82 specify the single-home layout as: `config.toml`, `overrides.toml`, `secrets/`, `repos/`, `sources/`, `state/`, `cache/`. `scratch/` is **not** in the spec. Either `scratch/` is renamed to `cache/` or the ADR layout is amended. **Open decision** — flag for resolution. | `verifiable-here` |
| c | tempest `npm_deps_hash` placeholder | High | `workestrate.toml:317` has `npm_deps_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="` — a placeholder FOD hash. The real hash must be computed on a nix-capable host (`nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`; see `justfile:233-234`). Until computed, `nix build .#tempest` will fail. | `HOST-NIX` |
| d | `ODYSSEUS_ADMIN_PASSWORD` placeholder | High | `workestrate.toml:47` declares `placeholder = "change_me_before_first_boot"` for the `ODYSSEUS_ADMIN_PASSWORD` secret. This must be replaced with a real secret (via SOPS) before first boot. `AUTH_ENABLED` is on by default (`workestrate.toml:174-175`). | `HOST-KVM` (runtime); secret provisioning is HOST-only (sops age key absent here) |
| e | tempest `install_layout` config/schema drift | **CRITICAL** | **APPLIED.** `.workestrate/repos/personal/workestrate.toml:317` previously set `install_layout = "app"` inside tempest's `binary` inline table. `BinarySpec` (`control/agentctl/src/config/types.rs:46-52`) has `#[serde(deny_unknown_fields)]` (`types.rs:44`) and **no `install_layout` field** — the nix-side `installLayout` param was REMOVED as a silent no-op (`nix/lib/recipes/npm-build.nix:16-25`). The field has been **REMOVED** from the bundle config (verified: `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no match). **PENDING:** runtime parse verification — `workestrate validate-config` is cargo-linked and runs in this container via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`; verified 2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`); it is the **first Lane A action** (`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c 'just workestrate validate-config'`). | `verifiable-here` (config edit applied; runtime parse verification is `verifiable-here` via `nix develop`) |

## Tooling facts

### CLI invocation

The workestrate CLI is invoked via the justfile passthrough recipe
(`justfile:135-136`):

```
just workestrate <args>
  →  cargo run --manifest-path control/agentctl/Cargo.toml -- <args>
```

The real CLI surface (verified from the codebase) includes: `check`, `init`,
`new`, `completions`, `run`, `validate-config`, `secrets-schema`,
`generate-env-example`, `ps`, `down-all`, `clean`, `context {list,current}`,
`generate-schema`, `config {add,update,list,trust,untrust,new,remove}`,
`secrets-target`, `doctor`, `source {clone,build,list,reset}`, typed workload
subcommands, `migrate-home`, catch-all workload dispatch, and a global
`--json` flag.

Environment variables: `WORKESTRATE_HOME`, `WORKESTRATE_CONFIG_DIR`,
`AGENTCTL_ROOT`, `WORKESTRATE_NO_PROJECT_CONFIG`.

### microsandbox crate pin

`control/agentctl/Cargo.toml:17`:

```toml
microsandbox = { version = "=0.5.6", features = ["net"] }
```

The `=0.5.6` exact pin matches the vendored `microsandbox-filesystem-0.5.6`
symlink managed by the Nix derivation (`agentctl.nix:61`).

### Golden-check scope

`just golden-check` (`justfile:86-91`) loops **only** over the three synthetic
example workloads — `example-service`, `example-agent`, `example-offensive` —
with `WORKESTRATE_CONFIG_DIR=config.reference`. It does **not** exercise the
five real workloads (litellm/pi/odysseus/opencode/tempest); those require
HOST-NIX image builds and HOST-KVM runtime.

### `just verify` composition

`just verify` (`justfile:71-72`) runs:

```
toolchain-check → check → test → spec-examples → litellm-check →
golden-check → schema-check → scaffold-check → lint-nix → store-audit
```

followed by a `git diff --exit-code HEAD -- control/agentctl/Cargo.lock`
stability check (`justfile:72`).

`just verify-full` (`justfile:75-76`) adds `nix build .#workestrate` (HOST-NIX).

## Gates status snapshot

### Gates classification (verified against `justfile`)

A **bare shell** in this container has no C toolchain (verified:
`command -v cc gcc` → not found; `cargo` exists at `~/.cargo/bin/cargo` but
cannot link). However, nix IS installed at
`/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` (not on PATH),
and `nix develop` provides a full C toolchain (verified 2026-07-29:
`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
then `nix develop -c bash -c 'cc --version'` → gcc 15.2.0; `cargo 1.97.1`,
`rustc 1.97.1`; `cargo check` compiles in ~27s; first devshell build takes
minutes, subsequent runs are fast). Therefore **every cargo-linked gate is
runnable in this container via `nix develop`** (store-path PATH prefix). The
shell/python/git-based gates run in a bare shell directly.

| Gate | Recipe | Status |
|---|---|---|
| Toolchain check | `just toolchain-check` (`justfile:12-32`) | Pass (shell/grep + `rustc --version`; rustc on PATH) |
| Format + clippy + check | `just check` (`justfile:34-37`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| Unit tests | `just test` (`justfile:132-133`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| Spec examples parse | `just spec-examples` (`justfile:65-66`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| LiteLLM config check | `just litellm-check` (`justfile:46-61`) | Pass (python3 + PyYAML path; falls back to `nix develop -c python3`) |
| Golden check | `just golden-check` (`justfile:86-91`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| Schema drift | `just schema-check` (`justfile:106-107`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| Scaffold template | `just scaffold-check` (`justfile:114-115`) | Runnable here via `nix develop` (store-path prefix; bare shell lacks `cc`) |
| Nix purity lint | `just lint-nix` (`justfile:343-344`) | Pass (runs `scripts/check-nix-paths.sh`) |
| Store audit | `just store-audit` (`justfile:266-291`) | SKIP (nix not on PATH — non-blocking, `justfile:273-275`; runnable via the store-path prefix if desired) |
| Cargo.lock stability | `git diff --exit-code` (`justfile:72`) | Pass |

**`just verify` overall: runnable in this container via `nix develop`** (the
cargo-linked gates run with the store-path PATH prefix +
`nix develop -c bash -c '<cmd>'`; the shell/python/git-based subset
— `toolchain-check`, `litellm-check`, `lint-nix`, `store-audit` (SKIP), and
the `Cargo.lock` stability `git diff` — passes in a bare shell).

### `HOST-NIX` (deferred to a nix-capable host)

| Gate | Command | Notes |
|---|---|---|
| Nix build (CLI) | `nix build .#workestrate` | `just verify-full` (`justfile:75-76`) |
| Image builds | `nix build .#workestrate-pi`, `.#tempest`, etc. | nix-layered image recipes |
| FOD hash computation | `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json` | tempest `npm_deps_hash` (Bundle fix c) |
| Schema generation | `just generate-schema` (`justfile:97-101`) | requires devshell RUSTFLAGS/libcap-ng |

### `HOST-KVM` (deferred to a KVM-capable host)

| Gate | Command | Notes |
|---|---|---|
| LiteLLM runtime | `workestrate litellm up` | Phase 1 step 1.11 / M.12 (`40-migration-process.md:103,230`) |
| Agent exec | `workestrate pi exec` | Phase 1 step 1.11 / M.12 |
| Odysseus first boot | `workestrate odysseus up` | requires real `ODYSSEUS_ADMIN_PASSWORD` (Bundle fix d) |

## Prerequisites checklist

Before executing [03-sibling-config-setup.md](03-sibling-config-setup.md),
[04-baseline-validation.md](04-baseline-validation.md), or
[05-host-validation.md](05-host-validation.md):

- [ ] Repo is on branch `migration/tool-model` (verified: `git branch --show-current` → `migration/tool-model`).
- [ ] `.workestrate/` bundle is present at repo root (verified: `config.toml`, `repos/personal/`, `state/{var,workspaces}`, `secrets/`, `sources/`, `scratch/`).
- [ ] `.workestrate/config.toml` is well-formed: `layers=["personal"]`, `default_context="personal"`, `home_version=2`, `configs.personal.ref="main"`, `rev=d2cd0c3506b5641507883078d01b626368f9d163`, `trusted_projects=[/home/node/Development/ai-workbench]` (verified, lines 1-15).
- [ ] Secrets can be decrypted — **HOST-only**. This container has no sops age key (`~/.config/sops/age/` absent, `SOPS_AGE_KEY` unset, `sops` not on PATH). Secret provisioning and decryption must happen on a host with the age key.
- [ ] No KVM here — runtime steps (`workestrate litellm up`, `workestrate pi exec`, odysseus first boot) are **deferred** to [05-host-validation.md](05-host-validation.md) on a KVM-capable host.
- [ ] `just verify` is runnable in this container via `nix develop` (nix installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; verified 2026-07-29: `nix develop -c bash -c 'cc --version'` → gcc 15.2.0, `cargo check` compiles in ~27s). Prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c 'just verify'` (first devshell build takes minutes). A bare shell lacks `cc` and runs only the shell/python/git subset (`toolchain-check`, `litellm-check`, `lint-nix`, `store-audit` SKIP, `Cargo.lock` stability). `just verify-full` (adds `nix build .#workestrate`) remains HOST-NIX.
- [ ] tempest `npm_deps_hash` placeholder is understood — the real FOD hash must be computed on a nix-capable host before `nix build .#tempest` will succeed (Bundle fix c, `HOST-NIX`).
- [ ] `ODYSSEUS_ADMIN_PASSWORD` placeholder is understood — must be replaced with a real SOPS secret before odysseus first boot (Bundle fix d, `HOST-KVM` runtime).
- [x] tempest `install_layout` drift is FIXED — the field has been REMOVED from tempest's `binary` table in `.workestrate/repos/personal/workestrate.toml:317` (verified: `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no match). `BinarySpec` has `deny_unknown_fields` and no such field (`types.rs:44-52`); the nix-side param was removed as a silent no-op (`nix/lib/recipes/npm-build.nix:16-25`). **PENDING:** runtime parse verification (`workestrate validate-config`) is the **first Lane A action**, runnable in this container via `nix develop` (store-path prefix; bare shell lacks `cc`).

### WORKESTRATE_HOME resolution

`WORKESTRATE_HOME` is resolved in `control/agentctl/src/config/paths.rs`. The
precedence (ADR 0023, `paths.rs:92-119`) is:

1. **Env** — `WORKESTRATE_HOME` (used verbatim, `~/` expanded) — `paths.rs:106-110`.
2. **Discovered** — a `.workestrate/config.toml` in a *trusted* ancestor of cwd, only when no `XDG_*_HOME` var is set — `paths.rs:116-119`.
3. **Legacy XDG** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME` set (compat, read-only + deprecation note).
4. **Default** — `~/.workestrate`.

In this container, `WORKESTRATE_HOME` is set to `$PWD/.workestrate` (via
`.envrc` / `scripts/local-xdg.sh`, per ADR 0023 §`.envrc` collapse, lines
117-123), so precedence step 1 applies and the bundle at
`/home/node/Development/ai-workbench/.workestrate/` is the active tool home.
