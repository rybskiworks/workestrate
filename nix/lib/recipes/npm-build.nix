# npm-build recipe: builds a TypeScript/Node app via buildNpmPackage.
# Parameters mirror the config schema's local_build.binary recipe.
#
# B3 (WP9): the recipe can now express pi's 4-workspace build order
# (tui -> ai -> agent -> coding-agent) and skip pi's root build script
# (which fails offline because generate-models deletes committed catalogs
# then can't refetch them). The logic originated in the deleted
# nix/packages/pi.nix call site (now ported to the personal config repo
# flake's pi pre-build) and is exposed here via three optional passthroughs:
#   dontNpmBuild  — skip buildNpmPackage's default `npm run build`
#   buildPhase    — custom build phase (e.g. pi's chained workspace builds)
#   installPhase   — custom install phase (e.g. pi's monorepo tree layout)
# When unset, the recipe falls back to the tempest-style default
# (default `npm run build` + app-style dist/node_modules install), preserving
# backward compatibility for single-package apps.
#
# B5 (WP9) — installLayout decision: REMOVED. The parameter was previously
# accepted (`installLayout ? "app"`) but was a silent no-op — nothing in the
# recipe flow branched on it and no second layout ("lib") consumer exists in
# this repo. A silent no-op is worse than absence (it implies behavior that
# does not exist), so the parameter has been removed from the recipe signature
# and from the buildImagesFromConfig call site in flake.nix. As of the
# consolidation wave the field is retired everywhere — recipe, Rust schema
# (control/agentctl/src/config.rs), and the migration spec. If a real second
# layout consumer is introduced later, re-add an explicit
# `installLayout ? "app"` param AND branch on it in installPhase — do not
# restore the no-op.
{
  pkgs,
  buildNpmPackage,
  nodejs_24,
  autoPatchelfHook,
  stdenv,
  libcap_ng,
  lib,
}:

{
  src,
  npmDepsHash,
  nodeVersion ? "nodejs_24", # config string → resolved by caller
  dontNpmBuild ? false, # B3: skip default `npm run build` (pi root script fails offline)
  buildPhase ? null, # B3: custom build phase (pi's 4-workspace order)
  installPhase ? null, # B3: custom install phase (pi's monorepo layout)
  ...
}:
let
  # Default app-style install: reproduce the runtime tree at $out so that
  # the entrypoint resolves dist/ and node_modules/ at the root. This is the
  # tempest-style layout (single-package app). pi overrides it with a
  # monorepo-aware installPhase (packages/ + node_modules + root package.json).
  defaultInstallPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r dist $out/dist 2>/dev/null || true
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json 2>/dev/null || true

    runHook postInstall
  '';
in
buildNpmPackage (
  {
    pname = src.pname or "npm-build";
    version = src.version or "0.1.0";

    inherit src npmDepsHash;

    # Skip lifecycle scripts (husky prepare, canvas node-gyp). Defensive parity
    # with pi.nix / tempest.nix.
    npmFlags = [ "--ignore-scripts" ];

    nodejs = nodejs_24;

    # tsgo/esbuild ship prebuilt ELF binaries; autoPatchelfHook patches their
    # interpreter / RPATH. stdenv.cc provides libstdc++. libcap_ng is needed by
    # gondolin's libkrun runner at runtime. pkgs.musl provides
    # libc.musl-x86_64.so.1 + ld-musl-x86_64.so.1 for the musl-linked native
    # node deps some workloads pull (lightningcss-linux-x64-musl,
    # @rolldown/binding-linux-x64-musl, @biomejs/cli-linux-x64-musl, esbuild);
    # without it auto-patchelf fails with "could not satisfy dependency
    # libc.musl-x86_64.so.1".
    nativeBuildInputs = [ autoPatchelfHook ];
    buildInputs = [
      stdenv.cc.cc.lib
      libcap_ng
      pkgs.musl
    ];

    installPhase = if installPhase != null then installPhase else defaultInstallPhase;

    dontStrip = true;

    meta = with lib; {
      description = "npm-built application tree";
      platforms = platforms.linux;
    };
  }
  // lib.optionalAttrs dontNpmBuild { inherit dontNpmBuild; }
  // lib.optionalAttrs (buildPhase != null) { inherit buildPhase; }
)
