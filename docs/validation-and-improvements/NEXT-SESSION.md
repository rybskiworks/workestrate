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

- HEAD: `f8aa276` (nix + OKF knowledge packs committed) + `d938f2e` (root
  `.cargo/config.toml` hazard fix so repo-root cargo runs resolve the patched
  crate). Branch `migration/tool-model`, working tree clean.
- Gates: green — 492 tests; `just lint-nix` passes.
- Container home `~/.workestrate` restored after the container-restart wipe,
  but uncommitted in its own git (fine — ephemeral).
- 14 improvement specs total in `06-improvements/`. IMPLEMENTED/DONE: **02**
  (main rename), **06** (`--home` flag), **08** (no repo-local home), **10**
  (config repos as working copies + dotfiles home), **11** (home provisioning
  + lockfile), **12** (per-instance addressing + discovery-lite), **13**
  (secret_env shorthand — commits a349d03, 1e7dc25; 492 tests green, golden
  plans byte-unchanged).
- Key commit refs: `421a54a` (docs env claims); `d7c5a83` (`repos/` →
  `config-repos/` rename); `bd99481` (dirty-guard test); `3894fb7`
  (`workestrate home init`); `bef1c37` (discovery tier removed); `418530a`
  (repo-local home machinery removed); `172d5dd` / `19ff272` / `be356f7`
  (`home init --from` + `workestrate.lock` + lock consumption) — interface SUPERSEDED by the `home init` / `home clone` verb split @ `c406630` (2026-07-31; ADR 0025 addendum); `d991252`
  (`--home` flag); `9107b87` / `de9aa62` / `f9fd2f0` / `c5837e7` (spec 12
  Wave 1); `4adad3f` / `7b65ad1` / `39c1694` (spec 12 Wave 2); `d1c1293`
  (`MSB_AGENTD_PATH` staging); `a349d03` / `1e7dc25` (spec 13: secret_env
  shorthand + spec_examples_parse promotion); `f8aa276`; `d938f2e`.
- The container-home wipe caveat above is updated to current reality: the wipe
  happened 2026-07-30; the home was since RESTORED (3 commands, see caveat);
  the personal config content survives durably at
  `ai-workbench/.tmp/config-repos-export` (verified `c41a707`); the durable fix
  is the host-side home (operator task — open thread ⑦).

---

## Open threads (pending-points register)

1. **Upstream microsandbox PR — ON HOLD.** Fork branch
   `fix/filesystem-agentd-path-override` @ `a4f8a3b8` is ready to push; PR +
   issue docs drafted in `.tmp/msb-upstream/` (ISSUE.md, PR.md, REASONING.md).
   Push + open issue/PR is a pending USER action; issue/PR ON HOLD per user.
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
6. **Host batch B1–B12** (single KVM-host pass per `07-execution-order.md`
   Step 6).
7. **Host-side home setup (operator task):** `workestrate home init` +
   `workestrate config add` from `.tmp/config-repos-export` + trust + compose
   mounts, on the host.
8. **Container-home ephemerality:** the container `$HOME` is ephemeral;
   restore = 3 commands (see caveat note in Current state).
9. **Re-enter `nix develop` after pulling** — the `MSB_AGENTD_PATH` export is
   new (commit `d1c1293`); stale devshells lack it.

---

## Remaining specs + recommended execution order

1. **05 cwd-fallback** (small) — the only bug-fix improvement; small, closes a
   silent config-discovery backdoor; gate `cargo test` runnable in-container
   via `nix develop`.
2. **14 env map form** (small) — spec authored 2026-07-31 (READY-TO-EXECUTE);
   additive serde-only; gates runnable in-container via `nix develop`.
3. **01 mounts WP1–WP3** (in-container; WP4 + Phase 0 spike are KVM and fold
   into the host batch) — highest-value hardening; WP1–3 verifiable
   in-container.
4. **03 dogfooding B1/B2** — structural isolation for self-development; B1/B2
   verifiable-here (B3 has a KVM tail).
5. **07 naming leftover** (trivial) — mechanical residue sweep; banner says
   DONE with cargo gates pending (run via `nix develop`).
6. **09 post-merge cleanup** — upstream-latency-bound; local action resumes
   only after the ON-HOLD PR is pushed/merged/released (then delete
   compensation machinery + bump pin).
7. **04 CLI config authoring** — DEFERRED by design until
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
"Remaining improvements — recommended order (2026-07-31)" section for the six
not-yet-landed specs; Steps 0.5/8a/8c/8d are DONE). Report lane-by-lane
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
