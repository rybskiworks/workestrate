# 05 — Config-reference cwd-fallback quirk (standalone fix spec)

> **STATUS: SPEC (bug fix candidate, small)**
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [03-dogfooding.md](03-dogfooding.md) ·
> [../../migration/50-decisions/0017-synthetic-reference-and-strip-down.md](../../migration/50-decisions/0017-synthetic-reference-and-strip-down.md)

This document specifies a standalone fix for a config-discovery quirk: when the
nix-installed `workestrate` binary is run from any directory containing both
`flake.nix` and `config.reference/`, the reference config is silently loaded
as the **base layer** off the current working directory — even though the
reference fixture was designed as the **tool's** fixture (ADR 0017), not as a
user-overridable layer. This is arguably a bug independent of dogfooding; the
dogfooding-specific application is covered in
[03-dogfooding.md](03-dogfooding.md).

### Environment markers

- `verifiable-here` — the check can be run inside the container without host
  privileges.
- `HOST-NIX` — requires `nix` on the host (e.g. running the nix-installed
  binary).

---

## 1. The bug

### 1.1 Mechanism (verified code path)

The reference config is loaded as the base layer of every `load_config()` call
that does not hit the `WORKESTRATE_CONFIG_DIR` bypass. The root from which
`config.reference/` is resolved is determined by `project_root()`, which falls
back to the **current working directory** when neither `AGENTCTL_ROOT` nor
`CARGO_MANIFEST_DIR` is set.

| Step | Function | File:line | Behavior |
|---|---|---|---|
| 1 | `project_root()` | `control/agentctl/src/config/mod.rs:90-119` | Resolves the project root. Precedence: `AGENTCTL_ROOT` env (`mod.rs:92-94`) → `CARGO_MANIFEST_DIR` walk-up (`mod.rs:96-103`, compile-time, cargo run only) → **current working directory** (`mod.rs:105-107`). The nix-installed binary has neither env var set, so **cwd wins**. |
| 2 | flake.nix validation | `mod.rs:110-116` | The only check on the resolved root is that it contains `flake.nix`. Any directory shaped like a workbench checkout passes. |
| 3 | `reference_config_path()` | `control/agentctl/src/config/paths.rs:327-344` | Builds `<root>/config.reference/workestrate.toml` off the root from step 1 (`paths.rs:327-329`). Returns `Some(path)` if it exists. |
| 4 | `load_config()` base layer | `control/agentctl/src/config/loading.rs:301-306` | Loads the reference config as the **base (lowest-precedence) layer** (`loading.rs:301-306`): `layers.push(Layer::load("reference", &path)?)`. |

The `WORKESTRATE_CONFIG_DIR` bypass (`loading.rs:287-297`) returns *before*
step 4 is reached, so setting it avoids the quirk entirely. But without it,
the reference layer is loaded from cwd.

### 1.2 Reproduction recipe

> Env marker: `HOST-NIX` (requires the nix-installed binary, which lacks
> `CARGO_MANIFEST_DIR`). `cargo run` from the repo does **not** reproduce it
> because `CARGO_MANIFEST_DIR` is set at step 1 (`mod.rs:96-103`).

```sh
# 1. Build the driver binary (HOST-NIX).
nix build .#workestrate

# 2. From the repo root (which has flake.nix + config.reference/), run the
#    nix-installed binary with NO AGENTCTL_ROOT and NO WORKESTRATE_CONFIG_DIR:
cd /home/node/Development/ai-workbench
unset AGENTCTL_ROOT WORKESTRATE_CONFIG_DIR
result/bin/workestrate workload plan pi --show-source
```

**Expected (buggy) behavior:** the `reference` layer appears in the provenance
output, sourced from `<cwd>/config.reference/workestrate.toml`.

**How `--show-source` demonstrates provenance:** the global `--show-source`
flag (`control/agentctl/src/main.rs:34`, `#[arg(long, global = true)]`) is
threaded into `dispatch_service`/`dispatch_agent` (`main.rs:306,310,314,318,
322,345,349`) and reaches `cmd_plan` (`commands/diagnostics.rs:14-31`). When
set, `cmd_plan` calls `workload.show_source()` (`diagnostics.rs:30-31`) which
renders the per-field provenance map. The workload is constructed via
`ConfigWorkload::new()` (`microsandbox/workload/config.rs:32-33`), which calls
`crate::config::load_config()` — so the plan path exercises the full
`load_config()` including the reference base layer. The `show_source` impl
lives at `microsandbox/workload/config.rs:129` and
`microsandbox/workload/show_source.rs:6`.

**To confirm the reference layer came from cwd (not the build-time root):**
edit `config.reference/workestrate.toml` in the cwd checkout (e.g. add a
comment or change a non-security field), re-run `plan --show-source`, and
observe the change reflected — proving the cwd copy, not a baked-in copy, was
loaded.

### 1.3 Why it is a bug, not just a dogfooding hazard

| Argument | Detail |
|---|---|
| **Any workbench-shaped directory hijacks the base layer.** | The only gate is `flake.nix` existence (`mod.rs:110-116`). A `git clone` of ai-workbench into an arbitrary directory, then running the nix-installed `workestrate` from there, loads that clone's `config.reference/` as the base layer. The operator may not realize the cwd is being used as a config source. |
| **The reference fixture is the tool's fixture, not a user layer.** | ADR 0017 (`0017-synthetic-reference-and-strip-down.md`) established `config.reference/` as a "minimal, machine-independent fixture that exercises the vocabulary" — a golden-test fixture + fresh-install fallback. It was not designed as a user-overridable config layer. Loading it from an arbitrary cwd conflates the tool's fixture with the operator's environment. |
| **It is silent.** | No warning is emitted when `project_root()` falls back to cwd (`mod.rs:104-107`), and no warning when the reference layer is loaded from a cwd-derived root (`loading.rs:301-306`). The operator has no signal that the base layer came from their current directory rather than the tool's build-time location. |
| **Dogfooding amplifies it.** | In the dogfooding topology (§3 of `03-dogfooding.md`), the cwd is an agent-writable worktree containing `config.reference/`. The agent can edit the base layer that the driver loads on its next invocation. But the bug exists *without* dogfooding: any workbench checkout on disk is a base-layer source. |

---

## 2. Fix options

Four options, compared. A recommendation is picked and justified.

### Option (a) — Remove the cwd fallback in `project_root()`

Require `AGENTCTL_ROOT` or `CARGO_MANIFEST_DIR`; error if neither is set
(instead of falling back to cwd at `mod.rs:104-107`).

| Aspect | Assessment |
|---|---|
| Closes the bug? | **Yes** — `reference_config_path()` (`paths.rs:327-329`) would no longer resolve a cwd-derived root. |
| Backward compat | `cargo run` from the repo: **unaffected** (`CARGO_MANIFEST_DIR` is set, `mod.rs:96-103`). `just golden-check`: **unaffected** — it sets `WORKESTRATE_CONFIG_DIR` (`justfile:86-91`), which bypasses `load_config()` entirely (`loading.rs:287-297`), so `project_root()` is never reached via the plan path. Nix-installed binary run from a workbench checkout: **breaks** — would now error unless `AGENTCTL_ROOT` is set. |
| Severity of break | Medium. The nix-installed binary is the primary production invocation; requiring `AGENTCTL_ROOT` for every call is friction. But `project_root_optional()` (`mod.rs:121-129`) already exists for callers that can degrade gracefully. |
| Verdict | Too broad — breaks the legitimate "run the installed binary from a checkout" use case. |

### Option (b) — Only load `config.reference` when the resolved root is the binary's own build-time root

Gate the reference-layer load on the root matching the build-time
`CARGO_MANIFEST_DIR`-derived path (baked into the binary at compile time).

| Aspect | Assessment |
|---|---|
| Closes the bug? | **Yes** — a cwd-derived root would not match the baked-in build root, so `reference_config_path()` would return `None`. |
| Backward compat | `cargo run`: the build-time root matches → reference loads (unchanged). Nix-installed binary: the build-time root is the nix store path baked at build → would match only when run from... nowhere useful (the nix store path isn't a cwd). This **breaks the fresh-install fallback** (ADR 0017 role i): a fresh clone with no registered config repo would no longer load the reference fixture, because the cwd ≠ the nix-store build root. |
| Severity of break | High — defeats ADR 0017's "fresh-install fallback" purpose. |
| Verdict | Breaks the documented purpose of `config.reference/`. Reject. |

### Option (c) — Mark reference-layer provenance and warn when root came from cwd

Keep the cwd fallback but emit a warning when `project_root()` resolved via cwd
(not `AGENTCTL_ROOT`/`CARGO_MANIFEST_DIR`) **and** the reference layer was
loaded from that cwd-derived root.

| Aspect | Assessment |
|---|---|
| Closes the bug? | **No** — the reference layer is still loaded from cwd; only a warning is added. The dogfooding backdoor remains exploitable (the agent's edit still takes effect). |
| Backward compat | Fully compatible — no behavior change, only a stderr warning. |
| Severity of break | None. |
| Verdict | Insufficient as a fix (the load still happens), but valuable as a **defense-in-depth signal**. Could be combined with (a) or (d). |

### Option (d) — Gate reference loading behind an explicit opt-in env when root == cwd  ⭐ RECOMMENDED

When `project_root()` resolved via cwd (i.e., neither `AGENTCTL_ROOT` nor
`CARGO_MANIFEST_DIR` was set), require an explicit opt-in env
(e.g. `WORKESTRATE_ALLOW_CWD_REFERENCE=1`) to load the reference layer from
that cwd-derived root. Without the opt-in, skip the reference layer (return
`None` from `reference_config_path()` when the root came from cwd and the
opt-in is unset) and emit a one-line stderr notice.

| Aspect | Assessment |
|---|---|
| Closes the bug? | **Yes** — by default, the reference layer is no longer loaded from an arbitrary cwd. The operator must explicitly opt in. |
| Backward compat | `cargo run`: `CARGO_MANIFEST_DIR` is set → root did **not** come from cwd → reference loads as before (no opt-in needed). `just golden-check`: uses `WORKESTRATE_CONFIG_DIR` (`justfile:86-91`) → bypasses `load_config()` entirely → **unaffected**. Nix-installed binary from a workbench checkout: reference layer is **skipped by default** (behavior change, but the safe direction); operator sets `WORKESTRATE_ALLOW_CWD_REFERENCE=1` to restore the old behavior. Fresh-install fallback (ADR 0017): a fresh clone with no config repo would now skip the reference layer unless the opt-in is set — `load_config()` would then error with "no config found" (`loading.rs:386-388`) unless a config repo is registered. This is a **minor** regression of the fallback, mitigated by the opt-in env or by `workestrate init`. |
| Severity of break | Low. The only break is the fresh-install-without-`init` path, which is already a "you must configure something" state. The opt-in env restores old behavior for operators who want it. |
| Verdict | **Recommended.** Defaults safe (no silent cwd loading), opt-in available, minimal compat impact, and the fresh-install fallback is preserved via the opt-in env. Combine with (c)'s warning for the opt-in case. |

### 2.1 Recommendation

**Adopt (d)** as the primary fix, **plus (c)** as a companion warning: when the
opt-in env is set and the reference layer is loaded from a cwd-derived root,
emit a one-line stderr warning ("reference config loaded from cwd
`<path>`; set `AGENTCTL_ROOT` to pin the root"). This gives operators a signal
even when they opt in.

**Why not (a):** too broad; breaks the legitimate installed-binary-from-checkout
use case without an opt-in escape.

**Why not (b):** breaks ADR 0017's fresh-install fallback by baking in a nix
store path that never matches a real cwd.

**Why not (c) alone:** does not close the backdoor; the agent-edited reference
layer still loads.

---

## 3. Regression test sketch

A unit test that asserts the chosen fix behavior. Place in
`control/agentctl/tests/` or the config module's `#[cfg(test)]` block.

```rust
// Test: cwd fallback + flake.nix + config.reference must NOT silently load
// the reference layer as the base config (per fix option d).
//
// Setup (tempdir):
//   <tmp>/
//     flake.nix                          # empty marker file
//     config.reference/
//       workestrate.toml                 # a minimal valid layer
//
// Env: unset AGENTCTL_ROOT, CARGO_MANIFEST_DIR, WORKESTRATE_CONFIG_DIR,
//      WORKESTRATE_ALLOW_CWD_REFERENCE.
// cwd: <tmp>
//
// Assert (post-fix, option d):
//   - load_config() does NOT include a layer named "reference" in its
//     provenance (reference_config_path() returned None because root came
//     from cwd and opt-in was unset).
//   - A stderr notice was emitted (or load_config errors with "no config
//     found" if no other layer is present — either is acceptable; the key
//     assertion is that the cwd's config.reference/ was NOT loaded).
//
// Assert (opt-in set: WORKESTRATE_ALLOW_CWD_REFERENCE=1):
//   - load_config() DOES include a "reference" layer sourced from
//     <tmp>/config.reference/workestrate.toml.
//   - A stderr warning was emitted naming the cwd path.
//
// Assert (AGENTCTL_ROOT pinned to <tmp>):
//   - load_config() includes a "reference" layer (root did not come from
//     cwd fallback; opt-in not required). This preserves the pinned-root
//     use case.
```

The provenance map (`crate::merge::Provenance`, `merge.rs:72-108`) records
which layer set each field; `take_provenance()` (`merge.rs:141`) retrieves it.
The test can assert `provenance.get("schema_version")` (or any field the
reference layer sets) is `None` (post-fix, no opt-in) vs `Some("reference")`
(opt-in or pinned root).

---

## 4. Effort & gate

| Item | Value |
|---|---|
| Effort | **S** (small) — one function (`reference_config_path()` or `project_root()`) gains a cwd-source check + env gate; one warning; one test. |
| Gate | `cargo test` (new regression test). `verifiable-here`. |
| Files touched | `control/agentctl/src/config/paths.rs` (`reference_config_path`, `paths.rs:327-344`); `control/agentctl/src/config/mod.rs` (`project_root`, `mod.rs:90-119` — expose whether root came from cwd); `control/agentctl/tests/` (regression test). |
| Risk | Low. The only behavior change is skipping the reference layer when root==cwd and opt-in is unset. `cargo run` and `just golden-check` are unaffected (verified: `CARGO_MANIFEST_DIR` and `WORKESTRATE_CONFIG_DIR` respectively short-circuit before the new gate). |
