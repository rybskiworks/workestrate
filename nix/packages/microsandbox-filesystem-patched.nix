{ stdenv, fetchCrate }:

stdenv.mkDerivation rec {
  pname = "microsandbox-filesystem-patched";
  version = "0.5.6";

  src = fetchCrate {
    crateName = "microsandbox-filesystem";
    inherit version;
    hash = "sha256-Y2jZNhV1OCCs30JwtYy+cIdLHUMThOiJMjI6JCKj3hU=";
  };

  patches = [ ./microsandbox-filesystem-agentd.patch ];
  patchFlags = [ "-p1" ];

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r . $out/
    runHook postInstall
  '';

  dontConfigure = true;
  dontBuild = true;
}
