# NEXT-SESSION — handoff prompt

Spec 22 host policy-file preparation and diagnostics CLI landed. Continue at the documented
SDK integration seam when the pinned microsandbox dependency gains the policy field.

> **2026-08-13 update — MERGE-READINESS ASSESSMENT DONE (no merges executed).** `feature/mount-masking` (24 commits on base c45494b) is ready to merge onto wr `migration/tool-model` @ `5c341ad` once the 8 content conflicts + 12 semantic auto-merges are resolved and the ADR 0028 collision is handled (renumber this branch's policy-scopes ADR → 0029). msb side verified mergeable (33 commits linear on fork main b43d7522). Full inventory + runbook: `~/Development/agent-workbench/handovers/2026-08-13-mount-merge-readiness.md`.

> **STATUS: HANDOFF**
> Prerequisites / see-also: [README.md](README.md) · [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [07-execution-order.md](07-execution-order.md)

> **2026-08-03 update — spec 23 (microsandbox fork nix flake packaging) authored** as DESIGN/DEFERRED at [06-improvements/23-microsandbox-fork-nix-flake-packaging.md](06-improvements/23-microsandbox-fork-nix-flake-packaging.md). Captures the deferred idea of packaging the fork as a nix flake output (encapsulated source-build; lockstep binary+SDK from one rev; reversibility via input-ref change). Implementation explicitly deferred until workestrate+passthrough usage stabilizes on host; no flake code written now; no ADR (deferred design). Supersedes spec 09 option 3 for the masking use case; unblocks spec 22 §14 runtime enforcement when implemented.

This file is the self-contained handoff for the next contextless session working
the `migration/tool-model` branch. It assumes no prior conversation. Everything
below is drawn from the four cited docs, which are the source of truth — do not
re-derive their contents.

> **⚠ CONTAINER-HOME EPHEMERALITY CAVEAT:** the container `$HOME` is ephemeral.
> A container restart on 2026-07-30 **WIPED** the `~/.workestrate` home. The home
> has since been **RESTORED** — restore is 3 commands:
> `workestrate home init` + `workestrate config add
> <repo>/.tmp/config-repos-export/personal personal` + `workestrate config
> trust /home/node/Development/ai-workbench`. The personal config content
> survives durably at `ai-workbench/.tmp/config-repos-export` (verified
> `c41a707`). The durable fix is the host-side home (operator task — open
> thread ⑦ below). The container home's own git is uncommitted — fine, it is
> ephemeral.

---

## Current state (as of 2026-08-01)


> **2026-08-04 update (12) — mount-masking polish + rebase complete on
> `experimental`:** the branch is rebased onto
> `sibling/migration/tool-model` @ `c45494b` (spec 21 image build/load
> lifecycle + plan-time preflight + directory-mode fixes now in the
> experimental lineage). The `case_sensitivity` parity is ported from the msb
> side (deserialize-path recompile, commit `587e3af`). The dead
> `mount_policy()` shim is removed (`d42001a`). An operator guide
> ([mount-masking-operator-guide.md](mount-masking-operator-guide.md)) and
> example configs ([examples/](examples/)) are added. HEAD is `0f7ca20`.
> The SDK seam (`apply_mount_policy`) remains a no-op pending the fork dep
> switch (spec 23, DESIGN/DEFERRED). Remaining: msb-side rebase onto fork
> `4a3133e5` lineage, dep switch (spec 23), HOST-KVM runtime smoke for spec 22
> §14. The 11 beads handover issues (`.beads/issues.jsonl`) track the rest.


> **2026-08-02 update (7-spec22) — spec 22 + ADR 0029 authored on the
> `experimental` branch:** spec 22 (dynamic mount masking policy) is
> DESIGN-APPROVED at
> [06-improvements/22-dynamic-mount-masking-policy.md](06-improvements/22-dynamic-mount-masking-policy.md);
> ADR 0029 records collect-and-compile (never merge). Spec 01 is
> dispositioned to SECONDARY/FALLBACK (WP1–WP4 frozen; deleted if spec 22
> lands).

> **2026-08-03 update (11) — spec 21 phase E (lifecycle wiring) is IMPLEMENTED
> in the working tree but UNCOMMITTED and UNVALIDATED.** HEAD is `3efd377`
> (post-Phase-D beads commits `30bfdb1` + `3efd377`; last CODE commit is Phase D
> `932b476`). 12 dirty files (10 modified + 2 intent-to-add):
> `control/agentctl/src/{cli_actions.rs, commands/deps.rs,
> commands/lifecycle.rs, images/build_cmd.rs, images/ensure.rs (new),
> images/mod.rs, main.rs, microsandbox/runtime/mod.rs,
> microsandbox/workload/mod.rs}` + `tests/{ensure_images_e2e.rs (new),
> flake_root_gate.rs}`. Key symbols: `images::ensure`
> (`ensure_should_run` / `ensure_images_for_workload(s)` / `ensure_resolved`
> reusing `build_cmd::process_target`); `InstanceSpec.images_ready`;
> `detach_args` appends `--images-ready` unconditionally (never
> `--reload-images`, D3); `--reload-images` on up/exec/batch-up;
> `cmd_workload_up_all(json, reload_images)` batch-ensures all starts before
> any spawn; `EnsurePreflight` dep-auto-start inheritance (`force=false`);
> `bare_up_reject_table` excludes `--reload-images`. **DO NOT DISCARD the
> working tree.** Next session MUST: (1) run targeted tests for the new code
> (cargo test for ensure.rs in-module tests + main.rs detach-args/bare-up tests
> + flake_root_gate renamed test) —
> `cargo test -p workestrate-agentctl ensure should_run missing_flake
> batch_force unforced nix_absent non_nix detach_args images_ready bare_up
> nix_layered_up_without_declaring_flake` via `nix develop`; (2) run `just
> verify` for full green (expect 721 baseline + new Phase E tests); (3) commit
> Phase E as e.g. `feat(images): ensure-images pre-flight lifecycle wiring
> (spec 21 phase E)` — stage ONLY the 12 code files; (4) then HOST-KVM gates
> (the #[ignore]'d ensure_images_e2e KVM variant + stale-tag rebuild e2e).
> Docs (STATUS/NEXT-SESSION/spec-21) were reconciled to this reality on
> 2026-08-03 in a docs-only commit. Phase F (multi-repo migration) remains
> pending after E lands.

> **2026-08-02 update (10) — spec 21 phase D (build/load pipeline) landed**
> (one commit on top of phase C `0729bb4`): the phase-C seam is now the REAL
> pipeline in `images/pipeline.rs` — async + trait-seamed (`ImageBuilder` /
> `ImageLoader`, cfg(test) fakes, the detect.rs pattern), running inside the
> build verb's still-held per-tag lock: `nix build <flake_root>#<attr>
> --no-link --print-out-paths` (stderr TEED live to the TTY AND retained for
> the §7 classification: fakeHash mismatch → `update-hashes` pointer;
> fetch/substituter → offline-context error; generic → stderr tail) → the
> §3.1 outPath re-load gate (pure `reload_decision`; skip `msb load` iff
> record out_path matches AND the tag is still in the store; out-of-band
> deletion loads anyway; phase-C `out_path=""` trust records upgrade on next
> build) → `gunzip -c <outPath>` | `msb load -t <tag>` (no shell, no staged
> tarball; msb via the doctor `MSB_PATH` convention) → post-load store
> re-probe (load success + tag gone = named error) → record upsert + save
> INSIDE the lock (`digest = None` — the §3.5 probe point). `process_target`
> now takes the 4 seams as `TargetSeams`; gate-skip reports the exact note
> `image unchanged in store; tag already current`. New read-only doctor
> check `image_records` (records vs `msb image ls`; missing tag or
> unreachable store = WARN, never FAIL). **E2E ran FULLY in-container** with
> a ~20 KiB fixture image (`tests/fixtures/image-flake/`, nixpkgs pinned to
> the repo's flake.lock rev, no FODs): `tests/image_pipeline_e2e.rs` (gates
> on nix + an explicit `MSB_PATH` to an UNWRAPPED msb — the devshell's
> wrapped `msb` FORCES `MSB_HOME=$HOME/.microsandbox`, so never run it
> through the wrapper) plus a real-CLI smoke (build → skip → `--force`
> gate-skip → out-of-band-delete reload; doctor OK→WARN). **§11 HOST-VERIFY:
> 4 of 5 items VERIFIED in-container** (digest surface exists; load/query
> normalization identical-verbatim; concurrent same-tag `msb load` unsafe —
> the outer flock is load-bearing; git+file dirty-worktree drvPath stable).
> Tests: 721 passed / 0 failed / 3 ignored (700 baseline + 21 new); `just
> verify` green (`verify-full` left as the host gate). Spec 21 phases E–F
> remain (E `HOST-KVM`; F `HOST-NIX`; phase-D remainder: real
> workestrate-pi/tempest builds + the §3.5 digest capture flip).
>
> **2026-08-02 update (9) — spec 21 phase C (`workload build` + change
> detection) landed** (one commit on top of phase B `290e91b`): new CLI verb
> `workestrate workload build [name] [--repo <config> | --all-repos] [--check]
> [--force]` (spec §5.1; `--json` is the global flag; verb-first only — NOT
> in the legacy shim). New modules: `images/detect.rs` (the `DrvEvaluator`
> and `StoreProbe` seams — real backends `NixCliEvaluator` (hermetic `nix
> eval --raw <flake_root>#<attr>.drvPath`) and `MsbStoreProbe`
> (`microsandbox::Image::get`; unreachable store = the named §7 "db
> unreachable" error reusing the ps.rs vocabulary); the drvPath-only
> freshness predicate; cfg(test) fakes), `images/build_cmd.rs` (selector
> resolution via declaring-layer provenance; the lock → probe → eval → skew
> → act flow; `--check` is lock-free and write-free; D1 trust records write
> `out_path = ""` until phase D), `images/pipeline.rs` (the marked **SPEC 21
> PHASE D SEAM** — `run_build_pipeline(BuildJob)` refuses with "build
> pipeline not yet implemented (spec 21 phase D)"). `skew.rs` `StoreTag`
> stays 2-variant (unreachable = named error, not a decision). Spec 21 §3.4
> gained a phase-C addendum recording these. Tests: 700 passed / 0 failed /
> 3 ignored (673 baseline + 27 new); `just verify` green. Smoke: real-repo
> drvPath eval works IN-CONTAINER (personal flake inputs already in the
> store); all `--check` selector forms green against the dev home with
> `state/` untouched; phase-D seam refusal + §7 vocabularies demonstrated
> live. Discovery: dev-home `personal-v2` fails the CURRENT policy gates
> standalone (`default_deny=false` without entitlement — pre-existing repo
> drift); `--all-repos` skips such repos with a note. HOST-NIX deferrals:
> `nix build`/`msb load` (phase D) and a live D1 trust against a populated
> store (the in-container msb store is empty). Spec 21 phases D–F remain
> (D/F `HOST-NIX`; E `HOST-KVM`).
>
> **2026-08-02 update (8) — spec 21 phase B (image-state store) landed** (one
> commit on top of phase A `02bea9a`): new library-only module
> `control/agentctl/src/images/` — `state.rs` (the `state/images.json` spec §8
> schema keyed by `<repo>#<tag>` with a `#` separator, atomic tmp+fsync+rename
> saves, corrupt/absent tolerance per the advisory-record posture), `lock.rs`
> (per-tag `state/image-locks/<sanitized-key>.lock`, O_EXCL + blocking-until-
> acquired + stale dead-PID recovery REUSED from the port-registry lock — NOT
> flock(2), because `unsafe_code = "forbid"` makes the unsafe `libc::flock`
> FFI uncallable; the pre-approved `libc` dep was NOT added and the
> kernel-release upgrade is a recorded follow-up), `repo_key.rs` (spec §4.3/§8
> registered-name-vs-canonical-path resolution, pure core + registry
> wrappers), `skew.rs` (the pure §3.4 skew matrix incl. the D1 TRUST branch
> and the `--reload-images` Skip/Trust → RebuildForced flip, exhaustive table
> test). No clap wiring, no production callers — phases C–E wire it later.
> Spec 21 §6.1 gained a phase-A addendum (the `.workestrate-build/` contract
> ships in the config-repo README, not inside the gitignored dir). Tests:
> 673 passed / 0 failed / 3 ignored (642 baseline + 31 new); `just verify`
> green. Spec 21 phases C–F remain (C/D/F `HOST-NIX`; E `HOST-KVM`).

> **2026-08-02 update (7) — spec 21 phase A (scaffold part) landed** (in the
> working tree on `migration/tool-model`): `.workestrate-build/` reserved in
> the scaffold template per USER DECISION D4 — a `.gitignore` entry + README
> contract section in BOTH template locations
> (`templates/workestrate-config/` and
> `control/agentctl/src/scaffold/template/`); the `.gitignore` addition is
> byte-identical in both (the `scaffold-check` parity overlap covers
> `.gitignore`; the README is outside the parity set). The undeclared
> `local_build` fallback default changed to `.workestrate-build/<name>`
> (declaring-layer-relative; declared fallbacks never overridden). Spec 21
> phases B–F remain (B `verifiable-here`; C/D/F `HOST-NIX`; E `HOST-KVM`;
> msb HOST-VERIFY cluster).

> **2026-08-02 update (8) — config-surface slice:** registry/config/workload/
> mount policy fields now collect ordered provenance-bearing fragments without
> merge.rs policy merging; runtime transmission and CLI diagnostics remain
> future slices.

> **2026-08-03 amendment:** consolidated write-rules, protect, tagging,
> cascade, symlink, and host-side transmission semantics landed; see spec 22
> §10/§12 and the ADR 0029 addendum. The pure library was revised; msb work is
> still pending.

> **2026-08-02 update (6) — spec 21 (image build/load lifecycle) authored**
> as DESIGN-APPROVED at
> [06-improvements/21-image-build-lifecycle.md](06-improvements/21-image-build-lifecycle.md)
> (user signed off on decisions 2026-08-02); implementation phases A–F pending
> (A/B `verifiable-here`, C/D/F `HOST-NIX`, E `HOST-KVM`; msb HOST-VERIFY
> cluster on the digest surface). The five locked user decisions:
> - **D1** — record-absent + tag-present → TRUST the store tag on plain `up`
>   (rebuild only via `--reload-images`); records are advisory, never
>   authoritative.
> - **D2** — stable tags verbatim from config TOML; no auto-prefixing;
>   cross-config collision → `validate-config`/`plan` WARN naming both repos;
>   manual TOML prefixing is the operator remedy.
> - **D3** — `--reload-images` batch scope = force ALL service workloads in
>   the batch; never forwarded in `detach_args`.
> - **D4** — `.workestrate-build/` reserved at config-repo root — gitignored,
>   artifact-only by construction, scaffold-provisioned, new default for
>   undeclared `local_build` fallbacks.
> - **D5** — cross-home collisions → warn + digest-detect; state records
>   keyed by (config-repo identity, name:tag).

> **2026-08-02 update — cleanup PHASE 0 landed** (one commit on top of
> `09b624f`; HEAD below is stale). Repo-relative mount/seed paths now resolve
> against the DECLARING config layer's directory, and `project_root()` is a
> LAZY gate in `build_sandbox` (nix-layered / local_build / relative
> build-path mounts only, error names the feature). Root-cause note for
> contextless sessions: the `workload up litellm` failure from a flake-less
> cwd was the unconditional flake-root gate (`run.rs`), NOT the stale log
> lines that append-mode logging made look current; `spawn.rs` now writes a
> per-run log delimiter. Tests: 634 passed / 0 failed / 3 ignored; KVM-gated
> regression in `control/agentctl/tests/flake_root_gate.rs`. Known spec drift
> (flagged): spec 17/20 say "repo-relative"; implementation is
> declaring-FILE-dir-relative — see STATUS.md §0.

> **2026-08-02 update (2) — cleanup PHASE 1 landed** (the phase-1 commit, on
> top of phase-0 HEAD `b9a3ed3`). Deleted duplicated
> `config.reference/agents/{pi,odysseus,opencode}` configs, consumerless
> `nix/packages/{odysseus,opencode}.nix` (+ flake attrs + `update-hashes`
> lines), repo-root `var/`/`workspaces/`, and `.assets/opencode-agent`
> (recoverable from git history); `docs/odysseus-full-capability.md` moved to
> the user's personal config repo. Spec-17 mount/seed path text reconciled
> with the phase-0 semantics (declaring-layer-dir); correction: spec 20 had
> no such text (grep-verified) — the phase-0 "spec 17/20" flag was
> spec-17-only. Remaining citation staleness is recorded in STATUS.md §0.

> **2026-08-02 update (3) — cleanup PHASE 2 landed** (uncommitted at time of
> writing; lands as one commit on top of phase 1). Tool repo decoupled from
> personal workflow content: check gates no longer cover `profiles/*` /
> sdk-notes; the `config.reference` base layer is opt-in via
> `WORKESTRATE_REFERENCE_CONFIG=1` (spec-05 cwd gate unchanged on top); the
> fixture is synthetic example-*-only (litellm → example-litellm, goldens
> regenerated). `profiles/`, `docs/litellm/schemas/`, and the
> `validation-litellm-config-check` skill + `litellm-check` recipe moved to
> the personal config repo (`workestrate-dev-home/config-repos/personal`);
> `config.reference/infra/litellm/*.yaml` deleted as redundant with the
> canonical personal-repo copies. Tests: 635 passed / 0 failed / 3 ignored.
> Phase-3 follow-ups recorded in STATUS.md §0 (tests/fixtures litellm
> workload, docs/litellm knowledge-repo decision, tempest package deletion).

> **2026-08-02 update (4) — cleanup PHASE 3 landed** (one commit on top of
> phase 2). Workflow image builds moved OUT of the tool repo: the personal
> config repo flake (`workestrate-dev-home/config-repos/personal`, commit
> `1000e60`) now builds `workestrate-pi:latest` + `tempest:latest` via the
> tool's lib recipes + `buildImagesFromConfig` (one flake input per
> `flake://` source; nix-only enrichment — pi binary_name/install_dir/
> assets, tempest npm_deps_hash HOST-GATE placeholder — attached
> post-parse). Tool side deleted: the 4 source inputs,
> `nix/packages/{pi,pi-bun,pi-image,tempest,tempest-image}.nix`, the
> `workload-images`/`load-images` attrs, the `.#workestrate-sandbox`
> wrappers (`apps.default` now runs the CLI), the devshell
> `WORKESTRATE_PI_BUILD` export + `agents/*/repo` population, and the
> `dev-build-pi`/`dev-run-pi`/`update-hashes`/`store-delta-check`/
> `load-images` just recipes. `lib.<system>` is fully intact (the config
> repo depends on it). Both scaffold templates synced byte-identical with
> the new reality. Tests: 640 passed / 0 failed / 3 ignored; `just verify`
> fully green (real copier parity); `nix flake show` clean (the phase-2
> tempest.nix lambda bug is mooted by deletion). Devshell genericization
> landed in phase 4 (update (5) below). Host follow-ups + citation-staleness
> notes in STATUS.md §0.

> **2026-08-02 update (5) — cleanup PHASE 4 landed** (uncommitted at time of
> writing; lands as one commit on top of phase 3 as
> `refactor(cli): generic CLI surface, policy, and scaffold (cleanup phase
> 4)`; HEAD below is stale). The 5 typed CLI subcommands
> (litellm/pi/odysseus/opencode/tempest) are REMOVED from
> `control/agentctl/src/main.rs` — the generic `workestrate workload <verb>
> <name>` path (ADR 0027) is now the only lifecycle path; stale strings fixed
> (`ps` footer + the ADR-0021-pinned refuse message now emit `workestrate
> workload down <name>`; `down --all` footer → `down-all`). policy.rs: the
> hardcoded personal 7-secret `SECRET_HOST_BINDINGS` table and the hardcoded
> `tempest` `DEFAULT_DENY_FALSE_ENTITLEMENT` are deleted — secrets with
> `env_var` now bind only hosts in `ALLOWED_EGRESS_HOSTS` (fail-closed;
> env_var-less secrets skipped), and `default_deny = false` requires the new
> config-declared `workloads.{name}.entitlements = ["default_deny_false"]`
> (closed vocabulary `ALLOWED_ENTITLEMENTS`; grant-only union with provenance
> before the network gate). Scaffold/templates emit generic
> `EXAMPLE_API_KEY` + `GITHUB_TOKEN` only; the copier `generate-env-example`
> post-copy task is removed. The `litellm_proxy` egress recipe is KEPT as
> generic OSS vocabulary (verdict + evidence recorded in code comments). The
> devshell shellHook now only mutates the repo when the caller's toplevel is
> the workestrate tool checkout (marker probe) — `nix develop` from another
> repo no longer litters vendor symlinks/agents build dirs there; banner
> strings ai-workbench→workestrate. Flaky `probe_free_ports_*` tests now
> re-probe on TOCTOU bind conflict (up to 8 cycles). `check_required_files`
> optional entries genericized to `example-{agent,offensive}`. Coordinated
> personal-repo change (committed by lead): `tempest/workload.toml` declares
> the entitlement. Validation: 3 consecutive full cargo test runs 640 passed
> / 0 failed / 3 ignored each (matches baseline); `just verify` exit 0; smoke
> green (`--help` personal-subcommand-free, TempDir `config new` generic-only
> secrets, `check` green, devshell scratch-cwd write-free). Closing sweep:
> ~600 test-fixture/doctest personal-name hits deferred as a future dedicated
> sweep. Remaining phases 5–6 items are USER DECISIONS — open thread 12 and
> STATUS.md §0/§5 item 11.

- HEAD: `3efd377` (post-Phase-D beads commits; latest code commit is Phase D
  `932b476`; Phase E implemented-uncommitted-unvalidated in the working tree —
  see the 2026-08-03 update note above).
  **Phase B landed (2026-08-01):** final unified secret/env
  model `19b2cf0` (spec 16 — per-binding `bound`, `true` sugar,
  `allowed_hosts`, `schema_version` back to 1), directory-mode config repos
  `e3194d9` (spec 17 — `workestrate/{default,secrets}.toml` + `workloads/`
  capsules), tombi include globs `ea48f45`, patch slim `19d94e8`.
  personal-v2 is at directory-mode + final-model; `schema_version = 1`
  everywhere (v2 retracted pre-release). Branch `migration/tool-model`,
  working tree clean.
- Gates: green — ~608 tests; `just lint-nix` passes; `just tombi-check` is part of `just verify`.
- Container home `~/.workestrate` restored after the container-restart wipe,
  but uncommitted in its own git (fine — ephemeral).
- 19 improvement specs total in `06-improvements/`. OBSOLETE: **02** (main
  rename — applied in Step 0(a), mooted by spec 08 execution). IMPLEMENTED/
  DONE: **06** (`--home` flag), **08** (no repo-local home), **10** (config
  repos as working copies + dotfiles home), **11** (home provisioning +
  lockfile), **12** (per-instance addressing + discovery-lite — refuse-only
  /no-auto-start stance SUPERSEDED 2026-08-01 by the ADR 0026 addendum
  default-on lifecycle; `--use` override retained; W5 config-wiring plan in
  spec §4), **13** (secret_env shorthand — commits a349d03, 1e7dc25;
  shorthand SUPERSEDED by the final model's `true` sugar), **14** (env map
  form — commits 1035073, 564deca; map survives as the final model's unified
  env map), **15** (TOML toolchain: tombi — commits 92b6b6f, d97576d,
  9ffad0e + scaffold gap-fix wave; personal-repo apply 0750876), **16**
  (final unified secret/env model — commit 19b2cf0, 2026-08-01), **17**
  (config repo directory mode — commit e3194d9, 2026-08-01; home tombi glob
  ea48f45). INTENT-TO-EXPLORE: **18** (cross-home dependencies), **19**
  (visualization + inspection surfaces).
- **Decisions recorded (2026-08-01, commit 33eeb87):**
  - **ADR 0027** ([../migration/50-decisions/0027-verb-first-workload-dispatch.md](../migration/50-decisions/0027-verb-first-workload-dispatch.md))
    — verb-first workload dispatch: `workestrate workload
    {up,exec,plan,down,logs} <name>` + the `workestrate workloads` discovery
    verb; names are args not subcommands; supersedes ADR 0006.
  - **ADR 0026 addendum (2026-08-01)**
    ([../migration/50-decisions/0026-per-instance-addressing-and-discovery.md](../migration/50-decisions/0026-per-instance-addressing-and-discovery.md))
    — compose-mirrored default-on dependency lifecycle: declared `depends_on`
    deps start BY DEFAULT on `up`/`exec` (topological-order closure, singleton
    slots only; service-kind deps start detached with bounded wait-for-port
    ~15s; agent-kind deps REFUSE with remediation); `--no-deps` opts out
    (required dep → refuse-with-remediation; optional dep → fall back per
    convention + warn); an occupied slot counts as satisfied; `plan` NEVER
    starts anything; `--use` RETAINED as a pure instance-selection override
    (unknown instance → hard error; NO parallel auto-start); cycle detection
    becomes MANDATORY in config validation (currently absent — A→B→A loads
    cleanly today); construction-order rule (deps start before the
    dependent's `ConfigWorkload` is constructed); E1 deferral unchanged.
  - **ADR 0021 addendum (2026-08-01)**
    ([../migration/50-decisions/0021-instance-lifecycle-model.md](../migration/50-decisions/0021-instance-lifecycle-model.md))
    — bare `workload up` topo-starts ALL service-kind workloads in the active
    context (detached, singleton slots; agent-kind printed SKIPPED);
    single-context batch — does NOT reopen ADR 0019's multi-context deferral;
    `down --all` remains unordered BY DESIGN.
  - 17 improvement specs total (16 + 17 added as INTENT-TO-EXPLORE).
  - Implementation sequenced as the **W-wave W1→W6** (W3 = the `workestrate
    workloads` discovery verb per ADR 0027; W5 = config wiring retiring
    hardcoded URLs; wave contents per the 2026-08-01 session brief — the
    ADRs are authoritative for semantics, the wave split is recorded here).
- tombi 1.2.5 is on PATH in the devshell (`nix develop`); `just tombi-check`
  is wired into `just verify` (`justfile:71`).
- Key commit refs: `421a54a` (docs env claims); `d7c5a83` (`repos/` →
  `config-repos/` rename); `bd99481` (dirty-guard test); `3894fb7`
  (`workestrate home init`); `bef1c37` (discovery tier removed); `418530a`
  (repo-local home machinery removed); `172d5dd` / `19ff272` / `be356f7`
  (`home init --from` + `workestrate.lock` + lock consumption) — interface SUPERSEDED by the `home init` / `home clone` verb split @ `c406630` (2026-07-31; ADR 0025 addendum); `d991252`
  (`--home` flag); `9107b87` / `de9aa62` / `f9fd2f0` / `c5837e7` (spec 12
  Wave 1); `4adad3f` / `7b65ad1` / `39c1694` (spec 12 Wave 2); `d1c1293`
   (`MSB_AGENTD_PATH` staging); `a349d03` / `1e7dc25` (spec 13: secret_env
   shorthand + spec_examples_parse promotion); `1035073` / `564deca` (spec 14:
   env map form); `f8aa276`; `d938f2e`; `92b6b6f` / `d97576d` / `9ffad0e`
   (spec 15: tombi 1.2.5 package, scaffold emission, repo-wide gates).
- The container-home wipe caveat above is updated to current reality: the wipe
  happened 2026-07-30; the home was since RESTORED (3 commands, see caveat);
  the personal config content survives durably at
  `ai-workbench/.tmp/config-repos-export` (verified `c41a707`); the durable fix
  is the host-side home (operator task — open thread ⑦).
- **2026-08-01: the v2 secret/env model landed** (commit `1ed2e6d`, P1 Wave 1):
  unified `SecretDefConfig` with an explicit `delivery` field
  (`env` | `host_bound`, default `host_bound`); workload `env` is now
  `EnvBindings` (name-keyed map of typed `EnvBinding` = `Literal | Secret`;
  remaps live at the binding site); the `secret_env` namespace was REMOVED
  from the v2 schema (v1 layers fold via the one-cycle shim
  `fold_legacy_secret_model`); `EXPECTED_SCHEMA_VERSION = 2`; spec **13**
   SUPERSEDED; spec **14**'s map form became the v2 binding map. *(SUPERSEDED 2026-08-01 by the final model `19b2cf0` — the v2 `delivery`/shim machinery was retracted pre-release; `schema_version` is 1 everywhere.)*
- **What was done in v2 (4-line summary):**
  1. Unified `SecretDefConfig` + explicit `delivery` field (`1ed2e6d`); workload
     `env` = EnvBindings map; `secret_env` removed from the v2 schema (v1 shim
     folds with warnings); `EXPECTED_SCHEMA_VERSION = 2`.
  2. P0 fix (`d635ef0`): secret-backed env entries resolve against the merged
     secrets map — no more literal `"${LITELLM_MASTER_KEY}"` reaching guests.
  3. Delivery assignments: `LITELLM_MASTER_KEY` + `ODYSSEUS_ADMIN_PASSWORD` +
     the odysseus/opencode `OPENAI_API_KEY` binding are `delivery = "env"`
     (OPENAI_API_KEY flipped from host-bound — on 0.5.6 plain-HTTP the old
     host-bound path substituted nothing); provider keys + `GITHUB_TOKEN` stay
     host-bound (`$MSB_*` placeholders + TLS substitution).
  4. Personal config migrated to native v2 (export `56f3557`, `personal-v2`
     `99c9985`); docs wave `92a3d1f`; 588 gates green in-container; the KVM
     runtime smoke is pending (05 B13 — open thread 11).
- **2026-08-01: the FINAL secret/env model + config-repo layout docs wave.**
  Two new specs added: **spec 16**
  ([06-improvements/16-unified-secret-env-model.md](06-improvements/16-unified-secret-env-model.md)
  — authored by a parallel task; authoritative for the definitions below) and
  **spec 17**
  ([06-improvements/17-config-repo-directory-mode.md](06-improvements/17-config-repo-directory-mode.md)).
  Final-model summary (spec 16 is authoritative — do not re-derive here):
  a unified secrets CATALOG; env values take **the four env value forms defined
  in spec 16**; **bound defaults** are supported; **`allowed_hosts`** is
  declared per secret.
- **Spec-number assignment (2026-08-01):** **16 = unified-secret-env-model**,
  **17 = config-repo-directory-mode**. The previous exploration specs 16/17
  (cross-home dependencies; visualization + inspection) are renumbered by
  parallel task A — this file defers to
  [06-improvements/00-index.md](06-improvements/00-index.md) for the final
  numbering.
- **Phase B state (2026-08-01): LANDED.** Final model `19b2cf0`, directory
  mode `e3194d9`, tombi globs `ea48f45`, patch slim `19d94e8`; personal-v2
  at directory-mode + final-model; `schema_version = 1` everywhere.
  PENDING: the host batch B1–B13 + KVM items (incl. B13 final-model
  secret-delivery smoke + E1 guest-reachability), the upstream microsandbox
  PR force-push + open (USER action, branch @ `bc7640b8`), spec 05
  cwd-fallback fix, spec 01 mounts WP1–WP3 (+ Phase 0 KVM spike), spec 03
  dogfooding B1/B2, spec 20 (schema-evolution) to be written, and
  Experiment E1.
- **2026-08-02 (in-flight on `migration/tool-model`; fix landing as a new
  commit on top of `867a96e`):** `workestrate home clone <src> <dest>` from a
  git-initialized but COMMITLESS source home took the git-clone path, yielding
  an empty tree (`config.toml` uncommitted) and breaking provisioning. Root
  cause: the git-clone path never checked for a resolvable HEAD. Fix: it now
  requires one (new `git_has_head` helper in `control/agentctl/src/git.rs`);
  commitless sources fall back to the existing file-copy path. Regression
  test: `from_commitless_git_src_falls_back_to_file_copy` in
  `control/agentctl/tests/cmd_home_provision.rs`.

---

## Open threads (pending-points register)

1. **Upstream microsandbox PR — HARDENED, READY TO OPEN.** Fork branch
   `fix/filesystem-agentd-path-override` amended to `bc7640b8` with the
   8-point review hardening + 6 build-script tests; PR draft
   `.tmp/msb-upstream/PR.md` updated. Next actions (pending USER):
   force-push with `git -C .tmp/microsandbox push --force-with-lease origin
   fix/filesystem-agentd-path-override`, then open the PR from
   `.tmp/msb-upstream/PR.md`. Interim 0.5.6 nix patch aligned to the same
   semantics + the read-only-dest remove fix (`daa5140`; gates green —
   574 tests). See spec 09 §"Review hardening — EXECUTED (2026-08-01)".
2. **Upstream perpetual-rebuild finding (upstream's bug, NOT our PR).** Their
   `crates/filesystem/build.rs` watches a nonexistent `<workspace>/build/agentd`
   path, so the build script re-runs on every cargo invocation. Possible
   separate upstream issue later — do NOT conflate with our MSB_HOME parity PR
   (thread 1).
3. **One flaky lib test seen once** (374/375 passed in one run; NOT reproduced
   in ~10 subsequent runs — likely timing-sensitive). Watch it; no action
   unless it recurs.
4. **~~`just verify`'s litellm-check needs PyYAML~~ — MOVED (2026-08-02,
   cleanup phase 2).** `litellm-check` left the tool repo for the personal
   config repo's justfile; the PyYAML note applies there (run inside a
   devshell with PyYAML).
5. **Experiment E1 NEEDS-KVM** (guest-reachability of non-`127.0.0.1`
   loopbacks; in `05-host-validation.md` B10; the deferred binding decision
   stays NEEDS-KVM per ADR 0026).
6. **Host batch B1–B13** (single KVM-host pass per `07-execution-order.md`
   Step 6).
7. **Host-side home setup (operator task):** `workestrate home init` +
   `workestrate config add` from `.tmp/config-repos-export` + trust + compose
   mounts, on the host.
8. **Container-home ephemerality:** the container `$HOME` is ephemeral;
   restore = 3 commands (see caveat note in Current state).
9. **Re-enter `nix develop` after pulling** — the `MSB_AGENTD_PATH` export is
   new (commit `d1c1293`); stale devshells lack it.
10. **W2a: migrate the personal config repo to native `schema_version = 2` —
    CLOSED (2026-08-01).** Landed: `.tmp/config-repos-export/personal` @
    `56f3557` and `.tmp/config-repos/personal-v2` @ `99c9985` — LITELLM_AUTH
    remap def + `description` fields dropped; remaps live at binding sites
    (`OPENAI_API_KEY = { secret = "LITELLM_MASTER_KEY" }`). Remaining tail: the
    live home checkout (`~/.workestrate/config-repos/personal`, still `c41a707`
    v1, parses via the shim) refreshes as part of the host-side home setup
    (thread 7).
11. **Schema v2 secret-delivery smoke suite NEEDS-KVM** — full commands + pass
    criteria in `05-host-validation.md` **B13**; one line per item: *(2026-08-01: re-pointed at the final model per spec 16 §9 — guest-bound verifiers + host-bound placeholders + `allowed_hosts`; the v2 framing below is historical.)*
    (1) litellm: `LITELLM_MASTER_KEY` real value in guest (P0 regression check)
    + `/v1/models` 200;
    (2) pi: `models.json` `${LITELLM_MASTER_KEY}`/`${LITELLM_ADDR}`
    substitutions resolve to the port-registry address;
    (3) odysseus: `ODYSSEUS_ADMIN_PASSWORD` real value in guest env
    (`delivery = "env"`, no hosts);
    (4) odysseus/opencode: `OPENAI_API_KEY` real value in guest env (the
    env-delivery flip; first working litellm auth on 0.5.6);
    (5) provider keys: `$MSB_*` placeholders only in guest + a chat completion
    per provider succeeds (TLS substitution);
    (6) `GITHUB_TOKEN`: placeholder in guest + git/gh to
    github.com/api.github.com succeed from an agent sandbox;
    (7) failure semantics: missing required secret → start refuses naming it;
    placeholder value → rejected;
    (8) v1 shim: legacy forms load with stderr deprecation warnings and fold
    correctly.
12. **Cleanup phases 5–6 (remaining genericization) — USER DECISIONS.**
    Phase 4 landed the CLI/policy/scaffold/devshell genericization (update
    (5) above; STATUS.md §0). Phases 5–6 await user decision and are NOT in
    flight: age-key path rename (`ai-workbench-secrets.txt`); cache path
    renames (`~/.cache/ai-workbench-msb`, `CARGO_TARGET_DIR` ai-workbench);
    README/SPEC reframing (headline personal examples); canonical
    config-flake input URL (`git+file://` vs `github:georgrybski/...`);
    broad `ai-workbench` user-facing string sweep; `ALLOWED_EGRESS_HOSTS`
    still contains personal provider hosts (api.kimi.com /
    api.neuralwatt.com / api.minimax.io) — the same violation class as the
    phase-4-retired secret table; test-fixture personal-name sweep (~600
    `#[cfg(test)]`/doctest hits, deferred by the phase-4 closing sweep).
    Standing: container-home ephemerality/host-side home (threads 7/8);
    `stash@{0}` on `406b5b5` never to be touched.

---

## Remaining specs + recommended execution order

The remaining work is now sequenced as the W-wave (W1→W6 — see
`07-execution-order.md` "Remaining improvements"), with the six-spec backlog
below running after/parallel.

1. **05 cwd-fallback** (small) — the only bug-fix improvement; small, closes a
   silent config-discovery backdoor; gate `cargo test` runnable in-container
   via `nix develop`.
2. **01 mounts WP1–WP3** (in-container; WP4 + Phase 0 spike are KVM and fold
   into the host batch) — highest-value hardening; WP1–3 verifiable
   in-container.
3. **03 dogfooding B1/B2** — structural isolation for self-development; B1/B2
   verifiable-here (B3 has a KVM tail).
4. **07 naming leftover** (trivial) — mechanical residue sweep; banner says
   DONE with cargo gates pending (run via `nix develop`).
5. **09 post-merge cleanup** — upstream-latency-bound; local action resumes
   only after the ON-HOLD PR is pushed/merged/released (then delete
   compensation machinery + bump pin).
6. **04 CLI config authoring** — DEFERRED by design until
   `02-config-requirements.md` sign-off.

---

## Environment honesty

This container has:

- **No KVM** (`ls /dev/kvm` → not found) — no sandbox runtime can execute.
- **No sops age key** (`~/.config/sops/age/` absent, `SOPS_AGE_KEY` unset,
  `sops` not on PATH) — secret decryption FAILS CLOSED. Do NOT attempt to work
  around it; secret provisioning is HOST-only.
- **`cc` via `nix develop`** — a bare shell has NO `cc` linker (verified:
  `command -v cc gcc` → not found), but nix IS installed at
  `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin` (not on PATH) and
  `nix develop` provides a full C toolchain (verified 2026-07-29:
  `export PATH="/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin:$PATH"`
  then `nix develop -c bash -c 'cc --version'` → gcc 15.2.0; `cargo 1.97.1`,
  `rustc 1.97.1`). All cargo-linked gates (`check`, `test`, `spec-examples`,
  `golden-check`, `schema-check`, `scaffold-check`) are runnable here via
  `nix develop`.

HOST-KVM gates and the genuine HOST-NIX gates (`nix build` image builds,
`nix run nixpkgs#...` prefetch jobs, `just verify-full`, `just generate-schema`)
are DEFERRED per `05-host-validation.md` and BATCHED into the single host pass
at `07-execution-order.md` Step 6 (B1–B12). Do not make a host trip for one
gate — batch.

---

## How to work

Work `07-execution-order.md` in order, top to bottom (see its
"Remaining improvements — recommended order (2026-08-01)" section for the six
not-yet-landed specs; Steps 0.5/8a/8c/8d and spec 15 are DONE). Report lane-by-lane
honestly: `verifiable-here` (this container: TOML, golden files, git,
shell/python, AND cargo-linked gates via `nix develop`) vs `HOST-NIX` (host
only: nix builds, FOD hashes, `just verify-full`, `just generate-schema`) vs
`HOST-KVM` (runtime: service boot, agent exec, instance lifecycle). Report
validation lanes honestly: `validated` / `partially_validated` /
`not_validated`.

---

## Experiment home

The disposable experiment home setup is `03-sibling-config-setup.md` —
bootstrap is `export WORKESTRATE_HOME=/tmp/workestrate-exp` → `just workestrate
config new exp --empty`. Keep it OUTSIDE the repo. Destructive probes (P1–P5:
contexts, instances, mounts, secrets, seed_files) ONLY EVER happen there —
never against the real home. Guard invariant: `test "$WORKESTRATE_HOME" =
"/tmp/workestrate-exp"`.

---

## Constraints

- **No code/config semantic change without a cited ADR.** ADRs live at
  `ai-workbench/docs/migration/50-decisions/`; they stand and are not
  re-litigated here.
- **The config-requirements contract is `02-config-requirements.md`** —
  schema, merge/layering, policy ceiling, trust model. It is frozen pending
  sign-off.
- **The CLI authoring tool (`06-improvements/04-cli-config-authoring.md`)
  stays DEFERRED** until that contract is signed off. Do not start it.
- **Destructive operations only in the experiment home.**
- **Report validation lanes honestly:** `validated` / `partially_validated` /
  `not_validated`. Do not imply correctness beyond what was actually run.

---

## Report back

End every session with three things:

- **What was confirmed** — with evidence (command + output, or file:line
  citation).
- **What is blocked and exactly why** — name the missing capability (KVM /
  sops age key; note: `cc` is available via `nix develop` in this container,
  so cargo gates are NOT blocked) and the step it gates.
- **The next concrete action** — one sentence, the very next command or edit.

---

## Maintenance

Update this file whenever `README.md`, `01-current-state-and-prereqs.md`, or
`07-execution-order.md` state changes materially — e.g. a spec moves from SPEC
to IMPLEMENTED, a Step's env marker changes, a new improvement spec is added,
or an open thread resolves. The handoff prompt must never go stale: its facts
are a strict subset of the four cited docs, so when those docs change,
re-derive the affected prompt section from them and note the update here.

**2026-07-31 refresh:** full rewrite to current reality — stale "IN-PROGRESS
spec 07" / "spec 09 IN-FLIGHT" / "next session starts at Lane A" framing
removed (those waves are done); Current state, the nine-item pending-points
register (Open threads), and the six-spec recommended execution order added;
the container-home wipe caveat updated (wipe 2026-07-30, home since restored);
spec 09 re-framed as PR PREPARED, ON HOLD.

**2026-08-01 refresh:** spec 15 (tombi TOML toolchain) EXECUTED — removed from
the remaining list (now six specs); tombi 1.2.5 noted in the devshell;
`just tombi-check` wired into `just verify`.

**2026-08-01 refresh (2):** ADR 0026 addendum default-on lifecycle supersession
annotated on spec 12 (refuse-only /no-auto-start stance SUPERSEDED; `--use`
retained; W5 config-wiring plan in spec §4); exploration specs 16 (cross-home
dependencies) + 17 (visualization + inspection surfaces) added as
INTENT-TO-EXPLORE (17 specs total); W-wave W1→W6 order recorded (ADR 0027 =
verb-first workload dispatch, the W3 anchor; lifecycle semantics live in the
ADR 0026/0021 addenda, commit 33eeb87).

**2026-08-01 refresh (3):** v2 unified secret/env model landed (`1ed2e6d`, P1
Wave 1) — Current-state bullet added; spec 13 SUPERSEDED + spec 14 annotated;
W2a (personal config repo native-v2 migration) added as open thread 10.

**2026-08-01 refresh (4):** W2a CLOSED (personal config migrated to native v2 —
export `56f3557`, `personal-v2` `99c9985`); v2 4-line summary block added to
Current state; host batch widened to B1–B13; v2 secret-delivery smoke suite
added as open thread 11 (05 B13).

**2026-08-01 refresh (5):** final secret/env model + config-repo layout docs
wave — spec 16 (unified-secret-env-model, parallel task A) + spec 17
(config-repo-directory-mode, READY-TO-EXECUTE design) added to Current state;
final-model 4-line summary recorded (unified secrets catalog, the four env
value forms per spec 16, bound defaults, per-secret `allowed_hosts`);
spec-number assignment recorded (16/17; exploration specs renumbered by task
A — defer to `06-improvements/00-index.md`); pending-implementation wave state
added (final model, directory-mode loader, personal restructure, fork/interim
slim reconciliation, old-personal removal, version collapse).

**2026-08-01 refresh (6):** Phase B landed — HEAD `19d94e8`, ~608 tests;
final model `19b2cf0` (spec 16 EXECUTED), directory mode `e3194d9` (spec 17
EXECUTED), tombi globs `ea48f45`, patch slim `19d94e8`; personal-v2 at
directory-mode + final-model; `schema_version = 1` everywhere; spec 02
flipped OBSOLETE (mooted by spec 08); pending list refreshed (host
batch/KVM items, upstream PR push, spec 05 cwd-fallback, spec 01 mounts,
spec 03 dogfooding, spec 20 schema-evolution spec to be written, E1).

**2026-08-02 refresh:** HEAD ref refreshed `19d94e8` → `867a96e`; the
commitless-home clone bug + fix recorded in Current state (in-flight, landing
on top of `867a96e`; `git_has_head` HEAD gate on the git-clone path;
regression test `from_commitless_git_src_falls_back_to_file_copy`).

**2026-08-02 refresh (2):** cleanup PHASE 1 landed (phase-1 commit on top of
phase-0 `b9a3ed3`) — duplicated `config.reference` agent configs,
consumerless nix packages + flake attrs, repo-root `var/`/`workspaces/`, and
the legacy `.assets/opencode-agent` fleet deleted;
`docs/odysseus-full-capability.md` moved to the personal config repo;
spec-17 path-semantics text reconciled with the phase-0 implementation
(spec 20 confirmed to need no amendment).

**2026-08-02 refresh (3):** cleanup PHASE 2 landed (uncommitted at time of
writing) — personal workflow content decoupled from the tool repo:
opt-in `config.reference` base layer, synthetic example-* fixture,
`profiles/` + litellm schemas/config-check skill + `litellm-check` recipe
moved to the personal config repo; open thread 4 (litellm-check PyYAML)
re-scoped to the personal repo.

**2026-08-02 refresh (4):** cleanup PHASE 3 landed — update note (4) added to
Current state (workflow image builds moved to the personal config repo flake;
tool flake now tool-only; `lib` intact; templates synced; 640/0/3).

**2026-08-02 refresh (5):** cleanup PHASE 4 landed — update note (5) added to
Current state (typed CLI subcommands removed; policy.rs de-personalized with
config-declared `entitlements`; scaffold/templates genericized;
`litellm_proxy` recipe kept as OSS vocabulary; devshell shellHook
checkout-marker containment; flaky port-probe test fix); the phase-3 note's
"Phase 4 owns devshell genericization" forward reference flipped to past
tense; open thread 12 added (phases 5–6 remaining genericization items —
USER DECISIONS).

**2026-08-02 refresh (6):** spec 21 (image build/load lifecycle) authored as
DESIGN-APPROVED (`06-improvements/21-image-build-lifecycle.md`; user signed
off 2026-08-02) — update note (6) added to Current state with the five locked
user decisions D1–D5; index row/summary/dependency-graph entries added;
STATUS.md §5 item 12 added (implementation phases A–F pending).

**2026-08-02 refresh (7):** spec 21 phase A (scaffold part) landed — update
note (7) added to Current state (`.workestrate-build/` reserved in both
scaffold template locations, `.gitignore` byte-parity + README contract
section; undeclared `local_build` fallback default →
`.workestrate-build/<name>` declaring-layer-relative; phases B–F remain);
STATUS.md gained a new §0 latest-landing section and its §5 item 12 was
annotated.

**2026-08-02 refresh (8):** spec 21 phase B (image-state store) landed —
update note (8) added to Current state (`control/agentctl/src/images/`:
`images.json` schema + atomic IO, per-tag O_EXCL lock with stale-PID
recovery — not flock(2), unsafe-code lint — repo_key, skew matrix;
library-only, phases C–E wire it later; 673/0/3); STATUS.md §0 added for
phase B (phase A demoted to §0.01, commit `02bea9a`), §5 item 12 annotated,
spec 21 §6.1 addendum recorded.

**2026-08-02 refresh (9):** spec 21 phase C (`workload build` + drvPath
change detection) landed — update note (9) added to Current state (new
verb + `images/{detect,build_cmd,pipeline}.rs`, the phase-D seam, the §7
ladders; 700/0/3); STATUS.md §0 rewritten for phase C (B demoted to §0.01,
A to §0.02), §5 item 12 annotated (incl. the personal-v2 policy-gate
discovery), spec 21 §3.4 phase-C addendum recorded.

**2026-08-03 refresh:** docs reconciled to post-D + uncommitted-E reality;
STATUS.md header + §0/§1/§5-item-12 + spec-21 header/§10/§13 updated; origin
rewiring to GitHub BLOCKED (private repo, `ls-remote` auth failure — see
STATUS/origin notes).
