# NEXT-SESSION — handoff prompt

> **STATUS: HANDOFF**
> Prerequisites / see-also: [README.md](README.md) · [00-overview.md](00-overview.md) ·
> [01-current-state-and-prereqs.md](01-current-state-and-prereqs.md) ·
> [07-execution-order.md](07-execution-order.md)

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

## Current state (as of 2026-07-31)

- HEAD: `9ffad0e` (spec 15 repo-wide tombi gates — root `tombi.toml`,
  `scripts/check-toml.sh`, `just tombi-check` in `just verify`,
  `lib.checks.tombiCheck`, home-hook gate; earlier spec-15 wave: `92b6b6f`
  tombi 1.2.5 nix package + devshell, `d97576d` scaffold emission, plus the
  scaffold gap-fix wave in flight). Branch `migration/tool-model`, working
  tree clean.
- Gates: green — 498 tests; `just lint-nix` passes.
- Container home `~/.workestrate` restored after the container-restart wipe,
  but uncommitted in its own git (fine — ephemeral).
- 17 improvement specs total in `06-improvements/`. IMPLEMENTED/DONE: **02**
  (main rename), **06** (`--home` flag), **08** (no repo-local home), **10**
  (config repos as working copies + dotfiles home), **11** (home provisioning
  + lockfile), **12** (per-instance addressing + discovery-lite — refuse-only
  /no-auto-start stance SUPERSEDED 2026-08-01 by the ADR 0026 addendum
  default-on lifecycle; `--use` override retained; W5 config-wiring plan in
  spec §4), **13**
   (secret_env shorthand — commits a349d03, 1e7dc25; 492 tests green, golden
   plans byte-unchanged), **14** (env map form — commits 1035073, 564deca;
   498 tests green, golden plans byte-unchanged), **15** (TOML toolchain:
   tombi — commits 92b6b6f, d97576d, 9ffad0e + scaffold gap-fix wave;
   personal-repo apply 0750876). INTENT-TO-EXPLORE: **16** (cross-home
   dependencies), **17** (visualization + inspection surfaces).
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
  SUPERSEDED; spec **14**'s map form became the v2 binding map.
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
- **Current wave state (2026-08-01):** spec 17 (directory mode) added as
  READY-TO-EXECUTE design. PENDING: implementation of the final secret/env
  model (spec 16), the directory-mode loader (spec 17), the personal config
  restructure to capsules, fork push + interim-patch slim reconciliation,
  old-personal removal, and the version-collapse execution.

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
4. **`just verify`'s litellm-check needs PyYAML** — run inside `nix develop`
   (store-path prefix; bare shell lacks it).
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
    criteria in `05-host-validation.md` **B13**; one line per item:
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
