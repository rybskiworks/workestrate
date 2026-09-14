# Contributing

Target `main` with a focused branch and PR. The old migration line was integrated
in PR #32; do not maintain a parallel CI or dependency-update channel there. Use a
conventional subject and link the issue or Bead where relevant.

Read [AGENTS.md](AGENTS.md), the [specification map](SPEC.md), relevant ADRs and
[build ownership](docs/nix-build.md) before changing configuration or runtime code.
Future architecture is not evidence that a capability is already implemented.

## Contribution licensing

Unless explicitly stated otherwise and accepted by the maintainer, original
contributions intentionally submitted for inclusion are offered under Apache-2.0,
consistent with section 5 of [LICENSE](LICENSE). Contributors retain ownership of
their work; this is not a copyright assignment or a separate CLA. Submit only
material you have authority to license, including any required employer/client
permission. Do not identify an agent or GitHub organization as a substitute for
the actual rights holder.

For copied or adapted material, identify its source, revision, actual license,
required attribution and modifications. Preserve existing notices and separately
mark third-party exceptions. Do not describe imported material as your original
Apache-licensed contribution. Disclose relevant provenance for AI-assisted work
rather than assuming tool output clears third-party rights.

See [LICENSING.md](LICENSING.md) and [THIRD-PARTY.md](THIRD-PARTY.md). A DCO sign-off
can document submission authority but is not an assignment; this change does not
add a new mandatory sign-off or contributor-agreement gate retroactively.

## Validation

With Nix and just available, `just bootstrap` supplies pinned tools, `just shell`
opens the interactive environment and `just verify` is the local entrypoint.
Avoid ambient Cargo outside pinned recipes/the development shell. Nix's Git-backed
source includes tracked files; stage new source deliberately before checking.
Do not bypass hooks to hide a failed validation.

```sh
python3 -m unittest discover -s scripts/ci/tests -v
python3 scripts/ci/check_repository.py
python3 scripts/ci/toolchain.py check --role consumer
python3 -m unittest discover -s tests/licensing -v
```

Record exact commands, results and tested revisions. Separate lint, evaluation,
compilation, native integration and KVM evidence; list anything not run. The
Python contracts do not replace application or runtime tests.

## Source versus operator state

Keep schemas, fixtures, ADRs and intentional tracker exports. Do not commit local
handoffs, build results, caches, VM disks, logs, keys or decrypted configuration.
Do not initialize/migrate Beads or rewrite operator homes as incidental cleanup.
Use [private reporting guidance](SECURITY.md) for security findings.

Toolchain changes are reviewed supplier-pin and generated lockfile changes, not a
second Rust version in YAML. Preserve one-way follows. Document public-interface,
schema, security-policy and migration/rollback consequences. Source changes do not
authorize live runtime, database, signing or protection changes.

See [CI/release qualification](docs/ci-release-foundation.md) and
[governance](docs/github-governance.md) for the merge and release boundaries.
