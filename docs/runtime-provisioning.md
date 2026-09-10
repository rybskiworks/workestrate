# Runtime provisioning — MSB home, wrapper contract, schema flow

> **Companion to ADR 0035.** This doc owns the canonical MSB home, the wrapper contract, the env override matrix, and the schema distribution flow. ADR 0035 owns the network policy ladder. The MSB state-generations model is summarized below (decision note: `docs/migration/50-decisions/0037-msb-state-generations.md`).

## Canonical MSB home (state generations)

msb (microsandbox) discovers its home via `MSB_HOME`, resolved per
`microsandbox_utils::resolve_home`: **non-empty `MSB_HOME` verbatim** (empty
treated as unset), else **`$HOME/.microsandbox`**, else `./.microsandbox`.
The packaged Workestrate and `msb-wrapped` entry points default that variable
to a **generation-keyed** home (see the next section): the runtime home is the
`$HOME/.microsandbox/current` symlink. Explicit generation convergence flips
it atomically between `generations/<hash12>` dirs when adopting a different
pinned msb build; building or entering a shell does not perform that flip.

| # | Home | Path | When active | Purpose |
|---|------|------|-------------|---------|
| 1 | **canonical runtime default** | `$HOME/.microsandbox/current` → `generations/<hash12>` | Runtime (guarded `wrapProgram --run` in `nix/packages/agentctl.nix` postInstall) | Cache, db, state and `sandboxes/`, normally one dir per pinned msb build. The guard defaults unset AND empty `MSB_HOME` while honoring an explicit non-empty caller override. Shell expansion happens at wrapper execution time, not build time. |

## Explicit guest init

The optional workload `init` block forwards Microsandbox's existing typed PID 1
handoff. It does not change the separate workload command run through exec-stream:

```toml
[workloads.example]
kind = "service"
image = { recipe = "registry", ref = "example:latest" }
workdir = "/"
command = ["/bin/example-service"]

[workloads.example.init]
mode = "handoff"
cmd = "/init"
args = []
env = { container = "microsandbox" }
```

The image must supply the executable and its closure. For NixOS, use the image's
generated stage-2 entry point, not a later `exec` of bare systemd. Init currently
inherits the SDK workdir; choose a directory that exists before activation.
Arguments and environment are guest literals: no host lookup, shell expansion,
secret resolution or OCI auto-detection is performed. Do not put credentials in
this public plan/config block. Unknown fields/modes, nonabsolute executable paths,
backslashes, NUL bytes and invalid environment names are rejected.

Omitting `init` preserves existing plan bytes, configuration hashes and agentd
behavior. A higher layer's entire init block replaces the lower declaration;
`init = { mode = "agentd" }` explicitly resets a handoff. The declared block
appears in plan JSON/text and source provenance, and changes its configuration
identity. Explicit reset and omission are distinct declarations even though both
use the default PID 1.

This is a create-time setting. Reusing or starting an existing sandbox retains
its persisted SDK specification; the existing skew policy may only warn.
Explicitly recreate an owned instance to apply a new init/image configuration.
A returned sandbox handle proves neither completed NixOS activation nor a ready
Nix daemon/application. Check those services separately. Guest-init shutdown is
owned by the pinned runtime; successful forwarding alone does not certify clean
systemd poweroff or the runtime's mixed-libc shutdown behavior.

The pinned runtime now prepares a volatile `/run` before runtime writers and
uses receiver-owned systemd poweroff for init handoffs. Existing runtime mounts
and explicit user mounts retain their documented precedence. These changes do
not activate an arbitrary image or prove completed service startup/shutdown;
validate the exact image and observe guest service flush and runtime termination.

`workestrate versions` identifies the paired agentd by a content hash without
executing it. Agentd has no version-only command: invoking it starts guest
bootstrap. The existing JSON `agentd.version_or_sha` field is retained and reports
`sha256:<12 hexadecimal characters>` for a readable executable regular file,
or an availability/executable marker otherwise. Microsandbox's host CLI still
supports its ordinary `--version` probe.

## Immutable build inputs are not runtime homes

Nix builds and the default development shell supply the SDK with
`MSB_BUILD_RUNTIME=${microsandbox}` and
`MSB_AGENTD_PATH=${microsandbox}/libexec/agentd`. The former is an immutable
package directory containing the runtime and its libraries; the latter is the
static guest agent. An invalid explicit build runtime fails rather than
downloading a replacement. Neither variable selects mutable VM state.

There is no SDK runtime staging under `$HOME/.cache/ai-workbench-msb` or a
build-time `$TMPDIR/.microsandbox`. The Nix build prepares its SDK source
symlink and Cargo configuration inside its build tree. `just shell` prepares
the checkout's SDK symlink and exports these build inputs plus
`CARGO_TARGET_DIR`; it does not create a runtime home, export `MSB_HOME`, build
agent workloads, install hooks or run migrations. An inherited `MSB_HOME`
remains the caller's runtime-state choice.

`just bootstrap` supplies pinned tools without application/runtime packages,
SDK link setup or SDK build-input variables. Use it when the application does
not yet build. See [Nix builds](nix-build.md) for both entry points.

Pre-convergence hosts may still carry state under the legacy devshell path
(`$HOME/.cache/ai-workbench-msb/db/msb.db`): `scripts/migrate-msb-home.sh`
migrates it to the canonical home (newest-DB-wins, timestamped backups,
`--check-only`/`--dry-run`/`--rollback <ts>`), `workestrate doctor` (the
`msb` check) flags skew/unmigrated/downgrade states, and
`scripts/host-provision.sh` runs the generation converge best-effort
between binary sync and doctor — the converge script delegates the
cache-home migration to `migrate-msb-home.sh` first, then absorbs any
pre-generation root as `generations/legacy`.

These migration scripts are explicit provisioning operations, not build or
shell hooks. Their presence does not mean an existing host has been migrated.
Review their dry-run/check output and rollback behavior before applying them
to existing state; successful package checks do not establish live migration
or SSH custody readiness.

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
- **Socket budget**: Unix endpoints must fit the platform's `sun_path` byte
  limit. The remaining root budget depends on the specific endpoint suffix,
  encoded instance name and any canonicalization/short-path indirection; there
  is no universal 59-character `MSB_HOME` limit. Short generation keys help,
  but do not prove that every endpoint fits. Validate the actual constructed
  endpoints when choosing a custom runtime root, especially with non-ASCII or
  long instance names. Keep reloadable image/cache storage distinct from short
  runtime socket paths where the backend configuration supports it.
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

- **`--set MSB_PATH`** — binary from `microsandbox-fork.packages.<system>.microsandbox`. The SDK patches, runtime and static agentd all come from this same pinned flake input; Workestrate no longer duplicates the fork's runtime build recipes. The fork package currently bundles `libkrunfw.so.5.6.1` from the fixed-hash `v0.6.8` release tarball. Building the separate libkrunfw fork does not automatically replace that firmware.
- **`--set MSB_AGENTD_PATH`** — baked musl static `agentd` path paired with the runtime. Nix builds and the default shell also supply this path directly for the filesystem crate's prebuilt branch.
- **`--run MSB_HOME`** — guarded default: honors an explicit non-empty `MSB_HOME` override while defaulting unset AND empty to `$HOME/.microsandbox/current` (the `current` generation symlink) at wrapper execution time so `$HOME` expands per-user, not per-build. No build-staging path is baked here.
- **`--prefix PATH : sops`** — `sops` binary for `workestrate secrets` (ADR 0034).

The default devshell (`flake.nix` `devenv.shells.default` `enterShell`) exports
immutable SDK build inputs without staging or selecting runtime state:

```bash
export MSB_BUILD_RUNTIME="${microsandbox}"
export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"
```

## Env override matrix

| Path control | Set by | Precedence | Effect |
|---------|--------|------------|--------|
| `MSB_HOME` | caller env, else wrapper `--run` guard | Non-empty caller value wins; packaged entry points default unset/empty to `$HOME/.microsandbox/current`. Direct unwrapped SDK use instead defaults to the `.microsandbox` root described above. | Mutable runtime home, independent of SDK compilation |
| `MSB_BUILD_RUNTIME` | Nix build / default devshell | Explicit immutable package directory, validated by the SDK build script | Offline compilation input; never a mutable state directory |
| `MSB_PATH` | wrapper `--set` | The packaged wrapper fixes the runtime executable rather than honoring a caller replacement | Its store-path hash segment keys the managed state generation |
| `MSB_AGENTD_PATH` | wrapper `--set` / build preBuild / default devshell | Exact paired static guest agent | SDK filesystem prebuilt input and runtime pairing |
| `WORKESTRATE_HOME` / `--home` | user / `workestrate --home` flag | `--home` > non-empty `WORKESTRATE_HOME` > explicit legacy XDG selectors > `~/.workestrate` | Tool home/registry; orthogonal to the MSB home |
| `WORKESTRATE_STATE_DIR` | caller env | Non-empty env > registry `settings.state_dir` > active home's state default | Workload state, runtime registries and Workestrate image bookkeeping |
| registry `settings.store_dir` | selected tool-home registry | Explicit setting > active home's store default | Managed config-repo and source checkouts, not the SDK runtime package |
| `MSB_CONFIG_PATH` | caller env | Explicit backend configuration file | Selects Microsandbox backend configuration separately from Workestrate's tool home |
| `HOME` | user / OS | Expands at wrapper execution time for `MSB_HOME=$HOME/.microsandbox/current` | Must not be baked at nix build time |

These path controls are already implemented; changing them does not migrate
existing data. Workestrate's `images.json` bookkeeping is under its selected
state directory, while the loaded image store belongs to the selected
Microsandbox backend. A build in the Nix store, an imported runtime image and
persisted workload data have different owners and lifetimes.

Microsandbox backend JSON may select custom roots in addition to `MSB_HOME`.
Inspect the effective backend configuration before assuming every operation
uses one directory; changing Workestrate's `--home` alone is not full isolation.
Disposable tests should supply a separate `MSB_CONFIG_PATH` containing `{}`,
explicit tool/state/MSB roots and a scrubbed environment. Path reconciliation
and migration/query side effects still require dedicated regression coverage.

## Nested virtualization and deployment limits

The pinned runtime distinguishes traversal-only directories from denied
regular files. Allowing a directory's descendant does not make a denied
regular file traversable, listable or readable. Already admitted file handles
and directory snapshots retain their admission semantics; this is not live
revocation. Existing VMs do not acquire a new runtime or policy merely because
the CLI pin changes: recreation is a separate, explicit lifecycle operation.

The fork family can run nested workloads on suitable Linux x86_64 hosts with
the bundled firmware; guest KVM is not categorically a future-only feature.
Request it explicitly through `virtualization.nested`, and evaluate host
capability and operator policy for the exact runtime/firmware pair.

The current Linux libkrun implementation does not enforce the forwarded off
flag, so a default/off request is not proof that usable guest KVM is absent.
Successful permitted nested execution does not validate default-off confinement,
reuse after policy changes, SSH broker custody or safe credential deployment.
ADR 0036 retains the design history; historical future-firmware wording is not
an observation of the current running artifact.

## SSH broker receipt bounds

The host broker's signing, divert and egress ingress paths allow ten seconds
to receive one complete length-prefixed frame. Header and payload share one
deadline; partial progress does not restart it. Existing size limits remain
8 MiB for signing/divert and 64 KiB for egress. Incomplete frames are closed,
not retried as direct connections. A stop request is checked during frame
receipt, at intervals of at most 100 ms of socket wait, so a stalled accepted
peer does not require EOF before the listener can stop. Scheduler delays are
not covered by that polling interval.

Receipt restores the stream's previous read timeout before a successful
handoff, so these limits do not impose a ten-second SSH session lifetime or
idle timeout. Divert freshness and audit timestamps are sampled after the
complete prelude is received; the existing 300-second skew limit and the
unavailable-clock refusal are unchanged. CLI options, credential schemas and
wire encodings are unchanged.

Console epoch provisioning also has a ten-second asynchronous budget, shared
by dialing, handshake, request write and acknowledgement. Timeout or caller
cancellation drops the one-shot SDK client and aborts its transport tasks;
a successful handshake does not leave an unlimited reply wait. The persisted
wire sequence is bumped before this exchange and is not rolled back on failure,
so an explicit retry gets a new epoch. Synchronous registry locking and disk I/O
are outside this console budget. Low-level helpers using a borrowed console
channel do not impose this ownership or timeout contract themselves.

These bounds do not establish operational SSH custody. Broker boot, trusted
instance/generation routing, pre-protocol diversion, exact credential selection
and shared listener ownership still need their own integration. Other response
deadlines, relay-worker quotas/cancellation and bounded upstream resolution are
also separate from frame receipt. In particular, listener shutdown is not proof
that all previously admitted sessions have drained.

## SSH credential context

SSH continues to flow from configuration into the compiled `CredentialsPlan`;
there is no separate user-maintained routing or grant catalog. The implemented
`credentials.ssh` representation contains independent host and username lists.
Their existence does not settle the destination–username binding semantics of
the proposed scope-local `ssh.toml` format. That schema and its migration need
an explicit design decision before implementation.

The host shim selects complete compiled credential records for the resolved
instance and endpoint, then hands those records to its relay. Names, material
references, host/user/port lists, custody bindings and per-credential violation
policies remain separate and unchanged. Endpoint selection is not authorization
to authenticate as a username or use a key. Missing credential context refuses
relay; any matching broker-bound record retains the existing broker-first
behavior, without a direct fallback if the broker is unavailable.

This context is currently an in-process Rust contract, not a new serialized
configuration or protocol. The older Microsandbox network projection and broker
prelude still do not carry all this information. Completing those contracts,
checking session A's requested username against trusted instance context, and
selecting session B's key remain necessary before SSH custody is operational.
Do not infer pairing rules, combine separate credentials' permissions, or
interpret an endpoint allow decision as a successful SSH authorization test.

## Schema flow (Rust types → committed schema → distribution)

```
Rust types (serde + schemars)
  control/agentctl/src/config/types.rs        PolicyConfig, WorkloadConfig, ...
  control/agentctl/src/config/registry.rs    Registry.policy
         │
         ▼ generate_schema_triple()
  control/agentctl/src/commands/diagnostics.rs
         │
         ├─► workestrate generate-schema --output schemas/workestrate.schema.json
         │     --output-workload schemas/workestrate-workload.schema.json
         │     --output-registry schemas/registry.schema.json
         │
         ├─► schemas/workestrate.schema.json  (committed, §4 of ADR 0035)
         │   schemas/workestrate-workload.schema.json
         │   schemas/registry.schema.json
         │
         └─► schemas update / schemas update --check
             template, initialized tool home and qualifying registered config repos
```

All three schemas are generated and committed. `registry.schema.json` describes
the tool-home `config.toml`, including its policy. Rust types remain the runtime
validator; JSON Schemas are editor/tooling projections, not a separate source
of policy truth.

`workestrate schemas update` distributes all three artifacts idempotently to
the tool template when available, an existing tool home, and registered config
repos already carrying a `schemas/` directory. Missing/unmanaged destinations
are skipped. `--repo <name>` selects only that registered repo, excluding the
template and home. `--check` reports missing/stale copies without writing and
fails on drift. Use an explicit tool home when checking deployed consumers.
Repository `just verify` checks repository-owned schema copies separately;
neither it nor shell entry updates live consumers.

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
- **`nix/packages/agentctl.nix`** — immutable SDK build inputs and wrapper contract.
- **`flake.nix` (`msb-wrapped` + `devenv.shells.default` `enterShell`)** — runtime wrapper guard and development build-input exports.
- **msb state model (both ends)** — `docs/nix/msb-state-model.md` (consumer side) → fork `nix/README.md` (producer side).
