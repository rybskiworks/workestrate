# ai-workbench Specification

## Purpose

ai-workbench is a local development environment for running AI coding agents
(Pi, Odysseus) inside isolated Microsandbox microVMs. LiteLLM provides a
unified proxy to multiple LLM providers. The control plane (`agentctl`) manages
sandbox definitions, plans, and health checks.

## Current milestone

Milestone 1 (M1) delivers:
- Compile-checked Rust control plane (`agentctl`)
- Nix flake for reproducible dev shell
- Microsandbox SDK integration (0.5.6, `net` feature)
- Sandbox plans for LiteLLM, Pi, and Odysseus
- Verified `cargo check`, `cargo clippy`, `cargo fmt`
- No runtime claims — this environment lacks KVM

## Runtime architecture

The host runs NixOS or Nix on Linux with KVM. `agentctl` builds sandbox
configurations using the Microsandbox Rust SDK. Each agent and LiteLLM runs in
its own microVM with explicit network policies, secret injection, and resource
limits. Agents communicate with LiteLLM via a well-known host IP/port.

## Components

### agentctl

Rust CLI built with Tokio and Clap. Commands: `check`, `litellm plan`,
`agent plan <name>`. The plan commands print sandbox configurations built
with `SandboxBuilder` and `NetworkPolicyBuilder`. No runtime execution yet.

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
`packages/coding-agent`. Expected to call LiteLLM via `OPENAI_BASE_URL`.
Full integration is future work.

### Odysseus

Coding agent from `github:georgrybski/odysseus`. Expected to call LiteLLM
via `OPENAI_BASE_URL`. Full integration is future work.

### Nix

Reproducible build environment. `nix develop` provides Rust toolchain,
`just`, and build dependencies. `nix build .#agentctl` compiles the CLI.

## Source model

- `agentctl` sources: `control/agentctl/src/`
- Flake inputs: `nixpkgs`, `pi` (GitHub fork), `odysseus` (GitHub fork)
- Agent repos are optional local overrides in `agents/`, ignored by git
- Profiles live in `profiles/agents/`

## Secrets model

LiteLLM holds a dummy provider key. The real key is injected at the network
boundary by Microsandbox's egress layer (secret_env + header rewrite). This is
documented from SDK source but not yet tested at runtime. Host secrets live in
`infra/litellm/.env` (mode 0600, gitignored).

## Egress model

Network policy uses `default_deny` with explicit allow rules for the real
provider host. All other outbound traffic is blocked. The egress layer also
performs secret injection (dummy key → real key) for allowed destinations.

## Filesystem model

- Source code: tracked in git
- Build artifacts: `control/agentctl/target/` (gitignored)
- Runtime state: `var/log/`, `var/run/` (gitignored)
- Secrets: `infra/litellm/.env` (gitignored, mode 0600)
- Agent overrides: `agents/*` (gitignored)

## Milestone 1 acceptance criteria

- [x] `nix develop` produces a working shell
- [x] `cargo check` passes with zero warnings
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt -- --check` passes
- [x] `just check` passes
- [x] `agentctl check` reports all required files present
- [x] `agentctl litellm plan` prints a valid sandbox plan
- [x] `agentctl agent plan pi` prints a valid sandbox plan
- [x] `agentctl agent plan odysseus` prints a valid sandbox plan
- [ ] Runtime sandbox execution (blocked: no KVM)

## Future milestones

- **M2**: Runtime Microsandbox execution on KVM host; secret injection and
egress policy tested end-to-end.
- **M3**: Pi and Odysseus honor `OPENAI_BASE_URL`; agents run in microVMs
and communicate with LiteLLM proxy.
- **M4**: Postgres-backed LiteLLM with virtual keys, per-agent spend tracking,
and team isolation.
- **M5**: Production deployment profiles, CI/CD, automated updates.

## Known gaps

See `docs/gaps.md` for the full list. Key items:
- Runtime testing blocked by missing KVM
- Agent integration with LiteLLM (`OPENAI_BASE_URL`) is theoretical
- Secret injection and egress enforcement are compile-checked but untested
- `nix build .#agentctl` is best-effort in M1
