# ADR 0037: MSB state generations

**Status:** Accepted
**Date:** 2026-09-05
**References:** `control/agentctl/src/microsandbox/generation.rs` (key derivation + the ONE resolution rule); `scripts/msb-generation-converge.sh` (converge/GC owner); `scripts/host-provision.sh` (Step B½ wiring, best-effort); `control/agentctl/src/commands/doctor.rs` (`doctor_check_generation` row); `control/agentctl/src/microsandbox/runtime/run.rs` (fail-closed up gate + `.booted-ok`); `control/agentctl/src/microsandbox/runtime/down_scope.rs` (multi-generation down sweeps); `nix/packages/agentctl.nix` postInstall (wrapper default `$HOME/.microsandbox/current`); `tests/msb-generation-converge/` (host-runnable fixtures); `docs/runtime-provisioning.md` (live docs)

## 1. Title / Status / Date

- **Title:** MSB state generations — key msb state by the pinned build's nix store path, converge atomically between per-generation homes
- **Status:** Accepted.
- **Date:** 2026-09-05
- **Amends:** the "single home" reading of `docs/runtime-provisioning.md` (one canonical ROOT stands; state beneath it is generation-keyed).

## 2. Context

The canonical msb home (`$HOME/.microsandbox`) was a single shared dir: every pinned msb build read/wrote the same `msb.db`, and a pin bump let the NEW binary mutate SHARED state in place (schema migration on first touch) with no rollback and no quiesce discipline. The fork pins `msb` per revision via `MSB_PATH`, so the build identity is already a nix store path — a natural, stable key for the state that belongs to it.

## 3. Design

- **Generation key:** canonicalize the baked `MSB_PATH` (`/nix/store/<hash32>-microsandbox-<ver>/bin/msb`), require the `<store-dir>/bin/msb` tail, split the store-dir basename at the first `-microsandbox-`, require a 32-char lowercase `[a-z0-9]` hash segment; the key is its **12-char prefix**. Any mismatch (raw PATH install, canonicalize failure, pattern mismatch) is key `unmanaged` — single-generation legacy behavior, nothing to converge.
- **Layout:** `$HOME/.microsandbox/generations/<hash12>/{db,sandboxes,run,...}`; `current` symlink flipped atomically (tmp symlink + rename under a `.flip.lock` flock owned by the converge script); per-generation `.booted-ok` marker written by the runtime on first verified up.
- **ONE resolution rule** (`resolve_msb_home_generation`, mirrored in shell): non-empty `MSB_HOME` verbatim (explicit override; out of converge scope — the identity check still canonicalizes through the `current` symlink) > `current` symlink target > missing `current` + exactly one generation dir → heal > zero generations + `db/` at the root → pre-generation home, absorbed as `generations/legacy` > fresh. Missing `current` + MORE THAN ONE generation dir is an operator error (refused, naming the keys). A dangling `current` counts as missing.
- **Wrapper default change:** the `agentctl.nix` wrapper defaults unset/empty `MSB_HOME` to `$HOME/.microsandbox/current`; msb resolves the symlink itself, so its home is the target generation dir.
- **Runtime mirrors:** the doctor `generation` row (OK/WARN/FAIL naming `scripts/host-provision.sh`), the fail-closed `up` gate on generation mismatch, and the `down` home/everything sweeps that iterate all retained generations.

## 4. Converge chain + GC

`scripts/msb-generation-converge.sh` (Step B½ of host-provision, best-effort — failures never fail provisioning; the doctor row carries the FAIL):

1. **Quiesce gate** — probe EVERY generation dir; REFUSE (exit 2, never partial) on live sandboxes or unproven quiescence, printing per-generation reap commands.
2. **Copy** — reflink-copy the state whitelist (`db/msb.db`(+wal/shm), `sandboxes`, `volumes`, `snapshots`, `secrets`, `tls`, `ssh`, `mount-policy`, `config.json`; NEVER `run/`/`tmp/`/`bin/`/`lib/`) into a `.converge-tmp-*` staging dir.
3. **Forward-migrate + verify** with the new binary (`msb list`, plus `sqlite3 PRAGMA integrity_check` when available).
4. **Reset on failure** — ANY converge failure (copy error, integrity non-ok, schema refusal) removes staging and FRESH-INITs the new generation (deterministic empty `db/` skeleton); the old generation is untouched and IS the rollback.
5. **Flip** — `current` flips only after a verified converge.
6. **GC** — keep exactly `{current, newest other generation carrying .booted-ok}`; liveness probes fail-closed (probe error → KEEP); interrupted-converge staging dirs and old `migrate-msb-home` backups are swept.

## 5. Adjudications

- **(a) 12-char hash prefix.** The total MSB_HOME path length is a fork hard limit of 59 chars (the unix-socket paths derived beneath MSB_HOME must fit `sun_path`). The full 32-char base32 segment would blow the budget on realistic `$HOME` lengths; 12 chars of a nix store hash keeps generation paths short while remaining collision-safe for the handful of live generations (GC keeps at most 2 plus the same-run source).
- **(b) Copy-then-reset chain.** Converge NEVER migrates the old generation in place: state is copied to staging, and any failure resets the NEW generation to a deterministic fresh init instead of leaving a half-migrated copy. Rollback is therefore trivial (the old generation is byte-untouched; repoint `current` or re-run converge with the previous pin). Corollary: the migration SOURCE of a converge that just flipped (including a freshly absorbed `legacy` generation) is EXEMPT from that run's GC sweep — it is the rollback — and becomes eligible on the next successful converge.

## 6. Consequences

- Pin bumps no longer mutate shared state in place; a corrupt/unreadable new generation self-heals to a fresh init on the next converge, and rollback never requires a backup restore.
- Operators can hold at most a small, bounded set of generation dirs (GC keep-rule); debris under `generations/` surfaces as a doctor WARN.
- `MSB_HOME` explicit overrides bypass converge entirely (verbatim); the identity check still flags a mismatch when the override canonicalizes to a different `generations/<key12>` dir.
- Cost: the first `up` after a pin bump requires a quiesced fleet (the gate refuses otherwise), and a schema refusal silently costs the carried state (documented: the reset is deterministic and the old generation survives).
