# 23 — Microsandbox fork nix flake packaging: encapsulated source-build consumption

> **STATUS: DESIGN / DEFERRED (authored 2026-08-03; implementation deferred until workestrate+passthrough usage stabilizes on host)**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [22-dynamic-mount-masking-policy.md](22-dynamic-mount-masking-policy.md) ·
> [09-microsandbox-agentd-offline-build.md](09-microsandbox-agentd-offline-build.md) ·
> [../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md) ·
> [../../migration/50-decisions/0028-policy-scopes-collect-and-compile.md](../../migration/50-decisions/0028-policy-scopes-collect-and-compile.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | This design document, links, indexing, and documentation-only checks can be validated in this container. |
| `HOST-NIX` | The deferred fork flake evaluation and source build require nix on the user's host. |
| `HOST-KVM` | Deferred runtime enforcement and passthrough behavior require KVM on the user's host. |

This is a docs-only design record. No flake is created by this spec; the flake
build would be `HOST-NIX`, and runtime use of the resulting binary would be
`HOST-KVM`.

---

## Summary

Spec 22's dynamic mount masking requires the fork's `PassthroughFs` enforcement
in both the `msb` binary and the Cargo SDK, but the current release-binary plus
crates.io consumption model cannot carry those fork changes together. The
deferred design is for the microsandbox fork to publish a nix flake output built
from its Rust workspace, while workestrate consumes both the binary and SDK
source from one pinned input revision. The fork would encapsulate agentd and the
libkrunfw wrinkle, preserve a reversible input boundary, and leave no flake code
in this commit.

Locked design points:

- the fork's own `flake.nix` is recommended over consumer-side packaging;
- one flake-input revision pins the runtime binary and SDK source in lockstep;
- the input may later move back to upstream after the relevant PRs merge;
- `MSB_AGENTD_PATH` and interim release-tarball `libkrunfw.so` lifting remain
  proven migration mitigations;
- this spec supersedes spec 09 option 3 for the masking use case, not for every
  possible microsandbox version bump;
- ADR 0029 is reserved/unused; no ADR is warranted while this design is
  deferred.

## 1. Problem

Spec 22 ([22-dynamic-mount-masking-policy.md](22-dynamic-mount-masking-policy.md))
transmits a keyed mount-spec token such as `policy=<path>` to `msb` and requires
runtime `PassthroughFs` enforcement. The released 0.5.6 binary rejects unknown
keyed mount-spec tokens, while the desired enforcement exists on the fork branch
`feat/passthrough-mount-path-policy`. Therefore the masking design is blocked by
consumption, not by the policy compiler or its host-side transmission model.

## 2. Background

The fork at `github.com/georgrybski/microsandbox`, branch
`feat/passthrough-mount-path-policy`, HEAD `482f9957`, contains workspace members
for `crates/cli` (`msb`), `crates/filesystem` (`PassthroughFs`), `crates/agentd`,
`crates/runtime`, and `sdk/rust` (the Cargo SDK crate). It has no `flake.nix`
today. The pending upstream agentd-path fix is
`fix/filesystem-agentd-path-override` at `d9b4d12e`; it changes filesystem
`build.rs` to honor `MSB_AGENTD_PATH` for an offline prebuilt agentd.

## 3. Current consumption state

- `nix/packages/microsandbox.nix` uses `pkgs.stdenv.mkDerivation` and
  `fetchurl`s the released 0.5.6 `microsandbox-linux-x86_64.tar.gz` (sha256
  beginning `b550b1f5`) and prebuilt `agentd-x86_64` (sha256 beginning
  `ecb46b8c`). It auto-patchelf's the binary, installs `msb` to `$out/bin/msb`,
  lifts `libkrunfw.so.5.2.1` plus `libkrunfw.so.5`/`libkrunfw.so` symlinks into
  `$out/lib`, and installs agentd at `$out/libexec/agentd`. This is binary
  repackaging, not a source build.
- `nix/packages/microsandbox-filesystem-patched.nix` fetches the
  `microsandbox-filesystem` 0.5.6 crate with hash
  `sha256-Y2jZNhV1OCCs30JwtYy+cIdLHUMThOiJMjI6JCKj3hU=` and applies
  `./microsandbox-filesystem-agentd.patch`.
- `nix/packages/agentctl.nix:59-96` stages a temporary `$MSB_HOME`; line 92
  sets `MSB_AGENTD_PATH=${microsandbox}/libexec/agentd`, proving the agentd
  injection mitigation for the patched build.
- `control/agentctl/Cargo.toml:18` pins
  `microsandbox = { version = "=0.5.6", features = ["net"] }` from crates.io.

## 4. The lockstep gap

The release tarball consumed by `microsandbox.nix` cannot contain the fork's
runtime changes. Conversely, the crates.io `=0.5.6` SDK cannot describe or
transmit a fork-only runtime contract. Updating only one surface would produce
an incompatible pair: a binary that does not understand policy tokens, or an
SDK that can request behavior the binary does not implement. A viable model must
carry the CLI/runtime and SDK source from one revision.

## 5. Proposed design: fork-as-flake

The fork should publish a nix flake output for `msb`, built from its workspace
and including the compatible agentd/runtime/filesystem pieces. Workestrate would
consume that output as a flake input, for example:

```toml
# spec-test: skip
inputs.microsandbox-fork.url = "github:georgrybski/microsandbox/<rev>"
```

Local development may use a `git+file://` input. The exact output names,
systems, lock-file shape, and whether the SDK is consumed directly or through a
workspace path remain implementation questions, not decisions made here.

## 6. Encapsulation: the libkrunfw wrinkle

`libkrunfw.so.5.2.1` is currently lifted from the release tarball in
`microsandbox.nix`; a source build of the fork must provide it either by
building libkrunfw or by retaining that proven release-tarball lift. With the
fork-owned flake, this complexity is solved once by the fork and presented as a
stable output. It does not leak into workestrate's flake or couple the consumer
to the fork's internal workspace packaging. The interim tarball lift remains an
explicit fallback until a source-built libkrunfw path is proven.

## 7. Lockstep contract (one rev)

The flake input revision is the compatibility boundary: it pins the `msb`
binary and the SDK crate source to the same fork revision. A workestrate update
must update the input lock and the Cargo dependency together, never independently.
This makes the passthrough mount-spec grammar, `PassthroughFs` implementation,
agentd path behavior, and SDK model one reviewable compatibility unit.

## 8. Reversibility

The fork input is temporary consumption infrastructure, not permanent fork
lock-in. After passthrough support and the agentd-path fix are upstream and
released, changing the flake input back to upstream consumption returns
workestrate to the upstream model. The Cargo dependency can likewise return to
the released crates.io pin once the upstream release contains the needed API.
The workestrate policy design remains independent of that ref switch.

## 9. Migration path (binary-repack → source-build)

Migration is staged rather than a simultaneous rewrite:

1. retain the current 0.5.6 binary repack and patched filesystem as the known
   interim state;
2. add and validate the fork-owned flake output on a host;
3. carry forward `MSB_AGENTD_PATH` from `nix/packages/agentctl.nix:92` so the
   offline build can inject the staged agentd;
4. carry forward release-tarball `libkrunfw.so` lifting from
   `nix/packages/microsandbox.nix` until source-building libkrunfw is reliable;
5. switch the binary input and SDK dependency together, then exercise spec 22's
   runtime transmission and enforcement on `HOST-KVM`.

## 10. Cargo dep switch mechanics (0.5.6 → fork git dep; 0.6.8 API surface)

The SDK dependency changes from the crates.io exact pin to a git/fork dependency
at the same revision as the flake input. A possible Cargo shape is illustrative
only:

```toml
# spec-test: skip
[dependencies.microsandbox]
git = "https://github.com/georgrybski/microsandbox"
rev = "<same-rev-as-flake-input>"
features = ["net"]
```

If transitive crates require it, a `[patch.crates-io]` entry may be used instead
of changing the direct declaration; the final choice belongs to implementation.
The investigation found the `agentctl` BUILDER surface used by workestrate is
signature-stable across 0.5.6→0.6.8. Risk concentrates in `NetworkPolicy`, exec,
and error types. This is the same 0.6.x migration surface as spec 09 option 3,
but here the driver is the fork's passthrough feature, not a standalone version
bump. The yanked 0.6.5 must not be selected.

## 11. Where the packaging lives (fork flake vs workestrate nix/packages)

**(A) Fork repository's own `flake.nix` — RECOMMENDED.** The fork owns
libkrunfw, agentd, and Rust workspace build complexity; any consumer can reuse a
stable output; and the one-revision lockstep contract is visible at the package
boundary.

**(B) workestrate `nix/packages/` — possible but not preferred.** This follows
the status quo represented by `microsandbox.nix`, but leaks the fork's build
internals and libkrunfw wrinkle into the consumer flake and couples workestrate
to the fork's workspace layout.

Recommendation: the fork carries its own flake and workestrate consumes
`inputs.microsandbox-fork.url = "github:georgrybski/microsandbox/<rev>"` (or
`git+file://` for local development). This is consistent with the repository's
fork-carries-compat policy in ADR 0011's original Decision text, while remaining
distinct from the 2026-07-30 reversal of the fork-as-Cargo-dep carrier.

## 12. Risks

- Fork build reproducibility may be harder than binary repackaging, especially
  around libkrunfw and fixed-output dependencies.
- The SDK API migration may expose type-level changes outside the stable BUILDER
  signatures, particularly network policy, exec, and error handling.
- A mismatched flake lock and Cargo lock would violate the one-rev contract.
- Upstream PR/release timing may prolong the temporary fork boundary.
- Host Nix and KVM validation are unavailable to this docs-only landing.

## 13. Open questions

- Should the fork flake build libkrunfw from source immediately, or formalize the
  release-tarball `.so` as an interim fixed-output input?
- What exact flake output exposes `msb`, agentd, and SDK source for all required
  systems?
- Should workestrate use a direct git dependency or `[patch.crates-io]`?
- Which 0.6.8 API changes, if any, require adapters after the fork dependency is
  compiled against current agentctl code?
- What upstream release and PR state is sufficient to switch the input back?

## 14. Dependencies

Implementation depends on spec 22's policy/compiler surfaces and on the pending
`fix/filesystem-agentd-path-override` PR at `d9b4d12e`. It also depends on a
fork revision that contains passthrough mount-path policy. Spec 09 option 1,
the upstream agentd PR, remains independent. No ADR is created: ADR 0029 is
reserved but unused until implementation is approved and a durable decision is
needed.

## 15. When to implement (trigger)

Do not implement this spec merely because the design is written. Start when
workestrate's passthrough usage and the host runtime have stabilized enough to
run a source-build migration and `HOST-KVM` enforcement validation as one
bounded change. Approval should also identify the fork revision, upstream PR
state, and the chosen libkrunfw strategy.

## 16. Cross-references

- [22-dynamic-mount-masking-policy.md](22-dynamic-mount-masking-policy.md) is
  the consumer; this design unblocks its runtime enforcement.
- [09-microsandbox-agentd-offline-build.md](09-microsandbox-agentd-offline-build.md)
  is related but distinct; this spec supersedes its option 3 for masking.
- [ADR 0011](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md)
  is adjacent: flake packaging is not the reversed Cargo-dep carrier.
- [ADR 0028](../../migration/50-decisions/0028-policy-scopes-collect-and-compile.md)
  is adjacent because its v1 transmission depends on the nix-patched filesystem.
- ADR 0029 is reserved/unused; no ADR is warranted for this deferred design.
- Pending upstream PR: `fix/filesystem-agentd-path-override` at `d9b4d12e` on
  `github.com/georgrybski/microsandbox`.

## 17. Acceptance criteria

- [ ] This spec is present as `23-microsandbox-fork-nix-flake-packaging.md` and
      is indexed in `00-index.md`.
- [ ] The spec records the current binary repack, patched filesystem, agentd
      injection, and crates.io SDK state with file-level citations.
- [ ] The lockstep one-revision binary-plus-SDK contract is explicit.
- [ ] Fork-owned flake packaging is assessed against workestrate-side packaging
      and marked recommended without implementing either option.
- [ ] The libkrunfw wrinkle, migration mitigations, reversibility, API risk, and
      host validation boundaries are documented.
- [ ] Spec 22, spec 09, ADR 0011, ADR 0028, and the reserved/unused ADR 0029
      posture are cross-referenced.
- [ ] STATUS.md and NEXT-SESSION.md record the deferred design and no flake code
      is added.
- [ ] The four documentation files land in one commit with no unrelated files.

**Key decision:** defer implementation; recommend a fork-owned nix flake for
lockstep source-build consumption, with no flake code or ADR created now.
