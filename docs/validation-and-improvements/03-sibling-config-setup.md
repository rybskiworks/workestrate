# 03 — Sibling Config Setup (Disposable Experiment Home)

> **STATUS: READY-TO-EXECUTE (verifiable-here; no KVM needed for plan/config-plane probes)**
> Prerequisites / see-also: [README.md](README.md) · [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) · [02-config-requirements.md](02-config-requirements.md) · [04-baseline-validation.md](04-baseline-validation.md)

This document establishes a **disposable workestrate tool home** for
destructive experiments — scaffolding throwaway config repos, layering probes,
context probes, and secret-layering probes — that **cannot corrupt the real
bundle** at `/home/node/Development/ai-workbench/.workestrate/`. It defines the
3-home topology, the exact bootstrap commands, and the probe workload set that
[04-baseline-validation.md](04-baseline-validation.md) and
[05-host-validation.md](05-host-validation.md) build on.

A contextless reader should know two things up front:

- **What a tool home is.** The workestrate *tool home* is the directory holding
  `config.toml` (the registry), `repos/` (managed config-repo clones — being
  renamed to `config-repos/` per spec 10; both spellings cited below),
  `secrets/`, `sources/`, and `state/`. It is resolved by
  `resolve_home_with_kind()` (`control/agentctl/src/config/paths.rs:104-150`).
  Setting `WORKESTRATE_HOME` to a throwaway path isolates ALL registry writes,
  repo clones, and state from the real bundle.
- **Why a sibling home is needed.** The probes below edit `workestrate.toml`,
  define contexts, and toggle `secrets = "none"` — operations that must NEVER
  touch the real `.workestrate/config.toml` or the real personal config repo.
  A disposable home under `/tmp` makes corruption impossible by construction.

> Every claim below cites a file:line read during this session. The codebase is
> on branch `migration/tool-model` (verified: `git branch --show-current` →
> `migration/tool-model`).

---

## 1. The 3-home topology

> **AMENDED by spec 10:** config-repo DEVELOPMENT happens in the home's
> `config-repos/` working copies (or `repos/` until the spec-10 rename lands),
> per spec 10 Decision A — the home clones are first-class working repos
> (edit/commit/push directly; remote is canonical), NOT read-only managed
> clones. The mount set for dev agents becomes: home ro at the default path +
> `config-repos/` rw shadow + dev home rw. See
> [06-improvements/10-config-repos-as-working-copies.md](06-improvements/10-config-repos-as-working-copies.md).

Workestrate resolves its tool home with a 4-step precedence
(`paths.rs:104-150`, ADR 0023):

1. **Env** — `WORKESTRATE_HOME` (used verbatim, leading `~/` expanded) →
   `HomeKind::Env` (`paths.rs:106-110`).
2. **Discovered** — a `.workestrate/config.toml` in a *trusted* ancestor of
   the cwd, only when no `XDG_*_HOME` var is set (`paths.rs:116-139`).
3. **Legacy XDG** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
   set (`paths.rs:142-145`).
4. **Default** — `~/.workestrate` (`paths.rs:148-149`).

The three homes used across this doc set:

| Home | Path | Purpose | Mutability | Lifetime |
|---|---|---|---|---|
| **REAL bundle** | `/home/node/Development/ai-workbench/.workestrate/` | The live personal deployment: 5 workloads, 8 secrets, real state. | Read-mostly (never written by any probe here) | Persistent (repo-tracked) |
| **EXPERIMENT home** | `/tmp/workestrate-exp` (via `WORKESTRATE_HOME=/tmp/workestrate-exp`) | Disposable home for scaffold/layering/context probes. All registry writes, repo clones, and state land here. | Read-write (probes may edit freely) | Ephemeral (deleted after probes) |
| **Host end-state default** | `~/.workestrate` | The default home when no env and no discovery (`paths.rs:148-149`). Not used in this doc set, but is the production end-state after `workestrate migrate-home` (ADR 0023). Config-repo development happens in its `config-repos/` working copies (spec 10 Decision A; `repos/` until the rename lands). | Read-write | Persistent (host-local) |

> **Invariant:** the REAL bundle is touched ONLY by read commands
> (`config list`, `check`, `plan`, `validate-config`). Every probe that writes
> (scaffold, context edit, `secrets = "none"` toggle) runs against the
> EXPERIMENT home. See §5.

---

## 2. Bootstrap the experiment home

All commands below use the `just workestrate` passthrough recipe
(`justfile:135-136`), matching the spelling in
[04-baseline-validation.md](04-baseline-validation.md):

```makefile
workestrate *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}
```

The recipe does NOT set `WORKESTRATE_HOME` (`justfile:135-136`); the env var
must be exported or prefixed on the command line.

> **Guard suggestion.** Before every experiment session, verify the env is
> pointed at the throwaway home — never at the real bundle:
>
> ```sh
> echo $WORKESTRATE_HOME   # MUST print /tmp/workestrate-exp, NOT the repo path
> ```

### Step 1 — Export the experiment home

```sh
export WORKESTRATE_HOME=/tmp/workestrate-exp
```

This makes `resolve_home_with_kind()` return `HomeKind::Env` with home
`/tmp/workestrate-exp` (`paths.rs:106-110`). All subsequent `just workestrate`
commands resolve the registry at `/tmp/workestrate-exp/config.toml`, the store
at `/tmp/workestrate-exp/repos/`, and state at `/tmp/workestrate-exp/state/`.

### Step 2 — Scaffold a throwaway config repo

The `workestrate config new` command (`cli_actions.rs:120-168`,
`config_cmd.rs:241-507`) scaffolds a new config repo. Two content modes:

| Flag | Content | When to use |
|---|---|---|
| `--empty` | Minimal `workestrate.toml` (header + `schema_version = 1` + `.gitignore` only). No secrets/sops/readme. (`config_cmd.rs:348-367`) | Probes that build up a config from scratch (P1–P5 below). |
| `--from-reference` | Seeds `workestrate.toml` from `config.reference/workestrate.toml` — the full 5-workload fixture. (`config_cmd.rs:377-388`) | Probes that need a realistic starting config to edit incrementally. |

**Recommended (simplest) — scaffold directly into the store:**

```sh
just workestrate config new exp --empty
```

This scaffolds into `<store>/repos/exp/` = `/tmp/workestrate-exp/repos/exp/`
(the default destination, `config_cmd.rs:276-280`) and **auto-registers** `exp`
as a local-path config repo (url = canonicalized path, `ref = None`,
`rev = None`, `config_cmd.rs:422-443`). No git remote, no commit, no branch
needed — local-path entries are skipped by `config update`
(`config_cmd.rs:531-539`). Layer resolution finds the repo at
`<store>/repos/exp/workestrate.toml` (`paths.rs:382-386`).

**Alternative — scaffold at a custom path, then clone into the store:**

```sh
# Scaffold at a custom path; --no-register avoids double-registration.
just workestrate config new exp --path /tmp/workestrate-exp-repo --empty --no-register

# The scaffold runs `git init` (git.rs:27-37) but makes NO commit. To use
# `config add` (which clones via `git clone --depth 1 --branch <ref>`), you
# must commit and ensure the branch matches the --ref default ("main"):
cd /tmp/workestrate-exp-repo
git add . && git commit -m init
git branch -m main   # `git init` with no -b flag may produce "master"

# Clone into the store and register with ref="main" (cli_actions.rs:105-106).
just workestrate config add /tmp/workestrate-exp-repo exp
```

> **Branch caveat (verified).** `git_init()` (`git.rs:27-37`) runs plain
> `git init` with **no** `-b`/`--initial-branch` flag. The resulting branch
> name depends on the git config `init.defaultBranch` (default `master` on
> older git, `main` on newer). `config add` defaults `--ref` to `"main"`
> (`cli_actions.rs:105-106`) and `git_clone()` runs
> `git clone --depth 1 --branch <ref>` (`git.rs:9-19`). If the throwaway
> repo's branch is `master`, `config add exp` fails with
> `git clone failed`. Either rename the branch (`git branch -m main`) or pass
> `--ref master`. This is the same drift as Bundle fix (a) in
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle
> fixes needed".

> **Filesystem path URLs work.** `cmd_config_add` calls
> `git_clone(url, &dest, Some(git_ref))` (`config_cmd.rs:205`), and `git clone`
> accepts a filesystem path as the source URL. No remote is required.

### Step 3 — Trust the experiment project dir (if probing project layers)

If you will probe the project-layer (`./workestrate.toml` in cwd) or
local-override (`./workestrate.local.toml`) layers, trust the experiment
project directory:

```sh
mkdir -p /tmp/workestrate-exp-proj
just workestrate config trust /tmp/workestrate-exp-proj
```

This adds `/tmp/workestrate-exp-proj` to `[[trusted_projects]]` in the
experiment registry (`config_cmd.rs:629-634`). Project-layer loading is
trust-gated (`loading.rs:335-384`, ADR 0014).

### Step 4 — Verify the experiment home is wired

```sh
just workestrate config list
just workestrate context current
just workestrate validate-config
just workestrate doctor
```

Expected outcomes (with `WORKESTRATE_HOME=/tmp/workestrate-exp`):

| Command | Expected output | Source |
|---|---|---|
| `config list` | `exp: <path> (ref main, rev unknown, clean) [OK]` (local-path entries show `ref main` as the display default, `config_cmd.rs:605`); `Layers: ["exp"]`; `Trusted projects:` lists `/tmp/workestrate-exp-proj` if trusted. | `cmd_config_list` (`config_cmd.rs:580-627`) |
| `context current` | `Active context: (none — using bare layers)` with `source: bare` (no contexts defined yet; `default_context` unset in the fresh experiment registry). | `cmd_context_current` (`config_cmd.rs:145-185`) |
| `validate-config` | `workestrate.toml is valid.` (the `--empty` scaffold has no workloads, so validation passes trivially). | `cmd_validate_config` (`diagnostics.rs:375`) |
| `doctor` | `=== workestrate doctor ===` then per-check lines. KVM/nix/sops absent in this container → overall `FAIL` (exit 1); the config-repo and home checks still pass. | `cmd_doctor` (`doctor.rs:218`) |

---

## 3. Context selection mechanism (verified)

Contexts are named layer-sets defined in the registry `[contexts.<name>]`
(`types.rs:267`, ADR 0019). The active context is resolved by
`resolve_active_context()` (`registry.rs:178-242`) with this precedence:

1. **`WORKESTRATE_CONTEXT` env** (set by the global `--context <name>` flag,
   `main.rs:36-37` + `main.rs:242-244`, OR by the user directly) —
   `registry.rs:198-215`. If set but not found in `[contexts]`, hard error.
2. **`[settings] default_context`** — `registry.rs:218-235`. If set but not
   found in `[contexts]`, hard error.
3. **Bare `layers` (backward compat)** — when NO contexts are defined at all
   (`registry.contexts.is_empty()`), returns `name: None, layers:
   registry.layers` — `registry.rs:190-195`.
4. **Error** — if contexts ARE defined but neither env nor `default_context`
   resolves — `registry.rs:238-241`.

`workestrate context current` (`config_cmd.rs:145-185`) reports the resolution
source: `env` (when `WORKESTRATE_CONTEXT` is set), `default` (when
`default_context` is set), or `bare` (no contexts defined).

> **CLI gap (documented plainly).** There is **no CLI command to create or
> define a context.** `ContextAction` exposes only `List` and `Current`
> (`cli_actions.rs:185-190`). `ConfigAction` has no `context-add`/`context-set`
> subcommand. Contexts are defined **only by hand-editing the registry
> `config.toml`** to add a `[contexts.<name>] layers = [...]` table. The
> future CLI authoring tool (DEFERRED, see
> [02-config-requirements.md](02-config-requirements.md) §9) is the intended
> non-manual path. For the probes below, hand-editing the experiment registry
> is the documented mechanism.

The REAL bundle's `.workestrate/config.toml` has `[contexts]` **empty**
(`config.toml:12`) with `default_context = "personal"` (`config.toml:4`). Per
the precedence above, the empty-contexts check (`registry.rs:190`) fires FIRST,
so the live bundle runs in **bare-layers backward-compat mode** (`name: None`,
`layers: ["personal"]`), NOT in a named context — despite `default_context`
being set.

---

## 4. The probe workload set

Each probe is a small throwaway `workestrate.toml` edit in the experiment
repo, exercised end-to-end through `plan` (the config-plane, no KVM needed).
Field names conform to
[02-config-requirements.md](02-config-requirements.md) exactly.

All probes assume Step 1–2 are complete (`WORKESTRATE_HOME=/tmp/workestrate-exp`,
repo scaffolded at `/tmp/workestrate-exp/repos/exp/workestrate.toml`).

### P1 — Contexts: define, switch, verify provenance

**Goal:** prove that defining a second context and switching to it changes the
active layer-set, observable via `context current` and `plan --show-source`.

**Setup — hand-edit the experiment registry** at
`/tmp/workestrate-exp/config.toml`:

```toml
layers = ["exp"]

[settings]
default_context = "primary"

[configs.exp]
url = "/tmp/workestrate-exp/repos/exp"

[contexts.primary]
layers = ["exp"]

[contexts.secondary]
layers = ["exp"]
```

**Add a marker workload** to the experiment `workestrate.toml` so plan output
differs between contexts (instance namespacing, ADR 0019 §Instance namespacing):

```toml
schema_version = 1

[workloads.probe-svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["python", "-m", "http.server", "8080"]
ports = [{ host = 8080, guest = 8080 }]
```

**Commands:**

```sh
# Default context (primary):
just workestrate context current
# Expected: Active context: primary, source: default, layers: [exp]

# Switch via --context flag (sets WORKESTRATE_CONTEXT, main.rs:242-244):
just workestrate --context secondary context current
# Expected: Active context: secondary, source: env (--context flag or WORKESTRATE_CONTEXT), layers: [exp]

# Provenance: --show-source prints each plan field with its source layer.
just workestrate --context secondary probe-svc plan --show-source
# Expected: plan output with [exp] (or [core]) source annotations per field
# (show_source_render, microsandbox/workload/show_source.rs:6-178).
```

**Expected outcome:** `context current` reflects the switch; `plan --show-source`
renders the `SandboxPlan` with per-field provenance annotations
(`diagnostics.rs:30-31` → `workload.show_source()`). The `--show-source` flag is
a global flag on the `Cli` struct (`main.rs:33-34`).

### P2 — Instances: `--new` / `--instance` / per-instance addressing (plan-plane)

**Goal:** verify the instance-lifecycle flags render correctly in `plan`
(runtime execution is `HOST-KVM` — marked).

The instance flags are on `ServiceAction::Up` / `AgentAction::Exec`
(`cli_actions.rs:16-38`, `cli_actions.rs:67-83`): `--new` (auto-allocate slug),
`--instance <id>` (target parallel instance). Per ADR 0026, parallel slots
publish on per-instance loopback IPs (`127.0.0.N`, `N >= 2`); `plan
--instance <id>` renders the prospective parallel plan (bind computed from
the registry view, read-only).

**Commands:**

```sh
# Plan a parallel-slot instance (config-plane, no KVM):
just workestrate probe-svc plan --instance canary
# Expected: the plan renders the prospective per-instance bind (127.0.0.2)
# when the bind differs from 127.0.0.1 (conditional additive rendering,
# ADR 0026). The singleton plan stays byte-identical.

# Plan with --show-source + --instance:
just workestrate probe-svc plan --show-source --instance canary
```

> **`--new` and `--instance` are runtime-only** (they allocate/target a
> parallel sandbox slot via `build_instance_spec`, `lifecycle.rs`). They do NOT
> affect `plan` output (plan renders the singleton). Verifying them end-to-end
> requires `HOST-KVM` (see [05-host-validation.md](05-host-validation.md)).

### P3 — Mounts: add a probe mount, verify plan rendering

**Goal:** verify a `[[workloads.<name>.mounts]]` entry renders in the plan.

**TOML to add** (field names per
[02-config-requirements.md](02-config-requirements.md) §1.3.5):

```toml
[[workloads.probe-svc.mounts]]
host = "${MSB_HOME}/sandboxes/probe/logs"
guest = "/var/log/probe"
read_only = false
```

**Command:**

```sh
just workestrate probe-svc plan
# Expected: includes "mount: ${MSB_HOME}/sandboxes/probe/logs:/var/log/probe"
# (MountPlan Display, plan.rs:89).
```

**Expected outcome:** the mount line appears in the plan output. `${MSB_HOME}`
is passed through to the microsandbox runtime (not resolved at plan time,
[02-config-requirements.md](02-config-requirements.md) §6).

### P4 — Secrets layering: probe secret + `secrets = "none"`

**Goal:** verify that a `[secrets.*]` definition with `required = false` is
accepted by `secrets-schema`, and that `secrets = "none"` on the registry
`[configs.<name>]` entry skips decryption for that layer.

**Add a probe secret** to the experiment `workestrate.toml` (field names per
[02-config-requirements.md](02-config-requirements.md) §1.2):

```toml
[secrets.PROBE_KEY]
env_var = "PROBE_KEY"
required = false
description = "Throwaway probe secret (no real value)."
```

**Commands:**

```sh
# secrets-schema lists env_var-backed secrets (cmd_secrets_schema,
# secrets_target.rs:86):
just workestrate secrets-schema
# Expected: PROBE_KEY appears in the sorted list.

# validate-config passes (required = false, no value needed for plan):
just workestrate validate-config
# Expected: workestrate.toml is valid.

# plan renders the secret as redacted (no decrypted value needed):
just workestrate probe-svc plan
```

**Toggle `secrets = "none"` on the registry entry** — edit
`/tmp/workestrate-exp/config.toml`:

```toml
[configs.exp]
url = "/tmp/workestrate-exp/repos/exp"
secrets = "none"      # "file" (default) | "none" (types.rs:235)
```

**Verify the skip:** `secrets = "none"` sets `SecretsLayer.skip = true`
(`loading.rs:478-484`), so `resolve_secrets_layers()` marks the `exp` layer as
skipped — no `.env.enc` decryption attempted for that layer. The
`secrets = "none"` opt-out is also honored in the `WORKESTRATE_CONFIG_DIR`
override branch via `env_dir_secrets_none()` (`loading.rs:412-423`, tested at
`loading.rs:1082-1107`).

```sh
just workestrate doctor
# Expected: the exp config-repo check still passes; secrets skip is silent
# (no decryption attempted).
```

**Expected outcome:** `secrets-schema` lists `PROBE_KEY`; `validate-config`
passes; `plan` renders the secret as `(redacted)`; `secrets = "none"` skips
decryption for the `exp` layer without error.

### P5 — `seed_files`: probe entry, verify plan/validation

**Goal:** verify a `[[workloads.<name>.seed_files]]` entry is accepted and
does not break `plan` / `validate-config`.

**TOML to add** (field names per
[02-config-requirements.md](02-config-requirements.md) §1.3.6):

```toml
[[workloads.probe-svc.seed_files]]
source = "agents/probe/config/defaults.json"
target = "workspaces/probe-state/defaults.json"
only_if_missing = true
```

**Commands:**

```sh
just workestrate validate-config
# Expected: workestrate.toml is valid. (seed_files source path is validated
# for no `..`, no absolute, no template tokens — validate.rs:72-96. The source
# file need NOT exist for validate-config/plan; it is resolved at runtime.)

just workestrate probe-svc plan
# Expected: plan renders normally (seed_files do not appear in the SandboxPlan
# Display format; they are applied at runtime, not in plan output).
```

**Expected outcome:** `validate-config` passes; `plan` renders without error.
`seed_files` are runtime-applied (not in `SandboxPlan` Display), so they do
not appear in plan output — this is correct behavior, not a gap.

---

## 5. What is disposable vs persistent

| Artifact | Location | Disposable? | Notes |
|---|---|---|---|
| Experiment tool home | `/tmp/workestrate-exp/` | **Yes** | Deleted after probes (§6). Contains registry, repos, state. |
| Throwaway config repo | `/tmp/workestrate-exp-repo/` (if `--path` used) | **Yes** | Deleted after probes. |
| Experiment project dir | `/tmp/workestrate-exp-proj/` (if trusted) | **Yes** | Deleted after probes. |
| Probe learnings / golden outputs | Captured into `docs/` if valuable | **Persistent** | If a probe reveals a bug or golden output worth keeping, capture it into the doc tree (not `/tmp`). |
| **REAL bundle** | `/home/node/Development/ai-workbench/.workestrate/` | **NEVER touched** | No probe in this document writes to the real bundle. The invariant: **never run any probe command without `WORKESTRATE_HOME` set to the experiment home.** |

> **Guard invariant (restated).** Before every probe session:
>
> ```sh
> test "$WORKESTRATE_HOME" = "/tmp/workestrate-exp" || \
>   { echo "ABORT: WORKESTRATE_HOME is not the experiment home"; exit 1; }
> ```

---

## 6. Cleanup

Run these in the experiment home to tear down:

```sh
# Stop any running sandboxes (no-op if none started; HOST-KVM if any were):
WORKESTRATE_HOME=/tmp/workestrate-exp just workestrate down-all --yes

# Remove state-dir contents (workspaces, var, run). Does not touch repos/config.
WORKESTRATE_HOME=/tmp/workestrate-exp just workestrate clean --yes

# Unregister the experiment config repo (and delete the store clone):
WORKESTRATE_HOME=/tmp/workestrate-exp just workestrate config remove exp --delete

# Remove the throwaway directories:
rm -rf /tmp/workestrate-exp /tmp/workestrate-exp-repo /tmp/workestrate-exp-proj

# Unset the env var so subsequent commands resolve the real bundle:
unset WORKESTRATE_HOME
```

**`config remove` flags (verified, `cli_actions.rs:170-180`):**

| Flag | Effect |
|---|---|
| `--delete` | Also remove the store clone at `<store>/repos/<name>` (`config_cmd.rs:49-63`). Refuses if the clone is dirty unless `--force` is also passed. |
| `--force` | Force deletion even if the clone is dirty (uncommitted changes). |

`cmd_config_remove` (`config_cmd.rs:41-80`) also scrubs the name from the bare
`layers` list and every context's `layers`, and clears `default_context` if it
pointed at the removed name (with a warning).

---

## Environment markers

Reproduced from [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)
§Environment markers:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix is at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

All probes in §4 are `verifiable-here` (config-plane / plan-plane only). Runtime
execution (`up`, `exec`, `--new`/`--instance` allocation) is `HOST-KVM` — see
[05-host-validation.md](05-host-validation.md).
