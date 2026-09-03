# ADR 0035: Hierarchical egress/ingress policy — collect-and-compile network policy

**Status:** Draft (for iteration — NOT yet Proposed)
**Date:** 2026-09-03
**References:** `control/agentctl/src/policy.rs:4-17` (`ALLOWED_EGRESS_HOSTS`); `control/agentctl/src/config/types.rs:1367-1399` (`SecretsPolicyFragment` ladder + provenance); `control/agentctl/src/microsandbox/workload/secrets.rs` (resolve walk); `docs/mount-policy/03-hierarchy-and-precedence.md` (collect-and-compile precedent); `docs/mount-policy/02-config-surface.md`; ADR 0029 (collect-and-compile); ADR 0031 (mount-policy surface); ADR 0004 (security allowlist); ADR 0018/0019 (secrets ladder); `control/agentctl/src/config/registry.rs` (`Registry.policy`); `schemas/workestrate.schema.json` + `workestrate-workload.schema.json` (schemars derives); `tombi.toml` (formatting); `nix/packages/agentctl.nix` (MSB wrapper contract); `nix/devshells/default.nix` (MSB homes)

> **Draft note:** This is a temporary draft for the user's final iteration. It records adjudicated decisions only; it is NOT yet `Proposed`. Open questions for final iteration are in §15. Design-notes companion: [`./0035-design-notes.md`](./0035-design-notes.md). Shelved items: [`../shelved-todo.md`](../shelved-todo.md).

## 1. Title / Status / Date

- **Title:** Hierarchical egress/ingress policy — collect-and-compile network policy
- **Status:** Draft (for iteration — NOT yet Proposed)
- **Date:** 2026-09-03
- **Supersedes / amends:** Amends ADR 0004 (replaces its `ALLOWED_EGRESS_HOSTS` mechanism); extends the ladder machinery of ADR 0018/0029/0031 to `policy.egress`/`policy.ingress`.

## 2. Context

### 2.1 The hardcoded allowlist flaw

`control/agentctl/src/policy.rs:4-17` defines `ALLOWED_EGRESS_HOSTS` — 12 hardcoded personal providers compiled into the **public** `workestrate` binary (workestrate is a public repo). Every user is locked to the maintainer's vendor list. Adding `api.tempo.io` required a Rust change — the real case: the `tempo_buddy` workload was blocked because its host was absent from the const. Config layers could *reference* a recipe (`[[network.egress]] recipe = "https" hosts = [...]`) but never *extend* the ceiling; the check in `control/agentctl/src/config/validation.rs:339-366` and `control/agentctl/src/merge.rs:719` rejects any host absent from `ALLOWED_EGRESS_HOSTS`.

The flaw conflates two distinct boundaries:

- **Supply-chain boundary (valid):** untrusted configs must not self-grant network access the operator did not approve.
- **Operator authority (invalid as implemented):** the operator *owns the machine* — `~/.microsandbox`, `WORKESTRATE_HOME`, and the host firewall are theirs. The ceiling must be operator-configurable, not maintainer-compiled.

Because the binary is public, the hardcoded list also leaks personal vendor choices into every downstream consumer's binary.

### 2.2 Ladder precedent

Two subsystems already implement the correct machinery:

- **Secrets ladder** (`control/agentctl/src/config/types.rs:1367-1399` `SecretsPolicyFragment`, `control/agentctl/src/microsandbox/workload/secrets.rs` resolve walk, provenance): rungs `built-in passthrough → home registry (Registry.policy.secrets) → config layers → workload → per-secret`, with `final` freeze and provenance.
- **Mount-policy collect-and-compile** (`docs/mount-policy/03-hierarchy-and-precedence.md`, ADR 0029): fragments collected per scope in stack order, dedicated compiler owns precedence / freeze / trust, provenance retained.

This ADR extends **the same machinery** to network policy (`policy.egress` / `policy.ingress`). No new merge semantics are introduced; `merge.rs` never sees policy fragments (same as mounts/secrets).

### 2.3 Related docs

- **Schema staleness flow:** `schemas/workestrate.schema.json` is generated from Rust types via `control/agentctl/src/commands/diagnostics.rs:generate_schema_pair` → `workestrate generate-schema --output` → `schemas update` distribution; `validate-config` emits a staleness warning when the committed schema drifts (P3 warning, not a hard error).
- **Runtime provisioning:** `docs/runtime-provisioning.md` (companion to this ADR, §8 + §9 below) — the MSB three-homes, wrapper contract, and schema flow.
- **Design notes:** [`./0035-design-notes.md`](./0035-design-notes.md) — evolution log, decision log, open-questions resolution, implementation order.
- **Shelved follow-ups:** [`../shelved-todo.md`](../shelved-todo.md) — deferred items with context for future sessions.

## 3. Decision — rungs, namespaces, entries

### 3.1 Rungs (authority descending)

| # | Rung | Source artifact | TOML location | Authority |
|---|------|-----------------|---------------|-----------|
| 1 | **home registry** | `<home>/config.toml` (the tool home registry file) | `Registry.policy` — file `<home>/config.toml` fragment `[policy.*]` | **highest** (operator) |
| 2 | **config layer(s)** | each `workestrate.toml` layer in registry stack order | `ConfigFile.policy` — per-layer `[policy.*]` | middle |
| 3 | **workload** | `[workloads.<name>]` capsule | `[workloads.<name>.policy]` monolithic **or** bare `[policy.*]` inside a directory-mode capsule | **lowest** |

Ordering within rungs 2 is stack order (earlier = lower). No other rungs exist. The workload rung is always least-privileged; a home `final` seals every lower rung.

### 3.2 Namespaces

```
policy.egress.allow  | policy.egress.deny
policy.ingress.allow | policy.ingress.deny
```

Each **polarity table** (`policy.egress.allow`, `policy.egress.deny`, `policy.ingress.allow`, `policy.ingress.deny`) carries three scalars:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `all` | `bool` | `false` | allow-all / deny-all for that direction+polarity (least-specific entry; see §5) |
| `on_conflict` | `enum` `"ignore" \| "warn" \| "fail"` | `"ignore"` (built-in) | what to do when a lower rung conflicts with a frozen higher rung — **per-ladder** (see §8: egress and ingress ladders have independent `on_conflict`; each polarity table overridable; `final` seals the choice) |
| `final` | `bool` | `false` | freezes the **whole ladder** against lower rungs (see §5 invariants). Fragment-level freeze. |

Axis entries live as **array-of-tables under the polarity table**:

```toml
[[policy.egress.allow.domain]]  domains = ["github.com", "api.github.com"]  port = 443  protocol = "tcp"  final = false
[[policy.egress.allow.host]]    ports = [53]  protocols = ["tcp", "udp"]  final = false   # host-bridge (dns)
[[policy.egress.deny.domain]]   domains = [".tracker.io"]  port = 443  final = false
[[policy.egress.deny.domain]]   domains = [".evil.com"]  final = false                     # port omitted = all ports (deny-only)
[[policy.ingress.allow.port]]    ports = [4000]  protocol = "tcp"  scope = "local"  final = false
[[policy.egress.deny]]          all = true  final = true   # deny-all (also [[policy.ingress.deny]] all = true)
```

New idiom for allow-all (least specific rank, see §5):

```toml
[[policy.egress.allow]]  all = true         # or with final:
[[policy.egress.allow]]  all = true  final = true
```

### 3.3 Entry fields

| Axis | Table name | Required fields | Optional fields | Notes |
|------|------------|-----------------|-----------------|-------|
| `egress.allow.domain` | `[[policy.egress.allow.domain]]` | `domains: string[]`, `port: u16` | `protocol = "tcp"` (default; `udp` on host only), `final = false` | `port` **REQUIRED** on allow — see asymmetry rationale below |
| `egress.allow.host` | `[[policy.egress.allow.host]]` | `ports: u16[]`, `protocols: string[]` | `final` | host-bridge (dns + litellm proxy); `host` is the msb host bridge |
| `egress.deny.domain` | `[[policy.egress.deny.domain]]` | `domains: string[]` | `port: u16`, `protocol` (default `tcp`), `final` | `port` **optional** — omitted = **all ports** (more expressive than old port-agnostic-only) |
| `ingress.allow.port` | `[[policy.ingress.allow.port]]` | `ports: u16[]` | `protocol = "tcp"`, `scope` (see §3.4 vocabulary), `final` | ingress is **port-only, peer-source-scoped** — domain rules rejected (see §3.4) |
| `ingress.deny.port` | `[[policy.ingress.deny.port]]` | `ports: u16[]` | `protocol`, `scope`, `final` | deny counterpart; `scope` vocabulary same as allow |
| `*_deny.all` | `[[policy.egress.deny]]` / `[[policy.ingress.deny]]` | `all = true` | `final` | deny-all |
| `*_allow.all` | `[[policy.egress.allow]]` / `[[policy.ingress.allow]]` | `all = true` | `final` | allow-all (least specific) |

**Port asymmetry rationale (intentional):**

- **Allow without port rejected:** `[[policy.egress.allow.domain]] domains=["github.com"]` with no `port` would silently mean `443`. That hidden default was rejected — every allow must state its port explicitly; there is no "default port" for an authorization grant. Reviewer sees `port = 443` on the line; no inference.
- **Deny without port = intentionally broad, fail-closed:** `[[policy.egress.deny.domain]] domains=[".evil.com"]` with no `port` intentionally means **all ports** for that domain/suffix. A deny that is broader than intended is safe (fail-closed); a deny that silently narrowed to 443 would be a bypass. This matches the prior decision (§15 item 1 resolution) and is more expressive than the old port-agnostic-only deny.
- **Compiler enforcement:** `validate-config` emits hard error if `egress.allow.domain` omits `port`; `egress.deny.domain` with omitted `port` is valid and compiles to port-agnostic denial (see §5 invariant #19 and §6 compiler ordering).
- **SDK mapping:** Both forms are fully enforceable via the ordered builder (see §5A) — e.g. `e.tcp().port(443).deny_domains([...])` for port-scoped deny vs `e.tcp().deny_domains([...])` for port-agnostic deny, and `e.tcp().port(443).allow_domains([...])` for allow (port required).

**Domain syntax:**

- `domains` entries are **exact** (`"evil.com"` = only that apex) or **suffix** (`".evil.com"` = apex + every subdomain). Suffix is inclusive of the apex (`.evil.com` covers `evil.com` and `*.evil.com`).
- **Rejected at validation** (hard `validate-config` error with span): wildcards (`*`), scheme (`https://`), port-in-domain (`evil.com:443`), path (`evil.com/foo`), empty strings, absolute forms.
- **Non-ASCII flag-gated** (`policy.idna.mode = "reject"` (default) | `"uts46"`, ladder with `final`, see §3.5, §5#17, §12.1) — `reject` (default): hard error with codepoint `U+XXXX` + index + punycode hint; `uts46`: strict `ToASCII` → A-label, UTS39 warnings.
- `protocol` is currently always `"tcp"` for domains; `udp` is valid only on `host` entries. Unknown protocols are `deny_unknown_fields`.

### 3.4 Ingress scope vocabulary — peer-source groups (NOT bind addresses)

Ingress scope is a **peer-source classifier**, not a bind address. This is an APPROVED expansion based on SDK reality.

#### 3.4.1 Vocabulary

| Scope value | Peer-source group | Exact SDK `DestinationGroup` membership | Notes |
|-------------|-------------------|------------------------------------------|-------|
| `loopback` | loopback only | `Loopback` ( `127.0.0.0/8` + `::1` ) | narrowest; single-host self only |
| `local` | loopback + link-local + host gateway | `Loopback` + `LinkLocal` + `Host` (gateway) — **exactly today's `allow_local`** | NOT `Metadata` (explicitly excluded), NOT LAN/private, NOT routable |
| `private` | RFC1918 + CGN + ULA | `Private` (`10/8`, `172.16/12`, `192.168/16`) + `CGNAT 100.64/10` + `ULA fc00::/7` | available group, not currently exposed by default; operator may allow |
| `public` | routable public | `Public` (routable, non-private/link-local/loopback) | today's `allow_public` |
| `any` | any peer | union of all groups (`Any`) — no source filter | least-specific ingress scope; lint-gated (see §8, §15 item 4 deferred lint) |

`private` and `loopback` are valid SDK groups. The compiler maps `scope` to the SDK `Destination::Group(group)` / `Destination::Any` discriminator.

**`allow_local` exact semantics (preserved):** `local` = `127/8` + `::1` + `169.254/16` + `fe80::/10` (link-local) + host gateway (`Host`) peers. Explicitly **excludes** `Metadata` (`169.254.169.254` etc — available group, excluded), and **excludes** `private` LAN (`192.168/16` etc) and `public` routable. This is the SDK's `allow_local` today; the new vocabulary simply names the components separately while preserving the `local` shorthand.

**`allow_public` = `public` group (routable). `allow_private` would be `private` group (RFC1918+CGN+ULA) — available but not currently exposed as a preset; reachable by selecting `scope = "private"` or `"any"`.**

#### 3.4.2 Bind is orthogonal

| Axis | TOML location | Values | Meaning |
|------|---------------|--------|---------|
| **bind** | `[[ports]]` / `PublishedPort.host_bind` (existing workload port surface, NOT policy) | `loopback` (default `127.0.0.1`) \| `wildcard` (`0.0.0.0`/`::`) \| `interface` (`<ip>`) | Where the host actually binds the published port |

**Final inbound reachability = `bind` ∧ `policy` ∧ `port`.** A `scope = "any"` ingress policy does not open a `bind = "loopback"` port to the world; the kernel bind is the first gate. Conversely, `bind = "wildcard"` with `ingress.deny all=true` remains closed. Both gates are shown in `plan --provenance`.

Examples:

```toml
# litellm gateway: host binds loopback, policy allows local peers → local-only reachability
[[workloads.litellm.ports]] guest = 4000 host = 4000 bind_ip = "127.0.0.1"  # bind = loopback
[[workloads.litellm.policy.ingress.allow.port]] ports = [4000] scope = "local"

# future: public service — wildcard bind + public scope
# [[workloads.api.ports]] guest = 8080 host = 8080 bind_ip = "0.0.0.0"  # bind = wildcard
# [[workloads.api.policy.ingress.allow.port]] ports = [8080] scope = "public"
```

#### 3.4.3 What ingress does NOT support

- **Ingress domain rules NOT supported — hard validation error.** There is no SNI for TCP peer connections; the ingress evaluator sees only peer IP + port. `[[policy.ingress.allow.domain]]` is a `validate-config` error (`ingress domain rules not supported — no SNI for peers`). Domain-based peer filtering would require reverse-DNS trust, which is not a policy primitive.
- **UDP ingress gated.** The MSB SDK today does not implement UDP ingress relay (runtime unsupported). `protocol = "udp"` on ingress entries is gated: `validate-config` warns and `plan` notes `runtime unsupported`; the compiler may emit but the sandbox will not enforce until runtime support lands.
- **ICMP ingress rejected.** The SDK `IngressDoesNotSupportIcmp` — any ingress entry with `protocol = "icmp"` / `"icmpv4"` / `"icmpv6"` is a hard `validate-config` error with span. Egress ICMP (echo) is allowed (see §5A), but ingress ICMP is not a policy concept.

### 3.5 `policy.idna` — flag-gated IDNA mode (ladder, default-off)

`policy.idna` is a dedicated policy fragment collected like other `policy.*` fragments (never via `merge.rs`; resolved before domain validation). It controls how non-ASCII in `domains` is handled.

```toml
[policy.idna]
mode = "reject"   # or "uts46"
final = false     # fragment-level final seals lower rungs (same as policy ladder, invariant #11)
```

**`[policy.idna] mode` semantics (ladder, like `on_conflict`):**

- `mode = "reject"` (built-in default, `final` not set) | `"uts46"` — enum, `deny_unknown_fields`.
- **Ladder:** built-in `"reject"` → `home` → `config layers` (stack) → `workload` (lowest). Higher rung with `final = true` seals lower rungs (same as policy ladder, invariant #11). Fragment-level `final` on `[policy.idna]`.
- **`reject` (default):** any `domains` entry containing non-ASCII byte (`>0x7F`) is hard validation error at gate 1, citing codepoint (`U+XXXX`), byte index, and punycode suggestion (e.g. `domain "münchen.de" at index 1 (U+00FC) is non-ASCII — in reject mode use punycode "xn--mnchen-3ya.de" (idna::domain_to_ascii_strict)`). No idna processing stored; error before resolution. Deterministic, fail-closed.
- **`uts46`:** validator accepts non-ASCII, runs idna strict path (IDNA2008 + UTS46 nontransitional + STD3 + Bidi/Joiner + hyphen + DNS length) → `ToASCII` A-label (punycode, lowercased, normalized). On idna error → hard validation error (reason + codepoint). On success → canonical A-label used for coverage/specificity/Rule emit (matched as ASCII vs SNI bytes). Original Unicode retained in provenance (`idna_original`) for display.
- Plus **UTS39 skeleton linter** (`unicode-security` crate) in `uts46` mode: for each derived A-label, compute `skeleton` and mixed-script restriction level; if confusable with any other allow/deny A-label in resolved set (same skeleton, different bytes) or mixed-script `HighlyRestrictive`/`MinimallyRestrictive` → **WARNING** (`stderr` + provenance `idna_warning: confusable/mixed-script`), does NOT block plan by default (policy-authorization bypass risk, author must review). Future `policy.idna.confusable = "fail"` may be considered but not in v1.
- **TOML surface examples:**

  ```toml
  # home seals reject (default) — no lower rung can enable IDNA
  [policy.idna]
  mode = "reject"
  final = true

  # operator explicitly allows UTS46 at home (or config layer for testing)
  [policy.idna]
  mode = "uts46"
  # final = false (default) — lower rungs could override, but home final would seal

  # workload trying to use IDN under reject → error; under uts46 → A-label
  [[policy.egress.allow.domain]]
  domains = ["münchen.de"]  # reject: error; uts46: → ["xn--mnchen-3ya.de"] port=443
  ```

`policy.idna` is collected like other policy fragments, never via `merge.rs`; resolved before domain validation (gate 1 early, before coverage/specificity).

## 4. TOML surface — full examples

### 4.1 Home registry — operator deny-all-final (the strict home)

```toml
# <home>/config.toml  — Registry.policy (operator scope, highest authority)
# The operator seals the ladder: no lower rung can carve exceptions.

[policy.egress.deny]
all = true
final = true
on_conflict = "warn"   # lower-rung conflicts warn (see §8; per-ladder: this governs egress deny ladder)

[policy.ingress.deny]
all = true
final = true
on_conflict = "warn"   # separate ladder — ingress deny has its own on_conflict
```

With `final = true` the whole ladder is frozen at the home rung — every lower `allow` is frozen out. With `on_conflict = "warn"` each frozen-out entry emits `stderr` (domain, lower origin/entry, frozen-by origin) and the plan continues; `fail` would aggregate and refuse to start. `on_conflict` is per-ladder (see §8) — `policy.egress.deny on_conflict` does not affect `policy.ingress.deny`.

### 4.2 Config-layer carve-out attempt (warns under a final home)

```toml
# workestrate.toml — a config-repo layer (middle rung)
# This layer tries to allow github; the home final above freezes it.

[policy.egress.allow]
on_conflict = "warn"   # own ladder for entries THIS rung freezes (egress-allow ladder)

[[policy.egress.allow.domain]]
domains = ["github.com", "api.github.com"]
port = 443
protocol = "tcp"

[[policy.egress.allow.host]]
ports = [53]
protocols = ["tcp", "udp"]   # dns host-bridge (universal — placed at config layer per §9)

[[policy.egress.deny.domain]]
domains = [".tracker.io"]
# port omitted = all ports denied for that suffix
```

Under the home deny-all-final above, the `github.com:443` allow is **frozen out** — `warn` emits one line per conflict (`domain=github.com lower=config:personal entry=allow.domain#0 frozen-by=home:config.toml`). Plan still succeeds (default-deny remains).

### 4.3 Workload — pi (strict variant, least-privilege)

```toml
# BEFORE (recipes — removed)
# [workloads.pi.network]
# # relies on ALLOWED_EGRESS_HOSTS const
# [[workloads.pi.network.egress]]
# recipe = "https"
# hosts = ["api.openai.com", "api.kimi.com"]
# [[workloads.pi.network.egress]]
# recipe = "dns"

# AFTER — hierarchical policy
[workloads.pi.policy.egress.allow]
on_conflict = "ignore"

[[workloads.pi.policy.egress.allow.domain]]
domains = ["api.openai.com"]
port = 443

[[workloads.pi.policy.egress.allow.domain]]
domains = ["api.kimi.com"]
port = 443

[[workloads.pi.policy.egress.allow.domain]]
domains = [".anthropic.com"]
port = 443

[[workloads.pi.policy.egress.deny.domain]]
domains = [".tracker.io"]
# port omitted = all ports

[workloads.pi.policy.ingress.deny]
all = true   # pi exposes no ingress

# [network.defaults] unchanged — see §10
[workloads.pi.network.defaults]
egress = "deny"
```

DNS host-bridge (53 tcp+udp) lives at the **config-layer rung**, not here — pi inherits it. Per-workload placement keeps pi minimal.

### 4.4 Workload — litellm (the listener, owns the gateway ingress)

```toml
# BEFORE
# [workloads.litellm.network]
# [[workloads.litellm.network.egress]]
# recipe = "dns"
# [[workloads.litellm.network.ingress]]
# protocol = "tcp"
# port = 4000
# scope = "local"

# AFTER — peer-source group semantics (§3.4)
[workloads.litellm.policy.egress.allow]
# dns inherited from config layer; no per-workload egress needed

[workloads.litellm.policy.ingress.allow]
[[workloads.litellm.policy.ingress.allow.port]]
ports = [4000]
protocol = "tcp"
scope = "local"   # loopback + link-local + gateway peers (NOT metadata, NOT private LAN)

[workloads.litellm.network.defaults]
egress = "deny"
```

Gateway `4000/tcp` is **per-workload** (least-privilege — litellm is the listener, `example-service`/`smoke` does not need it). `scope = "local"` means local peers only; `scope = "private"` would additionally allow LAN/RFC1918 peers; `scope = "public"` would allow routable peers. Bind remains `127.0.0.1` by default — reachability = `bind ∧ policy` (see §3.4.2).

### 4.5 Workload — tempo_buddy (the blocked case)

```toml
# BEFORE — blocked: api.tempo.io not in ALLOWED_EGRESS_HOSTS
# [[workloads.tempo_buddy.network.egress]]
# recipe = "https"
# hosts = ["api.tempo.io"]   # validate-config error: host not in ALLOWED_EGRESS_HOSTS

# AFTER — operator or workload can allow it without a Rust change
[workloads.tempo_buddy.policy.egress.allow]

[[workloads.tempo_buddy.policy.egress.allow.domain]]
domains = ["api.tempo.io"]
port = 443
protocol = "tcp"

[[workloads.tempo_buddy.policy.egress.allow.domain]]
domains = ["github.com", "api.github.com"]
port = 443

[workloads.tempo_buddy.network.defaults]
egress = "deny"
```

Adding a vendor is now a TOML edit at the appropriate rung, not a binary rebuild.

### 4.6 Workload with suffix carve-out and host-bridge + ingress

```toml
[workloads.example-service.policy.egress.allow]

[[workloads.example-service.policy.egress.allow.domain]]
domains = ["github.com", "api.github.com"]
port = 443

[[workloads.example-service.policy.egress.allow.domain]]
domains = [".superradcompany.ai"]
port = 443

[[workloads.example-service.policy.egress.deny.domain]]
domains = [".evil.superradcompany.ai"]
port = 443   # carve-out under own final is legal (see §5)

[[workloads.example-service.policy.ingress.allow.port]]
ports = [8080]
protocol = "tcp"
scope = "local"   # or "private"/"public"/"any" per §3.4
```

### 4.7 Allow-all workload (least-specific rank)

```toml
[workloads.smoke.policy.egress.allow]
[[workloads.smoke.policy.egress.allow]]
all = true            # egress allow-all (least specific; see §5 rank)
# no ingress — defaults to deny (fail-closed)
```

## 5. Semantics spec — invariants (load-bearing subset)

Invariants are numbered for test anchoring. The 32-spec condensed to the ~20 load-bearing invariants; gaps are shelved (§12). Full evolution and deferred items: [`./0035-design-notes.md`](./0035-design-notes.md), [`../shelved-todo.md`](../shelved-todo.md).

1. **Rung order is authority order.** `home registry > config layer(s) in stack order > workload`. Lower = less authority. Deterministic total order; no tie-breaking by file order beyond stack order.
2. **Collection, not merge.** Policy fragments are **collected per rung**, never merged via `merge.rs` (same as `SecretsPolicyFragment` and mount-policy). `merge.rs` never sees `PolicyConfig`.
3. **Axis exclusivity.** `egress` and `ingress` are independent axes. A rule on one never affects the other. `allow`/`deny` polarities are opposite within an axis; cross-polarity interaction is defined by invariants 10-16.
4. **Deny_unknown_fields.** Every `PolicyConfig` / polarity table / entry table uses `#[serde(deny_unknown_fields)]`. Unknown keys are hard `validate-config` errors with file/span (no silent ignore).
5. **Coverage model — exact vs suffix.** Exact `"evil.com"` covers exactly that label. Suffix `".evil.com"` covers `evil.com` and every label-suffixed subdomain (`a.evil.com`, `a.b.evil.com`) — inclusive of the apex. No other pattern forms exist.
6. **Specificity rank.** `all` (allow-all/deny-all) < suffix < exact. For suffix vs suffix, longer suffix wins (`".a.evil.com"` > `".evil.com"`). Most specific covering entry wins within the unfrozen set.
7. **Same-rung: most specific covering entry wins.** Within one rung, if multiple entries cover a domain:port, the most specific (per #6) decides. A carve-out under **own** `final` is **legal** (e.g., `deny .evil.com` with `final`, then `allow evil.com` same rung — the allow is more specific and wins; the final only freezes *lower* rungs).
8. **Same-rung: identical coverage, opposite polarity = hard validation fail.** Regardless of `final` flags, two entries in the same rung with identical coverage (same suffix/exact + same port + same protocol) and opposite polarity is a hard `validate-config` error. `deny_unknown_fields` does not swallow it; provenance of both entries is named.
9. **Cross-rung tie (same coverage, opposite polarity, neither final): deny wins (fail-closed).** When two rungs declare equal-coverage entries of opposite polarity and neither entry nor fragment is final, the `deny` polarity prevails.
10. **Redundant same-polarity = no-op with provenance.** Two entries of identical coverage and same polarity are redundant, not an error. Provenance records both; the higher rung's entry is the effective one.
11. **Finality invariant (verbatim):** "A lower-rung entry may override a higher-rung entry iff no higher fragment-final and no covering higher entry-final exists; when unfrozen, the most specific covering entry wins (exact > suffix, longer > shorter; all is least specific); when frozen, the freezing entry's polarity prevails absolutely."
12. **Fragment-final vs entry-final.** `final = true` on a polarity table (fragment-final) freezes the **entire ladder** below that rung for that polarity+axis. `final = true` on an entry (entry-final) freezes only its covering set below that rung. Either suffices to trigger #11's frozen path.
13. **Entitlement bypasses default-deny, not the deny hierarchy.** `network.defaults.egress = "allow"` + `entitlements = ["default_egress_allow"]` (and `default_ingress_allow` for ingress) bypasses the SDK's default-deny but does **NOT** bypass the `policy.egress.deny` / `policy.ingress.deny` hierarchy. A home deny-final seals even entitled workloads (fail-closed).
14. **on_conflict ladder — per-ladder granular (CONFIRMED).** Each polarity table carries `on_conflict = "ignore" | "warn" | "fail"` (default `"ignore"` built-in). `final` on a rung seals its own `on_conflict` choice against lower rungs (home `final` seals the choice). **Crucially, `on_conflict` is per-ladder: egress vs ingress are separate ladders with independent `on_conflict` values, overridable per polarity table.** `policy.egress.allow on_conflict = "warn"` does not affect `policy.ingress.allow`; `policy.ingress.deny on_conflict = "fail"` is independent. The final choice at each ladder freezes lower rungs' ability to downgrade or upgrade that ladder's conflict handling (see §8 for UX and sealing). This is CONFIRMED and unambiguous.
15. **on_conflict = fail aggregates.** When `fail` is the effective policy, the compiler collects **ALL** frozen conflicts before failing, then emits one structured aggregate error; `plan` fails and the workload does not start.
16. **Determinism.** Resolution is deterministic given the same rung order + entries. Same inputs → same plan bytes (golden-plan stable). No hash-map iteration order leaks.
17. **IDNA / non-ASCII flag-gated (default reject, explicit uts46).** `policy.idna.mode` ladder (built-in `"reject"` → home → layers → workload, `final` seals). In `reject` (default) any non-ASCII byte in `domains` is hard validation error (gate 1) with codepoint + index + punycode suggestion (via `idna::domain_to_ascii_strict`). In `uts46` non-ASCII is processed via idna strict (IDNA2008 + UTS46 nontransitional + STD3 + Bidi/Joiner + hyphen + DnsLength::Verify) → A-label; failure → hard error; success → matched as ASCII. In `uts46` mode a UTS39 confusable/mixed-script linter (`unicode-security`) emits WARNINGs when derived A-labels are confusable (same skeleton) or mixed-script (`HighlyRestrictive` etc). Fully specified in §3.5 and §12.1 (implementation reality + test vectors).
18. **Wildcard / scheme / port-in-domain / path rejected.** `*`, `https://`, `evil.com:443`, `evil.com/foo` are all hard validation errors.
19. **Deny port-optional semantics — asymmetry rationale (APPROVED).** On `egress.deny.domain` the `port` field is **optional**; omitted = **all ports** for that domain/suffix. On `egress.allow.domain` `port` is **REQUIRED**. **Asymmetry rationale:** allow-without-port would be a hidden default to `443` (rejected — every allow must be explicit, no inference, no hidden 443). Deny-without-port intentionally means **all ports** — the broad interpretation is fail-closed (a deny that is broader than intended is safe; a deny silently narrowed to 443 would be a bypass). Compiler emits ordered SDK rules: **port-scoped denies → port-agnostic denies → allows, within specificity ordering** (see §6, §5A). The legacy SDK prepend helper is port-agnostic and **must NOT be used** for port-scoped intent.
20. **Migration hard cutover.** Recipes are deleted with no compat aliases (see §9). All existing workloads are rewritten in the same PR. No dual-stack.
21. **Defaults unchanged.** `[network.defaults]` + `entitlements` gating is unchanged (see §10). Only the allow/deny rule surface migrates.
22. **All is least specific.** `all = true` entries rank below every suffix and exact entry (see #6). An `all` allow does not shadow a higher suffix deny under #11's frozen path.

## 5A. Enforcement reality — SDK findings

This section is the **enforcement reality** sibling to the TOML surface — how the TOML compiles to the MSB SDK's `NetworkPolicy` and what the runtime can and cannot enforce. It is based on SDK code findings.

### 5A.1 SDK rule model

The SDK policy is an **ordered first-match-wins `Rule` list** evaluated in-process on the smoltcp poll thread (no netfilter / no iptables):

```rust
Rule {
  destination: Destination::Any
             | Destination::Cidr(IpNet)
             | Destination::Domain(String)        // exact
             | Destination::DomainSuffix(String)  // suffix incl. apex
             | Destination::Group(DestinationGroup),
  protocols: HashSet<Protocol>,   // { Tcp, Udp, Icmpv4, Icmpv6 } subset
  ports: PortRangeSet,            // empty = any-port, else specific ports
  action: Action::Allow | Action::Deny,
}
// DestinationGroup: Host | Metadata | Loopback | Private | LinkLocal | Public
// Protocol: Tcp | Udp | Icmpv4 | Icmpv6  (closed enum; see §5A.5)
```

- **Evaluation:** Every packet is evaluated against the ordered list; first matching rule decides; if no rule matches, the default action (`defaults.egress/ingress`) decides. There is no "no rule blocked" bypass — `allow` default still evaluates every packet; the default is just the fallback.
- **DNS interception:** The sandbox gateway intercepts DNS at `53/tcp+udp` and `853` (DoT stub). IP→hostname cache is populated from DNS responses. Outbound `A`/`AAAA` queries that would otherwise leave the sandbox are answered from the cache when possible.
- **TLS SNI peek:** On the first flight of a TCP connection, the proxy peeks the TLS ClientHello SNI (Server Name Indication) when present. Domain rules match SNI + DNS cache (see bypass limits below).
- **ICMP:** Echo-only relay (`Icmpv4`/`Icmpv6` echo request/reply). No other ICMP types. UDP non-DNS is relayed. No raw sockets.
- **Allow-default note:** `defaults.egress = "allow"` + entitlement means "no rule blocked it" — **every packet is still evaluated** against the ladder-derived rules; the default is just the final fallback, not a short-circuit.

### 5A.2 Domain enforcement bypass limits (honest)

| Bypass primitive | What it evades | Fail-closed? | Mitigation / note |
|------------------|----------------|--------------|-------------------|
| **IP-literal without prior DNS + no SNI** (e.g. `curl https://1.2.3.4` with no ClientHello SNI, or plaintext HTTP to IP) | `egress.allow.domain` / `egress.deny.domain` (domain rules) — the cache has no IP→hostname mapping and SNI is absent | **Deny paths fail closed** (no allow match → default-deny); **allow paths need SNI+cache** so spoofing an allow requires controlling DNS | Deny domain rules use SNI alone (no cache dependency) so `deny .evil.com` is robust even without cache. Allow paths require **both** SNI and cache entry — an attacker cannot forge an allow by spoofing IP alone without DNS. |
| **DoH (DNS-over-HTTPS on 443)** | DNS cache — DoH queries are TLS on 443, indistinguishable from regular HTTPS | Shares `443/tcp allow` with normal TLS; DoH host must be explicitly allowed if DoT is to be distinguished | If DoH host is allowed, DNS queries can egress via HTTPS; policy cannot block DoH without blocking that host's 443. Documented as known limit. |
| **DoT on 853 without intercept** | DNS cache if DoT is refused — `853/tcp` is the implicit DoT intercept; if DoT is blocked without intercept, DoT queries fail | Refused without intercept — `853/tcp` DoT that is not intercepted is refused, not allowed | The host-bridge `allow host 53` covers classic DNS; DoT (`853`) is not a general allow — the proxy intercepts or refuses. |

**Summary:** The honest limits are narrow: **direct-IP + no-DNS + no-SNI** evades domain rules (deny still fails closed), and DoH-on-443 shares the TLS allow. These are documented as known limits, not hidden. Deny is robust (SNI alone); allow is controlled (SNI + cache).

### 5A.3 allow_local exact semantics (SDK truth)

Already in §3.4.1, restated for enforcement:

- `allow_local` (now `scope = "local"`) = `Loopback` (`127/8`, `::1`) + `LinkLocal` (`169.254/16`, `fe80::/10`) + `Host` (gateway) — **and nothing else**.
- Explicitly **NOT** `Metadata` (`169.254.169.254` — `DestinationGroup::Metadata` exists but is excluded).
- Explicitly **NOT** `Private` (RFC1918 `10/8`, `172.16/12`, `192.168/16` + CGN `100.64/10` + ULA `fc00::/7`) — that is `scope = "private"`.
- Explicitly **NOT** `Public` (routable) — that is `scope = "public"`.
- `scope = "any"` = all groups (no filter). `scope = "loopback"` = `Loopback` only. `scope = "private"` = `Private` group. `scope = "public"` = `Public` group.

See scope vocabulary table in §3.4.1 for exact `DestinationGroup` membership.

### 5A.4 Port-scoped domain rules — compiler ordering & builder mapping

**APPROVED and fully enforceable via the SDK builder under BOTH defaults (first-match-wins ordered rules).**

- **TOML → builder:**
  - `[[policy.egress.deny.domain]] domains=[".evil.com"] port=443` → `e.tcp().port(443).deny_domains([".evil.com"])` (port-scoped deny)
  - `[[policy.egress.deny.domain]] domains=[".evil.com"]` (port omitted = all ports) → `e.tcp().deny_domains([".evil.com"])` (port-agnostic deny, `PortRangeSet::Any`)
  - `[[policy.egress.allow.domain]] domains=["github.com"] port=443` → `e.tcp().port(443).allow_domains(["github.com"])` (port-scoped allow — port required)

- **Compiler emit order (load-bearing):** The compiler MUST emit builder calls in **specificity-then-port-scope order:**
  1. **Port-scoped denies** (most specific within port-scoped)
  2. **Port-agnostic denies** (all-ports denies)
  3. **Allows** (port-scoped, since allow always has port) — within specificity rank (exact > longer suffix > shorter suffix > `all`)

  This ensures first-match-wins evaluates the most specific deny before a broader allow, and a port-agnostic deny before an allow that would otherwise shadow it. The resolver's `#6` ranking plus this port-scope tier is the canonical order.

- **Legacy prepend helper gap:** The SDK's legacy `prepend` helper (used by the old recipe expansion) is **port-agnostic** (`deny_domains` without `port()`), and **must NOT be used** for port-scoped intent. Grep: the old `recipes.rs` prepend path always emitted `deny_domains` without a preceding `port()` call. The new compiler must not reuse that helper for port-scoped denies — it must call `e.tcp().port(n).deny_domains(...)` explicitly.

- **Both defaults:** Under `defaults.egress = "deny"` and `"allow"`, the ordered rules are identical — the default is only the fallback when no rule matches. Port-scoped denies work under either default because they are ordered before allows and carry their port filter.

### 5A.5 Protocol extensibility — closed enum, fail-closed on unknown

The SDK `Protocol` is a **closed enum** today: `Tcp | Udp | Icmpv4 | Icmpv6`. Consequences:

- **Extension path** = new enum variant + builder method (e.g. `.sctp()`) + evaluator branch — not a runtime-discovered protocol.
- **Fail-closed on unknown:** Older hosts refuse newer policy cleanly — a policy carrying an unknown protocol string is a hard `validate-config` error (`deny_unknown_fields` / unknown protocol variant). There is no silent downgrade to `Tcp`. This preserves fail-closed semantics across version skew.

## 6. Resolution algorithm — summary

```
collect rungs in authority order: [home, config-layers... (stack), workload]
  each rung contributes 0..N entries per polarity table + maybe all/final/on_conflict

for each axis (egress, ingress) and each polarity:
  1. validate same-rung invariants (#8, #4, #17, #18, plus ingress domain rejection §3.4.3) → hard errors at validate-config gate
  2. total order entries by (rung authority, specificity rank #6, port-scope tier #19 / §5A.4, polarity tie-break at same specificity handled by #8/#9)
     port-scope tier: port-scoped deny → port-agnostic deny → allow (within specificity)
  3. walk rungs high → low:
       maintain `frozen: Option<(polarity, covering entry, frozen_by origin)>`
       if frozen.is_some(): lower entries covering frozen set are frozen_out
         → provenance `frozen_out: true, frozen_by: <origin>`
         → on_conflict handling per #14 — per-ladder (egress vs ingress independent)
       else:
         most specific covering entry per #6 wins among unfrozen entries
         if entry.final or fragment.final: set frozen
  4. cross-rung tie at equal specificity, unfrozen, opposite polarity → deny wins (#9)
  5. produce canonical resolved plan + provenance shape:
       Provenance { rung, origin (file + scope_kind), entry_index, polarity, final, frozen_out, frozen_by }
       plus bind-vs-policy provenance for ingress (bind = loopback/wildcard/interface, separate from scope)
  6. if on_conflict == fail and any frozen conflicts collected → aggregate error, plan fails

compiler emit (SDK builder) — ordered first-match-wins:
  for egress: emit port-scoped denies → port-agnostic denies → allows (within specificity, §5A.4)
  for ingress: emit per-scope (most specific first) port allows/denies — DestinationGroup-mapped
```

Provenance includes: declaring rung, file, `scope_kind`, entry index, domains/port/protocol/scope, `final`, and when frozen: `frozen_out` + `frozen_by` (origin of the freezing entry). Same as secrets/mount-policy provenance model. For ingress, provenance also records `bind` (from `PublishedPort`) separately from `scope` (from policy).

## 7. Enforcement matrix — 3 gates

| Gate | When | What it checks | On failure |
|------|------|----------------|------------|
| **1. validate-config** | `workestrate validate-config` / `check` | Same-rung authoring errors: `deny_unknown_fields`, identical-coverage opposite-polarity (#8), syntax rejects (#17, #18), `port` required on allow / optional on deny (#19), ingress domain rejection (§3.4.3), `scope` vocabulary closed set, ICMP ingress rejection, `all` vs entries exclusivity | Hard error with file + span; non-zero exit; see §9.4 for ADR-citing message convention |
| **2. plan** | `workestrate workload plan <name>` / `up` (pre-create) | Canonical cross-rung resolution + provenance (§6); `on_conflict` (`ignore`/`warn`/`fail`) applied **per-ladder** (egress vs ingress independent); entitlement check vs `network.defaults`; ordered builder emit (§5A.4) | `fail` → structured aggregate error, plan fails, workload does not start; `warn` → stderr logs + provenance, plan continues |
| **3. sandbox-create** | `msb` sandbox creation (SDK `NetworkPolicy` build) | Defense-in-depth: re-derive `NetworkPolicy` from the resolved plan; SDK deny is fail-closed; first-match-wins ordered `Rule` list (§5A.1) | Sandbox does not start; host-side error |

Entitlement bypass (†) lives at gates 2 and 3 but never bypasses the deny hierarchy (#13).

## 8. on_conflict UX — per-ladder granular

Each polarity table: `on_conflict = "ignore" | "warn" | "fail"` (default `"ignore"` built-in). `final` on a higher rung seals lower rungs' `on_conflict` choice **on that ladder**. This section is explicit: **egress vs ingress are separate ladders; each has its own `on_conflict`, overridable per polarity table, and `final` freezes the choice on that ladder.**

- `policy.egress.allow on_conflict` governs only egress-allow conflicts.
- `policy.egress.deny on_conflict` governs only egress-deny conflicts.
- `policy.ingress.allow on_conflict` and `policy.ingress.deny on_conflict` are independent.
- A home `final` on `policy.egress.deny` seals `policy.egress.deny on_conflict` against lower rungs, but does **not** affect `policy.ingress.deny on_conflict`.
- Final seals the choice: lower rungs cannot downgrade `fail` to `warn` or upgrade `ignore` to `fail` on a frozen ladder.

This is CONFIRMED and already in §5 invariant #14; this section is the UX elaboration. For evolution / why per-table (not top-level) see [`./0035-design-notes.md` §4](./0035-design-notes.md).

### 8.1 `ignore` (built-in default)

Silent apart from provenance. No stderr. Inspectable via `workestrate workload plan --provenance` (or `explain`-equivalent). Use for silent operator sealing.

### 8.2 `warn`

One `stderr` line per frozen conflict, then continue (per-ladder):

```
warn: policy conflict: domain "github.com:443" allow from config layer "personal" (entry allow.domain#0) frozen by home registry "<home>/config.toml" (deny all final); provenance: frozen_by=home
warn: policy conflict: egress deny ".tracker.io:443" from workload "pi" frozen out by home final (already denied); provenance: frozen_by=home
```

Plan succeeds; `network.defaults` + sandbox-create proceed.

### 8.3 `fail`

Collects **ALL** conflicts **on that ladder**, then one structured aggregate error and `plan` fails (workload does not start):

```
error: policy conflicts (3) frozen by home registry "<home>/config.toml" — egress ladder:
  - domain "github.com:443" allow from config layer "personal" allow.domain#0 frozen by home deny all final
  - domain "api.github.com:443" allow from config layer "personal" allow.domain#0 frozen by home deny all final
  - domain "api.tempo.io:443" allow from workload "tempo_buddy" allow.domain#0 frozen by home deny all final
hint: remove the lower-rung allow or relax the home final; per-entry log with on_conflict=warn
```

No partial sandbox is created. Ingress `fail` aggregates separately (`ingress ladder` header).

### 8.4 Sealing the choice

```toml
# home seals both the deny AND the conflict behavior — per ladder:
[policy.egress.deny]
all = true
final = true
on_conflict = "fail"   # lower rungs cannot downgrade this to warn/ignore on the egress-deny ladder

[policy.ingress.deny]
all = true
final = true
on_conflict = "warn"   # ingress-deny ladder has its own choice, independently sealed
```

## 9. Migration

### 9.1 Removed recipe → new syntax

| Removed (`recipes.rs`, `NetworkConfig.egress/deny/ingress`) | New hierarchical syntax |
|---|---|
| `[[network.egress]] recipe = "dns"` | `[[policy.egress.allow.host]] ports=[53] protocols=["tcp","udp"]` at **config-layer** rung (universal) |
| `[[network.egress]] recipe = "https" hosts=["github.com"]` | `[[policy.egress.allow.domain]] domains=["github.com"] port=443 protocol="tcp"` |
| `[[network.egress]] recipe = "github"` | `[[policy.egress.allow.domain]] domains=["github.com","api.github.com"] port=443` |
| `[[network.egress]] recipe = "litellm_proxy"` | `[[policy.egress.allow.host]] ports=[4000] protocol="tcp"` + `[[policy.ingress.allow.port]] ports=[4000] protocol="tcp" scope="local"` (gateway is per-workload, scope vocabulary §3.4) |
| `[[network.egress]] recipe = "agent_base"` (= dns+litellm_proxy+github) | Expand to the three rows above: dns host-bridge at config layer + github domain allow at config layer + gateway per-workload where litellm is the listener |
| `[[network.deny]] domain_suffix="tracker.io"` | `[[policy.egress.deny.domain]] domains=[".tracker.io"]` (port omitted = all ports; optionally `port=443` for port-scoped) |
| `[[network.ingress]] protocol="tcp" port=4000 scope="local"` | `[[policy.ingress.allow.port]] ports=[4000] protocol="tcp" scope="local"` — now peer-source group (§3.4), not bind |
| `(implicit) allow-all` (via `defaults.egress="allow"`+entitlement) | `[[policy.egress.allow]] all=true` (+ optionally `final=true`) — least specific rank, explicit |

No compat aliases. Unknown `recipe` keys are hard `deny_unknown_fields` errors (see §10).

### 9.2 Per-workload adaptation summary — placement decisions

| Workload | Placement | Rationale |
|----------|-----------|-----------|
| `pi` | workload-specific domains (openai, kimi, anthropic) + deny `.tracker.io`; **no** dns/gateway (inherited) | least-privilege; pi does not listen |
| `prime` | workload-specific domains (openrouter, huggingface, etc.); dns inherited | edge model plane |
| `litellm` | **per-workload** `ingress.allow.port 4000/tcp local` + no per-workload egress (dns inherited) | litellm **is** the gateway listener; scope `local` = loopback+link-local+gateway peers; smoke does not need it |
| `tempest` | workload-specific egress domains | model runner |
| `odysseus` | workload-specific egress | agent runner |
| `tempo_buddy` | workload-specific `api.tempo.io:443` + `github.com:443` | the blocked case — now a TOML edit |
| `opencode` | workload-specific `opencode.ai:443` + github + anthropic | editor agent |
| `smoke` | `[[policy.egress.allow]] all=true` + no ingress | test fixture — allow-all least-specific |
| `duelbits/pi,prime,litellm` | same placement as personal counterparts | second config repo |
| `config.reference` 5 examples (`example-litellm`, `example-service`, etc.) | `example-litellm` keeps per-workload ingress 4000 `local`; others keep dns + github-style egress | synthetic fixture; not a real deployment |
| `templates/workestrate-config` jinja | rewritten to emit new syntax | template |
| **config-layer (universal)** | `[[policy.egress.allow.host]] ports=[53] protocols=["tcp","udp"]` (dns) | every workload needs DNS |
| **home (operator)** | optional `deny all final` with `on_conflict` per-ladder | operator seal; not declared per-workload |

`[network.defaults]` + `entitlements = ["default_egress_allow"]` are **unchanged** (see §10).

### 9.3 Verbatim before/after — pi / litellm / tempo_buddy

**pi — before:**

```toml
[workloads.pi]
kind = "service"
image = { recipe = "registry", ref = "workestrate-pi:latest" }
# ... mounts, env, ports ...

[workloads.pi.network.defaults]
egress = "deny"

[[workloads.pi.network.egress]]
recipe = "https"
hosts = ["api.openai.com", "api.kimi.com", "api.anthropic.com"]

[[workloads.pi.network.egress]]
recipe = "dns"

[[workloads.pi.network.deny]]
domain_suffix = ".tracker.io"
```

**pi — after:**

```toml
[workloads.pi]
kind = "service"
image = { recipe = "registry", ref = "workestrate-pi:latest" }

[workloads.pi.policy.egress.allow]
on_conflict = "ignore"

[[workloads.pi.policy.egress.allow.domain]]
domains = ["api.openai.com"]
port = 443

[[workloads.pi.policy.egress.allow.domain]]
domains = ["api.kimi.com"]
port = 443

[[workloads.pi.policy.egress.allow.domain]]
domains = [".anthropic.com"]
port = 443

[[workloads.pi.policy.egress.deny.domain]]
domains = [".tracker.io"]

[workloads.pi.policy.ingress.deny]
all = true

[workloads.pi.network.defaults]
egress = "deny"
```

**litellm — before:**

```toml
[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }

[[workloads.litellm.network.egress]]
recipe = "dns"

[[workloads.litellm.network.ingress]]
protocol = "tcp"
port = 4000
scope = "local"

[workloads.litellm.network.defaults]
egress = "deny"
```

**litellm — after:**

```toml
[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }

[workloads.litellm.policy.ingress.allow]
[[workloads.litellm.policy.ingress.allow.port]]
ports = [4000]
protocol = "tcp"
scope = "local"   # peer-source group (§3.4), NOT bind address

[workloads.litellm.network.defaults]
egress = "deny"
# egress dns is inherited from the config-layer rung (universal)
```

**tempo_buddy — before (blocked):**

```toml
[workloads.tempo_buddy]
kind = "service"
image = { recipe = "registry", ref = "workestrate-tempo:latest" }

[[workloads.tempo_buddy.network.egress]]
recipe = "https"
hosts = ["api.tempo.io", "github.com", "api.github.com"]
# validate-config error: api.tempo.io not in ALLOWED_EGRESS_HOSTS

[[workloads.tempo_buddy.network.egress]]
recipe = "dns"
```

**tempo_buddy — after (operator-editable):**

```toml
[workloads.tempo_buddy]
kind = "service"
image = { recipe = "registry", ref = "workestrate-tempo:latest" }

[workloads.tempo_buddy.policy.egress.allow]

[[workloads.tempo_buddy.policy.egress.allow.domain]]
domains = ["api.tempo.io"]
port = 443
protocol = "tcp"

[[workloads.tempo_buddy.policy.egress.allow.domain]]
domains = ["github.com", "api.github.com"]
port = 443

[workloads.tempo_buddy.network.defaults]
egress = "deny"
```

### 9.4 Error messages must reference governing ADR and intended behavior — removal-not-deprecation policy (pre-public)

**Convention — removed vocabulary is REMOVED outright.** No compat aliases, no deprecation windows, no `warn`-then-`remove` cycles, no dual-stack. Reason: **pre-public project, versions resettable** (no external consumers to migrate gradually). This is a **hard cutover in one PR** (already §9 hard cutover; §5 #20, §10 footnote). The same applies to every vocabulary this ADR deletes (`policy.rs:ALLOWED_EGRESS_HOSTS`, `recipes.rs`, `NetworkConfig.egress/deny/ingress`, `default_deny` shim, `read_only` → `mode = "ro"`, etc.). There is no period where both old and new syntax parse.

**Error messages for removed vocabulary MUST cite the governing ADR AND the INTENDED BEHAVIOR / replacement syntax.** The diagnostic is actionable: a user pasting an old `workestrate.toml` gets a compile error that directly names the next TOML to write, with the ADR path for rationale — not a generic "unknown field" dump. Pattern:

```
error: `network.default_deny` was removed — use `[network.defaults] egress = "deny"` and `entitlements = ["default_egress_allow"]` where needed; see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md) for rationale and migration
error: `recipe = "github"` was removed — use `[[policy.egress.allow.domain]] domains=["github.com","api.github.com"] port=443` instead; see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md) for rationale and migration
error: `recipe = "https" hosts=["github.com"]` was removed — use `[[policy.egress.allow.domain]] domains=["github.com"] port=443 protocol="tcp"`; see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)
error: `[[network.deny]] domain_suffix="tracker.io"` was removed — use `[[policy.egress.deny.domain]] domains=[".tracker.io"]` (or `port=443` for port-scoped); see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)
error: `[[network.ingress]] protocol="tcp" port=4000` was removed — use `[[policy.ingress.allow.port]] ports=[4000] protocol="tcp" scope="local"` (peer-source group, §3.4); see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)
error: `read_only` was removed — use `mode = "ro"` instead; see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)
```

This convention applies to every removed network (and mount) vocabulary item that this ADR governs, and to future policy migrations. The ADR path in the diagnostic MUST be the repo-relative `docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md` (or `docs/migration/50-decisions/0035-design-notes.md` for deeper context). The migration note in §9.1 and §10 enumerates the vocabulary covered.

**Drift guard:** behavioral tests asserting **intended semantics** (§11 matrix + golden plans + provenance snapshots) **+ docs** are the anti-drift mechanism. The error-citing-ADR convention plus intent tests replace deprecation windows. If a future change reintroduces a compat alias or silently accepts removed syntax, the §11 `deny_unknown_fields` + `recipe`/`default_deny`/`deny`/`ingress` negative tests and the golden-plan provenance pins will fail. Docs and tests co-guard intent; no window of dual syntax is needed to preserve it.

**Deprecation windows — REJECTED pre-public, revisit only post-public-adoption.** When semver stability requires gradual migration for external consumers, a `warn`-then-`remove` window may be appropriate — but **not now**. Pre-public, versions are resettable and the hard cutover with actionable errors + tests is the drift guard. Honestly noted: current policy is **REJECTED pre-public**; revisit only after public adoption (cite `../shelved-todo.md` entry 8 resolution). Future policy migrations after public adoption will follow `see ADR <next> (docs/migration/50-decisions/0036-....md)` with an explicit window; this ADR's migrations do not.

**Implementation:** `control/agentctl/src/config/validation.rs` `deny_unknown_fields` on the removed keys plus explicit deprecated-key checks (`network.default_deny`, `network.egress[]` with `recipe`, `network.ingress`, `network.deny`, `read_only`) — each emits the `see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)` suffix **with the replacement syntax** shown above. Future policy migrations follow the same pattern (`see ADR <next>`).

## 10. What is replaced

| Artifact | Status after this ADR |
|----------|----------------------|
| `control/agentctl/src/policy.rs:ALLOWED_EGRESS_HOSTS` | **deleted** entirely (12-entry const removed) |
| `ALLOWED_PACKAGES` (same file) | **kept** — same treatment shelved for later (see [`../shelved-todo.md`](../shelved-todo.md)) |
| `control/agentctl/src/recipes.rs` (`EgressRecipeRef` + `expand()`) | **deleted** (file removed); no compat aliases; error messages point to §9.4 (`see ADR 0035 ...`) |
| `control/agentctl/src/config/types.rs:NetworkConfig.egress` (`Vec<EgressRecipeRef>`) | **removed** — error cites ADR 0035 |
| `NetworkConfig.deny` (`Vec<DenyDomainRule>`) | **removed** (replaced by `policy.egress.deny.domain`) — error cites ADR 0035 |
| `NetworkConfig.ingress` (`Vec<IngressRule>`) | **removed** (replaced by `policy.ingress.allow.port` with peer-source scope vocabulary §3.4) — error cites ADR 0035 |
| `NetworkConfig.default_deny` shim | already removed (prior ADR) — error at `types.rs:548-556` cites ADR 0035 (`network.default_deny was removed — use [network.defaults] egress = "deny"`) |
| `NetworkConfig.defaults` (`NetworkDefaultsConfig`) + `entitlements` | **unchanged** — `egress = "deny"` / `ingress = "deny"` defaults and `default_egress_allow` / `default_ingress_allow` entitlement gating stay as-is; only the rule surface migrates |
| `config.reference/workestrate.toml` 5 example workloads | rewritten to new syntax (same semantics, scope vocabulary §3.4) |
| `templates/workestrate-config` jinja | rewritten to new syntax |
| Personal + duelbits workloads (personal: `pi`/`prime`/`litellm`/`tempest`/`odysseus`/`tempo_buddy`/`opencode`/`smoke`; duelbits: `pi`/`prime`/`litellm`; reference 5; template) | rewritten in **same PR** (hard cutover) |
| `workestrate.schema.json` / `workestrate-workload.schema.json` | regenerated via schemars derives from new `PolicyConfig` types (see §6 schema flow); `registry.schema.json` follow-up is **COMMITTED** (see §12.2 / [`../shelved-todo.md`](../shelved-todo.md)) |
| `read_only` (mounts) deprecation warning | retained — warning already cites ADR 0035 path per §9.4 convention |

No dual-stack, no deprecation window — hard cutover in one change (outright removal, see §9.4; pre-public, versions resettable per ADR 0031 precedent). Deprecation windows REJECTED pre-public (shelved entry 8 resolution); revisit only post-public-adoption.

## 11. Test matrix

No KVM required. All gates are cargo-verifiable; golden plans pin the output.

### 11.1 Resolution unit cases (per invariant + SDK mapping)

| Case | Invariant / § | Expected |
|------|-----------|----------|
| home deny-all-final + workload allow github | #11 frozen → deny prevails | plan denies github, provenance `frozen_by=home` |
| home deny suffix .evil.com + workload allow evil.com exact | #6 specificity exact > suffix, but #11 frozen → deny prevails when home entry is final | frozen deny; `warn` logs, `fail` aggregates |
| workload suffix .evil.com + same-workload exact evil.com allow, both non-final | #7 same-rung most specific wins | allow evil.com (carve-out legal) |
| same rung identical coverage opposite polarity, neither final | #8 | hard validation fail even without final |
| same rung identical coverage opposite polarity, one final | #8 | hard validation fail regardless of final |
| cross-rung same coverage opposite polarity, neither final | #9 | deny wins (fail-closed) |
| same coverage same polarity cross-rung | #10 | no-op, provenance records both |
| workload allow .a.evil.com vs home deny .evil.com (suffix vs suffix, longer suffix) | #6 longer wins, but frozen → deny | frozen deny when home final |
| `[[policy.egress.allow]] all=true` vs suffix deny | #6 all least specific → suffix wins unfrozen | allow-all does not shadow suffix |
| deny domain port omitted vs port-scoped allow same domain same port | #5/#6 + #19 + §5A.4 compiler order | port-scoped allow wins unfrozen at that port; other ports denied; compiler emits port-scoped deny before allow |
| port-scoped deny `port=443` vs port-agnostic deny + allow `port=443` | #19 / §5A.4 | port-scoped deny ordered before port-agnostic; most specific wins |
| entitlements allow + home deny-final | #13 | deny hierarchy seals even entitled workload |
| `on_conflict=ignore` frozen conflict (egress ladder) | #14 per-ladder | silent, provenance only; ingress ladder unaffected |
| `on_conflict=warn` frozen conflict (ingress ladder) | #14 per-ladder | stderr per conflict + continue; egress ladder separate |
| `on_conflict=fail` multiple frozen conflicts (egress) | #15 | single aggregate error per ladder, plan fails |
| `policy.idna.mode = "reject"` (default) + non-ASCII `münchen.de` | #17 §3.5 | hard error at index 1 `U+00FC` with punycode suggestion `xn--mnchen-3ya.de` |
| `policy.idna.mode = "uts46"` + `münchen.de` | #17 §3.5 §12.1 | success → canonical `xn--mnchen-3ya.de`, matched as ASCII, provenance retains `idna_original` |
| `uts46` + `gіthub.com` (Cyrillic `і` `U+0456`) confusable with `github.com` | UTS39 linter §3.5 §12.1 | **WARNING** confusable (skeleton equal), plan continues, provenance `idna_warning` |
| `uts46` + domain with `ZWNJ` `U+200C` invalid joiner context | IDNA strict §12.1 | hard error `Bidi`/`Joiner` contextual failure |
| `uts46` + `ß` deviation `U+00DF` (nontransitional) | IDNA strict §12.1 | verify `ß` stays `ß` → punycode (not `ss` transitional), strict path success |
| `reject` mode `final` at `home` seals workload `uts46` attempt | ladder `final` §3.5 #11 §12.1 | workload `uts46` frozen out, provenance `frozen_by=home`, `frozen_out` |
| `uts46` + `paypal.com` vs `pаypal.com` (Cyrillic `а` `U+0430`) mixed-script | UTS39 linter §12.1 | **WARNING** mixed-script (`HighlyRestrictive`/`MinimallyRestrictive`), plan continues |
| `uts46` + hyphen edge `3rd-4th` hyphen violation | IDNA strict `CheckHyphens` §12.1 | hard error hyphen check |
| `uts46` + label `>63` chars | IDNA strict `DnsLength::Verify` §12.1 | hard error length |
| `uts46` + `.münchen.de` suffix coverage | #5 suffix §12.1 | canonical `xn--` suffix covers apex + subdomains, specificity rank applies |
| wildcard / scheme / port-in-domain / path | #18 | hard validation error |
| unknown field `recipe` in new surface | #4 | hard `deny_unknown_fields` error citing ADR 0035 (§9.4) |
| ingress `scope = "local"` vs `private` vs `public` | §3.4 vocabulary | each maps to correct `DestinationGroup`; `any` = no filter |
| ingress domain rule `[[policy.ingress.allow.domain]]` | §3.4.3 | hard validation error — no SNI for peers |
| ingress `protocol = "icmp"` | §3.4.3 / §5A.5 | hard validation error `IngressDoesNotSupportIcmp` |
| ingress `protocol = "udp"` | §3.4.3 | gated — warn + runtime-unsupported note |
| `bind = loopback` + `scope = any` | §3.4.2 | reachability = bind ∧ policy — loopback bind keeps it local despite `any` |

### 11.2 Validate / plan error cases

- `validate-config` hard errors (gate 1): unknown fields, identical-coverage opposite polarity, non-ASCII, wildcard/scheme/port-in-domain/path, missing required `port` on allow, `all` vs entries misuse, ingress domain rules, ICMP ingress, unknown `scope` value — all cite ADR 0035 per §9.4 where applicable.
- `plan` errors (gate 2): `on_conflict=fail` aggregate per ladder with all conflicts listed; entitlement missing when `defaults.egress="allow"`; schema staleness warning (P3) is not a hard error (deferred — see [`../shelved-todo.md`](../shelved-todo.md)).
- Sandbox-create defense-in-depth (gate 3): SDK `NetworkPolicy` derivation from resolved plan; fail-closed on corruption; ordered `Rule` list verified.

### 11.3 Provenance snapshots

`workload plan --json` / provenance snapshot tests pin:

- `frozen_by` origin (file + rung)
- `frozen_out: true` on every frozen entry
- Specificity rank + port-scope tier applied per entry
- Ingress provenance records both `bind` and `scope` separately

### 11.4 Golden plans

`config.reference/workestrate.toml` 5 example workloads: committed `plan` JSON snapshots (like the mount-policy golden plans) rendered without KVM; byte-identical assertion in CI. Personal/duelbits workloads contribute additional golden vectors.

### 11.5 Schema tests

- `generate_schema_pair` round-trip: `schemars` derives for new `PolicyConfig` types; committed `schemas/workestrate.schema.json` matches `workestrate generate-schema --output` output.
- Drift guard: `validate-config` warns when committed schema is stale (same P3 warning as today; warn emission is deferred-todo for consumer copies — see [`../shelved-todo.md`](../shelved-todo.md)).
- `registry.schema.json` — **committed follow-up** (see §12.2): `schemars::schema_for!(Registry)` → `schemas/registry.schema.json` + `HOME_TOMBI_TOML` → `tombi lint` for `<home>/config.toml` and overrides.

## 12. Shelved & committed follow-ups

Detailed deferred items: [`../shelved-todo.md`](../shelved-todo.md). Design-notes resolution log: [`./0035-design-notes.md`](./0035-design-notes.md) §5.

| Item | Disposition | Note |
|------|-------------|------|
| `ALLOWED_PACKAGES` hierarchical treatment | **Shelved** — [`../shelved-todo.md`](../shelved-todo.md) | Same flaw as `ALLOWED_EGRESS_HOSTS`; same ladder solution later. Not in this ADR. |
| Path-level carve-outs (`/api/*` L7) | **Shelved** | Requires L7 proxy integration; deferred to proxy future. This ADR is domain+port+host only. |
| Home-customizable defaults (`on_conflict` default, `final` defaults) | **Shelved** | Operator can set per-polarity `on_conflict` today (per-ladder, §8); a global home-default knob is shelved. See shelved-todo. |
| IDNA / UTS46 + UTS39 (non-ASCII) | **Designed, flag-gated default-off — implement in v1** (`[policy.idna] mode = "reject"` (default) \| `"uts46"`, ladder with `final`, `idna` `1.1` + `unicode-security` crates) | Rejected by default (hard error with punycode hint, §3.5 §5#17); `uts46` mode runs strict `ToASCII` → A-label + skeleton/mixed-script warnings; see §12.1 implementation reality. Shelved entry 5 updated — now in-scope for engine phase. |
| Presets | **Dropped, shelved revisit** | Only **inline** `[[policy.egress.allow.domain]]` etc. exist. A preset registry (e.g., `preset = "github"`) was considered and rejected — inline is explicit, reviewable, and avoids a second allowlist. May revisit if vendor bundles prove ergonomic. See shelved-todo. |
| `registry.schema.json` distribution | **COMMITTED follow-up** — **not shelved** | APPROVED: generate via `schemars::schema_for!(Registry)` → vendored `schemas/registry.schema.json` → `HOME_TOMBI_TOML` `[[schemas]]` include `config.toml` + `overrides.toml` (§12.2). Moved from open question to committed. |
| Validate-config stale-schema warning (P3) | **DEFERRED** → [`../shelved-todo.md`](../shelved-todo.md) | `validate-config` would warn on stale consumer schema copies; `tombi strict` catches most; deferred to avoid `validate-config` mutation scope creep. See shelved-todo. |
| Ingress `all` lint (§15.4) | **Shelved** → [`../shelved-todo.md`](../shelved-todo.md) | `ingress.allow all=true` lint even when `on_conflict=ignore` — deferred. |
| Deprecation window / compat aliases | **REJECTED pre-public — outright removal** (§9.4) | Removed vocabulary is removed outright with ADR-citing errors + intent tests (pre-public, versions resettable). No compat aliases/windows. Revisit only post-public-adoption. See shelved entry 8 resolution. |

### 12.1 IDNA / UTS46 — designed, flag-gated default-off (IDNA2008 + UTS46 + UTS39)

**What IDNA/UTS46 is.** IDNA (Internationalized Domain Names in Applications, IDNA2008 / RFC 5890-5895) + UTS 46 (Unicode IDNA Compatibility Processing) define the mapping from a Unicode domain label (e.g. `münchen.de`, `gіthub.com`) to its ASCII `A-label` (`xn--mnchen-3ya.de`). That mapping is not a single table — it is **dozens of versioned Unicode tables** plus processing steps:

- **NFC/NFKC normalization** (Canonical/Compatibility decomposition + composition).
- **Mapping tables** (UTS46 §5 — `valid`, `ignored`, `mapped`, `deviation`, `disallowed` per codepoint, derived from `IdnaMappingTable.txt` — ~10k rows, versioned with Unicode).
- **Contextual rules** — `CONTEXTJ`/`CONTEXTO` for `U+200C`/`U+200D` (ZWNJ/ZWJ joiners) and Bidi rules (`Bidi_Domain` — RTL label checks).
- **Deviation characters** — `ß` (`U+00DF` → `ss` in transitional, preserved as `ß` → `xn--` punycode in nontransitional), `ς` (`U+03C2` → `σ`), etc — handling differs between UTS46 transitional vs nontransitional.
- **STD3 ASCII rules**, hyphen restrictions (no `CheckHyphens` violations such as `3rd-4th` hyphen `xn--` misuse), label length (`≤63` per label, `≤253` total), `UseSTD3ASCIIRules`, `CheckHyphens`, `CheckJoiners`.
- All of the above are **versioned with each Unicode release** (UTS46 explicitly notes version skew between encoder and decoder is a data-dependent defect).

In short: IDNA is a **supply-chain of Unicode tables**, not a one-function call.

**The homograph / policy-bypass attack — why this matters in a POLICY context.**

In a normal browser, IDNA enables `münchen.de` to resolve. In a **policy** context (an allowlist), confusables are an **authorization bypass**:

- `gіthub.com` with Cyrillic `і` (`U+0456`) is **visually identical** to `github.com` with Latin `i` (`U+0069`), but they are different byte sequences.
- A policy `allow = ["github.com"]` that naïvely accepts Unicode and then byte-compares against TLS SNI would **silently diverge** from user intent: the operator allowed `github.com` (Latin), the workload requested `gіthub.com` (Cyrillic), and the byte-level match says "different domain" — but the human reviewer saw "github."
- For an `allow` rule this is a **grant-bypass** (operator thought they allowed a safe host, workload used a lookalike to reach attacker-controlled infra that the allowlist did not intend). For a `deny` rule, the mirror is a **deny-bypass** (deny `.evil.com` does not match `еvil.com` with Cyrillic `е`).
- `LOOKS_LIKE` ≠ `IS` at the policy granularity — and policy is exactly where attacker incentive concentrates.

There is no safe "accept Unicode and map transparently" without also solving the confusable problem.

**Why v1 default remains `reject` (core vocab ASCII, zero current need, versioning burden) — now designed behind explicit flag.**

Historically this ADR rejected IDNA for v1. The rationale is preserved: the default stays **`reject`** (fail-closed), but the design is now **complete and flag-gated** (`[policy.idna] mode = "reject" | "uts46"` ladder with `final`, see §3.5). No workload gets IDNA implicitly; an operator must explicitly opt-in.

1. **Core vocab is already ASCII.** Every current workload's domains (`github.com`, `api.openai.com`, `api.tempo.io`, `.anthropic.com`, ...) are ASCII. There is **zero current need** for non-ASCII policy entries — so the built-in default is `reject`.
2. **IDNA versioning is a supply-chain.** Vendoring the mapping tables means tracking Unicode releases; a stale table is a correctness bug that silently changes policy meaning. Gating behind an explicit flag keeps the default surface zero-cost and signals deliberate Unicode handling.
3. **Confusable policy would need MORE than IDNA.** Even a correct IDNA `ToASCII` does not prevent `gіthub.com → xn--gthub-...` from being a *different* A-label than `github.com`. Preventing that requires **UTS39 (Unicode Security Mechanisms)** — `confusable` / `mixed-script` detection, `Highly Restrictive` / `Moderately Restrictive` profiles, and comparison against the **allowed set** (a linter: "this allowlist entry is confusable with an already-allowed entry or with a banned entry"). That is why `uts46` mode also runs the `unicode-security` skeleton linter as a WARNING (see below) — IDNA alone would reintroduce the bypass.
4. **Fail-closed alternative exists in `reject` mode.** Non-ASCII input is a **hard validation error** that suggests punycode with codepoint + index: `error: domain "münchen.de" at index 1 (U+00FC) is non-ASCII — in reject mode use punycode "xn--mnchen-3ya.de" (idna::domain_to_ascii_strict)` (same pattern as existing `validate-config` span errors). The operator who truly needs an IDN can either pre-convert to A-label or enable `mode = "uts46"` and the policy matches bytes (SNI bytes) correctly via the canonical A-label.

Therefore: **v1 implements `reject` by default; `uts46` is designed and in-scope for the engine phase, gated by `[policy.idna] mode`.** IDNA is not "rejected future" — it is **flag-gated default-off**, fully specified (§3.5 + this section), with a documented punycode migration path for the default.

**Implementation reality — crates, API, pipeline, cost, test vectors (implement in v1 engine phase).**

*Shelved entry 5 is now DESIGNED, flag-gated default-off — implement in v1 (see `../shelved-todo.md` entry 5). Ladder semantics are in §3.5; invariant #17 governs validation.*

- **Crates assessment:**
  - `idna` crate `1.1.0` already in `Cargo.lock` transitively via `url` / `hickory-proto` / etc. Check: present. Promoting to direct dependency in `control/agentctl/Cargo.toml` is trivial: add `idna = "1.1"` and `unicode-security = "0.1"` (`unicode-security` `0.1.2` available on crates.io, validates). No vendor fork impact — `microsandbox` fork vendor path unaffected (`patch.crates-io` only patches `microsandbox-*` crates; `idna`/`unicode-security` are crates.io, already in lock per existing patterns). Adding direct dep will lock versions, no workspace rebuild beyond `agentctl`.
- **idna crate API (strict path — do NOT use permissive `domain_to_ascii`):**
  - Use `idna::domain_to_ascii_strict(&str) -> Result<String, Errors>` which internally does `Uts46::new().to_ascii(domain.as_bytes(), AsciiDenyList::STD3, Hyphens::Check, DnsLength::Verify)` — this gives nontransitional + STD3 + Bidi/Joiner + hyphen checks + DNS length. Alternatively for custom: `idna::uts46::Uts46::new().to_ascii(domain.as_bytes(), AsciiDenyList::STD3, Hyphens::Check, DnsLength::Verify)`.
  - Flags match required: `use_std3_ascii_rules=true` (`STD3`), `transitional=false` (nontransitional is default, crate never does transitional), `check_hyphens=true`, `check_bidi=true` (always true in crate), `check_joiners=true` (always true). Do NOT use `domain_to_ascii` (`Allow`/`Ignore`) — too permissive.
- **unicode-security crate API (UTS39 linter):**
  - `unicode_security::general_security_profile::GeneralSecurityProfile`, `unicode_security::mixed_script::MixedScript`, `unicode_security::confusable_detection::skeleton`, `unicode_security::restriction_level::RestrictionLevelDetection`.
  - Policy linter: compute `skeleton(domain)` for each UTS46-derived A-label vs existing allow/deny A-labels; if skeletons equal but strings differ → confusable. Also `is_single_script` / `detect_restriction_level` — warn if `HighlyRestrictive` or `MinimallyRestrictive` etc. Provide **WARNING** (not hard error) per spec: mixed-script/confusable warnings are authorization-bypass risk in policy context.
- **Pipeline location:**
  - Gate in `validation.rs` early, before coverage/specificity: resolve `policy.idna.mode` via ladder first, then for each `domains` entry: if `mode==reject` and entry contains non-ASCII (any byte `>127`) → hard validation error with precise diagnostic: codepoint `U+XXXX`, byte index, punycode suggestion via `idna::domain_to_ascii_strict` attempt OR static punycode hint (call `idna` even in `reject` mode just for suggestion). If `mode==uts46` → run `idna` strict path; on `Err` → hard validation error (same codepoint index + reason: Bidi/Joiner/STD3/length). On `Ok` → replace domain with A-label (punycode) for matching (store canonical ASCII). Then run `unicode-security` linter: `skeleton` compare + mixed-script check; emit **WARNING** `stderr` + provenance tag `idna_warning` (not blocking plan unless future strict flag). Provenance retains original Unicode for display (`idna_original`).
- **Cost:**

  | Area | Effort |
  |------|--------|
  | Deps | 2 (`idna` `1.1` already in tree, `unicode-security` tiny tables) |
  | `validation.rs` | ~200-300 LOC (ladder resolve + strict path + linter + diagnostics) |
  | Compiler wiring | ~100 LOC (A-label canonicalization, provenance `idna_original`/`idna_warning`) |
  | Tests | `§11` vectors + `§3.5` ladder `final` sealing |
  | Binary size | negligible (`idna` already linked transitively; `unicode-security` small) |
  | Runtime overhead | none beyond `plan`/`validate` (no per-packet cost) |

- **Test vectors (must be in ADR §11.1 and `cargo test`):**
  - Cyrillic `і` (`U+0456`) `gіthub.com` vs `github.com` confusable **warn** (skeleton equal, WARNING not error, plan continues).
  - `ß` deviation (`U+00DF` — nontransitional keeps `ß` not `ss`, verify strict path maps via punycode, not transitional `ss`).
  - `ZWNJ` `U+200C` in joiner-invalid position → hard error (`Bidi`/`Joiner` contextual failure).
  - Legit IDN `münchen.de` → `xn--mnchen-3ya.de` round-trip + suffix `.münchen.de` coverage (suffix A-label covers apex + subdomains).
  - Mixed-script warn: `paypal.com` vs `pаypal.com` (Cyrillic `а` `U+0430`) → WARNING.
  - Hyphen edge `3rd-4th` hyphen (`Check`) violation → hard error.
  - Length `>63` label error → hard error (`DnsLength::Verify`).

No part of the pipeline was implemented in the earlier draft; the design is now locked for the **v1 engine phase** (§6 Phase 1).

### 12.2 Committed follow-up — registry.schema.json + tombi mapping (APPROVED)

**Moved from open question (§15 item 7) to committed follow-up — APPROVED.**

The registry home file (`<home>/config.toml`, `Registry` in `control/agentctl/src/config/registry.rs`) already derives `schemars::JsonSchema` on its `PolicyConfig` fragment but has no vendored schema distribution.

**Committed work (follow-up PR, not this ADR's PR):**

1. Generate via `schemars::schema_for!(Registry)` (same pattern as `generate_schema_pair()` in `control/agentctl/src/commands/diagnostics.rs:1017`) → vendored `schemas/registry.schema.json`.
2. `workestrate generate-schema --output-registry schemas/registry.schema.json` (new flag) + `workestrate schemas update` distributes it.
3. **HOME_TOMBI_TOML** — a separate `tombi.toml` that governs `<home>` files (distinct from the repo `workestrate/tombi.toml` which gates `config.reference/workestrate.toml`). Add:

   ```toml
   # <home>/tombi.toml  (or repo template for it)
   [[schemas]]
   path = "schemas/registry.schema.json"
   include = ["config.toml", "overrides.toml"]
   # plus existing schemas/workestrate.schema.json for layered workestrate.toml if desired
   ```

   `tombi lint` / `tombi format` for home files then uses `strict = true` against the vendored schema, same as the repo gate. This closes the gap noted in `docs/runtime-provisioning.md` ("Follow-up gap: Registry.policy ...").

Tracked in [`../shelved-todo.md`](../shelved-todo.md) as committed (not shelved) and in [`./0035-design-notes.md`](./0035-design-notes.md) §5 (Open questions & resolutions, item 7).

## 13. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Hard cutover in one PR** — all workloads rewritten at once; a missed workload breaks `plan` | `validate-config` / `plan` fails closed; no partial rollout | exhaustive workload table (§9.2) + `validate-config` CI gate + golden-plan pins |
| **Operator mis-seals with deny-all-final** — home final locks every config/workload allow, breaking new vendors | fail-closed deny; visible via `warn` logs and provenance | `on_conflict=warn` per-ladder (see §8) + provenance inspection; operator docs in `docs/runtime-provisioning.md` |
| **Port-optional deny semantics confusion** — omitted port = all ports is more permissive than intended if operator meant single port | over-broad deny could shadow a lower allow unexpectedly | docs + examples (§3.3, §5 #19, §5A.4); validation error hints; specificity + port-scope tests (§11) |
| **Allow-all as least-specific is surprising** — `all=true` plus a higher suffix deny → deny wins (expected) but may confuse authors expecting allow-all to dominate | least-specific rank makes allow-all a fallback, not an override | invariant #22 + cookbook + tests |
| **Schema drift silent if not run** — new types change the generated schema; stale committed schema confuses editors | schemastore lint vs format-only drift | `schemas update` flow + `validate-config` P3 staleness warning (deferred consumer-copy warn, see shelved-todo) + CI drift guard + `registry.schema.json` committed follow-up (§12.2) |
| **Presets dropped may annoy** — no `preset = "github"` shorthand | inline is verbose for common bundles | revisit if ergonomic pain is demonstrated (see [`../shelved-todo.md`](../shelved-todo.md)) |
| **Ingress scope vocabulary confusion (bind vs policy)** | Operator conflates `bind = wildcard` with `scope = public` | §3.4.2 bind-vs-policy table + `reachability = bind ∧ policy`; provenance records both; §11 tests pin it |
| **IDNA flag-gated risk — operator enables `uts46` and introduces confusable bypass** | Operator enables `mode = "uts46"` and authorizes `gіthub.com` (Cyrillic `і`) or mixed-script `pаypal.com` thinking it is `github.com`/`paypal.com` — attacker-controlled A-label grant; or `ZWNJ`/`ß`/hyphen/length edge mis-handled | **Default `reject`** (fail-closed, explicit opt-in only); **UTS39 warnings** (skeleton/mixed-script `idna_warning` provenance) require author review; strict `ToASCII` (STD3+`Check`+`Verify`) hard-errors on Bidi/Joiner/hyphen/length; test vectors (`§11.1`) pin `cyrillic і`, `ß`, `ZWNJ`, `münchen.de` round-trip, mixed-script, hyphen, length (§12.1) |

## 14. Alternatives considered

| Alternative | Verdict | Rationale |
|-------------|---------|-----------|
| **Status quo — `ALLOWED_EGRESS_HOSTS` const + recipes** | **Rejected** | Locks every user to maintainer's personal vendor list; requires Rust change to add a host (real case: `api.tempo.io`); leaks personal infra choices into a public binary; conflates supply-chain boundary with operator authority. |
| **Presets kept (e.g., `preset = "github"` expanding to domain lists)** | **Rejected (dropped; may revisit)** | A second allowlist with the same governance problem; inline domains are explicit, grep-able, and reviewable; presets hide policy in a registry that must itself be governed. Shelved (see [`../shelved-todo.md`](../shelved-todo.md)). |
| **Operator-only `final=allow` (allow-final only at home, deny-final anywhere)** | **Rejected per user decision** | Symmetric `final` on both polarities is chosen. Deny-final and allow-final both obey invariant #11; operator authority is about rung height, not polarity asymmetry. Mount-policy's final-allow-is-operator-only trust gate (§5. #7 trust gate) does not transfer — network policy's fail-closed cross-rung tie already gives deny the edge (#9). |
| **Wildcard domains (`*.evil.com`) + scheme + path matching** | **Rejected** | Wildcard dialect ambiguity, scheme conflation with port, and path matching implies L7 proxy semantics not present. Suffix form `.evil.com` (apex-inclusive) covers the needed carve-outs without dialect debt. Path-level carve-outs are shelved for the proxy future. |
| **Discrete `package` policy (ALLOWED_PACKAGES ladder)** | **Shelved, not rejected** | Same ladder solution as egress, but separate change. Doing both at once doubles blast radius (see [`../shelved-todo.md`](../shelved-todo.md)). |
| **Allow port-optional (hidden 443 default) / Deny port-required** | **Rejected** | Allow-without-port hides a default (see §3.3 asymmetry rationale); deny without port is the fail-closed broad form. Chosen: allow port REQUIRED, deny port optional = any-port. |
| **Ingress as bind addresses** | **Rejected** | Scope as bind conflates kernel bind with peer filter. Chosen: scope = peer-source group (§3.4) orthogonal to `bind`. |

## 15. Open questions for final iteration — RESOLVED (disposition table)

All 7 original open questions are dispositioned below. Full resolution log is in [`./0035-design-notes.md`](./0035-design-notes.md) §5; deferred items are in [`../shelved-todo.md`](../shelved-todo.md).

| # | Original question (§15 v1) | Disposition | Where incorporated |
|---|-----------------------------|-------------|--------------------|
| 1 | **Deny port-optional semantics confirm** — port omitted = all ports load-bearing; confirm no need for `all_ports = true` flag | **CONFIRMED & INCORPORATED** | §3.3 entry table + asymmetry rationale, §5 #19, §6 compiler order, §5A.4 builder mapping, §11 tests |
| 2 | **Host-bridge deny exposure** — should host-bridge (dns 53, litellm proxy) be deny-able per-port? | **CONFIRMED & INCORPORATED** — deny is domain-oriented (`egress.deny.domain`); host-bridge deny is not surfaced separately; per-workload removal of inherited `allow.host` is the mechanism (documented). No new `deny.host` table in v1. | §3.3 (deny host not surfaced), §10 |
| 3 | **on_conflict per-ladder granularity** — top-level default vs per-table explicit | **CONFIRMED & INCORPORATED** — `on_conflict` stays **per-polarity-table, per-ladder granular** (egress vs ingress separate, overridable, `final` seals). No top-level `policy.on_conflict` default — per-table explicit is reviewable. | §3.2, §5 #14, §6, §8 (per-ladder) |
| 4 | **Ingress `all` lint** — should `ingress.allow all=true` warn even when `on_conflict=ignore`? | **DEFERRED → shelved-todo** — lint is deferred; `all=true` on ingress remains legal and least-specific (§5 #22). Deferred to avoid scope creep. | [`../shelved-todo.md`](../shelved-todo.md) (ingress-all-lint entry) |
| 5 | **IDNA future** — should error auto-suggest punycode via `idna` crate? | **DESIGNED — FLAG-GATED DEFAULT-OFF, IMPLEMENT IN V1** — `mode = "reject"` (default, hard error with codepoint + punycode hint via `idna::domain_to_ascii_strict`) \| `"uts46"` (strict `ToASCII` with `STD3`/`Check`/`Verify` + `unicode-security` skeleton/mixed-script WARNINGs), ladder with `final` (§3.5). Test vectors §11.1; implementation reality §12.1; shelved entry 5 now designed flag-gated default-off — implement in v1. | §3.5 + §5#17 + §11.1 + §12.1 + [`../shelved-todo.md`](../shelved-todo.md) entry 5 resolution |
| 6 | **Deprecation of `default_deny` already done** — confirm no legacy `workestrate.toml` carries it; should error point to ADR? | **ELABORATED IN ADR** — confirmed removed; error MUST cite ADR 0035 (`see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)`) per §9.4 convention. Same convention for `read_only` deprecation warning and future policy migrations. | §9.4 (new), §10, §7 gate 1, §11.2 |
| 7 | **Registry schema gap follow-up** — `registry.schema.json` + `validate-config --home` vs `doctor`? | **APPROVED — COMMITTED FOLLOW-UP** — moved from open question to committed: `schemars::schema_for!(Registry)` → `schemas/registry.schema.json` + `HOME_TOMBI_TOML` `[[schemas]]` for `config.toml`/`overrides.toml`. Not shelved. | §10, §11.5, §12.2 (new), [`../shelved-todo.md`](../shelved-todo.md) (as committed note), [`./0035-design-notes.md`](./0035-design-notes.md) §5 item 7 |

**Disposition counts (updated 2026-09-03 iteration 3):** 3 confirmed-incorporated (#1, #2, #3), 1 deferred-to-todo (#4), 1 elaborated-in-ADR (#6), 1 designed-flag-gated (#5) — implement in v1, 1 approved-follow-up (#7). Original v1 was 3-1-2-1; row 5 now designed per user decisions lock.

---

*Document companions:* `docs/runtime-provisioning.md` — three MSB homes, wrapper contract, env override matrix, schema flow, formatting pointer. Design notes: [`./0035-design-notes.md`](./0035-design-notes.md). Shelved follow-ups: [`../shelved-todo.md`](../shelved-todo.md). This ADR's network policy is independent of runtime provisioning but shares the ladder precedent and schema machinery.

*Style note:* This ADR matches the structure/tone of ADR 0034 (status/date/references, context → options → decision → migration → open questions) and ADR 0033 (classification tables, rejected-vs-selected framing).
