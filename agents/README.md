# Agents

Each agent has a directory under `agents/<name>/` with two parts:

- `repo/` — upstream source code (gitignored). Populated automatically by `nix develop` via flake inputs, or clone your own locally.
- `config/` — workestrator-specific configuration (tracked in git).
- `build/` — vendored build output (gitignored). For Odysseus this includes `.deps/` (cp312 wheels installed by the `nix develop` shell via Nix `python3.12`); the microVM imports them via `PYTHONPATH=/app/.deps`.

## Structure

```
agents/<name>/
├── repo/       ← gitignored (flake symlink or local clone)
├── config/     ← tracked (workestrator adaptation)
└── .gitkeep
```

## Agents

| Agent | Flake input (default fork) | Upstream | Config |
|---|---|---|---|
| Pi | `github:georgrybski/pi` | `github:earendil-works/pi` | — |
| Odysseus | `github:georgrybski/odysseus` | `github:pewdiepie-archdaemon/odysseus` | `config/settings.json` |
| OpenCode | `github:georgrybski/opencode` | `github:anomalyco/opencode` | `config/opencode.jsonc` |
| T3MP3ST | `github:georgrybski/T3MP3ST` | `github:elder-plinius/T3MP3ST` | — |

## Overriding repos

Use Nix's `--override-input` to swap any agent repo:

```bash
# Use official upstreams
nix develop --override-input pi github:earendil-works/pi \
            --override-input odysseus github:pewdiepie-archdaemon/odysseus \
            --override-input opencode github:anomalyco/opencode \
            --override-input tempest github:elder-plinius/T3MP3ST

# Use your own fork
nix develop --override-input opencode github:myorg/opencode
```

Launch profiles live under `profiles/agents/`.
