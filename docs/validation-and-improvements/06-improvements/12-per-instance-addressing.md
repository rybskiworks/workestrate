# 12 — Per-instance addressing + discovery-lite

> **STATUS: IMPLEMENTED (Waves 1+2 landed — Wave 1: 9107b87 / de9aa62 / f9fd2f0 / c5837e7; Wave 2: 4adad3f / 7b65ad1 / 39c1694; Experiment E1 guest-reachability + the deferred binding decision remain NEEDS-KVM per ADR 0026; refuse-only no-auto-start semantics SUPERSEDED 2026-08-01 by the compose-mirrored default-on dependency lifecycle (ADR 0026 addendum); `--use` override retained; W5 config-wiring plan added (§4))**
> **Effort:** M
> Prerequisites / see-also: [README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [ADR 0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) ·
> [ADR 0026 addendum (2026-08-01)](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) (compose-mirrored default-on dependency lifecycle) ·
> [ADR 0021](../../migration/50-decisions/0021-instance-lifecycle-model.md) ·
> [../05-host-validation.md](../05-host-validation.md) Experiment E1.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

All gates in this spec are cargo-linked → run via `nix develop` / HOST-NIX
devshell. Experiment E1 is `HOST-KVM`.

---

## Summary

Executes ADR 0026. **12a (Wave 1)** = slot-based binding + bind-aware
registry + address surfacing + `--port-auto` + removal of `--port-offset`.
**12b (Wave 2)** = `depends_on` discovery-lite.

---

## 1. Addressing model

| Slot kind | Bind | Ports |
|---|---|---|
| Singleton slot (`<workload>` / `<context>-<workload>`) | `127.0.0.1` (shared) | declared ports (UNCHANGED — the well-known address static configs use) |
| Parallel slot (`<slot>@<id>`) | `127.0.0.N` (`N >= 2`, lowest free, locked allocator in the port registry, freed on unregister, stale records reserve) | declared ports on the per-instance IP |

- Collisions are keyed on `(bind_ip, port)`: the same port on different bind
  IPs does not collide; the same `(ip, port)` refuses with remediation.
- Legacy records without `bind_ip` are treated as `127.0.0.1`.

---

## 2. Wave 1 (12a) work packages

Four commits, each fully gated (`cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, full `cargo test`, `just golden-check` (3
MATCH), `just schema-check`, `just spec-examples`, `just scaffold-check`,
clean `control/agentctl/Cargo.lock`).

### C0 — `refactor(agentctl)!: remove --port-offset (superseded by ADR 0026)`

Remove the flag from `ServiceAction::Up`/`Plan` + `AgentAction::Exec`/`Plan`
(`cli_actions.rs`), `build_instance_spec` + `parse_port_offset` + dispatch
match arms (`commands/lifecycle.rs`), `InstanceSpec.port_offset`
(`runtime/mod.rs`), `offset_port` + its uses (`runtime/run.rs`),
`SandboxInstanceRecord.port_offset` (`port_registry/mod.rs`) +
register/collision signatures (`port_registry/store.rs`),
`PsEntry.port_offset` (`runtime/ps.rs`) + `PsEntryJson.port_offset`
(`json_out.rs`) + `cmd_plan` offset branches (`commands/diagnostics.rs`),
`detach_args` forwarding (`microsandbox/workload/mod.rs`), all affected tests
(`main.rs`, `store.rs`, `ps.rs`, `diagnostics.rs`, `lifecycle.rs`). Golden
plans unaffected (offset was never rendered).

### C1 — `feat(agentctl): bind-aware port registry + loopback allocator`

- `bind_ip: IpAddr` (serde default `127.0.0.1`) on `SandboxInstanceRecord` +
  `PortMapping` (`plan.rs`).
- Locked allocator `allocate_loopback_ip` (lowest free `127.0.0.N`, `N >= 2`,
  across records; freed on unregister; stale-record rule documented).
- `check_port_collisions` keyed on `(bind_ip, port)` — `store.rs` signatures
  + `run.rs` call sites (`run.rs` passes `127.0.0.1` for all slots in C1;
  per-IP wiring is C2).
- Tests: same port different IPs OK; same `(ip, port)` refuses; allocator
  reuse after `down`; legacy records (no `bind_ip`) treated as `127.0.0.1`.

### C2 — `feat(agentctl): parallel slots publish on per-instance IPs`

- Runtime wiring — parallel slots allocate + publish via
  `.port_bind(ip, host, guest)`; singleton unchanged (`.port`, shared bind
  `127.0.0.1`).
- `plan --instance <id>` renders the prospective parallel plan (bind computed
  from the registry view, read-only).
- Plan `Display` renders the bind ONLY when `!= 127.0.0.1` (conditional
  additive — existing golden plans stay byte-identical).
- New golden fixture for a parallel-slot plan (`example-service`) with
  Rust-side byte-parity test (hermetic via isolated state home).
- Tests.

### C3 — `feat(agentctl): address surfacing + --port-auto`

- `ps` / `ps --json` / `plan` show `bind_ip:port` (bind shown when
  `!= 127.0.0.1` in text; JSON gains the field additively).
- `--port-auto` on `up`/`exec` (lock-probed free port on the slot's bind,
  recorded in the instance record).
- `detach_args` forwards `--port-auto`.
- Tests.

---

## 3. Wave 2 (12b) discovery-lite — LANDED

All Wave 2 acceptance items landed (schema: `4adad3f`; resolution /
injection / egress: `7b65ad1`; `--use` overrides: `39c1694`):

- [x] `depends_on` declaration in `workestrate.toml` (`env` + `required`,
      `required` defaulting `false`) — `4adad3f`.
- [x] UNCONDITIONAL plan-time resolution (singleton default) — `7b65ad1`.
- [x] Env injection — the resolved address is injected into the dependent's
      env in the guest-visible `host.microsandbox.internal:<published-port>`
      form — `7b65ad1`.
- [x] Egress derivation — the dependent's egress gains a derived
      `tcp:<port> -> host` allow rule for the dep's `bind:port` (additive
      only; `default_deny` is never touched) — `7b65ad1`.
- [x] `--use <dep>@<instance>` = instance-selection override only, with hard
      plan-time errors: `--use` on a workload with no declared `depends_on`,
      `--use` naming an undeclared dep, and `--use` naming an instance that
      is not running all refuse (no declared-port fallback) — `39c1694`.
- [x] `required = true` and not running → refuse at plan time with
      remediation (`workestrate workload up <dep>`), no auto-start v1 — `7b65ad1`.

> **SUPERSEDED (2026-08-01, ADR 0026 addendum):** the refuse-only
> "no auto-start v1" stance above is superseded by the compose-mirrored
> default-on dependency lifecycle
> ([ADR 0026 addendum (2026-08-01)](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md)).
> Superseded rule:
>
> - "`required = true` and not running → refuse at plan time with
>   remediation, no auto-start v1" — replaced by default-on start.
>
> **RETAINED:** `--use` REMAINS a pure instance-selection override (unknown
> instance → hard error; NO parallel auto-start) per the same addendum.
>
> The default-on lifecycle semantics (ADR 0026 addendum): declared
> `depends_on` deps start **by default** on `up`/`exec` — topo-ordered
> closure, singleton slots only; service-kind dependencies start detached
> with bounded wait-for-port readiness (~15s); agent-kind dependencies
> **refuse** with remediation; `--no-deps` opts out (required dep →
> refuse-with-remediation; optional dep → fall back per convention + warn);
> an occupied slot counts as satisfied; `plan` **never** starts anything.
>
> Two new mandatory rules from the addendum: **cycle detection** becomes
> MANDATORY in config validation (currently ABSENT — A→B→A loads cleanly
> today); and the **construction-order rule** — deps must start BEFORE the
> dependent's `ConfigWorkload` is constructed.
>
> The landed Wave 2 record above is kept verbatim as **HISTORY**. The
> env-injection + egress-derivation mechanics (declaration-driven
> unconditional plan-time resolution) are **NOT superseded** — they compose
> with the default-on lifecycle. The declared-port fallback leg of the
> refusal/fallback matrix (`discovery.rs:22-29`) **is** superseded by the
> actual-record read (W4). E1 deferral is UNCHANGED.

---

## 4. W5 — config wiring: declare depends_on, retire hardcoded URLs (2026-08-01, ADR 0026 addendum)

The W5 wave of the default-on W-wave wires the lifecycle from the
[ADR 0026 addendum (2026-08-01)](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md)
into the real configs and retires the hardcoded URLs it replaces.

1. **Declare `depends_on` in `config.reference/workestrate.toml` and the
   personal config (`.tmp/config-repos-export/personal/workestrate.toml`).**
   Every agent workload (pi, odysseus, opencode, tempest) gains
   `[workloads.<agent>.depends_on.litellm]` with `required = true` and an
   address env var (e.g. `env = "LITELLM_ADDR"`).
2. **Retire the hardcoded URLs** — four literals to remove:
   - pi `models.json` `"baseUrl": "http://host.microsandbox.internal:4000/v1"`
     at `config.reference/agents/pi/config/models.json:5` and
     `.tmp/config-repos-export/personal/agents/pi/config/models.json:5`
     (also the live copy `workspaces/pi-state/agent/models.json:5`);
   - odysseus `OPENAI_BASE_URL` at
     `.tmp/config-repos-export/personal/workestrate.toml:159`;
   - opencode `OPENAI_BASE_URL` at
     `.tmp/config-repos-export/personal/workestrate.toml:218`;
   - tempest `TEMPEST_LOCAL_BASE_URL` at
     `.tmp/config-repos-export/personal/workestrate.toml:282`.
3. **Switch to the injected address.** `depends_on` env injection reads the
   ACTUAL running record from the port registry — the actual assigned port
   (`SandboxInstanceRecord.port_pairs`, `port_registry/mod.rs:24-41`), NOT
   the declared port — and the dependent composes its URL via env templating
   (`${VAR}` resolution, `env.rs:18-41`), pattern
   `OPENAI_BASE_URL = "http://${LITELLM_ADDR}/v1"`. The injected value is
   the guest-visible form `host.microsandbox.internal:<actual-port>`
   (`discovery.rs:12-15`, `discovery.rs:45`). Declared env still wins over
   injection (`discovery.rs:597-610`) — the templated composition is the
   declared env consuming the injected var.

---

## 5. Follow-ups

- `DependsOnSpec` gains additive `scheme` / `path_suffix` fields (today only
  `env` + `required`, `types.rs:409-421`; additive via `#[serde(default)]`
  per ADR 0021 §8) — retiring URL composition entirely: the dependency
  declaration carries scheme + path suffix and injection renders the full
  URL, so no dependent templates `http://${LITELLM_ADDR}/v1` by hand.
- Readiness/restart posture note: v1 readiness is wait-for-port (~15s) on
  the host-published port; guest healthchecks (image-declared,
  compose-style) are v2+. Restart posture is undecided — today an occupied
  slot = satisfied and a crashed dependency is not restarted.

---

## Open decisions

**E1 hook** — Experiment E1
([../../05-host-validation.md](../05-host-validation.md)) decides
guest-facing addressing for alternates:

- `127/8` reachable → per-IP guest-facing.
- `0.0.0.0`-only → providers publish `0.0.0.0`.
- `127.0.0.1`-only → shared-bind `--port-auto` for guest-facing alternates
  (the conservative default until E1 runs).

Record the outcome here when E1 lands.

**Status note (2026-07-30):** Waves 1+2 code landed (see banner); this open
decision is the ONLY remaining item and stays NEEDS-KVM until Experiment E1
runs in the host batch ([../07-execution-order.md](../07-execution-order.md)
Step 6 B10).
