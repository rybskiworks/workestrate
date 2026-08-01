# 17 — Visualization + inspection surfaces

> **STATUS: INTENT-TO-EXPLORE (2026-08-01 — enumerates view candidates + data sources, NO decisions)**
> **Effort:** exploration
> Prerequisites / see-also: [../README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [12-per-instance-addressing.md](12-per-instance-addressing.md) ·
> [16-cross-home-dependencies.md](16-cross-home-dependencies.md) ·
> [ADR 0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) ·
> [ADR 0027](../../migration/50-decisions/0027-verb-first-workload-dispatch.md) (verb-first workload dispatch — the W3 `workestrate workloads` verb anchor).

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs-only exploration (`verifiable-here`); it gates no code.

---

## Summary

Beyond `workestrate workloads` (the W3 discovery verb,
[ADR 0027 (verb-first workload dispatch)](../../migration/50-decisions/0027-verb-first-workload-dispatch.md)):
which home / config / dependency views should exist, and which
data source feeds each.

---

## 1. Anchor — `workestrate workloads` (W3)

The output shape: workload name, kind (agent/service), image, slots
(declared ports / parallel-slot status), running status resolved from the
port registry.

---

## 2. View candidates

| View | Data source(s) |
|---|---|
| `workestrate workloads` | merged `ConfigFile.workloads` (`types.rs:492-499`) + port-registry records (`port_registry/mod.rs:24-41`) |
| `--json`-first surfaces for tooling | every view emits `--json` first, text second (precedent: `PsEntryJson`, `json_out.rs:44-55`) |
| dependency-graph render | edges from merged `ConfigFile.workloads[].depends_on` (`types.rs:458-463`); topo order mirrors the default-on start order (ADR 0026 addendum 2026-08-01) |
| home inspect | home registry (config.toml `[configs]` / `[settings]`), layer provenance, trust status (`control/agentctl/src/config/trust.rs`), config-repos status (clean/dirty per `config list`), `workestrate.lock` pins (`HomeLock`/`LockedRepo`, `lockfile.rs:49-68`) |
| `ps --json` consumers | `PsEntry`/`PsEntryJson` as the stable machine-readable record for external tooling |

---

## 3. Data sources inventory

- **Merged config** — `ConfigFile` (post-merge workloads, contexts,
  settings).
- **Home registry** — `registry.toml` (`[configs]` / `[settings]`).
- **Port registry** — `SandboxInstanceRecord` (running instances, ports,
  `port_pairs`, `bind_ip`).
- **`workestrate.lock`** — `HomeLock` / `LockedRepo` pins per config repo.

---

## 4. Questions to investigate (NOT decisions)

1. Which views are verbs vs flags on existing verbs?
2. What `--json` schema stability guarantees apply for external consumers?
3. Is the dependency-graph render text-only or dot/mermaid?
4. How does home inspect compose with the cross-home questions in
   [spec 16](16-cross-home-dependencies.md)?
