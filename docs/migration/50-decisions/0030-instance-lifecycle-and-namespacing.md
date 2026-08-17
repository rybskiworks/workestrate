# ADR 0030 (DRAFT): Workload instance lifecycle + conflict management + namespacing

**Status:** DRAFT — design accepted; **Phase 0 IMPLEMENTED** (conflict
chains + shared reconcile + status/dir-aware occupancy; 2026-08-16 addendum
below). Phases 1–4 remain proposal.
**Date:** 2026-08-16
**Addendum:** 2026-08-16 (user design threads — depends_on scoping, parallel deps,
dynamic ports; refined phased plan; supersedes §6); 2026-08-16b (strategy
chains + port `on_occupied` options; supersedes the single-disposition model
and the `on_occupied = "auto"|"fail"` surface; resolves Q2)
**References:** ADR 0021 (instance lifecycle model), ADR 0026 (per-instance
addressing + discovery-lite, incl. the 2026-08-16 on_conflict addendum),
ADR 0019 (contexts + `<context>-<workload>` namespacing), ADR 0027 (verb-first
dispatch), spec 12 (`docs/validation-and-improvements/06-improvements/12-per-instance-addressing.md`),
`docs/migration/20-target-system-spec.md` §13 (instance model) + §3 (schema).

> **Why an ADR, not a spec:** the ADR series is the decision record for the
> lifecycle model (0021, 0026 and their addenda live there); the
> `06-improvements` spec series is for execution specs of already-decided
> features. This document is a decision proposal (assessment + design + plan)
> that needs user adjudication before implementation — ADR DRAFT status is the
> correct home. Once the design is accepted, a follow-up execution spec (or
> direct implementation phases with the gates below) can carry the work.

---

## 1. Context — triggering evidence (2026-08-16, host)

1. **`workload up litellm` (no `--replace`) fails while nothing is running:**
   `error: sandbox already exists: sandbox 'litellm' already exists; remove it,
   start the stopped sandbox, or recreate with .replace()` — while `msb ps`
   shows NO running sandboxes and the port-registry record is GONE. A STOPPED
   sandbox lingers in msb; the CLI neither starts it nor replaces it.
2. **`exec prime` dep auto-start hits the same wall** ("detached service
   'litellm' exited immediately... already running / already exists" variants).
   Commit `d452575` (on_conflict for DEP auto-start, landed today) covers the
   dep path; the NAMED `up`/`exec` path has no equivalent.
3. **Recurring theme:** litellm "mysteriously dies" between sessions; zombies
   (msb keep-alive Running + dead service process); restart-and-hope.

## 2. Current-state assessment

### 2.1 Instance model (ADR 0021, implemented)

- **Slots:** singleton slot `<workload>` (no context) or `<context>-<workload>`
  (context active, ADR 0019); parallel instance slot `<slot>@<id>` with slug
  rules enforced by `validate_instance_id` (`src/microsandbox/slots.rs`).
- **Refuse-on-occupied default:** `up`/`exec` on an occupied slot refuses with
  remediation; escapes are `--replace` (tear down + fresh), `--instance <id>`
  (target a parallel slot), `--new` (synthesize a random slug).
- **`down` family:** singleton down, `down --instance <id>`,
  `down --all-instances`, `down --all` (confirm-gated).
- **`ps`:** registry records + best-effort liveness probe (`Sandbox::get` per
  entry; NotFound → `stale: true`; unreachable/unknown → honest stderr notes).
- **`workloads`:** discovery view (name, kind, image, running instances) —
  registry-based, no msb liveness probe.

### 2.2 The occupancy gate (`check_occupied_or_replace`, runtime/mod.rs:137)

The gate is the single choke point for `up`/`exec`:

- `--replace` → `Sandbox::get` → `stop_and_remove` (or unregister if NotFound).
- no replace → `Sandbox::get` **Ok** → REFUSE (any msb existence counts as
  occupied); NotFound + state record → REFUSE (stale); NotFound + no record →
  OK; msb unavailable + record → REFUSE (fail-closed).

**Gaps that produce the 2026-08-16 failures:**

1. **Status-blind occupancy.** `Sandbox::get` returns a handle for STOPPED and
   CRASHED sandboxes too (msb DB rows persist). The gate treats any msb
   existence as "occupied" and refuses — it never distinguishes
   Running/Stopped/Crashed, and never uses msb's own
   `handle.start()` / `resolve_and_start` (start-stopped-sandbox) capability.
2. **Dir-blind occupancy.** The msb SDK's create gate
   (`prepare_create_target`, fork `sdk/rust/lib/backend/local/sandbox/create.rs`)
   refuses when `existing.is_some() || dir_exists` — the sandbox DIRECTORY
   (`~/.microsandbox/sandboxes/<name>/`) can linger after the DB row is gone.
   The workestrate gate never checks the directory, so it can pass while the
   create fails with the opaque `SandboxAlreadyExists` error observed.
3. **No liveness check on the named path.** The keep-alive entrypoint
   (`tail -f /dev/null`, run.rs:463) means msb "Running" does NOT imply the
   exec'd service is alive. The dep path (d452575) probes host ports; the
   named `up`/`exec` path does not — a zombie blocks `up` forever.

### 2.3 Port-registry lifecycle (`port_registry/store.rs`)

- Records at `${state_dir}/var/run/<instance>.json` with lifecycle metadata
  (`port_pairs`, `created_at`, `bind_ip`); legacy records parse via serde
  defaults.
- Atomic check+register (FN-6) closes the post-create TOCTOU; loopback
  allocator (`127.0.0.N`, N≥2) for parallel slots; `probe_free_ports` for
  `--port-auto` / `host = 0`; `unregister_sandbox` on `down`.
- **The registry is workestrate's "what is running" view, but it is a
  write-only-on-lifecycle-verb store** — nothing reconciles it against msb
  except the dep executor (d452575) and the `ps` liveness probe (read-only).

### 2.4 Detached supervision (run.rs + spawn.rs)

- `up` (detached) re-execs the binary as a child (`spawn_detached_service`);
  FS-8 gives a 500ms grace and surfaces immediate child exit with a log
  pointer; the child runs `build_sandbox` → the occupancy gate → create.
- The sandbox entrypoint is `tail -f /dev/null`; the real service runs via
  `exec_stream`. If the service dies, the sandbox stays Running (keep-alive
  zombie) and the registry record stays — nothing detects or cleans it.

### 2.5 deps on_conflict (d452575) — the seed machinery

`commands/deps.rs` now reconciles the planner's record view with msb before
starting a dep:

- `decide_dep_disposition(action, msb_running, healthy, recently_started)` →
  `Reuse | Replace | Start | Fail`, driven by `depends_on.<dep>.on_conflict =
  "reuse" | "replace" | "fail"` (default "reuse").
- Facts: msb occupancy (`Sandbox::get`), host-port health (500ms TCP probe),
  boot grace (record < 30s old → never kill mid-boot), stale record
  (record present, msb gone → replace).
- **Scope:** wired ONLY into `auto_start_dependencies` (the dep path). The
  named `up`/`exec` path and the bare `workload up` batch still use the
  status-blind gate; bare-up keeps record-as-authoritative skipping with the
  same msb reconciliation tracked as a follow-up (ADR 0026 addendum).

### 2.6 What msb itself supports (fork source)

- `Sandbox::get(name)` → handle; `handle.status_snapshot()` →
  Running/Stopped/Crashed/Created/Starting/Paused/Draining.
- `handle.start()` / `handle.start_detached()` — start a stopped/crashed
  sandbox (the CLI's `resolve_and_start` does exactly this: connect if
  Running, start if Stopped/Crashed).
- `handle.stop()`, `handle.kill()`, `handle.remove()`, `Sandbox::remove(name)`.
- `Sandbox::builder().replace()` — stop+remove prior, then create.
- **No rename** in the SDK surface we use; "rename" is not needed for the
  design below (instances are created under their final names).

### 2.7 Dynamic ports — verdict

- **EXISTS and WORKS.** Per-port `host = 0` auto-allocation (P1, ADR 0026
  addendum 2026-08-10) probes a free port on the slot's bind at boot
  (`apply_auto_ports`, run.rs:322) and records the EFFECTIVE port in the
  instance record. `--port-auto` (ADR 0026(c)) replaces ALL declared hosts.
  `probe_free_ports` (store.rs) is lock-serialized, skips registry-recorded
  ports on the same bind, and is NOT a reservation (post-create atomic
  check+register closes the window).
- **Why litellm pins host 4000:** the personal config explicitly declares
  `host = 4000` (named port `api`), and the ecosystem is built around the
  well-known address — `agent_base` egress recipe (tcp:4000 → host),
  `depends_on.litellm.exports = { api = "LITELLM_ADDR" }`, the ingress rule
  (tcp:4000 local), and static configs. Dynamic ports are opt-in per port;
  litellm chose the well-known address for static-config compatibility.
- **Named-port exports + dynamic ports:** `exports` resolves the NAMED port
  from the RUNNING record's `port_pairs` (`record_port_by_name`,
  discovery.rs) — so a dynamic port DOES flow through exports correctly once
  the dep is running (the record carries the effective port). The refusal
  cases are: a not-running dep with an auto port ("no address until it runs")
  and a named port missing on a running record ("restart it"). Both are
  correct fail-closed behavior, not dynamic-port bugs.

### 2.8 Namespacing现状

- Context namespacing (`<context>-<workload>`) and parallel instances
  (`<slot>@<id>` on per-instance loopback IPs) ARE the existing
  "different versions/configs coexisting" mechanism — e.g. a dev litellm
  alongside the main one is expressible as `litellm@dev` with its own
  `127.0.0.N` bind and (bind, port)-keyed collision model.
- **What is missing:** instance ids are opaque slugs with no declared
  version/config identity; there is no way to say "this instance is config
  variant X / version Y"; `--new` allocates random slugs; `workloads`/`ps`
  show no policy or version metadata; the CLI has no "instances" surface that
  groups by workload and shows per-instance status.

## 3. Root-cause class

The failures are NOT three separate bugs. They are one root-cause class:

> **State divergence between three stores — the port-registry record, the msb
> sandbox state (DB row + sandbox directory), and actual service liveness —
> combined with lifecycle events that are not transactional and an occupancy
> gate that is blind to two of the three stores.**

Concretely:

1. **Three-way mismatch anatomy (litellm case):**
   - registry record: GONE (never written in this state dir, or unregistered);
   - msb sandbox state: STOPPED row and/or lingering sandbox directory;
   - actual service: dead.
   The gate checks only (msb row existence) + (registry record) and refuses on
   the first, or passes into the msb create which refuses on the directory.
   No verb reconciles the three.
2. **Non-transactional lifecycle:** `up` = create → register (two steps, the
   create outside the registry lock); `down` = stop → unregister; a crash
   between steps leaves a record without a sandbox or a sandbox without a
   record. Nothing converges the stores on the next verb.
3. **Liveness is not a first-class fact:** the keep-alive entrypoint makes
   msb status meaningless for service health; only the dep path probes ports.

**Design principle:** one authoritative reconcile step that computes the
disposition from all three stores, used by every lifecycle verb — never
trust any single store, never let a stale store block a verb.

## 4. Design

### 4.1 Per-workload `instance` policy (schema, type-checked, validated)

New optional table `[workloads.<name>.instance]` (additive; absent = current
behavior where safe):

```toml
[workloads.litellm.instance]
strategy = "singleton"   # singleton | parallel | replace | reuse  (default "singleton")
on_conflict = "reuse"    # reuse | replace | fail                  (default "reuse")
port = "fixed"           # fixed | dynamic                         (default "fixed")
```

**`strategy`** — the instance model the workload defaults to:

| value | meaning |
|---|---|
| `singleton` (default) | `up`/`exec` target the singleton slot; `on_conflict` governs occupancy. |
| `parallel` | `up`/`exec` default to a FRESH parallel instance (auto slug, `--new`-style); `--instance <id>` targets a specific one. For workloads that make sense with many concurrent instances (prime/pi agents "working on different places"). |
| `replace` | singleton + always replace on start (the pre-ADR-0021 idempotent-restart behavior, now explicit). |
| `reuse` | singleton + reuse-if-healthy on start (the d452575 semantics generalized to the named path). |

`replace` and `reuse` are sugar for `singleton` + the matching `on_conflict`
default; they exist because the user asked for a four-value strategy enum and
because they read naturally in config.

**`on_conflict`** — what `up`/`exec` does when the target slot is occupied
(record, msb row, or dir):

| value | meaning |
|---|---|
| `reuse` (default) | reuse the running instance when healthy (host-port probe); auto-replace a keep-alive zombie (msb Running, dead port, old record) and a stale record (record present, msb gone); START a stopped/crashed sandbox (msb `handle.start()`) or replace it per the reconcile rules below. |
| `replace` | always down + start fresh. |
| `fail` | refuse with the standard occupied-instance message (the ADR 0021 default, now opt-in). |

Per-strategy `on_conflict` defaults: `singleton` → `reuse`; `parallel` →
`fail` (a targeted parallel instance that is occupied refuses — you cannot
"reuse" a different instance); `replace` → `replace`; `reuse` → `reuse`.

**`port`** — the port strategy:

| value | meaning |
|---|---|
| `fixed` (default) | declared host ports as-is (current behavior). |
| `dynamic` | every declared port behaves as `host = 0` (auto-allocate at boot on the slot's bind; effective ports recorded and shown by `ps`). Per-port `host = 0` still works and wins for that port. |

CLI flags remain explicit overrides: `--replace`, `--instance <id>`, `--new`,
`--port-auto`, `--no-deps` override the policy for that invocation (policy is
the default, flags are the escape — the same polarity as ADR 0021).

### 4.2 The reconciliation algorithm — one authoritative step

Generalize `decide_dep_disposition` (d452575) into a single
`reconcile_instance(state_dir, instance, policy)` used by `up`, `exec`,
`down`, `ps`, and the dep executor. Facts gathered per instance:

```
record            = port_registry.find_record(state_dir, instance)   # store 1
msb               = Sandbox::get(instance)                            # store 2a (DB row)
msb_status        = msb.status_snapshot() if msb else None            # Running/Stopped/Crashed/...
dir_exists        = sandbox_dir(instance).exists()                    # store 2b (dir; the msb create gate)
healthy           = probe_host_ports(record or declared)              # store 3 (service liveness)
recently_started  = record.created_at within BOOT_GRACE (30s)         # boot grace
```

Disposition table (per `on_conflict`):

| facts | reuse | replace | fail |
|---|---|---|---|
| no row, no dir, no record | **Start** | Replace | Start |
| msb Running + healthy (or no ports, or booting) | **Reuse** | Replace | Fail |
| msb Running + dead port + old record (zombie) | **Replace** | Replace | Fail |
| msb Stopped/Crashed | **Start** (msb `handle.start()`) or Replace (policy: start preserves state; replace is cleaner) | Replace | Fail |
| record present + msb gone (stale record) | **Replace** (down clears, start fresh) | Replace | Fail |
| msb gone + dir exists (lingering dir) | **Replace** (down/remove clears dir, start fresh) | Replace | Fail |
| msb unavailable | fail-closed: record present → Fail; else Start | Replace | Fail |

Notes:

- **Status-aware occupancy** replaces the status-blind `Sandbox::get` Ok →
  refuse: Stopped/Crashed is a STARTABLE state, not an obstacle.
- **Dir-aware occupancy** closes the `SandboxAlreadyExists` gap: the reconcile
  step checks the sandbox directory (the msb create gate) so a lingering dir
  is cleaned by the disposition, never surfaced as an opaque SDK error.
- **One step, all verbs:** `up`/`exec` reconcile before create; `down`
  reconciles (idempotent — a stale record is cleared, a stopped sandbox is
  removed, a zombie is stopped); `ps` reports the reconciled status
  (running / stopped / zombie / stale / unknown) instead of a binary
  stale flag; the dep executor calls the same step (d452575's
  `decide_dep_disposition` becomes a thin wrapper or is replaced).
- **Transactional discipline:** the reconcile step runs under the registry
  lock for the record-mutation parts (same FN-6 pattern); the msb mutations
  (start/stop/remove) stay outside the lock (async), and the post-mutation
  registration is the atomic check+register already in place.

### 4.3 Unify per-dep on_conflict

`depends_on.<dep>.on_conflict` (d452575) stays as the per-DEPENDENCY override
for auto-start; the per-workload `instance.on_conflict` becomes the default
for the dep's OWN named `up`/`exec` and for the dependent's auto-start when
the dep does not declare its own. Precedence: `depends_on.<dep>.on_conflict`
> `workloads.<dep>.instance.on_conflict` > built-in default (`reuse`). The
reconcile step is shared, so the dep path and the named path can never
disagree about a slot again.

### 4.4 CLI / observability surface

- **`workestrate instances [<workload>]`** (new verb, ADR 0027 style): list
  instances grouped by workload with slot, kind (singleton/parallel), status
  (running / stopped / zombie / stale / unknown), effective ports, policy
  (strategy/on_conflict/port), and version/config label (see 4.5). `--json`
  emits the extended record array.
- **`workestrate workloads`** gains policy + version columns (registry-based,
  unchanged liveness posture).
- **`workestrate ps`** gains the reconciled status classification (replacing
  the binary `stale` flag with a 5-state status; `stale` remains for
  back-compat in JSON).
- **`workestrate plan`** renders the policy-driven disposition (what `up`
  WOULD do: start/reuse/replace/fail) — plan stays pure (no msb mutations).

### 4.5 Versions / config variants (namespacing)

- Instance ids become SEMANTIC where the operator wants them: `litellm@dev`,
  `litellm@v2`, `prime@task-42` — already supported by `--instance <id>`.
- Add an optional `instance.label` (or `version`) field to the policy for
  display/grouping (`workloads.<name>.instance.label = "dev"`), and record it
  in the instance record so `instances`/`ps` can group by version.
- Ports/state never collide: parallel slots already get per-instance
  `127.0.0.N` binds and (bind, port)-keyed collisions; `port = "dynamic"`
  extends this to the port dimension for same-bind coexistence.
- `--use <dep>@<instance>` (ADR 0026(d)) already selects a specific instance
  of a dep; with semantic ids this becomes "use the dev litellm".

### 4.6 Migration / compat

- **Defaults = current behavior where safe.** `strategy = "singleton"` and
  `port = "fixed"` are the current behavior. The ONE deliberate default
  change: `on_conflict` defaults to `reuse` (was: implicit refuse). This is
  the fix — it makes `up`/`exec` on a stopped/zombie/stale slot converge
  instead of erroring. `fail` is available for workloads that want strict
  refusal (the ADR 0021 behavior, now explicit).
- Schema additive: new optional `[workloads.<name>.instance]` table; old
  configs parse with defaults; `generate-schema` + committed schema + drift
  guard (ADR 0021 §8) regenerate.
- CLI flags unchanged and still override the policy.
- `depends_on.<dep>.on_conflict` (d452575) unchanged; the per-workload policy
  only fills the default.

## 5. Acceptance criteria

1. `workload up litellm` with a STOPPED msb sandbox + no registry record →
   starts (or replaces per policy), never errors with "already exists".
2. `workload up litellm` with a zombie (msb Running + dead port + old record)
   → auto-replaces (down + fresh) under `on_conflict = "reuse"`.
3. `workload up litellm` with a healthy running instance → reuses (no
   restart, no port churn).
4. `exec prime` dep auto-start and named `up`/`exec` use the SAME reconcile
   step; the d452575 failure mode stays fixed.
5. `down` is idempotent across all three stores (stale record cleared,
   stopped sandbox removed, zombie stopped).
6. `ps`/`instances` show the reconciled 5-state status; `--json` carries it.
7. `port = "dynamic"` (and per-port `host = 0`) flow through
   `depends_on.<dep>.exports` to the effective port once running.
8. `strategy = "parallel"` makes `up`/`exec` default to a fresh instance;
   `--instance <id>` targets a specific one; `down --all-instances` cleans up.
9. Schema drift guard passes after the config-type change.
10. Full `just verify` (fmt, clippy, tests, spec-examples, schema, lint-nix)
    green on the implementation commits.
11. Conflict chains are attempted IN ORDER; a chain-exhausted failure reports
    the attempt sequence (e.g. `reuse (probe failed), start (not
    applicable)`). Port `on_occupied` chains likewise (preferred → increment
    → auto → error). (Added by the 2026-08-16 strategy-chains addendum, U8.)

## 6. Phased plan with gates

### Phase 0 — IMMEDIATE small fixes (no full design needed)

| # | fix | surface |
|---|---|---|
| 0.1 | `up`/`exec` on a stopped-existing sandbox starts or replaces per policy instead of erroring: status-aware occupancy (Stopped/Crashed → `handle.start()` or replace) + dir-aware occupancy (lingering sandbox dir cleaned, not surfaced as `SandboxAlreadyExists`). | `runtime/mod.rs` (`check_occupied_or_replace`), `runtime/run.rs` (create path), new small helper for sandbox-dir existence. |
| 0.2 | Reconcile registry-vs-msb divergence on EVERY lifecycle verb (generalize d452575's `decide_dep_disposition` into a shared step; wire into named `up`/`exec`, `down`, and bare `workload up`). | `commands/deps.rs` (extract), `runtime/mod.rs`, `runtime/run.rs`, `commands/deps.rs` bare-up executor. |
| 0.3 | Default `on_conflict` for named verbs: `reuse` default (reuse-if-healthy, replace zombie/stale, start stopped) with `fail` opt-in. | `runtime/mod.rs` + `InstanceSpec` (carry policy), `cli_actions.rs`/`main.rs` flag plumbing. |

**Gate 0:** acceptance criteria 1–5 pass on the host (KVM); `just verify`
green; no schema change yet (policy defaults are code-level).

### Phase 1 — Per-workload `instance` policy schema

- Add `[workloads.<name>.instance]` (`strategy`, `on_conflict`, `port`,
  optional `label`) to `config/types.rs`; validation (closed vocabularies,
  per-strategy on_conflict defaults); merge semantics (whole-table replace,
  last layer wins — consistent with `depends_on`); `generate-schema` +
  committed schema + drift guard.
- Wire policy into `InstanceSpec` (policy becomes the default; CLI flags
  override).

**Gate 1:** schema parse/deny/round-trip tests; merge tests; schema drift
guard green; `plan` renders the policy-driven disposition.

### Phase 2 — Unified reconciliation module

- Extract `reconcile_instance` (facts + disposition table, §4.2) as the one
  authoritative step; `decide_dep_disposition` becomes a thin wrapper.
- Wire into `up`/`exec`/`down`/`ps`/dep executor; transactional discipline
  (registry lock for record mutations, FN-6 pattern).

**Gate 2:** acceptance criteria 1–6; unit tests for the full disposition
table (mirroring the d452575 decision-table tests); KVM e2e for
stopped/zombie/stale/reuse.

### Phase 3 — CLI / observability surface

- `workestrate instances [<workload>]` verb; `workloads` policy/version
  columns; `ps` 5-state status; `--json` shapes extended (back-compat
  `stale` retained).

**Gate 3:** acceptance criteria 6; golden/JSON tests; docs (spec 12 / target
spec §13) updated.

### Phase 4 — Versions / config variants

- `instance.label`/version grouping in `instances`/`ps`; semantic-id
  conventions documented; `port = "dynamic"` + parallel-strategy e2e for a
  dev litellm alongside the main one.

**Gate 4:** acceptance criteria 7–8; host e2e with two litellm instances
(main + dev) coexisting; `--use litellm@dev` resolution.

## 7. What stays out of scope

- **Multi-context batch** (ADR 0019 deferral) — unchanged.
- **Name-based router / DNS registry** (ADR 0026 deferred trigger) — unchanged.
- **Guest-reachability of per-IP loopbacks** (ADR 0026(f) E1 deferral) —
  unchanged; `port = "dynamic"` on parallel slots inherits the E1 warning.
- **msb fork changes** — the fork is read-only; everything above uses the
  existing SDK surface (`handle.start()`, `status_snapshot()`, dir checks).
- **A real service supervisor / restart daemon** — the keep-alive entrypoint
  is a workaround; a supervisor that restarts dead services is a separate
  design (the reconcile step makes `up` converge, but does not auto-restart
  in the background).
- **Rename of sandboxes** — not needed; instances are created under their
  final names.
- **`--port-offset`** — remains removed (ADR 0021 addendum / ADR 0026).

## 8. Open questions for the user

1. **Default `on_conflict` for named verbs:** `reuse` (fixes the reported
   failures; changes ADR 0021's refuse default) vs `fail` (keeps strict
   refusal; requires config change per workload)? This proposal recommends
   `reuse`.
2. **Stopped-sandbox disposition under `reuse`:** START the stopped sandbox
   (preserves its state) vs REPLACE it (cleaner, but destructive)? This
   proposal defaults to START for `reuse` and REPLACE for `replace`.
3. **`strategy = "parallel"` default id:** auto-slug (`--new`-style) vs a
   declared default id vs require `--instance`? This proposal recommends
   auto-slug.
4. **Version/config identity:** is `instance.label` (display/grouping) enough,
   or do you want a stronger `version` field with validation?
5. **`instances` verb:** new verb vs extending `workloads`/`ps`? This
   proposal recommends a new verb (ADR 0027 style) with `ps` gaining status.
6. **Bare `workload up` batch:** should it adopt the same reconcile step
   (currently record-as-authoritative skip) in Phase 0.2, or stay
   conservative until Phase 2?

## 9. Options considered / rejected why

1. **Keep refuse-on-occupied as the only default; fix only the stopped case.**
   REJECTED: fixes the symptom (stopped sandbox) but not the root-cause class
   (three-store divergence, zombies, stale records) — the recurring
   "litellm mysteriously dies" theme would persist.
2. **Always-replace default (pre-ADR-0021 behavior).** REJECTED: destroys
   canaries and blue-green workflows; the destructive action must stay
   explicit (ADR 0021 reasoning).
3. **A separate `reconcile` command the user runs manually.** REJECTED:
   divergence must be converged by the verbs that touch the stores, not by a
   manual ritual users will forget.
4. **Per-workload policy as CLI-only flags.** REJECTED: the user requirement
   is schema-declared, type-checked defaults; flags remain overrides.
5. **A supervisor daemon as the primary fix.** REJECTED for this round: it is
   a larger, separate design; the reconcile step makes lifecycle verbs
   converge without background machinery (see §7).

---

## Addendum (2026-08-16): relationship to d452575

This ADR is the generalization of d452575's dep-path reconciliation. The
per-dep `on_conflict` knob, the disposition decision table, the boot-grace
window, and the host-port health probe all carry over unchanged; what this
ADR adds is (a) the per-workload policy surface, (b) status-aware and
dir-aware occupancy for the named path, (c) one shared reconcile step across
all lifecycle verbs, and (d) the observability surface. d452575 remains the
record of the dep-path fix; this ADR supersedes its "bare-up reconciliation
tracked as a follow-up" note.

---

## Addendum (2026-08-16): user design threads — depends_on scoping, parallel deps, dynamic ports

This addendum resolves the user's three design threads and refines the phased
plan. It SUPERSEDES §6 (phased plan) and answers open questions Q1, Q3
(partially), and Q4 (partially) from §8; Q2, Q5, Q6 remain open (marked
below).

### T1. depends_on scoping — per-config namespace

**User thread:** "scoping that to be per config, so you can have it depend on
that config's workloads and namespacing would go there."

#### T1.1 Today (verified from code)

- `resolve_depends_on` (discovery.rs:246) resolves each declared dep by
  filtering ALL registry records in the ACTIVE state dir by the bare
  `workload` field (`list_records_for_workload`), then selecting the
  singleton record (no `@`) or a `--use <dep>@<instance>`-selected parallel
  record. The slot is NOT computed at resolution time — the record's
  `instance` field already carries `<context>-<workload>` or `<workload>`.
- `plan_dep_starts` (deps.rs:251) computes the dep's slot via
  `slot_for(&dep, context)` where `context = active_context_name()` —
  process-level state set once per invocation inside `load_config` from
  `--context` / env / `settings.default_context` (ADR 0019).
- The config is MERGED across layers (config repos are layers;
  `merge_layers`), so `depends_on.<dep>` can name ANY workload in the merged
  config — including one declared by a DIFFERENT config repo. There is NO
  repo scoping today; the registry record has `workload` + `context` fields
  but no namespace field.
- **The declaring-repo seam already exists:** `merge_layers` returns
  `(ConfigFile, Provenance)` where `Provenance: HashMap<String, String>`
  maps `workloads.<name>.<field>` → layer name (merge.rs:433+), and
  `images/build_cmd.rs` already resolves each workload's repo identity from
  declaring-layer provenance (spec 17). `new_with_use_overrides`
  (workload/config.rs:151) already takes `take_provenance()`.

#### T1.2 Proposed: namespace-scoped resolution

- **Dep identity becomes `(namespace, workload, instance?)`** where
  `namespace` = the declaring config repo/layer of the DEPENDENT workload
  (from provenance). A workload depends on ITS OWN config's workloads.
- **Registry record gains a `namespace` field** (serde-default `"default"`
  for legacy records). Resolution filters records by
  `(namespace, workload)` instead of `workload` alone; `--use <dep>@<id>`
  selects within that namespace.
- **Slot scheme unchanged:** the singleton slot stays
  `<context>-<workload>`; the namespace is a resolution FILTER, not a slot
  prefix — existing records and the `ps`/`down` surfaces keep working.
- **Named-port exports unchanged in mechanics:** `exports` still reads the
  selected record's `port_pairs`; only SELECTION changes (scoped to the
  namespace).
- **Merge collision note (documented limitation):** the merged config's
  `workloads` map is keyed by bare name — two repos declaring the SAME
  workload name still collide (last layer wins). Namespacing does NOT
  reopen that merge decision; coexistence of same-name variants uses the
  parallel-instance mechanism (`litellm@dev`) or distinct names. The
  namespace field makes the collision VISIBLE (resolution refuses when the
  dependent's namespace has no record but another namespace does, with a
  remediation naming the namespace).

### T2. Parallel dependents + shared/owned deps

**User thread:** "check whether we need custom behavior for parallel
depends_on — if it'll just work properly instantiating a new one or reuse
existing, or if we need some policy on depends_on to specify whether it's
depends_on existing, new, etc — but better naming — or if it'd just follow
the workload's default strategy."

#### T2.1 The question

If prime runs parallel instances (`prime@1`, `prime@2`), what should litellm
mean to each? Today: dep auto-start targets the dep's SINGLETON slot
(`slot_for(&dep, context)`), so both share one litellm (the "shared" model).

#### T2.2 Options evaluated

1. **Follow the DEP's own instance strategy.** Ambiguous about WHO creates
   the instance and how it is named; conflates the dep's own lifecycle with
   the dependent's needs. REJECTED as the primary mechanism.
2. **Per-depends_on-entry policy** (`instance = "shared" | "scoped" |
   "fresh"`). Explicit at the point of use; the dependent declares what it
   needs from the dep. SELECTED.
3. **Just follow the workload's default strategy.** Adopted as the DEFAULT
   for the per-entry policy (see below), not as the whole answer.

#### T2.3 The chosen model

New field `depends_on.<dep>.instance` (closed vocabulary):

| value | meaning |
|---|---|
| `shared` (default) | target the dep's SINGLETON slot (today's model; all dependents share). |
| `scoped` | target a dep instance scoped to the DEPENDENT's instance: `litellm@prime-1` for dependent `prime@1`. Auto-start creates it if absent (using the dep's own strategy/on_conflict); each parallel dependent gets its own litellm. |
| `fresh` | target a fresh auto-slug dep instance per start (like `--new` for the dep). Each dependent start creates a new one. |

**Default = the DEP's own `instance.strategy`:** dep strategy
`singleton`/`replace`/`reuse` → default `shared`; dep strategy `parallel` →
default `fresh` (matches the "many concurrent instances" intent). This is
the "follow the workload's default strategy" answer as the default, with the
per-entry override for the cases that need custom behavior.

**Orthogonal to `on_conflict`:** `instance` SELECTS the target slot;
`on_conflict` (reuse/replace/fail, d452575) DISPOSES on the selected slot.
They compose cleanly: `scoped` + `reuse` = reuse my scoped litellm if
healthy, replace if zombie; `fresh` + `fail` = never reuse, refuse if the
fresh slug collides (effectively unreachable).

**Naming rationale:** `shared | scoped | fresh` is better than the user's
"existing/new" because it names the RELATIONSHIP (shared = one for all,
scoped = one per dependent, fresh = new each time), not the action.

**Answers Q3 (partially):** for the workload's OWN `strategy = "parallel"`,
the default id is auto-slug (`fresh`-style); `scoped` derives the dep
instance id from the dependent's instance id. The remaining sub-question
(declared default id vs auto-slug for the workload's own parallel starts)
stays open — auto-slug is the recommended default.

### T3. Dynamic ports for litellm

**User thread:** "start using the auto port for litellm (test that first) —
potentially have a default port it tries and option to allocate auto in case
of failure — evaluate how to handle/implement this and if it makes sense —
e.g. a policy in the configs."

#### T3.1 Current machinery verdict (verified from code)

- **Per-port `host = 0` auto-allocation EXISTS and WORKS:** `apply_auto_ports`
  (run.rs:322) probes a free port on the slot's bind at boot, rejects
  candidates colliding with declared hosts, bounded retries, fail-closed.
- **`--port-auto` (ADR 0026(c))** replaces ALL declared hosts with probed
  free ports.
- **`probe_free_ports` (store.rs:431)** is lock-serialized, skips
  registry-recorded ports on the same bind, and is NOT a reservation (the
  post-create atomic check+register closes the TOCTOU).
- **Effective-port recording:** `check_and_register_sandbox_lifecycle`
  records the (mutated) plan's host ports + port_pairs; `ps`/`down`/
  discovery read the effective ports.
- **Named-port exports flow effective ports:** `record_port_by_name`
  (discovery.rs:182) reads the record's `port_pairs`, so `LITELLM_ADDR` =
  `host.microsandbox.internal:<effective>` once running. VERIFIED.
- **Occupied FIXED port → hard error:** `check_port_collisions_locked`
  (store.rs:69) errors on `(bind_ip, port)` collision; the sandbox create
  fails. There is NO "skip to a free one" for a fixed port — only
  `host = 0` / `--port-auto` probe free ports. This is the current litellm
  failure mode (4000 occupied → `up` fails).

#### T3.2 The chosen capsule surface

`instance.port` becomes a union:

```toml
[workloads.litellm.instance]
port = 4000                                  # strict (current behavior): occupied → fail
# port = "auto"                             # always auto-allocate (host=0 on all ports)
# port = { preferred = 4000, on_occupied = "auto" }   # RECOMMENDED
# port = { preferred = 4000, on_occupied = "fail" }   # strict-with-preferred
```

- `port = 4000` (integer): strict, current behavior.
- `port = "auto"`: always auto-allocate (equivalent to `host = 0` on every
  declared port / `--port-auto`).
- `port = { preferred = N, on_occupied = "auto" | "fail" }` (RECOMMENDED):
  try the preferred port; if occupied, auto-allocate (or fail).
  **SUPERSEDED by the 2026-08-16 addendum (U5–U6):** `on_occupied` becomes a
  chain over `{auto, increment, fail}` with `increment` (stepwise-adjacent)
  added; the recommended default is `["increment", "auto"]`.
- Schema/merge/validation: `instance.port` is a serde untagged union
  (integer | "auto" | table); merge = whole-field replace (consistent with
  the instance table); validation = `preferred` in 1..=65535, `on_occupied`
  closed vocabulary. `generate-schema` + drift guard regenerate.

**Why recommended:** litellm's ecosystem is built around the well-known
4000 (agent_base egress, static configs, the smoke); a pure `"auto"` port
breaks every hardcoded consumer. `preferred + on_occupied = "auto"` keeps
4000 when free (stable, zero churn) and degrades gracefully to a free port
only when occupied — the exact "default port it tries and option to allocate
auto in case of failure" the user asked for.

#### T3.3 Downstream implications (verified)

- **LITELLM_ADDR export renders the EFFECTIVE port** (T3.1) — the export is
  correct on any port once litellm is running.
- **prime models.json renders at SEED TIME** from the env view
  (`build_seed_env_view` → `render_seed_text`, env.rs; `prepare` called in
  `build_sandbox` BEFORE create, run.rs:393). The env view includes the
  injected `LITELLM_ADDR`, so models.json renders the effective port at
  prime's BUILD time. **Staleness window:** if litellm's port changes after
  prime is built (litellm replaced on a different port), prime's baked
  `LITELLM_ADDR` + models.json are stale until prime is rebuilt. Mitigations:
  (a) the reconcile "reuse" default keeps the same instance (same port), so
  replacements are minimized; (b) `preferred = 4000` keeps the port stable
  across restarts when 4000 is free; (c) a future runtime-discovery
  improvement (re-resolve exports at exec time) is OUT OF SCOPE here but
  noted.
- **The smoke has SIX hardcoded `:4000` references** (flake.nix):
  the `models-json` baseUrl assert, the `litellm-guest` probe, the
  `litellm-subst` probe, the `litellm-host` probe, the optional
  `litellm-host-auth` probe, and the `models-json` PASS message. All must
  move to reading the effective port (from `workestrate ps --json` / the
  registry record / the export).
- **agent_base egress — CORRECTION to the premise:** `agent_base`
  (plan.rs:397) embeds `litellm_proxy` = **tcp:4000 → host** — it is
  PORT-based, not host-based. HOWEVER, the depends_on-derived egress rule
  (`ResolvedDependency::derived_egress_rule`, discovery.rs:136) emits
  tcp:<effective-port> → host, so prime's plan covers the effective port on
  ANY port. The agent_base tcp:4000 rule becomes redundant-but-harmless when
  litellm moves. No egress change needed.

### T4. Litellm dynamic-port TEST-FIRST sequence (user: "test that first")

> **SUPERSEDED by the 2026-08-16 addendum (U7):** the capsule uses
> `on_occupied = ["increment", "auto"]` and the sequence gains Test C
> (increment skip) and Test D (bound exhaustion → auto fallback).

Bounded validation sequence, run BEFORE the general dynamic-port policy
lands (Phase 3 gate):

1. **Capsule change:** litellm workload.toml gains
   `[workloads.litellm.instance] port = { preferred = 4000, on_occupied = "auto" }`
   (requires Phase 1 schema; the sequence runs at Phase 3).
2. **Test A — preferred path (free host):** `workload up litellm` → binds
   4000; assert `workestrate ps --json` port_pairs host == 4000; assert
   prime's `LITELLM_ADDR` == `host.microsandbox.internal:4000`; assert
   models.json baseUrl renders `:4000`; prime chat works through it.
3. **Test B — auto-fallback path (4000 occupied):** occupy 4000 (start a
   dev litellm or a dummy listener on 127.0.0.1:4000) → `workload up
   litellm` → auto-allocates a free port; assert the record carries the
   EFFECTIVE port; assert prime's `LITELLM_ADDR` ==
   `host.microsandbox.internal:<effective>`; assert models.json renders
   `<effective>`; prime chat works through it.
4. **Smoke update:** replace the six hardcoded `:4000` references with the
   effective port read from `workestrate ps --json` (or the registry
   record); the `models-json` assert becomes value-driven on the effective
   port.
5. **Regression:** re-run the full prime-smoke on BOTH paths; `just verify`
   green.

### T5. Resolved / remaining open questions

**Resolved by this addendum:**

- **Q1 (default `on_conflict` = reuse):** HOLDS under the parallel model.
  `on_conflict` is the disposition on the SELECTED slot; the parallel model
  changes SELECTION (`depends_on.<dep>.instance`), not disposition. Reuse
  remains the recommended default for named verbs and for `shared`/`scoped`
  dep targets.
- **Q3 (parallel default id, partial):** auto-slug for `fresh` and for the
  workload's own `strategy = "parallel"`; `scoped` derives the dep instance
  id from the dependent's instance id. Remaining sub-question (declared
  default id vs auto-slug) stays open.
- **Q4 (version/config identity, partial):** the NAMESPACE (T1) is the
  config identity for resolution; `instance.label`/version remains for
  display/grouping. Whether a stronger validated `version` field is wanted
  stays open.

**Still open for the user:**

- **Q2 (stopped-sandbox disposition under `reuse`):** **RESOLVED by the
  2026-08-16 strategy-chains addendum (U3)** — the default chain
  `["reuse", "start", "replace"]` starts a stopped sandbox and only replaces
  if the start fails.
- **Q5 (`instances` verb):** new verb vs extending `workloads`/`ps`.
  Recommendation unchanged (new verb, ADR 0027 style).
- **Q6 (bare `workload up` batch):** adopt the reconcile step in Phase 0.2
  vs stay conservative until Phase 2. Recommendation unchanged (adopt in
  Phase 0.2).
- **NEW — namespace merge collision:** two repos declaring the same
  workload name still collide in the merged config (last layer wins). Is
  that acceptable (coexist via parallel instances / distinct names), or do
  you want namespace-aware merge (bigger change, out of scope here)?

### T6. Refined phased plan (SUPERSEDES §6)

> **Partially SUPERSEDED by the 2026-08-16 addendum (U10):** the P0.3, P1,
> and P3 rows change (chain default, chain schema + validation + extended
> decision fn + U4 tests, port `on_occupied` chain + increment + U7 tests).

| Phase | Scope | Gates | Sizing |
|---|---|---|---|
| **0 — Immediate small fixes** | 0.1 status-aware + dir-aware occupancy (Stopped/Crashed → `handle.start()` or replace; lingering sandbox dir cleaned); 0.2 extract the shared reconcile step from d452575 + wire into named `up`/`exec`, `down`, bare `workload up`; 0.3 default `on_conflict = "reuse"` for named verbs (`fail` opt-in). | AC 1–5 host (KVM); `just verify`; no schema change. | ~3 files (runtime/mod.rs, runtime/run.rs, commands/deps.rs) + flag plumbing; small. |
| **1 — Instance policy schema** | `[workloads.<name>.instance]` (`strategy`, `on_conflict`, `port` union, optional `label`); validation; merge (whole-table replace); `generate-schema` + drift guard; wire policy into `InstanceSpec` (flags override). | schema parse/deny/round-trip; merge tests; drift guard; `plan` renders disposition. | config/types.rs + validation.rs + merge.rs + schemas; medium. |
| **2 — depends_on scoping + parallel semantics** | T1: registry record `namespace` field + namespace-scoped resolution (provenance-driven); T2: `depends_on.<dep>.instance = shared\|scoped\|fresh` with dep-strategy default; extend the reconcile step for scoped/fresh targets. | AC 4; unit tests for namespace filtering + scoped/fresh disposition; KVM e2e: two parallel primes each with scoped litellm. | discovery.rs + deps.rs + port_registry/store.rs + config/types.rs; medium-large. |
| **3 — Dynamic port policy + litellm migration** | `instance.port` union (`preferred`/`on_occupied`); the T4 test-first sequence (capsule change → Test A preferred → Test B auto-fallback → smoke update → regression). | T4 steps 2–5 all pass; AC 7; `just verify`. | run.rs (apply_auto_ports extension for preferred+fallback) + config + smoke (flake.nix, personal repo); medium. |
| **4 — Observability surface** | `workestrate instances [<workload>]` verb; `workloads` policy/namespace/version columns; `ps` 5-state status; `--json` shapes (back-compat `stale`). | AC 6; golden/JSON tests; docs (spec 12 / target spec §13). | diagnostics.rs + json_out.rs + main.rs; medium. |

**Dependencies / seams:**

- Phase 0–1 depend on NOTHING external (current msb SDK surface:
  `handle.start()`, `status_snapshot()`, dir checks — all present in 0.6.8).
- Phase 2 depends on Phase 1 (the `instance` policy provides the dep-strategy
  default for `depends_on.<dep>.instance`) and on the Phase 0.2 reconcile
  step (scoped/fresh targets need the disposition machinery).
- Phase 3 depends on Phase 1 (the `port` union lives in the instance policy).
- **Pending msb v2/develop work (mount-masking, spec 22):** NOTHING here
  hard-blocks on it. The fork pin is at the unsigned v2 rev `3bd051bf`
  (content-identical to the future signed rev); the SDK surface used by
  Phases 0–3 exists in 0.6.8. Seam to re-verify after the v2/develop move:
  the reconcile step's msb mutations (start/stop/remove) and the create
  surface — if v2 changes them, re-run the Phase 0/2 KVM gates.
- Phase 4 is independent and can run in parallel with Phase 3.

### T7. What this addendum does NOT change

- §4.1–4.6 (policy design, reconcile algorithm, CLI surface, migration)
  stand; T1–T3 refine the SELECTION and PORT dimensions, not the reconcile
  core.
- §7 (out of scope) stands; the runtime-discovery improvement (re-resolve
  exports at exec time) is added to the noted future work, not to scope.
- The d452575 dep-path fix and its `on_conflict` knob are unchanged; the
  per-entry `instance` field is additive.

---

## Addendum (2026-08-16): strategy chains + port `on_occupied` options (user refinement)

This addendum locks two design extensions from the user's 2026-08-16
refinement: (A) conflict disposition becomes an ORDERED CHAIN of strategies
attempted until one succeeds, and (B) the port `on_occupied` surface gains
stepwise-adjacent allocation (`increment`) and chain symmetry. It SUPERSEDES
the single-disposition model in §4.2/§4.3 and the `on_occupied = "auto"|"fail"`
surface in the previous addendum's T3.2; it RESOLVES open question Q2.

### U1. Strategy chains — surface

`on_conflict` accepts a scalar (back-compat: d452575's `"reuse"` etc. becomes
a one-element chain) OR an ordered list:

```toml
# scalar (back-compat, d452575): one-element chain
on_conflict = "reuse"
# ordered list: attempted in order until one succeeds
on_conflict = ["reuse", "start", "replace"]
```

Element vocabulary (closed, schema-validated):

| element | precondition | action |
|---|---|---|
| `reuse` | msb Running AND (healthy OR booting) | adopt the running instance (host-port probe; <30s record = booting, never killed) |
| `start` | msb Stopped/Crashed (row exists, not Running) | start it (`handle.start()`); on failure, continue the chain |
| `replace` | always applicable | down/remove + fresh create |
| `fail` | always applicable (terminal) | error with the standard occupied-instance message |

**Default chain: `["reuse", "start", "replace"]`.**

### U2. Strategy chains — behavior matrix

Facts: `msb_status ∈ {None, Running, Stopped, Crashed}`, `healthy ∈
{Some(true), Some(false), None}`, `recently_started`, registry record,
sandbox dir. The chain is iterated in order; the FIRST element whose
precondition holds wins. When the slot is FREE (no row, no dir, no record)
the plain Start path applies and the chain is irrelevant.

| state | reuse | start | replace | default-chain result |
|---|---|---|---|---|
| running + healthy | ✓ | n/a (running) | — | **reuse** |
| running + dead port + old (zombie) | ✗ probe fails | n/a (running) | ✓ | **replace** |
| running + dead port + booting (<30s) | ✓ booting | n/a | — | **reuse** |
| stopped | ✗ not running | ✓ | — | **start** |
| stopped + start fails | ✗ | ✗ errored | ✓ | **replace** |
| crashed | ✗ | ✓ | — | **start** |
| record present, msb gone (stale) | ✗ | ✗ no row | ✓ | **replace** |
| msb gone + dir exists | ✗ | ✗ | ✓ | **replace** |
| nothing exists | — | — | — | plain **start** (chain irrelevant) |

**Chain exhaustion without success → error listing the attempts in order**
(e.g. `conflict chain exhausted for 'litellm': reuse (probe failed), start
(not applicable) — no strategy succeeded; use --replace or down first`).

### U3. Strategy chains — validation, merge, orthogonality

**Validation (schema):**

- non-empty (a chain must have ≥ 1 element);
- duplicates rejected (`["reuse", "reuse"]` → error);
- unknown elements rejected (deny-unknown posture — serde closed vocabulary);
- elements AFTER `fail` rejected. **Justification:** `fail` is terminal by
  definition — it always errors when reached, so anything after it is dead
  config that can never execute. Rejecting it makes the config honest and
  matches the house deny-unknown/closed-vocabulary posture (same reasoning
  as `deny_unknown_fields` on config structs).
- scalar normalizes to a singleton chain at parse (back-compat).

**Merge:** whole-field last-layer-wins (consistent with the existing
`depends_on` merge — the whole spec including `on_conflict` is replaced per
dep, last layer wins). Scalar vs list forms normalize at parse, so a higher
layer re-declaring `on_conflict = "reuse"` RESETS the chain to `["reuse"]`
(never appends).

**Orthogonality:** `instance = shared|scoped|fresh` (SELECTION) stays
orthogonal; the chain is DISPOSITION on the selected slot. They compose:
`scoped` + `["reuse","start","replace"]` = reuse my scoped litellm if
healthy, start it if stopped, replace if zombie.

**Q2 RESOLVED:** the stopped-sandbox disposition question (START vs REPLACE)
is answered by the default chain — start-then-replace. A stopped sandbox is
STARTED (state preserved); only if the start fails does the chain fall
through to replace.

### U4. d452575 impact — `decide_dep_disposition` changes + new tests

- `DepConflict` (config/types.rs:525) becomes a chain type: the enum variants
  map to one-element chains; a new `ConflictChain` (Vec of steps) with scalar
  deserialization to a singleton. `DepConflict::Reuse/Replace/Fail` remain as
  the scalar sugar.
- `decide_dep_disposition` (deps.rs:165) changes from a single match to a
  chain iteration: return the FIRST disposition whose precondition holds.
  The fact set GROWS: `msb_running: bool` → `msb_status: Option<SandboxStatus>`
  (to distinguish Stopped/Crashed for `start`). The executor additionally
  handles a FAILED start (the `start` element was attempted, errored →
  continue to the next element).
- **New pure-decision test cases** (the d452575 decision-table tests grow):
  1. default chain, running+healthy → Reuse;
  2. default chain, stopped → Start;
  3. default chain, stopped + start-fails → Replace (executor retry);
  4. default chain, zombie (running, dead port, old) → Replace;
  5. default chain, stale record (record present, msb gone) → Replace;
  6. default chain, booting (running, dead port, <30s) → Reuse;
  7. `["replace"]` (scalar) → always Replace;
  8. `["fail"]` (scalar) → Fail when occupied, Start when free;
  9. `["reuse", "fail"]` → Reuse when healthy, Fail when zombie (no replace);
  10. `["start", "replace"]` → Start when stopped, Replace when running-zombie;
  11. `["reuse"]` on a zombie → chain-exhausted error listing attempts;
  12. validation: empty / duplicate / unknown / after-fail all rejected;
  13. scalar normalization round-trip (`"reuse"` ⇄ `["reuse"]`);
  14. merge last-layer-wins (scalar resets a list chain).

### U5. Port `on_occupied` — auto semantics from code (verified)

- `probe_free_ports` (store.rs:431): `TcpListener::bind((bind, 0))` — the OS
  assigns ANY free port in the EPHEMERAL range (Linux default 32768–60999).
  It skips registry-recorded ports on the same bind (defense-in-depth) and is
  NOT a reservation (the post-create atomic check+register closes the TOCTOU).
- `apply_auto_ports` (run.rs:322): per `host = 0` port, probe 1 free port,
  reject candidates colliding with already-concrete hosts in the plan,
  bounded retries (16), fail-closed.
- **So `auto` today = OS-assigned ANY-free ephemeral port** — NOT adjacent to
  the preferred port, NOT stable across restarts (the ephemeral range
  changes). That is correct for "I don't care about the port", but it is the
  wrong default for litellm-style workloads where adjacency is valuable.

### U6. Port `on_occupied` — `increment` + chain symmetry

**New element `increment`:** try `preferred+1`, `preferred+2`, … up to a
bound. Each candidate is probed before bind (occupied → next). Adjacent
ports are predictable, discoverable, and stable across restarts (main
litellm on 4000, dev on 4001).

**Naming rationale:** `increment` (RECOMMENDED) vs `next_available` vs
`next`. `next_available` is ambiguous — it could mean "the next free port
after preferred" (stepwise) or "any free port" (which is what `auto` does);
`next` is too terse and equally ambiguous. `increment` is precise:
stepwise-adjacent increments from the preferred port.

**Forms (refined 2026-08-16 — explicit RANGE property):**

```toml
# bare: default band preferred+1 .. preferred+100 (adjacent)
on_occupied = "increment"
# count-bound, relative — the simple parameterized form (kept)
on_occupied = { increment = { limit = 100 } }
# explicit absolute band — the recommended parameterized form (user's ask)
on_occupied = { increment = { range = [5000, 5100] } }
```

- **Bare `"increment"`** — default band `preferred+1 .. preferred+100`.
- **`{ increment = { limit = N } }`** — count-bound, relative: candidates
  `preferred+1 .. preferred+N`. Kept as the simple parameterized form.
- **`{ increment = { range = [START, END] } }`** — explicit ABSOLUTE band:
  candidates `START..=END` probed in order. The band MAY start at
  `preferred+1` (adjacent band) or be FULLY DISJOINT from preferred
  (dedicated band elsewhere — e.g. try 4000, else scan 5000–5100); both are
  legal and deliberate.

**Why `range` is the recommended parameterized form (over `limit`):**
`range` is self-documenting — `[5000, 5100]` states the exact band an
operator is reserving, with no arithmetic against `preferred`; it decouples
the band from the preferred port (a disjoint band is expressible directly,
whereas `limit` can only express adjacency); and `limit` is a DERIVED special
case of `range` (`limit = N` ⇔ `range = [preferred+1, preferred+N]`). Keeping
`limit` as the simple relative form is harmless (one derived case), but
`range` is the form to reach for when the band matters.

**Chain symmetry — YES.** `on_occupied` accepts a scalar OR an ordered list
from the closed vocabulary `{auto, increment, fail}`, with the SAME rule as
`on_conflict` (scalar = singleton chain; `fail` terminal — elements after it
rejected; non-empty; no duplicates; unknown rejected). Rationale: the user's
chain idea is general — conflict disposition is a chain, and port fallback is
the same shape (try preferred, then stepwise, then any-free, then error).
Uniformity keeps ONE mental model and ONE validation rule. The validation
stays simple because the vocabulary is small and closed.

**Updated surface:**

```toml
[workloads.litellm.instance]
port = { preferred = 4000, on_occupied = ["increment", "auto"] }  # RECOMMENDED default
# on_occupied = "auto"        # scalar = ["auto"]: any-free ephemeral
# on_occupied = "fail"        # scalar = ["fail"]: strict (occupied → error)
# on_occupied = "increment"   # scalar = ["increment"]: stepwise only
# on_occupied = ["increment", "auto"]  # stepwise, then any-free, then error
# on_occupied = { increment = { limit = 100 } }          # count-bound, relative
# on_occupied = { increment = { range = [5000, 5100] } } # explicit absolute band
# on_occupied = [{ increment = { range = [5000, 5100] } }, "auto"]  # band, then any-free
```

**Default `on_occupied`** (when the table form is used without it):
`["increment", "auto"]` — SUPERSEDES the earlier `"auto"` recommendation
(adjacency is more predictable and stable).

**Behavior matrix** (facts: preferred occupied? increment candidates free?):

| preferred | increment candidates | auto | result |
|---|---|---|---|
| free | — | — | **preferred** |
| occupied | preferred+1 free | — | **preferred+1** |
| occupied | +1..+limit all occupied | any-free available | **any-free ephemeral** |
| occupied | +1..+limit all occupied | none (exhausted) | **error listing attempts** |
| occupied | disjoint band [5000,5100]: 5000 free | — | **5000** (band scanned in order) |
| occupied | disjoint band exhausted | any-free available | **any-free ephemeral** |

**Validation:** same rules as `on_conflict` (non-empty, no duplicates, no
unknown, no after-`fail`); `preferred` in 1..=65535; `limit ≥ 1` and
`preferred + limit ≤ 65535`; `range`: `START ≤ END`, both in 1..=65535;
`START == END` is legal (single candidate) and is equivalent to `limit = 1`;
an empty/degenerate range (`START > END`) is rejected.

**Chain interplay:** a range exhausted (every candidate occupied) advances to
the next `on_occupied` element (e.g. `"auto"`) or, at chain end, produces the
attempts-listed error (U2 pattern).

### U7. Updated litellm test-first sequence (SUPERSEDES T4)

1. **Capsule change:** litellm workload.toml gains
   `[workloads.litellm.instance] port = { preferred = 4000, on_occupied = ["increment", "auto"] }`
   (requires Phase 1 schema; the sequence runs at Phase 3).
2. **Test A — preferred path (free host):** `workload up litellm` → binds
   4000; assert `workestrate ps --json` port_pairs host == 4000; assert
   prime's `LITELLM_ADDR` == `host.microsandbox.internal:4000`; assert
   models.json baseUrl renders `:4000`; prime chat works through it.
3. **Test B — increment path (4000 occupied):** occupy 4000 (dev litellm or
   a dummy listener on 127.0.0.1:4000) → `workload up litellm` → lands 4001;
   assert the record carries 4001; assert `LITELLM_ADDR` + models.json render
   `:4001`; prime chat works through it.
4. **Test C — increment skip + explicit range (4000 AND 4001 occupied):**
   occupy both → lands 4002; assert the record carries 4002; exports render
   `:4002`. Then pin an explicit DISJOINT band —
   `on_occupied = { increment = { range = [5000, 5002] } }` — with 4000
   occupied → lands 5000 (band scanned in order); occupy 5000 → lands 5001.
5. **Test C2 — validation-failure cases:** `range = [0, 100]` and
   `range = [65536, 70000]` rejected (outside 1..=65535); `range = [5000,
   4999]` rejected (`START > END`); `limit = 0` rejected.
6. **Test D — auto fallback (4000..4100 occupied, default +100 band
   exhausted):** → any-free ephemeral; assert the record carries the
   effective port; exports render it.
7. **Smoke update:** replace the six hardcoded `:4000` references with the
   effective port read from `workestrate ps --json` (or the registry record);
   the `models-json` assert becomes value-driven on the effective port.
8. **Regression:** re-run the full prime-smoke on ALL paths (A–D + C2);
   `just verify` green.

### U8. Acceptance criteria addition

Add to §5:

> 11. Conflict chains are attempted IN ORDER; a chain-exhausted failure
>     reports the attempt sequence (e.g. `reuse (probe failed), start (not
>     applicable)`). Port `on_occupied` chains likewise (preferred →
>     increment → auto → error). Increment RANGES are respected (adjacent or
>     disjoint band, scanned in order); a range exhausted advances the chain.

### U9. Open questions update (SUPERSEDES T5)

**Resolved by this addendum:**

- **Q2 (stopped-sandbox disposition):** RESOLVED by the default chain
  `["reuse", "start", "replace"]` — start-then-replace (U3).

**Still open for the user:**

- **Q5 (`instances` verb):** new verb vs extending `workloads`/`ps`.
  Recommendation unchanged (new verb, ADR 0027 style).
- **Q6 (bare `workload up` batch):** adopt the reconcile step in Phase 0.2
  vs stay conservative until Phase 2. Recommendation unchanged (adopt in
  Phase 0.2).
- **Namespace merge collision:** two repos declaring the same workload name
  still collide in the merged config (last layer wins). Acceptable (coexist
  via parallel instances / distinct names), or namespace-aware merge (bigger
  change, out of scope)?
- **Q3 sub-question (declared default id vs auto-slug for the workload's own
  parallel starts):** auto-slug recommended; a declared default id remains
  possible.
- **Q4 sub-question (stronger validated `version` field vs `label`):**
  `label` recommended; a validated `version` field remains possible.

### U10. Phased plan updates (SUPERSEDES the T6 rows where they conflict)

| Phase | Updated scope | Gates |
|---|---|---|
| **0 — Immediate small fixes** | 0.1 status-aware + dir-aware occupancy; 0.2 extract the shared reconcile step from d452575 + wire into named `up`/`exec`, `down`, bare `workload up`; 0.3 default conflict CHAIN `["reuse","start","replace"]` for named verbs (code-level default; d452575 scalar `on_conflict` still accepted). | AC 1–5 host (KVM); `just verify`; no schema change. |
| **1 — Instance policy schema** | `[workloads.<name>.instance]` (`strategy`, `on_conflict` CHAIN scalar-or-list, `port` union, optional `label`); chain validation (non-empty, no dup, no unknown, no after-`fail`, scalar normalization); `port` union shape: scalar `"auto"`/`"fail"`/`"increment"` OR parameterized `{ increment = { limit = N } }` / `{ increment = { range = [START, END] } }` (untagged serde union; range validation START ≤ END, 1..=65535); merge (whole-field last-layer-wins, scalar resets list); extended `decide_dep_disposition` (iterate chain, `msb_status` fact) + the U4 test matrix; `generate-schema` + drift guard. | schema parse/deny/round-trip incl. chain + range cases; merge tests; drift guard; `plan` renders the chain disposition. |
| **2 — depends_on scoping + parallel semantics** | unchanged (T1 namespace scoping; T2 `depends_on.<dep>.instance = shared\|scoped\|fresh`); the chain composes with selection. | unchanged. |
| **3 — Dynamic port policy + litellm migration** | `instance.port` union gains `on_occupied` CHAIN (`{auto, increment, fail}`, scalar-or-list) + parameterized `increment` (`limit` count-bound, default +100; `range` absolute band); `apply_auto_ports` extension (preferred → increment [limit|range] → auto, each candidate probed before bind); the U7 test sequence (Tests A–D + range-pinned Test C + C2 validation-failure cases); smoke update. | U7 steps 2–8 all pass; AC 7 + 11; `just verify`. |
| **4 — Observability surface** | unchanged. | unchanged. |

**Dependencies / seams:** unchanged from T6 — nothing hard-blocks on the
pending msb v2/develop work; the chain's `start` element uses the existing
`handle.start()` surface (0.6.8); re-verify the reconcile step's msb
mutations after the v2 move.

### U11. What this addendum does NOT change

- §4.1 (policy surface) stands except `on_conflict` becomes a chain;
- §4.2/§4.3 (reconcile algorithm, per-dep on_conflict) are SUPERSEDED by U1–U4
  (the reconcile step iterates the chain; per-dep `on_conflict` is a chain
  too, with the same precedence: `depends_on.<dep>.on_conflict` >
  `workloads.<dep>.instance.on_conflict` > default `["reuse","start","replace"]`);
- T1–T3 (namespace scoping, parallel deps, dynamic-port downstream
  implications) stand; T3.2's `on_occupied = "auto"|"fail"` surface is
  SUPERSEDED by U5–U6;
- §7 (out of scope) stands.

---

## Addendum (2026-08-16): Phase 0 IMPLEMENTED — conflict chains + shared reconcile + status/dir-aware occupancy

**Status update:** Phase 0 (U10 P0 rows 0.1–0.3) is IMPLEMENTED on
`migration/tool-model` (NO push). The remaining phases (1–4) stay as designed.

### P0.1 — What landed (commit refs)

| commit | scope |
|---|---|
| `05fd648` | **Conflict chains** (U1–U4): `DepConflict` (config/types.rs) becomes a chain type — `Vec<ConflictStep>` over `{reuse, start, replace, fail}` with scalar deserialization to a singleton (back-compat: d452575's `"reuse"` = `["reuse"]`); validation (non-empty, no dup, no after-`fail`, unknown rejected); default chain `["reuse","start","replace"]`. `decide_dep_disposition` (deps.rs) iterates the chain via the shared `decide_chain`; `DepDisposition::StartExisting` added; the executor starts a stopped/crashed dep sandbox via the detached child and ADVANCES the chain past a failed `start`. Merge: whole-value last-layer-wins (a higher-layer scalar RESETS a lower-layer list). |
| `bf9f9d4` | **Status+dir-aware occupancy + shared reconcile on the NAMED path** (U2, P0.1/P0.2): new `reconcile` module (facts from all three stores + `decide_chain`); `build_sandbox` routes through the default chain — Reuse (healthy/booting → no-op success), StartExisting (`msb handle.start()` on Stopped/Crashed + record registration + service re-run), Replace (zombie/stale/lingering-dir → idempotent teardown + fresh create), Fail (canonical refuse), chain-exhaustion error listing attempts. Detached `up` short-circuits Reuse/Fail in the PARENT (FS-8 grace). `--replace` flag behavior unchanged. |
| `d9df203` | **Schema regen**: `on_conflict` renders scalar-or-list; `ConflictStep` gains `start`. In-repo artifacts regenerated + template synced. |
| `c440acb` | **Fix (review)**: the chain's `replace` teardown now also removes the lingering sandbox DIRECTORY when the msb DB row is gone — without it the fresh create hit the msb create gate's opaque `SandboxAlreadyExists` (the ADR's "msb gone + dir exists" row). |

**Behavior matrix (U2) verified by unit tests for BOTH paths** (the dep path
and the named path share `reconcile::decide_chain`): running+healthy →
reuse; zombie (running + dead port + old) → replace; booting → reuse;
stopped/crashed → start (StartExisting); stale record → replace; msb gone +
dir exists → replace; nothing exists → plain start; msb unavailable +
record → fail (fail-closed). Chain exhaustion → error listing attempts
(`conflict chain exhausted for '<instance>': reuse (probe failed), start
(not applicable) — …`).

### P0.2 — Assessed and DELIBERATELY DEFERRED (note in the ADR)

- **Bare `workload up` batch (Q6):** the recommendation was adopt the
  reconcile step in P0.2. The batch's per-start correctness IS covered in
  this phase — each planned start spawns a detached child whose
  `build_sandbox` reconciles through the chain (a record-missing-but-msb-
  occupied slot is reused/started, never blind-created). What is NOT yet
  wired is the EXPLICIT parent-side reconcile in `cmd_workload_up_all`
  (plan_bare_up's record-as-authoritative AlreadyRunning skip does not yet
  re-probe a record-present-but-zombie slot, and the "started" message does
  not distinguish a reused slot). That stays a P0.2 follow-up — small,
  contained, and orthogonal to the correctness the child-side reconcile
  already provides.
- **`down` (where applicable):** assessed — `down_one` is already idempotent
  across the record + msb row + zombie (stop_and_remove / unregister /
  policy-dir cleanup). The lingering-DIR cleanup happens via the replace
  disposition on the up side (P0.1), not on `down` (down must not destroy
  the sandbox dir's logs while the operator may still want them). No change
  needed this phase.
- **Consumer schema copies:** the on_conflict schema changed. In-repo
  artifacts (schemas/ + templates/) are regenerated + drift-green. The
  PERSONAL-REPO consumer copies (registered config repos carrying a
  `schemas/` dir) need a host-side `workestrate schemas update` after the
  push — not touched from this phase.

### P0.3 — Gates

`cargo fmt --check`, `cargo clippy --all-targets -D warnings`, FULL
`cargo test` (957 baseline → 993 total, 0 failures, incl. the U4 14-case
decision suite + the reconcile behavior-matrix tests), `just lint-nix`,
schema drift guards (committed + subschema) green. `schema-sync-check`
reports ONLY the container-local consumer copies stale (tool home + personal
store clone) — the documented host-side follow-up.
