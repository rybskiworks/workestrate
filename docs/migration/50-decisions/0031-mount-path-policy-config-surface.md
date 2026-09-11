# ADR 0031: Mount path policy: unified read/write-axes config surface

**Status:** Accepted
**Date:** 2026-08-20
**References:** ADR 0029 (policy scopes: collect-and-compile — primary; its
2026-08-20 addendum records the same old→new mapping), ADR 0011 (microsandbox
fork consumption — the `write.allow` evaluator arm lives in the fork), spec
22 (`docs/validation-and-improvements/06-improvements/22-dynamic-mount-masking-policy.md`,
primary semantic reference), the mount-merge handover §9o
(`handovers/2026-08-13-mount-merge-readiness.md` — execution log of this
round), fork commits `63c0dff0` / `808bcf45` / `69b132a9` / `525c324a`,
tool commits `39fd670` / `a65bf67` / `8d429e9`.

## Context

The pre-release `[policy.mounts]` surface had grown by accretion into a
**lift of the runtime's wire vocabulary into user-facing config**: the
`mask` / `unmask` / `protect` lists, the `[policy.mounts.writes]` table,
the `masked_writes` scalar (already removed), the entry key `overridable`,
and mount-row sugar words `mask` / `unmask` / `protect` / `writes_deny`.

This is **SDK-shaped leakage**. Users had to reason in evaluator internals —
what is a mask versus a protect versus a writes rule; what does
`overridable` mean on the wire — instead of declaring intent. The
**translation principle**: config should speak the operator's intent — what
can the guest READ, what can it WRITE, is this entry FINAL — and the
COMPILER alone should translate intent into wire vocabulary (mask/unmask
effects, the `protect` bucket, `overridable`).

## Options considered

1. **Keep the wire-vocabulary lift as the config surface.** REJECTED:
   operators must learn evaluator internals to express simple intent, and
   every future evaluator evolution re-leaks into config.
2. **Rename-only aliasing (accept both vocabularies).** REJECTED: aliases
   keep both vocabularies alive indefinitely, hide the semantic mapping in
   silent sugar, and double the documented surface. This surface is
   pre-release; hard errors carry no migration debt.
3. **Unify on two intent axes and let the compiler translate.** SELECTED.

## Decision

- The config surface is UNIFIED on `[policy.mounts.read]` and
  `[policy.mounts.write]`, each with `deny` / `allow` lists. Entries are a
  compact string or `{ pattern = "...", final = true }` (`final` defaults
  false).
- Mount-entry sugar becomes `read.deny` / `read.allow` / `write.deny` /
  `write.allow` dotted keys directly on the `[[mounts]]` row (parse-time
  normalized into the row's policy fragment, as before).
- The old words are **hard unknown-field parse errors** — no aliases;
  pre-release, no migration debt.
- The **compiled-program wire format is UNCHANGED**
  (`PathPolicyRule { effect, pattern, overridable, origin }`, the `protect`
  bucket, the `writes` CompiledRuleSet, `"version": 1`): config
  `final = true` inverts onto the wire rule's `overridable = false`.

### Trust model

- A final ALLOW on either axis (`read.allow` or `write.allow`) is
  **operator-only** (home registry, user-global overrides); a final allow
  from any other scope is a compile error naming pattern, origin, and axis.
- Final DENIES are accepted from **any scope** — denying is the fail-closed
  direction.
- An **operator scope's final `read.deny` routes to the `protect` wire
  bucket**; a non-operator final `read.deny` compiles to an ordinary
  terminal mask (visibility only).
- The exact-duplicate same-scope conflict check (deny × allow, at least one
  entry final, both origins named) applies on **both axes**; the read-axis
  check covers final `read.deny` entries routed to the `protect` bucket.

### The `write.allow` evaluator arm (why the fork had to change)

Enforcement lives in the runtime evaluator, not in config. No adapter- or
config-side translation can manufacture `write.allow` semantics the
evaluator does not implement — so the fork's `decide_write` had to gain the
union arm: allow ∪ deny evaluated authority-ascending (home → config →
workload → mount), deny-before-allow within a scope, last non-frozen match
wins, terminal freeze in both directions, `protect` short-circuit
preserved, default allow. Fork commits `63c0dff0` + `808bcf45`
(authority-order review fix) on develop off `3bd051bf`.

The workestrate mirror evaluator
(`control/agentctl/src/mount_policy/program.rs` `decide_write`)
intentionally keeps the PINNED runtime (`3bd051bf`) semantics and must be
ported at re-pin time — recorded follow-up: mount-merge handover §9o step
2, and the flake.nix microsandbox-fork pin comment.

### Migration map (old → new)

| Old surface | New surface |
|---|---|
| `mask = [...]` | `[policy.mounts.read] deny = [...]` |
| `unmask = [...]` | `[policy.mounts.read] allow = [...]` |
| `protect = [...]` | OPERATOR-scope `read.deny` with `final = true` (the only protect-bucket route; the old non-terminal repo/workload protect has NO equivalent — protection is operator-final by construction) |
| `[policy.mounts.writes] { allow, deny }` | `[policy.mounts.write] { allow, deny }` |
| entry `{ pattern, overridable = false }` | `{ pattern, final = true }` |
| compact strings / `overridable = true` | non-final (`final` defaults false) |
| mount-row sugar `mask` / `unmask` / `protect` / `writes_deny` | `read.deny` / `read.allow` / `write.deny` / `write.allow` dotted keys on the `[[mounts]]` row |

## Semantic contract

The full pedagogical treatment lives in the mount-policy concept docs:

- [`../../mount-policy/00-overview.md`](../../mount-policy/00-overview.md) —
  what mount path policy is and how the pieces fit together.
- [`../../mount-policy/01-runtime-semantics.md`](../../mount-policy/01-runtime-semantics.md) —
  the runtime evaluator's visibility, write-admission, protect, tag, and
  cascade semantics.
- [`../../mount-policy/02-config-surface.md`](../../mount-policy/02-config-surface.md) —
  the unified `[policy.mounts.read]`/`[policy.mounts.write]` TOML surface.
- [`../../mount-policy/03-hierarchy-and-precedence.md`](../../mount-policy/03-hierarchy-and-precedence.md) —
  scopes, authority order, freeze, and trust gates.
- [`../../mount-policy/04-compiler-and-wire.md`](../../mount-policy/04-compiler-and-wire.md) —
  the collect-and-compile pipeline and the version-1 wire program.
- [`../../mount-policy/05-cookbook.md`](../../mount-policy/05-cookbook.md) —
  worked recipes.
- [`../../mount-policy/06-testing.md`](../../mount-policy/06-testing.md) —
  how to verify policy behavior (explain/preview, smoke checks, caveats).

## Consequences

**Positive:**

- **One intent-shaped vocabulary** — operators declare read/write/final;
  the compiler alone owns wire vocabulary.
- **Wire format untouched** — the runtime contract and the version-1
  gating (fail-closed on unknown/unsupported version) are unchanged.
- **Explain/preview/compiler reuse the same semantics** as runtime — no
  parallel evaluation logic to drift (ADR 0029 property preserved).
- **Hard errors instead of silent aliases** — stale configs fail loudly at
  parse time with the offending field named.

**Negative / costs:**

- **The `protect` keyword is gone** — protection is now operator-only by
  construction; a repo/workload layer can no longer declare protection at
  all.
- **`write.allow` is inert at the pinned runtime `3bd051bf`** until the
  re-pin: configs compile and `explain` reports them, but the allow arm
  does not enforce yet.
- **The mirror-evaluator port is an open re-pin step** (handover §9o
  step 2) — until it lands, the mirror's `decide_write` matches the pinned
  runtime, not develop.
