# ai-workbench Specification

## Purpose

ai-workbench is a local development environment for running AI coding agents
(Pi, Odysseus) inside isolated Microsandbox microVMs. LiteLLM provides a
unified proxy to multiple LLM providers. The control plane (`workestrate`) manages
sandbox definitions, plans, and health checks.

## Current milestone

Milestone 1 (M1) delivers:
- Compile-checked Rust control plane (`workestrate`)
- Nix flake for reproducible dev shell
- Microsandbox SDK integration (0.5.6, `net` feature)
- Sandbox plans for LiteLLM, Pi, and Odysseus
- Verified `cargo check`, `cargo clippy`, `cargo fmt`
- No runtime-validated claims — this environment lacks KVM; `up`/`down` are
  implemented and compile-checked only in M1.

## Runtime architecture

The host runs NixOS or Nix on Linux with KVM. `workestrate` builds sandbox
configurations using the Microsandbox Rust SDK. Each agent and LiteLLM runs in
its own microVM with explicit network policies, secret injection, and resource
limits. Agents communicate with LiteLLM via a well-known host IP/port.

## Components

### workestrate

Rust CLI built with Tokio and Clap. Commands: `check`, plus per-workload subcommands. **Services** (litellm,
odysseus) expose `{up,down,logs,plan}`; **agents** (pi, opencode) expose
`{exec,down,plan}`. The `plan` subcommands print sandbox configurations
built with `SandboxBuilder` and `NetworkPolicyBuilder`. The `up`/`down`
subcommands drive the Microsandbox runtime (compile-checked in M1;
runtime-validated on a KVM host in M2). Service `up` starts detached by
default (`--foreground` to block); `logs` tails the detached service's log.

### Microsandbox

MicroVM runtime. SDK version 0.5.6. Provides `Sandbox`, `SandboxBuilder`,
`NetworkPolicy`, and builder methods for images, resources, ports, env vars,
volumes, and network rules. Async-only, requires Tokio.

### LiteLLM

LLM proxy/gateway. Runs as a sandboxed workload. Holds only dummy provider
credentials; real keys are injected at the egress boundary. M1 uses
in-memory storage; Postgres for virtual keys/spend tracking is deferred.

### Pi

Coding agent from `github:georgrybski/pi`. Target package:
`packages/coding-agent`. Expected to call LiteLLM through an OpenAI-compatible
endpoint. Pi does not honor `OPENAI_BASE_URL`; it requires seeding
`~/.pi/agent/models.json` with a custom provider pointing at LiteLLM. Full
integration is future work.

### Odysseus

Coding agent from `github:georgrybski/odysseus`. Expected to call LiteLLM
through an OpenAI-compatible endpoint. Odysseus does not honor
`OPENAI_BASE_URL`; it requires seeding `data/settings.json` with a custom
provider pointing at LiteLLM. Full integration is future work.

### Nix

Reproducible build environment. `nix develop` provides Rust toolchain,
`just`, and build dependencies. `nix build .#workestrate` compiles the CLI and wraps it with `MSB_PATH` pointing at
`.#microsandbox` so runtime commands find the Nix-managed daemon.

## Source model

- `workestrate` sources: `control/agentctl/src/`
- Flake inputs: `nixpkgs`, `pi` (GitHub fork), `odysseus` (GitHub fork)
- Agent repos are optional local overrides in `agents/`, ignored by git
- Profiles live in `profiles/agents/`

## Secrets model

LiteLLM holds the real provider keys inside its own microVM. Agents authenticate
to LiteLLM using `LITELLM_MASTER_KEY` in milestone 1 (virtual keys are deferred
to M4). The dummy-key egress rewrite is not implemented in M1.

## Egress model

Network policy uses `default_deny` with explicit allow rules for the real
provider host. All other outbound traffic is blocked. The egress layer also
performs secret injection (dummy key → real key) for allowed destinations.

## Filesystem model

- Source code: tracked in git
- Build artifacts: `control/agentctl/target/` (gitignored)
- Runtime state: `var/log/`, `var/run/` (gitignored)
- Secrets: `.env.enc` (committed, SOPS-encrypted); decrypted at runtime via `run-with-secrets`
- Agent overrides: `agents/*` (gitignored)

## Milestone 1 acceptance criteria

- [x] `nix develop` produces a working shell
- [x] `cargo check` passes with zero warnings
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt -- --check` passes
- [x] `just check` passes
- [x] `workestrate check` reports all required files present
- [x] `workestrate litellm plan` prints a valid sandbox plan
- [x] `workestrate pi plan` prints a valid sandbox plan
- [x] `workestrate odysseus plan` prints a valid sandbox plan
- [ ] Runtime sandbox execution (blocked: no KVM)

## Future milestones

- **M2**: Runtime Microsandbox execution on a KVM host; secret injection and egress
  policy tested end-to-end. The Microsandbox runtime daemon (`msb`) is now packaged
  in Nix as `.#microsandbox`; runtime execution still requires a host with KVM.
  The `up`/`down` subcommands are implemented in M1 and are compile-checked; M2
  validates them on a host with `/dev/kvm`.
- **M3**: Pi and Odysseus route through LiteLLM using their native configuration
mechanisms (Pi's `models.json` provider config, Odysseus's `data/settings.json`
/ `LLM_HOST`); agents run in microVMs and communicate with the LiteLLM proxy.
- **M4**: Postgres-backed LiteLLM with virtual keys, per-agent spend tracking,
and team isolation.
- **M5**: Production deployment profiles, CI/CD, automated updates.

## Known gaps

See `docs/gaps.md` for the full list. Key items:
- Runtime testing blocked by missing KVM
- Agent integration with LiteLLM requires native config seeding, not `OPENAI_BASE_URL`
- Secret injection and egress enforcement are compile-checked but untested
- `nix build .#workestrate` is available in M1 and is wired to the `.#microsandbox`
  runtime via `MSB_PATH`; runtime execution still requires a host with KVM.
