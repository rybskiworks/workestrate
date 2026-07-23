# bun-compile recipe: produces a standalone Bun-compiled binary.
# Wraps nix/packages/pi-bun.nix. The pi-binary-only strip logic
# (removeReferencesTo, rm doom-overlay/build.sh) lives inside this recipe.
#
# B4 (WP9): restores pi's runtime asset mirroring via an optional `assets`
# parameter — a list of {from, to} entries where `from` is a path relative
# to `src` (the built tree) and `to` is a destination relative to $out/bin.
# Each entry is mirrored next to the compiled binary so the binary can
# resolve package assets relative to process.execPath (pi's config.ts
# getPackageDir()). This mirrors nix/packages/pi-bun.nix:51-81 (themes,
# assets, export-html, photon wasm, package.json, docs, examples).
# Default [] keeps the previous (asset-less) behavior for non-pi callers.
{ pkgs, bun, stdenv, lib, removeReferencesTo }:

{ src, entrypoint, worker, assets ? [], ... }:
let
  # B4: generate the asset-mirroring shell. For each {from, to}:
  #   mkdir -p the destination's parent dir under $out/bin
  #   cp -r ${src}/<from> $out/bin/<to>
  # `from` is resolved against the built `src` tree (so node_modules,
  # packages/*, etc. are reachable); `to` is relative to $out/bin.
  # Both are interpolated inside shell double-quotes so spaces are preserved.
  mirrorAssets = lib.concatStringsSep "\n" (map (a:
    ''
      mkdir -p "$out/bin/$(dirname "${a.to}")"
      cp -r "${src}/${a.from}" "$out/bin/${a.to}"
    ''
  ) assets);
in
stdenv.mkDerivation {
  pname = "bun-compile";
  version = src.version or "0.1.0";

  # The caller supplies the already-built source tree (or a source path that
  # contains the required entrypoint/worker files).
  dontUnpack = true;

  nativeBuildInputs = [ bun removeReferencesTo ];

  buildPhase = ''
    runHook preBuild

    entry="${src}/${entrypoint}"
    wk="${src}/${worker}"

    if [ ! -f "$entry" ]; then
      echo "error: bun entrypoint not found: $entry" >&2
      exit 1
    fi
    if [ ! -f "$wk" ]; then
      echo "error: bun worker not found: $wk" >&2
      exit 1
    fi

    mkdir -p $out/bin
    # --compile embeds the Bun runtime; the binary is self-contained.
    bun build --compile "$entry" "$wk" --outfile "$out/bin/app"

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    chmod -R +w $out/bin
    # Strip the store reference to the source tree (e.g. pi-0.79.10 node_modules
    # bloat) from the compiled binary. The bun binary is self-contained and does
    # not need the source tree at runtime.
    remove-references-to -t ${src} $out/bin/app

    # Remove the doom-overlay build.sh that references bash (dev-time only).
    rm -f $out/bin/examples/extensions/doom-overlay/doom/build.sh

    # B4: mirror runtime assets (themes, assets, export-html, photon wasm,
    # package.json, docs, examples) next to the binary. No-op when `assets`
    # is empty (default), preserving the previous asset-less behavior.
    ${mirrorAssets}

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "Bun-compiled standalone binary";
    platforms = platforms.linux;
  };
}
