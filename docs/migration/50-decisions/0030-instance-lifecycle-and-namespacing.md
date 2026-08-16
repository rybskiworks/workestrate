# ADR 0030 (DRAFT): Workload instance lifecycle + conflict management + namespacing

**Status:** DRAFT — proposal for adjudication (no code changes this round)
**Date:** 2026-08-16
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
