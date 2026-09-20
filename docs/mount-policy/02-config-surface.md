# Config surface — the TOML vocabulary

> **What this doc teaches:** every key you can write in a `[policy.mounts]`
> fragment, the two entry forms, the pattern dialect, the mount-row sugar, and
> the migration map from the retired vocabulary. **Read first:**
> [00-overview.md](./00-overview.md). What the rules DO at runtime is
> [01-runtime-semantics.md](./01-runtime-semantics.md); how scopes stack is
> [03-hierarchy-and-precedence.md](./03-hierarchy-and-precedence.md).

## The fragment shape

A policy fragment is a TOML table `[policy.mounts]` with two axis sub-tables
and one program flag (parse: workestrate
`control/agentctl/src/mount_policy/scope.rs:76-106`):

| Field | Type | Default | Meaning |
|---|---|---|---|
| `read.deny` | list of entries | `[]` | Hide matching paths from the guest. |
| `read.allow` | list of entries | `[]` | Re-expose paths a deny matched. |
| `write.deny` | list of entries | `[]` | Deny guest writes at matching paths. |
| `write.allow` | list of entries | `[]` | Allow guest writes at matching paths. |
| `case_sensitivity` | string | `"sensitive"` | v1 accepts exactly `"sensitive"`; anything else is a compile error naming origin and value (compile.rs:175-185). |

## Entry forms

Every list entry is either a **compact string** or an **expanded table**
(parse: workestrate `control/agentctl/src/mount_policy/value.rs:27-55`):

```toml
[policy.mounts.read]
deny = ["**/.env", "**/*.pem"]
```

```toml
[policy.mounts.read]
deny = [
  "**/.env",
  { pattern = "**/secrets/**", final = true },
]
```

`final` defaults to `false`. A **final** entry freezes its match: no
later-scope rule can change the decision for that path (on the wire it becomes
`overridable: false` — see
[04-compiler-and-wire.md](./04-compiler-and-wire.md)). Final ALLOWS on either
axis are operator-scope-only; final DENIES are accepted from any scope (trust
gate: compile.rs:237-250; see
[03-hierarchy-and-precedence.md](./03-hierarchy-and-precedence.md)).

## Every effect, minimally

**Hide paths (read.deny, compact):**

```toml
[policy.mounts.read]
deny = ["**/.env"]
```

**Carve out an exception (read.allow):**

```toml
[policy.mounts.read]
deny = ["**/.env", "**/.env.*"]
allow = ["**/.env.example"]
```

**Freeze a decision against later scopes (final entry):**

```toml
[policy.mounts.write]
deny = [{ pattern = "guard/**", final = true }]
```

**Deny writes (write.deny):**

```toml
[policy.mounts.write]
deny = ["config/**"]
```

**Permit writes (write.allow):**

```toml
[policy.mounts.write]
deny = ["**"]
allow = ["scratch/**"]
```

> **Activation caveat:** at the pinned runtime (fork rev `3bd051bf`),
> `write.allow` is INERT — write admission today enforces `write.deny` +
> protect only. The recipe above only denies today; the `scratch/**` corner
> activates when the pin moves to the fork's union semantics. Details: gap 5 in
> [01-runtime-semantics.md](./01-runtime-semantics.md).
>
> **Historical note (2026-09-04):** the inert-`write.allow` caveat above
> records fork rev `3bd051bf`. The live pin is `78fb3ed1` (`flake.nix`:15-37,
> union semantics native). Full rewrite is a follow-up.

**Mount-row sugar:** the four dotted keys can sit directly on a
`[[workloads.<name>.mounts]]` row; they are normalized at parse time INTO the
row's `policy` fragment, concatenated AFTER an explicit `policy = {...}` table
(workestrate `control/agentctl/src/microsandbox/plan.rs:167-178, 218-229,
282-295`):

```toml
[[workloads.api.mounts]]
host = "workspaces/api-state"
guest = "/data"
read.deny = ["agent/auth.json"]
write.deny = ["logs/**"]
```

## Optional guest owner

A mount row can also declare `owner = { uid = 61040, gid = 61040 }`. Both numeric
IDs are required. This selects native fallback guest ownership, not a path-policy
rule or host ownership change; read-only mode and policy enforcement are unchanged.
Files with existing per-file virtual stat overrides retain those overrides.
See [guest ownership of bind mounts](../runtime-provisioning.md#guest-ownership-of-bind-mounts)
for accepted values, layer replacement and create-time behavior.

## The pattern dialect

Mount-root-relative globs (fork `mount_policy/pattern.rs`; the tool's mirror
is equivalent):

| Construct | Meaning |
|---|---|
| `*` / `?` | match within one path segment; never cross `/` |
| `**` | crosses directory boundaries |
| root-anchored (default) | pattern matches from the mount root |
| `**/`-prefixed | floating — matches at any depth |
| trailing `/` | dir-only marker, parsed (pattern.rs:116) — but see the gap below |
| `foo/**` | also matches `foo` itself (stem matcher, pattern.rs:133-147, 171-173) |
| ancestor matching | a rule matching a dir also matches everything beneath it (pattern.rs:175-184) |

Rejected at compile time, with the origin named in the error: empty patterns,
absolute paths (leading `/`), NUL bytes, `..` components, invalid globs
(pattern.rs:106-154; compile test compile.rs:636-651).

> **Gap — dir-only not enforced:** the runtime evaluators ignore the trailing
> `/` marker, so `.git/` currently also matches a FILE named `.git`. Write
> patterns as if `/` were not special. Evidence: known gap 1 in
> [01-runtime-semantics.md](./01-runtime-semantics.md).

## The combination gallery

**Hide everything but one file** (read.deny broad + read.allow carve):

```toml
[policy.mounts.read]
deny = ["**"]
allow = ["template.toml"]
```

**Read-only mount except one writable corner** (write.deny broad +
write.allow corner):

```toml
[policy.mounts.write]
deny = ["**"]
allow = ["tmp/**"]
```

> Carries the activation caveat: at the pinned runtime this recipe only
> denies; the `tmp/**` corner activates at re-pin.

**Sealed credential (protect tier)** — operator scope only (config registry or
user-global overrides); a final `read.deny` there compiles to the protect wire
bucket — hidden AND untouchable, unadoptable:

```toml
# config registry config.toml — operator scope
[policy.mounts.read]
deny = [{ pattern = "**/agent/auth.json", final = true }]
```

**Frozen config dir (visible but untouchable)** — from any scope, including a
workload: final `write.deny` keeps the path visible but denies all writes and
freezes the denial against later scopes:

```toml
[workloads.api.policy.mounts.write]
deny = [{ pattern = "guard/**", final = true }]
```

## Migration map from the retired vocabulary

The pre-unification words are REMOVED, not aliased: `mask`, `unmask`,
`protect`, the `[policy.mounts.writes]` table, `masked_writes`, the entry key
`overridable`, and the mount-row keys `mask`/`unmask`/`protect`/`writes_deny`
are all **hard unknown-field parse errors** (scope.rs:66-75; tests
scope.rs:288-315; value.rs:225-234). This is pre-release churn, so there are
no compatibility shims.

| Old | New |
|---|---|
| `mask = [...]` | `[policy.mounts.read] deny = [...]` |
| `unmask = [...]` | `[policy.mounts.read] allow = [...]` |
| `protect = [...]` | OPERATOR scope's `[policy.mounts.read] deny = [...]` with `final = true` (the only protect-bucket route) |
| `[policy.mounts.writes] { allow, deny }` | `[policy.mounts.write] { allow, deny }` |
| `masked_writes = "deny"` | gone; use `[policy.mounts.write] deny` |
| entry `{ pattern, overridable = false }` | `{ pattern, final = true }` |
| compact string / `overridable = true` | compact string / `final = false` (the default) |
| mount-row `mask` / `unmask` / `protect` / `writes_deny` | mount-row `read.deny` / `read.allow` / `write.deny` / `write.allow` dotted keys |

Note the asymmetry the migration creates: the old surface allowed a
non-terminal `protect` from any scope; the new one routes protection
exclusively through operator-final `read.deny` (compile routing:
compile.rs:198-216). A repo or workload scope can still freeze visibility
(final `read.deny` → terminal Mask rule) but cannot create protect-tier
entries.

Decision record:
[ADR 0031](../migration/50-decisions/0031-mount-path-policy-config-surface.md);
collect-and-compile rationale:
[ADR 0029](../migration/50-decisions/0029-policy-scopes-collect-and-compile.md).
