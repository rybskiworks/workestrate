# 10 — Config repos as working copies + dotfiles-style home

> **STATUS: READY-TO-EXECUTE (docs/decision); the code tasks (config-repos/
> rename, dirty-guard regression test, home-init scaffolding) are
> NEEDS-DEVSHELL (no `cc` linker in the authoring container — run in
> `nix develop`, HOST-NIX)**
> **Effort:** S (docs/decision) + M (code: rename + home-init scaffolding)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [08-no-repo-local-home.md](08-no-repo-local-home.md) ·
> [06-config-home-flag.md](06-config-home-flag.md) ·
> [../02-config-requirements.md](../02-config-requirements.md) ·
> [../03-sibling-config-setup.md](../03-sibling-config-setup.md) ·
> [../07-execution-order.md](../07-execution-order.md) ·
> [../../migration/50-decisions/0007-tool-xdg-dotfiles-model.md](../../migration/50-decisions/0007-tool-xdg-dotfiles-model.md) ·
> [../../migration/50-decisions/0008-config-repos-via-uniform-config-add.md](../../migration/50-decisions/0008-config-repos-via-uniform-config-add.md) ·
> [../../migration/50-decisions/0010-source-override-naming.md](../../migration/50-decisions/0010-source-override-naming.md) ·
> [../../migration/50-decisions/0013-layering-ordered-registry-layers.md](../../migration/50-decisions/0013-layering-ordered-registry-layers.md) ·
> [../../migration/50-decisions/0018-secrets-layering-and-per-repo-config.md](../../migration/50-decisions/0018-secrets-layering-and-per-repo-config.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md) ·
> [../../migration/60-glossary.md](../../migration/60-glossary.md)

> Every citation below was verified against the working tree on branch
> `migration/tool-model` during the authoring session (2026-07-29). Commands
> are spelled so a later contextless session can execute top-to-bottom without
> re-derivation.

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts). NOTE: cargo-linked gates are NOT runnable here — no `cc` linker; they run on the host (HOST-NIX devshell) |
| `HOST-NIX` | Requires nix on the user's host (this container has no nix / no `cc` linker) |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

---

## Summary

This spec records two NEW user decisions made after specs 08/09 and one name
decision. **Decision A:** consumed config repos are FIRST-CLASS working copies
inside the tool home at `$WORKESTRATE_HOME/config-repos/<name>/` — the user
edits, commits, pushes, and branches directly in them; the REMOTE is canonical
(gitops), not a separate canonical sibling clone. This supersedes the
standalone-sibling model in spec 08 step (a). **Decision B:** the home itself
becomes a dotfiles-style git repo via an explicit `workestrate home init`
scaffolding command (gitignore + pre-commit hook guarding against mode-160000
gitlinks and secret material) — completing ADR 0007's dotfiles-registry
intent. **Name decision:** rename `repos/` → `config-repos/` (collision with
`sources/`; ADR 0008 term of art). Three code tasks (NEEDS-DEVSHELL): the
rename, a dirty-safe `config update` regression test (guard already present),
and the `home init` scaffolding.

---

## 1. Decision A — config repos as working copies

**Decision.** Consumed config repos live at
`$WORKESTRATE_HOME/config-repos/<name>/` (or `repos/<name>/` until the §3
rename lands) and are FIRST-CLASS working git repos: the user edits, commits,
pushes, and branches directly in them. There is NO separate canonical checkout
alongside the home; the REMOTE is canonical. `workestrate config update <name>`
is how changes made elsewhere (or by another clone of the same remote) arrive
in the home working copy — it fetches and fast-forwards, refusing when the
working copy is dirty.

**Rationale.** One copy, at a standardized expected location, with no sync
dance between a "canonical clone" and a "managed clone." The standalone-sibling
model in spec 08 step (a) (clone `.workestrate/repos/personal` →
`/home/node/Development/workestrate-personal`, then point the registry at the
sibling) is SUPERSEDED: the home clone IS the personal working repo. Note
explicitly: the ADRs never mandated a read-only managed clone. ADR 0008
(`0008-config-repos-via-uniform-config-add.md:20, 28-29`) specifies uniform
cloning of config repos into the store via `workestrate config add <url>
<name>`; it says nothing about the clones being read-only. The "never edit home
clones" gloss was an orchestrator convention, not an ADR invariant — and it
created a needless two-copy sync burden.

**Remote-is-canonical gitops rule.** The remote is the source of truth. Push
early/often; `workestrate config update <name>` fetches from the registered
remote and fast-forwards the working copy. A local-only repo (no remote) works
but is a sole copy — the rule is: give it a remote when it matters. Arbitrary
local-path registry entries remain an escape hatch (see the FS-18 local-path
skip behavior at `config_cmd.rs:522-539` — local-path entries are skipped by
`config update` at `config_cmd.rs:531-539`).

**Dirty-safe `config update` requirement.** `workestrate config update` MUST
be dirty-safe: it must refuse to clobber a working copy with uncommitted
changes. This guard is VERIFIED PRESENT — `cmd_config_update` at
`control/agentctl/src/commands/config_cmd.rs:509-559` bails on dirty clones at
`config_cmd.rs:540-544` ("config repo '{}' has uncommitted changes; commit or
stash first"). `config list` reports dirty status at `config_cmd.rs:594-609`.
The spec task for this guard is regression-test coverage + documentation (§4
Task 2), NOT adding the guard.

---

## 2. Decision B — the home as a dotfiles-style git repo

**Decision.** The tool home (`~/.workestrate`) becomes a version-controlled
dotfiles-style git repo tracking `config.toml` + `overrides.toml` (+ optionally
encrypted secrets), scaffolded by an explicit command. This completes ADR
0007's dotfiles-registry intent (`0007-tool-xdg-dotfiles-model.md:21-22,
31-36`): ADR 0007 intended the registry to live in the user's dotfiles; this
makes the home itself that dotfiles repo.

**Failure mode the scaffolding must prevent.** A naive `git add -A` in the home
would register each `config-repos/*` working copy as a GITLINK (mode 160000
embedded-repo entry — a broken pseudo-submodule pointing at a commit that
isn't recorded as a submodule), and could stage secret material (age private
keys, plaintext `.env`, `*.pem`, `id_rsa*`). The scaffolding below prevents
both.

**Scaffolding spec — `workestrate home init`.** A NEW explicit command (propose
`workestrate home init` or `workestrate init --home`; note `workestrate init`
exists at `control/agentctl/src/commands/init.rs:24` as
`cmd_init(url: Option<&str>)`). NEVER auto-git-init the home — that is invasive
and surprising; the user opts in explicitly. The command must:

1. **`git init` the home** (if not already a git repo).
2. **Write `.gitignore`** with these entries (verbatim):
   ```
   /config-repos/
   /sources/
   /state/
   /cache/
   *.agekey
   age.txt
   *.pem
   id_rsa*
   .env
   ```
   Keep `/secrets/*.enc` committable — age ciphertext is safe to commit (the
   config-repo template at `templates/workestrate-config/.gitignore` already
   commits `.env.enc` per the same reasoning). NOTE: this `.gitignore` is for
   the HOME repo specifically; it is distinct from the config-repo template
   `.gitignore` at `templates/workestrate-config/.gitignore` (which is for
   config repos and ignores `.env` but keeps `.env.enc` committed).
3. **Install a pre-commit hook** REJECTING (exit non-zero with a clear message):
   - any mode-160000 (gitlink) entry staged in the home index;
   - any staged path under `config-repos/`, `sources/`, or `state/`;
   - any staged path matching secret-material patterns: `*.agekey`, `age.txt`,
     `*.pem`, `id_rsa*`, unencrypted `.env`.
4. **Print next-steps:** "Add a remote for the dotfiles repo:
   `git -C ~/.workestrate remote add origin <your-dotfiles-remote>` then push."

**Idempotency.** Re-running `workestrate home init` on an already-initialized
home must be a no-op (or refresh the `.gitignore` + hook to the canonical
content) — never destructive.

---

## 3. Name decision — `config-repos/`

**Decision.** Rename the home subdirectory `repos/` → `config-repos/`.

**Rationale.** The current `repos/` collides conceptually with `sources/`
(agent source-override checkouts, ADR 0010 at
`0010-source-override-naming.md:28`): "repos" vs "sources" is ambiguous in the
same tree. `config-repos/` matches ADR 0008's term of art ("config repo",
`0008-config-repos-via-uniform-config-add.md:20, 28-29`) and the
`[configs.<name>]` registry section in `config.toml`.

**Rejected alternatives:**
- `configs/` — ambiguous vs `config.toml` (the registry file itself).
- `layers/` — names the role (layering), not the content.
- `workloads/` — too narrow (config repos contain more than workloads).

**Fallback note.** `repos/` remains an acceptable fallback if the user prefers
— the rename is one constant at `paths.rs:216-218` (`config_repo_dir()`), so
reverting or keeping the old name is trivial. Nothing in production depends on
the directory name yet (zero data migration).

---

## 4. Required code changes (NEEDS-DEVSHELL / HOST-NIX)

All three tasks run inside `nix develop` (HOST-NIX devshell — this container has
no `cc` linker).

### Task 1 (code-S): rename `repos/` → `config-repos/`

**Sites:**
- `control/agentctl/src/config/paths.rs:216-218` — `config_repo_dir()` returns
  `resolve_store_dir().join("repos").join(name)`; change `"repos"` →
  `"config-repos"`.
- `control/agentctl/src/config/loading.rs:313, 468, 930, 1027` — other `repos`
  join sites.
- Config add/update/remove machinery that references the store directory name.
- Tests referencing `repos/` in expected paths.

**Doc follow-through:**
- ADR 0023 layout (`0023-single-tool-home.md:76` — `repos/` → `config-repos/`).
- Glossary (`60-glossary.md:55-58` — config-repo entry says "Cloned to
  `$WORKESTRATE_HOME/repos/<name>/`"; update to `config-repos/`).
- This tree's docs (spec 08, `03-sibling-config-setup.md`, etc.).

**Gates:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

### Task 2 (test-S): dirty-safe `config update` regression test

The guard is VERIFIED PRESENT (`config_cmd.rs:540-544`). The task is to add a
regression test asserting `config update` bails on a dirty clone (mirroring the
`config remove --delete` dirty-refusal precedent at `config_cmd.rs:52`), and to
document the invariant. Gate: `cargo test`.

### Task 3 (code-M): `workestrate home init` scaffolding

New explicit command (propose `workestrate home init` or `workestrate init
--home`; existing `workestrate init` at `init.rs:24`). Implements the §2
scaffolding spec: `git init`, `.gitignore` generation, pre-commit hook
installation, next-steps printout. NEVER auto-git-init.

**Gates:** `cargo test` (hook content generation, gitignore generation,
idempotency).

---

## 5. Mount model

Dev agents get the home mounted **read-only at the default guest path** PLUS a
nested `config-repos/` read-write shadow mount — the same nested-mount pattern
as `.assets/opencode-agent/docker-compose.yml:31-46` (a tmpfs or ro bind at a
path, then a second scoped bind over the same guest path). This lets a dev
agent read the registry and config layers (ro) while still being able to
commit/push edits to config repos (rw on the shadow).

This is an APPLICATION of the spec 01 mount-shadow machinery
([01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md)), not a new
mechanism: the nested shadow mount is exactly the pattern spec 01 WP4 specifies.

---

## 6. Precedence note — `WORKESTRATE_CONFIG_DIR`

`WORKESTRATE_CONFIG_DIR` stays as the ephemeral read-live dev override — for
foreign configs, tests, and golden fixtures (how golden tests consume
`config.reference` today; `loading.rs:283-287, 432`). It bypasses discovery
(`loading.rs:283-287`), is a single override layer branch (`loading.rs:432`),
and honors `secrets = "none"` via `env_dir_secrets_none()`
(`loading.rs:412-423`). It is NOT a working-copy mechanism. Unchanged by
Decisions A/B.

---

## 7. Amendments applied by this spec

- **Spec 08 step (a) AMENDED** — the standalone canonical clone target
  (`/home/node/Development/workestrate-personal`) is SUPERSEDED: NO standalone
  sibling. The existing bundle clone IS the personal working repo, moved to
  `~/.workestrate/config-repos/personal` (or `~/.workestrate/repos/personal`
  until the §3 rename lands). See [08-no-repo-local-home.md](08-no-repo-local-home.md)
  §1 + step (a) AMENDED markers.
- **`../03-sibling-config-setup.md` topology AMENDED** — config-repo
  development happens in the home's `config-repos/` working copies (per
  Decision A); the mount set for dev agents becomes: home ro at the default
  path + `config-repos/` rw shadow + dev home rw. See its AMENDED markers.

---

## 8. Acceptance criteria

- [ ] Spec 10 written and indexed in
  [00-index.md](00-index.md) + the [../README.md](../README.md) doc map +
  [../07-execution-order.md](../07-execution-order.md).
- [ ] Spec 08 step (a) + `../03-sibling-config-setup.md` topology carry
  `**AMENDED by spec 10**` markers.
- [ ] [00-index.md](00-index.md), [../README.md](../README.md),
  [../07-execution-order.md](../07-execution-order.md), and
  [../NEXT-SESSION.md](../NEXT-SESSION.md) updated.
- [ ] **(code, devshell-gated — NEEDS-DEVSHELL / HOST-NIX)** Task 1 rename
  landed with `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`
  green.
- [ ] **(code, devshell-gated)** Task 2 dirty-guard regression test green.
- [ ] **(code, devshell-gated)** Task 3 `home init` scaffolds gitignore + hook
  and rejects gitlinks/secret paths (test-verified).
- [ ] `repos/` fallback documented (one constant at `paths.rs:216-218`).
