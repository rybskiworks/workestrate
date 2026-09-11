---
type: Reference
title: Nix Source Map
description: Provenance index mapping all crawled nix.dev sources to the Nix documentation topic files.
tags: [provenance, source-map]
timestamp: 2026-07-24T00:58:00Z
---

# Nix Source Map

## Purpose

This document maps every crawled source URL to the Nix documentation files that use it. It is the authoritative provenance index behind the `docs/nix/` topic files. Future agents can trace any claim back to its primary source, identify coverage gaps, and assess which sources remain available for future expansion. All 55 sources were extracted from a clone of the `github.com/nixos/nix.dev` repository (commit `139034be5e14320c05f792872e6150bd981490d5`, dated 2026-07-21); the extracted content is persisted in `docs/nix/.crawl/01-55`.

The intended audience is any future maintainer, reviewer, or agent who needs to verify a claim, add a topic, or audit coverage. It is a provenance layer between the generated guidance and the official sources, not a replacement for them. Every source is an official nix.dev documentation page maintained by the [Nix documentation team](https://nixos.org/community/teams/documentation). The nix.dev site is the canonical home of official documentation for the Nix ecosystem, covering tutorials, guides, reference material, concepts, contributing guidelines, and acknowledgements.

The crawl also captured four raw HTML artifacts in `docs/nix/.crawl/_raw/`: the nix.dev table-of-contents sidebar (`nix-toc.html`, 61 KB), the Nixpkgs reference manual (`nixpkgs-full.html`, 2.8 MB), and two TOC fetches from `nixos.org` (`nixos-toc.html`, `nixpkgs-toc.html`) that both returned HTTP 404 — the nixos.org site does not host a parallel documentation TOC. These raw artifacts are retained for provenance but were not used as primary extraction sources; the 55 crawl files were extracted directly from the repository's MyST markdown source.

## Source coverage summary

The corpus comprises 55 crawled source URLs across 8 documentation families, all official primary and all from nix.dev.

| Family | Count | Authority |
|---|---|---|
| Root (nix.dev index) | 1 | Official primary |
| Install (getting started) | 2 | Official primary |
| Tutorials (first steps, language, packaging, module system, NixOS) | 24 | Official primary |
| Guides (best practices, FAQ, troubleshooting, recipes) | 12 | Official primary |
| Reference (glossary, Nix manual, pinning Nixpkgs) | 4 | Official primary |
| Concepts (flakes, FAQ) | 3 | Official primary |
| Contributing (how-to, documentation framework, style guide) | 8 | Official primary |
| Acknowledgements | 1 | Official primary |
| **Total** | **55** | |

The Tutorials family is the largest (24 pages) and forms the operational backbone — covering ad hoc shell environments, declarative shells, reproducible scripts, Nixpkgs pinning, the Nix language, packaging, cross-compilation, the module system, and NixOS deployment/testing. The Guides family (12 pages) supplies best practices, troubleshooting, and recipe-style solutions for CI, direnv, binary caches, and language-specific environments. The Reference family (4 pages) provides the glossary, the Nix reference manual, and Nixpkgs pinning. The Concepts family (3 pages) covers flakes and conceptual FAQs. The Contributing family (8 pages) documents how to contribute, the documentation framework (Diátaxis), the style guide, and how to write tutorials. All 55 pages were extracted to completion from the repository clone.

## Link expansion coverage

- **Depth-0 seed pass:** The crawl began with a single entry point — the nix.dev root index page (`https://nix.dev/index.html`, crawl 01). This is the depth-0 seed. The root page's MyST toctree lists the top-level sections: `install-nix`, `tutorials/index`, `guides/index`, `reference/index`, `concepts/index`, `contributing/index`, `acknowledgements/index`.
- **Depth-1 discovery:** From the root toctree (01), the top-level section index pages were reached: Install Nix (02), Tutorials index (04), Guides index (28), Reference index (40), Concepts index (44), Contributing index (47), and Acknowledgements (55). The Further reading page (03) was discovered from the Install page (02).
- **Depth-2 discovery (tutorials):** From the Tutorials index (04), the tutorial sub-sections were followed: First Steps index (05) and its children (06–09), Nix language basics (10), Working with local files (11), callPackage (12), Cross compilation (13), Packaging existing software (14), Module system index (15) and its children (16–17), and NixOS index (18) and its children (19–27).
- **Depth-2 discovery (guides):** From the Guides index (28), the guide pages were followed: Best practices (29), FAQ (30), Troubleshooting (31), Recipes index (32) and its children (33–39).
- **Depth-2 discovery (reference):** From the Reference index (40), the reference pages were followed: Glossary (41), Nix reference manual (42), Pinning Nixpkgs (43).
- **Depth-2 discovery (concepts):** From the Concepts index (44), the concept pages were followed: Concepts FAQ (45), Flakes (46).
- **Depth-2 discovery (contributing):** From the Contributing index (47), the contributing pages were followed: How to contribute (48), How to get help (49), Contributing documentation index (50) and its children (51–54).
- **Families with rich discovered links:** The root page (01) had the richest cross-referencing — its toctree links to every top-level section. The Tutorials index (04) was the hub for 23 tutorial pages. The Guides index (28) was the hub for 11 guide pages. The NixOS tutorials index (18) was the hub for 9 NixOS-specific tutorial pages (19–27). The Recipes index (32) was the hub for 7 recipe pages (33–39). The Contributing documentation index (50) was the hub for 4 documentation sub-pages (51–54).
- **Crawl methodology:** Unlike the Gleam and Elixir corpora (which were live-fetched via curl/HTTP), the Nix corpus was extracted from a local clone of the `github.com/nixos/nix.dev` Git repository. This ensures byte-exact reproducibility — the crawl is pinned to commit `139034be5e14320c05f792872e6150bd981490d5` (2026-07-21). Each crawl file's metadata header records `fetch: cloned from github.com/nixos/nix.dev` and the commit hash. The nix.dev site is built with Sphinx/MyST, so the extracted content is the source markdown rather than rendered HTML.
- **Raw HTML artifacts:** Four additional HTML files were fetched and stored in `docs/nix/.crawl/_raw/`:
  - `nix-toc.html` (61 KB) — the nix.dev mdBook sidebar table of contents, used to verify the page enumeration was complete.
  - `nixpkgs-full.html` (2.8 MB) — the full Nixpkgs reference manual rendered as a single HTML page. Retained for future reference but not extracted into individual crawl files; the Nixpkgs manual is a separate, much larger corpus.
  - `nixos-toc.html` (20 KB) — a fetch of `https://nixos.org/` TOC that returned HTTP 404 (Not Found). The nixos.org site does not host a parallel documentation TOC; nix.dev is the canonical documentation home.
  - `nixpkgs-toc.html` (20 KB) — same 404 result from nixos.org.
- Note: the crawl ledger headers carry a `feeds_docs` field currently set to `TBD` for all 55 pages. This field will be populated once the Nix topic docs are written from these crawl extractions. Origin (seed vs discovered) below is inferred from the toctree structure and crawl order — page 01 (the root index) is classified as seed; pages 02, 04, 28, 40, 44, 47, 55 (top-level section indexes reachable from the root toctree) are classified as seed; all remaining pages are classified as discovered.

## Sources by topic

Legend:

- **Origin:** seed = depth-0 root index or depth-1 top-level section index reachable from the root toctree; discovered = reached by following toctree links at depth 2+.
- **Authority:** official primary = nix.dev documentation maintained by the Nix documentation team.
- **Coverage status:** fully extracted = the page's MyST markdown source was extracted to completion from the repository clone. All 55 pages are fully extracted.

### 1. Root (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 01 | https://nix.dev/index.html | - | seed | Root | Top-level documentation hub/index; maps entire nix.dev documentation surface (tutorials, guides, reference, concepts, contributing, acknowledgements); what you can do with Nix; who Nix is for | TBD | Official primary | Fully extracted |

### 2. Install (2 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 02 | https://nix.dev/install-nix.html | - | seed | Install | Install Nix; getting started entry point | TBD | Official primary | Fully extracted |
| 03 | https://nix.dev/recommended-reading.html | - | discovered | Install | Further reading; recommended reading list for Nix beginners | TBD | Official primary | Fully extracted |

### 3. Tutorials (24 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 04 | https://nix.dev/tutorials/index.html | - | seed | Tutorials | Tutorials index; maps the tutorial series (first steps, language, packaging, module system, NixOS) | TBD | Official primary | Fully extracted |
| 05 | https://nix.dev/tutorials/first-steps/index.html | - | discovered | Tutorials | First steps index; introduction to the first-steps tutorial series | TBD | Official primary | Fully extracted |
| 06 | https://nix.dev/tutorials/first-steps/ad-hoc-shell-environments.html | - | discovered | Tutorials | Ad hoc shell environments; `nix shell`, `nix run`, `nix-build`; imperatively creating shell environments with specific packages | TBD | Official primary | Fully extracted |
| 07 | https://nix.dev/tutorials/first-steps/declarative-shell.html | - | discovered | Tutorials | Declarative shell environments with `shell.nix`; `mkShell`; reproducible development environments | TBD | Official primary | Fully extracted |
| 08 | https://nix.dev/tutorials/first-steps/reproducible-scripts.html | - | discovered | Tutorials | Reproducible interpreted scripts; `#!/usr/bin/env nix` shebang; pinning interpreters and dependencies | TBD | Official primary | Fully extracted |
| 09 | https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs.html | - | discovered | Tutorials | Towards reproducibility: pinning Nixpkgs; `--override-input`, `--update-input`; reproducible builds via pinned inputs | TBD | Official primary | Fully extracted |
| 10 | https://nix.dev/tutorials/nix-language.html | - | discovered | Tutorials | Nix language basics; reading the Nix language; purely functional, lazily evaluated, dynamically typed DSL; derivations; builtins, pkgs.lib, stdenv.mkDerivation, override, overlays, callPackage; basic data types, let bindings, functions, attribute sets, lists, with, assert, imports | TBD | Official primary | Fully extracted |
| 11 | https://nix.dev/tutorials/working-with-local-files.html | - | discovered | Tutorials | Working with local files; `builtins.path`, `builtins.readFile`, `builtins.filterSource`, `builtins.pathType`; local file ingestion into derivations | TBD | Official primary | Fully extracted |
| 12 | https://nix.dev/tutorials/callpackage.html | - | discovered | Tutorials | Package parameters and overrides with `callPackage`; `callPackage` pattern, `override`, `overrideAttrs`, `overrideDerivation`; dependency injection in Nixpkgs | TBD | Official primary | Fully extracted |
| 13 | https://nix.dev/tutorials/cross-compilation.html | - | discovered | Tutorials | Cross compilation; `pkgsCross`, cross-system attributes, `buildPlatform`/`hostPlatform`/`targetPlatform` | TBD | Official primary | Fully extracted |
| 14 | https://nix.dev/tutorials/packaging-existing-software.html | - | discovered | Tutorials | Packaging existing software with Nix; `stdenv.mkDerivation`, `nativeBuildInputs`, `buildInputs`, phases, `meta` attributes, `passthru`, `fetchFromGitHub` | TBD | Official primary | Fully extracted |
| 15 | https://nix.dev/tutorials/module-system/index.html | - | discovered | Tutorials | Module system index; introduction to the Nix module system tutorial series | TBD | Official primary | Fully extracted |
| 16 | https://nix.dev/tutorials/module-system/a-basic-module/index.html | - | discovered | Tutorials | A basic module; `mkEnableOption`, `mkOption`, `types`, `mkIf`; defining and using a NixOS-style module | TBD | Official primary | Fully extracted |
| 17 | https://nix.dev/tutorials/module-system/deep-dive.html | - | discovered | Tutorials | Module system deep dive; `lib.evalModules`, `moduleType`, `submodule`, option merging, `mkDefault`/`mkForce`/`mkOverride`, priority system, `mkMerge`, deferred modules | TBD | Official primary | Fully extracted |
| 18 | https://nix.dev/tutorials/nixos/index.html | - | discovered | Tutorials | NixOS index; introduction to the NixOS tutorial series (binary cache, Docker, ISO, Terraform, distributed builds, Raspberry Pi, VM testing, provisioning) | TBD | Official primary | Fully extracted |
| 19 | https://nix.dev/tutorials/nixos/binary-cache-setup.html | - | discovered | Tutorials | Setting up an HTTP binary cache; `nix-serve`, `harmonia`, `attic`; serving the Nix store over HTTP | TBD | Official primary | Fully extracted |
| 20 | https://nix.dev/tutorials/nixos/building-and-running-docker-images.html | - | discovered | Tutorials | Building and running Docker images; `pkgs.dockerTools`, `buildImage`, `streamLayeredImage`; container image creation with Nix | TBD | Official primary | Fully extracted |
| 21 | https://nix.dev/tutorials/nixos/building-bootable-iso-image.html | - | discovered | Tutorials | Building a bootable ISO image; `config.system.build.isoImage`, `nixos-generate`; creating bootable installation media | TBD | Official primary | Fully extracted |
| 22 | https://nix.dev/tutorials/nixos/deploying-nixos-using-terraform.html | - | discovered | Tutorials | Deploying NixOS using Terraform; `nixos_anywhere`, `disko`; infrastructure-as-code deployment of NixOS machines | TBD | Official primary | Fully extracted |
| 23 | https://nix.dev/tutorials/nixos/distributed-builds-setup.html | - | discovered | Tutorials | Setting up distributed builds; `nix build --builders`, remote builder protocol, `nix.distributedBuilds`; offloading builds to remote machines | TBD | Official primary | Fully extracted |
| 24 | https://nix.dev/tutorials/nixos/installing-nixos-on-a-raspberry-pi.html | - | discovered | Tutorials | Installing NixOS on a Raspberry Pi; ARM cross-compilation, SD card imaging, `nixos-install` | TBD | Official primary | Fully extracted |
| 25 | https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines.html | - | discovered | Tutorials | Integration testing with NixOS virtual machines; `nixosTests`, `runVM`, `test-instrumentation`; declarative VM-based integration tests | TBD | Official primary | Fully extracted |
| 26 | https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html | - | discovered | Tutorials | NixOS virtual machines; `nixos-shell`, `qemu`; running NixOS configurations as VMs for development and testing | TBD | Official primary | Fully extracted |
| 27 | https://nix.dev/tutorials/nixos/provisioning-remote-machines.html | - | discovered | Tutorials | Provisioning remote machines via SSH; `nixos-rebuild`, `deploy-rs`, `colmena`; remote NixOS deployment and provisioning | TBD | Official primary | Fully extracted |

### 4. Guides (12 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 28 | https://nix.dev/guides/index.html | - | seed | Guides | Guides index; maps the guides section (best practices, FAQ, troubleshooting, recipes) | TBD | Official primary | Fully extracted |
| 29 | https://nix.dev/guides/best-practices.html | - | discovered | Guides | Best practices; idiomatic Nix usage patterns, flake structure, versioning, reproducibility guidance | TBD | Official primary | Fully extracted |
| 30 | https://nix.dev/guides/faq.html | - | discovered | Guides | Frequently Asked Questions; common Nix questions and answers; flakes vs channels, GC roots, store paths | TBD | Official primary | Fully extracted |
| 31 | https://nix.dev/guides/troubleshooting.html | - | discovered | Guides | Troubleshooting; common Nix errors, debugging techniques, store corruption, build failures | TBD | Official primary | Fully extracted |
| 32 | https://nix.dev/guides/recipes/index.html | - | discovered | Guides | Recipes index; maps the recipes section (binary cache, CI, dependency management, direnv, post-build hooks, Python, sharing deps) | TBD | Official primary | Fully extracted |
| 33 | https://nix.dev/guides/recipes/add-binary-cache.html | - | discovered | Guides | Configure Nix to use a custom binary cache; `substituters`, `trusted-public-keys`, `nix.conf`; cache configuration | TBD | Official primary | Fully extracted |
| 34 | https://nix.dev/guides/recipes/continuous-integration-github-actions.html | - | discovered | Guides | Continuous integration with GitHub Actions; `cachix-action`, `nix-build-action`, CI caching strategies | TBD | Official primary | Fully extracted |
| 35 | https://nix.dev/guides/recipes/dependency-management.html | - | discovered | Guides | Automatically managing remote sources with `npins`; `npins` as a flake-compatible alternative to niv; source pinning | TBD | Official primary | Fully extracted |
| 36 | https://nix.dev/guides/recipes/direnv.html | - | discovered | Guides | Automatic environment activation with `direnv`; `direnv` + `nix-direnv` integration; `use flake`, `use nix` | TBD | Official primary | Fully extracted |
| 37 | https://nix.dev/guides/recipes/post-build-hook.html | - | discovered | Guides | Setting up post-build hooks; `post-build-hook`, automatic cache uploading after builds | TBD | Official primary | Fully extracted |
| 38 | https://nix.dev/guides/recipes/python-environment.html | - | discovered | Guides | Setting up a Python development environment; `python3.withPackages`, `poetry2nix`, virtualenv integration | TBD | Official primary | Fully extracted |
| 39 | https://nix.dev/guides/recipes/sharing-dependencies.html | - | discovered | Guides | Dependencies in the development shell; sharing dev environments, `shell.nix` portability | TBD | Official primary | Fully extracted |

### 5. Reference (4 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 40 | https://nix.dev/reference/index.html | - | seed | Reference | Reference index; maps the reference section (glossary, Nix manual, pinning Nixpkgs) | TBD | Official primary | Fully extracted |
| 41 | https://nix.dev/reference/glossary.html | - | discovered | Reference | Glossary; Nix terminology definitions (derivation, store path, profile, generation, channel, flake, overlay, closure, FOD, etc.) | TBD | Official primary | Fully extracted |
| 42 | https://nix.dev/reference/nix-manual.html | - | discovered | Reference | Nix reference manual; link to the official Nix manual (external); Nix CLI, configuration, store, daemon | TBD | Official primary | Fully extracted |
| 43 | https://nix.dev/reference/pinning-nixpkgs.html | - | discovered | Reference | Pinning Nixpkgs; `flake.lock`, `--override-input`, `--update-input`, `fetchTarball` with revision pinning; reproducibility via input pinning | TBD | Official primary | Fully extracted |

### 6. Concepts (3 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 44 | https://nix.dev/concepts/index.html | - | seed | Concepts | Concepts index; maps the concepts section (FAQ, flakes) | TBD | Official primary | Fully extracted |
| 45 | https://nix.dev/concepts/faq.html | - | discovered | Concepts | Frequently Asked Questions (concepts); conceptual Nix questions; channels vs flakes, purity, reproducibility philosophy | TBD | Official primary | Fully extracted |
| 46 | https://nix.dev/concepts/flakes.html | - | discovered | Concepts | Flakes; `flake.nix` structure, `inputs`/`outputs`, `flake.lock`, experimental feature (Nix 2.4+), built-in output types, flake commands | TBD | Official primary | Fully extracted |

### 7. Contributing (8 pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 47 | https://nix.dev/contributing/index.html | - | seed | Contributing | Contributing index; maps the contributing section (how to contribute, how to get help, documentation) | TBD | Official primary | Fully extracted |
| 48 | https://nix.dev/contributing/how-to-contribute.html | - | discovered | Contributing | How to contribute; contribution workflow, GitHub PRs, review process, nix.dev repository structure | TBD | Official primary | Fully extracted |
| 49 | https://nix.dev/contributing/how-to-get-help.html | - | discovered | Contributing | How to get help; community resources, Discourse forum, Matrix chat, documentation channels | TBD | Official primary | Fully extracted |
| 50 | https://nix.dev/contributing/documentation/index.html | - | discovered | Contributing | Contributing documentation index; maps the documentation contribution section (framework, resources, style guide, writing tutorials) | TBD | Official primary | Fully extracted |
| 51 | https://nix.dev/contributing/documentation/diataxis.html | - | discovered | Contributing | Documentation framework; Diátaxis framework (tutorials, how-to guides, reference, explanation); nix.dev documentation organization philosophy | TBD | Official primary | Fully extracted |
| 52 | https://nix.dev/contributing/documentation/resources.html | - | discovered | Contributing | Documentation resources; tooling, Sphinx/MyST, build instructions, previewing changes | TBD | Official primary | Fully extracted |
| 53 | https://nix.dev/contributing/documentation/style-guide.html | - | discovered | Contributing | Style guide; writing conventions, formatting rules, code block style, heading style, tone guidelines for nix.dev | TBD | Official primary | Fully extracted |
| 54 | https://nix.dev/contributing/documentation/writing-a-tutorial.html | - | discovered | Contributing | How to write a tutorial; tutorial structure, learning objectives, step-by-step guidance, Diátaxis tutorial guidelines | TBD | Official primary | Fully extracted |

### 8. Acknowledgements (1 page)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|
| 55 | https://nix.dev/acknowledgements/index.html | - | seed | Acknowledgements | Acknowledgements; sponsoring organisations (Antithesis, flox, Tweag, Determinate Systems, Cachix); documentation team history and contributors | TBD | Official primary | Fully extracted |

## Redirects

No redirects were encountered during the crawl. All 55 seed URLs are identical to their canonical URLs. The crawl was performed by cloning the `github.com/nixos/nix.dev` repository rather than live HTTP fetching, so URL resolution and redirect behaviour were not a factor. The `canonical_url` field in every crawl ledger header matches the `seed_url` field exactly.

Two auxiliary HTTP fetches from `nixos.org` (stored in `docs/nix/.crawl/_raw/nixos-toc.html` and `nixpkgs-toc.html`) returned HTTP 404 (Not Found). These were exploratory fetches to check whether `nixos.org` hosted a parallel documentation TOC; it does not. The canonical documentation source is `nix.dev`.

## Discovered but not crawled

The following sources were discovered as cross-references or known ecosystem resources during the crawl but were not themselves extracted into crawl files. They are candidates for a future expansion pass.

### Nix reference manual (external)

- **Nix manual** (`https://nixos.org/manual/nix/`) — the official Nix manual covering the Nix CLI, configuration, store, daemon, and language reference. Referenced by crawl 42 (`nix.dev/reference/nix-manual.html`) as an external link. Not crawled because it is a separate documentation site (nixos.org/manual) with its own structure; the nix.dev reference page serves as a pointer.

### Nixpkgs reference manual

- **Nixpkgs manual** (`https://nixos.org/manual/nixpkgs/`) — the official Nixpkgs manual covering `stdenv.mkDerivation`, build helpers, `pkgs.lib`, cross-compilation, and package conventions. The raw HTML was fetched and stored in `docs/nix/.crawl/_raw/nixpkgs-full.html` (2.8 MB) but was not extracted into individual crawl files. It is a separate, much larger corpus that would require its own crawl pass.

### NixOS manual

- **NixOS manual** (`https://nixos.org/manual/nixos/`) — the official NixOS manual covering NixOS configuration, modules, services, and system administration. Not crawled; it is a separate documentation site.

### nixos.org homepage and ecosystem

- **nixos.org** (`https://nixos.org/`) — the NixOS project homepage. The TOC fetch returned 404 (stored in `_raw/nixos-toc.html`). The homepage is a marketing/landing site, not reference documentation.
- **NixOS Wiki** (`https://nixos.wiki/`) — the community-maintained NixOS Wiki. Discovered via community references; not crawled. It is a community resource, not official primary documentation.
- **NixOS Discourse** (`https://discourse.nixos.org/`) — the official community forum. Referenced by crawl 49 (how-to-get-help). Not crawled; it is a discussion forum, not reference documentation.

### External tools referenced in tutorials

- **`nixos-anywhere`** (`https://github.com/nix-community/nixos-anywhere`) — referenced by crawl 22 (deploying NixOS using Terraform). Not crawled; it is a tool repository, not nix.dev documentation.
- **`disko`** (`https://github.com/nix-community/disko`) — referenced by crawl 22. Not crawled; tool repository.
- **`colmena`** (`https://github.com/zhaofengli/colmena`) — referenced by crawl 27 (provisioning remote machines). Not crawled; tool repository.
- **`deploy-rs`** (`https://github.com/serokell/deploy-rs`) — referenced by crawl 27. Not crawled; tool repository.
- **`nixos-shell`** (`https://github.com/Mic92/nixos-shell`) — referenced by crawl 26 (NixOS virtual machines). Not crawled; tool repository.
- **`npins`** (`https://github.com/andir/npins`) — referenced by crawl 35 (dependency management). Not crawled; tool repository.
- **`niv`** (`https://github.com/nmattia/niv`) — referenced by crawl 35 as the predecessor to npins. Not crawled; tool repository.
- **`cachix`** (`https://cachix.org/`) — referenced by crawl 34 (CI with GitHub Actions). Not crawled; commercial service.
- **`direnv`** (`https://direnv.net/`) — referenced by crawl 36. Not crawled; external tool documentation.
- **`nix-direnv`** (`https://github.com/nix-community/nix-direnv`) — referenced by crawl 36. Not crawled; tool repository.
- **`attic`** (`https://github.com/zhaofengli/attic`) — referenced by crawl 19 (binary cache setup). Not crawled; tool repository.
- **`harmonia`** (`https://github.com/nix-community/harmonia`) — referenced by crawl 19. Not crawled; tool repository.
- **`nix-serve`** (`https://github.com/edolstra/nix-serve`) — referenced by crawl 19. Not crawled; tool repository.
- **`poetry2nix`** (`https://github.com/nix-community/poetry2nix`) — referenced by crawl 38 (Python environment). Not crawled; tool repository.

## Skipped

The following categories of source were intentionally excluded from the crawl:

- **nixos.org homepage** (`https://nixos.org/`) — marketing/landing site; not reference documentation. The TOC fetch returned 404.
- **NixOS Wiki** (`https://nixos.wiki/`) — community-maintained wiki; not official primary documentation. May be useful as a secondary source in future.
- **NixOS Discourse forum** (`https://discourse.nixos.org/`) — discussion forum; context-only, not reference documentation.
- **GitHub source repositories** (`github.com/nixos/nix`, `github.com/nixos/nixpkgs`, `github.com/nix-community/...`) — source code repositories; not primary documentation. The nix.dev tutorials and guides already capture the usage surface. Individual tool repositories (nixos-anywhere, disko, colmena, etc.) were referenced but not crawled.
- **Commercial service homepages** (cachix.org, determinate.systems, floxdev.com, tweag.io, antithesis.com) — referenced in the acknowledgements (55) and CI guide (34) but are vendor homepages, not Nix documentation.
- **YouTube/video links** — not text reference documentation.
- **Blog posts and news** — context-only, not reference documentation.

## Verification method

All 55 pages were extracted from a local clone of the `github.com/nixos/nix.dev` repository at commit `139034be5e14320c05f792872e6150bd981490d5` (dated 2026-07-21). The extraction was performed by reading the MyST markdown source files from the repository's `src/` directory and persisting them as crawl ledger files in `docs/nix/.crawl/01-55`. Each crawl file carries:

1. **YAML frontmatter** with `type: Crawl Source`, `title`, `description`, `resource` (the canonical nix.dev URL), `tags`, and `timestamp`.
2. **A metadata header block** (blockquote) recording `seed_url`, `canonical_url`, `family`, `fetch` method (`cloned from github.com/nixos/nix.dev`), `version` (commit hash), and `feeds_docs` (currently `TBD` — to be populated when topic docs are written).

The page enumeration was verified against the nix.dev mdBook sidebar TOC (`docs/nix/.crawl/_raw/nix-toc.html`), confirming that all pages listed in the site's navigation are represented in the crawl corpus. No redirects were encountered (all `seed_url` == `canonical_url`). The `feeds_docs` field is set to `TBD` for all 55 pages because the Nix topic docs have not yet been written; this field will be populated during the topic-doc authoring pass. Origin (seed vs discovered) is inferred from the MyST toctree structure and crawl order because the ledger headers do not carry an explicit origin field.
