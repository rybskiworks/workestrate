# Licensing and distribution

## Workestrate's license and copyright

Workestrate's original material is licensed under Apache-2.0, except where a file
or third-party component specifies otherwise. `LICENSE` is the single root
project license file; `LICENSES/Apache-2.0.txt` is retained for REUSE tooling.
Copyright for Georg Rybski's original material is attributed to Georg Rybski,
not to the GitHub organization as a substitute legal owner.

A copyright notice does not transfer another contributor's rights. Original
contributions, imported material, employment/contract assignments, and upstream
fork changes must be distinguished. No blanket copyright reassignment or new
contributor license agreement is introduced by this change. Third-party material
is excluded from the project-default license; see `THIRD-PARTY.md`.

The source before this change, including revision
`76fe5e4c42c60fd371e213465ac57af8be0d7973`, was offered as MIT OR Apache-2.0.
Existing grants for those versions are not revoked. The original MIT text and
copyright notice are preserved in `LICENSES/MIT.txt` for that history and any
applicable retained material. Its presence is not a new MIT alternative for
subsequent Apache-only changes. The dependency allow-list remains broader than
Apache because dependency permission and this project's license are different.

## Automated Rust notices

`deny.toml` remains the dependency-license policy. `cargo-deny` checks acceptance;
`scripts/licensing/cargo_notices.py` derives the `cargo-about` configuration from
that policy and produces a separate notice bundle. Do not maintain a second
independent accepted-license list.

The Nix package uses Cargo and cargo-about from the pinned toolchain/package set.
Generation runs locked and offline in the same patched build source, with the
same target and feature selection. Local Microsandbox crates are explicitly
included. Transitive and build dependencies are retained conservatively; purely
development dependencies are excluded. The generator records the lock and policy
hashes, target, features, and generator version.

The bundle contains `THIRD-PARTY.html`, normalized `cargo-about.json`,
`inventory.json`, original license/copyright/NOTICE files, and a checksum manifest.
The build fails on unresolved licenses, missing selected crates, missing original
legal evidence, or an invalid bundle. Generic SPDX text is not accepted as a
replacement for missing original attribution. Upstream source headers and unusual
licensing arrangements still require review; a passing report is not a legal
certification. Missing original files must be supplied from reviewed pinned
sources rather than worked around by ignoring the crate.

Wildcard per-crate exceptions remain per-crate. Version-constrained exceptions
and cargo-deny hash clarifications require explicit translation; the helper
refuses to silently broaden them or convert incompatible hash formats. AND
expressions are resolved by cargo-about, not replaced with OR. Apache is preferred
when a dependency genuinely offers it as an alternative.

Inside an already prepared pinned build/development environment:

```sh
python3 scripts/licensing/cargo_notices.py generate \
  --manifest control/agentctl/Cargo.toml --policy deny.toml \
  --target x86_64-unknown-linux-gnu \
  --source-root "$PWD" \
  --source-root "$(realpath control/agentctl/vendor/microsandbox-fork)" \
  --output /tmp/workestrate-notices
python3 scripts/licensing/cargo_notices.py verify /tmp/workestrate-notices
```

The output directory must not already exist. These commands neither provision
workloads nor fetch source dependencies. Nix stages dependencies before the
sandboxed generation step. The `verify` command also checks an assembled/copy of
a bundle, not only its original build directory.

## Native, firmware, and image boundaries

Rust notices are not a complete Nix closure inventory. Native libraries, embedded
executables, firmware and guest image contents retain their own terms. In
particular, libcap-ng, libc, compiler runtime libraries, SOPS and any image's Nix
implementation must be reviewed for the exact distributed closure. Static musl
agentd has no dynamic-library dependency, but still contains third-party code.

The firmware producer change is tracked in
https://github.com/rybskiworks/libkrunfw/pull/5. It retains the GPL kernel and LGPL
wrapper terms and provides a binary-bound corresponding-source bundle. The
Microsandbox assembler must carry that bundle and both CLI and embedded-agent
notices into its output. Workestrate's existing wrapper retains the runtime Nix
closure; portable distributions must copy the legal material, not just the ELF.

Do not publish an image or Nix cache closure on the basis of a Cargo report alone.
Before publication, verify actual contents, native notices, corresponding source,
redistribution permissions, and the delivery mechanism. A hash is not source, a
metadata license label is not a license text, and an SBOM is not a source offer.
The generic firmware bundle does not certify TEE/qboot/initrd or other variants.
The Lix migration is intentionally separate; replacing Nixd does not erase prior
distribution questions or Lix's own license obligations.

## Repository metadata and third-party documents

`REUSE.toml` deliberately annotates only the new reviewed licensing files. It does
not stamp Georg Rybski's copyright or Apache-2.0 on imported reference material.
The known nix.dev excerpt has its own companion `.license` attribution. Full
repository REUSE compliance is NOT claimed: other crawls, skills, and embedded
reference material need a provenance review before their metadata is asserted.
Use `reuse lint-file` for reviewed paths and `reuse lint` to discover remaining
coverage gaps; do not blanket-override those gaps with project-default metadata.

## Historical remediation

Run the read-only triage inventory from a complete local clone:

```sh
python3 scripts/licensing/audit_history.py --repo . > /tmp/license-history.json
```

This reports reachable objects and representative path hints only. It does not
fetch missing refs, discover external caches, infer infringement, or remove data.
Inventory releases, CI artifacts, caches and shared images separately. For an
older binary still offered, provide the source/notices for that exact build.
Fixing the current branch does not automatically repair or cure earlier copies.

Do not rewrite history merely because it once named a dependency. Actual content
without redistribution permission needs permission or an appropriately scoped
removal plan, including historic copies where applicable. Such removal, any
license-specific reinstatement, and remote/cache cleanup require a separate
review and explicit authorization. Prior license grants are preserved.

## Outstanding release qualification

The Python fixture tests validate tooling behavior, not every real dependency.
Before this stack is promoted: run pinned cargo-about and cargo-deny on actual
builds; verify missing upstream legal evidence; build/reconstruct firmware; promote
producer commit pins with Nix/Lix-generated lockfiles; inspect the final native
and image closures; complete the remaining crawl/skill and historic distribution
inventory. Keep the PRs in draft until their applicable build and promotion gates
pass. No Nix lock hashes are invented to bypass that work.

References:
- https://www.apache.org/licenses/LICENSE-2.0
- https://embarkstudios.github.io/cargo-about/cli/generate/index.html
- https://embarkstudios.github.io/cargo-about/cli/generate/config.html
- https://reuse.software/spec/
- https://www.gnu.org/licenses/old-licenses/gpl-2.0.html
- https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html
