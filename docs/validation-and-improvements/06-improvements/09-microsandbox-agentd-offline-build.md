# 09 — microsandbox-filesystem agentd offline build (ADR 0011 carrier)

> **STATUS: PR PREPARED, READY TO OPEN (option 1: branch fix/filesystem-agentd-path-override @ bc7640b8 (amended 2026-08-01: review hardening) ready to force-push; issue/PR docs in .tmp/msb-upstream/ — force-push + open pending USER; option 2 fork-carrier REVERSED per ADR 0011 addendum 2026-07-30 — transient PR vehicle only; option 3 NEEDS-DEVSHELL + HOST-NIX)**
> **Effort:** option 1 = **M** (including upstream review latency);
> option 2 = REVERSED (not executed); option 3 = **M** (patch rewrite +
> re-validation)
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

### Option 1 — UPSTREAM FIX (preferred end-state) — PR HARDENED, READY TO OPEN

Contribute an MSB_HOME-based agentd check to `crates/filesystem/build.rs`
mirroring the SDK crate's own pattern (skip download when
`$MSB_HOME/bin/agentd` exists + installed-version match +
`cargo:rerun-if-env-changed=MSB_HOME`). Precedent: #704 merged the identical
pattern into the SDK crate, and #701/#713 show the maintainers are responsive
to download-robustness fixes. After an upstream release containing the fix,
delete the patch, `microsandbox-filesystem-patched.nix`, the
`_setup_vendor_link` devshell hook, the agentctl.nix vendor staging, and the
vendor-unlock/lock justfile recipes.

**PR PREPARED, READY TO OPEN (2026-08-01):** branch `fix/filesystem-agentd-path-override`
@ `bc7640b8` (amended 2026-08-01 — 8-point review hardening + tests; force-push pending USER) on `github.com/georgrybski/microsandbox` ready to push; PR drafted at
`.tmp/msb-upstream/PR.md` mirroring upstream #704 (the SDK-crate MSB_HOME
precedent) with the `var_os` opt-in refinement. Force-push + PR open pending USER.

Trade-off: gated on upstream responsiveness; until merge+release we stay on
the compensation machinery.

### Option 2 — INTERIM CARRIER (ADR 0011) — REVERSED (2026-07-30)

**REVERSED per the ADR 0011 addendum (2026-07-30).** The fork
(`github.com/georgrybski/microsandbox`) exists **only** as the transient
vehicle to open the upstream PR; it is **NOT** consumed as a dependency — a
consumed git-fork dependency is maintenance rot (rebase-per-release churn,
fork accumulation). The `.tmp/microsandbox-fork` 0.5.6+patch branch
(`448937d4`) is moot as a dependency source (reference only). The nix-side
patch machinery stays as the interim.

~~Original plan (superseded):~~ Fork `microsandbox` (or a minimal fork
carrying the patched sub-crate), apply the patch on the fork, point
`[patch.crates-io]` at the fork git URL, then delete
`nix/packages/microsandbox-filesystem-patched.nix` + the patch file + the
`_setup_vendor_link` devshell hook (`nix/devshells/default.nix:132-158`) +
the agentctl.nix vendor staging (`nix/packages/agentctl.nix:59-84`) +
justfile vendor-unlock/lock recipes (`justfile:155-174`).

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

Option 1 upstream PR (PR PREPARED, READY TO OPEN — branch ready @ `bc7640b8` (amended 2026-08-01 — 8-point review hardening + tests), force-push
pending USER) → upstream release carrying the fix → bump
the microsandbox pin + delete ALL compensation machinery (the patch file, the
patched derivation, the `_setup_vendor_link` devshell hook, the agentctl.nix
vendor staging + the `.cargo/config.toml` `[patch.crates-io]` path entry, the
vendor-unlock/lock justfile recipes, the `.gitignore` vendor line). The
interim nix-side patch stays until then. Option 3 runs as a SEPARATE,
combinable track when runtime validation bandwidth exists.

## Review hardening — EXECUTED (2026-08-01)

The 8-point review of the prepared upstream PR was executed on BOTH tracks
(fork branch and interim 0.5.6 nix patch). Fork branch
`fix/filesystem-agentd-path-override` amended to `bc7640b8` (force-push
pending USER — the PR is READY to open immediately after the force-push).
Interim patch rewritten to the same semantics and committed `daa5140`
(patched derivation rebuilt; all gates green — 574 tests).

The 8 review points, one-line verdicts:

1. `MSB_AGENTD_PATH` authoritative — checked BEFORE the workspace-local
   path — **DONE**.
2. Fail-loud panic on an invalid env path
   (`MSB_AGENTD_PATH does not point to an agentd file: {}`) — **DONE**.
3. `cargo:rerun-if-changed` emitted only POST-validation (no watch on a
   rejected path) — **DONE**.
4. Marker file `.agentd-from-msb-agentd-path` — set → copy + mark; unset +
   marker → remove dest + marker; unset + no marker → preserve dest —
   **DONE**.
5. `cargo:rerun-if-env-changed=MSB_AGENTD_PATH` scoped to the prebuilt cfg —
   **DONE**.
6. 6 build-script tests — run via `rustc --test` because cargo does not
   execute build-script tests (disclosed in PR.md) — **DONE**.
7. PR.md updated to match the hardened semantics — **DONE**.
8. PR.md unset-fingerprint wording corrected: "byte-for-byte identical when
   unset" → "on a clean build with the variable unset, artifact selection
   and download behavior remain unchanged" — **DONE**.

Final semantics: `MSB_AGENTD_PATH` set + valid → copy + write marker, skip
download; set + invalid → panic BEFORE any download (offline-verified);
unset + marker → remove dest + marker, fall through to default behavior;
unset + no marker → preserve dest.

**Adaptation beyond the review (both tracks hit it independently):**
read-only dest EACCES — `fs::copy` inherits the immutable Nix-store source's
read-only mode, so build-script reruns fail without remove-before-copy.
Fork: `copy_agentd` unlinks the dest first. 0.5.6 patch: explicit
`remove_file` before the copy.

**Known test-env artifact:** 3 `not(prebuilt)` lib tests fail with the
12-byte dummy staged agentd (ELF offset assertion) — they would fail
identically pre-amend; disclosed in PR.md.

## Acceptance criteria

- [ ] Spec 09 exists and is indexed in
  [00-index.md](00-index.md) + the [../README.md](../README.md) doc map +
  [../07-execution-order.md](../07-execution-order.md).
- [ ] [../../../SPEC.md](../../../SPEC.md) Phase 0b.6 backlog note points here.
- [ ] [../../migration/70-open-items.md](../../migration/70-open-items.md)
  ADR 0011 entry points here.
- [ ] **Option 2 — SUPERSEDED/REVERSED (2026-07-30):** fork-carrier NOT
  consumed as a dependency; the fork is a transient PR vehicle only — see the
  ADR 0011 addendum.
- [ ] **Option 1 executed:** upstream PR merged referencing #704 precedent
  (PR PREPARED/READY TO OPEN 2026-08-01 — branch `fix/filesystem-agentd-path-override`
  @ `bc7640b8` (amended 2026-08-01 — 8-point review hardening + tests), draft `.tmp/msb-upstream/PR.md`; force-push pending USER); released;
  machinery deleted;
  `control/agentctl/Cargo.toml` pin updated.
- [ ] **Option 3 executed:** `=0.6.8` pin; rewritten patch applies and build
  passes offline (HOST-NIX); `msb --version` match re-validated; mount-filtering
  spec 01 cross-checked for RESOLVE_BENEATH interaction.
- [ ] Never on the yanked 0.6.5.

## Cross-references

- [ADR 0011](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md)
  (vendor → git-fork dependency; Accepted 2026-07-18; **addendum 2026-07-30
  REVERSES the fork-carrier** — option 2 is superseded; the fork is a
  transient PR vehicle only).
- [../../../SPEC.md](../../../SPEC.md) Phase 0b.6 backlog note (lines 233–235).
- [../../migration/70-open-items.md](../../migration/70-open-items.md) ADR 0011
  deferral entry (lines 82–89).
- [../../migration/40-migration-process.md](../../migration/40-migration-process.md)
  step 0b.6 (table line 47; status lines 50–54, 79–80; file-impact lines
  187–190).
- [../07-execution-order.md](../07-execution-order.md) (improvements section).
- [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) — option 3
  cross-reference (0.6.6 RESOLVE_BENEATH + 0.5.8+ guest-write quotas).
