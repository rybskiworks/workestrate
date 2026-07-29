# 02 — Config Requirements (workestrate.toml contract)

> **STATUS: READY-TO-EXECUTE (requirements frozen pending sign-off)**

> CLI config authoring is DEFERRED (see `06-improvements/04-cli-config-authoring.md`);
> this document is the contract it will target.

This is THE requirements-alignment document for the workestrate config surface.
It pins the full intended config schema, the merge/layering semantics, the
policy ceiling, the trust model, and the planned extensions — so that no config
semantics need re-litigation when the future CLI authoring tool is built. The
schema structs and merge engine already implement this contract; this document
is the human-readable stable contract that future tooling and config-repo
authors target.

## Prerequisites / see also

| Document | Role |
|---|---|
| `README.md` | Entry point for this validation-and-improvements doc set. |
| `01-current-state-and-prereqs.md` | Verified current state and prerequisites. |
| `04-baseline-validation.md` | Baseline validation gates and evidence. |
| `06-improvements/01-mount-filtering-shadowing.md` | Full spec for the planned mount filtering/shadowing extension. |
| `06-improvements/04-cli-config-authoring.md` | The DEFERRED toml_edit-based CLI authoring tool this doc is the contract for. |
| `../migration/50-decisions/README.md` | ADR index (0001–0023); the authoritative decision record. |

### Environment markers

- `verifiable-here` — the check can be run inside the container without host
  privileges (e.g. `workestrate validate-config`).
- `HOST-NIX` — requires `nix` on the host (e.g. `nix build` of a
  `nix-layered` image recipe).
- `HOST-KVM` — requires KVM on the host (e.g. bringing up a live sandbox for
  smoke testing).

### Terms

Brief inline definitions (full glossary at `../migration/60-glossary.md`):

- **Layer** — a config source (`workestrate.toml`) that participates in the
  merge order. Earlier layers are overridden by later ones for non-security
  fields; security fields use security-aware merge (ADR 0005).
- **Context** — a named layer-set in the registry (`[contexts.<name>]`); one
  context is resolved per invocation (ADR 0019).
- **Recipe** — a named, versioned, reviewable unit of executable logic in core
  that config references by name. Recipes are a closed vocabulary (ADR 0003):
  config is data; all logic is named recipes in core.

---

## 1. The `workestrate.toml` layer schema

The schema root of a single `workestrate.toml` layer is `ConfigFile`
(`control/agentctl/src/config/types.rs:194`):

```toml
schema_version = 1

[secrets.<NAME>]      # secret definitions (map, deep-merged per field)

[workloads.<name>]    # workload definitions (map, deep-merged per field)
```

All config-layer structs carry `#[serde(deny_unknown_fields)]`
(`types.rs:6-10`), so an unknown key in a config LAYER hard-errors at parse
time. The user-global overrides path stays lenient (warn + strip) — see §5.

### 1.1 `schema_version`

```toml
schema_version = 1
```

- Type: `u32` (`types.rs:196`).
- Currently `1` (`EXPECTED_SCHEMA_VERSION`, `config/validation.rs:21`).
- Missing/`0` is accepted as legacy with a stderr warning (backward compat).
- Any other value (`>= 2`) is a hard error (`validation.rs:121-137`).

### 1.2 `[secrets.<NAME>]` — secret definitions

Each named secret is a `SecretDefConfig` (`types.rs:176`):

```toml
[secrets.LITELLM_MASTER_KEY]
env_var = "LITELLM_MASTER_KEY"
hosts = ["host.microsandbox.internal"]
required = true
placeholder = "change_me_before_first_boot"   # optional
source = "LITELLM_MASTER_KEY"                 # optional: alias source
exposed_as = "OPENAI_API_KEY"                 # optional: alias target env var
description = "LiteLLM proxy authentication."  # optional
```

| Field | Type | Required | Semantics |
|---|---|---|---|
| `env_var` | `Option<String>` | no | Env var the resolved value is injected as. |
| `hosts` | `Option<Vec<String>>` | no | Constrains which egress hosts may receive it (validated against `SECRET_HOST_BINDINGS`). |
| `required` | `Option<bool>` | no | Missing value is a hard error when `true`. |
| `placeholder` | `Option<String>` | no | Placeholder shown when value is absent. |
| `source` | `Option<String>` | no | Name of another secret to alias the value from. |
| `exposed_as` | `Option<String>` | no | Env var name to expose the aliased value as (instead of `env_var`). |
| `description` | `Option<String>` | no | Human-readable description. |

**Alias pattern (LITELLM_AUTH):** a secret with `source` + `exposed_as` and no
`env_var` is an alias — it takes the resolved value of the `source` secret and
exposes it under the `exposed_as` env var name. The live example
(`.workestrate/repos/personal/workestrate.toml:9-11`):

```toml
[secrets.LITELLM_AUTH]
source = "LITELLM_MASTER_KEY"
exposed_as = "OPENAI_API_KEY"
```

This lets agent workloads consume the LiteLLM master key as `OPENAI_API_KEY`
(pointing at the in-sandbox proxy) without receiving the raw provider keys.

Merge: secrets are a map deep-merged per-field (last-layer-wins per field,
`merge.rs:195-267`).

### 1.3 `[workloads.<name>]` — workload definitions

Each workload is a `WorkloadConfig` (`types.rs:142`):

```toml
[workloads.<name>]
kind = "service"            # "service" | "agent"
image = { ... }              # ImageSpec (see 1.3.1)
workdir = "/app"            # optional
cpus = 2                    # optional, u8
memory_mib = 2048           # optional, u32
command = ["..."]           # Vec<String>
log_stop_errors = true      # optional
# ... env, secret_env, ports, mounts, seed_files, local_build, network
```

| Field | Type | Merge rule |
|---|---|---|
| `kind` | `String` (`"service"` or `"agent"`) | last-layer-wins |
| `image` | `ImageSpec` | last-layer-wins (wholesale replace) |
| `workdir` | `Option<String>` | last-layer-wins |
| `cpus` | `Option<u8>` | last-layer-wins |
| `memory_mib` | `Option<u32>` | last-layer-wins |
| `command` | `Vec<String>` | REPLACE (last-layer-wins) |
| `log_stop_errors` | `Option<bool>` | last-layer-wins |
| `env` | `Vec<EnvVarConfig>` | union-by-name (ADR 0020 Ruling 1) |
| `secret_env` | `Vec<SecretEnvConfig>` | additive-union (ADR 0005) |
| `ports` | `Vec<PortMapping>` | REPLACE |
| `mounts` | `Vec<MountPlan>` | REPLACE |
| `seed_files` | `Vec<SeedFileConfig>` | REPLACE |
| `local_build` | `Option<LocalBuildConfig>` | REPLACE |
| `network` | `NetworkConfig` | field-specific (see §3) |

#### 1.3.1 `image` — ImageSpec (`types.rs:27`)

```toml
image = { recipe = "registry", ref = "ghcr.io/berriai/litellm:v1.89.4" }
# or
image = { recipe = "nix-layered", name = "workestrator-pi", tag = "latest",
          contents = ["cacert", "busybox", "fakeNss"],
          binary = { recipe = "bun-compile", src = "flake://pi",
                     entrypoint = "packages/coding-agent/dist/bun/cli.js",
                     worker = "packages/coding-agent/src/utils/image-resize-worker.ts" },
          baked_files = [{ path = "root/.config/t3mp3st/config.json",
                           content = '{"defaultProvider":"local"}' }],
          features = ["create_tmp"] }
```

| Field | Type | Semantics |
|---|---|---|
| `recipe` | `String` | Acquisition strategy: `"registry"` or `"nix-layered"` (closed enum, `validation.rs:80`). |
| `ref` | `Option<String>` | Image ref (TOML key `ref`; Rust `r#ref`). Used with `recipe = "registry"`. |
| `name` | `Option<String>` | Image name. Used with `recipe = "nix-layered"`. |
| `tag` | `Option<String>` | Image tag. |
| `contents` | `Option<Vec<String>>` | Nix packages (validated against `ALLOWED_PACKAGES`). |
| `binary` | `Option<BinarySpec>` | A binary built from source and baked in. |
| `baked_files` | `Option<Vec<BakedFileSpec>>` | Static files baked in at build time (`path` + `content`). |
| `features` | `Option<Vec<String>>` | Named features from a closed vocabulary (`create_tmp`). |

**BinarySpec** (`types.rs:46`): `recipe` (build strategy), `src` (source
location), `entrypoint`, `worker`, `npm_deps_hash` (all optional). Build
recipes are a closed enum: `npm-build`, `bun-compile`, `pip-install`,
`bun-install` (`validation.rs:85`). The schema deliberately has **no
`install_layout` field** — it was removed as a silent no-op
(`nix/lib/recipes/npm-build.nix:16-25`), and `#[serde(deny_unknown_fields)]`
at `types.rs:44` makes any `install_layout` key a **hard parse error**. The
live personal bundle currently violates this (see
[01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) §"Bundle
fixes needed" item e).

**BakedFileSpec** (`types.rs:59`): `path` (in-image destination), `content`
(verbatim string body).

#### 1.3.2 `[[workloads.<name>.env]]` — EnvVarConfig (`types.rs:70`)

```toml
[[workloads.litellm.env]]
name = "PORT"
value = "4000"

[[workloads.litellm.env]]
name = "LITELLM_MASTER_KEY"
secret = "LITELLM_MASTER_KEY"   # reference to a [secrets.<name>] entry
```

Exactly one of `value` (literal) or `secret` (reference) is expected. `name`
must be a valid shell env identifier. Merges union-by-name (last-layer-wins per
key, `merge.rs:344-362`).

#### 1.3.3 `[[workloads.<name>.secret_env]]` — SecretEnvConfig (`types.rs:82`)

```toml
[[workloads.litellm.secret_env]]
secret = "OPENROUTER_API_KEY"
```

Names an entry in the top-level `secrets` map whose resolved value is injected
into the sandbox environment. Merges additive-union by secret name
(`merge.rs:385-404`).

#### 1.3.4 `[[workloads.<name>.ports]]` — PortMapping (`microsandbox/plan.rs:82`)

```toml
[[workloads.litellm.ports]]
host = 4000
guest = 4000
```

REPLACE merge (wholesale replace, no partial row merge, ADR 0020 Ruling 1).

#### 1.3.5 `[[workloads.<name>.mounts]]` — MountPlan (`microsandbox/plan.rs:89`)

```toml
[[workloads.litellm.mounts]]
host = "${MSB_HOME}/sandboxes/litellm/logs"
guest = "/var/log/litellm"
read_only = false
```

REPLACE merge. Host paths support template variables (see §6).

#### 1.3.6 `[[workloads.<name>.seed_files]]` — SeedFileConfig (`types.rs:93`)

```toml
[[workloads.pi.seed_files]]
source = "agents/pi/config/models.json"
target = "workspaces/pi-state/agent/models.json"
only_if_missing = true
```

`source` is a project-root-relative host path (no `..`, no absolute, no
template tokens — `validate.rs:72-96`). `target` starting with `workspaces/`
or `var/` is resolved relative to the state dir (`config.rs:150`).
REPLACE merge.

#### 1.3.7 `[workloads.<name>.local_build]` — LocalBuildConfig (`types.rs:106`)

```toml
[workloads.odysseus.local_build]
recipe = "pip-install"
source = "flake://odysseus"
requirements_file = "requirements.txt"
target = ".deps"
gating_file = "requirements.txt"
env_override = "WORKESTRATE_ODYSSEUS_BUILD"
fallback = "agents/odysseus/build"
```

| Field | Type | Semantics |
|---|---|---|
| `recipe` | `String` | Build strategy (closed enum: `npm-build`, `bun-compile`, `pip-install`, `bun-install`). |
| `source` | `String` | Checkout location (e.g. `flake://odysseus`). |
| `requirements_file` | `Option<String>` | For `pip-install`. |
| `target` | `Option<String>` | Output directory within the build. |
| `gating_file` | `Option<String>` | Incremental-build gate (rebuild when changed). |
| `env_override` | `Option<String>` | Env var name for the build path (also a mount template token). |
| `fallback` | `Option<String>` | Fallback path if the build is absent. |

REPLACE merge.

#### 1.3.8 `[workloads.<name>.network]` — NetworkConfig (`types.rs:123`)

```toml
[workloads.litellm.network]
default_deny = true

[[workloads.litellm.network.egress]]
recipe = "dns"

[[workloads.litellm.network.egress]]
recipe = "https"
hosts = ["openrouter.ai", "api.kimi.com"]

[[workloads.pi.network.deny]]
domain_suffix = ".pi.dev"

[[workloads.litellm.network.ingress]]
protocol = "tcp"
port = 4000
scope = "local"
```

| Sub-field | Type | Merge rule |
|---|---|---|
| `default_deny` | `Option<bool>` | monotonic-true (security-aware, ADR 0005) |
| `egress` | `Vec<EgressRecipeRef>` | additive-union with dedup (security-aware) |
| `deny` | `Vec<DenyDomainRule>` | additive-union (security-aware) |
| `ingress` | `Vec<IngressRule>` | REPLACE |

**EgressRecipeRef** (`recipes.rs:10`): a tagged enum (`recipe` discriminator,
snake_case). Closed vocabulary: `dns`, `litellm_proxy`, `github`,
`agent_base`, `https` (the latter takes `hosts: Vec<String>` validated against
`ALLOWED_EGRESS_HOSTS`).

**DenyDomainRule** (`plan.rs:144`): `domain_suffix: String`.

**IngressRule** (`plan.rs:150`): `protocol` (e.g. `tcp`), `port: u16`,
`scope` (e.g. `local`).

---

## 2. Registry file (`$WORKESTRATE_HOME/config.toml`)

The registry is the tool-home file at `$WORKESTRATE_HOME/config.toml`
(ADR 0023). Its schema root is `Registry` (`types.rs:278`):

```toml
# Ordered default layer stack (each name = a config repo in [configs]).
layers = ["personal"]

[settings]
default_context = "personal"     # optional
store_dir = "/custom/store"      # optional (default: $WORKESTRATE_HOME)
state_dir = "/custom/state"      # optional (default: $WORKESTRATE_HOME/state)
home_version = 2                 # optional (1 = legacy XDG, 2 = single home)

[configs.personal]
url = "https://example.invalid/personal.git"
ref = "main"                     # TOML key "ref" (Rust r#ref)
rev = "abc123"                   # pinned commit
secrets = "file"                 # "file" (default) | "none"
secrets_file = ".env.enc"        # default ".env.enc"
age_key_file = "~/.config/sops/age/keys.txt"  # optional

[contexts.personal]
layers = ["personal"]             # ordered layer-set for this context

[[trusted_projects]]
path = "/home/node/Development/ai-workbench"
```

| Section | Struct | Semantics |
|---|---|---|
| `layers` | `Vec<String>` | Default ordered layer stack (backward compat when no contexts). |
| `[settings]` | `RegistrySettings` (`types.rs:212`) | Tool-wide settings. |
| `[configs.<name>]` | `ConfigRepoEntry` (`types.rs:230`) | Registered config repo. |
| `[contexts.<name>]` | `Context` (`types.rs:267`) | Named layer-set. |
| `[[trusted_projects]]` | `TrustedProject` (`types.rs:258`) | Trusted project directory. |

Context selection precedence (ADR 0019): `--context <name>` flag >
`WORKESTRATE_CONTEXT` env > `[settings] default_context` > bare `layers`
(backward compat).

---

## 3. Merge / layering semantics (the contract)

### 3.1 Layer order (lowest → highest precedence)

Per `load_config()` (`config/loading.rs:274-297`):

1. **Reference config** (`config.reference/workestrate.toml`) — the base
   layer shipped with the tool.
2. **Registry layers** — the active context's `layers` array, in declared
   order (each a config repo at `<store>/repos/<name>/workestrate.toml`).
3. **User-global overrides** (`$WORKESTRATE_HOME/overrides.toml`): `[global]`
   applied to every context, then `[configs.<name>]` for each active context
   layer (ADR 0019).
4. **Trusted project config** (`./workestrate.toml` in cwd, if trusted —
   ADR 0014).
5. **Local overrides** (`./workestrate.local.toml` in cwd, if trusted —
   ADR 0020 Ruling 3).

**`WORKESTRATE_CONFIG_DIR` bypass:** when set, discovery is bypassed entirely
and a single dev/testing layer is loaded directly from
`$WORKESTRATE_CONFIG_DIR/workestrate.toml` (`loading.rs:287-297`). This is an
operator-trust escape valve (ADR 0020 Ruling 3): whoever controls the
environment controls the process.

### 3.2 Security-aware merge rules (ADR 0005 + ADR 0020)

The merge engine (`merge.rs`) applies field-specific rules:

| Field | Merge rule | Source |
|---|---|---|
| `default_deny` | **Monotonic-true**: once `true`, stays `true`. `false` requires core entitlement (`DEFAULT_DENY_FALSE_ENTITLEMENT`). Entitlement is checked BEFORE monotonic-true (ADR 0020 Ruling 2). | `merge.rs:449-479` |
| `deny` (deny_rules) | **Additive-union** within policy.rs ceiling. Cannot remove a more-trusted layer's deny rule. | `merge.rs:524-538` |
| `egress` (egress_rules) | **Additive-union** with canonical dedup (sorted+deduped hosts for `https`) within `ALLOWED_EGRESS_HOSTS` ceiling. | `merge.rs:481-522` |
| `secret_env` | **Additive-union** by secret name (last-layer-wins per secret). Cannot deprive a workload of required secrets. | `merge.rs:385-404` |
| `env` | **Union-by-name** (last-layer-wins per env-var key). ADR 0020 Ruling 1. | `merge.rs:344-362` |
| `ports`, `mounts`, `seed_files`, `local_build`, `network.ingress` | **REPLACE** (wholesale replace, no partial row merge). ADR 0020 Ruling 1. | `merge.rs:363-384`, `merge.rs:540-546` |
| All other scalars/maps | **RFC 7396**: last-wins scalars, deep-merge maps. | `merge.rs:310-343`, `merge.rs:195-267` |

**Key invariants:**

- A less-trusted layer **cannot** weaken `default_deny` (monotonic-true).
- A less-trusted layer **cannot** remove a deny rule or egress rule (additive).
- A less-trusted layer **cannot** deprive a workload of required secrets
  (additive `secret_env`).
- `default_deny = false` requires the workload name to be in
  `DEFAULT_DENY_FALSE_ENTITLEMENT` (currently `["tempest", "example-offensive"]`,
  `policy.rs:45`). For non-entitled workloads, the entitlement check rejects
  `false` before monotonic-true even applies — they can never reach
  `Some(false)`.

---

## 4. Policy layer (ADR 0004) + config purity (ADR 0003)

### 4.1 policy.rs — the code-side allowlist

`policy.rs` is the compiled-in ceiling that config cannot exceed. Enforcement
at three points: `validate-config` (pre-flight), `plan` (fail-closed), and
runtime `apply_plan_secrets` (ADR 0004).

| Const | Purpose | Source |
|---|---|---|
| `ALLOWED_EGRESS_HOSTS` | Hosts config may reference in `https` egress recipes. | `policy.rs:4-15` |
| `SECRET_HOST_BINDINGS` | Which secrets may bind to which hosts. | `policy.rs:23-31` |
| `ALLOWED_PACKAGES` | Package vocabulary for `nix-layered` image `contents`. | `policy.rs:34-41` |
| `DEFAULT_DENY_FALSE_ENTITLEMENT` | Workloads entitled to `default_deny = false`. | `policy.rs:45` |
| `GITHUB_HOSTS` | Shared by `github` recipe and `agent_base`. | `policy.rs:19` |

### 4.2 Config purity / closed vocabulary (ADR 0003)

Config is **declarative data only** — no arbitrary shell, no nix expressions,
no inline build scripts. All executable logic is named, versioned, reviewable
recipes in core. This is the Kustomize no-templating model combined with the
NixOS module system's bounded vocabulary.

**Closed vocabularies** (validated at `validate-config`, `validation.rs:80-91`):

| Category | Allowed values |
|---|---|
| Image recipes | `registry`, `nix-layered` |
| Binary/local_build recipes | `npm-build`, `bun-compile`, `pip-install`, `bun-install` |
| Egress recipes | `dns`, `litellm_proxy`, `github`, `agent_base`, `https` |
| Features | `create_tmp` |
| Packages (`contents`) | `cacert`, `busybox`, `fakeNss`, `nodejs_24`, `nmap`, `dnsutils` |

`baked_files` content is strings only. Escape hatch: new logic requires adding
a named recipe to core (reviewed, versioned) — the Terraform provider model.

---

## 5. Trust model (ADR 0014 + ADR 0020 Ruling 3)

Project (`./workestrate.toml`) and local (`./workestrate.local.toml`) layers
are **trust-gated**: they only load when the current working directory is
listed in `[[trusted_projects]]` in the registry (`loading.rs:335-384`).

- `workestrate config trust <dir>` — add a project to the trusted list.
- `workestrate config untrust <dir>` — remove trust.
- `WORKESTRATE_NO_PROJECT_CONFIG` env var — disables project+local layer
  loading entirely (escape hatch for running in untrusted directories).
- **Bootstrap exception:** when no registry exists yet (fresh install), the
  project and local layers load without a trust check
  (`loading.rs:349-352`, `loading.rs:378-381`).

**Operator-trust escape valves** (ADR 0020 Ruling 3, documented not hardened):

- `workestrate run -- <cmd>` loads all secrets and exec's the command — the
  operator invoking `run` already has host shell.
- `WORKESTRATE_CONFIG_DIR` bypasses discovery as a single layer — whoever
  controls the environment controls the process (the Unix model).

The auto-discovered `workestrate.local.toml` IS hardened (trust-gated like
`workestrate.toml`) because a hostile `git clone` + `cd` could otherwise
compromise the host without explicit operator action.

---

## 6. Path template variables

Mount `host` paths support template tokens resolved by
`resolve_mount_host_template` (`microsandbox/workload/validate.rs:110-140`),
in precedence order:

1. `${<env_override>}` — the workload's `local_build.env_override` var name
   (checked first so a custom override wins).
2. `${WORKESTRATE_<NAME>_BUILD}` — the default convention (`NAME` uppercased,
   `-` → `_`). Resolves to the build path.
3. `${CWD}` / `${CWD}/...` — current working directory (unchanged).

Anything else is returned unchanged (literal). The `${MSB_HOME}` token (used
in the live config for sandbox-internal paths like
`${MSB_HOME}/sandboxes/litellm/logs`) is passed through to the microsandbox
runtime, which resolves it.

The `workspaces/` prefix on `seed_files.target` paths is resolved relative to
the state dir (`config.rs:150`).

---

## 7. Planned extensions (NOT-YET-IMPLEMENTED — requirements pin)

These are the requirements pins for extensions that are not yet implemented.
They are recorded here so the contract is complete; implementation is tracked
separately.

### 7.1 Mount filtering / shadowing (Track A)

**Status:** NOT-YET-IMPLEMENTED. Full spec at
`06-improvements/01-mount-filtering-shadowing.md`.

Requirements:

- `exclude = ["glob", ...]` on `MountPlan` — glob patterns to exclude from a
  bind mount.
- `[[mounts.shadow]]` entries: `{ path: String, kind: "empty-file" | "empty-dir" | "tmpfs" }`
  — shadow a host path with an empty file, empty dir, or tmpfs.
- `allow_sensitive = bool` — trust-gated: only settable from trusted layers
  (per ADR 0014 / ADR 0020). Untrusted layers cannot set it.
- New `globset` dependency for glob matching.
- `#[serde(default)]` on all new fields so existing configs AND the golden
  plan format survive via conditional additive rendering (no breaking change
  to existing configs).
- `policy.rs` gains `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` applied **POST-MERGE at
  plan time** (monotonic deny, ADR 0005-conformant — no merge-engine change
  needed; the deny is enforced after merge, like the egress allowlist).

This design preserves the ADR 0005 invariant: a less-trusted layer cannot
weaken security. The sensitive-mount exclude patterns are a monotonic deny
applied after the merge engine produces the effective config, so the merge
engine itself is unchanged.

---

## 8. Validation surface

The CLI exposes these config-validation commands (`main.rs:374-396`):

| Command | Purpose | Env marker |
|---|---|---|
| `workestrate validate-config` | Validate the merged config against all invariants (schema, recipes, policy, trust). | `verifiable-here` |
| `workestrate generate-schema` | Print the JSON Schema for `workestrate.toml` to stdout (schemars-derived from `ConfigFile`). Committed at `schemas/workestrate.schema.json` (ADR 0021 §8). | `verifiable-here` |
| `workestrate secrets-schema` | Print the JSON Schema for the secrets definitions. | `verifiable-here` |
| `workestrate generate-env-example` | Generate an example `.env` file from the secret definitions. | `verifiable-here` |

**Schema drift guard** (ADR 0021 §8): a CI guard regenerates the schema in a
temp file and diffs against the committed copy. Drift fails `just verify`.
This catches the class where a `serde` struct change ships without a schema
update.

**taplo `#:schema` editor integration** (ADR 0021 §9): config repos can add
a top-level `#:schema` comment pointer to get real-time editor validation:

```toml
#:schema https://raw.githubusercontent.com/georgrybski/ai-workbench/main/schemas/workestrate.schema.json
schema_version = 1
```

---

## 9. CLI authoring contract (DEFERRED)

The future toml_edit-based CLI config authoring tool is **DEFERRED** (see
`06-improvements/04-cli-config-authoring.md`). This section records the
contract that tool must preserve so that no config semantics need
re-litigation later.

### 9.1 Lossless round-trip

The tool must use `toml_edit` (not `toml`/serde round-trip) to preserve:

- **Comments** — all `#` comments.
- **Ordering** — key/table declaration order.
- **Formatting** — whitespace, inline vs array-of-tables style, line breaks.

A serde round-trip would normalize formatting and drop comments, which is
unacceptable for a config file that humans read and edit.

### 9.2 `--layer` targeting

Edits must target a specific layer (which file/layer an edit lands in):

- `--layer reference` → `config.reference/workestrate.toml`
- `--layer <repo-name>` → `<store>/repos/<name>/workestrate.toml`
- `--layer global` → `$WORKESTRATE_HOME/overrides.toml` `[global]`
- `--layer config:<name>` → `$WORKESTRATE_HOME/overrides.toml` `[configs.<name>]`
- `--layer project` → `./workestrate.toml`
- `--layer local` → `./workestrate.local.toml`

The tool must respect the trust model: edits to `project`/`local` layers
require the directory to be trusted (or warn).

### 9.3 Validate after every mutation

After every edit, the tool must run `workestrate validate-config` (or the
in-process equivalent) to catch schema/policy violations immediately. A
config that parses but violates a policy invariant (e.g. non-allowlisted
egress host) must be rejected before the edit is committed to disk.

### 9.4 Envisioned commands (pointer only — DEFERRED)

The full command surface for the CLI authoring tool is specified in
`06-improvements/04-cli-config-authoring.md`. This document does not define
those commands; it defines the contract (schema, merge, policy, trust) that
they must target.

---

## 10. Running example: the personal deployment

The real current deployment lives at
`.workestrate/repos/personal/workestrate.toml` and defines **5 workloads**
and **8 secret definitions**:

**Workloads:**

| Name | Kind | Image recipe | Notes |
|---|---|---|---|
| `litellm` | service | `registry` (`ghcr.io/berriai/litellm:v1.89.4`) | Proxy; `default_deny = true`; egress to providers. |
| `pi` | agent | `nix-layered` (`workestrator-pi`) | `bun-compile` binary; `agent_base` egress; deny `.pi.dev`. |
| `odysseus` | service | `registry` (`python:3.12-slim`) | `pip-install` local_build; HF egress. |
| `opencode` | agent | `registry` (`node:24-bookworm-slim`) | `bun-install` local_build; `agent_base` egress. |
| `tempest` | agent | `nix-layered` (`tempest`) | `npm-build` binary; `default_deny = false` (entitled). |

**Secret definitions (8):**

`LITELLM_MASTER_KEY`, `LITELLM_AUTH` (alias → `OPENAI_API_KEY`),
`OPENROUTER_API_KEY`, `KIMI_CODE_API_KEY`, `NEURALWATT_API_KEY`,
`MINIMAX_CODING_API_KEY`, `GITHUB_TOKEN`, `ODYSSEUS_ADMIN_PASSWORD`.

This deployment exercises every config surface: both image recipes, three
build recipes, the alias pattern, `default_deny = false` entitlement
(tempest), `secret_env`, `env` with secret refs, mounts with `${CWD}` and
`${WORKESTRATE_*_BUILD}` templates, `seed_files`, `local_build`, `deny`
rules, `ingress`, and `https` egress with multiple hosts.

---

## Appendix: ADR cross-reference

| ADR | Topic | Relevance |
|---|---|---|
| 0002 | TOML config format | `workestrate.toml` is TOML (Nix `builtins.fromTOML` compat). |
| 0003 | Config purity + closed vocabulary | No arbitrary shell; closed recipe vocabulary. |
| 0004 | Security allowlist in policy.rs | `ALLOWED_EGRESS_HOSTS`, `SECRET_HOST_BINDINGS`, etc. |
| 0005 | Security-aware merge | Monotonic-true `default_deny`, additive deny/egress/secret_env. |
| 0014 | Trust-gated project config | `[[trusted_projects]]`; `config trust/untrust`. |
| 0019 | Contexts + user-global overrides | `[contexts.*]`, `overrides.toml`, instance namespacing. |
| 0020 | Review adjudications | env union-by-name; entitlement before monotonic-true; local.toml trust-gated. |
| 0021 | Instance lifecycle model | `generate-schema`, committed schema, `#:schema` editor integration. |
| 0023 | Single tool home | `$WORKESTRATE_HOME` flat layout; `home_version`. |
