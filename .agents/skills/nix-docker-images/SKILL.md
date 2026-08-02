# Nix Docker Image Building

## When to use

Build reproducible OCI container images with Nix instead of Dockerfiles. Use for creating pinned, minimal images for Microsandbox sandboxes, local services, or any container runtime.

## Key tools

All from `pkgs.dockerTools`:

- `streamLayeredImage` — **preferred default**, streams uncompressed tarball to stdout, avoids Nix store bloat
- `buildLayeredImage` — multi-layer image, best layer caching, use only when store-side caching is explicitly wanted
- `buildImage` — single-layer image, simpler but larger
- `pullImage` — fetch and pin an existing Docker Hub image by digest (migration tool)

## Pattern: minimal runtime image

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

## Pattern: build from source

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

## Pattern: pin base image (migration)

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

## Building and loading

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

## Pinning dependencies

**Flake-level pinning (recommended):**
```nix
inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
# Or pin to exact commit:
# inputs.nixpkgs.url = "github:NixOS/nixpkgs/abc123def456...";
```

**Package-level pinning:**
```nix
# Use specific package set
pkgs.nodejs_22  # or nodejs_24, python312, etc.
```

**Image digest pinning (for pullImage):**
```nix
imageDigest = "sha256:...";  # from `docker inspect --format='{{index .RepoDigests 0}}' node:24-bookworm-slim`
sha256 = nix-hash --base32 --type sha256 ...;
```

## Microsandbox compatibility

Nix-built images produce standard OCI tarballs. To use with Microsandbox:

1. Build the image: `nix-build image.nix`
2. Load into image store: `docker load < result` (or Microsandbox's import mechanism)
3. Reference by name in sandbox plan: `.image("workestrate-pi:latest")`

The Nix image is orthogonal to Microsandbox's network policy — the sandbox plan controls egress/ingress, not the image.

## When to prefer streamLayeredImage

`streamLayeredImage` streams the image tarball to stdout and never
materializes a large image path in the Nix store, so it is the right
default for CI and `docker load` pipelines and matches the purity rules
in [`docs/nix-purity.md`](../../../docs/nix-purity.md). `buildLayeredImage`
is the exception, chosen when repeated rebuilds benefit from cached
layers held in the store across runs.

## Best practices

- Prefer `streamLayeredImage` as the default — it streams the tarball to stdout and avoids materializing a large image path in the Nix store. Use `buildLayeredImage` only when you explicitly want store-side layer caching across rebuilds.
- Keep `contents` minimal — only what the runtime needs
- Use `streamLayeredImage` in CI to avoid Nix store bloat
- Pin nixpkgs in `flake.nix` for reproducibility
- Don't bake secrets into images — inject at runtime via env vars
- Test the built image locally before switching sandbox plans
- Use `fakeRootCommands` with `enableFakechroot = true` for non-root file ownership without KVM
