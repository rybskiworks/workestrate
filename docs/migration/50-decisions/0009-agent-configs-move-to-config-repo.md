# ADR 0009: Agent configs move to config repo

**Status:** Accepted
**Date:** 2026-07-18

## Context

Agent config files (`agents/pi/config/models.json`,
`agents/odysseus/config/settings.json`,
`agents/opencode/config/opencode.jsonc`) are currently tracked in the core
repo. They are deployment-specific (they reference `host.microsandbox.internal`
and LiteLLM model tiers). The `config_path()` method (`workload.rs:92-95`)
hardcodes `agents/<name>/config/<filename>`.

## Options considered

1. **Keep agent configs in core repo** — they're "reference" configs. Rejected:
   they're deployment-specific, not reference; they belong with the user's
   config.
2. **Move to config repo; mount resolution becomes config-relative** — agent
   configs live in the config repo at `agents/<name>/config/`; mount paths
   are resolved relative to the active config repo. Selected.

## Decision

Agent config files move to the config repo. `config.reference/` carries
sanitized reference copies (for CI/fresh-install). Mount resolution becomes
config-relative: a mount path like `agents/pi/config/models.json` resolves to
`<active-config-repo>/agents/pi/config/models.json`.

## Consequences

- Agent configs are deployment-specific (user's LiteLLM endpoint, model tiers).
- `config.reference/agents/*/config/` provides sanitized reference copies.
- The `config_path()` method is replaced by config-relative mount path
  resolution in the Rust CLI.
- `seed_files` source paths are also config-relative.

## Rejected why

Keeping in core: they're deployment-specific, not reference.
