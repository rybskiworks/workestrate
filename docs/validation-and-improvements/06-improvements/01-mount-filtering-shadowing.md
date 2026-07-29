# 06.01 — Mount Filtering / Shadowing (Track A)

> **STATUS: SPEC (not yet implemented); Phase 0 spike is NEEDS-KVM**
> Prerequisites / see-also: [../README.md](../README.md) ·
> [../02-config-requirements.md](../02-config-requirements.md) ·
> [00-index.md](00-index.md) ·
> [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md) ·
> [../../migration/50-decisions/0003-config-purity-closed-vocabulary.md](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md) ·
> [../../migration/50-decisions/0004-security-allowlist-policy-rs.md](../../migration/50-decisions/0004-security-allowlist-policy-rs.md) ·
> [../../migration/50-decisions/0005-security-aware-merge.md](../../migration/50-decisions/0005-security-aware-merge.md) ·
> [../../migration/50-decisions/0014-trust-gated-project-config.md](../../migration/50-decisions/0014-trust-gated-project-config.md) ·
> [../../migration/50-decisions/0020-review-adjudications.md](../../migration/50-decisions/0020-review-adjudications.md) ·
> [../../migration/50-decisions/0021-instance-lifecycle-model.md](../../migration/50-decisions/0021-instance-lifecycle-model.md)

This is the full engineering specification for **Track A**: excluding sensitive
files and directories from host→guest bind mounts **without copying** — via
nested shadow mounts over the live bind. A contextless reader needs nothing
else to understand the problem, the mechanism, the config surface, the policy
integration, the phased plan, and the security tradeoffs.

> Every code citation below was verified by reading the file during this
> session. The codebase is on branch `migration/tool-model`. Environment
> markers follow the convention in
> [`01-current-state-and-prereqs.md`](../01-current-state-and-prereqs.md) §Environment
> markers: `verifiable-here` (cargo/TOML/golden, in-container), `HOST-NIX`
> (requires nix on host), `HOST-KVM` (requires KVM on host).

### Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). NOTE: cargo-linked gates are NOT runnable here — no `cc` linker; they run on the host (HOST-NIX devshell) |
| `HOST-NIX` | Requires `nix` on the host (e.g. `just generate-schema`). |
| `HOST-KVM` | Requires KVM on the host (e.g. a live sandbox). |

---

## 1. Problem statement

Three of the five real workloads bind-mount the host current working directory
read-write into the sandbox guest:

| Workload | Host | Guest | RW | Source |
|---|---|---|---|---|
| `pi` | `${CWD}` | `/work` | yes | `.workestrate/repos/personal/workestrate.toml:142-144` |
| `opencode` | `${CWD}` | `/workspace` | yes | `.workestrate/repos/personal/workestrate.toml:283-285` |
| `tempest` | `${CWD}` | `/work` | yes | `.workestrate/repos/personal/workestrate.toml:346-348` |

`${CWD}` resolves to the project root (`microsandbox/workload/validate.rs:110-140`,
see [../02-config-requirements.md](../02-config-requirements.md) §6). The
project root contains host-repo secrets that an agent must not read freely:

- `.workestrate/` — the tool home: `config.toml` (registry, trusted projects),
  `repos/personal/workestrate.toml` (secret definitions, host bindings),
  `secrets/` (encrypted secrets), `state/` (per-workload state).
- `.env`, `.env.*` — operator environment files with provider API keys.
- `.git/` — git credentials, hooks, config.
- `*.pem`, `*.key` — TLS private keys.
- `.sops.yaml` — SOPS configuration.
- `node_modules/` — may contain `.npmrc` with tokens, or postinstall-modified
  files.

The microsandbox SDK pinned at `control/agentctl/Cargo.toml:17`
(`microsandbox = { version = "=0.5.6", features = ["net"] }`) exposes **no
exclude API** on its mount builder. The current `MountPlan`
(`control/agentctl/src/microsandbox/plan.rs:89-93`) carries only `host`,
`guest`, and `read_only` — there is no field to express "mount this tree but
hide that subtree." This spec defines that capability.

---

## 2. Vendor API verification

The mount-builder API was verified by reading the pinned `microsandbox-0.5.6`
crate source in the cargo registry
(`~/.cargo/registry/src/.../microsandbox-0.5.6/lib/sandbox/types.rs`). The
vendored symlink at `control/agentctl/vendor/microsandbox-filesystem-0.5.6`
points to a Nix store path (`/nix/store/...-microsandbox-filesystem-patched-0.5.6`)
that is **absent in this container** (no nix; see
[`01-current-state-and-prereqs.md`](../01-current-state-and-prereqs.md) §Devshell
/ vendor facts). That vendor is the **FUSE passthrough backend**
(`microsandbox-filesystem`), a different crate from the **SDK crate**
(`microsandbox`) that exposes `MountBuilder`. The SDK crate is consumed from
crates.io, not vendored.

> **Note on the two crates:** `microsandbox-filesystem-0.5.6` (vendored, patched
> per ADR 0011) is the in-guest FUSE filesystem that enforces `readonly` at the
> virtiofs layer. `microsandbox-0.5.6` (from crates.io) is the SDK that builds
> the sandbox config including `MountBuilder`. The shadow-mount design in §3
> uses the SDK `MountBuilder` API; the `EROFS` enforcement comes from the FUSE
> backend's `readonly` path.

### 2.1 Verified `MountBuilder` API surface

From `microsandbox-0.5.6/lib/sandbox/types.rs`:

| Method | Line | Effect |
|---|---|---|
| `MountBuilder::new(guest_path)` | ~510 | Create a mount entry targeting a guest path; `MountKind::Unset` until specified. |
| `.bind(host: impl Into<PathBuf>)` | 528 | Bind-mount from a host path (`MountKind::Bind`). |
| `.tmpfs()` | 558 | Memory-backed mount (`MountKind::Tmpfs`); no host path. |
| `.readonly()` | 611 | **"Enforced both at the host (virtiofs server rejects writes) and guest (kernel returns `EROFS`)."** (verbatim doc comment) |
| `.noexec()` | ~625 | Block direct execution from this mount. |
| `.nosuid()` | ~635 | Ignore setuid/setgid. |
| `.nodev()` | ~640 | Ignore device files. |

### 2.2 Verified mount ordering

`SandboxBuilder::volume(guest_path, f)` (from
`microsandbox-0.5.6/lib/sandbox/builder.rs:679-690`) pushes each built mount
onto `self.config.mounts` in **call order**:

```rust
pub fn volume(mut self, guest_path: impl Into<String>,
              f: impl FnOnce(MountBuilder) -> MountBuilder) -> Self {
    match f(MountBuilder::new(guest_path)).build() {
        Ok(mount) => self.config.mounts.push(mount),
        Err(e) => { /* ... */ }
    }
    self
}
```

Order is preserved (a `Vec::push`). This is the property the nested-shadow
design depends on: the parent bind mount must be pushed **before** the shadow
mounts that overlay subtrees of it. The current `apply_plan_mounts`
(`control/agentctl/src/microsandbox/mounts.rs:108-126`) already iterates
`plan.mounts` in order and calls `b.volume(&m.guest, |v| v.bind(host)...)`,
so extending the plan with ordered shadow rows is structurally compatible.

### 2.3 The one unverifiable thing

Whether the microsandbox runtime **honors nested mount ordering** — i.e.
whether a later `volume()` call whose guest path is *inside* an earlier
`volume()` call's guest path correctly shadows the subtree — cannot be
confirmed without a live sandbox. This is a Linux mount-namespace property
(child mounts shadow parents), but the SDK/runtime may flatten, reject, or
reorder mounts. **Phase 0 (§7) is the HOST-KVM spike that verifies this.**

---

## 3. Mechanism design: nested shadow mounts

### 3.1 Core idea

After the parent bind mount of `${CWD}` (or any host tree) into the guest,
bind an **empty read-only** filesystem object over each excluded path *inside*
that guest tree:

| Excluded path type | Shadow kind | Effect |
|---|---|---|
| File (e.g. `.env`) | empty read-only **file** bind | Guest sees an empty file; writes return `EROFS`. |
| Directory (e.g. `.git/`) | empty read-only **dir** bind (or `tmpfs`, opt-in) | Guest sees an empty dir; writes return `EROFS` (dir bind) or succeed-then-discard (tmpfs). |

This is **zero-copy** (no staging copy of the tree), **live** (the bind is the
real host tree, not a snapshot), and **fail-loud** for the dir-bind case (a
write attempt hits `EROFS` and is immediately observable).

### 3.2 Ordering guarantee

Mounts are applied in `Vec::push` order (§2.2). The shadow design requires:

1. The **parent** bind mount is pushed first (e.g. `${CWD} → /work`).
2. Each **shadow** mount is pushed after, with a guest path *inside* the
   parent's guest path (e.g. `/work/.env`, `/work/.git`).

If the runtime honors nested mount ordering (Linux mount-namespace semantics),
the shadow occludes the host file/dir at that guest path. If it does not, the
shadow is silently ineffective — which is why Phase 0 must confirm this before
WP4 ships.

### 3.3 `EROFS` fail-loud vs silent-empty tradeoff

- **Empty read-only file/dir bind (default):** a guest process that tries to
  `cat /work/.env` sees an empty file (the secret is hidden); a process that
  tries `echo x > /work/.env` gets `EROFS` (write fails loudly). This is the
  preferred default because the failure is observable and debuggable.
- **`tmpfs` (opt-in):** a guest process can *write* to the shadowed path, but
  the writes are discarded on sandbox exit and are invisible to the host.
  Useful when an agent legitimately needs to create `.env` or `.git/` *inside*
  the sandbox for tooling that expects them, without touching host state. The
  tradeoff: writes succeed silently, so a misconfigured exclude is less
  obvious. `tmpfs` is therefore opt-in per shadow entry, never the default.

### 3.4 Interaction with `read_only` parent mounts

A parent bind mount may itself be `read_only = true` (e.g. odysseus's
`${WORKESTRATE_ODYSSEUS_BUILD} → /app` ro). Shadow mounts over a read-only
parent are still meaningful: they hide the *contents* of specific files/dirs
even when the whole tree is already read-only. The shadow's `readonly()` is
redundant for writes (the parent already denies them) but is still applied for
defense-in-depth and for the `tmpfs` case (where the parent being read-only
does not make the tmpfs read-only).

---

## 4. Config schema extension

This section is the authoritative spec for the new config fields. It matches
the requirements pin in [../02-config-requirements.md](../02-config-requirements.md)
§7.1 verbatim. All new fields are `#[serde(default)]` so existing configs parse
unchanged and the golden plan format is preserved (§4.4).

### 4.1 `exclude` — glob patterns on a mount entry

```toml
[[workloads.pi.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false
exclude = [".env", ".env.*", "*.pem", "*.key", ".git/", ".workestrate/"]
```

- **Type:** `Vec<String>` (default `[]`).
- **Semantics:** glob patterns matched against the **guest-relative** path
  within this mount's guest root. A pattern matches a file or directory if the
  path (relative to the mount guest root, `/`-separated) matches the glob.
  Matching paths are shadowed with an empty read-only file (for file matches)
  or empty read-only dir (for directory matches).
- **Glob engine:** `globset` crate (new dependency, §4.5). Patterns use
  `globset` syntax: `*` matches within a path segment, `**` matches across
  segments, `?` matches one char, `[abc]` character classes. A trailing `/`
  (e.g. `.git/`) forces directory-only matching.
- **Merge:** `exclude` is part of `MountPlan`, which is a REPLACE list-of-rows
  (ADR 0020 Ruling 1). An override layer that replaces a mount row replaces
  its `exclude` list wholesale — there is no per-pattern union. This is
  consistent with how `ports`, `seed_files`, and `local_build` already behave.

### 4.2 `[[mounts.shadow]]` — explicit shadow entries

```toml
[[workloads.pi.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false

  [[workloads.pi.mounts.shadow]]
  path = "/work/.env"
  kind = "empty-file"

  [[workloads.pi.mounts.shadow]]
  path = "/work/.git"
  kind = "tmpfs"
```

- **Type:** `Vec<ShadowMount>` (default `[]`).
- **`path`:** `String` — the **absolute guest path** to shadow. Must be inside
  the parent mount's guest path (validated at `validate-config`).
- **`kind`:** enum `"empty-file" | "empty-dir" | "tmpfs"` (closed vocabulary,
  ADR 0003-conformant).
  - `"empty-file"` — bind an empty read-only file over the guest path.
  - `"empty-dir"` — bind an empty read-only directory over the guest path.
  - `"tmpfs"` — mount a tmpfs over the guest path (writes allowed, discarded,
    invisible to host).
- **Merge:** part of `MountPlan` (REPLACE). Shadow entries are rows within the
  mount row; replacing the mount row replaces its shadows wholesale.
- **Relationship to `exclude`:** `exclude` is sugar for "generate
  `empty-dir`/`empty-file` shadow entries for every matching path at plan
  time." Explicit `[[mounts.shadow]]` entries are for cases where the operator
  wants `tmpfs` or needs to name a specific path that a glob cannot express.
  Both may appear on the same mount; the plan-time expander deduplicates by
  guest path (explicit `shadow` entries win over `exclude`-generated ones).

### 4.3 `allow_sensitive` — trust-gated opt-out

```toml
[[workloads.pi.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false
allow_sensitive = false   # default; explicit for documentation
```

- **Type:** `bool` (default `false`).
- **Semantics:** when `true`, the policy-layer `SENSITIVE_MOUNT_EXCLUDE_PATTERNS`
  (§5) are **not** auto-applied to this mount. When `false` (default), the
  policy patterns are applied post-merge at plan time.
- **Trust gate (ADR 0014 / ADR 0020 Ruling 3):** `allow_sensitive = true` is
  **only honored from trusted layers** — the reference config, registry config
  repos, user-global overrides, and trusted project/local layers (the same
  trust set that loads at all, per
  [../02-config-requirements.md](../02-config-requirements.md) §5). An
  **untrusted** layer setting `allow_sensitive = true` is a **hard error**
  (fail-closed), not a warning.

  **Justification for hard error over ignore-with-warning:** the entire purpose
  of `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` is to prevent an agent from reading
  host secrets via a `${CWD}` bind. An untrusted layer (e.g. a hostile
  `git clone` + `cd` that somehow bypassed the trust gate, or a future
  less-trusted layer source) attempting to disable secret exclusion is an
  attempted security weakening. ADR 0005 mandates monotonic deny — a
  less-trusted layer cannot weaken security. A warning would let the weakening
  proceed silently; a hard error fails closed and surfaces the attempt. This
  matches the existing precedent: `default_deny = false` from a non-entitled
  workload is a hard error, not a warning (`policy.rs:45`,
  [../02-config-requirements.md](../02-config-requirements.md) §3.2).

### 4.4 Golden plan format preservation

The `SandboxPlan` `Display` impl (`plan.rs:156-214`) currently renders mounts
as:

```
mount: /data:/mnt
mount: /cfg:/etc/cfg (ro)
```

(lines 190-193). The new fields (`exclude`, `shadow`, `allow_sensitive`) are
rendered **only when present** (conditional additive rendering), so a
`MountPlan` with no shadow fields produces byte-identical output to today.
The three synthetic golden-check workloads (`example-service`, `example-agent`,
`example-offensive` — see [`01-current-state-and-prereqs.md`](../01-current-state-and-prereqs.md)
§Golden-check scope) have no shadow fields, so `just golden-check`
(`justfile:86-91`) stays green **without regeneration** of the golden files.

Proposed additive render (only the lines that change; existing lines unchanged):

```
mount: /data:/mnt
mount: /cfg:/etc/cfg (ro)
  shadow: /cfg/secret.key (empty-file)
  shadow: /cfg/.git (empty-dir)
  exclude: *.pem, *.key
```

Shadow and exclude lines are omitted entirely when the lists are empty;
`allow_sensitive` is omitted when `false` (the default).

### 4.5 New Rust dependency: `globset`

`control/agentctl/Cargo.toml` gains:

```toml
globset = "0.4"
```

`globset` is a mature, widely-used crate (part of the ripgrep ecosystem) for
fast glob matching with `**` semantics. It has no native-code dependencies
(pure Rust), so it does not affect the Nix build or the `libcap_ng` build-input
chain ([`01-current-state-and-prereqs.md`](../01-current-state-and-prereqs.md)
§Devshell / vendor facts).

### 4.6 `MountPlan` struct extension (proposed)

The `MountPlan` struct (`plan.rs:89-93`) becomes:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MountPlan {
    pub host: String,
    pub guest: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub shadow: Vec<ShadowMount>,
    #[serde(default)]
    pub allow_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShadowMount {
    pub path: String,
    pub kind: ShadowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ShadowKind {
    EmptyFile,
    EmptyDir,
    Tmpfs,
}
```

> **Note on `read_only` default:** the current struct has `pub read_only: bool`
> with no `#[serde(default)]` — it is a required field today. Adding
> `#[serde(default)]` to `read_only` is a **backward-compatible** widening
> (absent → `false`), but is **not required** by this spec and is left as an
> optional ergonomics improvement. The new fields (`exclude`, `shadow`,
> `allow_sensitive`) are all `#[serde(default)]` as required.

Per ADR 0021 §8, the `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`
derive means the new fields automatically flow into
`schemas/workestrate.schema.json` via `just generate-schema`
(`justfile:97-101`, HOST-NIX). The schema-drift CI guard
([../02-config-requirements.md](../02-config-requirements.md) §8) will catch a
struct change that ships without a schema regen.

---

## 5. Policy integration

### 5.1 New const: `SENSITIVE_MOUNT_EXCLUDE_PATTERNS`

`control/agentctl/src/policy.rs` gains a new const alongside the existing
allowlists (`ALLOWED_EGRESS_HOSTS` at line 4, `SECRET_HOST_BINDINGS` at line 23,
`ALLOWED_PACKAGES` at line 34, `DEFAULT_DENY_FALSE_ENTITLEMENT` at line 45):

```rust
/// Core-defined sensitive-path exclude patterns. Applied POST-MERGE at plan
/// time to every mount with allow_sensitive = false (the default). A trusted
/// layer may set allow_sensitive = true to opt out (ADR 0014).
///
/// These are a RECOMMENDATION — the initial set covers the known
/// host-repo secret locations. Add patterns here as new secret locations
/// are identified; the const is the single source of truth.
pub const SENSITIVE_MOUNT_EXCLUDE_PATTERNS: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    ".git/",
    ".workestrate/",
    ".sops.yaml",
    ".sops.yml",
];
```

**Initial list rationale:**

| Pattern | Protects |
|---|---|
| `.env` | Operator env file with provider keys. |
| `.env.*` | Env variants (`.env.local`, `.env.production`). |
| `*.pem` | TLS certificates / private keys. |
| `*.key` | Private keys (OpenSSH, etc.). |
| `.git/` | Git credentials, hooks, config. |
| `.workestrate/` | Tool home: registry, secret definitions, encrypted secrets, state. |
| `.sops.yaml`, `.sops.yml` | SOPS configuration (may reference key paths). |

This list is marked as a **recommendation** (the doc comment says so); it is
the initial ceiling, expandable by editing `policy.rs` (a reviewed, versioned
core change — the Terraform provider model, ADR 0003).

### 5.2 Application point: POST-MERGE at plan time

The sensitive-mount exclude patterns are applied **after** the merge engine
produces the effective config, at **plan time** (when `SandboxPlan` is built
from the merged `WorkloadConfig`). This is the same enforcement point as the
egress allowlist and the secret-host-binding check
([../02-config-requirements.md](../02-config-requirements.md) §4.1: "Enforced
at validate-config, plan (fail-closed), and runtime").

**Why post-merge plan-time application preserves monotonicity (ADR 0005) and
keeps merge semantics untouched:**

1. **The merge engine is unchanged.** `merge.rs` continues to apply REPLACE to
   `mounts` (list-of-rows, ADR 0020 Ruling 1). The merge engine does not know
   about `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` — it never sees them. This avoids
   adding a new merge rule (which would complicate the already-subtle
   security-aware merge in `merge.rs:449-546`).
2. **The deny is monotonic by construction.** The patterns are applied *after*
   the last layer has spoken. No layer — trusted or untrusted — can *remove* a
   pattern from the post-merge application, because the patterns come from
   `policy.rs` (compiled-in core), not from config. The only opt-out is
   `allow_sensitive = true`, which is itself trust-gated (§4.3). This is
   structurally identical to how `ALLOWED_EGRESS_HOSTS` works: the merge
   engine unions egress rules, then plan-time validation rejects any egress
   rule whose host is not in the allowlist. The allowlist is monotonic because
   it is post-merge and core-defined.
3. **A less-trusted layer cannot weaken security.** An untrusted local layer
   that sets `allow_sensitive = true` gets a hard error (§4.3). An untrusted
   layer that tries to shadow *fewer* paths by replacing a mount row still
   gets the policy patterns applied post-merge (because `allow_sensitive`
   defaults to `false` and the untrusted layer cannot set it `true`). The
   net effect: the policy patterns are a floor that only trusted layers can
   raise.

This mirrors the existing precedent exactly: `default_deny` is monotonic-true
post-merge (`merge.rs:449-479`), and `DEFAULT_DENY_FALSE_ENTITLEMENT` is a
core entitlement checked at plan time (`policy.rs:45`). The sensitive-mount
patterns follow the same shape.

### 5.3 Plan-time expansion algorithm

When building `SandboxPlan` from a merged `WorkloadConfig`, for each `MountPlan`
in `plan.mounts`:

1. If `allow_sensitive == false` (default), prepend
   `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` to the mount's `exclude` list (dedup;
   policy patterns first, then config patterns).
2. Expand `exclude` globs against the host tree (the resolved `host` path) to
   produce a set of guest-relative paths to shadow. For each matching path:
   - If it is a file → generate a `ShadowMount { path, kind: EmptyFile }`.
   - If it is a directory → generate a `ShadowMount { path, kind: EmptyDir }`.
3. Merge generated shadows with explicit `shadow` entries (explicit wins on
   path collision; dedup by guest path).
4. Sort shadows by guest path depth (deepest first) so that a shadow over
   `/work/.git/config` is applied before a shadow over `/work/.git` — though
   in practice the runtime's nested-mount semantics handle ordering within a
   shared parent, and the sort is defensive.
5. The final `MountPlan` (with expanded shadows) is what `apply_plan_mounts`
   (`mounts.rs:108-126`) consumes. The shadow rows are emitted as additional
   `b.volume()` calls **after** the parent bind, in order.

---

## 6. Ergonomics: audit and dry-run views

### 6.1 Current CLI surface (verified)

The `Plan` action exists on both `ServiceAction` and `AgentAction`
(`control/agentctl/src/cli_actions.rs:56-60` and `:92-95`). It currently has
exactly one flag:

```rust
Plan {
    /// Add N to every HOST port in the displayed plan (mirrors --port-offset on up).
    #[arg(long, default_value_t = 0, value_name = "N")]
    port_offset: u16,
},
```

There is **no** `--show-mounts` or `--mount-ls` flag today.

### 6.2 Proposed additions (NOT YET IMPLEMENTED)

Two new flags on the `Plan` action, clearly marked as proposed:

| Flag | Purpose | Env |
|---|---|---|
| `--show-mounts` | Print the effective mount list (including policy- and config-generated shadows) with **provenance** (which layer/policy contributed each shadow). | `verifiable-here` |
| `--mount-ls` | Host-side dry-run: evaluate the `exclude` globs and `shadow` entries against the resolved host paths and print what the guest **would see** at each shadowed path (e.g. "`.env` → empty file (policy)", `.git/` → empty dir (config)"). No KVM needed — pure host-side glob evaluation. | `verifiable-here` |

**`--show-mounts`** output shape (proposed):

```
$ workestrate pi plan --show-mounts
mount: workspaces/pi-state:/data
mount: ${CWD}:/work
  shadow: /work/.env (empty-file) [policy:SENSITIVE_MOUNT_EXCLUDE_PATTERNS]
  shadow: /work/.env.* (empty-file) [policy:SENSITIVE_MOUNT_EXCLUDE_PATTERNS]
  shadow: /work/.git (empty-dir) [policy:SENSITIVE_MOUNT_EXCLUDE_PATTERNS]
  shadow: /work/.workestrate (empty-dir) [policy:SENSITIVE_MOUNT_EXCLUDE_PATTERNS]
  shadow: /work/.npmrc (empty-file) [config:exclude]
```

**`--mount-ls`** output shape (proposed):

```
$ workestrate pi plan --mount-ls
evaluating mounts for workload "pi" (host root: /home/node/Development/ai-workbench)
mount /work <- ${CWD} (/home/node/Development/ai-workbench)
  [policy] .env           -> empty-file (host file exists, 412 bytes)
  [policy] .env.*         -> (no host matches)
  [policy] .git/          -> empty-dir  (host dir exists, 47 entries)
  [policy] .workestrate/  -> empty-dir  (host dir exists, 8 entries)
  [config] .npmrc         -> empty-file (host file exists, 0 bytes)
```

These flags are **proposed** — they require extending the `Plan` enum variant
in `cli_actions.rs` and adding rendering logic. They are scoped as part of WP3
(§7).

---

## 7. Phased plan

### Phase 0 — KVM nested-mount-ordering spike

| | |
|---|---|
| **Scope** | Minimal Rust PoC: build a sandbox with a parent bind mount of a host dir, then a second `volume()` call binding an empty read-only file over a file inside the parent's guest path. Confirm the guest sees the empty file, not the host file. Then repeat with an empty dir and a tmpfs. |
| **Files touched** | A throwaway example binary or test (not committed to main); uses the `microsandbox` SDK directly. |
| **Gate** | Spike report documenting: (a) does nested ordering work? (b) does `readonly()` on the shadow produce `EROFS` on write? (c) does `tmpfs()` shadow correctly? If any answer is "no", WP5 (staging-copy fallback) is activated. |
| **Env** | `HOST-KVM` |
| **Effort** | S |

### WP1 — Schema + glob

| | |
|---|---|
| **Scope** | Extend `MountPlan` (`plan.rs:89-93`) with `exclude`, `shadow`, `allow_sensitive` fields (all `#[serde(default)]`). Add `ShadowMount` and `ShadowKind` types. Add `globset = "0.4"` to `Cargo.toml`. Implement glob compilation and matching. Regen `schemas/workestrate.schema.json` via `just generate-schema`. |
| **Files touched** | `control/agentctl/src/microsandbox/plan.rs`, `control/agentctl/Cargo.toml`, `schemas/workestrate.schema.json`, `control/agentctl/Cargo.lock`. |
| **Gate** | `just check` (compile + clippy + fmt), `just test` (unit tests for glob matching), `just schema-check` (schema drift), `just golden-check` (3 synthetic workloads stay green — no shadow fields, no output change). |
| **Env** | `verifiable-here` (schema regen is `HOST-NIX` per `justfile:95-96`, but the schema diff can be verified-here if pre-generated). |
| **Effort** | M |

### WP2 — Policy + trust

| | |
|---|---|
| **Scope** | Add `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` to `policy.rs`. Implement the trust gate for `allow_sensitive` (hard error from untrusted layers). Implement plan-time expansion (§5.3): prepend policy patterns, expand globs to shadow entries, merge with explicit shadows. |
| **Files touched** | `control/agentctl/src/policy.rs`, `control/agentctl/src/microsandbox/plan.rs` (or a new `mount_filter.rs`), `control/agentctl/src/config/` (trust-gate hook at plan time). |
| **Gate** | `just check`, `just test` (unit tests for: policy application, trust gating — untrusted `allow_sensitive=true` is hard error, trusted is honored, monotonic deny across layers). |
| **Env** | `verifiable-here` |
| **Effort** | M |

### WP3 — Render + audit

| | |
|---|---|
| **Scope** | Extend `SandboxPlan::Display` (`plan.rs:156-214`) with conditional additive rendering of `shadow`/`exclude`/`allow_sensitive` lines (only when present). Add `--show-mounts` and `--mount-ls` flags to the `Plan` action (`cli_actions.rs:56-60, 92-95`). Prove golden parity: the 3 synthetic workloads produce byte-identical output. |
| **Files touched** | `control/agentctl/src/microsandbox/plan.rs` (Display impl), `control/agentctl/src/cli_actions.rs`, `control/agentctl/src/main.rs` (flag wiring), golden files (unchanged — that's the proof). |
| **Gate** | `just golden-check` (green without regeneration), `just check`, `just test` (unit tests for Display render: with-shadows, without-shadows, allow-sensitive-true). |
| **Env** | `verifiable-here` |
| **Effort** | M |

### WP4 — Runtime shadows

| | |
|---|---|
| **Scope** | Extend `apply_plan_mounts` (`mounts.rs:108-126`) to emit shadow `volume()` calls after the parent bind, in order. For `empty-file`/`empty-dir`: create a temp empty file/dir on host and bind it read-only. For `tmpfs`: call `.tmpfs()`. Ensure ordering (parent first, shadows after, deepest-first within a parent). |
| **Files touched** | `control/agentctl/src/microsandbox/mounts.rs`, possibly `control/agentctl/src/microsandbox/runtime/run.rs`. |
| **Gate** | Phase 0 spike must have passed. Live KVM test: bring up `pi` with `${CWD} → /work` and confirm `/work/.env` is empty and read-only inside the guest. |
| **Env** | `HOST-KVM` |
| **Effort** | M |

### WP5 — Fallback staging-copy (conditional)

| | |
|---|---|
| **Scope** | **Only if Phase 0 spike fails** (nested ordering does not work). Copy the mount tree minus excluded paths into a state-dir staging location and bind-mount the staging copy instead of the live host tree. |
| **Files touched** | `control/agentctl/src/microsandbox/mounts.rs` (alternative `apply_plan_mounts` path). |
| **Gate** | Live KVM test confirming exclusions are absent from the guest. Document the perf/copy cost. |
| **Env** | `HOST-KVM` |
| **Effort** | L |

**Why WP5 is the fallback, not the default:** staging-copy defeats two of the
three design goals. It is **not zero-copy** (it duplicates the tree, which for
`${CWD}` can be gigabytes — `node_modules/` alone). It is **not live** (the
guest sees a snapshot; host edits after sandbox start are invisible, breaking
the agent-editing-host-code workflow that is the entire point of the `${CWD}`
bind). It is only acceptable if the zero-copy shadow mechanism is provably
unavailable. The perf/copy cost and the staleness are documented in the WP5
implementation so operators understand the tradeoff.

---

## 8. Security analysis

### 8.1 Threat model

**What shadows protect against:**

- An agent (pi/opencode/tempest) reading host-repo secrets (`.env`, `.git/`,
  `.workestrate/`, `*.pem`, `*.key`) via the `${CWD}` bind mount.
- An agent exfiltrating secrets by reading them into context and sending them
  to an egress host (the network policy limits egress, but defense-in-depth
  means the secret should not be *readable* in the first place).
- An agent *modifying* host secrets (writing a new `.env` with a stolen key,
  or adding a git hook) — the `EROFS` fail-loud default prevents this.

**What shadows do NOT protect against:**

| Threat | Why not | Mitigation |
|---|---|---|
| **Symlink escapes** | A shadow over `/work/.git` does not prevent a symlink at `/work/evil → /etc` from being followed out of the bind tree. | The microsandbox FUSE backend (`microsandbox-filesystem`) enforces path confinement; this is a property of the runtime, not the shadow design. Out of scope for Track A. |
| **`/proc` and `/sys` access** | Shadows only cover the bind-mounted tree. `/proc` and `/sys` inside the guest are governed by the sandbox runtime, not by mount shadows. | Runtime hardening (out of scope). |
| **Hardlinks** | A hardlink inside the bind tree to a file *outside* the excluded set could expose a secret's contents under a non-excluded name. | Hardlinks across the bind boundary are constrained by the FUSE backend. The `exclude` patterns cover known secret *locations*; a determined adversary who can create hardlinks before the sandbox starts has already compromised the host. |
| **Race before mount** | A secret readable in the brief window between sandbox start and shadow application. | The SDK applies all mounts before the guest starts; there is no window. (To be confirmed in Phase 0.) |

### 8.2 Interaction with `read_only` mounts

A `read_only = true` parent mount already denies all writes via `EROFS`
(`plan.rs:92`, `mounts.rs:118-122`, enforced by the FUSE backend per the
`readonly()` doc comment in §2.1). Shadows over a read-only parent add
**read-hiding** (the file contents are replaced with empty) on top of the
existing write-denial. The shadow's own `readonly()` is redundant for writes
but is applied for defense-in-depth and for the `tmpfs` case.

### 8.3 `EROFS` observability

When a guest process writes to a shadowed path (empty-file/empty-dir kind),
the write fails with `EROFS` (read-only filesystem). This is **immediately
observable** in the guest (the syscall returns `EISDIR`/`EROFS`, the process
gets a Python/Node/Rust error). This is the fail-loud property: a misconfigured
exclude that shadows a path the agent legitimately needs produces a visible
error, not a silent empty result. The `tmpfs` kind trades this observability
for write-permissiveness (writes succeed but are discarded) — hence `tmpfs` is
opt-in.

---

## 9. Test plan

### 9.1 Unit tests (`verifiable-here`)

| Test | Module | Asserts |
|---|---|---|
| `glob_exclude_matches_file` | `plan.rs` or `mount_filter.rs` | `.env` pattern matches guest path `.env`; `.env.*` matches `.env.local`; `*.pem` matches `cert.pem`; `.git/` matches dir `.git` but not file `.git`. |
| `glob_exclude_does_not_match_unrelated` | same | `.env` does not match `README.md`; `.git/` does not match `.github/`. |
| `policy_patterns_prepended_post_merge` | `policy.rs` or plan-time | A mount with `allow_sensitive=false` gets `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` prepended to its `exclude` list at plan time. |
| `allow_sensitive_true_from_trusted_layer` | trust gate | A trusted layer setting `allow_sensitive=true` is honored (policy patterns not applied). |
| `allow_sensitive_true_from_untrusted_layer_is_hard_error` | trust gate | An untrusted layer setting `allow_sensitive=true` produces a hard error, not a warning. |
| `monotonic_deny_untrusted_cannot_remove_policy_patterns` | plan-time | An untrusted layer that replaces a mount row with an empty `exclude` list still gets policy patterns applied (because `allow_sensitive` defaults false and untrusted cannot set it true). |
| `display_renders_shadows_when_present` | `plan.rs` Display | A `MountPlan` with shadow entries renders `shadow:` lines. |
| `display_omits_shadows_when_absent` | `plan.rs` Display | A `MountPlan` with no shadows renders identically to today (golden parity). |
| `display_renders_allow_sensitive_only_when_true` | `plan.rs` Display | `allow_sensitive=false` (default) produces no extra line; `true` produces an `allow_sensitive: true` line. |

### 9.2 Fixture-layer merge tests (`verifiable-here`)

Using `WORKESTRATE_CONFIG_DIR` (the dev/testing bypass,
[../02-config-requirements.md](../02-config-requirements.md) §3.1):

| Test | Setup | Asserts |
|---|---|---|
| `exclude_replaces_wholesale` | Base layer mount with `exclude=[".env"]`; override layer replaces mount with `exclude=[".git/"]`. | Effective `exclude` is `[".git/"]` (REPLACE, not union) — consistent with ADR 0020 Ruling 1. |
| `shadow_replaces_wholesale` | Base layer mount with a `shadow` entry; override replaces mount with different `shadow`. | Effective shadows are the override's only. |
| `policy_patterns_applied_to_replaced_mount` | Override layer replaces a mount row (clearing `exclude`). | Policy patterns still applied post-merge (monotonic deny). |

### 9.3 Golden parity (`verifiable-here`)

`just golden-check` (`justfile:86-91`) over the 3 synthetic workloads
(`example-service`, `example-agent`, `example-offensive`) must pass **without
regenerating** the golden files. This proves the conditional additive rendering
preserves the existing format. If any golden file changes, WP3 has a bug.

### 9.4 KVM spike script (`HOST-KVM`)

Phase 0 spike (§7) produces a script that:

1. Creates a host dir with a secret file (`.env` with `SECRET=xyz`).
2. Builds a sandbox binding that dir to `/work` (rw).
3. Adds a shadow: empty read-only file over `/work/.env`.
4. Starts the sandbox and runs `cat /work/.env` inside — asserts output is
   empty (not `SECRET=xyz`).
5. Runs `echo hacked > /work/.env` inside — asserts it fails with `EROFS`
   (or `EISDIR`/permission denied).
6. Repeats with an empty-dir shadow over a subdir and a `tmpfs` shadow.

This script is the gate for WP4. If it fails, WP5 is activated.

---

## Appendix: ADR cross-reference

| ADR | Topic | Relevance to Track A |
|---|---|---|
| 0003 | Config purity / closed vocabulary | `ShadowKind` is a closed enum (`empty-file`/`empty-dir`/`tmpfs`); `exclude` patterns are data (globs), not shell. No new executable logic in config. |
| 0004 | Security allowlist in `policy.rs` | `SENSITIVE_MOUNT_EXCLUDE_PATTERNS` is a new core-defined const, same shape as `ALLOWED_EGRESS_HOSTS`. |
| 0005 | Security-aware merge | Patterns applied post-merge (monotonic deny); `allow_sensitive` trust-gated (untrusted cannot weaken). |
| 0014 | Trust-gated project config | `allow_sensitive=true` only honored from trusted layers; untrusted → hard error. |
| 0020 | Review adjudications | Ruling 1: `mounts` stay REPLACE (list-of-rows); `exclude`/`shadow` are part of the row, replaced wholesale. Ruling 2: entitlement-before-monotonic pattern reused for `allow_sensitive` trust gate. |
| 0021 | Instance lifecycle model | §8: new `serde` fields flow into `schemas/workestrate.schema.json` via `just generate-schema`; schema-drift CI guard catches missing regen. |
