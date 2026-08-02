# bun-install recipe: installs Bun/TypeScript deps
# Wraps the devshell build command for a Bun/TypeScript workload.
{ pkgs, bun, nodejs_24, stdenv }:

{ source, ... }:
stdenv.mkDerivation {
  pname = "bun-install";
  version = source.version or "0.1.0";

  src = source;

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

  meta = with pkgs.lib; {
    description = "Bun/TypeScript app tree with installed dependencies";
    platforms = [ "x86_64-linux" ];
  };
}
