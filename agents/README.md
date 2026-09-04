# Agents

Each agent has a directory under `agents/<name>/` with two parts:

- `repo/` — upstream source code (gitignored). Populate it by cloning your own into `agents/<name>/repo`; the devshell's shellHook builds it into `build/` on `just shell` entry. Flake inputs pin default forks but do not populate `repo/`.
- `config/` — workestrate-specific configuration (tracked in git).
- `build/` — vendored build output (gitignored). For Odysseus this includes `.deps/` (cp312 wheels installed by the devshell (`just shell`) via Nix `python3.12`); the microVM imports them via `PYTHONPATH=/app/.deps`.

## Structure

```
agents/<name>/
├── repo/       ← gitignored (flake symlink or local clone)
├── config/     ← tracked (workestrate adaptation)
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

Use Nix's `--override-input` to swap any agent repo (extra args pass
through `just shell`; it wires the devenv-root override itself):

```bash
# Use official upstreams
just shell --override-input pi github:earendil-works/pi \
           --override-input odysseus github:pewdiepie-archdaemon/odysseus \
           --override-input opencode github:anomalyco/opencode \
           --override-input tempest github:elder-plinius/T3MP3ST

# Use your own fork
just shell --override-input opencode github:myorg/opencode
```

Launch profiles live under `profiles/agents/`.
