# ADR 0020: Review adjudications (2026-07)

**Status:** Accepted
**Date:** 2026-07-19
**Supersedes (partially):** none — clarifies ADR 0005 and ADR 0014.
**References:** `80-remediation-plan.md` (the implementation plan).

## Context

A five-way parallel investigation (Rust core, Nix, security, docs coherence,
tooling gaps) plus main-lead synthesis reviewed `migration/tool-model` @
`12e89b6`. The review confirmed 17 of 18 spot-checked findings against the
cited code and surfaced four design tensions that required authoritative
interpretation of the existing ADRs.

The four rulings below settle those tensions. They do not introduce new
architecture — they pin the correct reading of ADR 0005 (merge semantics),
ADR 0014 (trust gating), and the operator-trust model implied by the CLI
design.

This ADR exists so that the rulings survive independent of the
`80-remediation-plan.md` document, which is the execution artifact.

## Ruling 1 — env union-by-name

**Closes:** review finding A3 (env REPLACE silently drops base vars).

**Decision:** `workloads.<name>.env` merges **union-by-name** (last-write-
wins per env-var key), matching the existing `secret_env` union-by-secret-
name pattern (`merge.rs:294-303`). The other non-security REPLACE lists
(`ports`, `mounts`, `seed_files`, `local_build`, `network.ingress`) stay
REPLACE — they are list-of-rows where partial replacement is ambiguous.

**Why:** env-vars are name-keyed, like secret_env. Every mature config system
(k8s EnvVar, docker-compose `environment`, helm) overrides by name. The
current REPLACE semantics silently drop base-layer vars when an override
adds one — surprising and error-prone. ADR 0005's REPLACE-list rule is
correct for list-of-rows; env is the one name-keyed list that was
misclassified.

**Implementation:** `80-remediation-plan.md` WP6, finding A3.

## Ruling 2 — entitlement checked before monotonic-true

**Closes:** review finding A4 (entitled workloads cannot relax
`default_deny` `true -> false`).

**Decision:** In `merge.rs::merge_network`, the entitlement check (`if !
DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&name)`) runs **before** the
monotonic-true check. An entitled workload (`tempest`, `example-offensive`)
may relax `default_deny` from `true` to `false` at a higher layer.
Monotonic-true is preserved as defense-in-depth for non-entitled workloads
(they can never reach `Some(false)` at all, because the entitlement check
rejects them first).

**Why:** `DEFAULT_DENY_FALSE_ENTITLEMENT` is the explicit, core-blessed
opt-out from `default_deny=true`. It exists *precisely* because tempest is
an offensive-security workload requiring network egress. If monotonic-true
fires first, any lower layer (the shipped base config, an operator-
hardening override, anything) that sets `default_deny=true` for tempest
permanently breaks tempest — defeating the entitlement mechanism. ADR 0005
specified "monotonic `default_deny` + core per-workload entitlement for
`false`" — the entitlement is the exception that proves the rule.

Note: `validate_config` (`config.rs:1067-1076`) already gets the order
right. Only `merge_network` had it inverted.

**Implementation:** `80-remediation-plan.md` WP3, finding A4.

## Ruling 3 — operator-trust model: document, do not harden (with local.toml exception)

**Closes:** review findings C11 (`workestrate run` loads all secrets) and
C12 (`WORKESTRATE_CONFIG_DIR` bypasses trust gating).

**Decision:** Both `workestrate run` and `WORKESTRATE_CONFIG_DIR` are
operator-trust escape valves. They are documented as such in a holistic
"Trust model" section of the README. They are **not** hardened further.

- **`workestrate run -- <cmd>`** loads every secret via `load_secrets()`
  and exec's `<cmd>` with them in env. This is intentional: the operator
  invoking `run` already has host shell and can `cat .env.enc` directly.
  Filtering secrets would be security theater. A one-time stderr warning
  fires when `run` loads more than a threshold count of secrets, pointing
  at the README section. Suppressible via
  `WORKESTRATE_NO_RUN_WARNING=1`.
- **`WORKESTRATE_CONFIG_DIR`** is an env-var-as-root: whoever controls the
  environment controls the process (the Unix model — cf. PATH, LD_PRELOAD).
  Hardening it would break legitimate CI and ad-hoc-override workflows.
  The README states explicitly: "Setting `WORKESTRATE_CONFIG_DIR` grants
  the workestrate process full config authority — treat it like setting
  PATH."

**Exception — `workestrate.local.toml`:** Unlike the env var, the local
layer is auto-discovered from cwd on every invocation. A hostile `git
clone` followed by `cd` and any `workestrate` invocation would compromise
the host without any explicit operator action. The local layer IS hardened
— it is added to the same trust gate (`is_trusted_project`) that applies
to `workestrate.toml` per ADR 0014. (Closes review finding A1; see
`80-remediation-plan.md` WP1.)

**Why:** A trust model that pretends to harden what it cannot harden is
worse than an honest documented one. The operator-trust boundary is "host
shell access = full authority"; `run` and `WORKESTRATE_CONFIG_DIR` sit
inside that boundary. The auto-discovered `local.toml` sits *outside* the
explicit-operator-action boundary and must be gated.

**Implementation:** `80-remediation-plan.md` WP1 (A1) + WP7 (C11/C12
documentation + run warning).

## Ruling 4 — spec-code consistency CI as a standing guard

**Closes:** review finding D1 (spec uses `rw`, code requires `read_only`,
in 12+ places).

**Decision:** A `spec_examples_parse` integration test is added to
`control/agentctl/tests/`. It extracts every fenced TOML block from
`docs/migration/20-target-system-spec.md`, parses each against the current
Rust schema (`ConfigFile` and sub-structs), and asserts no errors. Blocks
that are intentionally fragments are marked with a leading
`# spec-test: skip` comment. The test runs as part of `just verify` on
every change.

**Why:** D1 is the headline spec-code drift, but the pattern is general:
the migration is too large to keep spec and code in sync by hand. A trivial
CI guard (parse the spec examples against the schema) catches this class
of drift in seconds. The `# spec-test: skip` mechanism keeps intentional
fragments parseable while still flagging them for review.

**Implementation:** `80-remediation-plan.md` WP4.

## Consequences

- The merge engine has two name-keyed union lists (`env`, `secret_env`)
  and four list-of-rows REPLACE lists (`ports`, `mounts`, `seed_files`,
  `local_build`, `network.ingress`). Documented in ADR 0005's addendum.
- Entitled workloads can relax `default_deny` `true -> false`; non-entitled
  cannot reach `false` at all. The monotonic-true invariant is preserved
  as defense-in-depth for non-entitled workloads.
- The operator-trust model is stated once in the README; future features
  that look like escape valves cross-reference it.
- Spec drift is caught by CI; future spec edits that introduce
  unparseable TOML fail `just verify`.
- ADR 0005 gets an addendum pointer (not a rewrite) — its core decisions
  stand; this ADR clarifies two details.
- ADR 0014 is extended (not superseded) — its trust gate now also applies
  to `workestrate.local.toml`.

## Rejected why

- **Harden `run` and `WORKESTRATE_CONFIG_DIR` (Ruling 3 Option B):**
  rejected because it breaks the operator-escape-hatch use case (`run` is
  explicitly for "I have host shell and want full secret access") and CI
  workflows (which set `WORKESTRATE_CONFIG_DIR` to fixtures). The
  hardening is security theater given the existing host-shell trust
  boundary.
- **Strict REPLACE everywhere (Ruling 1 alternative):** rejected because
  it is the current state and is wrong for name-keyed env-vars.
- **Strict union everywhere:** rejected because list-of-rows (ports,
  mounts) need REPLACE semantics for sensible overrides.
- **Rewrite ADR 0005:** rejected; ADR 0005's decisions are correct. This
  ADR is a clarification, recorded as an addendum pointer in 0005.
