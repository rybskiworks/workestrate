# ADR 0036: Nested virtualization for microVM guests

**Status:** Accepted (Phase 1: VMM flag + workestrate semantics; Phase 2 firmware pending)
**Date:** 2026-09-04

> **2026-09-06 amendment — D1 REVERSED (env-gated fork).** The always-on port (fork b2c672c8, upstream #823 shape: `nested_virt(true)` hardcoded) contradicted the D2/D3 semantics: the host-KVM checks were cosmetic at the CPU level because every guest — `nested=off` included — got the nested CPU capability regardless. D1 is therefore reversed: the VMM flag is now env-gated behind `MSB_NESTED_VIRT` (default OFF; only the exact value `1` enables it) in the fork at b1424618 (`crates/runtime/lib/vm.rs` `build_vm()` reads `.nested_virt(nested_virt_enabled())`), and agentctl a7231be drives that var from the `nested_up_decision` in the pre-create gate (`check_nested_up_gate` now returns `Result<bool>`; `up` sets `MSB_NESTED_VIRT=1` exactly when the decision resolves nested-ON — `require` passing its probes, or `prefer` on a host that affirmatively offers nested — and REMOVES the var otherwise; refusals unchanged). The §8 "Always-on CPU … documented limitation, D1" risk entry is CLOSED effective with the pin bump to b1424618 — pending the host nix relock; the live flake pin is still b2c672c8 (always-on) until then, so the limitation still describes the currently pinned build. The historical sections below are left intact as history.
**References:** `control/agentctl/src/config/types.rs:1304-1387` (`NestedMode` / `VirtualizationConfig` / `VirtualizationPolicyFragment`); `control/agentctl/src/microsandbox/nested.rs` (probe + `resolve_virtualization` + `nested_up_decision`); `control/agentctl/src/microsandbox/plan.rs:93-102` (`VirtualizationPlan`); `control/agentctl/src/microsandbox/workload/config.rs:430-494` (`plan_virtualization_for`); `control/agentctl/src/microsandbox/runtime/run.rs:873-932,1322-1324` (pre-create gate + `MSB_NESTED_VIRT` env-flag application); `control/agentctl/src/commands/doctor.rs:103-158` (`doctor_check_nested_virt`); `control/agentctl/src/main.rs` (`Commands::Msb`); `control/agentctl/src/merge.rs:413-452` (`VirtualizationLadder`); `control/agentctl/src/config/loading.rs:999-1046` (ladder collect); `control/agentctl/src/microsandbox/provenance.rs:58-61` (hash exclusion); `scripts/host-check.sh:37-54`; `scripts/kvm-tests.sh:139-168`

## 1. Title / Status / Date

- **Title:** Nested virtualization for microVM guests (fork flag + workestrate config semantics + checks + CLI)
- **Status:** Accepted (Phase 1). No "nested works" claim until Phase 2 (firmware).
- **Date:** 2026-09-04
- **Amends:** None (additive). Related: upstream superradcompany/microsandbox#823 (commit cd58504, draft-open), companion libkrunfw#7 (draft-open).

## 2. Context

Upstream's nested-virt change is one line in `crates/runtime/lib/vm.rs` `build_vm()` (Linux x86_64 only): `m.split_irqchip(true)` becomes `m.nested_virt(true).split_irqchip(true)` — hardcoded ON, no flags/CLI/SDK/DB/version. The fork ports exactly that line and keeps version `0.6.16` (upstream had no bump either). `msb_krun 0.1.32` already carries `nested_virt(bool)` (present since 0.1.13), so no dependency bump. The sole libkrunfw tag is `v5.2.1`, which still ships `# CONFIG_KVM is not set` — the guest kernel cannot yet expose `/dev/kvm`, so the Track 1 VMM flag is INERT until the Phase 2 firmware rebuild lands. All refusal UX therefore lives in workestrate's gate (no fork-side error signal exists); host probe + pin bookkeeping is the only detection shape.

- **D1 — Accept always-on fork + honest docs.** **(SUPERSEDED — reversed by the 2026-09-06 amendment above: the always-on flag made the host-KVM checks cosmetic at the CPU level; the flag is now env-gated default-off in fork b1424618, driven by `nested_up_decision`. Original text retained for history.)** No fork-side flag (a permanent rebase tax on the exact file upstream touches, for marginal benefit while the firmware lacks `CONFIG_KVM`). Honesty rule, stated in errors/doctor/SPEC: `nested=off` does NOT remove CPU capability — it withholds the device promise. Backlog V2 gated-fork if the threat model ever needs true CPU gating.

## 3. Semantics (D2)

Flat per-workload `[workloads.<name>.virtualization]` with a single tri-state `nested = "require" | "prefer" | "off"` (default `"off"`; absent table — or absent key — means off: current behavior, no device expectation, no checks, no gate beyond an explicit `nested≠off` plus the home-final seal).

```toml
[workloads.kvm-job.virtualization]
nested = "require"   # "require" | "prefer" | "off"; default "off"; absent table = off
# operator seal at home registry (<home>/config.toml) — optional, restrict-only:
[policy.virtualization]
allow_nested = true
final = false
```

```rust
enum NestedMode { Off, Prefer, Require }                       // kebab-case "off"|"prefer"|"require"
struct VirtualizationConfig { nested: Option<NestedMode> }     // None = omitted → Off
struct VirtualizationPolicyFragment { allow_nested: Option<bool>, final: bool }
```

- `nested=require`: guest must have `/dev/kvm` + VT-x/AMD-V together (one unit; the VMM yields both as one); absent/unreadable → fail-closed at `up` (§4).
- `nested=prefer`: use if the host offers, else degrade without the device (warn/note at `plan`, machine-readable per D7, never refuse at `up`).
- `nested=off`/absent: current behavior, silent.
- `require_device` was DROPPED as redundant — `nested="require"` already demands the whole unit, and the old device-without-nested hole (`require_device=true + nested=off`) is closed BY DELETION, not by justification. No backend namespacing (YAGNI: KVM is the only backend on this Linux-microVM arch; non-x86_64 is out of scope; a second backend gets its own plan amendment then). No generic `on_conflict` field — the tri-state IS the conflict policy (`require` = fail, `prefer` = warn + degrade, `off` = ignore; a second knob could contradict the first and collides with `PolicyConfig.on_conflict`). No entitlement gate (mechanism removed 2026-09-04, pre-release): an explicit `nested≠off` stands alone, reviewable in the capsule.
- Ask-vs-grant policy fit (no new idiom): the workload ASKS (capability demand with strength require|prefer|off, like firewall allow-rules) while home GRANTS/restricts (`allow_nested` + `final`, like the egress/ingress ladder's home deny/seal and the secrets-policy `final`). Absent home fragment = no restriction. Explicit `allow_nested=false` (+`final=true`) = fleet-wide ban. `final` seals, never enables: frozen at `allow_nested=false` blocks lower rungs from enabling; `true` grants nothing by itself — the workload still needs an explicit `nested≠off`.
- Arch: non-Linux/x86_64 + `nested=require` fails closed with an ADR-citing error; `plan` stays portable-warn. Host KVM (msb itself) is already gated and unchanged; guest-nested is strictly narrower (cannot exist without host KVM).

## 4. Checks per layer + exact error shapes (D3)

- `validate-config`: static coherence only (closed vocab at parse via serde enum + `deny_unknown_fields`; cross-rung freeze is plan-time, NOT a validate error). No host I/O.
- `plan`: resolve + provenance (`rung`/`origin`/`final`/`frozen_out`/`frozen_by`) + advisory host probe; warn, never fail (stays portable). `require` + host lacks → warn line naming provenance ("plan continues; up will refuse"); `prefer` + lacks → note; `off` → silent.
- `up`: fail-closed pre-create refusal (after `ensure_images` + dep auto-start, before `SandboxBuilder::create`; no partial sandbox left behind). `prefer` never refuses.
- `doctor`: inventory only — `dev_kvm` kept verbatim (never renamed; external tooling greps for it); new `nested_virt` check reads `/dev/kvm` + `vmx|svm` + `kvm_intel`/`kvm_amd` `nested` param → OK (kvm + vmx/svm + nested=Y) | WARN (kvm ok, nested N/unknown) | FAIL (no `/dev/kvm`), each with remediation, plus the fork pin-rev note (live pin rev b2c672c8 carries the mount-policy stack plus the nested_virt VMM flag; Phase 1 stays inert — firmware-gated (libkrunfw CONFIG_KVM, Phase 2)).

Exact `up`-refusal shapes (single site: `nested_up_decision`; the negative exact-error test pins these verbatim — `{workload}` / `{nested}` / `{sealed_by}` / `{uid}` / `{OS} {ARCH}` interpolate):

- Home-final seal (both `require` and `prefer`; `off` never refuses):
  `error: workload '{workload}' requests nested virtualization (virtualization.nested="{nested}") but the home [policy.virtualization] seal forbids it (frozen by '{sealed_by}': allow_nested=false, final=true) — see ADR 0036 §4 (final seals, never enables)`
  `hint: ask the home operator to relax the seal, or set nested="off" to opt out`
- Unsupported arch (`require` only):
  `error: workload '{workload}' requires KVM (virtualization.nested="require") but nested virtualization is only supported on Linux x86_64 (this host: {OS} {ARCH}) — see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)`
  `hint: run on a Linux x86_64 host with virtualization enabled in BIOS; or nested="prefer" to degrade, or nested="off" to opt out`
- `/dev/kvm` missing (`require` only):
  `error: workload '{workload}' requires KVM (virtualization.nested="require") but host /dev/kvm is not found — see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)`
  `hint: enable virtualization in BIOS + sudo modprobe kvm(_intel|_amd) + sudo usermod -aG kvm $USER (re-login); or nested="prefer" to degrade, or nested="off" to opt out`
- `/dev/kvm` present but unreadable (`require` only; euid parsed from `/proc/self/status`, no libc — `unsafe_code` is forbidden workspace-wide):
  `error: workload '{workload}' requires KVM (virtualization.nested="require") but host /dev/kvm exists but not accessible by uid={uid} — see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)`
  `hint: enable virtualization in BIOS + sudo modprobe kvm(_intel|_amd) + sudo usermod -aG kvm $USER (re-login); or nested="prefer" to degrade, or nested="off" to opt out`
- CPU nested parameter affirmatively disabled (`require` only; unknown `None` is a plan note / doctor WARN, never a refusal):
  `error: workload '{workload}' requires KVM (virtualization.nested="require") but host CPU nested virtualization is disabled (kvm_intel/kvm_amd nested parameter is N) — see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)`
  `hint: enable nested virtualization (sudo modprobe -r kvm_intel && sudo modprobe kvm_intel nested=1, or the kvm_amd equivalent) + sudo modprobe kvm(_intel|_amd); or nested="prefer" to degrade, or nested="off" to opt out`

## 5. CLI (D4)

Default form (b) with locked `--`: `workestrate msb -- <args>` forwards verbatim to `MSB_PATH` (else `msb` on PATH), mirroring the `Run` precedent (`trailing_var_arg + allow_hyphen_values`). A per-subcommand `external_subcommand` spike (which would fire only after `msb` matches, so no verb-first/legacy-shim conflict, giving `workestrate msb ps` without `--`) was time-boxed and NOT adopted: help/JSON/completion cleanliness is unverifiable without a toolchain, so the fallback locks (b). Top-level hijack stays rejected (would swallow typos of workestrate's own verbs). `msb` joins the `--json` pre-scan stop-words so payload `--json` never enables JSON mode. Completions are static-only (`workestrate completions` covers wrapper flags, never inner msb args — stated in help). Follow-up, not this ADR: port upstream `completion` into the fork, then delegate dynamic completion.

## 6. Version, firmware, degrade record, seal site (D5-D8)

- **D5 — Version stays `0.6.16`** (upstream had no bump either). Signal via fork pin rev + lock-guard + doctor/versions inventory; no rebase-confusing bump for a change inert until firmware lands.
- **D6 — Firmware two-phase.** Phase 1 ships the VMM flag on the nested-port rev b2c672c8 (feat/nested-virt-port pushed; pin advanced from 78fb3ed1 to b2c672c8, which carries the mount-policy stack plus the VMM flag) + full workestrate semantics with degraded strings (`require` fails closed naming the missing firmware pin; `prefer` warns). Phase 2 (firmware rebuild with `CONFIG_KVM` + provenance + pin + KVM-host positive test incl. `KVM_GET_API_VERSION=12`) gates any "nested works" claim.
- **D7 — `prefer` degrade is machine-readable:** degraded-nested lands in the plan JSON (`SandboxPlan.virtualization`: effective ask + `degraded`/`frozen_out`/`frozen_by`/`origin`; off stays silent so legacy plan JSON is byte-identical) and the instance record, never stderr-only. Config-hash note: `virtualization` is EXCLUDED from `config_hash_of_plan` like `instance_policy` (verified in `control/agentctl/src/microsandbox/provenance.rs`) — orchestration policy, not a sandbox build input; `up` re-resolves at gate time, so no record write is needed.
- **D8 — Seal-enforcement site:** home `PolicyConfig.virtualization` fragment + `merge.rs`/`loading.rs` enforcement (collect, never merge — same idiom as the policy fragments). The frozen path mirrors the egress `final`: validate passes (per-rung legal), plan reports `frozen_out`, up refuses.

## 7. Test plan (Phase 1)

Unit (no KVM): kvm present/absent/unreadable × off/prefer/require → validate outcomes + plan warn/degrade + `up`-decision pure function (mocked probe); home-final frozen provenance; tri-state transitions; unknown-variant/typo fixtures; seal-fragment merge tests. Schema/golden: drift + subschema + sync + scaffold + golden parity (reference keeps its three golden workloads byte-identical; the nested example capsule is additive). KVM-host integration (opt-in e2e, `NESTED_GUEST_TEST=1`, needs `vmx|svm` + nested=Y + readable `/dev/kvm`, else SKIP): Phase 1 asserts the negative exact-error shape only; the Phase 2 positive (guest `/dev/kvm` + vmx + `KVM_GET_API_VERSION=12`) plugs into the same skip gate.

## 8. Risks

Always-on CPU for all guests incl. `nested=off` (documented limitation, D1) — **CLOSED 2026-09-06 (D1 reversed, see amendment): the env-gated flag (default OFF) removes the CPU capability once the pin bumps to b1424618; the entry still applies to the live pin b2c672c8 until the host relock.** `prefer` warn-fatigue masking real misconfig (distinct warn vs note severities + doctor summary mitigate). Atomic install ≠ atomic migration: first new-binary run mutates shared `msb.db` — quiesce + snapshot pre-bump, never auto-migrate unattended. Crash/partial-sandbox discipline at `up`-refusal (no leftovers — asserted in tests).
