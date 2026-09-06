## Summary

<!-- What does this PR change and why? Keep it short; link tickets/ADRs here. -->

-

## Test plan

<!-- How was this validated? List the commands you ran and their results.
     `just verify` is expected green before review. -->

- `just verify` ✅

## Agent conventions

- Agents must NOT self-apply the `nix-ci` label — the heavy e2e-nix CI leg is
  owner/maintainer-triggered only.
- Commit subjects use conventional commit prefixes (`feat:`, `fix:`, `chore:`,
  `docs:`, `ci:`, `build:`, ...). ADR numbers belong in commit bodies, never in
  subjects.
- Never commit with `--no-verify`. Hooks are check-only — fix findings properly
  instead of bypassing them.
- `just verify` must be green before requesting review.
