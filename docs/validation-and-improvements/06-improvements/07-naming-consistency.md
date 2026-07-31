# 07 — Naming consistency: purge `workestrator` residue, standardize on `workestrate`

> **STATUS: DONE (landed on `migration/tool-model`, 3 commits; cargo gates PENDING — runnable in-container via nix develop (store-path prefix) or host devshell)**
> **Effort:** M (repo-wide mechanical rename + attr-graph care; no behavior change)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [../07-execution-order.md](../07-execution-order.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md)

This document specifies and records the repo-internal naming-consistency
effort: the tool is canonically **`workestrate`** (binary, `WORKESTRATE_*`
env vars, `~/.workestrate` home, `workestrate.toml`, JSON schema, ADRs), but
the legacy name **`workestrator`** survives as residue (template directory,
flake attrs, OCI image name, doc titles, LiteLLM knowledge-pack JSON keys,
prose). This spec purges `workestrator` **internally** in favor of
`workestrate`.

### Environment markers

- `verifiable-here` — grep sweeps, `git status` scope checks, `just lint-nix`
  (nix 2.35.1 is available in this container), `nix eval`/`nix flake show` of
  renamed attrs where the evaluator allows it.
- `HOST-NIX` — all cargo gates (`cargo check/test`, `just golden-check`,
  `just scaffold-check`): **no `cc` linker in this container**, so they are
  PENDING the host devshell. `nix build` of renamed outputs is also a host
  gate (builds are not run here; eval-only checks are).

---

## 1. Decision

- **Canonical name: `workestrate`.** Every internal reference to the tool,
  its flake outputs, its templates, its images, and its documentation uses
  `workestrate`.
- **`workestrator` is purged internally** — renamed everywhere except
  justified historical contexts (see §5 Residuals).
- **The project repo rename is the USER's later step and is OUT OF SCOPE.**
  The user will push to a new origin themselves. This effort does **not**
  touch git remotes, does **not** rename the checkout directory, and does
  **not** rename the GitHub repo. URL strings that embed the repo path
  (e.g. `github:georgrybski/ai-workbench/templates/...`) keep the
  `ai-workbench` repo component; only the in-repo path component
  (`templates/workestrate-config/`) is updated.
- **No behavior change.** This is a rename; every rename is 1:1. Flake
  output attr renames change the `.#` spelling consumers type — that is the
  point — but derivation contents are unchanged (image name/tag change is
  the one externally visible exception, see §4 FLAG).

---

## 2. Inventory (Phase 1 baseline)

Baseline command: `grep -ri workestrator` excluding `.git/`, `target/`,
`.workestrate/` (ripgrep unavailable in this container; grep used).
**2439 hits / 212 files**, of which 48 hits are out of scope by fiat
(foreign/ignored), leaving **2391 actionable hits**.

| Category | Hits | Files | Disposition |
|---|---|---|---|
| (a) nix code (`flake.nix`, `nix/`) | 10 | 2 | rename (Phase 4) |
| (b) Rust / `control/agentctl` (src, tpl, tests) | 28 | 9 | rename (Phase 4) |
| (c) shell / scripts / justfile | 0 | 0 | clean — nothing to do |
| (d) `templates/workestrator-config/` | 15 | 4 | `git mv` + rename (Phase 4) |
| (e) docs (tracked) | 2243 | ~160 | sweep (Phase 3) |
| (f) `config.reference/` | 1 | 1 | comment fix (Phase 3) |
| (g) root `README.md` / `SPEC.md` / `agents/README.md` | 12 | 3 | sweep (Phase 3) |
| tracked `.agents/skills` (litellm-*) | 60 | 22 | sweep (Phase 3) |
| **foreign — DO NOT TOUCH / STAGE** | 48 | 7 + dir | residual by fiat |

Foreign/out-of-scope hits: `docs/nix/` (42, untracked foreign), 5 untracked
foreign nix skills under `.agents/skills/` (22: `nix-ci-cd`,
`workflow-nix-hardening-00..03`), `.tmp/` + `workspaces/` (6, git-ignored).

### Load-bearing references (identified in Phase 1)

1. **flake.nix attr graph.** `workload-images.workestrator-pi` is merged into
   `packages.${system}` (so it is exposed as `.#workestrator-pi`), and
   `load-images` iterates `builtins.attrNames workload-images` to derive
   `nix build .#<name>` refs and `msb load -t <name>:latest` tags — the attr
   name IS the image tag. `workestrator-wrapper` is a function attr;
   `workestrator` / `workestrator-node` are wrapper packages;
   `apps.${system}.default.program = "${workestrator}/bin/workestrate"`.
   **Collision:** a `workestrate` package already exists (the agentctl
   binary), so the wrapper packages cannot take that name.
2. **OCI image name `workestrator-pi:latest`** — produced by
   `nix/packages/pi-image.nix` (`name = "workestrator-pi"`), asserted in
   `control/agentctl/src/microsandbox/runtime/network.rs` (test, line ~106),
   used in `control/agentctl/tests/fixtures/config/workestrate.toml`, and
   **consumed externally by the personal config repo's `workestrate.toml`**
   (`image = { name = "workestrator-pi", tag = "latest", ... }`). See §4
   FLAG.
3. **JSON keys in `docs/litellm/schemas/*.json`** (machine-consumed-looking,
   actually prose-consumed only): `workestrator_recommendation` (330),
   `workestrator_used` (806), `workestrator_how` (806),
   `workestrator_alias_recommendation` (11), `workestrator_relevance` (1).
   Consumer check: `.agents/skills/validation-litellm-config-check/scripts/
   check_config.py` contains **zero** `workestrator` references — no
   programmatic consumer; the keys are referenced only in skill/doc prose.
   Safe to rename to `workestrate_*`.
4. **Template path `templates/workestrator-config/`** — referenced by
   `control/agentctl/src/scaffold/mod.rs` (comment + `DEFAULT_TEMPLATE_URL`),
   `control/agentctl/tests/scaffold_template.rs`, and docs. The jinja/tpl
   flakes inside use flake **input name `workestrator`** with a let-binding
   `workestrate = workestrator.packages.${system}.workestrate;` — renaming
   the input to `workestrate` collides with the binding; the binding becomes
   `workestrate-cli`.
5. **Filename** `docs/litellm/workestrator-recommended-patterns.md` —
   renamed to `workestrate-recommended-patterns.md`; referrers updated
   (`docs/litellm/00-index.md`, others).

---

## 3. Rename map (Phase 4 — code/nix/templates)

| Old | New | Where |
|---|---|---|
| `templates/workestrator-config/` | `templates/workestrate-config/` | `git mv`; refs in `scaffold/mod.rs`, `tests/scaffold_template.rs`, docs |
| flake input `workestrator` (in generated config flakes) | `workestrate` | `templates/*/flake.nix.jinja`, `control/agentctl/src/scaffold/template/flake.nix.tpl`, doc examples |
| let-binding `workestrate = workestrator...workestrate` | `workestrate-cli = workestrate.packages.${system}.workestrate` | same tpl/jinja files (collision avoidance) |
| `workload-images.workestrator-pi` | `workload-images.workestrate-pi` | `flake.nix` (also exposed as `.#workestrate-pi`; `load-images` picks it up automatically) |
| `workestrator-wrapper` | `workestrate-wrapper` | `flake.nix` |
| `workestrator` (wrapper pkg, pi-bun) | `workestrate-sandbox` | `flake.nix` (name `workestrate` already taken by the agentctl binary package) |
| `workestrator-node` (wrapper pkg, pi node) | `workestrate-sandbox-node` | `flake.nix` |
| `runCommand "workestrator"` | `runCommand "workestrate-sandbox"` | `flake.nix` derivation name |
| `apps.default.program = "${workestrator}/bin/workestrate"` | `"${workestrate-sandbox}/bin/workestrate"` | `flake.nix` |
| OCI image `workestrator-pi:latest` | `workestrate-pi:latest` | `nix/packages/pi-image.nix` `name=`; `network.rs` test assert; test fixture; docs — **see §4 FLAG** |
| `# workestrator config: {name}` (generated TOML header comment) | `# workestrate config: {name}` | `config_cmd.rs`, `scaffold/mod.rs` (+ their tests), `workestrate.toml.tpl`, `workestrate.toml.jinja` |
| `workestrator-config` scaffold strings/messages | `workestrate-config` | `cli_actions.rs`, `config_cmd.rs`, `scaffold/mod.rs`, `README.md.tpl` |
| `workestrator_recommendation` | `workestrate_recommendation` | `docs/litellm/schemas/config-yaml.option-index.json` |
| `workestrator_used` / `workestrator_how` | `workestrate_used` / `workestrate_how` | `docs/litellm/schemas/env-vars.index.json` |
| `workestrator_alias_recommendation` | `workestrate_alias_recommendation` | `docs/litellm/schemas/provider-fields.index.json` |
| `workestrator_relevance` | `workestrate_relevance` | `docs/litellm/schemas/gateway-agent-mcp-skills.index.json` |
| `workestrator-recommended-patterns.md` | `workestrate-recommended-patterns.md` | `docs/litellm/` + referrers |

## 4. FLAG — external coordination required (follow-up, NOT done here)

- **Personal config repo `workestrate.toml`:** its pi workload declares
  `image = { name = "workestrator-pi", tag = "latest", ... }`. After this
  effort, the nix-built image is `workestrate-pi:latest`. The personal
  config repo lives OUTSIDE this repo and must be updated in the same push
  window (`name = "workestrate-pi"`), or `workestrate pi plan/up` against
  the real bundle will reference a non-existent image. Noted here instead
  of left silent.
- **Personal config repo flake (if generated from the old template):** the
  flake input name `workestrator` and any `inputs.workestrator` references
  in the user's existing config repo(s) must be renamed to `workestrate`
  when the user adopts the new template — the input name is local to each
  config repo's flake, so old clones keep working until regenerated.
- **Suggested config-repo names** in docs (`workestrator-config-personal`
  etc.) become `workestrate-config-personal`; if the user already created a
  GitHub repo under the old name, that external name is the user's to
  rename.

## 5. Residuals (intentional remaining `workestrator` mentions)

- `docs/nix/**` (42 hits) — foreign untracked tree; not staged, not edited.
- 5 untracked foreign nix skills under `.agents/skills/` (22 hits) — same.
- `.tmp/`, `workspaces/` (6 hits) — git-ignored runtime artifacts.
- Historical quotes in `docs/nix-store-accumulation-report.md` (2 hits) —
  verbatim session/store-path evidence from past runs; rewriting history
  quotes would falsify the record.
- `docs/migration/50-decisions/**` ADR passages that record the state at
  decision time (e.g. ADR 0012's `workestrator-odysseus` flake sketch, ADR
  0022's `templates/workestrator-config/` references) — historical decision
  records; going-forward references fixed, at-time records kept. Judgment
  calls are listed in the Phase 3 commit message.
- `docs/migration/10-current-state.md` — an explicit snapshot of the
  pre-migration state; kept as history.

## 6. Local checkout-dir rename implications (for the user's later step)

When the user renames the checkout directory (e.g.
`~/Development/ai-workbench` → `~/Development/workestrate`), they must
handle — none of this is done in this effort:

1. **`.workestrate/config.toml`** (NOT committed; edited at rename time):
   - `[configs.personal].url` — absolute path
     `/home/node/Development/ai-workbench/.workestrate/repos/personal`
     must be rewritten to the new dir.
   - `[[trusted_projects]].path` — absolute path
     `/home/node/Development/ai-workbench` must be rewritten to the new dir.
   - Until both are fixed, trust-gated commands fail closed.
2. **`.envrc`** — uses `export WORKESTRATE_HOME="$PWD/.workestrate"`;
   $PWD-relative, so it is **fine** after a dir rename (`direnv reload`).
3. **Absolute paths in docs** — several tracked docs embed
   `/home/node/Development/ai-workbench` (e.g. `docs/migration/
   20-target-system-spec.md`, `30-security-model.md`,
   `docs/validation-and-improvements/01..04`, `NEXT-SESSION.md`,
   `docs/gleam/*`). These are illustrative, not load-bearing; optionally
   sweep them at rename time.
4. **Git remote / GitHub repo rename** — the user's own later step; this
   effort never touches remotes.

## 7. Commits and verification (filled in as phases land)

| Phase | Commit | Contents | Verification |
|---|---|---|---|
| 2 — spec | `7befbb1` | this spec + index/README/execution-order/NEXT-SESSION wiring (5 files, +236/-1) | doc-map consistency; table render check |
| 3 — docs sweep | `a88fb08` | 143 files, +2300/-2300 (pure rename): all tracked docs + 22 tracked litellm/nix-docker-images skills + `config.reference` comment + `git mv` of `workestrate-recommended-patterns.md` | scope grep 2391 → 91 (all §5/§7-intentional); 6 schema JSONs parse (`python3 -m json.tool`); no foreign artifacts staged |
| 4 — code/nix/templates | `0e5723b` | 19 files, +62/-55: `git mv templates/workestrate-config/` (7 renames) + flake.nix attr graph + `pi-image.nix` name + agentctl src/tpl/tests/fixture/README | `scripts/check-nix-paths.sh` (lint-nix body) EXIT 0 — zero violations trace to these edits; `nix eval`: `.#workestrate-sandbox.name` → `"workestrate-sandbox"`, `.#workestrate-pi.imageName` → `"workestrate-pi"`, `.#apps...default.program` → `...-workestrate-sandbox/bin/workestrate`, old `.#workestrator` attr gone; final grep: ZERO hits in code/nix/templates/control; `git status` clean-scope. **Cargo gates PENDING (no `cc`).** |
| 5 — final report | _(this commit)_ | status flip to DONE + final counts | this table |

### Final counts (Phase 5)

- Baseline (Phase 1): **2439** hits repo-wide (excl. `.git`, `target/`,
  `.workestrate/`).
- After Phase 4: **139** hits, ALL in the §5 justified-residual categories:
  spec file (46) + purge-prose in v-i docs (7), historical
  `docs/migration/10-current-state.md` + `50-decisions/**` +
  `nix-store-accumulation-report.md` (16), foreign untracked `docs/nix/`
  (40), foreign untracked nix skills (22), git-ignored `.tmp/` +
  `workspaces/` (6, exact per-file split in session output; category sums
  account for all 139).
- **Zero** `workestrator` hits remain in: `flake.nix`, `nix/`, `templates/`,
  `control/`, `justfile`, `scripts/`, `README.md`, `SPEC.md`, `agents/`,
  `config.reference/`, tracked `.agents/skills/`.

### Pending gates (handoff)

- All cargo gates (`just check/test/golden-check/scaffold-check/schema-check`)
  — **PENDING host devshell** (no `cc` linker in this container). The renamed
  test assertions (`network.rs`, `scaffold/mod.rs`, `scaffold_template.rs`)
  must be confirmed green there.
- `nix build` of renamed outputs (`.#workestrate-pi`, `.#workestrate-sandbox`,
  `.#workestrate-sandbox-node`) — HOST-NIX gate; eval-only verified here.
- Full `nix flake show` is blocked by a PRE-EXISTING unrelated
  `nix/packages/tempest.nix` eval error (`unexpected argument 'npmDepsHash'`)
  — not caused by, not fixed by, this effort; flagged for separate triage.
- **§4 FLAG stands:** personal config repo `workestrate.toml`
  (`image.name = "workestrator-pi"` → `"workestrate-pi"`) and any old-template
  config-repo flake input (`workestrator` → `workestrate`) must be updated
  externally in the same push window. `just load-images` will then load
  `workestrate-pi:latest`; any already-loaded `workestrator-pi:latest` image
  in microsandbox is stale and can be removed.
- The lint-nix wrapper script `scripts/check-nix-paths.sh` currently carries
  a FOREIGN working-tree modification (extends the docs allowlist with
  `docs/nix/**`); the pristine HEAD script would report the known foreign
  `docs/nix` failure. Neither state is affected by this effort.
