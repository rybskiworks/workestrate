# Standalone Bun-compiled pi binary.
#
# Reuses the existing npm-built pi tree (nix/packages/pi.nix output) and runs
# `bun build --compile` on the bun entrypoint + image-resize worker to produce
# a single self-contained executable at $out/bin/pi. The Bun runtime is embedded
# in the binary; no node/bun is needed at run time.
#
# Runtime assets: pi's config.ts resolves package assets (themes, package.json,
# README, CHANGELOG, export-html templates, photon wasm) relative to
# process.execPath when isBunBinary is true. We therefore mirror the upstream
# `copy-binary-assets` script (packages/coding-agent/package.json) and place
# those assets next to the binary in $out/bin/.
{ pi-built
, bun
, stdenv
, lib
}:

stdenv.mkDerivation {
  pname = "pi-bun";
  version = pi-built.version or "0.79.10";

  # No source to unpack; we operate on the already-built pi tree.
  dontUnpack = true;

  nativeBuildInputs = [ bun ];

  buildPhase = ''
    runHook preBuild

    ca="${pi-built}/packages/coding-agent"
    entry="$ca/dist/bun/cli.js"
    worker="$ca/src/utils/image-resize-worker.ts"

    if [ ! -f "$entry" ]; then
      echo "error: bun entrypoint not found: $entry" >&2
      exit 1
    fi
    if [ ! -f "$worker" ]; then
      echo "error: image-resize-worker not found: $worker" >&2
      exit 1
    fi

    mkdir -p $out/bin
    # --compile embeds the Bun runtime; the binary is self-contained.
    bun build --compile "$entry" "$worker" --outfile "$out/bin/pi"

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    bin="$out/bin"
    ca="${pi-built}/packages/coding-agent"

    # Mirror upstream `copy-binary-assets` so the binary can resolve package
    # assets relative to process.execPath (see config.ts getPackageDir()).
    cp "$ca/package.json" "$bin/"
    cp "$ca/README.md" "$bin/" 2>/dev/null || true
    cp "$ca/CHANGELOG.md" "$bin/" 2>/dev/null || true

    mkdir -p "$bin/theme"
    cp "$ca"/src/modes/interactive/theme/*.json "$bin/theme/"

    mkdir -p "$bin/assets"
    cp "$ca"/src/modes/interactive/assets/*.png "$bin/assets/" 2>/dev/null || true

    mkdir -p "$bin/export-html/vendor"
    cp "$ca"/src/core/export-html/template.html "$bin/export-html/" 2>/dev/null || true
    cp "$ca"/src/core/export-html/vendor/*.js "$bin/export-html/vendor/"

    # Photon wasm: photon.ts falls back to execDir/photon_rs_bg.wasm.
    cp "${pi-built}/node_modules/@silvia-odwyer/photon-node/photon_rs_bg.wasm" "$bin/" 2>/dev/null || true

    # docs/ and examples/ are referenced by some commands; ship them too.
    cp -r "$ca/docs" "$bin/docs" 2>/dev/null || true
    cp -r "$ca/examples" "$bin/examples" 2>/dev/null || true

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "pi coding agent (standalone Bun-compiled binary)";
    mainProgram = "pi";
    platforms = platforms.linux;
  };
}
