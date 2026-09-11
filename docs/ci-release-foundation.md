# CI and release foundation

Engineering base: `migration/tool-model`. This change does not promote the migration, change the default branch, update runtime/fork/tooling pins, publish a release, or enable automated merging.

## Required-check contract

The workflow now starts for every PR. `scripts/ci/select_checks.py` classifies changes internally, with unknown/missing/empty diffs selecting code checks. Ordinary docs-only changes can avoid compilation, while metadata, workflow lint and workflow security still run. Migration docs remain code-relevant.

The historical `required-gates (branch-protection anchor)` name remains, but it now aggregates all selected hard checks. `CI / required` is its stable new alias. A failed, cancelled, missing or unexpectedly skipped selected job fails the gate. The `nix-ci` label selects real Nix functional tests, and manual dispatch selects every tier, including the package build. Heavy tiers are not silently promoted to every-PR jobs while hosted-runner capacity is unproven.

The Nix installer and compiler versions are pinned independently of their Action source SHAs. The existing informational advisory policy remains informational; license, source and dependency-ban checks are hard requirements when selected. Actionlint is invoked directly, avoiding the previous wrapper's non-failing default reporting mode.

Only add GitHub required-check rules after these workflows have run and their exact emitted context/App identity is observed. A check name alone is not exclusive merge authority: use the separate human update-authority and PR rules described in the organization plan. A write-enabled agent could otherwise merge its own PR.

## Version authority and release boundaries

`control/agentctl/Cargo.toml` is authoritative. The Nix derivation reads its existing parsed manifest instead of repeating `0.1.0`. The metadata guard requires the corresponding local Cargo.lock entry to match. This change does not alter the actual version or any dependency lock.

Future release preparation must account for changes across the entire repository, including Nix and configuration. Do not configure a release calculator that sees only `control/agentctl` and misses Nix-only changes. Version updates, the local Cargo.lock record and changelog must be a tested transaction in the human-reviewed release PR.

Publication requires exact-source CI and runtime evidence, secretless build/test jobs, a separate narrow publisher, complete checksummed assets in a draft release, and publication only after verification. `doCheck = false`, ignored KVM tests, or tests skipped for missing msb/Nix inputs are not release evidence. Pre-promotion artifacts should use an explicitly approved prerelease channel; no stable release is inferred from the branch name or source version.

## Shared automation

The agent PR caller is opt-in through `RYBSKIWORKS_AGENT_PRS=true` and targets this integration branch, not legacy main. It references the immutable foundation implementation in `rybskiworks/.github`; review that PR before enabling this caller. It creates draft PRs only, with no automatic approval or merge. GITHUB_TOKEN-created PRs may require a human to approve the ensuing workflow runs. Do not solve that by placing a multi-repository App private key in an agent-writable repository.

Dependabot intentionally remains Actions-only, with `target-branch: migration/tool-model`. GitHub must see this configuration on the default branch as well, so a separate metadata-only bootstrap PR to main is required. Schedules/manual workflow registration on a non-default engineering branch require similar deliberate bootstrap; no legacy runtime code should be copied into the edge for this purpose.

## Local checks

```sh
python3 -m unittest discover -s scripts/ci/tests -v
python3 scripts/ci/check_release_metadata.py
```

Unit tests cover classification, every hard dependency result, permitted skips, failed optional jobs, missing dependency declarations, and release metadata consistency. They do not replace Rust, Nix or actual GitHub Actions validation.

See the [organization rollout plan](https://github.com/rybskiworks/.github/blob/agents/chatgpt/governance-foundation-20260911/docs/governance.md) for the complete policy, credential and release design. The existing CODEOWNERS and PR template remain unchanged.
