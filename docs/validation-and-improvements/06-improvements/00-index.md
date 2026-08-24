# 06 — Improvements Index

> **STATUS: INDEX**
> Prerequisites / see-also: [../README.md](../README.md) ·
> [../00-overview.md](../00-overview.md) ·
> [../07-execution-order.md](../07-execution-order.md)

This index catalogs the twenty-four post-validation improvement specifications under
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
home), [0024](../../migration/50-decisions/0024-dotfiles-home-and-working-copy-config-repos.md)
(dotfiles home + working-copy config repos), and
[0025](../../migration/50-decisions/0025-home-provisioning-and-lockfile.md)
(home provisioning + lockfile), and
[0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md)
(per-instance addressing + discovery-lite), and
[0027](../../migration/50-decisions/0027-verb-first-workload-dispatch.md)
(verb-first workload dispatch; supersedes 0006). The 2026-08-01 addenda to
[0026](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md)
(compose-mirrored default-on dependency lifecycle) and
[0021](../../migration/50-decisions/0021-instance-lifecycle-model.md)
(bare `workload up` topo-starts all service-kind workloads) also govern.
The ADR index is at
[`../../migration/50-decisions/README.md`](../../migration/50-decisions/README.md).
No spec here contradicts an ADR; where a spec extends the config surface, it
does so via `#[serde(default)]` additive fields (ADR 0021 §8) and post-merge
monotonic-deny enforcement (ADR 0005), never by weakening an existing
invariant.

---

## Master table

| Spec | Title | Status (verbatim banner) | Dependencies (file-level) | Effort | Env gate (worst) |
|---|---|---|---|---|---|
| [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) | Mount Filtering / Shadowing (Track A) | `SECONDARY / FALLBACK (2026-08-02)` — spec [22](22-dynamic-mount-masking-policy.md) (dynamic PassthroughFs masking) is PRIMARY; WP1–WP4 FROZEN; deleted if 22 lands; kept as the degraded-mode/static fallback (WP5 staging-copy essence survives) | Phase 0 KVM spike gates WP4; WP5 (staging-copy fallback) is conditional on spike failure only | Phase 0 = **S**; WP1–WP4 = **M** each; WP5 = **L** (conditional) | `HOST-KVM` |
| [02-main-standardization.md](02-main-standardization.md) | Standardize on `main` (rename the personal clone) | `OBSOLETE (2026-08-01)` — rename applied in Step 0(a); mismatch mooted by spec 08 execution (repo-local bundle retired; personal config now at `~/.workestrate/config-repos/personal` on `main`) | None — standalone one-shot rename | **S** (one-shot branch rename; branch-detection option deferred) | `verifiable-here` |
| [03-dogfooding.md](03-dogfooding.md) | Dogfooding: workestrate developing workestrate (Track C) | `SPEC (Phase 0 env pinning READY-TO-EXECUTE; B1/B2/B3 not implemented)` | Phase 0 is standalone; B1/B2/B3 are independent of each other; references [05](05-config-reference-cwd-fallback.md) for the underlying quirk | Phase 0 = **S** (env pinning); B1/B2/B3 = **M** each (not implemented) | `HOST-KVM` (B3 live-sandbox verification) |
| [04-cli-config-authoring.md](04-cli-config-authoring.md) | CLI Config Authoring (DEFERRED vision + requirements traceability) | `DEFERRED` — pending sign-off of [02-config-requirements.md](../02-config-requirements.md) | Gated on `02-config-requirements.md` sign-off; must be additive-tolerant of 01's mount exclude/shadow schema (§2 of the spec) | **M** (explicitly stated) | `verifiable-here` |
| [05-config-reference-cwd-fallback.md](05-config-reference-cwd-fallback.md) | Config-reference cwd-fallback quirk (standalone fix spec) | `SPEC (bug fix candidate, small)` | None — standalone bug fix; referenced by [03](03-dogfooding.md) as the underlying quirk | **S** (explicitly stated) | `verifiable-here` (fix gate: `cargo test`); reproduction is `HOST-NIX` |
| [06-config-home-flag.md](06-config-home-flag.md) | `--home` global CLI flag (idiomatic config-home override) | `EXECUTED (2026-07-30; commit d991252)` | None — standalone; referenced by [03](03-dogfooding.md) + [../03-sibling-config-setup.md](../03-sibling-config-setup.md) for ergonomics | **S** | `verifiable-here` (fix gate: `cargo test`); runnable here via `nix develop` (no `cc` in a bare shell) |
| [07-naming-consistency.md](07-naming-consistency.md) | Naming consistency: purge `workestrator` residue | `DONE (landed on migration/tool-model, 3 commits; cargo gates PENDING — runnable in-container via nix develop or host devshell)` | None — standalone rename; FLAG: personal config repo image-name coordination (spec §4) | **M** | `verifiable-here` (grep/lint-nix/nix eval); cargo gates `HOST-NIX` |
| [08-no-repo-local-home.md](08-no-repo-local-home.md) | No repo-local tool home (retire `.workestrate/` inside the checkout) | `EXECUTED (2026-07-30); commits d7c5a83, bd99481, 3894fb7, bef1c37, 418530a; home commit a42e597; ADR-0023-addendum follow-through landed in bef1c37` | None — stepwise internal ordering only (a→d strictly; e is the code step); sequencing-wise it should land EARLY (before Lane A / host batch) because it changes the paths those reference | **S** (steps a–d, f, g) + **code-S** (step e) | `verifiable-here` for a–d/f/g; step (b) verify + step (e) code gates are runnable here via `nix develop` (no `cc` in a bare shell) |
| [09-microsandbox-agentd-offline-build.md](09-microsandbox-agentd-offline-build.md) | microsandbox-filesystem agentd offline build (ADR 0011 carrier) | `MERGED TO FORK AS PR #1 (b43d7522, 2026-08-04); upstream superradcompany PR still pending; interim nix-side patch still in place. PR PREPARED, READY TO OPEN (branch fix/filesystem-agentd-path-override amended to bc7640b8 (8-point review hardening + tests, 2026-08-01); docs in .tmp/msb-upstream/; force-push + open pending user; interim 0.5.6 patch aligned daa5140; option 2 REVERSED per ADR 0011 addendum; option 3 NEEDS-DEVSHELL + HOST-NIX)` | None file-level on other improvement specs; option 3 cross-references [01](01-mount-filtering-shadowing.md) | option 1 = **M** (incl. upstream review latency); option 2 = REVERSED; option 3 = **M** | `HOST-NIX` (option 3 build); option 1 is upstream |
| [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md) | Config repos as working copies + dotfiles-style home | `EXECUTED (2026-07-30); code tasks landed (d7c5a83 rename, bd99481 dirty-guard test, 3894fb7 home init); docs/spec fully done` | amends [08](08-no-repo-local-home.md) step (a) + [../03-sibling-config-setup.md](../03-sibling-config-setup.md) topology; the `config-repos/` rename gates the final paths | **S** (docs/decision) + **M** (code: rename + home-init scaffolding) | `verifiable-here` (docs); code tasks HOST-NIX devshell |
| [11-home-provisioning-and-lockfile.md](11-home-provisioning-and-lockfile.md) | Home provisioning (`home clone <src> [<dest>]`) + `workestrate.lock` (ADR 0025 execution spec) | `EXECUTED (2026-07-30; commits 172d5dd, 19ff272, be356f7; verb split c406630 2026-07-31)` | ADR 0025; composes with [06](06-config-home-flag.md) (`--home` flag — same CLI/home-resolution surface, implement in the same wave); extends [10](10-config-repos-as-working-copies.md) Task 3 | **M** | `HOST-NIX` (cargo gates via `nix develop`) |
| [12-per-instance-addressing.md](12-per-instance-addressing.md) | Per-instance addressing + discovery-lite (ADR 0026 execution spec) | `IMPLEMENTED (Waves 1+2 landed: 9107b87/de9aa62/f9fd2f0/c5837e7 + 4adad3f/7b65ad1/39c1694); Experiment E1 guest-reachability NEEDS-KVM; refuse-only/no-auto-start SUPERSEDED 2026-08-01 (ADR 0026 addendum default-on); `--use` override retained; W5 wiring plan in spec §4` | ADR 0026; amends ADR 0021 (removes --port-offset); E1 hook in [../05-host-validation.md](../05-host-validation.md); 12b (discovery-lite) landed in Wave 2 | **M** | cargo gates `HOST-NIX` devshell; E1 `HOST-KVM` |
| [13-secret-env-shorthand.md](13-secret-env-shorthand.md) | Config ergonomics: string-or-table shorthand for `secret_env` | `EXECUTED (2026-07-31; commits a349d03, 1e7dc25; personal config converted in .tmp separately) — SUPERSEDED 2026-08-01 (v2 model 1ed2e6d removed the `secret_env` namespace; shorthand survives only in the v1 shim)` — shorthand form superseded by the final model's `true` sugar (16-unified-secret-env-model, 2026-08-01) | None — standalone ergonomics; merge/validation/plan-build code paths untouched (operate post-parse on the normalized struct) | **S** | `verifiable-here` (cargo gates via `nix develop`) |
| [14-env-map-form.md](14-env-map-form.md) | Config ergonomics: map form for `env` entries (formatting collapse) | `EXECUTED (2026-07-31; commits 1035073, 564deca) — annotated 2026-08-01: the map form became the v2 unified binding map with typed EnvBinding (1ed2e6d)` — map form survives as the unified map; binding semantics gained `bound` (16, 2026-08-01) | None — standalone ergonomics; serde-only normalize to `Vec<EnvVarConfig>`; merge/validation/plan-build untouched | **S** | `verifiable-here` (cargo gates via `nix develop`) |
| [15-toml-toolchain-tombi.md](15-toml-toolchain-tombi.md) | TOML toolchain: tombi format/lint/schema-validation for config repos + homes | `EXECUTED (2026-08-01; commits 92b6b6f, d97576d, 9ffad0e + scaffold gap-fix wave; personal-repo apply 0750876)` | ↔ [13](13-secret-env-shorthand.md)/[14](14-env-map-form.md) (notation — tombi keeps the collapsed map form tidy + validated; cannot restructure) | **M** | devshell in-container (cargo gates + scripts/check-toml.sh via `nix develop`) + `HOST-NIX` (`nix build .#tombi` package build) |
| [16-unified-secret-env-model.md](16-unified-secret-env-model.md) | Final unified secret/env model (per-binding bound; delivery-on-def removed) | `EXECUTED (2026-08-01; commit 19b2cf0)` | supersedes 13/14 notation; ADR 0018 second addendum; contract in ../02-config-requirements.md | M | cargo gates verifiable-here via nix develop; B13 runtime smoke HOST-KVM |
| [17-config-repo-directory-mode.md](17-config-repo-directory-mode.md) | Config repo directory mode (workestrate/ + workloads/ capsules) | `EXECUTED (2026-08-01; commit e3194d9)` | ↔ [10](10-config-repos-as-working-copies.md), [11](11-home-provisioning-and-lockfile.md), [15](15-toml-toolchain-tombi.md), [16](16-unified-secret-env-model.md) | M | loader/cargo gates verifiable-here via nix develop; runtime smoke HOST-KVM |
| [18-cross-home-dependencies.md](18-cross-home-dependencies.md) | Cross-home dependencies (wider up across homes/config sets) | INTENT-TO-EXPLORE (2026-08-01 — questions, no decisions) | ADR 0019/0021(addendum)/0023/0026(addendum); builds on [12](12-per-instance-addressing.md) default-on lifecycle | exploration | verifiable-here (docs-only) |
| [19-visualization-inspection.md](19-visualization-inspection.md) | Visualization + inspection surfaces (beyond the W3 workloads verb) | INTENT-TO-EXPLORE (2026-08-01 — candidates + data sources, no decisions) | ADR 0026(addendum)/0027 (W3 anchor); ↔ [12](12-per-instance-addressing.md), [18](18-cross-home-dependencies.md) | exploration | verifiable-here (docs-only) |
| [20-schema-evolution-and-migrations.md](20-schema-evolution-and-migrations.md) | Schema evolution policy + config migration tooling (post-launch) | `SPEC (design; not yet implemented)` | ↔ [15](15-toml-toolchain-tombi.md) (tombi validation + vendored schema) + [16](16-unified-secret-env-model.md) (schema_version collapse to 1) | M (migrate engine) + S (pull+lock) | verifiable-here (docs-only) |
| [21-image-build-lifecycle.md](21-image-build-lifecycle.md) | Image build/load lifecycle: ensure-images pre-flight, change detection, selectors, reserved build dir | `IMPLEMENTED (phases A–E landed 2026-08-02/03; phase F multi-repo migration pending)` | ↔ [17](17-config-repo-directory-mode.md), [11](11-home-provisioning-and-lockfile.md), [12](12-per-instance-addressing.md)/ADR 0026 addendum, [01](01-mount-filtering-shadowing.md), [07](07-naming-consistency.md); additive-only per ADR 0021 §8 | A=S, B=S, C=M, D=M, E=M, F=S–M (phased) | HOST-KVM (phase E); C/D/F HOST-NIX |
| [22-dynamic-mount-masking-policy.md](22-dynamic-mount-masking-policy.md) | Dynamic mount masking policy: hierarchical `[policy.mounts]` scopes, collect-and-compile, runtime program | `DESIGN-APPROVED (awaiting implementation; user decisions locked 2026-08-02)` | ↔ [01](01-mount-filtering-shadowing.md) (dispositioned to fallback; WP1–WP4 frozen), ADR 0029 (decision record), ADR 0020 Ruling 1 (unamended), ADR 0005/0004/0011 (adjacency) | M (parser + compiler + diagnostics; enforcement is a later spec/phase) | `verifiable-here` (docs + pure Rust); microsandbox runtime enforcement `HOST-KVM` |
| [23-microsandbox-fork-nix-flake-packaging.md](23-microsandbox-fork-nix-flake-packaging.md) | microsandbox fork nix flake packaging (encapsulated source-build consumption) | `DESIGN / DEFERRED (2026-08-03; implementation deferred until workestrate+passthrough usage stabilizes on host)` | ↔ [22](22-dynamic-mount-masking-policy.md) (the consumer; unblocks runtime enforcement), [09](09-microsandbox-agentd-offline-build.md) (supersedes option 3 for the masking use case), ADR 0011 (adjacent — flake packaging ≠ reversed cargo-dep carrier), ADR 0029 (transmission adjacency) | M (flake authoring; deferred) | `verifiable-here` (docs-only); flake build `HOST-NIX`; runtime `HOST-KVM` |
| [24-repo-content-boundary-migration.md](24-repo-content-boundary-migration.md) | Repo content boundary migration (ADR 0033 execution spec) | `SPEC (docs-only; migration executes on the user's go — no moves executed 2026-08-24)` | ADR 0033 + the agent-side knowledge/skills repos | **M** | `verifiable-here` (docs/link checks in-container); knowledge/skills repo pushes user-approved host-side |
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

**OBSOLETE (2026-08-01):** the rename was applied during Step 0(a) and spec
08's execution retired the repo-local bundle including the
`.workestrate/repos/personal` clone — the mismatch this spec resolved no
longer exists. Branch-detection (§7 of the spec) remains a DEFERRED option.

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

### 09 — microsandbox-filesystem agentd offline build (ADR 0011 carrier)

`microsandbox-filesystem` 0.5.6 performs a build-time network download of the
prebuilt `agentd` binary (`crates/filesystem/build.rs`, default `prebuilt`
feature), breaking sandboxed/offline builds (Nix/Bazel/distro/air-gapped CI).
The SDK crate HAS the standard MSB_HOME escape but it was never ported to the
filesystem sub-crate — upstream inconsistency/oversight, not philosophy. Three
options: **(1) UPSTREAM FIX** (preferred end-state) — contribute an
MSB_HOME-based agentd check to `crates/filesystem/build.rs` mirroring the SDK
crate's own pattern (precedent: #704 merged the identical pattern into the SDK
crate; #701/#713 show maintainers are responsive); **PR HARDENED, READY TO OPEN**
(amended @ `bc7640b8` 2026-08-01 — review hardening executed; force-push pending USER) —
branch `fix/filesystem-agentd-path-override` on
`github.com/georgrybski/microsandbox` ready to force-push, PR draft at
`.tmp/msb-upstream/PR.md`;
**(2) INTERIM CARRIER** (ADR 0011, decided 2026-07-18) — **REVERSED per the
ADR 0011 addendum (2026-07-30):** the fork is a transient PR vehicle only,
never consumed as a dependency; the nix-side patch machinery stays as the
interim; **(3) VERSION BUMP** — `=0.5.6` → `=0.6.8`
(requires patch rewrite against 0.6.x build.rs; 0.5.8+ added guest-write quotas,
0.6.6 added RESOLVE_BENEATH symlink protection — both relevant to
[01](01-mount-filtering-shadowing.md); NEVER the yanked 0.6.5). **Key
decision:** option 1 upstream PR → upstream release → bump the microsandbox
pin + delete ALL compensation machinery; the nix-patch interim stays until
then; option 3 is a separate combinable track.

### 10 — Config repos as working copies + dotfiles-style home

Two NEW user decisions + one name decision. **Decision A:** consumed config
repos are FIRST-CLASS working copies inside the tool home at
`$WORKESTRATE_HOME/config-repos/<name>/` — the user edits, commits, pushes, and
branches directly in them; the REMOTE is canonical (gitops), not a separate
canonical sibling clone. Supersedes the standalone-sibling model in spec 08
step (a). The dirty-safe `config update` guard is VERIFIED PRESENT
(`config_cmd.rs:540-544`); the spec task is a regression test + docs.
**Decision B:** the home itself becomes a dotfiles-style git repo via explicit
`workestrate home init` scaffolding (gitignore + pre-commit hook). The
gitlink failure mode: `git add -A` in the home would register each
`config-repos/*` as a mode-160000 gitlink (broken pseudo-submodule) and could
stage secret material — the hook rejects both. Completes ADR 0007's
dotfiles-registry intent. **Name decision:** rename `repos/` → `config-repos/`
(collision with `sources/`; ADR 0008 term of art). Three code tasks
(NEEDS-DEVSHELL): the rename, the dirty-guard regression test, and the
`home init` scaffolding. Mount model: dev agents get home ro at the default
path + nested `config-repos/` rw shadow (spec 01 pattern). **Key decision:**
the home clones ARE the working repos — no standalone sibling, no two-copy
sync dance.

### 11 — Home provisioning + lockfile

Execution spec for ADR 0025: `home clone <src> [<dest>]` provisions a home
from a source (src required positional; dest optional, default the resolved
home) — the `git clone <src> [<dest>]` idiom. Bare `home init` is an empty
scaffold at the resolved home only (zero positionals; custom path via
`--home`). The earlier `home init --from`/positional-dest shape was superseded
by the verb split (commit `c406630`, ADR 0025 addendum 2026-07-31). Introduces
the
generated `workestrate.lock` (typed serde struct pinning `url`/`ref`/`rev` per
config repo) — written by `config add`/`update`/`remove`/`home init`, consumed
by `home clone` provisioning (checks out locked revs, not "whatever main is
today"). Selective copy: never copies `state/` (ephemeral), excludes
`sources/`, creates `secrets/` empty. Fail-before-write pre-flight validation
+ reproducibility report. **Key decision:** the lock is the single pin
mechanism for `home clone`, future `up --pin`, and dogfooding B3/B4 — no
second
mechanism.

### 12 — Per-instance addressing + discovery-lite

Execution spec for ADR 0026: replaces ADR 0021 §5 `--port-offset` (removed
pre-release) with slot-based binding — the singleton publishes on the shared
bind `127.0.0.1` at the declared ports (the well-known address static configs
use), parallel slots publish on per-instance loopback IPs (`127.0.0.N`,
`N >= 2`) drawn from a locked allocator in the port registry. Collisions are
keyed on `(bind_ip, port)` (same port on different IPs is legal). `--port-auto`
picks a lock-probed free port on the slot's bind. Wave 1 (12a) = slot-based
binding + bind-aware registry + surfacing + `--port-auto` + `--port-offset`
removal; Wave 2 (12b) = `depends_on` discovery-lite (unconditional plan-time
resolution, env injection, egress derivation, `--use <dep>@<instance>`
override, refuse-if-required-not-running). **LANDED (2026-07-30):** Wave 1 commits `9107b87` / `de9aa62` / `f9fd2f0` / `c5837e7`; Wave 2 commits `4adad3f` / `7b65ad1` / `39c1694`; only Experiment E1 (guest-reachability) remains, NEEDS-KVM. The refuse-only "no auto-start v1" stance is SUPERSEDED (2026-08-01, ADR 0026 addendum) by the compose-mirrored default-on dependency lifecycle (deps start by default on up/exec, topo-ordered closure, singleton slots, service-kind detached + wait-for-port ~15s, agent-kind refuse, `--no-deps` opt-out, occupied = satisfied, plan never starts; `--use` RETAINED as a pure instance-selection override; mandatory cycle detection + construction-order rule added); spec §4 carries the W5 config-wiring plan (declare depends_on in config.reference + personal; retire hardcoded URLs via `${VAR}`-templated injection reading the actual port-registry record); §5 follow-ups (DependsOnSpec scheme/path_suffix; wait-for-port v1, guest healthchecks v2+). **Key decision:** guest-reachability
of non-`127.0.0.1` loopbacks is KVM-unverified (DEFERRED-PENDING-E1) —
conservative default: guest-facing alternates share `127.0.0.1` + `--port-auto`
until Experiment E1 runs.

### 16 — Final unified secret/env model

Locks the final secret/env surface: ONE unified `[workloads.<name>.env]` map
with a desugar table (the `KEY = true` sugar for same-name placeholders) and
a per-binding `bound` property (`host` default = placeholder; `guest` = real
value for verifiers) replacing the intermediate v2's delivery-on-def. Secret
defs become a pure catalog (`env_var`/`allowed_hosts`/`required`/
`placeholder`; `hosts` renamed to `allowed_hosts`; `delivery`/`description`/
remap fields removed), and `schema_version` collapses back to 1 with the v2
machinery retracted pre-release. The spec carries the full per-construct
migration guide from v1 and intermediate-v2 forms, plus a doc-intent
requirement: user-facing docs must teach the fail-safe placeholder default,
the rename-only `secret`, and why `bound` is per-binding — not just syntax.

**EXECUTED 2026-08-01** (commit `19b2cf0`): the final model landed —
per-binding `bound` (`host` default = placeholder; `guest` = real value for
verifiers), the `KEY = true` sugar, `allowed_hosts` as the credential
policy, the intermediate v2 machinery retracted, and `schema_version` back
to 1; personal-v2 migrated to the final model. The B13 runtime smoke stays
`HOST-KVM` (host batch).

### 17 — Config repo directory mode

Specifies directory mode for config repos as an alternative to the single-file
`workestrate.toml`: a `workestrate/` directory of cross-cutting files
(`default.toml`, optional `secrets.toml`) plus a `workloads/` tree of flat
per-workload files or **capsule directories** that colocate each workload's
definition with its app-native artifacts (`config.yaml`, `opencode.jsonc`,
seed files, `flake.nix`) — a layout/loader change only, with zero schema or
secret/env-model change. Hard semantics: both modes present in one repo → hard
error; duplicate workload names across files/dirs → hard error naming both
provenance paths; `schema_version` is authoritative in `default.toml` only;
provenance strings carry per-file `<repo>#<relpath>` granularity. Acceptance
requires the restructured personal config repo to load to a byte-identical
merged config with byte-unchanged golden plans. **Key decision:** capsules
make the config repo a self-contained deployment unit — clone it and every
artifact every workload needs is inside it, addressed by repo-relative paths.

**EXECUTED 2026-08-01** (commit `e3194d9`; home tombi glob `ea48f45`): the
directory-mode loader landed with the hard-error semantics (both-modes,
duplicate names, `schema_version` authority in `default.toml` only), the
scaffold/copier `tombi.toml` include globs cover `workestrate/**/*.toml`,
and personal-v2 is restructured to `workestrate/{default,secrets}.toml` +
`workloads/` capsules. Runtime smoke of the restructured repo stays
`HOST-KVM`.

### 18 — Cross-home dependencies

Explores a wider `workestrate up` across homes / config sets: the ADR 0026
addendum (2026-08-01) default-on lifecycle makes dependency-closure start
automatic within one
home, but homes are isolated by design (ADR 0023) — per-home port
registries and per-home loopback allocators cannot see each other, so a
cross-home `depends_on` finds no running record and bind allocations can
collide across homes. Enumerates the design space (cross-home discovery,
shared vs per-home bind allocation, the ADR 0019 multi-context
relationship — the ADR 0021 addendum's single-context bare `workload up`
batch explicitly does NOT reopen it,
E1/dynamic-port interaction) and the questions to
investigate. **Key decision:** none — INTENT-TO-EXPLORE; open questions
enumerated in the spec.

### 19 — Visualization + inspection surfaces

Enumerates which home / config / dependency views should exist beyond the
W3 `workestrate workloads` discovery verb (ADR 0027, verb-first workload
dispatch), and which data source
feeds each — merged config, home registry, port registry, and
`workestrate.lock`. Records view candidates (`--json`-first surfaces,
dependency-graph render, home inspect, `ps --json` consumers) and the open
questions (verbs vs flags, `--json` schema stability, render format,
cross-home composition with spec 18). **Key decision:** none —
INTENT-TO-EXPLORE; open questions enumerated in the spec.

### 20 — Schema evolution policy + config migration tooling (post-launch)

Pins the post-launch version policy for the config schema: `schema_version`
stays `1` pre-launch, and the first shipped breaking change (field removed,
semantics changed, validation strictly tightened) earns `2`, while additive
changes (new `#[serde(default)]` optional fields per ADR 0021 §8, sugar forms,
permitted enum variants, loosened validation) never bump. Specifies a schema
pull command that refreshes the config repo's vendored
`schemas/workestrate.schema.json` from the installed binary only (no network),
locks provenance (header comment + additive `schema` section on
`HomeLock`/`workestrate.lock`), and fail-closed refuses to pull when the
installed tool's `EXPECTED_SCHEMA_VERSION` is lower than the locked version.
Specifies the `workestrate config migrate [--dry-run] [--to <version>]`
framework: a registry of versioned pure-function toml_edit document steps
applied linearly, with guided-migration dry-run output per the v1-shim
deprecation-warning pattern, idempotent re-runs, a `.bak` backup plus
dirty-tree warning (the user commits; the tool never auto-commits), and
refusal when the config's declared version is newer than the tool's.
Non-goals: no runtime auto-migration on load, no network schema fetching, no
half-applied multi-version chains. **Key decision:** the bump trigger is user
exposure — pre-release breaking changes are absorbed and the version retracts
(spec 16 collapse precedent), not bumped.

### 21 — Image build/load lifecycle

Moves the custom nix-layered image lifecycle (`workestrate-pi:latest`,
`tempest:latest`) out of the manual config-repo justfile ritual (`nix build` +
`msb load`) and into the tool: a **parent-side ensure-images pre-flight** runs
before any spawn on `workload up`/`exec`/`batch-up` (never in the detached
child — an `images_ready` token on `InstanceSpec` plus a hidden
`--images-ready` flag in `detach_args` mirrors the `--no-deps` precedent);
**change detection** is eval-only `nix eval --raw <repo>#<name>.drvPath`
compared against an advisory per-home record at `state/images.json` (per-tag
flock spans eval→build→load→record; re-load only on outPath change); the CLI
gains **`workestrate workload build [name] [--repo | --all-repos] [--check]
[--force] [--json]`** and a `--reload-images` force flag on the lifecycle
verbs; and **`.workestrate-build/`** is reserved at each config-repo root —
gitignored, artifact-only by construction, scaffold-provisioned, the new
default for undeclared `local_build` fallbacks. Five user decisions are
locked: **D1** record-absent + tag-present TRUSTs the store on plain `up`
(records advisory, never authoritative); **D2** stable verbatim tags, no
auto-prefixing, cross-repo collision → `validate-config`/`plan` WARN naming
both repos; **D3** `--reload-images` forces ALL service workloads in a batch
and is never forwarded in `detach_args`; **D4** the `.workestrate-build/`
reservation; **D5** cross-home collisions warn + digest-detect with records
keyed by (config-repo identity, name:tag). Phased **A–F**: A spec+scaffold and
B image-state store are S/`verifiable-here`; C change detection + `workload
build` and D the build/load pipeline are M/`HOST-NIX` (parallelizable behind
B); E lifecycle wiring is M/`HOST-KVM` e2e; F multi-repo + personal repo
migration is S–M/`HOST-NIX`. An msb HOST-VERIFY cluster (digest surface,
bare-tag normalization, `msb load` stdin concurrency, tarball determinism,
`git+file` dirty-worktree drvPath stability) gates phase D/E. **Key
decision:** the tool owns freshness via eval-only drvPath change detection
against an advisory per-home record — the msb store stays ground truth for
presence, the record stays memory for provenance.

**STATUS (2026-08-13):** phases A–E IMPLEMENTED (state store, `workload
build`, build/load pipeline, ensure-images pre-flight, `--reload-images`);
phase F (multi-repo migration) pending. Flake-root resolution is
declaring-repo-based and CWD-independent (ADR 0028).

### 23 — microsandbox fork nix flake packaging

Spec 23 captures a deferred design for packaging the microsandbox passthrough
fork as a source-built nix flake output. One pinned flake revision would keep the
`msb` binary and Cargo SDK source in lockstep, while the fork encapsulates agentd
and the libkrunfw wrinkle. The recommended location is the fork's own flake,
consumed by workestrate as an input; workestrate-side `nix/packages/` remains an
assessed but leaky alternative. Implementation waits for host stabilization and
no ADR is created for the deferred idea.

**Key decision:** defer implementation; recommend fork-as-flake packaging for
reversible, one-revision binary-plus-SDK consumption.

### 24 — Repo content boundary migration

Execution spec for ADR 0033's repo content boundary: the repo enforces how the
repo works (project conventions, decisions, architecture, SDLC), not how an
agent achieves it. The extraction program already built the agent-side
destinations (knowledge repo: generic docs trees, byte-identical per its
MANIFEST; skills repo: 255 generic skills @ `b4f5ada`); this spec cleans the
workestrate source side. It enumerates the full move list (generic docs trees
and the litellm/nix generic files → knowledge; 255 generic skills → source-side
deletion only; the 34 project-specific skills, OUR-config litellm pair,
`docs/nix-purity.md`, and `docs/nix` project files STAY), the pointer/redirect
strategy (sibling-relative pointers or moved-annotations; never a dangling
reference; historical docs annotated, not rebased), six validation gates
(Phase 0 `diff -rq` per tree before ANY deletion; zero new dangling links),
and a five-phase execution (0 user go + re-verify; 1 pointer rewires; 2
source-side deletions, one commit per area; 3 BEADS.md split — conventions
stay, mechanics become a new beads skill in the skills repo; 4 gates + index
updates). **Key decision:** docs-only until the user's go; the boundary is
enforced at review time, not by tooling.
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

09-microsandbox-agentd-offline-build   [no file-level deps on other specs;
      │                                 option 2 REVERSED (ADR 0011 addendum
      │                                 2026-07-30); option 1 PR PREPARED/ON
      │                                 HOLD (`fix/filesystem-agentd-path-override`
       │                                 @ bc7640b8);
      │                                 option 3 ↔ 01 (RESOLVE_BENEATH cross-ref)]

10-config-repos-as-working-copies     [amends 08 step (a) + ../03 topology;
                                        code tasks gated on devshell (NEEDS-DEVSHELL);
                                        the config-repos/ rename gates the final
                                        paths referenced by Steps 0.5/3/5 prose]

11-home-provisioning-and-lockfile     [deps: ADR 0025; ↔ 06 (same wave);
                                        extends 10 Task 3]

12-per-instance-addressing [deps: ADR 0026; supersedes ADR 0021 §5; E1 ↔ ../05-host-validation.md]

13-secret-env-shorthand [no deps; additive serde-only shorthand; schema_version stays 1]

14-env-map-form [no deps; additive serde-only map form; schema_version stays 1;
                  supersedes 13 §3 "env genuinely needs tables" non-goal]

15-toml-toolchain-tombi [↔ 13/14 notation; additive toolchain; no schema_version change]

16-unified-secret-env-model [supersedes 13/14 notation; ADR 0018 2nd addendum; contract ../02-config-requirements.md]

17-config-repo-directory-mode [↔ 10/11/15/16; EXECUTED 2026-08-01 (e3194d9)]

18-cross-home-dependencies [deps: ADR 0019/0021 addendum/0023/0026 addendum; builds on 12 default-on lifecycle; exploration]

19-visualization-inspection [deps: ADR 0026 addendum / 0027 W3 anchor; ↔ 12/18; exploration]

20-schema-evolution-and-migrations [↔ 15 (tombi/vendored schema), 16 (schema_version collapse); docs-only spec]

21-image-build-lifecycle [↔ 17/11/12(ADR 0026 addendum)/01/07; ADR 0021 §8 additive-only; phased A–F — A–E IMPLEMENTED 2026-08-02/03; F pending]
23-microsandbox-fork-nix-flake-packaging [↔ 22 (consumer; unblocks runtime enforcement), 09 (supersedes option 3 for masking), ADR 0011 (adjacent), ADR 0028 (adjacent); DESIGN/DEFERRED — no flake code now]

24-repo-content-boundary-migration [deps: ADR 0033; ↔ knowledge/skills repos; docs-only until user go]```

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
- **10 (config repos as working copies)** amends spec 08 step (a) (no
  standalone sibling — the home clone IS the working repo) and
  `../03-sibling-config-setup.md` topology (config-repo development happens in
  the home's `config-repos/` working copies). Its code tasks (the
  `repos/` → `config-repos/` rename, the dirty-guard regression test, the
  `home init` scaffolding) are NEEDS-DEVSHELL / HOST-NIX. The rename gates the
  final paths referenced by `07-execution-order.md` Steps 0.5/3/5 prose
  (which cite both spellings until the rename lands).
- **11 (home provisioning + lockfile)** depends on ADR 0025 and composes with
  [06](06-config-home-flag.md) (`--home` flag) — both touch the same
  CLI/home-resolution surface (`cli_actions.rs` args, `home.rs` init,
  home-resolution precedence prose), so they should be implemented in the same
  wave. It extends [10](10-config-repos-as-working-copies.md) Task 3 (the
   `home init` scaffolding) with `home clone <src> [<dest>]` provisioning
   (verb split @ `c406630`) and the
   generated `workestrate.lock`. Its code tasks are NEEDS-DEVSHELL / HOST-NIX.
- **12 (per-instance addressing)** depends on ADR 0026 and supersedes ADR 0021
  §5 (`--port-offset`, removed pre-release). Wave 1 (12a) is the slot-based
  binding + bind-aware registry + surfacing + `--port-auto` code landing on
  `migration/tool-model`; Wave 2 (12b) is `depends_on` discovery-lite — BOTH waves landed (Wave 1: `9107b87`/`de9aa62`/`f9fd2f0`/`c5837e7`; Wave 2: `4adad3f`/`7b65ad1`/`39c1694`). The
  guest-reachability of non-`127.0.0.1` loopbacks is gated on Experiment E1
  ([../05-host-validation.md](../05-host-validation.md)) — the conservative
   default (shared `127.0.0.1` + `--port-auto` for guest-facing alternates)
   holds until E1 runs and its outcome is recorded in the spec's
   §open-decisions.
- **15 (tombi TOML toolchain)** ↔ 13/14 (notation): tombi validates whichever
  notation the author chose (array-of-tables or collapsed map form) — it
  normalizes layout only and never forces a form, so the 13/14 ergonomics are
  unaffected by the toolchain adoption.
- **18/19 (INTENT-TO-EXPLORE explorations)** are both seeded by the ADR 0026
  addendum (2026-08-01) default-on lifecycle: 18 explores cross-home
  composition (the wider
  `workestrate up` across homes/config sets), 19 enumerates inspection
  surfaces beyond the W3 `workestrate workloads` verb (anchored by ADR 0027,
  verb-first workload dispatch). Neither gates code.
- **23 (DESIGN/DEFERRED)** gates nothing now; when implemented, its recommended
  fork-as-flake consumption unblocks spec 22's §14 runtime enforcement.

### 13 — Config ergonomics: string-or-table shorthand for `secret_env`

`secret_env` entries today must be inline tables (`{ secret = "NAME" }`) even
though `SecretEnvConfig` has exactly one field — the litellm workload alone
spends 11 lines on nine bare-name tables that collapse to 1 line. The spec
accepts a heterogeneous array (bare strings as shorthand for
`{ secret = "NAME" }`, inline tables as the kept full form), implemented via an
untagged helper enum normalized to `Vec<SecretEnvConfig>` at deserialize time
— merge, validation, and plan build are untouched. `schema_version` stays 1
(the shorthand is additive and backward-compatible; the first breaking change,
if any, would be the schema v2 event). **Key decision:** serde-boundary-only
expansion — the parsed data model is byte-identical, golden plans stay
byte-unchanged, and typo'd bare names are caught precisely by
`validation.rs:318-326`.

**EXECUTED 2026-07-31** (commits `a349d03` + `1e7dc25`): `SecretEnvShorthand`
untagged enum + `deserialize_secret_env` normalization landed in
`control/agentctl/src/config/types.rs` with a custom bad-element error naming
the index/type/forms; 8 parse tests + 1 validation typo regression test; 492
tests green; golden plans byte-unchanged; schema regen committed (`anyOf`, not
`oneOf`); `spec_examples_parse.rs` promoted to real lib types (mirror deleted);
personal config converted in `.tmp` (separate commit there).

**SUPERSEDED 2026-08-01** by the v2 unified secret/env model (1ed2e6d): the
`secret_env` namespace was removed from the v2 schema; the bare-string
shorthand survives only inside the one-cycle v1 compat shim
(`fold_legacy_secret_model`). **Annotated 2026-08-01 (final model):** the
final unified secret/env model ([16](16-unified-secret-env-model.md))
replaces the shorthand entirely with the `true` sugar (`KEY = true`).

### 14 — Config ergonomics: map form for `env` entries (formatting collapse)

`env` entries today must be `[[workloads.x.env]]` array-of-tables blocks even
though the census (personal config: 20 blocks across 5 workloads; 16 literal,
4 secret) shows only 2 of 3 `EnvVarConfig` fields are ever used per entry.
The spec accepts EITHER the current array-of-tables OR a nested map
(`[workloads.x.env] KEY = "literal" | KEY = { secret = "NAME" }`), normalized
at parse time to `Vec<EnvVarConfig>` via a custom `deserialize_env` Visitor
(`visit_seq` unchanged; `visit_map` collects in DOCUMENT ORDER — toml 0.8.23
→ `toml_edit::de` → `indexmap` is unconditionally ordered; no `preserve_order`
feature, no new deps). Merge, validation, and plan build are untouched;
`schema_version` stays 1; the shared-blocks/`use` mechanism was explicitly
REJECTED. **Key decision:** never sort — `plan.rs:216` renders env in Vec
order and fixtures/tempest rely on non-alphabetical, interleaved order, so
`visit_map` document-order preservation is the gating constraint.

**Executed 2026-07-31** (commits `1035073`, `564deca`): `deserialize_env`
Visitor + `EnvSecretRef` (deny_unknown_fields) landed in
`control/agentctl/src/config/types.rs`; 10 new tests (document-order assertion
ZEBRA/MIDDLE/ALPHA, dup-key parse error, redefinition parse error, typo
regression with exact error `workload 'pi' env references undefined secret
'GITHUB_TOKEN_TYPO'`); 498 tests green; golden plans byte-unchanged; schema
`anyOf` [array, object]; `config.reference` example-service converted
(golden-equivalent).

**Annotated 2026-08-01:** the map form became the v2 unified env binding map
with typed `EnvBinding { Literal | Secret }` (1ed2e6d); the array-of-tables
form still parses via the same visitor for one shim cycle. **Annotated
2026-08-01 (final model):** the map survives as the ONE unified env map in
the final model ([16](16-unified-secret-env-model.md)); binding semantics
gained `bound` (default `host` = placeholder; `guest` = real value) and the
`true` sugar.

### 15 — TOML toolchain: tombi format/lint/schema-validation

Adopts tombi v1.2.5 (MIT) as the TOML formatter + linter + schema validator
for the repo, scaffolded config repos, and homes, packaged via
`nix/packages/tombi.nix` using the fetchurl-prebuilt pattern (the
`cargo install tombi-cli` path is a trap — a 0.0.1 placeholder; the nixpkgs
pin `a799d3e3` only ships 0.11.6, whose config keys differ from 1.x). The
gating fact is that tombi's formatter normalizes layout only (19 lexical
rules) and **cannot restructure** array-of-tables to map form — the 13/14
collapsed notation is already schema-valid, so tombi keeps it tidy + validated
but never forces a form. Three `tombi.toml` designs (repo root / scaffolded
config repo / home) with explicit include/exclude sets; the schema is vendored
into the binary via `include_str!` (compile-time template/schema version
match, `schema_drift.rs` freshness guard) and emitted into config repos as a
static copy, with the `#:schema` directive switching from the floating GitHub
URL to the relative `./schemas/workestrate.schema.json` at the 3 sites.
Hooks: a config-repo pre-commit hook (new const, installed after the git-init
block, `TOMBI_REQUIRED=1.2.5` version guard + `format --check` +
`lint --error-on-warnings`) and a home-hook tombi gate (warn-not-fail when
tombi absent). Flake wiring: `lib.checks.tombiCheck` (mirroring
`flake.nix:140-154`), `scripts/check-toml.sh`, and a `tombi-check` just
recipe wired into `just verify`. **Key decision:** package the pinned v1.2.5
binary rather than rely on the nixpkgs pin's 0.11.6.

**EXECUTED 2026-08-01** (commits `92b6b6f` — tombi 1.2.5 nix package +
devshell; `d97576d` — scaffold `tombi.toml` + vendored schema + config-repo
pre-commit hook; `9ffad0e` — repo-wide gates: root `tombi.toml`,
`scripts/check-toml.sh`, `just tombi-check` wired into `just verify`,
`lib.checks.tombiCheck`, home-hook gate + home `tombi.toml`/schema emission;
plus the `--empty`-mode and template self-format gap fixes in this wave;
personal config repo applied at `0750876`). Planted-violation evidence:
unknown key → "not allowed" error; `cpus = "two"` → type error; both exit 1.
Deviations recorded in spec §7.1 (`[schema.catalog] paths = []`,
`TOMBI_OFFLINE=true`, fixtures format-only + excluded from schema globs,
template self-formatting, copier parity HOST-NIX-gated).

---

## Environment markers

Reproduced from [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md)
§Environment markers:

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

---

## Inconsistencies between specs (for lead reconciliation)

These are discrepancies found while reading the specs (01–07, pre-08/09); they do not block
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
   to match 04's verified evidence. **Superseded 2026-07-29:** nix IS installed in this container at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` (not on PATH) and `nix develop` provides a full C toolchain (gcc 15.2.0, cargo 1.97.1) — all cargo-linked gates are `verifiable-here` via the store-path PATH prefix; see [../07-execution-order.md](../07-execution-order.md) Step 2 env-marker resolution.

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
   **RESOLVED:** runtime parse verification DONE — `workestrate
   validate-config` → "workestrate.toml is valid." exit 0 (Step 0.5,
   2026-07-30).
