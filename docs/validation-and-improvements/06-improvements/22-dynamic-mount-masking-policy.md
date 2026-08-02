# 22 — Dynamic mount masking policy: hierarchical [policy.mounts] scopes, collect-and-compile, runtime program

> **STATUS: DESIGN-APPROVED (awaiting implementation; user decisions locked 2026-08-02)**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) ·
> [../../migration/50-decisions/0004-security-allowlist-policy-rs.md](../../migration/50-decisions/0004-security-allowlist-policy-rs.md) ·
> [../../migration/50-decisions/0005-security-aware-merge.md](../../migration/50-decisions/0005-security-aware-merge.md) ·
> [../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md](../../migration/50-decisions/0011-microsandbox-vendor-to-git-fork.md) ·
> [../../migration/50-decisions/0020-review-adjudications.md](../../migration/50-decisions/0020-review-adjudications.md) ·
> [../../migration/50-decisions/0028-policy-scopes-collect-and-compile.md](../../migration/50-decisions/0028-policy-scopes-collect-and-compile.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs + pure-Rust design (`verifiable-here`): the policy parser,
the compiler, the explain/preview diagnostics, and the JSON program
serialization are all container-verifiable. The one `HOST-KVM` surface is
microsandbox runtime enforcement (§14), implemented in a later spec/phase.

---

## Summary

Host→guest bind mounts (notably the `${CWD}` project-root bind) expose
host-repo secrets to sandboxed agents. Spec 01 designed a static
shadow-mount mechanism; this spec supersedes it as the PRIMARY plan with a
**dynamic mount masking policy**: a hierarchical `[policy.mounts]` config
surface whose fragments are **collected** per layer and **compiled** by a
dedicated compiler into a per-mount program that the nix-patched
`microsandbox-filesystem` enforces dynamically (post-boot masking, no staging
copy, no nested shadow mounts). The locked user decisions:

- **Collect-and-compile, never merge (LOCKED)** — policy fragments are
  collected per layer in stack order; a compiler owns precedence. `merge.rs`
  is untouched (§8, ADR 0028).
- **Overridable/freeze semantics (LOCKED)** — per-scope mask-then-unmask
  ordering; a terminal match freezes the decision per path; exact-duplicate
  same-scope mask+unmask for the same pattern is a compile error naming both
  origins (§4).
- **Sensitive defaults ship as lowest-scope overridable masks (LOCKED)** —
  `.workestrate/`, `.env`, `.env.*`, `.git/`, `*.pem`, `*.key`, `.sops.yaml`,
  `node_modules/.npmrc`-style entries in the reference config (§9).
- **Runtime transmission v1 (LOCKED)** — the compiled program is serialized
  as JSON, written to the state dir, injected as a read-only mount at a
  well-known guest path plus a `MSB_MOUNT_POLICY` env var, and read by the
  nix-patched `microsandbox-filesystem`. v2 (a proper `VolumeMount::Bind`
  field) is deferred to the upstream-PR track (§12).
- **Spec 01 is dispositioned to fallback** — kept as the degraded-mode/static
  fallback (the WP5 staging-copy essence survives); it will be DELETED if
  this spec lands successfully (§17).

---

## 1. Problem

Spec 01 ([01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md))
established the threat: three of the real workloads bind-mount the host
`${CWD}` read-write into the guest, and the project root contains secrets
(`.workestrate/`, `.env`, `.git/`, `*.pem`, `*.key`, `.sops.yaml`,
`node_modules/.npmrc`) an agent must not read or write. Spec 01's nested
shadow mounts have three structural limits this spec removes:

1. **Static.** Shadows are fixed at plan time; a path created after boot
   (e.g. an agent writing `.env.local`) is unmasked. Dynamic post-boot
   masking requires enforcement in the filesystem layer itself.
2. **Flat policy.** `exclude` lives on the mount entry only. There is no way
   for an operator (home registry, user-global overrides) to declare a
   mask that applies across all repos and projects, nor for repo/project
   scopes to widen visibility under operator control.
3. **No provenance.** A plan-time-expanded shadow has no memory of which
   layer declared it, so diagnostics cannot explain a decision and trust
   validation cannot distinguish operator from repo intent.

This spec defines the dynamic policy system. The enforcement contract with
the patched `microsandbox-filesystem` is summarized in §14 and implemented in
a later spec/phase; everything else — parsing, compilation, provenance,
diagnostics, transmission — is specified here in full.

---

## 2. Hierarchical `[policy.mounts]` scopes

Policy is declared in `[policy.mounts]` tables across the layer stack. The
scopes, in **compile order** (which IS the authority order — §5):

1. **Home Registry `config.toml`** — the operator's global policy.
2. **`overrides.toml`** (user-global overrides, ADR 0019).
3. **Reference config** — ships the sensitive defaults (§9).
4. **Config-repo layers** — in registry stack order.
5. **Workload-level** — `[workloads.<name>.policy.mounts]`.
6. **Mount-entry** — `[[workloads.<name>.mounts]]` entry policy (see §8.3
   for the v1 limitation).

**Authority order = compile order.** Operator scopes (home registry,
user-global overrides) come FIRST and outrank repo/project scopes: an
operator's terminal decision cannot be reversed by anything later in the
stack (§4). This inverts the naive "last layer wins" intuition — for policy,
first speaker wins when it speaks terminally, because the operator is the
highest authority.

---

## 3. PolicyValue: compact and expanded forms

Each entry in a scope's mask/unmask list is a `PolicyValue`:

- **Compact string form** — the pattern itself, overridable by default:
  `mask = [".env", ".git/"]`.
- **Expanded table form** — `{ pattern = "...", overridable = false }`;
  `overridable` defaults to `true` in compact form and must be explicit in
  expanded form to be `false`.

Parsing follows the **`deserialize_any` visitor idiom**, NOT serde
`#[serde(untagged)]`. The precedent is the `RawBinding` /
`RawBindingVisitor` pair in `control/agentctl/src/config/types.rs:188-256`
(the spec-16 env binding parser): a `RawPolicyValue` enum
(`Pattern(String)` / `Expanded(RawPolicyEntry)`) with a
`de::Visitor` impl whose `visit_str`/`visit_string` produce the compact
form and whose `visit_map` delegates to a
`#[serde(deny_unknown_fields)]` raw struct via
`de::value::MapAccessDeserializer::new(map)` — preserving verbatim
unknown-field errors for inline tables. `Deserialize for RawPolicyValue`
calls `deserializer.deserialize_any(RawPolicyValueVisitor)`. Untagged enums
were rejected for the same reason as in spec 13/16: they buffer and retry,
producing positionless, context-free error messages; the visitor idiom
produces errors that name the offending value and form.

---

## 4. Overridable / freeze semantics (LOCKED)

Each scope's lists are applied **mask-then-unmask within that scope** (a
scope's `unmask` entries are evaluated after its `mask` entries, so a scope
can carve out an exception to its own mask). Across scopes:

- **A terminal match freezes the decision for that path.** A rule with
  `overridable = false` that matches a path is the final word for that path:
  later scopes (in compile order) cannot change the decision, whether to
  re-mask an unmasked path or unmask a masked one. The rule is recorded as
  the `frozen_by` origin in diagnostics (§13).
- **Overridable matches are provisional.** A rule with `overridable = true`
  decides the path until a later scope's matching rule supersedes it.
- **Exact-duplicate same-scope conflict = compile error.** If ONE scope
  contains both a mask and an unmask for the SAME pattern string, the
  compiler rejects with an error **naming both origins** (layer, file,
  scope kind — §11). Same-scope mask-then-unmask ordering makes this a
  contradiction the author must resolve, not a precedence question.
- **Overlapping-glob conflicts are surfaced, not rejected.** Two
  non-identical globs whose match sets intersect (e.g. `*.pem` in one scope,
  `certs/*.pem` in another, disagreeing on mask/unmask) are legal and
  resolved by the normal precedence rules; the conflict is visible via
  `explain` (§13), which lists every matching rule and its effect.

---

## 5. Trust model

- **Authority order = compile order** (§2): operator scopes first.
- **Non-overridable (terminal) unmask is REJECTED from non-operator scopes
  at compile time.** A repo or project layer may not declare a terminal
  unmask — that would let an untrusted repo permanently reopen paths an
  operator expects maskable. Compile error, naming the origin.
- **Non-overridable mask is allowed from ANY scope.** Masking is the
  fail-closed direction; any layer may harden, terminally.

This is the ADR 0005 monotonic-deny posture restated for a collected (not
merged) policy: a less-trusted scope can tighten but never irreversibly
loosen.

---

## 6. Pattern dialect

Patterns are a **globset-wrapper newtype `Pattern`** that preserves the raw
string (for diagnostics and for the exact-duplicate check of §4).

- **Syntax:** `*` (within a segment), `?` (one char), `**` (across
  segments).
- **Trailing `/` marks dir-only** (e.g. `.git/` matches the directory, not a
  file named `.git`).
- **Root-anchored by default.** Patterns match from the mount root;
  `**/`-prefixed patterns float (match at any depth).
- **Rejected at compile time, with errors naming the origin:**
  - absolute patterns (leading `/`);
  - NUL bytes;
  - `..`-escaping patterns (any pattern that can resolve outside the mount
    root);
  - empty patterns.
- **Non-UTF-8 paths are fail-closed (masked).** A path that cannot be
  represented as UTF-8 cannot be matched against the pattern set, so it is
  masked by default.
- **Case sensitivity:** a `case_sensitivity` flag lives in the **compiled
  program** (not per-pattern). v1 has an explicit default =
  **case-sensitive**; the flag is recorded so the runtime's behavior is
  pinned by the program, not by runtime defaults.

---

## 7. Traversal-only: the third state

A directory that is ITSELF masked but contains an unmasked descendant
becomes **TraversalOnly**:

- **Lookup/readdir are restricted to what leads to unmasked descendants.**
  The directory is traversable (an unmasked child remains reachable) but its
  other contents are invisible — readdir omits everything except the path
  components leading to unmasked descendants; lookup of any other name
  returns ENOENT.
- **`may_unmask_descendant(dir)`** decides whether a masked directory needs
  TraversalOnly treatment at all, via **literal-prefix analysis**: the
  compiler extracts the fixed literal prefix of each unmask pattern; if
  `dir` is not a prefix of (or prefixed by) any such literal prefix, the
  answer is `false`. Patterns without a fixed prefix (e.g. `**/*.env`)
  fall back to a **conservative `true`** — the runtime treats the directory
  as potentially containing an unmaskable descendant and applies
  TraversalOnly restriction, which is always the safe direction.

The three states per path are therefore: **Visible**, **Masked**,
**TraversalOnly**.

---

## 8. Collect-and-compile architecture (LOCKED)

### 8.1 No merging of policy

There is **NO `merge.rs` merging of policy**. Policy fragments are
**collected per layer in stack order** and handed to a dedicated **compiler**
that owns precedence, freeze semantics, trust validation, and conflict
detection. ADR 0028 is the decision record; the rejected alternatives
(extend the merge algebra with a policy merge kind; merge all scopes into
global mask/unmask sets) and the reasons are recorded there.

Why merging is incompatible:

- **ADR 0020 Ruling 1 (arrays wholesale-replace) would destroy lower-scope
  rules.** If `[policy.mounts]` lists merged as arrays, a repo layer's
  `mask` list would replace the operator's, not extend it.
- **Merging destroys provenance.** A merged set has no per-layer origins;
  §11's `RuleOrigin` and the explain trace (§13) require them.

### 8.2 The `mounts` vec itself is untouched

The workload `mounts` vec stays **wholesale-replace per ADR 0020 Ruling 1**
(arrays): an override layer that replaces a mount row replaces the row. This
spec changes nothing about mount-row merge semantics — only the POLICY
fragments are collected.

### 8.3 Mount-entry policy: declaring layer only (v1 limitation)

**Mount-entry-level policy comes only from the declaring layer** — the layer
that declares the mount row. Because mount rows replace wholesale, a policy
fragment attached to a mount entry in a DIFFERENT layer than the row's
declaring layer has no row to attach to after replacement. This is a **v1
semantic limitation, documented explicitly**: operators and repo authors
must place mount-entry policy in the same layer as the mount entry. Scopes
1–5 (§2) are unaffected.

---

## 9. Sensitive defaults (LOCKED)

The **reference config ships the sensitive defaults as lowest-scope,
overridable mask rules** — they are the initial floor, overridable by any
later scope but present out of the box:

- `.workestrate/` — the tool home (registry, secret definitions, encrypted
  secrets, state).
- `.env`, `.env.*` — operator environment files.
- `.git/` — git credentials, hooks, config.
- `*.pem`, `*.key` — TLS / private keys.
- `.sops.yaml` — SOPS configuration.
- `node_modules/.npmrc`-style entries — package-manager credential files.

Because they are overridable and lowest-precedence, a trusted repo or
workload scope may unmask a specific path deliberately; because they ship in
the reference config, a bare setup is fail-closed by default.

---

## 10. `masked_writes = "deny"` only

The program carries a `masked_writes` field. **v1 accepts exactly
`masked_writes = "deny"`** — any other value is **rejected explicitly at
compile time** with an error naming the offending value. Writes to masked
paths fail (EACCES at the enforcement layer, §14).

A passthrough-write-only mode (writes allowed through the mask, invisible to
the host or discarded) is **deferred to its own future investigation** — it
is NOT part of this spec and no syntax for it is reserved beyond the
explicit rejection.

---

## 11. Provenance: `RuleOrigin`

Every compiled rule carries its origin into the program:

```text
RuleOrigin { layer, file, scope_kind }
```

- `layer` — the layer name (registry, overrides, reference, repo name,
  workload, mount entry).
- `file` — the provenance path (`<repo>#<relpath>` granularity per spec 17).
- `scope_kind` — which of the six scopes (§2) declared it.

Diagnostics (§13) and compile errors (§4, §5, §6) display origins — the
exact-duplicate conflict error names BOTH origins; the terminal-unmask trust
rejection names the offending origin; explain traces show the origin of
every matching rule including the `frozen_by` rule.

---

## 12. Runtime program transmission v1 (LOCKED)

The compiled program reaches the guest filesystem as follows:

1. The compiler **serializes the program as JSON**.
2. The JSON is **written to the state dir** (per-instance, under
   `$WORKESTRATE_HOME/state/`, atomic tmp+rename per the port-registry
   discipline).
3. The file is **injected as a read-only mount at a well-known guest path**,
   and the path is also passed via the **`MSB_MOUNT_POLICY` env var**.
4. The **nix-patched `microsandbox-filesystem`** reads the env var, loads the
   JSON program, and enforces it for the mount's lifetime.

**v2 — a proper `VolumeMount::Bind` field carrying the policy inline in the
sandbox config — is deferred to the upstream-PR track** (the ADR 0011
adjacency: the fork/PR machinery that already carries the agentd fix is the
same channel a `policy` field on the mount struct would travel). v1's
state-dir + env-var channel is a shim by design: it requires no upstream
API change, works with the current nix patch, and is immutable per mount
lifetime by construction (read-only mount, written before spawn).

---

## 13. Diagnostics: `explain` and `preview`

Two CLI surfaces, **both using the SAME compiler as runtime** — no separate
evaluation logic to drift:

### 13.1 `workestrate policy mounts explain <path>`

Shows the **full match trace** for one path:

- every matching rule, in compile order;
- each rule's effect (mask / unmask, overridable / terminal) and its origin
  (`RuleOrigin`, §11);
- whether the match is terminal and which rule it is **frozen_by**;
- the final decision (Visible / Masked / TraversalOnly).

### 13.2 `workestrate policy mounts preview <dir>`

**Walks a host directory** and annotates each entry with its compiled state:
**Visible / Masked / TraversalOnly**. Additionally **warns on hardlink
aliases** via `(dev, ino)` detection: when a masked file's (device, inode)
pair is also reachable under a visible name, preview warns — the policy
masks NAMES, not objects (§16a), and the operator should know an alias
exists.

---

## 14. Microsandbox enforcement contract (summary)

The enforcement half is implemented in a later spec/phase against the
nix-patched `microsandbox-filesystem`; the contract this spec's compiler
produces for is summarized here:

- **lookup → ENOENT** for masked paths.
- **readdir omits masked entries** (and non-qualifying entries of
  TraversalOnly directories, §7).
- **create / rename-destination → EACCES** at masked paths
  (`masked_writes = "deny"`, §10).
- **Dynamic post-boot masking** — the program applies continuously, not just
  at spawn; files created after boot are masked per the program.
- **Policy immutable per mount lifetime** — the program is fixed when the
  mount is established (read-only channel, §12); changing policy means
  recreating the sandbox.
- **Linux-first.**

`HOST-KVM` for all of the above.

---

## 15. Config surface

All examples are FRAGMENTS (no `schema_version`), marked `# spec-test: skip`
for defense-in-depth.

### 15.1 Compact form — operator scope (home registry `config.toml`)

```toml
# spec-test: skip
[policy.mounts]
mask = [".env", ".env.*", "*.pem", "*.key", ".git/"]
```

### 15.2 Expanded form — terminal operator mask

```toml
# spec-test: skip
[policy.mounts]
mask = [
  ".env",
  { pattern = ".workestrate/", overridable = false },
  { pattern = ".sops.yaml", overridable = false },
]
```

### 15.3 Reference-config sensitive defaults (lowest scope, overridable)

```toml
# spec-test: skip
[policy.mounts]
mask = [".workestrate/", ".env", ".env.*", ".git/", "*.pem", "*.key", ".sops.yaml", "node_modules/.npmrc"]
```

### 15.4 Repo scope — carve out an exception under an overridable mask

```toml
# spec-test: skip
[policy.mounts]
mask = ["docs/secrets/**"]
unmask = ["docs/secrets/README.md"]
```

### 15.5 Workload scope

```toml
# spec-test: skip
[workloads.pi.policy.mounts]
mask = ["fixtures/prod-data/"]
unmask = [".env.example"]
```

### 15.6 Mount-entry scope (declaring layer only, §8.3)

```toml
# spec-test: skip
[[workloads.pi.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false
policy.mask = ["node_modules/"]
```

---

## 16. Known design trade-offs

Recorded so operators and reviewers see the accepted costs:

- **(a) Hardlink name-vs-object masking.** The policy masks **names, not
  objects**: a hardlink under a visible name to a masked file's inode reads
  the same bytes. Mitigation: `preview`'s `(dev, ino)` alias warnings (§13.2)
  surface the alias to the operator; an adversary who can pre-create
  hardlinks on the host has already compromised the host.
- **(b) TraversalOnly getattr reveals directory metadata.** A TraversalOnly
  directory must answer `getattr` (it is traversed), so its existence and
  basic metadata (mode, mtime) are visible even though its contents are
  restricted. Accepted: hiding the directory entirely would break the
  unmasked descendants it exists to expose.
- **(c) Quota byte accounting still counts masked bytes.** The underlying
  filesystem's quota/usage accounting sees the whole tree; masked files
  still consume their bytes. Masking is a visibility control, not a space
  control.
- **(d) ~5s negative-dentry delay.** A newly allowed file (one whose path
  was previously negatively cached by the guest kernel's dentry cache) may
  take ~5 seconds to become visible after it would newly match an unmask.
  Accepted as a kernel-caching property, not a policy staleness bug.

---

## 17. Relation to spec 01 (disposition: fallback)

Spec 01 (nested shadow mounts) is the **SECONDARY/FALLBACK** plan:

- It is kept as the **degraded-mode / static fallback** — the WP5
  staging-copy essence survives as the mechanism of last resort when the
  patched-filesystem channel is unavailable.
- Its WP1–WP4 are **FROZEN** — do not start them.
- If this spec (22) lands successfully, **spec 01 will be DELETED**.

---

## 18. Cross-references

- **ADR 0028** — the collect-and-compile decision record (primary).
- **ADR 0020 Ruling 1** — arrays wholesale-replace: explicitly NOT amended;
  the `mounts` vec merge semantics are untouched (§8.2).
- **ADR 0005** — security-aware merge: this spec sits adjacent to that
  family; policy is COLLECTED, not merged, so 0005's merge rules simply
  never apply to policy fragments (the monotonic-deny posture is carried by
  the compiler's trust validation instead, §5).
- **ADR 0004** — security allowlist posture: the sensitive defaults (§9) are
  the config-data analogue of the `policy.rs` const family.
- **ADR 0011** — microsandbox fork/consumption: v1's transmission channel
  depends on the nix-patched `microsandbox-filesystem`; v2 rides the same
  upstream-PR track.
- **Spec 01** — the fallback (§17).
- **Spec 16 / `types.rs:188-256`** — the `RawBindingVisitor` precedent for
  the `PolicyValue` parser (§3).

---

## 19. Acceptance criteria

- [ ] `[policy.mounts]` parsed at all six scopes (§2); compile order =
      authority order, operator scopes first.
- [ ] `PolicyValue` compact + expanded forms parsed via a
      `deserialize_any` visitor following the `RawBindingVisitor` precedent;
      no `#[serde(untagged)]` (§3).
- [ ] Per-scope mask-then-unmask; terminal match freezes per path;
      exact-duplicate same-scope mask+unmask = compile error naming both
      origins; overlapping-glob conflicts surfaced via `explain` only (§4).
- [ ] Terminal unmask rejected from non-operator scopes at compile time;
      terminal mask allowed from any scope (§5).
- [ ] `Pattern` newtype preserves the raw string; `*`/`?`/`**`; trailing-`/`=
      dir-only; root-anchored default with `**/`-prefix floating; absolute /
      NUL / `..`-escaping / empty patterns rejected with origin-named
      errors; non-UTF-8 fail-closed; `case_sensitivity` in the compiled
      program, v1 default case-sensitive (§6).
- [ ] TraversalOnly third state + `may_unmask_descendant` literal-prefix
      analysis with conservative-`true` fallback (§7).
- [ ] Collect-and-compile: no `merge.rs` policy merging; `mounts` vec stays
      wholesale-replace; mount-entry policy from the declaring layer only,
      documented as a v1 limitation (§8).
- [ ] Sensitive defaults ship in the reference config as lowest-scope
      overridable masks (§9).
- [ ] `masked_writes` accepts exactly `"deny"`; any other value is an
      explicit compile error (§10).
- [ ] Every compiled rule carries `RuleOrigin { layer, file, scope_kind }`;
      diagnostics and errors display origins (§11).
- [ ] Transmission v1: JSON program → state dir → read-only guest mount +
      `MSB_MOUNT_POLICY` env var → nix-patched microsandbox-filesystem; v2
      `VolumeMount::Bind` field deferred to the upstream-PR track (§12).
- [ ] `workestrate policy mounts explain <path>` shows the full trace with
      origins and frozen_by; `workestrate policy mounts preview <dir>`
      annotates Visible/Masked/TraversalOnly and warns on hardlink aliases
      via `(dev, ino)`; both reuse the runtime compiler (§13).
- [ ] Enforcement contract (§14) handed to the later spec/phase: lookup
      ENOENT, readdir omission, create/rename-destination EACCES, dynamic
      post-boot masking, immutable per mount lifetime, Linux-first.
- [ ] Spec 01 banner flipped to SECONDARY/FALLBACK with WP1–WP4 frozen
      (§17).

**Key decision:** policy is collected per layer and compiled by a single
compiler that owns precedence, freeze, trust, and provenance — the merge
engine never sees policy, the operator's terminal decisions are per-path and
irreversible by lower-trust scopes, and the same compiler backs runtime,
explain, and preview.
