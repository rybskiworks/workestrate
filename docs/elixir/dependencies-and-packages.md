# Dependencies and Package Hygiene

## Purpose

Provide repo-independent guidance for adding, updating, auditing, reviewing, and removing Elixir dependencies. Future agents should treat `mix.exs`, `mix.lock`, and the `mix deps.*` / `mix hex.*` tasks consistently, reproducibly, and aligned with the official Mix and Hex documentation.

## Sources used

- https://hexdocs.pm/mix/Mix.Tasks.Deps.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Tree.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Audit.html (PRIMARY)
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Outdated.html (PRIMARY)
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Policy.html
- https://hexdocs.pm/hex/Hex.Policy.html
- https://hex.pm (Hex.pm homepage, immutability policy)
- https://osv.dev/list?ecosystem=Hex
- https://github.com/mirego/mix_audit
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.ex
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.get.ex
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.update.ex
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/deps.unlock.ex

This page reflects Mix v1.20.2 / Hex v2.4.2 docs.

## Dependency Auditing

### Official retirement audit: `mix hex.audit`

From [Mix.Tasks.Hex.Audit.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Audit.html):

> "Shows all Hex dependencies that have been marked as retired."

`mix hex.audit` reads `mix.lock`, prefetches the Hex registry, and reports any locked Hex package versions whose maintainer has marked them retired.

```bash
$ mix hex.audit
Dependency  Version  Retirement reason
plug        1.14.0   security

Found retired packages
```

On a clean tree it prints:

```text
No retired packages found
```

Behavior to keep in mind:

- It operates on **locked versions only**, not on `mix.exs` requirements. A retired transitive dep in the lockfile fails the audit even if the top-level requirement would now resolve to a non-retired release.
- It audits only Hex packages. Git and path dependencies are silently ignored.
- It detects **retirement**, not CVEs or OSV advisories. Retirement is a maintainer-applied label; it is not the same thing as a vulnerability report.
- The task must be invoked before any task that starts the application, or `:hex` must be listed in `:extra_applications`.

### Important: `mix deps.audit` is NOT a real command

A number of older prompts and cheat sheets refer to a task named `mix deps.audit` and link to `https://hexdocs.pm/mix/Mix.Tasks.Deps.Audit.html`. **That task does not exist.** The linked Hexdocs page returns HTTP 404, and there is no `deps.audit.ex` in the Mix or Hex source repositories. Do not instruct users to run `mix deps.audit`, and do not write tooling that expects it.

For the same audit goals, use the real tools documented in the rest of this section:

- `mix hex.audit` for retired Hex releases.
- `mix_audit` (community) for OSV-based vulnerability checks against `mix.lock`.
- Dependabot or Renovate for automated lockfile monitoring (both understand `mix.lock`).
- OSV.dev (`ecosystem=Hex`) for public Hex security advisories.
- The `Advisories` tab on a hex.pm package page for package-specific notices.
- Hex dependency policies for policy-gated resolution.

### Vulnerability and transitive auditing

`mix hex.audit` covers retirement only. For CVE / security-advisory coverage, use one of these actual tools:

| Tool | What it detects | Modifies lock? | Official? |
|---|---|---|---|
| `mix hex.audit` | Retired Hex versions in `mix.lock` | No | Yes (Hex) |
| `mix_audit` | OSV advisories against `mix.lock` | No | No (community) |
| Dependabot | Known advisories for locked Hex deps | Opens PRs only | External service |
| Renovate | Known advisories + outdated deps | Opens PRs only | External service |
| OSV.dev (`ecosystem=Hex`) | Public Hex security advisories | No | Neutral database |
| hex.pm `Advisories` tab | Advisories for a single package | No | Yes (Hex) |
| Hex dependency policies | Policy-gated resolution (retired/CVE severity/cooldown) | Blocks resolution | Yes (Hex org feature) |

`mix_audit` (https://github.com/mirego/mix_audit) is the most common Elixir-specific community tool. It checks `mix.lock` against OSV and reports known vulnerabilities. It does not alter the lockfile; it only reports.

### Stale-lock detection: `mix deps.unlock --check-unused`

A lockfile entry that is no longer reachable from `mix.exs` is a hygiene risk: it hides dead code, slows resolution, and can confuse audits. Run:

```bash
$ mix deps.unlock --check-unused
```

From [Mix.Tasks.Deps.Unlock.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html):

> "Checks that the lockfile does not have unused dependencies. If it does, an error is raised."

This is a strong CI gate. If you need to remove the unused entries, run `mix deps.unlock --unused`.

## Updating Dependencies

### `mix deps.update`

From [Mix.Tasks.Deps.Update.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html):

> "Updates the given dependencies."

Syntax and behavior:

- `mix deps.update` without arguments **refuses** to run (destructive action gate).
- `mix deps.update dep1 dep2 ...` updates the named deps **and their transitive children** to the latest versions allowed by `mix.exs` requirements.
- `mix deps.update --all` updates the entire graph.

Flags in Mix v1.20.2:

| Flag | Effect |
|---|---|
| `--all` | Update every dependency |
| `--only ENV` | Limit to deps for environment `ENV` |
| `--target TARGET` | Limit to deps for target `TARGET` |
| `--no-archives-check` | Skip archive checks |

`mix deps.update` is **constraint-respecting**. A `~> 1.2` requirement will not jump to `2.0`. If an update does not move a dep, the cause is almost always another requirement pinning it.

There is **NO** `--unlock` or `--no-unlock` flag on `mix deps.update` as of Mix v1.20.2. The only flags are `--all`, `--only`, `--target`, and `--no-archives-check`.

### `mix deps.get`

From [Mix.Tasks.Deps.Get.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html):

> "Gets all dependencies."

`mix deps.get` fetches missing or out-of-date dependencies and updates the lockfile only when something is missing. It does **not** re-resolve the graph for a single dep unless you unlock that dep first.

Key flag for CI:

| Flag | Effect |
|---|---|
| `--check-locked` | Fail if a lockfile update is pending (prevents accidental regeneration) |

### `mix deps.unlock` + `mix deps.get` for a narrow update

To update a single dependency **without** dragging its transitive children along, unlock only that dep and refetch:

```bash
$ mix deps.unlock plug && mix deps.get
```

This tells the resolver to reconsider `plug` while leaving the rest of the lockfile untouched.

### `mix hex.outdated`

From [Mix.Tasks.Hex.Outdated.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Outdated.html):

> "Shows all Hex dependencies that have newer versions in the registry."

`mix hex.outdated` is read-only; it does not modify `mix.lock`. Run without arguments to see top-level deps, or pass an app name to see every requirement on that app across the tree (useful for diagnosing "why won't this update?").

Output columns:

```text
Dependency  Only  Current  Latest  Status
```

Status values:

| Status | Meaning |
|---|---|
| `Up-to-date` | Current version is the latest |
| `Update possible` | Newer version matches existing requirements |
| `Update not possible` | Newer version is outside requirements (e.g. major bump blocked by `~> 1.0`) |

Exit codes:

- Default: exits `1` if any dep is outdated.
- With `--within-requirements`: exits `1` only if at least one update is actually possible given constraints. Good for CI that allows major drift but blocks patch/minor drift.

Other useful flags:

| Flag | Effect |
|---|---|
| `--all` | Also show children of top-level deps |
| `--pre` | Include pre-releases |
| `--sort status` | Sort output by status |
| `--only a,b` | Filter by `only:` environments (comma-separated) |

Caveat: `mix hex.outdated` only inspects the project's current requirements plus the lock. A dep shown as `Update possible` may still not move if a transitive dep re-pins it during resolution.

### Major, minor, and patch strategy

- Patch/minor drift inside a `~>` constraint: update when convenient; these should be safe and can be batched.
- Major version bumps: treat as breaking changes. Read changelogs, run the full test suite, and prefer a dedicated PR.
- Use `mix hex.outdated --within-requirements` in CI if you want to block patch/minor drift while allowing major drift to be handled manually.
- Use `mix deps.update --all` only when you intend a full dependency sweep; it will move transitive deps.

### `~>` operator translation

The pessimistic version operator:

| Requirement | Equivalent range |
|---|---|
| `~> 2.0.0` | `>= 2.0.0 and < 2.1.0` |
| `~> 2.1.2` | `>= 2.1.2 and < 2.2.0` |
| `~> 2.0` | `>= 2.0.0 and < 3.0.0` |
| `~> 2.1` | `>= 2.1.0 and < 3.0.0` |

Rule:

- `~> A.B.C` ⇒ `>= A.B.C and < A.(B+1).0`
- `~> A.B` ⇒ `>= A.B.0 and < (A+1).0.0`

`~>` never matches a pre-release upper bound. In Hex, `:allow_pre` defaults to `false`; pre-releases do not match unless the operand itself is a pre-release.

### When to delete / regenerate `mix.lock`

Regenerating the lockfile (`mix deps.unlock --all && mix deps.get`) is appropriate for:

- An intentional major-version sweep.
- Debugging a resolution conflict that may be a stale-lock artifact.
- Switching branches with incompatible lockfiles.

CI should trust the lockfile as-is and not regenerate per-build unless that is explicit policy. Use `mix deps.get --check-locked` to detect drift.

## Cleaning Dependencies

### `mix deps.clean`

From [Mix.Tasks.Deps.Clean.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html):

> "Deletes the given dependencies' files."

`mix deps.clean` is destructive and requires either named deps or an option. It removes downloaded sources **and** build artifacts by default.

Flags:

| Flag | Effect |
|---|---|
| `dep1 dep2 ...` | Clean specific deps |
| `--all` | Clean all deps |
| `--unlock` | Also unlock the deps in `mix.lock` |
| `--build` | Remove only compiled artifacts, keep source |
| `--unused` | Clean deps no longer in `mix.exs` |
| `--only ENV` | Limit to environment `ENV` |

### `mix deps.unlock`

From [Mix.Tasks.Deps.Unlock.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html):

> "Unlocks the given dependencies."

`mix deps.unlock` is destructive and requires either named deps or an option. It removes entries from `mix.lock` only; it does not delete downloaded sources.

Flags:

| Flag | Effect |
|---|---|
| `dep1 dep2 ...` | Unlock specific deps |
| `--all` | Unlock every entry |
| `--filter NAME` | Unlock deps matching pattern |
| `--unused` | Unlock deps no longer in `mix.exs` |
| `--check-unused` | Exit non-zero if unused lockfile entries exist (CI gate) |

### Clean vs unlock

| Concern | Use |
|---|---|
| Delete fetched source and build artifacts | `mix deps.clean` |
| Remove lockfile entries so resolver can pick new versions | `mix deps.unlock` |
| Both remove lockfile entries and delete sources | `mix deps.clean --unlock dep` |
| CI check for stale lockfile entries | `mix deps.unlock --check-unused` |

## Package Retirement and Security

### Hex immutability and retirement semantics

From Hex.pm policy: in general you cannot remove or change an already-published package. Exceptions exist only in narrow windows:

- A version can be changed or unpublished within 60 minutes of that version's release.
- A version can also be changed or unpublished within 24 hours of the **initial release of the package**.
- Reserved names prevent reclaimed names from being re-registered.

Instead of unpublishing, the recommended action is to retire a package or release. Retirement is **per-version**, not per-package. Retired versions remain resolvable and fetchable by `mix deps.get`, but `mix hex.audit` reports them; `mix hex.outdated` will not auto-suggest them; hex.pm shows a `retired` badge; and dependency policies can block resolution to retired versions.

### Retirement reasons

| Reason | Meaning |
|---|---|
| `renamed` | The package or release has been renamed |
| `deprecated` | The release is deprecated |
| `security` | The release has a security issue |
| `invalid` | The release was published incorrectly |
| `other` | Some other reason |

### `mix hex.retire`

```bash
$ mix hex.retire PACKAGE VERSION REASON --message "..."
```

The `--message "..."` flag is **required** (up to 140 characters). Omitting it aborts with `Missing required flag --message`. The reason is a positional argument, not a flag; there is **NO** `--retire-reason` option.

Other flags:

| Flag | Effect |
|---|---|
| `--organization ORG` | Target an organization package |
| `--unretire` | Reverse a retirement |

### `mix hex.publish`

`mix hex.publish` packages the project per `:package` metadata in `mix.exs` and pushes it to hex.pm. Size limits are 8 MB compressed / 64 MB uncompressed.

Relevant flags:

| Flag | Effect |
|---|---|
| `--revert VERSION` | Revert a version within the allowed window |
| `--replace` | Replace an existing tarball |
| `--yes` | Skip confirmation prompts |
| `--dry-run` | Build tarball without publishing |

Revert windows:

- New package: 24 hours.
- New version of an existing package: 1 hour.
- Private packages: modifiable at any time.

By default `mix hex.publish` publishes both the package and its docs. To publish only the package and skip docs, use `mix hex.publish package`. To revert only previously published docs, use `mix hex.publish docs --revert VERSION`.

### Security advisories

Hex security advisories are published to OSV under ecosystem `Hex` (https://osv.dev/list?ecosystem=Hex). They are maintained by the Erlang Ecosystem Foundation (EEF) and package maintainers, with IDs such as `EEF-CVE-*` and `GHSA-*`. Security contact: security@hex.pm. Maintainers may also publish a GitHub Security Advisory.

`mix hex.audit` does **not** read OSV. Use `mix_audit`, Dependabot, Renovate, or OSV.dev directly for CVE-style checks.

### Hex dependency policies

Hex dependency policies are an organization feature: a signed payload published by an org admin that can block releases with security advisories above a chosen severity and/or releases retired for selected reasons.

Configuration options:

- In `mix.exs`: `hex: [policy: [org: "myorg", name: "strict-prod"]]`
- Environment variable: `HEX_POLICY`
- CLI: `mix hex.config policy hexpm:myorg/name`

Inspect policies with:

```bash
$ mix hex.policy
$ mix hex.policy show
$ mix hex.policy why PACKAGE
```

### Hex dependency cooldown

Cooldown sets a minimum age a release must reach before it is eligible for resolution. This mitigates supply-chain attacks on fresh releases.

Configuration options:

- In `mix.exs`: `hex: [cooldown: "7d"]`
- Environment variable: `HEX_COOLDOWN`
- CLI: `mix hex.config cooldown 7d`

Cooldown and policy compose by strictest-wins.

## Review checklist

- [ ] `mix.lock` is committed to version control and is not regenerated silently in CI.
- [ ] `mix deps.get --check-locked` passes in CI.
- [ ] `mix hex.audit` passes (no retired packages in the lockfile).
- [ ] `mix deps.unlock --check-unused` passes (no stale lockfile entries).
- [ ] Vulnerability scanning is configured via `mix_audit`, Dependabot, Renovate, or equivalent.
- [ ] Version requirements in `mix.exs` express the intended compatibility range (prefer `~>` over open-ended `>=`).
- [ ] Major version bumps are reviewed as breaking changes.
- [ ] Git dependencies lock to a full commit SHA, not a moving branch or tag alone.
- [ ] `only: :test` / `only: :dev` deps are not accidentally treated as prod deps.
- [ ] `optional:` and `runtime: false` deps are used intentionally and documented.

## Implementation checklist

- [ ] Declare new deps in `mix.exs` with the correct shape (`{app, req}`, `{app, opts}`, or `{app, req, opts}`).
- [ ] Choose `~>` constraints that reflect actual compatibility.
- [ ] Run `mix deps.get` and commit the resulting `mix.lock`.
- [ ] For a single-dep update without transitive churn: `mix deps.unlock DEP && mix deps.get`.
- [ ] For a full sweep: `mix deps.update --all`, then run tests.
- [ ] After removing a dep, run `mix deps.unlock --unused` (or `--check-unused` in CI).
- [ ] Add `mix hex.audit` and `mix deps.unlock --check-unused` to CI.
- [ ] Configure Dependabot/Renovate or `mix_audit` for security advisory scanning.

## Validation hooks

- `mix hex.audit` — catches retired Hex packages in `mix.lock`.
- `mix deps.unlock --check-unused` — catches stale lockfile entries.
- `mix deps.get --check-locked` — catches accidental lockfile drift in CI.
- `mix hex.outdated --within-requirements` — catches patch/minor drift that is resolvable without changing `mix.exs`.
- `mix_audit` — community OSV-based vulnerability scan for `mix.lock`.
- `mix deps.tree` — inspects who pulls in what; useful for transitive-dep investigations.
- `mix compile --no-optional-deps --warnings-as-errors` — verifies the project builds when optional deps are excluded.

## Examples

### Audit a repo

```bash
$ mix deps.get --check-locked
$ mix hex.audit
$ mix deps.unlock --check-unused
$ mix deps.tree --only prod
```

### Narrow update of one dep

```bash
$ mix deps.unlock plug && mix deps.get
$ mix test
$ git diff mix.lock
```

### Full dependency sweep

```bash
$ mix deps.update --all
$ mix test
$ mix hex.audit
$ git diff mix.lock
```

### Find why a dep will not update

```bash
$ mix hex.outdated plug
$ mix deps.tree plug
```

### Remove an unused dep

```bash
# 1. Remove the dep from mix.exs
# 2. Unlock unused entries
$ mix deps.unlock --unused
# 3. Clean fetched sources
$ mix deps.clean --unused
# 4. Verify CI gate still passes
$ mix deps.unlock --check-unused
```

### Retire a version as a maintainer

```bash
$ mix hex.retire plug 1.14.0 security --message "CVE-XXXX fixed in 1.14.1"
```

To unretire:

```bash
$ mix hex.retire plug 1.14.0 security --unretire --message "retired in error"
```

### CI gate with `--check-locked`

```bash
$ mix deps.get --check-locked
$ mix compile --warnings-as-errors
$ mix test
$ mix hex.audit
$ mix deps.unlock --check-unused
```

### Sample `mix.exs` deps block

```elixir
defp deps do
  [
    # Hex package with pessimistic version constraint
    {:plug, "~> 1.14"},

    # Hex package restricted to test environment
    {:ex_machina, "~> 2.7", only: :test},

    # Hex package that is optional at runtime
    {:jason, "~> 1.4", optional: true},

    # Git dependency pinned to a full SHA (tags can be force-pushed)
    {:private_dep,
     git: "https://github.com/acme/private_dep.git", ref: "a1b2c3d4e5f6..."},

    # Path dependency for local development
    {:local_lib, path: "../local_lib"},

    # Umbrella sibling
    {:sibling_app, in_umbrella: true},

    # Force this definition to win across the dependency tree
    {:some_lib, "~> 2.0", override: true},

    # Compile-time only; not started as an application
    {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}
  ]
end
```

## Common mistakes

- Forgetting to commit `mix.lock` to version control. The lockfile is the only guarantee of bit-for-bit identical dependencies across dev/CI/prod.
- Using `~> 1.0` and expecting it to allow `2.0`. `~>` is pessimistic; it blocks major version bumps.
- Running `mix deps.update --all` without realizing it moves transitive dependencies.
- Assuming `mix deps.audit` exists. It does not. Use `mix hex.audit`, `mix_audit`, or OSV-based tooling.
- Expecting `mix hex.audit` to catch CVEs. It only catches retirement; use OSV-based tools for CVEs.
- Using `--retire-reason` with `mix hex.retire`. The reason is a positional argument; `--retire-reason` does not exist.
- Pinning git dependencies to a moving branch or tag instead of a full 40-character SHA.
- Running `mix deps.clean --all` when a targeted `mix deps.clean --build dep` or `mix deps.unlock dep && mix deps.get` would suffice.
- Regenerating `mix.lock` in CI instead of using `mix deps.get --check-locked`.
- Treating `mix deps.get` as an update tool. It fetches missing/out-of-date deps but does not re-resolve individual deps without an unlock.

## Strict vs contextual guidance

### Strict (tool-enforced)

- `mix deps.update` without arguments refuses to run.
- `mix deps.clean` and `mix deps.unlock` without arguments/options refuse to run.
- `mix deps.get --check-locked` exits non-zero if the lockfile needs updating.
- `mix deps.unlock --check-unused` exits non-zero if `mix.lock` contains unused entries.
- `mix hex.audit` exits non-zero if retired packages are found.
- `mix hex.retire` aborts if `--message` is omitted.
- `~>` never matches a pre-release upper bound.
- Git `:ref` requires a full 40-character SHA.

### Conventions (not enforced)

- Always commit `mix.lock`.
- Prefer `~>` over open-ended `>=` requirements.
- Use `only:` to keep test/dev deps out of prod resolution.
- Use `optional: true` and `runtime: false` intentionally.
- Lock git deps to a SHA, not a branch or tag alone.
- Run `mix hex.audit` and `mix deps.unlock --check-unused` in CI.

### Contextual tradeoffs

- `mix deps.update --all` vs narrow `mix deps.unlock DEP && mix deps.get`: choose based on whether transitive updates are desired.
- `mix hex.outdated --within-requirements` in CI: useful for blocking patch/minor drift while tolerating major drift, but may produce noise on fast-moving ecosystems.
- `mix_audit` vs Dependabot/Renovate: `mix_audit` is local and fast; Dependabot/Renovate provide PR automation.
- Hex dependency policies + cooldown: valuable for orgs with supply-chain concerns, but add resolution constraints that may block urgent updates.

## Policy decisions for individual repos

- Which lockfile gates run in CI (`--check-locked`, `--check-unused`, `mix hex.audit`)?
- Which vulnerability scanner is adopted (`mix_audit`, Dependabot, Renovate, Snyk, etc.)?
- Whether to run `mix hex.outdated --within-requirements` in CI and whether to auto-file update PRs.
- Whether to allow git/path dependencies, and if so what review and pinning rules apply.
- Whether to enable Hex dependency policies and/or cooldown for the org.
- Whether to require a dedicated PR or full test run for `mix deps.update --all` sweeps.
- Whether `optional:` deps must be exercised with `mix compile --no-optional-deps` in CI.

## Related docs

- `docs/elixir/mix-project-structure.md` — dependency declaration in `mix.exs`, `:deps`, umbrella projects.
- `docs/elixir/static-analysis-credo.md` — credo can flag dependency hygiene issues and unused deps.
- `docs/elixir/documentation-and-publishing.md` — Hex publishing, retirement, and package metadata.
- `docs/elixir/configuration-and-runtime.md` — environment-specific configuration and runtime concerns.
- `docs/elixir/testing-exunit.md` — running tests after dependency updates.
- `docs/elixir/typespecs-and-dialyzer.md` — type-level checks that can surface after dep upgrades.

## Related BEAM guidance

- `../beam/releases.md` — release-packaging parallels: how Mix dependencies are bundled into a BEAM release and the release vs. dependency distinction at the VM level.

## Related skills

- None defined yet.
