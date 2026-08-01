# 05 — Host Validation (KVM + NIX runbook)

> **STATUS: NEEDS-KVM** — the entire file is a `HOST-KVM` / `HOST-NIX` runbook.
> None of B1–B13 can be executed in the authoring container (no `/dev/kvm`,
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
workestrate workload up litellm          # detached by default (ServiceAction::Up, cli_actions.rs:14-38)
workestrate workload logs litellm        # tail the detached log (cli_actions.rs:49-54)
curl -s -o /dev/null -w "%{http_code}" http://localhost:4000/health/liveliness
```

**Expected:** `workestrate workload up litellm` returns after detaching the sandbox;
`workestrate workload logs litellm` shows the proxy boot sequence ending in `LiteLLM Proxy running
on http://0.0.0.0:4000`; the curl returns `200`.

Port 4000 is from `workestrate.toml:83-85` (`host = 4000, guest = 4000`) and
the `PORT` env (`workestrate.toml:59-61`). The `/health/liveliness` route is
served by the LiteLLM proxy image
(`ghcr.io/berriai/litellm:v1.89.4`, `workestrate.toml:52`).

**Failure triage:** if `up` exits non-zero, check `workestrate workload logs litellm`
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
workestrate workload up odysseus
curl -s -o /dev/null -w "%{http_code}" http://localhost:7000/api/health
```

**Expected:** `workestrate workload up odysseus` detaches; the curl returns `200` with
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

**Failure triage:** if `up` fails, `workestrate workload logs odysseus`; the
`local_build` recipe (`workestrate.toml:241-248`) runs `pip-install` into
`.deps` — a missing `WORKESTRATE_ODYSSEUS_BUILD` falls back to
`agents/odysseus/build`.

### B8 — Pi agent attach smoke `HOST-KVM` (manual / interactive)

```bash
workestrate workload exec pi
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

**Failure triage:** if the TUI fails to attach, `workestrate workload plan pi` to
confirm the image (`workestrate-pi:latest`) is loaded (`just load-images`);
a missing image → re-run B2 + `just load-images`.

### B9 — OpenCode and Tempest agent attach smoke `HOST-KVM` (manual / interactive)

```bash
workestrate workload exec opencode
workestrate workload exec tempest
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

### Experiment E1 — Guest reachability of non-127.0.0.1 loopbacks `HOST-KVM`

**Purpose:** feeds ADR 0026(f) DEFERRED-PENDING-E1; decides guest-facing
addressing for parallel alternates. The conservative default (guest-facing
alternates share `127.0.0.1` + `--port-auto`) holds until this experiment
runs.

**On the HOST** — bind four HTTP servers on different loopback IPs:

```bash
python3 -m http.server 8081 --bind 127.0.0.1 &
python3 -m http.server 8082 --bind 127.0.0.2 &
python3 -m http.server 8083 --bind 127.0.0.3 &
python3 -m http.server 8084 --bind 0.0.0.0 &
ss -tlnp | grep 808   # verify all four are listening
```

**From INSIDE a pi sandbox** (`workestrate workload exec pi`, interactive — same CLI
limitation as B8/B9: no headless in-sandbox exec):

```bash
getent hosts host.microsandbox.internal
ip route
curl -sS http://host.microsandbox.internal:8081/   # 127.0.0.1 bind
curl -sS http://host.microsandbox.internal:8082/   # 127.0.0.2 bind
curl -sS http://host.microsandbox.internal:8083/   # 127.0.0.3 bind
curl -sS http://host.microsandbox.internal:8084/   # 0.0.0.0 bind
```

Note which of the four succeed (HTTP 200 / directory listing) and which fail
(connection refused / timeout).

**Cleanup:** `kill` the four python servers.

**Decision table:**

| E1 outcome | ADR 0026 consequence |
|---|---|
| All four binds reachable from the guest | `127/8` is guest-reachable → parallel slots may publish guest-facing alternates on their own `127.0.0.N` |
| Only the `0.0.0.0` bind reachable | Guest-facing alternates publish `0.0.0.0` (host-only services keep `127.0.0.N`) |
| Only `127.0.0.1` reachable (`0.0.0.0` works or not) | Conservative default confirmed: guest-facing alternates share the `127.0.0.1` bind and differentiate with `--port-auto` |

The outcome MUST be recorded in
[06-improvements/12-per-instance-addressing.md](06-improvements/12-per-instance-addressing.md)
§open-decisions.

### B10 — Instance lifecycle: parallel instances, per-instance addressing, teardown `HOST-KVM`

This step exercises the ADR 0021 instance-lifecycle model
(`docs/migration/50-decisions/0021-instance-lifecycle-model.md`).

```bash
# 1. List running instances (reads the port registry).
workestrate ps

# 2. Start a parallel litellm instance (--new auto-allocates a slug id >= 2).
workestrate workload up litellm --new
#    Equivalent explicit form: workestrate workload up litellm --instance <id>
#    The instance name becomes <slot>@<id> (lifecycle.rs:59, main.rs:534-570).

# 3. Start a parallel instance publishing on its own per-instance IP.
workestrate workload up litellm --new
#    The parallel instance publishes 127.0.0.2:4000 (same guest port 4000,
#    bind differs; ADR 0026).

# 4. Stop the singleton AND every parallel instance of litellm.
workestrate workload down litellm --all-instances

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
| `--port-auto` | `up`/`exec` | lock-probed free port on the slot's bind (recorded in the instance record) | ADR 0026 (`--port-offset` was removed pre-release) |
| `--all-instances` | `down` | stop singleton + every parallel instance of this workload | `cli_actions.rs:46-47` |

`--replace`, `--instance`, and `--new` are **mutually exclusive**
(`lifecycle.rs:32-41`).

**Expected:**

- `workestrate ps` lists the singleton after B5/B7, then the parallel
  instance after `--new`, with distinct instance names and bind IPs.
- The parallel instance publishes `127.0.0.2:4000` (guest unchanged) while
  the singleton holds `127.0.0.1:4000`; collisions are keyed on
  `(bind_ip, port)` so no offset arithmetic is needed (ADR 0026).
- `down --all-instances` removes every litellm instance; `down-all --yes`
  removes every sandbox across all workloads (`lifecycle.rs:355-411`).

**Failure triage:** a "port collision" error → another process holds the
shifted port; a "already running" error on `--instance <id>` → use `--replace`
or pick a different id.

### B11 — ORIGINAL-5 runtime parity `HOST-KVM`

Compare runtime behavior against the pre-migration baseline established in
[`04-baseline-validation.md`](04-baseline-validation.md) (Lane A).

**Plan parity (already done in Lane A):** `workestrate workload plan <name>` output
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
- **Egress parity** requires **interactive** `workestrate workload exec pi` (B8). From
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
stragglers and `workestrate workload down <name> --instance <id>` them individually.

### B13 — Schema v2 secret-delivery smoke suite `HOST-KVM`

**Purpose:** runtime proof of the v2 unified secret/env model
([ADR 0018 addendum](../migration/50-decisions/0018-secrets-layering-and-per-repo-config.md),
commit `1ed2e6d`; P0 fix `d635ef0`). The static half is already proven
in-container (588 gates; see `02-config-requirements.md` §1.3.4 "Verification
status"); this suite is the HOST-KVM runtime half. Prerequisite: the full B1–B5
host prerequisite set (KVM, sops age key, decrypted `.env.enc`) plus a
**v2 personal config** in the active home — the migrated config lives at
`.tmp/config-repos-export/personal` @ `56f3557` (`personal-v2` @ `99c9985`); if
the live home checkout is still v1 (`c41a707`), refresh it as part of the
host-side home setup (NEXT-SESSION open thread 7) before items 1–7.

**Delivery model under test** (ADR 0018 addendum): `delivery = "env"` secrets
(`LITELLM_MASTER_KEY`, `ODYSSEUS_ADMIN_PASSWORD`, and the odysseus/opencode
`OPENAI_API_KEY` binding) land in the guest env as the REAL decrypted value.
`delivery = "host_bound"` secrets (the four provider keys, `GITHUB_TOKEN`) land
as `$MSB_*` placeholders in the guest env; the TLS proxy substitutes the real
value only for the bound egress hosts.

**CLI limitation (same as B8/B9/B11):** no headless in-sandbox exec exists.
In-guest env checks on agent workloads run inside the interactive TUI
(`workestrate workload exec <agent>`); service workloads (litellm, odysseus)
have no interactive attach, so their items use **behavioral proof** — an
authenticated request that can only succeed if the guest holds the real value.

1. **litellm master key — env delivery + P0 regression check** `HOST-KVM`

   ```bash
   workestrate workload plan litellm     # env line: LITELLM_MASTER_KEY=(redacted) — NOT a host-bound secret_env line
   workestrate workload up litellm
   curl -s -o /dev/null -w "%{http_code}\n" http://localhost:4000/v1/models \
     -H "Authorization: Bearer $LITELLM_MASTER_KEY"
   ```

   **Pass:** the plan renders `LITELLM_MASTER_KEY` as an `env` entry; the curl
   returns `200`. The 200 IS the P0 (`d635ef0`) regression check: had the
   literal template `"${LITELLM_MASTER_KEY}"` reached the guest (the P0 defect —
   secret-backed env entries resolved against the plan map instead of the
   merged secrets map), the proxy's master key would be that literal string and
   this request would 401. A 200 is only possible if the guest holds the REAL
   decrypted value.

2. **pi `models.json` substitutions resolve** `HOST-KVM`

   ```bash
   # host: clear any stale seeded file so substitution re-runs (seed is only_if_missing)
   workestrate workload exec pi          # interactive; litellm auto-starts by default
                                         # (depends_on default-on lifecycle — ADR 0026(d)
                                         #  as amended by the 2026-08-01 addendum)
   # inside the pi TUI:
   cat /data/agent/models.json
   ```

   **Pass:** no literal `${...}` template remains in the seeded file —
   `${LITELLM_MASTER_KEY}` is the REAL key and `${LITELLM_ADDR}` is the running
   litellm instance's actual `address:port`, matching the port-registry record
   shown by `workestrate ps` on the host.

3. **odysseus admin password — env delivery** `HOST-KVM`

   ```bash
   workestrate workload plan odysseus    # env line: ODYSSEUS_ADMIN_PASSWORD=(redacted) — delivery="env", no hosts
   workestrate workload up odysseus
   curl -s http://localhost:7000/api/health    # 200 baseline
   ```

   **Pass:** the plan renders `ODYSSEUS_ADMIN_PASSWORD` as an `env` entry (no
   `hosts`); an admin-authenticated odysseus request carrying the REAL password
   succeeds (`AUTH_ENABLED=true`, `LOCALHOST_BYPASS=false` — had the TOML
   placeholder `change_me_before_first_boot` or a `$MSB_*` placeholder reached
   the guest env, admin auth would reject every real-password request).

4. **odysseus/opencode `OPENAI_API_KEY` — the env-delivery flip** `HOST-KVM`

   ```bash
   workestrate workload plan odysseus    # env line: OPENAI_API_KEY=(redacted) — v1 was a host-bound LITELLM_AUTH secret_env line
   workestrate workload plan opencode    # same
   workestrate workload exec opencode    # interactive
   # inside the opencode TUI:
   env | grep OPENAI_API_KEY             # REAL value, not a $MSB_* placeholder
   ```

   **Pass:** `OPENAI_API_KEY` holds the REAL `LITELLM_MASTER_KEY` value in the
   guest env, and the agent completes a model call through litellm
   end-to-end. Context: this binding was host-bound in v1; on msb 0.5.6
   plain-HTTP the host-bound path substituted nothing, so this is the FIRST
   time odysseus/opencode auth to litellm actually works end-to-end on 0.5.6.

5. **litellm provider keys — host-bound placeholders + TLS substitution** `HOST-KVM`

   ```bash
   workestrate workload plan litellm
   # Pass criterion 1: the four provider keys render as host-bound secret_env
   # entries — $MSB_* placeholders, NEVER the real value.
   # Pass criterion 2: a chat completion per provider succeeds through the proxy
   # (proves TLS substitution to each allowed host):
   for alias in coding.free kimi-code-k3 minimax-m3 coding.fast; do
     curl -s -o /dev/null -w "%{http_code} $alias\n" http://localhost:4000/v1/chat/completions \
       -H "Authorization: Bearer $LITELLM_MASTER_KEY" \
       -H "Content-Type: application/json" \
       -d "{\"model\":\"$alias\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"max_tokens\":1}"
   done
   ```

   **Pass:** plan shows placeholders only; every alias returns `200`
   (`coding.free`→OPENROUTER_API_KEY, `kimi-code-k3`→KIMI_CODE_API_KEY,
   `minimax-m3`→MINIMAX_CODING_API_KEY, `coding.fast`→NEURALWATT_API_KEY).

6. **`GITHUB_TOKEN` — host-bound placeholder + git/gh substitution** `HOST-KVM`

   ```bash
   workestrate workload exec pi          # interactive
   # inside the pi TUI:
   env | grep GITHUB_TOKEN               # $MSB_* placeholder only, never the real value
   git ls-remote https://github.com/microsandbox/microsandbox HEAD
   gh api user                           # api.github.com
   ```

   **Pass:** the guest env holds only the placeholder; the git and gh requests
   to `github.com` / `api.github.com` SUCCEED from the agent sandbox (the TLS
   proxy substitutes the real token for the bound hosts).

7. **Failure semantics — refuse on missing, reject on placeholder** `HOST-KVM`

   ```bash
   # (a) required secret missing: blank a required key in .env.enc (sops edit), then:
   workestrate workload up litellm       # REFUSES to start, error NAMES the missing secret; non-zero exit
   # (b) secret set to its declared placeholder value (e.g. change_me_before_first_boot), then:
   workestrate workload up odysseus      # REJECTS (placeholder rejection); non-zero exit
   # restore .env.enc afterwards (sops edit back, or re-run just setup-secrets update)
   ```

   **Pass:** each run fails LOUDLY before any sandbox starts, naming the
   offending secret; restoring `.env.enc` returns `up` to green.

8. **v1 shim — legacy forms load with deprecation warnings and fold** `HOST-KVM`

   ```bash
   # Point the active home at a v1 (schema_version = 1) config — e.g. the
   # pre-migration personal checkout c41a707 — then:
   workestrate validate-config 2>/tmp/v1-shim-warnings.txt   # loads, exit 0
   grep -i "deprecat" /tmp/v1-shim-warnings.txt              # one warning per legacy form
   workestrate workload plan odysseus
   ```

   **Pass:** the v1 config LOADS (exit 0) with stderr deprecation warnings, and
   the fold is correct: each legacy `secret_env` array entry renders as an env
   map entry; the `[secrets.LITELLM_AUTH]` `source`/`exposed_as` remap folds to
   the direct binding `OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY" }`
   (plan shows the `OPENAI_API_KEY` env line); `description` fields are ignored
   silently. Restore the v2 home checkout afterwards.

## Capability → proof → acceptance matrix

| Capability | Proof (step + command) | Acceptance criterion |
|---|---|---|
| Image build (pi) | B2: `nix build .#workestrate-pi` | `result` symlink produced; `just load-images` loads `workestrate-pi:latest` into microsandbox |
| Image build (tempest) | B2: `nix build .#tempest` (after FOD hash pin) | `result` symlink produced; `just load-images` loads `tempest:latest` |
| Config load | B4: `workestrate validate-config` | exit 0, no schema/policy violations |
| Secrets injection | B5: `workestrate workload up litellm` boots with `LITELLM_MASTER_KEY` from `.env.enc` | proxy boots and `/v1/models` returns 200 with `Authorization: Bearer $LITELLM_MASTER_KEY` |
| LiteLLM service boot | B5: `workestrate workload up litellm` + `curl /health/liveliness` | HTTP 200 |
| Provider egress | B6: `POST /v1/chat/completions` with a model alias | HTTP 200, non-empty `choices[0].message.content` |
| Odysseus service boot | B7: `workestrate workload up odysseus` + `curl /api/health` | HTTP 200, `{"status":"healthy"}` |
| Agent attach (pi) | B8: `workestrate workload exec pi` (interactive) | TUI prompt appears; `/work` mount visible |
| Agent attach (opencode) | B9: `workestrate workload exec opencode` (interactive) | TUI prompt appears |
| Agent attach (tempest) | B9: `workestrate workload exec tempest` (interactive) | TUI prompt appears |
| Parallel instances | B10: `workestrate workload up litellm --new` | `workestrate ps` lists `<slot>@<id>` with distinct bind IP |
| Per-instance IP | B10: `workestrate workload up litellm --new` (second parallel) | `ps` shows `127.0.0.2:4000` vs singleton `127.0.0.1:4000`; guest port unchanged |
| Teardown | B12: `workestrate down-all --yes` | `workestrate ps` empty |
| Plan parity (original-5) | B11 (Lane A baseline in `04-baseline-validation.md`) | `workestrate workload plan <name>` matches baseline for all 5 workloads |
| Runtime parity (mounts/env) | B11: inferred from plan parity + service boot | plan matches baseline AND services boot (no headless in-sandbox exec exists) |
| Runtime parity (egress) | B11: interactive `workestrate workload exec pi` | denied domain (`.pi.dev`) fails; allowed domain (`github.com`) succeeds |
| Master key env delivery (P0) | B13.1: `workestrate workload up litellm` + `curl /v1/models -H "Bearer $LITELLM_MASTER_KEY"` | HTTP 200 (proves REAL value in guest, not the literal template) |
| models.json substitution | B13.2: interactive `workestrate workload exec pi` → `cat /data/agent/models.json` | no `${...}` literal remains; `${LITELLM_ADDR}` matches `workestrate ps` record |
| Admin password env delivery | B13.3: `workestrate workload plan odysseus` + admin-authenticated request | env line, no hosts; real-password auth succeeds |
| OPENAI_API_KEY env flip | B13.4: `workestrate workload exec opencode` → `env \| grep OPENAI_API_KEY` | REAL value in guest env; end-to-end litellm call works on 0.5.6 |
| Provider key host-binding | B13.5: plan + per-provider chat completion | `$MSB_*` placeholders only; `coding.free`/`kimi-code-k3`/`minimax-m3`/`coding.fast` all 200 |
| GITHUB_TOKEN host-binding | B13.6: interactive pi → `git ls-remote` / `gh api user` | placeholder in guest; github.com + api.github.com succeed |
| Secret failure semantics | B13.7: `up` with missing / placeholder-valued secret | refuses/rejects naming the secret, non-zero exit, no sandbox started |
| v1 shim fold | B13.8: `validate-config` + `plan` against a v1 config | exit 0 + stderr deprecation warnings; secret_env→env fold; remap→direct binding |

## Environment honesty footer

None of B1–B13 was executed in the authoring container. This container has
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
| E1 | `HOST-KVM` | guest reachability probe (interactive in-sandbox curl) |
| B11 | `HOST-KVM` | runtime parity (interactive egress check) |
| B12 | `HOST-KVM` | sandbox teardown |
| B13 | `HOST-KVM` | v2 secret-delivery runtime smoke (sandbox runtime + live HTTP) |

All commands were verified against the CLI source (`main.rs:50-186`,
`cli_actions.rs:14-96`, `lifecycle.rs:22-68,355-464`) and the config
(`workestrate.toml`). No command or flag is invented.
