# 10 — Current State (Verified)

> Every claim in this document cites a file:line read during this session.
> The codebase is at commit `840e8b7` on `main`.

## Monorepo layout

```
ai-workbench/
├── flake.nix                  # tool flake + devshell + image builds
├── flake.lock
├── justfile                   # validation recipes
├── README.md, SPEC.md        # project docs
├── .env.enc                  # SOPS-encrypted secrets (committed)
├── .env.example              # secrets schema template (committed, 24 lines)
├── .sops.yaml                # SOPS config (committed, 13 lines, 1 recipient)
├── .gitignore
├── control/agentctl/         # Rust CLI (workestrate)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs           # CLI + workloads! macro + dispatch
│       ├── config.rs         # project_root() + check_required_files()
│       └── microsandbox/
│           ├── mod.rs
│           ├── plan.rs       # SandboxPlan IR + types
│           ├── workload.rs   # Workload trait
│           ├── runtime.rs    # sandbox lifecycle (up/down/exec/logs)
│           ├── secrets.rs    # const SecretDefinition
│           ├── secrets_loader.rs  # sops decrypt + env inject
│           ├── mounts.rs     # mount resolution + apply
│           └── env.rs        # ${VAR} template resolution
├── control/agentctl/src/workloads/
│   ├── mod.rs                # pub use litellm::Litellm; ...
│   ├── litellm.rs            # Litellm workload (hardcoded plan)
│   ├── pi.rs                 # Pi workload (hardcoded plan)
│   ├── odysseus.rs           # Odysseus workload (hardcoded plan)
│   ├── opencode.rs           # Opencode workload (hardcoded plan)
│   └── tempest.rs            # Tempest workload (hardcoded plan)
├── nix/
│   ├── devshells/default.nix # devshell (MSB_HOME, _setup_agent_repos, _build_agents)
│   └── packages/
│       ├── agentctl.nix      # workestrate build
│       ├── microsandbox.nix  # microsandbox runtime
│       ├── microsandbox-filesystem-patched.nix
│       ├── microsandbox-filesystem-agentd.patch
│       ├── pi.nix             # pi npm build (105 lines)
│       ├── pi-bun.nix         # pi bun-compile (90 lines)
│       ├── pi-image.nix       # pi dockerTools.buildLayeredImage (73 lines)
│       ├── tempest.nix        # tempest npm build (61 lines)
│       └── tempest-image.nix  # tempest dockerTools.buildLayeredImage (47 lines)
├── infra/
│   ├── litellm/
│   │   ├── config.yaml        # LiteLLM settings (35 lines, file-driven)
│   │   ├── models.yaml        # LiteLLM model_list (160 lines, file-driven)
│   │   └── README.md
│   └── microsandbox/sdk-notes.md
├── agents/
│   ├── README.md              # documents repo/config/build split
│   ├── pi/{config/models.json, repo/, build/, .build-hash}
│   ├── odysseus/{config/settings.json, repo/, build/, .build-hash}
│   ├── opencode/{config/opencode.jsonc, repo/, build/, .build-hash}
│   └── tempest/{repo/, build/, .build-hash}
├── profiles/
│   ├── litellm.md             # human-readable LiteLLM profile
│   └── agents/{pi,odysseus,opencode,tempest}.md
├── scripts/
│   ├── setup-secrets.sh       # 560-line secrets lifecycle
│   ├── host-check.sh          # KVM/Nix/memory/disk check
│   └── validate-secrets-workflow.sh
├── docs/
│   ├── secrets.md             # secrets threat model + wrapper reference
│   └── litellm/               # LiteLLM schema docs + skills
├── .agents/skills/            # validation skills (litellm-config-check, etc.)
├── workspaces/                # per-agent scratch (gitignored except .gitkeep)
└── var/                       # runtime logs/pidfiles (gitignored except .gitkeep)
```

## Hardcoded workloads (compiled-in, not data-driven)

### Workload registry — `main.rs:49-69`

The `workloads!` macro enumerates all 5 workloads with their kind:

```rust
// main.rs:49-69
macro_rules! workloads {
    ($macro:ident) => {
        $macro!(
            Litellm, workloads::Litellm, Service, "LiteLLM proxy sandbox";
            Odysseus, workloads::Odysseus, Service, "Odysseus agent sandbox";
            Pi, workloads::Pi, Agent, "Pi coding agent sandbox";
            Opencode, workloads::Opencode, Agent, "OpenCode agent sandbox";
            Tempest, workloads::Tempest, Agent, "T3MP3ST offensive-security agent sandbox";
        );
    };
    // ...
}
```

Adding a workload requires Rust edits in 3 places: the macro in `main.rs`,
`workloads/mod.rs`, and a new `workloads/<name>.rs`. The `cmd_new` scaffold
(`main.rs:178-227`) prints instructions telling the user to do this manual
wiring.

### Per-workload hardcoded facts

| Fact | litellm | pi | odysseus | opencode | tempest |
|---|---|---|---|---|---|
| **Image** | `ghcr.io/berriai/litellm:v1.89.4` (`litellm.rs:16`) | `workestrator-pi:latest` (`pi.rs:28`) | `python:3.12-slim` (`odysseus.rs:18`) | `node:24-bookworm-slim` (`opencode.rs:23`) | `tempest:latest` (`tempest.rs:27`) |
| **Port** | 4000 (`litellm.rs:35`) | — | 7000 (`odysseus.rs:43`) | 3000 (`opencode.rs:42`) | — |
| **CPUs** | 2 (`litellm.rs:22`) | 2 (`pi.rs:34`) | 2 (`odysseus.rs:24`) | 2 (`opencode.rs:29`) | 2 (`tempest.rs:33`) |
| **Memory** | 2048 (`litellm.rs:23`) | 2048 (`pi.rs:35`) | 2048 (`odysseus.rs:25`) | 2048 (`opencode.rs:30`) | 2048 (`tempest.rs:34`) |
| **Command** | `/app/.venv/bin/litellm --config /app/config/config.yaml --host 0.0.0.0` (`litellm.rs:59-62`) | `/app/bin/pi` (`pi.rs:79`) | `python -m uvicorn app:app --host 0.0.0.0 --port 7000` (`odysseus.rs:66-70`) | `opencode` (`opencode.rs:65`) | `node dist/cli.js` (`tempest.rs:90`) |
| **default_deny** | true (`litellm.rs:41`) | true (`pi.rs:60`) | true (`odysseus.rs:49`) | true (`opencode.rs:56`) | **false** (`tempest.rs:77`) |
| **Egress hosts** | openrouter.ai, api.kimi.com, api.neuralwatt.com, api.minimax.io (`litellm.rs:44-49`) | (agent_base) | huggingface.co, cdn-lfs.huggingface.co, cdn-lfs-us-1.huggingface.co (`odysseus.rs:52-56`) | (agent_base) | (broad) |
| **Deny rules** | — | `.pi.dev` (`pi.rs:62-63`) | — | — | — |
| **log_stop_errors** | true (default) | false (`pi.rs:82-84`) | false (`odysseus.rs:74-76`) | false (`opencode.rs:68-70`) | false (`tempest.rs:93-95`) |

### Secret definitions — `secrets.rs:33-92`

7 secrets defined as `const SecretDefinition` with hardcoded env_var names, egress
host bindings, required/optional flags, and placeholder values:

| Secret | env_var | hosts | required | placeholder | File:line |
|---|---|---|---|---|---|
| `LITELLM_MASTER_KEY` | `LITELLM_MASTER_KEY` | `host.microsandbox.internal` | true | None | `secrets.rs:33-39` |
| `LITELLM_AUTH` (remap) | `LITELLM_MASTER_KEY`→`OPENAI_API_KEY` | (same) | (same) | (same) | `secrets.rs:41-44` |
| `OPENROUTER` | `OPENROUTER_API_KEY` | `openrouter.ai` | true | None | `secrets.rs:46-52` |
| `KIMI` | `KIMI_CODE_API_KEY` | `api.kimi.com` | true | None | `secrets.rs:54-60` |
| `NEURALWATT` | `NEURALWATT_API_KEY` | `api.neuralwatt.com` | true | None | `secrets.rs:62-68` |
| `MINIMAX` | `MINIMAX_CODING_API_KEY` | `api.minimax.io` | true | None | `secrets.rs:70-76` |
| `GITHUB_TOKEN` | `GITHUB_TOKEN` | `github.com, api.github.com` | false | None | `secrets.rs:78-84` |
| `ODYSSEUS_ADMIN_PASSWORD` | `ODYSSEUS_ADMIN_PASSWORD` | `[]` (internal) | true | `change_me_before_first_boot` | `secrets.rs:86-92` |

### Egress recipe helpers — `plan.rs:270-305`

```rust
// plan.rs:271-284 — EgressRule::dns()
// plan.rs:285-291 — EgressRule::litellm_proxy()  (port 4000 to host)
// plan.rs:292-298 — EgressRule::https(domains)   (port 443 to domains)
// plan.rs:299-304 — EgressRule::agent_base()     (dns + litellm_proxy + github)
```

These bake in port 4000 and GitHub as defaults.

### Required-files check — `config.rs:77-129`

`check_required_files()` walks a hardcoded list of `CheckSpec` entries:
`flake.nix`, `infra/litellm/config.yaml`, `infra/litellm/models.yaml`,
`infra/microsandbox/sdk-notes.md`, `profiles/litellm.md`,
`profiles/agents/*.md`, `workspaces/`, `var/`,
`agents/odysseus/config/settings.json`, `agents/opencode/config/opencode.jsonc`,
`.env.enc`, `.sops.yaml` — all required. Plus 8 optional agent repo/build
checks (`config.rs:121-128`).

## Secrets flow (end-to-end)

1. **Schema**: `.env.example` (24 lines) defines key names.
2. **REQUIRED_KEYS derivation**: `setup-secrets.sh:39-44` greps `.env.example`,
   excludes `AI_WORKBENCH_*_DIR`, sorts unique.
3. **Encryption**: `setup-secrets.sh` validates buffer against REQUIRED_KEYS,
   encrypts via `sops --config .sops.yaml` → `.env.enc`.
4. **Runtime decryption**: `secrets_loader.rs:16-61` `load_secrets()` runs
   `sops decrypt --output-type json`, parses with `serde_json`, loads ALL keys
   into process env (no filtering by workload).
5. **Binding**: `secrets.rs` const definitions bind each secret to egress hosts
   + required/optional + placeholder.
6. **Injection**: `runtime.rs:107-145` `apply_plan_secrets()` resolves
   `${VAR}` templates (`env.rs:7-28`), validates required/placeholder, calls
   `b.secret_env(name, value, host)` per allowed host.
7. **Env injection**: `runtime.rs:147-158` `apply_plan_envs()` resolves
   `${VAR}` in plain env vars (e.g. `LITELLM_MASTER_KEY` for Pi's `models.json`
   substitution).

## SandboxPlan IR — `plan.rs:57-70`

```rust
pub struct SandboxPlan {
    pub name: String,
    pub image: Option<String>,
    pub workdir: Option<String>,
    pub command: Vec<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub env: Vec<EnvVar>,
    pub secret_env: Vec<HostBoundSecret>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountPlan>,
    pub network: NetworkPlan,
}
```

This is a clean IR with a `Display` impl (`plan.rs:140-198`). The problem: it's
populated by Rust `plan()` methods, not loaded from files. No `serde` derive;
`Cargo.toml:13-19` has only `serde_json = "1"` (used solely for parsing sops
JSON output at `secrets_loader.rs:50`).

## Workload trait — `workload.rs:45-96`

```rust
pub trait Workload: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn plan(&self) -> SandboxPlan;
    fn exec(&self) -> SandboxCommand;
    fn detach_args(&self) -> Vec<String> { /* default */ }
    fn prepare(&self) -> Result<()> { /* default: Ok(()) */ }
    fn log_stop_errors(&self) -> bool { /* default: true */ }
    fn entrypoint(&self) -> EntrypointSpec { /* default: Shell */ }
    fn build_path(&self) -> String { /* reads WORKESTRATE_<NAME>_BUILD */ }
    fn config_path(&self, filename: &str) -> String { /* agents/<name>/config/<filename> */ }
}
```

Key methods that carry semantics beyond pure data:
- `prepare()` (`workload.rs:62-65`): pi (`pi.rs:86-92`) and odysseus
  (`odysseus.rs:78-84`) override this to seed config files into
  `workspaces/<name>-state/` only if the target is missing.
- `build_path()` (`workload.rs:80-90`): reads `WORKESTRATE_<NAME>_BUILD` env
  var (NAME uppercased, `-`→`_`), falls back to `agents/<name>/build`.
- `config_path()` (`workload.rs:92-95`): returns
  `format!("agents/{}/config/{}", self.name(), filename)`.

These three methods capture semantics that the config schema must preserve via
`seed_files`, `local_build.env_override`+`fallback`, and config-relative mount
paths respectively.

## flake.nix

### Inputs — `flake.nix:7-25`

```nix
inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pi = { url = "github:georgrybski/pi"; flake = false; };
    odysseus = { url = "github:georgrybski/odysseus"; flake = false; };
    opencode = { url = "github:georgrybski/opencode"; flake = false; };
    tempest = { url = "github:georgrybski/T3MP3ST"; flake = false; };
};
```

All agent sources are `flake = false` inputs, overridable via `--override-input`.

### workestrator wrapper — `flake.nix:105-119`

```nix
workestrator-wrapper = { pi-build }: pkgs.runCommand "workestrator" {
    nativeBuildInputs = [ pkgs.makeWrapper ];
} ''
    mkdir -p $out/bin
    makeWrapper ${workestrate}/bin/workestrate $out/bin/workestrate \
      --set WORKESTRATE_PI_BUILD ${pi-build}
'';
```

Bakes `WORKESTRATE_PI_BUILD` into the environment. This is the "bake config
into binary" pattern.

### workload-images attrset — `flake.nix:74-97`

```nix
workload-images = {
    workestrator-pi = pkgs.callPackage ./nix/packages/pi-image.nix { inherit pi-bun-built pi-built; };
    tempest = pkgs.callPackage ./nix/packages/tempest-image.nix { inherit tempest-built; };
    # Future: workestrator-odysseus = ...; workestrator-opencode = ...;
};
```

Data-driven image loading — adding an image = one attrset entry. The
`load-images` script (`flake.nix:82-97`) iterates the attrset.

### Secrets wrappers — `flake.nix:134-196`

`decrypt-env`, `write-env`, `setup-secrets` as `writeShellApplication` wrappers.
All default `SOPS_AGE_KEY_FILE` to
`~/.config/sops/age/ai-workbench-secrets.txt` and `SECRET_FILE` to `.env.enc`.

## Devshell — `nix/devshells/default.nix`

### _setup_agent_repos — `nix/devshells/default.nix:127-167`

Copies flake inputs into `agents/<name>/repo` as writable copies (not
symlinks). Handles 4 cases: existing symlink (replace), real directory with
content (leave alone), empty directory (populate), doesn't exist (copy).

### _build_agents — `nix/devshells/default.nix:169-241`

`_build_if_needed` with dep-manifest gating (sha256 of lockfile). Per-agent
build commands:

| Agent | Build command | Gating file | File:line |
|---|---|---|---|
| odysseus | `python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./.deps -r requirements.lock|requirements.txt` | `requirements.txt` | `nix/devshells/default.nix:225-227` |
| opencode | `HUSKY=0 bun install` | `bun.lock` | `nix/devshells/default.nix:230-232` |
| tempest | `npm install && npm run build` | `package-lock.json` | `nix/devshells/default.nix:235-237` |

**VERIFIED GAP**: odysseus and opencode have NO nix derivations
(`flake.nix:77` comment: `# Future: workestrator-odysseus = ...;
workestrator-opencode = ...;`). They are devshell-only builds. This is a
Phase 0a prerequisite (ADR 0012).

### Image-loaded check — `nix/devshells/default.nix:244-254`

Checks that workload images are loaded into microsandbox, driven by the
`workload-images` attrset via `imageNames`.

## agents/<name>/repo idiom

- `.gitignore:20-22`: `agents/*/repo`, `agents/*/build`, `agents/*/.build-hash`
  are gitignored.
- `config.rs:121-128`: `agents/<name>/repo` and `agents/<name>/build` are
  reported as `[MISSING] (optional)` by `workestrate check`.
- `agents/README.md:5-6`: "repo/ — upstream source code (gitignored).
  Populated automatically by nix develop via flake inputs, or clone your own
  locally."

This is the existing pattern for gitignored, optional, flake-materialized
checkouts inside the repo. The source-override model (`workestrate source
clone`) generalizes it to the XDG store.

## LiteLLM file-driven config

Already file-driven and separated from Rust code:
- `infra/litellm/config.yaml` (35 lines): `general_settings`,
  `router_settings`, `litellm_settings`.
- `infra/litellm/models.yaml` (160 lines): 20+ `model_list` entries with
  provider prefixes, `api_base`, `api_key` via `os.environ/`.
- Consumed by LiteLLM at runtime; Rust code only references the mount path
  (`litellm.rs:38`: `MountPlan::readonly("infra/litellm", "/app/config")`).

## Agent config files (already file-driven)

- `agents/pi/config/models.json` (19 lines): Pi's provider config.
- `agents/odysseus/config/settings.json` (9 lines): Odysseus provider config.
- `agents/opencode/config/opencode.jsonc` (14 lines): OpenCode config.

Seeded into `workspaces/<name>-state/` by `prepare()` only when target is
missing (`pi.rs:86-92`, `odysseus.rs:78-84`).

## .env.example (schema template)

`.env.example` (24 lines): 7 key names + 2 optional path vars.
`setup-secrets.sh:39-44` derives `REQUIRED_KEYS` by grepping it.

## .sops.yaml

`.sops.yaml` (13 lines): single age recipient
(`age125ahf9ekcgqejr2u47e5cc0k8u509mw6wdwngpscrgty24djr49qyuzgza`), one
`creation_rules` entry matching `^.env.enc$` with `input_type: dotenv`,
`key_groups` with one age key.

## M1 status / KVM limits

- `SPEC.md:18-19`: "No runtime-validated claims — this environment lacks KVM;
  `up`/`down` are implemented and compile-checked only in M1."
- `SPEC.md:165`: `[ ] Runtime sandbox execution (blocked: no KVM)`
- `README.md:511-513`: "Running microVMs at runtime requires a host with
  `/dev/kvm`; this development container has none."

## Microsandbox vendor symlink

- `control/agentctl/vendor/microsandbox-filesystem-0.5.6` is a Nix-managed
  symlink to `${microsandbox-filesystem-patched}`.
- `nix/devshells/default.nix:99-125`: `_setup_vendor_link` refreshes it on
  devshell entry.
- `justfile:59-88`: `vendor-unlock` (replace symlink with writable copy) and
  `vendor-lock` (remove copy so devshell recreates symlink).
- `nix/packages/agentctl.nix:41-47`: preBuild stages the patched crate.
- `nix/packages/microsandbox-filesystem-agentd.patch`: the patch file.

This vendor symlink mechanism migrates to a git-fork dependency as Phase 0
cleanup (ADR 0011).

## What is ALREADY file-driven vs compiled-in

| Aspect | File-driven? | Evidence |
|---|---|---|
| LiteLLM config values | YES | `infra/litellm/config.yaml`, `models.yaml` |
| Agent config files | YES | `agents/*/config/*.json` |
| `.env.example` schema | YES | `.env.example` (24 lines) |
| `.sops.yaml` | YES | `.sops.yaml` (13 lines) |
| Flake inputs (agent sources) | YES | `flake.nix:7-25` |
| Workload definitions (ports, egress, images, commands) | **NO** | `workloads/*.rs` hardcoded |
| Secret definitions (env_var→hosts) | **NO** | `secrets.rs:33-92` const |
| Required-files check list | **NO** | `config.rs:77-129` hardcoded |
| Egress recipe helpers | **NO** | `plan.rs:270-305` hardcoded |
| SandboxPlan IR | Data model exists but no serde/loader | `plan.rs:57-70` |
