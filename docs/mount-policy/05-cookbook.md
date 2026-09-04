# Cookbook — mount policy recipes

> **What this doc teaches:** five copy-paste recipes with the guest-visible
> outcome and the caveats that bite. **Read first:**
> [02-config-surface.md](./02-config-surface.md) (vocabulary) and
> [03-hierarchy-and-precedence.md](./03-hierarchy-and-precedence.md) (scope
> placement). Errno expectations come from
> [01-runtime-semantics.md](./01-runtime-semantics.md).

## Recipe 1: secrets hidden, with one carve-out

**Goal:** the guest cannot see dotfiles carrying secrets, except the example
template.

```toml
# any scope — e.g. a config-repo layer
[policy.mounts.read]
deny = ["**/.env", "**/.env.*", "**/.ssh/**", "**/*.pem", "**/*.key"]
allow = ["**/.env.example"]
```

**What the guest sees:** lookups and read-opens of `.env` return `ENOENT`;
directory listings omit the names; `.env.example` reads normally. (The
reference config already ships a superset of this list non-final at
`config.reference/workestrate.toml:24-34` — you usually only need the
carve-out.)

**Caveats:** this is a passive seal — a guest that write-opens a masked name
adopts and tags it (the adoption caveat in
[01-runtime-semantics.md](./01-runtime-semantics.md)). For genuinely
untouchable content, use recipe 3.

## Recipe 2: read-only mount with a writable corner

**Goal:** the mount is read-only to the guest except `tmp/`.

```toml
[policy.mounts.write]
deny = ["**"]
allow = ["tmp/**"]
```

**What the guest sees:** writes anywhere outside `tmp/` fail `EACCES`; reads
are unaffected.

**Caveats — ACTIVATION:** at the pinned runtime (fork rev `3bd051bf`)
`write.allow` is INERT — today this recipe denies writes everywhere and the
`tmp/**` corner does NOT open. The corner activates when the fork pin moves to
the union semantics (known gap 5 in
[01-runtime-semantics.md](./01-runtime-semantics.md)). Authoring it now is
safe: it compiles, validates, and shows in `explain`/`preview`.

> **Historical note (2026-09-04):** the inert-`write.allow` caveat above
> records fork rev `3bd051bf`. The live pin is `78fb3ed1` (`flake.nix`:15-37,
> union semantics native). Full rewrite is a follow-up.

## Recipe 3: sealed credential (protect tier)

**Goal:** `agent/auth.json` is hidden AND untouchable, and no repo or
workload layer can ever reopen it.

```toml
# OPERATOR scope only: home registry config.toml or overrides.toml
[policy.mounts.read]
deny = [{ pattern = "**/agent/auth.json", final = true }]
```

**What the guest sees:** lookup `ENOENT`, omitted from listings, create-over
and write-open `EACCES`, unlink/rename `ENOENT`. The name can never be tagged,
so adoption does not apply. The final flag freezes the decision against every
later scope.

**Caveats:** only operator scopes route to the protect bucket; the SAME
declaration in a repo/workload scope compiles to a visibility-only terminal
Mask rule (still adoptable). See the routing table in
[03-hierarchy-and-precedence.md](./03-hierarchy-and-precedence.md).

## Recipe 4: org-wide .env seal from the home registry

**Goal:** every workload on this machine hides `.env` files, as operator
policy rather than per-repo convention.

```toml
# home registry config.toml (operator scope)
[policy.mounts.read]
deny = ["**/.env", "**/.env.*"]
```

**What the guest sees:** identical to recipe 1's deny behavior, for every
workload, without any repo cooperation.

**Caveats:** these entries are non-final, so a config-repo or workload layer
can still carve out a specific file with `read.allow` (e.g. a demo fixture).
If you want the seal to be unrelaxable, make the entries final — which, from
an operator scope, promotes them to the protect tier (recipe 3 semantics).
Choose deliberately: final means no project can ever legitimately unseal a
path.

## Recipe 5: dev instance with a port band and a small mount policy

**Goal:** a scratch workload whose state mount hides its secrets tree and
freezes a guard directory, on a self-healing dev port.

```toml
schema_version = 1

[workloads.dev-scratch]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["python", "-m", "http.server", "8080"]

[workloads.dev-scratch.instance]
port = { preferred = 4000, on_occupied = [{ increment = { range = [5000, 5100] } }, "auto"] }

[workloads.dev-scratch.policy.mounts.read]
deny = ["secrets/**"]

[workloads.dev-scratch.policy.mounts.write]
deny = [{ pattern = "guard/**", final = true }]

[[workloads.dev-scratch.mounts]]
host = "workspaces/dev-scratch-state"
guest = "/data"
```

**What the guest sees:** `secrets/` is absent from listings and `ENOENT` on
access; `guard/` is visible and readable, but every write-class op there
returns `EACCES`, frozen against later scopes. This is the shape of the E6
host-e2e capsule (see [06-testing.md](./06-testing.md)).

**Caveats:** the port band gives a self-healing dev port — preferred 4000,
incrementing through the 5000–5100 band when occupied, then any free port.
The `instance.port` field's full `preferred`/`on_occupied` semantics —
instance naming, address allocation, occupied-port chains — belong to
[ADR 0030](../migration/50-decisions/0030-instance-lifecycle-and-namespacing.md);
this recipe only shows the combined capsule shape. The mount policy is
workload-scope (non-operator), so nothing here is protect-tier.
