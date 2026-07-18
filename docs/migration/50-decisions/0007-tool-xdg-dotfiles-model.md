# ADR 0007: Tool+XDG+dotfiles organization model

**Status:** Accepted
**Date:** 2026-07-18

## Context

The ai-workbench repo is today both a tool (workestrate CLI, nix flake) and a
workspace (personal config, secrets, runtime state). This coupling creates
problems: (a) homeless-registry recursion (config-repo references have no
tracked home if children are gitignored); (b) distribution friction (tool
coupled to workspace); (c) day-to-day overhead (must `cd` into workspace).

## Options considered

1. **Child config repos inside workspace (`config.d/<name>/`)** — gitignored
   children, like `agents/<name>/repo`. Rejected: homeless-registry recursion
   (config.lock.json/layers.toml untracked → second machine can't reconstruct).
2. **Sibling repos** — config repo as a sibling directory. Rejected by user:
   "not a sibling repo but children repo."
3. **Tool+XDG+dotfiles** — workestrate as a tool; registry in
   `~/.config/workestrate/config.toml` (user's dotfiles); config repos in
   `~/.local/share/workestrate/repos/<name>/`; state in
   `~/.local/state/workestrate/`. Selected.
4. **Everything-as-flake** — home-manager-style personal flake composing tool
   + config. Rejected for primary: nix-fluency barrier for team distribution.
   Kept as optional mode (config-repo-flake, Phase 2).

## Decision

workestrate-as-tool + XDG + dotfiles-registry model. The registry
(`~/.config/workestrate/config.toml`) IS the user's dotfiles — tracked in
their dotfiles repo. This is the kubeconfig model: `~/.kube/config` holds
cluster references; `~/.config/workestrate/config.toml` holds config-repo
references. The recursion terminates at the dotfiles repo, bootstrapped via
`workestrate init <dotfiles-url>` (chezmoi-init style).

### Precedent (live-verified)

- **kubectl**: contexts + KUBECONFIG multi-file merge.
  [Source: https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/]
- **git**: system < global < local < worktree precedence.
  [Source: https://git-scm.com/docs/git-config#FILES]
- **Claude Code**: managed > user > project > local layers.
  [Source: https://code.claude.com/docs/en/settings#how-scopes-interact]
- **mise**: `~/.config/mise/config.toml` (global) + `mise.toml` (project).
  [Source: https://mise.jdx.dev/configuration.html#configuration-hierarchy]
- **chezmoi**: `chezmoi init <repo>` bootstraps; `~/.config/chezmoi/chezmoi.toml`
  holds machine config; source state at `~/.local/share/chezmoi/`.
  [Source: https://chezmoi.io/reference/commands/init/]

## Consequences

- Tool is decoupled from any workspace; runs from any project dir.
- Registry has a natural home (user's dotfiles repo).
- `workestrate init <url>` bootstraps a fresh machine.
- Devshell reads ONLY `config.reference/` (tool-dev); user config never enters
  nix eval (pure-eval invisibility by construction).
- `workspaces/`, `var/`, `agents/<name>/repo` move to XDG state/store.

## Rejected why

Child repos: homeless-registry recursion. Sibling repos: user rejected.
Everything-as-flake: nix-fluency barrier.
