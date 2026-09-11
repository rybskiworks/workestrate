# Repository audit: 2026-09-11

## Scope and baseline

Repository structure, documentation, pin ownership, CI and observable GitHub
governance. This is not an exhaustive Rust security review or fresh fleet validation.

Workestrate main: `8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f`.
Former migration tip: `09e63162e856541f59fe011c24864bc3845a6ebc`.
Related nix-tooling main: `1120aa22cddf4a9a3424f38aadbebadd8a963c4b`.

[PR #32](https://github.com/rybskiworks/workestrate/pull/32) merged at 13:24:24 UTC
on September 11. The comparison showed migration zero commits ahead, one behind
main, and no divergent file delta. Consolidate to main rather than maintain two
copies of an integration pipeline.

## Findings and disposition

| Priority | Finding | Disposition |
| :--- | :--- | :--- |
| High | Readable repository/inherited rulesets returned empty; branch listing reported main/migration unprotected. Administration-only legacy reads were denied. | Disabled ruleset proposals and staged verification. No claim that all administration settings were inspected. |
| High | Dependabot and agent PR automation still targeted the integrated migration branch. | Default/main routing, main-only integration CI and merge-group coverage. Historical refs retained. |
| High | CODEOWNERS used a bare organization handle. | Route to verified administrator @georgrybski; host-side review enforcement is separate. |
| High | Same-repository agent pushes can cross the CI trust boundary if agents can edit push workflows. Secret/admin settings were not visible. | Document central actor/event execution policies, fork/protected-environment alternatives and negative permission tests. |
| High | SPEC.md mixed ai-workbench naming, personal-secret catalogs and outdated nested-virtualization guarantees. | Replace it with a maintained specification map and immutable historical reference; update migration index. |
| Medium | CI independently hardcoded Rust 1.97.1 and checked only a major/minor flake comment. | Derive the exact release from locked Fenix metadata; validate supplier identity/follows and installed compiler. |
| Medium | Nix-sensitive PRs needed a label to select functional checks. | Select Nix/config/workflow paths automatically; fail closed on unknown diffs; cover merge groups. Heavy package builds remain manual. |
| Medium | Repeated curl-to-tar vendoring and broad Rust cache-save behavior. | Pinned checkout of validated runtime source; trusted main-push cache writers. |
| Medium | Root handoffs held retired/local-machine instructions. | Remove NEXT-SESSION.md, NEXT-SESSION-fleet-tool-refactor.md, STATUS.md and redundant .agents/skills/.gitkeep; add hygiene tests. |
| Medium | Advisory findings are informational. | Preserve that policy explicitly; propose reviewed expiring exceptions before a blocking gate. |
| Low | README mixed positioning and detailed operator setup. | New landing page, verified CLI examples, separate setup/secrets guide and clear evidence limits. |

## Pins and preserved source

The immutable tooling pin already existed:
`46e62f450396ea16aa884568d1d0a9591bfb6299`. This change preserves it and its NAR
hash, instead of fabricating a lock update or consuming an unmerged supplier PR.
The corresponding Fenix revision is `fa09e6473a0dfd673e6cb9a37741aec513b4bb2a`;
its stable Linux manifest selects Rust 1.97.1. These are dated audit observations,
not another editable compiler-version definition.

Runtime source remains `8ae14c22963c0680b231f61280f43db364693a5c`. Application Rust
sources, Cargo.lock, flake.lock, operator state and dependency pins are unchanged.
ADRs, schemas, fixtures, licenses, source patches, agent skills and intentional
Beads exports remain source, not generic build artifacts. No tracker/database is
initialized or migrated and no branch history is rewritten.

## Validation and limits

The offline Python CI-contract suite passed 32 tests during preparation, including
real temporary-Git rename controls, missing/cancelled gate failures, exact compiler
patch resolution, follows cycles, wrong suppliers, mutable pins, integrity metadata,
network failure and artifact classification.

The execution environment did not provide Nix, Cargo, actionlint, zizmor or KVM,
and its container could not clone GitHub. Reads/writes used the GitHub connection;
Python tests used locally reconstructed files. No whole-application build, actual
Nix evaluation, linter execution or runtime isolation test is claimed. Review the
PR's hosted results before merging. Initial main check-run enumeration was empty;
that is not proof workflows are disabled and not evidence of passing CI.

## Next priorities

Activate actual protections after validating contexts and a viable reviewer model.
Establish independent CI/security review, identity-specific push/execution controls,
advisory policy and ephemeral trusted KVM evidence. Then promote a tested supplier
revision in a separate lockfile PR, test public consumers, and add provenance-bearing
release automation. Generate or check CLI examples/docs against executable help so
stale normative claims cannot silently return.

See [governance](github-governance.md) for the detailed, staged host-side proposals.

## Evidence

- [Baseline workflow](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/.github/workflows/ci.yml)
- [Baseline Dependabot](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/.github/dependabot.yml)
- [Baseline CODEOWNERS](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/.github/CODEOWNERS)
- [Input ownership](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/flake.nix)
- [Pinned Fenix manifest](https://github.com/nix-community/fenix/blob/fa09e6473a0dfd673e6cb9a37741aec513b4bb2a/data/stable.json)
- [Current runtime caveats](https://github.com/rybskiworks/workestrate/blob/8536e2e0c1e6550edd9a2779f16ef9f7cc51d08f/AGENTS.md)
