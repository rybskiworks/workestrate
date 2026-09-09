# Nested Workestrate smoke

This standalone workload boots a local child VM using Workestrate inside an
outer Workestrate VM. Copy the complete directory, including `flake.lock`, to
use it independently; no sibling files or source checkout are required.

The image tooling follows the shared tooling pin. The guest and optional host
runner intentionally retain Workestrate commit
`fed92957ccf6a77990efd7abc6c7bf2b8b711f5c` and its own locked runtime closure,
which were validated together. Update that runtime input only with a new real
nested lifecycle test, not merely an image-build check.

The Workestrate source is private. Fetching its immutable SSH input requires
an authenticated GitHub SSH identity with repository access. No access token
belongs in the flake, lockfile, image, or Nix configuration committed here.

```sh
nix flake check --no-update-lock-file
nix build --no-update-lock-file .#workestrate-nested-smoke
nix build --no-update-lock-file .#workestrate
```

The image check inspects archive contents without starting a VM. The real test
requires accessible host KVM with nesting enabled, two outer CPUs and 2 GiB
RAM. Both operator `allow_nested = true` and workload `nested = "require"`
are explicit. The child uses one CPU and 256 MiB. Neither workload has host
mounts, credentials, external providers, ingress, or egress permission.

Run only through a fresh, short test `MSB_HOME`, a separate JSON
`MSB_CONFIG_PATH` containing `{}`, and isolated `HOME`, tool home, config and
state roots. Set `WORKESTRATE_CONFIG_DIR` to this complete directory,
`WORKESTRATE_REFERENCE_CONFIG=0`, and use `--no-project-config`. Host image
building needs Nix; no Nix daemon, host store, or host home is mounted into
the guest. `workload build outer` builds and imports the image; then start
`workload up outer --instance selfhost`. Stop only that instance with
`workload down outer --instance selfhost`, including after a failed test.

The guest uses fresh `/tmp/inner` roots and an explicitly empty backend config,
imports the embedded child image without a registry pull, validates/plans the
child, boots it, resolves its actual backend name, executes a command, and
removes it. A pinned CA bundle is required even for local image loading because
the runtime initializes an HTTP client. Buffered noninteractive exec receives
`/dev/null` explicitly so an inherited open stdin cannot block its launch.

Success requires `OUTER_KVM_API=12`, `INNER_EXEC_OK`,
`NESTED_TEARDOWN_OK`, and `NESTED_WORKESTRATE_OK` in the outer log. The last
marker follows an empty child-backend assertion. The guest then remains alive
for normal host teardown; the workload also has a 180-second command limit
with a five-second forced-termination grace period. Individual inner CLI calls
use a 30-second limit with the same grace period.
Always perform explicit teardown: command exit alone does not establish VM or
foreground-controller cleanup with the pinned runtime.

This proves nested packaged-runtime use, not builds through a guest Nix daemon,
SSH custody, or denial of nesting when it is omitted. Guest KVM was also exposed
with nesting omitted in a separate test; do not treat the opt-in request as a
verified confinement boundary. Audit leftover foreground controllers separately
from empty backend/registry results, and never use broad process cleanup.
