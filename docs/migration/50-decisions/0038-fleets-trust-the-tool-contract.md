# ADR 0038: Fleets trust the tool contract — config repos never pin the tool

**Status:** Proposed
**Date:** 2026-09-06
**References:** `flake.nix` (`lib.buildImagesFromConfig`); `nix/lib/recipes.nix` + `nix/lib/recipes/` (npm-build, bun-compile, pip-install, bun-install, nix-layered, registry); `control/agentctl/src/images/` (`build_cmd.rs`, `pipeline.rs`, `detect.rs`, `ensure.rs`, `state.rs`); `control/agentctl/src/config/validation.rs` (`EXPECTED_SCHEMA_VERSION`); `control/agentctl/src/config/lockfile.rs` + `control/agentctl/src/commands/home.rs` (`workestrate.lock`); `control/agentctl/src/commands/versions.rs` (the pin quadruple); ADR 0025 (`workestrate.lock`), ADR 0032 (content-addressed image tags, config source model), ADR 0003 (config = data, closed recipe vocabulary); fleet repos `workestrate-dev-home/config-repos/{personal,duelbits}`; shadow pilot `config-repos/personal/workloads/tempo-buddy/flake.nix`

## 1. Title / Status / Date

- **Title:** Fleets trust the tool contract — pins flow DOWN the hierarchy, and nothing pins the tool
- **Status:** Proposed.
- **Date:** 2026-09-06.
- **Amends:** the fleet-side consumption model implied by ADR 0008/0025 (config repos consuming the tool as a pinned flake input); the spec 21 §7 "No flake.nix in the declaring repo" row (nix-layered builds no longer require a fleet flake root).

## 2. Context

Today every config repo (fleet) pins the tool: `personal/flake.nix` and `duelbits/flake.nix` carry a `workestrate` flake input (lock-pinned rev), call `workestrate.lib.<sys>.buildImagesFromConfig { pkgs; config; sources; }`, build the workload images fleet-side, and ship `inputs.workestrate.packages.<sys>.workestrate` + `msb-wrapped` in their devshells. The consequences are verified, not hypothetical:

- **The relock-tax is structural.** Every tool rev bump requires a relock of every consuming fleet. The tempo-buddy shadow pilot and `workloads/common/versions.nix` pin `c3b18d4…` while the personal root lock has already moved to `ba49ed9…` — the "bump these URLs by hand" discipline failed in practice within weeks. duelbits' lock is older still (`074fc02…`). Three consumers, three different tool revs, all "current".
- **Fleets are not shareable.** A fleet checked out at a pinned tool rev only builds with exactly that tool rev; two fleets on different pins cannot be served by one installed CLI.
- **The capsule is not self-contained.** `workestrate/workloads/pi/workload.toml` says `binary.src = "flake://pi"`, but what `pi` MEANS — the npm-build with its custom build/install phases, the `npmDepsHash` let-bindings, the `binary_name`/`install_dir`/`assets` enrichment — lives in fleet `flake.nix` Nix code, not in the capsule. The fleet's `sources` map passes live Nix DERIVATIONS (`piBuilt`, `primeBuilt`) into the tool's builder function, so the build only exists inside fleet-side Nix evaluation.
- **The direction of the pin is wrong.** The owner directive: pins flow DOWN the hierarchy. The home registry pins fleets (`workestrate.lock` `repos.*` url/ref/rev — already true, ADR 0025). Fleets will pin per-workload repos (the tempo-buddy pilot's destination). NOTHING pins the tool. The tool↔fleet relationship is a CONTRACT — a schema version — not a pin.

What already works in the tool's favor (verified): the CLI's image pipeline (`workestrate workload build`, spec 21 phases C/D/E) is a lock → probe → eval → skew → build → load → record flow over trait seams, with content-addressed tags computed from the evaluated `outPath` (ADR 0032 A2); `schema_version` enforcement exists (`EXPECTED_SCHEMA_VERSION = 1`, missing warns and assumes 1, anything above hard-errors); the recipes are a closed, versioned-in-tree vocabulary mirrored by Rust allowlists; the host already installs the CLI ambiently via `nix profile` (the doctor remediation pair `nix profile remove workestrate` / `nix profile install .#workestrate`).

## 3. Decision

**The pinning axiom:** pins flow DOWN only. Home registry → fleets (`workestrate.lock`, unchanged). Fleet → workload repos (fleet-level `workestrate.lock`, §8). Capsule → payload sources (in-capsule url+rev+hash, §5). The tool is pinned in exactly ONE place: the operator's host profile, owned by host provisioning (`scripts/host-provision.sh`) — never by any repo. A repo that pins the tool is a contract violation.

**Image building moves to the tool.** The CLI builds images FROM CAPSULE DATA via a builder expression baked into the installed tool package. Fleets stop owning image builds: the fleet flake sheds the `workestrate` input, the `sources` map, the hash let-bindings, the image/check outputs, and the pinned devshell packages. What remains in a fleet is data: `workestrate/` capsules, schemas, scripts, tests, the static kit — and an OPTIONAL devshell for that static kit's tooling (tombi/just/jq), which uses the ambient CLI from PATH.

**The tool↔fleet contract is the `schema_version` window** (§4): the tool declares the window of capsule schema versions it understands; the fleet declares its version; a mismatch is a clean error naming both sides.

## 4. The contract definition

There are four version axes; name them so they stop blurring:

1. **Lock format version** (`workestrate.lock` `version = 1`) — home↔fleet pin file format. Unchanged.
2. **Home layout version** (`home_version = 2`) — the tool-home directory layout. Unchanged.
3. **Capsule schema version** (`schema_version`, currently `EXPECTED_SCHEMA_VERSION = 1`) — **THE contract.** It covers the capsule TOML shape, the recipe vocabulary and recipe parameter semantics, the `source://` reference scheme, and the builder's interpretation of all of it.
4. **Builder identity** (`builder_id`, NEW, §9) — the tool's nixpkgs rev + recipes-tree hash. NOT part of the schema; provenance only.

The single const `EXPECTED_SCHEMA_VERSION` becomes a **window**: `SCHEMA_VERSION_MIN` / `SCHEMA_VERSION_MAX` (both 1 today — the mechanism lands before it is needed). The window lives where the const lives today (`control/agentctl/src/config/validation.rs`), mirrored in the committed JSON schema, and surfaces in `workestrate versions` output. Rules:

- Capsule/layer declares a version inside [MIN, MAX]: accepted.
- Missing version: warn + assume MIN (today's backward-compat posture), becoming a hard error at schema 2.
- Declared version > MAX (capsule too new for the tool): hard error naming the declared version, the tool's window, the tool version, and the remediation — upgrade the tool via the host profile (`nix profile install`). This is the contract doing its job for fleet SHARING: a consumer with an older tool gets a sentence, not a confusing eval failure.
- Declared version < MIN (capsule too old): hard error naming both sides + the migration recipe.

Any tool change that alters the MEANING of an in-window capsule (recipe semantics, vocabulary removals, `source://` resolution) is a contract break and MUST bump the window (raise MIN, or MAX with a migration window). A tool nixpkgs bump is NOT a contract break: it changes `builder_id`, hence possibly the output bytes — which the content-addressed tags make visible and the provenance record attributes (§9).

## 5. Capsule design: `[image.sources]`, `source://`, build parameters as data

The capsule becomes self-contained. Every payload the image is built from is declared as DATA in the capsule (or inherited from a fleet layer via the existing merge — `default.toml` can declare shared sources, so pi's pin lives once per fleet and pi + tempo-buddy inherit it).

### 5.1 Source declarations

New optional capsule table `[image.sources.<name>]`:

```toml
[image.sources.pi]
url = "github:earendil-works/pi"          # any fetchTree-compatible URL scheme
rev = "371adcf37130629ffb9bbeed9f5548ce08ffa93b"
hash = "sha256-…"                          # narHash of the fetched tree (fixed-output)

# Optional prepare stage: a build recipe applied to the fetched tree.
# Exactly the existing recipe parameter vocabulary, now as TOML data.
[image.sources.pi.build]
recipe = "npm-build"
npm_deps_hash = "sha256-1EGs8lX8XoAnRtS+pw4lBRm24U/vtVB2loVRmZyd4Z8="
dont_npm_build = true
build_phase = '''
cd packages/tui && npm run build && cd ../..
cd packages/ai && ../../node_modules/.bin/tsgo -p tsconfig.build.json && cd ../..
cd packages/agent && npm run build && cd ../..
cd packages/coding-agent && npm run build && cd ../..
'''
install_phase = '''
mkdir -p $out
cp -r packages $out/packages
cp -r node_modules $out/node_modules
cp package.json $out/package.json
cp package-lock.json $out/package-lock.json
'''
```

Resolution (the tool's builder): `builtins.fetchTree { …; narHash = hash; }` — one mechanism for every scheme, fixed-output pinned, no consumer-side flake.lock, no IFD. With a `[build]` block the recipe runs over the fetched tree and `source://pi` denotes the PREPARED tree; without one it denotes the raw fetch.

### 5.2 References

`image.binary.src` and `image.extra_contents` entries take the closed reference form `source://<name>`:

```toml
[image.binary]
recipe = "bun-compile"
src = "source://pi"
entrypoint = "packages/coding-agent/dist/bun/cli.js"
worker = "packages/coding-agent/src/utils/image-resize-worker.ts"
binary_name = "pi"                  # ↓ the fleet-side Nix enrichment moves here, as data
install_dir = "app/bin"
assets = [ /* the 10 pi asset entries, verbatim */ ]
```

Anywhere a recipe is named (`image.binary`, `image.sources.<name>.build`), that recipe's FULL parameter vocabulary is legal — one rule, no per-position special cases. `build_phase`/`install_phase` strings are the contract's explicit escape hatch: legal, schema-covered, and discouraged (a capsule using them couples itself to recipe internals; a recipe-semantics change that alters escape-hatch interpretation is a contract break per §4).

`image.env` values gain ONE closed placeholder form, `${source:<name>}`, expanded by the builder to the prepared tree's store path (needed by prime's `PRIME_AGENT_KERNEL_PYTHON`). This is a closed reference vocabulary — like `source://` itself — not general templating; ADR 0003's no-templating rule is amended by exactly this entry.

`npm-build` gains one optional parameter: `lockfile` (capsule-relative path overlaid onto the fetched tree before `npm ci`) — absorbing prime's committed `package-lock.json` overlay (`primeFixed`) into the recipe vocabulary. Prime's koffi-prebuild `rm -rf` rides the existing `install_phase` escape hatch; its wrapper script becomes a `baked_files` entry (a static file committed in `workestrate/workloads/prime/` — static files are capsule content; logic is recipes).

### 5.3 `flake://` dies

`flake://<name>` remains parseable for ONE schema window as the legacy form: a capsule using it builds via the OLD path (the CLI evals/builds `<declaring-fleet-flake>#<image.name>`, exactly as today) with a deprecation warning. At schema_version 2 the legacy form and the fleet-flake build path are removed. The fleet's source flake inputs (`pi`, `tempest`, `prime`, …) die with the sources map; the pins move into the capsules as url+rev+hash. Hash maintenance moves to the tool: `workestrate workload update-hashes [name]` recomputes source narHashes and `npm_deps_hash` values (prefetch must match the recipe's fetch — which is why the tool, owner of the recipes, owns the command) and rewrites the capsule TOML. The fleet justfiles' `update-hashes` recipes become thin wrappers and eventually disappear.

### 5.4 Full target example (pi)

```toml
schema_version = 1

kind = "agent"
workdir = "/work"
cpus = 4
memory_mib = 8192
command = ["/app/bin/pi"]

[image]
recipe = "nix-layered"
name = "workestrate-pi"
tag = "latest"
contents = ["cacert", "busybox", "fakeNss", "ripgrep", "fd"]
features = ["create_tmp"]

[image.sources.pi]
url = "github:earendil-works/pi"
rev = "371adcf37130629ffb9bbeed9f5548ce08ffa93b"
hash = "sha256-…"

[image.sources.pi.build]
recipe = "npm-build"
npm_deps_hash = "sha256-1EGs8lX8XoAnRtS+pw4lBRm24U/vtVB2loVRmZyd4Z8="
dont_npm_build = true
build_phase = '''…'''
install_phase = '''…'''

[image.binary]
recipe = "bun-compile"
src = "source://pi"
entrypoint = "packages/coding-agent/dist/bun/cli.js"
worker = "packages/coding-agent/src/utils/image-resize-worker.ts"
binary_name = "pi"
install_dir = "app/bin"
assets = [ /* … */ ]
```

## 6. The builder seam: the tool builds from capsule data

Today's blocker: the fleet's `sources` map passes NIX DERIVATIONS — the CLI cannot consume derivations as data. The seam is therefore: **capsules carry pins (url+rev+hash), the tool does its own fetching and building.**

- **Baked builder.** The tool package installs its nix lib: `nix/lib/` (recipes aggregator + `recipes/` + `vocabulary.nix`) lands at `$out/share/workestrate/nix-lib/`, plus a new entry `nix/lib/build-capsule.nix`: `{ capsuleJsonPath }: <derivation>` — reads the capsule with `builtins.fromJSON (builtins.readFile …)`, resolves `[image.sources]` via `fetchTree`, runs the prepare/binary recipes, and calls `image.nix-layered`. The nixpkgs the builder imports is the TOOL's OWN nixpkgs, spliced in at tool build time (substituted by `nix/packages/agentctl.nix`): the tool owns the recipe toolchain, exactly as it owns the recipes. The binary resolves the builder relative to its own store path (`/proc/self/exe` → `../share/workestrate/nix-lib/`), with a `WORKESTRATE_NIX_LIB` env override for dev.
- **CLI flow.** `workestrate workload build` keeps its ENTIRE existing flow — selector, provenance, per-tag lock, store probe, skew matrix, content-addressed tag, pipeline, records, GC (build_cmd/pipeline/detect/ensure/state are untouched in shape). Two seams change implementation: the evaluator and the builder stop referencing `<declaring-fleet-flake>#<image.name>` and instead (1) serialize the merged workload's resolved `image` section to `<state>/build/<name>.capsule.json`, then (2) `nix build --impure --expr 'import <builder> "<capsule.json>"' --no-link --print-out-paths`. The `--impure` is honest and narrow: the only ambient input is the capsule JSON path; every fetched input is narHash-pinned, so the build itself is pure. The outPath-first eval (A2) evaluates the same expression's `.outPath` — tag computation, freshness, and the re-load gate are unchanged.
- **The flake-root requirement dies for builds.** `SelectSkip::NoFlakeRoot` (spec 21 §7) applies only to legacy `flake://` capsules during the transition window. A fleet needs no flake.nix to have its images built.
- **`buildImagesFromConfig` dies.** Not repurposed, not kept: its logic is reincarnated INSIDE the tool as `build-capsule.nix`, driven by CLI-serialized JSON instead of fleet-passed derivations. The public `lib.<sys>.buildImagesFromConfig` export is removed at schema 2 (pre-1.0 tool; two known consumers; this ADR is the migration notice). `lib.<sys>.recipes` remains exported for fleet-local experimentation but is no longer on the build path.
- **What remains in the fleet flake:** the devshell for the static kit (just, tombi, jq — no tool packages, §7) and nothing else. The `images`/`packages`/`legacyPackages.sources` outputs, the `prime-smoke` app, the image checks, the `sources` map, the hash let-bindings, and the `workestrate` input all go away. Fleet-side static checks worth keeping (e.g. `prime-image-static`) move into the tool as golden fixtures (§9) or become capsule-content tests run by `workestrate validate-config`.

## 7. The devshell change: ambient CLI

Fleet devshells drop `inputs.workestrate.packages.<sys>.workestrate` and `.msb-wrapped`; PATH provides the host-profile binaries. Verified consequences:

- **Nothing else was provided.** The fleet devshell package lists are `just, workestrate, msb-wrapped, curl, gzip, python3` (identical in both fleets) — no sops, no other tool-flake packages; jq/git arrive via the tooling devenv modules, nix via the host.
- **msb-wrapped leaves too.** Its only fleet-side consumer was the `load-images` ritual's `msb load`. The CLI's pipeline locates msb via the baked `MSB_PATH` convention (`doctor::msb_binary`), so the fleet needs NO msb on PATH at all. Host provisioning installs the profile pair (`workestrate` + `msb-wrapped`) and owns upgrades.
- **What the pin used to guarantee is replaced by a check, not a pin:** `workestrate doctor` gains a schema-window row comparing the ambient CLI's [MIN, MAX] against every registered fleet's declared `schema_version` (fail-closed, naming both sides), and `workestrate versions` prints the window. Version skew becomes visible instead of impossible-by-construction — the correct trade for shareable fleets, because skew is now a supported state with a clean error, not a relock.
- The justfiles' `nix develop <tool-flake> -c workestrate …` shims (`validate-config`, `home init`) collapse to bare `workestrate …` invocations.

## 8. Registry hierarchy: fleets pin workloads

"Home pins fleets" stands as-is (`workestrate.lock` `repos.*` url/ref/rev, ADR 0025 — verified mechanics). For the per-workload-repo roadmap (the tempo-buddy pilot's destination), re-evaluating the earlier sketch that put per-workload url+rev in the HOME registry: **rejected — it skips a level.** The home would have to know fleet internals (which workload repos a fleet composes), and a shared fleet would not carry its own composition. Under the axiom each level pins exactly one level down:

- Workload repos are pinned by the FLEET that composes them, in a **fleet-level `workestrate.lock`** (same format, `version = 1`, `[repos.<workload>] url/ref/rev`), reusing `config/lockfile.rs`. The CLI resolves a workload's declaring repo through the existing merge provenance, then reads the pin from THAT fleet's lock.
- Two pin KINDS coexist and must not be conflated: **repo pins** (lock files: home→fleet, fleet→workload-repo — which revision of a composable unit) and **artifact pins** (in-capsule `[image.sources]` url+rev+hash — the exact payload content, fixed-output). The capsule's artifact pins travel WITH the workload repo; the fleet's lock says which revision of that repo (hence which capsule, hence which artifacts).
- A consumer cloning a fleet gets the fleet's workload composition in the clone — the shareability property falls out of the placement.

The verbs (`workestrate workload add <url>` inside a fleet, lock generation/guardrails mirroring `home clone`) are deferred to the per-workload-repos phase; this ADR fixes the PLACEMENT only.

## 9. Reproducibility and builder drift

Without a tool pin, what guarantees the tool builds the same image? Precisely:

- **Invariant:** same capsule + same `builder_id` ⇒ same `outPath`. This is nix purity over narHash-pinned fetches and the tool-pinned nixpkgs/recipes — already the strongest reproducibility the current setup actually has (the fleet's `nixpkgs.follows = "workestrate/nixpkgs"` means the fleet never controlled the base packages anyway).
- **`builder_id`** = the tool's nixpkgs rev + the recipes-tree content hash, baked at tool build time, printed by `workestrate versions`, and RECORDED on every image record (`images.json` gains `tool_version` + `builder_id` alongside the existing drv_path/out_path provenance, spec §8).
- **Drift is DETECTED, not prevented — and detection already exists:** the content-addressed tag (ADR 0032 A2) changes iff the evaluated outPath changes. A tool upgrade that alters output surfaces as a new tag, a rebuild+load, and a provenance record attributing the image to the new builder_id. The old tag stays in the store until keep-last-N GC — rollback of the IMAGE is `msb load`-free tag selection.
- **Contract rule (§4):** semantics changes are schema-window bumps; byte-level base-package drift rides `builder_id`. Silent drift is the one unacceptable outcome and is structurally impossible: output change ⇒ tag change ⇒ recorded.
- **Golden fixtures:** the tool repo gains a CI check building a fixed fixture-capsule set (one per recipe, incl. a prepare+binary two-stage fixture mirroring pi) and asserting stable outPaths across the supported window. A tool change that alters a fixture's outPath without bumping the window fails CI. This is the "how do we NOTICE" mechanism, moved left of release.

## 10. Failure modes

| Failure | Behavior |
|---|---|
| Tool too old for capsule (declared > MAX) | Hard error at config load naming declared version, tool window, tool version; remediation: upgrade via host profile. The fleet-sharing case works by construction. |
| Capsule too old for tool (declared < MIN) | Hard error naming both sides + migration recipe; at the v1→v2 event a `workestrate migrate-capsules` rewriter ships with the window bump. |
| Missing `schema_version` | Warn + assume MIN (today's posture); hard-required from schema 2. Capsules in per-workload repos MUST declare it from introduction. |
| Source hash mismatch (stale/fakeHash) | The existing `classify_build_stderr` fixed-output row, now pointing at `workestrate workload update-hashes` (the declaring repo no longer owns the ritual). |
| Source unreachable / offline | `fetchTree` fixed-output failure → the existing OfflineFetch named row; the nix-absent D1 ladder (trust-with-note / hard error) is unchanged. Pre-realized stores keep working offline. |
| Builder drift (same capsule, different image across tools) | Visible by construction: new content-addressed tag + `builder_id` provenance; golden-fixture CI catches it pre-release; a semantics change without a window bump is a contract violation (process, enforced by the fixture check). |
| Ambient CLI absent from PATH in a fleet shell | Doctor row FAIL naming `host-provision.sh`; the justfile `workestrate …` invocations fail with the shell's own not-found — no silent fallback to a stale pinned copy (that failure mode is exactly what is being removed). |
| `flake://` capsule after schema 2 | Hard parse/validation error naming the field and the `source://` migration. |

## 11. Alternatives rejected

- **Status quo (fleet pins the tool).** The relock-tax is structural, sharing is broken, and the drift is already observed in practice (shadow pins `c3b18d4…` vs root lock `ba49ed9…`; duelbits a third rev).
- **Tool pins fleets (invert the input).** Backwards: the tool would need to know every fleet; composition belongs to the operator's home, not the tool.
- **Fleet keeps building; tool only loads.** Keeps fleet-side Nix complexity, the sources-as-derivations seam, and the devshell pin (the relock-tax survives). The CLI already owns eval/build/load/record — halving that ownership is the worst of both.
- **Capsule sources as flake URLs + a fleet-side flake.lock for sources only.** Retains a Nix dependency in every fleet for zero ownership gain; hash-in-capsule is simpler, shareable as plain data, and reviewable in a diff.
- **Passing the capsule via flake override-input or a generated consumer flake.** Data-via-input abuses the flake mechanism and reintroduces a lock to manage; `--expr` over a baked builder is the idiomatic fixed-output shape.
- **New top-level `[image.source]`/`[image.build]` sections instead of `[image.sources.<name>]` + `source://` refs.** A singleton section cannot serve `extra_contents` (prime has NO binary but two prepared inputs) and forces pin duplication across pi/tempo-buddy; the named map + closed reference scheme covers both and composes with layer merge.
- **Separate per-host builder service / remote builds.** Out of scope; the host already has nix.

## 12. Migration path

Order is dependency-driven; every phase is independently committable and rollback is `git revert` (fleet) or `nix profile rollback` (tool) at each step.

- **Phase 1 — tool, additive (no fleet change, nothing breaks).** Capsule schema gains `[image.sources]`, `source://` refs, the enrichment fields, the `lockfile` recipe param, `${source:<name>}` env placeholders (serde defaults; `flake://` untouched). Baked nix-lib + `build-capsule.nix` + exe-relative resolution. Evaluator/builder seams take the capsule path when `image.sources` is present, the legacy fleet-flake path otherwise. `ImageRecord` gains `tool_version`/`builder_id`; `versions` prints the window + builder_id; doctor gains the schema-window row; `SCHEMA_VERSION_MIN/MAX` consts (both 1); `workestrate workload update-hashes`; golden-fixture CI.
- **Phase 2 — fleet `personal`, one workload at a time** (tool from Phase 1 is installed on the host first). pi first (tempo-buddy inherits its source via layer merge), then tempest, then prime last (the hard case: `lockfile` param, koffi escape-hatch, wrapper→baked_files). Per workload: migrate the capsule, `workestrate workload build <name>`, exec acceptance. The fleet flake keeps working throughout (legacy path). Once all four (`workestrate-pi`, `workestrate-tempo-buddy`, `workestrate-prime`, `tempest`) build from capsules, ONE commit slims the flake (drop the workestrate input, sources map, hashes, image outputs, checks, prime-smoke), rewrites `load-images` to `workestrate workload build`, drops the two tool devshell packages, and simplifies the nix-develop shims. What breaks in that commit: `nix build .#workestrate-pi` muscle memory (covered by the justfile rewrite) and the stale-pin shadow at `workloads/tempo-buddy/flake.nix` (deleted — its purpose is served).
- **Phase 3 — `duelbits`.** Same shape, two images (`workestrate-pi`, `workestrate-prime`); benefits from personal's templates.
- **Phase 4 — per-workload repos.** tempo-buddy extracts to its own repo (capsule + assets + skills; the shadow pilot's structure MINUS its flake is the template), pinned by personal's fleet-level `workestrate.lock` (§8; CLI fleet-lock resolution lands here). The home↔config↔fleet renames batch here per the rename plan.
- **Phase 5 — schema 2.** Remove `flake://` parsing, the legacy fleet-flake build path, and the `lib.buildImagesFromConfig` export; missing `schema_version` becomes an error. Only when a real breaking change motivates it — the window mechanism does not require a near-term bump.

Rollback notes: through Phase 2's per-workload steps, old tool + old fleet remain self-consistent (the legacy path is live); the slimming commit is the point of no return for a fleet and is gated on all its images building from capsules. Tool rollback never strands a fleet: any tool with MAX ≥ 1 builds any v1 capsule.

## 13. Consequences

- Fleet repos become pure data + static kit: cloneable, shareable, reviewable; the tool rev disappears from every lock file but the host's.
- The relock-tax dies: a tool bump is `nix profile install` on the host, zero fleet commits.
- The capsule becomes the single source of truth for what an image IS (source pins, hashes, build parameters) — diffable TOML instead of fleet Nix code.
- The CLI gains a narrow impure-eval surface (capsule JSON path) and a shipped nix-lib — new tool-package surface to keep honest; the golden fixtures guard it.
- Version skew between ambient CLI and fleets becomes a supported, checked state (doctor row + window errors) instead of an impossibility — the deliberate trade for shareability.
- spec 21 §7's "No flake.nix in the declaring repo" row and ADR 0008/0025's consumption model are amended as noted in §1; ADR 0003 gains the closed `${source:<name>}` reference entry.
