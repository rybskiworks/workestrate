# 08 — No repo-local tool home (retire `.workestrate/` inside the checkout)

> **STATUS: READY-TO-EXECUTE (docs/decision); the code step (e) is
> NEEDS-DEVSHELL (no `cc` linker in the authoring container — run in
> `nix develop`, HOST-NIX)**
> **Effort:** S (steps a–d, f, g: shell/git/docs) + code-S (step e: one
> resolution tier removed + tests updated)
> Prerequisites / see-also: [README.md](../README.md) · [00-index.md](00-index.md) ·
> [06-config-home-flag.md](06-config-home-flag.md) ·
> [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md) ·
> [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md) ·
> [../07-execution-order.md](../07-execution-order.md) ·
> [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md) ·
> [../../migration/60-glossary.md](../../migration/60-glossary.md)
>
> **AMENDED by spec 10 (2026-07-29):** steps (a) and (b) are refined by the
> working-copy model — consumed config repos are FIRST-CLASS working copies
> inside the tool home at `$WORKESTRATE_HOME/config-repos/<name>/` (or
> `repos/<name>/` until the spec-10 rename lands), NOT a standalone sibling.
> The standalone canonical clone target
> (`/home/node/Development/workestrate-personal`) is SUPERSEDED: the existing
> bundle clone IS the personal working repo, moved into the new home. The
> registry URL becomes the in-home path OR the remote when added. See
> [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md).

This document records and operationalizes a **user decision**: the workestrate
tool home must **NEVER** live inside the repo checkout. workestrate is a
regular CLI; its home is the user-global `~/.workestrate` (the
[ADR 0023](../../migration/50-decisions/0023-single-tool-home.md) default).
The current repo-local bundle at `<repo>/.workestrate/` — created for
container-$HOME-ephemeral persistence, pinned by `.envrc`, gitignored and
untracked — is **retired entirely**, together with every piece of machinery
that supports repo-local homes (the `.envrc`/`local-xdg.sh` pins, the legacy
migration script, and the trusted-ancestor discovery tier in `paths.rs`).

> Every citation below was verified against the working tree on branch
> `migration/tool-model` during the authoring session (2026-07-29). Commands
> are spelled so a later contextless session can execute top-to-bottom without
> re-derivation.

> **⚠ INTERIM WARNING (read first):** until step (a) of the execution plan
> lands, the **only committed copy of the personal config** (rev `c41a707` on
> `main`) lives inside the ephemeral container bundle at
> `.workestrate/repos/personal`. The bundle is gitignored and untracked —
> it is NOT in git. **Do NOT rebuild the container, do NOT delete the bundle,
> and do NOT run steps (c)–(d) before step (a) is verified.** See §5.

## Environment markers

- `verifiable-here` — steps (a), (c), (d), (f), (g): git surgery, tracked-file
  deletions, doc edits. Runnable in this container.
- `HOST-NIX` — step (b) verification (`workestrate config list` /
  `validate-config`) and step (e) (code change): require the `cc` linker from
  the nix devshell; this container has none (verified: `command -v cc gcc` →
  not found).

---

## 1. Decision + rationale

> **AMENDED by spec 10:** Decision A (config repos as working copies in the
> home) refines this spec's execution model — the home clones are first-class
> working repos (edit/commit/push directly in them; remote is canonical), NOT
> read-only managed clones. See
> [10-config-repos-as-working-copies.md](10-config-repos-as-working-copies.md).

**Decision.** workestrate's tool home is the user-global `~/.workestrate`
(the ADR 0023 default), full stop:

1. **No repo-local homes.** A `.workestrate/` directory inside a repo
   checkout is never a valid tool home. The `<repo>/.workestrate/` bundle is
   retired, and nothing replaces it inside the checkout.
2. **No trusted-ancestor discovery.** The walk-up discovery of
   `.workestrate/config.toml` in ancestor directories
   (`control/agentctl/src/config/paths.rs:116-139`, precedence step 2 of
   `resolve_home_with_kind`) is **removed from the code**, not merely unused.
   With no repo-local homes permitted, discovery can only ever find things
   that must not be homes — the tier is dead weight with live risk.
3. **The tool remains a regular CLI.** It consumes config repos *from
   elsewhere* (registered in the user-global registry by URL/path); it does
   not embed its brain into any checkout it happens to be invoked from.

**Rationale — why repo-local homes are wrong:**

- **Split-brain via discovery.** With a `.workestrate/` inside the repo, the
  discovery tier (`paths.rs:116-139`) silently shadows the user-global home
  whenever the cwd is inside the checkout. The same binary then operates on
  two different registries depending on where it is invoked from — the exact
  ambient-state failure mode a CLI's home selection must not have.
- **Config coupled to a checkout.** The bundle is gitignored
  (`.gitignore:46`) and untracked (verified: `git ls-files .workestrate` →
  empty; `git check-ignore -v .workestrate` → `.gitignore:46:/.workestrate/`).
  Its contents are unrecoverable on a fresh clone — yet it holds the only
  committed copy of the personal config repo (§2). A tool's irreplaceable
  state must not live somewhere `git clean`-adjacent and clone-ephemeral.
- **Every worktree becomes a candidate home.** Any `git worktree` of the repo
  that happens to contain (or acquire) a `.workestrate/config.toml` becomes a
  discovery candidate — an unbounded, invisible set of shadow homes.
- **Model inconsistency.** The architecture is "CLI consumes config repos
  from elsewhere" (ADR 0007 tool-as-tool + dotfiles-registry; ADR 0023
  single home, precedents `~/.kube` / `~/.docker` / `~/.cargo`). A home
  inside the consumed workspace inverts that: the workspace carries the CLI's
  brain.

---

## 2. Verified current state

All items verified in the authoring container on `migration/tool-model`
(2026-07-29):

- **The bundle** at `/home/node/Development/ai-workbench/.workestrate/`:
  - `config.toml` — the registry: `layers = ["personal"]`,
    `[settings] default_context = "personal"`, `home_version = 2`,
    `[configs.personal]` with
    `url = "/home/node/Development/ai-workbench/.workestrate/repos/personal"`,
    `ref = "main"`, and a **stale** `rev = "d2cd0c3..."` (actual clone HEAD
    is newer — see next bullet; this is the known registry-rev staleness).
    Plus `[[trusted_projects]] path = "/home/node/Development/ai-workbench"`.
  - `repos/personal` — the personal config repo, on branch `main` at
    **`c41a707`** (full: `c41a70736cd3f65032f6c4e5351302694251fef0`;
    verified: `git -C .workestrate/repos/personal log --oneline -3` →
    `c41a707 fix(tempest): remove retired install_layout field (schema
    drift)`, `d2cd0c3 clean up test-agent entry`, `074adb8 initial personal
    config`). It has **NO origin remote** (verified: `git remote -v` →
    empty). **This is the ONLY committed copy of the personal config.**
  - `secrets/`, `sources/`, `state/` (workspaces/, var/), and a non-standard
    `scratch/` (ADR 0023's layout,
    `docs/migration/50-decisions/0023-single-tool-home.md:73-82`, specifies
    `cache/` — the bundle has neither `cache/` contents worth keeping nor a
    standard name; see step (b) and the Step 0(b) open decision in
    [../07-execution-order.md](../07-execution-order.md)).
  - Gitignored at `.gitignore:46` (`/.workestrate/`, comment: "Repo-local
    workestrate XDG state (contains age private key — NEVER commit)") and
    **untracked** (verified: `git ls-files .workestrate` → empty).
- **`.envrc`** pins the bundle: `export WORKESTRATE_HOME="$PWD/.workestrate"`
  (final line; the file is otherwise direnv comments about the age key).
- **`scripts/local-xdg.sh:9`** exports the same pin
  (`export WORKESTRATE_HOME="$PWD/.workestrate"`).
- **`scripts/migrate-xdg-to-repo.sh`** — the legacy shell script that created
  the repo bundle. Its own header already declares it **SUPERSEDED (ADR
  0023)** in favor of the native `workestrate migrate-home`. Deleting it
  loses nothing.
- **Discovery code** (`control/agentctl/src/config/paths.rs`):
  - `resolve_home_with_kind()` at `paths.rs:104-150` — precedence: env
    (`:106-110`) → **discovery (`:116-139`)** → legacy XDG (`:141-145`) →
    default `~/.workestrate` (`:147-149`).
  - `HomeKind::Discovered` variant at `paths.rs:17-18` (enum at `:14-22`).
  - `emit_untrusted_discovery_warn()` at `paths.rs:43-52` (called at `:130`).
  - The discovery trust check calls
    `crate::config::is_dir_trusted_via_base_registry(dir)` at `paths.rs:126`;
    that function lives in `control/agentctl/src/config/trust.rs:12`, is
    re-exported at `control/agentctl/src/config/mod.rs:31`, and **has a
    second caller at `trust.rs:327`** (project-config trust) — so the
    function itself must be KEPT; only the discovery call site is removed
    (see step (e)).
  - Discovery-referencing tests: `paths.rs:458, 508, 511, 544, 548`
    (assertions on `HomeKind::Discovered` and the XDG-override interplay).

---

## 3. Execution plan (ordered)

Steps (a) → (d) are strictly ordered; do not reorder. Step (e) is the code
step (HOST-NIX devshell). Steps (f)–(g) are doc amendments that may land with
or after (e).

### (a) PRESERVE the personal config repo FIRST — `verifiable-here`

> **AMENDED by spec 10:** the standalone canonical clone target
> (`/home/node/Development/workestrate-personal`) is SUPERSEDED — NO standalone
> sibling. The existing bundle clone IS the personal working repo, moved into
> the new home as THE working copy (per spec 10 Decision A). The preservation
> intent (the bundle clone must survive before step (d)) is KEPT, but
> re-targeted: preserve by moving/copying the clone into the new home. Until
> the spec-10 `repos/` → `config-repos/` rename lands, the target is
> `~/.workestrate/repos/personal`; after the rename, `~/.workestrate/config-repos/personal`.

The personal config exists **only** in the ephemeral bundle. Move/copy it into
the new home as THE working copy (spec 10 Decision A — the home clone is the
first-class working repo; give it a remote when ready):

```sh
mkdir -p ~/.workestrate/repos    # or config-repos/ after the spec-10 rename
cp -a /home/node/Development/ai-workbench/.workestrate/repos/personal \
    ~/.workestrate/repos/personal
```

**Verify:** `git -C ~/.workestrate/repos/personal log --oneline -2` shows
`c41a707 fix(tempest): remove retired install_layout field (schema drift)` on
`main` as HEAD.

**Gate:** this step MUST be verified before anything else proceeds. It is
the only safeguard against the §5 data-loss scenario.

### (b) Create the real home at `~/.workestrate` — `verifiable-here` + HOST-NIX verify

> **AMENDED by spec 10:** under the working-copy model, the registry URL
> becomes the in-home path OR the remote when added (NOT the standalone
> sibling). Origin on the managed clone = the real remote when ready (NOT the
> standalone sibling).

```sh
cp -a /home/node/Development/ai-workbench/.workestrate ~/.workestrate
rm -rf ~/.workestrate/scratch    # non-standard; see below
```

Under the amended flow, step (a) has already placed the personal working copy
at `~/.workestrate/repos/personal`, so this `cp -a` merges the rest of the
bundle (registry, secrets, sources, state) around it — the clone is the same
content, and `cp -a` is idempotent here.

- **Drop `scratch/`.** ADR 0023's layout
  (`docs/migration/50-decisions/0023-single-tool-home.md:73-82`) specifies
  `cache/`, not `scratch/`. **This resolves the pending scratch/cache open
  decision** (Step 0(b) in [../07-execution-order.md](../07-execution-order.md)
  and [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md)
  §"Bundle fixes needed" item b) in favor of: **neither — drop it.** The
  bundle's `scratch/` carries no required state; the standard layout is kept
  as-is and `cache/` is created lazily by the tool if ever needed.
- **Edit `~/.workestrate/config.toml`:**
  - `[configs.personal].url` → under the working-copy model (spec 10 Decision
    A), the registry URL becomes the in-home path
    (`~/.workestrate/repos/personal` — or `config-repos/personal` after the
    rename) OR the remote when one is added. State this plainly: point at the
    in-home working copy for now; switch to the remote URL once a remote is
    added.
  - `[configs.personal].rev` →
    `c41a70736cd3f65032f6c4e5351302694251fef0` — this also **clears the
    known registry-rev staleness** (`d2cd0c3` → `c41a707`).
- **Set origin on the managed clone:** the copied
  `~/.workestrate/repos/personal` currently has no remote. Under spec 10,
  origin = the real remote when ready (NOT the standalone sibling). Point it
  at the real remote so `workestrate config update` has somewhere to fetch
  from:

  ```sh
  git -C ~/.workestrate/repos/personal remote add origin <your-real-remote>
  ```

- **Verify (HOST-NIX devshell — needs `cc`):** inside `nix develop`, run
  `workestrate config list` (shows `personal` @ `c41a707`, ref `main`,
  clean) and `workestrate validate-config` against the **default-resolved**
  home (no `WORKESTRATE_HOME` export — after step (c) the `.envrc` pin is
  gone, so resolution falls through to the default `~/.workestrate`).

### (c) Remove the repo-local machinery — `verifiable-here`, one commit

Tracked-file deletions + edits, landed as a single commit:

1. `git rm scripts/local-xdg.sh`
2. `git rm scripts/migrate-xdg-to-repo.sh`
3. Edit `.envrc`: remove the
   `export WORKESTRATE_HOME="$PWD/.workestrate"` line. **Read `.envrc`
   first and keep the rest intact** — the remaining lines are the SOPS
   age-key placement comments, which stay.
4. Edit `.gitignore`: remove the `/.workestrate/` entry (line 46) **and its
   comment line** ("# Repo-local workestrate XDG state (contains age private
   key — NEVER commit)", line 45). Keep the file otherwise untouched.

**Verify:** `git status --short` shows exactly the four paths above and
nothing else; `rg -n 'local-xdg|migrate-xdg-to-repo' --type sh --type md -g '!docs/**' .`
returns no live references outside this spec (review any hits before
committing).

### (d) Delete the repo bundle — `verifiable-here`, ONLY after (b) is verified

```sh
rm -rf /home/node/Development/ai-workbench/.workestrate
```

**Precondition (hard):** step (b)'s HOST-NIX verification passed —
`workestrate config list` run with **no** `WORKESTRATE_HOME` export shows
`personal` @ `c41a707` resolving from `~/.workestrate`.

**Verify the repo is clean of bundle references:**

```sh
rg -l '\.workestrate' -g '!docs/**' -g '!*.lock' .
```

Review every hit: expected residuals are none outside historical prose in
`docs/` (excluded) and `Cargo.lock` (excluded); anything in `scripts/`,
`nix/`, `control/`, or dotfiles must be assessed (most should have been
removed in (c); the code references to `.workestrate` as a *directory name
under the default home* are fine — what must be gone is any assumption of a
repo-local bundle).

### (e) CODE: remove the discovery tier — **NEEDS-DEVSHELL** (HOST-NIX)

Inside `nix develop`:

1. In `control/agentctl/src/config/paths.rs`:
   - Remove the discovery block from `resolve_home_with_kind()`
     (`paths.rs:116-139`, including the `xdg_explicit` pre-check at
     `:112-114` that exists only to gate discovery — but KEEP the
     `xdg_var_set` helper, still used by the legacy-XDG step).
   - Remove the `HomeKind::Discovered` variant (`paths.rs:17-18`) and every
     match arm / assertion referencing it (tests at `paths.rs:458, 508,
     511, 544, 548` — rewrite these tests for the new precedence).
   - Remove `emit_untrusted_discovery_warn()` (`paths.rs:43-52`) and the
     `DISCOVERY_WARN` static.
   - Audit `resolve_home_base_with_kind()` (`paths.rs:61-78`) and
     `base_registry_path()` (`paths.rs:84-90`): they exist to serve the
     discovery trust check. If no other caller remains, remove them too.
2. **Audit `is_dir_trusted_via_base_registry` before touching it:** it has a
   second caller at `control/agentctl/src/config/trust.rs:327`
   (project-config trust, ADR 0014) — **KEEP the function**; only the
   discovery call site at `paths.rs:126` goes away. Do not remove the
   re-export at `config/mod.rs:31` while `trust.rs:327` still uses it.
3. Update the module doc-comment on `resolve_home_with_kind()`
   (`paths.rs:92-103`) to the new precedence:
   **flag (`--home`, which sets the env var — see
   [06-config-home-flag.md](06-config-home-flag.md)) > env
   (`WORKESTRATE_HOME`) > legacy XDG > default `~/.workestrate`.**
   The discovery step and its XDG-gating caveat are gone.
4. **Gates (all in `nix develop`):** `cargo fmt --check`,
   `cargo clippy -- -D warnings`, `cargo test`. All must pass; discovery-era
   tests must be rewritten, not deleted without replacement (the XDG-override
   and env-wins invariants still need coverage).

### (f) ADR 0023 amendment + glossary — `verifiable-here`

1. Append an **addendum section** to
   [../../migration/50-decisions/0023-single-tool-home.md](../../migration/50-decisions/0023-single-tool-home.md)
   recording this decision: repo-local homes and the discovery tier are
   **removed**; the "container: `<repo>/.workestrate`" language in the
   Decision section (`0023-single-tool-home.md:70`) and the auto-discovery
   precedence step (`:84-94`) + trust model (`:126-137`) are **superseded**
   by the addendum. Include the §1 rationale (split-brain, checkout coupling,
   worktree candidates, model inversion) in compressed form, and cite this
   spec as the execution record. Per ADR-convention, do not rewrite history
   above the addendum — append.
2. Update [../../migration/60-glossary.md](../../migration/60-glossary.md):
   - The **Home (WORKESTRATE_HOME)** entry (`60-glossary.md:62-68`): drop
     "container: `<repo>/.workestrate`" and the auto-discovery clause from
     the resolution list; new resolution: `--home` flag →
     `WORKESTRATE_HOME` env → legacy XDG → default `~/.workestrate`.
   - The **Bundle (repo-local home)** entry (`60-glossary.md:71-75`):
     mark **RETIRED** with a pointer to this spec and the ADR 0023
     addendum (keep the entry as historical vocabulary so old docs remain
     readable).

### (g) Compose/mount guidance update — `verifiable-here` (with the §4 sweep)

Update the mount/compose guidance that assumed a bundle inside the repo
mount:

- The real home to shadow-mount **read-only** into sandboxes is now
  `~/.workestrate` — and when mounted, it goes at an **explicit non-default
  guest path** (never the default home location inside the guest, so a guest
  `workestrate` never auto-resolves to it).
- The repo mount carries **no bundle at all** — dogfooding simplifies: no
  `.workestrate/` inside `${CWD}`, so the driver/target isolation in
  [03-dogfooding.md](03-dogfooding.md) §6.4 (".workestrate/ absent from
  worktree" invariant) becomes trivially true and the sensitive-path
  exposure listed in [README.md](../README.md) §Track A (`.workestrate/`
  leaked via `${CWD}` binds) drops out of the mount-filtering blast radius.

Apply these edits as part of the §4 doc sweep (or immediately if executing
this spec before the sweep).

---

## 4. Follow-ups (doc sweep)

The following docs reference the repo bundle / repo-local home and must be
updated — either as part of executing this spec or as a listed sweep
immediately after:

- [../01-current-state-and-prereqs.md](../01-current-state-and-prereqs.md) —
  the bundle inventory and "Bundle fixes needed" section (the bundle no
  longer exists; item (b) scratch/cache is RESOLVED by step (b) above).
- [../03-sibling-config-setup.md](../03-sibling-config-setup.md) — topology:
  homes are siblings of the checkout / user-global, never inside it. **Also
  amended by spec 10:** config-repo development happens in the home's
  `config-repos/` working copies (per spec 10 Decision A); see its AMENDED
  markers.
- [../04-baseline-validation.md](../04-baseline-validation.md) and
  [../05-host-validation.md](../05-host-validation.md) — command paths that
  pass `WORKESTRATE_HOME=/home/node/Development/ai-workbench/.workestrate`
  must drop the explicit home (default resolution now finds
  `~/.workestrate`) or be re-pointed.
- [../07-execution-order.md](../07-execution-order.md) — Step 0/1/3 command
  spellings referencing the bundle path.
- [../README.md](../README.md) — "The real bundle is read-mostly" invariant
  and the Track A sensitive-path list (`.workestrate/` exposure).
- [../NEXT-SESSION.md](../NEXT-SESSION.md) — Current State / prompt facts
  referencing the bundle.
- [06-config-home-flag.md](06-config-home-flag.md) — **gains weight:** with
  discovery gone, `--home` becomes THE explicit per-invocation override
  mechanism; the precedence in its §4 simplifies to **flag > env > legacy
  XDG > default** (no discovery tier to be implicitly disabled). Its §2
  multi-home list (real bundle at repo root) and its tests (c) also need
  updating.

---

## 5. INTERIM WARNING (data-loss hazard until step (a))

**Until step (a) lands and is verified, the only committed copy of the
personal config — rev `c41a707` on `main`, including the tempest
`install_layout` fix — exists solely inside the ephemeral container bundle
at `/home/node/Development/ai-workbench/.workestrate/repos/personal`.**
That directory is gitignored (`.gitignore:46`), untracked, and lives on
container storage that does not survive a container rebuild.

Therefore, until `git -C ~/.workestrate/repos/personal log --oneline -2`
shows `c41a707` as HEAD (`repos/` until the spec-10 rename lands — matching
the spelling convention in step (a)):

- **Do NOT rebuild or recreate the authoring container.**
- **Do NOT `rm -rf` or otherwise disturb `.workestrate/`.**
- **Do NOT execute steps (c)–(d).**
- Treat any container-recycling automation as a **data-loss event** for the
  personal config.

After step (a) is verified, the hazard is retired: the personal config
exists as a normal git repo at `~/.workestrate/repos/personal` (or
`config-repos/personal` after the spec-10 rename) on host-persisted storage
— the in-home working copy per spec 10 Decision A. Step (b) then copies the
rest of the home (registry, secrets, sources, state) around it.
