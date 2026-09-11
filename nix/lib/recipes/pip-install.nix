# pip-install recipe: installs Python deps into .deps/
# Wraps the devshell build command for a Python service workload.
{
  pkgs,
  python312,
  stdenv,
}:

{
  source,
  requirementsFile,
  target ? ".deps",
  ...
}:
stdenv.mkDerivation {
  pname = "pip-install";
  version = source.version or "0.1.0";

  src = source;

  nativeBuildInputs = [
    python312
    python312.pkgs.pip
  ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR
    export PIP_NO_CACHE_DIR=1

    REQ=$(if [ -f requirements.lock ]; then echo requirements.lock; else echo ${requirementsFile}; fi)
    mkdir -p ${target}
    python3.12 -m pip install \
      --only-binary=:all: \
      --break-system-packages \
      --target ./${target} \
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

  meta = with pkgs.lib; {
    description = "Python app tree with pip-installed dependencies";
    platforms = [ "x86_64-linux" ];
  };
}
