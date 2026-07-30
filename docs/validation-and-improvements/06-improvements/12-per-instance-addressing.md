# 12 — Per-instance addressing + discovery-lite

> **STATUS: IN-PROGRESS (Wave 1 / 12a code landing on migration/tool-model; Experiment E1 + guest-reachability reassessment NEEDS-KVM)**
> **Effort:** M
> Prerequisites / see-also: [README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [ADR 0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) ·
> [ADR 0021](../../migration/50-decisions/0021-instance-lifecycle-model.md) ·
> [../05-host-validation.md](../05-host-validation.md) Experiment E1.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). NOTE: cargo-linked gates are NOT runnable here — no `cc` linker; they run on the host (HOST-NIX devshell) |
| `HOST-NIX` | Requires nix on the user's host (this container has no nix / no `cc` linker) |
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

## 3. Wave 2 (12b) discovery-lite sketch

- `depends_on` declaration in `workestrate.toml`.
- UNCONDITIONAL plan-time resolution (singleton default).
- Env injection (resolved address into the dependent's env).
- Egress derivation (dependent's egress gains the dep's `bind:port`).
- `--use <dep>@<instance>` = instance-selection override only.
- `required = true` and not running → refuse at plan time with remediation
  (`workestrate <dep> up`), no auto-start v1.

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
