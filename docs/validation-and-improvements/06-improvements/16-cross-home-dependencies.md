# 16 — Cross-home dependencies

> **STATUS: INTENT-TO-EXPLORE (2026-08-01 — questions to investigate, NO decisions)**
> **Effort:** exploration
> Prerequisites / see-also: [../README.md](../README.md) ·
> [00-index.md](00-index.md) ·
> [12-per-instance-addressing.md](12-per-instance-addressing.md) ·
> [ADR 0019](../../migration/50-decisions/0019-contexts-and-user-global-overrides.md) ·
> [ADR 0023](../../migration/50-decisions/0023-single-tool-home.md) ·
> [ADR 0026 addendum (2026-08-01)](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md) (compose-mirrored default-on dependency lifecycle) ·
> [ADR 0021](../../migration/50-decisions/0021-instance-lifecycle-model.md) (addendum 2026-08-01: bare `workload up` topo-starts all service-kind workloads).

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs-only exploration (`verifiable-here`); it gates no code.

---

## Summary

A wider `workestrate up` across homes / config sets: the default-on
lifecycle ([ADR 0026 addendum (2026-08-01)](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md);
the spec 12 supersession in
[12-per-instance-addressing.md](12-per-instance-addressing.md) §3) makes
dependency-closure start automatic **within one home**. This spec explores
what happens when the workloads to compose do not live in the same home or
config set.

---

## 1. What already works today

Cross-config-REPO dependencies within ONE merged context are free via
layering: `depends_on` keys on workload name in the merged config,
regardless of which config repo defines it. `ConfigFile.workloads` is a
single post-merge HashMap (`types.rs:492-499`); layers merge
union-by-dependency-name (`types.rs:458-463`); discovery resolves against
the merged config + the home's port registry. A litellm defined in repo A
and a pi defined in repo B compose today, provided both repos layer into
the same home/context.

---

## 2. The real gap — cross-home

Homes are isolated by design (ADR 0023): per-home state dirs, per-home
registries, per-home port registries, per-home addressing (the loopback
allocator is per-registry). The blind spot: home A's port registry cannot
see home B's running records — a `depends_on` resolution in home A against
a litellm running under home B finds NO record. Also: two homes' loopback
allocators can both hand out `127.0.0.2` (bind-allocation collision domain
across homes).

---

## 3. Design space (candidates, NOT decisions)

- **Service discovery across homes** — shared/inter-home registry view;
  well-known cross-home address; explicit address handoff.
- **Shared vs per-home bind allocation** — a system-wide loopback
  allocator vs a per-home allocator + collision rules.
- **The ADR 0019 multi-context batch relationship** — contexts already
  batch multiple config sets within one home; is cross-home "more
  contexts" or a new tier? The ADR 0021 addendum (2026-08-01) makes bare
  `workload up` a SINGLE-context topo batch that explicitly does NOT
  reopen ADR 0019's multi-context deferral, so cross-home/cross-context
  widening is genuinely unexplored.
- **E1 / dynamic-port interaction** — guest reachability E1 stays
  NEEDS-KVM; `--port-auto` dynamic ports make cross-home hardcoding
  impossible, so any cross-home scheme must ride the actual-record
  injection (spec 12 §4).

---

## 4. Questions to investigate (NOT decisions)

1. Should contexts span homes?
2. How should a cross-home `depends_on` be addressed — home-qualified
   workload names? an explicit endpoint escape hatch?
3. Should there be a shared "litellm-as-a-service" home that other homes
   consume?
