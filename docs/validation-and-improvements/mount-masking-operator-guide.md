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

The **compiler, config wiring, validation, and diagnostics CLI work today**,
and the runtime enforcement channel is wired through the pinned microsandbox
fork (see `flake.nix`: fork rev `3bd051bf`). One caveat: the pinned evaluator
treats `write.allow` as **inert** — write admission at this pin is
write.deny-only (see "The write axis" below). There is no KVM
in this container, so runtime enforcement is also unverified here.

## Configuring `[policy.mounts]`

A fragment is a TOML table named `[policy.mounts]` (or
`[workloads.<name>.policy.mounts]` at the workload scope, or an inline `policy`
table on a `[[workloads.<name>.mounts]]` entry). It has two axis sub-tables —
`read` (visibility) and `write` (write admission) — plus one program flag:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `read.deny` | list of patterns | `[]` | Hide matching paths (fail-closed direction). |
| `read.allow` | list of patterns | `[]` | Re-expose paths a lower scope denied. |
| `write.allow` | list of patterns | `[]` | Allow writes at matching paths. |
| `write.deny` | list of patterns | `[]` | Deny writes at matching paths (beats `allow`). |
| `case_sensitivity` | `"sensitive"` | `"sensitive"` | v1 accepts only `"sensitive"`; `"insensitive"` is reserved for a future version. |

Each list entry is either a **compact string** or an **expanded table**:

```toml
# compact (final defaults to false)
[policy.mounts.read]
deny = ["**/.env", "**/*.pem"]
```

```toml
# expanded: the final flag freezes the entry against later scopes
[policy.mounts.read]
deny = [
  "**/.env",
  { pattern = "**/secrets/**", final = true },
]
```

`final = true` makes the entry **final**: its match freezes the
decision for that path — a later scope cannot change it. Final ALLOWS
(`read.allow` / `write.allow`) are operator-only (see Trust model below);
final denies are accepted from any scope.

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
scopes). Within a scope, `deny` entries are applied before `allow` entries on
each axis.

| # | Scope | Where it lives | Operator? |
|---|---|---|---|
| 1 | Home registry | home `config.toml` `[policy.mounts]` | yes |
| 2 | User-global overrides | `overrides.toml` `[policy.mounts]` | yes |
| 3 | Reference config | `config.reference/workestrate.toml` `[policy.mounts]` | no (ships defaults) |
| 4 | Config-repo layer | a repo layer's `[policy.mounts]` | no |
| 5 | Workload | `[workloads.<name>.policy.mounts]` | no |
| 6 | Mount entry | `[[workloads.<name>.mounts]]` `policy = { ... }` or the `read.*`/`write.*` row sugar (declaring layer only, v1) | no |

**Trust model:** only operator scopes (1–2) may declare final ALLOWS
(`read.allow` / `write.allow`). Any scope may declare a final DENY (denying is
the fail-closed direction). An operator scope's final `read.deny` additionally
routes to the protect tier (see below). This preserves the monotonic-deny
posture: an untrusted repo cannot permanently reopen a path an operator
expects closable.

## Sensitive defaults (shipped)

The reference config ships these lowest-scope read denies, all non-final
(the default) so any config-repo or workload layer can carve out exceptions
with `read.allow`:

```toml
[policy.mounts.read]
deny = [
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

The reference config intentionally declares nothing final and nothing
protected — protection is an operator-only tier declared in the home registry
or user-global overrides (a final `read.deny` there compiles to the protect
wire bucket).

## Semantics operators need to know

- **Allow+tag default writes:** when no write rule matches, the guest may write
  the real file (there is one filesystem, no copy); the write is tagged with an
  alias so the host can identify it. `write.deny` forbids writes; `write.allow`
  re-permits them. Deny beats allow on overlap.
- **Protect tier:** protection is independent of the finality axis. A protected
  path is hidden and cannot be written, regardless of write rules. It is
  declared as a final `read.deny` from an operator scope (home registry or
  user-global overrides) — only those entries compile to the protect wire
  bucket.
- **Traversal-only:** a masked directory that contains an unmasked descendant
  is shown in directory listings (so the guest can reach the descendant) but
  its own masked contents are filtered out.
- **Cascade:** when a masked directory is removed, descendant alias tags are
  evicted. Tags are alias-scoped, identity-pinned, and fail-safe.
- **Hardlinks limitation:** masking one hardlink does NOT mask the other.
  Policy is path-based; two paths sharing an inode are analyzed independently.
  The `preview` command warns when a masked path shares an inode with a visible
  path.

### The write axis

`write.deny` / `write.allow` are pattern-keyed write-admission rules. All
scopes' write rules are UNIONED into one program and evaluated in
authority-ascending order (operator scopes first); within a scope deny is
applied before allow; the last non-final matching rule wins; and when nothing
matches the default is allow (allow+tag). A final `write.deny` freezes the
denial against every later scope; a final `write.allow` is operator-only
(same trust gate as `read.allow`).

**Activation caveat:** the pinned runtime (microsandbox fork rev `3bd051bf`)
treats `write.allow` as INERT — at this pin, write admission enforces
`write.deny` (and protect) only. `write.allow` activates when the fork pin is
bumped after the fork's `feat/write-allow-semantics` branch lands on the
pinned rev. Authoring `write.allow` rules today is safe: they compile,
validate, and show up in `explain`/`preview`. The workestrate-side mirror
evaluator (`control/agentctl/src/mount_policy/program.rs` `decide_write`)
intentionally keeps the PINNED runtime's semantics and must be ported to the
union semantics at re-pin time (see the `microsandbox-fork` input comment in
`flake.nix`).

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
[policy.mounts.read]
deny = ["**/.env", "**/*.pem", "**/.git/**"]

[workloads.api]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["python", "-m", "http.server", "8080"]

# Workload-scope policy: deny the secrets tree, carve out a public cert.
[workloads.api.policy.mounts.read]
deny = ["secrets/**"]
allow = ["secrets/public.pem"]

[[workloads.api.mounts]]
host = "workspaces/api-state"
guest = "/data"
read_only = false

[[workloads.api.mounts]]
host = "config"
guest = "/config"
read_only = true
# Mount-entry policy (declaring layer only, v1): deny everything except the
# public template.
policy = { read.deny = ["**"], read.allow = ["template.toml"] }
```

### Protect + writes (operator scope, home registry)

```toml
# In the home registry config.toml — operator scope.
[policy.mounts.read]
# Protection is operator-only: a final read.deny here compiles to the protect
# wire bucket — hidden AND untouchable.
deny = [{ pattern = "**/admin/**", final = true }]

[policy.mounts.write]
# Deny writes to config; allow writes to the data dir.
deny = ["config/**"]
allow = ["data/**"]
```

A config-repo layer can then carve out a specific config file with
`read.allow` (non-final default), but cannot lift the protection or override
the operator's final deny.
