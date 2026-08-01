{ pkgs }:

pkgs.stdenv.mkDerivation rec {
  pname = "tombi";
  version = "1.2.5";

  src = pkgs.fetchurl {
    url = "https://github.com/tombi-toml/tombi/releases/download/v${version}/tombi-cli-${version}-x86_64-unknown-linux-musl.tar.gz";
    sha256 = "sha256-BThb30fRCCjB/CcZDuCtRLd3pX9W6pXJ71aMhF47eHI=";
  };

  installPhase = ''
    runHook preInstall

    if [ -f tombi-cli-${version}-x86_64-unknown-linux-musl/tombi ]; then
      install -Dm755 tombi-cli-${version}-x86_64-unknown-linux-musl/tombi $out/bin/tombi
    elif [ -f tombi ]; then
      install -Dm755 tombi $out/bin/tombi
    else
      echo "error: tombi not found in tarball" >&2
      exit 1
    fi

    runHook postInstall
  '';

  meta = with pkgs.lib; {
    description = "TOML formatter, linter, and language server";
    homepage = "https://github.com/tombi-toml/tombi";
    license = licenses.mit;
    platforms = [ "x86_64-linux" ];
    mainProgram = "tombi";
  };
}
