# Hermetic build of the OpenCode TypeScript/Bun agent for the workestrate
# runtime tree. Produces $out laid out so that mounting $out at /app makes the
# opencode workspace tree available to the runtime.
#
# Purity model (fixed-output derivation pattern, two-stage):
#   (1) bunDeps — a fixed-output derivation (FOD) that runs `HUSKY=0 bun
#       install` against the opencode source, capturing the resulting
#       node_modules tree. The FOD is the ONLY step permitted network access;
#       its output is content-addressed via outputHash, so once the correct
#       hash is supplied the FOD is cached and the network fetch never repeats.
#   (2) opencode — the main derivation, which has NO network access. It copies
#       the source tree and overlays the prefetched node_modules from bunDeps.
#
# Why a custom bunDeps FOD (option a) instead of nixpkgs buildBunPackage
# (option b): buildBunPackage at this nixpkgs rev is still maturing for
# non-trivial Bun workspace monorepos with the `catalog:` workspace feature
# and `bun.lock`; the project layout here is heavy enough (packages/* +
# packages/console/* + packages/stats/* + packages/sdk/js + packages/slack,
# plus postinstall hooks like fix-node-pty) that a one-line custom FOD
# mirrors the existing devshell install exactly and avoids builder-specific
# compatibility surprises. The task wording permitted either; the custom FOD
# is the more deterministic choice given the project shape.
#
# Bun workspace symlinks: bun creates symlinks in node_modules/@<scope>/ that
# point back into the source tree. Because bunDeps and the main derivation
# share the SAME `${opencode}` source input, those absolute symlinks resolve
# correctly at runtime (the original source path remains in the nix store
# closure as a runtime dependency via bunDeps). The output layout is therefore
# identical to the prior live-install output.
#
# Output contract (unchanged): $out is the full opencode tree with
# node_modules populated.
{ opencode
, bun
, nodejs_24
, stdenv
, lib
}:

let
  # Stage 1 — fixed-output fetcher. Runs `bun install` with network access
  # (allowed for FODs; output is hash-bounded). Output: a directory containing
  # node_modules/ + the lockfiles.
  #
  # HOST-GATE: replace outputHash with the value reported by:
  #   nix build .#opencode-built 2>&1 | grep 'got:'
  # (first build fails loudly with lib.fakeHash via the standard nix
  # `specified: sha256-AAA... got: sha256-XXX` mismatch; copy the `got:`
  # value, prefixed with `sha256-`, into outputHash below). With lib.fakeHash,
  # `nix eval .#opencode-built.drvPath` still succeeds — only the FOD build
  # requires the correct hash.
  bunDeps = stdenv.mkDerivation {
    pname = "opencode-bun-deps";
    version = "1.17.9";

    src = opencode;

    nativeBuildInputs = [ bun nodejs_24 ];

    impureEnvVars = lib.fetchers.proxyImpureEnvVars;

    buildPhase = ''
      runHook preBuild

      export HOME=$TMPDIR
      HUSKY=0 bun install

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out
      cp -r node_modules $out/node_modules
      # Carry the lockfiles for debuggability / parity with the live install.
      cp package.json $out/package.json
      cp bun.lock $out/bun.lock

      runHook postInstall
    '';

    # Fixed-output derivation attributes — content-address the node_modules tree.
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = lib.fakeHash;
  };
in
# Stage 2 — offline assembly. No network access; reads node_modules from bunDeps.
stdenv.mkDerivation {
  pname = "opencode";
  version = "1.17.9";

  src = opencode;

  # bun/nodejs are retained in nativeBuildInputs to mirror the devshell's
  # runtime expectations (the install already happened in bunDeps).
  nativeBuildInputs = [ bun nodejs_24 ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r . $out/

    # Overlay the prefetched node_modules tree from the FOD, replacing any
    # source-tree node_modules (the upstream repo does not ship one, but we
    # remove first to be defensive against accidental commits).
    rm -rf $out/node_modules
    cp -r ${bunDeps}/node_modules $out/node_modules

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "OpenCode TypeScript/Bun agent (hermetic nix build; node_modules prefetched via FOD)";
    platforms = [ "x86_64-linux" ];
  };
}
