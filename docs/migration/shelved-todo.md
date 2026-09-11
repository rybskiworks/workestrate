# Shelved TODO — deferred items with context for future sessions

> **Scope:** This file tracks deferred (shelved) follow-ups that are NOT in scope for the current ADR/change. Each entry has what / why deferred / what future work looks like + ADR section reference, so a future session can pick up without re-derivation.
>
> **ADR 0035 companions:** ADR: [`./50-decisions/0035-hierarchical-egress-ingress-policy.md`](./50-decisions/0035-hierarchical-egress-ingress-policy.md) · Design notes: [`./50-decisions/0035-design-notes.md`](./50-decisions/0035-design-notes.md) · Runtime provisioning: `docs/runtime-provisioning.md`

## How to use

- **Shelved** = intentionally not in the current PR; revisit when the triggering condition is met.
- **Committed follow-up** (e.g. `registry.schema.json`) is tracked here for visibility but is NOT shelved — it is APPROVED and has an implementation plan.
- Each entry should be cheap to re-hydrate: keep the 2-3 lines of context; don't re-derive from scratch.

---

## Shelved items

### 1. Presets — dropped, inline-only; revisit if repetition emerges

- **What:** A preset registry (e.g. `preset = "github"` expanding to `domains = ["github.com","api.github.com"] port = 443`) was considered and rejected. Only inline `[[policy.egress.allow.domain]]` etc. exist.
- **Why deferred/dropped:** A preset is a second allowlist with the same governance problem as `ALLOWED_EGRESS_HOSTS`; inline is explicit, reviewable, grep-able, and avoids a registry that must itself be governed. Revisit only if vendor bundles prove ergonomic repetition (e.g. 10 workloads repeating the same 5-domain + port tuple).
- **Future work if revisited:** Define a preset catalog (likely per-config-repo or home-scoped), with explicit `preset = "github"` → expansion at collect time (before precedence), provenance showing the expansion, and a lint for stale presets. Needs a governance decision (who owns the preset list — operator vs config-repo author). See ADR 0035 §12 row, §13 risks, §14 alternatives.

### 2. Home-customizable defaults (`on_conflict` default, `final` defaults)

- **What:** Operator sets a global home-default knob (e.g. `[policy] on_conflict = "warn"` that lower rungs inherit, or a default `final` for a ladder).
- **Why deferred:** Per-polarity-table `on_conflict` (per-ladder granular, see ADR §3.2, §5 #14, §8) is already reviewable and explicit; a top-level default would add inheritance semantics and another precedence layer for little current value. Every rung can already set `on_conflict` on the ladder it governs; `final` seals the choice.
- **Future work:** Add an optional `[policy.defaults]` or `[policy] on_conflict = ...` (per-ladder) that populates `None` polarity tables at resolution time. Needs validation (no silent downgrade of `fail` to `ignore` across ladders) and a `final`-sealing story. See ADR 0035 §12 row.

### 3. ALLOWED_PACKAGES hierarchical treatment — same ladder model

- **What:** `ALLOWED_PACKAGES` in `control/agentctl/src/policy.rs` has the same flaw as `ALLOWED_EGRESS_HOSTS` (hardcoded const in public binary, operator not governing). The fix is the same ladder (`Registry.policy.packages` or `policy.packages` with `allow`/`deny` + `final` + `on_conflict` + provenance).
- **Why deferred:** Same solution, separate change — doing both at once doubles blast radius (two policy surfaces, two schema migrations, two workload tables). Egress/ingress is higher priority (blocks real workloads like `tempo_buddy`); packages are lower frequency.
- **Future work:** Mirror the egress ladder for `policy.packages` (or `policy.egress.packages` if scoped). Reuse the collect-and-compile compiler, invariants #11–#16, and §5A enforcement reality pattern (but for package allowlist, not SDK rules). See ADR 0035 §10 row, §12 row, §14 alternative.

### 4. Path-level carve-outs (`/api/*` L7 outbound proxy future)

- **What:** Current policy is domain + port + host only. Path-level matching (`domains = ["api.github.com"] path = "/api/*"`) would require an L7 outbound proxy that can see HTTP path (TLS-terminating or CONNECT-aware).
- **Why deferred:** No L7 proxy in the current MSB SDK (only domain/port via DNS cache + SNI, see ADR §5A.1); path matching implies a different enforcement primitive (proxy policy, not `NetworkPolicy` `Rule`). Deferred to proxy future.
- **Future work:** Introduce a host-side L7 outbound proxy (HTTP-aware) gated behind `policy.egress.allow.domain` with an optional `paths = [...]` field; compile to proxy rules (not SDK `Rule` list); validation rejects `paths` without the proxy feature flag. See ADR 0035 §12 row, §14 alternative.

### 5. IDNA / UTS46 + UTS39 — DESIGNED, flag-gated default-off — implement in v1 (Unicode domains)

- **Status 2026-09-03: DESIGNED, flag-gated default-off — implement in v1** (`idna` `1.1` + `unicode-security` crates, ADR §3.5 §12.1). `mode = "reject"` (built-in default, hard error with codepoint `U+XXXX` + byte index + punycode suggestion via `idna::domain_to_ascii_strict`) | `"uts46"` (strict `ToASCII` → A-label + `unicode-security` skeleton/mixed-script WARNINGs), ladder `built-in reject → home → layers → workload` with `final` sealing. **This entry retained for historical context; implementation now in-scope for v1 engine phase — no longer shelved, but not deleted for traceability.**
- **What (historical + current):** Non-ASCII domains (`münchen.de`, `gіthub.com` Cyrillic `і`) are hard validation errors in `reject` mode (suggest punycode `xn--mnchen-3ya.de` with `U+00FC` index); in `uts46` mode they are accepted via IDNA2008 + UTS46 nontransitional + STD3 + Bidi/Joiner + hyphen + DNS length → `ToASCII` A-label (punycode, lowercased, normalized) + UTS39 confusable/mixed-script linter (WARNING `idna_warning` provenance). See ADR §3.5 flag-gated semantics, §12.1 deep dive + implementation reality, §11.1 test vectors, §5 invariant #17.
- **Why rejected v1 (historical rationale preserved):** Core vocab is ASCII (zero current need); IDNA is a versioned supply-chain of Unicode tables (NFC/NFKC, mapping `valid`/`ignored`/`mapped`/`deviation`/`disallowed`, contextual ZWNJ/Bidi, deviation `ß`/`ς`, STD3, hyphen, length); homograph in a POLICY context = authorization bypass (confusables like `gіthub.com` vs `github.com` visually identical but different bytes → allowlist match diverges from human intent); even correct `ToASCII` needs UTS39 confusable linter (compare against allowed set). Not justified before demand — hence default remains `reject`; `uts46` requires explicit opt-in.
- **Implementation reality (now in-scope for v1 engine phase, §6 Phase 1):**
  - **Crates:** `idna` `1.1.0` already in `Cargo.lock` transitively via `url`/`hickory-proto`/etc.; promote to direct `idna = "1.1"` + `unicode-security = "0.1"` (`0.1.2` on crates.io) in `control/agentctl/Cargo.toml`; fork `microsandbox-*` `patch.crates-io` unaffected; negligible binary size (already linked).
  - **idna API:** `idna::domain_to_ascii_strict(&str) -> Result<String, Errors>` (internally `Uts46::new().to_ascii(..., AsciiDenyList::STD3, Hyphens::Check, DnsLength::Verify)`) — nontransitional + STD3 + Bidi/Joiner (always true in crate) + hyphen + length; or `idna::uts46::Uts46::new().to_ascii(..., STD3, Check, Verify)`. Do NOT use `domain_to_ascii` (`Allow`/`Ignore`) — too permissive. Flags: `UseSTD3ASCIIRules=true`, `transitional=false`, `CheckHyphens=true`, `CheckBidi=true`, `CheckJoiners=true`.
  - **unicode-security API:** `unicode_security::general_security_profile::GeneralSecurityProfile`, `unicode_security::mixed_script::MixedScript`, `unicode_security::confusable_detection::skeleton`, `unicode_security::restriction_level::RestrictionLevelDetection`; skeleton compare (equal skeleton, different bytes → confusable) + `detect_restriction_level` (`HighlyRestrictive` etc.) → WARNING `stderr` + provenance `idna_warning` (not blocking plan; future `confusable = "fail"` not in v1).
  - **Pipeline:** `validation.rs` early gate, before coverage/specificity: resolve `policy.idna.mode` via ladder first, then per `domains` entry — `reject` + non-ASCII → hard error (codepoint, index, punycode hint via `idna` call); `uts46` → `idna` strict; on `Err` → hard error (reason + codepoint); on `Ok` → canonical A-label for matching (SNI bytes), original Unicode retained in `idna_original` provenance; then skeleton/mixed-script linter emits WARNINGs. Ladder `final` seals lower rungs (same as `on_conflict` invariant #11).
  - **Cost:** 2 deps, ~200-300 LOC validation + ~100 LOC compiler wiring + tests, negligible size, no per-packet runtime overhead beyond `plan`/`validate`.
  - **Test vectors (ADR §11.1 §12.1):** Cyrillic `і` `U+0456` `gіthub.com` vs `github.com` confusable WARNING; `ß` `U+00DF` deviation nontransitional (preserved → punycode, not `ss`); `ZWNJ` `U+200C` invalid joiner context → hard error; legit IDN `münchen.de` → `xn--mnchen-3ya.de` round-trip + suffix `.münchen.de` coverage; mixed-script `pаypal.com` (Cyrillic `а`) WARNING; hyphen edge `3rd-4th` `Check` violation; length `>63` label error; `reject` `final` at `home` seals workload `uts46` attempt (`frozen_by=home`).
- **Future work if extended:** Optional `policy.idna.confusable = "fail"` (hard error on confusable) may be considered but not in v1; default remains WARNING. See ADR 0035 §3.5, §5 #17, §12.1 implementation reality, §11.1 matrix, §6 Phase 1, §15 item 5 resolution (now DESIGNED).

### 6. validate-config stale-schema warning (P3) — deferred

- **What:** `validate-config` would warn on stale consumer schema copies (committed `schemas/workestrate.schema.json` vs `generate_schema_pair()` drift). Today it is a P3 warning, not a hard error.
- **Why deferred:** Would require mutating `validate-config` to track consumer schema copies beyond the repo `schemas/` (scope creep for ADR 0035); `tombi strict` lint (see `docs/runtime-provisioning.md` schema flow) already catches most drift. Avoids `validate-config` mutation in the ladder PR.
- **Future work:** Extend `validate-config` (or `workestrate doctor`) to emit a `warn` when a consumer's vendored `workestrate.schema.json` / `workload.schema.json` / `registry.schema.json` is stale vs `generate_schema_pair()`; gate with `--strict` or `tombi lint` parity. See ADR 0035 §12 row, §15 deferred note, `docs/runtime-provisioning.md` companion.

### 7. Ingress `all` lint (§15 item 4)

- **What:** `[[policy.ingress.allow]] all = true` (allow-all ingress) is legal and least-specific (invariant #22) but opens the host firewall to any peer (subject still to `bind`, see ADR §3.4.2).
- **Why deferred:** Lint would be a `warn`-grade extra gate even when `on_conflict = "ignore"`; useful for safety but not load-bearing for correctness. Deferred to avoid scope creep; current behavior is still explicit and `final`/`on_conflict` per-ladder (ADR §8) governs it.
- **Future work:** Add a `validate-config` / `plan` lint that warns on `ingress.allow all = true` (suggest `scope = "local"` or more specific scopes), possibly with a `#[allow(ingress_allow_all)]` escape. See ADR 0035 §12 row, §15 item 4 (deferred-to-todo).

### 8. Deprecation window — REJECTED pre-public (outright removal policy, §9.4)

- **Status 2026-09-03: REJECTED pre-public — outright removal (no deprecation windows, no compat aliases, no warn-then-remove).** See ADR 0035 §9.4 (removal-not-deprecation policy) + design-notes A8 + ADR §10 footnote + §12 row `Deprecation window / compat aliases` (REJECTED). This entry retained for historical context; not a shelved future todo pre-public.
- **What was considered:** This ADR does a hard cutover for recipes (deleted, no aliases) — fine pre-release. For future breaking changes to presets or recipe-like vocabulary, a `warn`-grade deprecation window (e.g. `recipe = "github"` still parses but emits `warn: deprecated — use [[policy.egress.allow.domain]] ... see ADR <next>`) → next minor removes the alias was previously noted as a possible pattern.
- **Why REJECTED pre-public:** Pre-public, versions are resettable (no external consumers to migrate gradually); hard cutover with actionable errors + intent tests is the drift guard (no migration debt per ADR 0031 precedent). The §9.4 `see ADR 0035 (...)` error-message convention (citing governing ADR **and intended behavior/replacement syntax**) plus behavioral tests ( §11 matrix + golden plans + provenance snapshots) replace `warn`-then-`remove` cycles. Compat aliases and dual-stack would add debt and obscure `deny_unknown_fields` (fail-closed) without benefit pre-public.
- **Future reassessment only post-public-adoption:** When semver stability requires gradual migration for external consumers, a deprecation window may be appropriate — but not now. Revisit only after public adoption; then use `see ADR <next> (docs/migration/50-decisions/0036-....md)` suffix for every removed/deprecated key as documented in ADR §9.4. Until then, removed vocabulary is **removed outright** with `see ADR 0035 (docs/migration/50-decisions/0035-hierarchical-egress-ingress-policy.md)` + replacement Toml in the diagnostic. See ADR 0035 §9.4, §10, §12, §15, design-notes A8, §4 item 6.

---

## Committed follow-up (not shelved) — tracked here for visibility

### C1. `registry.schema.json` + tombi mapping — APPROVED, not shelved

- **What:** `Registry.policy` in `control/agentctl/src/config/registry.rs` already derives `schemars::JsonSchema` but has no vendored `schemas/registry.schema.json` and no `tombi lint` gate for `<home>/config.toml` / `overrides.toml`.
- **Why it is NOT shelved:** APPROVED as committed follow-up (ADR §15 item 7 resolution). Moved from open question to committed.
- **Implementation (follow-up PR, not deferred):**
  1. `schemars::schema_for!(Registry)` (same pattern as `generate_schema_pair()` in `control/agentctl/src/commands/diagnostics.rs:1017`) → `schemas/registry.schema.json` (vendored).
  2. `workestrate generate-schema --output-registry schemas/registry.schema.json` (+ `workestrate schemas update` distribution).
  3. **HOME_TOMBI_TOML** — a `tombi.toml` for `<home>` files (distinct from repo `workestrate/tombi.toml`): `[[schemas]] path = "schemas/registry.schema.json" include = ["config.toml", "overrides.toml"]` with `strict = true`. Then `tombi lint` covers home files. See ADR 0035 §12.2, §10, §11.5, `docs/runtime-provisioning.md` companion.
- **Status:** Tracked here and in ADR §15 resolution table as **approved follow-up** (counts as 1 of the 7 dispositions). Design notes §4 item 7 mirrors this.

---

## Cross-reference checklist

- ADR → this file: `../shelved-todo.md` (from ADR in `50-decisions/`)
- ADR → design notes: `./0035-design-notes.md`
- Design notes → ADR: `./0035-hierarchical-egress-ingress-policy.md`
- Design notes → this file: `../shelved-todo.md`
- This file → ADR: `./50-decisions/0035-hierarchical-egress-ingress-policy.md`
- This file → design notes: `./50-decisions/0035-design-notes.md`
