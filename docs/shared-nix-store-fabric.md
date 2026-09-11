# Shared Nix Store Fabric for Workestrate

> Status: design exploration / architecture note, not a committed ADR.
>
> Goal: work toward a host and microVM topology where many Workestrate workloads reuse the same Nix artifacts, the same VM image bases, and as much physical RAM as is safely practical, without turning the shared state into one giant writable trust boundary.

## Executive summary

The interesting architecture is not simply "put Cachix in a microVM and mount its `/nix/store` everywhere".

The more robust shape is a small **build/cache/publication plane** plus many **consumer workload VMs**:

1. A dedicated builder/cache VM owns Nix builds and is the only ordinary component with Cachix write credentials.
2. Cachix is used as the durable binary-cache plane. Cachix itself is a hosted service; the VM runs Nix plus the Cachix client/daemon, not a self-hosted Cachix server.
3. A local publication store contains the subset of the Nix store that should be shared with workloads.
4. That publication store is frozen into **immutable generations**. This is necessary because Linux OverlayFS does not permit a lower directory to change while it is mounted as an OverlayFS lower layer.
5. Each workload VM receives one immutable generation read-only, ideally through virtio-fs, and constructs its own `/nix/store` as:

   `shared immutable lower + private writable upper -> merged /nix/store`

6. Nix inside the guest uses the experimental `local-overlay` store so its metadata model understands the same lower/upper split that OverlayFS provides physically.
7. VM/root image storage uses CoW independently. Current Microsandbox already has useful primitives here, including shared OCI image layers and flat root disks cloned with Linux `FICLONE` when available.
8. RAM sharing is a separate layer. The immediate path is **KSM** (Kernel Samepage Merging), which deduplicates identical anonymous guest RAM pages after the fact. True warm-template memory CoW requires VM snapshot/restore or fork semantics and is a substantially larger VMM feature.
9. The NixOS host should normally keep its own boot-critical local `/nix/store`. It can delegate all builds to the builder VM and use the same Cachix/publication infrastructure, but making the active host store itself depend on the guest overlay design is not worth coupling into the first version.

The resulting mental model is:

```text
                       durable / remote distribution
                    +-------------------------------+
                    |            Cachix             |
                    +---------------^---------------+
                                    |
                              push / substitute
                                    |
+-----------------------------------------------------------------------+
| NixOS host                                                            |
|                                                                       |
|  +---------------------------+       +------------------------------+ |
|  | builder/cache microVM     |       | store publisher             | |
|  |                           |       |                              | |
|  | nix-daemon                |------>| mutable publication store    | |
|  | remote builder endpoint   | copy  | /srv/.../live/nix/...       | |
|  | cachix uploader           |       |                              | |
|  | private build store       |       | freeze -> immutable gen N   | |
|  +---------------------------+       +---------------+--------------+ |
|                                                    |                  |
|                                              read-only export          |
|                                                    | virtio-fs         |
|       +-----------------------+--------------------+----------------+  |
|       |                       |                    |                |  |
|  +----v---------+        +----v---------+     +----v---------+      |  |
|  | workload VM |        | workload VM |     | workload VM |      |  |
|  |              |        |              |     |              |      |  |
|  | lower: gen N |        | lower: gen N |     | lower: gen N |      |  |
|  | upper: own   |        | upper: own   |     | upper: own   |      |  |
|  | /nix/store   |        | /nix/store   |     | /nix/store   |      |  |
|  +--------------+        +--------------+     +--------------+      |  |
|                                                                       |
|  root disks/images: reflink / shared OCI layers / other storage CoW   |
|  guest RAM: KSM first, true template-memory CoW later                  |
+-----------------------------------------------------------------------+
```

The core architectural rule is: **share immutable state aggressively; never share one writable Nix store among mutually independent workloads.**

---

## 1. Terminology and a useful correction

### 1.1 "Cachix VM" is shorthand, not literally a Cachix server

Cachix provides hosted Nix binary caches. A machine can push with `cachix push`, watch new store outputs, or use a post-build hook, and other machines can configure the Cachix cache as a substituter.

So the proposed infrastructure VM is more precisely a:

- Nix remote builder,
- Cachix uploader/client,
- optionally a store-publication source,
- optionally a local HTTP cache endpoint.

If a fully local binary-cache server is useful, that is a separate component. Harmonia is attractive for this topology because it can serve an existing Nix store directly over HTTP. Attic is another option, but it is a different storage architecture and currently still describes itself as an early prototype. Neither is required for the shared lower-store path.

### 1.2 KSM, not KMS

The RAM feature is **KSM, Kernel Samepage Merging**.

KSM scans memory regions explicitly marked mergeable, finds identical anonymous pages, replaces them with a shared write-protected page, and lets the kernel copy a page again on write.

This is not the same thing as starting N VMs from one warm parent snapshot:

- KSM: independent VMs boot normally, then identical anonymous pages converge later.
- Template/snapshot CoW: children start with the same physical memory pages immediately and diverge on write.

KSM is much easier to add to the existing libkrun/Microsandbox path. True memory CoW needs VM state capture and restore/fork semantics.

### 1.3 Three independent forms of sharing

It is useful not to collapse these into "CoW":

| Layer | Mechanism | What is shared | When copies happen |
| --- | --- | --- | --- |
| Nix store | Nix `local-overlay` + Linux OverlayFS | immutable store objects | guest-created/downloaded paths go to its upper |
| VM/root disk | reflink, shared OCI layers, qcow2/backing chain, snapshots | filesystem blocks | first write to a private clone/delta |
| Guest RAM | KSM | identical anonymous 4 KiB pages | page write after dedup |
| Warm VM RAM | snapshot mapped `MAP_PRIVATE` or equivalent | parent/template memory | first child write |

They complement each other. None replaces the others.

---

## 2. Why a live mutable shared lower store is the wrong shape

Nix 2.34 documents `local-overlay` as an experimental store type built around an OverlayFS lower and upper.

There are two relevant consistency rules:

1. Nix expects the lower store not to lose or mutate store objects.
2. Linux OverlayFS is stricter: **the lower store directory cannot change at all while an overlay using it is mounted**.

That means this is not a safe design:

```text
builder writes /nix/store continuously
            |
            +---- same live directory is lowerdir for VM A
            +---- same live directory is lowerdir for VM B
            +---- same live directory is lowerdir for VM C
```

Even though Nix store objects are immutable, adding a new path changes the lower directory and violates the OverlayFS requirement.

The primitive should instead be an immutable **store generation**:

```text
mutable builder/publication store
        |
        | publish
        v
immutable generation G42
        |
        +---- VM A lowerdir
        +---- VM B lowerdir
        +---- VM C lowerdir

builder continues changing elsewhere
```

A later publication creates `G43`. New workloads can use `G43`. Existing workloads keep `G42` until they are restarted or explicitly migrated.

This generation model also makes rollback, GC, observability, testing, and Workestrate planning much cleaner.

---

## 3. Reference topology

### 3.1 Components

#### Host

NixOS is the preferred host because it makes all of the following declarative:

- KVM and nested-virtualization policy,
- filesystems and Btrfs/XFS mount layout,
- Nix daemon and remote-builder settings,
- KSM sysfs/service configuration,
- store publisher services/timers,
- Workestrate and its fork pins,
- secrets via SOPS,
- observability and resource limits.

The design is still usable on a generic Linux host with KVM, Nix, systemd, and OverlayFS. NixOS mainly turns the host control plane into a reproducible flake/module instead of a pile of imperative setup.

#### Builder/cache VM

A trusted infrastructure workload, not an ordinary agent workload.

Responsibilities:

- run `nix-daemon`,
- expose a Nix remote-builder endpoint,
- build derivations requested by the host and workloads,
- use ordinary upstream substituters such as `cache.nixos.org`,
- push desired outputs to Cachix,
- never expose its Cachix write credential to ordinary workload VMs,
- optionally stage selected closures for publication.

For derivations requiring KVM, declare the builder as supporting `kvm` only when nested virtualization is deliberately enabled and tested. Otherwise omit that feature and let such derivations route elsewhere.

#### Store publisher

Owns a local store that exists specifically to become the shared lower layer.

It should not be the host's normal `/nix/store` and does not need to be the builder's complete store.

Responsibilities:

- receive selected closures from the builder or Cachix,
- keep a coherent Nix local-store root containing both store objects and Nix metadata,
- freeze immutable versions,
- maintain generation manifests and reference counts,
- expose only immutable generations to workload VMs,
- delete old generations only when no live workload pins them.

#### Workload VM

Receives:

- a CoW root image,
- one pinned read-only shared Nix generation,
- a private upper store,
- its own Nix overlay metadata/state,
- optionally remote-builder access,
- read-only Cachix credentials if the cache is private,
- no Cachix write credential by default.

---

## 4. Store publication strategies

There are two sane variants.

### 4.1 Isolation-first, recommended for the first implementation

The builder VM owns its private mutable `/nix`. The host owns a separate publication store.

Publication is roughly:

```text
builder private store
      |
      | nix copy selected closures
      v
host publication store: /srv/workestrate/nix/live
      |
      | atomic/read-only filesystem snapshot
      v
/srv/workestrate/nix/generations/<generation-id>
```

Advantages:

- VM compromise does not give the builder direct write access to the host's publication directory.
- Publication is an explicit trust boundary.
- Only selected closures need to be published.
- The host can validate/sign/inspect before making a generation visible.
- It works even if the builder's own root disk is just a private Microsandbox block image.

Cost:

- the builder store and publication store contain one additional local copy of shared paths.

That is still far better than N copies for N agent VMs.

### 4.2 Density-first, later optimization

Give the trusted builder VM writable virtio-fs access to a dedicated host Btrfs subvolume that is its `/nix` root or its publication store.

The host freezes read-only Btrfs snapshots from that subvolume.

Advantages:

- avoids the builder-store -> publication-store copy,
- snapshots are block-sharing CoW generations,
- a large amount of data can be published nearly instantly.

Requirements:

- the builder must be considered infrastructure-trusted,
- its host-visible path must be isolated in the VMM's host mount namespace,
- publication must quiesce writes long enough to snapshot a coherent store plus metadata,
- a read-only snapshot, never the live subvolume, is exposed to workloads.

This is an optimization, not a prerequisite.

### 4.3 Preferred host filesystem

For the publication tree, Btrfs is the cleanest initial choice because read-only subvolume snapshots map almost exactly onto store generations.

XFS with reflink support is also viable, but generation management is less directly expressed as a first-class subvolume snapshot.

A fully immutable generated image is another option:

- EROFS image,
- SquashFS image,
- read-only ext4 image.

Those are attractive for portability, but they add a materialization step. A Btrfs snapshot exposed read-only through virtio-fs is the most direct path on a NixOS/Linux host.

---

## 5. What exactly is in a published generation

Do not snapshot only `/nix/store` and then invent metadata separately.

A local Nix store includes filesystem objects plus store metadata. A generation should therefore represent a coherent **store root**, for example:

```text
/srv/workestrate/nix/live/
  nix/
    store/
    var/nix/
      db/
      ...
```

The publisher can populate it using a separate local Nix store root and `nix copy` from the builder.

Then snapshot the whole root after publication is complete.

A generation manifest should be explicit and machine-readable. Conceptually:

```toml
schema = 1
id = "sha256-..."
created_at = "2026-09-10T00:00:00Z"
system = "x86_64-linux"
nix_version = "2.34.9"

[top_level]
paths = [
  "/nix/store/...-workestrate",
  "/nix/store/...-pi",
  "/nix/store/...-rust-toolchain",
]

[store]
root = "/srv/workestrate/nix/generations/<id>"
closure_bytes = 0

[cache]
cachix = "rybskiworks"
```

The manifest is useful for:

- deterministic VM plans,
- pinning,
- GC/reference counts,
- rollbacks,
- debugging "why does this VM have path X?",
- benchmarking generation size,
- deciding whether two VMs can safely share a memory/template family.

A generation does not need the builder's whole store. Prefer publishing curated closures that are broadly reusable:

- Workestrate runtime,
- compilers/toolchains,
- agent executables,
- common language runtimes,
- common dev tools,
- base NixOS/Nix closures,
- known project dependencies when reuse is high.

Highly project-specific or short-lived outputs can remain in per-VM uppers or Cachix.

---

## 6. Guest `/nix/store`: local-overlay + OverlayFS

The guest gets the generation as a read-only mount, for example:

```text
/run/workestrate/nix-base/nix/store
/run/workestrate/nix-base/nix/var/nix/...
```

It also has private writable directories:

```text
/var/lib/workestrate/nix-overlay/upper
/var/lib/workestrate/nix-overlay/work
/var/lib/workestrate/nix-overlay/state
```

The physical OverlayFS mount is conceptually:

```bash
mount -t overlay overlay \
  -o lowerdir=/run/workestrate/nix-base/nix/store \
  -o upperdir=/var/lib/workestrate/nix-overlay/upper \
  -o workdir=/var/lib/workestrate/nix-overlay/work \
  /nix/store
```

Nix is then configured to treat the merged store as a local-overlay store, with the immutable generation as its lower store and the private directory as its upper layer.

The exact URI/config should be generated by Workestrate rather than hand-written into every image. The Nix manual's shape is:

```text
local-overlay://?root=<merged-root>&lower-store=<lower-store-root>&upper-layer=<upper-dir>
```

Important invariants:

- lower is mounted and exported read-only,
- lower never changes for the lifetime of the guest overlay,
- every VM gets a separate upper and Nix state DB,
- upper and OverlayFS workdir live on the same filesystem,
- Nix `check-mount` remains enabled unless there is a demonstrated reason to disable it,
- upper GC never attempts to mutate the lower generation,
- generation deletion is controlled by the host publisher, not a guest.

### 6.1 GC warning on current Nix

As of September 2026, Nix issue #16269 remains open for `local-overlay`: finite-limit GC paths, including automatic `min-free`/`max-free` GC, can mis-handle `bytesFreed` for lower-only paths and stop before reclaiming upper-store garbage.

For an initial Workestrate implementation:

- pin a Nix version with a verified fix once available, or carry the tiny patch,
- otherwise do not rely on automatic finite-limit GC for overlay guests,
- ephemeral agents can simply delete their whole upper when the workload dies,
- persistent dev VMs can use explicitly tested unlimited GC and/or upper-volume rotation,
- provide the `remount-hook` behavior Nix expects when GC operations require OverlayFS remounting.

This is exactly the sort of low-level behavior the Workestrate black-box/property test harness should continuously exercise.

---

## 7. Build and cache flows

There should be more than one workload policy.

### 7.1 Consumer agent

Normal autonomous agent:

```text
lower generation -> immediate reused closure
Cachix            -> cache miss fallback
remote builder    -> derivation absent from both
private upper     -> receives downloaded or returned paths
```

Policy:

- local builds disabled or heavily restricted,
- remote builder configured,
- no Cachix write token,
- upper usually ephemeral,
- optional project workspace persistent separately from `/nix/store`.

### 7.2 Development VM

Interactive/persistent dev workload:

- same lower generation,
- persistent upper,
- local builds may be allowed,
- remote builder still preferred for expensive builds,
- promotion to the shared publication set is explicit.

Do not let every dev/agent upper automatically become global shared state. Promotion should be a Workestrate action that either:

1. asks the trusted builder to reproduce the derivation, or
2. imports a result into a trusted staging store, verifies it, signs/pushes it, and publishes it.

### 7.3 Infrastructure builder

The only default writer to the shared Cachix cache.

It can run a Cachix post-build hook or daemon-based upload path so successful builds are asynchronously pushed.

`builders-use-substitutes = true` should be enabled so the remote builder fetches its own dependencies from `cache.nixos.org`, Cachix, or a local cache instead of forcing the client to upload every input first.

---

## 8. Host behavior

### 8.1 Recommended NixOS host model

The host keeps a normal local store because the running NixOS system, bootloader generations, host services, and recovery path depend on it.

Centralize **building**, not necessarily every physical store byte.

A host can force builds onto the builder VM with the normal Nix remote-builder mechanism:

```nix
{
  nix.distributedBuilds = true;

  nix.settings = {
    builders-use-substitutes = true;
    # Set to 0 if this host should never build locally.
    max-jobs = 0;
  };

  nix.buildMachines = [
    {
      hostName = "workestrate-builder";
      sshUser = "nixbuilder";
      sshKey = "/run/secrets/workestrate-builder-key";
      system = "x86_64-linux";
      protocol = "ssh-ng";
      maxJobs = 16;
      supportedFeatures = [ "big-parallel" ];
      # Add "kvm" only when nested KVM is actually available in the builder.
    }
  ];
}
```

This gives the desired operational behavior:

- evaluation/orchestration can happen on the host,
- build CPU/RAM usage happens in the builder VM,
- results are copied back to the host when the host needs them,
- the builder also pushes reusable outputs to Cachix,
- the same outputs can be included in the next shared generation.

It does **not** force the host OS to boot from a remote or experimental overlay store.

### 8.2 Could the host itself use the shared local-overlay store?

Technically, this can be explored later. Architecturally, it should be a separate milestone.

Problems introduced by making the active NixOS host use the same overlay design include:

- boot/recovery coupling,
- generation lifetime ordering during `nixos-rebuild`,
- host GC becoming dependent on local-overlay edge cases,
- the host becoming both producer and consumer of the same generation mechanism,
- harder rescue semantics when the publisher/builder path is broken.

The expected gain is also smaller because there is only one host but potentially dozens or hundreds of workload VMs.

So the first design should optimize the multiplicative part: **N workload VMs**.

### 8.3 Local cache endpoint

For a single host, sending every miss out to Cachix and back is unnecessary if the artifact is already present locally.

Optional optimization:

- run Harmonia over the publication store or another local cache store,
- give it higher substituter priority than Cachix,
- keep Cachix as durable/off-host distribution and recovery.

This is not required for the first shared-lower implementation because lower-store hits are already local.

---

## 9. Builder VM shape

A builder image should itself be declared by a flake.

It can be a full NixOS VM or a Nix-generated Linux rootfs suitable for the selected Workestrate backend. The important property is not the branding of the guest OS, but that the build environment is reproducible and contains:

- Nix daemon,
- build users,
- SSH or a brokered Nix remote-store transport,
- Cachix client,
- SOPS/secret injection integration,
- resource controls,
- storage volume for the private build store,
- network policy allowing required source/cache endpoints,
- optional nested KVM.

If Workestrate eventually owns a first-class guest-control channel, the remote builder does not have to be exposed as a normal network service to the LAN. The host can bridge the Nix builder transport over a scoped vsock/SSH endpoint.

Write credentials should be injected only into this infrastructure workload. Read-only private-cache auth can be given to consumers separately.

---

## 10. VM/root image CoW

This part is less speculative than the memory side because current Microsandbox already has useful storage sharing.

Current Microsandbox supports:

- content-addressed, shared read-only OCI image layers,
- a per-sandbox writable root layer,
- a flat OCI root disk mode that materializes a reusable ext4 base,
- `clone=auto`, which prefers a native CoW clone and falls back to sparse copy,
- `clone=reflink`, which requires native clone support,
- Linux native cloning through `FICLONE`.

For a NixOS/Linux Workestrate host, prefer a reflink-capable host filesystem and use these existing root-disk primitives rather than inventing a second root-image CoW system immediately.

Longer term, qcow2 backing chains can provide filesystem-independent block deltas, but Microsandbox's managed snapshot system still treats snapshots as disk-only artifacts and there is an open design space around storage-efficient snapshot chains. Reflinks are the simpler Linux-first primitive today.

The shared Nix store also makes the root image smaller in concept:

- root image: kernel-facing userspace/init, mount tooling, Nix client/daemon pieces, certificates, Workestrate guest glue,
- shared generation: large reusable packages and toolchains,
- private upper: workload-specific store additions.

This is preferable to the older Workestrate workaround of baking Nix-built agent artifacts directly into every Microsandbox image because `/nix/store` bind mounting failed on the old runtime path.

---

## 11. RAM sharing

### 11.1 Phase 1: KSM

KSM is the pragmatic first implementation.

Required pieces:

1. Host kernel has `CONFIG_KSM`.
2. Host enables and tunes `ksmd` through `/sys/kernel/mm/ksm/*` or an equivalent NixOS service/module.
3. The VMM marks the eligible anonymous guest-memory mapping with `madvise(..., MADV_MERGEABLE)`.
4. Workestrate exposes this as an explicit backend capability and policy, not an unconditional global behavior.

Proposed configuration semantics:

```toml
[memory]
sharing = "ksm"          # "none" | "ksm" | future "template-cow"
trust_domain = "agents"  # policy label, not a kernel primitive
```

KSM is system-wide for marked memory. A trust-domain label therefore means Workestrate only marks memory mergeable for workloads that policy allows to participate. Do not enable KSM across mutually untrusted tenants by default because memory deduplication has a history of timing/side-channel concerns.

Metrics should include:

- `pages_shared`,
- `pages_sharing`,
- `pages_unshared`,
- `full_scans`,
- scan CPU cost,
- per-VMM RSS/PSS,
- guest working-set behavior.

KSM is particularly interesting after many VMs boot from identical userspace and run similar agent/tooling stacks. It is not instant: dedup occurs as the scanner converges.

### 11.2 virtio-fs DAX experiment

libkrun exposes a virtio-fs DAX window size in its lower-level API.

A read-only Nix store is an unusually good candidate to benchmark with DAX because its content is immutable and read-heavy. In principle, avoiding copies into independent guest page caches can reduce duplicate memory before KSM is even involved.

This should be an experiment, not a design dependency, because Microsandbox does not currently expose a clean per-mount DAX policy through its public workload model and some cross-process/custom filesystem backend designs cannot support DAX mappings.

### 11.3 Phase 2: true warm-template memory CoW

The target behavior is closer to:

```text
warm template VM memory
      |
      +---- child A: MAP_PRIVATE/shared pages -> private on write
      +---- child B: MAP_PRIVATE/shared pages -> private on write
      +---- child C: MAP_PRIVATE/shared pages -> private on write
```

This gives two benefits KSM cannot:

- memory sharing exists immediately at fork/restore time,
- a warm userspace/agent can start without repeating full boot/init.

But it requires substantially more machinery:

- pause/quiesce VM,
- capture vCPU state,
- capture device state,
- capture guest RAM,
- restore into multiple children,
- map parent RAM privately or otherwise implement page-level CoW,
- regenerate per-child identity,
- reconnect virtio-fs/network/vsock devices safely,
- handle clocks, entropy, VMGenID-like semantics, sockets, and guest daemons,
- ensure writable disks also fork from a CoW point consistent with RAM.

Current Microsandbox snapshots explicitly do **not** capture memory, processes, or network state. Its public snapshot contract already has a `resumable` concept, but the local runtime reports it unsupported until VM pause/resume state capture exists.

Current upstream libkrun also still has snapshot/restore work in progress rather than a mature Linux/KVM fork primitive.

Therefore:

- do KSM first,
- keep Workestrate's backend capability model able to express future `memory_snapshot_restore` / `memory_template_cow`,
- use Clone/forkd-style systems as implementation references,
- only fork libkrun deeply for template memory if measurements show KSM + shared store + root-disk CoW are insufficient.

A future alternate Workestrate backend may be cleaner than forcing every VMM to emulate the same mechanism.

---

## 12. Security model

The density optimization must not undo Workestrate's isolation goals.

### 12.1 Shared lower is read-only at every layer

Do not rely only on `mount -o ro` inside a guest.

The host/VMM export itself should be read-only so a compromised guest kernel cannot remount it writable.

Upstream libkrun has `krun_add_virtiofs3(..., read_only)`. The current `rybskiworks/libkrun` public header on the `krun` branch exposes `krun_add_virtiofs` and `krun_add_virtiofs2`, but not that newer read-only variant, so this is a concrete fork gap to reconcile.

### 12.2 VMM host namespace

The virtio-fs server sees host paths. Even with path-containment logic, the VMM process should run in a host mount namespace where only the exact generation directory and other explicitly brokered paths are visible.

The goal is defense in depth:

```text
guest compromise
  -> virtio-fs implementation
    -> restricted host mount namespace
      -> immutable generation only
```

### 12.3 Cachix credentials

- write token/key: builder/cache VM only,
- read token: only if private cache and only where needed,
- public workloads: preferably public cache or brokered read access,
- agents cannot directly poison the shared cache.

### 12.4 Promotion is privileged

A workload producing `/nix/store/foo` does not make `foo` globally trusted.

Promotion should cross a policy boundary controlled by Workestrate/builder infrastructure.

### 12.5 KSM trust boundary

Only mark memory mergeable for workloads allowed to share the same deduplication domain. The easiest safe default is "same user / same Workestrate fleet / same trust level only".

---

## 13. Workestrate integration

This should not become a pile of Microsandbox-specific flags in workload code.

The right abstraction is a Workestrate storage/build plan with backend capabilities.

### 13.1 Proposed Workestrate concepts

Conceptual config:

```toml
[store]
mode = "overlay"
generation = "latest-compatible"

[store.upper]
lifecycle = "ephemeral"
size = "20GiB"

[store.builder]
name = "nix-builder"
strategy = "remote-first"

[store.cache]
cachix = "rybskiworks"
local_binary_cache = true

[vm.root]
clone = "reflink-preferred"

[vm.memory]
sharing = "ksm"
```

The exact schema can evolve, but the control plane should resolve this into a backend-neutral plan resembling:

```rust
struct StorePlan {
    generation: StoreGenerationId,
    lower: ReadOnlyStoreMount,
    upper: UpperStorePlan,
    nix_store: LocalOverlayPlan,
    builder: Option<BuilderEndpoint>,
    substituters: Vec<Substituter>,
}

struct BackendCapabilities {
    readonly_directory_mount: bool,
    reflink_root_clone: bool,
    block_cow_root: bool,
    nested_kvm: bool,
    ksm_guest_memory: bool,
    vm_snapshot_restore: bool,
    template_memory_cow: bool,
    virtiofs_dax: bool,
}
```

Workestrate decides what semantics are required. A backend reports whether it can satisfy them.

### 13.2 Lifecycle

`workestrate plan` should be able to say, before boot:

```text
workload: ganymede
backend: microsandbox
store generation: sha256:...
lower: read-only virtio-fs
upper: ephemeral 20 GiB
builder: nix-builder over brokered SSH/vsock
cache: local -> Cachix -> cache.nixos.org
root clone: reflink
memory sharing: KSM
nested virtualization: disabled
```

VM creation then becomes deterministic:

1. resolve/pin generation,
2. increment generation lease/reference,
3. create root-disk clone,
4. create upper store volume,
5. attach read-only generation,
6. boot guest,
7. guest init mounts lower and OverlayFS,
8. start Nix daemon/client configuration,
9. start workload,
10. on destroy, release generation lease and delete ephemeral upper.

---

## 14. Current `rybskiworks` stack: concrete gaps and required changes

This is where the design intersects the repositories as they exist today.

### 14.1 `rybskiworks/workestrate`

Current default branch is still the older implementation centered on Microsandbox 0.5.x-era behavior. It contains the workaround where Pi's Nix-built binary was baked into the sandbox image because the runtime could not bind-mount the host `/nix/store` path and returned `Permission denied`.

Needed:

- update the Microsandbox integration to the current `rybskiworks/microsandbox` API line,
- introduce backend capabilities rather than hard-coded image/mount assumptions,
- add a first-class infrastructure workload for the Nix builder/cache plane,
- add store-generation registry and leases,
- add lower-store + private-upper plan generation,
- add guest init/bootstrap support for OverlayFS + Nix local-overlay,
- add promotion/publish commands,
- move reusable agent artifacts back to ordinary Nix closures instead of baking every artifact into each OCI image where possible,
- expose root clone and memory-sharing policy in workload/fleet configuration,
- add black-box tests covering all of this.

### 14.2 `rybskiworks/microsandbox`

Current fork is on Microsandbox 0.6.17 and already contains much more useful storage machinery than Workestrate's current pin expects:

- volume model,
- read-only mounts,
- disk-only snapshots,
- flat OCI root disks and reflink clone mode,
- current root-disk abstractions,
- the public future `resumable` snapshot contract.

However, its workspace currently pins published `msb_krun = 0.1.32` / `msb_krun_utils = 0.1.32` rather than automatically consuming `rybskiworks/libkrun` HEAD.

That means a feature added only to `rybskiworks/libkrun` does not magically reach Microsandbox. The fork stack needs an explicit dependency strategy:

- publish/version the forked `msb_krun` line, or
- use a `[patch.crates-io]`/git pin in the rybskiworks Microsandbox fork, or
- make the Nix flake patch the dependency reproducibly.

Do not leave this as an implicit local Cargo override.

Microsandbox changes likely needed:

- make read-only virtio-fs a hard backend guarantee for the shared store mount,
- surface a per-mount DAX size if the DAX experiment is pursued,
- surface a `mergeable guest RAM` / KSM option once libkrun supports it,
- keep root-disk reflink capability discoverable,
- expose nested-KVM capability consistently for builder workloads,
- eventually surface resumable VM snapshot/restore if/when libkrun supports it,
- ensure the runtime child process/mount namespace can see exactly the published generation path and not arbitrary host storage.

A dedicated generic "Nix store" mount type is not strictly required. A secure read-only directory mount is sufficient for the first version. Workestrate can own the higher-level semantics.

### 14.3 `rybskiworks/libkrun`

Useful work already exists in this fork, including the Nix build work and unique per-VM guest CID support.

For this design, the priority list is:

#### A. Reconcile true read-only virtio-fs

Current upstream libkrun exposes `krun_add_virtiofs3(..., shm_size, read_only)`.

The current `rybskiworks/libkrun` public header does not expose that variant. Port/reconcile it into the `msb_krun` fork line and bubble it into Microsandbox.

This is more important than optimization work because the shared store must be fail-closed read-only.

#### B. KSM opt-in

Add an explicit VM-memory option that, on Linux, applies `MADV_MERGEABLE` to eligible anonymous guest RAM.

Requirements:

- off by default,
- error clearly when unsupported,
- report effective state,
- interact correctly with NUMA placement already present in the fork,
- do not silently mark non-RAM/DAX/device mappings,
- add native KVM tests and host metrics verification.

API shape could be something like a builder policy rather than a raw `madvise` flag, because later memory-sharing modes may exist.

#### C. Snapshot/restore later

Do not make the first shared-store milestone wait for this.

When pursuing true memory CoW, libkrun needs a coherent Linux/KVM snapshot/restore story including device state. Upstream is still actively working in this area.

Unique guest CIDs, already added in the fork, become particularly useful when restoring/forking multiple children from a common parent, but Microsandbox must stop discarding that identity if Workestrate needs to broker connections using it.

### 14.4 `rybskiworks/libkrunfw`

No KSM change belongs here because KSM is a host-memory feature.

The guest kernel/firmware must, however, provide everything the store design needs:

- virtio-fs,
- OverlayFS,
- ext4 for private upper/root disks if used,
- any EROFS/SquashFS support if immutable store images are explored,
- nested KVM guest support only for builder profiles that require it.

Virtio-fs is already fundamental to Microsandbox. OverlayFS support should be verified explicitly in the pinned firmware config instead of assumed. If missing, enable it in the libkrunfw fork and test the exact local-overlay mount sequence in CI.

### 14.5 Nix package/version

The architecture currently depends on an experimental Nix store type and must therefore pin Nix intentionally.

Do not simply inherit whatever Nix happens to be on the host/guest.

The Workestrate flake should carry:

- a known-good Nix version,
- the `local-overlay-store` experimental feature,
- a patch for #16269 if the pinned release still contains it,
- integration tests for GC, store queries, builds, substitutions, and daemon restart.

---

## 15. Non-NixOS hosts

### Generic Linux

Still viable if the host provides:

- KVM,
- Linux OverlayFS in guests,
- a reflink/snapshot-capable host filesystem if storage CoW is desired,
- Nix,
- systemd or another service supervisor,
- permissions for virtio-fs/mount namespace setup.

Workestrate can generate systemd units/mounts or manage them directly.

The loss compared with NixOS is primarily declarative integration, not a fundamental runtime capability.

### macOS

The host itself cannot use Linux OverlayFS, but Linux guests can still use a local-overlay Nix store if their lower store can be presented through the VMM.

Host-side root-disk CoW uses the platform's native clone primitive rather than Linux `FICLONE`.

KSM is Linux-specific, so the RAM-sharing phase would require a different host mechanism.

### Windows

Same conceptual split: guest Linux can use OverlayFS, but host storage cloning and RAM-sharing primitives differ.

This design should therefore model capabilities rather than make `btrfs + KSM + FICLONE` part of Workestrate's universal contract.

The Linux/NixOS backend can be the reference and most capable implementation.

---

## 16. Generation update and GC policy

Generations should behave like immutable release artifacts.

Example lifecycle:

```text
G40  refs=0  eligible for delete
G41  refs=3  pinned by running VMs
G42  refs=11 current default
G43  building/publishing
```

Rules:

- never mutate a published generation,
- never delete a generation with a live lease,
- current default is only a pointer/manifest update,
- existing VMs do not silently change lower generations,
- failed publication never changes the current pointer,
- publisher GC is separate from guest upper GC,
- Cachix retention is separate again.

This also fits future Yggdrasil/checkpoint semantics naturally: a workload snapshot/checkpoint can record the exact store generation ID it depends on.

---

## 17. Failure behavior

The design should degrade predictably.

### Cachix unavailable

- lower generation still works,
- local cache may still work,
- builder can still build if sources are available,
- pushes queue/retry,
- new uncached substitutions may fail depending on egress policy.

### Builder unavailable

- existing lower generation works,
- Cachix substitutions work,
- workloads can continue if everything needed exists,
- local building is allowed only for policies that explicitly enable it.

### Publisher unavailable

- running VMs remain pinned to already-mounted generations,
- new VM creation can use the latest existing generation,
- only publishing/promoting new shared closures is blocked.

### Generation corrupted

- detect using Nix metadata/signatures plus generation manifest/integrity checks,
- mark generation invalid,
- refuse new leases,
- roll current pointer back,
- preserve evidence until diagnostics complete.

---

## 18. Testing and proof that sharing actually works

This architecture deserves black-box tests, not only unit tests.

### Store semantics

- two VMs mount the same generation,
- both can query lower paths through Nix,
- VM A building/installing a new path only changes A's upper,
- VM B cannot see A's upper,
- neither can mutate lower,
- deleting A leaves lower and B unchanged,
- a new G+1 does not alter running G guests,
- GC cannot delete lower paths,
- patched/current GC correctly reclaims upper paths.

### Publication

- publish selected closure,
- verify closure property,
- snapshot/freeze,
- crash publisher at every step and prove current pointer remains valid,
- lease/release concurrent generations,
- reject deletion with live lease.

### Remote build

- host with `max-jobs=0` builds on builder,
- guest consumer builds on builder,
- builder uses substituters itself,
- returned output lands in guest upper,
- builder pushes output to Cachix,
- next generation can include it.

### Storage CoW

Measure actual allocated blocks, not apparent file size:

- N identical VM roots,
- write divergence in one child,
- verify only changed extents allocate,
- compare reflink vs forced copy.

### KSM

- start N identical workloads,
- wait for convergence,
- inspect `/sys/kernel/mm/ksm` counters,
- record host PSS/RSS before/after,
- dirty memory in one VM and verify CoW separation,
- benchmark CPU cost and convergence time,
- prove workloads not configured for sharing are not marked mergeable.

### True memory CoW, future

Acceptance should be much stricter:

- identical parent page initially one physical page,
- child write allocates only that child's page,
- device/vsock/network identities are unique,
- clock and entropy semantics sane,
- storage and memory point-in-time state consistent,
- no child can influence siblings through writable parent state.

---

## 19. Observability

Workestrate should make density visible.

Useful host-level metrics:

```text
workestrate_store_generation_bytes{generation=...}
workestrate_store_generation_leases{generation=...}
workestrate_store_upper_bytes{workload=...}
workestrate_nix_cache_hit_total{source=lower|local-cache|cachix|builder}
workestrate_builder_seconds
workestrate_builder_queue_depth
workestrate_root_reflink_allocated_bytes{workload=...}
workestrate_vm_rss_bytes{workload=...}
workestrate_vm_pss_bytes{workload=...}
workestrate_ksm_pages_shared
workestrate_ksm_pages_sharing
workestrate_ksm_full_scans
```

The useful headline is not "100 VMs use 100 x the image/store/RAM". It becomes measurable as:

```text
physical storage ~= one shared generation
                 + one shared/root-image base
                 + sum(private changed blocks)
                 + sum(private Nix uppers)

physical RAM ~= non-shareable VM overhead
             + unique working sets
             + shared/DAX-backed file pages
             + KSM/template-shared anonymous pages
```

That is the density model Workestrate should optimize.

---

## 20. Staged implementation plan

### Stage 0: prove Nix local-overlay in one Linux VM

No Cachix, no KSM.

- create a separate lower Nix store root,
- freeze it read-only,
- expose through current Microsandbox,
- attach private upper,
- boot two guests,
- run Nix queries/builds,
- test teardown and GC.

This proves the hardest Nix/OverlayFS semantics before adding infrastructure.

### Stage 1: publication generations

- host publication store,
- Btrfs read-only snapshots,
- generation manifests,
- leases,
- atomic current pointer,
- guest generation pinning.

### Stage 2: builder/cache VM

- remote Nix builder,
- Cachix push path,
- read-only consumers,
- host remote-build config,
- explicit promotion into publication store.

### Stage 3: Workestrate first-class integration

- `StorePlan`,
- backend capabilities,
- config schema,
- NixOS module/flake outputs,
- guest init generation,
- CLI status/plan/publish/gc commands,
- black-box harness coverage.

### Stage 4: maximize storage sharing

- migrate Workestrate to current Microsandbox root-disk clone primitives,
- reflink root images,
- trim baked artifacts from workload images,
- optional local Harmonia cache,
- density-first builder/publication topology if worthwhile.

### Stage 5: KSM

- libkrun `MADV_MERGEABLE` feature,
- Microsandbox option/capability,
- Workestrate trust policy,
- NixOS KSM service/tuning,
- benchmarks and safety gates.

### Stage 6: warm-template memory CoW research

Only after measuring stages 0-5.

Compare:

- extending libkrun snapshot/restore,
- process/template fork semantics,
- a Clone-like backend,
- a Firecracker/forkd-like backend,
- userfaultfd/lazy restore approaches.

The Workestrate API should make this replaceable rather than bake one VMM's mechanism into the product model.

---

## 21. Concrete fork dependency order

If implementing on the current rybskiworks stack, do the lower layers in this order:

```text
1. Nix pin / local-overlay GC fix
2. libkrun: true RO virtio-fs (port/reconcile)
3. libkrunfw: verify OverlayFS kernel config
4. rybskiworks/microsandbox: consume rybskiworks msb_krun build reproducibly
5. microsandbox: expose required RO mount capability
6. workestrate: generation + upper-store orchestration
7. builder/cache workload + publisher
8. storage-CoW integration
9. libkrun KSM
10. microsandbox/workestrate KSM bubbling
11. later snapshot/restore/template-memory CoW
```

Do not start by implementing true memory fork. It is the deepest change and produces less immediate value than fixing the store/image duplication first.

---

## 22. Decisions I would make for the first serious prototype

If building this on the current machine today:

- **Host:** NixOS.
- **Host filesystem for Workestrate state:** Btrfs.
- **Builder:** dedicated infrastructure microVM managed by Workestrate.
- **Builder store:** private initially.
- **Cachix:** durable off-host cache, builder is the only writer.
- **Local published store:** separate host local-store root populated by `nix copy`.
- **Generation mechanism:** read-only Btrfs subvolume snapshots.
- **Guest lower:** generation directory over true read-only virtio-fs.
- **Guest upper:** separate ephemeral/persistent ext4-backed or host directory-backed storage depending threat/performance requirements.
- **Guest Nix:** pinned current Nix with `local-overlay-store` enabled and #16269 fixed/patched.
- **Root image:** current Microsandbox flat/reflink mode where compatible.
- **RAM:** no special sharing for Stage 0-3; then KSM opt-in.
- **Warm memory fork:** explicitly deferred.
- **Host Nix:** normal local host store, all expensive builds optionally forced to remote builder with `max-jobs = 0`.
- **Local HTTP cache:** optional Harmonia after the basic path works.

That gets most of the practical density benefit without making the system depend on immature VM-memory snapshot semantics.

---

## 23. Open questions worth resolving with benchmarks

1. Is virtio-fs or a read-only block image faster for a very large Nix lower store under agent-style metadata-heavy workloads?
2. Does virtio-fs DAX materially reduce host RSS for shared Nix-store executable/library pages with libkrun?
3. How much memory does KSM actually recover after booting 10, 50, and 100 near-identical Workestrate VMs?
4. What is KSM convergence time and CPU cost under active coding-agent workloads?
5. Is a separate publication store's extra copy significant enough to justify the density-first builder-mounted-host-subvolume topology?
6. What should a store generation contain: broad fleet base, per-project bases, or composable layered generations?
7. Should persistent dev VMs promote frequently used upper paths automatically by derivation identity, or should promotion stay explicit?
8. Does the current Microsandbox virtio-fs path remain stable under Nix's directory/stat-heavy access pattern at high concurrency?
9. Should store publication happen eagerly after every important build or batch into epochs?
10. At what VM count does true warm-template memory CoW become worth carrying a VMM-level fork versus KSM?

---

## 24. References and current upstream state

Nix:

- Experimental local-overlay store, Nix 2.34.9: https://nix.dev/manual/nix/2.34/store/types/experimental-local-overlay-store
- Distributed builds setup: https://nix.dev/tutorials/nixos/distributed-builds-setup.html
- Current local-overlay GC bug #16269: https://github.com/NixOS/nix/issues/16269

Cachix:

- Cachix overview: https://www.cachix.org/
- Getting started: https://docs.cachix.org/getting-started
- Pushing paths / watch-store: https://docs.cachix.org/pushing
- Hydra/post-build-hook example: https://docs.cachix.org/continuous-integration-setup/hydra
- Security model: https://docs.cachix.org/security

Local cache alternatives:

- Harmonia: https://github.com/nix-community/harmonia
- Attic: https://github.com/zhaofengli/attic

Microsandbox:

- Filesystem/image security model: https://github.com/superradcompany/microsandbox/blob/main/docs/security/filesystem.mdx
- Root-disk clone modes: https://github.com/superradcompany/microsandbox/blob/main/docs/cli/sandbox-commands.mdx
- Snapshots are currently disk-only: https://github.com/superradcompany/microsandbox/blob/main/docs/sdk/typescript/snapshots.mdx
- Storage-efficient snapshot chains issue: https://github.com/superradcompany/microsandbox/issues/1278

libkrun / memory:

- Upstream libkrun API, including read-only virtio-fs: https://github.com/libkrun/libkrun/blob/main/include/libkrun.h
- libkrun snapshot/restore RFC/feature issue #756: https://github.com/libkrun/libkrun/issues/756
- Linux KSM documentation: https://docs.kernel.org/admin-guide/mm/ksm.html

rybskiworks forks:

- https://github.com/rybskiworks/workestrate
- https://github.com/rybskiworks/microsandbox
- https://github.com/rybskiworks/libkrun
- https://github.com/rybskiworks/libkrunfw

---

## Bottom line

The idealized system is not one enormous writable `/nix/store` mounted into every VM.

It is a hierarchy of sharing:

```text
build once in trusted builder
        |
        +-> push durable result to Cachix
        |
        +-> publish selected closure into immutable local store generation
                         |
                         +-> share generation RO across many VMs
                                      |
                                      +-> private Nix OverlayFS upper per VM

shared root image/base -> storage CoW per VM
shared anonymous RAM   -> KSM first
warm parent RAM        -> true VM CoW later
```

That gives Workestrate the useful property we actually want: **a fleet can behave as if every workload owns a full Nix machine while the host physically pays for common state as close to once as the underlying isolation layers allow.**
