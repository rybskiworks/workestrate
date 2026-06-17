# ai-workbench

A local AI workbench that runs Pi and Odysseus coding agents inside
Microsandbox microVMs, with LiteLLM as the unified LLM proxy. Everything
is driven from a single Rust CLI (`agentctl`) and orchestrated through
Nix flakes and SOPS-encrypted secrets.

## What is this

`ai-workbench` is a single-host sandbox for experimenting with
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
   `msb` and `agentd` from the Nix store, exports `MSB_HOME` and
   `MSB_PATH` to a per-pid scratch directory, and refreshes the
   `control/agentctl/vendor/microsandbox-filesystem-0.5.6` symlink.
   ```bash
   git clone <repo-url> ai-workbench
   cd ai-workbench
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
5. Start the LiteLLM proxy:
   ```bash
   run-with-secrets litellm up
   ```
6. Start an agent (for example Pi):
   ```bash
   run-with-secrets agent up pi
   ```

## Secrets setup

Secrets are stored in `.env.enc`, encrypted with SOPS using an
age key that lives outside the repo at
`$HOME/.config/sops/age/ai-workbench-secrets.txt`. The wrappers
`setup-secrets`, `with-secrets`, `run-with-secrets`, `decrypt-env`, and
`write-env` (provided by the flake) all default `SOPS_AGE_KEY_FILE` to
that path.

1. Generate the project age key and create `.env.enc` (one-time):
   ```bash
   setup-secrets init
   ```
   This creates the age key with mode `0600` if missing, replaces the
   `age1PLACEHOLDER…` recipient in `.sops.yaml` with the real public
   key, and either uses the required env vars (if all five are set) or
   opens `$EDITOR` (falling back to `nano`, `vi`, or `vim`) with a
   pre-filled buffer of required and optional keys from `.env.example`.
   `init` refuses to overwrite an existing `.env.enc`.

   | Key | Used for |
   |---|---|
   | `LITELLM_MASTER_KEY` | Local LiteLLM proxy authentication (any `sk-…` string; `sk-change-me-local-only` is rejected) |
   | `OPENROUTER_API_KEY` | OpenRouter provider |
   | `KIMI_CODE_API_KEY` | Kimi for Coding provider |
   | `MINIMAX_CODING_API_KEY` | MiniMax Coding provider |
   | `INCEPTION_API_KEY` | Inception Labs provider |

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
run-with-secrets litellm up      # start
run-with-secrets litellm down    # stop
nix run . -- litellm plan        # show the sandbox plan without secrets
```

Once the proxy is up, you can talk to it directly on the host:

```bash
# List the four configured models
nix develop -c with-secrets -- curl -sS http://127.0.0.1:4000/v1/models -H "Authorization: Bearer $LITELLM_MASTER_KEY"

# Smoke-test a chat completion
nix develop -c with-secrets -- curl -sS http://127.0.0.1:4000/v1/chat/completions \
  -H "Authorization: Bearer $LITELLM_MASTER_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"minimax-coding","messages":[{"role":"user","content":"ping"}]}'
```

These curls run on the host and reach the proxy at `127.0.0.1:4000`. From
inside the agent sandboxes, the same proxy is reached at
`http://host.microsandbox.internal:4000`.

The proxy is configured by `infra/litellm/config.yaml` and exposes:

| Model name | Upstream | Provider routing | Why |
|---|---|---|---|
| `openrouter-gpt-4o` | `openrouter/openai/gpt-4o` | OpenRouter | Generic OpenAI-compatible routing |
| `kimi-for-coding` | `anthropic/kimi-for-coding` at `https://api.kimi.com/coding` | Anthropic | Endpoint speaks the Anthropic Messages API |
| `minimax-coding` | `anthropic/MiniMax-M3` at `https://api.minimax.io/anthropic` | Anthropic | Endpoint speaks the Anthropic Messages API |
| `inception-mercury-2` | `inception/mercury-2` at `https://api.inceptionlabs.ai/v1` | OpenAI | Inception's native OpenAI-compatible API |

Kimi and MiniMax are routed through the `anthropic/` provider because
their endpoints speak the Anthropic Messages API, not OpenAI's.
`general_settings.master_key` reads `os.environ/LITELLM_MASTER_KEY`,
`general_settings.completion_model` is `minimax-coding`, and
`litellm_settings.drop_params` is `true`.

Egress is locked down to DNS (`tcp/53` and `udp/53`) to the host
and `tcp/443` to the four upstream hosts above. Each upstream API key
is bound to its destination via `allowed_host`; `LITELLM_MASTER_KEY`
is exposed inside the proxy as a regular environment variable (passed
via `env()`, not `secret_env()`); it remains a secret and is bound to
the proxy host only.

## Running agents

```bash
run-with-secrets agent up pi          # start the Pi coding agent
run-with-secrets agent up odysseus    # start the Odysseus coding agent
run-with-secrets agent down pi        # stop
nix run . -- agent plan pi            # show sandbox plan without secrets (or: agent plan odysseus)
```

Note: `agents/pi` and `agents/odysseus` must be cloned into the
`agents/` directory before `agent up` will work; `agentctl check`
reports them as `[MISSING] (optional)` and does not fail, but the
corresponding `agent up` command requires the checkout to exist.

Agents reach the proxy at `http://host.microsandbox.internal:4000`.
The agent's `OPENAI_API_KEY` is bound to `LITELLM_MASTER_KEY` for that
host only, so an agent that exfiltrates the secret cannot reuse it
against a different destination.

The Pi sandbox plan sets `PI_OFFLINE=1` and `PI_TELEMETRY=0`, denies
the `domain suffix .pi.dev`, mounts `agents/pi` and `workspaces/pi`,
and allows only `tcp/4000` egress to the host. The Odysseus sandbox
plan runs `python:3.12-slim` with
`uvicorn app:app --host 0.0.0.0 --port 7000`, publishes `7000:7000`,
mounts `agents/odysseus` and `workspaces/odysseus-data`, and likewise
allows only `tcp/4000` egress to the host. Both plans bind
`OPENAI_API_KEY` to `LITELLM_MASTER_KEY` for
`host.microsandbox.internal`.

`agents/pi` and `agents/odysseus` are optional local overrides and are
expected to be absent on a fresh clone; `agentctl check` reports them
as `[MISSING] (optional)` and does not fail.

## Development workflow

Common `just` recipes:

| Recipe | What it does |
|---|---|
| `just check` | Run `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo check` for `control/agentctl` |
| `just build` | Build the `agentctl` binary |
| `just fmt` | Format the Rust workspace |
| `just agentctl …` | Run `cargo run --manifest-path control/agentctl/Cargo.toml -- …` (e.g. `just agentctl litellm plan`) |
| `just plan` | Run `litellm plan`, `agent plan pi`, and `agent plan odysseus` via `cargo run` |
| `just host-check` | Verify KVM, Nix, memory, and disk prerequisites |
| `just validate-secrets` | Exercise the SOPS/age workflow against ephemeral test values |
| `just setup-secrets init` | Run `setup-secrets init` from the dev shell |
| `just vendor-unlock` | Replace the vendor symlink with a writable copy of the patched Microsandbox crate |
| `just vendor-lock` | Remove the vendor copy so the dev shell recreates the symlink |

`control/agentctl/vendor/microsandbox-filesystem-0.5.6` is a Nix-managed
symlink to `${microsandbox-filesystem-patched}` from the flake. The dev
shell hook refreshes it on every entry. To inspect or temporarily
modify the patched source, run `just vendor-unlock` (this expands the
symlink into a real directory you can edit, with `chmod -R u+w`).
Run `just vendor-lock` to delete the directory; the next `nix develop`
recreates the symlink from the flake input.

## Architecture

```
                        ┌──────────────────────────────────────────────┐
                        │            Host (Nix + /dev/kvm)             │
                        │                                              │
  user ─── agentctl ──▶ │  ┌────────────┐    ┌───────────────────────┐  │
                        │  │  msb       │    │  litellm microVM      │  │
                        │  │ (Nix store)│    │  :4000  (in-memory)   │  │
                        │  └─────┬──────┘    └──────────┬────────────┘  │
                        │        │  drives              │               │
                        │   ┌────┴──────────┐  ┌────────┴──────────┐    │
                        │   │ pi microVM    │  │ odysseus microVM  │    │
                        │   │               │  │ :7000             │    │
                        │   └──┬────────────┘  └────────┬──────────┘    │
                        │      │      host.microsandbox.internal:4000   │
                        │      │                       │                │
                        └──────┼───────────────────────┼────────────────┘
                               │                       │
                               ▼                       ▼
                  Upstream LLM providers (OpenRouter, Kimi, MiniMax, Inception)
```

- The Microsandbox SDK is pinned to `microsandbox = "=0.5.6"` with the
  `net` feature.
- Sandbox plans use a default-deny network policy; only the
  destinations listed above have explicit egress.
- Secrets are bound to a specific egress destination via
  `allowed_host`; the same value cannot be reused against another host.
- LiteLLM runs in-memory; no Postgres, no virtual keys, no persistent
  spend tracking in M1.
- Agents only ever see `OPENAI_API_KEY`, bound to the proxy's
  `host.microsandbox.internal` and equal to `LITELLM_MASTER_KEY`.

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
- **Stale `~/.microsandbox`.** Safe to delete. The dev shell stages
  `msb` into a per-pid scratch directory under
  `${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/ai-workbench-msb-…`; `nix build
  .#agentctl` instead runs the Nix-store `msb` directly and the wrapper
  sets `MSB_HOME="$HOME/.microsandbox"`. The old `~/.microsandbox/bin/msb`
  path is no longer used at runtime.
- **Port 4000 already in use.** Another process is bound to the
  LiteLLM port. Stop it, or change the proxy port in the sandbox plan
  and update any agent configuration that points at `:4000`.
- **Dangling vendor symlink.** If
  `control/agentctl/vendor/microsandbox-filesystem-0.5.6` points
  nowhere (for example after a `nix store` GC), re-enter the dev
  shell (`exit` then `nix develop`) or run `just vendor-unlock` to
  materialise a real copy, then `just vendor-lock` to put the symlink
  back.
- **`[MISSING] (optional)` for `agents/pi` / `agents/odysseus`.** This
  is expected on a fresh clone. The agent checkouts are gitignored;
  clone the agent repos there yourself only if you intend to run them.
- **"missing secrets" failures.** `agentctl` reports which wrapper to
  use; the fix is almost always to prefix the command with
  `run-with-secrets` so the SOPS-encrypted `.env.enc` is decrypted into
  the process environment.

## Important notes

- **M1 scope.** Sandbox plans, the `agentctl` CLI, and the LiteLLM
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
- **Optional agent checkouts.** `agents/pi` and `agents/odysseus` are
  gitignored. You only need the checkouts if you want to run the agents
  themselves. (Flake inputs for the agent sources exist but are not yet
  consumed by the build.)
- **Secrets discipline.** `.env.enc` is the only encrypted artifact in
  the repo and is restricted by `.sops.yaml` to a single recipient.
  Treat the age key file as the recovery seed for the entire workflow;
  see [docs/secrets.md](docs/secrets.md) for the full threat model.
