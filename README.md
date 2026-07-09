# workestrator

A local AI workbench that runs Pi, Odysseus, OpenCode, and T3MP3ST agents inside
Microsandbox microVMs, with LiteLLM as the unified LLM proxy. Everything
is driven from a single Rust CLI (`workestrate`) and orchestrated through
Nix flakes and SOPS-encrypted secrets.

## What is this

`workestrator` is a single-host sandbox for experimenting with
LLM-driven coding agents without giving them direct network or host
access. A small Rust CLI builds Microsandbox plans for one or more
agent microVMs and a local LiteLLM proxy; the proxy terminates
authentication, fans requests out to upstream providers, and exposes
an OpenAI-compatible endpoint at `http://host.microsandbox.internal:4000`.
Agent microVMs start with a default-deny network policy and only receive
the secrets needed to reach the proxy.

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

1. Clone the repository and enter the dev shell. The shell hook stages
   `msb` and `agentd` from the Nix store, uses a persistent cache at
   `$HOME/.cache/ai-workbench-msb` for `cargo check`/`build.rs`, cleans
   up legacy per-shell tmpfs dirs from earlier versions, and refreshes
   the `control/agentctl/vendor/microsandbox-filesystem-0.5.6` symlink.
   The dev shell pins `nodejs_24` (was `nodejs_22`; fixes pi's gondolin
   `EBADENGINE`) and exports `WORKESTRATE_PI_BUILD` pointing at the
   canonical `.#pi-bun` standalone binary, so dev-shell `workestrate pi
   exec` mounts the bun binary at `/app/bin/pi`.
   ```bash
   git clone <repo-url> workestrator
   cd workestrator
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
4. Initialise encrypted secrets (one-time, see [Secrets setup](#secrets-setup)):
   ```bash
   setup-secrets init
   ```
5. Start the LiteLLM proxy (starts detached; add `--foreground` to block):
   ```bash
   workestrate litellm up
   ```
6. Attach to an agent (for example Pi):
   ```bash
   workestrate pi exec
   ```

   Services start detached by default: `workestrate <svc> up` returns
   immediately and the sandbox keeps running in the background. Use
   `workestrate <svc> up --foreground` (or `-f`) to block until Ctrl-C.
   Tail a detached service's logs with `workestrate <svc> logs` (written to
   `~/.microsandbox/sandboxes/<svc>/workestrate.log`). Detached mode works
   through `workestrate` — the detached child inherits the parent's
   decrypted environment, so `workestrate litellm up` starts detached
   and works without `nohup`.

## Secrets setup

Secrets are stored in `.env.enc`, encrypted with SOPS using an
age key that lives outside the repo at
`$HOME/.config/sops/age/ai-workbench-secrets.txt`. The wrappers
`setup-secrets`, `decrypt-env`, and `write-env` (provided by the flake)
all default `SOPS_AGE_KEY_FILE` to that path. `workestrate` loads secrets
internally before starting sandboxes or running commands.

1. Generate the project age key and create `.env.enc` (one-time):
   ```bash
   setup-secrets init
   ```
   `.sops.yaml` currently contains the project's age recipient.
   `setup-secrets init` will create a new age key and update
   `.sops.yaml` for you. If `.sops.yaml` already has a different
   recipient, the script will ask you to update it manually. The
   command either uses the required env vars (if all are set) or
   opens `$EDITOR` (falling back to `nano`, `vi`, or `vim`) with a
   pre-filled buffer of required and optional keys from `.env.example`.
   `init` refuses to overwrite an existing `.env.enc`.

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
   setup-secrets update
   ```
   This decrypts `.env.enc`, opens the editor with current values
   pre-filled, and re-encrypts on save. To rotate the master key
   non-interactively, export `LITELLM_MASTER_KEY` and run
   `setup-secrets update`. To update other values non-interactively,
   pipe the plain values on stdin in the order of `REQUIRED_KEYS`
   (one value per line, no `KEY=` prefix).

For the full threat model and wrapper reference, see
[docs/secrets.md](docs/secrets.md).

## Running the LiteLLM proxy

Inside the dev shell:

```bash
workestrate litellm up      # start (detached by default)
workestrate litellm down    # stop
workestrate litellm logs    # tail the detached service's log
nix run . -- litellm plan        # show the sandbox plan without secrets
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

The proxy is configured by `infra/litellm/config.yaml` (which `include:`s `models.yaml`) and exposes
coding-tier model names. Each tier targets a different cost/capability
point:

| Tier | Primary | Fallback chain | Use case |
|---|---|---|---|
| `coding` | Kimi K2.7 (`anthropic/kimi-for-coding`) | neural-kimi-k2.7-code → coding.free | Default coding work |
| `coding.fast` | Qwen 3.6 35B fast (Neuralwatt) | neural-qwen3.6-35b → coding | Quick edits, autocomplete |
| `coding.pro` | GLM-5.2 short (Neuralwatt) | neural-glm-5.2 → neural-kimi-k2.7-code | Complex refactoring, reasoning |
| `coding.free` | Qwen 3 Coder free (OpenRouter) | neural-qwen3.6-35b-fast | Experimentation, no cost |
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
Provider API keys (OpenRouter, Kimi, Neuralwatt, MiniMax) are host-bound via
`secret_env()`; `LITELLM_MASTER_KEY` is passed as a plain environment
variable via `env()` (not host-bound) because LiteLLM reads it from
the process env at startup.

## Running agents

Workloads split into two kinds: **services** (litellm, odysseus) support
`up`/`down`/`logs`/`plan`; **agents** (pi, opencode, tempest) support
`exec`/`down`/`plan` (agents have no `up` or `logs` — you attach to them
interactively with `exec`).

```bash
# Services (start detached, tail with `logs`)
workestrate litellm up          # LiteLLM proxy
workestrate odysseus up         # Odysseus agent (service)
workestrate odysseus logs       # tail Odysseus's detached log

# Agents (interactive TUI attach)
workestrate pi exec            # attach to the Pi coding agent
workestrate opencode exec      # attach to the OpenCode coding agent
workestrate tempest exec       # attach to the T3MP3ST offensive-security agent

# Stop any workload
workestrate <name> down

# Show a sandbox plan without secrets
nix run . -- <name> plan            # e.g. pi plan, odysseus plan, litellm plan
```

### Secret loading

`workestrate` loads secrets from `.env.enc` automatically before commands that need them (`exec`, `up`, `run`). Commands that don't need secrets (`plan`, `check`, `new`, `completions`) skip decryption entirely — they work on a fresh clone before `setup-secrets init` has been run.

To run an arbitrary command with decrypted secrets:

```bash
workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'
workestrate run -- bash    # interactive shell with secrets
```

The `run` subcommand decrypts `.env.enc` via `sops`, loads all keys into the process environment, then execs the given command.

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
`<name> up` will work; `workestrate check` reports them as `[MISSING] (optional)`
and does not fail, but the corresponding `<name> up` command requires the
checkout to exist.

Agents reach the proxy at `http://host.microsandbox.internal:4000`.
Odysseus and OpenCode receive `OPENAI_API_KEY` (remapped from
`LITELLM_MASTER_KEY`) host-bound to `host.microsandbox.internal`. Pi
receives `LITELLM_MASTER_KEY` as a plain process env var (not
host-bound) because Pi's `models.json` performs
`${LITELLM_MASTER_KEY}` substitution at startup; host-bound secrets
are not visible in the guest env and would leave the substitution
empty. The authoritative security control for all agents is network
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
`GITHUB_TOKEN` to `github.com` and `api.github.com`. Odysseus binds
`OPENAI_API_KEY` (remapped from `LITELLM_MASTER_KEY`) to
`host.microsandbox.internal`; Pi receives `LITELLM_MASTER_KEY` as a
plain process env var instead (see above).

`agents/pi/repo`, `agents/odysseus/repo`, `agents/opencode/repo`, and
`agents/tempest/repo` are optional local overrides and are expected to be
absent on a fresh clone; `workestrate check` reports them as `[MISSING] (optional)`
and does not fail.

## Development workflow

Common `just` recipes:

| Recipe | What it does |
|---|---|
| `just check` | Run `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo check` for `control/agentctl` |
| `just litellm-check` | Validate `infra/litellm/config.yaml` against the schema indexes |
| `just verify` | Full pre-merge gate: `just check` plus `cargo test`, `just litellm-check`, and `Cargo.lock` stability check |
| `just verify-full` | Heaviest validation: `just verify` plus `nix build .#workestrate` |
| `just build` | Build the `workestrate` binary |
| `just fmt` | Format the Rust code |
| `just fmt-check` | Check formatting without modifying files |
| `just clippy` | Run Clippy with `-D warnings` |
| `just test` | Run unit tests for `control/agentctl` |
| `just workestrate …` | Run `cargo run --manifest-path control/agentctl/Cargo.toml -- …` (e.g. `just workestrate litellm plan`) |
| `just plan` | Run all workload plans (litellm, pi, odysseus, opencode, tempest) via `cargo run` |
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
- `.#workestrator` — `runCommand` + `makeWrapper` wrapper around
  `.#workestrate` that bakes `WORKESTRATE_PI_BUILD=${pi-bun}` into the
  environment, so `nix build .#workestrator && ./result/bin/workestrator pi exec`
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
`agents/<name>/build`. This is the mechanism the `.#workestrator` wrapper
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
  the sandbox. `.#pi` (npm/node) is the fallback. The `.#workestrator`
  wrapper bakes `WORKESTRATE_PI_BUILD` so `nix build .#workestrator` runs
  hermetic.
- Sandbox plans use a default-deny network policy; only the
  destinations listed above have explicit egress.
- `LITELLM_MASTER_KEY` is passed to Pi and to the LiteLLM proxy as a
  plain process env var (`EnvVar::secret`, not host-bound) so Pi's
  `${LITELLM_MASTER_KEY}` substitution in `models.json` resolves to the
  actual key. Provider secrets (OpenRouter, Kimi, Neuralwatt) remain
  host-bound in the LiteLLM proxy via `secret_env`.
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
(SDK notes), `agents/` (optional agent checkouts), `workspaces/`
(per-agent scratch), and `var/` (runtime logs and pidfiles).

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
  proxy are compile-checked and exercised against `nix build`. Running
  microVMs at runtime requires a host with `/dev/kvm`; this
  development container has none, so end-to-end agent runs have only
  been verified up to the planning stage.
- **In-memory LiteLLM.** No Postgres, no virtual keys, no persistent
  state. Agents reuse `LITELLM_MASTER_KEY` for the lifetime of the
  proxy; rotating the master key requires a `litellm down` followed by
  `litellm up` with the new `.env.enc`.
- **No Docker.** Microsandbox talks to KVM directly, so the host does
  not need Docker, `containerd`, or any other container runtime.
- **Optional agent checkouts.** `agents/pi/repo`, `agents/odysseus/repo`,
  `agents/opencode/repo`, and `agents/tempest/repo` are gitignored. You
  only need the checkouts if you want to run the agents themselves. (Flake
  inputs for the agent sources exist; pi and tempest are consumed by their
  respective nix derivations, odysseus and opencode still use the dev-shell
  auto-build.)
- **Secrets discipline.** `.env.enc` is the only encrypted artifact in
  the repo and is restricted by `.sops.yaml` to a single recipient.
  Treat the age key file as the recovery seed for the entire workflow;
  see [docs/secrets.md](docs/secrets.md) for the full threat model.
