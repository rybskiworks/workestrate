# Operating model

Use this guide to update a fleet, try a development branch, inspect running
instances, and choose a teardown scope. Start with [getting started](getting-started.md)
for first-time setup or the [command reference](cli.md) for command syntax.

The sections below explain content pins, fleet selection, image retention, and
instance identity. The [mechanism index](#appendix-mechanism-index) points to
their implementations; the [architecture decisions](migration/50-decisions/README.md)
record the rationale. For host preparation, credentials, and runtime acceptance,
see [runtime provisioning](runtime-provisioning.md), [secrets](secrets.md), and
[testing](testing.md).

## 1. Stable deployments and content pins

`main` is the stable integration branch. A deployment combines two pins:

* **Content pin** — registry entries carry `ref` (typically `"main"`) and
  the generated lockfile `workestrate.lock` records the exact rev each
  entry was resolved to. Runtime verbs consume that committed content.
* **Binary pin** — the installed binary comes from the nix profile
  (`scripts/host-provision.sh` runs `nix profile install .#workestrate`
  when the profile is stale) and reports its build rev via
  `workestrate --version` (`0.1.0-<rev>`).

The lockfile is v2 (ADR 0032 addendum §Config source model; ADR 0025(e)):
each `[fleets.<name>]` entry carries `{url, ref, rev, sha, fetched_at}` —
`rev` is the pin, `sha` the commit the content archive was produced from
(equal to `rev` today), `fetched_at` the RFC3339 UTC resolution time.
Additional refs of the same repo are pinned per-ref under
`[fleets.<name>.refs.<ref>]` with `{rev, sha, fetched_at}` fields; here `rev`
records the requested ref and `sha` records its resolved commit.

Manage this generated file through `fleet add`, `fleet update`, `fleet remove`,
`config init`, and `config clone`. Consuming an unpinned entry or ref records
its first resolution and prints a notice. To update a registered fleet's primary
pin, run `workestrate fleet update <name>`; omitting the name updates all
registered Git-backed fleets. Commit or stash changes in their managed clones
first. Local plain-path fleets are consumed directly and skipped by this update.

The registry supplies each entry's `ref` and recorded `rev`. For development,
`--config-ref <ref>` or a workload's inline ref selects an additional pinned
revision while preserving the primary pin.

## 2. Dev by branch name

Select a development revision in either of two forms:

* **Whole-config override** — `workestrate --config-ref feat-x <verb>`
  resolves every git-backed config entry at that ref (via the archive
  cache). A branch ref contributes a fleet-name candidate, subject to the
  [selection order below](#7-fleet-model-quick-reference). A commit SHA selects
  content without supplying a fleet-name candidate.
* **Per-workload inline override** — `workestrate workload up example-service:feat-x@canary`.
  Grammar `name[:ref][@id]`: `:` = config branch, `@` = instance id. The
  named workload's capsule is read at `<ref>` from its declaring repo's
  archive while everything else stays config-scoped. Dependencies never
  follow the override.

The ref must resolve in the managed clone. Its first use adds a per-ref lock
entry with a notice; later invocations reuse that pinned commit. Selecting a
branch name therefore selects a recorded revision of that branch.

Use `--instance <id>` to select an instance independently of a ref, including
for `down` and `logs`. The inline `@id` form belongs to a combined
`name:ref@id` override. Bare `name@id` and `down`/`logs` with `:ref` are
rejected.

Instance-id precedence, highest first: explicit
`--instance <id>`; then inline `@id`; then `--new` allocation or the
`:ref`-derived sanitized id (when both are given, `--new` wins the id
while the ref still drives the substitution); then per-dir derivation
(`strategy = "per-dir"` only); then the strategy default (`parallel`
auto-slug; otherwise the singleton).

The `:ref`-derived id is the ref run through `sanitize_instance_id`
(lowercased, non-`[a-z0-9]` runs collapsed to `-`, capped at 32 chars,
fail-closed on all-symbol or purely numeric results), so `example-service:feat-x`
uses instance id `feat-x` within the selected fleet's workload slot.

## 3. Commit-before-consume

Git-backed entries resolve to `<state>/cache/gitv3/<sha>/`, a
content-addressed directory produced by `git archive <sha>` from the
single managed clone per repository. Commit edits before selecting them for
execution. Archive consumption uses the selected commit independently of
uncommitted edits in that clone. Local plain-path entries load their current
filesystem content with layer branch `local`.

For ordinary Git-backed consumption, resolution checks the lock's entry for the
effective ref, then a matching primary pin, then the registry-recorded `rev`.
If none exists, it resolves the ref and records a pin with a stderr notice.
Existing pins, including those carried forward from a v1 lock, are read without
a notice. After changing a registry `ref`, use `workestrate fleet update <name>`
to refresh its recorded revision: an existing registry `rev` remains a fallback
until it is updated.

## 4. Lock v2 vs old binaries: ordering matters

`LOCK_VERSION` is 2. Reading a v1 lock (version absent or 1) preserves its
version and supplies defaults for newer fields. A lock with a version newer
than the binary supports is a **hard error**:

```
config created by a newer workestrate (lock version N > M supported by this
binary); upgrade this workestrate before using the config at <path>
```

Update the installed binary before using a config written by a newer release.
A binary that supports only v1 refuses a v2 lock.

## 5. Editing branches

Use Git worktrees to edit several branches concurrently. Git-backed runtime
consumption reads the selected revisions from archive directories under
`cache/gitv3/`, using the managed clone as its object database.

## 6. Selecting a fleet in the shell

Use `--fleet <name>` for one command or export `WORKESTRATE_FLEET` for a shell
session. The flag sets the environment variable at CLI entry and propagates the
selection to detached children. Without an explicit selection, the CLI inspects
the configured checkout's branch using the order below.

## 7. Fleet model quick reference

Selection order, first match wins:

1. explicit `--fleet` / `WORKESTRATE_FLEET` — strict semantics: when
   fleets are registered the name must be one of them (hard error
   otherwise); a fleets-less config ignores the env var;
2. `--config-ref <branch>` — a branch-shaped ref becomes the fleet-name
   candidate (a sha implies nothing);
3. checkout branch of the first registry layer;
4. default resolution — `settings.default_fleet`, else bare layers when
   no fleets are registered, else an error requesting a fleet selection.

Branch candidates are normalized to lowercase with punctuation runs replaced by
hyphens. A candidate matching a registered fleet selects that fleet's layer.
Otherwise it supplies the runtime identity while content comes from
`settings.default_fleet`, or from bare layers when no fleets are registered.
Use an explicit `--fleet` to select a registered fleet consistently across branches.

Fleet names must match `^[a-z0-9][a-z0-9-]*$` with no trailing hyphen;
they become `<fleet>-<workload>` slot prefixes. Registry loading reports
invalid names so you can correct the key in `config.toml`. Fleet registration
also validates names. Early directory resolution uses a lenient registry read.
Legacy instance records with `context: null` remain valid with an unknown context.

## 8. Identity and image tags

Identity has three axes (ADR 0032 §Identity model): workload, context,
instance id. **slot** = `<workload>` (no context) or `<ctx>-<workload>`;
**instance** = the slot (singleton) or `slot@id` (parallel / per-dir /
scoped-dep shapes).

Image tags are immutable content addresses: `name:ctx.sha` per build, or
`name:sha` without a context. The image state file's `pointers` map tracks the
current tag for each group, keeping builds in different contexts independent.

Retention is keep-last-N with a cascade (first configured value wins):

```
built-in default N=5  <  config settings image_keep_last
                      <  fleet entry image_keep_last
                      <  workload capsule image.keep_last
```

Any explicitly provided `0` is a hard error (N >= 1: the just-loaded tag
always counts toward N). Enforcement points:

* **prune-on-load** — after a successful image build+load (build mode
  only), older tags in the same `(name, ctx)` group beyond N are removed;
* **`workestrate images gc`** — manual sweep of all recorded groups. It resolves
  N from the fleet, config, and built-in settings because this operation has
  state-file groups rather than workload capsules.

Running sandboxes are never affected: the protection set is every
`image_tag` recorded across all port-registry records
(`${state_dir}/var/run/*.json`). Stale records over-protect until teardown
unregisters them; a pruned tag simply rebuilds-from-store on recreate. If
the protection set is unreadable, gc refuses and prune-on-load skips
(fail-closed in the don't-prune direction).

## 9. Provenance stamps

Instance registry records carry `{image_out_hash, config_hash}` stamps (ADR 0032
§Provenance stamps). The config hash is FNV-1a 64-bit over a canonical
labeled serialization of the plan's **runtime-relevant fields only**:
image, workdir, command, resources, env (sorted, effective post-merge
view), secret names, ports, mounts (including policy fragments and declared
guest ownership), network rules, SSH policy, and explicit guest init and
root-disk capacity.
Ports contribute their guest port and optional name; allocated host ports and
bind IPs are excluded. Seeds refresh through `--reseed`, independently of
staleness: seed declarations and content are excluded, while their destination
mounts remain hashed. Comments, formatting, identity metadata, and orchestration
policy (`on_conflict`, port strategy, `on_skew`) are also excluded.

Display: hashes are stored full-length (16 lowercase hex); human surfaces
show the first 4 hex characters. A stale `ps` row reads
`stale (config xxxx → current yyyy)`; JSON includes a `staleness` object.
Records without comparable stamps omit this information and retain unknown
version status; they do not trigger automatic replacement through `on_skew`.

Disposition is the workload knob `instance.on_skew` with exactly three
accepted values: `warn` (default — proceed and print the divergence),
`replace` (tear down and start fresh), `reuse-silently` (adopt without
comment). Both the child reuse path and the detached-service parent compare
stamps and apply this setting.

## 10. The down ladder

Teardown scopes, narrowest to widest:

```
instance < workload < fleet < config-ref < config (--all) < everything
```

Stop a workload with `workestrate workload down <name>`. Add `--instance <id>`
to select one parallel instance, or `--all-instances` to include every instance
of that workload.

For a broader sweep, use `workestrate down` with exactly one scope selector:
`--all`, `--fleet <name>`, `--config-ref <ref>`, or `--everything`. A bare
`down` reports a usage error. The compatibility alias `down-all` accepts the
same selectors.

Confirmation and selection rules:

* **Managed scopes** (fleet/config-ref/config) require confirmation unless
  `--yes` is given. Interactive use prompts; piped `y`/`yes` also confirms.
  A declined prompt aborts with exit 1.
* **Everything scope** requires `--everything --everything` and confirmation
  that every Microsandbox sandbox, including unmanaged ones, may be stopped.
  Interactive use prompts; non-interactive use requires `--yes`.
* `down --config-ref` validates the ref before teardown: a 40-hex
  SHA is refused ("a sha does not imply a fleet"); an unknown ref errors
  listing the known refs (registry entry refs + lockfile entry refs and
  ref keys).
* Fleet scope uses the record's context, with the `<fleet>-` slot prefix as
  corroborating evidence.

The `--all` and `--everything` scopes include all retained Microsandbox state
generations under `$HOME/.microsandbox/generations/` (ADR 0037). Fleet and
config-ref scopes operate on the current runtime home.

Classification evidence per target: registry record ∨ dashed slot pattern
(`<ctx>-<workload>`) ∨ the `workestrate.log` artifact. The detached-child log
lives at `<state>/logs/<instance>/workestrate.log`; teardown also checks the
legacy sandbox-directory location.

Outcomes: every target goes through the hardened six-step teardown (stop →
wait-exit → remove → unregister → policy-dir → sandbox-dir); per-target
results print under one scope header (`down --all (config): N target(s)`),
JSON wraps them as `{scope, results}`; any failure exits nonzero; an empty
selection exits 0 reporting `0 target(s)`. Unmanaged candidates exist only
under `--everything` and are reported with empty evidence.

`workestrate clean` removes the contents of the state directories `workspaces/`,
`var/`, and `run/`. Stop running workloads before clearing their state; `clean`
does not stop VMs.

## 11. Per-directory instances (`strategy = "per-dir"`)

Set `instance.strategy = "per-dir"` to give each project directory its own
workload instance (ADR 0030 V-addendum §V1). The workload must declare a
cwd-templated mount (`${CWD}` or `${CWD}/...`); configuration validation
checks this requirement. The instance id is `<dirname-slug>-<hash8>` of the
canonical invocation cwd: last path component slugged (lowercased, separator runs collapsed,
≤23 chars) plus the first 8 hex chars of an FNV-1a 64 hash of the full
canonical path. The same directory produces the same id; hashing the full
path distinguishes directories with equal basenames.
Mount hosts and seed targets at or below `workspaces/<workload>-state` gain
the instance-key segment immediately after that root. Other paths, including
`workspaces/<fleet>/<workload>-state`, remain unchanged and can be shared by
multiple instances. Use the supported state-root layout and verify both mount
and seed destinations before running instances concurrently.
Source-gone semantics: the record persists (with `source_dir` recorded at
create) when the directory disappears; such records are ordinary members
of applicable down sweeps. The parent-side staleness comparison applies
`on_skew` when reusing a detached per-directory instance.

## 12. Parallel instances and msb sandbox-name encoding

Workestrate uses `slot@id` identities in registries, CLI commands, and teardown
scopes (ADR 0030). It encodes the name passed to the Microsandbox SDK to fit
the SDK's allowed characters. An SDK-legal name such as `personal-service`
passes through unchanged. Otherwise every illegal character becomes `--` (so
`@` → `--`) and the result gains a `-<fnv1a64 hex8 of the original
identity>` suffix, making encoded names collision-resistant against legal
names that literally contain `--` (`a--b` stays `a--b`; `a@b` becomes
`a--b-<hash>`). Encoded names are clamped to the SDK's 128-byte cap with
the suffix intact.

Directories under `<MSB_HOME>/sandboxes/` use the encoded spelling, such as
`personal-service--canary-<hash8>`. Teardown looks up the encoded name first,
then the legacy raw spelling for older sandboxes. Use registry records to
resolve instance identities; decoding a directory name supplies only a
best-effort display value.

## 13. Known limitations

* **Multi-context staleness under-reporting.** `ps` staleness compares
  each record against the ACTIVE config view only. Instances built under a
  different context/ref than the one active at `ps` time either compare
  against a view they were not built from or resolve as honest-unknown and
  display nothing; drift relative to other contexts is never surfaced.
* **Untracked store tags are invisible to the gc sweep** (open question,
  ADR 0032): `images gc` sweeps groups derived from the state file only; a
  computed-shape tag present in the msb store but absent from the state
  file accumulates until recorded or removed manually.
* **Cross-repo same-name groups lack a sweep warning** (open question,
  ADR 0032): the gc rung lookup for a same-named group is deterministic
  (lexicographically-first record's repo decides) but pathological
  cross-repo collisions produce no warning surface yet.
* **Encoded msb names are collision-resistant, not cryptographic** (ADR 0030
  2026-08-26 addendum): the `-<fnv1a64 hex8>` suffix deters accidental
  collisions, but adversarially constructed identities can still collide.
  Accepted for the local single-operator threat model. Implication: never
  treat an encoded name as a unique identifier in a multi-tenant or
  adversarial context — registry records, not names, are the identity
  authority.
* **Decode cannot reconstruct every identity** (same addendum): a sanitized
  base containing a literal `--` or starting with an illegal character is
  unrecoverable — a display-only corner; every record-driven path is
  unaffected. Implication: never parse meaning out of an msb sandbox dir
  name; always resolve through the registry record.

The [testing guide](testing.md) separates repository checks from VM acceptance.
Use the [runtime status](../README.agents.md#runtime-status) when assessing a
deployment's outstanding acceptance gates.

## Appendix: Mechanism index

Primary code paths for the mechanisms above
(`control/agentctl/src/` abbreviated as `src/`).

| Claim | Primary code path |
|---|---|
| Lock v2 shape `{rev, sha, fetched_at}` + per-ref map | `src/config/lockfile.rs` — `LockedFleet`, `LockedRef`, `LOCK_VERSION` |
| Newer-lock hard error | `src/config/lockfile.rs` — `load_config_lock_at` |
| v1 loads via defaults, never bumped on read | `src/config/lockfile.rs` — serde defaults + `default_lock_version` |
| Primary and per-ref pin writers | `src/config/lockfile.rs` — `upsert_locked_pin`, `upsert_locked_ref` |
| `--config-ref` global flag | `src/main.rs` — `Cli.config_ref` (global), `async_main` env stamping |
| Inline grammar `name[:ref][@id]`; bare `@` rejected | `src/config/inline_ref.rs` — `parse_workload_selector` |
| Instance-id precedence chain | `src/main.rs` — `rewrite_action_for_inline_selector`; `src/commands/lifecycle.rs` — `resolve_dependent_instance_id` |
| Ref-derived id sanitization | `src/microsandbox/slots.rs` — `sanitize_instance_id` |
| Archive store `<state>/cache/gitv3/<sha>/` | `src/config/archive.rs` — `archive_store_root`, `ensure_archive` |
| Resolution precedence: lock → registry rev → notice | `src/config/loading.rs` — `pinned_layer_content_root` |
| Plain-path exception, branch `"local"` | `src/config/loading.rs` — `layer_content_root`; `src/config/registry.rs` — `source_kind` |
| Profile binary pinning | `scripts/host-provision.sh` (`nix profile install .#workestrate`); `control/agentctl/build.rs` (rev-stamped version) |
| Slot / instance naming | `src/microsandbox/slots.rs` — `slot_for`, `instance_name` |
| Fleet derivation ladder | `src/config/registry.rs` — `resolve_active_fleet`, `config_ref_branch_candidate`, `checkout_branch_candidate` |
| Fleet-name rule + fail-closed load (G4) | `src/config/registry.rs` — `validate_fleet_prefix`, `load_registry` |
| `WORKESTRATE_FLEET` read sites | `src/config/registry.rs` (resolver step a); `src/main.rs` (flag → env) |
| Image tags `name:ctx.sha` + current-pointer | `src/images/state.rs` — `PointerRecord`, `pointer_key`; `src/images/gc.rs` — `split_computed_tag` |
| GC cascade capsule→repo→settings→default(5) | `src/images/gc.rs` — `resolve_keep_last`, `DEFAULT_IMAGE_KEEP_LAST` |
| Prune-on-load / manual sweep asymmetry | `src/images/build_cmd.rs` — `process_target`; `src/images/gc.rs` — `cmd_images_gc` |
| Running-sandbox protection via record `image_tag` | `src/images/gc.rs` (protection set); `src/microsandbox/port_registry/mod.rs` — `SandboxInstanceRecord.image_tag` |
| Provenance hash scope (ports guest+name only; seeds excluded) | `src/microsandbox/provenance.rs` — module doc, `canonical_plan_bytes` |
| Short-hash display (first 4 hex) | `src/microsandbox/provenance.rs` — `PROVENANCE_DISPLAY_LEN`, `short_hash` |
| Pre-stamp records never auto-stale | `src/microsandbox/provenance.rs` (module doc); `src/commands/diagnostics.rs` — `apply_config_staleness` |
| `on_skew` values warn/replace/reuse-silently | `src/config/types.rs` — `OnSkew`; wiring: `src/microsandbox/runtime/run.rs` — `skew_disposition` |
| Down ladder scopes + double gate | `src/microsandbox/runtime/down_scope.rs` — `DownScope`, `resolve_cli_scope`, `everything_gate` |
| Everything prompt wording / non-interactive refusal | `src/commands/lifecycle.rs` — `EVERYTHING_PROMPT`, `everything_interactive_confirm` |
| Managed-rung single yes-gate (piped y confirms) | `src/commands/lifecycle.rs` — `confirm_or_abort`, `managed_scope_prompt` |
| Classification evidence (record ∨ slot ∨ log) | `src/microsandbox/runtime/down_scope.rs` — `classify`, `Evidence` |
| Config-ref scope validation (sha refused) | `src/microsandbox/runtime/down_scope.rs` — `validate_config_ref`, `known_config_refs` |
| Hardened six-step teardown at every scope | `src/microsandbox/runtime/mod.rs` — `down_hardened`; `src/commands/lifecycle.rs` — `cmd_down_ladder` |
| JSON envelope `{scope, results}` | `src/json_out.rs` — `down_scope_results_json` |
| `clean` = hygiene only | `src/commands/lifecycle.rs` — `cmd_clean` |
| Per-dir requires `${CWD}` mount | `src/config/validation.rs` (per-dir validity gate) |
| Per-dir id `<slug>-<hash8>` | `src/microsandbox/slots.rs` — `per_dir_instance_id`, `dirname_slug`, `fnv1a64_hex8` |
| Per-instance state mounts | `src/microsandbox/workload/config.rs` (state-target keying); ADR 0030 §V2 |
| Source-gone records swept normally | `src/microsandbox/runtime/down_scope.rs` tests (source-gone sweep pin) |
| msb-name encoding (@→`--` + hash suffix, 128-byte clamp) | `src/microsandbox/slots.rs` — `msb_name_of_instance`, `MAX_MSB_NAME_BYTES` |
| Legacy raw-`@` fallback lookup | `src/microsandbox/slots.rs` — `lookup_msb_names` |
| Enumeration dual-spelling dedup | `src/microsandbox/runtime/down_scope.rs` — `enumerate_targets` |
