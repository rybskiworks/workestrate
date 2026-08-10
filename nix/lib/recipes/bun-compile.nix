# bun-compile recipe: produces a standalone Bun-compiled binary.
# (Originated as the extracted form of the deleted nix/packages/pi-bun.nix;
# the pi-binary-only strip logic — removeReferencesTo, rm
# doom-overlay/build.sh — lives inside this recipe.)
#
# The `worker` parameter is OPTIONAL: prime-agent compiles workerless (no
# image-resize-worker), while pi passes its worker file. When `worker` is null
# (the flake passes an explicit null for a missing worker) the worker existence
# check is skipped and `bun build --compile` runs without a worker argument.
#
# B4 (WP9): restores pi's runtime asset mirroring via an optional `assets`
# parameter — a list of {from, to} entries where `from` is a path relative
# to `src` (the built tree) and `to` is a destination relative to
# $out/${installDir}. Each entry is mirrored next to the compiled binary so
# the binary can resolve package assets relative to process.execPath (pi's
# config.ts getPackageDir()). The personal config repo's pi capsule ports
# the historical asset list (themes, assets, export-html, photon wasm,
# package.json, docs, examples).
# Default [] keeps the previous (asset-less) behavior for non-pi callers.
#
# Cleanup phase 3: optional `binaryName` (default "app") and `installDir`
# (default "bin") let config-repo image builds choose the output layout;
# the defaults preserve the historical $out/bin/app behavior exactly.
{ pkgs, bun, stdenv, lib, removeReferencesTo }:

{ src, entrypoint, worker ? null, assets ? [], binaryName ? "app", installDir ? "bin", ... }:
let
  # B4: generate the asset-mirroring shell. For each {from, to}:
  #   mkdir -p the destination's parent dir under $out/${installDir}
  #   cp -r ${src}/<from> $out/${installDir}/<to>
  # `from` is resolved against the built `src` tree (so node_modules,
  # packages/*, etc. are reachable); `to` is relative to $out/${installDir}.
  # Both are interpolated inside shell double-quotes so spaces are preserved.
  mirrorAssets = lib.concatStringsSep "\n" (map (a:
    ''
      mkdir -p "$out/${installDir}/$(dirname "${a.to}")"
      cp -r "${src}/${a.from}" "$out/${installDir}/${a.to}"
    ''
  ) assets);

  # The worker is optional: prime-agent compiles workerless (no
  # image-resize-worker); pi passes one. When `worker` is null (the flake
  # passes an explicit null for a missing worker) the worker existence check
  # is skipped and `bun build --compile` runs WITHOUT a worker argument.
  # Note: worker is deliberately NOT interpolated into a string here — Nix
  # 2.35 errors on null-to-string coercion, so the check/arg are emitted
  # conditionally instead.
  workerCheck = if worker == null then "" else ''
    wk="${src}/${worker}"
    if [ ! -f "$wk" ]; then
      echo "error: bun worker not found: $wk" >&2
      exit 1
    fi
  '';
  workerArg = if worker == null then "" else " \"$wk\"";
in
stdenv.mkDerivation {
  pname = "bun-compile";
  version = src.version or "0.1.0";

  # The caller supplies the already-built source tree (or a source path that
  # contains the required entrypoint and, when `worker` is set, the worker
  # file).
  dontUnpack = true;

  nativeBuildInputs = [ bun removeReferencesTo ];

  buildPhase = ''
    runHook preBuild

    entry="${src}/${entrypoint}"

    if [ ! -f "$entry" ]; then
      echo "error: bun entrypoint not found: $entry" >&2
      exit 1
    fi
    ${workerCheck}

    mkdir -p $out/${installDir}
    # --compile embeds the Bun runtime; the binary is self-contained.
    # workerArg is empty for workerless compiles (prime-agent) and ' "$wk"'
    # when a worker file is supplied (pi).
    bun build --compile "$entry"${workerArg} --outfile "$out/${installDir}/${binaryName}"

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    chmod -R +w $out/${installDir}
    # Strip the store reference to the source tree (e.g. pi-0.79.10 node_modules
    # bloat) from the compiled binary. The bun binary is self-contained and does
    # not need the source tree at runtime.
    remove-references-to -t ${src} $out/${installDir}/${binaryName}

    # Remove the doom-overlay build.sh that references bash (dev-time only).
    rm -f $out/${installDir}/examples/extensions/doom-overlay/doom/build.sh

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
