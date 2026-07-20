# Hermetic build of the Odysseus Python agent for the workestrate runtime tree.
# Produces $out laid out so that mounting $out at /app and setting
# PYTHONPATH=/app/.deps makes the app work.
#
# Purity model (fixed-output derivation pattern, two-stage):
#   (1) pipDeps — a fixed-output derivation (FOD) that runs `pip download`
#       against requirements.txt (or requirements.lock if present), capturing
#       all wheels into a content-addressed directory. The FOD is the ONLY
#       step permitted network access; once the correct outputHash is supplied
#       the FOD is cached and the network fetch never repeats.
#   (2) odysseus — the main derivation, which has NO network access. It
#       installs from ${pipDeps} via `pip install --no-index --find-links`.
#
# Why FOD `pip download` (option a) instead of python3.withPackages (option b):
# the requirements include several packages that are NOT packaged in nixpkgs'
# Python package set at this rev — chromadb-client, fastembed, mcp, caldav,
# youtube-transcript-api, croniter, httpx2 — and pinning each individually
# (or vendoring the missing ones) would be far messier than a single wheel
# cache that pip resolves for us from upstream PyPI. The wheel-cache approach
# is also robust to future requirements.txt additions without per-package
# nixpkgs work. Option (a) it is.
#
# Output contract (unchanged): $out is the source tree plus $out/.deps/
# populated with installed wheels (PYTHONPATH=/app/.deps at runtime).
{ odysseus
, python312
, stdenv
, lib
}:

let
  # Stage 1 — fixed-output wheel fetcher. Runs `pip download` with network
  # access (allowed for FODs; output is hash-bounded). Output: a directory of
  # wheels (and the .requirements-marker naming which requirements file was
  # used, so the offline installer in stage 2 uses the same one).
  #
  # HOST-GATE: replace outputHash with the value reported by:
  #   nix build .#odysseus-built 2>&1 | grep 'got:'
  # (first build fails loudly with lib.fakeHash via the standard nix
  # `specified: sha256-AAA... got: sha256-XXX` mismatch; copy the `got:`
  # value, prefixed with `sha256-`, into outputHash below). With lib.fakeHash,
  # `nix eval .#odysseus-built.drvPath` still succeeds — only the FOD build
  # requires the correct hash.
  pipDeps = stdenv.mkDerivation {
    pname = "odysseus-pip-deps";
    version = "0.1.0";

    src = odysseus;

    nativeBuildInputs = [ python312 python312.pkgs.pip ];

    impureEnvVars = lib.fetchers.proxyImpureEnvVars;

    buildPhase = ''
      runHook preBuild

      export HOME=$TMPDIR

      REQ=$(if [ -f requirements.lock ]; then echo requirements.lock; else echo requirements.txt; fi)
      mkdir -p wheels
      python3.12 -m pip download \
        --only-binary=:all: \
        --dest wheels \
        -r "$REQ"

      # Persist the chosen requirements filename so stage 2 uses the same one
      # without re-running the lock-vs-txt detection.
      echo "$REQ" > wheels/.requirements-marker

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out
      # `cp -r wheels/.' copies dotfiles too (.requirements-marker).
      cp -r wheels/. $out/

      runHook postInstall
    '';

    # Fixed-output derivation attributes — content-address the wheel cache.
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = lib.fakeHash;
  };
in
# Stage 2 — offline install. No network access; resolves everything from
# ${pipDeps} via --no-index --find-links.
stdenv.mkDerivation {
  pname = "odysseus";
  version = "0.1.0";

  src = odysseus;

  nativeBuildInputs = [ python312 python312.pkgs.pip ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR
    export PIP_NO_CACHE_DIR=1

    REQ="$(cat ${pipDeps}/.requirements-marker)"
    mkdir -p .deps
    python3.12 -m pip install \
      --only-binary=:all: \
      --no-index \
      --find-links ${pipDeps} \
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
    description = "Odysseus Python agent (hermetic nix build; pip wheels prefetched via FOD)";
    platforms = [ "x86_64-linux" ];
  };
}
