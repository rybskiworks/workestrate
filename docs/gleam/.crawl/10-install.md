# Crawl: getting-started/installing/
- seed_url: https://gleam.run/getting-started/installing/
- canonical_url: https://gleam.run/install
- family: Gleam official docs
- fetch: 200 (after redirect)
- gleam_version: not present on this page
- feeds_docs: project-structure-and-cli.md, overview.md

## Purpose
The seed URL `https://gleam.run/getting-started/installing/` is a legacy path that
the Gleam site redirects to the canonical install hub at `https://gleam.run/install`
(HTTP 200; the redirect target serves a 147-byte JS redirect stub from
`/getting-started/installing/` → `/install`, then `/install` serves the real page).

This page is an **OS chooser hub**, not a single set of install instructions. It asks
"What operating system do you use?" and branches to a per-OS subpage. The detailed
install methods, Erlang/OTP requirement, and version notes live on those subpages —
NOT on this hub page. This crawl records only what is present on the hub itself; the
per-OS pages are listed under Discovered links (crawl later).

NOTE on link 7: link 7 discovered `/install/` (trailing slash). The canonical og:url
declared by the page is `https://gleam.run/install` (no trailing slash). The
`/getting-started/installing/` seed and `/install/` both resolve to the same
`/install` resource. **Canonical = `https://gleam.run/install`** (og:url); the
trailing-slash form `/install/` is the served URL after redirect and is equivalent.

## Install methods (version managers, package managers, source, releases)
Not enumerated on this hub page. The hub only presents OS selection:
Linux, macOS, Windows, Android, FreeBSD, OpenBSD.

Each OS links to a dedicated subpage (see Discovered links) where the actual
methods (version managers, package managers, build from source, GitHub releases)
are documented. This page contains no version-manager, package-manager, source,
or release instructions itself.

## Erlang/OTP runtime requirement
Not stated on this hub page. The Gleam runtime requirement (Gleam compiles to
Erlang/OTP and needs an Erlang installation) and any OTP version requirement are
documented on the per-OS subpages, not here. No OTP version number appears on
this page.

## Strict rules
- Treat `https://gleam.run/install` as the canonical URL (per og:url).
- Do NOT treat `/getting-started/installing/` as a separate resource — it is a
  redirect stub.
- The trailing-slash `/install/` and no-slash `/install` are the same resource;
  prefer the no-slash og:url form for canonical records.
- This hub page is NOT a source of install commands; do not extract install
  instructions from it. Crawl the per-OS subpages for actual instructions.

## Verbatim quotes
- `<title>`: "Installing the Gleam Programming Language"
- meta description: "Get your computer ready for Gleam development"
- og:url: "https://gleam.run/install"
- Hero heading: "Installing Gleam"
- Hero subtitle: "What operating system do you use?"
- OS options (in order): "Linux", "macOS", "Windows", "Android", "FreeBSD", "OpenBSD"

## Version notes
- No Gleam version is mentioned on this page.
- No Erlang/OTP version requirement is mentioned on this page.
- Page is a static chooser; version-specific guidance is on per-OS subpages.

## Discovered links

### Relevant (crawl later)
Per-OS install subpages (each contains the actual install methods + OTP requirement):
1. https://gleam.run/install/linux
2. https://gleam.run/install/macos/gleam
3. https://gleam.run/install/windows/gleam
4. https://gleam.run/install/android/gleam
5. https://gleam.run/install/freebsd/gleam
6. https://gleam.run/install/openbsd/gleam

Other potentially relevant (lower priority):
- https://gleam.run/documentation  (Docs hub — may overlap with already-crawled doc pages)

### Skipped
- / (site root, nav)
- /news
- /community
- /sponsor
- https://packages.gleam.run
- /install (self / nav)
- https://github.com/gleam-lang
- https://discord.gg/Fm8Pwmy
- https://shop.gleam.run/en-gbp
- https://tour.gleam.run
- https://playground.gleam.run
- https://gleamweekly.com/
- /roadmap
- /case-studies
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md
