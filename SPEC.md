# ai-workbench Specification

## Purpose

ai-workbench is a **tool**, not a workspace-bound project. The `workestrate`
Rust CLI drives a config-driven workload system: workloads (agents and
services) are defined by config repos, not hardcoded in the tool. A single
tool home (`$WORKESTRATE_HOME`) holds the registry, config-repo clones,
ordered layers/contexts, user-global overrides, secrets, sources, and
runtime state. A layering/merge engine composes config from multiple
sources with security-aware semantics. The instance lifecycle model
(singleton slots, parallel instances, refuse-on-occupied) governs
`up`/`exec`. Source overrides let users point at local or nix-built agent
binaries. Workloads run inside Microsandbox microVMs with default-deny
egress. LiteLLM provides a unified, in-memory LLM proxy.

## Components

### workestrate

Rust CLI built with Tokio and Clap. Sources live in `control/agentctl/src/`.
Commands span workload lifecycle (`up`/`down`/`logs`/`exec`/`plan`), config
management (`config add/new/update/list/trust`), source management
(`source clone/build/list/reset`), bootstrap (`init`, `migrate-home`),
introspection (`check`, `ps`, `validate-config`, `secrets-schema`,
`generate-env-example`, `generate-schema`, `completions`), and the
secret-bearing escape hatch (`run`). `plan` prints sandbox configurations
built with `SandboxBuilder` and `NetworkPolicyBuilder`; `plan --show-source`
annotates per-field provenance with layer names. `up`/`exec` target a slot
(singleton `<workload>` or `<context>-<workload>`) or a parallel instance
(`<slot>@<id>`); occupied slots refuse by default (`--replace` to recycle,
`--instance <id> --port-offset N` or `--new --port-offset N` for parallel
canaries). `--port-offset N` shifts host ports by `+=N` (guest unchanged).

`apps.default` runs the workestrate CLI directly. Workload image builds
live in the config repo flake (cleanup phase 3), which consumes the tool
flake's `lib` recipes (`buildImagesFromConfig`, `bun-compile`,
`npm-build`, `image.nix-layered`); the `WORKESTRATE_<NAME>_BUILD` env
override (`Workload::build_path()`) can point the CLI at any built tree.

### Microsandbox

MicroVM runtime. SDK version 0.5.6 with the `net` feature. Provides
`Sandbox`, `SandboxBuilder`, `NetworkPolicy`, and builder methods for
images, resources, ports, env vars, volumes, and network rules. Async-only,
requires Tokio. Runtime execution requires a host with `/dev/kvm`.

### LiteLLM

LLM proxy/gateway. Runs as a sandboxed workload. **In-memory storage** in
the current milestone: no Postgres, no virtual keys, no persistent spend
tracking. Agents authenticate to LiteLLM using `LITELLM_MASTER_KEY`;
virtual keys and per-agent spend tracking are deferred to a future
milestone (M4). Provider keys are injected into the LiteLLM microVM via
host-bound `secret_env`.

### Agents (pi / odysseus / opencode / tempest)

Agents are **defined by the config repo**, not by the tool. A fresh clone
ships only synthetic reference workloads (`example-service`,
`example-agent`, `example-offensive`); real agent workloads (`pi`,
`odysseus`, `opencode`, `tempest`) appear once a personal config repo is
registered.

- **Pi** — coding agent. Built two ways from one source (the remote fork)
  by the config repo flake: a bun-compile standalone binary (canonical,
  ~110 MB, Bun runtime embedded, exec'd at `/app/bin/pi`) and an npm/node
  fallback. Pi does not honor
  `OPENAI_BASE_URL`; it requires seeding `~/.pi/agent/models.json` with a
  custom provider pointing at LiteLLM. The bun-binary path is compile- and
  plan-verified but pending KVM runtime validation; the npm/node variant
  is the fallback.
- **Odysseus** — coding agent (service). Requires `ODYSSEUS_ADMIN_PASSWORD`
  because `AUTH_ENABLED=true`. Does not honor `OPENAI_BASE_URL`; requires
  seeding `data/settings.json` with a custom provider pointing at LiteLLM.
- **OpenCode** — coding agent. Receives `OPENAI_API_KEY` (remapped from
  `LITELLM_MASTER_KEY`) host-bound to `host.microsandbox.internal`.
- **T3MP3ST (tempest)** — offensive-security multi-agent framework. Uses
  `default_deny: false` (broad egress) because it scans arbitrary targets;
  the microVM boundary is the containment layer. Connects to LiteLLM via
  the `local` provider (`TEMPEST_LOCAL_*` env vars; `TEMPEST_LOCAL_API_KEY`
  remapped from `LITELLM_MASTER_KEY`).

See `README.md` for the full depth on each agent.

### Nix

Reproducible build environment. `nix develop` provides the Rust toolchain,
`just`, and build dependencies. `nix build .#workestrate` compiles the CLI
and wraps it with `MSB_PATH` pointing at `.#microsandbox`. The flake exposes
`lib.*` exports (recipes, vocabulary, `buildWorkloadImage`,
`buildImagesFromConfig`, `checks.validateConfig`) for config-repo-flake
consumption. The dev shell pins `nodejs_24`.

## Source model

- `workestrate` sources: `control/agentctl/src/`
- Flake inputs: `nixpkgs`, `pi`, `odysseus`, `opencode`, `tempest` (GitHub
  forks)
- Agent repos are optional local overrides in `agents/` (gitignored) or in
  the managed sources store (`$WORKESTRATE_HOME/sources/<name>/`)
- Config repos live in the managed store (`$WORKESTRATE_HOME/config-repos/<name>/`)
- Profiles live in `profiles/` (tracked, reference)

## Secrets model

Secrets are SOPS-encrypted (age recipient) per config repo. Each config
repo holds its own `.env.enc` + `.sops.yaml` in
`$WORKESTRATE_HOME/config-repos/<name>/`. A user-global secrets layer
(`$WORKESTRATE_HOME/secrets/.env.local.enc`) applies per-key across all
contexts.

**Multi-layer per-key merge** (later layer wins per key; process env is
lowest precedence):

1. Process env (only for defined secrets; lowest precedence)
2. `WORKESTRATE_CONFIG_DIR` (single override, bypasses discovery)
3. Reference config dir (shipped with tool — no `.env.enc` expected)
4. Context layers in declared order, each with its own `.env.enc`
   (per-repo `secrets_file` and `age_key_file` overrides honored from the
   registry `[configs.<name>]`)
5. User-global secrets (`$WORKESTRATE_HOME/secrets/.env.local.enc`) —
   applied per-key AFTER the context's domain layers, BEFORE project layers
6. Trusted project dir (cwd, if trusted) — may have `.env.enc`
7. Local overrides dir

**The 7 secrets:**

| Secret | Used for |
|---|---|
| `LITELLM_MASTER_KEY` | Local LiteLLM proxy auth (any `sk-…`; `sk-change-me-local-only` rejected) |
| `OPENROUTER_API_KEY` | OpenRouter provider |
| `KIMI_CODE_API_KEY` | Kimi for Coding provider |
| `NEURALWATT_API_KEY` | Neuralwatt provider |
| `MINIMAX_CODING_API_KEY` | MiniMax Coding provider |
| `GITHUB_TOKEN` | GitHub PAT for agent sandboxes (git ops + API) |
| `ODYSSEUS_ADMIN_PASSWORD` | Odysseus admin login (required because `AUTH_ENABLED=true`) |

(Plus optional reserved-but-unconsumed: `AI_WORKBENCH_WORKSPACES_DIR`,
`AI_WORKBENCH_VAR_DIR`.)

**age key location:** `~/.config/sops/age/ai-workbench-secrets.txt` on the
HOST. NEVER in the repo, NEVER in the bundle, NEVER under `.workestrate/`
or `$WORKESTRATE_HOME`. The repo is agent-reachable via `${CWD}` mounts, so
a key inside it would be exposed to sandboxes. In a container the key is
simply absent and secret operations fail closed by design.

**Failure semantics:** undecryptable layer → WARNING + continue; required
secret unsatisfied → hard fail naming the secret + layers tried +
remediation. `secrets = "none"` repos are skipped silently.

## Security model

- **Config purity:** closed vocabulary (egress recipes, build recipes,
  image features, package names) — reviewed like core code.
- **Policy ceiling:** `default_deny` is monotonic (once true, cannot be
  set false by a later layer); `default_deny = false` requires the core
  entitlement `DEFAULT_DENY_FALSE_ENTITLEMENT`; egress hosts are validated
  against `ALLOWED_EGRESS_HOSTS` at merge time (fail-closed);
  `deny_rules`/`egress_rules`/`secret_env` are additive-union.
- **Trust gating:** project-layer config (`./workestrate.toml`,
  `./workestrate.local.toml`) is only loaded if cwd is in
  `[trusted_projects]`. Untrusted discovery prints a one-time warning and
  stops walking.
- **LiteLLM in-memory:** no Postgres, no virtual keys, no persistent spend
  tracking in the current milestone.

## Egress model

Network policy uses `default_deny` with explicit allow rules for the real
provider hosts. All other outbound traffic is blocked. Provider secrets
are host-bound via `secret_env` so they are only visible to the LiteLLM
microVM.

T3MP3ST (tempest) is an exception: it uses `default_deny: false` (broad
egress) because it is an offensive-security tool that needs to reach
arbitrary targets for scanning. The microVM boundary itself is the
containment layer for tempest. `default_deny: false` requires the
`DEFAULT_DENY_FALSE_ENTITLEMENT` core entitlement.

## Filesystem model

Single tool home (`$WORKESTRATE_HOME`, default `~/.workestrate`; container
`<repo>/.workestrate`). Flat layout:

| Path | Contents |
|---|---|
| `$WORKESTRATE_HOME/config.toml` | Registry: config repos, ordered layers/contexts, trusted projects |
| `$WORKESTRATE_HOME/overrides.toml` | User-global overrides (optional) |
| `$WORKESTRATE_HOME/secrets/` | Machine-local secrets incl. `.env.local.enc` |
| `$WORKESTRATE_HOME/config-repos/<name>/` | Config repo clones: `workestrate.toml`, `.env.enc`, `.sops.yaml`, `infra/litellm/`, `agents/*/config/` |
| `$WORKESTRATE_HOME/sources/<name>/` | Agent source checkouts + builds |
| `$WORKESTRATE_HOME/state/` | `workspaces/`, `var/` (runtime state) |
| `$WORKESTRATE_HOME/cache/` | Cache |

- Source code: tracked in git
- Build artifacts: `control/agentctl/target/` (gitignored)
- Runtime state: under `$WORKESTRATE_HOME/state/` (gitignored)
- Secrets: `.env.enc` committed in config repos (ciphertext-safe);
  decrypted at runtime by `workestrate` internally
- Agent overrides: `$WORKESTRATE_HOME/sources/` (gitignored)

Home resolution: `WORKESTRATE_HOME` env > auto-discovery (walk-up,
trust-gated, finds `.workestrate/` in trusted ancestor) > legacy XDG
(read-only compat + deprecation) > default `~/.workestrate`.

## Config resolution order

Lowest → highest precedence:

1. `WORKESTRATE_CONFIG_DIR` env var (bypasses discovery; single layer, no
   merging)
2. Reference config (`config.reference/workestrate.toml`) — synthetic
   fallback shipped with the tool
3. Context layers (registry `[contexts.<name>] layers = [...]`, in declared
   order; each a config repo's `workestrate.toml`)
4. User-global overrides (`$WORKESTRATE_HOME/overrides.toml`: `[global]`
   applied to every context, then `[configs.<name>]` for each active
   context layer)
5. Trusted project config (`./workestrate.toml` in cwd, IF cwd is in
   `[trusted_projects]`)
6. Local overrides (`./workestrate.local.toml` in cwd, same trust gate as
   #5)

Fail-closed: with no config repos registered, the synthetic reference
config is used (placeholder secrets — `example-service plan` works;
`up`/`exec` for real workloads require a registered config repo).

## Phases and milestones status

### Done

- **Phase 0a (data-driven workloads):** `just verify` passes, golden
  parity, `workloads/*.rs` deleted, odysseus/opencode derivations exist
  (HOST-NIX).
- **Phase 0b (nix recipe parameterization):** PARTIALLY IMPLEMENTED —
  0b.1–0b.5 + Phase 2 core exports + source-build wiring DONE; 0b.6
  (vendor→git-fork, ADR 0011) NOT done, deferred to backlog. The single
  source of truth for the 0b.6 improvement is
  `docs/validation-and-improvements/06-improvements/09-microsandbox-agentd-offline-build.md`.
- **Phase 1 (tool home, ADR 0023):** implemented — config
  add/update/list/trust, init, source clone/build/list/reset,
  migrate-home. Runtime on KVM = HOST-KVM gate (NOT validated).
- **Phase 2 (config-repo-flake):** core `lib.*` exports DONE;
  template-only/HOST-NIX remaining.
- **Phase 3 (layering engine):** IMPLEMENTED (cargo-verified in-container)
  — merge engine, fixture tests, `plan --show-source`, multi-recipient
  SOPS template, copier template. HOST-GATE items (copier copy,
  multi-recipient live, config-repo nix build) deferred.
- **Review remediation:** WP1–WP11 DONE; WP12 = backlog.

### Host-gated (remaining unvalidated pieces)

- **KVM runtime** — the remaining unvalidated piece. `up`/`exec`/`logs`,
  detached mode, internal secret loading, the bun binary in the microVM,
  egress enforcement, and secret isolation are all compile- and
  plan-verified but PENDING KVM runtime validation. This container has no
  `/dev/kvm`. `.#pi` (node) is the fallback if the bun binary misbehaves
  at runtime.
- **HOST-NIX** — nix builds (`nix build .#workestrate`, image builds,
  config-repo flake builds) require a host with nix; this container has
  none.
- **Copier/SOPS live** — `copier copy`, multi-recipient SOPS with two age
  keys, and `nix build` in a config repo are HOST-GATE items deferred from
  Phase 3.

### Future milestones (forward-looking)

- **M2:** Runtime Microsandbox execution on a KVM host; secret injection
  and egress policy tested end-to-end. The `up`/`down`/`exec` subcommands
  are implemented and compile-checked; M2 validates them on a host with
  `/dev/kvm`.
- **M3:** Agents route through LiteLLM using their native configuration
  mechanisms (Pi's `models.json` provider config, Odysseus's
  `data/settings.json`); agents run in microVMs and communicate with the
  LiteLLM proxy.
- **M4:** Postgres-backed LiteLLM with virtual keys, per-agent spend
  tracking, and team isolation.
- **M5:** Production deployment profiles, CI/CD, automated updates.

## Known gaps

- Runtime testing blocked by missing KVM (the remaining unvalidated piece)
- Agent integration with LiteLLM requires native config seeding, not
  `OPENAI_BASE_URL`
- Secret injection and egress enforcement are compile-checked but
  untested at runtime
- `nix build .#workestrate` is wired to the `.#microsandbox` runtime via
  `MSB_PATH`; runtime execution still requires a host with KVM
