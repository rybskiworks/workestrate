---
name: workflow-gleam-code-review-04-review
description: |
  Use only for the review phase of the Gleam code-review workflow. Perform the
  human-judgment review using per-doc checklists across Result/error handling,
  conventions, FFI/OTP, API design, and documentation. Do not use for
  scoping, analysis, running gates, or issuing a verdict.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-code-review
  org.phase: review
  org.phase_order: "04"
---

# Phase 04: review (Gleam code review)

## Phase purpose

Perform the human-judgment review using the per-doc checklists across
Result/Option/error handling, conventions, externals/FFI, OTP/actor/supervision
(on the Erlang target), API design, and documentation. Walk each topic doc's
`## Review checklist` section explicitly and cite items rather than
paraphrasing.

## Steps to perform

1. **Result/Option and error handling** — consult `docs/gleam/result-option-and-errors.md`
   and the `gleam-language` / `constraint-gleam-result` skills. Look for
   `panic`/`let assert`/`todo` on recoverable conditions, swallowed `Result`
   values, `Option` used as a fallible return type, invented `result`/`option`
   function names, and missing `Result` chains.
2. **Conventions and type design** — consult `docs/gleam/conventions-patterns-antipatterns.md`,
   `docs/gleam/types-records-and-patterns.md`, and the `gleam-language` /
   `constraint-gleam-conventions` skills. Look for unqualified function imports,
   missing type annotations on public functions, catch-all `_` patterns that
   disable exhaustiveness, fragmented modules, abbreviations, and representable
   invalid states.
3. **Externals/FFI safety** (only if externals/FFI are present) — consult
   `docs/gleam/externals-and-ffi.md` and the `gleam-otp-interop` /
   `gleam-packages-ffi` skills. Verify `@external` declarations, target-specific
   bodies, data mapping across the FFI boundary, and the absence of `Dynamic`
   used as an FFI type. Note that both Erlang and JavaScript targets must be
   reviewed when both are supported.
4. **OTP/actor/supervision** (only if the diff touches OTP/actors/supervision on
   the **Erlang target**) — consult `docs/gleam/otp-actors-and-supervision.md`,
   `docs/gleam/erlang-interop.md`, `docs/beam/supervision.md`, and
   `docs/beam/processes-and-messages.md`, plus the `gleam-otp-interop` skill.
   Check actor lifecycle, child specs, restart strategies, `process.new_name`
   atom usage, and `send`/`call` panic semantics. On the **JavaScript target**,
   BEAM docs do not apply; concurrency is `gleam/javascript/promise`, not BEAM
   processes/OTP.
5. **API design / package surface** — consult `docs/gleam/conventions-patterns-antipatterns.md`
   and the `gleam-packages-ffi` skill. Check naming, `pub opaque type` usage,
   internal modules, public function annotations, and documentation on the
   public API.
6. Walk each topic doc's `## Review checklist` section explicitly; record
   pass/fail/N-A per item. **Cite checklist items; do not paraphrase them.**

## Docs to consult

- `docs/gleam/conventions-patterns-antipatterns.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/types-records-and-patterns.md`

If the diff touches OTP/actors/supervision or Erlang interop, also consult:

- `docs/gleam/otp-actors-and-supervision.md`
- `docs/gleam/erlang-interop.md`
- `docs/beam/supervision.md`
- `docs/beam/processes-and-messages.md`

On the JavaScript target, the BEAM docs do not apply; concurrency is
`gleam/javascript/promise`, not BEAM processes/OTP.

## Operational skills to load

- `gleam-language`
- `gleam-otp-interop` (if OTP/FFI/Erlang interop present)
- `gleam-packages-ffi`

## Constraints to apply

- `constraint-gleam-result` — enforce Result/Option boundaries; flag
  `panic`/`let assert` on recoverable conditions and swallowed errors.
- `constraint-gleam-conventions` — enforce naming, imports, annotations,
  module structure, and anti-patterns.
- `constraint-beam-supervision` / `constraint-beam-failure` /
  `constraint-beam-process-isolation` — apply **only** if the diff touches
  OTP/actors/supervision/interop on the **Erlang target**. On the JavaScript
  target these do not apply.
- Scope discipline — review only the diff; do not request changes to untouched
  code.

## Validations to run

None — this is a manual review phase that uses per-doc checklists. Automated
validations ran in phase 03 (`workflow-gleam-code-review-03-check`).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-gleam-code-review-00-orchestration`. Set:

- `outcome`: `pass` if no findings; `partial` if findings exist but none are
  blockers; `fail` if a blocker-level finding was identified.
- `constraints_applied`: all constraints listed above that applied.
- `risks`: each finding with file:line, the checklist item or doc rule
  violated (cited), and a concrete suggested fix. Severity classification
  happens in phase 05-verdict; here record the raw findings.
- `assumptions`: any N-A checklist items and why.
- `next_phase`: `05-verdict`.
- `next_workflow`: `null`.
