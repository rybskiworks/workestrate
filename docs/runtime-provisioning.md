# Runtime provisioning — MSB homes, wrapper contract, schema flow

> **Companion to ADR 0035.** This doc owns the three MSB homes, the wrapper contract, the env override matrix, and the schema distribution flow. ADR 0035 owns the network policy ladder.

## Three MSB homes (msb three-homes)

msb (microsandbox) discovers its home via `MSB_HOME`. workestrate stages three distinct homes depending on lifecycle phase — only one is active per invocation.

| # | Home | Path | When active | Purpose |
|---|------|------|-------------|---------|
| 1 | **devshell cache** | `$HOME/.cache/ai-workbench-msb` (`_msb_home` in `nix/devshells/default.nix`) | `nix develop` shellHook | Offline `cargo check` builds — `cargo` runs without network by pointing at a staged `msb`/`libkrunfw.so*` cache. Ephemeral per-shell (`ai-workbench-msb-$$`). |
| 2 | **msb-wrapped persistent** | `$HOME/.microsandbox` (`~/.microsandbox`) | Runtime (`wrapProgram --run 'export MSB_HOME="$HOME/.microsandbox"'` in `nix/packages/agentctl.nix:133-135`) | **Canonical runtime home** — cache, db, state, `sandboxes/`. The SDK (`msb` crate) requires a stable `MSB_HOME` at `workestrate workload up/exec/...` time. Shell expansion at wrapper execution time (not build time). |
| 3 | **build TMPDIR** | `$TMPDIR/.microsandbox` (`export MSB_HOME=$TMPDIR/.microsandbox` in `nix/packages/agentctl.nix:111`) | `nix build .#workestrate` preBuild | Hermetic build — the fork's `build.rs` finds `msb` + `agentd` via `MSB_HOME`/`MSB_AGENTD_PATH` without network downloads. Wiped after build. |

Additional override: `MSB_AGENTD_PATH = ${microsandbox}/libexec/agentd` (musl static `agentd`) satisfies the fork's filesystem crate `build.rs` prebuilt branch (`nix/packages/agentctl.nix:117`).

## Wrapper contract (`nix/packages/agentctl.nix`)

The `agentctl.nix` wrapper (built `workestrate` binary, currently `0.1.0`) bakes the runtime contract at build time:

```nix
wrapProgram $out/bin/workestrate \
  --set MSB_PATH "${microsandbox}/bin/msb" \
  --prefix PATH : ${pkgs.sops}/bin \
  --run 'export MSB_HOME="$HOME/.microsandbox"'
```

- **`--set MSB_PATH`** — baked `0.6.16` `msb` binary path (`nix/packages/microsandbox.nix:54`, fork rev `78fb3ed1`). The SDK spawns `MSB_PATH`; it must be a live-resize-capable `msb` (linux `libkrunfw.so.5.6.1` from the `v0.6.8` release tarball, `nix/packages/microsandbox.nix` Branch A).
- **`--run MSB_HOME`** — conditional export: `export MSB_HOME="$HOME/.microsandbox"` runs at wrapper execution time so `$HOME` expands per-user (not per-build). The devshell's ephemeral `_msb_home` and the build's `$TMPDIR/.microsandbox` are **not** baked here.
- **`--prefix PATH : sops`** — `sops` binary for `workestrate secrets` (ADR 0034).

The devshell (`nix/devshells/default.nix:130-131`) mirrors this for offline checks:

```bash
export MSB_HOME="$_msb_home"   # /run/user/*/ai-workbench-msb-$$ or $HOME/.cache/ai-workbench-msb
export MSB_PATH="$_msb_home/bin/msb"
export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"
```

## Env override matrix

| Env var | Set by | Precedence | Effect |
|---------|--------|------------|--------|
| `MSB_HOME` | wrapper `--run` / devshell shellHook / build preBuild | **build preBuild > devshell > wrapper** (only one active per phase) | Where msb reads/writes cache, db, `sandboxes/` |
| `MSB_PATH` | wrapper `--set` / devshell shellHook | Explicit path to `msb` binary; `cargo` build.rs and SDK use it | Must point at the pinned `0.6.16` binary |
| `MSB_AGENTD_PATH` | build preBuild / devshell shellHook | Guest init binary path | Fork's `build.rs` prebuilt branch; musl static `agentd` |
| `WORKESTRATE_HOME` / `--home` | user / `workestrate --home` flag | `--home` flag > `WORKESTRATE_HOME` env > `~/.workestrate` | Tool home (registry `config.toml`, `PolicyConfig`); orthogonal to MSB homes |
| `HOME` | user / OS | Expands at wrapper execution time for `MSB_HOME=$HOME/.microsandbox` | Must not be baked at nix build time |

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

- **ADR 0035** — `docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md` (network ladder).
- **ADR 0029/0031 + mount-policy docs** — precedent for collect-and-compile + `deny_unknown_fields`.
- **ADR 0034** — secrets ladder precedent for rungs + provenance.
- **`nix/packages/agentctl.nix:111-135`** — authoritative wrapper contract.
- **`nix/devshells/default.nix:130-131`** — devshell MSB staging.
