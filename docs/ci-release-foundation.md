# CI and release foundation

Integration branch: `main`, following PR #32. This consolidation changes repository
automation, not live runtime state, dependency pins, releases or merge authority.

## Required-check contract

CI starts for every main-targeting PR, main push, requested merge group and manual
dispatch. There are no workflow-level path filters that could leave the required
aggregate absent. Selection happens in `scripts/ci/select_checks.py`.

Policy, metadata, workflow lint and workflow security run on every PR. Ordinary
docs-only changes may skip compilation; migration docs remain code-relevant.
Nix/config/workflow changes select the Nix functional tier without a label.
Unknown/missing diffs fail closed. Main code pushes and merge groups select code
and Nix functional checks. `nix-ci` opts other PRs in; manual dispatch also selects
the heavy package build. This is not every heavy/KVM test on every PR.

The historical `required-gates (branch-protection anchor)` remains for transition.
`CI / required` is the canonical aggregate. Failed, cancelled, missing or
unexpectedly skipped selected jobs fail it. Keep the job dependency set and the
tested gate contract synchronized. A merge-group trigger does not enable a queue.

## Exact compiler and source ownership

The compiler is derived from the locked Fenix manifest reached through the pinned
nix-tooling input. CI validates supplier identity and the follows graph, installs
the exact patch release and verifies rustc's reported version. Source checkouts
use the validated Microsandbox lock revision. Remote actions/reusable workflows
remain SHA-pinned, the Nix installer is version-pinned, code jobs are read-only,
and only main push jobs save the Rust cache.

License, dependency-ban and source checks remain blocking when selected.
Advisories remain informational under the existing policy; green CI is not a
zero-vulnerability claim. Actionlint and zizmor execute directly.

```sh
python3 -m unittest discover -s scripts/ci/tests -v
python3 scripts/ci/check_release_metadata.py
python3 scripts/ci/check_repository.py
python3 scripts/ci/toolchain.py check --role consumer
just verify
```

`toolchain.py resolve --role consumer` additionally fetches a fixed immutable
Fenix manifest over HTTPS with a bounded read. Network failure is fatal, not a
fallback to stable or the runner compiler. Nix verifies/realizes locked source
integrity for actual Nix builds. The Python suite is not a full flake evaluator.

Nix functional checks provision pinned msb/agentd and execute selected tests. The
manual heavy tier builds Workestrate and invokes its version command. Neither
implies that ignored, optional native or KVM isolation tests ran. Build/source
selection is separate from host provisioning, guest boot and state migration.

## Release qualification

`control/agentctl/Cargo.toml` remains authoritative. Nix reads that manifest;
the metadata guard checks the local Cargo.lock entry. No source version or lock
is bumped here. Release calculation must account for the complete repository,
including Nix/config changes outside the crate directory.

A release PR should update version, local lock metadata and changelog as a tested
transaction. Publication needs exact-source CI/runtime evidence, a separate narrow
publisher, complete checksummed assets and provenance in a draft, then explicit
verification. Ignored tests, `doCheck = false` and skipped runtime tests are not
evidence. `.github/release.yml` categorizes notes; it publishes nothing.

## Shared automation and administration

The agent draft-PR caller stays off unless `RYBSKIWORKS_AGENT_PRS=true`. It targets
main through the pinned organization implementation, grants PR-write only to its
metadata job, and never approves/merges. Dependabot remains Actions-only and uses
the default branch. The old default-branch bootstrap split is retired.

Required statuses, review and identity-specific push authority are host-side
controls. Observe actual check contexts and validate the reviewer model before
activation; see [governance](github-governance.md). A passing self-modifiable
workflow alone is not independent merge authority.

The two repositories carry the same small bootstrap verifier so neither executes
mutable remote code nor consumes the companion unmerged PR. It owns no version
pins. A tested supplier promotion can introduce a versioned verification interface
and remove the copy; do not fetch scripts from main just to deduplicate them.
