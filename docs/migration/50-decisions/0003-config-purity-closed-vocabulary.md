# ADR 0003: Config purity + closed recipe vocabulary

**Status:** Accepted
**Date:** 2026-07-18

## Context

workestrate is a sandboxing tool where config defines security policy (egress
hosts, secret bindings, network deny rules). If config could contain executable
logic (arbitrary shell, nix expressions), then write access to the config repo
would be equivalent to arbitrary code execution on the host — defeating the
sandbox's purpose.

## Options considered

1. **Pure data config + closed recipe vocabulary in core** — config is
   declarative data only; all logic is named, versioned, reviewable recipes in
   core. Selected.
2. **Config with escape hatches (run: scripts, extraCommands)** — GitHub
   Actions / Ansible model. Rejected: `run:` scripts are too permissive for a
   security boundary.
3. **Config as nix expressions** — devenv model. Rejected: nix is code, not
   data; breaks purity.

## Decision

Config repo = declarative data + SOPS-encrypted secrets + static files ONLY.
All executable logic = named, versioned, reviewable recipes in core. This is
the Kustomize model (no-templating is a stated non-goal) combined with the
NixOS module system (bounded vocabulary of typed options) and the devcontainer
features model (install scripts owned by the feature, not by user config).

### Precedent (live-verified)

- **Kustomize**: no-templating/no-scripting is a stated **non-goal**
  (kubernetes-sigs/kustomize issue #2052, maintainer monopole: "template
  business… is a non-goal of kustomize").
  [Source: https://github.com/kubernetes-sigs/kustomize/issues/2052]
- **NixOS modules**: bounded vocabulary of typed options.
  [Source: https://nixos.org/manual/nixos/stable#sec-option-types]
- **devcontainer features**: declarative features, install scripts owned by
  feature not user config.
  [Source: https://devcontainers.github.io/implementors/features]

### Precision note

The Kustomize no-templating stance is confirmed from official maintainer
statements in issue #2052 (verbatim: "template business… is a non-goal of
kustomize"). The NixOS module system's bounded vocabulary is confirmed from
the official manual. The devcontainer features model is confirmed from the
official spec. These are facts from official sources, not inferences.

## Consequences

- Config cannot supply `extraCommands` (arbitrary shell), nix expressions, or
  inline build scripts.
- `baked_files` content is strings only; `features` are named from a closed
  core vocabulary.
- Escape hatch: new logic requires adding a named recipe to core (reviewed,
  versioned). This is the Terraform provider model.
- Vocabulary governance: entries reviewed like core code; periodic audits.

## Rejected why

Escape hatches (option 2) defeat the security boundary. Nix-in-config (option
3) breaks purity.
