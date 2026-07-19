# 70 — Open Items

## Review findings (2026-07)

A five-way parallel investigation (Rust core / Nix / security / docs / tooling)
plus main-lead synthesis reviewed `migration/tool-model` @ `12e89b6`. The
remediation plan is documented in
[`80-remediation-plan.md`](80-remediation-plan.md); the four design-tension
adjudications are recorded in [ADR 0020](50-decisions/0020-review-adjudications.md).

**Per-WP status (WP1-WP5 IMPLEMENTED; merge-gate MET):**

| WP | Severity | Closes | Status | Commit |
|---|---|---|---|---|
| WP1 — Trust-boundary and path validation | FIX-NOW | A1, A2, C2, C3, C4, A20=C14, A18=C8 | DONE | `279015b` |
| WP2 — Purity and Nix image correctness | FIX-NOW | C1, B1, B2, B14 | DONE (HOST-NIX gated) | `73cfd53` |
| WP3 — Policy enforcement fix (entitlement order) | FIX-NOW | A4 | DONE | `05b6bb7` |
| WP4 — Spec/code reconciliation + CI guard | FIX-NOW | D1 | DONE | `89d1658` |
| WP5 — New-user journey unblock (`workestrate check`) | FIX-NOW | E1 | DONE (Decision 1 = standalone) | `973bff3` |
| WP6 — Schema and semantic fixes | FIX-SOON | A3, A5, A6, C9, C10, E2 | queued | — |
| WP7 — Trust-model docs and escape-hatch warnings | FIX-SOON | C11, C12, D7, D11, D12, D13 | queued | — |
| WP8 — Setup-secrets alignment + missing commands | FIX-SOON | A7, E4, E5, E6, E7, E8, C13 | queued | — |
| WP9 — Nix image completeness | FIX-SOON | B3, B4, B5, B6, B10 | queued | — |
| WP10 — Provenance, TOCTOU, registry robustness | FIX-SOON | A9, A17=C6, A11, A12, A16, A24 | queued | — |
| WP11 — Production-path test coverage | FIX-SOON | A21 | queued | — |
| WP12 — Backlog sweep | BACKLOG | see WP12 item list | trickle | — |

Plus post-implementation hardening: `2498f6a` hardened the `just litellm-check`
recipe so it works in containers without the devshell's PyYAML (closes the
first independent-verification condition).

**Merge-readiness gate:** MET. WP1-WP5 implemented and green; the
trust-boundary, entitlement, and spec-examples regression tests land and pass;
no FIX-NOW item remains open; the holistic trust-model README statement is
deferred to WP7 (recorded in ADR 0020, referenced from README — does not
block merge). Independent verification ruled MET-WITH-CONDITIONS; both
conditions closed (litellm-check portability at `2498f6a`, docs flip in this
update).

### Items the review found already-resolved

- **D8** (40-migration-process.md step 0b.6 status inaccuracy): FIXED in this
  update cycle — the misleading "Step 0b.6" sub-section has been relabeled,
  the blanket "IMPLEMENTED" Status line corrected to "PARTIALLY IMPLEMENTED",
  and ADR 0011 (vendor->git-fork) is now explicitly deferred here.
- **D7** (ADR 0013 not marked superseded by ADR 0019): scheduled for WP7.

### Verification environment fixes (post-WP5)

- **litellm-check recipe portability** (`2498f6a`): the prior recipe was a
  bare `python3 check_config.py` invocation, which failed in containers
  whose python3 lacks PyYAML ("PyYAML is required but not installed", exit
  2). The new recipe is a shebang-form bash block that tries direct python3
  first, falls back to `nix develop -c python3` if PyYAML is missing, and
  prints actionable install guidance if neither path is available. The
  fallback matches the existing pattern used by `setup-secrets`,
  `validate-secrets`, and `load-images`.

### New deferred items the review surfaced (not in WP1-WP11)

The investigators surfaced the following items that are NOT addressed by any
work package. They are deferred to backlog (per `80-remediation-plan.md`'s
"OUT OF SCOPE" list) and tracked here.

- **E16 backup/restore story** — `workestrate backup`/`restore` over the
  registry + state dir. No data-loss risk in current single-operator model.
- **E12 dynamic shell completions** — `clap_complete` integration. UX polish.
- **E14 `workestrate uninstall`** — inverse of `init`. Useful but not
  load-bearing; manual `rm` is acceptable.
- **E13 `workestrate plan --all`** — convenience; the per-workload form works
  today.
- **E17 `workestrate config resolve --dump`** — whole-config resolution dump
  for debugging. Referenced in WP7's README rewrite as a future aid.
- **E18 multi-context batch** — explicitly deferred by ADR 0019; no current
  use case. Confirmed deferred here.
- **B8/B9/B11/B13** — Nix pure-eval and recipe polish; no acute impact on
  headline agents (which use the per-agent `.nix` files, not the recipes,
  until WP9 lands).
- **A22 broad edge-test sweep** — WP11 covers the production-path gap;
  broader edge tests land opportunistically with each bug fix.

### ADR 0011 (vendor -> git-fork) — deferred to backlog

The review confirmed (finding D8) that Phase 0b step 0b.6 (microsandbox
vendor symlink -> git-fork dependency, ADR 0011) is NOT implemented despite
the prior status line claiming Phase 0b IMPLEMENTED. The vendor symlink still
exists in `nix/packages/agentctl.nix`. This is not blocking the cargo-verifiable
or runtime path (the vendored crate builds) but should land before any pure-
eval Nix build claim. Deferred to backlog; ADR 0011 stays Accepted.

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

### Contexts (named layer-sets) — RESOLVED (ADR 0019)

**Status**: RESOLVED. Implemented in Phase 3.5 (ADR 0019). Registry gains
`[contexts.<name>] layers = [...]`. Selection: `--context` flag >
`WORKESTRATE_CONTEXT` env > `[settings] default_context` > bare-layers
backward-compat.

### Team machinery

**Deferred until**: a second consumer (team member) exists. Phase 3 builds the
layering engine with fixture-repo tests (base/team/personal). The actual team
config repo, multi-recipient SOPS with a real team key, and team CI are
deferred.

### .local siblings for overrides

**Deferred until**: a user needs project-local overrides that complement
the user-global `overrides.toml`. Currently, project-local overrides use
`./workestrate.local.toml` (trusted project layer). A matching
`./overrides.local.toml` for user-global-style overrides at the project
level is not implemented. The existing `workestrate.local.toml` covers
the common case.

### --port-offset escape hatch

**Deferred until**: a user needs to run the same workload from two
contexts simultaneously on the same host. Currently, port collisions
between contexts produce a hard error with remediation (change the port
in one config repo). A `--port-offset N` flag that shifts all host ports
by N would allow side-by-side execution, but complicates ingress rules
and is not needed for the current single-user, single-context-at-a-time
model.

### Multi-context batch

**Deferred until**: a use case requires merging multiple contexts in one
invocation. Currently, one invocation resolves exactly ONE context
(ADR 0019). Multi-context batch would complicate instance naming
(`<context>-<workload>` assumes one context) and port collision detection.
No current use case; revisit if orchestration scenarios emerge.

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

### Git history still contains stripped secrets

Root `.env.enc`, `.sops.yaml`, and the old `infra/litellm/` values were removed
from the working tree in the final strip-down, but they remain in the git
history of the `ai-workbench` repository. New clones receive the full history,
including these files. This is acceptable because `.env.enc` is encrypted and
`.sops.yaml` only contains a public age recipient, but it is a hygiene note:
future rotations of the age key or secrets should not assume the files were
permanently erased from git.

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

## Known limitations / future work

### Multi-layer secrets resolution — RESOLVED (ADR 0018)

**Status**: RESOLVED. Implemented per-key value merge across layers.

`secrets_loader.rs::load_secrets()` now loads `.env.enc` from EVERY resolved
layer in precedence order (process env < reference < registry layers in
declared order < trusted project), merging decrypted values per-key (later
layer wins). Per-key provenance is tracked for `plan --show-source`.

Per-repo secrets config (`secrets`, `secrets_file`, `age_key_file`) is
honored from the registry. `secrets = "none"` repos are skipped silently.

Failure semantics: undecryptable layer → WARNING + continue; required
secret unsatisfied → hard fail naming the secret + layers tried + remediation.

See ADR 0018 for the full decision and rationale.

## Environment note: container persistence

workestrate XDG state (registry, config repos, secrets, runtime state) lives
in a gitignored `.workestrate/` directory inside the repo. This directory is
bind-mountable for container persistence. The `.envrc` (direnv) and
`scripts/local-xdg.sh` export `XDG_CONFIG_HOME`, `XDG_DATA_HOME`,
`XDG_STATE_HOME`, and `SOPS_AGE_KEY_FILE` to point at `.workestrate/`
subdirectories.

**Warning**: `.workestrate/` contains the age private key. Never commit it.
The `.gitignore` entry is the guard.

### setup-secrets.sh per-repo override alignment

`setup-secrets.sh` currently targets the correct config repo directory via
`--config <name>` but hardcodes `SECRET_FILE=.env.enc` and uses
`SOPS_AGE_KEY_FILE` env only. The Rust `load_secrets()` honors per-repo
`secrets_file` and `age_key_file` overrides from the registry
(`[configs.<name>]` fields). The shell script should be updated to read
these overrides from the registry when `--config <name>` is used, so that
`setup-secrets --config team init` uses `team`'s `age_key_file` and
`secrets_file` automatically.

**Non-blocking**: users can set `SOPS_AGE_KEY_FILE` manually as a workaround.

### sops+age integration tests in CI

The 5 unit tests for secrets layering (`merge_secrets_later_layer_wins_per_key`,
`process_env_lowest_precedence`, `required_secret_unsatisfied_error_message`,
`missing_env_enc_produces_required_secret_error`,
`resolve_secrets_layers_uses_config_dir_override`) test the merge logic
without invoking the `sops` binary. The actual sops decrypt/encrypt
behavior (per-repo `age_key_file`, undecryptable layer warning, multi-layer
`.env.enc` loading) was verified empirically via `nix shell nixpkgs#sops
nixpkgs#age` but is not covered by `cargo test`. A CI step that runs
sops+age integration tests (similar to `scripts/validate-secrets-workflow.sh`)
should be added to catch regressions in the sops integration.

**Non-blocking**: the merge logic is tested; sops integration is stable
and unlikely to regress without code changes to `decrypt_layer()`.
