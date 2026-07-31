---
name: workflow-nix-hardening-02-plan
description: |
  Use only for the plan phase of the Nix hardening workflow. List the
  hardening edits, each tied to a verbatim config key + source. Do not use for
  scoping, implementation, validation, or verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-hardening
  org.phase: plan
  org.phase_order: "02"
---

# Phase 02: plan (Nix hardening)

## Phase purpose

Turn the scope-phase gap table into an ordered edit list. Every edit MUST be
tied to a verbatim config key (a `nix.conf` setting, a `flake.nix` attribute,
or a justfile recipe) and the docs page that documents it. No edit may
introduce a key not present in the `docs/nix/` corpus.

## Steps to perform

1. Read the scope-phase handoff (the gap table).
2. For each `gap` row, define the exact edit: target file, key, value, and the
   verbatim source citation.
3. Order edits by target file (`flake.nix` `nixConfig` block, `nix.conf`,
   `nix/devshells/default.nix`, `nix/packages/agentctl.nix`, `justfile`) to
   minimize diff churn.
4. For each edit, record the anti-hallucination citation: verbatim key +
   `default` + source page.
5. Mark every deployment-specific recommendation `[WORKESTRATOR NOTE]`.
6. Produce the edit list (below) as the plan contract for the implement phase.

## Docs to consult

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/caching-and-binary-caches.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/testing.md`
- `.agents/skills/nix-usage/SKILL.md`

## Operational skills to load

- `nix-usage` — for the recommended value at each gap.

## Constraints to apply

There are no Nix-specific constraint skills in the registry. Reference the
docs/nix/ corpus rules directly:

- Purity rules from `docs/nix/purity-and-sandboxing.md` — every edit key must
  be a real Nix setting; values must match the documented type. No `--impure`,
  no bare `builtins.path`/`cleanSourceWith` without `filter`.
- Supply-chain rules from `docs/nix/supply-chain-security.md` — no placeholder
  hashes; FOD `outputHash`/`npmDepsHash`/`cargoHash` must be real.
- Store-hygiene rules from `docs/nix/store-hygiene-and-gc.md` — GC and
  optimisation keys must be real Nix settings; no `result*` symlink leaks.

## Validations to run

None — validations run in phases 04 and 05.

## Edit list (each tied to a verbatim config key + source)

| # | Target file | Verbatim key | Planned value | default | Source page | Citation |
| --- | --- | --- | --- | --- | --- | --- |
| E1 | `flake.nix` (nixConfig) | `sandbox` | `true` | true (Linux daemon) | purity-and-sandboxing.md | make sandbox explicit in flake nixConfig; `[WORKESTRATOR NOTE]` |
| E2 | `flake.nix` (nixConfig) | `require-sigs` | `true` | true | caching-and-binary-caches.md | document the default explicitly |
| E3 | `flake.nix` (nixConfig) | `auto-optimise-store` | `true` | false | store-hygiene-and-gc.md | enable store deduplication |
| E4 | `flake.nix` (nixConfig) | `keep-derivations` | `true` | true | store-hygiene-and-gc.md | document the default explicitly |
| E5 | `flake.nix` (nixConfig) | `keep-outputs` | `false` | false | store-hygiene-and-gc.md | document the default explicitly |
| E6 | `flake.nix` (nixConfig) | `substituters` | `https://cache.nixos.org` | cache.nixos.org | caching-and-binary-caches.md | `[WORKESTRATOR NOTE]` — see decision below |
| E7 | `flake.nix` (nixConfig) | `trusted-public-keys` | `cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=` | cache.nixos.org-1:... | caching-and-binary-caches.md | `[WORKESTRATOR NOTE]` — see decision below |
| E8 | `flake.nix` (nixConfig) | `trusted-users` | `root` | root | caching-and-binary-caches.md | `[WORKESTRATOR NOTE]` — restrict to root |
| E9 | `nix/packages/tempest.nix` | `npmDepsHash` | (real hash, computed) | — | supply-chain-security.md, purity-and-sandboxing.md | replace placeholder `sha256-AAAA...` with computed hash |
| E10 | `justfile` | `nix flake check` recipe | wire into `just verify-full` | — | testing.md | add `nix flake check --no-build` to verify-full |
| E11 | `justfile` | `just store-delta-check` | wire into `just verify-full` (optional) | — | store-hygiene-and-gc.md | optional hardening; `[WORKESTRATOR NOTE]` |

### E6/E7 decision: substituters / trusted-public-keys

`flake.nix` has NO `nixConfig` block; the project relies on the host
`nix.conf` + `cache.nixos.org`. Two options:

- **Option A (pin in flake `nixConfig`):** add a `nixConfig` block to
  `flake.nix` pinning `substituters` and `trusted-public-keys` explicitly to
  `cache.nixos.org`. `[WORKESTRATOR NOTE]` — requires `--accept-flake-config`
  on first use; value is deployment-specific, not from upstream docs.
- **Option B (document the control):** leave `nix.conf` reliance on the host
  and record in the handoff that host `nix.conf` is the substituter authority.

The implement phase applies the chosen option. Record the decision in
`assumptions`.

### Already-satisfied items (no edit)

Rows 1, 2, 13, 15, 16, 17, 18, 19, 20, 21 from the scope table (`just lint-nix`,
`cleanSourceWith` filter, `flake.lock` narHash, `--no-link --print-out-paths`,
`CARGO_TARGET_DIR`, devshell gcroot, `SOPS_AGE_KEY_FILE`, `just store-audit`,
`just lint-nix` wired, `just gc`) are already satisfied — no edit planned. The
implement phase MUST NOT regress them.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-hardening-00-orchestration`. Set:

- `outcome` to `pass` once every edit is tied to a verbatim key + source.
- `constraints_applied` to include the purity rules from
  `docs/nix/purity-and-sandboxing.md`, supply-chain rules from
  `docs/nix/supply-chain-security.md`, and store-hygiene rules from
  `docs/nix/store-hygiene-and-gc.md`.
- `next_phase: 03-implement`.
- `blockers: []` unless the E6/E7 decision is unresolved (then `blockers`
  lists it and `handoff_requires_hil: true`).

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
assumptions:
  - "E6/E7 decision: <Option A or B>"
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
