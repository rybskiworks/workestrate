# 30 — Security Model

## Threat model

workestrate is a **sandboxing tool**. Config defines security policy (egress
hosts, secret bindings, network deny rules). The trust boundary is: **config
is untrusted data; the compiled tool (core) is trusted.**

This is the same model as Terraform (HCL is untrusted; providers are trusted)
and Kustomize (overlays are untrusted; the base + strategic merge is the
contract). Config can come from arbitrary paths/repos; the core ceilings are checked
against the loaded config's contents, not its provenance.

The JSON Schema files (`schemas/workestrate.schema.json` +
`schemas/workestrate-workload.schema.json`) are derived artifacts for
editor/tombi UX, NOT a trust boundary — the authoritative checker is the
compiled tool (`validate-config` enforces `deny_unknown_fields` + the
`schema_version` gate regardless of what any editor-side schema says);
`workestrate schemas update` only refreshes derived copies and cannot weaken
validation.

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
| `seed_files: [{ source\|glob, target, only_if_missing, template }]` (template = true renders `${VAR}` from the guest-visible env view: host-bound → `$MSB_<binding key>` placeholder, guest-bound → real value, defined-but-unbound secret → hard error; no process env) | Arbitrary file operations |
| `[policy.egress]` / `[policy.ingress]` rule tables (host/domain scopes, `final` seals) | Rules outside the schema'd rule vocabulary; removal/weakening of a sealed rule |
| `env` map bindings: `KEY = true` (host-bound placeholder), `{ secret = "ID" }` (rename), `{ bound = "guest" }` (real value, verifier opt-in) | Inline secret values |

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

## Core ceilings

What config cannot exceed:

- **Package allowlist**: `policy::ALLOWED_PACKAGES` (the one surviving core
  const allowlist) gates package names in nix-layered images, enforced at
  `validate-config`.
- **Closed recipe vocabulary**: image recipes (`registry`, `nix-layered`),
  build recipes (`npm-build`, `bun-compile`, `pip-install`, `bun-install`),
  and features are named core code; config references them by name only.
- **Hierarchical policy rules**: egress/ingress control is the
  `policy.egress` / `policy.ingress` ladder (specificity + `final` seals).
  The former core egress-host allowlist (`ALLOWED_EGRESS_HOSTS`) and the
  egress-recipe vocabulary were removed (ADR 0035) — an unsealed rule can
  be added by any layer the trust model loads, but a `final` seal in a
  more-trusted rung cannot be undone by less-trusted layers; relaxed
  defaults surface as loud plan NOTEs (see "Relaxed network defaults"
  below).
- **Secret bindings**: `apply_plan_secrets` substitutes the real value only
  for each secret's `allowed_hosts`; placeholder/required checks gate the
  rest. The definition-side `allowed_hosts` list is the credential policy —
  since ADR 0035 it is no longer validated against a hardcoded core
  binding allowlist (the former `SECRET_HOST_BINDINGS` const is gone), so
  it is reviewable in the capsule, not core-gated.

Secret definitions are a pure catalog of intrinsic credential properties;
exposure is declared per workload at the binding site via `bound` on the env
binding. `bound = "guest"` injects the REAL secret value into the guest
(verifier opt-in — for workloads like litellm/odysseus that verify their
callers); the default `bound = "host"` injects a placeholder (the guest sees
a non-secret marker; the host-side rewrite substitutes the real value only
for `allowed_hosts`).

## Enforcement points (three-layer defense)

| Point | When | What it checks | Failure behavior |
|---|---|---|---|
| `workestrate validate-config` | Pre-flight (user-invoked or CI) | All package names against `ALLOWED_PACKAGES`; policy rule well-formedness; schema validity; cross-references (secret refs exist, mount sources exist, port conflicts) | Exits non-zero with clear error citing the allowlist |
| `workestrate workload plan <name>` | Pre-flight (fail-closed) | Same checks as validate-config, plus: required secrets present (or placeholder); mount sources exist; seed sources exist; per-direction relaxed-default NOTEs | Fails with error citing the violated invariant |

> *Implemented 2026-08-03 (commit `c6a6b47`): the plan-time mount/seed existence preflight is now real — a missing read-only mount source or seed source fails at `plan` before any KVM/runtime work (the failure-1 doubling signal). `validate-config` runs the same check warn-only (synthetic/reference configs may legitimately lack the files). See `microsandbox::mounts::preflight_existence`.*
| `apply_plan_secrets` (`microsandbox/runtime/run.rs`) | Runtime (before sandbox start) | Template resolution against the merged secrets map; each secret's `allowed_hosts` drive per-host substitution entries (placeholder forwarded elsewhere); `reject_if_placeholder`; required secrets non-empty | Refuses to start sandbox; clear error |

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
**Mitigation**: the project layer is bounded by the same core ceilings as
every layer (package allowlist, closed recipe vocabulary), and it cannot undo
a home `final` policy seal. Additionally, trust gating means the malicious
`./workestrate.toml` is **not loaded at all** unless the user explicitly
trusted the project directory. This is the `direnv allow` model.

## Malicious-config-repo scenario

An attacker publishes a config repo with a `workestrate.toml` that widens
egress. The user adds it via `workestrate config add <url> team`.

**Bounded by**: the core ceilings. The attacker's config can add policy
rules, but cannot exceed the package allowlist, cannot use unlisted recipes,
cannot undo a home `final` seal, and cannot execute arbitrary code (purity
invariant — no `extraCommands`, no nix expressions).

**Residual risk**: the attacker can add unsealed egress rules (the core
egress-host allowlist is gone) or widen a secret's `allowed_hosts` that the
user didn't intend. Mitigation: `plan` output + provenance attribute every
rule to its layer; `just` review before `up`/`exec`; home `final` seals
veto the directions that matter.

## Relaxed network defaults — entitlements removed; seals + review are the veto

The `entitlements` mechanism was **removed 2026-09-04** (pre-release): the
vocabulary, the core const, and the validate/merge gate are gone. Today:

- `network.defaults.egress = "allow"` / `network.defaults.ingress = "allow"`
  is an explicit declaration that **stands alone** — no entitlement key is
  consulted; there is no core allowlist of entitled workloads.
- Setting the removed `entitlements` key in any config layer is a hard parse
  error: `deny_unknown_fields` rejects it as an unknown field.
- Fail-closed by convention is unchanged: **absent = deny**.

The veto against a hostile layer relaxing a default is not a declaration
gate; it is:

1. **Home `final` seals on the policy rules** (`policy.egress` /
   `policy.ingress` ladder): a sealed deny in a more-trusted rung cannot be
   undone by any less-trusted layer, whatever that layer declares in
   `network.defaults`.
2. **Review surface**: `workestrate workload plan` prints a loud per-workload
   NOTE for every relaxed direction
   (`NOTE: workload '<name>' runs relaxed defaults (egress=allow)`) — the
   grep-able review surface that replaced the old magic-word check.

## Hierarchical policy rules (rule surface)

The rule surface is the `policy.egress` / `policy.ingress` hierarchy
(specificity ladder + `final` seals). The former flat `deny_rules` /
`egress_rules` additive unions are gone — configs that still use those
fields hard-error at parse with ADR-citing messages. A sealed (`final`) rule
cannot be weakened by a less-trusted layer; unsealed rules resolve through
the ladder.

## Nix-store boundary

### What nix may read

Nix (the parent flake, devshell, `nix build`) reads **ONLY** core-owned,
tracked, non-secret config:
- `config.reference/workestrate.toml` (tracked, sanitized, placeholder secrets)
- `nix/lib/` recipe functions, vocabulary, config reader

### What nix never reads

- User config repos (`$WORKESTRATE_HOME/config-repos/<name>/`, ADR
  0023/0024) — these are runtime-only, consumed by the Rust CLI, never by
  nix eval.
- `.env.enc` (encrypted secrets) — never in nix store.
- `.sops.yaml` (public recipient keys + path rules) — metadata, but not read
  by nix.
- `~/.local/state/workestrate/` (runtime state) — not config.

### Pure-eval invisibility (confirmed by construction)

Nix flakes in pure evaluation copy only git-tracked files to the store.
User config repos are at `$WORKESTRATE_HOME/config-repos/` (outside the
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
| Config uses the removed `network.egress` / `network.deny` / `network.ingress` fields | Parse fails with an ADR-citing message pointing to the hierarchical `policy.*` surface. |
| Config references unknown secret | `validate-config` fails: "secret 'FOO' not defined in secrets: section." |
| Config sets the removed `entitlements = [...]` key | Parse fails: "unknown field `entitlements`" (`deny_unknown_fields`) — mechanism removed 2026-09-04. |
| Config sets `network.defaults.egress`/`ingress = "allow"` | Accepted — no declaration gate; `plan` prints a relaxed-defaults NOTE per direction; home `final` policy seals still veto via the ladder. |
| Required secret is placeholder | `apply_plan_secrets` (`microsandbox/runtime/run.rs`) refuses: "secret 'LITELLM_MASTER_KEY' is set to placeholder." |
| Required secret is empty | `apply_plan_secrets` (`microsandbox/runtime/run.rs`) refuses: "required secret 'LITELLM_MASTER_KEY' is set but empty." |
| Untrusted project has `./workestrate.toml` | Ignored (not in `[trusted_projects]`). No error (silent skip). |

## Current vs migrated trust boundaries

| Aspect | Current (M1) | Migrated |
|---|---|---|
| Config source | Rust code (compiled) | TOML files (data) |
| Trust boundary | Compile-time (Rust review) | Runtime (policy.rs allowlist) |
| Egress hosts | Hardcoded in `workloads/*.rs` | Declared as hierarchical `policy.egress`/`policy.ingress` rules + `final` seals (core host allowlist removed) |
| Secret bindings | `secrets.rs` const | Declared in config (`allowed_hosts`); core binding allowlist removed — review + runtime host-scoped substitution |
| Network policy | `plan.rs` helpers | Hierarchical policy engine in `policy/` + `plan` (recipe enum removed) |
| Image contents | `pi-image.nix`, `tempest-image.nix` | `nix/lib/vocabulary.nix` + `buildWorkloadImage` |
| Extra shell in images | `extraCommands` in nix files | `baked_files` + `features` (declarative, no arbitrary shell) |
| Config repo trust | N/A (no config repos) | Bounded by allowlist; provenance irrelevant to enforcement |
