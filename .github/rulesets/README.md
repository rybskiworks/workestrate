# Proposals, not active protection

Every JSON ruleset is deliberately disabled. Committing or importing these files
does not activate enforcement. Reconcile them with current organization and legacy
branch rules before enabling anything; detailed administration reads were unavailable.

Read [the activation plan](../../docs/github-governance.md). Independent review needs
a second eligible CODEOWNER/team; the author cannot be the only possible approver.
The first-stage PR-plus-CI baseline has zero approvals and is not independent review.
No blanket administrator or machine-user bypass is included.

History protection prevents deletion and rewrites, not fast-forward updates. Tag
immutability protects existing tags, not who may create them. Neither workflow
execution policies nor signing credentials are configured by these JSON files.
