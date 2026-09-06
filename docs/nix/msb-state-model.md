---
# msb state model (consumer side)

How workestrate consumes the microsandbox fork's three-layer state model.
Producer side (build inputs, homeless shelter, fail-closed nix contract):
the fork's `nix/README.md`. Live provisioning reference:
`docs/runtime-provisioning.md`; decision note: ADR 0037
(`docs/migration/50-decisions/0037-msb-state-generations.md`).

## The three layers, from the consuming end

| # | Layer | Owner | Location | Workestrate posture |
|---|-------|-------|----------|---------------------|
| 1 | Build inputs (pure) | fork flake | nix store, `flake.lock`-pinned | Pin the fork rev (`microsandbox-fork` input in `flake.nix`); never mutate |
| 2 | Build/check state (ephemeral) | fork flake | `$TMPDIR/.microsandbox` in check `preBuild` | Not our state; must never touch `$HOME` |
| 3 | Runtime state (impure by design) | workestrate (user-owned) | `$HOME/.microsandbox/current` → `generations/<hash12>/` | Converge, never migrate in place |

## msb is host tooling

`msb` runs on the host beside `workestrate`, not inside workload images:
image builds and migration history are verified no-impact across msb pin
changes. What a pin change DOES move is the generation key (below) — state
placement, not image content.

## Fork-flake flip = new generation key by design

The generation key is the 12-char prefix of the baked `MSB_PATH` store-path
hash (`control/agentctl/src/microsandbox/generation.rs`); `current` flips
atomically between `generations/<hash12>/` dirs and the converge script
(`scripts/msb-generation-converge.sh`, 614 lines, whitelist
`db sandboxes volumes snapshots secrets tls ssh mount-policy config.json`)
copies state forward, fresh-initing on ANY failure. Flipping to fork-flake
consumption therefore creates a NEW generation key by design: it never
corrupts existing state, and the old generation is untouched — it IS the
rollback. `unmanaged` (non-store msb) keeps single-generation legacy
behavior; nothing to converge.

## Fail-closed env asserts are repo idiom

`nix` builds forbid ambient inputs, so every input the e2e tests probe is
provisioned by the job and asserted before export (`.github/workflows/ci.yml`,
e2e-nix: nix on `PATH`, `MSB_PATH`, `MSB_AGENTD_PATH`, `gunzip` — assert all
four, else fail before export). The fork's nix contract follows the same
idiom: its check `preBuild` must assert `MSB_AGENTD_PATH` is set so the
`build.rs` release-download fallback can never silently fire (producer
detail: fork `nix/README.md`).

## Aside: the homeless shelter

Nix sandboxed builds set `HOME=/homeless-shelter`, a path that deliberately
does not exist — a canary that kills builds writing to `$HOME` loudly
(`Permission denied: /homeless-shelter`). Never "fix" it by creating the
directory; redirect the app's state dir (`MSB_HOME=$TMPDIR/.microsandbox`).
Full rationale, including the naming aside: fork `nix/README.md`.
Full layering: `docs/runtime-provisioning.md`.

## Pointers

- Fork producer doc: microsandbox fork `nix/README.md` (layers 1–2, shelter, fail-closed contract).
- `docs/runtime-provisioning.md` — canonical MSB home, wrapper contract, env matrix.
- ADR 0037 (`docs/migration/50-decisions/0037-msb-state-generations.md`) — generations decision.
- `control/agentctl/src/microsandbox/generation.rs` — key derivation + the ONE resolution rule.
- `scripts/msb-generation-converge.sh` — converge/GC owner.
---
