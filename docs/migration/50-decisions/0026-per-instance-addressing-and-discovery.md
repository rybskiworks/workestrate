# ADR 0026: Per-instance addressing + discovery-lite (supersedes ADR 0021 §5 `--port-offset`)

**Status:** Accepted
**Date:** 2026-07-30
**References:** ADR 0021 (instance lifecycle), ADR 0020 Ruling 4,
`docs/validation-and-improvements/06-improvements/12-per-instance-addressing.md`
(execution spec),
`docs/validation-and-improvements/05-host-validation.md` Experiment E1.

## Context

ADR 0021 gave parallel instances `--port-offset N` (host ports `+= N`) so a
canary can coexist with the singleton. Offsets are a poor addressing model:
they say nothing about WHERE a port is bound, they break any static config
that hardcodes the well-known address (e.g. agents reach the LiteLLM proxy
at `host.microsandbox.internal:4000` — the address static configs use), and
they force every consumer to recompute arithmetic per instance.

Meanwhile the SDK already supports per-bind publishing (`.port_bind(IpAddr,
host, guest)`; `.port()` is `.port_bind(127.0.0.1, ...)`). Pre-release is the
only cheap window to replace the model — there are no released users to keep
compat for.

## Options considered

1. **Keep `--port-offset` alongside the slot model.** Rejected: dead weight
   pre-release; two addressing mechanisms for the same problem; offsets break
   static-config addressing anyway.

2. **A provider-level flag selecting bind strategy per workload.** Rejected:
   the slot model already carries the distinction — singleton vs parallel — a
   second knob duplicates it.

3. **A name-based router/registry (e.g. DNS or a proxy mapping names to
   instances).** Rejected-for-now: DEFERRED; trigger documented — adopt when
   >1 guest-facing alternate per workload must coexist AND E1 shows neither
   per-IP reachability nor `0.0.0.0` publishing is acceptable.

## Decision

(a) **SLOT-BASED BINDING** — the singleton slot publishes on the shared bind
`127.0.0.1` at the declared ports (UNCHANGED — this is the well-known address
static configs use, e.g. `host.microsandbox.internal:4000`); parallel slots
(`--instance`/`--new`) publish on per-instance loopback IPs (`127.0.0.N`,
`N >= 2`) drawn from a locked allocator in the port registry (lowest free `N`
across records; freed on unregister; stale records still reserve their IP
until cleared — conservative by design).

(b) Port collisions are keyed on `(bind_ip, port)` — the same port on
different bind IPs does not collide; the same `(ip, port)` refuses with
remediation. Legacy records without `bind_ip` are treated as `127.0.0.1`.

(c) **`--port-auto`** — pick a lock-probed free port on the slot's bind (for
cases where even the per-IP port must not be assumed); the chosen port is
recorded in the instance record.

(d) **depends_on discovery-lite** — declaring a dependency in config triggers
UNCONDITIONAL plan-time resolution (singleton instance by default), env
injection of the resolved address, and egress derivation. `--use
<dep>@<instance>` is ONLY an instance-selection override (not an on/off switch
— declaration alone activates discovery). A required dependency that is not
running → REFUSE at plan time with a remediation message; NO auto-start in v1.

(e) **`--port-offset` is REMOVED pre-release** (superseded by this ADR; no
compat framing, no deprecation window — the tool has not shipped).

(f) **DEFERRED-PENDING-E1:** guest-reachability of non-`127.0.0.1` loopbacks
is KVM-unverified. Conservative default: guest-facing alternates publish on
`127.0.0.1` with `--port-auto` until Experiment E1
(`05-host-validation.md`) proves whether a guest can reach host services
bound on `127.0.0.N` via `host.microsandbox.internal`. Reassess after E1 (3
outcomes documented in the E1 decision table).

## Consequences

- Parallel instances no longer need port arithmetic; static configs keep one
  well-known address per workload (the singleton).
- The `(bind_ip, port)` collision model makes same-port parallels legal.
- discovery-lite makes cross-workload wiring declarative.
- `--port-offset` scripts break loudly (unknown flag) rather than silently —
  acceptable pre-release.
- Historical references to `--port-offset` remain in
  `docs/migration/20-target-system-spec.md` §13 and the ADR 0021 body
  (retained for history; the ADR 0021 addendum + this ADR are the
  supersession of record).

## Rejected why

- **Keep `--port-offset` alongside the slot model:** dead weight pre-release;
  two addressing mechanisms for the same problem; offsets break
  static-config addressing anyway.
- **A provider-level flag selecting bind strategy per workload:** the slot
  model already carries the distinction — singleton vs parallel — a second
  knob duplicates it.
- **A name-based router/registry (e.g. DNS or a proxy mapping names to
  instances):** DEFERRED — adopt when >1 guest-facing alternate per workload
  must coexist AND E1 shows neither per-IP reachability nor `0.0.0.0`
  publishing is acceptable.

---

The execution spec is
`docs/validation-and-improvements/06-improvements/12-per-instance-addressing.md`
(IN-PROGRESS; guest-reachability parts NEEDS-KVM).

---

## Addendum (2026-08-01): dependency lifecycle is compose-mirrored, default-on

This addendum SUPERSEDES the "(d) NO auto-start in v1" stance of the original
decision.

- Declared `depends_on` deps now START BY DEFAULT on `up`/`exec`:
  topological-order closure over the dependency graph; singleton slots only;
  service-kind deps start detached with a bounded wait-for-port (~15s);
  agent-kind deps REFUSE with a remediation message (agents are interactive
  and cannot be auto-started detached).
- `--no-deps` opt-out: for a required dep, refuse-with-remediation; for an
  optional dep, fall back per convention + warn.
- An occupied dep slot counts as SATISFIED (no restart of an already-running
  dep).
- `plan` NEVER starts anything (plan-time resolution remains pure discovery).
- `--use` remains a pure instance-selection override: unknown instance → hard
  error; NO parallel auto-start.
- CYCLE DETECTION becomes MANDATORY in config validation: it is currently
  ABSENT (A→B→A loads cleanly today — discovery-lite has no topological need;
  ordering does).
- Construction-order rule: deps must start BEFORE the dependent's
  `ConfigWorkload` is constructed (otherwise the required-refusal fires
  first).
- The E1 deferral (per-IP guest-reachability, ADR 0026 (f)) is UNCHANGED.

---

## Addendum (2026-08-10): namespaced ports + auto-allocation + named exports

- `[[ports]]` entries gain an optional `name` (slug `^[a-z0-9][a-z0-9-]*$`); the
  UNNAMED port stays the primary (legacy `env`) port. `host = 0` marks an
  auto-allocated port: the registry probes a free port on the slot's bind at
  boot (per-port `--port-auto`), recorded in the instance record.
- `depends_on.<dep>` gains `exports = { <port-name> = "<ENV_VAR>" }`: each entry
  injects the resolved address of that NAMED port; `env` remains the primary
  port. At least one of env/exports is required; an auto port on a
  not-running dep refuses (no address until it runs).
- (Schema: `docs/migration/20-target-system-spec.md` §3.)

## Addendum (2026-08-16): idempotent dependency auto-start + on_conflict

- Auto-start now RECONCILES the planner's record-as-authoritative view with
  the msb runtime BEFORE starting a dep: a singleton slot that msb reports
  Running is treated as SATISFIED (reuse) even when no port-registry record
  exists in the active state dir. Previously the planner emitted StartService
  and the detached child hit the occupancy gate ("instance '<slot>' is
  already running", exit 3) — the observed `workload exec prime` failure.
- `depends_on.<dep>` gains `on_conflict = "reuse" | "replace" | "fail"`
  (default "reuse"):
  - "reuse": use the running instance when healthy — a short host TCP probe
    (500ms) of the published host ports. A dep with no published ports cannot
    be probed cheaply, so it is reused optimistically (use "replace" to force
    a fresh start). A slot whose record was created within the last 30s is
    treated as BOOTING and reused (never killed mid-boot). A keep-alive
    zombie (msb Running, port dead, record old) is AUTO-REPLACED (down +
    start fresh); a stale record (record present, msb NOT running) is also
    replaced (down clears it, then start fresh).
  - "replace": always down + start fresh.
  - "fail": refuse with the standard occupied-instance message (the pre-fix
    failure mode, opted in).
- Keep-alive hazard (unchanged mechanics, now handled): the sandbox entrypoint
  is `tail -f /dev/null` (run.rs), so msb "Running" does NOT imply the exec'd
  service process is alive. The reuse path therefore probes the published
  host ports instead of trusting sandbox status.
- Merge: depends_on maps merge union-by-dependency-name, last layer wins per
  dep — the WHOLE spec (including on_conflict) is replaced. A higher layer
  re-declaring a dep without on_conflict resets it to the "reuse" default.
- Bare `workestrate workload up` (all workloads) keeps record-as-authoritative
  skipping; the same msb reconciliation there is tracked as a follow-up.
