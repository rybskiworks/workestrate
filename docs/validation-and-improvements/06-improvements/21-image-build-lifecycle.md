# 21 — Image build/load lifecycle: ensure-images pre-flight, change detection, selectors, reserved build dir

> **STATUS: IMPLEMENTING — phases A–D LANDED (commits 02bea9a / 290e91b / 0729bb4 / 932b476); phase E IMPLEMENTED in the working tree but UNCOMMITTED + UNVALIDATED (2026-08-03); phase F (multi-repo migration) PENDING. User signed off on decisions D1–D5 2026-08-02.**
> Prerequisites / see-also: [00-index.md](00-index.md) ·
> [17-config-repo-directory-mode.md](17-config-repo-directory-mode.md) ·
> [11-home-provisioning-and-lockfile.md](11-home-provisioning-and-lockfile.md) ·
> [12-per-instance-addressing.md](12-per-instance-addressing.md) ·
> [01-mount-filtering-shadowing.md](01-mount-filtering-shadowing.md) ·
> [07-naming-consistency.md](07-naming-consistency.md) ·
> [../../migration/50-decisions/0021-instance-lifecycle-model.md](../../migration/50-decisions/0021-instance-lifecycle-model.md) ·
> [../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md](../../migration/50-decisions/0026-per-instance-addressing-and-discovery.md)

## Environment markers

| Marker | Meaning |
|---|---|
| `verifiable-here` | Can be validated in this container (TOML, golden files, git, shell/python scripts, AND cargo-linked gates via `nix develop` — nix at `/nix/store/q6yfdws28aj556jlz5yayaggiddmb0b5-nix-2.35.1/bin`, not on PATH; the devshell provides a full C toolchain, verified 2026-07-29) |
| `HOST-NIX` | Requires nix on the user's host for the genuine host gates only: `nix build` image builds, `nix run nixpkgs#...` FOD prefetch, `just verify-full`, `just generate-schema` |
| `HOST-KVM` | Requires KVM on the user's host (this container has no `/dev/kvm`) |

This spec is docs-only (`verifiable-here`); its implementation phases carry the
per-phase tags given in §10.

---

## Summary

Custom nix-layered images (`workestrate-pi:latest`, `tempest:latest`) are built
and loaded by a manual out-of-band ritual in the config repo (`nix build` +
`msb load` from a justfile recipe), while the CLI blindly consumes whatever the
msb store holds — with no freshness check, no reserved artifact location, and
no collision policy across config repos or homes. This spec moves the build/
load lifecycle into the tool: a parent-side **ensure-images pre-flight** that
runs before any spawn, **drvPath-based change detection** against a per-home
record at `$WORKESTRATE_HOME/state/images.json`, a **`workestrate workload
build`** verb family with context/repo selectors plus a `--reload-images`
force flag on the lifecycle verbs, and a reserved **`.workestrate-build/`**
directory at each config-repo root. Five user decisions are locked:

- **D1** — record-absent + tag-present → TRUST the store tag on plain `up`
  (rebuild only via `--reload-images`); records are advisory, never
  authoritative.
- **D2** — stable tags verbatim from config TOML; no auto-prefixing;
  cross-config collision → `validate-config`/`plan` WARN naming both repos;
  manual TOML prefixing is the operator remedy.
- **D3** — `--reload-images` batch scope = force ALL service workloads in the
  batch; never forwarded in `detach_args`.
- **D4** — `.workestrate-build/` reserved at config-repo root — gitignored,
  artifact-only by construction, scaffold-provisioned, new default for
  undeclared `local_build` fallbacks.
- **D5** — cross-home collisions → warn + digest-detect; state records keyed
  by (config-repo identity, name:tag).

Each decision is marked inline at the place it governs with `(USER DECISION
Dn)`. Effort: phased **A–F** (S/M/M/M/M/S–M); phases A/B are
`verifiable-here`, C/D/F are `HOST-NIX`, E is `HOST-KVM`.

---

## 1. Problem

Today the image lifecycle is split across three unconnected systems:

1. **Manual build/load ritual.** The custom nix-layered images
   (`workestrate-pi:latest`, `tempest:latest`) are built and loaded by an
   out-of-band recipe in the config repo's justfile: `nix build` the image
   attr, then `msb load` the resulting tarball. The operator must remember to
   run it, and must know when to run it.
2. **Blind consumption.** The CLI consumes whatever the msb image store holds.
   msb's `PullPolicy::IfMissing` checks PRESENCE only — if the tag exists, it
   is used as-is. There is NO freshness check: edit the flake, forget to
   re-load, and the tag silently serves old bits. Stale-tag drift is invisible
   until behavior diverges.
3. **Global store, no isolation.** The msb image store is global per OS user
   (`~/.microsandbox`). Two homes, or two config repos in one home, that
   declare the same `name:tag` collide on one store entry with no detection
   and no provenance record of who loaded it.
4. **No reserved artifact location.** Config repos have no workload-agnostic
   place for build artifacts. Undeclared `local_build` fallbacks have no
   canonical home; ad-hoc output dirs multiply.

The result: the freshness of a running sandbox's rootfs depends on operator
memory, and two correctness hazards (stale tags, cross-home tag collisions)
are silent.

---

## 2. ensure-images pre-flight

### 2.1 Where it runs — parent-side only, never in the detached child

ensure-images runs on the **parent** side of `workload up`, `workload exec`,
and `workload batch-up`, before any spawn, and is inherited by dependency
auto-start (a dependency that is auto-started gets the same pre-flight first).

It must NEVER run in the detached child. Two mechanisms make child-side
builds guaranteed-invisible timeouts:

- `spawn.rs` redirects the detached child's stdout/stderr to the sandbox log —
  a multi-minute `nix build` + `msb load` produces zero operator-visible
  output.
- The FS-8 500ms grace window and the dependency-readiness `DEFAULT_WAIT`
  (15s) bound how long anyone waits on the child; a build that takes minutes
  always loses that race and surfaces as an opaque readiness timeout.

### 2.2 Parent/child handoff — the `images_ready` token + `--images-ready`

The parent/child split mirrors the `--no-deps` spec-riding precedent of spec
12 / the ADR 0026 addendum: a hidden flag rides the re-exec so the child knows
the parent already handled a lifecycle concern.

- The parent sets an **`images_ready`** token on `InstanceSpec` before
  detaching.
- `detach_args` appends a hidden **`--images-ready`** flag to the child's
  re-exec argv.
- The child sees `--images-ready` (equivalently, `images_ready` on its
  reconstituted `InstanceSpec`) and SKIPS ensure-images entirely.
- `--foreground` up carries no token and no flag → the foreground process IS
  the parent → it ensures.

### 2.3 Eligibility — nix-layered only

A workload is eligible for ensure-images iff **`image.recipe ==
"nix-layered"`** — the same gating predicate the `flake_root_requirement`
uses (a nix-layered image is exactly the workload class that needs a flake
root). Registry-image and `local_build`-only workloads are **no-ops**: nothing
to build, nothing to check.

### 2.4 Ordering — ensure before dependency auto-start

On `up`/`exec`, ensure the NAMED workload's images BEFORE
`auto_start_dependencies` runs: fail fast on the workload the operator
actually asked for before spending minutes starting its dependency closure.
Dependency auto-start then inherits the same pre-flight per dependency (§2.1).

---

## 3. Change detection

### 3.1 The signal — eval-only drvPath

Change detection is:

```
nix eval --raw <repo>#<image.name>.drvPath
```

Properties that make drvPath the right signal:

- **Eval-only.** No build is triggered; the eval is cheap and works with
  fakeHash placeholders (a placeholder hash changes neither the derivation
  structure nor its drvPath).
- **Precise for config edits.** TOML edits outside `image.*` do not perturb
  the image derivation → no false positive from unrelated config churn.
- **Conservatively noisy for tool churn.** tool-lib / nixpkgs input churn
  perturbs the drvPath even when the final outPath is identical — a
  conservative false positive, bounded by nix-store dedup (the rebuild
  short-circuits when the outPath already exists in the store).
- **Re-load gate.** Only re-load into msb when the **outPath differs** from
  the recorded one — eval churn that resolves to an already-realized outPath
  costs no `msb load`.

### 3.2 The record — per-home `state/images.json`

The comparison baseline is a per-home record at
**`$WORKESTRATE_HOME/state/images.json`**, keyed by **(config-repo identity,
name:tag)** (USER DECISION D5). The record is **advisory, never authoritative
over the msb store** (USER DECISION D1): the store is ground truth for what is
loaded; the record is the tool's memory of what it loaded and why.

### 3.3 Concurrency — per-tag flock

A per-tag `flock` under `state/image-locks/` spans the whole
**eval → build → load → record** critical section:

- **Duplicate-work avoidance** — a second `up` racing the first waits instead
  of building the same tag twice.
- **Atomicity** — the record is written only after the load completes, inside
  the lock.
- msb has its own internal per-ref/per-layer locks; our lock is at the
  lifecycle level above them, not a replacement.

### 3.4 Skew matrix

| Record | Store tag | Action |
|---|---|---|
| fresh (drvPath + outPath match current eval) | present | **skip** — nothing to do |
| stale (drvPath or outPath differs) | present | **rebuild** → load → record |
| present | gone | **rebuild** → load → record |
| absent | present | **TRUST the store tag on plain `up`** (USER DECISION D1); rebuild on `--reload-images`, then record |
| absent | absent | **build** → load → record |

The TRUST branch exists because the store may have been populated by the old
manual ritual (or another home) before this tool ever recorded it — refusing
to trust it would force a spurious rebuild of every pre-existing tag on first
run. `--reload-images` is the explicit escape that re-establishes the record.

**Phase-C implementation note (2026-08-02):** in phase C the freshness
predicate compares **drvPath only** — the outPath re-load gate is phase D,
and records written by the phase-C D1-trust branch carry `out_path = ""`
(unknown until the pipeline realizes the drv; the drvPath-only predicate
never consults it). An unreachable msb store is a named ERROR per §7, not a
skew-matrix state, so `skew.rs`'s `StoreTag` stays 2-variant. `--check`
takes NO per-tag lock (it is a read-only report with no critical section).
Under `--all-repos`, a repo whose standalone load/merge fails (e.g. it
fails the current policy gates) is skipped with a note naming the repo and
the rest of the batch proceeds; explicit `--repo <name>` hard-errors on the
same failure.

**Phase-D implementation note (2026-08-02):** the outPath re-load gate is
now REAL (`images/pipeline.rs::reload_decision`, pure): after `nix build`
the pipeline re-loads `images.json` fresh INSIDE the still-held per-tag
lock and skips `msb load` only when the recorded `out_path` is non-empty
AND equals the realized outPath AND the store probe still finds the tag —
an out-of-band tag deletion loads anyway even with a matching outPath, and
phase-C trust records with `out_path = ""` never match the gate, so they
are **upgraded to full phase-D records (real `out_path`) on the next
build/rebuild** with no migration step. `nix build` stderr is TEED live to
the operator TTY (parent-side, user-facing, minutes-long) AND retained for
the §7 classification: fixed-output hash mismatch → the `update-hashes`
recipe pointer (not a raw nix error wall); fetch/substituter failure → the
offline-context error; anything else → a named error with the stderr tail.
The load stage is `gunzip -c <outPath>` piped into `msb load -t <tag>` (no
shell, no staged tarball — the anti-accumulation posture), with a post-load
store re-probe: `msb load` reporting success while the tag stays gone is a
named error. `digest` stays `null` at the phase-D write site: §11 item 1
verified the msb digest surface EXISTS (see below), but capture is deferred
until the §3.5 digest-COMPARISON design lands — the write site is the
documented probe point. A read-only `image_records` doctor check compares
`images.json` against `msb image ls` (missing recorded tag or unreadable
store → WARN, never FAIL).

### 3.5 Digest upgrade path

The drvPath/outPath comparison answers "did the BUILD INPUTS change", not "is
the LOADED tag the same BITS". Once msb exposes a manifest digest (HOST-VERIFY
cluster, §11), the record gains the msb manifest digest captured at load
time:

- In the TRUST branch (record absent + tag present), a **tag-digest mismatch
  flips the branch to rebuild**: the store tag is not the bits this repo would
  have loaded.
- Digest comparison is a **one-way signal**: match = same image (trust
  confirmed); mismatch = UNKNOWN provenance → rebuild. A mismatch never
  confirms freshness, only suspicion.

---

## 4. Image naming / collision policy

### 4.1 Stable tags, no auto-prefixing (USER DECISION D2)

Image tags are **stable**: `image.name:tag` is used **verbatim** from the
config TOML. There is NO auto-prefixing (no `<repo>-<name>` rewriting by the
tool). Auto-prefixing was rejected because:

- it breaks the phase-3 stable-tag decision (config-repo flake builds
  `workestrate-pi:latest` and `tempest:latest` under those exact names);
- it breaks the existing `load-images` flows, which load under the declared
  name;
- a prefixed form like `personal:pi` parses as **name:tag** in OCI grammar —
  the "prefix" would silently become a tag, corrupting the reference.

### 4.2 Cross-config-repo collisions (USER DECISION D2)

`validate-config` (and `plan`) **WARN** when two registered repos declare the
same image `name:tag`, naming BOTH repos in the warning. The warning is not an
error — the collision is legal and sometimes intentional — but it must be
loud. The operator's remedy is **manual prefixing in TOML**, e.g.
`image.name = "work-pi"`, which is a config edit, not a tool behavior.

### 4.3 Cross-home collisions (USER DECISION D5)

Because the msb store is global per OS user, a second home loading the same
tag overwrites the first home's bits. Policy: **warn + digest-detect**:

- Records are keyed by (config-repo identity, name:tag) — `repo_key` in
  `images.json` (§8).
- When a load/ensure observes a record whose `repo_key` differs from the
  current repo for the same tag, it WARNS (cross-home collision surface).
- Once the msb digest surface lands (§3.5), digest comparison sharpens the
  warning into detection: digest match = same bits, benign share; mismatch =
  genuine collision → warn with the digest evidence.

---

## 5. CLI surface

### 5.1 `workestrate workload build`

```
workestrate workload build [name] [--repo <config> | --all-repos] [--check] [--force] [--json]
```

| Selector | Scope |
|---|---|
| `build <name>` | one workload in the active context |
| `build` (bare) | all nix-layered workloads in the active context — mirrors the bare-up grammar of the ADR 0021 addendum |
| `build --repo <config>` | all nix-layered workloads declared by one registered repo |
| `build --all-repos` | all nix-layered workloads across ALL REGISTERED repos (`registry.configs`); repos without `flake.nix` are skipped with a note |

Flags:

- `--check` — print the staleness matrix (§3.4) only; build/load nothing.
- `--force` — rebuild regardless of the change-detection outcome.
- `--json` — machine-readable output (per ADR 0027's `--json`-first
  inspection-surface convention).

Eligibility (§2.3) applies per workload: non-nix-layered workloads in a
selected set are skipped.

**Zero-eligible-context behavior:** when a selector resolves to ZERO eligible
workloads, the command is a **no-op** and prints a note to **stderr**:

```
note: no nix-layered workloads in scope; nothing to build
```

### 5.2 `--reload-images` on `up` / `exec` / `batch-up`

`--reload-images` is the parent-side force switch on the lifecycle verbs: it
forces the ensure-images pre-flight to rebuild+load+record even when the skew
matrix would skip or trust (including flipping the D1 TRUST branch, §3.4).

- **Batch scope (USER DECISION D3):** under `batch-up` (and bare-up), the
  force applies to ALL service workloads in the batch — not just the first,
  not just the named one.
- **Never forwarded in `detach_args`** (USER DECISION D3): the flag is
  parent-side only; the child must never re-force a build it is skipping
  anyway via `--images-ready`.
- It must be **excluded from the bare-up flag-reject loop** (the guard that
  rejects name-scoped flags on bare `workload up`) and **threaded into
  `cmd_workload_up_all`** so the batch path honors it.

---

## 6. Reserved build location: `.workestrate-build/`

### 6.1 The reservation (USER DECISION D4)

Each config repo reserves **`.workestrate-build/` at the config-repo root** as
the workload-agnostic artifact location:

- **Workload-agnostic layout:** `<dir>/<workload-name>/` inside — one
  subdirectory per workload, shared across recipes.
- **Gitignored.** The directory never enters version control.
- **Artifact-only BY CONSTRUCTION.** Because the directory is gitignored, it
  is invisible to nix flake eval — a `git+file` / path-flake input sees only
  committed content, so `.workestrate-build/` can NEVER become a flake input
  source. The artifact/source separation is enforced structurally, not by
  discipline.
- **Scaffold-provisioned:** the scaffold template ships a `.gitignore` entry,
  a README inside the directory explaining the contract, and the in-tree
  `.tpl` copy — template + static copy + parity in ONE commit, with
  `scaffold-check` enforcing parity (the same mechanism that guards the
  tombi/flake templates).
  **Phase-A implementation note (2026-08-02):** the contract README ships as a
  section in the config-repo README (`README.md` / `README.md.tpl`), not as a
  README *inside* `.workestrate-build/` — a gitignored directory cannot carry
  committed content, so an in-directory README was impossible by construction.
- **New default for UNDECLARED `local_build` fallbacks:** a workload with a
  `local_build` recipe but no declared output dir falls back to
  `.workestrate-build/<workload-name>/`, resolved declaring-layer-relative
  (per the spec-17 / phase-0 declaring-layer-dir semantics). **Explicitly
  declared fallbacks keep working unchanged** — the reservation changes the
  default, never overrides a declaration.

### 6.2 Vocabulary separation from source build

`.workestrate-build/` is strictly separate vocabulary from the **source
build** / `WORKESTRATE_<NAME>_BUILD` env_override mechanism:

- They are **different artifact layers**: source build produces the MOUNTED
  APP ARTIFACTS (the binary the app runs); `.workestrate-build/` and the
  nix-layered pipeline produce the SANDBOX ROOTFS (the image the sandbox boots
  from).
- They have **different staleness domains**: app-artifact staleness is keyed
  on source-tree changes; image staleness is keyed on drvPath/outPath (§3).
- A future `workestrate build` umbrella verb unifying the two layers is
  explicitly **DEFERRED** — the vocabularies stay separate until that verb is
  designed.

---

## 7. Failure modes

| Failure | Behavior |
|---|---|
| fakeHash placeholder in the flake | Eval-only change detection works fine (§3.1); the BUILD fails at the FOD — the error points at the config repo's `update-hashes` recipe, NOT a raw nix hash-mismatch error wall |
| No `flake.nix` in the declaring repo | HARD ERROR naming the repo; under `batch-up` / `--all-repos` → skip-with-note (the missing flake is named, the rest of the batch proceeds) |
| CWD-derived project root (nix-layered workload, cwd not the declaring repo) | RESOLVED by ADR 0028 (2026-08-13): the flake root comes from the declaring config repo (registry-known), never CWD; the CWD/"Set AGENTCTL_ROOT" error is gone. The remaining failure is the row above (declaring repo genuinely flake-less) |
| nix absent from PATH + store tag present | Proceed with a stderr note (the tag may be fresh; nothing verifiable without nix — degrade, don't block) |
| nix absent from PATH + store tag missing | HARD ERROR + remediation (install nix or load the image manually via the config-repo ritual) |
| msb store unreachable | Reuse the `ps.rs` unreachable-DB vocabulary — same error shape and remediation wording as the port-registry/instance-DB unreachable path |
| Offline | Locked inputs eval fine (drvPath eval needs no network against a pinned flake.lock); builds proceed only if all inputs are already realized in the store — otherwise the nix fetch failure surfaces with the offline context noted |
| Cross-home collision (record `repo_key` ≠ current repo) | WARN per D5 (§4.3); digest-detect when the msb digest surface lands |
| Concurrent same-tag build | Block on the per-tag flock (§3.3), then RE-CHECK the skew matrix inside the lock — the winner's rebuild usually flips the waiter's verdict to skip |

---

## 8. State schema — `state/images.json`

One record per (config-repo identity, name:tag):

```json
{
  "version": 1,
  "images": {
    "personal#workestrate-pi:latest": {
      "repo": {
        "name": "personal",
        "path": "$WORKESTRATE_HOME/config-repos/personal",
        "flake_root": "$WORKESTRATE_HOME/config-repos/personal"
      },
      "attr": "workestrate-pi",
      "tag": "workestrate-pi:latest",
      "drv_path": "/nix/store/…-workestrate-pi.tar.gz.drv",
      "out_path": "/nix/store/…-workestrate-pi.tar.gz",
      "digest": null,
      "built_at": "2026-08-02T10:15:00Z",
      "loaded_at": "2026-08-02T10:16:12Z",
      "loader": "workestrate 0.1.0",
      "host": "devbox",
      "user": "node"
    }
  }
}
```

- `version` — schema version of the state file itself (additive bumps only,
  same posture as the config schema).
- `repo` — the config-repo identity (name + path + flake_root); the map key is
  the `<repo>#<tag>` composite.
- `digest` — the msb manifest digest at load time; `null` until msb exposes it
  (§3.5, §11).
- `loader` / `host` / `user` — provenance for the D5 cross-home collision
  warning (who loaded this, from where).

**Write discipline:** atomic tmp+rename, the same discipline as the
port-registry store and `workestrate.lock` — write to a sibling temp file,
`fsync`, rename over the target. Never a torn record after a crash mid-load.

---

## 9. Relation to source build

The image build/load lifecycle and the source build
(`WORKESTRATE_<NAME>_BUILD`, `local_build`) share vocabulary — "build", "stale",
"artifact" — but are different layers (§6.2): mounted app artifacts versus the
sandbox rootfs, with disjoint staleness domains. This spec governs the ROOTFS
layer only. The one place the layers meet: nix-layered images bake
`image.binary` from a `flake://` source at the revs pinned in the config
repo's `flake.lock` — so `--reload-images` refreshes the baked binary **only
as far as the lock allows**. Refreshing the baked binary past the lock is a
flake-input update (operator action in the config repo), not a reload.

---

## 10. Phased plan

| Phase | Scope | Files (symbol level) | Tests | Gates | Validation tag |
|---|---|---|---|---|---|
| **A** | This spec + scaffold reservation: `.workestrate-build/` `.gitignore` + README + `.tpl` parity, `local_build` undeclared-fallback default | `scaffold/template/` (gitignore/README/.tpl), `copier` static copy, `local_build` fallback resolution in `types.rs` / `plan.rs` | scaffold parity test (existing `scaffold-check` harness); fallback-default unit test | `just scaffold-check`, `cargo test` | S, `verifiable-here` |
| **B** | Image-state store: `state/images.json` schema + atomic tmp+rename IO + `state/image-locks/` flock helper | new `control/agentctl/src/images/state.rs`; reuse the port-registry atomic-write pattern | round-trip serde test; tmp+rename crash-safety test; flock contention test | `cargo test` | S, `verifiable-here` |
| **C** | Change detection + `workload build` verb: drvPath eval, skew matrix, selectors, `--check`, `--force`, `--json`, zero-eligible note | new `images/detect.rs`, `images/build_cmd.rs`; verb wiring in `main.rs`; `registry.configs` iteration | drvPath-eval unit tests against fixtures; skew-matrix table tests; selector-scope tests; stderr-note assertion | `cargo test`; live `nix eval` smoke | M, `HOST-NIX` |
| **D** | Build/load pipeline: `nix build` + `msb load` orchestration, outPath re-load gate, digest capture (pending msb surface) | new `images/pipeline.rs`; msb invocation wrapper alongside the existing `microsandbox` SDK call sites | pipeline staging tests with a stub loader; re-load-on-outPath-change test | `cargo test`; `nix build` + `msb load` smoke; msb digest-surface verification | M, **DONE 2026-08-02 — validated in-container through the FULL e2e** (fixture image: nix build → gated `msb load` → record → gate-skip → out-of-band-delete reload, via the real CLI AND `tests/image_pipeline_e2e.rs`); `HOST-NIX` only for the real workestrate-pi/tempest images (network FODs) — see the §11 verification block |
| **E** | Lifecycle wiring: ensure-images pre-flight in `cmd_workload_up`/`exec`/`batch-up`, `images_ready` on `InstanceSpec`, `--images-ready` in `detach_args`, `--reload-images` threading into `cmd_workload_up_all` | `spawn.rs`, `run.rs`, `main.rs`, `InstanceSpec`, `detach_args`, bare-up flag-reject loop | parent-ensures/child-skips integration tests; batch force-scope test; KVM e2e: up after TOML edit rebuilds before spawn | `cargo test`; guest boot + stale-tag e2e | IMPLEMENTED-UNCOMMITTED-UNVALIDATED (2026-08-03): ensure.rs + `InstanceSpec.images_ready` + `detach_args --images-ready` + `--reload-images` on up/exec/batch-up + `cmd_workload_up_all` batch ensure + `EnsurePreflight` dep inheritance. Needs: targeted cargo tests + `just verify` + commit, then the HOST-KVM e2e (stale-tag rebuild before spawn; #[ignore]'d ensure_images_e2e KVM variant). |
| **F** | Multi-repo + personal repo migration: `--repo`/`--all-repos` breadth, personal config repo cutover from the manual justfile ritual, record seeding | `images/build_cmd.rs` (repo iteration), personal repo justfile (`update-hashes` / `load-images` recipes retired) | multi-repo selector tests; first-run record-seeding smoke | `cargo test`; personal-repo `up` smoke | S–M, `HOST-NIX` |

**Ordering:** A and B are independent; C and D may parallelize behind B; E
depends on C + D; F depends on E.

---

## 11. HOST-VERIFY cluster (msb + nix behaviors to verify on the host)

These behaviors are load-bearing assumptions of this spec that cannot be
verified in this container. Each is a phase-D/E gate item:

1. **msb `image ls` digest surface.** Does `msb image ls` (or an equivalent
   msb API) expose the manifest digest per tag? The digest upgrade path (§3.5)
   and D5 collision detection depend on it; if absent, `digest` stays `null`
   and digest-detect defers.
2. **Bare-tag normalization vs docker.io pull fallback.** If msb normalizes
   bare tags differently at load time versus at `create()` time, the loaded
   tag can be INVISIBLE to `create()` — which then falls back to pulling from
   docker.io, surfacing as a confusing offline failure. Verify the
   normalization is identical on both paths.
3. **`msb load` stdin concurrency.** `msb load` reads the tarball from stdin;
   verify behavior under our per-tag flock (§3.3) — msb's internal per-ref/
   per-layer locks must compose, not deadlock, with an outer lock held across
   the load.
4. **Tarball → digest determinism.** dockerTools build reproducibility plus
   bun-compile FOD determinism: the same flake.lock rev must yield the same
   tarball digest, or the digest comparison (§3.5) produces constant false
   mismatches.
5. **`git+file` dirty-worktree drvPath stability.** For a `git+file` flake
   input, an uncommitted (dirty) worktree must produce a stable drvPath across
   evals — otherwise every eval looks stale and change detection (§3.1) is
   useless on dirty trees.

**Phase-D in-container verification (2026-08-02, nix 2.35.1 / msb 0.5.6):**
most of the cluster turned out to be verifiable in-container after all —
`msb load` is a pure STORE IMPORT and needs no KVM (KVM gates only RUNNING
sandboxes), and a ~20 KiB `dockerTools.buildLayeredImage` fixture image
(`control/agentctl/tests/fixtures/image-flake/`, no network FODs, nixpkgs
pinned to this repo's `flake.lock` rev) exercised every item:

1. **VERIFIED in-container.** `msb image ls` prints a `DIGEST` column
   (truncated); `msb image inspect <ref>` prints the FULL manifest digest
   (`sha256:3a48c1e72d5a0e6f…` for the fixture); the Rust SDK exposes
   `ImageHandle::manifest_digest() -> Option<&str>` (microsandbox 0.5.6,
   `lib/image/mod.rs`). The §3.5 surface EXISTS. Phase D still records
   `digest: null` — capture is deferred to the §3.5 comparison design (the
   one-way signal in the D1 trust branch), with the pipeline's record-write
   site as the documented probe point. Host remainder: none for the surface
   itself.
2. **VERIFIED at the load/query level in-container.** `msb load -t
   othername` (bare) registers the reference VERBATIM as `othername` — NOT
   normalized to `othername:latest` — and `msb image inspect
   othername:latest` then fails "image not found" while `inspect othername`
   succeeds: load-time and query-time normalization are the SAME (verbatim)
   in 0.5.6. D2's always-`name:tag` tags keep the tool on the safe side by
   construction. Host remainder: the `create()`-time docker.io pull
   fallback for a missing tag is exercised with the phase-E KVM e2e.
3. **VERIFIED in-container.** Two parallel `gunzip -c … | msb load -t
   <same tag>` runs raced: one loaded, the other FAILED with `cache error
   at …/cache/manifests/<hash>.json: No such file or directory (os error
   2)`. msb's internal locking does NOT make concurrent same-tag loads
   safe — the outer per-tag flock (§3.3) is load-bearing — and the loser
   ERRORS rather than deadlocking, so the locks compose as spec'd.
4. **PARTIALLY verified in-container.** The same fixture tarball reloaded
   across separate `MSB_HOME`s and separate loads yields the IDENTICAL
   manifest digest; for the fixture, same-rev reproducibility reduces to
   nix content-addressing (same outPath → same bytes). Host remainder:
   bun-compile FOD determinism for the real workestrate-pi/tempest images
   (HOST-NIX).
5. **VERIFIED in-container.** A git flake with a dirty (uncommitted)
   worktree produces a drvPath that is STABLE across repeat evals AND
   tracks the dirty content: editing the derivation (`name = "img"` →
   `"img-dirty"`) without committing changed the drvPath
   (`…-img-dirty.drv`), and repeat dirty evals returned the identical
   drvPath. Change detection is sound on dirty trees.

One container caveat discovered during verification: the workestrate
**devshell wraps `msb` with a forced `MSB_HOME=$HOME/.microsandbox`**,
overriding the caller's `MSB_HOME`. The phase-D e2e
(`control/agentctl/tests/image_pipeline_e2e.rs`) therefore gates on an
explicit `MSB_PATH` pointing at an UNWRAPPED msb binary — running it
through the wrapper would write fixture images into the real home store.
On a host without the wrapper, plain `msb` honors `MSB_HOME` naturally.


---

## 12. Governance + migration notes

### 12.1 Additive-only per ADR 0021 §8

This spec adds: new flags (`--reload-images`, `--images-ready` hidden), a new
verb (`workload build`), one state file (`state/images.json`), and one
reserved gitignored directory. **No config-schema change**: `image.name`,
`image.tag`, `image.recipe`, and `local_build` already exist; every new
behavior keys off existing fields. `schema_version` stays `1` (spec 20's
additive rule).

### 12.2 Cross-references

- **Spec 17** — declaring-layer provenance: the `.workestrate-build/` fallback
  resolves declaring-layer-relative, and the per-repo `flake_root` in the
  record (§8) is the declaring repo's root.
- **Spec 11** — home/lockfile state conventions: `state/images.json` lives
  under `$WORKESTRATE_HOME/state/` with the same atomic-write discipline as
  the port registry; it is ephemeral state, never copied by `home clone`.
- **Spec 12 / ADR 0026 addendum** — the detach-token precedent: `--images-ready`
  rides `detach_args` exactly as `--no-deps` does.
- **Spec 01** — mounts interplay: `.workestrate-build/` is host-side artifact
  state; nothing in it is ever mounted into a guest, and the mount-exclusion
  vocabulary does not apply to it.
- **Spec 07** — the image-name stability decision: D2's stable-verbatim tags
  are the continuation of the spec-07 name-parity decision (image name/tag ↔
  msb store parity).

**ADR 0028 may be minted at implementation time** if the ensure token
(`images_ready` / `--images-ready`) or the cross-home collision policy (D5)
proves load-bearing beyond this spec — recorded here so the minting is a
deliberate act, not an afterthought.

### 12.3 Migration notes — personal config repo

- The personal repo's flake is **hand-maintained** (no copier propagation), so
  phase A's scaffold changes do NOT reach it automatically — the
  `.workestrate-build/` `.gitignore` entry is a manual one-line add there.
- The tempest **explicit `local_build` fallback keeps working** — declared
  fallbacks are never overridden by the new default (§6.1).
- The **first `--reload-images` seeds the records**: pre-existing store tags
  hit the D1 TRUST branch on plain `up`; one explicit `--reload-images` run
  rebuilds, loads, and writes the initial `images.json` entries, moving the
  repo from manual-ritual provenance to tool-recorded provenance.

---

## 13. Acceptance criteria

- [~] ensure-images is parent-side only; the detached child skips it via
      `images_ready` on `InstanceSpec` + hidden `--images-ready` in
      `detach_args`; `--foreground` ensures (§2) (implemented-uncommitted-unvalidated, phase E working tree: ensure.rs + InstanceSpec.images_ready + detach_args --images-ready; pending targeted tests + commit + HOST-KVM e2e).
- [~] Eligibility gated on `image.recipe == "nix-layered"` mirroring
      `flake_root_requirement`; ensure runs BEFORE
      `auto_start_dependencies` (§2.3–2.4) (implemented-uncommitted-unvalidated, phase E working tree; pending targeted tests + commit + HOST-KVM e2e).
- [x] Change detection is eval-only `nix eval --raw <repo>#<name>.drvPath`;
      re-load only on outPath change; per-tag flock spans
      eval→build→load→record (§3) (landed phases B+C+D: 290e91b lock/state/skew, 0729bb4 detect, 932b476 pipeline reload_decision).
- [x] Skew matrix implemented exactly as §3.4, including the D1 TRUST branch
      and the `--reload-images` flip; digest one-way signal when the msb
      surface lands (§3.5) (landed phase B 290e91b); digest one-way signal DEFERRED pending §3.5 design (§11 item 1 verified the msb digest surface exists, 932b476).
- [~] Stable verbatim tags, no auto-prefixing; `validate-config`/`plan` WARN
      on cross-repo name:tag collision naming both repos (D2); cross-home
      warn + digest-detect with records keyed by (repo identity, name:tag)
      (D5) (§4) — PARTIAL: stable verbatim tags landed (0729bb4); cross-repo collision WARN + cross-home digest-detect NOT yet implemented (D5 digest-detect deferred to §3.5).
- [~] `workestrate workload build [name] [--repo | --all-repos] [--check]
      [--force] [--json]` with the selector table of §5.1; zero-eligible
      no-op + stderr note; `--reload-images` on up/exec/batch-up with D3
      batch scope, excluded from the bare-up flag-reject loop, threaded into
      `cmd_workload_up_all` (§5) — PARTIAL: build verb landed (0729bb4); `--reload-images` on up/exec/batch-up + D3 batch scope + bare-up reject-loop exclusion + `cmd_workload_up_all` threading implemented-uncommitted (phase E working tree).
- [x] `.workestrate-build/` reserved, gitignored, artifact-only by
      construction, scaffold-provisioned with `scaffold-check` parity; new
      default for undeclared `local_build` fallbacks only (D4) (§6) (landed phase A 02bea9a).
- [x] Failure-mode table (§7) implemented row-for-row, including the
      `update-hashes` pointer and the `ps.rs` unreachable-DB vocabulary
      (landed phases C+D 0729bb4/932b476; §7 missing-flake row reused by ensure — implemented-uncommitted phase E).
- [x] `images.json` per §8, atomic tmp+rename; `version` field present
      (landed phase B 290e91b).
- [x] HOST-VERIFY cluster (§11) items each recorded verified-or-deferred in
      the phase-D/E gate notes (recorded phase D 932b476 — §11 verification block: 4 of 5 verified in-container, item 4 partially; host remainders noted).
- [x] No config-schema change; `schema_version` stays `1` (§12.1) (true throughout).
- The build/ensure path works from any CWD: the flake root resolves from the declaring config repo (ADR 0028); `AGENTCTL_ROOT` is never required for config-repo-derived roots.

**Key decision:** the tool owns freshness via eval-only drvPath change
detection against an advisory per-home record — the msb store stays ground
truth for presence, the record stays memory for provenance, and the five user
decisions (D1–D5) pin every place the two could disagree.

---

## 14. Open follow-up — KVM-test MSB_HOME convention

> **STATUS: open follow-up (recorded 2026-08-03).** Pragmatic workaround in
> place; a more idiomatic long-term approach should be brainstormed later.

The two HOST-KVM integration tests (`tests/lifecycle_detached.rs`,
`tests/ensure_images_e2e.rs`) boot a real sandbox, so microsandbox derives
its agent-relay unix socket as `$MSB_HOME/run/agent/<32hex>.sock` =
`len(MSB_HOME) + 48` bytes, capped at the 108-byte Linux `sockaddr_un`
limit → `len(MSB_HOME)` must be `<= 59`. The codebase's usual
`workestrate-<label>-<pid>-<nanos>` temp dir under a deep `$TMPDIR` blows
that budget, so the tests were decoupled via a `common::short_msb_home()`
helper that uses a fixed short `/tmp/wk-msb-<pid>-<nanos>` base (the `/tmp`
literal diverges from the `temp_dir()` idiom deliberately — documented on
the helper).

This is a pragmatic workaround, not the idiomatic long-term answer.
Follow-up to brainstorm later (tracked as a beads issue, spec-21 epic):

- msb supporting a **configurable run/socket dir** via `paths.sandboxes`
  (or equivalent), so the socket path is independent of `MSB_HOME` depth;
  OR
- a **canonical short-`MSB_HOME` test convention** surfaced as a first-class
  test-support helper with a documented contract, rather than a `/tmp`
  literal in two KVM tests.

Either path lets the KVM tests return to the `temp_dir()` idiom and removes
the `/tmp` divergence. Refactor when the socket-path pressure is revisited.


## 15. Open follow-ups

> **STATUS: open follow-ups (recorded 2026-08-03).** Tracked as beads issues
> (spec-21 epic); noted here so the spec carries the decision pointers.

- **wrk-ayz — Canonical config-flake input URL (Phase-5 decision).** The
  personal config repo's `workestrate` flake input is currently a host-absolute
  `git+file:///home/rybski/...` path (a workaround committed during the
  host-boot fix pass; the prior `git+file:///home/node/...` was container-only
  and unusable on the host). The canonical fix is a pinned
  `github:georgrybski/workestrate` input, deferred to Phase 5 because it
  requires the tool repo's `origin/migration/tool-model` branch to be pushed
  to the remote first (verify push state on the host with `git fetch`). See
  §10 (phased plan) and §12.3 (migration notes — the config-flake input is
  what makes the nix-layered image builds reproducible across machines).

- **wrk-23b — ensure-images: keep eval-error fail-closed vs nix-absent trust
  posture (decision).** Decision (recorded 2026-08-03): nix eval errors
  FAIL-CLOSED (an eval error almost always conceals a real config bug — a
  missing attr, a broken overlay, a stale lock — so trusting past it would
  silently run a stale/foreign image); nix-absent degrades to a TRUST posture
  (the documented standalone-install path, §3.4 / §8); tag-absent ALWAYS
  forces a build/load. The two postures are deliberately distinct (config
  signal vs environment signal) and must not be collapsed by a future
  refactor. This is a documentation/decision bead — no code change.
