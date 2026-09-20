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

## Service startup TCP readiness budget

For slow-starting services, set the budget on the service being started:

```toml
[workloads.example-service]
kind = "service"
readiness_timeout_secs = 120
```

`readiness_timeout_secs` accepts integer seconds from 1 through 3600. Omission
keeps the 15-second default. Higher layers may replace this scalar while
inheriting `kind` from a lower layer; the effective workload must be a service.
Inspect the effective value and declaring layer with
`workestrate workload plan <service> --show-source`; omission is shown as
the 15-second core default. This orchestration setting is not added to the VM plan or image
identity merely for display.

The budget applies after starting a service for a dependency or bare
`workestrate workload up` batch. It includes registry polling (at most five
seconds) and TCP connection checks for all of that instance's published ports,
under one absolute deadline. The selected service's setting applies, not the
dependent's. Async connection attempts and retry sleeps are cancelled when
their waiting future is dropped; no background retry worker remains. Ordinary
process signal behavior is unchanged. Local registry file reads remain the
existing synchronous reads; this is not a hard filesystem-I/O deadline.

A connection proves only TCP reachability, not HTTP health, completed NixOS
activation, or a ready Nix daemon. No published ports means no TCP readiness
requirement; no application probe is invented. Existing-instance reuse keeps
its separate 500-millisecond health probe. Named detached startup keeps its
existing child startup check, without adding a service TCP wait. Image builds,
VM creation, and guest command execution timeouts are unchanged. Changing this
orchestration budget does not resize, restart, or replace an existing VM.

## Managed guest root-disk capacity

Workloads may request the capacity of a new OCI image's managed writable layer:

```toml
[workloads.builder]
kind = "service"
image = { recipe = "registry", ref = "example:latest" }
memory_mib = 8192
root_disk_mib = 16384
command = ["/bin/builder"]
```

`root_disk_mib` is an integer number of MiB (1 MiB = 1,048,576 bytes), not RAM,
host build scratch, the read-only OCI image size, or a host-mounted directory's
quota. It accepts the SDK's nonzero u32 representation: 1 through 4,294,967,295.
Zero, negative, fractional, unit-suffixed and unrepresentable values are rejected.
Representability does not reserve host space or guarantee that the filesystem
formatter/backend can satisfy the size; allocation can still fail, including
for a size too small for its metadata or insufficient host capacity.

Omission preserves existing configuration/plan bytes, hashes and backend defaults
(normally a 4,096 MiB managed writable layer). A higher layer replaces the whole
scalar; the effective declaration is shown in plan JSON/text and source
provenance, and explicit values change the configuration hash. No new setting is
added implicitly to existing workloads.

This scalar supports managed OCI storage only. Host-directory and disk-image
root filesystems are refused, as are backend or explicit OCI writable modes
selecting tmpfs, flat storage or a user-owned disk image: a capacity request must
not silently change storage mode. The SDK cloud request can carry the managed
size; that wire support is not evidence of provider allocation or resize support.

Only fresh creation applies the builder setting. Reuse and restart keep the
stored disk untouched and retain the existing conflict/skew policy. With an
explicit request, these paths report desired capacity and the retained SDK
declaration when readable, or an unknown/default declaration otherwise. Matching
declarations are not measured capacity or readiness. Recreate an owned instance
explicitly to enforce a changed value, after preserving any data it owns; this
setting performs no live resize, reformat, automatic migration or data recovery.

## Guest ownership of bind mounts

An optional paired numeric owner sets the fallback UID/GID presented inside the
guest for a declared bind mount:

```toml
[[workloads.database.mounts]]
host = "workspaces/database-state"
guest = "/data"
mode = "rw"
owner = { uid = 61040, gid = 61040 }
```

Bare workload capsules use the same fields under `[[mounts]]`. Both IDs are
required integers in the SDK's u32 range, 0 through 4,294,967,295. Names, partial
pairs and unknown owner fields are rejected; no host or guest account lookup is
performed. An explicit `0:0` is valid and distinct from omission.

This forwards native stat virtualization, not host `chown`. It applies to files
without per-file virtual stat overrides; existing guest ownership overrides take
precedence. Native private host permissions, read-only mode, mount path policy,
source-path validation and symlink restrictions remain unchanged. It does not
grant new host access or make a read-only bind writable. Host source creation
and seed-file ownership are not changed by this declaration.

Omission preserves old plan bytes, configuration hashes and native defaults.
The owner appears in plan JSON/text and changes configuration identity. Mount
arrays still replace wholesale between layers; omitting `owner` in a replacing
row returns to the native default rather than inheriting the previous owner.

This is a create-time setting. Starting/reusing an existing sandbox retains its
persisted mount configuration under the existing skew policy. Recreate an owned
instance explicitly to change its view, preserving its state first. This field
does not migrate existing virtual ownership metadata or prove that an application
can read/write its data; validate the exact guest service separately.

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
| `WORKESTRATE_CONFIG` / `--config` | user / `workestrate --config` flag | `--config` > non-empty `WORKESTRATE_CONFIG` > explicit legacy XDG selectors > `~/.workestrate` | Config/registry; orthogonal to the MSB home |
| `WORKESTRATE_STATE_DIR` | caller env | Non-empty env > registry `settings.state_dir` > active config state default | Workload state, runtime registries and Workestrate image bookkeeping |
| registry `settings.store_dir` | selected tool-config registry | Explicit setting > active config store default | Managed fleet and source checkouts, not the SDK runtime package |
| `MSB_CONFIG_PATH` | caller env | Explicit backend configuration file | Selects Microsandbox backend configuration separately from Workestrate's config |
| `HOME` | user / OS | Expands at wrapper execution time for `MSB_HOME=$HOME/.microsandbox/current` | Must not be baked at nix build time |

These path controls are already implemented; changing them does not migrate
existing data. Workestrate's `images.json` bookkeeping is under its selected
state directory, while the loaded image store belongs to the selected
Microsandbox backend. A build in the Nix store, an imported runtime image and
persisted workload data have different owners and lifetimes.

Microsandbox backend JSON may select custom roots in addition to `MSB_HOME`.
Inspect the effective backend configuration before assuming every operation
uses one directory; changing Workestrate's `--config` alone is not full isolation.
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

## Foreground service supervision

Foreground service `up` keeps its exec receiver in the foreground task; it does
not detach a log-draining task. A process `Started` event reports process
creation, not application readiness. An unexpected exit, including exit zero,
spawn failure, interruption or terminal-less EOF ends supervision with an
error instead of waiting for Ctrl-C while the service is already dead.
Interruption reason and process-termination evidence remain separate: even an
interruption accompanied by exit zero is not normal completion.

During explicitly requested Ctrl-C shutdown, an exec cancellation may report
unconfirmed process termination even when the separately retained runtime has
already exited. The foreground owner accepts that cancellation only after its
normal SDK stop succeeds and the original owned local runtime's native child
wait reports exit zero. It does not look up a replacement by name or PID, and
the exec interruption itself is not rewritten as an exited event. A terminal
backend row alone, non-owner/cloud stop, cancellation timeout, transport failure,
unexpected service exit, unsuccessful runtime exit or incomplete SSH retirement
still fails. The original primary failure is never erased by later VM shutdown.

The exec request and initial `Started` event share a 30-second startup budget.
There is no corresponding lifetime cap on a running service. Ctrl-C closes new
SSH admission without releasing its CID/socket reservation and triggers cleanup
of the original exec session and retained sandbox. Cancellation observation has
a ten-second budget; stopped-state
observation has a separate 45-second budget; joined SSH-shim retirement gets two
seconds. Timeout or signal delivery is not termination or retirement proof.
Primary service failure and cleanup failures are reported together.
During cancellation observation the receiver is retained but not concurrently
drained; final buffered shutdown log lines may be discarded. The SDK queue stays
bounded, and missing log output is not treated as termination evidence.

Only successful stopped-state observation authorizes SSH reservation release.
On stop failure/timeout the fenced shim's original loop thread waits, retaining
the reservation; it can finalize after a later explicit retirement request, or
exit without releasing it when its owner is dropped. The SSH handle remains in
the borrowed foreground config after incomplete cleanup, allowing its owner to
decide whether to retry. The CLI currently
returns an error and drops that config: this requests cancellation but does not
claim joined retirement or release the reserved CID/socket on the strength of a
dropped handle. A future shared owner must retain it for retries. Legacy broker
egress shutdown still has a synchronous unbounded join, outside these independent
async budgets; this is not a total-shutdown deadline or a complete shared broker
lifecycle implementation. Interactive `exec` retains its existing TTY/attach and
detach behavior.

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
instance/generation routing, exact credential selection and shared broker
ownership still need their own integration. The pinned Microsandbox dispatches
configured SSH endpoints before direct upstream connection or bytes. Other response
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

## SSH launch identity

Broker DLP patterns contain raw credential material. They are no longer added
to the ordinary workload's Microsandbox builder: a bootstrap field is delivered
to its target VM even when excluded from the serialized sandbox spec. Pattern
resolution is a separate broker-only operation and follows the same secret-ID
to `source_env_var` mapping as sealed key custody, without process-environment
fallback. Preparing patterns does not install them or make custody ready; they
must reach the direct managed broker connection. Ordinary guest bootstrap is
not a broker management channel.

For fresh workloads with compiled SSH credentials, Workestrate reserves a CID
under its existing registry lock before creating the VM. It prepares a distinct
`divert-<cid>.sock` endpoint and passes the endpoint and reserved CID together to
the SDK. Microsandbox assigns that CID to libkrun and checks it before guest
execution; network slots only select network addresses. The host shim rejects
a divert prelude whose CID differs from its reserved runtime context.

The launch setup handle owns its listener and registry binding. Failed create,
registration or cancellation drops that handle; stopping one launch does not
remove another launch's listener. Cleanup expires only the exact instance and
launch token, not a replacement that has acquired the same numeric CID. Existing
files at a newly reserved endpoint are refused, not overwritten. Shutdown wakes
the owned thread directly, including when its socket pathname has disappeared;
cleanup preserves a replacement inode at that pathname. This is an
internal path/lifetime change: old running workloads using the singleton endpoint
are not automatically attached to the new listener. No live-home migration is
performed by a build.

Host dispatch setup is not broker readiness. It no longer tries to provision the
workload's console as though that were the shared broker's control channel.
Acknowledged multi-instance policy installation, broker boot/supervision,
generation-bound session context, managed SSH host-certificate trust and reuse/
restart reconciliation remain required before claiming operational custody.
The current timestamped prelude is not a launch-generation proof, and listener
teardown does not yet cancel every admitted relay worker.

CID registry records are authorization state, not optional diagnostics. A corrupt,
unreadable, misnamed or invalid record now prevents loading the registry; it is
never skipped to make a number appear free. Allocation also refuses malformed,
out-of-range, exhausted or rolled-back counters and a missing counter when
records already exist. Explicit bindings and automatic allocations share the
same counter; reserved CIDs, including the wildcard, cannot identify a launch.

Writers publish complete owner-only files using a synced temporary file and
same-directory rename, then sync the parent. Allocation advances its durable
counter before publishing a binding, so a failed publication consumes a number
rather than reusing it. Existing valid record shapes still decode. These
guarantees require updated writers: do not run older registry writers against
the same state concurrently. The existing lock-recovery implementation still
needs separate concurrency validation.

On a registry error, stop launches and preserve the files for inspection. Do not
delete a counter or broken record to bypass the error: first quiesce all writers
and reconcile reservations against actual running VMs, then restore a verified
backup or explicitly repair the affected state. There is no automatic live-state
repair or migration, and a fresh empty state directory is a new authority domain,
not a way to recover still-running workloads.

## Explicit host control owner

`workestrate control serve --state-dir /absolute/private/control --instance NAME`
serves one Unix endpoint for selected, already-running instances. Repeat
`--instance` to retain several launches on the same dispatcher. The supplied
control-state directory must already exist, be canonical, private and owned by
the invoking operator. Use `--initialize` only for a new desired store; reopening
never treats a missing or corrupt store as empty. This path does not replace
`MSB_HOME`, start workloads, change active context or decrypt SSH key material.

Before its first await, the command captures the active configuration and each
selected registry/workload association and compiles immutable policy ceilings.
It then uses one existing local backend to retain the exact running SDK objects;
connection failure does not start or replace a VM. JSON startup output names the
endpoint and public launch identities. The endpoint initially authenticates only
the local operator. Guest UIDs are not promoted to operator authority.

The endpoint, durable SSH controller and thin common dispatcher share one owner.
There are at most 16 owned connection tasks, 32 queued requests and 32 retained
exec operations. Existing codec framing and native call bounds still apply.
Each queued request retains its server socket descriptor. Immediately before
dispatch, a fresh nonblocking poll refuses a completely closed client or socket
error, without mistaking a normal write-half-close for cancellation. Closure
after that check or issuance is not atomic cancellation; a call already issued
remains potentially effective and is never replayed when its client leaves. No broker is attached by
this entrypoint yet: SSH desired changes stay unobserved and not ready. It does
not send custody through guest exec or fabricate broker acknowledgments.

The currently wired native capabilities are exact-launch inspection, stop and
owned guest execution. Broker transport, trusted policy preparation and managed
SSH diversion remain private integration seams, not an available deployment
mode. Item-specific lint expectations identify the missing callers and must be
removed when those callers are connected. Eight apply only outside tests; the
native attachment wrapper has no caller in either target because tests exercise
the synthetic transport boundary instead. The ordinary all-target lint gate
still checks these method bodies and their dependencies. No successful native
operation, Hello or Probe substitutes for a complete matching Applied policy.

Ctrl-C and SIGTERM fence request admission and start one fair ten-second drain
of original exec leases and connection tasks. Cancellation acceptance is not
process-exit evidence. Confirmed terminal operations are released; indeterminate
operations, including interruption with unconfirmed termination, remain owned.
If anything is pending, the process stays fenced with the same owner and reports
bounded public IDs/states. Another Ctrl-C or SIGTERM requests another finite
ten-second cleanup attempt only. It cannot reopen admission, extend an in-flight
deadline, replay commands or stop adopted VMs. A retry need not make progress;
persistent uncertainty may require explicit operator escalation. Forced process
exit is not clean retirement. Original service and cleanup failures remain
reported even if a later drain finishes.

Endpoint and desired-store identity loss both fence the owner. Endpoint identity
is observed while dispatch is pending. Desired state is checked before/after
dispatch and immediately after native launch verification, before another
stateful effect. Replacement during an already-issued native call is observed
on its return or existing five-second call budget, not by an independent store
watcher. Any uncertain operation remains on the same owner for cleanup. Broker
admission fencing must additionally use the owned direct link when that link is
attached; this broker-absent entrypoint makes no such runtime claim.

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
             template, initialized config and qualifying registered fleets
```

All three schemas are generated and committed. `registry.schema.json` describes
the config `config.toml`, including its policy. Rust types remain the runtime
validator; JSON Schemas are editor/tooling projections, not a separate source
of policy truth.

`workestrate schemas update` distributes all three artifacts idempotently to
the tool template when available, an existing config, and registered config
repos already carrying a `schemas/` directory. Missing/unmanaged destinations
are skipped. `--repo <name>` selects only that registered repo, excluding the
template and config. `--check` reports missing/stale copies without writing and
fails on drift. Use an explicit config when checking deployed consumers.
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
