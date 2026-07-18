# bun-compile recipe: produces a standalone Bun-compiled binary.
# Wraps nix/packages/pi-bun.nix. The pi-binary-only strip logic
# (removeReferencesTo, rm doom-overlay/build.sh) lives inside this recipe.
{ pkgs, bun, stdenv, lib, removeReferencesTo }:

{ src, entrypoint, worker, ... }:
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

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "Bun-compiled standalone binary";
    platforms = platforms.linux;
  };
}
