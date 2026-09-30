# Config test fleet

`workestrate-config-test` is an end-to-end (E2E) validation fleet: one capsule
per feature, each capsule booting a sandbox and asserting that the feature
behaves as declared. It is a separate repository, used by development and
validation tooling only. User workloads never reference it. The fleet exists
to hold the fixtures that only make sense while a feature is landing.

Once the fleet remote exists, its checkout lives at
`tests/fleets/workestrate-config-test` as a pinned submodule of this
repository. The capsule table, the assertions each capsule makes, and the
fleet's own safety rules live in that repository's `README.md` and
`README.agents.md`; do not duplicate them here.

## Fetch the submodule

```sh
git submodule update --init tests/fleets/workestrate-config-test
```

Keep that checkout pinned DETACHED. Never leave it on a feature branch. When
no explicit name is supplied, the active fleet name is derived from the first
layer's checkout branch, and a derived name can name a runtime slot while
another fleet supplies the content. The derivation ladder is described under
[selecting config and fleets](cli.md#selecting-config-and-fleets) and
implemented by `resolve_active_fleet` in
`control/agentctl/src/config/registry.rs`. A submodule pinned at a reviewed
commit takes the checkout branch out of that ladder.

## Select the fleet explicitly

Always name the fleet you mean. A one-command local override, in the form the
repository's own golden check uses (`just _golden-check-inner`):

```sh
env -u WORKESTRATE_CONFIG -u WORKESTRATE_FLEET -u WORKESTRATE_NO_PROJECT_CONFIG \
  WORKESTRATE_FLEET_DIR=tests/fleets/workestrate-config-test \
  cargo run --manifest-path control/agentctl/Cargo.toml -- workload plan <capsule>
```

The `env -u` prefixes drop ambient selectors so the command cannot pick up a
fleet from the environment. That override reads `<dir>/workestrate.toml`, so it
applies to a file-mode fleet (one `workestrate.toml`). A directory-mode fleet
(`workestrate/default.toml` plus `workestrate/workloads/`) is selected through
the registry entry below instead.

A registered local-path fleet entry is the other option. Then a name selects
it:

```sh
workestrate --config <config> --fleet workestrate-config-test workload plan <capsule>
```

A registry entry is `[fleets.<name>]` with `url`, `ref`, and `rev`
(`schemas/registry.schema.json`). Remote and `git+file://` entries consume a
recorded revision; a plain local-path entry loads the working files directly
([fleet management](cli.md#configuration-and-fleet-management)), so the
submodule's working tree is what gets read.

`--fleet` takes a NAME, never a path. `--fleet-dir` is a secrets-only
selector ([secrets](secrets.md)); it does not change the fleet that `workload`
commands read.

## Capsules and parse status

The first capsules target the mount features still pending on `main`:

- Disk attachments: `kind = "disk"` mount rows with `format`, `fstype`, and
  `readonly` fields.
- Per-mount write budgets: `quota_mib` on the mount row.

Those fields land with the declared disk attachments work (issue #109), so
these capsules do not parse against `main` today: `workload plan` rejects
fields that are not part of the wire type yet. Parse them against a checkout
of that work.

Capsules that wait on the same work:

- Attach-only disks and LUKS: `attach_only` is not yet a field of the mount
  wire type.
- Quarantine: waits on the same field.
- SSD storage profile: needs host SSD backing for `$MSB_HOME/sandboxes`
  before it can boot.

## Status

The submodule wiring here is provisional and tracked by issue #95 (optional
configuration submodule with feature fleets). `docs/adr.md` records the
repository convention that agent repositories are flake inputs, not
submodules (ADR 0001, kept as history), so that decision comes before this
shape is final.

`just verify-config-test` is the host gate for the fleet. It is a stub: the
capsule list, the golden plan files, and the CI wiring arrive with the fleet
repository and with issue #95. The recipe fails fast when
`tests/fleets/workestrate-config-test` is absent, and it runs the capsules
only on a host that sets `WORKESTRATE_CONFIG_TEST_HOST=1` and provides
`/dev/kvm`. Hosted CI does not run it: the CI workflow fetches no submodules
and does not certify KVM.

## Where to read more

- The fleet repository's `README.md`: the capsule table and run instructions.
- The fleet repository's `README.agents.md`: ownership, per-capsule
  assertions, and the safety rules for adding capsules.
- [Testing Workestrate](testing.md): the repository gate this stub stays out
  of.
