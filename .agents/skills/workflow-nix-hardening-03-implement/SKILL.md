---
name: workflow-nix-hardening-03-implement
description: |
  Use only for the implement phase of the Nix hardening workflow. Apply the
  planned edits to `flake.nix`, `nix/devshells/default.nix`,
  `nix/packages/agentctl.nix`, and optionally a new `nix.conf` under the active
  constraints. Do not use for scoping, planning, validation, or verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-hardening
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Nix hardening)

## Phase purpose

Apply the plan-phase edit list to the Nix flake configuration. Every edit must
match a verbatim config key from the `docs/nix/` corpus. This phase does not
invent keys, regress already-satisfied items, touch package derivations' build
logic, or introduce secrets.

## Steps to perform

1. Read the plan-phase handoff (the edit list + E6/E7 decision).
2. Read `flake.nix`, `nix/devshells/default.nix`, `nix/packages/agentctl.nix`,
   `nix/packages/tempest.nix`, `justfile`.
3. Apply edits grouped by target file, in this order: `flake.nix` (`nixConfig`
   block), `nix/packages/tempest.nix` (FOD hash), `justfile` (verify-full
   wiring). Preserve existing keys; only add the planned keys.
4. For each edit, verify the key is a real Nix setting documented in the
   `docs/nix/` corpus before writing.
5. Do NOT touch package derivations' build logic (no behavior change). Only
   edit the `nixConfig` block, FOD hashes, and justfile recipes.
6. Do NOT regress the already-satisfied items: `just lint-nix`,
   `cleanSourceWith` filter, `--no-link --print-out-paths`, `CARGO_TARGET_DIR`,
   devshell gcroot, `just store-audit`, `just gc`, `flake.lock` narHash,
   `SOPS_AGE_KEY_FILE` indirection.
7. Apply the active constraints (below) during the edit.
8. Record the exact diff in `files_touched`.

## Docs to consult

- `docs/nix/purity-and-sandboxing.md` — verify each nix.conf key.
- `docs/nix/store-hygiene-and-gc.md` — GC and optimisation keys.
- `docs/nix/caching-and-binary-caches.md` — substituters and trusted keys.
- `docs/nix/supply-chain-security.md` — FOD hash discipline.
- `.agents/skills/nix-usage/SKILL.md` — the hardened shape.

## Operational skills to load

- `nix-usage` — for the recommended value at each edit.

## Constraints to apply

There are no Nix-specific constraint skills in the registry. Reference the
docs/nix/ corpus rules directly:

- Secret hygiene (from `docs/nix/secrets-and-sops.md`) — all secrets MUST
  remain `SOPS_AGE_KEY_FILE` indirection; no hardcoded secrets. New keys must
  not introduce secret literals.
- Purity rules (from `docs/nix/purity-and-sandboxing.md`) — no `--impure`, no
  bare `builtins.path`/`cleanSourceWith` without `filter`. Every written
  nix.conf key must be a real Nix setting with the documented type.
- Supply-chain rules (from `docs/nix/supply-chain-security.md`) — no
  placeholder hashes; FOD `outputHash`/`npmDepsHash`/`cargoHash` must be real
  computed hashes. Do NOT add `database_url`-style or non-Nix keys.

## Validations to run

None in this phase — `just lint-nix`, `just store-audit`, and
`nix flake check --no-build` run in phase 04. A lightweight
`nix flake check --no-build` sanity check is permitted to avoid handing off a
syntactically broken flake, but it is not a gate.

## Expected resulting shape

After the edits, `flake.nix` should contain a `nixConfig` block (additions
marked `# [HARDENING]`; existing keys unchanged):

```nix
{
  description = "ai-workbench";

  # [HARDENING] E1–E8 — flake-level nixConfig block [WORKESTRATOR NOTE]
  nixConfig = {
    sandbox = true;                                    # [HARDENING] E1
    require-sigs = true;                              # [HARDENING] E2
    auto-optimise-store = true;                        # [HARDENING] E3
    keep-derivations = true;                           # [HARDENING] E4
    keep-outputs = false;                              # [HARDENING] E5
    substituters = [ "https://cache.nixos.org" ];      # [HARDENING] E6 [WORKESTRATOR NOTE]
    trusted-public-keys = [                           # [HARDENING] E7 [WORKESTRATOR NOTE]
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
    ];
    trusted-users = [ "root" ];                        # [HARDENING] E8 [WORKESTRATOR NOTE]
  };

  inputs = {
    # existing (unchanged) — narHash on all locked inputs
  };

  outputs = { self, nixpkgs, ... }@inputs: {
    # existing (unchanged)
  };
}
```

`nix/packages/tempest.nix` should replace the placeholder hash (E9):

```nix
  # [HARDENING] E9 — replace placeholder with computed hash
  npmDepsHash = "sha256-<real-computed-hash>";         # was: sha256-AAAA...
```

`justfile` should wire `nix flake check` into `verify-full` (E10):

```makefile
# [HARDENING] E10 — wire nix flake check into verify-full
verify-full: verify
    nix build .#workestrate
    nix flake check --no-build
```

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-hardening-00-orchestration`. Set:

- `outcome` to `pass` once all planned edits are applied and the flake
  evaluates.
- `constraints_applied` to include the secret-hygiene, purity, and
  supply-chain rules listed above.
- `files_touched` to list each edited file with the change summary.
- `next_phase: 04-validate`.
- `blockers: []` unless an edit key failed corpus verification.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: flake.nix
    change: "added nixConfig block (E1–E8); no input/output changes; no secret regressions"
  - path: nix/packages/tempest.nix
    change: "replaced placeholder npmDepsHash with computed hash (E9)"
  - path: justfile
    change: "wired nix flake check --no-build into verify-full (E10)"
constraints_applied:
  - secret-hygiene (docs/nix/secrets-and-sops.md)
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
assumptions:
  - ...
risks:
  - ...
tests_run:
  - "nix flake check --no-build sanity check"
tests_needed:
  - "just lint-nix (phase 04)"
  - "just store-audit (phase 04)"
  - "nix flake check --no-build (phase 04)"
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
