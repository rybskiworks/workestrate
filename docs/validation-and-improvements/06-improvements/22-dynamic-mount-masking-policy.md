# 22 — Dynamic mount masking policy: hierarchical [policy.mounts] scopes, collect-and-compile, runtime program

> **STATUS: DESIGN-AMENDED → LANDED (library/config/CLI landed on `feature/mount-masking` @ `894e1bc`; msb enforcement pending the microsandbox fork dependency switch; merge onto `migration/tool-model` PLANNED — see merge-readiness addendum 2026-08-13)**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) ·
> [../../migration/50-decisions/0004-security-allowlist-policy-rs.md](../../migration/50-decisions/0004-security-allowlist-policy-rs.md) ·
> [../../migration/50-decisions/0005-security-aware-merge.md](../../migration/50-decisions/0005-security-aware-merge.md) ·
> [../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md) ·
> [../../migration/50-decisions/0020-review-adjudications.md](../../migration/50-decisions/0020-review-adjudications.md) ·
> [../../migration/50-decisions/0029-policy-scopes-collect-and-compile.md](../../migration/50-decisions/0029-policy-scopes-collect-and-compile.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, and cargo-linked gates via `nix develop`). |
| `HOST-NIX` | Requires nix on the user's host for genuine host gates. |
| `HOST-KVM` | Requires KVM on the user's host. |

This spec is docs + pure-Rust design (`verifiable-here`): parsing, compilation,
diagnostics, and JSON serialization are container-verifiable. Microsandbox
runtime enforcement (§14) is a later `HOST-KVM` phase.

---

## Summary

Host→guest bind mounts, notably the `${CWD}` project-root bind, expose host
secrets to sandboxed agents. This spec supersedes spec 01 as the PRIMARY plan:
a hierarchical `[policy.mounts]` surface is collected per layer and compiled
into a per-mount program enforced dynamically by the nix-patched
`microsandbox-filesystem` (post-boot masking, no staging copy, no nested shadow
mounts). The locked decisions are:

- policy is collected, never merged; one compiler owns precedence, provenance,
  trust, and diagnostics (§8, ADR 0029);
- masks are a read boundary; a successful write at a mask-matching path is
  allowed and tags the inode (§10);
- pattern-keyed write rules default to allow+tag, with deny-over-allow and
  independent protected paths (§10);
- tags are alias-scoped, identity-pinned, bounded, and fail safe (§10.5);
- the compiled JSON travels through the host state dir and host mount spec,
  not a guest file or guest environment variable (§12);
- spec 01 remains the degraded static fallback (§17).

## 1. Problem

Spec 01 ([01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md))
established the threat: read-write project-root binds contain `.workestrate/`,
`.env`, `.git/`, `*.pem`, `*.key`, `.sops.yaml`, and package credentials.
Static shadows are fixed at plan time, have no hierarchical provenance, and do
not enforce post-boot policy. Dynamic enforcement in the filesystem layer is
required.

## 2. Hierarchical `[policy.mounts]` scopes

Policy is declared at these scopes, in compile order (and therefore authority
order): home registry `config.toml`, user-global `overrides.toml`, reference
config, config-repo layers, workload policy, and the declaring mount-entry
policy. Operator scopes come first and outrank repo/project scopes: an
operator terminal decision cannot be reversed later (§4).

## 3. PolicyValue: compact and expanded forms

Every `read.deny`, `read.allow`, `write.deny`, and `write.allow` item is a
`Vec<PolicyValue<Pattern>>` with the same compact/expanded, finality, and
provenance machinery. Compact strings are non-final by default; expanded
tables use `{ pattern = "...", final = true }` (`final` defaults to `false`
and must be explicit to be `true`). A config-surface `final = true` compiles
to the wire rule's `overridable = false` — the compiled-program JSON wire
format (§12, §15) is unchanged. Parsing uses the
`deserialize_any` visitor idiom and `#[serde(deny_unknown_fields)]`, following
the `RawBindingVisitor` precedent in `control/agentctl/src/config/types.rs:188-256`.

## 4. Final / freeze semantics (LOCKED)

Within a scope, deny rules are applied before allow rules on each axis
(per-scope deny-before-allow). A non-final match is provisional; a match with
`final = true` freezes its decision for that path. Later scopes cannot change
a frozen decision. An exact duplicate same-scope deny+allow pair for the same
raw pattern where at least one entry is final is a compile error naming both
origins — on BOTH axes (read and write); overlapping non-identical globs are
legal and surfaced by `explain` (§13). Finality is separate from the protect
bucket (§10.2): a final read.deny from a non-operator scope is an ordinary
terminal deny, NOT protection.

## 5. Trust model

Authority order is compile order. A final allow — on EITHER axis
(`read.allow` or `write.allow`) — is rejected from non-operator scopes at
compile time. Final denies (`read.deny` / `write.deny`) are allowed from any
scope: denying is the fail-closed direction. An OPERATOR scope's final
read.deny routes to the protect wire bucket (§10.2). This is the ADR 0005
monotonic-deny posture for collected policy.

## 6. Pattern dialect

Patterns use a globset-wrapper `Pattern` preserving the raw string. `*` is
within a segment, `?` is one character, and `**` crosses segments; trailing
`/` is dir-only; patterns are mount-root anchored unless prefixed with `**/`.
Absolute, empty, NUL-containing, and `..`-escaping patterns are rejected with
origin-named errors. Non-UTF-8 paths are masked fail-closed. The compiled
program records an explicit, case-sensitive v1 `case_sensitivity` default.

## 7. Traversal-only: the third state

A masked directory containing an unmasked descendant is `TraversalOnly`:
lookup/readdir expose only components leading to that descendant. The compiler
uses literal-prefix analysis for `may_unmask_descendant`; patterns without a
fixed prefix conservatively return true. The three states remain Visible,
Masked, and TraversalOnly. The read boundary in §10.1 applies to masked entries;
TraversalOnly is only the route needed for an allowed descendant.

## 8. Collect-and-compile architecture (LOCKED)

### 8.1 No merging of policy

There is no `merge.rs` policy merge. Fragments are collected per layer in stack
order and handed to a compiler owning precedence, freeze, trust validation,
conflicts, and provenance. ADR 0020 Ruling 1 still governs ordinary arrays.

### 8.2 The `mounts` vec itself is untouched

The workload `mounts` vec remains wholesale-replace per ADR 0020 Ruling 1.
Only policy fragments are collected.

### 8.3 Mount-entry policy: declaring layer only (v1 limitation)

Mount-entry policy comes only from the layer declaring the mount row. A policy
attached in another layer has no row after wholesale replacement. Scopes 1–5
are unaffected. The mount row also accepts the axis sub-tables as sugar
directly on the entry (`read.deny = [...]`, `read.allow = [...]`,
`write.deny = [...]`, `write.allow = [...]`); the sugar is parse-time
normalized into the row's `policy` fragment and concatenates with an explicit
`policy` table on the same row.

## 9. Sensitive defaults (LOCKED)

The reference config ships `.workestrate/`, `.env`, `.env.*`, `.git/`, `*.pem`,
`*.key`, `.sops.yaml`, and `node_modules/.npmrc`-style entries as lowest-scope,
non-final `read.deny` entries, relaxable via `read.allow` carve-outs from
later scopes. They remain read boundaries: the write-axis default when no
write rule matches is allow+tag. Protection (an operator scope's final
read.deny, §10.2) is a separate operator-only tier, not the default for
sensitive entries.

## 10. Write rules, protect tier, and tagging

### 10.1 Read boundary and default writes

Untagged masked entries return ENOENT for lookup, stat, readlink, and read-open;
readdir/readdirplus omit them. Masking is purely a read boundary. Create,
mkdir, mknod, symlink at a masked path are allowed whether occupied or not;
write-open (`O_WRONLY`, no `O_CREAT`) of an existing masked file is allowed.
Any successful write-ish operation at a mask-matching path tags the inode.
Tagged entries are fully visible and writable (including stat, list, rename,
and delete). There is one filesystem and no copy: guest writes hit the real
host files. Anti-exfiltration is the goal; mounting implies acceptance of
write risk, and blind-write-clobber is accepted.

### 10.2 Pattern-keyed write rules and protect

`[policy.mounts]` carries two axis sub-tables — `[policy.mounts.read]`
(visibility) and `[policy.mounts.write]` (write admission), each with `deny`
and `allow` lists:

```toml
# spec-test: skip
[policy.mounts.write]
allow = ["generated/**"]
deny = [".env", ".git/**"]
```

All lists are `Vec<PolicyValue<Pattern>>`, with compact/expanded forms,
final flags, and provenance. If no write rule matches, allow+tag is the
default. Deny beats allow on overlap; final deny freezes; protect beats
everything. Rules compound across scopes in the same authority order as read
rules. Protected entries are fully untouchable: reads return ENOENT,
readdir omits, create/write is denied (never tagged), unlink/rmdir and
rename-source return ENOENT, rename-destination returns ENOENT, and cascade is
blocked. The protect tier is declared as a final read.deny from an OPERATOR
scope: only those entries route to the protect wire bucket. A final read.deny
from any other scope is legal but compiles to an ordinary terminal deny
(visibility only); a final allow on either axis from a non-operator scope is
a compile error (§5). Protection and finality are independent concepts:
finality freezes precedence; protection removes the path entirely.

### 10.3 Consolidated semantics table

| Operation | Untagged masked | Tagged | Protected | Visible |
|---|---|---|---|---|
| read/stat/readlink/read-open | ENOENT | ✓ | ENOENT | ✓ |
| readdir/readdirplus | omit | include | omit | include |
| create + write | ✓ + tag | ✓ | deny | ✓ |
| unlink + rmdir | ENOENT | ✓ | ENOENT | ✓ |
| cascade | removes | blocks | blocks | normal |
| rename-source | ENOENT | ✓ | ENOENT | ✓ |

Rename from a masked source is always ENOENT (anti-laundering); it cannot rename
to a visible name. The destination is checked against write rules and protect.

### 10.4 Tag records and identity

Tags are alias-scoped records keyed by the parent synthetic inode and name
bytes, with retained identity: an `O_PATH` fd pins the host inode and prevents
inode-number reuse. Lookup revalidates identity before honoring a tag. A
visible-path write never tags. Hardlinks are a known name-level limitation
(§16): a visible alias can expose the object.

### 10.5 Tag lifecycle and bounds

Guest unlink/rmdir removes a tag. Rename invalidates it (never moves it;
recompute at the destination); rename-overwrite invalidates the target.
Inode forget/eviction removes tags, and mount destruction clears them. Every
eviction fails safe: a lost tag remasks, never exposes. The store is bounded at
about 10,000 LRU records (~0.5–2 MiB plus one `O_PATH` fd per tag); cap
exhaustion is counted. Restart loses all tags and therefore remasks. Xattr
persistence is a documented follow-up.

### 10.6 Cascade delete

Direct unlink/rmdir of an untagged masked entry returns ENOENT. On `do_rmdir`
with `ENOTEMPTY`, open the directory with `O_DIRECTORY|O_NOFOLLOW`, snapshot
children, and use an iterative work queue. Cascade only removes
untagged-masked entries: regular files unlink; masked directories recurse to a
maximum depth; eligible symlinks are deleted as entries and are never followed.
Protected, tagged, or visible leftovers, host additions during the cascade,
identity mismatch, and depth overflow return `ENOTEMPTY`, fail closed, and do
not leak names.

## 11. Provenance: `RuleOrigin`

Every compiled rule (read-axis deny/allow, protect-bucket, write-axis
allow/deny) carries `RuleOrigin { layer, file, scope_kind }`. Diagnostics name
origins, duplicate errors name both origins, and explain traces include all
matches and the `frozen_by` rule.

## 12. Runtime program transmission v1 (LOCKED)

The compiler atomically writes compiled policy JSON with restrictive
permissions to the host state directory. The SDK mount model carries an
optional policy path; the host converts it to a mount-spec keyed token (for
example `policy=<path>`); msb parses it; `PassthroughFs` loads and validates it
**before VM startup**, then keeps an immutable in-memory program for the mount
lifetime.

The loading contract is strict: the path is confined to the approved state
directory, symlinks and traversal are rejected, and the file is parsed once.
Malformed JSON or an unsupported/missing version fails closed and refuses
sandbox startup. Program JSON contains an explicit `"version": 1`; its
deserializer rejects missing or unsupported versions with a clear error.

The prior guest-file + `MSB_MOUNT_POLICY` environment channel is **INVALIDATED**.
`PassthroughFs` is constructed host-side before the guest exists, while
`LaunchConfig.env` is guest-only and cannot be its host loading channel.

## 13. Diagnostics: `explain` and `preview`

Both CLI surfaces use the runtime compiler. `workestrate policy mounts explain
<path>` shows every matching rule, effect, terminal flag, origin, frozen-by
decision, and final state. `preview <dir>` annotates Visible/Masked/
TraversalOnly and warns on hardlink aliases using `(dev, ino)` detection.
The current workestrate CLI uses explicit flags (`--workload`, `--mount`, and
`--path` for explain; `--root` for preview) rather than positional path operands.

## 14. Microsandbox enforcement contract (summary)

The later msb phase enforces:

- untagged masked lookup/stat/readlink/read-open → ENOENT; readdir omission;
- successful masked write-ish operations → allow and tag; visible writes do not
  tag; protected writes → deny;
- tag identity validation before visibility, with fail-safe eviction;
- direct masked delete → ENOENT and the §10.6 cascade contract;
- masked-source rename → ENOENT; destination rules and protection apply;
- dynamic post-boot policy, immutable program per mount, and one host filesystem;
- brokered traversal with the symlink contract (§14bis); Linux-first.

`HOST-KVM` applies to runtime enforcement.

## 14bis. Symlink contract

The guest sees symlinks as symlinks; `readlink` returns the raw target string,
an accepted name-level metadata leak. The broker never follows interior
symlinks: openat2 uses `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_MAGICLINKS`.
The guest resolves targets component-wise in the guest namespace; absolute
targets are guest-absolute and cannot escape to the host. Every component
lookup hits the policy gate, so a visible symlink to a masked target yields
ENOENT at target lookup. Tagging a symlink never unmasks its target and there
is no symlink laundering primitive (unlike the hardlink limitation). Root
symlink following is mount-root-only, default false, and not exposed by
workestrate. The pre-5.6 openat fallback is weaker and pre-existing. Cascade
never dereferences symlinks. Contract verified at microsandbox `d9b4d12e`.

## 15. Config surface

All examples are fragments (no `schema_version`) and retain `# spec-test: skip`
for defense-in-depth and `spec_examples_parse` discipline.

### 15.1 Operator scope: read denies, protection, and write rules

```toml
# spec-test: skip
[policy.mounts.read]
# The final entry is operator-scope, so it routes to the protect wire bucket
# (§5, §10.2); the compact entries stay ordinary relaxable denies.
deny = [".env", ".env.*", "*.pem", "*.key", ".git/", { pattern = ".workestrate/", final = true }]

[policy.mounts.write]
allow = ["generated/**"]
deny = [".env", ".git/**"]
```

### 15.2 Expanded rules and the `final` flag

```toml
# spec-test: skip
[policy.mounts.read]
# final = true freezes the entry against later scopes (§4). From an OPERATOR
# scope it also routes to the protect wire bucket; from any other scope it is
# an ordinary terminal deny (§5).
deny = [{ pattern = ".sops.yaml", final = true }]

[policy.mounts.write]
allow = ["reports/**"]
deny = [{ pattern = "reports/private/**", final = true }]
```

### 15.3 Reference-config sensitive defaults

```toml
# spec-test: skip
[policy.mounts.read]
deny = [".workestrate/", ".env", ".env.*", ".git/", "*.pem", "*.key", ".sops.yaml", "node_modules/.npmrc"]
```

### 15.4 Repo scope: deny and carve out an exception

```toml
# spec-test: skip
[policy.mounts.read]
deny = ["docs/secrets/**"]
allow = ["docs/secrets/README.md"]

[policy.mounts.write]
allow = ["docs/secrets/generated/**"]
deny = ["docs/secrets/**"]
```

### 15.5 Workload scope

```toml
# spec-test: skip
# Workload scopes may NOT protect (protection is an operator scope's final
# read.deny, §5); a workload can still freeze its own denies on both axes.
[workloads.example.policy.mounts.read]
deny = ["fixtures/prod-data/", { pattern = "fixtures/prod-data/locked/", final = true }]

[workloads.example.policy.mounts.write]
allow = ["fixtures/prod-data/output/**"]
deny = [{ pattern = "fixtures/prod-data/locked/**", final = true }]
```

### 15.6 Mount-entry scope (declaring layer only, §8.3)

```toml
# spec-test: skip
[[workloads.example.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false
read.deny = ["node_modules/"]
write.allow = ["build/**"]
write.deny = [".env"]
```

## 16. Known design trade-offs

Recorded accepted costs:

- **(a) Hardlink name-vs-object masking.** Visible hardlink aliases expose
  content; preview warns via `(dev, ino)` and v1 does not auto-mask aliases.
- **(b) TraversalOnly getattr reveals directory metadata.** This is required
  to reach unmasked descendants.
- **(c) Quota byte accounting still counts masked bytes.** Masking is not space
  control.
- **(d) ~5s negative-dentry delay.** This is a kernel-caching property.
- **(e) Blind-write-clobber.** Allowing writes to masked paths accepts the risk.
- **(f) Tag cap exhaustion.** The bounded store counts exhaustion and fails safe.
- **(g) Restart re-masks.** Tags are volatile until a future xattr-persistence
  design.
- **(h) Symlink name-level leak.** Raw `readlink` targets remain visible.
- **(i) Hardlink aliases.** Name masking is not object masking (§13.2).

## 17. Relation to spec 01 (disposition: fallback)

Spec 01 remains the SECONDARY/FALLBACK degraded static mechanism when the
patched-filesystem channel is unavailable. WP1–WP4 are frozen; if spec 22
lands successfully, spec 01 will be deleted.

### 17.1 Phased plan

The next msb enforcement commits include the tag store, identity checks,
write admission, cascade delete, anti-laundering rename, and protected-tier
handling together; they are not a write-denial-only slice.

## 18. Cross-references

- **ADR 0029** — collect-and-compile and the amended transmission decision.
- **ADR 0020 Ruling 1** — arrays wholesale-replace; ordinary mount semantics
  are untouched (§8.2).
- **ADR 0005** — security-aware merge adjacency; policy is collected, not
  merged.
- **ADR 0004** — security allowlist posture and sensitive defaults (§9).
- **ADR 0011** — microsandbox fork/consumption and enforcement channel.
- **Spec 01** — the fallback (§17).
- **Spec 16 / `types.rs:188-256`** — `RawBindingVisitor` precedent (§3).

## 19. Acceptance criteria

- [ ] All six scopes parse; collect order is authority order and operator scopes
      come first.
- [ ] Compact/expanded `PolicyValue` uses the `deserialize_any` visitor; no
      `#[serde(untagged)]`.
- [ ] Read-axis deny/allow freeze, duplicate errors on both axes,
      overlapping-glob diagnostics, provenance, and the final-allow trust
      gates work as specified.
- [ ] Pattern validation, non-UTF-8 fail-closed behavior, case sensitivity,
      and TraversalOnly analysis are implemented.
- [ ] Write-rule allow/deny/default/overlap/final behavior is implemented;
      no-match defaults to allow+tag and deny beats allow.
- [ ] Protect is fully untouchable and beats all rules; the protect bucket is
      reached only by an operator scope's final read.deny; protection is
      distinct from finality.
- [ ] Tags validate identity, evict fail-safe, enforce bounds/counters, and
      follow unlink, rename, overwrite, forget, destroy, and restart lifecycle.
- [ ] Cascade, anti-laundering rename, and symlink contract are enforced.
- [ ] JSON transmission uses the host state-dir → SDK mount model → keyed
      host mount-spec → msb → pre-start `PassthroughFs` path; version 1 and
      malformed/unsupported fail-closed loading are tested.
- [ ] `explain` and `preview` reuse the compiler and preview warns on hardlink
      aliases.
- [ ] Spec 01 remains SECONDARY/FALLBACK with WP1–WP4 frozen (§17).

**Key decision:** policy is collected per layer and compiled by one compiler;
masking is a read boundary, write rules default to allow+tag, protection is an
independent operator-controlled tier, and the host loads one immutable,
versioned program before the guest starts.


---

## Merge-readiness addendum (2026-08-13)

> Assessment by the cross-repo merge-readiness session (see
> `~/Development/agent-workbench/handovers/2026-08-13-mount-merge-readiness.md`).

- **Branch:** `feature/mount-masking` @ `894e1bc` = common history through
  `c45494b` + **exactly 24 spec-22/23 commits** (verified via `git merge-base`).
- **Target:** workestrate `migration/tool-model` @ `5c341ad` (59 commits past
  the same base on the prime-work lineage). Merge is NOT conflict-free:
  `git merge-tree --write-tree` reports **8 content conflicts** (agentctl
  Cargo.{lock,toml}, cli_actions.rs, commands/mod.rs, main.rs,
  microsandbox/mod.rs, 50-decisions/README.md, 00-index.md) plus **12
  auto-merged files that need semantic verification** (mounts.rs, plan.rs,
  types.rs, validation.rs, run.rs, schema JSONs, …).
- **ADR 0028 collision:** this branch carries
  `0029-policy-scopes-collect-and-compile.md` while the target carries
  `0028-location-independent-execution.md` (same number, unrelated topics).
  Resolution plan: renumber this ADR → **0029** during the merge and update its
  references (spec 22, spec 21, 50-decisions/README.md, 00-index.md, STATUS,
  NEXT-SESSION).
- **SDK seam:** the workestrate flake pin moved from `74919059` to fork rev
  `79dc8a19` containing the msb mount path policy field (Phase 3 of the
  runbook, landed 2026-08-28).
- **Tests:** committed on this branch (`tests/cmd_policy.rs` + policy suite);
  must be re-run against the merged tree (schema regen re-run included).
