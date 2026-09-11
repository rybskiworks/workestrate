---
name: workflow-nix-hardening-01-scope
description: |
  Use only for the scope phase of the Nix hardening workflow. Assess the
  current flake config against the hardening baseline and identify gaps tied to
  verbatim config keys. Do not use for planning, implementation, validation, or
  verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-hardening
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Nix hardening)

## Phase purpose

Assess the ai-workbench Nix flake configuration against the production-hardening
baseline and identify gaps. Each gap MUST be tied to a verbatim config key (a
`nix.conf` setting, a `flake.nix` attribute, or a justfile recipe) and the docs
page that documents it. This phase produces the gap table that the plan phase
turns into edits and that the verify phase checks against.

## Steps to perform

1. Load the `nix-usage` skill for the hardening baseline.
2. Read `flake.nix` (the current flake config, `nixConfig`, source filters).
3. Read the corpus pages that define the hardening keys:
   - `docs/nix/purity-and-sandboxing.md`
   - `docs/nix/store-hygiene-and-gc.md`
   - `docs/nix/caching-and-binary-caches.md`
   - `docs/nix/supply-chain-security.md`
   - `docs/nix/secrets-and-sops.md`
4. Read the supporting config files: `nix/devshells/default.nix`,
   `nix/packages/agentctl.nix`, `flake.lock`, `justfile`.
5. Cross-reference each baseline item against the `docs/nix/` corpus to capture
   the verbatim key, default, and source page.
6. For each baseline item, classify the current config state as `satisfied` or
   `gap`, recording the verbatim key + source. Mark any deployment-specific
   recommendation `[WORKESTRATOR NOTE]`.
7. Record the gap table (below) as the scope contract for the rest of the
   workflow.

## Docs to consult

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/caching-and-binary-caches.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/secrets-and-sops.md`
- `docs/nix/testing.md`
- `.agents/skills/nix-usage/SKILL.md`

## Operational skills to load

- `nix-usage` — the hardening baseline checklist and agent rules. Load at the
  start of this phase; it defines which config keys constitute a hardened Nix
  deployment.

## Constraints to apply

There are no Nix-specific constraint skills in the registry. Instead, reference
the docs/nix/ corpus rules directly as the schema authority:

- Purity rules from `docs/nix/purity-and-sandboxing.md` — every gap must trace
  to a real Nix setting (`nix.conf` key, flake attribute, or justfile recipe);
  no invented keys. The purity rules are the schema authority for nix.conf
  keys.
- Supply-chain rules from `docs/nix/supply-chain-security.md` — FOD hashes and
  `flake.lock` narHash must be real, not placeholders.
- Store-hygiene rules from `docs/nix/store-hygiene-and-gc.md` — GC and
  optimisation keys must be real Nix settings.

## Validations to run

None — validations run in phases 04 and 05.

## Gap assessment table (baseline → current state)

Each row ties a hardening baseline item to a verbatim config key and the docs
page that documents it. Fill the `current` and `state` columns from the repo.

| # | Baseline item | Verbatim key | default | Source page | Current config value | State |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Eval-time purity: no `--impure`, no bare `builtins.path` | `just lint-nix` (scripts/check-nix-paths.sh) | — | purity-and-sandboxing.md, nix-usage | ACTIVE, wired into `just verify` | satisfied |
| 2 | Source filter excludes `target/`, `result*`, `node_modules/`, `agents/*/build/`, `.workestrate/` | `cleanSourceWith { filter = ...; }` (agentctl.nix:15-25) | — | purity-and-sandboxing.md | present (excludes target, result*, vendor, .cargo/config.toml) | satisfied |
| 3 | Build sandbox enabled | `sandbox` (nix.conf) | true (Linux daemon) | purity-and-sandboxing.md | NO project nix.conf; relies on host default | **gap** `[WORKESTRATOR NOTE]` |
| 4 | Signature verification on substitution | `require-sigs` (nix.conf) | true | caching-and-binary-caches.md | NOT explicitly set; relies on default true | **gap** (document the default) |
| 5 | Store deduplication | `auto-optimise-store` (nix.conf) | false | store-hygiene-and-gc.md | NOT set in any project config | **gap** |
| 6 | GC keeps derivations | `keep-derivations` (nix.conf) | true | store-hygiene-and-gc.md | NOT explicitly set; relies on default | **gap** (document the default) |
| 7 | GC keeps outputs | `keep-outputs` (nix.conf) | false | store-hygiene-and-gc.md | NOT explicitly set; relies on default | **gap** (document the default) |
| 8 | Substituter list pinned | `substituters` / `extra-substituters` (nix.conf or flake `nixConfig`) | cache.nixos.org | caching-and-binary-caches.md | flake.nix has NO `nixConfig` block; relies on host | **gap** `[WORKESTRATOR NOTE]` |
| 9 | Trusted public keys pinned | `trusted-public-keys` / `extra-trusted-public-keys` | cache.nixos.org-1:... | caching-and-binary-caches.md | NOT set in project config | **gap** `[WORKESTRATOR NOTE]` |
| 10 | Trusted substituters allow-list | `trusted-substituters` (nix.conf) | — | caching-and-binary-caches.md | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 11 | Eval restriction (no impure paths) | `restrict-eval` (nix.conf) | false | purity-and-sandboxing.md | NOT set | **gap** (optional) |
| 12 | Allowed URIs for fetchers | `allowed-uris` (nix.conf) | — | supply-chain-security.md | NOT set | **gap** (optional) |
| 13 | Flake lock pinning (all inputs have narHash) | `flake.lock` nodes.*.locked.narHash | — | supply-chain-security.md | narHash on all locked inputs (nixpkgs, fenix, pi, odysseus, opencode, tempest) | satisfied |
| 14 | FOD outputHash discipline (no placeholder hashes) | `outputHash` / `npmDepsHash` / `cargoHash` on fetchers | — | supply-chain-security.md, purity-and-sandboxing.md | tempest.nix has `npmDepsHash = "sha256-AAAA..."` placeholder (flake.nix:186) | **gap** (supply chain) |
| 15 | No `result*` symlink leaks | `nix build --no-link --print-out-paths` (flake.nix:230) | — | store-hygiene-and-gc.md, nix-usage | used in flake.nix load-images | satisfied |
| 16 | CARGO_TARGET_DIR relocated out of source tree | `CARGO_TARGET_DIR` (justfile:5, devshell:121) | — | store-hygiene-and-gc.md, nix-usage | relocated | satisfied |
| 17 | Devshell gcroot pin exists | `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell` | — | nix-usage | documented (nix-usage SKILL.md rule 6); verify pin exists | satisfied (documented) |
| 18 | Secret hygiene: sops/age, no hardcoded secrets | `SOPS_AGE_KEY_FILE` (flake.nix:281,302,332) | — | secrets-and-sops.md | indirection used in decrypt-env/write-env/setup-secrets | satisfied |
| 19 | Store-audit gate wired into verify | `just store-audit` (scripts/store-audit.py --warn-if-source-over 50) | — | store-hygiene-and-gc.md, justfile:266 | wired into `just verify` with `--warn-if-source-over 50` | satisfied |
| 20 | Purity lint gate wired into verify | `just lint-nix` (scripts/check-nix-paths.sh) | — | purity-and-sandboxing.md, justfile:343 | ACTIVE, wired into `just verify` | satisfied |
| 21 | GC cadence recipe | `just gc` (nix-collect-garbage --delete-old + nix store optimise) | — | store-hygiene-and-gc.md, justfile:253 | recipe exists | satisfied |
| 22 | Flake check gate | `nix flake check` | — | testing.md | NOT wired into `just verify` or `just verify-full` | **gap** |
| 23 | Store-delta periodic check | `just store-delta-check` (justfile:299) | — | store-hygiene-and-gc.md | exists but NOT wired into `just verify` | **gap** (optional hardening) |
| 24 | trusted-users restriction | `trusted-users` (nix.conf) | root | caching-and-binary-caches.md | NOT set in project config | **gap** `[WORKESTRATOR NOTE]` |

> `[WORKESTRATOR NOTE]` rows: deployment-specific. Rows 3, 8, 9, 10, 24 involve
> introducing a project `nix.conf` or a flake `nixConfig` block — these go beyond
> upstream Nix defaults and are deployment choices. Row 3 (`sandbox`): the host
> Nix daemon already enables sandboxing by default on Linux; a project nix.conf
> would make it explicit. Rows 8–10 (`substituters`/`trusted-public-keys`/
> `trusted-substituters`): pinning these in a flake `nixConfig` block requires
> `--accept-flake-config` and is a deployment decision.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-hardening-00-orchestration`. Set:

- `outcome` to `pass` once the gap table is filled and every gap is tied to a
  verbatim key + source.
- `constraints_applied` to include the purity rules from
  `docs/nix/purity-and-sandboxing.md`, supply-chain rules from
  `docs/nix/supply-chain-security.md`, and store-hygiene rules from
  `docs/nix/store-hygiene-and-gc.md`.
- `next_phase: 02-plan`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: flake.nix
    change: read-only assessment (no edits in scope phase)
constraints_applied:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 02-plan
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
