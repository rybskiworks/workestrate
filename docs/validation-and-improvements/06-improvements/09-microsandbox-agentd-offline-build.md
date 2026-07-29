# 09 — microsandbox-filesystem agentd offline build (ADR 0011 carrier)

> **STATUS: READY-TO-EXECUTE (option 2 gated on fork push access; option 1
> gated on upstream responsiveness; option 3 NEEDS-DEVSHELL + HOST-NIX)**
> **Effort:** option 2 = **S** (once unblocked); option 1 = **M** (including
> upstream review latency); option 3 = **M** (patch rewrite + re-validation)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [../07-execution-order.md](../07-execution-order.md) ·
> [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) ·
> [../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md) ·
> [../../migration/70-open-items.md](../../migration/70-open-items.md) ·
> [../../../SPEC.md](../../../SPEC.md)

> Every citation below was verified against the working tree on branch
> `migration/tool-model` and against upstream sources (crates.io API +
> raw.githubusercontent.com at pinned tags) during the authoring session
> (2026-07-29). Commands are spelled so a later contextless session can
> execute top-to-bottom without re-derivation.

## Summary

`microsandbox-filesystem` 0.5.6 performs a build-time network download of the
prebuilt `agentd` binary, breaking sandboxed/offline builds (Nix/Bazel/distro/
air-gapped CI). This spec is the single source of truth for the validated
improvement: root cause, current compensation machinery, upstream research,
three options, and a recommended sequence.

## Root cause

- `control/agentctl/Cargo.toml:17` pins
  `microsandbox = { version = "=0.5.6", features = ["net"] }`. The transitive
  dep `microsandbox-filesystem` 0.5.6's `crates/filesystem/build.rs` (default
  `prebuilt` feature) performs a build-time HTTP GET (ureq) of `agentd-x86_64`
  from GitHub releases — compile-time network access, breaking sandboxed/offline
  builds (Nix/Bazel/distro/air-gapped CI). At 0.5.6 the only escapes are
  `dest.exists()` in `OUT_DIR` and a CI/GITHUB_ACTIONS gate copying
  `<workspace_root>/build/agentd`.
- The SDK crate HAS the standard escape but it was never ported to the
  filesystem sub-crate: at v0.5.6 `crates/microsandbox/build.rs` (NOTE: at
  0.5.6 it's `crates/microsandbox/`, NOT `sdk/rust/` — `sdk/rust/build.rs`
  exists only on current main, same logic) skips downloading msb/libkrunfw
  when they exist under `$MSB_HOME`, with `cargo:rerun-if-env-changed=MSB_HOME`
  and an installed-version match check. Frame this as upstream
  inconsistency/oversight, not philosophy.

## Current compensation inventory (file:line, verified)

- `nix/packages/microsandbox-filesystem-agentd.patch` — 14-line insertion into
  build.rs: if `$MSB_HOME/bin/agentd` exists, copy it, skip download.
- `nix/packages/microsandbox-filesystem-patched.nix` —
  `fetchCrate(0.5.6, `sha256-Y2jZNhV1OCCs30JwtYy+cIdLHUMThOiJMjI6JCKj3hU=`)`
  + patch.
- `nix/devshells/default.nix:132-158` — `_setup_vendor_link` symlinks the
  patched derivation to `control/agentctl/vendor/microsandbox-filesystem-0.5.6`
  on every `nix develop`; consumed via `[patch.crates-io]` in
  `control/agentctl/.cargo/config.toml:10-11`; `vendor/` gitignored
  (`.gitignore:40`).
- `nix/packages/agentctl.nix:59-84` — preBuild recreates the symlink + a
  generated `.cargo/config.toml` and stages a temp `$MSB_HOME` with msb,
  agentd (copied from `${microsandbox}/libexec/agentd`, `:76`), `libkrunfw.so*`.
- agentd itself comes from `nix/packages/microsandbox.nix:12-13` which
  fetchurls the PREBUILT `agentd-x86_64` (hash-pinned fixed-output —
  sandbox-legal), installed to `$out/libexec/agentd` (`:71`). **CORRECTION:**
  the raised issue said "builds from source" — that was REFUTED; it's
  fetchurl.
- Editing workflow: `justfile:155-174` vendor-unlock/vendor-lock;
  `README.md:647-651` dangling-symlink-after-GC troubleshooting entry.

## Upstream research (verified 2026-07-29)

- crates.io `microsandbox-filesystem` history since 0.5.6: 0.5.7 (2026-06-14),
  0.5.8 (06-22), 0.5.10 (06-24), 0.6.0 (06-27) → 0.6.4, 0.6.5 (YANKED ~4h
  after publish), 0.6.6, 0.6.7 (07-25), 0.6.8 (07-29, latest). **CORRECTION to
  the raised issue:** 0.5.9 NEVER EXISTED on crates.io (404) — it was not
  yanked, it never existed. The `microsandbox` crate follows the same cadence.
  Canonical repo: github.com/superradcompany/microsandbox. Cite
  `https://crates.io/crates/microsandbox-filesystem/versions`.
- v0.6.0 (PR #1019 "Windows support", merge commit `dfbe947`, merged
  2026-06-26) replaced the CI gate with an unconditional workspace-relative
  override `CARGO_MANIFEST_DIR/../..` + `copy_agentd()` helper — for crates.io
  consumers this resolves into `~/.cargo/registry` and NEVER fires, so the
  download always runs. build.rs is byte-identical v0.6.0 → v0.6.8 → main;
  still no MSB_HOME/env-var path. Cite
  `https://github.com/superradcompany/microsandbox/pull/1019` and the raw file
  at the pinned tag
  `https://raw.githubusercontent.com/superradcompany/microsandbox/v0.6.8/crates/filesystem/build.rs`.
- The 0.5.6 patch will NOT apply to 0.6.x (CI-gate anchor deleted, helper
  extracted); a rewrite inserts immediately before the new
  `if local.is_file() { copy_agentd(&local, &dest); return; }` branch.
- **CORRECTION to the raised issue's "no upstream issue/PR" claim (REFUTED —
  this strengthens option 1):** #701 (issue: agentd download fails behind
  TLS-intercepting proxies/custom CAs), #713 (merged: trust system certs for
  prebuilt downloads), #704 (merged: "honor MSB_HOME in prebuilt download
  path" — SDK crate only; THE precedent for option 1). No Nix/hermetic-specific
  tracker item exists. Cite
  `https://github.com/superradcompany/microsandbox/issues/701`, `/pull/713`,
  `/pull/704`.

## Options

### Option 1 — UPSTREAM FIX (preferred end-state)

Contribute an MSB_HOME-based agentd check to `crates/filesystem/build.rs`
mirroring the SDK crate's own pattern (skip download when
`$MSB_HOME/bin/agentd` exists + installed-version match +
`cargo:rerun-if-env-changed=MSB_HOME`). Precedent: #704 merged the identical
pattern into the SDK crate, and #701/#713 show the maintainers are responsive
to download-robustness fixes. After an upstream release containing the fix,
delete the patch, `microsandbox-filesystem-patched.nix`, the
`_setup_vendor_link` devshell hook, the agentctl.nix vendor staging, and the
vendor-unlock/lock justfile recipes.

Trade-off: gated on upstream responsiveness; until merge+release we stay on
the compensation machinery.

### Option 2 — INTERIM CARRIER (ADR 0011, already decided 2026-07-18)

Fork `microsandbox` (or a minimal fork carrying the patched sub-crate) to
`github:georgrybski/microsandbox-filesystem`, apply the patch on the fork,
point `[patch.crates-io]` at the fork git URL, then delete
`nix/packages/microsandbox-filesystem-patched.nix` + the patch file + the
`_setup_vendor_link` devshell hook (`nix/devshells/default.nix:132-158`) + the
agentctl.nix vendor staging (`nix/packages/agentctl.nix:59-84` adjusted) +
justfile vendor-unlock/lock recipes (`justfile:155-174`).

**PREREQUISITE/BLOCKER:** push access to create the fork
(`github:georgrybski/microsandbox-filesystem`) — previously recorded as
blocked.

Trade-offs: removes the devshell symlink machinery and makes the build
pure-eval friendly (the vendored crate currently blocks any pure-eval Nix
build claim — see
[../../migration/70-open-items.md](../../migration/70-open-items.md)); cost is
maintaining a fork pin; superseded by option 1 once upstream merges.

### Option 3 — VERSION BUMP (independent, combinable)

`=0.5.6` → `=0.6.8` in `control/agentctl/Cargo.toml:17`. Requires: (a)
rewriting the agentd patch against the 0.6.x build.rs (insertion point:
immediately before the new
`if local.is_file() { copy_agentd(&local, &dest); return; }` branch — the
0.5.6 patch's CI-gate anchor was deleted by PR #1019); (b) re-validating the
`msb --version` match check (see the comment at
`nix/packages/agentctl.nix:67-70`).

**SECURITY RATIONALE the raised issue missed:** 0.5.8+ added guest-write
quotas and 0.6.6 added RESOLVE_BENEATH symlink protection — both directly
relevant to [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md)
(cross-reference it explicitly). NEVER upgrade to the yanked 0.6.5.

Trade-offs: independent of options 1/2 and combinable with either; gated
NEEDS-DEVSHELL + HOST-NIX (patch rewrite + cargo build/test + runtime
`msb --version` re-validation).

## Recommended sequence

Option 2 first (once the fork push-access blocker clears) → option 1 upstream
contribution → after upstream release, delete the fork + all compensation
machinery. Option 3 runs as a SEPARATE track when runtime validation bandwidth
exists; it is combinable with 1 or 2.

## Acceptance criteria

- [ ] Spec 09 exists and is indexed in
  [00-index.md](00-index.md) + the [../README.md](../README.md) doc map +
  [../07-execution-order.md](../07-execution-order.md).
- [ ] [../../../SPEC.md](../../../SPEC.md) Phase 0b.6 backlog note points here.
- [ ] [../../migration/70-open-items.md](../../migration/70-open-items.md)
  ADR 0011 entry points here.
- [ ] **Option 2 executed:** `[patch.crates-io]` points at fork git URL;
  patch file + patched derivation + `_setup_vendor_link` + vendor staging +
  vendor-unlock/lock recipes deleted; `nix flake check` green (HOST-NIX).
- [ ] **Option 1 executed:** upstream PR merged referencing #704 precedent;
  released; machinery deleted; `control/agentctl/Cargo.toml` pin updated.
- [ ] **Option 3 executed:** `=0.6.8` pin; rewritten patch applies and build
  passes offline (HOST-NIX); `msb --version` match re-validated; mount-filtering
  spec 01 cross-checked for RESOLVE_BENEATH interaction.
- [ ] Never on the yanked 0.6.5.

## Cross-references

- [ADR 0011](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md)
  (vendor → git-fork dependency; Accepted 2026-07-18) — option 2 is its carrier.
- [../../../SPEC.md](../../../SPEC.md) Phase 0b.6 backlog note (lines 233–235).
- [../../migration/70-open-items.md](../../migration/70-open-items.md) ADR 0011
  deferral entry (lines 82–89).
- [../../migration/40-migration-process.md](../../migration/40-migration-process.md)
  step 0b.6 (table line 47; status lines 50–54, 79–80; file-impact lines
  187–190).
- [../07-execution-order.md](../07-execution-order.md) (improvements section).
- [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) — option 3
  cross-reference (0.6.6 RESOLVE_BENEATH + 0.5.8+ guest-write quotas).
