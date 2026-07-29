# 05 — Host Validation (KVM + NIX runbook)

> **STATUS: NEEDS-KVM** — the entire file is a `HOST-KVM` / `HOST-NIX` runbook.
> None of B1–B12 can be executed in the authoring container (no `/dev/kvm`,
> no `nix` on PATH).
>
> Prerequisites / see-also:
> [README.md](README.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [04-baseline-validation.md](04-baseline-validation.md) ·
> [03-sibling-config-setup.md](03-sibling-config-setup.md)

This is the Lane B ordered runbook: everything that **cannot** be validated in
the authoring container and must run on the user's host in one batched pass. A
contextless operator on a KVM-capable, nix-capable host follows it top-to-bottom.

Every command below was verified against the real CLI surface in
`control/agentctl/src/main.rs:50-186` and
`control/agentctl/src/cli_actions.rs:14-96`. No flag is invented.

## Host prerequisites

Before B1, the host must satisfy all of the following. Each is checkable via
`workestrate doctor` (B3) but is listed here so a failed prerequisite is
diagnosed before any build is attempted.

| # | Prerequisite | How to verify | Source |
|---|---|---|---|
| 1 | KVM available | `ls -l /dev/kvm` exists and is openable | `doctor.rs:84-100` (`doctor_check_kvm`) |
| 2 | `nix` installed | `nix --version` succeeds | `doctor.rs:221-226` (`doctor_check_tool("nix", ...)`) |
| 3 | `sops` installed | `sops --version` succeeds | `doctor.rs:227-232` |
| 4 | `age-keygen` installed | `age-keygen --version` succeeds | `doctor.rs:233-238` |
| 5 | SOPS age key present | `~/.config/sops/age/ai-workbench-secrets.txt` exists, mode `600` | `doctor.rs:49-58` (`default_age_key_path`); `cli_actions.rs:128-131` (`ConfigAction::New` default `--age-key-file`); `setup-secrets.sh:156` |
| 6 | `msb` reachable | `msb --version` succeeds (or `MSB_PATH` set) | `doctor.rs:61-63,141-150` |
| 7 | Repo on branch `migration/tool-model` | `git branch --show-current` → `migration/tool-model` | `01-current-state-and-prereqs.md` §Environment honesty |
| 8 | Bundle `.workestrate/` present | `ls .workestrate/config.toml .workestrate/repos/personal/workestrate.toml` | `01-current-state-and-prereqs.md` §Bundle inventory |
| 9 | Secrets decrypted | `just setup-secrets init` (interactive) or `just setup-secrets update` | `justfile:148-149`; `setup-secrets.sh:663-679` (`cmd_init`) |
| 10 | `ODYSSEUS_ADMIN_PASSWORD` placeholder replaced | the secret is non-empty after `setup-secrets init` (the placeholder `change_me_before_first_boot` at `workestrate.toml:47` is a TOML marker, not a runtime value; the real value lives in `.env.enc`) | `workestrate.toml:43-48`; `01-current-state-and-prereqs.md` Bundle fix (d) |

### About `setup-secrets`

`just setup-secrets` (`justfile:148-149`) wraps `nix develop -c setup-secrets`,
which execs `scripts/setup-secrets.sh`. The script (`setup-secrets.sh`):

- Defaults `SOPS_AGE_KEY_FILE` to `~/.config/sops/age/ai-workbench-secrets.txt`
  (`setup-secrets.sh:156`), generating the key via `age-keygen` if absent
  (`setup-secrets.sh:185-196`).
- Reads required secret keys from `workestrate secrets-schema`
  (`setup-secrets.sh:163-168`).
- Opens an editor (`$EDITOR`, else `nano`/`vi`/`vim`) on a pre-filled dotenv
  buffer, validates it, and encrypts to `.env.enc` via sops
  (`setup-secrets.sh:545-579`).
- Supports non-interactive init when every required env var is already set
  (`setup-secrets.sh:496-513`).

## Ordered steps

### B1 — Build the workestrate CLI binary `HOST-NIX`

```bash
nix build .#workestrate
```

**Expected:** a `result` symlink whose `bin/workestrate` runs and prints
`--version`. This is the same gate as `just verify-full`
(`justfile:75-76`).

**Failure triage:** if the build fails on native crates (`aws-lc-rs`,
`parking_lot_core`), ensure you are building via `nix build` (which provides
`libcap_ng` via `nix/packages/agentctl.nix`), not bare `cargo build` outside
the devshell (`01-current-state-and-prereqs.md` §Devshell / vendor facts).

### B2 — Build the nix-layered workload images `HOST-NIX`

The two nix-built images are exposed as explicit flake outputs
(`flake.nix:210-213`, `flake.nix:349-368`):

```bash
nix build .#workestrate-pi    # pi sandbox image (dockerTools.buildLayeredImage)
nix build .#tempest            # tempest sandbox image
```

(Aliases `.#pi-image` and `.#tempest-image` also exist — `flake.nix:357,362` —
and are identical derivations.)

**Expected:** each produces a `result` symlink pointing at a gzipped Docker
image tarball. Load them into microsandbox with `just load-images`
(`justfile:205-206`), which iterates the `workload-images` attrset and pipes
each through `gunzip | msb load` (`flake.nix:223-238`).

**Tempest FOD hash placeholder fix (if the build fails on hash mismatch):**
`flake.nix:184-187` and `nix/packages/tempest.nix` declare
`npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="` — a
placeholder. The config repo's `workestrate.toml:317` carries the same
placeholder as `npm_deps_hash`. The standard FOD hash discovery procedure
(`justfile:230-246`, `just update-hashes`):

1. Run `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`.
2. Copy the printed `sha256-...` value into **both**:
   - `nix/packages/tempest.nix` (`npmDepsHash`), and
   - `.workestrate/repos/personal/workestrate.toml:317` (`npm_deps_hash`).
3. Re-run `nix build .#tempest` to confirm.

**Failure triage:** a `hash mismatch` error with a `got:` line means the
placeholder is still present — copy the `got:` hash and re-pin.

### B3 — `workestrate doctor` `HOST-KVM` `HOST-NIX`

```bash
workestrate doctor
```

**Expected:** all checks `OK` (or `WARN`), overall verdict not `FAIL`. The
checks run (`doctor.rs:218-242`) are:

| Check | What it verifies | Source |
|---|---|---|
| `dev_kvm` | `/dev/kvm` exists and is openable | `doctor.rs:84-100` |
| `nix` | `nix --version` on PATH | `doctor.rs:221-226` |
| `sops` | `sops --version` on PATH | `doctor.rs:227-232` |
| `age_keygen` | `age-keygen --version` on PATH | `doctor.rs:233-238` |
| `age_key_file` | age key file exists at the resolved path, mode `≤0700` | `doctor.rs:110-139` |
| `msb` | `msb --version` reachable (`MSB_PATH` or `msb` on PATH) | `doctor.rs:141-150` |
| `home` | `WORKESTRATE_HOME` exists | `doctor.rs:152-161` |
| `config_repos` | every registered config repo clone exists and is clean | `doctor.rs:163-213` |

A `FAIL` anywhere flips the exit code to 1 (`doctor.rs:281-283`).

**Failure triage:** each `FAIL`/`WARN` line prints a `→ remediation` hint
(`doctor.rs:261-263`); follow it.

### B4 — `workestrate validate-config` `HOST-KVM`

```bash
workestrate validate-config
```

**Expected:** exit 0, no policy/schema violations. This is the same loader
used at runtime (`main.rs:256`, `commands/diagnostics::cmd_validate_config`).

**Failure triage:** the error names the offending field and the violated
allowlist/schema rule; fix `workestrate.toml` and re-run.

### B5 — Boot the LiteLLM proxy `HOST-KVM`

```bash
workestrate litellm up          # detached by default (ServiceAction::Up, cli_actions.rs:14-38)
workestrate litellm logs        # tail the detached log (cli_actions.rs:49-54)
curl -s -o /dev/null -w "%{http_code}" http://localhost:4000/health/liveliness
```

**Expected:** `workestrate litellm up` returns after detaching the sandbox;
`litellm logs` shows the proxy boot sequence ending in `LiteLLM Proxy running
on http://0.0.0.0:4000`; the curl returns `200`.

Port 4000 is from `workestrate.toml:83-85` (`host = 4000, guest = 4000`) and
the `PORT` env (`workestrate.toml:59-61`). The `/health/liveliness` route is
served by the LiteLLM proxy image
(`ghcr.io/berriai/litellm:v1.89.4`, `workestrate.toml:52`).

**Failure triage:** if `up` exits non-zero, check `workestrate litellm logs`
for the proxy's stderr; a config parse error points at
`infra/litellm/config.yaml` (mounted at `/app/config`, `workestrate.toml:92-95`).

### B6 — LiteLLM smoke test `HOST-KVM`

This step reuses the exact procedure defined in
`.agents/skills/validation-litellm-smoke/SKILL.md` (lines 36-54) so this doc
and the skill agree verbatim. `LITELLM_MASTER_KEY` must be exported in the
shell (it is the `LITELLM_MASTER_KEY` secret, `workestrate.toml:3-7,67-69`).

```bash
BASE=http://localhost:4000
KEY=$LITELLM_MASTER_KEY

# 1. Model aliases are exposed.
curl -s -o /tmp/models.json -w "%{http_code}" \
  "$BASE/v1/models" -H "Authorization: Bearer $KEY"

# 2. Liveliness probe.
curl -s -o /tmp/health.txt -w "%{http_code}" \
  "$BASE/health/liveliness"

# 3. Chat completion round-trip with a model alias.
curl -s -o /tmp/chat.json -w "%{http_code}" \
  "$BASE/v1/chat/completions" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"<model-alias>","messages":[{"role":"user","content":"ping"}],"max_tokens":1}'
```

**Pass criteria** (from `validation-litellm-smoke/SKILL.md:56-63`):

- `GET /v1/models` → HTTP 200 and the body lists every `model_name` alias
  defined in `model_list`.
- `GET /health/liveliness` → HTTP 200.
- `POST /v1/chat/completions` → HTTP 200 and the body contains a `choices`
  array with non-empty `message.content`.

Use a cheap/free alias (e.g. `coding.free`) and `max_tokens: 1` to minimize
provider token cost (`validation-litellm-smoke/SKILL.md:86-87`).

**Failure triage:** a 401 → `LITELLM_MASTER_KEY` not exported or wrong; a 404
on `/v1/models` → wrong port or proxy not booted; a provider error body on the
chat call → provider credential invalid (check the `secret_env` block,
`workestrate.toml:71-81`).

### B7 — Boot Odysseus `HOST-KVM`

```bash
workestrate odysseus up
curl -s -o /dev/null -w "%{http_code}" http://localhost:7000/api/health
```

**Expected:** `workestrate odysseus up` detaches; the curl returns `200` with
a body of `{"status":"healthy",...}`.

Port 7000 is from `workestrate.toml:207-209` (`host = 7000, guest = 7000`)
and the `APP_PORT` env (`workestrate.toml:169-171`). The health route is
`GET /api/health` — verified in the odysseus source at
`agents/odysseus/repo/app.py:864-866`:

```python
@app.get("/api/health")
async def health_check() -> Dict[str, str]:
    return {"status": "healthy", "timestamp": datetime.now(timezone.utc).isoformat()}
```

(A readiness probe `GET /api/ready` also exists at `app.py:868`, returning 503
until every critical dependency is up — useful as a stricter check.)

**Failure triage:** if `up` fails, `workestrate odysseus logs`; the
`local_build` recipe (`workestrate.toml:241-248`) runs `pip-install` into
`.deps` — a missing `WORKESTRATE_ODYSSEUS_BUILD` falls back to
`agents/odysseus/build`.

### B8 — Pi agent attach smoke `HOST-KVM` (manual / interactive)

```bash
workestrate pi exec
```

**Expected:** the pi sandbox boots and attaches an interactive TUI
(`AgentAction::Exec`, `cli_actions.rs:64-83`). The operator confirms the agent
prompt appears and the `${CWD}` mount (`workestrate.toml:141-144`) is visible
at `/work` inside the sandbox.

**CLI limitation — no headless in-sandbox exec:** the workestrate CLI exposes
**no generic shell/exec command that runs an arbitrary command inside a
sandbox**. `workestrate run` (`main.rs:75-80`) runs a command **on the host**
with decrypted secrets, not inside a sandbox. The only in-sandbox entry for an
agent workload is `exec`, which is an interactive TUI attach
(`cli_actions.rs:66-67`: "Attach to the sandbox interactively (TUI)").

Therefore B8 is a **manual verification step**: the operator attaches
interactively, confirms the prompt, then detaches (`Ctrl-C` or the agent's
exit command). There is no scriptable one-liner for this with the current CLI.

**Failure triage:** if the TUI fails to attach, `workestrate pi plan` to
confirm the image (`workestrate-pi:latest`) is loaded (`just load-images`);
a missing image → re-run B2 + `just load-images`.

### B9 — OpenCode and Tempest agent attach smoke `HOST-KVM` (manual / interactive)

```bash
workestrate opencode exec
workestrate tempest exec
```

**Expected:** each attaches an interactive TUI (`AgentAction::Exec`). The
operator confirms the agent prompt appears.

**Same CLI limitation as B8:** no headless in-sandbox exec exists. These are
**manual verification steps**.

**Failure triage:** opencode uses a registry image (`node:24-bookworm-slim`,
`workestrate.toml:252`) — no nix build needed, but the `local_build`
(`workestrate.toml:308-313`) must resolve `WORKESTRATE_OPENCODE_BUILD` or fall
back to `agents/opencode/build`. Tempest uses the nix-built `tempest:latest`
image (`workestrate.toml:317`) — ensure B2 + `just load-images` succeeded.

### B10 — Instance lifecycle: parallel instances, port offset, teardown `HOST-KVM`

This step exercises the ADR 0021 instance-lifecycle model
(`docs/migration/50-decisions/0021-instance-lifecycle-model.md`).

```bash
# 1. List running instances (reads the port registry).
workestrate ps

# 2. Start a parallel litellm instance (--new auto-allocates a slug id >= 2).
workestrate litellm up --new
#    Equivalent explicit form: workestrate litellm up --instance <id>
#    The instance name becomes <slot>@<id> (lifecycle.rs:59, main.rs:534-570).

# 3. Start a parallel instance with an explicit port offset (host ports += N).
workestrate litellm up --new --port-offset 10000

# 4. Stop the singleton AND every parallel instance of litellm.
workestrate litellm down --all-instances

# 5. Stop every running workestrate sandbox across ALL workloads/contexts.
workestrate down-all --yes
```

**Flags verified** (`cli_actions.rs:14-61`, `main.rs:582-620`):

| Flag | Scope | Effect | Source |
|---|---|---|---|
| `--foreground` | `up` (service) | block until Ctrl-C instead of detaching | `cli_actions.rs:17-18` |
| `--replace` | `up`/`exec` | tear down existing instance at this slot first | `cli_actions.rs:22-23,69-70` |
| `--instance <id>` | `up`/`exec`/`down`/`logs` | target `<slot>@<id>`; refuses if already running | `cli_actions.rs:27-28,42-43` |
| `--new` | `up`/`exec` | auto-allocate lowest free integer id ≥ 2 | `cli_actions.rs:32-33` |
| `--port-offset <n>` | `up`/`exec`/`plan` | add N to every **host** port (guest unchanged) | `cli_actions.rs:36-37`; ADR 0021 §5 |
| `--all-instances` | `down` | stop singleton + every parallel instance of this workload | `cli_actions.rs:46-47` |

`--replace`, `--instance`, and `--new` are **mutually exclusive**
(`lifecycle.rs:32-41`).

**Expected:**

- `workestrate ps` lists the singleton after B5/B7, then the parallel
  instance after `--new`, with distinct instance names and host ports.
- `--port-offset 10000` shifts the litellm host port from 4000 to 14000
  (guest stays 4000); the shifted port is checked against the port registry
  for collisions (ADR 0021 §5, `70-open-items.md:166-178`).
- `down --all-instances` removes every litellm instance; `down-all --yes`
  removes every sandbox across all workloads (`lifecycle.rs:355-411`).

**Failure triage:** a "port collision" error → another process holds the
shifted port; a "already running" error on `--instance <id>` → use `--replace`
or pick a different id.

### B11 — ORIGINAL-5 runtime parity `HOST-KVM`

Compare runtime behavior against the pre-migration baseline established in
[`04-baseline-validation.md`](04-baseline-validation.md) (Lane A).

**Plan parity (already done in Lane A):** `workestrate <name> plan` output
for all 5 workloads was diffed against the baseline in
`04-baseline-validation.md`. This step does **not** re-run plans.

**Live runtime parity — what can and cannot be checked headlessly:**

| Aspect | Checkable headlessly? | How |
|---|---|---|
| Service boot (litellm, odysseus) | Yes | B5 + B7 (health endpoints return 200) |
| Mounts visible inside sandbox | **No** | requires in-sandbox exec (see CLI limitation below) |
| Env (secrets redacted) | **No** | requires in-sandbox exec; secrets are injected via `secret_env` (`workestrate.toml:71-81,133-134,201-205,267-271`) and are not visible in `plan` output by design |
| Egress posture | **No** (headless) / Yes (interactive) | requires running a command inside the sandbox to test denied vs. allowed domains |

**CLI limitation — no generic in-sandbox exec:** as noted in B8, the CLI
exposes no command to run an arbitrary command inside a sandbox headlessly.
`workestrate run` (`main.rs:75-80`) runs on the **host** with decrypted
secrets, not inside a sandbox. Therefore:

- **Mounts / env parity** is verified via **plan parity** (Lane A) + successful
  service boot (B5/B7). The plan output shows every mount and env entry; if
  the plan matches the baseline and the sandbox boots, the mounts are wired.
- **Egress parity** requires **interactive** `workestrate pi exec` (B8). From
  inside the pi TUI, the operator verifies:
  - a denied domain fails: `curl https://evil.pi.dev` → connection refused
    (the `deny` rule at `workestrate.toml:152-153` blocks `.pi.dev`).
  - an allowed domain succeeds: `curl https://github.com` → 200 (the
    `agent_base` egress recipe at `workestrate.toml:149-150` permits it).

  The pi egress posture is `default_deny = true` + `agent_base` recipe +
  `deny .pi.dev` (`workestrate.toml:146-153`).

If interactive verification is not possible in a given pass, mark B11 egress
as **deferred to manual** and rely on plan parity + service health.

### B12 — Teardown and cleanup `HOST-KVM`

```bash
workestrate down-all --yes     # stop every running sandbox (main.rs:96-99, lifecycle.rs:355-411)
workestrate clean --yes        # remove state-dir contents: workspaces/, var/, run/ (main.rs:101-105, lifecycle.rs:418-464)
workestrate ps                 # confirm empty
```

**Expected:**

- `down-all --yes` skips the confirmation prompt (`main.rs:96-99`) and stops
  every running workestrate sandbox across all workloads/contexts
  (`lifecycle.rs:355-411`).
- `clean --yes` skips the confirmation prompt (`main.rs:101-105`) and removes
  the **contents** of `state/{workspaces,var,run}` — it never touches
  `repos/`, `sources/`, or any config file (`lifecycle.rs:413-417,421`).
- `workestrate ps` prints an empty instance list (exit 0).

**Failure triage:** if `down-all` reports "one or more instances failed to
stop" (`lifecycle.rs:407-409`), re-run `workestrate ps` to identify the
stragglers and `workestrate <name> down --instance <id>` them individually.

## Capability → proof → acceptance matrix

| Capability | Proof (step + command) | Acceptance criterion |
|---|---|---|
| Image build (pi) | B2: `nix build .#workestrate-pi` | `result` symlink produced; `just load-images` loads `workestrate-pi:latest` into microsandbox |
| Image build (tempest) | B2: `nix build .#tempest` (after FOD hash pin) | `result` symlink produced; `just load-images` loads `tempest:latest` |
| Config load | B4: `workestrate validate-config` | exit 0, no schema/policy violations |
| Secrets injection | B5: `workestrate litellm up` boots with `LITELLM_MASTER_KEY` from `.env.enc` | proxy boots and `/v1/models` returns 200 with `Authorization: Bearer $LITELLM_MASTER_KEY` |
| LiteLLM service boot | B5: `workestrate litellm up` + `curl /health/liveliness` | HTTP 200 |
| Provider egress | B6: `POST /v1/chat/completions` with a model alias | HTTP 200, non-empty `choices[0].message.content` |
| Odysseus service boot | B7: `workestrate odysseus up` + `curl /api/health` | HTTP 200, `{"status":"healthy"}` |
| Agent attach (pi) | B8: `workestrate pi exec` (interactive) | TUI prompt appears; `/work` mount visible |
| Agent attach (opencode) | B9: `workestrate opencode exec` (interactive) | TUI prompt appears |
| Agent attach (tempest) | B9: `workestrate tempest exec` (interactive) | TUI prompt appears |
| Parallel instances | B10: `workestrate litellm up --new` | `workestrate ps` lists `<slot>@<id>` with distinct host port |
| Port offset | B10: `workestrate litellm up --new --port-offset 10000` | host port = 4000 + 10000 = 14000; guest port unchanged |
| Teardown | B12: `workestrate down-all --yes` | `workestrate ps` empty |
| Plan parity (original-5) | B11 (Lane A baseline in `04-baseline-validation.md`) | `workestrate <name> plan` matches baseline for all 5 workloads |
| Runtime parity (mounts/env) | B11: inferred from plan parity + service boot | plan matches baseline AND services boot (no headless in-sandbox exec exists) |
| Runtime parity (egress) | B11: interactive `workestrate pi exec` | denied domain (`.pi.dev`) fails; allowed domain (`github.com`) succeeds |

## Environment honesty footer

None of B1–B12 was executed in the authoring container. This container has
no `/dev/kvm` and no `nix` on PATH (`01-current-state-and-prereqs.md` §Environment
honesty). Every step above is marked `HOST-NIX` or `HOST-KVM` as applicable:

| Step | Env marker | Why |
|---|---|---|
| B1 | `HOST-NIX` | `nix build` |
| B2 | `HOST-NIX` | `nix build` + FOD hash computation |
| B3 | `HOST-KVM` + `HOST-NIX` | probes `/dev/kvm`, `nix`, `sops`, `age`, `msb` |
| B4 | `HOST-KVM` | loads the real bundle (needs the runtime config stack) |
| B5 | `HOST-KVM` | sandbox runtime execution |
| B6 | `HOST-KVM` | live HTTP requests to a running sandbox |
| B7 | `HOST-KVM` | sandbox runtime execution |
| B8 | `HOST-KVM` | sandbox runtime execution (interactive) |
| B9 | `HOST-KVM` | sandbox runtime execution (interactive) |
| B10 | `HOST-KVM` | sandbox lifecycle (up/down/ps) |
| B11 | `HOST-KVM` | runtime parity (interactive egress check) |
| B12 | `HOST-KVM` | sandbox teardown |

All commands were verified against the CLI source (`main.rs:50-186`,
`cli_actions.rs:14-96`, `lifecycle.rs:22-68,355-464`) and the config
(`workestrate.toml`). No command or flag is invented.
