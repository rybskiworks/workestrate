# 30 — Security Model

## Threat model

workestrate is a **sandboxing tool**. Config defines security policy (egress
hosts, secret bindings, network deny rules). The trust boundary is: **config
is untrusted data; the compiled tool (core) is trusted.**

This is the same model as Terraform (HCL is untrusted; providers are trusted)
and Kustomize (overlays are untrusted; the base + strategic merge is the
contract). Config can come from arbitrary paths/repos; the allowlist is checked
against the loaded config's contents, not its provenance.

## Purity invariant

**Config repo = declarative data + SOPS-encrypted secrets + static files ONLY.**
All executable logic = named, versioned, reviewable recipes in core.

| Config may declare | Config may NOT declare |
|---|---|
| `image.recipe = "registry"` + `ref` string | Arbitrary nix expressions |
| `image.recipe = "nix-layered"` + `contents[]` from core vocabulary + `baked_files{}` + `features[]` from core vocabulary | `extraCommands` (arbitrary shell) |
| `image.binary.recipe = <named>` + parameters | Inline shell build scripts |
| `local_build.recipe = <named>` + parameters | Arbitrary build commands |
| `baked_files: [{ path, content }]` (string content only) | Baked files with executable content or templating that evaluates code |
| `seed_files: [{ source, target, only_if_missing }]` | Arbitrary file operations |
| `egress: [{ recipe, hosts? }]` (recipe from core vocabulary) | Custom egress rules not expressible as recipes |
| `secret_env: [{ secret }]` (secret from `secrets:` section) | Inline secret values |

**Escape hatch**: if a workload needs custom logic not expressible with the
named recipe vocabulary, a new named recipe is added to core (reviewed,
versioned). Config references it by name.

### Precedent (live-verified)

- **Kustomize**: no-templating/no-scripting is a stated **non-goal**
  (kubernetes-sigs/kustomize issue #2052, maintainer monopole: "template
  business… is a non-goal of kustomize"). This is the strongest purity
  precedent. [Source: https://github.com/kubernetes-sigs/kustomize/issues/2052]
- **NixOS modules**: bounded vocabulary of typed options
  (`mkOption { type = ...; }` with `types.enum`, `types.submodule`); users can
  only set declared keys. [Source: https://nixos.org/manual/nixos/stable#sec-option-types]
- **devcontainer features**: install scripts owned by the feature, not by user
  config; user config bounded to declared options.
  [Source: https://devcontainers.github.io/implementors/features]

## Closed vocabulary + vocabulary governance

### Vocabulary items

| Category | Items | Location |
|---|---|---|
| Egress recipes | `dns`, `litellm_proxy`, `github`, `agent_base`, `https` | Rust enum in `recipes.rs` |
| Build recipes | `npm-build`, `bun-compile`, `pip-install`, `bun-install` | Nix functions in `nix/lib/recipes/` |
| Image recipes | `registry`, `nix-layered` | Nix functions in `nix/lib/recipes/` |
| Features | `create_tmp` | `nix/lib/vocabulary.nix` |
| Packages | `cacert`, `busybox`, `fakeNss`, `nodejs_24`, `nmap`, `dnsutils` | `nix/lib/vocabulary.nix` |

### Governance

Vocabulary entries are reviewed like core code (they ARE core code). Each new
recipe must justify why it can't be expressed with existing vocabulary +
parameters. Periodic vocabulary audits. Documented governance policy (inspired
by NixOS module review process).

**Vocabulary-creep failure mode**: if the vocabulary grows without governance,
it becomes a dumping ground of one-off recipes that are effectively
config-as-code, defeating the purity principle. Mitigation: the review process
+ audits.

## policy.rs ceiling + per-recipe scoping

### Data structures (sketched)

```rust
// policy.rs

/// Core-defined allowlist of egress hosts. Config may only reference hosts
/// from this set. Enforced at validate-config, plan (fail-closed), and
/// runtime apply_plan_secrets.
pub const ALLOWED_EGRESS_HOSTS: &[&str] = &[
    "openrouter.ai", "api.kimi.com", "api.neuralwatt.com", "api.minimax.io",
    "github.com", "api.github.com",
    "huggingface.co", "cdn-lfs.huggingface.co", "cdn-lfs-us-1.huggingface.co",
    "host.microsandbox.internal",
];

/// Core-defined secret→host binding allowlist. Each secret may only bind
/// to listed hosts. Replaces the const SecretDefinition hosts field.
pub const SECRET_HOST_BINDINGS: &[(&str, &[&str])] = &[
    ("LITELLM_MASTER_KEY",        &["host.microsandbox.internal"]),
    ("OPENROUTER_API_KEY",        &["openrouter.ai"]),
    ("KIMI_CODE_API_KEY",         &["api.kimi.com"]),
    ("NEURALWATT_API_KEY",        &["api.neuralwatt.com"]),
    ("MINIMAX_CODING_API_KEY",    &["api.minimax.io"]),
    ("GITHUB_TOKEN",              &["github.com", "api.github.com"]),
    ("ODYSSEUS_ADMIN_PASSWORD",   &[]),  // internal, no egress binding
];

/// Core-defined package vocabulary for nix-layered images.
pub const ALLOWED_PACKAGES: &[&str] = &[
    "cacert", "busybox", "fakeNss", "nodejs_24", "nmap", "dnsutils",
];

/// Per-recipe allowed_hosts scoping: recipes that accept `hosts` parameter
/// validate against ALLOWED_EGRESS_HOSTS. Recipes without host params
/// (dns, litellm_proxy, github, agent_base) are fully fixed in core.
/// The `https` recipe takes `hosts: Vec<String>` validated against the allowlist.
```

### Per-recipe scoping

- `dns`, `litellm_proxy`, `github`, `agent_base`: no host parameters; fully
  fixed in core. Config references by name only.
- `https`: takes `hosts: Vec<String>`; each host validated against
  `ALLOWED_EGRESS_HOSTS`. If a host is not in the allowlist, `validate-config`
  fails with: "host 'evil.com' is not in the core egress allowlist."
- Each secret's bindable hosts are fixed in `SECRET_HOST_BINDINGS`. Config
  declares which secrets to use; core validates the binding.

## Enforcement points (three-layer defense)

| Point | When | What it checks | Failure behavior |
|---|---|---|---|
| `workestrate validate-config` | Pre-flight (user-invoked or CI) | All egress hosts against `ALLOWED_EGRESS_HOSTS`; all secret bindings against `SECRET_HOST_BINDINGS`; all package names against `ALLOWED_PACKAGES`; schema validity; cross-references (secret refs exist, mount sources exist, port conflicts) | Exits non-zero with clear error citing the allowlist |
| `workestrate workload plan <name>` | Pre-flight (fail-closed) | Same checks as validate-config, plus: required secrets present (or placeholder); mount sources exist | Fails with error citing the violated invariant |
| `apply_plan_secrets` (`runtime.rs:107-145`) | Runtime (before sandbox start) | Each secret's `allowed_hosts` against `SECRET_HOST_BINDINGS`; `reject_if_placeholder` (`runtime.rs:10-22`); required secrets non-empty | Refuses to start sandbox; clear error |

## Trust gating — `[trusted_projects]`

### Mechanism

Project-layer config (`./workestrate.toml` in cwd) is **trust-gated**. Only
projects listed in `[trusted_projects]` in the registry have their
`./workestrate.toml` loaded.

```bash
workestrate config trust /home/node/Development/my-project
# Adds to [[trusted_projects]] in ~/.config/workestrate/config.toml
```

`--no-project-config` flag disables project-layer loading entirely (escape
hatch for running in untrusted directories).

### Malicious-project scenario

An attacker creates a project directory with a `workestrate.toml` that widens
egress or rebinds secrets. The user `cd`s into it and runs `workestrate workload
exec pi`.

**Without trust gating**: the malicious config is loaded as a project layer.
**Mitigation**: the project layer is bounded by the policy.rs ceiling (same as
all layers). The attacker cannot exceed `ALLOWED_EGRESS_HOSTS` or
`SECRET_HOST_BINDINGS`. Additionally, trust gating means the malicious
`./workestrate.toml` is **not loaded at all** unless the user explicitly
trusted the project directory. This is the `direnv allow` model.

## Malicious-config-repo scenario

An attacker publishes a config repo with a `workestrate.toml` that widens
egress. The user adds it via `workestrate config add <url> team`.

**Bounded by**: the policy.rs ceiling. The attacker's config can reference
egress hosts, but only from `ALLOWED_EGRESS_HOSTS`. The attacker can rebind
secrets, but only to hosts in `SECRET_HOST_BINDINGS`. The attacker cannot
execute arbitrary code (purity invariant). The attacker cannot supply
`extraCommands` or nix expressions.

**Residual risk**: the attacker can add new egress hosts (from the allowlist)
that the user didn't intend. Mitigation: `validate-config` reports all egress
hosts; `plan --show-source` attributes each to its layer; the user reviews
before running `up`/`exec`.

## Monotonic default_deny + entitlement

`default_deny` is **monotonic-true**: if any layer sets `default_deny = true`,
the merged result is `true`. A less-trusted layer cannot set `default_deny =
false` to weaken a more-trusted layer's policy.

**Entitlement**: core defines which workloads are entitled to
`default_deny = false`. Currently only `tempest` (offensive-security tool,
`tempest.rs:77`). The entitlement is a core const:

```rust
/// Core-defined entitlement: workloads allowed to use default_deny = false.
/// All other workloads are forced to default_deny = true regardless of config.
pub const DEFAULT_DENY_FALSE_ENTITLEMENT: &[&str] = &["tempest"];
```

If a config layer sets `default_deny = false` for a workload not in this list,
`validate-config` fails: "workload 'pi' is not entitled to default_deny=false."

## Additive deny/egress unions

- `deny_rules`: additive-union across layers. A layer can add deny rules but
  cannot remove them. A less-trusted layer cannot un-deny a domain that a
  more-trusted layer denied.
- `egress_rules`: additive-union across layers, within the `policy.rs` ceiling
  + per-recipe `allowed_hosts()` scoping. A layer can add egress rules (from
  the allowlist) but cannot remove them.

## Nix-store boundary

### What nix may read

Nix (the parent flake, devshell, `nix build`) reads **ONLY** core-owned,
tracked, non-secret config:
- `config.reference/workestrate.toml` (tracked, sanitized, placeholder secrets)
- `nix/lib/` recipe functions, vocabulary, config reader

### What nix never reads

- User config repos (`~/.local/share/workestrate/repos/<name>/`) — these are
  runtime-only, consumed by the Rust CLI, never by nix eval.
- `.env.enc` (encrypted secrets) — never in nix store.
- `.sops.yaml` (public recipient keys + path rules) — metadata, but not read
  by nix.
- `~/.local/state/workestrate/` (runtime state) — not config.

### Pure-eval invisibility (confirmed by construction)

Nix flakes in pure evaluation copy only git-tracked files to the store.
User config repos are at `~/.local/share/workestrate/repos/` (outside the
flake tree entirely). Even if they were inside the tree as gitignored paths,
they would be invisible to pure eval. **The nix-store-leak concern is
neutralized by construction**: nix simply cannot ingest user config at eval
time.

## Git-history warning for stripped secrets

When the ai-workbench repo is stripped of `.env.enc` and `.sops.yaml` (Phase 1,
step M.10), the **git history still contains them**. The encrypted `.env.enc`
is safe (encrypted), but `.sops.yaml` contains the age recipient (a public
key — safe to leak). However, if any plaintext secrets were ever committed
accidentally, they remain in history.

**Mitigation**: `git filter-repo` or BFG Repo-Cleaner to purge history. This
is optional and destructive (rewrites history). For the current single-user
repo, it's likely unnecessary. Document the risk in the migration process.

## Fail-closed behaviors

| Scenario | Behavior |
|---|---|
| No config repos registered (fresh install) | Falls back to `config.reference/` (placeholder secrets). `plan`/`check`/`validate-config` work. `up`/`exec` refuse (placeholder secrets rejected by `reject_if_placeholder`, `runtime.rs:10-22`). |
| Config repo not cloned | `workestrate check` reports `[MISSING] (optional)`. `plan` uses reference config. `up`/`exec` refuse. |
| Config references unknown egress host | `validate-config` fails: "host 'evil.com' not in allowlist." `plan` fails (fail-closed). |
| Config references unknown secret | `validate-config` fails: "secret 'FOO' not defined in secrets: section." |
| Config sets `default_deny = false` for non-entitled workload | `validate-config` fails: "workload 'pi' not entitled to default_deny=false." |
| Required secret is placeholder | `apply_plan_secrets` (`runtime.rs:107-145`) refuses: "secret 'LITELLM_MASTER_KEY' is set to placeholder." |
| Required secret is empty | `apply_plan_secrets` refuses: "required secret 'LITELLM_MASTER_KEY' is set but empty." |
| Untrusted project has `./workestrate.toml` | Ignored (not in `[trusted_projects]`). No error (silent skip). |

## Current vs migrated trust boundaries

| Aspect | Current (M1) | Migrated |
|---|---|---|
| Config source | Rust code (compiled) | TOML files (data) |
| Trust boundary | Compile-time (Rust review) | Runtime (policy.rs allowlist) |
| Egress hosts | Hardcoded in `workloads/*.rs` | Declared in config, validated against `ALLOWED_EGRESS_HOSTS` |
| Secret bindings | `secrets.rs` const | Declared in config, validated against `SECRET_HOST_BINDINGS` |
| Network policy | `plan.rs:270-305` helpers | `recipes.rs` enum, expanded by core |
| Image contents | `pi-image.nix`, `tempest-image.nix` | `nix/lib/vocabulary.nix` + `buildWorkloadImage` |
| Extra shell in images | `extraCommands` in nix files | `baked_files` + `features` (declarative, no arbitrary shell) |
| Config repo trust | N/A (no config repos) | Bounded by allowlist; provenance irrelevant to enforcement |
