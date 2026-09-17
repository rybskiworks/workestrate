# Temporary local recovery

This is an explicit alternative to the normal package, not a completed repair
of its distribution-notice pipeline. The operator requested a removable way to
restore local agent development while the normal repair remains under review.

The latest strict build failed after finding 22 packages without discoverable
original legal evidence. They require a source/vendoring audit; repeatedly
rebuilding the application after adding one document is not a recovery plan.
The normal root flake and its checks remain unchanged by this subflake.

## Install

Run on the existing x86_64 Linux host, as the normal user, from any directory:

```sh
nix profile add -L \
  'github:rybskiworks/workestrate/8f884a39b85f0eaae8be7dafc1e26014843fc784?dir=recovery#workestrate-local-recovery' \
  --no-write-lock-file

hash -r
workestrate --version
```

No clone, separate dependency installation, input overrides, sudo, PR merge or
working-tree changes are required. Do not append the previous Microsandbox or
firmware overrides: this subflake deliberately uses the original matched pins.
Its single input has an immutable revision and a committed transitive lock;
this subflake has no handwritten lockfile. Nix resolves its top-level input for
this invocation without writing a lockfile.

The expected version includes `0.1.0-bf6fce4-local-recovery-registry-fix`.
The package also installs `share/licenses/workestrate/LOCAL-RECOVERY.txt`,
explaining the omission and recording the applied source patch.
A successful installation is not proof that agent provisioning or VM acceptance
has passed. Continue using the existing operator home and fleet; do not recreate,
reset, migrate or delete state as part of installing this package.

## What differs

The package selects:

- Workestrate `bf6fce49037d9ae870b847d7734785fa94dcef79` plus the one-line patch below.
- Its locked Microsandbox `251b368a868d578ead123071c3e6bc8eec013817`.
- That runtime's locked libkrunfw `d575b13e79368b23246be3d93d7935899dec5a3b`.

Assertions reject a different source/runtime/firmware selection. The original
runtime predates the new notice-generation hooks. The Workestrate override
omits its runtime notice-bundle precondition and its Rust notice-generation and
installation jobs. It retains SDK staging, source/version coupling, the original
runtime paths, the sops/age wrapper, and the original MSB_HOME default behavior.
Workestrate's own LICENSE, NOTICE, LICENSING.md and THIRD-PARTY.md remain installed.
No fake notice manifests are created. Microsandbox runtime source, KVM permissions,
credential policy, sandbox configuration and operator data are not changed.

The initial recovery at `56c1dd000cf404cc282572c5639e40461c1de3e9` reached
Workestrate compilation and exposed a separate source error in `secrets.rs`:
`load_registry()` returns `Result<Option<Registry>>`, while its autodetection
branch matched only `Ok(registry)` and then accessed `.configs` on the Option.
`secrets-registry-option.patch` corrects that match to `Ok(Some(registry))`.
It is applied through the standard Nix `patches` attribute against the frozen
crate, without changing the Cargo lockfile or runtime selection. No `unwrap()`
is introduced; missing/error registry results retain the existing cwd fallback.
This recovery patch does not itself repair the checked-in normal Rust source.
The old recovery revision remains in history but its install command is superseded.

This is a deliberate bypass of incomplete distribution-notice processing, not a
new claim that those obligations have been satisfied. Do not release, distribute
or publish this output to a shared binary cache as a qualified package. Existing
upstream licenses still apply. The subflake has no publication workflow; the
local-only label does not prevent a separately configured uploader from running.

## Remove after the normal package is ready

The explicitly selected package attribute is `workestrate-local-recovery`.
Inspect the profile and remove its entry before installing the corrected normal
package, to avoid two entries providing the same `bin/workestrate`:

```sh
nix profile list
nix profile remove workestrate-local-recovery
```

Use the displayed entry name if Nix has added a duplicate suffix. Profile removal
does not require deleting the Workestrate home, fleets, Microsandbox state, keys
or VM disks. Reclaiming unreferenced Nix store objects is separate from profile
removal; no garbage collection or history deletion is part of this recovery.

## Validation and follow-up

Initial candidate: source inspection checked the exact original recipe and
version build script. Six local snippet-level checks passed for hook-boundary
selection, refusal of missing/duplicated markers, preserved SDK setup, and
retained-wrapper shell syntax. Those checks modeled the Nix string operation in
Python with placeholder store paths; they were not Nix evaluation, the repository
suite or a package build. The subsequent user build exposed the Rust error above.

Registry correction: GNU patch applies the one-line change to the exact fetched
failing source excerpt without fuzz or offset. Reapplying the patch is refused
rather than reversed. These checks do not compile the Rust code or validate the
whole source file, Nix expression or runtime. Nix, Cargo and rustc are unavailable
in the editing environment; no successful full build is claimed.

The permanent repair remains in Workestrate #68 and Microsandbox #30. Apply the
registry match correction to the normal Rust source and cover the missing, failed,
empty, single and multiple registry cases. Preserve original upstream evidence
through vendoring, supply exact-version supplemental evidence where archives
genuinely omit it, audit all selected packages in a preflight, and validate the
real producer/consumer builds and generated locks. Do not mark either repair
complete based on this recovery package working.

References:

- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-profile-add.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-profile-remove.html
- https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake.html
