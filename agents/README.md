# Agent development directories

Existing checkouts may keep local agent sources and build artifacts under
`agents/<name>/`. These are development locations, not packages automatically
managed by the Workestrate shell:

- `repo/` — gitignored source checkout, populated explicitly by its owner.
- `config/` — any tracked Workestrate adaptation for that checkout.
- `build/` — gitignored output created by an explicit workload build recipe.

`just shell` does not clone these sources, install their dependencies, populate
Python wheels or build agents. It supplies the Workestrate development tools
and immutable Microsandbox SDK inputs. `just bootstrap` supplies tools without
requiring the application/runtime packages. Both leave existing real source
and build directories alone.

The tool flake no longer declares `pi`, `odysseus`, `opencode` or `tempest`
source inputs. Passing those names to `just shell --override-input` is therefore
not an agent-source selection mechanism. Agent source pins, runtime
configuration and application-specific build recipes belong to the workload's
own repository or fleet. Follow that owner's explicit build instructions.

See [workload authoring](../docs/workloads.md) for supported workload layouts
and source/artifact ownership, and [Nix builds](../docs/nix-build.md) for the
tool's build and shell contracts. An existing `agents/<name>/build` path may
still be referenced explicitly by a workload. These directories are not
automatically relocated or removed.
