# ADR 0032: Deployment versioning, provenance, and cleanup (blue-green/promote deferred)

**Status:** Accepted (blue-green/promote deferred)
**Date:** 2026-08-23
**Addendum:** 2026-08-24 (config source model + selection ladder + down
scope ladder + image tags DECIDED + operating model; supersedes the
`clean --vms` verb shape in § Cleanup family — the ladder and the
`down`/`clean` verb split below govern)
**References:** ADR 0019 (contexts + `<context>-<workload>` namespacing),
ADR 0021 (instance lifecycle + AI-native surfaces; `down --all` back-compat
alias), ADR 0026 (per-instance addressing + dynamic ports), ADR 0030
(instance lifecycle + conflict management + namespacing), the stop-fix
microsandbox fork work (commits `56297b1` / `36a8be2` — the hardened
stop→wait-exit→remove teardown path).

## Context

Deployments today have **no version identity**: a rebuilt image, an edited
config, and a stale running instance are indistinguishable in `ps`. There is
no honest answer to "is what is running what I just built?", and cleanup of
accumulated sandboxes/images is ad-hoc (`down --all` and manual `msb`
surgery). The operator model needs three things: a **version axis** in the
identity model, **provenance** on registry records so staleness is visible,
and a **cleanup command family** that uses the hardened teardown semantics.
Full blue-green/promote deployment was analyzed and is **explicitly
deferred** (§ Deferred: blue-green / promote) — per the user: "I'd like the
idea noted down, but this is too much for now".

## Decision

### Identity model: three axes

Deployment identity has **three orthogonal axes**:

1. **Context (why/where)** — environment isolation (ADR 0019); slots are
   named `<context>-<workload>`.
2. **Version (what)** — a **runtime-relevant content hash** of the inputs
   that actually determine runtime behavior.
3. **Instance (how many)** — singleton vs parallel (ADR 0021/0030).

Full identity form: **`<workload>@<instance>` in context `<ctx>` built from
`<ver>`**.

### Image identity: content-hash tags + per-context alias

> **Superseded by the 2026-08-24 addendum** (§ Image tags — DECIDED): the
> tag format is `name:ctx:sha` immutable per build, and the per-context
> mutable alias TAG is replaced by the state-dir image record as the
> mutable current-pointer.

Images are tagged with **immutable content-hash tags** plus a **per-context
mutable alias**:

- **`workestrate-prime:sha-<hash>`** — always written; the content identity.
- **`:main` / `:feat-x` alias tags** — resolved by `up`/`exec` to the
  current hash tag for that context.

Rationale:

- **Branch isolation** — a dev build cannot overwrite prod's tag; each
  context's alias moves independently.
- **Content identity** — identical inputs reuse the identical image; changed
  inputs produce a new tag.
- **Blue-green storage** — old and new images coexist by construction.
- **GC note** — hash tags accumulate; policy is keep-last-N per context, or
  a future `workestrate images gc` (open question).

### Provenance stamps

Registry records gain **`{ image_out_hash, config_hash, created_at }`**. This
enables:

- **Staleness display in `ps`** — e.g. `stale (config a1b2 → current d4e5)`.
- **A future version-aware disposition** (e.g. `clean --vms --stale`).

**Hash scope is runtime-relevant fields only**: image out path, env
(including baked), mounts + policies, command, resources, network, secret
names. Explicitly **NOT** hashed: comments, docs, formatting, unrelated-
workload changes — edits that cannot change runtime behavior must not churn
the hash.

### Cleanup family

> **Superseded in verb shape by the 2026-08-24 addendum** (§ Down scope
> ladder): VM teardown moves to `down` with an explicit scope ladder, and
> `clean` keeps state/cache hygiene only. The classification model,
> hardened-teardown requirement, per-target outcomes, and nonzero-exit
> rules below STAND and are carried into the ladder unchanged.

**Accepted; build order next** (see Consequences):

```
workestrate clean --vms [--context <ctx>] [--all-contexts] [--everything]
                        [--dry-run] [--yes] [--json]
```

- **Classification model**: a target is **workestrate-managed** iff any of:
  registry record ∨ slot-name pattern (`<context>-<workload>`) ∨ artifact
  evidence (`workestrate.log` in the sandbox dir, `workestrate-*` image tag).
- **Context scope** = slot-prefix match on `<context>-`.
- **`--everything`** = every msb sandbox regardless of classification —
  **double-gated**.
- **Every target goes through the hardened path** — stop → wait-exit →
  remove → unregister → policy-dir → sandbox-dir (the stop-fix semantics from
  the recent microsandbox fork work, commits `56297b1` / `36a8be2`).
- **Per-target outcomes reported**; **nonzero exit on any failure**.
- **`down --all` (down-all) is aliased to `clean --vms --all-contexts`** for
  back-compat.
- **Optional `--stale`** (kill only records whose provenance hash != current)
  once stamps land.

### Deferred: blue-green / promote

**Explicitly deferred — future work.** Recorded so the analysis is not lost.
The user's framing constraint shaped both modalities: **"needs to stay up
till it's switched, test, THEN switch"**. Two honest modalities exist:

**(a) Parallel candidate + promote.** Bring the new instance up on a
different port via dynamic ports, verify it via its own address, re-point
dependents, retire the old. Requires a **promote primitive** and
**re-resolution semantics** for dependents — neither exists today.

**(b) Isolated context staging.** A full parallel environment via the context
model: `feat-x-litellm` + `feat-x-prime` on their own band, **zero port
conflicts**. Promote = switch the prod config content + normal replace.
**Modality (b) is nearly free with current machinery and is the RECOMMENDED
near-term pattern for vital components.**

**The verification problem.** Health is **workload-specific** — there is no
honest generic "healthy". The design space:

- **External probes** — port / HTTP checks from the host.
- **In-guest probes** — `msb exec` inside the sandbox.
- **Capsule-declared verify hooks** — possibly reusing the prime-smoke check
  vocabulary.

This is why auto-promote needs custom test logic per workload and is
deferred.

## Consequences

**Positive:**

- **Version axis makes staleness visible** — `ps` can say *what* drifted
  (config hash a1b2 → d4e5), not just *that* something is stale.
- **Dev/prod image isolation by construction** — per-context aliases mean a
  dev rebuild never disturbs the tag prod resolves.
- **Cleanup is one hardened path** — every removal reuses the
  stop→wait-exit→remove semantics already proven in the fork; no parallel
  teardown logic to drift.
- **Classification is conservative and explainable** — registry ∨ slot-name
  ∨ artifact evidence; `--everything` is double-gated, never accidental.
- **Back-compat preserved** — `down --all` keeps working as an alias.
- **Modality (b) gives vital components a blue-green-like pattern today** —
  context staging needs no new primitives.

**Negative / costs:**

- **Hash tags accumulate** — without `images gc` or a keep-last-N policy,
  registry/image storage grows monotonically.
- **Promote (modality a) is unavailable** — no in-place blue-green with
  dependent re-pointing until the promote primitive + re-resolution
  semantics land.
- **No honest generic health check** — verification must be declared per
  workload; auto-promote cannot be trusted without it.
- **Provenance stamps are a registry schema change** — old records lack
  hashes and must be treated as unknown-version (never auto-stale).

**Implementation order:**

1. Context-at-create verification.
2. Provenance stamps + `ps` staleness display.
3. Context-scoped + hash image tags with alias.
4. `clean --vms` family.
5. Devshell context hook + operating-model doc (prod = pinned profile binary
   on main; dev = branch worktrees + `nix develop` with context-from-branch).
6. Later: `--stale`, `images gc`, and the deferred promote work.

## Open questions

**RESOLVED (2026-08-24, design session):**

- **Tag format** — DECIDED: `name:ctx:sha` immutable per build; the
  per-context mutable alias is replaced by the state-dir image record as the
  mutable current-pointer (§ Image identity, 2026-08-24 decision).

**RESOLVED (2026-08-24, user decisions):**

- **Per-dir default-for-agents** — RESOLVED: **opt-in first**; the capsule
  declares `strategy = "per-dir"`; default-for-agents-with-cwd-mounts is
  reconsidered after the host batch proves it (ADR 0030 §V5).
- **Per-workload config-branch override UX** — AMENDED 2026-08-24
  (same-day): RESOLVED: **ONLY the inline colon syntax** `prime:feat-x`
  (`:` = config branch, consistent with the `name:ctx:sha` image tags;
  `@` = instance id; the combined form `prime:feat-x@canary` is legal);
  the `--from <ref>` flag form is DROPPED. Superseded original (retained
  as historical record): ~~RESOLVED: **BOTH forms** — the `--from <ref>`
  flag AND the inline colon syntax `prime:feat-x`~~. Rationale: (1)
  `--from` already carries two other meanings in the CLI's history —
  `migrate home --from <xdg|bundle>` (legacy) and the dropped
  `home init --from <src>` (ADR 0025 addendum, removed as "surface area
  without leverage", guard test at control/agentctl/src/main.rs:1121) —
  a third meaning on workload up/exec would collide; (2) one grammar
  (`name:ref[@instance]`) is leaner than two forms and consistent with
  the image-tag shape; (3) `config new --from-reference` is a different
  flag/subcommand and is UNAFFECTED. Default = the home's pinned ref
  (unchanged).
- **GC policy for hash tags** — RESOLVED: **schema-configurable cascade** —
  built-in default N=5 < home settings < config repo < workload capsule (a
  keep/retain-count setting; exact field name per repo conventions);
  automatic prune-on-load of older sha tags per `name:ctx` beyond N;
  `workestrate images gc` remains the manual sweep; running sandboxes are
  never affected (a pruned tag = rebuild-from-store on recreate).
- **Prod config strategy** — RESOLVED: **NO prod branch**; `main` is the
  stable line (tidy discipline). Homes pin `ref = "main"` + the lockfile
  rev; everything stays configurable per entry (`ref`/`rev`) and per
  invocation (`--config-ref`). Default-ref order reaffirmed: explicit >
  `origin/HEAD` (covers main/master/trunk) > checkout branch for local
  working repos > hard error.
- **Prod/dev secrets** — RESOLVED: **shared** `LITELLM_MASTER_KEY` across
  all litellm instances; the egress `allowed_hosts` scoping is the control.
  Reasoning: the blast radius is local/self-hosted — separate per-context
  keys would buy ceremony, not isolation.
- **auth.json hardening** — RESOLVED for now: **keep the passive mask**.
  The harder-seal TEST PLAN is recorded in
  `docs/mount-policy/06-testing.md` (§ Future hardening tests); the decision
  to seal harder (`write.deny` / operator-final `read.deny`) is DEFERRED
  until that test plan runs.
- **migration/tool-model → main** — RESOLVED: promote when the current
  line SETTLES (host batch green + pushes done), not mid-flight.
- **Personal-repo remote target** — RESOLVED: **local-only FOR NOW**; the
  user will push to origin eventually. NOTE: nothing in the personal config
  repo is backed up off-host until then.
- **Context-at-create verification (A1)** — RESOLVED (2026-08-24): registry
  records keep recording `context` = active context at register time
  (unchanged). NEW write-side refuse: the registry refuses a record whose
  `context` does not match the slot prefix of the instance name
  (`slot_of_instance(instance) == slot_for(workload, context)`); namespaced
  instance + `context: null` is refused; bare name + null stays legal
  (backward compat). Read-side: adopting an existing record
  (reuse/start-existing/dep-satisfied) warns (never fails) when record
  context ≠ invocation context; legacy `context: null` records are
  unknown-context forever — never migrated, never hard-failed (they are the
  A4 cleanup classification's "unknown" bucket). ALSO RESOLVED: context
  names are validated at registry load, fail-closed — slug shape (start
  `[a-z0-9]`, continue `[a-z0-9-]`, no trailing `-`) because names become
  instance-name prefixes; the lenient dir-resolution load path stays
  lenient. Context display folded into A1: `instances` text and `workloads`
  text+JSON gain the context column/field.

**REMAINING (parked):**

- **Promote command shape when un-deferred**: explicit `workload promote` vs
  chain element promote-if-healthy. Parked WITH the deferred blue-green /
  promote design (§ Deferred: blue-green / promote) — resolves when that
  work is un-deferred.
- **Whether models.json seed content belongs in the config hash** or stays a
  `--reseed` concern. Parked; tied to the provenance-stamp implementation
  round.

---

## Addendum (2026-08-24): config source model + selection ladder + down scope ladder + image tags + operating model

This addendum records the 2026-08-24 design session decisions. It extends
the identity/provenance decisions above with the config-consumption model
they depend on, supersedes the `clean --vms` verb shape (§ Cleanup family),
and resolves the tag-format open question.

### Config source model (the "repository thing")

Registry entries are `{ url, ref, rev }` with a SINGLE `url` field,
**scheme-discriminated**:

| url form | kind | semantics |
|---|---|---|
| `github:` / `https` / `ssh` / `git://` | remote | cloned into the managed store; network fetch. |
| `git+file://` | local git repo | full ref support (branches, tags, shas resolve), NO network. |
| plain path | local working repo (git) or plain dir | a git working repo resolves refs against its own object store; a plain dir is consumed content-as-is with branch = `"local"`. |

- **Generated lockfile `<home>/workestrate.lock` (v2)** — `{ rev, sha,
  fetched_at }` per entry+ref. Written ONLY by an explicit
  `workestrate config update` or by first-resolution-with-notice; **never
  silently** (no verb mutates pins as a side effect).
- **Content-addressed archive store** `<state>/cache/gitv3/<sha>/` —
  produced by `git archive <sha>` from the EXISTING single managed clone
  per repo. Recorded explicitly: **NO worktrees, NO checkouts** — one clone
  IS the object database; cache entries are plain immutable directories;
  simultaneous branches are free (two refs of one repo are two archive
  dirs, never two checkouts).
- **Default-ref resolution order:** explicit `ref` > `origin/HEAD`
  (clones/remotes) > checkout HEAD (local working repos) > HARD ERROR
  naming the repo.
- **Platform/deps:** the git binary is already required; `tar` on unix
  (git archive emits tar); a zip fallback is the noted Windows path
  (`git archive --format=zip`). **No new dependencies.**

### Selection ladder

> **UX decision (2026-08-24):** ~~the per-workload override has BOTH forms —
> the `--from <ref>` flag AND inline colon syntax `prime:feat-x`~~
> AMENDED 2026-08-24 (same-day): the per-workload override is ONLY the
> inline colon syntax `prime:feat-x` (`:` = config branch, consistent
> with image tags; `@` = instance id; combined `prime:feat-x@canary`
> legal); the `--from <ref>` flag form is DROPPED (rationale: `--from`
> collision history — `migrate home --from <xdg|bundle>` legacy + dropped
> `home init --from` per ADR 0025 addendum, guard test at
> control/agentctl/src/main.rs:1121 — plus one-grammar leanness
> (`name:ref[@instance]`) consistent with the image-tag shape;
> `config new --from-reference` is a different flag/subcommand,
> UNAFFECTED). Default = the home's pinned ref.

Precedence, highest first:

1. **`--home`** — the hard boundary: which home (registry + state) the
   invocation operates on at all.
2. **`--config-ref <branch|sha>`** — "the home on that branch": resolves
   every config entry at the given ref (via the archive cache) and IMPLIES
   the context (the ref's branch becomes the context).
3. **Per-workload inline override `prime:feat-x`** — capsule-only
   substitution: the named workload's capsule is read at `<ref>` from its
   declaring repo while everything else stays home-scoped.

Inline override rules:

- **Deps NEVER follow the override in v1** — `depends_on` resolves against
  the home-scoped config; a `--with-deps` closure flag is noted as possible
  later work (implementation-detail, parked).
- **Instance identity carries the override ref as a parallel instance** —
  `workload up prime:feat-x` plans `prime@feat-x`, which COEXISTS
  with `prime@main` (parallel-slot bind + dynamic ports per ADR 0030); no
  port or state collision by construction.
- **Validation:** the ref must EXIST in the declaring repo AND the workload
  must EXIST at that ref — both fail-closed at plan time.

**Context derivation order:** explicit `--context` > `--config-ref` branch
> checkout branch (local working repo) > `main`.

**Defaults:** pinned ref everywhere — an unadorned invocation consumes the
locked rev (the stable line); branch names are opt-in freshness.

### Down scope ladder (supersedes the `clean --vms` sketch)

`down` gains an explicit scope ladder, narrowest to widest:

```
instance  <  workload  <  context (= branch)  <  config-ref  <  home (--all)  <  everything (--everything, double-gated)
```

- **Classification engine** (carried over unchanged from § Cleanup family):
  a target is workestrate-managed iff registry record ∨ slot-name pattern
  (`<context>-<workload>`) ∨ artifact evidence (`workestrate.log` in the
  sandbox dir, `workestrate-*` image tags).
- **Hardened teardown path** (the stop-fix semantics, commits `56297b1` /
  `36a8be2`): every target at every scope goes stop → wait-exit → remove →
  unregister → policy-dir → sandbox-dir.
- **Per-target outcomes reported; nonzero exit on ANY failure.**
- **Verb split (recorded):** `down` owns ALL VM teardown across the ladder;
  `clean` keeps state/cache hygiene only (the existing state-dir content
  removal — workspaces/, var/run/ — plus, under this addendum, archive-cache
  GC bounds) and NEVER tears down VMs. The `clean --vms` sketch is
  superseded; `down --all` remains the home-wide teardown (no alias needed
  — it already IS a `down` scope).

### Image tags — DECIDED

- **Tag format: `name:ctx:sha`, immutable per build.** The earlier
  alias-pair sketch (`name:sha-<hash>` + `:ctx` mutable alias tag) is
  replaced: the **state-dir image record is the mutable current-pointer**
  for the context — no mutable registry tags at all, so a dev build cannot
  even transiently move what prod resolves.
- **GC (RESOLVED 2026-08-24):** keep-last-N per context as a
  schema-configurable cascade — built-in default N=5 < home settings <
  config repo < workload capsule — with automatic prune-on-load of older
  sha tags per `name:ctx` beyond N; `workestrate images gc` remains the
  manual sweep; running sandboxes are never affected (a pruned tag =
  rebuild-from-store on recreate).

### Operating model (A6 preview)

- **prod** = pinned rev (workestrate.lock v2) + pinned profile binary.
  **No `prod` branch (RESOLVED 2026-08-24):** `main` IS the stable line
  (tidy discipline); homes pin `ref = "main"` + the lockfile rev; per-entry
  `ref`/`rev` and per-invocation `--config-ref` remain the overrides.
- **dev** = any branch BY NAME via refs (`--config-ref` / inline
  `prime:feat-x`) —
  freshness without moving pins.
- **Nothing is checked out unless it is being edited** — consumption reads
  the archive cache, not a working copy.
- **Commit-before-consume** — only committed content is archivable; a dirty
  working tree is invisible to the pinned/remote paths (a local working-repo
  entry consumed content-as-is is the explicit exception).
- **Worktrees are for ONE thing only:** simultaneous editing of two
  branches. They are a human editing tool, never a consumption mechanism.

## Addendum (2026-08-24): config source model LANDED — implementation notes

- Landed as commits `24385e9` (foundations: source-kind classification,
  git archive plumbing, archive store, lock v2), `0ff3b30` (pinned archive
  consumption + lock-writer semantics), `db60a8e` (`--config-ref` + context
  derivation), `43deba1` (inline override), `2367acb` (review fixes) — all
  on `migration/tool-model`.
- **Lockfile collision RESOLVED**: the addendum's
  `<home>/config-repos.lock` does NOT exist as a file. The existing
  `<home>/workestrate.lock` (ADR 0025(e)) was EVOLVED to version 2 to
  carry the A5 semantics — its own header mandated one lock story ("do not
  build a second one"), and every signed-off semantic (per entry+ref pins
  `{rev, sha, fetched_at}`; written only by explicit `config update`/`add`
  or first-resolution-with-notice; never silently by runtime verbs) is
  preserved; only the filename differs from the addendum's letter. v1
  locks read via serde defaults; v2 written by current binaries; a v2 lock
  read by an old binary hard-errors ("home created by a newer workestrate")
  by design.
- **`sha` semantics**: `sha` = the commit sha the archive was produced
  from (== `rev` for git-backed entries today). The field separates the
  pin (`rev`) from the content key (`sha`) so a future tree-hash keying
  never migrates pins. Archive dirs are content-addressed by it:
  `<state>/cache/gitv3/<sha>/`.
- **Grammar** (one-grammar, inline-only; no `--from` flag):
  `name[:ref][@id]` — `:` = config branch (consistent with `name:ctx:sha`
  image tags), `@` = instance id, combined `prime:feat-x@canary` legal.
  Bare `name@id` (no colon) is rejected, pointing at `--instance`. Id
  precedence: `--instance` > `@id` > `:ref`-derived (sanitized) > `--new`
  > per-dir derivation > strategy default.
- **Two-phase arming** (deps never follow): the override is pending at
  parse and arms only after dep auto-start (up/exec) or immediately
  (plan); detached children re-arm from `WORKESTRATE_WORKLOAD_REF`,
  name-gated and verb-scoped (down/logs never arm).
- **Devshell context hook**: NOT shipped — subsumed by the CLI's
  checkout-branch context derivation (the hook's semantics). A6 documents
  this; an explicit per-shell pin stays optional.

### Open questions surfaced by the A5 implementation (2026-08-24)

- **Registry-rev fallback ignores ref changes**: a hand-edited `ref` with
  a stale registry `rev` consumes the old rev silently until
  `config update` (fail-safe direction). Track or reject?
- **Bare homes acquire branch-derived context names** (checkout-branch
  derivation step): existing no-[contexts] homes with a git-checkout first
  layer gain `context = Some(<branch>)` and slot renaming on next
  invocation. Pinned design; needs a release note / migration note in the
  operating-model doc (A6).
- **ensure-images pre-flight runs pre-arming** — **RESOLVED (2026-08-24,
  A2 stage 2)**: the override path now lands POST-ARMING and tags under the
  OVERRIDE's tag context (`ensure_after_arming` reorders the named ensure
  after dep auto-start + `arm_inline_override`, so it sees the substituted
  declaration and moves only the `(repo, attr, "feat-x")` pointer); the
  non-override path was VERIFIED NO-GAP (an ensure never runs while an
  override is merely pending on up/exec parents — it either runs
  pre-auto-start with no override in play or post-arming under the override
  ctx; detached children skip ensure entirely via the token). Flow tests
  pin both shapes. See the image-GC LANDED addendum below.
- **Leftover exported `WORKESTRATE_WORKLOAD_REF` + bare `workload up`**:
  a name-matching batch child arms the override (explicit export =
  intent; accepted, recorded).

## Addendum (2026-08-24): image GC LANDED — implementation notes

Landed as commit `16af715` (code + schemas + tests) and this docs commit,
both on `migration/tool-model`. Stage 1 (`001a2ca`) landed the immutable
`name:ctx:sha` store tags + the state-dir current-pointer; stage 2 closes
A2 with the keep-last-N GC cascade (RESOLVED user decision 3). The
mechanics below are PINNED (they are the recorded contract, verified by
the test suite):

- **Field-name pins** (all `Option<u32>`, schemars-derived, snake_case;
  both committed schemas + the template copies regenerated):
  capsule `ImageSpec.keep_last` → `[workloads.<name>.image] keep_last = N`;
  home settings `RegistrySettings.image_keep_last` → registry.toml
  `[settings]`; config-repo entry `ConfigRepoEntry.image_keep_last` →
  `[configs.<name>]`. Cascade resolver `resolve_keep_last(settings, repo,
  capsule)`: first Some wins scanning **capsule → repo → settings →
  default**; ANY explicitly-provided `0` is a hard error naming the field
  and locus (`keep_last >= 1`: the just-loaded/current tag always counts
  toward N and is always retained). `DEFAULT_IMAGE_KEEP_LAST = 5`.
- **Prune trigger point**: in `process_target`, immediately AFTER
  `run_build_pipeline` returns Ok, INSIDE the still-held per-tag lock,
  build mode only — never `--check`, never the D1-trust path, never Skip.
  BOTH `LoadAction` outcomes prune ("prune-on-load"). The capsule rung
  rides `BuildTarget.keep_last`; repo/settings rungs resolve from the live
  registry (unreadable → defaults, the advisory posture). Prune failures
  never fail the load — they aggregate into one stderr note.
- **Manual-gc cascade scope asymmetry (documented)**: `workestrate images
  gc` resolves keep_last WITHOUT the capsule rung (settings < repo entry <
  default 5), because the sweep operates on state-dir groups, not
  invocations; the capsule rung is enforced at load time by prune-on-load.
  Repo lookup per group uses the repo identity of the group's
  lexicographically-first record; canonical-path (unregistered) repo keys
  match no entry → fall through. Cross-repo same-name groups are
  pathological (D5-warning territory); this pin is deterministic.
- **Migration shape = TOLERATE**: no rewrite-on-read. Legacy records stay
  under legacy keys indefinitely; GC/prune candidates must parse as
  computed-shape `<name>:<sha>` / `<name>:<ctx>:<sha>` with a 12-char
  lowercase-alphanumeric sha segment (`split_computed_tag`). Legacy
  declared tags (e.g. `img-pi:latest`) NEVER parse → never candidates,
  never touched. Known edge (accepted): a user-DECLARED tag that happens
  to look computed-shape would parse as a candidate — shape is the only
  discriminator by design.
- **Running-sandbox protection mechanics**: `SandboxInstanceRecord` gains
  additive `#[serde(default)] image_tag: Option<String>`, populated ONLY
  where the image is known at create (the create-from-plan path records
  `plan.image`; `start_existing` re-starts and reconcile re-registrations
  pass None — documented). Protection set for any prune/gc = every
  `image_tag` across ALL `${state_dir}/var/run/*.json` records.
  Conservative: stale records over-protect until teardown unregisters
  them. Legacy records parse as None → unprotected, but their tags are
  legacy-shape and never candidates anyway. An UNVERIFIABLE protection set
  (unreadable port registry) makes gc REFUSE the sweep and prune-on-load
  skip pruning — fail-closed in the don't-prune direction.
- **Selection + ordering**: group candidate tags by `(name, ctx)` (tags
  carry no repo; records do); order by `ImageRecord.loaded_at` DESCENDING
  (missing/empty stamp → oldest; a tag in several records takes its newest
  stamp and appears once), tie-break tag ASC. Retain the newest N; NEVER
  pruned: any pointer-referenced tag (pointers never dangle), any
  running-protected tag (reported as skipped-running), the newest tag.
- **Removal seam**: `ImageRemover { fn remove_tag(&mut self, tag) -> impl
  Future<Output = Result<(), RemoveError>> + Send }` next to `ImageLoader`
  in pipeline.rs; `RemoveError::{Refused, Unreachable}` (Display names
  msb, LoadError style). Real backend `MsbCliRemover` calls
  `microsandbox::Image::remove(tag, false)`; `ImageNotFound` maps to Ok
  (idempotent). Execution: probe → Present → remove → drop the record
  under every `<repo>#<tag>` key + any pointer whose `.tag == tag`;
  Gone → same state cleanup counted "already gone"; Err → aggregate and
  continue. State saves: ONE fresh-reload save per batch under the held
  lock (prune-on-load) / one save per pruned tag under its own per-tag
  `ImageTagLock` (gc).
- **GC verb**: `workestrate images gc` (`ImagesAction::Gc`,
  `cmd_images_gc(json)`). Text: summary line `images gc: swept G group(s),
  pruned P tag(s), skipped S running, errors E` + one line per group
  `name[:ctx] kept=<k> pruned=[…] skipped-running=[…] gone=[…]`
  (deterministic order). `--json`: bare-array envelope of `{group, kept,
  pruned, skipped_running, already_gone, errors}`. Exit nonzero AFTER
  sweeping everything if any removal errored (cleanup-family aggregate
  rule).
- **Ensure-seam verification result**: non-override home-scoped resolution
  VERIFIED NO-GAP (see the resolved open question above); override-path
  flow test pins `<attr>:feat-x:<sha>` tagging + only-the-override-pointer
  movement under ARMED override; home-context flow test pins the
  three-segment tag + `(repo, attr, Some(ctx))` pointer when an active
  context is set. Skew invariant pinned from the GC side: a pruned tag
  reads `StoreTag::Gone` → §3.4 row 3 Rebuild, never an error
  ("pruned tag = rebuild-from-store on recreate"), with a
  recreate-after-prune flow test proving the current-pointer survives.

### Open questions surfaced by the A2 stage-2 implementation (2026-08-24)

- **Untracked store tags are invisible to the sweep**: `images gc` sweeps
  groups derived from `images.json` records only. A computed-shape tag
  present in the msb store but absent from the state file (loaded manually
  or by an older tool version) is never a candidate and accumulates until
  it is either recorded (D1 trust on next ensure) or removed manually.
  Extend the sweep to enumerate the msb store listing?
- **Cross-repo same-name groups have no warning surface yet**: the pin is
  deterministic (lexicographically-first record's repo identity decides
  the rung lookup), but a D5-style warning naming pathological
  cross-repo same-name groups at sweep time is unbuilt.
