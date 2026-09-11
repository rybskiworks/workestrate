# Testing — how to verify mount policy

> **What this doc teaches:** the unit gates, the smoke check, the host e2e,
> and how to decode failures. **Read first:**
> [01-runtime-semantics.md](./01-runtime-semantics.md) — you cannot read a
> failure without knowing the expected errnos.

## Unit gates

**Fork (runtime + enforcement):** `cargo test --workspace` in
`/home/node/Development/agent-workbench/microsandbox-mount-policy` — 671 tests
plus clippy/fmt green at the last full gate round (2026-08-20). The two policy
test files live under
`crates/filesystem/lib/backends/passthroughfs/unix/tests/`:

| File | Covers |
|---|---|
| `test_mount_policy.rs` | read-axis evaluation (Visible/Masked/TraversalOnly), protect short-circuit, terminal freeze + `frozen_out`, write-axis union semantics, wire-format version/unknown-field gating, case sensitivity, no-policy byte-identical behavior |
| `test_mutation_policy.rs` | event-level enforcement: lookup/readdir/read-open/create/write/unlink/rmdir/rename errnos per state (masked-untagged, masked-tagged, write-denied, protected), tag lifecycle, cascade delete, fail-closed leftovers |

**Tool (config + compiler + writer + CLI):** `just verify` in the workestrate
repo — green at the last gate round EXCEPT `schema-sync-check` inside the
container: the host's `~/.workestrate` is bind-mounted read-only there, so the
check needs `WORKESTRATE_HOME=../workestrate-dev-home` to pass in-container
(the host-side refresh is a host step).

## Smoke check: mount-mask-auth

The personal config-repo flake's prime smoke carries a `mount-mask-auth`
check (`workestrate-dev-home/config-repos/personal/flake.nix:947-980`):

- **What it asserts:** in-guest `cat /data/agent/auth.json` FAILS and
  `ls /data/agent/` omits the name — the prime capsule masks
  `agent/auth.json` on the `/data` mount.
- **SKIP precondition:** if the host file
  `state/workspaces/prime-state/agent/auth.json` is absent, the check SKIPs —
  otherwise an absent file would produce a false PASS (cat fails regardless).
- **Tag-fragility caveat (in the check's own comment):** a green result only
  proves the HOST ORIGINAL is sealed at probe time. Masking is a passive seal:
  a guest write-open would adopt+tag the host file, and rename-exchange or
  cascade removal can evict tags. Green does not mean the name is immutable
  guest-side.
- **Gating:** HOST-KVM gated like the rest of the smoke; pending host e2e.

## Host e2e: E6

The E6 end-to-end (KVM-gated, not yet run) uses a scratch capsule plus an
eight-row expectation table, authored in
`handovers/2026-08-13-mount-merge-readiness.md` §9c.4 and corrected 2026-08-20
to the unified vocabulary (workload-scope FINAL `write.deny` expresses
"visible but untouchable"; true protect posture is an operator-scope final
`read.deny`). Summary of the expectation rows:

| # | Probe (in-guest) | Expected |
|---|---|---|
| 1 | `ls /data` | `secrets` omitted from the listing |
| 2 | `ls /data/secrets` | `ENOENT` (dir hidden) |
| 3 | `cat /data/guard/keep.txt` | readable — final `write.deny` denies WRITES only |
| 4 | `touch /data/guard/new` | `EACCES`; the `final` freezes the deny against later scopes |
| 5 | append to `/data/logs/app.log` | `EACCES` (write.deny) |
| 6 | `cat /data/logs/app.log` | readable |
| 7 | `ls $MSB_HOME/mount-policy/` | per-mount `<slug>.json` present (MSB_HOME-anchored approved root, not the workestrate state dir) |
| 8 | `workload down mount-policy-e2e` | exits 0; instance removed |

## Future hardening tests (auth.json seal levels)

The prime capsule currently ships the PASSIVE mask on `agent/auth.json`
(non-final `read.deny`; user decision 2026-08-24 — see the smoke's
tag-fragility caveat above for what a passive seal does NOT prove). Whether
to seal harder (`write.deny`, or an operator-scope final `read.deny`) is
DEFERRED until this test plan runs. The runtime's canonical behavior source
is the fork's `test_mutation_policy.rs` (§ Unit gates) — these tests prove
the semantics end-to-end against real capsules; the unit table already
defines the expected errnos per state.

- **(a) `write.deny` tolerance boot.** Add a non-final `write.deny` on the
  auth.json entry and boot prime: does prime / the prime agent TOLERATE
  `EACCES` on an auth.json write (start cleanly, stay usable), and does it
  PERSIST auth state across sessions without that write path? The seal
  level is only adoptable if both answers are yes.
- **(b) Adoption-clobber probe.** With the passive mask: a guest
  write-open of the masked host file adopts + tags it and can clobber —
  prove the clobber is VISIBLE host-side (host file content/mtime after a
  guest write-open + write), so the failure mode of the current posture is
  documented, not theoretical.
- **(c) Final-seal non-overridability.** An operator-scope FINAL `read.deny`
  on the entry: verify a config-level (repo/workload) `read.allow` naming
  the same pattern is REJECTED at compile time (the trust gate — final
  allows are operator-only, see
  [03-hierarchy-and-precedence.md](./03-hierarchy-and-precedence.md)).
- **(d) Anti-laundering rename-out.** Renaming the masked path away (or
  into it) stays `ENOENT` — the name stays sealed, not just the content;
  a rename cannot launder the mask into visibility.
- **(e) Guest-initiated probe matrix.** An agent probing from INSIDE the
  sandbox against the masked path: create-over, write-open, rename-out,
  unlink, setattr, fallocate — record the errno per op and check the table
  against `test_mutation_policy.rs` expectations (§ Decoding errnos).
- **(f) Traversal-only with carve-outs.** Mask a parent dir with an
  unmasked carve-out beneath it: the parent lists without the name, the
  ancestor walks as `TraversalOnly`, and the carve-out stays reachable —
  the pattern a harder auth.json seal would use if a sibling path must
  remain guest-visible.

## Reading failures

### Decoding errnos

| Errno | Implies |
|---|---|
| `ENOENT` on lookup / read-open / unlink / rename-source | the path is masked (read.deny matched or protect matched) and NOT tagged; on remove-class ops, invisibility answers before write-deny |
| name missing from `ls` but parent walkable to a deeper unmasked path | `TraversalOnly` ancestor — expected when a carve-out lives beneath a masked dir |
| `EACCES` on create / write-open / write / fallocate / setattr / truncate | a write.deny matched a VISIBLE path — or the path is protected (protect denies writes but still answers `ENOENT` on remove-class and lookup) |
| `ENOTEMPTY` on rmdir of a masked tree | cascade found a tagged, protected, or visible leftover — fail-closed by design, no data loss |

### Using `explain`

```sh
workestrate policy mounts explain --workload W --mount M --path P
```

prints the decision, the write verdict, `frozen_by` (the terminal rule's
origin, or none), and every matching rule in compile order with provenance —
including matches flagged `frozen_out` that a terminal rule neutralized. A
protected path is labeled `Protected` (`control/agentctl/src/commands/policy.rs:103`).
`workestrate policy mounts preview --root DIR` walks a host tree and prints
the decision + write verdict per path. Both subcommands reuse the same
compiler and mirror evaluator as runtime (the mirror intentionally matches the
pinned runtime — see
[04-compiler-and-wire.md](./04-compiler-and-wire.md#the-mirror-evaluator)), so
what `explain` says is what the guest experiences at the current pin —
including the pinned `write.allow`-inert behavior.
