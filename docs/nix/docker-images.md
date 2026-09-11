---
type: Reference
resource: https://nixos.org/manual/nixpkgs/stable/#sec-pkgs-dockerTools
title: Docker Images
description: Building reproducible Docker/OCI images with Nix dockerTools — buildImage, buildLayeredImage, streamLayeredImage, pullImage, layer strategy, image loading, pinning, and Microsandbox compatibility.
tags: [nix, docker, oci, dockerTools, buildImage, streamLayeredImage]
timestamp: 2026-07-24T01:30:00Z
---

# Docker Images

## Purpose

Provide concrete, repo-independent guidance for building reproducible
Docker/OCI images with Nix `pkgs.dockerTools` instead of Dockerfiles. Covers
the four primary functions — `buildImage`, `buildLayeredImage`,
`streamLayeredImage`, and `pullImage` — their attribute surfaces, the layer
strategy, image loading, nixpkgs pinning, runtime dependency selection
(`cacert`, `busybox`, `fakeNss`), and Microsandbox compatibility. This
document is intended as generic reference material for future AI coding
agents working in any Nix flake. Project-specific examples from
`ai-workbench` are included as real-world illustrations, not as the
authoritative scope.

Agents should use this document as the authoritative reference when creating
or modifying a Nix-built Docker image, choosing between `streamLayeredImage`
and `buildLayeredImage`, wiring `contents`/`copyToRoot`, deciding whether
`runAsRoot` (KVM) or `fakeRootCommands` (no KVM) is appropriate, pinning a
base image with `pullImage`, or diagnosing a runtime failure caused by a
missing glibc, CA roots, or uid resolution.

## Sources used

- Crawl file: `docs/nix/.crawl/68-nixpkgs-dockerTools.md` —
  <https://nixos.org/manual/nixpkgs/stable/#ssec-pkgs-dockerTools>
- Skill file: `.agents/skills/nix-docker-images/SKILL.md` — existing docker
  images skill
- Project file: `nix/packages/pi-image.nix` — real image build
  (workestrator-pi)
- Project file: `nix/packages/tempest-image.nix` — real image build (tempest)
- Project file: `nix/lib/recipes/nix-layered.nix` — layered image recipe
  abstraction

## Core guidance

### `pkgs.dockerTools` overview

`pkgs.dockerTools` is a set of functions for creating and manipulating
Docker images according to the Docker Image Specification v1.3.1. From the
nixpkgs manual (crawl 68):

> "`pkgs.dockerTools` is a set of functions for creating and manipulating
> Docker images according to the Docker Image Specification v1.3.1. Docker
> itself is not used to perform any of the operations done by these
> functions." (crawl 68)

Nix builds images as derivations, not via Dockerfiles. The reproducibility
advantage is structural: the same nixpkgs pin produces byte-identical image
bytes, because every store path is content-addressed and the default
`created` timestamp is the static `"1970-01-01T00:00:01Z"`. There is no
`RUN apt-get update` non-determinism and no Docker daemon is required during
the build.

The four primary functions are:

| Function            | Output                                  | Layers     | KVM?  | Default? |
|---------------------|-----------------------------------------|------------|-------|----------|
| `buildImage`        | Compressed tarball in the Nix store     | Single     | Only if `runAsRoot` set | No |
| `buildLayeredImage` | Compressed tarball in the Nix store     | Multi      | No    | No |
| `streamLayeredImage`| A script that streams tarball to stdout | Multi      | No    | **Yes** |
| `pullImage`         | Uncompressed tarball in the Nix store   | (passthrough) | No | No (migration) |

### `dockerTools.buildImage` — single-layer image

From the nixpkgs manual (crawl 68):

> "This function builds a Docker-compatible repository tarball containing a
> single image." (crawl 68)

> "This function will create a single layer for all files (and dependencies)
> that are specified in its argument." (crawl 68)

`buildImage` produces a single layer for everything in `copyToRoot` plus
its closure. Only new dependencies not already in the base image
(`fromImage`) are copied. The result is a `.tar.gz` (by default) suitable for
`docker image load`.

Key attributes:

- `name` (String) — image name.
- `tag` (String or Null; default `null`) — image tag; `null` means the nix
  derivation hash is used.
- `fromImage` (Path or Null; default `null`) — base image tarball;
  equivalent of `FROM fromImage`. `null` means `FROM scratch`.
- `copyToRoot` (Path, List of Paths, or Null; default `null`) — files to
  add; equivalent of `ADD contents/ /`. **Replaces the deprecated
  `contents` attribute.**
- `config` (Attribute Set or Null; default `null`) — container config per
  the Docker Image Specification v1.3.1.
- `runAsRoot` (String or Null; default `null`) — bash script run as root
  inside a VM; equivalent of `RUN ...`. **Requires KVM.**
- `extraCommands` (String; default `""`) — bash script run before the layer
  is finalised; **not** run as root, **no VM**. Use this when KVM is
  unavailable.
- `created` (String; default `"1970-01-01T00:00:01Z"`) — image creation
  timestamp. `"now"` breaks reproducibility.
- `compressor` (String; default `"gz"`) — `"none"`, `"gz"`, or `"zstd"`.
- `includeNixDB` (Boolean; default `false`) — populate the nix database with
  `copyToRoot` dependencies so nix commands work inside the container.

> "This attribute is deprecated, and users are encouraged to use
> `copyToRoot` instead." (crawl 68, on `contents`)

> "Using this attribute requires the `kvm` device to be available... If the
> `kvm` device isn't available, you should consider using `buildLayeredImage`
> or `streamLayeredImage` instead." (crawl 68, on `runAsRoot`)

Example 351 from the crawl — `buildImage` with `buildEnv`, `runAsRoot`, and
`config`:

```nix
{
  dockerTools,
  buildEnv,
  redis,
}:
dockerTools.buildImage {
  name = "redis";
  tag = "latest";

  copyToRoot = buildEnv {
    name = "image-root";
    paths = [ redis ];
    pathsToLink = [ "/bin" ];
  };

  runAsRoot = ''
    mkdir -p /data
  '';

  config = {
    Cmd = [ "/bin/redis-server" ];
    WorkingDir = "/data";
    Volumes = {
      "/data" = { };
    };
  };
}
```

Build and load:

```bash
$ nix-build
/nix/store/p4dsg62inh9d2ksy3c7bv58xa851dasr-docker-image-redis.tar.gz

$ docker image load -i /nix/store/p4dsg62inh9d2ksy3c7bv58xa851dasr-docker-image-redis.tar.gz
Loaded image: redis:latest
```

### `dockerTools.buildLayeredImage` — multi-layer image

From the nixpkgs manual (crawl 68):

> "`buildLayeredImage` uses `streamLayeredImage` underneath to build a
> compressed Docker-compatible repository tarball. Basically,
> `buildLayeredImage` runs the script created by `streamLayeredImage` to
> save the compressed image in the Nix store. `buildLayeredImage` supports
> the same options as `streamLayeredImage`..." (crawl 68)

> "Despite the similar name, `buildImage` works completely differently from
> `buildLayeredImage` and `streamLayeredImage`. Even though some of the
> arguments may seem related, they cannot be interchanged." (crawl 68)

`buildLayeredImage` supports the same options as `streamLayeredImage` (see
below) but **materializes the compressed image tarball in the Nix store**.
This enables store-side layer caching across rebuilds but causes store bloat
on repeated builds. Use it only when that caching is explicitly wanted.

Example 355 from the crawl:

```nix
{ dockerTools, hello }:
dockerTools.buildLayeredImage {
  name = "hello";
  tag = "latest";

  contents = [ hello ];

  config.Cmd = [ "/bin/hello" ];
}
```

Build output shows one layer per Nix store path:

```bash
$ nix-build
building '/nix/store/bk8bnrbw10nq7p8pvcmdr0qf57y6scha-hello.tar.gz.drv'...
No 'fromImage' provided
Creating layer 1 from paths: ['/nix/store/...-libunistring-1.1']
Creating layer 2 from paths: ['/nix/store/...-libidn2-2.3.4']
Creating layer 3 from paths: ['/nix/store/...-xgcc-12.3.0-libgcc']
Creating layer 4 from paths: ['/nix/store/...-glibc-2.38-27']
Creating layer 5 from paths: ['/nix/store/...-hello-2.12.1']
Creating layer 6 with customisation...
Adding manifests...
Done.
/nix/store/hxcz7snvw7f8rzhbh6mv8jq39d992905-hello.tar.gz

$ docker image load -i /nix/store/hxcz7snvw7f8rzhbh6mv8jq39d992905-hello.tar.gz
Loaded image: hello:latest
```

### `dockerTools.streamLayeredImage` — preferred default

From the nixpkgs manual (crawl 68):

> "`streamLayeredImage` builds a **script** which, when run, will stream to
> stdout a Docker-compatible repository tarball containing a single image,
> using multiple layers to improve sharing between images. This means that
> `streamLayeredImage` does not output an image into the Nix store, but only
> a script that builds the image, saving on IO and disk/cache space,
> particularly with large images." (crawl 68)

`streamLayeredImage` is the **preferred default** for CI and `docker load`
pipelines because it never materializes a large image path in the Nix store.
`nix-build` produces a script; running `./result | docker image load`
streams the tarball directly into Docker.

Key attributes:

- `name` (String) — image name.
- `tag` (String or Null; default `null`) — image tag; `null` = derivation
  hash.
- `fromImage` (Path or Null; default `null`) — base image tarball.
- `contents` (Path or List of Paths; default `[]`) — directories whose
  contents are added; equivalent of `ADD contents/ /`. Added as a final
  layer of symlinks to the actual store paths.
- `config` (Attribute Set or Null; default `null`) — container config. The
  closure of `config` is automatically included in the image.
- `maxLayers` (Number; default `100`) — maximum layer count. If
  `fromImage` is set, its layers are subtracted from this budget.
- `extraCommands` (String; default `""`) — bash script run in the context
  of the final symlink layer; only `contents` are available as links.
- `fakeRootCommands` (String; default `""`) — bash script run inside a
  fakeroot environment after `extraCommands`. Use for `chown` and other
  privileged file operations without KVM.
- `enableFakechroot` (Boolean; default `false`) — when `true`, creates a
  more complete chroot environment via `proot` before running
  `fakeRootCommands`. Equivalent of `RUN ...` in a Dockerfile.
- `includeStorePaths` (Boolean; default `true`) — when `false`, only
  symlinks are added; the actual files are not in the image. **Not
  recommended** unless you have other tooling to bind-mount the host store.
- `includeNixDB` (Boolean; default `false`) — populate the nix database
  with `copyToRoot` dependencies.
- `created` (String; default `"1970-01-01T00:00:01Z"`) — image creation
  timestamp. `"now"` breaks reproducibility.
- `mtime` (String; default `"1970-01-01T00:00:01Z"`) — modification
  timestamp for files within layers. Non-constant values break layer
  deduplication.
- `uid`/`gid` (Number; default `0`/`0`), `uname`/`gname` (String; default
  `"root"`/`"root"`) — credentials for Nix store ownership.

Layer creation strategy (crawl 68):

> "The function will attempt to create one layer per object in the Nix store
> that needs to be added to the image. In case there are more objects to
> include than available layers, the function will put the most 'popular'
> objects in their own layers, and group all remaining objects into a
> single layer." (crawl 68)

The "popularity" ranking comes from
[`references-by-popularity`](https://github.com/NixOS/nixpkgs/tree/release-23.11/pkgs/build-support/references-by-popularity).
An additional final layer is created with symlinks (built via
[`symlinkJoin`](https://nixos.org/manual/nixpkgs/stable/#trivial-builder-symlinkJoin))
pointing to the store paths specified in `contents`.

Example 356 from the crawl:

```nix
{ dockerTools, hello }:
dockerTools.streamLayeredImage {
  name = "hello";
  tag = "latest";

  contents = [ hello ];

  config.Cmd = [ "/bin/hello" ];
}
```

Build and stream:

```bash
$ nix-build
/nix/store/wsz2xl8ckxnlb769irvq6jv1280dfvxd-stream-hello

$ /nix/store/wsz2xl8ckxnlb769irvq6jv1280dfvxd-stream-hello | docker image load
No 'fromImage' provided
Creating layer 1 from paths: ['/nix/store/...-libunistring-1.1']
...
Creating layer 6 with customisation...
Adding manifests...
Done.
Loaded image: hello:latest
```

Example 358 — the closure of `config` is automatically included:

```nix
{
  dockerTools,
  hello,
  lib,
}:
dockerTools.streamLayeredImage {
  name = "hello";
  tag = "latest";
  config.Cmd = [ "${lib.getExe hello}" ];
}
```

### `dockerTools.pullImage` — pin existing Docker Hub images

From the nixpkgs manual (crawl 68):

> "This function is similar to the `docker image pull` command, which means
> it can be used to pull a Docker image from a registry that implements the
> Docker Registry HTTP API V2." (crawl 68)

`pullImage` is a **migration tool** for pinning an existing Docker Hub image
into the Nix store as an uncompressed tarball, suitable for use as a
`fromImage` base with `buildImage`/`buildLayeredImage`/`streamLayeredImage`.

It requires **two** different hashes because the image must be uniquely
identified in two different systems (the Docker registry and the Nix store):

- `imageDigest` (String) — registry-side identity (e.g.
  `sha256:...`). Guarantees content immutability.
- `sha256` or `hash` (String) — Nix-side content verification, passed to
  the `outputHash` attribute of the resulting derivation.

> "Tags are often updated to point to different image contents... An image
> tag isn't enough to guarantee the contents of an image won't change, but a
> digest guarantees this. Providing a digest helps ensure that you will
> still be able to build the same Nix code and get the same output even if
> newer versions of an image are released." (crawl 68, "Why can't I specify
> a tag" tip)

Other attributes:

- `imageName` (String) — image name with optional registry prefix
  (default `docker.io`).
- `finalImageName` (String; default = `imageName`) — name after download.
- `finalImageTag` (String; default `"latest"`) — tag after download.
- `os` (String; default `"linux"`) — per OCI Image Configuration spec.
- `arch` (String; default = `pkgs.go.GOARCH`) — per OCI spec; commonly
  `386`, `amd64`, `arm`, or `arm64`.
- `tlsVerify` (Boolean; default `true`) — disable for HTTP registries.

Example 359 — pulling `nixos/nix`:

```nix
{ dockerTools }:
dockerTools.pullImage {
  imageName = "nixos/nix";
  imageDigest = "sha256:b8ea88f763f33dfda2317b55eeda3b1a4006692ee29e60ee54ccf6d07348c598";
  finalImageName = "nix";
  finalImageTag = "2.19.3";
  hash = "sha256-zRwlQs1FiKrvHPaf8vWOR/Tlp1C5eLn1d9pE4BZg3oA=";
}
```

Use `nix-prefetch-docker` to discover the digest and hash values (Example
361):

```bash
$ nix run nixpkgs#nix-prefetch-docker -- --image-name nixos/nix --image-tag 2.19.3 --arch amd64 --os linux
{
  imageName = "nixos/nix";
  imageDigest = "sha256:498fa2d7f2b5cb3891a4edf20f3a8f8496e70865099ba72540494cd3e2942634";
  hash = "sha256-OEgs3uRPMb4Y629FJXAWZW9q9LqHS/A/GUqr3K5wzOA=";
  finalImageName = "nixos/nix";
  finalImageTag = "latest";
}
```

### `dockerTools.buildImageWithNixDb` / `includeNixDB`

The `includeNixDB` attribute (available on `buildImage` and
`streamLayeredImage`) populates the nix database in the image with the
dependencies of `copyToRoot`/`contents`. Its purpose is to allow nix commands
to run inside the container.

> "Be careful since this doesn't work well in combination with `fromImage`.
> In particular, in a multi-layered image, only the Nix paths from the lower
> image will be in the database." (crawl 68)

It also neglects to register store paths pulled in as a dependency of one of
the other values but not a direct dependency of `copyToRoot`. Use
`includeNixDB` only when nix commands must run inside the container.

### Common attributes reference table

| Attribute            | Functions                                          | Type                       | Default                     | Description |
|----------------------|----------------------------------------------------|----------------------------|-----------------------------|-------------|
| `name`               | all                                                | String                     | (required)                 | Image name |
| `tag`                | all                                                | String or Null             | `null` (= drv hash)        | Image tag |
| `fromImage`          | buildImage, buildLayeredImage, streamLayeredImage  | Path or Null               | `null` (= FROM scratch)    | Base image tarball |
| `copyToRoot`         | buildImage                                         | Path/List/Null             | `null`                      | Files to add (replaces deprecated `contents`) |
| `contents`           | buildLayeredImage, streamLayeredImage              | Path/List                  | `[]`                        | Directories whose contents are added as a final symlink layer |
| `config`             | all                                                | AttrSet or Null            | `null`                      | Container config (Docker Image Spec v1.3.1) |
| `config.Cmd`         | all                                                | List of String             | —                           | Entrypoint command |
| `config.Env`         | all                                                | List of String             | —                           | Runtime environment variables |
| `config.WorkingDir`  | all                                                | String                     | —                           | Working directory |
| `config.ExposedPorts`| all                                                | AttrSet                    | —                           | Ports to expose |
| `config.Volumes`     | all                                                | AttrSet                    | —                           | Declare volumes |
| `extraCommands`      | buildImage, buildLayeredImage, streamLayeredImage  | String                     | `""`                        | Bash script run before layer is finalised (no root, no VM) |
| `runAsRoot`          | buildImage                                         | String or Null             | `null`                      | Bash script run as root in a VM (**requires KVM**) |
| `fakeRootCommands`   | buildLayeredImage, streamLayeredImage              | String                     | `""`                        | Bash script run in fakeroot env (no KVM) |
| `enableFakechroot`   | buildLayeredImage, streamLayeredImage              | Boolean                    | `false`                     | Use `proot` chroot for `fakeRootCommands` (RUN equivalent) |
| `maxLayers`          | buildLayeredImage, streamLayeredImage              | Number                     | `100`                       | Maximum layer count |
| `created`            | all                                                | String                     | `"1970-01-01T00:00:01Z"`    | Image creation timestamp (`"now"` breaks reproducibility) |
| `mtime`              | buildLayeredImage, streamLayeredImage              | String                     | `"1970-01-01T00:00:01Z"`    | File mtime within layers |
| `compressor`         | buildImage                                         | String                     | `"gz"`                      | `"none"`, `"gz"`, or `"zstd"` |
| `includeNixDB`       | buildImage, streamLayeredImage                     | Boolean                    | `false`                     | Populate nix database for in-container nix commands |
| `includeStorePaths`  | streamLayeredImage                                 | Boolean                    | `true`                      | Include actual files (false = symlinks only) |
| `uid`/`gid`          | buildImage, streamLayeredImage                     | Number                     | `0`/`0`                     | File ownership uid/gid |
| `architecture`       | all                                                | String                     | `pkgs.go.GOARCH`            | Image architecture (OCI spec) |

### Layer strategy

What goes in each layer:

- Each Nix store path (dependency) becomes its own layer.
- The most "popular" paths (by
  [`references-by-popularity`](https://github.com/NixOS/nixpkgs/tree/release-23.11/pkgs/build-support/references-by-popularity))
  get individual layers; the rest are grouped into a single layer when the
  `maxLayers` budget is exhausted.
- A final layer is created with symlinks (built via
  [`symlinkJoin`](https://nixos.org/manual/nixpkgs/stable/#trivial-builder-symlinkJoin))
  pointing to the store paths specified in `contents`.

Layer caching benefits: unchanged dependencies produce identical store paths,
which produce identical layers, so rebuilds reuse cached layers. This is the
content-addressed deduplication advantage over Dockerfile `RUN` layers.

`maxLayers` defaults to `100`. Docker has a layer limit (see
[docker/docs#8230](https://github.com/docker/docs/issues/8230)); if
`fromImage` is set, its layer count is subtracted from the `maxLayers`
budget.

### Image loading

```bash
# buildImage / buildLayeredImage: result is a tarball
docker load < result
# or
docker image load -i result

# streamLayeredImage: result is a script that streams the tarball
./result | docker image load

# With flakes
nix build .#myImage
docker load < result
```

### Image pinning

**Flake-level (recommended):** pin nixpkgs in `flake.nix`:

```nix
inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
# Or pin to exact commit:
# inputs.nixpkgs.url = "github:NixOS/nixpkgs/abc123def456...";
```

**Package-level:** use a specific package set:

```nix
pkgs.nodejs_22  # or nodejs_24, python312, etc.
```

**Digest pinning (for `pullImage`):** obtain the digest from a pulled image:

```bash
docker inspect --format='{{index .RepoDigests 0}}' node:24-bookworm-slim
# -> node@sha256:...
```

### Microsandbox compatibility

Nix-built images produce standard OCI tarballs and are loadable by any
container runtime, including Microsandbox.

**glibc requirements:** the image must ship the same glibc the binary's
`PT_INTERP` points at. The `pi-image.nix` comment captures the failure mode:

> "the `.#pi-bun` binary's PT_INTERP points at nix glibc 2.42
> (e.g. /nix/store/...-glibc-2.42-61/lib/ld-linux-x86-64.so.2). The previous
> sandbox image (`node:24-bookworm-slim`) ships glibc 2.36, so the
> interpreter path does not exist and the binary exits 1 immediately."
> (pi-image.nix)

The fix: build the image with `dockerTools.buildLayeredImage` from the same
nixpkgs as the binary. The glibc closure (pulled in transitively via
`cacert` + `busybox`) lives at the same store path the binary requests, so
the baked-in `/app/bin/pi` finds its interpreter inside the image.

**Runtime deps checklist:**

- `cacert` — CA roots for TLS egress.
- `busybox` — `/bin/sh` + coreutils (`tail -f /dev/null` keeps the sandbox
  alive for the relay's `exec_stream`).
- `fakeNss` (`pkgs.dockerTools.fakeNss`) — `/etc/passwd` + `/etc/group`
  for uid resolution.

**Load into Microsandbox:**

```bash
just load-pi-image    # workestrator-pi
just load-images       # tempest + others
```

The Nix image is orthogonal to Microsandbox's network policy — the sandbox
plan controls egress/ingress, not the image.

### `pkgs.dockerTools` vs Dockerfiles

| Dimension          | dockerTools                                  | Dockerfiles                          |
|--------------------|----------------------------------------------|--------------------------------------|
| Reproducibility    | Same nixpkgs pin = byte-identical image      | `RUN apt-get update` non-determinism |
| Build daemon       | No Docker daemon needed during build         | Requires Docker daemon               |
| Layer dedup        | Content-addressed via Nix store paths         | Instruction-based, less precise      |
| Ecosystem          | Less familiarity, Nix learning curve         | Ubiquitous                           |
| Secrets            | Inject at runtime via env vars               | Risk of `COPY .env` leakage          |

### Anti-accumulation: why `streamLayeredImage` is preferred

From the skill file:

> "`streamLayeredImage` streams the image tarball to stdout and never
> materializes a large image path in the Nix store, so it is the right
> default for CI and `docker load` pipelines." (nix-docker-images skill)

- `buildLayeredImage` materializes the compressed tarball in the Nix store
  → store bloat on repeated builds.
- `streamLayeredImage` outputs only a script → no store accumulation.
- Use `buildLayeredImage` only when you explicitly want store-side layer
  caching across rebuilds.

## Practical rules

- Prefer `streamLayeredImage` as the default.
- Use `buildLayeredImage` only when store-side layer caching is explicitly
  wanted.
- Use `buildImage` only for single-layer simplicity or when `runAsRoot` VM
  semantics are needed.
- Keep `contents` minimal — only what the runtime needs.
- Pin nixpkgs in `flake.nix` for reproducibility.
- Don't bake secrets into images — inject at runtime via env vars.
- Use `fakeRootCommands` with `enableFakechroot = true` for non-root file
  ownership without KVM.
- Use `copyToRoot` (not deprecated `contents`) with `buildImage`.
- Use `contents` with `buildLayeredImage`/`streamLayeredImage` (these use a
  different `contents` semantics).
- Add `pkgs.cacert` for TLS, `pkgs.busybox` for `/bin/sh`,
  `pkgs.dockerTools.fakeNss` for uid resolution.
- Create `/tmp` in `extraCommands` if needed (busybox doesn't create it).
- Test the built image locally before switching sandbox plans.

## Review checklist

- [ ] Image uses `streamLayeredImage` unless store caching is explicitly
      needed
- [ ] `contents`/`copyToRoot` is minimal (no dev dependencies)
- [ ] nixpkgs is pinned in `flake.nix`
- [ ] No secrets baked into image
- [ ] `cacert` included if TLS egress is needed
- [ ] `busybox` or `binSh` included if `/bin/sh` is needed
- [ ] `fakeNss` included if uid resolution is needed
- [ ] `/tmp` created in `extraCommands` if runtime needs it
- [ ] `created` is not `"now"` (breaks reproducibility)
- [ ] `maxLayers` respects runtime layer limits
- [ ] Image tested with `docker load` before deployment

## Implementation checklist

- [ ] Choose the right dockerTools function (`streamLayeredImage` preferred)
- [ ] List runtime dependencies in `contents`
- [ ] Set `config.Cmd` to the entrypoint
- [ ] Set `config.Env` for runtime environment
- [ ] Add `extraCommands` for `/tmp`, config files, etc.
- [ ] Add `fakeRootCommands` + `enableFakechroot = true` if file ownership
      changes needed
- [ ] Build: `nix-build image.nix` or `nix build .#myImage`
- [ ] Load: `docker load < result` or `./result | docker image load`
- [ ] Verify: `docker run --rm <image>` executes the entrypoint

## Runtime / debugging checklist

- [ ] Binary exits immediately (exit code 1): check glibc/`PT_INTERP`
      mismatch — build image from same nixpkgs as binary
- [ ] TLS errors ("certificate has unknown CA"): add `pkgs.cacert` to
      contents
- [ ] "getProtocolByName: does not exist": add `pkgs.iana-etc` to
      `copyToRoot`
- [ ] No `/bin/sh`: add `pkgs.busybox` or `dockerTools.binSh`
- [ ] uid resolution failures: add `pkgs.dockerTools.fakeNss`
- [ ] Image too large: check for unwanted closure dependencies; use
      `removeReferencesTo` to strip dev-time store references
- [ ] `runAsRoot` fails: KVM not available — switch to `fakeRootCommands` +
      `enableFakechroot = true`
- [ ] Layer limit exceeded: reduce `maxLayers` or split image

## Validation hooks

- Build the image: `nix build .#<imageName>` (must succeed)
- Load into Docker: `docker load < result` (must show "Loaded image:
  `<name>:<tag>`")
- Run smoke test: `docker run --rm <name>:<tag>` (entrypoint must execute)
- Check image size: `docker image ls <name>` (should be reasonable)
- Inspect layers: `docker image inspect <name>` (verify layer count ≤
  `maxLayers`)
- For Microsandbox: `just load-<name>-image` then verify sandbox plan
  references the image

## Examples

### Minimal runtime image (streamLayeredImage)

The pattern from the skill file — `my-service` with nodejs, `config.Cmd`,
and `extraCommands`:

```nix
{ pkgs }:

# streamLayeredImage: preferred default, avoids store materialization
pkgs.dockerTools.streamLayeredImage {
  name = "my-service";
  tag = "latest";

  contents = [
    pkgs.nodejs_22
    # or pkgs.python312
    # add runtime deps here
  ];

  config = {
    Cmd = [ "node" "server.js" ];
    WorkingDir = "/app";
    Env = [
      "NODE_ENV=production"
    ];
  };

  # Copy application code
  extraCommands = '''
    mkdir -p /app
    cp -r ${./my-app}/* /app/
  ''';
}
```

### Layered image with source build (buildLayeredImage)

The pattern from the skill file — `myApp` `stdenv.mkDerivation` +
`buildLayeredImage`:

```nix
{ pkgs }:

let
  myApp = pkgs.stdenv.mkDerivation {
    name = "my-app";
    src = ./my-app;
    nativeBuildInputs = [ pkgs.nodejs_22 pkgs.npm ];
    buildPhase = "npm run build";
    installPhase = "cp -r dist $out";
  };
in
# buildLayeredImage: chosen here for layer caching across rebuilds
pkgs.dockerTools.buildLayeredImage {
  name = "my-service";
  contents = [ pkgs.nodejs_22 myApp ];
  config.Cmd = [ "node" "${myApp}/server.js" ];
}
```

### Pinning a base image (pullImage + buildImage)

The pattern from the skill file — `pullImage` + `buildImage` with
`fromImage`:

```nix
{ pkgs }:

let
  baseImage = pkgs.dockerTools.pullImage {
    imageName = "node";
    imageDigest = "sha256:abcdef...";
    sha256 = "000000...";
    finalImageTag = "24-bookworm-slim";
  };
in
pkgs.dockerTools.buildImage {
  name = "my-service";
  fromImage = baseImage;
  contents = [ /* additional deps */ ];
  config.Cmd = [ "node" "server.js" ];
}
```

### Real-world: workestrator-pi image

Reference: `nix/packages/pi-image.nix`. The actual code:

```nix
# Nix-built Docker image for the pi sandbox.
#
# Background: the `.#pi-bun` binary's PT_INTERP points at nix glibc 2.42
# (e.g. /nix/store/...-glibc-2.42-61/lib/ld-linux-x86-64.so.2). The previous
# sandbox image (`node:24-bookworm-slim`) ships glibc 2.36, so the interpreter
# path does not exist and the binary exits 1 immediately.
#
# Fix: build the image with `dockerTools.buildLayeredImage` from the same
# nixpkgs as pi-bun. The glibc closure (pulled in transitively via cacert +
# busybox) lives at the same store path the binary requests, so the baked-in
# `/app/bin/pi` finds its interpreter inside the image.
# busybox provides `/bin/sh` + `tail` (for the tail -f /dev/null entrypoint
# used by the relay); cacert provides CA roots for TLS egress.
#
# SIZE OPTIMIZATION: pi-bun-built is NOT in `contents` (that pulled its entire
# nix closure — including pi-built's 438M node_modules — into the image).
# Instead, an intermediate `pi-binary-only` derivation copies ONLY the bun
# binary + assets (dereferenced) and strips the Nix store reference
# to pi-0.79.10 using `remove-references-to`. The bun binary is self-contained
# (bun build --compile embeds the JS); the pi-0.79.10 hash embedded in the
# compiled binary is not needed at runtime. Stripping it breaks the closure
# dependency so dockerTools doesn't pull pi-0.79.10 into the image.
# The glibc reference (needed for PT_INTERP) is preserved.
#
# Load into microsandbox with: `just load-pi-image`.
{ pkgs, pi-bun-built, pi-built }:

let
  # Intermediate derivation: copy the pi-bun binary + assets (dereferenced),
  # then strip the Nix store reference to pi-0.79.10 (the source package with
  # node_modules — 438M). The pi-0.79.10 hash is embedded 2998 times in the
  # compiled binary but is not needed at runtime. Stripping it breaks the
  # closure dependency so dockerTools doesn't pull pi-0.79.10's node_modules
  # into the image. The glibc reference (needed for PT_INTERP) is preserved.
  pi-binary-only = pkgs.runCommand "pi-binary-only" {
    nativeBuildInputs = [ pkgs.removeReferencesTo ];
  } ''
    mkdir -p $out/app/bin
    cp -rL ${pi-bun-built}/bin/* $out/app/bin/
    # Make copied files writable so we can strip references and remove
    # dev-time files below.
    chmod -R +w $out/app/bin
    # Strip the reference to pi-0.79.10 (node_modules bloat) from the binary.
    # The bun binary is self-contained (bun build --compile embeds the JS);
    # it does not need pi-0.79.10 at runtime.
    remove-references-to -t ${pi-built} $out/app/bin/pi
    # Remove the doom-overlay build.sh that references bash (not needed at
    # runtime; it's a dev-time build script).
    rm -f $out/app/bin/examples/extensions/doom-overlay/doom/build.sh
  '';
in

pkgs.dockerTools.buildLayeredImage {
  name = "workestrator-pi";
  tag = "latest";

  # cacert: CA roots for TLS egress (github.com, LiteLLM proxy TLS interception).
  # busybox: /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
  #          for the relay's exec_stream; also provides /tmp if needed).
  # fakeNss: /etc/passwd + /etc/group (uid resolution for the pi binary).
  # pi-binary-only: the self-contained Bun-compiled pi binary + assets, with
  #                 the pi-0.79.10 reference stripped (no node_modules bloat).
  # All closures pull in nix glibc 2.42 transitively — matching pi-bun's
  # PT_INTERP exactly (same nixpkgs, same flake).
  contents = [ pkgs.cacert pkgs.busybox pkgs.dockerTools.fakeNss pi-binary-only ];

  extraCommands = ''
    # /tmp is needed by some bun internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp
    chmod 1777 tmp
  '';
}
```

**glibc matching strategy:** the image is built with
`dockerTools.buildLayeredImage` from the same nixpkgs as `pi-bun`. The glibc
closure (pulled in transitively via `cacert` + `busybox`) lives at the same
store path the binary requests, so the baked-in `/app/bin/pi` finds its
interpreter inside the image.

**`removeReferencesTo` size optimization:** `pi-bun-built` is **not** placed
directly in `contents` — that would pull its entire nix closure (including
`pi-built`'s 438M `node_modules`) into the image. Instead, an intermediate
`pi-binary-only` derivation copies only the bun binary + assets
(dereferenced) and strips the Nix store reference to `pi-0.79.10` using
`remove-references-to`. The bun binary is self-contained (`bun build
--compile` embeds the JS); the `pi-0.79.10` hash embedded in the compiled
binary is not needed at runtime. Stripping it breaks the closure dependency
so `dockerTools` doesn't pull `pi-0.79.10` into the image. The glibc
reference (needed for `PT_INTERP`) is preserved.

### Real-world: tempest image

Reference: `nix/packages/tempest-image.nix`. The actual code:

```nix
# Nix-built Docker image for the T3MP3ST sandbox.
#
# T3MP3ST runs as `node dist/cli.js` (interactive CLI TUI). The image provides:
#  * nodejs_24 — the Node.js runtime
#  * tempest-built — the compiled T3MP3ST tree (dist/ + node_modules/ + package.json)
#  * nmap — network recon (T3MP3ST's kill-chain drives nmap)
#  * bind.dnsutils — dig, nslookup (DNS recon)
#  * cacert — CA roots for TLS egress (LiteLLM proxy)
#  * busybox — /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
#              for the relay's exec_stream)
#  * fakeNss — /etc/passwd + /etc/group (uid resolution)
#
# The defaultProvider:"local" config is baked into the image via extraCommands
# (root/.config/t3mp3st/config.json) so T3MP3ST uses the env-var-driven local
# provider without any conf-store secret. No secrets in this file — just
# provider selection.
#
# Load into microsandbox with: `just load-images`.
{ pkgs, tempest-built }:

pkgs.dockerTools.buildLayeredImage {
  name = "tempest";
  tag = "latest";

  contents = [
    pkgs.cacert
    pkgs.busybox
    pkgs.dockerTools.fakeNss
    pkgs.nodejs_24
    tempest-built
    pkgs.nmap
    pkgs.bind.dnsutils
  ];

  extraCommands = ''
    # /tmp is needed by some node internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp
    chmod 1777 tmp

    # Bake defaultProvider:"local" into the image so T3MP3ST uses the
    # env-var-driven local provider. This is the ONLY conf-store dependency;
    # all secrets come from env vars (TEMPEST_LOCAL_API_KEY, etc.).
    mkdir -p root/.config/t3mp3st
    echo '{"defaultProvider":"local"}' > root/.config/t3mp3st/config.json
  '';
}
```

This image bakes a non-secret config (`defaultProvider:"local"`) into the
image via `extraCommands`. All actual secrets come from env vars at runtime
(`TEMPEST_LOCAL_API_KEY`, etc.) — nothing secret is baked.

### Real-world: nix-layered recipe abstraction

Reference: `nix/lib/recipes/nix-layered.nix`. The actual code:

```nix
# nix-layered image recipe: builds a dockerTools.buildLayeredImage.
# Parameters: name, tag, contents (list of package name strings),
# binary (derivation from a build recipe), baked_files, features.
{ pkgs, vocab }:

{ name, tag ? "latest", contents ? [], binary ? null, bakedFiles ? [], features ? [] }:
let
  contentsList = vocab.resolvePackages contents;
  allContents = contentsList ++ (if binary != null then [binary] else []);
  featureShell = vocab.resolveFeatures features;
  bakedShell = vocab.resolveBakedFiles bakedFiles;
in
pkgs.dockerTools.buildLayeredImage {
  inherit name tag;
  contents = allContents;
  extraCommands = featureShell + "\n" + bakedShell;
}
```

This abstraction wraps `dockerTools.buildLayeredImage` with a vocabulary
layer (`vocab`) that resolves package-name strings to derivations
(`resolvePackages`), feature flags to shell snippets (`resolveFeatures`),
and baked-file specs to shell snippets (`resolveBakedFiles`). It lets a
recipe file declare an image declaratively without touching `dockerTools`
directly.

### Building and loading

```bash
# Build the image derivation
nix-build image.nix

# Load into Docker (if using Docker runtime)
docker load < result

# Or stream directly (avoids intermediate Nix store path)
nix-build image.nix | docker load

# With flakes
nix build .#myImage
docker load < result
```

## Common mistakes

- Using `buildImage` when `streamLayeredImage` would suffice (single layer =
  larger image, no layer caching).
- Using `contents` with `buildImage` (deprecated — use `copyToRoot`).
- Using `copyToRoot` with `buildLayeredImage` (wrong API — use `contents`).
- Setting `created = "now"` (breaks reproducibility).
- Forgetting `cacert` (TLS failures at runtime).
- Forgetting `busybox`/`binSh` (no `/bin/sh`).
- Forgetting `fakeNss` (uid resolution failures).
- Not creating `/tmp` in `extraCommands` (busybox doesn't create it).
- Using `runAsRoot` without KVM available.
- Baking secrets into the image.
- Including dev-time dependencies in `contents` (closure bloat — e.g.
  `node_modules`).
- Using `fakeNss` AND `shadowSetup` together (causes breakage).

## Strict vs contextual guidance

- **STRICT:** Always prefer `streamLayeredImage` for CI and `docker load`
  pipelines.
- **STRICT:** Never set `created = "now"` in production images.
- **STRICT:** Never bake secrets into images.
- **STRICT:** Use `copyToRoot` (not `contents`) with `buildImage`.
- **CONTEXTUAL:** `buildLayeredImage` is acceptable when store-side layer
  caching across rebuilds is explicitly wanted.
- **CONTEXTUAL:** `buildImage` is acceptable for simple single-layer images
  or when `runAsRoot` VM semantics are needed and KVM is available.
- **CONTEXTUAL:** `pullImage` is for migration scenarios pinning existing
  Docker Hub images.
- **CONTEXTUAL:** `includeNixDB` only when nix commands must run inside the
  container.

## Policy decisions for individual repos

- This repo (`ai-workbench`) uses `buildLayeredImage` for `pi-image` and
  `tempest-image` because the images are loaded into Microsandbox and
  benefit from layer caching across rebuilds during development.
- `streamLayeredImage` is the recommended default for CI pipelines and
  one-off image builds.
- nixpkgs is pinned at the flake level in `flake.nix`.
- Images are loaded via `just load-pi-image` and `just load-images` justfile
  recipes.

## Related docs

- `/docs/nix/overview.md` — Nix overview
- `/docs/nix/flake-anatomy.md` — Flake anatomy (nixpkgs pinning)
- `/docs/nix/derivations-and-builds.md` — Derivations and builds
- `/docs/nix/nix-store-and-paths.md` — Nix store and paths (layer
  deduplication)
- `/docs/nix/source-map.md` — Nix source map (provenance index)

## Related skills

- `.agents/skills/nix-docker-images` — Docker image building
  (`streamLayeredImage` vs `buildLayeredImage` vs `buildImage`)
- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference

## Citations

1. [nixpkgs — dockerTools](https://nixos.org/manual/nixpkgs/stable/#sec-pkgs-dockerTools)
2. [Docker Image Specification v1.3.1](https://github.com/moby/docker-image-spec/blob/v1.3.1/spec.md)
3. [OCI Image Configuration Specification](https://github.com/opencontainers/image-spec/blob/v1.1.1/config.md#properties)
4. [Nix store paths](https://nixos.org/manual/nix/stable/store/store-path)
5. [symlinkJoin](https://nixos.org/manual/nixpkgs/stable/#trivial-builder-symlinkJoin)
6. [references-by-popularity (nixpkgs)](https://github.com/NixOS/nixpkgs/tree/release-23.11/pkgs/build-support/references-by-popularity)
7. [Docker layer limit issue](https://github.com/docker/docs/issues/8230)
8. [system-features (KVM)](https://nixos.org/manual/nix/stable/command-ref/conf-file.html#conf-system-features)
9. Local: `/nix/packages/pi-image.nix` — workestrator-pi image build
10. Local: `/nix/packages/tempest-image.nix` — tempest image build
11. Local: `/nix/lib/recipes/nix-layered.nix` — layered image recipe
    abstraction
12. Local: `.agents/skills/nix-docker-images/SKILL.md` — docker images skill
