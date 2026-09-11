# GitHub governance and staged activation

Status: proposed administrative configuration, not active enforcement. The cleanup
PR changes files only, not live rules, access, secrets, runners, tags or releases.

## Consolidation and observable state

Use `main` as the only integration branch. Dependabot follows the default branch;
managed agent PRs target main. Preserve migration/upstream refs as history, not
alternate CI/release channels. Keep reusable workflows in `rybskiworks/.github`,
SHA-pinned, and package-specific selectors/gates beside the code they test.

On 2026-09-11 readable Workestrate and nix-tooling ruleset enumeration, including
inherited rules, returned empty arrays. Workestrate's branch listing reported
main and migration unprotected. The administration-only legacy protection read
returned HTTP 403, so the detailed legacy settings were not verified. Re-read live
settings before activation; committed JSON or CODEOWNERS is not proof of enforcement.

## Proposed rulesets

[JSON proposals](../.github/rulesets) all have `enforcement: disabled` and no
bypass actors. Importing disabled proposals does not turn protection on.

| Proposal | Effect after activation | Prerequisite |
| :--- | :--- | :--- |
| main-integrity | PRs, resolved threads, no deletion/force push, strict `CI / required` from GitHub Actions. | Observe the exact successful context and reconcile live rules. |
| main-independent-review | One approving code-owner review, stale dismissal and approval of the last push by another person. | Add a second eligible CODEOWNER or visible team with write access. |
| retained-source-history | Prevent deletion/rewrites of upstream refs and the retained migration ref. | Confirm the intended retained refs. |
| immutable-release-tags | Prevent updates/deletion of existing `v*` tags. | Establish release identity and recovery procedures before publishing. |

The first stage deliberately has zero approvals so it does not deadlock a
one-maintainer project. It is NOT independent review. Do not enable the second
rule while the author is the only eligible CODEOWNER. A bare organization handle
is not a code owner. Add an actual second owner/team, not just a requested reviewer.

The proposed check source is GitHub Actions App ID 15368, observed in check-run
metadata. That authenticates the App, not the workflow implementation: writable
workflow code can emit the same context. Protect CI changes with independent
review or an organization-required trusted workflow where available.

History protection allows fast-forward updates; it does not freeze a completed
migration. No branch is deleted here. The separate `upstream/*` and
`upstream/**/*` patterns account for slash matching. Tag immutability does not
authorize tag creation; restrict creation to a reviewed release identity separately.

## Activation order

1. Review and merge the cleanup after selected CI passes. Verify the exact
   aggregate context on fresh PRs and merge-group events; do not enable a queue yet.
2. Export current repository, inherited and legacy rules. Reconcile ref patterns,
   contexts, merge methods and bypass actors. Import proposals disabled first.
3. Protect source-history refs before automatic merged-branch deletion. Activate
   main integrity only after testing a normal PR, failing check, direct push and
   protected-ref deletion attempt with disposable/non-production controls.
4. Establish a second eligible CODEOWNER/team and test self-approval rejection,
   then activate independent review. Keep recovery outside agent credentials.
5. Restrict agent/release identities and workflow execution; collect negative-test
   evidence before enabling automated promotion, publishing or a merge queue.

Prefer squash merge with a conventional PR title as the commit title. Change merge
methods only after checking outstanding workflows. Keep auto-merge off until review
and gate enforcement are proven. Enable automatic branch deletion only after
retained refs are protected. Require signing only after every contributor/bot has
a tested signing path; these API-created cleanup commits are unsigned.

## Agent identity is not just a branch prefix

`agents/*` does not authenticate its writer. Give agents separate scoped identities,
no main/release bypass, and test rejection of updates to main, arbitrary non-agent
refs, upstream history and release tags. Keep review, signing and merge authority
outside the proposing guest.

Same-repository push workflows are a separate risk from fork PRs: a writer able
to change a workflow can request permissions and run code before review. Read-only
workflow defaults are not an immutable ceiling against a modified workflow. Do
not put broad App keys, signing credentials or operator secrets in that context.

Evaluate GitHub's **workflow execution protections** (public preview as of this
audit): central actor/event rules can separate contribution rights from workflow
execution. Start in evaluate mode, allow only intended maintainers/Apps/events,
and explicitly test the agent and Dependabot paths before enforcement. This is
an administrative policy, not another check in agent-editable YAML. Availability
and preview behavior must be verified for this organization.

A fork-based proposal flow is another option. Use protected environments and narrow
trusted identities for privileged jobs. Do not assume public repositories support
the same push/file-path rules as private/internal repositories; verify plan and
repository eligibility before relying on such a restriction.

## Actions, caches and runtime workers

Set default workflow tokens read-only and grant only necessary per-job permissions.
PR-write belongs in metadata automation, not jobs executing PR code. Keep full SHA
pins, compatible Dependabot updates grouped and majors reviewed separately. Avoid
casual `secrets: inherit` and privileged `pull_request_target` checkout of PR code.
Configure allowed Actions, runner groups, secret scanning/push protection, private
reporting and suitable dependency alerts, then verify actual behavior. None is
claimed enabled by this PR. A blocking advisory gate needs reviewed, expiring
exceptions instead of an unconditional ignore policy.

Current GitHub documentation says GITHUB_TOKEN-created opened/synchronize/reopened
PR events create approval-required runs; other PR activity types do not. Approve
the intended runs deliberately. Do not use a broad PAT or multi-repository App key
merely to remove approval friction.

Use disposable hosted workers for untrusted PR code. Native/KVM jobs need a
separate ephemeral trust boundary and an explicit trusted trigger, without host
homes, SSH agents, signing keys or release/cache-write credentials inherited by
the guest. Record runtime pin, host kernel and exact test tier. A build and a
nested-virtualization request do not establish isolation enforcement.

Separate trusted cache writers from PR readers and retain signature verification.
A signed artifact can still be malicious if untrusted code can sign it. Keep log
retention bounded and prohibit routine uploads of homes, decrypted configuration
or guest disks. Cache deployment/GC is not part of this source cleanup.

## Release foundation

Workestrate versions come from Cargo; nix-tooling versions come from version.txt.
Neither source metadata nor generated release-note categories prove a release
exists. Define versioned public exports and a downstream compatibility matrix.
A publisher must validate source/tag match and exact-source evidence, use narrow
separate authority, and attach checksums, SBOM/provenance and input revisions to a
complete draft before publication. Keep publishing credentials out of PR CI.

Update the supplier, test it, then advance consumer pins in separate reviewed
lockfile PRs. Never promote an unmerged supplier PR or fabricate integrity hashes.

## References

- [Repository rules API](https://docs.github.com/en/rest/repos/rules)
- [Available rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)
- [CODEOWNERS requirements](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)
- [Workflow execution protections](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/actions-policies/workflow-execution-protections)
- [GITHUB_TOKEN behavior](https://docs.github.com/en/actions/concepts/security/github_token)
- [Secure Actions use](https://docs.github.com/en/actions/reference/security/secure-use)
