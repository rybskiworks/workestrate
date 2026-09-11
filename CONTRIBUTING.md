# Contributing

Target `main` with a focused branch and PR. The old migration line was integrated
in PR #32; do not maintain a parallel CI or dependency-update channel there. Use a
conventional subject and link the issue or Bead where relevant.

Read [AGENTS.md](AGENTS.md), the [specification map](SPEC.md), relevant ADRs and
[build ownership](docs/nix-build.md) before changing configuration or runtime code.
Future architecture is not evidence that a capability is already implemented.

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
