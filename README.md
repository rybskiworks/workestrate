# workestrate

**workestrate** is an agent-first microVM workload orchestrator: a Rust CLI
plus a Nix flake that run declaratively-configured workloads (agents and
services) inside Microsandbox microVMs with default-deny networking and
SOPS-encrypted secrets. Workloads are **declared in TOML config repos**,
not hardcoded in the tool — the CLI composes config from ordered layers
(registry, contexts, trusted projects, local overrides) with
security-aware merge semantics.

## Positioning

workestrate is **agnostic workload configuration/orchestration**, built
agent-first. Agents happen to be the first-class use case today (coding
agents behind a local LiteLLM proxy, each in its own microVM), but the
machinery is deliberately generic: any workload you can describe as an
image + command + env/ports/mounts/seeds/network policy can be declared in
a config repo and orchestrated the same way. The intent is to generalize
beyond the current agent setup — applicable to anything else that benefits
from declarative, policy-gated microVM workloads — not to be tied to one
agent stack.

The rest of this README documents **what exists today**, accurately.

## How it works today

- **Tool, not a workspace project.** The `workestrate` Rust CLI
  (`control/agentctl/`) drives everything. Config lives in a single tool
  home (`$WORKESTRATE_HOME`, default `~/.workestrate`) holding the
  registry, config-repo clones, ordered layers/contexts, user-global
  overrides, secrets, sources, and runtime state.
- **Nix-native.** Builds, devshells, and tool pins all flow through the
  flake (`flake.nix`). The flake also exports `lib.*` recipes
  (`buildImagesFromConfig`, `bun-compile`, `npm-build`, `nix-layered`,
  `checks.validateConfig`) for config-repo flakes to consume. Workload
  image builds live in the config repo flake, not the tool repo.
- **Built on a Microsandbox fork.** The `microsandbox` SDK is pinned to
  `=0.6.16` with the `net` feature, source-built from the pinned
  `microsandbox-fork` flake input. Runtime execution requires a host with
  `/dev/kvm`; no Docker is involved.
- **Default-deny network policy.** Egress and ingress default to deny;
  allow rules are explicit. Policy is a ladder: `allow` stands alone, home
  `final` seals veto, and `entitlements` is a retired key (hard parse
  error) — see ADR 0035 and `docs/migration/30-security-model.md`.
- **Fail-closed without config.** A fresh clone ships only a synthetic
  reference config (`config.reference/`) with placeholder workloads
  (`example-service`, `example-agent`, `example-offensive`). `plan` and
  `validate-config` work immediately; `up`/`exec` for real workloads
  require a registered config repo.

## CLI surface

Workload names (`litellm`, `pi`, `odysseus`, `opencode`, `tempest`) are
defined by your active config repo, not by the tool. Workloads split into
two kinds: **services** support `up`/`down`/`logs`/`plan`; **agents**
support `exec`/`down`/`plan` (agents are interactive — you attach with
`exec`).

```bash
workestrate workload plan example-service        # plan works on a fresh clone
workestrate workload plan <name> --show-source   # per-field layer provenance
workestrate workload up <service>                # start detached (topo-ordered deps)
workestrate workload up <service> --foreground   # block until Ctrl-C
workestrate workload logs <service>              # tail a detached service log
workestrate workload exec <agent>                # attach interactively (TUI)
workestrate workload down <name>                 # stop a singleton slot
workestrate workloads                            # list configured workloads + status
workestrate ps [--json]                          # running instances (all contexts)
```

**Instance lifecycle** (ADR 0021/0026): `up`/`exec` target a **slot** — a
singleton (`<workload>`, or `<context>-<workload>` with a context active)
or a parallel instance (`<slot>@<id>`). Occupied slots **refuse** by
default; `--replace` recycles explicitly, `--instance <id>`/`--new` start
a parallel canary on its own per-instance loopback IP (`127.0.0.N`).
`host = 0` ports and `--port-auto` probe a free port at boot. Scoped
teardown: `workestrate down --all|--context <ctx>|--config-ref
<ref>|--everything` (exactly one selector; `workestrate clean` is
state/cache hygiene only and never tears down VMs). Full detail:
`docs/operating-model.md` and `docs/migration/20-target-system-spec.md`
§13.

**Config and source management:**

```bash
workestrate home init                     # initialize the tool home (idempotent)
workestrate home clone <src> [dest]       # provision a home from an existing one
workestrate config add <url> <name>       # clone + register a config repo
workestrate config new <name> [dest]      # scaffold a new config repo (in-store auto-registers)
workestrate config update [name]          # pull latest, refuse dirty clones
workestrate config list                   # registered repos with rev + dirty status
workestrate config trust <dir>            # trust a project dir for project-layer config
workestrate source clone|build|list|reset # agent source checkouts in the managed store
workestrate migrate-home                  # legacy XDG layout → single home
```

**Introspection and escape hatches:**

```bash
workestrate check | doctor | versions     # layout sanity / provisioning state / pin quadruple
workestrate validate-config               # schema + policy allowlist validation
workestrate secrets-schema                # secret env var names from config
workestrate generate-env-example          # .env.example from the config secrets section
workestrate generate-schema | schemas update
workestrate completions <shell>           # bash, zsh, fish, elvish, powershell
workestrate msb -- <args>                 # verbatim passthrough to the pinned msb binary
workestrate run -- <cmd>                  # exec a command with decrypted secrets (BY DESIGN
                                          # full secret access — the operator's escape hatch)
```

## Config repos and the layering model

A config repo declares workloads and the secrets schema in
`workestrate.toml` (or directory-mode `workestrate/` capsules; ADR 0017)
and carries its own `.env.enc` + `.sops.yaml`. Config repos live in the
managed store at `$WORKESTRATE_HOME/config-repos/<name>/`; agent source
checkouts live in `$WORKESTRATE_HOME/sources/<name>/`.

Config resolution (lowest → highest precedence):

1. `WORKESTRATE_CONFIG_DIR` env (dev/testing bypass; single layer, no merge)
2. Reference config shipped with the tool (synthetic fallback)
3. Context layers (`[contexts.<name>] layers = [...]`, in declared order)
4. User-global overrides (`$WORKESTRATE_HOME/overrides.toml`)
5. Trusted project config (`./workestrate.toml`, only if cwd is trusted)
6. Local overrides (`./workestrate.local.toml`, same trust gate)

The merge is **security-aware** (ADR 0005/0035): deny rules and egress
rules are additive-union, env bindings union by key, and everything else
uses RFC 7396 merge-patch. Egress hosts are validated against the closed
`ALLOWED_EGRESS_HOSTS` vocabulary at merge time (fail-closed).

**Seeds.** `[[seed_files]]` entries copy files into a sandbox before
start; `template = true` renders `${VAR}` against the guest-visible env
view (host-bound secrets appear as `$MSB_<name>` placeholders; a missing
var is a hard error), `glob` seeds every sorted match to
`target/<rel-path>`, and `--reseed` forces template re-render. This is how
agents get their LiteLLM provider config without hardcoding values. See
`docs/migration/20-target-system-spec.md` §3.

**msb runtime model.** `workestrate` converges on one canonical msb home
(`~/.microsandbox/current`, generation-keyed per pinned msb build) and the
`msb`/`versions` verbs expose the exact pin. msb semantics are
Docker-inspired but intentionally different — read
`docs/runtime-provisioning.md` before assuming anything.

## Setup

Prerequisites: Debian/Ubuntu on x86_64 with virtualization enabled,
`/dev/kvm` accessible to your user, Nix with flakes, ~4 GB RAM and ~20 GB
free disk. No Docker required. Run `just host-check` to verify.

```bash
git clone <repo-url> workestrate
cd workestrate
just shell                 # devshell (pins the Rust toolchain, just, nodejs_24)

workestrate home init
workestrate config new personal          # or: workestrate config add <url> personal
setup-secrets --config personal init     # one-time secrets bootstrap (see below)
workestrate check
workestrate workload up litellm          # example service from your config repo
workestrate workload exec pi             # example agent from your config repo
```

> Nix builds see tracked files only — `git add` new files before building.

## Secrets workflow

Secrets are SOPS-encrypted (age recipient) per config repo; each config
repo holds `.env.enc` (ciphertext-safe to commit) + `.sops.yaml`, and an
optional user-global layer (`$WORKESTRATE_HOME/secrets/.env.local.enc`)
applies per-key across contexts. Layers merge per key (later wins; process
env is lowest precedence).

- `setup-secrets --config <name> init` — one-time: creates the age key
  (mode 0600) if missing, fixes up `.sops.yaml`, opens an editor with a
  pre-filled buffer of required keys, and writes the encrypted `.env.enc`.
  Refuses to overwrite an existing file — use `update` for changes.
- `setup-secrets --config <name> update` — decrypts, edits, re-encrypts.
- `setup-secrets --global init|update` — targets the user-global
  `.env.local.enc`.
- `workestrate run -- <cmd>` — decrypts into the process environment and
  execs a command (escape hatch, not sandboxed). Commands that don't need
  secrets (`plan`, `check`, `completions`, …) skip decryption entirely.

The **age private key stays on the host** at
`~/.config/sops/age/ai-workbench-secrets.txt` — never in any repo, never
under `$WORKESTRATE_HOME` (the repo is agent-reachable via `${CWD}`
mounts). In a container the key is absent and secret operations fail
closed by design. Back the key up: without it, `.env.enc` is
undecryptable. Full threat model and wrapper reference:
`docs/secrets.md`.

The secrets catalog (defined per config repo; the current personal setup
uses seven): `LITELLM_MASTER_KEY`, provider keys (`OPENROUTER_API_KEY`,
`KIMI_CODE_API_KEY`, `NEURALWATT_API_KEY`, `MINIMAX_CODING_API_KEY`),
`GITHUB_TOKEN`, and `ODYSSEUS_ADMIN_PASSWORD`. Secret exposure is per
binding: `host`-bound (default) shows the guest only a placeholder and
substitutes the real value in host-side traffic to the secret's
`allowed_hosts`; `guest`-bound injects the real value and is reserved for
workloads that verify the credential (e.g. the LiteLLM proxy itself).

## Development workflow

**just-first.** Run `just <recipe>` from a plain host shell — recipes
self-enshell (`nix develop`) as needed. Key recipes:

| Recipe | What it does |
|---|---|
| `just shell` | Interactive devshell (`just shell -c <cmd>` passes through) |
| `just verify` | **The gate before pushing/review**: lock-guard, toolchain/versions checks, fmt/clippy/check/test, spec-examples, tombi, golden/schema/scaffold drift guards, lint-nix, **deny-check**, store-audit |
| `just verify-full` | `verify` plus `nix build .#workestrate` |
| `just check` / `just test` | fmt + clippy + check / unit tests for `control/agentctl` |
| `just deny-check` | cargo-deny supply-chain gates (config: `deny.toml`) |
| `just host-check` / `just host-provision` / `just provision-check` | Host prerequisites / provisioning / read-only provisioning check |
| `just kvm-tests` | The ignored KVM tests, host-side |
| `just gc` / `just store-audit` | Nix store hygiene |
| `just lint-nix` / `just tombi-check` | Purity and TOML gates |

Derivation purity rules live in `docs/nix-purity.md`; devshell rules in
`docs/nix/devshells.md`.

**Git hooks.** Install from `scripts/git-hooks/` (pure-sh fallbacks:
pre-commit runs the tier-1/2 gates, pre-push the tier-3 gate). Hooks are
check-only — **never commit with `--no-verify`**; fix findings properly.

**Commit conventions.** Conventional commit prefixes (`feat:`, `fix:`,
`chore:`, `docs:`, `ci:`, `build:`, …). ADR numbers belong in commit
bodies/footers, never in subjects. Secrets stay encrypted (`.env.enc`) —
never commit decrypted material.

**CI gate posture.** PRs run light gates defined in the workflow itself
(rustup-pinned cargo fmt/clippy/test plus pure-script gates) — the devenv
is never booted in CI. The heavy **e2e-nix** leg is on-demand via the
`nix-ci` PR label (owner/maintainer-triggered; agents must NOT self-apply
it). Supply-chain gates run via cargo-deny (`just deny-check` locally;
hard gate on licenses/bans/sources, soft on advisories). CODEOWNERS covers
`.github/`, `flake.nix`, `flake.lock`, and `deny.toml`.

## Canonical docs

- [`SPEC.md`](SPEC.md) — the normative system specification (components,
  secrets model, security/egress model, resolution order, phase status).
- [`docs/migration/`](docs/migration/README.md) — the migration tree:
  [`20-target-system-spec.md`](docs/migration/20-target-system-spec.md)
  (CLI surface, seeds, lifecycle — the implementation-authoritative spec),
  [`30-security-model.md`](docs/migration/30-security-model.md) (policy
  ladder + secrets model).
- [`docs/migration/50-decisions/README.md`](docs/migration/50-decisions/README.md)
  — ADR index (0021 instance lifecycle, 0026 per-instance addressing, 0035
  hierarchical policy, 0036 nested-virt, 0037 msb state generations, …).
- [`docs/runtime-provisioning.md`](docs/runtime-provisioning.md) — msb
  runtime provisioning, state generations, pin wiring.
- [`docs/secrets.md`](docs/secrets.md) — secrets threat model and wrapper
  reference.
- [`docs/operating-model.md`](docs/operating-model.md) — teardown ladder
  and operating semantics.

## Runtime status

Everything above is implemented and cargo-verified. KVM runtime is
host-validated for pi/prime (microVM boot, egress + secret substitution,
kernel env; 2026-08-13); other workload runtime parity remains
host-unvalidated. This container has no `/dev/kvm` — host runs happen on a
KVM host. Nested virtualization is opt-in (`virtualization.nested`) but
guest `/dev/kvm` arrives with the Phase-2 firmware rebuild (ADR 0036).
LiteLLM runs in-memory (no Postgres, no virtual keys, no persistent spend
tracking) in the current milestone.
