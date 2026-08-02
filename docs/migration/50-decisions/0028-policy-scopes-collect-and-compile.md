# ADR 0028: Policy scopes: collect-and-compile (never merge)

**Status:** Accepted
**Date:** 2026-08-02
**References:** ADR 0020 (Ruling 1: arrays wholesale-replace — explicitly NOT
amended), ADR 0005 (security-aware merge — adjacency: policy is collected,
not merged, so 0005's merge rules simply never apply to policy fragments),
ADR 0011 (microsandbox fork/consumption — adjacency: the v1 runtime
transmission channel depends on the nix-patched microsandbox-filesystem),
spec 22 (`docs/validation-and-improvements/06-improvements/22-dynamic-mount-masking-policy.md`,
primary), spec 01 (`docs/validation-and-improvements/06-improvements/01-mount-filtering-shadowing.md`,
fallback).

## Context

The dynamic mount masking policy (spec 22) declares `[policy.mounts]`
fragments at scopes spanning the entire layer stack: home registry
`config.toml`, user-global `overrides.toml`, the reference config,
config-repo layers, workload level, and mount entries. The existing merge
algebra is incompatible with this surface on two grounds:

1. **Arrays wholesale-replace (ADR 0020 Ruling 1).** If policy lists merged
   as arrays, a repo layer's `mask` list would REPLACE the operator's —
   destroying lower-scope rules wholesale instead of composing them.
2. **Merging destroys provenance.** Diagnostics (`explain` traces), the
   exact-duplicate conflict error, and compile-time trust validation all
   need per-layer origins (`RuleOrigin { layer, file, scope_kind }`). A
   merged structure forgets which layer contributed which rule at every
   merge boundary.

Additionally, the needed precedence semantics are path-dependent: a terminal
(`overridable = false`) match must freeze the decision **for that path**,
with later scopes unable to change it — a per-path freeze, not a per-set
precedence.

## Options considered

1. **Extend the merge algebra with a third merge kind for policy.**
   REJECTED: precedence logic would be split between the merger (ordering,
   replacement) and the evaluator (freeze, trust) with no single owner;
   provenance is still lost at merge boundaries.
2. **Merge all scopes into global mask/unmask sets.** REJECTED: loses
   path-dependent overridability/freeze semantics — a terminal decision must
   freeze per-path, not per-set; a merged global set cannot express "this
   path is frozen by the operator, that one is still provisional."
3. **Collect-and-compile.** SELECTED.

## Decision

- Policy fragments are **collected per scope in layer-stack order**; a
  dedicated **compiler** owns precedence, freeze semantics, trust
  validation, and conflict detection. `merge.rs` never sees policy.
- **Authority order = compile order**: operator scopes (home registry,
  user-global overrides) come FIRST and outrank repo/project scopes.
- **Exact-duplicate same-scope terminal mask+unmask conflict = compile
  error** naming both origins; overlapping-glob conflicts are surfaced via
  `explain`, not rejected.
- **Trust validation at compile time:** non-overridable (terminal) unmask is
  rejected from non-operator scopes; non-overridable mask is allowed from
  any scope.
- **Runtime transmission v1:** the compiled program is serialized as JSON,
  written to the state dir, injected as a read-only mount at a well-known
  guest path plus the `MSB_MOUNT_POLICY` env var, and read by the nix-patched
  microsandbox-filesystem. v2 (a proper `VolumeMount::Bind` field) is
  deferred to the upstream-PR track.
- **Spec 01 is dispositioned to fallback/degraded-mode** (the WP5
  staging-copy essence survives) and will be DELETED if spec 22 lands.

## Consequences

**Positive:**

- **Single precedence owner** — one compiler implements ordering, freeze,
  trust, and conflicts; no split-brain between merger and evaluator.
- **Full provenance** — every compiled rule carries `RuleOrigin` into the
  program; diagnostics and errors name origins.
- **Explain/preview reuse the same compiler** as runtime — no parallel
  evaluation logic to drift.
- **`merge.rs` untouched** — ADR 0020 Ruling 1 stands unamended; the
  `mounts` vec stays wholesale-replace.

**Negative / costs:**

- A **new compiler module** is built and maintained (parser → collected
  fragments → compiled program → JSON serialization).
- **Mount-entry policy comes only from the declaring layer** — a v1 semantic
  limitation, documented explicitly in spec 22 §8.3.
- The **transmission channel is a shim** (state-dir read-only mount + env
  var) until the v2 `VolumeMount::Bind` field lands via the upstream-PR
  track (ADR 0011 adjacency).
