# Mount masking — operator guide

> Practical, operator-facing configuration reference for the dynamic mount
> masking policy (spec 22). For the internal handover, see
> [MOUNT-MASKING-HANDOVER.md](MOUNT-MASKING-HANDOVER.md); for the full spec,
> see
> [06-improvements/22-dynamic-mount-masking-policy.md](06-improvements/22-dynamic-mount-masking-policy.md).

## What mount masking does

Mount masking hides, protects, or restricts writes to paths inside a guest
mount — without copying the mount. The operator declares glob patterns in a
`[policy.mounts]` fragment at one of six hierarchical scopes; the compiler
collects every scope's fragment (never merges them) and compiles a single
ordered program that the runtime enforces for the mount's lifetime. A masked
path returns `ENOENT` to the guest and is omitted from directory listings; a
protected path is hidden AND untouchable; writes default to allow+tag (the
guest writes the real file, tagged with an alias).

## v1 status (important)

The **compiler, config wiring, validation, and diagnostics CLI work today**.
The **runtime SDK seam is dormant**: `apply_mount_policy` is a no-op pending
the microsandbox fork dependency switch (spec 23, DESIGN/DEFERRED). That means
you can author, compile, validate, and inspect policies now, but end-to-end
guest enforcement is not exercised until the dep switch lands. There is no KVM
in this container, so runtime enforcement is also unverified here.

## Configuring `[policy.mounts]`

A fragment is a TOML table named `[policy.mounts]` (or
`[workloads.<name>.policy.mounts]` at the workload scope, or an inline `policy`
table on a `[[workloads.<name>.mounts]]` entry). It has these fields:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `mask` | list of patterns | `[]` | Hide matching paths (fail-closed direction). |
| `unmask` | list of patterns | `[]` | Re-expose paths a lower scope masked. |
| `protect` | list of patterns | `[]` | Hide AND forbid writes; operator-only terminal tier. |
| `writes.allow` | list of patterns | `[]` | Allow writes at matching paths. |
| `writes.deny` | list of patterns | `[]` | Deny writes at matching paths (beats `allow`). |
| `case_sensitivity` | `"sensitive"` | `"sensitive"` | v1 accepts only `"sensitive"`; `"insensitive"` is reserved for a future version. |

Each list entry is either a **compact string** or an **expanded table**:

```toml
# compact
mask = ["**/.env", "**/*.pem"]

# expanded (overridable defaults to true)
mask = [
  "**/.env",
  { pattern = "**/secrets/**", overridable = false },
]
```

`overridable = false` makes the rule **terminal**: its match freezes the
decision for that path — a later scope cannot change it. Terminal `unmask` and
terminal `protect` are operator-only (see Trust model below).

### Pattern dialect

- `*` and `?` never cross `/`; `**` crosses directory boundaries.
- Patterns are root-anchored (match from the mount root) unless they start with
  `**/` (floating — match at any depth).
- A trailing `/` marks a pattern dir-only (`.git/` matches the directory, not a
  file named `.git`).
- Rejected at compile time: absolute paths (leading `/`), `..` components, NUL
  bytes, empty patterns.

## Scope hierarchy

Scopes are compiled in authority order (operator scopes outrank repo/workload
scopes). Within a scope, `mask` is applied before `unmask`.

| # | Scope | Where it lives | Operator? |
|---|---|---|---|
| 1 | Home registry | home `config.toml` `[policy.mounts]` | yes |
| 2 | User-global overrides | `overrides.toml` `[policy.mounts]` | yes |
| 3 | Reference config | `config.reference/workestrate.toml` `[policy.mounts]` | no (ships defaults) |
| 4 | Config-repo layer | a repo layer's `[policy.mounts]` | no |
| 5 | Workload | `[workloads.<name>.policy.mounts]` | no |
| 6 | Mount entry | `[[workloads.<name>.mounts]]` `policy = { ... }` (declaring layer only, v1) | no |

**Trust model:** only operator scopes (1–2) may declare terminal `unmask` or
`protect`. Any scope may declare a terminal `mask` (masking is the fail-closed
direction). This preserves the monotonic-deny posture: an untrusted repo cannot
permanently reopen a path an operator expects maskable.

## Sensitive defaults (shipped)

The reference config ships these lowest-scope masks, all `overridable = true`
so any config-repo or workload layer can carve out exceptions with `unmask`:

```toml
[policy.mounts]
mask = [
  "**/.env",
  "**/.env.*",
  "**/.ssh/**",
  "**/*.pem",
  "**/*.key",
  "**/.git/**",
  "**/.sops.yaml",
  "**/node_modules/.npmrc",
]
```

`protect` is intentionally NOT in the reference config — it is an operator-only
tier declared in the home registry or user-global overrides.

## Semantics operators need to know

- **Allow+tag default writes:** when no write rule matches, the guest may write
  the real file (there is one filesystem, no copy); the write is tagged with an
  alias so the host can identify it. `writes.deny` forbids writes; `writes.allow`
  re-permits them. Deny beats allow on overlap.
- **Protect tier:** `protect` is independent of `overridable`. A protected path
  is hidden and cannot be written, regardless of write rules. Only operator
  scopes may declare it.
- **Traversal-only:** a masked directory that contains an unmasked descendant
  is shown in directory listings (so the guest can reach the descendant) but
  its own masked contents are filtered out.
- **Cascade:** when a masked directory is removed, descendant alias tags are
  evicted. Tags are alias-scoped, identity-pinned, and fail-safe.
- **Hardlinks limitation:** masking one hardlink does NOT mask the other.
  Policy is path-based; two paths sharing an inode are analyzed independently.
  The `preview` command warns when a masked path shares an inode with a visible
  path.

## CLI: `workestrate policy mounts`

Two diagnostics subcommands (both accept the global `--json` flag):

### `explain` — one path's decision

```sh
workestrate policy mounts explain \
  --workload example-service \
  --mount /data \
  --path secrets/api.key
```

Output (human form):

```
workload: example-service
mount: /data -> workspaces/example-service-state
path: secrets/api.key
decision: Masked
write: Deny
frozen_by: layer 'personal' (config.toml, home-registry scope)
matches:
  rule #0: mask terminal=false frozen_out=false origin=layer 'personal' (...) pattern=**/*.key
parents:
  secrets: may_unmask_descendant=false
```

`decision` is one of `Visible`, `Masked`, `TraversalOnly`, or `Protected`.
`frozen_by` names the terminal rule that froze the decision (or `(none)`).
`matches` lists every matching rule in compile order with provenance.

### `preview` — the whole mount tree

```sh
workestrate policy mounts preview \
  --workload example-service \
  --mount /data \
  --root /path/to/mount/host/root
```

Walks the mount host tree and prints one row per path: the relative path, the
decision, and the write verdict. It also reports warnings when a masked path
shares an inode with a visible path (the hardlink limitation). `--root`
overrides the host-root resolution; omit it to resolve against the declaring
layer's content root.

## Worked examples

### Multi-mount workload with per-mount policy

```toml
schema_version = 1

# Sensitive defaults inherited by every workload (reference scope).
[policy.mounts]
mask = ["**/.env", "**/*.pem", "**/.git/**"]

[workloads.api]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["python", "-m", "http.server", "8080"]

# Workload-scope policy: mask secrets, unmask a public cert.
[workloads.api.policy.mounts]
mask = ["secrets/**"]
unmask = ["secrets/public.pem"]

[[workloads.api.mounts]]
host = "workspaces/api-state"
guest = "/data"
read_only = false

[[workloads.api.mounts]]
host = "config"
guest = "/config"
read_only = true
# Mount-entry policy (declaring layer only, v1): mask everything except the
# public template.
policy = { mask = ["**"], unmask = ["template.toml"] }
```

### Protect + writes (operator scope, home registry)

```toml
# In the home registry config.toml — operator scope.
[policy.mounts]
# Protect is operator-only: hidden AND untouchable.
protect = ["**/admin/**"]

[policy.mounts.writes]
# Deny writes to config; allow writes to the data dir.
deny = ["config/**"]
allow = ["data/**"]
```

A config-repo layer can then `unmask` a specific config file (overridable
default), but cannot unprotect it or override the operator's terminal deny.
