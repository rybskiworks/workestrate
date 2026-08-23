# ADR 0032: Deployment versioning, provenance, and cleanup (blue-green/promote deferred)

**Status:** Accepted (blue-green/promote deferred)
**Date:** 2026-08-23
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

- **Tag format**: `name:ctx` + `name:sha` alias pair vs single `name-ctx:sha`.
- **GC policy for hash tags**: keep-last-N vs manual.
- **Promote command shape when un-deferred**: explicit `workload promote` vs
  chain element promote-if-healthy.
- **Whether models.json seed content belongs in the config hash** or stays a
  `--reseed` concern.
