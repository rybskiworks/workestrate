# Build and workload continuation

This is a September 2026 checkpoint, not a declaration that the refactor or
personal workload deployment is complete. Read the current native tracker and
remote branch heads before acting on these recorded results.

## Recover the task graph

The authoritative task graph is embedded Dolt at `.beads/embeddeddolt/wrk`.
Its history is published separately to the private Workestrate Git remote at
`refs/dolt/data`. `.beads/issues.jsonl` is the reviewable source snapshot, not
a complete database backup. An ordinary source push does not synchronize Dolt.

Use the pinned repository entry point and serialize access:

```sh
just beads version
just beads context --json
just beads ready --json
just beads list --all --json
```

For a fresh clone **without an existing database**, follow the inspected
bootstrap procedure in [the tracker guide](README.md). Never bootstrap, pull,
force-import or delete an existing database to resolve unpublished history.
Take a native backup and full export before integrating another writer. Keep
automatic push and automatic hook replacement disabled.

The reviewed Git JSONL snapshot preserves 159 issues, 159 blocking edges and
134 parent links. Native history has since advanced with additional workload,
Determinate, init, and fork-maintenance tasks. Query the recovered database for
current counts; an older Git export must not overwrite the newer native plan.
Issue notes retain earlier failures; later exact-input results supersede their
status without making unexecuted acceptance tests pass.

## What has actually passed

- Workestrate's documented `just verify` gate: eight pinned Nix checks,
  48 generic script fixtures and 1,708 Rust tests, with five existing ignores.
  The normal repaired-runtime adoption passed this full gate, followed by the
  unchanged external microVM denied-file regression and clean signed-source
  pre-push Rust check. PR #14 landed at
  `7cbb8ab28802e28e43b2d51a510bc6151d5334b0` on `migration/tool-model`.
- Microsandbox's read-only traversal admission repair: all 701 filesystem
  tests, coherent runtime/static-agent packages and the unchanged external
  guest regression. The full isolated default workspace suite passed 3,228
  tests with 139 existing ignores, including all 90 binary and 21 doctest
  targets. Standard formatting, Clippy, rustdoc and CLI-build hooks passed.
  PR #11 landed on its `develop` branch at `67807cd587ea50a4054659a2989dea5886b40e8a`.
- Real Codex app-server and Prime AgentSession captured and retrieved each
  other's exact native records through one ai-memory service and real LiteLLM.
  Both native session endings and normal cleanup passed. The model endpoint
  was deterministic, native providers were disabled and credentials synthetic.
  This is not four-microVM deployment or real-provider certification.
- That shared test uses ai-memory `a8b17db89c8bbd1e8ae8cd8183432cc2a87b62b4`.
  Its full Linux contributor gate passed 3,128 tests with ten existing ignores,
  followed by formatting and all-target Clippy. Its native package/service
  gates and actual two-session Prime check also passed.
- Published ai-memory `109e54579539842b7a26766129c7b583871e82fe` additionally
  repairs support-script reinstallation from immutable bundles. Its full Linux
  contributor gate passed 3,133 tests with ten existing ignores, formatting
  and all-target Clippy. The normal native package passed 192 core tests and
  service checks; the unchanged copied-executable reinstall/uninstall smoke
  and real two-session Prime capture/recall/cleanup both passed. These new
  results do not change the shared fleet test's a8 provenance.
- The personal fleet's native ai-memory capsule and pinned LiteLLM readiness
  profile landed on `main` at `c96be9005636e6b7bd6836a4484d4d5cccf4578a`.
  The native image/archive, immutable effective configuration, bearer/Host
  checks, 23 memory fixtures and 96 LiteLLM fixtures passed. This is not guest
  startup or a four-VM result; the effective-config test uses native SIGINT.
- Runtime-neutral shared image/NixOS constructors landed in nix-tooling at
  `5d4317ded007433951d33603057768119e818f61`. Seven normal checks, two opt-in
  image checks, 18 contract assertions and 11 archive-oracle fixtures passed.
  No guest engine, NixOS activation, daemon or cache deployment is implied.
- Operator configuration docs are on `main` at
  `3110361eb87500f3da7b0cb1522e1c8a8529c1a5`. No live configuration
  synchronization, state migration or credential-bearing rollout followed.

## Next work and repository ownership

| Area | Next concrete boundary |
| --- | --- |
| Workestrate runtime adoption | Normal immutable pin/lock/diagnostic/package provenance adoption and its unchanged guest regression are complete. Existing VMs still need explicit recreation; the personal fleet must separately adopt the new consumer pin. |
| Microsandbox Nix verification | Package a source-owned xattr-capable full-workspace gate, preferably using standard NixOS test infrastructure. The passing isolated host suite does not make the ordinary Nix syscall-filter failure disappear. Preserve all test targets, strictness and explicit capability prerequisites. |
| ai-memory source | The immutable support-script repair and Linux native packaging are validated at `109e54579539842b7a26766129c7b583871e82fe`. Keep the analogous legacy `setup-agent` copier as a separate repair, with its own immutable-source and platform tests. Per-file replacement is not whole-bundle atomicity or hostile ancestor-race confinement. |
| ai-memory contribution history | Do not merge the compatibility, native-packaging or Prime-adapter PRs into the fork. Preserve them for upstream contribution and consume exact published revisions. Windows execution and reconciliation with newer upstream changelog/marker/session changes still need their own reviewed tests; Actions remains disabled. |
| Personal fleet clients | First adopt the repaired Workestrate and shared-image pins, then wire the native installer into fresh Prime/Codex profiles and run the four independently packaged workloads through actual `depends_on`. Keep hook trust and tool approval explicit. Extend restart, refinement, identity, redaction, queue and MCP failure tests separately. |
| LiteLLM workload | The credential-scoped OpenCode session-header callback and nonstreaming wire tests are published but unactivated. Finish actual client identity emission, cache-enabled/streaming negatives, retry/fallback/redirect behavior and bounded error mapping before rollout. |
| Runtime policy and lifecycle | Enforce nested-off across launch/reuse/restart; fix write ordering, sensitive guest path normalization, lifecycle readiness/rollback/orphans and exact teardown. Preserve the read-only repair's admitted-handle/snapshot semantics. |
| Fleet regression suite | Extend direct and nested mount masking, differential/property/stateful cases, networking, ports, dependencies, storage and credentials. Keep workload-specific tests outside Workestrate; distinguish authored tests from final passing runtime gates. |
| Optional workload repositories | Generic source/content/runtime provenance exists, but pinned external workload imports and layout-aware scaffolding still need implementation. Capsule-owned flakes can advance independently of repository imports or state migration. |
| nix-tooling shared images | Adopt the merged package-set-aligned constructors without changing image identity. Select the supported Determinate distribution boundary, prove it under standard NixOS first, then actual Microsandbox activation and guest-owned sandboxed builds. Installing a CLI or using the current command-boot shim is not this daemon proof. |
| Explicit init and shutdown | Forward Microsandbox's existing typed init API through generic Workestrate configuration. Repair the source-confirmed mixed-libc shutdown signal mismatch: static-musl agentd derives signal 39 while glibc systemd treats it as reboot, not poweroff. Require actual receiver/init tests and clean owned teardown. |
| Builder and signed cache | Implement the existing builder/cache epic: isolated writable build store, separate reviewed publisher, read-only signed cache. Reuse shared image/daemon profiles and package versions. Start with independent local positive/negative cache tests; Cachix account/backend adoption is a separate decision. |
| Fork maintenance | Keep operational libkrun `krun`, with independent exact fast-forward mirrors for Superradcompany and original libkrun upstreams; one upstream mirror is sufficient for Microsandbox. Resolve organization-disabled Actions before automation, apply additive protections and runnable pinned Nix gates, and give public Microsandbox its own public-safe Beads project. Never copy the private program database into the public fork or auto-backmerge upstream changes. |
| Storage and SSH | Rehearse explicit roots, preview/no-write inspection, partial recovery and rollback. Broker boot/routing/ownership/timeout and synthetic SSH gates remain unfinished. Do not migrate live state or introduce real keys implicitly. |
| Later fleet rollout | Personal workloads first; then separately bind ai-memory, LiteLLM and Tempo into Duelbits. Preserve existing user customization and separate personal/company credentials and publication domains. |

The builder/cache is planned, not deployed. Cachix's client does not provide a
remote builder or a self-hosted cache server. Do not share a writable host Nix
store or daemon socket with workloads, or give general agents signing keys.
Ordinary remote compilation does not require nested KVM; image-build-plus-inner
VM acceptance is a separate opt-in gate.

## Verification and publication boundaries

Use `just verify`, not a blanket claim based on bare `nix flake check`:
evaluating devenv container outputs also requires its root-directory context.
Keep source/runtime provenance truthful. Revision-only check invalidation and
compiled Cargo dependency reuse have separate tracked improvements; a binary
cache alone does not fix either input-identity problem.

Publish normal signed feature commits and reviewed PRs, attach all PRs to the
existing Workestrate GitHub project, and verify the resulting tree before local
fast-forward synchronization. Preserve branch history; no force/reset or
failed-hook bypass is part of this workflow. Source publication, native tracker
publication, integration-branch merges and main release acceptance are separate.

Workestrate's full refactor remains on `migration/tool-model`, not `main`.
Its main promotion still requires the agreed runtime-policy and release
acceptance. Passing test fixtures and documentation merges do not authorize
production cutover or certify the complete deployment.
