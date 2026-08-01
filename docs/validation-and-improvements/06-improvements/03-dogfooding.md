# 03 — Dogfooding: workestrate developing workestrate (Track C)

> **STATUS: SPEC (Phase 0 env pinning READY-TO-EXECUTE; B1/B2/B3 not implemented)**
> **Effort:** Phase 0 = S (env pinning); B1/B2/B3 = M each (not implemented)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [05-config-reference-cwd-fallback.md](05-config-reference-cwd-fallback.md) ·
> [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) ·
> [../../migration/50-decisions/0014-trust-gated-project-config.md](../../migration/50-decisions/0014-trust-gated-project-config.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md)

This document specifies **Track C**: dogfooding the workestrate tool by using
it to develop itself. An agent (running inside a workestrate sandbox) edits the
workestrate source tree; the **driver** workestrate binary that spawned that
sandbox must remain insulated from the agent's writes. Track C has two phases:

- **Phase 0** — an env-pinning wrapper that closes the config-reference
  cwd-fallback backdoor *today*, with no code change. READY-TO-EXECUTE.
- **Phase 1** — three hardening features (B1 self-home mount guard, B2
  config-free teardown verification, B3 spawn provenance) that make the
  isolation structural rather than convention-based. NOT IMPLEMENTED.

The standalone fix spec for the underlying quirk lives in
[05-config-reference-cwd-fallback.md](05-config-reference-cwd-fallback.md);
this document covers the dogfooding-specific application and the Phase 1
hardening.

### Environment markers

- `verifiable-here` — the check can be run inside the container without host
  privileges.
- `HOST-NIX` — requires `nix` on the host (e.g. building the driver binary).
- `HOST-KVM` — requires KVM on the host (e.g. bringing up a live sandbox).

---

## 1. Why dogfood

Dogfooding is the strongest possible validation that workestrate's trust model
is sound: the tool's own development becomes the adversarial test case. The
agent developing workestrate has every incentive to influence the driver that
spawns it — if the isolation holds under self-development, it holds under
general use. Specifically:

- It exercises the **driver/target topology** (an immutable driver binary
  spawning a sandbox that mutates the driver's own source tree).
- It stresses the **trust boundary** between the tool's config brain
  (`$WORKESTRATE_HOME`, `config.reference/`) and the agent's writable
  workspace.
- It surfaces config-discovery hazards that a non-self-hosting user would
  never hit (§3).

---

## 2. The driver/target model

### 2.1 Topology

```
                         ┌─────────────────────────────────────────────┐
                         │  HOST                                       │
                         │                                             │
                         │  ┌───────────────────────────┐              │
                         │  │  DRIVER (immutable)        │              │
                         │  │  - nix-store workestrate   │              │
                         │  │    binary (no source)      │              │
                         │  │  - WORKESTRATE_HOME =      │              │
                         │  │    ~/.workestrate-driver   │              │
                         │  │    (the tool brain;        │              │
                         │  │     agent must NOT reach)  │              │
                         │  └────────────┬───────────────┘              │
                         │               │ spawns                       │
                         │               ▼                              │
                         │  ┌───────────────────────────┐              │
                         │  │  TARGET sandbox            │              │
                         │  │  - git worktree of         │              │
                         │  │    ai-workbench            │              │
                         │  │  - agent runs here, edits  │              │
                         │  │    workestrate source      │              │
                         │  │  - ${CWD} mounted rw       │              │
                         │  │    → /work (or /workspace) │              │
                         │  └───────────────────────────┘              │
                         └─────────────────────────────────────────────┘
```

| Role | What it is | Home | Writable by agent? |
|---|---|---|---|
| **Driver** | The immutable nix-store `workestrate` binary that launches and manages sandboxes. | `~/.workestrate-driver` (via `WORKESTRATE_HOME`) | **No** — must be unreachable. |
| **Target** | A git worktree of `ai-workbench` where the agent develops workestrate itself. | (none — the worktree is the workspace) | **Yes** — the agent edits source here. |

The driver and target are **the same repository** (ai-workbench) at different
git refs/worktrees. The driver binary is built once (`nix build .#workestrate`,
HOST-NIX) and is immutable in the nix store; the target is a mutable worktree
the agent writes to.

### 2.2 The `.workestrate/` exposure problem

The `pi` and `opencode` agent workloads mount `${CWD}` read-write into the
sandbox (pi: `${CWD}` → `/work` rw, `.workestrate/repos/personal/workestrate.toml:141-144`;
opencode: `${CWD}` → `/workspace` rw). `${CWD}` resolves to the directory the
driver was invoked from (the host cwd, unchanged — see
[02-config-requirements.md](../02-config-requirements.md) §6).

If the driver is invoked from the **main checkout** of ai-workbench, then
`${CWD}` = the repo root, which contains `.workestrate/` — the tool brain
(registry, config repos, secrets, state). The agent would gain read-write
access to the driver's own home, including `config.toml`, `repos/personal/`,
and `secrets/`. That is a total trust-boundary collapse.

### 2.3 How the worktree topology closes it

The target is a **git worktree** of ai-workbench, not the main checkout. A
worktree shares git objects but has its own working tree and index. The
`.workestrate/` directory lives only in the main checkout; it is gitignored
(`.gitignore:45-46`: `/.workestrate/`), so it is **not** present in any
worktree. Running the driver from the worktree cwd means `${CWD}` = the
worktree root, which contains the source tree (flake.nix, `control/`,
`agents/`, etc.) but **not** `.workestrate/`.

| Invocation cwd | `${CWD}` contains `.workestrate/`? | Agent reaches tool brain? |
|---|---|---|
| Main checkout root | Yes | **Yes — catastrophic.** |
| Git worktree root | No (gitignored, not in worktree) | **No — closed.** |

**Verified:** `.gitignore:45-46` ignores `/.workestrate/` (comment: "Repo-local
workestrate XDG state (contains age private key — NEVER commit"). A `git
worktree add` does not materialize gitignored paths, so the worktree is clean
of the tool brain.

This closes the *filesystem* exposure. But it introduces a *config-discovery*
hazard, covered next.

---

## 3. The config-reference cwd-fallback quirk (dogfooding application)

A git worktree of ai-workbench contains **both** `flake.nix` **and**
`config.reference/` (the latter is tracked, not gitignored). When the driver
binary is run *from the worktree*, a discovery quirk silently loads the
worktree's `config.reference/workestrate.toml` as the **base config layer** —
and the worktree is agent-writable. Summary of the mechanism (full standalone
spec + fix options in
[05-config-reference-cwd-fallback.md](05-config-reference-cwd-fallback.md)):

1. `project_root()` (`control/agentctl/src/config/mod.rs:90-119`) resolves the
   project root by precedence: `AGENTCTL_ROOT` env → `CARGO_MANIFEST_DIR`
   (compile-time, cargo run only) → **current working directory**
   (`mod.rs:105-107`). The nix-installed driver binary has no
   `CARGO_MANIFEST_DIR`, so cwd wins. The only validation is that the resolved
   root contains `flake.nix` (`mod.rs:110-116`) — which the worktree does.
2. `reference_config_path()` (`control/agentctl/src/config/paths.rs:327-344`)
   builds `<root>/config.reference/workestrate.toml` off that root
   (`paths.rs:327-329`).
3. `load_config()` (`control/agentctl/src/config/loading.rs:285-306`) loads the
   reference config as the **base layer** (`loading.rs:301-306`).

**The backdoor:** the agent could edit
`config.reference/workestrate.toml` in its worktree, and the driver's *next*
invocation (run from the same worktree) would silently load that
agent-edited file as the base config layer. The reference fixture was designed
as the **tool's** fixture (ADR 0017), not as an agent-overridable layer.

Phase 0 (below) closes this with env pinning — no code change required.

---

## 4. Phase 0: driver env-pinning wrapper (READY-TO-EXECUTE)

Run the driver through a wrapper that pins the environment so that **no**
config discovery can reach the worktree. The recommended wrapper sets four
environment variables:

```sh
#!/usr/bin/env bash
# workestrate-driver.sh — Phase 0 dogfooding driver wrapper.
set -euo pipefail

export WORKESTRATE_HOME="${HOME}/.workestrate-driver"
export WORKESTRATE_CONFIG_DIR="${WORKESTRATE_HOME}"
export AGENTCTL_ROOT="${HOME}/Development/ai-workbench"   # the REAL checkout
export WORKESTRATE_NO_PROJECT_CONFIG=1

exec workestrate "$@"
```

### 4.1 Justification per variable

| Variable | Role | Strictly required? | Why |
|---|---|---|---|
| `WORKESTRATE_HOME` | Isolate the driver's tool brain into a dedicated home (`~/.workestrate-driver`), distinct from any project-local `.workestrate/`. | **Yes** | Without this, the driver may discover a `.workestrate/` in a trusted ancestor of cwd (ADR 0023 discovery, `paths.rs` precedence step 2). The driver home must not be the worktree. |
| `WORKESTRATE_CONFIG_DIR` | Bypass **all** config discovery (including reference autodiscovery) and load a single layer directly from `$WORKESTRATE_CONFIG_DIR/workestrate.toml` (`loading.rs:287-297`). | **Yes** (closes the §3 backdoor) | This is the primary backdoor closure: it short-circuits `load_config()` before `reference_config_path()` is ever consulted (`loading.rs:287-297` runs before `loading.rs:301-306`). The driver loads only the operator-controlled config dir. |
| `AGENTCTL_ROOT` | Pin `project_root()` to the real checkout (`mod.rs:92-94`), so any code path that calls `project_root()` (not just `load_config`) resolves the real root, not the worktree cwd. | **Defense-in-depth** | `WORKESTRATE_CONFIG_DIR` already prevents the reference layer from loading via `load_config()`. But `project_root()` is also called by other paths (e.g. `reference_config_path()` itself, `paths.rs:328`). Pinning `AGENTCTL_ROOT` ensures *all* of them resolve the real checkout. Belt-and-suspenders. |
| `WORKESTRATE_NO_PROJECT_CONFIG` | Disable trusted-project layer pickup from the worktree cwd (`loading.rs:336`). | **Defense-in-depth** | Under `WORKESTRATE_CONFIG_DIR`, the project layer is never reached (the bypass returns early, `loading.rs:287-297`). But if `WORKESTRATE_CONFIG_DIR` is ever unset (e.g. a wrapper bug), this prevents the worktree's `./workestrate.toml` (if present) from loading as a trusted-project layer. |

**Net effect:** with `WORKESTRATE_CONFIG_DIR` set, `load_config()` returns at
`loading.rs:295` — the reference, registry, overrides, project, and local
layers are **all** skipped. The driver loads exactly one operator-controlled
config file. The agent's worktree cannot influence it.

> **Note on `WORKESTRATE_CONFIG_DIR` semantics:** it loads a *single* layer
> with no merging (`loading.rs:287-297`). The driver config dir must therefore
> be a complete, self-contained `workestrate.toml` (not a delta). See the
> runbook (§6) for how to populate it.

---

## 5. Phase 1: structural hardening (NOT IMPLEMENTED)

Phase 0 relies on an operator remembering to set the wrapper env. Phase 1
makes the isolation structural — the tool itself refuses the unsafe
configurations. Three features, each independently shippable.

### 5.1 B1 — Self-home mount guard

| | |
|---|---|
| **Problem** | If the driver's `WORKESTRATE_HOME` (or any ancestor/descendant of it) is mounted into a sandbox — e.g. via a `${CWD}` mount when the driver is invoked from the main checkout (§2.2) — the agent reaches the tool brain. Phase 0 prevents this by convention (worktree + gitignore); B1 prevents it structurally. |
| **Design sketch** | At plan time, after mount resolution, check each resolved host path against the driver's resolved `WORKESTRATE_HOME` (and `$WORKESTRATE_HOME/state`, `/secrets`, `/repos`). If a mount's host path is the home itself or a descendant/ancestor, **refuse** the plan (fail-closed) with a clear error naming the collision. This is a monotonic deny applied post-merge, like the egress allowlist (ADR 0004) — no merge-engine change. |
| **Files touched** | `control/agentctl/src/microsandbox/workload/validate.rs` (mount validation); `control/agentctl/src/config/paths.rs` (expose resolved home); `control/agentctl/src/policy.rs` (guard constant if desired). |
| **Gate** | Unit test: a workload mounting `${CWD}` where cwd == `WORKESTRATE_HOME` → plan fails with the guard error. `verifiable-here`. |
| **Env marker** | `verifiable-here` (static check at plan time; no runtime needed). |

### 5.2 B2 — Config-free teardown verification

| | |
|---|---|
| **Problem (as posed)** | `down`/`down --all`/`clean` must work even when config loading fails, so a misconfigured driver can still tear down running sandboxes. |
| **Verified current behavior** | Teardown does **not** call `load_config()` (the full merged-config loader). `cmd_down` (`commands/lifecycle.rs:275`), `cmd_down_all` (`lifecycle.rs:355`), and `cmd_clean` (`lifecycle.rs:418`) use only `crate::config::active_context_name()` (`config/mod.rs:76-82`) and `crate::config::resolve_state_dir()` (`config/paths.rs:254-265`). `resolve_state_dir()` calls `load_registry_for_dir_resolution()` (`paths.rs:240-248`), which loads **only** the registry (`config.toml`) — not the merged config — and **warns + falls back** to `<home>/state` on registry error (`paths.rs:243-247`). `active_context_name()` returns `None` gracefully if `load_config()` was never called (it reads a process-global set during `load_config()`). The main.rs dispatch for `DownAll`/`Clean` (`main.rs:260-261`) calls these directly with no `load_config()` in the path. |
| **Finding** | **The B2 premise is largely already satisfied.** Teardown is already config-load-free in the merged-config sense and degrades gracefully on registry corruption. The residual gap is narrow: (a) `resolve_state_dir()` still *reads* the registry (not the merged config) — but it falls back safely; (b) there is no explicit test asserting "teardown works when `load_config()` would fail." |
| **Design sketch** | Reframe B2 as a **verification + test** item rather than a behavior change: add a regression test that runs `down`/`clean` with a corrupt/absent registry and asserts graceful fallback (no panic, state dir resolves to default). Optionally, add a `--no-config` escape hatch to `down`/`clean` that skips even registry resolution and uses `<home>/state` directly — but this is likely unnecessary given the existing fallback. |
| **Files touched** | `control/agentctl/src/commands/lifecycle.rs` (test only, unless `--no-config` is added); `control/agentctl/tests/` (regression test). |
| **Gate** | `cargo test` — new test: corrupt registry → `resolve_state_dir()` returns default, `cmd_clean` succeeds. `verifiable-here`. |
| **Env marker** | `verifiable-here`. |

> **Honesty note:** the original B2 framing ("teardown currently requires
> config load") was **not verified** and is **refuted** by reading
> `lifecycle.rs:275,355,418` + `paths.rs:240-265`. Teardown already avoids
> `load_config()`. B2 is therefore a verification hardening item, not a
> behavior-fix gap.

### 5.3 B3 — Spawn provenance

| | |
|---|---|
| **Problem** | When a sandbox is running, there is no record of *which* driver binary / home / config spawned it. In a dogfooding setup where the driver may be rebuilt and re-run against the same state dir, stale instances from an older driver can linger. Provenance lets an operator audit "who spawned this." |
| **Design sketch** | Extend `SandboxInstanceRecord` (`microsandbox/port_registry/mod.rs`, the `SandboxInstanceRecord` struct) with a `spawned_by` field recording: the driver binary path (or nix-store hash), the resolved `WORKESTRATE_HOME`, and the `AGENTCTL_ROOT` (or "cwd") used at spawn time. Written atomically alongside the existing record fields at registration. |
| **Where instance records live** | Per-instance JSON state files under `${state_dir}/var/run/*.json` (`microsandbox/port_registry/store.rs`, `read_record_loud` / `check_port_collisions` scan `${state_dir}/var/run/*.json`). The record struct is `SandboxInstanceRecord` (`port_registry/mod.rs`), already `#[serde(default)]`-extensible for new fields (backward-compat pattern documented in the struct doc). |
| **Files touched** | `control/agentctl/src/microsandbox/port_registry/mod.rs` (add `spawned_by` field, `#[serde(default)]`); `control/agentctl/src/microsandbox/port_registry/store.rs` (populate at registration); `control/agentctl/src/microsandbox/runtime/run.rs` (pass provenance at spawn). |
| **Gate** | Unit test: a registered instance record round-trips with `spawned_by` populated; `ps` displays it. `verifiable-here`. |
| **Env marker** | `verifiable-here` (state-file round-trip); `HOST-KVM` to verify against a live sandbox. |

---

## 6. Operational runbook

### 6.1 Set up the driver home (once)

The driver needs its own isolated tool home, distinct from any project-local
`.workestrate/`:

```sh
# 1. Create the driver home.
export WORKESTRATE_HOME="${HOME}/.workestrate-driver"
mkdir -p "${WORKESTRATE_HOME}"

# 2. Initialize the registry + a config repo (or point at an existing one).
workestrate init <your-config-repo-url>
#   → writes ${WORKESTRATE_HOME}/config.toml + clones repos/<name>/

# 3. (Alternative) add a config repo to an existing home:
workestrate config add <name> --url <url> --ref main

# 4. Trust the REAL checkout (not the worktree) if you want project-layer
#    loading from the main checkout:
workestrate config trust "${HOME}/Development/ai-workbench"
```

> The driver config dir (`WORKESTRATE_CONFIG_DIR`) must contain a
> **self-contained** `workestrate.toml` (single-layer, no merge — see
> `loading.rs:287-297`). If you want the driver to use the merged registry
> layers instead, **omit** `WORKESTRATE_CONFIG_DIR` and rely on
> `AGENTCTL_ROOT` + `WORKESTRATE_NO_PROJECT_CONFIG` alone — but then the
> reference-layer backdoor (§3) is closed only by `AGENTCTL_ROOT` winning the
> `project_root()` precedence, which is defense-in-depth, not a hard gate.
> **Recommendation: set `WORKESTRATE_CONFIG_DIR` for dogfooding.**

### 6.2 Create the target worktree

```sh
# From the main checkout:
cd "${HOME}/Development/ai-workbench"
git worktree add "../ai-workbench-dogfood" migration/tool-model
cd "../ai-workbench-dogfood"

# Verify .workestrate/ is NOT present (gitignored):
test ! -d .workestrate && echo "OK: worktree is clean of tool brain" \
  || echo "FAIL: .workestrate leaked into worktree"
```

### 6.3 Launch the agent against the worktree

```sh
# From the worktree (this becomes ${CWD}):
cd "${HOME}/ai-workbench-dogfood"

# Run the driver via the Phase 0 wrapper (§4):
workestrate-driver.sh workload up pi
#   → driver home = ~/.workestrate-driver (isolated)
#   → config loaded from $WORKESTRATE_CONFIG_DIR (no worktree discovery)
#   → ${CWD} = the worktree (mounted rw into the sandbox)
#   → .workestrate/ absent from ${CWD} (gitignored)
```

### 6.4 Invariants to check

| # | Invariant | How to verify |
|---|---|---|
| 1 | `.workestrate/` is absent from the worktree cwd | `test ! -d .workestrate` in the worktree |
| 2 | Driver home is not the worktree | `echo $WORKESTRATE_HOME` ≠ the worktree path |
| 3 | `WORKESTRATE_CONFIG_DIR` is set | `test -n "$WORKESTRATE_CONFIG_DIR"` |
| 4 | The reference layer is NOT loaded from the worktree | `workestrate-driver.sh workload plan pi --show-source` — provenance must not list a `reference` layer sourced from the worktree's `config.reference/` (`diagnostics.rs:30-31` renders provenance via `workload.show_source()`, `workload/config.rs:129`) |
| 5 | The agent sandbox cannot read the driver home | From inside the sandbox: `ls ~/.workestrate-driver` → not found (not mounted) |

---

## 7. Risks / open questions

| # | Risk / question | Status |
|---|---|---|
| R1 | `WORKESTRATE_CONFIG_DIR` loads a single layer with no merge (`loading.rs:287-297`). The driver config must be self-contained — it cannot rely on registry-layer merging. | **Documented** (§6.1). Operator must maintain a complete driver config. |
| R2 | If the operator forgets the wrapper and runs the bare `workestrate` from the worktree, the §3 backdoor reopens. | Phase 0 is convention-based. B1 (§5.1) makes it structural. |
| R3 | `AGENTCTL_ROOT` is defense-in-depth under `WORKESTRATE_CONFIG_DIR` (the bypass returns before `reference_config_path()` is called). Is it worth keeping? | **Yes** — other code paths call `project_root()` directly (`paths.rs:328`); pinning it is cheap insurance. |
| R4 | B2's original premise ("teardown requires config load") was refuted. Is a code change needed? | **No behavior change needed.** Add a regression test only (§5.2). |
| R5 | B3 provenance adds a field to `SandboxInstanceRecord`. Existing records (legacy) must still parse. | The struct is already `#[serde(default)]`-extensible (`port_registry/mod.rs` struct doc). Safe. |
| R6 | Does the nix-installed driver binary truly lack `CARGO_MANIFEST_DIR`? | **Yes** — `CARGO_MANIFEST_DIR` is set by cargo at build/run time for the crate being compiled; a nix-store binary is invoked directly, not via cargo. Verified: `project_root()` falls through to cwd (`mod.rs:104-107`) when both `AGENTCTL_ROOT` and `CARGO_MANIFEST_DIR` are absent. |
