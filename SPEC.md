# ai-workbench Specification

## Purpose

ai-workbench is a local development environment for running AI agents
(Pi, Odysseus, OpenCode, T3MP3ST) inside isolated Microsandbox microVMs.
LiteLLM provides a unified proxy to multiple LLM providers. The control plane
(`workestrate`) manages sandbox definitions, plans, and health checks.

## Current milestone

Milestone 1 (M1) delivers:
- Compile-checked Rust control plane (`workestrate`)
- Nix flake for reproducible dev shell
- Microsandbox SDK integration (0.5.6, `net` feature)
- Sandbox plans for LiteLLM, Pi, Odysseus, OpenCode, and T3MP3ST
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
odysseus) expose `{up,down,logs,plan}`; **agents** (pi, opencode, tempest) expose
`{exec,down,plan}`. The `plan` subcommands print sandbox configurations
built with `SandboxBuilder` and `NetworkPolicyBuilder`. The `up`/`down`
subcommands drive the Microsandbox runtime (compile-checked in M1;
runtime-validated on a KVM host in M2). Service `up` starts detached by
default (`--foreground` to block); `logs` tails the detached service's log.

The `.#workestrator` wrapper (`runCommand` + `makeWrapper`) bakes
`WORKESTRATE_PI_BUILD` (pointing at the `.#pi-bun` standalone binary)
into the environment, so `nix build .#workestrator && ./result/bin/workestrator …`
runs the hermetic bun-binary pi sandbox without `nix develop`.
`apps.default` points at this wrapped binary.

### Microsandbox

MicroVM runtime. SDK version 0.5.6. Provides `Sandbox`, `SandboxBuilder`,
`NetworkPolicy`, and builder methods for images, resources, ports, env vars,
volumes, and network rules. Async-only, requires Tokio.

### LiteLLM

LLM proxy/gateway. Runs as a sandboxed workload. Holds only dummy provider
credentials; real keys are injected at the egress boundary. M1 uses
in-memory storage; Postgres for virtual keys/spend tracking is deferred.

### Pi

Coding agent from `github:georgrybski/pi` (remote fork; single canonical
source). Built two ways from one source, one `npmDepsHash`:

- `.#pi-bun` (canonical) — `bun build --compile` produces a self-contained
  standalone `pi` binary (~110 MB, Bun runtime embedded) at `$out/bin/pi`,
  with runtime assets mirrored alongside. The pi sandbox execs `/app/bin/pi`;
  no node/bun is needed inside the microVM at runtime.
- `.#pi` (fallback) — hermetic `buildNpmPackage` of the monorepo runtime
  tree, exec'ing `node` against the `packages/coding-agent` CLI (the
  npm/node artifact, as opposed to the bun standalone binary).

**Fork-carries-compat policy:** nix-build compatibility (patches, lockfile,
committed catalogs) lives on the agent fork, not as nix-side patches in
this repo. The flake consumes the fork verbatim. Local pi hacking uses
`just dev-build-pi` (hashless native npm into `agents/pi/build`), not a
nix override.

Target package: `packages/coding-agent`. Expected to call LiteLLM through
an OpenAI-compatible endpoint. Pi does not honor `OPENAI_BASE_URL`; it
requires seeding `~/.pi/agent/models.json` with a custom provider pointing
at LiteLLM. The bun-binary sandbox path is compile- and plan-verified but
pending KVM runtime validation; `.#pi` (node) is the fallback.

### Odysseus

Coding agent from `github:georgrybski/odysseus`. Expected to call LiteLLM
through an OpenAI-compatible endpoint. Odysseus does not honor
`OPENAI_BASE_URL`; it requires seeding `data/settings.json` with a custom
provider pointing at LiteLLM. Full integration is future work.

### T3MP3ST (tempest)

Offensive-security multi-agent framework from `github:georgrybski/T3MP3ST`
(remote fork of `elder-plinius/T3MP3ST`). Built via `buildNpmPackage` (single
TypeScript package; `tsc` emits `dist/`).

- `.#tempest-built` — hermetic nix build of the T3MP3ST tree.
- `.#tempest-image` — `dockerTools.buildLayeredImage` with nodejs_24 + nmap +
  bind.dnsutils + the compiled tree + a baked `defaultProvider:"local"` config.

T3MP3ST uses the `local` LLM provider, which reads all config from env vars
(`TEMPEST_LOCAL_BASE_URL`, `TEMPEST_LOCAL_MODEL`, `TEMPEST_LOCAL_API_KEY`).
`TEMPEST_LOCAL_API_KEY` is remapped from `LITELLM_MASTER_KEY`. The
`defaultProvider:"local"` config is baked into the image so no secrets are
stored in T3MP3ST's conf store. T3MP3ST execs `node dist/cli.js` (interactive
CLI TUI, like Pi). Unlike other agents, tempest uses `default_deny: false`
(broad egress) because it scans arbitrary targets. Compile- and plan-verified
in M1; pending KVM runtime validation.

### Nix

Reproducible build environment. `nix develop` provides Rust toolchain,
`just`, and build dependencies. `nix build .#workestrate` compiles the CLI and wraps it with `MSB_PATH` pointing at
`.#microsandbox` so runtime commands find the Nix-managed daemon.
`.#workestrator` wraps `.#workestrate` with `WORKESTRATE_PI_BUILD` baked
in (pointing at `.#pi-bun`); `apps.default` is the wrapped binary. `.#pi`
(npm/node) and `.#pi-bun` (standalone bun binary) are the two pi
derivations from the remote fork. The dev shell pins `nodejs_24` (was
`nodejs_22`).

## Source model

- `workestrate` sources: `control/agentctl/src/`
- Flake inputs: `nixpkgs`, `pi` (GitHub fork), `odysseus` (GitHub fork),
  `opencode` (GitHub fork), `tempest` (GitHub fork)
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

T3MP3ST (tempest) is an exception: it uses `default_deny: false` because it
is an offensive-security tool that needs to reach arbitrary targets for
scanning. The microVM boundary itself is the containment layer for tempest.

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
- [x] `workestrate opencode plan` prints a valid sandbox plan
- [x] `workestrate tempest plan` prints a valid sandbox plan
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
