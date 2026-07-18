# 70 — Open Items

## Pending user defaults

These are decisions that genuinely need user input. Recommended defaults are
provided.

### 1. Personal config repo git remote

**Needed for**: Phase 1, step M.9 (create personal config repo).
**Recommended**: `git@github.com:georgrybski/workestrator-config-personal.git`
(private). Matches the existing fork pattern (`flake.nix:7-25`).
**User may prefer**: a different remote, or local-only initially (no remote
until ready to distribute).

### 2. Dotfiles repo for `workestrate init`

**Needed for**: Phase 1, step 1.4 (`workestrate init <url>`).
**Recommended**: add `~/.config/workestrate/config.toml` to the user's
existing dotfiles repo (chezmoi/yadm/stow). If no dotfiles repo exists,
`~/.config/workestrate/` can be a standalone git repo.
**User may prefer**: a different dotfiles tool or location.

### 3. Default layer-set name

**Needed for**: Phase 1 (registry `layers` array).
**Recommended**: `personal` (matches the default config repo name).
**User may prefer**: `default` or `home`.

### 4. State dir location

**Needed for**: Phase 1, step 1.2 (XDG path resolution).
**Recommended**: `~/.local/state/workestrate/` (XDG State spec).
**User may prefer**: `~/.local/share/workestrate/state/` (single tree; XDG
spec separates share from state, but some tools co-locate them).

## KVM gates

These steps require a host with KVM (`/dev/kvm`). This container has no KVM.

| Gate | Phase | Step | What it verifies |
|---|---|---|---|
| Runtime sandbox execution | 1 | 1.11 | `workestrate litellm up` + `workestrate pi exec` succeed on KVM host |
| Runtime sandbox execution (M-step) | 1 | M.12 | Same as 1.11 |

All other gates are verifiable in this container (cargo, TOML, golden files)
or on a host with nix (HOST-NIX gates).

## Deferred items

These are intentionally deferred to later phases or until a trigger condition
is met.

### Contexts (named layer-sets)

**Deferred until**: 3+ layers exist (ADR 0013). Phase 3 ships a single
ordered `layers` array. Named contexts (`[[contexts.<name>]]` with
`--context <name>` flag) are a future enhancement.

### Team machinery

**Deferred until**: a second consumer (team member) exists. Phase 3 builds the
layering engine with fixture-repo tests (base/team/personal). The actual team
config repo, multi-recipient SOPS with a real team key, and team CI are
deferred.

### Per-workload packs

**Rejected** (ADR 0015). Workload definitions are ~20-line TOML entries in
config repos. The recipe vocabulary (in core) is the distribution unit.

## Residual risks

### Vocabulary creep

The closed vocabulary (egress recipes, build recipes, image features, package
names) creates pressure to add new entries for every new workload need. If the
vocabulary grows without governance, it becomes a dumping ground of one-off
recipes that are effectively config-as-code.

**Mitigation**: vocabulary entries are reviewed like core code (they ARE core
code); each new recipe must justify why it can't be expressed with existing
vocabulary + parameters; periodic vocabulary audits; documented governance
policy (inspired by NixOS module review process).

### Schema drift

Core engine v0.6 changes the `SandboxPlan` schema; config repo pinned to v0.5
breaks.

**Mitigation**: `schema_version` field in `workestrate.toml`; `#[serde(default)]`
for forward-compat; clear error messages on mismatch; copier update for template
sync; CI tests config against latest core.

### lib API drift (Phase 2)

Core exports `lib.*` (recipes, vocabulary, buildWorkloadImage). If the lib API
changes, config-repo-flakes break.

**Mitigation**: lib API versioning; CI in config repos runs against pinned core
version; `nix flake update` in config repo updates core pin.

## Verification caveats

### No nix in this container

This container has no nix installed. All HOST-NIX gates (Phase 0b, Phase 2,
some Phase 0a/1 steps) cannot be runtime-verified here. The nix code can be
written and reviewed but not executed. Claims about nix behavior
(`builtins.fromTOML`, flake source filtering, pure-eval invisibility) are based
on documented Nix semantics, not runtime verification in this session.

### No KVM in this container

This container has no `/dev/kvm`. All HOST-KVM gates (Phase 1, step 1.11/M.12)
cannot be runtime-verified here. `workestrate <name> up`/`exec` cannot be
runtime-tested. Plan output (`workestrate <name> plan`) is verifiable; runtime
execution is not.

### External precedents live-verified

External pattern precedents (Kustomize, NixOS modules, devcontainer features,
SOPS, Flux, Terragrunt, chezmoi, kubectl, git, Claude Code, mise) were
live-verified via web research (searcher agent). URLs are cited in the
relevant ADRs:
- ADR 0003: Kustomize no-templating (https://github.com/kubernetes-sigs/kustomize/issues/2052),
  NixOS modules (https://nixos.org/manual/nixos/stable#sec-option-types),
  devcontainer features (https://devcontainers.github.io/implementors/features)
- ADR 0007: kubectl (https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/),
  git (https://git-scm.com/docs/git-config#FILES),
  Claude Code (https://code.claude.com/docs/en/settings#how-scopes-interact),
  mise (https://mise.jdx.dev/configuration.html#configuration-hierarchy),
  chezmoi (https://chezmoi.io/reference/commands/init/)
- SOPS multi-recipient: https://getsops.io/docs/usage/identities/key-groups
- Flux SOPS: https://fluxcd.io/flux/guides/mozilla-sops
- sops-nix: https://github.com/Mic92/sops-nix
- copier update: https://copier.readthedocs.io/en/stable/updating
