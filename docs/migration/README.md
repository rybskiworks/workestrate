# Migration design records

**Current integration branch:** `main`, after [PR #32](https://github.com/rybskiworks/workestrate/pull/32).
**Historical migration branch:** `migration/tool-model`.
**Original record:** authored 2026-07-18 from `840e8b7`; later ADR amendments are dated individually.

This tree preserves the migration design and its rationale. The former branch is
not a second integration or release channel. Older references to pending approval,
phase gates or host verification describe the state of the record at that time;
they are not instructions to repeat a migration or proof of current test results.

Start with the current [specification map](../../SPEC.md),
[operating model](../operating-model.md) and [runtime guide](../runtime-provisioning.md).
Use the ADRs and their dated amendments to understand the decisions behind them.

## Reading order

| Record | Purpose |
| :--- | :--- |
| [Executive summary](00-executive-summary.md) | Original migration goals and scope. |
| [Current-state snapshot](10-current-state.md) | The dated baseline, not a rolling deployment report. |
| [Target system](20-target-system-spec.md) | Detailed system semantics, read with ADR amendments. |
| [Security model](30-security-model.md) | Threat model, invariants and enforcement points. |
| [Migration process](40-migration-process.md) | Phases, consequence sweeps and historical gates. |
| [ADR index](50-decisions/README.md) | The maintained decision inventory. |
| [Glossary](60-glossary.md) | Shared vocabulary. |
| [Open items](70-open-items.md) | Residual questions and gates to reconcile with current work. |
| [Remediation plan](80-remediation-plan.md) | Historical review findings and their disposition. |

Do not infer a fresh Nix or KVM validation from these documents. The initial
record was authored without those runtime capabilities; later evidence must be
read with its exact source revision, host and test scope. Implementation changes
now go through PRs to main with the affected contracts and tests updated together.
