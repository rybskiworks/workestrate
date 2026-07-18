# npm-build recipe: builds a TypeScript/Node app via buildNpmPackage.
# Parameters mirror the config schema's local_build.binary recipe.
{ pkgs, buildNpmPackage, nodejs_24, autoPatchelfHook, stdenv, libcap_ng, lib }:

{ src
, npmDepsHash
, nodeVersion ? "nodejs_24"   # config string → resolved by caller
, installLayout ? "app"       # "app" = dist/ + node_modules + package.json at root
, ...
}:
# This is a thin wrapper — the actual build logic stays in the per-agent
# .nix files (pi.nix, tempest.nix) because each has build-specific quirks
# (pi's 4-workspace build order, tempest's single-package build).
# The recipe function provides the COMMON interface; per-agent files
# call it with their specific parameters.
#
# For Phase 0b, we keep the per-agent files and expose them via the recipe
# interface. Full parameterization (merging pi.nix and tempest.nix into
# one generic function) is a future optimization.
buildNpmPackage {
  pname = src.pname or "npm-build";
  version = src.version or "0.1.0";

  inherit src npmDepsHash;

  # Skip lifecycle scripts (husky prepare, canvas node-gyp). Defensive parity
  # with pi.nix / tempest.nix.
  npmFlags = [ "--ignore-scripts" ];

  nodejs = nodejs_24;

  # tsgo/esbuild ship prebuilt ELF binaries; autoPatchelfHook patches their
  # interpreter / RPATH. stdenv.cc provides libstdc++. libcap_ng is needed by
  # gondolin's libkrun runner at runtime.
  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib libcap_ng ];

  # Generic app-style install: reproduce the runtime tree at $out so that
  # the entrypoint resolves dist/ and node_modules/ at the root.
  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r dist $out/dist 2>/dev/null || true
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json 2>/dev/null || true

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "npm-built application tree";
    platforms = platforms.linux;
  };
}
