# Hermetic build of the Odysseus Python agent for the workestrate runtime tree.
# Produces $out laid out so that mounting $out at /app and setting
# PYTHONPATH=/app/.deps makes the app work.
#
# Design choices:
#  * Mirrors the devshell build: `pip install` into ./.deps from
#    requirements.txt (or requirements.lock if present).
#  * Uses stdenv.mkDerivation so the resulting tree is a plain app directory,
#    not a Python site-packages layout.
{ odysseus
, python312
, stdenv
, lib
}:

stdenv.mkDerivation {
  pname = "odysseus";
  version = "0.1.0";

  src = odysseus;

  nativeBuildInputs = [
    python312
    python312.pkgs.pip
  ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR
    export PIP_NO_CACHE_DIR=1

    REQ=$(if [ -f requirements.lock ]; then echo requirements.lock; else echo requirements.txt; fi)
    mkdir -p .deps
    python3.12 -m pip install \
      --only-binary=:all: \
      --break-system-packages \
      --target ./.deps \
      -r "$REQ"

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
    description = "Odysseus Python agent (hermetic nix build with pip dependencies)";
    platforms = [ "x86_64-linux" ];
  };
}
