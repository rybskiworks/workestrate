# 06 — Improvements Index

> **STATUS: INDEX**
> Prerequisites / see-also: [../README.md](../README.md) ·
> [../00-overview.md](../00-overview.md) ·
> [../07-execution-order.md](../07-execution-order.md)

This index catalogs the eight post-validation improvement specifications under
`06-improvements/`. Each spec is a self-contained engineering document for a
post-migration enhancement to the config-driven workestrate tool — work that is
**not** required for the migration itself to be complete, but that hardens,
extends, or cleans up the config-driven setup after the baseline validation
([04-baseline-validation.md](../04-baseline-validation.md)) and host validation
([05-host-validation.md](../05-host-validation.md)) have passed.

Every improvement spec in this directory conforms to the governing ADR set:
[0002](../../migration/50-decisions/0002-toml-config-format.md) (TOML config
format), [0003](../../migration/50-decisions/0003-config-purity-closed-vocabulary.md)
(config purity / closed vocabulary),
[0004](../../migration/50-decisions/0004-security-allowlist-policy-rs.md)
(security allowlist in `policy.rs`),
[0005](../../migration/50-decisions/0005-security-aware-merge.md)
(security-aware merge),
[0014](../../migration/50-decisions/0014-trust-gated-project-config.md)
(trust-gated project config),
[0019](../../migration/50-decisions/0019-contexts-and-user-global-overrides.md)
(contexts + user-global overrides),
[0020](../../migration/50-decisions/0020-review-adjudications.md)
(review adjudications),
[0021](../../migration/50-decisions/0021-instance-lifecycle-model.md)
(instance lifecycle model),
[0022](../../migration/50-decisions/0022-config-repo-scaffolding.md)
(config-repo lifecycle), and
[0023](../../migration/50-decisions/0023-single-tool-home.md) (single tool
home). The ADR index is at
[`../../migration/50-decisions/README.md`](../../migration/50-decisions/README.md).
No spec here contradicts an ADR; where a spec extends the config surface, it
does so via `#[serde(default)]` additive fields (ADR 0021 §8) and post-merge
monotonic-deny enforcement (ADR 0005), never by weakening an existing
invariant.

---

## Master table

| Spec | Title | Status (verbatim banner) | Dependencies (file-level) | Effort | Env gate (worst) |
|---|---|---|---|---|---|
| [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) | Mount Filtering / Shadowing (Track A) | `SPEC (not yet implemented); Phase 0 spike is NEEDS-KVM` | Phase 0 KVM spike gates WP4; WP5 (staging-copy fallback) is conditional on spike failure only | Phase 0 = **S**; WP1–WP4 = **M** each; WP5 = **L** (conditional) | `HOST-KVM` |
| [02-main-standardization.md](02-main-standardization.md) | Standardize on `main` (rename the personal clone) | `READY-TO-EXECUTE` (rename is host-local git surgery, no KVM/nix needed; branch-detection is a DEFERRED option) | None — standalone one-shot rename | **S** (one-shot branch rename; branch-detection option deferred) | `verifiable-here` |
| [03-dogfooding.md](03-dogfooding.md) | Dogfooding: workestrate developing workestrate (Track C) | `SPEC (Phase 0 env pinning READY-TO-EXECUTE; B1/B2/B3 not implemented)` | Phase 0 is standalone; B1/B2/B3 are independent of each other; references [05](05-config-reference-cwd-fallback.md) for the underlying quirk | Phase 0 = **S** (env pinning); B1/B2/B3 = **M** each (not implemented) | `HOST-KVM` (B3 live-sandbox verification) |
| [04-cli-config-authoring.md](04-cli-config-authoring.md) | CLI Config Authoring (DEFERRED vision + requirements traceability) | `DEFERRED` — pending sign-off of [02-config-requirements.md](../02-config-requirements.md) | Gated on `02-config-requirements.md` sign-off; must be additive-tolerant of 01's mount exclude/shadow schema (§2 of the spec) | **M** (explicitly stated) | `verifiable-here` |
| [05-config-reference-cwd-fallback.md](05-config-reference-cwd-fallback.md) | Config-reference cwd-fallback quirk (standalone fix spec) | `SPEC (bug fix candidate, small)` | None — standalone bug fix; referenced by [03](03-dogfooding.md) as the underlying quirk | **S** (explicitly stated) | `verifiable-here` (fix gate: `cargo test`); reproduction is `HOST-NIX` |
| [06-config-home-flag.md](06-config-home-flag.md) | `--home` global CLI flag (idiomatic config-home override) | `SPEC (small, not yet implemented)` | None — standalone; referenced by [03](03-dogfooding.md) + [../03-sibling-config-setup.md](../03-sibling-config-setup.md) for ergonomics | **S** | `verifiable-here` (fix gate: `cargo test`); runs in HOST-NIX devshell (no `cc` here) |
| [07-naming-consistency.md](07-naming-consistency.md) | Naming consistency: purge `workestrator` residue | `IN-PROGRESS THIS BRANCH (migration/tool-model)` | None — standalone rename; FLAG: personal config repo image-name coordination (spec §4) | **M** | `verifiable-here` (grep/lint-nix/nix eval); cargo gates `HOST-NIX` |
| [08-no-repo-local-home.md](08-no-repo-local-home.md) | No repo-local tool home (retire `.workestrate/` inside the checkout) | `READY-TO-EXECUTE (docs/decision); code step (e) is NEEDS-DEVSHELL` | None — stepwise internal ordering only (a→d strictly; e is the code step); sequencing-wise it should land EARLY (before Lane A / host batch) because it changes the paths those reference | **S** (steps a–d, f, g) + **code-S** (step e) | `verifiable-here` for a–d/f/g; step (b) verify + step (e) code gates are `HOST-NIX` devshell (no `cc` here) |

> **Effort legend:** S = small (hours), M = medium (days), L = large (week+).
> Effort values are pulled verbatim from each spec's status banner where the
> spec carries an explicit estimate; values marked "(inferred)" are deduced
> from the spec's scope where no explicit tag exists (see §Inconsistencies
> below).

---

## Per-spec summaries

### 01 — Mount Filtering / Shadowing (Track A)

Full spec for excluding sensitive files/dirs from host→guest bind mounts
**without copying**, via nested shadow mounts over the live bind. Adds
`exclude` (glob patterns), `[[mounts.shadow]]` (explicit shadow entries:
`empty-file` / `empty-dir` / `tmpfs`), and `allow_sensitive` (trust-gated
opt-out) to `MountPlan`. New `globset` dependency. Policy integration via
`SENSITIVE_MOUNT_EXCLUDE_PATTERNS` applied **post-merge at plan time**
(monotonic deny, ADR 0005-conformant — no merge-engine change). Phased as
Phase 0 (KVM spike) → WP1 (schema+glob) → WP2 (policy+trust) → WP3
(render+audit) → WP4 (runtime shadows); WP5 (staging-copy fallback) is
**conditional** — activated only if the Phase 0 spike proves nested mount
ordering does not work. **Key decision:** the Phase 0 HOST-KVM spike is the
gate for WP4; if it fails, WP5 (zero-copy-defeating staging copy) is the
fallback.

### 02 — Standardize on `main` (rename the personal clone)

One-shot `git branch -m master main` on the personal clone at
`.workestrate/repos/personal` to resolve the registry/clone mismatch (registry
declares `ref = "main"`, clone is on `master`). The tool is already main-biased
in five code sites (`cli_actions.rs:105`, `config_cmd.rs:548,605`,
`migration.rs:686`, `scaffold/mod.rs:36`); the clone is the sole outlier.
**Key decision:** the rename needs **no registry edit** — `config.toml:9`
already declares `ref = "main"`. Branch-detection robustness (auto-detect
remote default branch in `config add`) is **DEFERRED** as an open decision
(§7 of the spec) because it partially conflicts with the standardize-on-`main`
goal.

### 03 — Dogfooding: workestrate developing workestrate (Track C)

Spec for dogfooding the workestrate tool by using it to develop itself, in a
driver/target topology (immutable nix-store driver binary spawns a sandbox
that mutates the driver's own source tree via a git worktree). **Phase 0**
(READY-TO-EXECUTE) is an env-pinning wrapper script (`workestrate-driver.sh`)
that closes the config-reference cwd-fallback backdoor by setting
`WORKESTRATE_HOME`, `WORKESTRATE_CONFIG_DIR`, `AGENTCTL_ROOT`, and
`WORKESTRATE_NO_PROJECT_CONFIG` — no code change required. **Phase 1** (NOT
IMPLEMENTED) adds three structural hardening features: **B1** self-home mount
guard (refuse plans that mount the driver's `WORKESTRATE_HOME` into a
sandbox), **B2** config-free teardown verification (regression test — the
premise was refuted: teardown already avoids `load_config()`), **B3** spawn
provenance (record driver binary/home/root in `SandboxInstanceRecord`).
**Key decision:** Phase 0 env pinning is the immediate mitigation; B1 makes
it structural.

### 04 — CLI Config Authoring (DEFERRED)

Vision and requirements-traceability record for the future `toml_edit`-based
CLI config-authoring tool (`workestrate config edit workload/env/mount/secret/
context ...`). **DEFERRED** pending sign-off of
[02-config-requirements.md](../02-config-requirements.md) — the contract this
tool targets. Six hard requirements: lossless round-trip (toml_edit, not
serde), `--layer` targeting, validate-after-mutation (atomic write), never
write secret values (only references), trust gating respected, schema-driven
field set (derived from `schemars` types, same as `generate-schema`).
**Key decision:** the CLI must be **additive-tolerant** of 01's mount
exclude/shadow schema — either ship after the schema is finalized, or be
schema-driven from the start (§4.6, the intended design) so new fields
auto-appear without a CLI change.

### 05 — Config-reference cwd-fallback quirk (standalone fix spec)

Standalone bug fix for a config-discovery quirk: when the nix-installed
`workestrate` binary is run from any directory containing both `flake.nix`
and `config.reference/`, the reference config is silently loaded as the **base
layer** off the current working directory — even though the reference fixture
was designed as the tool's fixture (ADR 0017), not a user-overridable layer.
Four fix options compared; **recommended: option (d)** — gate reference loading
behind an explicit opt-in env (`WORKESTRATE_ALLOW_CWD_REFERENCE=1`) when
`project_root()` resolved via cwd, plus option (c)'s warning as
defense-in-depth. **Key decision:** option (d) defaults safe (no silent cwd
loading), preserves the fresh-install fallback via the opt-in env, and leaves
`cargo run` / `just golden-check` unaffected (they short-circuit before the
new gate).

### 06 — `--home` global CLI flag

Additive global `--home <DIR>` flag that sets `WORKESTRATE_HOME` from
`async_main` (`main.rs:237-244`), becoming the highest-precedence home
override with **no path-resolution change** — `resolve_home_with_kind()`
checks `WORKESTRATE_HOME` first (`paths.rs:106`), so the flag wins over any
ambient export and implicitly disables walk-up discovery for that
invocation. Idiomatic (the `git --git-dir` / `cargo --manifest-path` /
`docker --config` / `kubectl --kubeconfig` / `terraform -chdir` pattern),
discoverable via `--help`, and per-invocation (no leaked `export`). Simplifies
the experiment-home flow ([../03-sibling-config-setup.md](../03-sibling-config-setup.md))
and the dogfood-driver flow ([03-dogfooding.md](03-dogfooding.md)): e.g.
`alias workestrate-driver='workestrate --home ~/.workestrate-driver'`.
**Key decision:** reuse the env-var precedence step (flag sets the env var)
rather than adding a new resolution layer — zero change to `paths.rs`.

### 07 — Naming consistency (purge `workestrator` residue)

Repo-internal rename standardizing every reference on the canonical
`workestrate` name: template directory (`templates/workestrate-config/`),
flake attrs (`.#workestrate-pi`, `workestrate-wrapper`, `workestrate-sandbox`
/`workestrate-sandbox-node`), the OCI image (`workestrate-pi:latest`),
scaffold strings, LiteLLM knowledge-pack JSON keys (`workestrate_*`), and
all doc/skill prose. The project repo rename (new origin, checkout dir,
`.workestrate/config.toml` absolute paths) is the user's later step —
documented in spec §6, not done here. **Key decision:** the `workestrator`
wrapper package could not become `workestrate` (attr already taken by the
agentctl binary), so it becomes `workestrate-sandbox`; the image rename is
FLAGGED for coordinated update of the personal config repo.

### 08 — No repo-local tool home

User decision: the workestrate tool home must **NEVER** live inside the repo
checkout — the home is the user-global `~/.workestrate` (ADR 0023 default)
only. Retires the repo-local bundle at `.workestrate/` (created for
container-$HOME persistence, `.envrc`-pinned, gitignored at `.gitignore:46`,
untracked) and ALL repo-local-home machinery: `scripts/local-xdg.sh`,
`scripts/migrate-xdg-to-repo.sh`, the `.envrc` pin, the `.gitignore` entry,
and — in the code step — the trusted-ancestor discovery tier of
`resolve_home_with_kind()` (`paths.rs:116-139`), the `HomeKind::Discovered`
variant, and `emit_untrusted_discovery_warn`. New precedence: **flag
(`--home`, [06](06-config-home-flag.md)) > env > legacy XDG > default
`~/.workestrate`**. Execution plan is strictly ordered: (a) preserve the
personal config (clone `.workestrate/repos/personal` @ `c41a707` →
`/home/node/Development/workestrate-personal`) → (b) create the real home at
`~/.workestrate` (drop non-standard `scratch/` — resolves the Step 0(b)
scratch/cache open decision as: neither, drop it; fix the stale registry rev
`d2cd0c3` → `c41a707`) → (c) delete the machinery (one commit) → (d) delete
the bundle → (e) code (HOST-NIX) → (f) ADR 0023 addendum + glossary →
(g) compose/mount guidance. **Key decision:** no repo-local homes, no
discovery — the tool consumes config repos from elsewhere. **INTERIM
WARNING (spec §5):** until step (a) lands, the only committed copy of the
personal config lives in the ephemeral container bundle — do NOT rebuild the
container or delete the bundle.

---

## Dependency graph

Indented list (parent → child). `→` means "must land first"; `↔` means
"references but does not block".

```
(standalone)
├── 08-no-repo-local-home            [no deps; LAND EARLY — it changes the
│     │                               paths Lane A / host batch reference;
│     │                               internal order a→d strict, e = code]
├── 07-naming-consistency            [no deps; this branch; mechanical rename]
├── 02-main-standardization          [no deps; one-shot git rename]
├── 06-config-home-flag               [no deps; additive CLI front-end]
├── 05-config-reference-cwd-fallback [no deps; standalone bug fix]
│     ↑
│     ↔ 03-dogfooding (Phase 0 is the convention-based mitigation for the
│                      same quirk; 05 is the structural code fix)
│
├── 03-dogfooding
│     ├── Phase 0 (env-pinning wrapper)     [standalone; READY-TO-EXECUTE]
│     └── Phase 1: B1 / B2 / B3             [each independently shippable]
│
├── 01-mount-filtering-shadowing (Track A)
│     ├── Phase 0 (KVM spike)  → gates WP4
│     ├── WP1 (schema+glob)    → WP2, WP3
│     ├── WP2 (policy+trust)   [depends on WP1 types]
│     ├── WP3 (render+audit)   [depends on WP1 + WP2]
│     ├── WP4 (runtime shadows) [depends on Phase 0 spike passing]
│     └── WP5 (staging-copy fallback) [ONLY if Phase 0 spike fails]
│
└── 04-cli-config-authoring
      ├── GATED on 02-config-requirements.md sign-off (NOT an improvement spec;
      │   a sibling requirements doc — see ../02-config-requirements.md)
      └── Should be additive-tolerant of 01's mount exclude/shadow schema
          (either ship after 01 WP1 lands, or be schema-driven per §4.6)
```

**Key dependency notes:**

- **08 (no repo-local home)** has no file-level dependency on any other spec,
  but **sequencing matters**: it changes the paths that Lane A
  ([../04-baseline-validation.md](../04-baseline-validation.md)) and the host
  batch ([../05-host-validation.md](../05-host-validation.md)) reference
  (`WORKESTRATE_HOME=<repo>/.workestrate` spellings), so it should land
  BEFORE those lanes run — see [../07-execution-order.md](../07-execution-order.md)
  Step 0.5. It also **strengthens 06 (`--home` flag)**: with the discovery
  tier removed, `--home` becomes THE explicit per-invocation override and the
  precedence simplifies to flag > env > legacy XDG > default.
- **04 (CLI authoring)** is gated on **`02-config-requirements.md` sign-off**
  (a sibling requirements doc, not an improvement spec). It is `DEFERRED`
  until that sign-off. Additionally, it must be **additive-tolerant** of 01's
  mount schema: the `exclude` / `[[mounts.shadow]]` / `allow_sensitive` fields
  added by 01 WP1 must either be finalized before the CLI ships, or the CLI
  must be schema-driven (§4.6 of 04) so new fields auto-appear. The spec's §2
  documents two acceptable orderings (schema-first or additive-tolerant).
- **05 (cwd-fallback fix)** and **03 (dogfooding)** are related but
  independent: 03 Phase 0 is the convention-based mitigation (env-pinning
  wrapper) for the quirk that 05 specifies the structural code fix for. Either
  can land first; both should eventually land.
- **01 (mount filtering)** has no file-level dependency on any other
  improvement spec. Its internal dependency is the Phase 0 KVM spike → WP4
  gate, and the WP5 conditional on spike failure.

---

## Environment markers

Reproduced from [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md)
§Environment markers:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). NOTE: cargo-linked gates are NOT runnable here — no `cc` linker; they run on the host (HOST-NIX devshell) |
| `HOST-NIX` | Requires nix on the user's host (this container has no nix / no `cc` linker) |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

---

## Inconsistencies between specs (for lead reconciliation)

These are discrepancies found while reading the specs (01–07, pre-08); they do not block
this index but should be reconciled:

1. **Effort estimates are inconsistent in format.** 01 breaks effort down per
   phase/WP (S/M/M/M/M/L); 04 and 05 carry a single explicit tag (M, S); 02
   and 03 now carry explicit effort tags in their status banners (02: S;
   03: Phase 0 = S, B1/B2/B3 = M each) — **RESOLVED** (tags added per the
   consistency-fix pass). If a uniform effort column is still desired, all
   specs now have explicit tags.

2. **03 B1/B2/B3 have no effort estimates.** The spec describes each feature's
   scope, files, and gate, but did not assign S/M/L. **RESOLVED:** the status
   banner now carries `B1/B2/B3 = M each (not implemented)` per the
   consistency-fix pass. B2 is noted as "verification + test item, not a
   behavior change" but is tagged M to match B1/B3 (each involves code or
   test work across multiple files).

3. **05 env markers vs. gate.** The spec's env-markers section lists both
   `verifiable-here` and `HOST-NIX`, but the gate (§4) is `cargo test` marked
   `verifiable-here`. The `HOST-NIX` marker applies only to the reproduction
   recipe (§1.2), not to the fix itself. The master table above reflects this
   distinction.

4. **01-vs-04 `just verify` contradiction — RESOLVED.** 01 §"Gates status
   snapshot" previously claimed `just verify` overall: **PASS** in this
   container, while 04 §"Lane A gate suite" said the cargo-based gates are
   `HOST-NIX` (no `cc` linker). **Resolution:** the contradiction is
   resolved — this container has no C toolchain (verified:
   `command -v cc gcc` → not found; `cargo` exists at `~/.cargo/bin/cargo`
   but cannot link). All cargo-linked gates (`just check`, `just test`,
   `just spec-examples`, `just golden-check`, `just schema-check`,
   `just scaffold-check`) are uniformly treated as `HOST-NIX` / host-devshell
   gates; only the shell/python/git-based subset (`toolchain-check`,
   `litellm-check`, `lint-nix`, `store-audit` SKIP, `Cargo.lock` stability)
   is `verifiable-here`. 01's gates table and checklist have been corrected
   to match 04's verified evidence.

5. **Tempest `install_layout` config/schema drift — CRITICAL, blocks
   validation.** `.workestrate/repos/personal/workestrate.toml:317` sets
   `install_layout = "app"` in tempest's `binary` inline table, but
   `BinarySpec` (`control/agentctl/src/config/types.rs:44-52`) has
   `#[serde(deny_unknown_fields)]` and no `install_layout` field (the
   nix-side param was removed as a silent no-op,
   `nix/lib/recipes/npm-build.nix:16-25`). Any load of the personal config
   layer hard-errors at parse time, blocking all Lane A commands against the
   real bundle until the one-line bundle edit lands. See
   [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md)
   §"Bundle fixes needed" item e. This is a bundle-config issue, not an
   improvement-spec issue, but it blocks validation of every spec that
   exercises the real bundle. **RESOLVED (edit applied):** the
   `install_layout = "app"` field has been REMOVED from
   `.workestrate/repos/personal/workestrate.toml:317` (bundle fix e applied;
   verified: grep for `install_layout` in that file returns no match).
   Runtime parse verification (`workestrate validate-config`) is PENDING —
   it requires a `cc` linker (HOST-NIX devshell), the first Lane A action.
