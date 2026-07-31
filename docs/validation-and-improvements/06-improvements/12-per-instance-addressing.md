# 12 — Per-instance addressing + discovery-lite

> **STATUS: IMPLEMENTED (Waves 1+2 landed — Wave 1: 9107b87 / de9aa62 / f9fd2f0 / c5837e7; Wave 2: 4adad3f / 7b65ad1 / 39c1694; Experiment E1 guest-reachability + the deferred binding decision remain NEEDS-KVM per ADR 0026)**
> **Effort:** M
> Prerequisites / see-also: [README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [ADR 0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) ·
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
      remediation (`workestrate <dep> up`), no auto-start v1 — `7b65ad1`.

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
