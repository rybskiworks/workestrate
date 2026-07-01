# Architecture Decisions

## ADR 0001: Single-root ai-workbench

Decision: Use one repo root for the entire workbench (control plane,
infrastructure, profiles, and docs).

Rationale: A monorepo simplifies Nix flake references, `agentctl` path
resolution, and atomic updates across components. Agent repos (Pi, Odysseus)
are flake inputs, not submodules.

## ADR 0002: Microsandbox SDK through Rust

Decision: `agentctl` uses the Microsandbox Rust SDK as the implementation path.

Rationale: The Rust SDK provides typed builders (`SandboxBuilder`,
`NetworkPolicyBuilder`) that are compile-checked. This is safer than
shelling out to a CLI or writing YAML by hand. The SDK is async and integrates
naturally with Tokio.

## ADR 0003: LiteLLM without Postgres in milestone 1

Decision: Do not add Postgres until virtual keys/spend tracking are implemented.

Rationale: Postgres adds operational complexity (schema migrations, backup,
Docker/volume management). M1 focuses on compile-checked SDK integration and
sandbox plans. In-memory LiteLLM is sufficient for planning.

## ADR 0004: Agents consume LiteLLM only

Decision: Pi/Odysseus call LiteLLM, not provider APIs directly.

Rationale: Centralizing all LLM traffic through LiteLLM enables unified
observability, rate limiting, and virtual key management. Agents should not
hold provider keys.

## ADR 0005: Agent repos are forked external repos

Decision: Flake inputs pin my forks; `agents/<name>/repo` directories are
optional ignored local overrides.

Rationale: Forks allow applying workbench-specific patches without waiting for
upstream. The `agents/` directory is gitignored so local clones do not conflict
with flake inputs.

## ADR 0006: Pi is a full monorepo

Decision: Use the whole Pi repo; `packages/coding-agent` is only the package
of interest.

Rationale: The Pi repo is a monorepo with multiple packages. We import the
entire repo via flake input and reference the specific package we need. This
preserves upstream structure and simplifies updates.
