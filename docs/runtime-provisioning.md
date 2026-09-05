# Runtime provisioning — MSB home, wrapper contract, schema flow

> **Companion to ADR 0035.** This doc owns the canonical MSB home, the wrapper contract, the env override matrix, and the schema distribution flow. ADR 0035 owns the network policy ladder. The MSB state-generations model is summarized below (decision note: `docs/migration/50-decisions/0037-msb-state-generations.md`).

## Canonical MSB home (state generations)

msb (microsandbox) discovers its home via `MSB_HOME`, resolved per
`microsandbox_utils::resolve_home`: **non-empty `MSB_HOME` verbatim** (empty
treated as unset), else **`$HOME/.microsandbox`**, else `./.microsandbox`.
workestrate converges every flow on that ONE root — but state beneath it is
**generation-keyed** (see the next section): the runtime home is the
`$HOME/.microsandbox/current` symlink, flipped atomically between
`generations/<hash12>` dirs as the pinned msb build changes.

| # | Home | Path | When active | Purpose |
|---|------|------|-------------|---------|
| 1 | **canonical runtime** | `$HOME/.microsandbox/current` → `generations/<hash12>` | Runtime (guarded `wrapProgram --run` in `nix/packages/agentctl.nix` postInstall) | **The only runtime home** — cache, db, state, `sandboxes/`, one dir per pinned msb build. The guard (`if [ -z "${MSB_HOME:-}" ]; ...`) defaults unset AND empty to the canonical home while honoring an explicit non-empty caller override. `--set-default` would NOT handle the empty-string case, hence the explicit guard. Shell expansion at wrapper execution time (not build time). |

Build-time-only staging (NOT homes — no msb state ever lands here):

- **devshell staging** (`_msb_home="$HOME/.cache/ai-workbench-msb"` in
  `flake.nix` `devenv.shells.default` `enterShell`) — stages the `msb`
  binary + `libkrunfw.so*` symlinks/copies so offline `cargo check` builds
  find the runtime without network. The devshell deliberately does **not**
  export `MSB_HOME`/`MSB_PATH` (no second runtime home); it exports only
  `MSB_AGENTD_PATH` (musl static `agentd`, needed by the fork's filesystem
  crate `build.rs` prebuilt branch) plus `CARGO_TARGET_DIR`, the vendor
  link, and agent builds.
- **nix build TMPDIR** (`export MSB_HOME=$TMPDIR/.microsandbox` in
  `nix/packages/agentctl.nix:111` preBuild) — hermetic build: the fork's
  `build.rs` finds `msb` + `agentd` via `MSB_HOME`/`MSB_AGENTD_PATH`
  without network downloads. Wiped after build.

Pre-convergence hosts may still carry state under the legacy devshell path
(`$HOME/.cache/ai-workbench-msb/db/msb.db`): `scripts/migrate-msb-home.sh`
migrates it to the canonical home (newest-DB-wins, timestamped backups,
`--check-only`/`--dry-run`/`--rollback <ts>`), `workestrate doctor` (the
`msb` check) flags skew/unmigrated/downgrade states, and
`scripts/host-provision.sh` runs the generation converge best-effort
between binary sync and doctor — the converge script delegates the
cache-home migration to `migrate-msb-home.sh` first, then absorbs any
pre-generation root as `generations/legacy`.

Additional override: `MSB_AGENTD_PATH = ${microsandbox}/libexec/agentd` (musl static `agentd`) satisfies the fork's filesystem crate `build.rs` prebuilt branch (`nix/packages/agentctl.nix:117`).

## MSB state generations

A **generation** is the mutable state of one pinned msb build. Layout under
the canonical root `$HOME/.microsandbox`:

```
generations/<hash12>/{db,sandboxes,run,...}   one dir per pinned msb build
generations/<hash12>/.booted-ok               written by the runtime on first verified up
current                                       symlink; atomic flip = tmp symlink + rename
.flip.lock                                    flock owned by the converge script
```

- **Key derivation** (`control/agentctl/src/microsandbox/generation.rs`):
  canonicalize the baked `MSB_PATH`
  (`/nix/store/<hash32>-microsandbox-<ver>/bin/msb`), require the
  `<store-dir>/bin/msb` tail, split the store-dir basename at the first
  `-microsandbox-`, require a 32-char lowercase `[a-z0-9]` hash segment; the
  key is its **12-char prefix**. Any mismatch (raw PATH install, missing
  binary, pattern mismatch) is key `unmanaged` — single-generation legacy
  behavior, nothing to converge.
- **ONE resolution rule** (`resolve_msb_home_generation`, mirrored in the
  converge script): non-empty `MSB_HOME` verbatim (explicit override, out of
  converge scope) > `current` symlink target > `current` missing + exactly
  one generation dir → heal the symlink > no generations + `db/` at the root
  → pre-generation home, absorbed as `generations/legacy` > fresh. `current`
  missing + **more than one** generation dir is an operator error (refused,
  naming the keys). A dangling `current` symlink counts as missing.
- **Converge flow** (`scripts/msb-generation-converge.sh`, wired best-effort
  into `scripts/host-provision.sh` Step B½; `--dry-run` / `--check-only`):
  quiesce gate (REFUSE when any generation has live sandboxes or cannot be
  proven quiesced, printing per-generation reap commands) → reflink-copy the
  state whitelist (`db/msb.db`(+wal/shm), `sandboxes`, `volumes`,
  `snapshots`, `secrets`, `tls`, `ssh`, `mount-policy`, `config.json`;
  **never** `run/`/`tmp/`/`bin/`/`lib/`) into a staging dir →
  forward-migrate + verify with the new binary (+ `sqlite3
  integrity_check` when available) → **any failure FRESH-INITs the new
  generation** (rm staging + deterministic empty `db/` skeleton; the old
  generation is untouched and IS the rollback) → atomic flip of `current`
  under `.flip.lock` → GC keeps exactly `{current, newest other generation
  carrying .booted-ok}` plus the **same-run source exemption** (the converge
  source survives THAT run's sweep as the rollback; eligible next run).
- **Wrapper default change**: the `agentctl.nix` wrapper now defaults
  unset/empty `MSB_HOME` to `$HOME/.microsandbox/current` (was the root);
  msb resolves the symlink itself. The generation IDENTITY check (doctor
  row + fail-closed `up` gate) canonicalizes through the symlink rather
  than trusting the literal value.
- **Socket budget**: the fork derives unix-socket paths beneath MSB_HOME
  that must fit `sun_path`, making the total MSB_HOME length a hard 59-char
  limit — 12-char keys keep generation paths inside it.
- **Observability**: the `workestrate doctor` `generation` row (OK/WARN/
  FAIL with remediation naming `scripts/host-provision.sh`) reports the
  baked key vs the resolved generation, debris under `generations/`, and
  legacy-root/ambiguous states.

## Wrapper contract (`nix/packages/agentctl.nix`)

The `agentctl.nix` wrapper (built `workestrate` binary, currently `0.1.0`) bakes the runtime contract at build time:

```nix
wrapProgram $out/bin/workestrate \
  --set MSB_PATH "${microsandbox}/bin/msb" \
  --set MSB_AGENTD_PATH "${microsandbox}/libexec/agentd" \
  --prefix PATH : ${pkgs.sops}/bin \
  --run 'if [ -z "${MSB_HOME:-}" ]; then export MSB_HOME="$HOME/.microsandbox/current"; fi'
```

- **`--set MSB_PATH`** — baked `0.6.16` `msb` binary path (`nix/packages/microsandbox.nix:54`, fork rev `78fb3ed1`). The SDK spawns `MSB_PATH`; it must be a live-resize-capable `msb` (linux `libkrunfw.so.5.6.1` from the `v0.6.8` release tarball, `nix/packages/microsandbox.nix` Branch A).
- **`--set MSB_AGENTD_PATH`** — baked musl static `agentd` path for the fork's filesystem crate prebuilt branch.
- **`--run MSB_HOME`** — guarded default: honors an explicit non-empty `MSB_HOME` override while defaulting unset AND empty to `$HOME/.microsandbox/current` (the `current` generation symlink) at wrapper execution time so `$HOME` expands per-user (not per-build). Neither the devshell staging dir nor the build `$TMPDIR/.microsandbox` is baked here.
- **`--prefix PATH : sops`** — `sops` binary for `workestrate secrets` (ADR 0034).

The devshell (`flake.nix` `devenv.shells.default` `enterShell`) stages the
same runtime for offline builds but exports **only** `MSB_AGENTD_PATH`:

```bash
_msb_home="$HOME/.cache/ai-workbench-msb"   # build-time staging only, NOT a runtime home
mkdir -p "$_msb_home/bin" "$_msb_home/lib"  # no rm -rf of live dirs, no MSB_HOME/MSB_PATH exports
export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"
```

## Env override matrix

| Env var | Set by | Precedence | Effect |
|---------|--------|------------|--------|
| `MSB_HOME` | caller env, else wrapper `--run` guard | non-empty `MSB_HOME` > `$HOME/.microsandbox/current` > `./.microsandbox/current` (SDK `resolve_home` + workestrate default; empty treated as unset) | Where msb reads/writes cache, db, `sandboxes/` — the canonical home, resolving through the `current` generation symlink |
| `MSB_PATH` | wrapper `--set` | Explicit path to `msb` binary; `cargo` build.rs and SDK use it | Must point at the pinned `0.6.16` binary; its store-path hash segment keys the state generation |
| `MSB_AGENTD_PATH` | wrapper `--set` / build preBuild / devshell `enterShell` | Guest init binary path | Fork's `build.rs` prebuilt branch; musl static `agentd` |
| `WORKESTRATE_HOME` / `--home` | user / `workestrate --home` flag | `--home` flag > `WORKESTRATE_HOME` env > `~/.workestrate` | Tool home (registry `config.toml`, `PolicyConfig`); orthogonal to the MSB home |
| `HOME` | user / OS | Expands at wrapper execution time for `MSB_HOME=$HOME/.microsandbox/current` | Must not be baked at nix build time |

## Schema flow (Rust types → committed schema → distribution)

```
Rust types (serde + schemars)
  control/agentctl/src/config/types.rs        PolicyConfig, WorkloadConfig, ...
  control/agentctl/src/config/registry.rs    Registry.policy
         │
         ▼ generate_schema_pair()
  control/agentctl/src/commands/diagnostics.rs:1017
         │
         ├─► workestrate generate-schema --output schemas/workestrate.schema.json
         │   workestrate generate-schema --output-workload schemas/workestrate-workload.schema.json
         │   (also: workestrate schemas update — distributes to every consumer location)
         │
         ├─► schemas/workestrate.schema.json  (committed, §4 of ADR 0035)
         │   schemas/workestrate-workload.schema.json
         │
         └─► validate-config staleness warning (P3)
             control/agentctl/src/config/validation.rs — warns when committed schema drifts
             from generate_schema_pair(); not a hard error (format-only drift is P3).
```

Follow-up gap (ADR 0035 §12): `Registry.policy` (`<home>/config.toml`) derives `schemars::JsonSchema` but no `registry.schema.json` is yet distributed — `validate-config --home` schema check is deferred.

## Formatting standardization — tombi

TOML formatting is standardized via **tombi** (`tombi.toml`, `1.2.5+`, `toml-version = "v1.0.0"`):

- `include = ["config.reference/**/*.toml", "control/agentctl/Cargo.toml", "tombi.toml"]`
- `exclude = ["control/agentctl/tests/fixtures/**", ".tmp/**", "target/**"]` — hostile fixtures deliberately out of schema + format gates.
- `[format.rules] indent-width = 2, line-width = 100`
- `[schema] enabled = true, strict = true` — lint applies **only** to `config.reference/workestrate.toml` (`[[schemas]] path = "schemas/workestrate.schema.json" include = ["config.reference/workestrate.toml"]`).
- Offline-pinned: `[schema.catalog] paths = []` — no schemastore remote.

Run `tombi format` / `tombi lint` (or `nix fmt`) — the ADR 0035 policy examples are tombi-formatted.

## Pointers

- **ADR 0037** — `docs/migration/50-decisions/0037-msb-state-generations.md` (state generations decision note).
- **`control/agentctl/src/microsandbox/generation.rs`** — generation keys + the ONE resolution rule (Rust mirror).
- **`scripts/msb-generation-converge.sh`** — converge/GC owner; **`tests/msb-generation-converge/`** — host-runnable fixture tests.
- **`workestrate doctor` `generation` row** — baked-vs-resolved generation status.
- **ADR 0035** — `docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md` (network ladder).
- **ADR 0029/0031 + mount-policy docs** — precedent for collect-and-compile + `deny_unknown_fields`.
- **ADR 0034** — secrets ladder precedent for rungs + provenance.
- **`nix/packages/agentctl.nix:111-136`** — authoritative wrapper contract.
- **`flake.nix` (`msb-wrapped` + `devenv.shells.default` `enterShell`)** — msb wrapper guard + devshell MSB staging.
