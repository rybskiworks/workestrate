# Config test fleet

`workestrate-config-test` is an end-to-end (E2E) validation fleet: one capsule
per feature, each capsule booting a sandbox and asserting that the feature
behaves as declared. It is a separate repository, used by development and
validation tooling only. User workloads never reference it. The fleet exists
to hold the fixtures that only make sense while a feature is landing.

The checkout lives at `tests/fleets/workestrate-config-test` as a submodule of
this repository, pinned DETACHED at the commit the superproject records (the
gitlink records one commit, `e26fade`; `git submodule status` prints the live
pin). The capsule table, the assertions each capsule makes, and the fleet's own
safety rules live in that repository's `README.md` and `README.agents.md`; do
not duplicate them here.

## Fetch the submodule

```sh
git submodule update --init tests/fleets/workestrate-config-test
```

A plain `git clone` of this repository leaves that path empty: the recorded
commit is fetched by the update command above, not by the clone. Keep the
checkout pinned DETACHED; never leave it on a feature branch. Hosted CI fetches
no submodules — every `actions/checkout` step in `.github/workflows/ci.yml`
omits `submodules:` — so the path stays empty there and the host-gated target
below cannot run in CI.

When no explicit name is supplied, the active fleet name is derived from the
first layer's checkout branch, and a derived name can name a runtime slot while
another fleet supplies the content. The derivation ladder is described under
[selecting config and fleets](cli.md#selecting-config-and-fleets) and
implemented by `resolve_active_fleet` in
`control/agentctl/src/config/registry.rs`. A submodule pinned at a reviewed
commit takes the checkout branch out of that ladder, and the fleet is still
named explicitly on every command.

## Select the fleet explicitly

`--fleet` takes a NAME, never a path, so the checkout is registered as a fleet
in the active config's registry first. The fleet repository ships the entry
shapes in `configs/registry-example.toml`: a plain local-path entry whose `url`
is the absolute path of this checkout, and a pinned-remote entry with `url`,
`ref`, and `rev` (`schemas/registry.schema.json`). `workestrate fleet add`
writes the same shape. Remote and `git+file://` entries consume a recorded
revision; a plain local-path entry loads the working files directly
([fleet management](cli.md#configuration-and-fleet-management)), so the
submodule's working tree is what gets read. Then name the fleet every time:

```sh
workestrate --config <config> --fleet workestrate-config-test workload plan box-smoke
```

Selecting explicitly is the point: with no selector the name is derived, and
step (c) of that ladder (`checkout_branch_candidate`) reads the first
configuration layer's checkout branch — a derived name can name a runtime slot
or a different fleet while the default fleet supplies the content.

`WORKESTRATE_FLEET_DIR` cannot select this fleet. That override loads
`<dir>/workestrate.toml` as a single layer
(`control/agentctl/src/config/loading.rs`), and this fleet is directory mode:
`workestrate/default.toml` plus `workestrate/workloads/<capsule>/workload.toml`.
`--fleet-dir` is a secrets-only selector ([secrets](secrets.md)); it does not
change the fleet that `workload` commands read.

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

The submodule is wired and pinned at the reviewed fleet commit. The host gate
is still incomplete:

- Golden plans. `just verify-config-test` diffs each capsule's `workload plan`
  against `golden/<capsule>.plan.txt` in the fleet checkout, and the fleet
  repository carries no `golden/` directory yet. The recipe reads the fleet
  through `WORKESTRATE_FLEET_DIR`, which cannot select this directory-mode
  fleet, so it needs the registered-name form above before it can pass. Its
  capsule list names `box-smoke` and `disk-attach`; the fleet's third capsule,
  `write-budget`, is not in the list.
- CI wiring. Nothing in `.github/workflows/ci.yml` fetches the submodule or
  certifies KVM, so the target stays host-only.

The submodule wiring here is provisional and tracked by issue #95 (optional
configuration submodule with feature fleets). `docs/adr.md` records the
repository convention that agent repositories are flake inputs, not submodules
(ADR 0001, kept as history), so that decision comes before this shape is final.

The recipe fails fast when `tests/fleets/workestrate-config-test` is absent, and
it runs the capsules only on a host that sets `WORKESTRATE_CONFIG_TEST_HOST=1`
and provides `/dev/kvm`.

## Where to read more

- The fleet repository's `README.md`: the capsule table and run instructions.
- The fleet repository's `README.agents.md`: ownership, per-capsule
  assertions, and the safety rules for adding capsules.
- [Testing Workestrate](testing.md): the repository gate this stub stays out
  of.
