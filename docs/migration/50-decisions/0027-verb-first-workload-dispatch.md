# ADR 0027: Verb-first workload dispatch

**Status:** Accepted
**Date:** 2026-08-01
**References:** ADR 0006 (hybrid CLI dispatch, superseded by this ADR),
ADR 0021 (instance lifecycle), ADR 0026 (per-instance addressing + discovery).

## Context

The hybrid dispatch model (ADR 0006) puts workload names in `argv[1]` via the
clap `external_subcommand` catch-all. A workload named after a built-in
subcommand (`home`, `config`, `ps`, etc.) is **silently shadowed**: named
subcommands match before `external_subcommand` — verified in the
clap_builder 4.6.0 parser source (`possible_subcommand`/`find_subcommand`
exact-match runs before the external branch). Concretely,
`workestrate home up` errors as a `HomeAction` usage error and the workload
is permanently unreachable. No reserved-name validation exists in config
loading. Additionally, completions/help cover only the 5 typed workloads;
catch-all workloads are invisible to discovery.

## Options considered

1. **Reserved-name denylist** — reject config workloads named after built-ins.
   Rejected: constrains the config namespace, grows with every new verb,
   retroactively breaks previously valid configs.
2. **Prefix/namespace token** — e.g. a `workestrate wl <name> up` style
   prefix. Rejected: cryptic, inverts the precedent order, and `run` is
   already taken.
3. **Keep hybrid + document the limitation.** Rejected: maximizes future
   breakage on a pre-release branch — every future built-in becomes a new
   landmine.
4. **Grouped verb-first dispatch.** Selected.

## Decision

Grouped verb-first dispatch:

- `workestrate workload {up,exec,plan,down,logs} <name>` — workload names are
  ARGUMENTS, never subcommands.
- New `workestrate workloads` discovery verb: lists configured workloads with
  kind + running status (fills the discovery gap — today there is no way to
  list configured workloads; `ps` shows only running ones).
- Kind-check at dispatch: service-kind → `up`/`logs` (detached); agent-kind →
  `exec` (interactive); `plan`/`down` are universal. Wrong-kind usage produces
  a clear error (e.g. "pi is an agent; use exec").
- The catch-all (`external_subcommand`) is REMOVED; a one-cycle stderr alias
  shim warns on the old `workestrate <wl> <verb>` shape.
- Supersedes ADR 0006's hybrid decision.

## Consequences

- Collision-freedom by construction: `argv[1]` is a closed verb set; future
  built-ins can never break existing configs.
- Completions/help cover all workloads (verbs are typed; names are dynamic
  args).
- The dual parser (typed subcommands + raw matchers) is deleted.
- `detach_args` child argv flips to verb-first shape.
- Supersedes ADR 0006's hybrid decision — ADR 0006 is marked superseded
  (its Status line only).
- Precedent: `docker compose` (group + verb + name, `docker compose up
  <service>`) is the structural analogue for a multi-domain CLI;
  kubectl/systemctl flat verb-first belongs to single-domain tools.

## Rejected why

- **Reserved-name denylist:** constrains the config namespace, grows with
  every new verb, retroactively breaks previously valid configs.
- **Prefix/namespace token:** cryptic, inverts the precedent order, and `run`
  is already taken.
- **Keep hybrid + document the limitation:** maximizes future breakage on a
  pre-release branch — every future built-in becomes a new landmine.
