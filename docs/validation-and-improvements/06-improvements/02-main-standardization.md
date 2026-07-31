# 02 — Standardize on `main` (rename the personal clone)

> **STATUS: READY-TO-EXECUTE** (rename is host-local git surgery, no KVM/nix
> needed; branch-detection is a DEFERRED option)
> **Effort:** S (one-shot branch rename; branch-detection option deferred)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md)

This document specifies the one-shot branch rename that resolves the
registry/clone mismatch flagged in
[`01-current-state-and-prereqs.md`](../01-current-state-and-prereqs.md) §Bundle
fixes (a): the registry declares `ref = "main"` but the personal clone is on
`master`. The tool is already main-biased in four code sites; the clone is the
only outlier. A contextless reader can execute this from the evidence table
alone.

> Every citation below was verified against the working tree on branch
> `migration/tool-model` during this session.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the host (not needed for the rename). |
| `HOST-KVM` | Requires KVM on the host (not needed for the rename). |

The rename itself is `verifiable-here` — it is plain `git branch -m` on a local
clone with no remote.

---

## 1. Evidence: the tool is already main-biased

Four code sites hardcode or default to `main`:

| # | Site | Behavior | Citation |
|---|---|---|---|
| 1 | `config add --ref` default | `ConfigAction::Add` defaults `r#ref` to `"main"` when `--ref` is omitted. | `control/agentctl/src/cli_actions.rs:105` (`#[arg(long, default_value = "main")]`) |
| 2 | `config update` ref fallback | `cmd_config_update` resolves the git ref via `entry.r#ref.as_deref().unwrap_or("main")` when an entry's `ref` is `None`. | `control/agentctl/src/commands/config_cmd.rs:548` |
| 3 | `config update` list rendering | `cmd_config_list` displays the ref via `entry.r#ref.as_deref().unwrap_or("main")`. | `control/agentctl/src/commands/config_cmd.rs:605` |
| 4 | `migrate-home` HEAD fixture | The migrate-home test simulates a checked-out repo by writing `ref: refs/heads/main\n` to `.git/HEAD`. | `control/agentctl/src/config/migration.rs:686` |
| 5 | scaffolder copier ref | `DEFAULT_COPIER_VCS_REF = "main"` is written to `.copier-answers.yml` so `copier update` targets `main`. | `control/agentctl/src/scaffold/mod.rs:36` |

The tool assumes `main` everywhere. The only place `master` survives is the
live personal clone.

---

## 2. Evidence: the live registry/clone mismatch

The registry at `.workestrate/config.toml` and the clone at
`.workestrate/repos/personal` disagree:

| Artifact | Field | Value | Citation |
|---|---|---|---|
| Registry | `[configs.personal] ref` | `"main"` | `.workestrate/config.toml:9` |
| Registry | `[configs.personal] rev` | `d2cd0c3506b5641507883078d01b626368f9d163` | `.workestrate/config.toml:10` |
| Clone | current branch | `master` | `git -C .workestrate/repos/personal branch --show-current` → `master` |
| Clone | HEAD rev | `d2cd0c3506b5641507883078d01b626368f9d163` | `git -C .workestrate/repos/personal rev-parse HEAD` → `d2cd0c3...` |
| Clone | remotes | *(none)* | `git -C .workestrate/repos/personal remote -v` → empty |

The revs match (the clone IS at the pinned commit), but the branch name does
not. The registry already says `ref = "main"`; the clone just needs its branch
renamed.

---

## 3. Consequence: `config update` fails today

`workestrate config update personal` does **not** silently stay on `master`.
It errors out. The exact path, traced through the code:

1. `cmd_config_update` (`config_cmd.rs:522-561`) iterates the named repos.
2. It classifies the entry via `entry_is_local_path`
   (`config/registry.rs:125-135`): an entry is local-path ONLY when its url is
   not a git remote AND `ref` is `None` AND `rev` is `None`. The personal entry
   has `ref = Some("main")` and `rev = Some("d2cd0c3...")`, so
   `entry_is_local_path` returns **`false`** — the pull is NOT skipped
   (`config_cmd.rs:531-539`).
3. The dirty check passes (clone is clean, `config_cmd.rs:540`).
4. The git ref resolves to `entry.r#ref.unwrap_or("main")` = `"main"`
   (`config_cmd.rs:546-549`).
5. `git_pull(&dest, "main")` (`git.rs:67-77`) runs:
   ```
   git -C .workestrate/repos/personal pull origin main
   ```
6. The clone has **no `origin` remote** (verified: `remote -v` is empty), so
   git fails:
   ```
   fatal: 'origin' does not appear to be a git repository
   fatal: Could not read from remote repository.
   ```
   (verified by running the exact command in this session, exit code 1.)
7. `git_pull` returns `Err`, and `cmd_config_update` bails with
   `git pull failed for .workestrate/repos/personal` (`git.rs:73-75`).

**Exact failure mode:** `workestrate config update personal` errors with
`git pull failed for <dest>` because (a) the clone has no `origin` remote
configured, and (b) even if a remote existed, branch `main` does not exist in
the clone (it is on `master`), so `git pull origin main` would additionally
fail with `couldn't find remote ref main`. The rename below resolves (b); the
absence of a remote (a) is a separate property of this local-path clone — see
§5.

---

## 4. The rename plan (exact commands, in order)

This is host-local git surgery. No KVM, no nix. Run from the repo root
(`/home/node/Development/ai-workbench`).

### 4.1 Pre-checks (confirm the starting state)

```sh
# Confirm the clone is currently on master and at the pinned rev.
git -C .workestrate/repos/personal branch --show-current
# expected: master

git -C .workestrate/repos/personal rev-parse HEAD
# expected: d2cd0c3506b5641507883078d01b626368f9d163  (matches config.toml:10)

# Confirm the clone is clean (no uncommitted changes).
git -C .workestrate/repos/personal status --porcelain
# expected: empty output
```

### 4.2 Rename the branch

```sh
# Rename master -> main in the personal clone. This is a local-only rename;
# it does not move commits, change HEAD's commit, or touch any remote.
git -C .workestrate/repos/personal branch -m master main
```

### 4.3 Post-checks (confirm the rename succeeded)

```sh
# HEAD is now on main.
git -C .workestrate/repos/personal branch --show-current
# expected: main

# The pinned rev is UNCHANGED (branch rename preserves commits).
git -C .workestrate/repos/personal rev-parse HEAD
# expected: d2cd0c3506b5641507883078d01b626368f9d163  (still matches config.toml:10)
```

### 4.4 Registry edit

**None needed.** The registry already declares `ref = "main"`
(`config.toml:9`). The mismatch was purely in the clone's branch name; the
rename brings the clone into alignment with the registry. No
`.workestrate/config.toml` edit is required.

### 4.5 Remote handling

```sh
# Check for any configured remote.
git -C .workestrate/repos/personal remote -v
# verified: empty — this clone has NO remote.
```

The personal clone has no remote (its registry `url` is a local filesystem
path pointing at itself, `config.toml:8`). There is therefore **no remote
rename and no upstream reset** to perform. If a remote is later added (e.g.
the clone is pushed to a git host), the remote branch must also be renamed
via `git push origin --delete master && git push -u origin main` on that
host — but that is a future concern, not part of this rename.

### 4.6 Rollback

If the rename needs to be reverted:

```sh
git -C .workestrate/repos/personal branch -m main master
```

This restores the prior state exactly (the commit is unchanged; only the
branch label moves back).

---

## 5. Verification (after the rename)

```sh
# 1. config list shows clean status with ref=main, rev=d2cd0c3..., [OK].
just workestrate config list
# expected line:
#   personal: <url> (ref main, rev d2cd0c3, clean) [OK]

# 2. config update personal — at the SAME rev, this is expected to either:
#    (a) error with "git pull failed" because there is still no `origin` remote
#        (the rename fixes the branch name, not the missing remote — see §3),
#    or (b) be a no-op if a remote is later configured and is already at d2cd0c3.
#    The rename's success criterion is that the failure is NO LONGER
#    "couldn't find remote ref main" — the branch now exists.
just workestrate config update personal
```

> **Note on the missing remote:** the rename resolves the branch-name
> mismatch (the registry says `main`, the clone now has `main`). It does NOT
> add a remote. `config update` will still fail with `git pull failed` until a
> remote named `origin` is configured, because `git_pull` (`git.rs:67-77`)
> unconditionally runs `git pull origin <branch>`. That is a pre-existing
> property of this local-path clone (registered via a filesystem `url`, not a
> git URL), not a regression introduced or fixed by this rename. The rename's
> contract is: the clone's default branch matches the registry's `ref`.

---

## 6. Other clones / repos affected

| Artifact | Status | Detail |
|---|---|---|
| `.workestrate/sources/` | Empty | `ls .workestrate/sources/` → only `.` and `..`. No agent source checkouts exist; nothing to rename. |
| Workbench repo (`/home/node/Development/ai-workbench`) | Already has `main` | `git -C /home/node/Development/ai-workbench branch -a` shows `main`, `backup/main`, `migration/tool-model` (current), and `remotes/origin/main`. The migration docs reference base `840e8b7` on `main` (`docs/migration/README.md:5`, `docs/migration/10-current-state.md:4`). The workbench repo is already main-based; no rename needed. |

No other managed clones exist in this bundle. The personal clone is the sole
outlier.

---

## 7. Optional robustness (DECISION DEFERRED — document, do not prescribe)

The rename above is a one-shot fix for the current clone. A complementary
robustness improvement would make `config add` detect the clone's actual
default branch instead of blindly writing `"main"` when `--ref` is omitted.

### 7.1 Behavior sketch

When `workestrate config add <url> <name>` is invoked WITHOUT `--ref`, the
tool could probe the remote's default branch before registering the entry:

```sh
git ls-remote --symref <url> HEAD
# parses to: ref: refs/heads/<default>  HEAD
```

The detected `<default>` would replace the hardcoded `"main"` default
(`cli_actions.rs:105`).

### 7.2 Code sites

| Site | Current | Proposed |
|---|---|---|
| `cli_actions.rs:105` | `#[arg(long, default_value = "main")]` on `ConfigAction::Add::r#ref` | Remove the `default_value`; resolve the ref in the `cmd_config_add` handler when `--ref` is absent. |
| `cmd_config_add` (the `Add` dispatch in `config_cmd.rs`) | Uses the clap default directly. | Add a `git ls-remote --symref` probe when `--ref` is `None`; fall back to `"main"` on network failure. |

### 7.3 Trade-offs

| For | Against |
|---|---|
| Eliminates the entire class of registry/clone branch mismatches at registration time. | Adds a network call to `config add` (currently a pure local clone + register). |
| Auto-adapts to upstreams that still default to `master`. | Surprising non-determinism: two `config add` calls against the same URL could record different `ref` values if the upstream renames its default branch between calls. |
| Aligns with the main-standardization goal without forcing it on upstreams the tool does not control. | The main-standardization goal is to standardize ON `main`; auto-detection could register `master` entries, which contradicts the goal. |

### 7.4 Decision

**DEFERRED.** This is an open decision for the user. The one-shot rename in §4
resolves the immediate mismatch; branch-detection is a separate robustness
improvement that trades local determinism for upstream adaptability. It should
not be implemented without explicit sign-off, because it partially conflicts
with the standardize-on-`main` goal (§7.3, "Against" row 3).
