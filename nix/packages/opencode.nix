# Hermetic build of the OpenCode TypeScript/Bun agent for the workestrate
# runtime tree. Produces $out laid out so that mounting $out at /app makes the
# opencode workspace tree available to the runtime.
#
# Design choices:
#  * Mirrors the devshell build: `HUSKY=0 bun install`.
#  * stdenv.mkDerivation (not buildNpmPackage) because the project uses Bun
#    workspaces and bun.lock.
#  * The output is the full source tree plus node_modules so that `bun run ...`
#    resolves workspace and external dependencies.
{ opencode
, bun
, nodejs_24
, stdenv
, lib
}:

stdenv.mkDerivation {
  pname = "opencode";
  version = "1.17.9";

  src = opencode;

  nativeBuildInputs = [
    bun
    nodejs_24
  ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR
    HUSKY=0 bun install

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r . $out/

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "OpenCode TypeScript/Bun agent (hermetic nix build with bun install)";
    platforms = [ "x86_64-linux" ];
  };
}
