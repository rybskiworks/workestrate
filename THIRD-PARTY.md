# Third-party material and distribution boundaries

This is an initial reviewed inventory, not an exhaustive bill of materials or
statement that every historical distribution was compliant. Workestrate's
Apache-2.0 default does not relicense the material described here. Keep original
copyright notices, license texts and applicable modification/NOTICE information.

## Runtime components

- Microsandbox workspace: Apache-2.0, pinned by `flake.lock`. Preserve Super Rad
  Company and other applicable upstream notices. Fork ownership is not wholesale
  ownership of upstream code.
- libkrun Rust API and included upstream components: the selected msb_krun API is
  Apache-2.0. Preserve the notices for libkrun, Firecracker, rust-vmm and other
  applicable code; collect the actual selected graph and source evidence.
- libkrunfw: library code LGPL-2.1-only; bundled Linux kernel and kernel patches
  GPL-2.0-only. Carry its exact corresponding-source/legal bundle. These are
  licenses of different components, not alternative licenses for one component.
- Rust dependencies: use the generated per-build report, not this prose inventory.
  MPL and compound-license dependencies retain their obligations. A NOTICE file
  alone does not satisfy source requirements.
- Native libraries: libcap-ng, libc/musl, compiler runtimes and other actual linked
  code need their own notices and source/relinking treatment where applicable.
- Guest images and operator-selected agents: evaluate their own complete closure.
  They are not automatically Apache-2.0 because Workestrate launches them.

The packaging code and generated inventories identify what was selected. This
file does not certify redistribution of unreviewed proprietary binaries or the
complete closure of a NixOS guest image.

## Identified nix.dev excerpt

Path: `docs/nix/.crawl/50-contributing-documentation.md`

Upstream: nix.dev contributors, https://github.com/NixOS/nix.dev

Source revision: `139034be5e14320c05f792872e6150bd981490d5`

Source page: https://nix.dev/contributing/documentation/index.html

License: Creative Commons Attribution-ShareAlike 4.0 International (CC-BY-SA-4.0),
https://creativecommons.org/licenses/by-sa/4.0/legalcode.en

The repository copy is a crawl excerpt with additional front matter/provenance
wrapping and conversion to Markdown. The licensing cleanup adds attribution and
does not claim that the excerpt is a complete or current upstream document.
The upstream disclaimer applies; no warranties are provided by the upstream
license. Its text remains third-party material, not Georg Rybski's original work.
Retain source attribution and change notices with redistributed copies and
comply with applicable ShareAlike conditions for adaptations.

## Pending provenance review, not covered by an ownership assertion

Other `docs/**/.crawl/**`, derived reference pages, and `.agents/skills/**` may
contain material from different upstreams and licenses. Inspect each source and
revision and any per-file exception. Do not apply the nix.dev license to every
crawl, nor Apache-2.0 to every skill. The history inventory helper provides triage
hints, not proof of ownership or license compatibility. New imported material
must include its origin, immutable revision where available, actual license,
applicable attribution, and modification description at submission time.

## Private configurations and private agent forks

This cleanup does not release private fleet/configuration repositories or reassign
contract/client-owned material. Prime-agent, T3MP3ST and other agent forks retain
upstream licenses, including MIT/AGPL and separately licensed benchmark assets as
applicable. Their independent audits are not replaced by Workestrate's license.
