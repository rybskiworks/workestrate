# Operating model — as-built operator guide (A6)

The operator-facing description of how workestrate homes run day to day:
what "prod" means, how dev work happens without moving pins, what gets
consumed from where, and how teardown, images, and provenance behave. It
is **as-built**: every mechanism below was verified against the code on
`migration/tool-model` at `bffa481`. Decisions are cited to their ADRs
under [migration/50-decisions/README.md](migration/50-decisions/README.md)
(0025 home/lockfile, 0026 per-instance addressing, 0030 instance
lifecycle, 0032 deployment versioning/cleanup). Companion docs:
[secrets.md](secrets.md), [testing.md](testing.md),
[nix-purity.md](nix-purity.md). A final appendix ("Mechanism index") maps
every major claim to its primary code path so each one traces.

## 1. Prod and dev: one stable line, pins everywhere

There is **no `prod` branch** (ADR 0032, RESOLVED user decision): `main`
IS the stable line. A prod deployment is two pins held together:

* **Content pin** — registry entries carry `ref` (typically `"main"`) and
  the generated lockfile `workestrate.lock` records the exact rev each
  entry was resolved to. Runtime verbs consume the locked content, not
  "whatever main is today".
* **Binary pin** — the installed binary comes from the nix profile
  (`scripts/host-provision.sh` runs `nix profile install .#workestrate`
  when the profile is stale) and reports its build rev via
  `workestrate --version` (`0.1.0-<rev>`).

The lockfile is v2 (ADR 0032 addendum §Config source model; ADR 0025(e)):
each `[repos.<name>]` entry carries `{url, ref, rev, sha, fetched_at}` —
`rev` is the pin, `sha` the commit the content archive was produced from
(equal to `rev` today), `fetched_at` the RFC3339 UTC resolution time.
Additional refs of the same repo are pinned per-ref under
`[repos.<name>.refs.<ref>]` with the same `{rev, sha, fetched_at}` shape.
The lock is generated — never hand-edited — and written only by explicit
verbs (`config add`, `config update`, `config remove`, `home init`,
`home clone`) or first-resolution-with-notice; no
verb moves a pin silently as a side effect.

Overrides exist at two levels: per-entry (`ref`/`rev` fields in the
registry) and per-invocation (`--config-ref <ref>`, a global flag valid on
every subcommand). Dev work (next section) uses these overrides and never
touches the pins.

## 2. Dev by branch name

Dev freshness is opt-in by ref, in two forms:

* **Whole-home override** — `workestrate --config-ref feat-x <verb>`
  resolves every git-backed config entry at that ref (via the archive
  cache) and *implies the context*: a branch-shaped ref becomes the active
  context name. A 40-hex sha is legal for consumption but implies no
  context.
* **Per-workload inline override** — `workload up prime:feat-x[@canary]`.
  Grammar `name[:ref][@id]`: `:` = config branch, `@` = instance id. The
  named workload's capsule is read at `<ref>` from its declaring repo's
  archive while everything else stays home-scoped. Dependencies never
  follow the override in v1.

A **bare `name@id` (no colon) is rejected** with guidance to use
`--instance`: instance ids ride the flag; the `@` form is only valid in
the combined `name:ref@id` override. `down`/`logs` also reject the `:ref`
form (teardown targets an instance id via `--instance`, not a ref).

Instance-id precedence, highest first (as implemented): explicit
`--instance <id>`; then inline `@id`; then `--new` allocation OR the
`:ref`-derived sanitized id (when both are given, `--new` wins the id
while the ref still drives the substitution); then per-dir derivation
(`strategy = "per-dir"` only); then the strategy default (`parallel`
auto-slug; otherwise the singleton).

The `:ref`-derived id is the ref run through `sanitize_instance_id`
(lowercased, non-`[a-z0-9]` runs collapsed to `-`, capped at 32 chars,
fail-closed on all-symbol or purely numeric results), so `prime:feat-x`
plans instance `prime@feat-x` coexisting with `prime@main`.

## 3. Commit-before-consume

Runtime verbs consume **locked archive content**, never a working copy:
git-backed entries resolve to `<state>/cache/gitv3/<sha>/`, a
content-addressed directory produced by `git archive <sha>` from the
single managed clone per repo. Consequences: only committed content is
archivable, so a dirty working tree is invisible to the pinned/remote
paths; nothing is checked out to consume content; and local plain-path
entries are the explicit exception — consumed content-as-is from the
filesystem (merge layer name `branch = "local"`), where
commit-before-consume does NOT apply.

**Upgrade note (verified behavior).** Resolution precedence for a git-backed
entry is: (1) the lock pin — silent; (2) the registry-recorded `rev` —
**also silent**; (3) first resolution with a stderr notice — the ONLY
notice path. A home upgraded from a pre-lock-v2 binary therefore has pins
already (a v1 lock's `rev`, or a registry-recorded `rev`) and switches to
pinned-archive consumption **without any notice firing**; the notice only
announces a first-ever resolution of an unpinned entry. Expected
post-upgrade behavior: run `workestrate config update` explicitly to
re-pin entries at their current refs. Related fail-safe quirk (recorded in
the ADR open questions): a hand-edited `ref` with a stale registry `rev`
consumes the old rev silently until `config update`.

## 4. Lock v2 vs old binaries: ordering matters

`LOCK_VERSION` is 2. A v1 lock (version absent or 1) loads via serde
defaults and is NOT version-bumped on read — readers never rewrite. A lock
with a version newer than the binary supports is a **hard error**:

```
home created by a newer workestrate (lock version N > M supported by this
binary); upgrade this workestrate before using the home at <path>
```

This posture is by design (fail-closed, like every other versioned file).
**Ordering consequence for operators:** sync the nix profile to the new
binary BEFORE pointing old binaries at an updated home. An old binary
meeting a v2 lock refuses the home rather than misreading it.

## 5. Worktrees are for editing, never for consumption

Git worktrees have exactly ONE sanctioned use: simultaneous human editing
of two branches. Consumption never creates checkouts or worktrees — two
refs of one repo are two archive directories under `cache/gitv3/`, not two
working copies. Nothing is checked out unless someone is actively editing
it (ADR 0032 addendum §Operating model; the archive module's own header:
"NO worktrees, NO checkouts").

## 6. Context hook status: not shipped

The devshell context hook from the original ADR 0032 implementation order
is **NOT shipped** (ADR 0032: "subsumed by the CLI's checkout-branch
context derivation"). What exists instead, exactly: `WORKESTRATE_CONTEXT`
is read by the context resolver (step (a) of the derivation ladder, next
section) and by `context current` to classify the resolution source; it
can be set by hand or exported in a shell. The global `--context <name>`
flag sets `WORKESTRATE_CONTEXT` at CLI entry, which also propagates it to
detached children via spawn env inheritance. Checkout-branch derivation
happens CLI-side inside the invocation — the resolver inspects the first
layer's checkout branch itself. There is no shell integration to install.
(The only hooks in the tree are unrelated: the home repo's pre-commit
tombi gate and workload pre-start seed hooks.)

## 7. Context model quick reference

Derivation order (pinned; first match wins):

1. explicit `--context` / `WORKESTRATE_CONTEXT` — strict semantics: when
   contexts are defined the name must be one of them (hard error
   otherwise); a contexts-less home ignores the env var;
2. `--config-ref <branch>` — a branch-shaped ref becomes the context-name
   candidate (a sha implies nothing);
3. checkout branch of the FIRST layer's checkout;
4. default resolution — `settings.default_context`, else bare layers when
   no contexts are defined, else the existing hard error. This final step
   IS the ADR's "> main": main is the stable line, and there is no literal
   `"main"` context name.

Context names must match `^[a-z0-9][a-z0-9-]*$` with no trailing hyphen
(names become `<context>-<workload>` slot prefixes). Enforcement is
fail-closed at registry load: a registry carrying an invalid context name
fails to load with an error naming the offending key (the `G4` gate in the
code; ADR 0032 A1 resolution). The lenient dir-resolution load path used
for early path setup deliberately stays lenient. There is **no automatic
migration** for homes with pre-existing odd names: the fix is hand-editing
the registry TOML (contexts are created by hand-editing; there is no
`context new`). Legacy records with `context: null` are unknown-context
forever — never migrated, never hard-failed.

## 8. Identity and image tags

Identity has three axes (ADR 0032 §Identity model): workload, context,
instance id. **slot** = `<workload>` (no context) or `<ctx>-<workload>`;
**instance** = the slot (singleton) or `slot@id` (parallel / per-dir /
scoped-dep shapes).

Image tags are immutable content addresses: `name:ctx.sha` per build
(ctx-less `name:sha` when no context is in play; the dot separator is
ADR 0032's 2026-08-28 amendment — the two-colon form is an invalid OCI
reference); the mutable current-pointer lives in the state-dir image
state (`pointers` map), not in any registry tag, so a dev rebuild cannot
move what prod resolves.

Retention is keep-last-N with a cascade (first configured value wins):

```
built-in default N=5  <  home settings image_keep_last
                      <  config-repo entry image_keep_last
                      <  workload capsule image.keep_last
```

Any explicitly provided `0` is a hard error (N >= 1: the just-loaded tag
always counts toward N). Enforcement points:

* **prune-on-load** — after a successful image build+load (build mode
  only), older tags in the same `(name, ctx)` group beyond N are removed;
* **`workestrate images gc`** — manual sweep of ALL groups. Documented
  asymmetry: the sweep resolves N WITHOUT the capsule rung (it operates on
  state-dir groups, not invocations).

Running sandboxes are never affected: the protection set is every
`image_tag` recorded across all port-registry records
(`${state_dir}/var/run/*.json`). Stale records over-protect until teardown
unregisters them; a pruned tag simply rebuilds-from-store on recreate. If
the protection set is unreadable, gc refuses and prune-on-load skips
(fail-closed in the don't-prune direction).

## 9. Provenance stamps

Registry records carry `{image_out_hash, config_hash}` stamps (ADR 0032
§Provenance stamps). The config hash is FNV-1a 64-bit over a canonical
labeled serialization of the plan's **runtime-relevant fields only**:
image, workdir, command, resources, env (sorted, effective post-merge
view), secret NAMES, ports, mounts (+ policy fragments), network rules.
Pinned exclusions operators should know: **ports contribute guest port +
optional name ONLY** (host ports and bind IPs are allocation-dependent
and never churn the hash); **seed declarations and seed content are
excluded**, so reseeding never makes an instance stale — seeds refresh
via the explicit `--reseed` flow (the mount a seed lands in remains
hashed); comments, formatting, identity metadata, and orchestration
policy (`on_conflict`, port strategy, `on_skew`) are not build inputs.

Display: hashes are stored full-length (16 lowercase hex); human surfaces
truncate to the FIRST 4 hex chars. A stale `ps` row reads
`stale (config xxxx → current yyyy)`; JSON gains an additive `staleness`
object. Both are omitted where staleness is not computable — including
**pre-stamp records, which are unknown-version and NEVER auto-stale**
under any policy.

Disposition is the workload knob `instance.on_skew` with exactly three
accepted values: `warn` (default — proceed and print the divergence),
`replace` (tear down and start fresh), `reuse-silently` (adopt without
comment). Since the provenance landing this wiring is REAL: both reuse
paths (the child Reuse arm and the detached-up parent short-circuit)
compare stamps and honor the disposition.

## 10. The down ladder

Teardown scopes, narrowest to widest:

```
instance < workload < context < config-ref < home (--all) < everything
```

The instance/workload rungs stay on
`workload <name> down [--instance|--all-instances]`. The four sweep rungs
live on `workestrate down`: exactly ONE selector per invocation —
`--all`, `--context <ctx>`, `--config-ref <ref>`, or `--everything`.
Bare selector-less `down` is a usage error naming the ladder; it never
guesses a scope. Scripted automation migrates as
`down-all --yes` → `down --all --yes` (`down-all` survives as a hidden
alias but now requires a selector too — the old bare `down-all --yes` is a
usage error).

Exact gates, as implemented:

* **Managed rungs** (context/config-ref/home) take the standard single
  yes-gate: interactive prompt unless `--yes`; a piped `y`/`yes`
  confirms; a declined prompt aborts with exit 1.
* **`--everything` is DOUBLE-gated.** (a) The flag must appear TWICE —
  `--everything --everything`; a single occurrence is a usage error raised
  BEFORE any prompt. (b) The yes-gate with a distinct widened-blast-radius
  prompt on a tty: `This will stop EVERY msb sandbox INCLUDING ones
  workestrate does not manage. Continue? [y/N]`. `--yes` skips it;
  non-interactive stdin WITHOUT `--yes` HARD-REFUSES — there is no
  piped-y escape for this rung.
* `down --config-ref` validates fail-closed BEFORE any teardown: a 40-hex
  sha is refused ("a sha does not imply a context"); an unknown ref errors
  listing the known refs (registry entry refs + lockfile entry refs and
  ref keys).
* Context scope keys on RECORD context (primary) or the `<ctx>-` slot
  prefix (corroborating only) — never bare-name equality.

Classification evidence per target: registry record ∨ dashed slot pattern
(`<ctx>-<workload>`) ∨ the `workestrate.log` artifact in the sandbox dir.
The image-tag evidence form from the original cleanup sketch is
deliberately NOT implemented (ADR 0032 narrowing pin: tags are immutable
`name:ctx.sha` now, and the log alone carries full recall).

Outcomes: every target goes through the hardened six-step teardown (stop →
wait-exit → remove → unregister → policy-dir → sandbox-dir); per-target
results print under one scope header (`down --all (home): N target(s)`),
JSON wraps them as `{scope, results}`; ANY failure exits nonzero; an empty
selection exits 0 reporting `0 target(s)`. Unmanaged candidates exist only
under `--everything` and are reported with empty evidence honestly.

`workestrate clean` is state/cache hygiene only (contents of
`workspaces/`, `var/`, `run/`) and NEVER tears down VMs.

## 11. Per-directory instances (`strategy = "per-dir"`)

Opt-in fifth strategy value (ADR 0030 V-addendum §V1; USER DECISION:
opt-in, not default-for-agents). Mechanics: it requires a cwd-templated
mount (`${CWD}` or `${CWD}/...`) on the workload — declaring `per-dir`
without one is a validation error, fail-closed at config validation. The
instance id is `<dirname-slug>-<hash8>` of the CANONICALIZED invocation
cwd: last path component slugged (lowercased, separator runs collapsed,
≤23 chars) plus the first 8 hex chars of an FNV-1a 64 hash of the full
canonical path — same directory → same id; different directories never
share one (even with equal basenames, since the full path is hashed).
State mounts are instance-scoped: the declared state root gains the
instance-key segment, so two per-dir instances never share state.
Source-gone semantics: the record persists (with `source_dir` recorded at
create) when the directory disappears; such records are ordinary members
of every down sweep (pinned by test). `on_skew` is REAL for per-dir
instances post-A3: the parent-side staleness comparison covers the
detached-reuse path per-dir instances actually take.

## 12. Parallel instances and msb sandbox-name encoding

Landed at `bffa481` (ADR 0030 addendum 2026-08-26). Workestrate identities
remain `slot@id` EVERYWHERE — registries, slots, CLI, down scopes. Only
the name handed to the microsandbox SDK is encoded, because the SDK
validates sandbox names itself and `@` is illegal in them. An
already-SDK-legal name passes through UNCHANGED (plain slots like
`personal-litellm` keep their exact observable names — zero churn for
existing singleton homes). Otherwise every illegal char becomes `--` (so
`@` → `--`) and the result gains a `-<fnv1a64 hex8 of the original
identity>` suffix, keeping encoded names collision-free against legal
names that literally contain `--` (`a--b` stays `a--b`; `a@b` becomes
`a--b-<hash>`). Encoded names are clamped to the SDK's 128-byte cap with
the suffix intact.

**Observable change:** `~/.microsandbox/sandboxes/*` directory names for
parallel/per-dir/scoped-dep instances are now the ENCODED spelling (e.g.
`personal-litellm--canary-<hash8>`); scripts reading that directory must
expect it. Sandboxes created by older forks under raw-`@` names remain
manageable: never-refuse teardown paths try the legacy raw spelling as a
fallback lookup (encoded first). Decode from an encoded name is
best-effort display convenience only — registry records stay the source
of truth.

## 13. Known limitations

Stated plainly, each verified against the current tree:

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
  name; always resolve through the registry record. Open question (dated
  2026-08-26, nothing scheduled): candidate upgrades if either ever bites —
  a wider (e.g. 16-hex) or keyed suffix hash, fully reversible
  percent-style escape encoding, or an upstream widening of the SDK's
  `validate_sandbox_name` charset.
* **README verb-shape drift (RESOLVED 2026-08-26).** The README CLI rows
  described the old `down-all` verb shape; the README sweep updated them to
  the as-built ladder. No longer applicable.
* **Ignored-test inventory (exact, counted at `bffa481` — five total).**
  Two DB-pool lib tests in `src/microsandbox/runtime/mod.rs` run only with
  `--ignored` (the SDK pins a process-global database pool):
  `down_all_instances_returns_notfound_when_msb_db_empty_but_openable` and
  `down_hardened_notfound_walks_the_full_six_step_sequence`. Three
  KVM-host integration tests need real virtualization and a loaded image:
  `detached_up_new_registers_slot_at_slug_and_down_stops_it`
  (tests/lifecycle_detached.rs),
  `detached_example_litellm_up_from_flake_less_cwd_passes_project_root_gate`
  (tests/flake_root_gate.rs), and
  `kvm_up_after_image_content_edit_rebuilds_before_spawn`
  (tests/ensure_images_e2e.rs). Everything else runs in the normal
  `cargo test` pass.

## Appendix: Mechanism index

Every major claim above, mapped to its primary code path
(`control/agentctl/src/` abbreviated as `src/`).

| Claim | Primary code path |
|---|---|
| Lock v2 shape `{rev, sha, fetched_at}` + per-ref map | `src/config/lockfile.rs` — `LockedRepo`, `LockedRef`, `LOCK_VERSION` |
| Newer-lock hard error, exact phrase | `src/config/lockfile.rs` — `load_home_lock_at` |
| v1 loads via defaults, never bumped on read | `src/config/lockfile.rs` — serde defaults + `default_lock_version` |
| Pin writers (explicit only) | `src/config/lockfile.rs` — `upsert_locked_pin`, `upsert_locked_ref` |
| `--config-ref` global flag | `src/main.rs` — `Cli.config_ref` (global), `async_main` env stamping |
| Inline grammar `name[:ref][@id]`; bare `@` rejected | `src/config/inline_ref.rs` — `parse_workload_selector` |
| Instance-id precedence chain | `src/main.rs` — `rewrite_action_for_inline_selector`; `src/commands/lifecycle.rs` — `resolve_dependent_instance_id` |
| Ref-derived id sanitization | `src/microsandbox/slots.rs` — `sanitize_instance_id` |
| Archive store `<state>/cache/gitv3/<sha>/` | `src/config/archive.rs` — `archive_store_root`, `ensure_archive` |
| Resolution precedence: lock → registry rev → notice | `src/config/loading.rs` — `pinned_layer_content_root` |
| Plain-path exception, branch `"local"` | `src/config/loading.rs` — `layer_content_root`; `src/config/registry.rs` — `source_kind` |
| Profile binary pinning | `scripts/host-provision.sh` (`nix profile install .#workestrate`); `control/agentctl/build.rs` (rev-stamped version) |
| Slot / instance naming | `src/microsandbox/slots.rs` — `slot_for`, `instance_name` |
| Context derivation ladder | `src/config/registry.rs` — `resolve_active_context`, `config_ref_branch_candidate`, `checkout_branch_candidate` |
| Context-name rule + fail-closed load (G4) | `src/config/registry.rs` — `validate_context_name`, `load_registry` |
| `WORKESTRATE_CONTEXT` read sites | `src/config/registry.rs` (resolver step a); `src/commands/config_cmd.rs` (`context current` source); `src/main.rs` (flag → env) |
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
