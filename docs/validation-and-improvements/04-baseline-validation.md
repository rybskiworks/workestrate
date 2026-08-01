# 04 — Baseline Validation

> **STATUS: READY-TO-EXECUTE (Lane A verifiable-here; runtime parity is HOST-KVM — see [05-host-validation.md](05-host-validation.md))**
> Prerequisites / see-also: [README.md](README.md) · [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) · [03-sibling-config-setup.md](03-sibling-config-setup.md) · [05-host-validation.md](05-host-validation.md) · [../migration/50-decisions/0017-synthetic-reference-and-strip-down.md](../migration/50-decisions/0017-synthetic-reference-and-strip-down.md)

This document defines HOW to establish the pre-migration behavioral baseline
for the **original 5 workloads** (`litellm`, `pi`, `odysseus`, `opencode`,
`tempest`) and prove the new config-driven setup reproduces it. It is the
authoritative procedure for parity proof between the deleted hardcoded Rust
workload modules and the TOML-driven `ConfigWorkload` that replaced them.

A contextless reader should know two things up front:

- **What a golden plan is.** A *golden plan* is a committed text file that
  captures the exact output of `workestrate workload plan <name>` for a workload. The
  `plan` subcommand prints the `SandboxPlan` via its `Display` impl
  (`control/agentctl/src/microsandbox/plan.rs:156-200`). A golden file is the
  frozen reference; `just golden-check` regenerates the plan and diffs it
  byte-for-byte against the committed file. Drift = failure.
- **What the `SandboxPlan` Display format looks like.** Here is the real
  committed golden for the synthetic `example-service` workload
  (`control/agentctl/tests/golden/example-service.plan.txt`):

  ```text
  name: example-service
  image: python:3.12-slim
  workdir: /app
  command: python -m http.server 8080
  cpus: 2
  memory: 1024 MiB
  env: APP_PORT=8080
  secret_env: OPENAI_API_KEY (value redacted, allowed: host.microsandbox.internal, required)
  port: 8080:8080
  mount: workspaces/example-service-state:/data
  mount: config.reference/infra/litellm:/app/config (ro)
  network: default_deny=true
    ingress: tcp:8080 local
    egress: tcp:53 -> host
    egress: udp:53 -> host
    egress: tcp:443 -> github.com, api.github.com
  ```

  Note: secret env vars render as `NAME=(redacted)` (see `EnvVar::fmt` at
  `plan.rs:114-122`), so `plan` output never contains decrypted secret values
  and can be generated without a SOPS age key.

> Every claim below cites a file:line or git object read during this session.
> The codebase is on branch `migration/tool-model` (verified:
> `git branch --show-current` → `migration/tool-model`).

---

## Why there is no committed golden for the original 5

The original 5 workloads were hardcoded Rust files under
`control/agentctl/src/workloads/*.rs` (`litellm.rs`, `pi.rs`, `odysseus.rs`,
`opencode.rs`, `tempest.rs`), wired through a `workloads!` macro at
`main.rs:49-69` (pre-migration). They were **deleted** in commit `32040c3`
("feat(agentctl): migrate workloads to config-driven TOML + hybrid CLI
dispatch"), which replaced them with a single generic `ConfigWorkload` driven
by `workestrate.toml`.

The pre-migration base is commit `840e8b7` on `main`. Verified:

```text
$ git log --oneline 840e8b7 -1
840e8b7 docs: document workestrate run subcommand and secret loading behavior
```

The deletion (abbreviated `git show 32040c3 --stat`):

```text
commit 32040c330162564754deeb71873b3d3dd4cd9a70
    feat(agentctl): migrate workloads to config-driven TOML + hybrid CLI dispatch
 control/agentctl/src/main.rs                  | 344 +++++++++++++------------
 control/agentctl/src/workloads/litellm.rs     |  66 -----
 control/agentctl/src/workloads/mod.rs         |  11 -
 control/agentctl/src/workloads/odysseus.rs    | 105 --------
 control/agentctl/src/workloads/opencode.rs    |  73 ------
 control/agentctl/src/workloads/pi.rs          | 128 ----------
 control/agentctl/src/workloads/tempest.rs     | 113 --------
 workestrate.toml                              | 351 ++++++++++++++++++++++++++
 (15 files changed, 890 insertions(+), 815 deletions(-))
```

The old `workloads!` macro (verified via
`git show 840e8b7:control/agentctl/src/main.rs | sed -n '40,75p'`):

```text
macro_rules! workloads {
    ($macro:ident) => {
        $macro!(
            Litellm, workloads::Litellm, Service, "LiteLLM proxy sandbox";
            Odysseus, workloads::Odysseus, Service, "Odysseus agent sandbox";
            Pi, workloads::Pi, Agent, "Pi coding agent sandbox";
            Opencode, workloads::Opencode, Agent, "OpenCode agent sandbox";
            Tempest, workloads::Tempest, Agent, "T3MP3ST offensive-security agent sandbox";
        );
    };
    ...
}
```

**Committed golden plans exist ONLY for the 3 synthetic `config.reference`
workloads** (`example-service`, `example-agent`, `example-offensive`) — see
`justfile:78-90` (`golden-generate` / `golden-check`) and
`control/agentctl/tests/golden/`. These were regenerated post-migration from
the synthetic fixture (ADR 0017), **NOT** from the original 5. Verified:

```text
$ ls control/agentctl/tests/golden/
example-agent.plan.txt
example-offensive.plan.txt
example-service.plan.txt
```

```makefile
# justfile:78-90 (verbatim)
golden-generate:
    @for name in example-service example-agent example-offensive; do \
        WORKESTRATE_CONFIG_DIR=config.reference cargo run --manifest-path control/agentctl/Cargo.toml -- $name plan \
          > control/agentctl/tests/golden/$name.plan.txt; \
    done
golden-check:
    @for name in example-service example-agent example-offensive; do \
        WORKESTRATE_CONFIG_DIR=config.reference cargo run --manifest-path control/agentctl/Cargo.toml -- $name plan \
          | diff - control/agentctl/tests/golden/$name.plan.txt \
          || (echo "golden mismatch for $name; run 'just golden-generate' to update" && exit 1); \
    done
```

**Consequence:** the original 5 workloads have **no committed pre-migration
golden plans**. `just golden-check` does NOT cover them. Parity for the
original 5 must therefore be proven by a **baseline recovery procedure**
(generate old plans from the base commit, generate new plans from the current
config, diff) — which is the subject of this document.

A **partial executable spec** of pre-migration behavior survives in unit tests
at `control/agentctl/src/microsandbox/runtime/network.rs`. These tests assert
specific invariants of the *config-driven* plans (they construct
`ConfigWorkload::new("pi")` etc.), so they pin the post-migration behavior
that must match the old hardcoded behavior. They are enumerated in §3d.

---

## Baseline recovery procedure

This procedure is runnable in this container via `nix develop` (the old binary
must be built with cargo, which requires the dev shell's `cc` linker and
native crates — absent in a bare shell, but present inside `nix develop`).
Nix is installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`
(not on PATH); prefix with
`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
then run inside `nix develop -c bash -c '<cmd>'` (verified 2026-07-29:
`cc --version` → gcc 15.2.0; `cargo check` compiles in ~27s; first devshell
build takes minutes, subsequent runs are fast).

### Step 1 — Create a git worktree at the base commit

```sh
git worktree add /tmp/workestrate-base 840e8b7
```

This checks out the pre-migration source read-only. The old
`control/agentctl/src/workloads/*.rs` files exist there. Remove the worktree
afterwards with `git worktree remove /tmp/workestrate-base`.

### Step 2 — Build and run the OLD binary; capture baseline plans

> **Note (2026-08-01, ADR 0027):** the CLI is now verb-first —
> `workestrate workload {up,exec,plan,down,logs} <name>`; the old
> `workestrate <wl> <verb>` shape still works via a one-cycle stderr alias
> shim. The baseline-capture commands in this step are transcripts of
> commands actually run against the old binary pinned at commit `840e8b7`,
> which only understands the old argv — the old shape is preserved verbatim
> below.

In the worktree, build and run the old binary for each of the 5 workloads:

```sh
mkdir -p /tmp/baseline
for name in litellm pi odysseus opencode tempest; do
  cargo run --manifest-path /tmp/workestrate-base/control/agentctl/Cargo.toml \
    -- "$name" plan > "/tmp/baseline/${name}.plan.txt"
done
```

**Cautions (verified from the old source):**

1. **Project root resolution.** The old binary resolves the project root via
   `crate::config::project_root()` (`control/agentctl/src/config.rs:1-31` at
   commit `840e8b7`), with precedence: (1) `AGENTCTL_ROOT` env var, (2)
   `CARGO_MANIFEST_DIR` (set by `cargo run` to
   `/tmp/workestrate-base/control/agentctl`, popped twice →
   `/tmp/workestrate-base`), (3) `std::env::current_dir()`. The root **must
   contain `flake.nix`** or the binary bails. The worktree at `840e8b7`
   contains `flake.nix` at its root, so `CARGO_MANIFEST_DIR` resolution
   succeeds. **Pin `AGENTCTL_ROOT=/tmp/workestrate-base` explicitly** for
   reproducibility:

   ```sh
   AGENTCTL_ROOT=/tmp/workestrate-base cargo run --manifest-path /tmp/workestrate-base/control/agentctl/Cargo.toml -- "$name" plan
   ```

2. **No `WORKESTRATE_HOME` / registry.** The old binary predates the
   tool+XDG+registry model (ADR 0023). It has no `WORKESTRATE_HOME`, no
   config-repo registry, no contexts. Secrets are `const` definitions in
   `microsandbox/secrets.rs` (e.g. `LITELLM_MASTER_KEY`, `GITHUB_TOKEN`),
   referenced via `HostBoundSecret::from(...)`. The `plan` subcommand emits
   `${VAR}` placeholders for secret env vars (rendered as `NAME=(redacted)`
   by `EnvVar::fmt`), so **no decrypted secrets are needed** to generate
   plans. The SOPS age key is NOT required for this step.

3. **Absolute paths in plan output.** The old `pi.rs` (verified at
   `840e8b7:control/agentctl/src/workloads/pi.rs:15-60`) embeds
   `std::env::current_dir()` as the host side of the `/work` mount:

   ```rust
   let work_host = std::env::current_dir()
       .map(|p| p.to_string_lossy().into_owned())
       .expect("failed to determine current working directory for /work mount");
   // ...
   MountPlan::readwrite(work_host, "/work"),
   ```

   So the baseline plan for `pi` (and any other workload using `${CWD}`) will
   contain an absolute path. **Pin the cwd** for reproducibility:

   ```sh
   cd /tmp/workestrate-base && AGENTCTL_ROOT=/tmp/workestrate-base cargo run --manifest-path control/agentctl/Cargo.toml -- pi plan
   ```

4. **Cargo network access.** The old `Cargo.lock` deps may need fetching on a
   fresh worktree. This is online; if offline, ensure the cargo cache is warm.
   The dev shell (`nix develop`, via the store-path prefix) provides the `cc`
   linker and native deps (`aws-lc-rs`, `parking_lot_core`, etc.) required to
   build — runnable in this container (verified 2026-07-29: `cc --version` →
   gcc 15.2.0 inside `nix develop`).

### Step 3 — Generate NEW config-driven plans

```sh
mkdir -p /tmp/new
for name in litellm pi odysseus opencode tempest; do
  WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate \
    just workestrate workload plan "$name" > "/tmp/new/${name}.plan.txt"
done
```

**How `WORKESTRATE_HOME` is resolved** (verified at
`control/agentctl/src/config/paths.rs:55-160`):

The function `resolve_home_with_kind()` (`paths.rs:96-160`) applies this
precedence (first match wins):

1. **Env** — `WORKESTRATE_HOME` (used verbatim, leading `~/` expanded) →
   `HomeKind::Env`.
2. **Discovered** — a `.workestrate/config.toml` in a *trusted* ancestor of
   the cwd, but only when no `XDG_*_HOME` var is set. Trust is checked via the
   global registry (`is_dir_trusted_via_base_registry`); an untrusted
   `.workestrate/` prints a one-time warning, stops walking, and falls
   through.
3. **LegacyXdg** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
   set and non-empty (compatibility; one-time migration note).
4. **Default** — `~/.workestrate`.

**In-repo `.workestrate/` auto-discovery.** The repo's
`.workestrate/config.toml` registers `/home/node/Development/ai-workbench` as
a trusted project (verified: `[[trusted_projects]] path =
"/home/node/Development/ai-workbench"` in `.workestrate/config.toml`). So when
running from the repo root with no `WORKESTRATE_HOME` set and no `XDG_*_HOME`,
discovery resolves to `.workestrate/` automatically (`HomeKind::Discovered`).
Setting `WORKESTRATE_HOME=.../.workestrate` explicitly (`HomeKind::Env`) is
the deterministic, cwd-independent form and is preferred for the baseline
procedure.

The `just workestrate` recipe (`justfile:135-136`) is a passthrough:

```makefile
workestrate *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}
```

It does NOT set `WORKESTRATE_HOME`; the env var must be exported or prefixed
on the command line.

### Step 4 — Diff procedure

#### 4a. Byte-diff after normalization

Both old and new plans embed the absolute cwd for `${CWD}`-based mounts (old:
`std::env::current_dir()` in `pi.rs`; new: `resolve_mount_host_template` at
`control/agentctl/src/microsandbox/workload/validate.rs:110-139` resolves
`${CWD}` → `std::env::current_dir()`). Normalize before diffing:

```sh
mkdir -p /tmp/norm
for name in litellm pi odysseus opencode tempest; do
  # Replace absolute paths under /tmp or $HOME with the literal token ${CWD}
  sed -E "s|$(pwd)|\${CWD}|g; s|/tmp/workestrate-base|\${CWD}|g" \
    "/tmp/baseline/${name}.plan.txt" > "/tmp/norm/old-${name}.plan.txt"
  sed -E "s|$(pwd)|\${CWD}|g; s|/tmp/workestrate-base|\${CWD}|g" \
    "/tmp/new/${name}.plan.txt" > "/tmp/norm/new-${name}.plan.txt"
  diff -u "/tmp/norm/old-${name}.plan.txt" "/tmp/norm/new-${name}.plan.txt"
done
```

#### 4b. Field-by-field comparison table

For each workload, compare these fields between old and new plans:

| Field | Display line | Notes |
|---|---|---|
| `name` | `name: <name>` | Sandbox instance name |
| `image` | `image: <image>` | Recipe-resolved image ref |
| `workdir` | `workdir: <dir>` | |
| `command` | `command: <bin> <args...>` | Space-joined |
| `cpus` | `cpus: <n>` | |
| `memory` | `memory: <n> MiB` | |
| `env` (names) | `env: NAME=...` | Secret envs render `NAME=(redacted)`; compare names + `is_secret` flag, redact values |
| `secret_env` | `secret_env: NAME (value redacted, allowed: <hosts>, <required\|optional>)` | Host-bound secrets |
| `ports` | `port: <host>:<guest>` | |
| `mounts` | `mount: <host>:<guest>[(ro)]` | Normalize `${CWD}`/absolute paths |
| `network.default_deny` | `network: default_deny=<bool>` | |
| `ingress_rules` | `  ingress: <proto>:<port> <scope>` | |
| `egress_rules` | `  egress: <proto>:<port> -> <target>` | |
| `deny_rules` | `  egress: deny domain suffix <suffix>` | |

#### 4c. Semantic checklist (from `network.rs` unit tests)

The following invariants are asserted by unit tests in
`control/agentctl/src/microsandbox/runtime/network.rs` (line numbers verified).
These pin post-migration behavior that must match the old hardcoded behavior.
Check each against BOTH old and new plans:

| Test (file:line) | Workload | Invariant asserted |
|---|---|---|
| `litellm_network_plan_converts_without_error` (`network.rs:78`) | litellm | Network plan converts to policy without error |
| `pi_network_plan_converts_without_error` (`network.rs:91`) | pi | Network plan converts to policy without error |
| `pi_plan_uses_nix_built_image` (`network.rs:100`) | pi | `image == "workestrate-pi:latest"` (nix-built, NOT `node:24-bookworm-slim`) |
| `odysseus_network_plan_converts_without_error` (`network.rs:111`) | odysseus | Network plan converts to policy without error |
| `odysseus_plan_includes_admin_password_secret` (`network.rs:124`) | odysseus | `ODYSSEUS_ADMIN_PASSWORD` present as secret env var; NOT in `secret_env` (not host-bound) |
| `odysseus_plan_has_expected_data_mount` (`network.rs:149`) | odysseus | 2 mounts; `/data` ← `workspaces/odysseus-state` (rw); `/app` ← `agents/odysseus/build` (ro); NO `/app/data` mount |
| `odysseus_plan_uses_data_dir_env_and_drops_hardcoded_db_url` (`network.rs:173`) | odysseus | `ODYSSEUS_DATA_DIR=/data` set; `DATABASE_URL` NOT hardcoded |
| `opencode_network_plan_converts_without_error` (`network.rs:193`) | opencode | Network plan converts to policy without error |
| `pi_plan_has_expected_egress` (`network.rs:206`) | pi | `default_deny=true`; 4 egress rules; rule[0] tcp:53→host; exactly one tcp:443 rule → `github.com, api.github.com` |
| `pi_plan_redirects_config_to_data_dir` (`network.rs:238`) | pi | `PI_CODING_AGENT_DIR=/data/agent` set; `PI_OFFLINE` NOT set |
| `pi_plan_exposes_litellm_master_key_as_env_not_host_bound` (`network.rs:258`) | pi | `LITELLM_MASTER_KEY` in `env` (secret); NOT in `secret_env` (not host-bound) |
| `pi_plan_has_expected_mounts` (`network.rs:284`) | pi | 2 mounts; NO `/app` mount (baked into image); `/data` ← `workspaces/pi-state` (rw); `/work` (rw); NO `/workspace` mount |

### Step 5 — Acceptance criteria

**Parity** means: exact byte-match after normalization (§4a) for all 5
workloads, PLUS every invariant in the semantic checklist (§4c) holds for the
new plans.

If a byte-diff reveals a delta, it must be either:

- **Normalized away** (absolute path / cwd token) — acceptable.
- **An intentional, justified delta** — each must be listed in a table:

  | Workload | Field | Old value | New value | Justification | ADR |
  |---|---|---|---|---|---|
  | _(none expected; populate if found)_ | | | | | |

Any unjustified delta is a regression and blocks merge.

---

## Lane A gate suite (no-KVM, verifiable-here)

Lane A = config-plane and plan-plane validation that does NOT require KVM or
decrypted secrets. These commands validate the config-driven setup is
structurally correct and produces well-formed plans. Runtime parity (actual
sandbox execution) is Lane B / HOST-KVM — see
[05-host-validation.md](05-host-validation.md).

> **Run-here note.** A bare shell in this container has no `cc` linker, but
> nix IS installed at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`
> (not on PATH), and `nix develop` provides a full C toolchain. Prefix with
> `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
> then run `nix develop -c bash -c '<cmd>'` (verified 2026-07-29:
> `cc --version` → gcc 15.2.0; `cargo check` compiles in ~27s; first devshell
> build takes minutes). So the cargo-based commands below are runnable in this
> container via `nix develop` (`verifiable-here`). The read-only git / grep /
> file inspections in §"Why there is no committed golden" were run here in a
> bare shell and their outputs pasted verbatim.

> **Bundle-load blocker (CRITICAL) — edit LANDED, runtime verification PENDING.**
> The `install_layout = "app"` field has been REMOVED from
> `.workestrate/repos/personal/workestrate.toml:317` (fix e applied; verified:
> `grep -n install_layout .workestrate/repos/personal/workestrate.toml` → no
> match), so the parse no longer hard-errors. What remains is the runtime
> verification itself — `workestrate validate-config` is cargo-linked and runs
> in this container via `nix develop` (nix at
> `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`; verified
> 2026-07-29: `cc --version` → gcc 15.2.0 inside `nix develop`); it is the
> first Lane A action (`export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
> then `nix develop -c bash -c 'just workestrate validate-config'`). The
> historical note below is kept for the record.
>
> **Historical (pre-fix).** Even on a host with `cc`, the personal
> config layer previously **FAILED to load** due to the tempest `install_layout`
> drift: `.workestrate/repos/personal/workestrate.toml:317` sets
> `install_layout = "app"` in tempest's `binary` inline table, but
> `BinarySpec` (`control/agentctl/src/config/types.rs:44-52`) has
> `#[serde(deny_unknown_fields)]` and no `install_layout` field (the nix-side
> param was removed as a silent no-op, `nix/lib/recipes/npm-build.nix:16-25`).
> Consequence: every Lane A command below that loads the real personal bundle
> (`validate-config`, `check`, `doctor`, `<name> plan` for any of the 5
> workloads, `secrets-schema`, `generate-env-example`, `config list`,
> `context list/current`) **hard-errored at parse time** until the one-line
> bundle edit landed (remove `install_layout = "app"` from the inline table —
> see [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)
> §"Bundle fixes needed" item e; **now APPLIED**). Baseline recovery for tempest
> (and any whole-config load) was **blocked** until that fix landed. The synthetic
> `config.reference` gates (A2 `golden-check`) are unaffected — they use
> `WORKESTRATE_CONFIG_DIR=config.reference`, which does not load the personal
> layer.

### A1. `just verify` — full pre-merge gate suite

```sh
just verify
```

Component gates (verbatim from `justfile:71`):

```
verify: toolchain-check check test spec-examples litellm-check golden-check schema-check scaffold-check lint-nix store-audit
```

| Gate | What it checks | Env |
|---|---|---|
| `toolchain-check` | `rustc` major.minor matches `RUST_TOOLCHAIN_VERSION` in `flake.nix` | `verifiable-here` |
| `check` | `cargo check` (compile) | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `test` | `cargo test` (unit + integration, incl. `network.rs` invariants) | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `spec-examples` | Every fenced TOML block in `20-target-system-spec.md` parses against the schema | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `litellm-check` | LiteLLM config YAML validity | `verifiable-here` (falls back to `nix develop -c python3`) |
| `golden-check` | Synthetic 3 golden plan parity (see A2) | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `schema-check` | `schemas/workestrate.schema.json` drift guard | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `scaffold-check` | `workestrate config new` scaffold drift guard | `verifiable-here` via `nix develop` (bare shell lacks `cc`) |
| `lint-nix` | Nix purity lint (`scripts/check-nix-paths.sh`) | `verifiable-here` |
| `store-audit` | Nix store closure-size audit | `verifiable-here` (skips if nix absent) |

**Expected outcome:** all gates pass; `git diff --exit-code HEAD --
control/agentctl/Cargo.lock` (the trailing lockfile-stability check) passes.

### A2. `just golden-check` — synthetic 3 only

```sh
just golden-check
```

**Why this does NOT cover the original 5:** `golden-check` iterates only over
`example-service example-agent example-offensive` (`justfile:87`), which are
the synthetic `config.reference` workloads (ADR 0017). The original 5
(`litellm pi odysseus opencode tempest`) have no committed golden files (see
§"Why there is no committed golden for the original 5"). Parity for the
original 5 is proven by the baseline recovery procedure (§3), not by
`golden-check`.

**Expected outcome:** no diff for all 3 synthetic plans; exit 0.

### A3. Config validation

```sh
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate \
  just workestrate validate-config
```

Runs `cmd_validate_config` (`control/agentctl/src/commands/diagnostics.rs:375`),
which loads the active config and calls `config::validate_config(&config)`.

**Expected outcome:** prints `workestrate.toml is valid.`; exit 0.

### A4. `workestrate check` and `workestrate doctor`

```sh
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate check
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate doctor
```

- **`check`** (`cmd_check`, `diagnostics.rs:158`) reports: registry path +
  status, each config repo (name, url, ref, rev, dirty/clean, OK/MISSING),
  layers, trusted projects, active context + layers, and tool home + kind.
  Expected shape: `personal` layer, `ref main`, `rev d2cd0c3...`, `[OK]`,
  active context `personal`, tool home `.../.workestrate (Env)` (or
  `Discovered`).
- **`doctor`** (`cmd_doctor`, `commands/doctor.rs:218`) runs health checks:
  KVM, `nix`, `sops`, `age-keygen`, age key file, `msb`, home, config repos.
  Prints `=== workestrate doctor ===` then one line per check
  (`NAME: STATUS (message)`), with `→ remediation` hints. Overall verdict:
  `OK` / `WARN` / `FAIL` (exits 1 on FAIL). In this container, KVM/nix/sops
  are absent → expected `FAIL` here; on a nix-capable host with KVM, expected
  `OK`.

### A5. Plan generation for all 5 workloads

```sh
for name in litellm pi odysseus opencode tempest; do
  WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate \
    just workestrate workload plan "$name" > "/tmp/new/${name}.plan.txt"
  test -s "/tmp/new/${name}.plan.txt" || echo "EMPTY: $name"
done
```

**Expected outcome:** each command exits 0; each output file is non-empty and
matches the `SandboxPlan` Display format (starts with `name: <name>`, ends
with the `network:` block). No decrypted secrets required (secret envs render
as `(redacted)`).

### A6. Secrets schema and env example

```sh
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate secrets-schema
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate generate-env-example
```

- **`secrets-schema`** (`cmd_secrets_schema`, `commands/secrets_target.rs:86`)
  prints the `env_var` name of every `[secrets.*]` entry that has an `env_var`
  field, sorted. Verified from
  `.workestrate/repos/personal/workestrate.toml:3-48`: there are **8
  `[secrets.*]` entries**, of which **7 are `env_var`-backed** and **1 is an
  alias** (`LITELLM_AUTH`, which uses `source`/`exposed_as` instead of
  `env_var`). `secrets-schema` filters by `env_var` presence, so it lists
  exactly these 7:

  ```text
  GITHUB_TOKEN
  KIMI_CODE_API_KEY
  LITELLM_MASTER_KEY
  MINIMAX_CODING_API_KEY
  NEURALWATT_API_KEY
  ODYSSEUS_ADMIN_PASSWORD
  OPENROUTER_API_KEY
  ```

  > **Correction of a common miscount:** the personal config defines **8
  > secret definitions** but only **7 env-var-backed secrets**. `LITELLM_AUTH`
  > is a *remapped alias* (`source = "LITELLM_MASTER_KEY"`, `exposed_as =
  > "OPENAI_API_KEY"`) with no `env_var` field of its own, so it does NOT
  > appear in `secrets-schema` output. It DOES appear in workload
  > `secret_env` blocks (e.g. odysseus, opencode, tempest) as a host-bound
  > secret exposed under the name `OPENAI_API_KEY`.

- **`generate-env-example`** (`cmd_generate_env_example`,
  `diagnostics.rs:398`) emits a `.env.example` with one line per
  `env_var`-backed secret (same 7), each commented with its `description`.

**Expected outcome:** exit 0; output lists exactly the 7 env-var-backed
secrets above.

### A7. Config-plane introspection

```sh
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate config list
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate context list
WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate just workestrate context current
```

These subcommands exist (verified: `ConfigAction::List` at
`cli_actions.rs:111`; `ContextAction::{List, Current}` at
`cli_actions.rs:185-193`; dispatched in `main.rs`).

- **`config list`** (`cmd_config_list`, `commands/config_cmd.rs:580`) prints
  each registered config repo: `  <name>: <url> (ref <ref>, rev <short>,
  <dirty|clean>) [OK|MISSING]`, then `Layers: [...]` and `Trusted projects:`.
  Expected: `personal` layer, `ref main`, `rev d2cd0c3...`, `clean`, `[OK]`.
- **`context list`** (`cmd_context_list`, `config_cmd.rs:91`) prints
  `Contexts:` then `  <name> (default)` / `  <name>` with `    layers:
  <layers>`. Expected: `personal (default)`, `layers: personal`.
- **`context current`** (`cmd_context_current`, `config_cmd.rs:145`) prints
  the active context and resolution source (`env` / `default` / `bare`).
  Expected: `personal`, source `default` (when `WORKESTRATE_CONTEXT` unset and
  `default_context = "personal"` in `.workestrate/config.toml`).

**Expected outcome:** all exit 0; output matches the shapes above.

---

## Known gaps and risks

### G1. Secrets-dependent commands degrade without the SOPS age key

The SOPS age key is **HOST-only** (this container: `~/.config/sops/age/`
absent, `SOPS_AGE_KEY` unset — see [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)
§Environment honesty). Commands that decrypt secrets at runtime (`up`,
`exec`, `run`, `secrets-target <name>` with decryption) will fail without it.

**Lane A commands that work WITHOUT secrets** (plan-plane only; secrets
render as `(redacted)` / `${VAR}` placeholders):

- `validate-config`, `check`, `doctor` (degrades: sops/age checks WARN/FAIL)
- `<name> plan` for all 5 workloads
- `secrets-schema`, `generate-env-example`
- `config list`, `context list`, `context current`
- `golden-check` (synthetic 3)

**Lane A commands that DEGRADE without secrets:**

- `doctor` → `sops`, `age-keygen`, age-key-file checks report `FAIL`/`WARN`;
  overall verdict becomes `FAIL` (exit 1). The config-repo and home checks
  still pass.

**Lane B (HOST-KVM) commands that REQUIRE secrets:**

- `<name> up`, `<name> exec`, `run` — actual sandbox execution needs
  decrypted secret env vars injected into the guest.

### G2. `master` vs `main` branch mismatch affects `config update`

The personal config repo's registry entry declares `ref = "main"` (verified:
`.workestrate/config.toml` → `[configs.personal] ref = "main"`, `rev =
"d2cd0c3506b5641507883078d01b626368f9d163"`), but the clone at
`.workestrate/repos/personal` is on branch `master` (verified in
[01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle
fixes needed", item a).

`cmd_config_update` (`control/agentctl/src/commands/config_cmd.rs:509-557`)
resolves the git ref via:

```rust
let git_ref = entry_ref
    .and_then(|e| e.r#ref.as_deref())
    .unwrap_or("main")      // config_cmd.rs:548
    .to_string();
git_pull(&dest, &git_ref)?;  // pulls branch "main"
```

**Consequence:** `workestrate config update personal` runs `git pull main`
against a clone whose checked-out branch is `master`. Depending on the
remote's branch layout, this either fails (`ref 'main' not found`) or pulls
the wrong branch. The same `unwrap_or("main")` default appears at
`config_cmd.rs:605` (in `cmd_config_list` display) and
`diagnostics.rs:186` (in `cmd_check` display).

**Fix:** rename the clone's branch `master` → `main` (or re-clone). See
[06-improvements/02-main-standardization.md](06-improvements/02-main-standardization.md).

### G3. `config.reference` golden plans are synthetic, not behavioral

The 3 committed golden plans (`example-service/agent/offensive`) validate
that the `ConfigWorkload` + `SandboxPlan` Display pipeline is stable, but
they do NOT validate that the original 5 workloads' behavior is preserved.
That is the job of the baseline recovery procedure (§3). Do not treat
`golden-check` passing as evidence of original-5 parity.

---

## Environment markers

Reproduced from [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md)
§Environment markers:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). INCLUDES cargo-linked gates run via `nix develop` (nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH — prefix with `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"` then `nix develop -c bash -c '<cmd>'`; verified 2026-07-29: `cc --version` → gcc 15.2.0, `cargo check` compiles in ~27s). A bare shell (outside `nix develop`) has no `cc`. |
| `HOST-NIX` | Requires nix on the user's host: `nix build` image builds, `nix run nixpkgs#...` prefetch jobs, full `just verify-full`, and `just generate-schema` (devshell RUSTFLAGS/libcap-ng). Cargo-linked `just` gates are NOT here — they run in-container via `nix develop` (see `verifiable-here`). |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |
