# Language Guidance Index

## Purpose

This index is the top-level map across the BEAM/Elixir/Gleam/Nix guidance corpora. It explains how the four corpora relate, where to look for a given topic, and the invariant that keeps the language corpora independent. Start here when you are unsure which corpus owns a topic. Nix is infrastructure/tooling, not a language — it provides the build environment for all language code.

## Corpus model

```
docs/beam/   = shared runtime / OTP source of truth (Erlang-native, source-verified)
docs/elixir/ = Elixir-specific; references docs/beam/ for shared runtime
docs/gleam/  = Gleam-specific; references docs/beam/ for shared runtime
docs/nix/    = infrastructure/tooling corpus; the build environment for all language code (NOT a language)
```

Elixir and Gleam are SIBLINGS — neither depends on the other. Both defer to `docs/beam/` for the underlying Erlang/OTP runtime semantics; neither language corpus reads from the other. Nix is the foundational build tool — all Rust/Elixir/Gleam code is built inside the Nix devshell — and Nix cross-references all language corpora for packaging concerns.

## Where to look

- **Runtime / OTP semantics** (processes, supervision, exits, apps, releases, logger, distribution, ports, NIFs, ETS, timers) → `docs/beam/`
- **Elixir syntax, Mix, ExUnit, GenServer/Supervisor Elixir APIs, Credo/Dialyxir, Ecto/Phoenix** → `docs/elixir/`
- **Gleam syntax, gleam CLI/gleam.toml, gleam_otp actors/supervisors, gleam_erlang interop, gleam_javascript, gleeunit** → `docs/gleam/`
- **Nix flakes, devshells, derivations, overlays, packaging, cross-compilation, Nix store/GC, CI/CD, Docker images** → `docs/nix/`

## Corpus summaries

- **docs/beam/** — 22 topic docs, 47 crawled sources, 10 skills (4 constraints + 6 runtime-diagnosis), 1 workflow (runtime-diagnosis only); the shared runtime foundation. BEAM is the complementary reference+invariants layer, not a workflow target.
- **docs/elixir/** — 15 topic docs, 15 skills (8 operational + 2 constraints + 5 validations), 5 workflows; Elixir-specific, aligned to `docs/beam/`.
- **docs/gleam/** — 19 topic docs, 8 skills (3 operational + 2 constraints + 3 validations), 6 workflows (5 SDLC + interop); Gleam-specific, references `docs/beam/`.
- **docs/nix/** — 25 topic docs, 73 crawled sources, 27 skills (14 operational + 6 constraints + 7 validations), 7 workflows; the foundational build tool. Nix is NOT a language — it is the infrastructure/tooling corpus that provides the build environment for all language code and cross-references Rust/Elixir/Gleam for packaging concerns.

> Doc counts are topic docs (corpus `.md` files excluding `index.md` and `source-map.md`), matching each corpus's own self-description. Workflow counts are prose workflow docs (including each `workflows/index.md`); each prose workflow also has a corresponding skill package (see Workflows map). BEAM is NOT a language — it has no implementation/code-review/refactoring/debugging/validation workflows; those live in Elixir/Gleam. BEAM's single workflow, runtime-diagnosis, is the cross-language operational layer.

## Skills map

Skills are grouped by layer: **operational** (domain how-to), **constraints** (in-skill rules enforced during execution), and **validations** (gate-running skills). All live under `.agents/skills/`.

- **BEAM-common operational (8):** `beam-supervision`, `beam-gen-server`, `beam-gen-statem`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`, `beam-observability-debugging`
- **BEAM-common constraints (4):** `constraint-beam-supervision`, `constraint-beam-failure`, `constraint-beam-process-isolation`, `constraint-beam-nif-safety`
- **Elixir operational (8):** `elixir-coding`, `elixir-config`, `elixir-docs-publishing`, `elixir-error-handling`, `elixir-otp`, `elixir-project-setup`, `elixir-static-analysis`, `elixir-testing`
- **Elixir constraints (2):** `constraint-elixir-style`, `constraint-elixir-otp-api`
- **Elixir validations (5):** `validation-elixir-compile`, `validation-elixir-credo`, `validation-elixir-dialyzer`, `validation-elixir-format`, `validation-elixir-test`
- **Gleam operational (3):** `gleam-language`, `gleam-otp-interop`, `gleam-packages-ffi`
- **Gleam constraints (2):** `constraint-gleam-result`, `constraint-gleam-conventions`
- **Gleam validations (3):** `validation-gleam-check`, `validation-gleam-format`, `validation-gleam-test`
- **Nix operational (14):** `nix-language`, `nix-flake-anatomy`, `nix-derivations`, `nix-devshells`, `nix-modules`, `nix-packaging-recipes`, `nix-nixpkgs-library`, `nix-overlays`, `nix-testing`, `nix-store-gc`, `nix-cross-compilation`, `nix-ci-cd`, `nix-usage`, `nix-docker-images`
- **Nix constraints (6):** `constraint-nix-purity`, `constraint-nix-reproducibility`, `constraint-nix-store-hygiene`, `constraint-nix-secret-hygiene`, `constraint-nix-sandbox-safety`, `constraint-nix-scope-discipline`
- **Nix validations (7):** `validation-nix-flake-check`, `validation-nix-build`, `validation-nix-format`, `validation-nix-lint`, `validation-nix-eval`, `validation-nix-test`, `validation-nix-supply-chain`

> **Shared layer:** the BEAM-common constraints and the `docs/beam/` runtime docs apply to Elixir and Gleam when targeting the Erlang/OTP runtime. Per-language constraints and validations are toolchain/language-specific (e.g. Credo/Dialyxir for Elixir, `gleam check`/`gleam format`/gleeunit for Gleam).
>
> **Gleam-on-JS target diverges:** the BEAM constraints and `docs/beam/` runtime docs do NOT apply when Gleam targets JavaScript; concurrency is `gleam/javascript/promise`, not OTP processes/links/supervisors.
>
> **Nix is the cross-cutting build layer:** Nix is infrastructure, not a language. The Nix operational/constraint/validation skills apply to the build environment that produces all language artifacts; Nix packaging recipes cross-reference Rust/Elixir/Gleam for per-language packaging concerns.

## Workflows map

Workflows chain across the SDLC. Each prose workflow doc in `docs/<lang>/workflows/` has a corresponding **workflow skill package** under `.agents/skills/` — `workflow-<lang>-<name>-00-orchestration` … `-05-<phase>` (Shape A, mirroring `workflow-rust-*`). Phases: `00-orchestration`, then `01..05` per workflow. Orchestrations declare their `next_workflow` (chaining).

### SDLC chaining graph

```
implementation ─┐
refactoring ────┼─► code-review ──(approve)──► validation ──► stop
interop (Gleam)─┘        │
                    (changes requested) ─► implementation | refactoring | debugging
debugging ──────────────────────────────► validation ──► stop
runtime-diagnosis (BEAM) ──(code defect)──► debugging (elixir/gleam) ; (operational) ─► stop
```

### Workflows per corpus

- **BEAM (1 workflow):** `docs/beam/workflows/runtime-diagnosis.md` + index. Skill package: `workflow-beam-runtime-diagnosis-*`. BEAM is NOT a language — no implementation/code-review/refactoring/debugging/validation workflows at BEAM; those live in Elixir/Gleam.
- **Elixir (5 workflows):** `docs/elixir/workflows/` — implementation, code-review, refactoring, debugging, validation + index. Skill packages: `workflow-elixir-{implementation,code-review,refactoring,debugging,validation}-*`.
- **Gleam (6 workflows):** `docs/gleam/workflows/` — implementation, code-review, refactoring, debugging, validation, **interop** + index. Skill packages: `workflow-gleam-{implementation,code-review,refactoring,debugging,validation,interop}-*`.
- **Nix (7 workflows):** `docs/nix/workflows/` — implementation, code-review, refactoring, debugging, validation, **packaging**, **hardening** + index. Skill packages: `workflow-nix-{implementation,code-review,refactoring,debugging,validation,packaging,hardening}-*`. Nix is NOT a language — its workflows govern the build/packaging environment that produces all language artifacts.

Note: BEAM's `runtime-diagnosis` is the one cross-language workflow; it hands off to a concrete language's `debugging` when a code defect is found, or stops when the issue is purely operational. Gleam's `interop` is new and Gleam-only. Nix's `packaging` and `hardening` workflows are Nix-only.

## Sibling independence

**Invariant:** Elixir docs reference `docs/beam/` but NOT `docs/gleam/`; Gleam docs reference `docs/beam/` but NOT `docs/elixir/`. The two language corpora are independent and meet only at the shared `docs/beam/` runtime layer. The validation pass confirms this. Nix is orthogonal to the Elixir/Gleam sibling-independence invariant: Nix cross-references all language corpora for packaging but does not break the language-corpus independence invariant.

## Related indexes

- [docs/beam/index.md](beam/index.md)
- [docs/elixir/index.md](elixir/index.md)
- [docs/gleam/index.md](gleam/index.md)
- [docs/nix/index.md](nix/index.md)
