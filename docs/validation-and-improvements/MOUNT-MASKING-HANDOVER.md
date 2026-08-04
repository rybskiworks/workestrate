# MOUNT-MASKING HANDOVER — dynamic mount masking policy (spec 22)

> **STATUS: HANDOVER (2026-08-04, post-rebase + polish; pre-SDK-switch; pre-host-KVM smoke)**
> Cross-references: STATUS.md, NEXT-SESSION.md, BEADS.md, spec 22 (06-improvements/22-dynamic-mount-masking-policy.md), spec 23 (06-improvements/23-microsandbox-fork-nix-flake-packaging.md), ADR 0028 (50-decisions/0028-policy-scopes-collect-and-compile.md).
> Prerequisites: see-also — README.md, 06-improvements/00-index.md.

## Environment markers (same table as other specs in this tree)
| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, cargo-linked gates via `nix develop`). |
| `HOST-NIX` | Requires nix on the user's host for genuine host gates. |
| `HOST-KVM` | Requires KVM on the user's host. |

## TL;DR (≤6 lines)
The dynamic mount-masking feature is implemented on two branches not yet wired together. msb side: feat/passthrough-mount-path-policy @ ccceb48a (32 commits since d9b4d12e) — full PassthroughFs enforcement: policy core, mutation policy + tag store, cascade, write-deny as global write ACL, SDK/runtime threading, docs, tests. workestrate side: experimental @ 0f7ca20 (rebased onto sibling/migration/tool-model @ c45494b; 17+ policy/doc commits since the base) — compiler library, hierarchical scope collection, config wiring, per-mount policy_file lifecycle, sensitive defaults, validation, diagnostics CLI, case_sensitivity parity (msb 6e51840b ported), dead-code shim removed, operator guide + example configs added. The SDK seam (apply_mount_policy) is a no-op pending the fork dep switch (spec 23, deferred). Cross-repo wire contract byte-aligned (MountPolicyProgramWire identical). End-to-end runtime enforcement unexercised (no KVM in container).

## 1. Repo state

### 1.1 Microsandbox feature clone
- Path: /home/node/Development/agent-workbench/microsandbox-mount-policy
- Branch: feat/passthrough-mount-path-policy
- HEAD: ccceb48a
- Base: d9b4d12e (fix(filesystem): honor MSB_AGENTD_PATH prebuilt agentd in build.rs — the ORIGINAL agentd fix; the REWRITTEN agentd fix 4a3133e5 lives on the fork clone forks/microsandbox/repo at 6d7b52ea and is the upstream-PR lineage)
- Author: Georg Rybski
- Signed: ALL 32 COMMITS UNSIGNED (`%G? = N`)
- Working tree: clean
- 32 commits in d9b4d12e..ccceb48a (use this exact commit list — verified by prior capture):
  1. 38ce7918 feat(filesystem): mount path-policy evaluation core
  2. 07226253 feat(filesystem): filter masked names from directory snapshots
  3. 4cdbf3b8 feat(filesystem): mutation policy with tag store and cascade
  4. bbb66952 feat(types,sdk,runtime): thread mount path policy to passthrough
  5. 482f9957 fix(filesystem): use real parent inode in cascade tag check
  6. 581e9733 fix(filesystem): evict descendant tags during masked cascade removal
  7. 7fdc5ed3 fix(filesystem): evict tags on identical-identity rename exchange
  8. e5bf48fa fix(filesystem): enforce writes.deny on mutation and writes
  9. 41217be1 fix(types,sdk): update all VolumeMount::Bind construction sites
  10. 2a2fb100 fix(filesystem): use fd-relative operations in cascade_remove
  11. dc5bd072 docs(filesystem): add SAFETY comments to new unsafe blocks
  12. e9308da0 fix(filesystem): genericize test fixture name (.workestrate → .secret)
  13. 7abf8b5b docs(filesystem): document mount_policy public API items
  14. 7243ff12 docs: mount path policy feature documentation
  15. 380f48dd refactor(filesystem): drop duplicate cfg attribute on write tag
  16. 75dc4b1b refactor(filesystem): document tag store invariants and trade-offs
  17. e695d500 refactor(types): document dropped mount fields in cloud conversion
  18. 86cf5c9b refactor(filesystem): clarify decide_write rule_index semantics
  19. fa88fbcc refactor(filesystem): use is_none check for lexical path in do_open
  20. 68e3d788 refactor(filesystem): drop redundant O_NOFOLLOW in tag_inode reopen
  21. d699b8c8 refactor(filesystem): name rmdir admission probe sentinel
  22. 49cf235c Revert "refactor(filesystem): name rmdir admission probe sentinel"
  23. 8621c889 refactor(filesystem): name rmdir admission probe sentinel (re-applied)
  24. f7ee5756 refactor(runtime): remove diverging binding in load_mount_policy
  25. 1b0e872f refactor(filesystem): extract tag store creation helper
  26. 6e66130d docs(filesystem): add SAFETY comments to file_ops.rs and remove_ops.rs
  27. 6e51840b fix(filesystem): apply case_sensitivity to glob compilation
  28. 8444ac6e docs(filesystem): fix TraversalOnly doc to match readdir behavior
  29. 2094b1dd docs(filesystem): document mount_policy pub items; remove #[allow(missing_docs)]
  30. c7f8e665 fix(runtime): typed error for load_mount_policy
  31. c9507cde fix(filesystem): enforce writes.deny on fallocate, copy_file_range, ftruncate, setattr
  32. ccceb48a refactor(filesystem): minor cleanups from final review

  Squash notes: #21+22+23 are a rename→revert→re-apply cycle (collapse to one); #5,6,7,10,27 are enforcement fixups that could fold into #3/#8 for a clean linear history; docs/SAFETY-comment commits #11,16,26,28,29 are PR-acceptable but squashing reduces noise. User will squash themselves per their decision.

- Diff stat: 33 files, +3483/-22
- Isolation: changes are filesystem/runtime/sdk/types/docs only — all mount-policy-scoped; clean for upstream PR slice.

### 1.2 Workestrate experimental clone
- Path: /home/node/Development/agent-workbench/workestrate-clones/experimental
- Branch: experimental
- HEAD: 0f7ca20 (rebased onto sibling/migration/tool-model @ c45494b; was 56e2684 at the prior handover, then bfe56ae after the architecture-enrichment commit, then rebased + polished: 8029348 MountRoots rebase fix, ba7c3b1 MountPlan test helpers, 587e3af case_sensitivity parity, d42001a dead-code shim removal, 5b98d3c operator guide, 0f7ca20 example configs)
- Base: 43cfcbc (docs(specs): 21 image build/load lifecycle)
- Author: Georg Rybski
- Signed: ALL 14 COMMITS UNSIGNED
- Working tree: clean
- 16 commits in 43cfcbc..56e2684 (14 policy commits listed below + e32a97b `docs(handover): mount-masking feature handover` + 56e2684 `chore(beads): add mount-masking handover issues`; the 14 policy commits are the ones to assess):
  1. 5ca39fc docs(specs): 22 dynamic mount masking policy + ADR 0028; spec 01 → fallback
  2. 7fd7edc feat(policy): pure mount-policy core — Pattern, PolicyValue<T>, compiler, evaluator
  3. 1e4f072 feat(config): collect hierarchical mount masking policy scopes
  4. 8c72bc7 docs(specs): 22 consolidated amendment — write rules, protect, tagging, cascade, symlink, transmission
  5. 21edd0a feat(policy): mount-policy write rules, protect tier, versioned fail-closed program
  6. 50705ae feat(policy): mount-policy runtime transmission prep + diagnostics CLI
  7. bdbf395 test(policy): integration tests for policy mounts CLI + clippy fixes
  8. 60ddf22 style(policy): fmt module ordering
  9. 6fa722c fix(policy): per-mount policy grouping + compile
  10. 35afdad fix(policy): per-mount explain/preview program selection
  11. a36b074 feat(policy): per-mount policy_file + lifecycle
  12. 22856b5 docs(specs): 23 microsandbox fork nix flake packaging (design, deferred)
  13. 7a04406 feat(policy): ship sensitive mount defaults + scaffold discovery + schema regen
  14. 0c18bc2 fix(policy): reject user-supplied policy_file at validation

  Squash notes: 1e4f072 was amended 5× (already collapsed to one). a36b074 amended once. Series is linear and clean; no further squash needed.

- Diff stat: 42 files, +5487/-12

## 2. What was done (the two slices)

### 2.1 Microsandbox slice (feat/passthrough-mount-path-policy, d9b4d12e..ccceb48a, 32 commits)
The msb side implements the full PassthroughFs enforcement layer. Commit range 38ce7918..ccceb48a delivers:
- **Policy core** (38ce7918): mount path-policy evaluation core — pattern matching, rule application, the decide/decide_write entry points that the rest of the filesystem calls into.
- **Directory snapshot masking** (07226253): masked names are filtered from readdir directory snapshots so masked entries are invisible to the guest at listing time.
- **Mutation policy + tag store + cascade** (4cdbf3b8): the inode tag store that records which masked inodes have been written (and thus become visible), plus the cascade machinery for recursive removal of untagged-masked descendants on rmdir.
- **SDK/runtime threading** (bbb66952, 41217be1): VolumeMount::Bind gains the policy field; the runtime threads the compiled program into PassthroughFs at mount setup; all construction sites updated.
- **Enforcement fixups** (482f9957, 581e9733, 7fdc5ed3, 2a2fb100, 6e51840b): real-parent-inode cascade tag check, descendant-tag eviction on masked cascade removal, tag eviction on identical-identity rename exchange, fd-relative ops in cascade_remove, case_sensitivity applied to glob compilation.
- **writes.deny as global write ACL** (e5bf48fa, c9507cde): writes.deny blocks create/write/unlink/rmdir/rename at matching paths regardless of mask state; hoisted above the masked branch (M3/M4); enforced on fallocate, copy_file_range, ftruncate, setattr.
- **Runtime hardening** (f7ee5756, c7f8e665): remove diverging binding in load_mount_policy, typed error for load_mount_policy.
- **Docs/SAFETY** (dc5bd072, 7abf8b5b, 7243ff12, 75dc4b1b, 6e66130d, 8444ac6e, 2094b1dd): SAFETY comments on every new unsafe block, public-API docs, feature documentation, tag-store invariant documentation, TraversalOnly doc fix, missing_docs removal.
- **Refactors** (380f48dd, e695d500, 86cf5c9b, fa88fbcc, 68e3d788, d699b8c8→49cf235c→8621c889, 1b0e872f): duplicate cfg drop, dropped-mount-fields doc, decide_write rule_index semantics, is_none lexical-path check, redundant O_NOFOLLOW drop, rmdir admission probe sentinel naming (rename→revert→re-apply cycle), tag store creation helper extraction.
- **Fixture de-personalization** (e9308da0): test fixture renamed .workestrate → .secret to avoid leaking the project name into upstream test data.
- **Final review cleanups** (ccceb48a): minor cleanups from the final review pass.

### 2.2 Workestrate slice (experimental, 43cfcbc..0c18bc2, 14 commits)
The workestrate side implements the policy compiler library, config collection, and the diagnostics CLI. Commit range 5ca39fc..0c18bc2 delivers:
- **Spec 22 + ADR 0028 authoring** (5ca39fc): the dynamic mount masking policy spec and the collect-and-compile ADR; spec 01 dispositioned to SECONDARY/FALLBACK.
- **Pure policy core** (7fd7edc): Pattern, PolicyValue<T>, the compiler, and the evaluator as a pure library with no runtime dependencies.
- **Hierarchical scope collection** (1e4f072): config surfaces collect ordered provenance-bearing fragments across the six scopes (operator/reference/repo/workload/mount-entry/global) without merging — the compiler owns precedence.
- **Consolidated amendment** (8c72bc7): spec 22 locks write-rules/protect/tagging/cascade/symlink/transmission semantics and the corrected host-side transmission channel.
- **Write rules + protect tier + versioned fail-closed program** (21edd0a): WritePolicy (allow/deny CompiledRuleSet), the protect tier (untouchable, terminal protect operator-only), and the versioned fail-closed MountPolicyProgramWire (version:1, reject None/≠1).
- **Runtime transmission prep + diagnostics CLI** (50705ae): the workestrate→msb transmission scaffolding and the `workestrate policy mounts explain|preview` CLI.
- **Integration tests + clippy** (bdbf395): integration tests for the policy mounts CLI plus clippy fixes.
- **Per-mount grouping + compile** (6fa722c): mount-entry scopes partitioned per mount; each mount gets its own compiled program.
- **Per-mount explain/preview selection** (35afdad): explain/preview select the correct per-mount program.
- **Per-mount policy_file + lifecycle** (a36b074): each mount gets its own policy_file with a runtime lifecycle (write to host state-dir, validate, load).
- **Spec 23 authoring** (22856b5): microsandbox fork nix flake packaging design (DEFERRED).
- **Sensitive mount defaults + scaffold discovery + schema regen** (7a04406): sensitive mount defaults shipped, scaffold discovery, config schema regenerated.
- **Validation: reject user-supplied policy_file** (0c18bc2): validation rejects user-supplied policy_file (runtime-resolved only; spec 22 §12).

## 3. Cross-repo wire contract (MountPolicyProgramWire)

The wire contract is byte-aligned between the two repos. The side-by-side capture:

| Field | Type | Serialization | Deserialization |
|---|---|---|---|
| `version` | `Option<u32>` | `Some(1)` | reject `None` and `≠1` (fail-closed) |
| `rules` | `Vec<PathPolicyRule>` | — | — |
| `protect` | `Vec<PathPolicyRule>` | — | — |
| `writes` | `WritePolicy` = `CompiledRuleSet { allow, deny }` | — | — |
| `case_sensitivity` | `CaseSensitivity` (lowercase) | — | — |
| `deny_unknown_fields` | — | yes on wire | yes (fail-closed on unknown) |

Sub-structs (ALL IDENTICAL across repos):
- `RuleEffect`: `Mask` / `Unmask` (snake_case)
- `ScopeKind`: 6 variants (snake_case)
- `RuleOrigin`
- `PathPolicyRule`
- `WriteRuleEffect`: `Allow` / `Deny` / `Protect`
- `Decision`: `Visible` / `Masked` / `TraversalOnly`
- `WriteDecision`: `Allow` / `Deny`

Drift notes (non-wire, harmless):
- **Explain-trace structs**: workestrate `RuleMatch`/`WriteRuleMatch` have an extra `pattern: String` field; msb omits it. NOT on the wire; explain-trace only. Harmless.
- **CaseSensitivity::Insensitive**: reserved-for-future on the workestrate side; msb 6e51840b applies it to glob compilation. Benign mismatch (workestrate never emits Insensitive yet); document for future alignment.
- **MountPlan.policy_file** (workestrate plan.rs:122): runtime-resolved `Option<PathBuf>`; never user-set; validation rejects it (validation.rs:380). Lives on the plan struct rather than a side channel — flagged wart.

## 4. File maps (condensed)

### 4.1 Microsandbox slice (33 files, +3483/-22)
| Area | Files |
|---|---|
| Policy core | `crates/filesystem/src/mount_policy/mod.rs`, `policy.rs`, `pattern.rs`, `evaluator.rs` |
| Enforcement (PassthroughFs) | `crates/filesystem/src/file_ops.rs`, `remove_ops.rs`, `rename.rs`, `readdir.rs`, `open.rs`, `write.rs`, `attr.rs` |
| Tag store | `crates/filesystem/src/mount_policy/tag_store.rs` |
| SDK/runtime threading | `crates/sdk/src/...`, `crates/runtime/src/...`, `crates/types/src/...` |
| Tests | `crates/filesystem/tests/test_mount_policy.rs`, `test_mutation_policy.rs` |
| Docs | `docs/mount-path-policy.md`, inline rustdoc |

### 4.2 Workestrate slice (42 files, +5487/-12)
| Area | Files |
|---|---|
| Policy core | `control/policy/src/pattern.rs`, `compiler.rs`, `evaluator.rs`, `value.rs`, `lib.rs` |
| Config collection | `control/config/src/...` (scope collection, schema) |
| Wiring | `control/agentctl/src/microsandbox/mounts.rs`, `plan.rs` |
| Validation | `control/agentctl/src/validation.rs` |
| CLI | `control/agentctl/src/cli/policy.rs` |
| Defaults | `control/policy/src/defaults.rs`, scaffold discovery |
| Schema | `control/config/schema/` (regenerated) |
| Tests | `control/policy/tests/`, `control/agentctl/tests/cmd_policy.rs` |
| Specs | `docs/validation-and-improvements/06-improvements/22-...md`, `23-...md` |
| ADR | `docs/migration/50-decisions/0028-...md` |

## 5. Test status

- **msb**: 44 policy tests across `test_mount_policy.rs` (11) + `test_mutation_policy.rs` (33); full filesystem crate 664/664 PASS (`verifiable-here`). MSB runtime/VM tests require KVM (not run here — `HOST-KVM`).
- **workestrate**: mount_policy 56/56, cmd_policy integration 10/10, full agentctl lib 565/566 (1 ignored). Full `just verify` runnable in `nix develop` (`verifiable-here`).
- **spec 22 §19 acceptance checklist**: all items covered by tests — six-scope authority order; PolicyValue visitor; freeze+duplicate+overlap+provenance+terminal-unmask trust; pattern validation+non-UTF-8+case sensitivity+TraversalOnly; write-rule allow/deny/default/overlap/terminal; protect untouchable+terminal-protect-operator-only; tag identity/eviction/bounds/lifecycle; cascade+anti-laundering rename; JSON v1+malformed fail-closed; explain/preview reuse compiler+hardlink warning; per-mount selection; duplicate-guest-path+user-supplied policy_file rejection.
- **Gaps**: symlink contract enforcement is tested indirectly (loader); restart-lifecycle eviction not explicit (forget/destroy are); minor.

## 6. Open threads (T1–T16)

| ID | Item | Severity | Blocker | Owner |
|---|---|---|---|---|
| T1 | Sign MSB mount-masking commits (%G? = N → G) | high | blocks clean upstream PR | user (host) |
| T2 | Rebase MSB mount-masking onto fork 4a3133e5 lineage | high | blocks upstream PR | user (host) |
| T3 | Implement spec 23 — microsandbox fork as nix flake output | high | blocks T4 (SDK switch) | user (host, HOST-NIX) |
| T4 | Flip apply_mount_policy from no-op seam to real per-mount SDK call | critical | blocked by T3 | dev |
| T5 | Run cargo clippy --workspace -- -D warnings + cargo fmt --check on MSB branch | medium | — | dev (verifiable-here) |
| T6 | W5 dead-code sweep in workestrate mount_policy | low | — | dev |
| T7 | W7 author mount-masking operator guide | low | — | dev/docs |
| T8 | W9 add mount-masking config examples | low | — | dev/docs |
| T9 | W10 refresh STATUS.md §0.0 with §9/§12 landings | low | — | dev/docs |
| T10 | Host-KVM runtime smoke for spec 22 §14 enforcement | high | — | user (host, HOST-KVM) |
| T11 | Open upstream MSB PR (force-push fork fix/filesystem-agentd-path-override @ 6d7b52ea) | medium | USER action | user (host) |
| T12 | Cross-repo evaluator duplication/drift — golden decision-vector test or shared crate | major | pre-merge recommended | dev |
| T13 | tag_store poison handling — propagate or document panic-on-poison + fd-leak contract | major | pre-merge recommended | dev |
| T14 | CaseSensitivity::Insensitive reserved vs applied alignment | low | — | dev |
| T15 | MountPlan.policy_file wart (runtime-resolved on plan struct) | low | — | dev |
| T16 | Quota byte accounting still counts masked bytes (masking ≠ space control) | low | — | dev |

## 7. Key decisions (spec 22 LOCKED)

- **Collect-not-merge (ADR 0028)**: fragments collected per layer, compiler owns precedence/freeze/trust/provenance; merge.rs never sees policy; mounts vec wholesale-replace unchanged.
- **Mask = read boundary**: untagged masked → lookup ENOENT/readdir omit; masked create/write-opens allowed; successful masked write tags the inode; tagged = fully visible.
- **writes.deny is GLOBAL WRITE ACL** (user decision): blocks create/write/unlink/rmdir/rename at matching paths regardless of mask state. Default = allow+tag.
- **Protect tier**: independent from overridable; fully untouchable; terminal protect operator-only (same trust as terminal unmask); cascades blocked.
- **Tag identity**: alias-scoped (parent synthetic ino + name bytes) + retained O_PATH fd (pins host inode, prevents inode-number reuse); identity revalidated at lookup.
- **Cascade**: do_rmdir on ENOTEMPTY → cascade only untagged-masked; protected/tagged/visible/host-added/depth-overflow → ENOTEMPTY fail-closed; bounded depth 64; symlinks never followed.
- **Rename anti-laundering**: masked-source rename → ENOENT always; destination per write rules + protect.
- **Host state-dir transmission v1 (CORRECTED)**: workestrate atomically writes policy JSON to host state-dir; SDK mount model carries optional policy path; host mount-spec keyed token (policy=<path>); msb parses; PassthroughFs loads + validates BEFORE VM startup; immutable in-memory for mount lifetime; path confined to approved state dir, no symlinks/traversal, parse-once; version:1, fail-closed on missing/unsupported. The prior guest-file+env channel is INVALIDATED.
- **Symlink contract**: broker never follows interior symlinks (openat2 RESOLVE_BENEATH|NO_SYMLINKS|NO_MAGICLINKS); guest resolves targets in guest namespace; tagging a symlink never unmasks its target.
- **Per-mount compilation**: each mount gets its own compiled program + policy_file (mount-entry scopes partitioned per mount, global+workload shared).
- **Spec 01 → SECONDARY/FALLBACK** (WP1-WP4 frozen); deleted if spec 22 lands.

## 8. Code review findings (rust-code-review verdict: approve-with-comments)

- **(major, pre-merge recommended)** Cross-repo evaluator duplication/drift — add a golden decision-vector test pinned in both repos, or extract a shared crate. (T12)
- **(major, pre-merge recommended)** tag_store poison handling — propagate or document the panic-on-poison + fd-leak contract (8 `.lock().unwrap()` sites; risk: 10k O_PATH fd leak on poison). (T13)
- **(minor, follow-up)** decide_write per-call sort; evict_parent double-lock; cascade errno loss.
- **(minor, follow-up)** CLI walk_tree depth cap; magic-number configurability (10k LRU, 64 depth).
- **(nit, optional)** CLI JSON struct dedup; policy_file tmp-name uniqueness; lazier per-mount compile.

Praise: exemplary unsafe hygiene (every block has SAFETY comment); strong fail-closed loader; M3/M4 writes.deny correctly hoisted above masked branch.

## 9. Risks / drift

- CaseSensitivity::Insensitive reserved vs applied (benign). (T14)
- MountPlan.policy_file wart (runtime-resolved on plan struct). (T15)
- Explain-trace pattern field asymmetry (non-wire).
- Symlink readlink name-level metadata leak (accepted).
- Hardlink name-vs-object masking (documented limitation).
- ~5s negative-dentry caching (kernel property).
- macOS no-op (Linux-first; spec 22 §14bis).
- Quota byte accounting still counts masked bytes (masking is not space control). (T16)

## 10. Environment (capabilities table)
| capability | this container | how | gates runnable |
|---|---|---|---|
| cargo/rustc 1.97.1 | via nix develop | bare shell has no cc | cargo test/clippy/fmt/check (both repos) |
| nix 2.35.1 | not on PATH | /nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin/nix | nix develop, nix flake show |
| just | not in bare shell | provided by nix develop | just verify (inside devshell) |
| bd (beads) | not installed | nix shell nixpkgs#beads (ephemeral) when needed | cannot enumerate issues here without ephemeral binary |
| KVM | absent | — | no sandbox runtime; spec 22 §14 enforcement is HOST-KVM |
| sops/age | absent | — | secret decryption fails closed (host-only) |
| flake.nix | workestrate yes; experimental yes; msb NO | — | msb built via cargo/just, not nix |

## 11. Critical warnings (contextless session must know)

1. **HEAD drift**: msb @ ccceb48a (NOT e5bf48fa which is commit #8 mid-series); workestrate @ 56e2684 (NOT 0c18bc2 which was the original-handover HEAD, and NOT 22856b5 which is policy commit #12). Use `d9b4d12e..HEAD` (msb, 32 commits) and `43cfcbc..HEAD` (workestrate, 16 commits — 14 policy + 2 doc/beads) as the true ranges. The 14 policy commits end at 0c18bc2; e32a97b and 56e2684 are doc/beads only.
2. **agentd-fix lineage**: d9b4d12e (msb branch base, ORIGINAL) and 4a3133e5 (fork fix/filesystem-agentd-path-override parent, REWRITTEN) are DIFFERENT agentd fixes. 4a3133e5 is NOT reachable in the msb-mount-policy repo. When preparing the upstream PR, rebase onto the 4a3133e5 lineage (fork clone forks/microsandbox/repo), never d9b4d12e.
3. **The SDK seam is a no-op**: `apply_mount_policy` (control/agentctl/src/microsandbox/mounts.rs:67-72) writes the policy JSON to host state-dir but does NOT pass it to the SDK. The whole workestrate→msb transmission is dormant until spec 23 lands and the seam is flipped. Do not assume runtime enforcement works end-to-end yet.
4. **Invalid guest-file+env channel**: spec 22 §12 explicitly forbids transmitting the policy as a guest file or guest env var. The only valid v1 channel is host state-dir → SDK mount-spec → msb PassthroughFs pre-start.
5. **.workestrate fixture leakage ALREADY FIXED**: msb e9308da0 renamed to .secret. Do not re-introduce.
6. **Per-mount compilation requirement**: each mount gets its own compiled program + policy_file. There is no single program per workload.
7. **MountPlan.policy_file wart**: runtime-resolved `Option<PathBuf>` on the plan struct; never user-set; validation rejects it; lives on the plan rather than a side channel.
8. **All commits unsigned on both branches** (`%G? = N`). MSB AGENTS.md mandates `git commit -S`.
9. **No bd on PATH, no issues.jsonl**: beads initialized but empty in this container; creating the issues below requires installing bd or running on the host.
10. **Container home is ephemeral**: `~/.workestrate` can be wiped on container restart. The real home must be set up host-side.

## 12. Recommended next actions (ordered)

1. Sign MSB mount-masking commits (host, GPG key). (T1)
2. Rebase MSB mount-masking onto fork 4a3133e5 lineage (NOT d9b4d12e). (T2)
3. Spec 23 impl: package microsandbox fork as nix flake output (encapsulated source-build; one pinned input for binary+SDK). Unblocks T2/T4. (T3)
4. Flip apply_mount_policy from no-op to real per-mount SDK call: `builder.volume(&m.guest, |v| v.bind(host).mount_policy(m.policy_file))`. (T4)
5. Cargo dep switch: workestrate microsandbox =0.5.6 → fork.
6. Nix packaging: build feature msb from source via spec 23 flake.
7. Host-KVM runtime smoke for spec 22 §14 enforcement. (T10)
8. Open upstream MSB PR (force-push fork fix/filesystem-agentd-path-override @ 6d7b52ea). (T11)
9. Workestrate polish tail (W5 dead code, W7 guide, W9 examples, W10 STATUS refresh). (T6–T9)

## 13. References

- Spec 22: `docs/validation-and-improvements/06-improvements/22-dynamic-mount-masking-policy.md` (426 lines)
- Spec 23: `docs/validation-and-improvements/06-improvements/23-microsandbox-fork-nix-flake-packaging.md` (260 lines)
- ADR 0028: `docs/migration/50-decisions/0028-policy-scopes-collect-and-compile.md` (123 lines)
- STATUS.md: `docs/validation-and-improvements/STATUS.md`
- NEXT-SESSION.md: `docs/validation-and-improvements/NEXT-SESSION.md`
- BEADS.md: `./BEADS.md` (beads procedures)
- msb feature clone: `/home/node/Development/agent-workbench/microsandbox-mount-policy` (branch feat/passthrough-mount-path-policy @ ccceb48a)
- msb original fork clone: `/home/node/Development/agent-workbench/forks/microsandbox/repo` (branch fix/filesystem-agentd-path-override @ 6d7b52ea; contains the REWRITTEN agentd fix 4a3133e5)
- workestrate experimental: `/home/node/Development/agent-workbench/workestrate-clones/experimental` (branch experimental @ 0c18bc2)
- msb upstream PR docs (status: PREPARED, READY TO OPEN): `/home/node/Development/agent-workbench/forks/microsandbox/pr-issue-docs/PR.md` + `ISSUE.md`

## 14. Maintenance

Update this doc whenever: a phase lands on either branch; the SDK seam flips; spec 23 implements; host-KVM smoke runs; upstream PR opens/merges; STATUS.md §0.0 changes materially. The handover must never go stale relative to the two branches' HEADs and the open-threads table.

## 15. Technical architecture — microsandbox enforcement (msb)

This section describes what the msb enforcement code actually does, with
file:line evidence under `crates/filesystem/lib/backends/passthroughfs/unix/`
(unless another path is given). All policy enforcement is `#[cfg(target_os =
"linux")]`-gated; macOS is a no-op for masking (spec 22 §14bis). The policy
module lives at `mount_policy/` and the enforcement hooks are spread across the
existing PassthroughFs op files.

### 15.1 Policy types & wire schema

The compiled program is `MountPolicyProgram` (`mount_policy/program.rs:122-134`):
a pure, in-memory struct with fields `version: u32`, `rules: Vec<PathPolicyRule>`
(ordered mask/unmask visibility rules), `protect: Vec<PathPolicyRule>`,
`writes: WritePolicy` (= `CompiledRuleSet`), and `case_sensitivity:
CaseSensitivity`. It is constructed once per mount and held immutable for the
mount's lifetime.

The wire type is the private `MountPolicyProgramWire` (`program.rs:136-144`),
`#[derive(Serialize, Deserialize)]` with `#[serde(deny_unknown_fields)]`. Its
fields: `version: Option<u32>`, `rules`, `protect`, `writes: WritePolicy`,
`case_sensitivity: CaseSensitivity`. `deny_unknown_fields` means any unknown
JSON key (including the retired `masked_writes` scalar) fails the load.

**Version fail-closed Deserialize** (`program.rs:408-434`): `Serialize` always
emits `version: Some(1)` (`program.rs:397-404`). `Deserialize` first deserializes
the wire struct, then `match wire.version` — `Some(1)` recompiles patterns with
the recorded case sensitivity and returns the program; `Some(version)` returns
`de::Error::custom("unsupported mount policy program version {version};
supported version is 1")`; `None` returns `de::Error::custom("mount policy
program version is required (expected version 1)")`. So both a missing version
and any non-1 version are hard load failures. (The msb side additionally calls
`recompile_patterns` on deserialize at `program.rs:423`; the workestrate side
does not, since it never round-trips through JSON at compile time — see §16.1
drift note.)

**`PathPolicyRule`** (`rule.rs:52-62`): `{ effect: RuleEffect, pattern: Pattern,
overridable: bool, origin: RuleOrigin }`. `is_terminal()` is `!overridable`
(`rule.rs:70-72`) — a terminal rule freezes further matches for that path.

**`RuleEffect`** (`rule.rs:13-20`): `Mask` / `Unmask`, `#[serde(rename_all =
"snake_case")]`.

**`RuleOrigin`** (`rule.rs:41-49`): `{ layer: String, file: PathBuf, scope_kind:
ScopeKind }` — full provenance carried into the wire format and into explain
traces.

**`ScopeKind`** (`rule.rs:23-38`): six variants, `#[serde(rename_all =
"snake_case")]`, ordered by `PartialOrd/Ord` so that declaration order IS
authority order: `HomeRegistry` (highest) < `UserGlobalOverrides` <
`ReferenceConfig` < `ConfigRepoLayer` < `Workload` < `MountEntry` (lowest).
`authority()` is `self as u32` (`rule.rs:77-79`).

**`CaseSensitivity`** (`program.rs:62-71`): `Sensitive` (default) / `Insensitive`,
`#[serde(rename_all = "lowercase")]`. `Insensitive` is applied at glob compile
time on the msb side (commit 6e51840b, `program.rs:151-163` `recompile_patterns`
→ `Pattern::set_case_insensitive`); workestrate never emits `Insensitive` (v1
rejects it at compile, §16.2).

**`CompiledRuleSet` / `WritePolicy`** (`program.rs:52-60`): `{ allow:
Vec<PathPolicyRule>, deny: Vec<PathPolicyRule> }`. `WritePolicy` is a type alias
for `CompiledRuleSet`.

**`WriteRuleEffect`** (`program.rs:40-49`): `Allow` / `Deny` / `Protect`,
`snake_case`. (`Protect` appears only in explain traces for protect-tier
matches; the wire `writes` bucket only carries allow/deny.)

**`Decision`** (`program.rs:18-27`): `Visible` / `Masked` / `TraversalOnly`,
`snake_case`. `TraversalOnly` is a masked directory that may contain an unmasked
descendant — shown in readdir so the guest can traverse to the descendant, with
masked contents filtered.

**`WriteDecision`** (`program.rs:30-37`): `Allow` / `Deny`, `snake_case`.

**`Explained<T, M>`** (`program.rs:105-115`): `{ decision: T, matches: Vec<M>,
frozen_by: Option<RuleOrigin>, fail_closed_non_utf8: bool }` — retains every
matching rule in compile order, including frozen-out matches.

**`RuleMatch`** (`program.rs:74-86`): `{ rule_index, effect, terminal, origin,
frozen_out }`. (msb omits the `pattern: String` field that workestrate's
`RuleMatch` carries — non-wire, explain-trace only; see §3 drift note.)

### 15.2 Pattern evaluation

`Pattern` (`mount_policy/pattern.rs:17-23`) wraps a `globset::GlobMatcher` plus
an optional `stem` matcher (for `.../**` patterns) and a `dir_only: bool`. It
preserves the `raw: String`.

Compilation (`pattern.rs:106-154`, `compile_inner`):
- Empty → `PatternErrorKind::Empty`.
- Contains NUL → `NulByte`.
- Starts with `/` → `Absolute` (patterns are mount-root-relative).
- Trailing `/` → `dir_only = true`; the body is the pattern without the slash.
- Any `..` component (split on `/`) → `ParentEscape`.
- `GlobBuilder::new(body).literal_separator(true).case_insensitive(case_insensitive).build()` →
  `compile_matcher()`. `literal_separator(true)` means `*` and `?` never cross
  `/`, so patterns are root-anchored by default.
- A `body` ending in `/**` (and not just `/**`) additionally compiles a `stem`
  matcher on the prefix, so `foo/**` matches `foo` itself as well as `foo/bar`.
  This is the `**`-descendant form; a bare `**/`-prefixed pattern (floating) has
  no literal prefix.

Matching (`pattern.rs:157-184`): `matches_unknown` (the entry point used by the
evaluator) is `is_match(path) || matches_any_ancestor(path)`. `is_match` tries
the main matcher then the stem matcher. `matches_any_ancestor` walks up the path
by `rfind('/')` so a pattern matching an ancestor directory also matches the
descendant (fail-closed union semantics — a dir-only pattern matching the path
exactly counts). `matches_path` adds the `!dir_only` guard for file paths;
`matches_dir` does not.

`literal_prefix()` (`pattern.rs:187-201`) extracts the leading literal components
(stops at `**` or any glob metachar) for `may_unmask_descendant` analysis;
`extends_below` records whether the pattern has a glob tail beyond the literal
prefix.

Non-UTF-8 fail-closed: `LexicalPath::from_bytes` (`lexical.rs:73-82`) produces a
sentinel `non_utf8: true` path with empty components; the evaluator's `decide`
returns `Decision::Masked` with `fail_closed_non_utf8: true` (`program.rs:172-179`)
and `decide_write` returns `WriteDecision::Deny` (`program.rs:252-258`).

Case sensitivity at compile (commit 6e51840b): `MountPolicyProgram::recompile_patterns`
(`program.rs:151-163`) iterates rules+protect+writes.allow+writes.deny and calls
`rule.pattern.set_case_insensitive(case_insensitive)` after deserialize, so the
recorded `case_sensitivity` actually pins matcher behavior at load time.

### 15.3 Lexical path derivation

The enforcement layer needs the mount-root-relative lexical path of an inode to
evaluate policy, but PassthroughFs does not cache paths (renames would
invalidate them, and generation counters add bookkeeping). Instead it derives
the path on demand by walking the inode's *anchor alias*.

Each `InodeData` carries an `anchor_parent: AtomicU64` and `anchor_name:
RwLock<Vec<u8>>` (the parent synthetic inode + name bytes of one trusted alias),
plus an `aliases: RwLock<BTreeSet<NamespaceAlias>>` of all known (parent, name)
pairs. `current_anchor_alias` (`inode.rs:~688-705`) returns the anchor alias,
preferring the stored anchor and falling back to the first alias.

`lexical_child_path(fs, parent, name)` (`inode.rs:254-265`) takes the read lock
on the inode table, calls `build_anchor_components_locked(&inodes, parent,
&mut seen)`, appends `name`, and joins the components with `/`. It returns
`None` if any component is non-UTF-8 (so the caller fails closed).

`build_anchor_components_locked` (`inode.rs:757-772`) is the recursive anchor
walk: base case `inode == 1` (mount root) → empty components; otherwise it
inserts `inode` into a `seen` HashSet (cycle break → `EIO`), fetches the inode's
`current_anchor_alias`, and recurses through `build_alias_components_locked`
(`inode.rs:741-754`) which validates the name component and recurses on
`alias.parent`. `validate_component` (`inode.rs:775-786`) rejects empty/`.`,
`..`, `/`, and NUL — so a corrupted anchor cannot escape the mount root.

**Why lazy wins:** a rename simply rewrites the anchor alias of the moved inode
(via `register_alias_locked` / `remove_alias_locked` in `do_rename`); no path
cache needs invalidation, and no generation counter needs bumping. The next
lookup derives the new path for free. Host-side reconcile (the kernel's
`/proc/self/fd/N` reopen in `get_inode_fd_linux`, `inode.rs:647-660`) is also
lazy: it tries the current anchor alias, then candidate aliases, then the
retained fd — so a stale anchor just means "try the next alias" (`inode.rs:658`).

`lexical_inode_path` (`inode.rs:271-280`) is the inode-keyed variant (used by
`do_open`/`do_write` which have an inode, not a parent+name).

`synthetic_inode_for_host_path` is the cascade helper (see §15.6): given a host
fd, it reads `linux_alt_key_from_fd` (statx → `InodeAltKey{ino, dev, mnt_id}`)
and looks up the inode table by alt-key to recover the synthetic inode number
needed for tag eviction.

### 15.4 Tag store (`tag_store.rs`)

The tag store records which masked aliases the guest has written (and thus
become visible). It is the mechanism that turns "mask = read boundary" into a
write-tagging visibility upgrade.

**Data structure** (`tag_store.rs:27-41`):
```rust
type TagKey = (u64, Vec<u8>);  // (parent_synthetic_inode, name bytes)
struct TagValue {
    file: std::fs::File,   // held to pin the duplicated O_PATH fd
    alt_key: InodeAltKey,  // identity recorded at tag time
}
struct TagStore {
    cache: Mutex<LruCache<TagKey, TagValue>>,
    exhausted: AtomicU64,
}
```
Capacity is `const CAPACITY: usize = 10_000` (`tag_store.rs:20`), wrapped in a
`std::sync::Mutex` (`tag_store.rs:39`). The `exhausted` `AtomicU64` counts LRU
evictions (`tag_store.rs:110`, `Ordering::Relaxed`) — it is a diagnostic counter
for capacity exhaustion, exposed as `evictions_exhausted()` (`tag_store.rs:138-140`).

**Why alias-scoped, not inode-only:** the key is `(parent_synthetic_inode,
name_bytes)`, not the host inode. If a masked file `foo` is tagged (visible) and
then the host replaces `foo` with a different object under the same name, the
tag must NOT unmask the replacement. Keying by alias (parent+name) plus
revalidating the stored `alt_key` against the current identity at lookup
(`inode.rs:340-363`) catches the swap: a mismatch evicts the tag and returns
ENOENT. Keying by inode alone would wrongly inherit visibility.

**Why O_PATH fd pins the host inode:** `tag()` (`tag_store.rs:81-113`) dups the
caller's O_PATH fd via `fcntl(F_DUPFD_CLOEXEC)` and stores the resulting `File`
in `TagValue.file`. The kernel will not recycle an inode that still has an open
file reference, so the host inode number cannot be reused by an unrelated file
while the tag is live — closing the alias-replacement race window from the
kernel side.

**Linux gating:** the whole module is compiled under the passthroughfs/unix
tree and the enforcement call sites are `#[cfg(target_os = "linux")]`; macOS has
no tag store.

**Eviction invariants:** `evict(parent, name)` (`tag_store.rs:116-118`) pops one
key. `evict_parent(parent)` (`tag_store.rs:121-134`) collects all keys with
matching parent under one lock, then re-locks to pop them (the double-lock is
the review nit in §8). `clear()` (`tag_store.rs:143-145`) drops every retained
fd. LRU eviction (`push` returning `Some`) drops the least-recently-used
`TagValue`, closing its O_PATH fd — the host inode may then be recycled.

**The `.lock().unwrap()` poison sites** (T13): `contains` (`tag_store.rs:66`),
`get_identity` (`:75`), `tag` (`:107`), `evict` (`:117`), `evict_parent` (`:125`
and `:130` — two locks), `clear` (`:144`). That is 7 `.lock().unwrap()` call
sites in `tag_store.rs` (the 8th lock in the broader feature is workestrate's
`COLLECTED` mutex in `mount_policy/mod.rs:58`/`:62`, which uses
`unwrap_or_else(|e| e.into_inner())` and is already poison-recovering). On the
msb side, a panic while holding the tag-store mutex poisons it; every subsequent
`.unwrap()` would panic again, and the 10k retained O_PATH fds would leak until
process exit. T13 asks to either propagate the poison or document the
panic-on-poison + fd-leak contract.

### 15.5 Enforcement map

Exhaustive op → site → behavior. All hooks are `#[cfg(target_os = "linux")]`.

| Op | Site (file:line) | Behavior |
|---|---|---|
| `do_lookup` | `inode.rs:220-249` | Pre-gate: derive `lexical_child_path`; `is_protected`→ENOENT; `decide==Masked && !tagged_visible(parent,name)`→ENOENT. Then `do_lookup_linux` (`:292-364`): open with `RESOLVE_BENEATH`; if masked, revalidate stored tag identity vs `alt_key` from statx — no tag→close fd+ENOENT; mismatch→`evict`+close fd+ENOENT; match→proceed. |
| `build_snapshot` | `dir_ops.rs:296-326` | `entries.retain`: skip `.`/`..`; derive lexical child path; `is_protected`→drop; `decide==Masked`→keep only if `tagged_visible(dir_inode,name)`; `TraversalOnly`/`Visible`→keep. Offsets renumbered contiguously by the snapshot builder. |
| `do_readdirplus` / `do_readdirplus_for_each` | `dir_ops.rs:101-142`, `:145-230` | Per-entry `do_lookup`; `lookup_says_gone` (ENOENT/ENOTDIR) → `continue` (defense-in-depth: masked entries that slipped past the snapshot filter vanish). Non-gone errors → `no_lookup_entry()` (inode 0). |
| `do_open` | `file_ops.rs:37-184` | `is_protected`→EACCES (`:56-58`); if `write_intent` and `decide_write==Deny`→EACCES (M3/M4 hoisted above masked branch, `:60-67`); `decide==Masked`: read-open untagged→ENOENT (`:73-75`); write-open→allow (tag inserted after open). Original `flags` checked for write_intent BEFORE writeback O_RDWR widening (`:50`, `:99-104`). Post-open masked revalidation (`:110-149`): no stored identity & read-only→ENOENT; identity mismatch→`evict`+ENOENT. `masked && write_intent`→`fs.tag_inode(inode)` (`:152-154`). |
| `do_write` | `file_ops.rs:213-283` | `is_protected`→EACCES (`:240-242`); `decide_write==Deny`→EACCES (`:244-249`); compute `write_masked` from `decide==Masked`. After successful `written > 0 && write_masked`→`fs.tag_inode(inode)` (`:264-266`). |
| `do_create` | `create_ops.rs:47` | `admit_child` (`:524-544`): `is_protected`||`decide_write==Deny`→EACCES; `decide==Masked||TraversalOnly`→masked=true. On success `masked`→`fs.tag_child(newparent,newname)` (`:518-520`). |
| `do_mkdir` | `create_ops.rs:165` | Same `admit_child` gate; `masked`→tag child. |
| `do_mknod` | `create_ops.rs:231` | Same `admit_child` gate; `masked`→tag child. |
| `do_symlink` | `create_ops.rs:332` | Same `admit_child` gate; `masked`→tag child. |
| `do_link` | `create_ops.rs:454` | Same `admit_child` gate; `masked`→tag child. (Uses `/proc/self/fd/N` with `AT_SYMLINK_FOLLOW` to link by fd reference, `:481`.) |
| `do_unlink` | `remove_ops.rs:69-210` | `remove_admission` (`:836`...): `is_protected`→ENOENT; `decide_write==Deny`→EACCES; masked-untagged→ENOENT (no host mutation); masked-tagged→allow. On success: `tagged_masked`→`tags.evict(parent,name)` (`:179-181`). |
| `do_rmdir` | `remove_ops.rs:213-335` | `rmdir_admission` (`:837-901`): `is_protected`→ENOENT; visible→`decide_write==Deny`?EACCES:allow; tagged-masked→`decide_write==Deny`?EACCES:allow; untagged-masked→probe `decide_child(PROBE)`; if descendants masked→check emptiness fd-relative, empty→ENOENT, non-empty→allow (cascade will run); else ENOENT. On `unlinkat` ENOTEMPTY + masked + `cascade_remove` success → retry `unlinkat`. `tagged_masked`→`tags.evict` (`:331-333`). |
| `do_rename` | `remove_ops.rs:572-797` | `rename_admission` (`:904-938`): src `is_protected`||(masked && !tagged)→ENOENT (anti-laundering); dst `is_protected`→ENOENT; dst `decide_write==Deny`→EACCES. After `renameat2`: source alias removed+re-registered at dst; overwrite target alias removed (fd retained if detached). Tags invalidated: source `evict(olddir,oldname)` + dst `evict(newdir,newname)` (`:742-745`). `RENAME_EXCHANGE` (`:681-712`): if `target_key == source_key` (identity swap) → evict both tags and return early; otherwise swap aliases and evict both tags. |
| `forget` / `destroy` | `inode.rs:551-599` | `forget_one_locked`: when refcount hits 0, `evict_inode_tags(fs, &data, inode)` (`:582`) removes orphaned tags for that inode, then `maybe_remove_inode_locked`. `destroy` (PassthroughFs drop) calls `tags.clear()` to drop every retained O_PATH fd. |
| `metadata` / `xattr` / `special` | `metadata.rs`, `xattr_ops.rs`, `special.rs` | No policy gate: all entry points are inode-keyed (`get_inode_fd` → `stat_inode` / xattr ops), and the inode can only have been obtained through a policy-gated `do_lookup`/`do_open`. Verified no by-path bypass exists — there is no public op that takes a raw host path. |

### 15.6 Cascade algorithm (`remove_ops.rs` `cascade_remove` / `cascade_remove_fd`)

When `do_rmdir` hits `ENOTEMPTY` on a masked directory whose descendants are also
masked (`rmdir_admission` probe at `:875-878`), it calls `cascade_remove(fs,
lexical, depth=0, dir_synthetic_inode=None)` (`:276`). The cascade walks the
host directory tree and unlinks only untagged-masked entries, so a guest `rm -rf`
on a masked dir succeeds without ever exposing the masked names.

`cascade_remove` (`:338-389`):
- `depth > 64` → return false (fail-closed; `:344`).
- Root itself (`lexical.components().is_empty()`) → false (`:351`).
- Open the dir fd-relative from the backend root via
  `inode::secure_open_path_linux` (`:364-371`, `inode.rs:788-829`): per-component
  `O_PATH|O_NOFOLLOW|O_DIRECTORY` with `open_beneath` — never follows symlinks,
  contained beneath the root.
- Resolve the synthetic inode for tag eviction: prefer the caller value, else
  `linux_alt_key_from_fd(dir_fd)` → inode table `get_alt` (`:376-386`).
- Delegate to `cascade_remove_fd(fs, dir_fd, lexical, depth, dir_synthetic_inode,
  policy)` (`:388`), which consumes `dir_fd` via `fdopendir`/`closedir`.

`cascade_remove_fd` (`:394-569`):
- `depth > 64` → close fd, return false (`:402-407`).
- `fdopendir(dir_fd)` takes ownership (`:411`); null → close+false.
- Loop `readdir` (errno=0 before each call to distinguish EOF from error,
  `:423`):
  - skip `.`/`..`.
  - non-UTF-8 name → closedir+false (`:445-450`, fail-closed).
  - `lexical.child(name)` → invalid → closedir+false.
  - `is_protected(child)` → closedir+false (`:457-462`).
  - `decide(child)`:
    - `Visible` → closedir+false (`:464-469`) — a visible entry blocks the cascade.
    - `Masked` but `tagged_visible(parent,name)` → closedir+false (`:470-478`) — tagged entries block.
    - `Masked`/`TraversalOnly` untagged → `fstatat(AT_SYMLINK_NOFOLLOW)`:
      - `S_IFDIR` → `openat(O_RDONLY|O_NOFOLLOW|O_DIRECTORY)`, recurse
        `cascade_remove_fd(..., depth+1, child_synth, policy)`; on success
        `unlinkat(AT_REMOVEDIR)` the now-empty dir (`:501-542`).
      - non-dir (file/symlink/special) → `unlinkat` the entry; symlinks are
        deleted as entries only, never followed (`:543-555`).
  - After each successful unlink, `tags.evict(parent_inode, name)` (`:556-560`).
- EOF → closedir, return true.

Identity mismatch / host-added entry / depth overflow all return false
(ENOTEMPTY fail-closed) — the guest sees `ENOTEMPTY` and the directory is not
removed, so no masked content is silently destroyed.

### 15.7 Write-deny as global write ACL

`writes.deny` is a user-decided global write ACL (spec 22 §10): it blocks
create/write/unlink/rmdir/rename-dst at matching paths *regardless of mask
state*. The M3/M4 enforcement hoists `decide_write==Deny → EACCES` ABOVE the
masked branch so deny applies even to visible paths.

Precedence ordering (cited in code):
- `is_protected(path)` → EACCES (protect beats deny; protect is the highest
  read+write boundary).
- `decide_write(path) == Deny` → EACCES (write-deny).
- masked-untagged → ENOENT (mask precedence for reads; an invisible path is
  "missing" before it is "denied").

Sites: `do_open` (`file_ops.rs:60-67`), `do_write` (`:244-249`),
`admit_child` for create/mkdir/mknod/symlink/link (`create_ops.rs:532-538`),
`remove_admission` for unlink (`remove_ops.rs:820-824`),
`rmdir_admission` for visible/tagged dirs (`:853-857`, `:863-867`),
`rename_admission` for dst (`:931-935`). Commit c9507cde extended this to
`fallocate`, `copy_file_range`, `ftruncate`, and `setattr` (the metadata-mutation
paths) — each checks `decide_write==Deny` before mutating.

### 15.8 Fail-closed loader (`vm.rs` `load_mount_policy`)

`load_mount_policy(approved_root, policy_path)` (`vm.rs:1900-2019`) is the
runtime's only entry point for policy JSON. It is called BEFORE VM startup
during mount setup, so a bad policy fails the boot rather than running an
unprotected guest.

Confinement (`:1904-1913`): reject empty or absolute `policy_path`
(`Path::new(policy_path).is_absolute()` → `PathEscape`); reject any
`Component::ParentDir` → `PathEscape`. So the policy file must be a relative
path beneath `approved_root`.

Per-component open (`:1915-2006`):
- Open `approved_root` with `O_RDONLY|O_DIRECTORY|O_CLOEXEC|O_NOFOLLOW`
  (`:1924-1929`). `O_NOFOLLOW` rejects a symlinked state dir; `ELOOP`→
  `MountPolicyLoadError::SymlinkRejected`, `ENOENT`→`FileNotFound(approved_root)`.
- Walk each `Component::Normal` of `policy_path`: `openat(current, name,
  O_RDONLY|O_CLOEXEC|O_NOFOLLOW | (O_DIRECTORY if not last))` (`:1959-1966`).
  Intermediate components require `O_DIRECTORY`; the final component is the
  policy file. `O_NOFOLLOW` on every component rejects symlink swaps mid-path.
  `ELOOP`→`SymlinkRejected`, `ENOENT`→`FileNotFound(policy_path)`.
- On the final component, `File::from_raw_fd(current)` + `read_to_end` → bytes
  (`:1995-2004`).

Parse-once (`:2009-2012`): `serde_json::from_slice::<MountPolicyProgram>(&bytes)`
→ `MountPolicyLoadError::from`. The version fail-closed Deserialize (§15.1)
runs here, so `None`/`≠1`/unknown-field JSON all reject. The program is then
handed to `PassthroughConfig` and held immutable for the mount lifetime.

Every `unsafe` block has a SAFETY comment (e.g. `:1919-1923`, `:1967-1972`,
`:1989-1992`) documenting fd ownership and why `O_NOFOLLOW`+`O_DIRECTORY` make
the path non-escapable. Typed error (`MountPolicyLoadError`) landed in commit
c7f8e665; the diverging-binding cleanup in f7ee5756.

### 15.9 Symlink contract

The broker never follows interior symlinks. Path resolution uses `openat2` with
`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS`
(`platform.rs:535-546`, `OPENAT2_RESOLVE_FLAGS`). `open_beneath`
(`platform.rs:606-633`) issues `syscall(SYS_OPENAT2=437, dirfd, name, &OpenHow,
sizeof)` with those resolve flags; on `ENOSYS` (pre-5.6 kernel) it falls back to
`openat(dirfd, name, flags|O_CLOEXEC)` (`:627-632`). The fallback is documented
as pre-existing (`platform.rs:14-17`, `inode.rs:288-291`): `openat(O_NOFOLLOW)`
blocks the final-component symlink but not interior ones, so pre-5.6 loses the
kernel-enforced containment — the broker still never *intentionally* follows
symlinks, but the kernel no longer enforces it atomically.

`has_openat2` is probed once at PassthroughFs init (`platform.rs:576-600`,
`builder.rs:204-206`) and cached in an `AtomicBool` (`mod.rs:216`).

`follow_root_symlinks: bool` (mount-root only; default `false`; parsed in
`parse_mount_spec` at `vm.rs:2095-2104`) opts a mount out of the default
no-follow root resolution. Workestrate does NOT expose this flag (it always
ships the protective default).

`/proc/self/fd/N` reopen (`open_inode_fd`, `inode.rs:1045-1061`): before
reopening, `fstat` the inode fd and reject `S_IFLNK` with `ELOOP` (`:1050-1052`)
— a real host symlink inode is never followed through procfd. `O_NOFOLLOW` is
deliberately NOT added to the reopen flags because `/proc/self/fd/N` is itself a
symlink and would always fail with `ELOOP` (`inode.rs:1042-1044`).

`do_readlink` (`create_ops.rs:555-`) returns the raw target for real host
symlinks (`platform::readlink_fd`, `:567`) — a name-level metadata leak (the
guest sees the symlink target string) accepted in v1. Tagging a symlink never
unmasks its target: the tag is on the symlink's alias, and `do_lookup` of the
target re-evaluates policy on the target's lexical path.

### 15.10 Hardlink limitation

Name-masked ≠ object-masked. Two directory entries (names) can point to the same
host inode (hardlink); masking one name does not mask the other, because policy
is evaluated on the lexical path (the name), not the inode. A guest could read a
masked file through an unmasked hardlink.

The runtime does not auto-mask aliases in v1. Instead `preview` (workestrate
side, §16.8) warns: it collects `(dev, ino)` for every visible entry, then for
every masked entry checks whether its `(dev, ino)` matches a visible entry's
(`commands/policy.rs:293-302`, `:406-416`) and emits a `hardlink alias` warning.
This is a diagnostics-only detection; enforcement does not block the hardlink
read.

## 16. Technical architecture — workestrate compiler & CLI

This section describes the workestrate-side pure compiler library, config
collection, per-mount compilation, policy-file lifecycle, and diagnostics CLI
under `control/agentctl/src/`. The library is pure (no I/O except the
diagnostics CLI's `std::fs` tree walk).

### 16.1 The `mount_policy/` library

Module map (`mount_policy/mod.rs:36-53`):
- `value.rs` — `PolicyValue<T>` compact/expanded entry forms.
- `scope.rs` — `PolicyScope`, `ScopeKind`, `MountsFragment`, `WritesFragment`,
  `CollectedPolicy`.
- `pattern.rs` — `Pattern` glob dialect newtype (mirrors msb's `Pattern`).
- `lexical.rs` — `LexicalPath` (mirrors msb's).
- `rule.rs` — `PathPolicyRule`, `RuleEffect`, `RuleOrigin`.
- `compile.rs` — the compiler.
- `program.rs` — `MountPolicyProgram` + pure evaluator.

`Pattern` is a globset wrapper that preserves the `raw: String`; its dialect is
identical to msb's (§15.2): root-anchored, `literal_separator(true)`, `**`
descendant form compiles a stem matcher, trailing `/` is dir-only, absolute/NUL/
`..`/empty rejected.

`PolicyValue<T>` (`value.rs:27-34`): `{ value: T, overridable: bool }`.
`overridable` defaults to `true` in both forms and must be explicit to be
`false` (a non-overridable value becomes a terminal rule at compile time). It is
parsed with the `deserialize_any` visitor idiom (`value.rs:101-165`):
`PolicyValueVisitor` implements `visit_str`/`visit_string` (compact form →
`PolicyValue::overridable`) and `visit_map` (expanded form → delegates to a
`deny_unknown_fields` `RawExpanded` struct via `MapAccessDeserializer` so
unknown-field errors stay verbatim). `#[serde(untagged)]` is deliberately NOT
used (`value.rs:9-15`): untagged enums buffer-and-retry, producing
positionless, context-free errors. The precedent is `RawBindingVisitor` in
`config/types.rs` (spec 16). The scalar `T` is either `String` (collected
fragment form; compiler validates against the declaring origin) or `Pattern`
(expanded-form key `pattern`).

`ScopeKind` (`scope.rs:17-33`): the same six variants as msb, `snake_case`,
`PartialOrd/Ord` so authority order = compile order. `is_operator()` (`:44-49`)
returns true only for `HomeRegistry` and `UserGlobalOverrides` — only operator
scopes may declare terminal unmasks/protects.

`PolicyScope` (`scope.rs:143-157`): `{ scope_kind, layer_name, source_path:
PathBuf, fragment: MountsFragment, mount_guest: Option<String> }`. `for_mount(guest)`
(`:193-196`) tags a mount-entry scope with its declaring mount's guest path.
`origin()` (`:199-205`) builds the `RuleOrigin` the compiled rules will carry.

`MountsFragment` (`scope.rs:73-98`): `#[serde(deny_unknown_fields)]` with
`mask: Vec<PolicyValue<String>>`, `unmask`, `protect`, `writes:
Option<WritesFragment>`, `case_sensitivity: Option<String>`. The former scalar
`masked_writes` field is intentionally gone — it is rejected as an unknown field
(`scope.rs:71-72`, test at `:290-293`) rather than silently accepting old
policy. `WritesFragment` (`scope.rs:101-110`): `{ allow, deny }` — NOT a scalar;
the user-locked global write ACL is pattern-keyed.

`CollectedPolicy` (`scope.rs:163-173`): `{ global: Vec<PolicyScope>, workloads:
HashMap<String, Vec<PolicyScope>> }` — global scopes shared by every workload,
workload scopes keyed by name. A process-global `COLLECTED: Mutex<Option<CollectedPolicy>>`
(`mod.rs:55-63`) holds the collected policy between the config-load phase and
the workload-construction phase; it uses `unwrap_or_else(|e| e.into_inner())` so
it is poison-recovering (unlike the msb tag store).

### 16.2 The compiler (`compile.rs`)

`compile(scopes: Vec<PolicyScope>) -> Result<MountPolicyProgram, CompileError>`
(`compile.rs:120-145`):
- **Authority sort**: `scopes.sort_by_key(|s| s.scope_kind.authority())`
  (`:122`) — stable sort, so operator scopes come first and same-kind scopes
  (e.g. config-repo layers) keep registry stack order.
- Per scope: `validate_program_flags` (case_sensitivity must be `"sensitive"`,
  `:148-158`) then `compile_scope` (`:163-245`).
- `compile_scope` emits mask rules then unmask rules (per-scope mask-then-
  unmask, `:168-173`), compiles `protect` (`:174-187`), and compiles
  `writes.allow`/`writes.deny` (`:188-200`).
- Output: `MountPolicyProgram { rules, version: 1, protect, writes,
  case_sensitivity: Sensitive }` (`:136-144`).

**Trust validation** (`:202-216`): a terminal (`!overridable`) `Unmask` from a
non-operator scope → `CompileError::TerminalUnmaskFromNonOperator` naming the
origin and pattern. Terminal MASKS are allowed from any scope (masking is the
fail-closed direction). Terminal `Protect` from a non-operator scope →
`TerminalProtectFromNonOperator` (`:180-187`).

**Exact-duplicate conflict** (`:218-242`): the same raw pattern as both mask and
unmask in ONE scope, where at least one is terminal, →
`CompileError::DuplicateTerminalConflict` naming BOTH origins
(`DuplicateConflict { pattern, mask_origin, unmask_origin }`). Overridable-only
duplicates resolve harmlessly by mask-then-unmask ordering (test `:494-506`).
Duplicates across different scopes are normal precedence, not an error
(`:509-527`).

**Overlapping-glob conflicts are NOT compile errors**: two patterns whose globs
overlap (e.g. `foo/**` and `foo/bar`) are both kept; the evaluator's
last-non-frozen-rule-wins + freeze semantics resolves them at decision time, and
`explain` surfaces the full match trace. The compiler only rejects exact-string
terminal contradictions.

**Mount-guest tagging**: `PolicyScope.mount_guest` partitions mount-entry scopes
per mount. The collector tags each mount-entry scope with `for_mount(guest)`
(`loading.rs:471-479`); the per-mount compiler (`workload/config.rs:94-97`)
filters mount-entry scopes by `scope.mount_guest == Some(mount.guest)`.

`CompileError` variants (`:28-58`): `Pattern{source}`, `DuplicateTerminalConflict`,
`TerminalUnmaskFromNonOperator`, `TerminalProtectFromNonOperator`,
`UnsupportedCaseSensitivity`. Every variant names the declaring origin.

### 16.3 Per-mount compilation (`workload/config.rs` `new_with_use_overrides`)

`ConfigWorkload::new_with_use_overrides` (`:70-150`) compiles one program per
mount. The loop (`:81-114`):
```
for mount in workload.mounts (DECLARATION ORDER):  // NOT HashMap iteration
    scopes = collected.global.clone()
    scopes.extend(workload scopes where scope.scope_kind != MountEntry
                  OR scope.mount_guest == Some(mount.guest))
    if scopes.is_empty(): skip
    program = compile(scopes)
    mount_policies.push((mount.guest.clone(), program))
```
`workload.mounts` is a `Vec<MountPlan>` (`config/types.rs:536`), so iteration is
in declaration order — determinism (a `HashMap` would make the compiled program
order-dependent on hash seed). The result is stored as
`mount_policies: Vec<(String, MountPolicyProgram)>` (`:52`) keyed by guest path.

`mount_policy_for(guest)` (`:250-255`) is the accessor: linear find by
`mount_guest == guest`. (`mount_policy()` at `:245-248` returns the first
program — a legacy accessor kept for the `Workload` trait; the diagnostics CLI
uses `mount_policy_for` for true per-mount selection, commit 35afdad.)

This fixed the cross-mount rule leakage bug (commit 6fa722c): the prior code
shipped one flat per-workload program (all mount-entry scopes merged), so a
rule declared on mount A leaked onto mount B. The per-mount partition is
necessary because each mount's PassthroughFs is constructed once with one
program (§17).

### 16.4 Evaluator API (`program.rs`)

`MountPolicyProgram::decide(&LexicalPath) -> Explained<Decision>`
(`program.rs:195-273`): non-UTF-8 → `Masked` fail-closed; `protect` matches →
`Masked` with `frozen_by` from the first terminal protect; otherwise iterate
`rules` in compile order, last non-frozen match wins (`Mask`→Masked,
`Unmask`→Visible), terminal match freezes; if final decision is `Masked` and
`may_unmask_descendant(path)` → `TraversalOnly`. Every match (including
frozen-out) is retained in `matches` with `{ rule_index, effect, terminal,
origin, frozen_out, pattern: String }` (`RuleMatch`, `:72-86`).

`decide_child(dir, name)` (`:386-396`): `dir.child(name)` then `decide`; invalid
child name → `Masked` fail-closed.

`decide_write(path) -> Explained<WriteDecision, WriteRuleMatch>`
(`:278-375`): non-UTF-8 → `Deny` fail-closed; protect matches → `Deny` (protect
beats writes); then reconstitute authority order across the public allow/deny
split (`:318-337`: collect `(authority, bucket, index)` for both buckets, sort
by `(authority, allow-before-deny, index)`), apply last-non-frozen-wins. A
terminal deny freezes lower-authority rules including later allows.

`may_unmask_descendant(dir)` (`:408-427`): conservative literal-prefix analysis.
For each unmask rule: no literal prefix (e.g. `**/`-prefixed) → `true`; else
`below_dir` (the literal prefix is a proper descendant of `dir`) OR
`within_prefix` (the pattern extends below its literal prefix and `dir` is
within it). Always-true is the safe direction (a masked dir becomes
`TraversalOnly` rather than invisible).

`is_protected(path)` (`:377-382`): any protect rule matches.

### 16.5 Policy-file lifecycle (`policy_file.rs`)

`policy_dir(state_dir)` = `state_dir/policy` (`:13-15`).
`policy_file_path(state_dir, instance, mount_slug)` =
`state_dir/policy/<instance>/<mount_slug>.json` (`:18-22`) — per-instance subdir
layout.
`mount_slug(guest)` (`:28-30`): `guest.strip_prefix('/').unwrap_or(guest).replace('/', "_")`
— so `/workspace` → `workspace`, `/data/cache` → `data_cache`.

`write_policy_file` (`:33-52`): `create_dir_all(policy/<instance>)`; write to a
tmp file `policy/<instance>/<slug>.json.tmp.<pid>`; `serde_json::to_vec_pretty`;
`std::fs::write(&tmp, bytes)`; on unix `set_permissions(tmp, 0o600)` BEFORE the
rename; `rename(tmp, final_path)` (atomic). Returns the final path.

`remove_policy_dir(state_dir, instance)` (`:55-61`): best-effort
`remove_dir_all(policy/<instance>)`; `NotFound` → Ok (idempotent).

`write_policy_file` is called in `build_sandbox` (runtime/run.rs) per mount with
a policy; `remove_policy_dir` is called on `down` (best-effort).

### 16.6 Plan integration seam (`plan.rs` + `mounts.rs`)

`MountPlan` (`plan.rs:110-123`): `{ host, guest, read_only, policy:
Option<MountsFragment>, policy_file: Option<PathBuf> }`. `policy_file` is
`#[serde(default, skip_serializing_if = "Option::is_none")]` (`:121-122`) — it
is a runtime-resolved field, never user-set. `SandboxPlan` has NO `policy_file`
field (it was REMOVED in the per-mount migration, commit a36b074); each
`MountPlan` carries its own.

`plan()` leaves `policy_file: None` for every mount (`plan.rs:417,424`) —
deferral. The policy file is written later in `build_sandbox` (after the state
dir is resolved) and the path is stored on the `MountPlan` for the (currently
dormant) SDK call.

The `cmd_plan --instance` cosmetic fix: `runtime/run.rs:345-347` overrides
`plan.name = spec.instance.clone()` (the `slot@id` form) so the plan display
matches the runtime instance name, instead of the bare workload name.

`apply_mount_policy(builder, plan)` (`mounts.rs:67-75`) is the **no-op seam**:
```rust
pub(crate) fn apply_mount_policy(builder: SandboxBuilder, plan: &SandboxPlan)
    -> Result<SandboxBuilder> {
    for m in &plan.mounts { let _ = m.policy_file.as_ref(); }
    Ok(builder)
}
```
The exact TODO (`mounts.rs:61-66`) for the SDK switch (T4, blocked by spec 23):
once the Cargo dep moves to the fork, replace with
`builder = builder.volume(&m.guest, |v| { v.bind(host).policy_file(m.policy_file.as_ref()) });`
(or the fork's final per-sandbox equivalent). Until then the whole
workestrate→msb transmission is dormant — the policy JSON is written to the host
state dir but never passed to the SDK.

### 16.7 Config surface

**Scope collection** (`config/loading.rs:407-486`, `collect_policy_scopes`):
- `HomeRegistry`: from `registry.policy.mounts` (`:413-421`).
- Per layer (`:423-484`): `ReferenceConfig` if `layer.name == "reference"`,
  `UserGlobalOverrides` if `layer.name.ends_with("-override")`, else
  `ConfigRepoLayer`. Each layer's top-level `policy.mounts` → global scope
  (`:435-442`); each workload's `policy.mounts` → workload scope (`:443-456`).
- Mount-entry scopes (`:457-483`): mount rows are wholesale-replaced (ADR 0020
  Ruling 1), so only the FINAL layer that declares this workload's `mounts`
  contributes entry policies. The collector `retain`s non-`MountEntry` scopes
  then appends one `MountEntry` scope per mount row that has a `policy`,
  tagged with `for_mount(mount.guest)` (`:466-481`).

**`[policy.mounts]` surface** (`config/types.rs:602-609`): `PolicyConfig {
mounts: Option<MountsFragment> }` — present on both `Config` (global) and
`WorkloadConfig`. The scaffold template (`scaffold/template/workestrate.toml.tpl:77-85`)
ships a commented example: `mask = ["**/.env", "**/.ssh/**"]`, `unmask =
["**/.env.example"]`, `protect = ["**/.secret"]`, `[policy.mounts.writes] deny =
["**/.secret/**"]`.

**Validation** (`config/validation.rs:377-399`): per workload, per mount —
`policy_file.is_some()` → bail ("runtime-resolved field; cannot be set in
configuration", `:380-386`); `validate_mount_host` + `validate_mount_guest`;
duplicate guest path → bail ("per-mount policy files are keyed by guest slug",
`:393-398`).

**Sensitive defaults**: the reference config ships overridable masks for
sensitive files (`.env`, `.ssh`, `.secret`) as the lowest-authority
`ReferenceConfig` layer — operators/workloads can unmask carve-outs.

### 16.8 Diagnostics CLI (`commands/policy.rs`)

`workestrate policy mounts explain --workload X --mount /ws --path apps/.env`
(`cmd_explain`, `:146-244`): constructs the workload via
`new_with_use_overrides` (the SAME compile path as runtime), resolves `--mount`
to the `MountPlan` then `policy_for_mount(&wl, &m.guest)` → `mount_policy_for`
(per-mount isolation, `:118-123`), builds `LexicalPath::new(path)`, calls
`program.decide` + `program.decide_write`. Prints the full match trace: each
match's `rule_index`, `pattern`, `effect`, `terminal`, `frozen_out`, `origin`;
`frozen_by`; `fail_closed_non_utf8`; per-parent `may_unmask_descendant`; final
decision label (`visible`/`masked`/`traversal_only`/`protected`,
`decision_label` `:90-99`); write verdict (`allow`/`deny`). `--json` emits an
`ExplainJson` envelope (`:161-199`).

`workestrate policy mounts preview --workload X --mount /ws [--root <dir>]`
(`cmd_preview`, `:268-345`): resolves the mount host root, walks the tree with
`std::fs` recursion (`walk_tree`, `:347-404`) — entries `sort_by_key(file_name)`
for deterministic output. Per entry: `decide` + `decide_write` + `is_protected`;
annotate `[visible|masked|traversal_only|protected]` + `[write-deny]`; prune a
dir when `protected || (Masked && !may_unmask_descendant)` (`:381-383`) and do
not recurse into pruned dirs. Hardlink alias detection (`:293-302`,
`masked_inodes` `:406-416`): collect `(dev, ino)` for visible entries, then for
each masked entry check whether its `(dev, ino)` matches a visible one →
`hardlink alias` warning. `--json` emits a `PreviewJson` envelope with `tree`
and `hardlink_warnings`.

Exit codes: `Ok(())` → 0 for answers (including "masked"); `Err` → 1 for
compile/resolution errors (workload not found, mount not found, compile error,
root missing). Integration tests: `tests/cmd_policy.rs` (434 lines, 10 tests)
cover explain/preview/per-mount isolation/hardlink warning/JSON shape.

## 17. Cross-repo architectural relationship

End-to-end flow from workestrate config to msb enforcement:

```
 workestrate (control/agentctl)                  │ WIRE BOUNDARY (JSON, host state-dir)
 ─────────────────────────────────────────────   │ ─────────────────────────────────────
 [policy.mounts] fragments (6 scopes)            │
        │ collect_policy_scopes (loading.rs)     │
        ▼                                        │
 CollectedPolicy { global, workloads }           │
        │ new_with_use_overrides (config.rs)     │  per-mount: global + workload + this mount's entries
        ▼                                        │
 compile(scopes) → MountPolicyProgram  (×N mounts)│
        │ write_policy_file (policy_file.rs)     │  atomic tmp+rename, 0600, policy/<instance>/<slug>.json
        ▼                                        │
 ┌─────────────────── HOST STATE DIR ───────────┼──►  policy/<instance>/<slug>.json
 │  (host filesystem, approved_root)             │
 └───────────────────────────────────────────────┼──►  SDK mount model: MountBuilder::mount_policy(path)
        │ apply_mount_policy (mounts.rs) ◄ NO-OP SEAM (T4 dormant)        │
        ▼                                        │   mount-spec token: policy=<path>  (spawn.rs:1971-1972)
 ┌─────────────────── HOST BOUNDARY ─────────────┼──►  msb runtime --mount tag:host:opts,policy=<relpath>
 │  vm.rs parse_mount_spec (:2028) → ParsedMountSpec.policy_path          │
 │        │ load_mount_policy (vm.rs:1900) ← approved_root confinement    │
 │        ▼  O_NOFOLLOW+O_DIRECTORY per-component, parse-once, version:1  │
 │  MountPolicyProgram (immutable, mount lifetime)                        │
 │        │ PassthroughConfig → PassthroughFs (one per bind)              │
 │        ▼                                                                │
 │ ┌─────────────────── GUEST BOUNDARY ────────────────────────────────── │
 │ │  PassthroughFs op hooks (do_lookup/do_open/...) evaluate policy      │
 │ │  per lexical path; tag store pins written masked inodes; cascade     │
 │ │  cleans untagged-masked on rmdir; write-deny = global write ACL      │
 │ └───────────────────────────────────────────────────────────────────── │
 └─────────────────────────────────────────────────────────────────────────
```

**Wire boundary**: the JSON file under the host state dir. `MountPolicyProgramWire`
is byte-aligned across the two repos (§3). Workestrate writes it; msb reads it
once and never re-reads.

**Host boundary**: the approved runtime state dir. The policy file lives there;
the SDK mount-spec carries a relative `policy=<path>` token; msb's
`load_mount_policy` confines the path to `approved_root` with per-component
`O_NOFOLLOW`+`O_DIRECTORY` (§15.8). The guest never sees the policy file.

**Guest boundary**: the PassthroughFs op hooks. Every guest-visible operation
(lookup, readdir, open, write, create, unlink, rmdir, rename) is gated at the
guest boundary by evaluating the lexical path against the mount's program. The
tag store and cascade are internal to PassthroughFs.

**Why per-mount files, not per-instance**: PassthroughFs is constructed ONCE
per bind mount with ONE immutable program. The decision is evaluated per mount
root — a path under `/workspace` is decided by `/workspace`'s program, not by
`/data`'s. A single per-instance file would force one program for all mounts,
reintroducing the cross-mount rule leakage bug (§16.3). The per-mount partition
(`<instance>/<slug>.json`) gives each bind its own program while keeping the
per-instance subdir for clean lifecycle cleanup (`remove_policy_dir`).

## 18. How to assess each branch

A contextless session can verify the work by following these steps in order.
All `cargo` commands require the nix devshell:
`/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin/nix develop -c bash -c '...'`.

### 18.1 To assess the msb branch (`feat/passthrough-mount-path-policy` @ ccceb48a)

1. **Read the policy types**: `crates/filesystem/lib/backends/passthroughfs/unix/mount_policy/{rule,pattern,program,lexical,mod}.rs`.
   Confirm every field of `MountPolicyProgramWire` (program.rs:136-144), the
   version fail-closed Deserialize (program.rs:408-434), `ScopeKind` 6 variants
   (rule.rs:23-38), `RuleOrigin` (rule.rs:41-49), `Pattern` dialect (pattern.rs:106-154).
2. **Read the tag store**: `tag_store.rs` end-to-end. Confirm `TagKey=(u64,Vec<u8>)`,
   `TagValue{file, alt_key}`, capacity 10_000, the 7 `.lock().unwrap()` sites
   (§15.4), the O_PATH pin in `tag()` (tag_store.rs:81-113), the exhaustion
   counter.
3. **Trace a single enforcement — masked-untagged lookup**: start at `do_lookup`
   (inode.rs:220) → `lexical_child_path` → `is_protected`/`decide` gate →
   `tagged_visible` false → ENOENT (inode.rs:236-239). Then trace a masked-tagged
   write: `do_open` (file_ops.rs:37) → write_intent → `decide_write` →
   `decide==Masked` → open → `fs.tag_inode(inode)` (file_ops.rs:152-154).
4. **Trace the cascade**: `do_rmdir` ENOTEMPTY (remove_ops.rs:264-303) →
   `cascade_remove` (remove_ops.rs:338) → `cascade_remove_fd` (remove_ops.rs:394)
   → per-child `fstatat`+`decide` → untagged-masked only → recurse/unlink →
   `tags.evict`.
5. **Trace the loader**: `vm.rs:1900-2019` (`load_mount_policy`) →
   `approved_root` confinement → per-component `O_NOFOLLOW`+`O_DIRECTORY` →
   `ELOOP`→`SymlinkRejected`, `ENOENT`→`FileNotFound` → `serde_json::from_slice`
   → version Deserialize reject None/≠1.
6. **Trace the wire**: `packages/microsandbox-types/rust/lib/domain.rs:1278-1300`
   (hand-serde for `VolumeMount::Bind.mount_policy`), `sdk/rust/lib/sandbox/types.rs:313`
   (`MountBuilder::mount_policy`), `sdk/rust/lib/runtime/spawn.rs:1971-1972`
   (mount-spec encoder `policy={path}`), `crates/runtime/lib/vm.rs:2028-2145`
   (`parse_mount_spec` parsing the `policy=` token).
7. **Run the gates**:
   `cd microsandbox-mount-policy; nix develop -c bash -c 'cargo test -p microsandbox-filesystem'`
   (expect 664/664 pass); `cargo clippy -p microsandbox-filesystem --all-targets -- -D warnings`;
   `cargo fmt -p microsandbox-filesystem -- --check`.
8. **Verify isolation**: `git -C microsandbox-mount-policy diff d9b4d12e..HEAD --stat`
   — confirm all changes are filesystem/runtime/sdk/types/docs.
9. **Verify no workestrate leakage**: `grep -ri 'workestrate\|workstrate\|georgryb' microsandbox-mount-policy/crates/ microsandbox-mount-policy/sdk/ microsandbox-mount-policy/packages/` (production code) — expect zero hits.
10. **Verify the test fixture rename**: `grep -rn '\.workestrate' microsandbox-mount-policy/crates/filesystem/lib/backends/passthroughfs/unix/tests/` — expect zero hits (post e9308da0).

### 18.2 To assess the workestrate branch (`experimental` @ 56e2684)

1. **Read the design**: spec 22 (`docs/validation-and-improvements/06-improvements/22-dynamic-mount-masking-policy.md`) + ADR 0028 (`docs/migration/50-decisions/0028-policy-scopes-collect-and-compile.md`) + spec 23 (`06-improvements/23-microsandbox-fork-nix-flake-packaging.md`).
2. **Read the library**: `control/agentctl/src/mount_policy/{pattern,value,lexical,rule,scope,compile,program,mod}.rs` — pure, no I/O. Confirm `PolicyValue` visitor (value.rs:101-165), `ScopeKind` 6 variants (scope.rs:17-33), `compile` authority sort + trust + duplicate (compile.rs:120-245), `decide`/`decide_write`/`may_unmask_descendant` (program.rs:195-427).
3. **Read the config wiring**: `config/loading.rs:407-486` (scope collection), `config/types.rs:602-609` (`[policy.mounts]` surface), `config/validation.rs:377-399` (trust rules + duplicate-guest + policy_file rejection).
4. **Read the per-mount compilation**: `microsandbox/workload/config.rs:70-150` (`new_with_use_overrides`) + `mount_policy/scope.rs:193-196` (`PolicyScope.mount_guest`). Confirm the per-mount filter at config.rs:94-97.
5. **Read the file lifecycle**: `microsandbox/policy_file.rs` (write/remove, atomic, 0600 pre-rename, per-instance subdir, `mount_slug`) + `plan.rs:110-123` (`MountPlan.policy_file`) + `mounts.rs:61-75` (`apply_mount_policy` no-op seam + TODO).
6. **Read the CLI**: `commands/policy.rs` (`cmd_explain` `:146-244`, `cmd_preview` `:268-345`, `walk_tree` `:347-404`) + `tests/cmd_policy.rs` (integration, 434 lines).
7. **Read the defaults**: `scaffold/template/workestrate.toml.tpl:77-85` (commented `[policy.mounts]` example).
8. **Run the gates**: `cd workestrate-clones/experimental; nix develop -c bash -c 'cargo test'` (expect 565+ pass in lib, 10/10 in cmd_policy); `cargo clippy --all-targets -- -D warnings`; `cargo fmt --check`; if `just` is in the devshell: `just verify`.
9. **Verify the cross-repo wire contract**: open both `mount_policy/program.rs` files side-by-side — msb at `microsandbox-mount-policy/crates/filesystem/lib/backends/passthroughfs/unix/mount_policy/program.rs` and workestrate at `workestrate-clones/experimental/control/agentctl/src/mount_policy/program.rs`. Every field/sub-struct of `MountPolicyProgramWire` must match (§3). The only intentional asymmetry is workestrate's `RuleMatch.pattern: String` (non-wire, explain-trace only).
10. **Try the diagnostics**: with a workspace built, run `workestrate --home <dev-home> policy mounts explain --workload <wl> --mount /workspace --path .env` and `... preview --root <dir>`. Verify per-mount isolation: if a workload has two mounts with different entry policies, paths masked by one mount's rules must NOT be masked by the other mount's rules (this is the 6fa722c + 35afdad fix).

### 18.3 To verify the two-repo contract end-to-end

1. Build both repos in their devshells (`nix develop`).
2. Generate a sample policy JSON from workestrate (a workload with a policy-bearing mount; inspect `policy/<instance>/<slug>.json` in the state dir).
3. Hand-craft the same JSON in a file under an approved msb state dir.
4. **(HOST-KVM-only)** Boot msb with the policy-bearing mount (`--mount tag:host:...,policy=<relpath>`).
5. **(HOST-KVM-only)** Verify enforcement: masked paths return ENOENT on lookup and are omitted from readdir; write-deny paths return EACCES on open-for-write/unlink/rmdir/rename-dst; `rm -rf` on a masked dir cascades untagged-masked descendants; identity revalidation holds across a rename (rename a tagged file → the tag evicts → the old name is ENOENT).

End-to-end runtime enforcement is **not yet exercisable in this container** (no
KVM; the SDK seam is a no-op). Steps 4–5 require the user's host with KVM and
the SDK switch (T3+T4) landed.

## 19. What to verify vs what is already verified

| Locked decision | Enforced where | Verifying gate | Status |
|---|---|---|---|
| Collect-not-merge (ADR 0028) | workestrate `loading.rs:407-486` (collect), `compile.rs` (compile) | `cargo test` mount_policy + config | **verified** (unit) |
| Six-scope authority order | `scope.rs:17-33`, `compile.rs:122` | `compile.rs` test `authority_order_is_compile_order_operator_first` | **verified** (unit) |
| PolicyValue visitor (no untagged) | `value.rs:101-165` | `value.rs` tests | **verified** (unit) |
| Terminal-unmask trust (operator-only) | `compile.rs:202-216` | `compile.rs` test `terminal_unmask_from_a_non_operator_scope_is_a_compile_error` | **verified** (unit) |
| Terminal-protect operator-only | `compile.rs:180-187` | `compile.rs` test `terminal_protect_is_operator_only` | **verified** (unit) |
| Exact-duplicate terminal conflict | `compile.rs:218-242` | `compile.rs` test `exact_duplicate_same_scope_terminal_mask_unmask_is_a_compile_error` | **verified** (unit) |
| Pattern validation (abs/NUL/`..`/empty) | `pattern.rs:106-154` | `compile.rs` test `pattern_errors_are_compile_errors_naming_the_origin` | **verified** (unit) |
| Non-UTF-8 fail-closed | `lexical.rs:73-82`, `program.rs:172-179,252-258` | `program.rs` test `non_utf8_paths_are_masked_fail_closed` | **verified** (unit) |
| Case sensitivity applied at compile (msb) | msb `program.rs:151-163` (6e51840b) | msb `cargo test -p microsandbox-filesystem` | **verified** (unit) |
| Mask = read boundary (lookup ENOENT, readdir omit) | msb `inode.rs:220-249`, `dir_ops.rs:296-326` | msb `test_mount_policy.rs` (11) | **verified** (unit) |
| Write-tagging (masked write → visible) | msb `file_ops.rs:152-154,264-266`, `tag_store.rs` | msb `test_mutation_policy.rs` (33) | **verified** (unit) |
| writes.deny global write ACL (M3/M4 hoisted) | msb `file_ops.rs:60-67,244-249`, `create_ops.rs:532-538`, `remove_ops.rs:820-824,931-935` | msb `test_mutation_policy.rs` | **verified** (unit) |
| Protect tier (untouchable, terminal operator-only) | msb `program.rs:180-206`, `inode.rs:230-231`; workestrate `compile.rs:180-187` | both unit suites | **verified** (unit) |
| Tag identity revalidation | msb `inode.rs:340-363`, `file_ops.rs:110-149` | msb `test_mutation_policy.rs` | **verified** (unit) |
| Cascade (untagged-masked only, depth 64, no symlink follow) | msb `remove_ops.rs:338-569` | msb `test_mutation_policy.rs` | **verified** (unit) |
| Rename anti-laundering (masked source → ENOENT) | msb `remove_ops.rs:904-927` | msb `test_mutation_policy.rs` | **verified** (unit) |
| RENAME_EXCHANGE identity check | msb `remove_ops.rs:681-693` | msb `test_mutation_policy.rs` | **verified** (unit) |
| JSON v1 + malformed fail-closed | workestrate `program.rs:156-178`; msb `program.rs:408-434` + `vm.rs:1900-2019` | workestrate `program.rs` test `program_version_is_required...`; msb loader tests | **verified** (unit, both sides) |
| Per-mount compilation (no cross-mount leakage) | workestrate `config.rs:81-114` (6fa722c) | `config.rs` test `cw-per-mount-policy` (PER_MOUNT_POLICY_TOML) | **verified** (unit) |
| Per-mount explain/preview selection | workestrate `commands/policy.rs:118-123,156` (35afdad) | `tests/cmd_policy.rs` | **verified** (integration) |
| Duplicate-guest-path rejection | workestrate `validation.rs:393-398` | `validation.rs` test `validate_config_rejects_duplicate_mount_guest_path` | **verified** (unit) |
| User-supplied policy_file rejection | workestrate `validation.rs:380-386` (0c18bc2) | `validation.rs` tests | **verified** (unit) |
| Hardlink alias warning (preview) | workestrate `commands/policy.rs:293-302` | `tests/cmd_policy.rs` | **verified** (integration) |
| Fail-closed loader (approved_root, O_NOFOLLOW, version) | msb `vm.rs:1900-2019` | msb `cargo test -p microsandbox-runtime` (loader unit tests) | **partially verified** (unit; not host-KVM) |
| Wire byte-alignment (cross-repo) | both `MountPolicyProgramWire` | manual side-by-side (§18.2 step 9) | **partially verified** (manual; no shared golden test — T12) |
| End-to-end runtime enforcement | msb PassthroughFs in a live VM | host-KVM smoke (spec 22 §14) | **not verified** (blocked on T3+T4 SDK switch + KVM) |
| SDK seam (apply_mount_policy real call) | workestrate `mounts.rs:67-75` | — | **not verified** (dormant no-op; T4) |

## 20. Known gaps and follow-ups (expanded from §6 T1–T16)

| ID | What | Where | Effort | Dependency | Who |
|---|---|---|---|---|---|
| T1 | Sign MSB mount-masking commits (`%G? = N → G`) | msb branch | small | GPG key on host | user (host) |
| T2 | Rebase MSB mount-masking onto fork `4a3133e5` lineage (NOT `d9b4d12e`) | msb branch | small | T1 | user (host) |
| T3 | Spec 23: package microsandbox fork as nix flake output (encapsulated source-build; one pinned input) | workestrate flake + fork | large | — | user (host, HOST-NIX) |
| T4 | Flip `apply_mount_policy` from no-op seam to real per-mount SDK call: `builder.volume(&m.guest, \|v\| v.bind(host).mount_policy(m.policy_file))` | `mounts.rs:61-75` | small | T3 | dev |
| T5 | `cargo clippy --workspace -- -D warnings` + `cargo fmt --check` on MSB branch | msb | small | — | dev (verifiable-here) |
| T6 | W5 dead-code sweep in workestrate `mount_policy` | workestrate | small | — | dev |
| T7 | W7 author mount-masking operator guide | docs | medium | — | dev/docs |
| T8 | W9 add mount-masking config examples | docs/scaffold | small | — | dev/docs |
| T9 | W10 refresh STATUS.md §0.0 with §9/§12 landings | docs | small | — | dev/docs |
| T10 | Host-KVM runtime smoke for spec 22 §14 enforcement | msb VM | medium | T3+T4 | user (host, HOST-KVM) |
| T11 | Open upstream MSB PR (force-push fork `fix/filesystem-agentd-path-override` @ 6d7b52ea) | upstream | small | T1,T2 | user (host) |
| T12 | Cross-repo evaluator duplication/drift — golden decision-vector test pinned in both repos, or extract a shared crate | both `mount_policy/program.rs` | medium | — | dev |
| T13 | `tag_store` poison handling — propagate or document panic-on-poison + fd-leak contract (7 `.lock().unwrap()` sites in tag_store.rs; risk: 10k O_PATH fd leak on poison) | msb `tag_store.rs` | small | — | dev |
| T14 | `CaseSensitivity::Insensitive` reserved vs applied alignment (workestrate never emits; msb applies at compile) | both | small | — | dev |
| T15 | `MountPlan.policy_file` wart (runtime-resolved `Option<PathBuf>` on the plan struct; never user-set; validation rejects) | workestrate `plan.rs:122` | small | — | dev |
| T16 | Quota byte accounting still counts masked bytes (masking ≠ space control) | msb quota | small | — | dev |
