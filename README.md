# workestrate

A local AI workbench that runs Pi, Odysseus, OpenCode, and T3MP3ST agents inside
Microsandbox microVMs, with LiteLLM as the unified LLM proxy. Everything
is driven from a single Rust CLI (`workestrate`) and orchestrated through
Nix flakes and SOPS-encrypted secrets.

## What is this

`workestrate` is a single-host sandbox for experimenting with
LLM-driven coding agents without giving them direct network or host
access. A small Rust CLI builds Microsandbox plans for one or more
agent microVMs and a local LiteLLM proxy; the proxy terminates
authentication, fans requests out to upstream providers, and exposes
an OpenAI-compatible endpoint at `http://host.microsandbox.internal:4000`.
Agent microVMs start with a default-deny network policy and only receive
the secrets needed to reach the proxy.

## Issue tracking

Work items (issues, claims, status) are tracked with beads (`bd`); see [BEADS.md](BEADS.md) for procedures. Issue prefix: `wrk`. Narrative state stays in the docs — beads tracks work, docs track state.

## Trust model

workestrate has a small number of deliberate escape hatches. Naming them
in one place:

- **`workestrate run` = full secret access BY DESIGN.** It decrypts
  `.env.enc` into the process environment and execs a command — that is
  its purpose. `run` is the operator's escape hatch for ad-hoc commands
  that need decrypted secrets; it is not sandboxed.
- **`WORKESTRATE_CONFIG_DIR` = root authority.** Whoever controls this env
  var controls which config loads (it bypasses discovery and merging).
  Env control = root.
- **`WORKESTRATE_HOME` = root of trust for the home.** Whoever sets it
  controls the registry, config repos, secrets, sources, and runtime
  state.
- **`trusted_projects` gating.** Project-layer config
  (`./workestrate.toml`, `./workestrate.local.toml`) only loads if cwd is
  in `[trusted_projects]`. Untrusted discovery prints a one-time warning
  and stops walking — a `cd` into an untrusted directory cannot silently
  inject config.
- **`.env.enc` is ciphertext-safe in repo** (SOPS-encrypted; safe to
  commit in config repos). The **age key is NEVER in repo** — the repo is
  agent-reachable via `${CWD}` mounts, so a key inside it would be exposed
  to sandboxes. The key stays at `~/.config/sops/age/ai-workbench-secrets.txt`
  on the host.
- **Network segmentation (default-deny egress) is the authoritative
  runtime control**, not credential binding. An agent that exfiltrates a
  key can only reach the explicitly-allowed hosts; the network policy —
  not the secrecy of the key — is what prevents misuse.

## Prerequisites

- Debian or Ubuntu on x86_64, with virtualization extensions enabled in
  firmware.
- `/dev/kvm` present and accessible to your user (typically membership
  in the `kvm` group).
- Nix with flakes enabled.
- At least 4 GB of RAM and 20 GB of free disk space in the working
  directory.
- No Docker required; Microsandbox talks to KVM directly.

Run `just host-check` (or `./scripts/host-check.sh`) to verify these
before continuing.

## Quick start

A fresh clone ships a **synthetic reference config** in `config.reference/`. It
lets you run `workestrate validate-config` and `workestrate workload plan example-service`
immediately, but it does not contain real workloads like `pi` or `litellm` — those
live in your personal config repo.

1. Clone the repository and enter the dev shell. The shell hook stages
   `msb` and `agentd` from the Nix store, uses a persistent cache at
   `$HOME/.cache/ai-workbench-msb` for `cargo check`/`build.rs`, cleans
   up legacy per-shell tmpfs dirs from earlier versions, and refreshes the
   `control/agentctl/vendor/microsandbox-filesystem-0.5.6` symlink.
   The dev shell pins `nodejs_24` (was `nodejs_22`; fixes pi's gondolin
   `EBADENGINE`) and exports `WORKESTRATE_PI_BUILD` pointing at the
   canonical `.#pi-bun` standalone binary, so dev-shell `workestrate workload
   exec pi` mounts the bun binary at `/app/bin/pi` (once a personal config repo
   is registered).
   ```bash
   git clone <repo-url> workestrate
   cd workestrate
   nix develop
   ```
2. Verify the workbench layout:
   ```bash
   nix run . -- check
   ```
3. Confirm the host is ready (KVM, Nix, memory, disk):
   ```bash
   just host-check
   ```
4. Create or import your personal config repo (contains `workestrate.toml`,
   `.env.enc`, `.sops.yaml`, `infra/litellm/`, `agents/*/config/`):

   **Create a new config repo** (recommended for first-time users):
   ```bash
   workestrate config new personal
   ```
   The repo is created in the managed store (`<store>/config-repos/personal`)
   and auto-registered as a layer — no `cd` needed to start using it. Use
   `workestrate config list` to see the path. Pass a destination positional
   (`workestrate config new <name> <dest>`) to scaffold elsewhere: the
   in-store default is registered automatically, while an out-of-store
   destination is scaffold-only — NOT registered, so the repo is not active
   for layer resolution until added via `workestrate config add`.

   **Import an existing config repo** (e.g. from a dotfiles backup):
   ```bash
   workestrate home init
   workestrate config add <your-config-repo-url> personal
   ```
5. Initialise encrypted secrets (one-time, see [Secrets setup](#secrets-setup)):
   ```bash
   setup-secrets --config personal init
   ```
6. Start the LiteLLM proxy (starts detached; add `--foreground` to block):
   ```bash
   workestrate workload up litellm
   ```
7. Attach to an agent (for example Pi):
   ```bash
   workestrate workload exec pi
   ```

   Services start detached by default: `workestrate workload up <svc>` returns
   immediately and the sandbox keeps running in the background. Use
   `workestrate workload up <svc> --foreground` (or `-f`) to block until Ctrl-C.
   Tail a detached service's logs with `workestrate workload logs <svc>` (written to
   `~/.microsandbox/sandboxes/<svc>/workestrate.log`). Detached mode works
   through `workestrate` — the detached child inherits the parent's
   decrypted environment, so `workestrate workload up litellm` starts detached
   and works without `nohup`.

## Secrets setup

Secrets live in your **personal config repo** (`$WORKESTRATE_HOME/config-repos/personal/`):
`.env.enc` (SOPS-encrypted) and `.sops.yaml` (SOPS recipient). The wrappers
`setup-secrets`, `decrypt-env`, and `write-env` (provided by the flake) use an
age key that lives outside all repos at
`$HOME/.config/sops/age/ai-workbench-secrets.txt`. `workestrate` loads secrets
from the active config repo before starting sandboxes or running commands.

1. Generate the project age key and create the config repo's `.env.enc` (one-time):
   ```bash
   setup-secrets --config personal init
   ```
   The config repo's `.sops.yaml` contains the project's age recipient.
   `setup-secrets init` will create a new age key and update `.sops.yaml`
   for you. If `.sops.yaml` already has a different recipient, the script will
   ask you to update it manually. The command either uses the required env vars
   (if all are set) or opens `$EDITOR` (falling back to `nano`, `vi`, or `vim`)
   with a pre-filled buffer of required and optional keys from
   `workestrate generate-env-example`. `init` refuses to overwrite an existing
   `.env.enc`.

   | Key | Used for |
   |---|---|
   | `LITELLM_MASTER_KEY` | Local LiteLLM proxy authentication (any `sk-…` string; `sk-change-me-local-only` is rejected) |
   | `OPENROUTER_API_KEY` | OpenRouter provider |
    | `KIMI_CODE_API_KEY` | Kimi for Coding provider |
    | `NEURALWATT_API_KEY` | Neuralwatt provider |
   | `MINIMAX_CODING_API_KEY` | MiniMax Coding provider |
   | `GITHUB_TOKEN` | GitHub Personal Access Token for agent sandboxes (git operations + API) |
   | `ODYSSEUS_ADMIN_PASSWORD` | Odysseus admin login (required because `AUTH_ENABLED=true`; without it Odysseus auto-generates a random password printed to logs) |

   The optional keys `AI_WORKBENCH_WORKSPACES_DIR` and
   `AI_WORKBENCH_VAR_DIR` are reserved for future use and are not yet
   consumed by the code; default paths are used regardless.

   Back up `~/.config/sops/age/ai-workbench-secrets.txt` to a secure
   location. Without this key, `.env.enc` cannot be decrypted.

2. Edit encrypted secrets later:
   ```bash
   setup-secrets --config personal update
   ```
   This decrypts the config repo's `.env.enc`, opens the editor with current
   values pre-filled, and re-encrypts on save. To rotate the master key
   non-interactively, export `LITELLM_MASTER_KEY` and run
   `setup-secrets --config personal update`. To update other values
   non-interactively, pipe the plain values on stdin in the order of
   `REQUIRED_KEYS` (one value per line, no `KEY=` prefix).

For the full threat model and wrapper reference, see
[docs/secrets.md](docs/secrets.md).

## Running the LiteLLM proxy

Inside the dev shell:

```bash
workestrate workload up litellm      # start (detached by default)
workestrate workload down litellm    # stop
workestrate workload logs litellm    # tail the detached service's log
nix run . -- workload plan litellm        # show the sandbox plan without secrets
```

Once the proxy is up, you can talk to it directly on the host:

```bash
# List the configured models (pretty-printed with jq)
workestrate run -- bash -c \
  'curl -sS http://127.0.0.1:4000/v1/models \
  -H "Authorization: Bearer $LITELLM_MASTER_KEY"' | jq

# Smoke-test a chat completion (pretty-printed with jq)
workestrate run -- bash -c \
  'curl -sS http://127.0.0.1:4000/v1/chat/completions \
  -H "Authorization: Bearer $LITELLM_MASTER_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"coding\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}"' | jq
```

The `bash -c` wrapper with single quotes is required because `$LITELLM_MASTER_KEY`
must be expanded by the inner shell (after `workestrate run` has decrypted it into
the environment), not by the outer shell (where it is unset).  Without this
wrapper the variable expands to empty and the request fails authentication.

These curls run on the host and reach the proxy at `127.0.0.1:4000`. From
inside the agent sandboxes, the same proxy is reached at
`http://host.microsandbox.internal:4000`.

The proxy is configured by the active config repo's `infra/litellm/config.yaml`
(which `include:`s `models.yaml`) and exposes coding-tier model names. Each tier
targets a different cost/capability point:

| Tier | Primary | Fallback chain | Use case |
|---|---|---|---|
| `coding` | Kimi K2.7 (`anthropic/kimi-for-coding`) | neural-kimi-k2.7-code → coding.free | Default coding work |
| `coding.fast` | Qwen 3.6 35B fast (Neuralwatt) | neural-qwen3.6-35b → coding | Quick edits, autocomplete |
| `coding.pro` | GLM-5.2 short (Neuralwatt) | neural-glm-5.2 → neural-kimi-k2.7-code | Complex refactoring, reasoning |
| `coding.free` | Qwen 3 Coder free (OpenRouter) | neural-qwen3.6-35b-fast → coding | Experimentation, no cost |
| `coding.vision` | Kimi K2.6 (Neuralwatt) | neural-qwen3.6-35b → neural-kimi-k2.7-code | Vision + code |
| `orchestrator` | GLM-5.2 (Neuralwatt, 1M ctx) | neural-glm-5.2-short → neural-qwen3.5-397b | Orchestration, long context |
| `lead` | GLM-5.2 short (Neuralwatt) | neural-glm-5.2 → neural-kimi-k2.7-code | Lead agent reasoning |
| `vision` | Kimi K2.6 (Neuralwatt) | neural-qwen3.6-35b | Vision-only tasks |

The `neural` umbrella alias and 11 `neural-*` direct-access aliases
(map to individual Neuralwatt catalog models) are also available for
agents that need to pin a specific model rather than a tier. The
`orchestrator`, `lead`, `vision`, and `coding.vision` tiers are
available for future agent wiring; no agent currently pins them.

The `minimax-m3` direct-access alias (MiniMax-M3 via the Anthropic Messages API at `api.minimax.io/anthropic`) is also available for agents that need to pin a specific model rather than a tier.

Kimi is routed through the `anthropic/` provider because its endpoint
speaks the Anthropic Messages API, not OpenAI's. All Neuralwatt models
use the `openai/` prefix with `api_base: https://api.neuralwatt.com/v1`.
`general_settings.master_key` reads `os.environ/LITELLM_MASTER_KEY`,
`general_settings.disable_spend_logs` is `true`, and
`litellm_settings.drop_params` is `true`. There is no
`completion_model` default; clients must specify the model explicitly
in every request.

Egress is locked down to DNS (`tcp/53` and `udp/53`) to the host
and `tcp/443` to `openrouter.ai`, `api.kimi.com`, `api.neuralwatt.com`, and `api.minimax.io`.
All OpenRouter models use the same `openrouter.ai` egress host.
Secret exposure is decided per binding by the `bound` property: `host` (the
default) renders the secret's placeholder in the guest and substitutes the
real value only in host-side traffic to the secret's `allowed_hosts`;
`guest` injects the real value as a plain environment variable and is
reserved for workloads that verify the credential. Provider API keys
(OpenRouter, Kimi, Neuralwatt, MiniMax) and `GITHUB_TOKEN` stay host-bound —
presenters such as pi receive only the placeholder. The only guest-bound
real values are the verifiers: `LITELLM_MASTER_KEY` on the LiteLLM proxy
(it verifies client auth at startup) and `ODYSSEUS_ADMIN_PASSWORD` on
Odysseus. A secret's `allowed_hosts` is its credential policy — the only
hosts its real value may be sent to.

## Running agents

Workload names (`litellm`, `pi`, `odysseus`, `opencode`, `tempest`) are defined by
your active config repo, not by the tool. A fresh clone only has the synthetic
reference workloads (`example-service`, `example-agent`, `example-offensive`) in
`config.reference/`. Register your personal config repo to access the real
workloads.

Workloads split into two kinds: **services** (e.g. `litellm`, `odysseus`) support
`up`/`down`/`logs`/`plan`; **agents** (e.g. `pi`, `opencode`, `tempest`) support
`exec`/`down`/`plan` (agents have no `up` or `logs` — you attach to them
interactively with `exec`).

```bash
# Services (start detached, tail with `logs`)
workestrate workload up litellm          # LiteLLM proxy (requires config repo defining litellm)
workestrate workload up odysseus         # Odysseus agent (service)
workestrate workload logs odysseus       # tail Odysseus's detached log

# Agents (interactive TUI attach)
workestrate workload exec pi            # attach to the Pi coding agent
workestrate workload exec opencode      # attach to the OpenCode coding agent
workestrate workload exec tempest       # attach to the T3MP3ST offensive-security agent

# Stop any workload
workestrate workload down <name>

# Show a sandbox plan without secrets
nix run . -- workload plan <name>            # e.g. nix run . -- workload plan example-service
```

### Instance lifecycle: slots, parallel instances, refuse-on-occupied

`up`/`exec` target a **slot** (the workload's sandbox identity). A slot is
either a **singleton** (`<workload>`, or `<context>-<workload>` when a
context is active) or a **parallel instance** (`<slot>@<id>`).

**Occupied-slot behavior (BEHAVIOR CHANGE):** `up`/`exec` on an occupied
slot **refuses** by default (was: silent replace). The error names the
occupying instance and the escape flags. This makes destructive restarts
explicit — an agent (or human) validating a config change can no longer
accidentally nuke a running baseline by re-running `up`.

```bash
# Refuse-safe defaults
workestrate workload up litellm                       # refuses if litellm slot is occupied
workestrate workload up litellm --replace             # explicit recycle (was the old default)
workestrate workload up litellm --instance canary   # parallel canary on 127.0.0.2:4000
workestrate workload up litellm --new               # auto-named canary on 127.0.0.N:4000

# Listing + teardown
workestrate ps                               # list running instances for the active context
workestrate ps --json                        # machine-readable (agents, CI)
workestrate workload down litellm --instance canary   # stop one parallel instance
workestrate workload down litellm --all-instances     # stop singleton + all parallel instances
workestrate down --all                       # stop everything (confirms unless --yes)
```

The singleton slot publishes on the shared bind `127.0.0.1` at the declared
ports (the well-known address static configs use, e.g.
`host.microsandbox.internal:4000`); parallel slots publish on per-instance
loopback IPs (`127.0.0.N`, `N >= 2`) drawn from a locked allocator in the
port registry. Collisions are keyed on `(bind_ip, port)` — the same port on
different bind IPs does not collide. `--port-auto` picks a lock-probed free
port on the slot's bind for cases where even the per-IP port must not be
assumed. See [ADR 0026](docs/migration/50-decisions/0026-per-instance-addressing-and-discovery.md)
and [ADR 0021](docs/migration/50-decisions/0021-instance-lifecycle-model.md).

### Blue-green config changes

The refuse-on-occupied default + `--new`/per-instance addressing make a safe
blue-green workflow for an agent (or operator) modifying the project native.
Example: validating a candidate LiteLLM `config.yaml` without touching the
serving proxy.

```bash
# 1. Edit the candidate config in your config repo (or a working copy).
# 2. Bring up a canary on its own per-instance IP alongside the live proxy.
workestrate workload up litellm --new
#    → live proxy stays on 127.0.0.1:4000; canary on 127.0.0.2:4000 (guest still :4000).

# 3. Smoke-test the canary.
workestrate run -- bash -c \
  'curl -sS http://127.0.0.2:4000/v1/chat/completions \
   -H "Authorization: Bearer $LITELLM_MASTER_KEY" \
   -H "Content-Type: application/json" \
   -d "{\"model\":\"coding\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}"' | jq

# 4a. Promote: stop the old singleton, start the new one on the singleton slot.
workestrate workload down litellm                     # stop the old singleton
workestrate workload up litellm --replace             # (slot is now free; --replace is belt-and-suspenders)
# 4b. Or roll back: stop the canary, leave the singleton untouched.
workestrate workload down litellm --all-instances     # or target the canary id from `ps`
```

This workflow is the reason per-instance addressing exists (ADR 0026): it
lets a canary and a live instance of the same workload coexist on the same
host long enough to compare them.

### Synthetic reference workloads

On a fresh clone without a config repo, you can still exercise the machinery:

```bash
workestrate workload plan example-service
workestrate workload plan example-agent
workestrate workload plan example-offensive
workestrate validate-config
```

### Secret loading

`workestrate` loads secrets from the active config repo's `.env.enc`
automatically before commands that need them (`exec`, `up`, `run`). Commands
that don't need secrets (`plan`, `check`, `new`, `completions`) skip decryption
entirely — they work on a fresh clone before a config repo is registered.

To run an arbitrary command with decrypted secrets:

```bash
workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'
workestrate run -- bash    # interactive shell with secrets
```

The `run` subcommand decrypts `.env.enc` via `sops`, loads all keys into the process environment, then execs the given command.

> **Note (landing, Track B):** `workestrate run` will warn when loading
> many secrets. This flag is landing via Track B and is not yet in the
> code.

Pi runs in its sandbox as a **bun standalone binary** at `/app/bin/pi` —
a self-contained executable with the Bun runtime embedded, so no node
or bun is needed inside the microVM at runtime. The binary is produced
by the `.#pi-bun` nix derivation and mounted at `/app` via
`WORKESTRATE_PI_BUILD`. The `.#pi` derivation (npm/node, exec'ing `node
/app/packages/coding-agent/dist/cli.js`) remains the runtime fallback if
the bun binary misbehaves. The bun-binary path through the microVM is
compile- and plan-verified but pending KVM runtime validation; `.#pi`
(node) is the fallback.

Note: `agents/pi/repo`, `agents/odysseus/repo`, `agents/opencode/repo`, and
`agents/tempest/repo` must be cloned into the `agents/` directory before
`workestrate workload up <name>` will work; `workestrate check` reports them as `[MISSING] (optional)`
and does not fail, but the corresponding `workestrate workload up <name>` command requires the
checkout to exist.

Agents reach the proxy at `http://host.microsandbox.internal:4000`.
Every agent binding defaults to host-bound (`bound = "host"`): the guest
sees the secret's placeholder and the real value is substituted only in
host-side traffic to the secret's `allowed_hosts`. Pi's
`LITELLM_MASTER_KEY = true` sugar is such a host-bound placeholder binding
— pi gets the placeholder, not the real key. Odysseus and OpenCode receive
`OPENAI_API_KEY` (remapped from `LITELLM_MASTER_KEY`), likewise host-bound.
Guest-bound real values (`bound = "guest"`) are reserved for verifiers: the
LiteLLM proxy's `LITELLM_MASTER_KEY` and Odysseus's
`ODYSSEUS_ADMIN_PASSWORD`. The authoritative security control for all agents is network
segmentation (default-deny egress); an agent that exfiltrates the key
can only reach the proxy (tcp/4000) and GitHub (tcp/443). Runtime
enforcement is unverified in M1 (compile-checked only).

The Pi sandbox plan sets `PI_OFFLINE=1` and `PI_TELEMETRY=0`, denies
the `domain suffix .pi.dev`, mounts `agents/pi/repo` and `workspaces/pi`,
and allows DNS (`udp/53`, `tcp/53`) to the host, `tcp/4000` to the
host for the LiteLLM proxy, and `tcp/443` to `github.com` and
`api.github.com` for GitHub access. The Odysseus sandbox plan runs
`python:3.12-slim` with
`uvicorn app:app --host 0.0.0.0 --port 7000`, publishes `7000:7000`,
mounts `agents/odysseus/repo`, `workspaces/odysseus-data`,
`~/.microsandbox/sandboxes/odysseus/data` → `/data` (persistent state),
and `agents/odysseus/config/settings.json` →
`/app/data/settings.json` (read-only, tracked file), and likewise allows DNS (`udp/53`, `tcp/53`) to the host,
`tcp/4000` to the host for the LiteLLM proxy, and `tcp/443` to
`github.com` and `api.github.com` for GitHub access. Both plans bind
`GITHUB_TOKEN` to `github.com` and `api.github.com` (its `allowed_hosts`).
Odysseus binds `OPENAI_API_KEY` (remapped from `LITELLM_MASTER_KEY`) to
`host.microsandbox.internal`; Pi's `LITELLM_MASTER_KEY` binding is the
host-bound placeholder sugar (`= true`, see above).

`agents/pi/repo`, `agents/odysseus/repo`, `agents/opencode/repo`, and
`agents/tempest/repo` are optional local overrides and are expected to be
absent on a fresh clone; `workestrate check` reports them as `[MISSING] (optional)`
and does not fail.

## Development workflow

Common `just` recipes:

| Recipe | What it does |
|---|---|
| `just check` | Run `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo check` for `control/agentctl` |
| `just litellm-check` | Validate `config.reference/infra/litellm/config.yaml` against the schema indexes |
| `just tombi-check` | TOML format/lint/schema gate via tombi 1.2.5 (repo, scaffolded config repos, homes) |
| `just verify` | Full pre-merge gate: `just check` plus `cargo test`, `just litellm-check`, `just tombi-check`, and `Cargo.lock` stability check |
| `just verify-full` | Heaviest validation: `just verify` plus `nix build .#workestrate` |
| `just build` | Build the `workestrate` binary |
| `just fmt` | Format the Rust code |
| `just fmt-check` | Check formatting without modifying files |
| `just clippy` | Run Clippy with `-D warnings` |
| `just test` | Run unit tests for `control/agentctl` |
| `just workestrate …` | Run `cargo run --manifest-path control/agentctl/Cargo.toml -- …` (e.g. `just workestrate workload plan litellm`) |
| `just plan` | Run synthetic workload plans (`example-service`, `example-agent`, `example-offensive`) via `cargo run` |
| `just host-check` | Verify KVM, Nix, memory, and disk prerequisites |
| `just validate-secrets` | Exercise the SOPS/age workflow against ephemeral test values |
| `just setup-secrets init` | Run `setup-secrets init` from the dev shell |
| `just vendor-unlock` | Replace the vendor symlink with a writable copy of the patched Microsandbox crate |
| `just vendor-lock` | Remove the vendor copy so the dev shell recreates the symlink |
| `just dev-build-pi` | Build pi into `agents/pi/build` with native npm (hashless local dev loop); workestrate falls back to `agents/pi/build` when `WORKESTRATE_PI_BUILD` is unset. Requires the dev shell's npm/node (run inside `nix develop`). |

> **Nix note:** New files must be `git add`-ed before `nix build` or `nix develop`
> will see them. Nix flakes only include git-tracked files in the source tree.

`control/agentctl/vendor/microsandbox-filesystem-0.5.6` is a Nix-managed
symlink to `${microsandbox-filesystem-patched}` from the flake. The dev
shell hook refreshes it on every entry. To inspect or temporarily
modify the patched source, run `just vendor-unlock` (this expands the
symlink into a real directory you can edit, with `chmod -R u+w`).
Run `just vendor-lock` to delete the directory; the next `nix develop`
recreates the symlink from the flake input.

## Nix build integration

Beyond the dev shell, the flake exposes hermetic agent derivations so a
full `nix build` produces ready-to-run artifacts without `nix develop`:

- `.#pi` — hermetic `buildNpmPackage` of the pi monorepo (npm-workspaces)
  from the remote fork (`github:georgrybski/pi`). Output tree laid out so
  `node $out/packages/coding-agent/dist/cli.js` resolves workspace siblings.
  `dontNpmBuild` + a custom buildPhase skip `generate-models` (offline;
  uses committed catalogs) and chain the four workspace builds in
  dependency order. `libcap_ng` is included for gondolin/libkrun.
- `.#pi-bun` — standalone Bun-compiled `pi` binary (~110 MB, Bun runtime
  embedded). Reuses the `.#pi` tree and runs `bun build --compile` on the
  bun entrypoint + image-resize worker, then mirrors upstream
  `copy-binary-assets` (themes, package.json, export-html templates, photon
  wasm) next to the binary so pi resolves package assets relative to
  `process.execPath`. **This is the canonical pi artifact** mounted at
  `/app` in the sandbox.
- `.#tempest-built` — hermetic `buildNpmPackage` of T3MP3ST from the
  remote fork (`github:georgrybski/T3MP3ST`). Single-package TypeScript
  app; `tsc` emits `dist/`. The tempest sandbox execs
  `node dist/cli.js` from the image's working directory.
- `.#tempest-image` — `dockerTools.buildLayeredImage` for the tempest
  sandbox. Provides nodejs_24 + nmap + bind.dnsutils + the compiled
  T3MP3ST tree + a baked `defaultProvider:"local"` config so T3MP3ST
  uses the env-var-driven local LLM provider (no conf-store secrets).
- `.#workestrate-sandbox` — `runCommand` + `makeWrapper` wrapper around
  `.#workestrate` that bakes `WORKESTRATE_PI_BUILD=${pi-bun}` into the
  environment, so `nix build .#workestrate-sandbox && ./result/bin/workestrate workload exec pi`
  runs the hermetic bun-binary pi sandbox with no `nix develop` and no
  extra env. `apps.default` points at this wrapped binary.

**Single canonical source.** Both `.#pi` and `.#pi-bun` build from one
source (the remote fork) with one `npmDepsHash`. The old `-local`/`-remote`
dual-output was collapsed: local pi hacking is **not** a nix override —
use `just dev-build-pi` (hashless native npm into `agents/pi/build`),
which workestrate picks up via the `agents/<name>/build` fallback when
`WORKESTRATE_PI_BUILD` is unset.

**Fork-carries-compat policy.** nix-build compatibility (patches,
lockfile, committed catalogs) lives on the agent fork itself, not as
nix-side patches in this repo. The flake consumes the fork verbatim.

**Per-agent build-path override.** `Workload::build_path()` reads
`WORKESTRATE_<NAME>_BUILD` (NAME uppercased, `-`→`_`) and falls back to
`agents/<name>/build`. This is the mechanism the `.#workestrate-sandbox` wrapper
uses to point workestrate at the nix store path for pi; the same mechanism
is available for future agent derivations (odysseus, opencode).

**Dev shell.** `nix develop` exports `WORKESTRATE_PI_BUILD` (canonical
bun pi) so dev-shell `workestrate` mounts the bun binary at `/app/bin/pi`.
pi is no longer built by the shellHook `_build_agents` auto-build; the
canonical artifact comes from the `.#pi-bun` derivation. odysseus and
opencode still auto-build on `nix develop` (via `pip --only-binary=:all:`
and `HUSKY=0 bun install` respectively) until their own derivations land.

**Runtime caveat.** The bun binary in the microVM, `up`/`exec`/`logs`,
and detached-mode + internal secret loading are compile- and
plan-verified but pending KVM runtime validation. `.#pi` (node) is the
fallback if the bun binary misbehaves at runtime.

## Derivation purity

Nix derivations in this flake must stay pure at both eval time and build
time. The rules, the motivating 29 GB-per-eval incident, the store-growth
model, and the new-derivation checklist live in
[docs/nix-purity.md](docs/nix-purity.md). The `just lint-nix` guard (backed
by `scripts/check-nix-paths.sh`) runs inside `just verify` and forbids the
common impurity patterns (`toString ./`, `getFlake`, `--impure`, bare
`src = ./.`, unfiltered `cleanSourceWith`). Run `just gc` and
`just store-audit` for store hygiene.

For the operational agent quick-reference (anti-accumulation patterns
table + verbatim rules), see
[`.agents/skills/nix-usage`](.agents/skills/nix-usage/SKILL.md).

## Shell completions

`workestrate` ships shell completions for bash, zsh, fish, elvish, and
powershell. Install them for `workestrate`:

```bash
# bash
workestrate completions bash > ~/.local/share/bash-completion/completions/workestrate

# zsh
workestrate completions zsh > ~/.zfunc/_workestrate

# fish
workestrate completions fish > ~/.config/fish/completions/workestrate.fish
```

Reload your shell (or `source` the completion file) afterwards.

## Architecture

```
                        ┌──────────────────────────────────────────────┐
                        │            Host (Nix + /dev/kvm)             │
                        │                                              │
  user ─── workestrate ──▶ │  ┌────────────┐    ┌───────────────────────┐  │
                         │  │  msb       │    │  litellm microVM      │  │
                         │  │ (Nix store)│    │  :4000  (in-memory)   │  │
                         │  └─────┬──────┘    └──────────┬────────────┘  │
                         │        │  drives              │               │
                          │   ┌────┴──────────┐  ┌────────┴──────────┐  ┌────────┴──────────┐  ┌────────┴──────────┐
                          │   │ pi microVM    │  │ odysseus microVM  │  │ opencode microVM  │  │ tempest microVM   │
                          │   │               │  │ :7000             │  │ :3000             │  │ (offsec agent)    │
                          │   └──┬────────────┘  └────────┬──────────┘  └────────┬──────────┘  └────────┬──────────┘
                          │      │     host.microsandbox.internal:4000           │                      │
                          │      │                        │                      │                      │
                          └──────┼────────────────────────┼──────────────────────┼──────────────────────┼──────────────┘
                                 │                        │                      │                      │
                                 ▼                        ▼                      ▼                      ▼
                     Upstream LLM providers (OpenRouter, Kimi, Neuralwatt, MiniMax)
```

- The Microsandbox SDK is pinned to `microsandbox = "=0.5.6"` with the
  `net` feature.
- The pi microVM runs a **bun standalone binary** (`/app/bin/pi`, from
  `.#pi-bun`) with the Bun runtime embedded; no node/bun is needed inside
  the sandbox. `.#pi` (npm/node) is the fallback. The `.#workestrate-sandbox`
  wrapper bakes `WORKESTRATE_PI_BUILD` so `nix build .#workestrate-sandbox` runs
  hermetic.
- Sandbox plans use a default-deny network policy; only the
  destinations listed above have explicit egress.
- `LITELLM_MASTER_KEY` is guest-bound (real value) only on the LiteLLM
  proxy — the verifier that checks client auth at startup. Everywhere else
  it is host-bound: Pi's binding uses the `LITELLM_MASTER_KEY = true`
  same-name sugar and pi receives the placeholder, and provider secrets
  (OpenRouter, Kimi, Neuralwatt) remain host-bound with their real values
  substituted only toward each secret's `allowed_hosts`.
- The authoritative security control is network segmentation:
  default-deny egress plus local-only ingress (`local_tcp(4000)` on the
  LiteLLM proxy). An agent inside a VM cannot reach any external host
except the explicitly-allowed ones (`openrouter.ai`, `api.kimi.com`,
`api.neuralwatt.com`, `api.minimax.io`, `github.com`). The network policy — not
  credential binding — is what prevents misuse of the key.
- Runtime enforcement of egress and secret isolation is designed but
  unverified in M1 (compile-checked only; requires a KVM host for
  runtime testing).
- LiteLLM runs in-memory; no Postgres, no virtual keys, no persistent
  spend tracking in M1.
- Odysseus and OpenCode receive `OPENAI_API_KEY` (remapped from
  `LITELLM_MASTER_KEY`) host-bound to `host.microsandbox.internal`.
- T3MP3ST (tempest) uses `default_deny: false` (broad egress) because it
  is an offensive-security tool that scans arbitrary targets. It connects
  to LiteLLM via the `local` provider (`TEMPEST_LOCAL_BASE_URL`), with
  `TEMPEST_LOCAL_API_KEY` remapped from `LITELLM_MASTER_KEY`. The
  `defaultProvider:"local"` config is baked into the image; no secrets
  are stored in the T3MP3ST conf store. `T3MP3ST_HOST` is set to
  `127.0.0.1` so the Express API server stays inside the microVM.

The top-level layout (already documented in
[`agents/README.md`](agents/README.md) and
[`profiles/litellm.md`](profiles/litellm.md)) is: `control/agentctl/`
(Rust CLI), `infra/litellm/` (LiteLLM config), `infra/microsandbox/`
(SDK notes), and `agents/` (optional agent checkouts).

## Troubleshooting

- **`/dev/kvm` issues.** Load the `kvm` and `kvm_intel` (or `kvm_amd`)
  kernel modules, add your user to the `kvm` group, log out and back in,
  and confirm virtualization is enabled in firmware. `just host-check`
  surfaces all of these.
- **Stale `~/.microsandbox`.** Safe to delete. The dev shell uses a
  persistent cache at `$HOME/.cache/ai-workbench-msb` for `cargo check`/
  `build.rs` and also cleans up legacy per-shell tmpfs dirs from earlier
  versions; `nix build .#workestrate` instead runs the Nix-store `msb`
  directly and the wrapper sets `MSB_HOME="$HOME/.microsandbox"`. The old
  `~/.microsandbox/bin/msb` path is no longer used at runtime.
- **Port 4000 already in use.** Another process is bound to the
  LiteLLM port. Stop it, or change the proxy port in the sandbox plan
  and update any agent configuration that points at `:4000`.
- **Dangling vendor symlink.** If
  `control/agentctl/vendor/microsandbox-filesystem-0.5.6` points
  nowhere (for example after a `nix store` GC), re-enter the dev
  shell (`exit` then `nix develop`) or run `just vendor-unlock` to
  materialise a real copy, then `just vendor-lock` to put the symlink
  back.
- **`[MISSING] (optional)` for `agents/pi/repo` / `agents/odysseus/repo` / `agents/opencode/repo` / `agents/tempest/repo`.** This
is expected on a fresh clone. The agent checkouts are gitignored;
clone the agent repos into `agents/<name>/repo` only if you intend to run them.
- **"missing secrets" failures.** `workestrate` loads secrets from
  `.env.enc` automatically before starting sandboxes; ensure
  `setup-secrets init` has been run and the age key is present at
  `~/.config/sops/age/ai-workbench-secrets.txt`.

## Important notes

- **M1 scope.** Sandbox plans, the `workestrate` CLI, and the LiteLLM
  proxy are implemented and cargo-verified in-container. Runtime
  `up`/`exec`/detached-mode is HOST-KVM gated (pending runtime validation;
  this container has no `/dev/kvm`), so end-to-end agent runs are
  plan-verified but not yet runtime-validated.
- **In-memory LiteLLM.** No Postgres, no virtual keys, no persistent
  state. Agents reuse `LITELLM_MASTER_KEY` for the lifetime of the
  proxy; rotating the master key requires a `workestrate workload down litellm` followed by
  `workestrate workload up litellm` with the new `.env.enc`.
- **No Docker.** Microsandbox talks to KVM directly, so the host does
  not need Docker, `containerd`, or any other container runtime.
- **Optional agent checkouts.** `agents/pi/repo`, `agents/odysseus/repo`,
  `agents/opencode/repo`, and `agents/tempest/repo` are gitignored. You
  only need the checkouts if you want to run the agents themselves. (Flake
  inputs for the agent sources exist; pi and tempest are consumed by their
  respective nix derivations, odysseus and opencode still use the dev-shell
  auto-build.)
- **Secrets discipline.** The user's personal config repo contains the
  only encrypted artifact (`.env.enc`) and `.sops.yaml`. Treat the age key
  file as the recovery seed for the entire workflow; see
  [docs/secrets.md](docs/secrets.md) for the full threat model. The old root
  `.env.enc` and `.sops.yaml` were stripped from the tool repo but remain in
  git history.

## Configuration model (single tool home)

workestrate supports a single tool home configuration model where the tool is
decoupled from any workspace. Configuration lives in a single tool home
directory (`$WORKESTRATE_HOME`) with a flat layout (ADR 0023):

| Layer | Path | Contents |
|---|---|---|
| **Home** | `$WORKESTRATE_HOME/` (default `~/.workestrate`) | Single tool home directory |
| **Registry** | `$WORKESTRATE_HOME/config.toml` | Tool settings, config-repo registry, ordered layers, trusted projects |
| **Overrides** | `$WORKESTRATE_HOME/overrides.toml` | User-global overrides (optional) |
| **Config repos** | `$WORKESTRATE_HOME/config-repos/<name>/` | `workestrate.toml`, `.env.enc`, `.sops.yaml`, `infra/litellm/`, `agents/*/config/` |
| **State** | `$WORKESTRATE_HOME/state/` | `workspaces/`, `var/` (runtime state) |
| **Sources** | `$WORKESTRATE_HOME/sources/<name>/` | Agent source checkouts + builds |
| **Cache** | `$WORKESTRATE_HOME/cache/` | Cache |

### Quick start (single home model)

```bash
# Initialize the registry (one-time)
workestrate init

# Add a personal config repo (local or remote)
workestrate config add <url> personal

# Trust the current project directory for project-layer config
workestrate config trust $(pwd)

# Verify
workestrate check
workestrate workload plan pi
```

### Config resolution order

1. `WORKESTRATE_HOME` env var (explicit override)
2. Legacy XDG (read-only compat + deprecation note — reads old `XDG_CONFIG_HOME/workestrate/` etc. if present)
3. Default: `~/.workestrate`

Then within the resolved home, config layers merge in this order (lowest → highest precedence):

1. `WORKESTRATE_CONFIG_DIR` env var (bypasses discovery; single layer, no merging)
2. Reference config (`config.reference/workestrate.toml`) — synthetic fallback shipped with the tool
3. Context layers (registry `[contexts.<name>] layers = [...]`, in declared order; each a config repo's `workestrate.toml`)
4. User-global overrides (`$WORKESTRATE_HOME/overrides.toml`: `[global]` applied to every context, then `[configs.<name>]` for each active context layer)
5. Trusted project config (`./workestrate.toml` in cwd, IF cwd is in `[trusted_projects]`)
6. Local overrides (`./workestrate.local.toml` in cwd, same trust gate as #5)

Fail-closed: with no config repos registered, the synthetic reference config is used
(placeholder secrets — `example-service plan` works; `up`/`exec` for real workload names
require a registered config repo).

### New commands

| Command | Description |
|---|---|
| `workestrate init [url]` | DEPRECATED — use `workestrate home init` (bare `init` warns; `init <url>` errors in favor of `workestrate home clone <src>`) |
| `workestrate home init` | Initialize the resolved tool home as a dotfiles-style git repo (git init + `.gitignore` + pre-commit hook; idempotent; `--config <url> [--name <n>]` also clones + registers a config repo) |
| `workestrate home clone <src> [dest]` | Provision a home from an existing one (git-clone semantics; registry urls rewritten to dest-local `config-repos/<name>` paths; dest defaults to the resolved home) |
| `workestrate config add <url> <name> [--ref main]` | Clone a config repo into the managed store |
| `workestrate config new <name> [dest] [--age-recipient <key>] [--with-flake] [--no-register] [--no-git-init] [--from-reference \| --empty] [--json]` | Scaffold a new config repo with a minimal valid `workestrate.toml`, `.sops.yaml`, `.env.example`, README, and `.gitignore`. The in-store default (`<store>/config-repos/<name>`) auto-registers in the registry and runs `git init`; an out-of-store `[dest]` is scaffold-only (not registered until `config add`). Writes `.copier-answers.yml` for future `copier update`. |
| `workestrate config update [name]` | Pull latest for a config repo (or all) |
| `workestrate config remove <name> [--delete] [--force]` | Unregister a config repo (`--delete` also deletes the store clone; `--force` overrides the dirty-clone refusal) |
| `workestrate config list` | List registered config repos with rev + dirty status |
| `workestrate config trust <dir>` | Trust a project directory for project-layer config |
| `workestrate config untrust <dir>` | Remove trust from a project directory |
| `workestrate source clone <name> [path]` | Clone agent source into the managed store |
| `workestrate source build <name>` | Build agent from source (prints recipe instructions) |
| `workestrate source list` | List agent source checkouts with status |
| `workestrate source reset <name>` | Reset agent source to canonical |
| `workestrate validate-config` | Validate active config against schema + policy allowlists |
| `workestrate secrets-schema` | Print secret env_var names from config |
| `workestrate generate-env-example` | Generate `.env.example` from config secrets section |
| `workestrate ps [--json] [--all-contexts]` | List running workestrate sandboxes (instance records) |
| `workestrate workloads` | List configured workloads with kind + running status (discovery verb, ADR 0027) |
| `workestrate down --all [--yes]` | Stop every running workestrate sandbox (destructive; confirms unless `--yes`) |
| `workestrate generate-schema` | Print the JSON Schema for `workestrate.toml` (schemars-derived; committed at `control/agentctl/schema/workestrate.toml.json`) |
| `workestrate --no-project-config <cmd>` | Disable project-layer config loading |

### Secrets targeting

`setup-secrets.sh` supports `--config <name>` to target a specific config
repo's `.env.enc` and `.sops.yaml`:

```bash
setup-secrets --config personal init
setup-secrets --config personal update
```

Without `--config`, it auto-detects a single registered config repo, or
falls back to the repo root (backwards compat).

## Tool home layout

workestrate stores its state (registry, config repos, secrets, runtime state)
in a single tool home directory (`$WORKESTRATE_HOME`, default `~/.workestrate`).
The home defaults to `~/.workestrate` everywhere (container and host alike);
`WORKESTRATE_HOME` remains as an explicit override. The home can be versioned
as a dotfiles-style git repo via `workestrate home init` (scaffolds `.gitignore`
+ a pre-commit hook that rejects gitlinks, store-dirs, and secret material).

### Layout

```
~/.workestrate/                        → WORKESTRATE_HOME (default ~/.workestrate)
├── config.toml                        (registry: config repos, layers, trusted projects)
├── overrides.toml                     (user-global overrides, optional)
├── secrets/                           (machine-local secrets)
├── config-repos/                      (config repo working copies)
│   └── personal/                      (workestrate.toml, .env.enc, .sops.yaml, ...)
├── sources/                           (agent source checkouts)
├── state/                             (runtime state)
│   ├── workspaces/                    (per-agent scratch)
│   └── var/                           (runtime logs, pidfiles)
└── cache/                             (cache)
```

### One-time migration from legacy XDG

If you previously had workestrate state in the legacy XDG three-home layout
(`~/.config/workestrate/`, `~/.local/share/workestrate/`,
`~/.local/state/workestrate/`), run:
```bash
workestrate migrate-home
```

This moves the registry, config repos, and runtime state into the single
home layout (`$WORKESTRATE_HOME`) and cleans up the old XDG dirs. The SOPS
age key is intentionally NOT moved — it stays at `~/.config/sops/age/` on the
host (see "Security warning" below).

### Security warning: age key location

The SOPS age private key is **NEVER** in the bundle or anywhere under this
repository (the repo is agent-reachable via `${CWD}` mounts, so a key inside
it would be exposed to sandboxes). It stays at
`~/.config/sops/age/ai-workbench-secrets.txt` on the host. `workestrate`
secret operations (`setup-secrets`, `with-secrets`, `run-with-secrets`,
`decrypt-env`, `write-env`) therefore run on the host; in the container
the key is simply absent and secret operations fail closed by design.

The home (`~/.workestrate`) contains the encrypted `.env.enc`, the registry,
and config repos. If you version the home as a dotfiles repo via
`workestrate home init`, the generated `.gitignore` ignores store dirs and
secret material — but **NEVER commit unencrypted secrets.**
