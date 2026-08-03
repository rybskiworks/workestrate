# MOUNT-MASKING HANDOVER — dynamic mount masking policy (spec 22)

> **STATUS: HANDOVER (2026-08-03, post-spec-23 docs; pre-SDK-switch; pre-host-KVM smoke)**
> Cross-references: STATUS.md, NEXT-SESSION.md, BEADS.md, spec 22 (06-improvements/22-dynamic-mount-masking-policy.md), spec 23 (06-improvements/23-microsandbox-fork-nix-flake-packaging.md), ADR 0028 (50-decisions/0028-policy-scopes-collect-and-compile.md).
> Prerequisites: see-also — README.md, 06-improvements/00-index.md.

## Environment markers (same table as other specs in this tree)
| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, cargo-linked gates via `nix develop`). |
| `HOST-NIX` | Requires nix on the user's host for genuine host gates. |
| `HOST-KVM` | Requires KVM on the user's host. |

## TL;DR (≤6 lines)
The dynamic mount-masking feature is implemented on two branches not yet wired together. msb side: feat/passthrough-mount-path-policy @ ccceb48a (32 commits since d9b4d12e) — full PassthroughFs enforcement: policy core, mutation policy + tag store, cascade, write-deny as global write ACL, SDK/runtime threading, docs, tests. workestrate side: experimental @ 0c18bc2 (14 commits since 43cfcbc) — compiler library, hierarchical scope collection, config wiring, per-mount policy_file lifecycle, sensitive defaults, validation, diagnostics CLI. The SDK seam (apply_mount_policy) is a no-op pending the fork dep switch (spec 23, deferred). Cross-repo wire contract byte-aligned (MountPolicyProgramWire identical). End-to-end runtime enforcement unexercised (no KVM in container).

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
- HEAD: 0c18bc2
- Base: 43cfcbc (docs(specs): 21 image build/load lifecycle)
- Author: Georg Rybski
- Signed: ALL 14 COMMITS UNSIGNED
- Working tree: clean
- 14 commits in 43cfcbc..0c18bc2 (use this exact commit list):
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

1. **HEAD drift**: msb @ ccceb48a (NOT e5bf48fa which is commit #8 mid-series); workestrate @ 0c18bc2 (NOT 22856b5 which is commit #12). Use `d9b4d12e..HEAD` (msb, 32 commits) and `43cfcbc..HEAD` (workestrate, 14 commits) as the true ranges.
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
