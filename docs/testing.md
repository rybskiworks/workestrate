# Testing Workestrate

Run the repository gate from a plain host shell:

```sh
just verify
```

This builds the pinned Nix checks and runs source guards, lockfile consistency
and store checks. It does not deploy workloads, migrate live state or require
real credentials. See [Nix build ownership](nix-build.md) for individual gates.
KVM-dependent ignores are not evidence of successful VM acceptance.

For focused Rust checks, use the pinned development environment:

```sh
just shell -c cargo test --manifest-path control/agentctl/Cargo.toml \
  microsandbox::broker::property_tests
```

## Property testing

The [property-testing reference](testing/property-based-testing/index.md) covers
generators, shrinking and models. Workestrate pins Proptest 1.11.0 as a development
dependency with only `std`; Cargo.lock owns the exact dependency graph. The Nix
unit check runs 256 cases per property, seed 20260910 and at most 4096 shrink
iterations. Focused local tests use Proptest's defaults; expand or replay them:

```sh
just shell -c env PROPTEST_CASES=4096 PROPTEST_RNG_SEED=1234 \
  cargo test --manifest-path control/agentctl/Cargo.toml \
  microsandbox::broker::property_tests
```

Current generated broker coverage checks:

- Every launch-token byte and the exact CID, including negative controls.
- Complete compiled credential records, independent per-instance endpoint
  selection and narrowing that cannot add authority.
- Allocation, revision, expiry and reopen sequences against an independent
  launch-state model, with a separate disk reader after every transition.

These are not complete session-user authorization, concurrent registry locking
or actual SSH/Git VM tests. Keep those acceptance gates explicit. Extend native
properties to newly implemented runtime contracts as they become executable.

Prefer bounded valid-by-construction generators over broad filtering. Define an
independent oracle; a serializer round trip or comparing two copies of the same
matcher is insufficient. Each case owns disposable state. Never change process
environment, live homes, providers or credentials inside generated cases.

Commit failing seeds under `proptest-regressions` and a readable minimized
fixture for meaningful defects. Test dependencies and regression files must be
included in the Nix source. Replay selected concrete cases through disposable
VMs separately; do not put VM launches inside an automatic shrinking loop.
Source mutation testing likewise needs an isolated checkout, a passing baseline
and an assertion failure; compilation failures and timeouts are not kills.

## VM acceptance

Generic orchestration/runtime tests belong with their source owner. Application
fixtures, provider contracts and personal fleet compositions belong in fleet or
workload repositories. Use synthetic SSH keys and disposable Git upstreams.
Record exact runtime/image revisions, positive controls, observed refusals and
cleanup independently. Unit checks passing do not establish transparent SSH
custody or the nested development loop.
