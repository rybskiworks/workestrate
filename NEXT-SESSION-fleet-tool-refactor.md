# Next session: fleet-tool refactor (ADR 0038) + workload-repos plan

Paste this whole file as your prompt, or point the agent at it.

## 1. Mission

1. **Implement ADR 0038** — the fleet-tool contract refactor: fleets never pin the tool; the tool builds images from capsule data via `[image.sources]` + the tool-baked builder; ambient devshell CLI; schema min/max window. Follow its phased order: **Phase 1 tool-additive first, so nothing breaks.**
2. **Immediately after the pin changes land:** investigate + plan the workloads→repos conversion (per-workload repos pinned by the fleet; tempo-buddy pilot; fleet-level `workestrate.lock`; the interface contract). Output is a written plan, not code.

## 2. Reading order

1. `/home/node/Development/agent-workbench/workestrate/AGENTS.md`
2. `/home/node/Development/agent-workbench/workestrate-dev-home/NOTES-roadmap-and-architecture.md` (state + backlog)
3. `/home/node/Development/agent-workbench/workestrate/docs/migration/50-decisions/0038-fleets-trust-the-tool-contract.md` (**THE spec** — follow its decisions; open questions in §5)
4. `/home/node/Development/agent-workbench/workestrate-dev-home/NOTES-state-and-rename-plan.md` (living state)
5. `/home/node/Development/agent-workbench/workestrate-dev-home/assessment-2026-09-04/` (evidence if needed; subdirs: code-health, config-fleet, nix-devenv)
6. `/home/node/Development/agent-workbench/workestrate/docs/migration/50-decisions/README.md` (ADR index)

## 3. Current state (git facts, verified 2026-09-06)

- **workestrate repo**: `/home/node/Development/agent-workbench/workestrate`, branch `migration/tool-model`, HEAD `4f08204` "docs: ADR for fleet-tool contract separation (fleets never pin the tool)" — ADR 0038. Local tracking ref `origin/migration/tool-model` is AT `4f08204` (0 ahead).
- **Last verified-green commit**: `beacda4` "fix(test): seed tombi.toml in scratch repo for hook version probe". Gates green at `beacda4`; subsequent commits `3be49ab`, `4c50fcf`, `c6a01a9`, `1ce3f80`, `4f08204` are docs/justfile fixes after it.
- **home repo** (workestrate-dev-home): HEAD `6be220b` "docs: record roadmap and architecture state", 0 ahead of `origin/main` per local refs. Everything else in sync.
- **Caveat**: remote reachability could NOT be verified from the container (no ssh), so push state is per local tracking refs. Owner should confirm the push from the host.
- **CI POC** is committed but first-run status unknown. Check `gh run list` — note: `gh` from this container returns HTTP 401 bad credentials; run it from the host where auth works. Last verified state: gates green at `beacda4` + subsequent commits; prime+litellm run on host.

## 4. Rules of engagement

- **just-first / nix-first** — never raw cargo outside just recipes or the devshell; no `--impure`.
- Enter via `just shell` (devenv-root override already handled).
- Plain commits with hooks — never `--no-verify`.
- Conventional commit subjects, **no ADR numbers in subjects**.
- Pushes need host auth (no ssh in container).
- `~/.workestrate` is a downstream clone of dev-home — generated artifacts: commit canonically in dev-home, clones pull; never commit generated files in the live clone; the container mounts `~/.workestrate` read-only (host-only writes there).
- Disk guardrails: `df -h` first; no blind `nix gc` (optimise only); cargo caches are regenerable.
- Worktree/container-shared-tree reality: verify heads before acting.

## 5. Open questions needing the owner (from ADR 0038)

- `update-hashes` verb placement.
- koffi escape-hatch ruling.
- Schema MIN-bump sign-off cadence.

## 6. Acceptance for Phase 1 (per ADR 0038)

- Capsule fields parse + validate.
- Builder bakes into the tool package.
- Legacy path intact (existing 4 images still build).
- Window row in doctor.
- Golden-fixture stability check.
- `just verify` green.
- Schemas regenerated + distributed.

## 7. Workload→repos investigation scope (phase 2)

Read ADR 0038's fleet-level lock design + the earlier interface sketch (per-workload repo exposes `packages.<image>`, `checks`, `lib.workload` metadata). Evaluate: pin mechanics (fleet `workestrate.lock`: per-workload url+ref+rev); what moves from the monorepo; tempo-buddy as pilot (smallest, reuses piBuilt); what CI needs. **Output = a written plan (new NOTES or ADR section), NOT code.**

## 8. Do NOT (scope limits)

- No schema-2 removals this session.
- No rename execution (config/fleet renames batch later).
- No firmware Phase 2 (needs an owner).
- No perf work.

## Addendum — post-handover landings (2026-09-06)

Everything below landed after this handover was drafted (`b68ebb7` in dev-home; moved here in `9d1a371`). All SHAs verified in `git log` 2026-09-06.

### CI evolution (3 iterations)

- **POC (`3799d42`, 2 jobs)**: PR quality-gate workflow (verify chain + build). Pushed and RAN on GitHub — failed on disk fill: the verify job booted the full devenv closure on a fresh runner (GH runners ~14G).
- **v2 (`8803ff8`)**: rework to light path-gated PR job — rustup toolchain via `dtolnay/rust-toolchain@1.97` (guarded against the flake marker); cargo fmt/clippy/test gates defined in-run with repo configs; pure-script gates (lock-guard / versions-check / lint-nix) run without nix; `nix build` moved to a `workflow_dispatch`-only heavy job; rust-cache added.
- **v3 (`8b68eee`)**: self-contained `e2e-nix` job (ai-memory-inspired patterns) — label-gated (`nix-ci`); in-action provisioning of pinned msb+agentd with fail-closed env asserts via `$GITHUB_ENV`; build-then-run smoke in heavy-build.
- **Deferred**: self-hosted KVM runner (dblab42), org cachix, nightly flake check, SHA pinning of actions.

### Edition 2024 (crate + formatter, forward-only posture)

- `gen` identifiers renamed: `f20bfca` (params) + `adfb22e` (alias/debt).
- `unsafe_code` forbid→deny deviation with scoped allows (E0133 env-mutation fix, `1f89e48`) — **needs owner sign-off**.
- Clippy 106 → 0.
- Formatter/checks pinned at edition 2024: workestrate `9b0fd97` + nix-tooling `a9bd083`.

### Gates to green

- Hook auto-fixes + lint debt: `7106ad4`, `adfb22e`.
- Verify chain moved inside the devshell: `4f64adf`.
- Entry decoupled from shell-entry lint: nix-tooling `b6636bb` / `62e0694`.
- Nested up-decision logic bug fixed: `1ce3f80`.
- Schema regen + distribution: `ef56e79` + resync commits (home `e9c2162`).
- Result: `just test` 1545/0, `just check` / `just verify` exit 0, devshell entry clean without skips.

### Runtime proof

- prime + litellm running end-to-end on the host; seed-staleness root-caused → prime capsule `only_if_missing = false` (personal config repo `7c5eede`).
- pi↔litellm connectivity proven.
- live-home repaired: on main, clones fresh, receives via github origin.

### Disk

- nix store optimise freed +1G; regenerable cargo caches deleted (+12.5G) → ~18G free.

### Push state (verified fresh 2026-09-06)

- **workestrate**: `origin/migration/tool-model` == HEAD `8b68eee` — 0 unpushed (local tracking refs).
- **home (workestrate-dev-home)**: HEAD `b36eb28`, 2 ahead of `origin/main` (`6be220b`) — unpushed: `b68ebb7` (handover draft) + `b36eb28` (move to tool repo).
- **nix-tooling**: `origin/main` == HEAD `a9bd083` — 0 unpushed (local tracking refs).
- **live-home (~/.workestrate)**: content synced via github origin (HEAD `e9c2162` == its `origin/main`). Direct push `main:main` was refused (checked-out branch); `receive.denyCurrentBranch=updateInstead` is the one-line fix if direct pushes are wanted.
