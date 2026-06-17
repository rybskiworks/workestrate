{ pkgs }:

pkgs.stdenv.mkDerivation rec {
  pname = "microsandbox";
  version = "0.5.6";

  src = pkgs.fetchurl {
    url = "https://github.com/superradcompany/microsandbox/releases/download/v${version}/microsandbox-linux-x86_64.tar.gz";
    sha256 = "b550b1f5f0785d8fb6f9e1322c216afa6fe4e83020ea7f364e12308050d84e55";
  };

  agentd = pkgs.fetchurl {
    url = "https://github.com/superradcompany/microsandbox/releases/download/v${version}/agentd-x86_64";
    sha256 = "ecb46b8c13234283e4c88b4cf8c32b7150985826b1804f2cd0a55b7d370443ea";
  };

  sourceRoot = ".";

  nativeBuildInputs = with pkgs; [
    autoPatchelfHook
  ];

  buildInputs = with pkgs; [
    libcap_ng
    stdenv.cc.cc.lib
  ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/lib $out/libexec

    # msb daemon
    if [ -f msb ]; then
      install -Dm755 msb $out/bin/msb
    elif [ -f bin/msb ]; then
      install -Dm755 bin/msb $out/bin/msb
    else
      echo "error: msb not found in tarball" >&2
      exit 1
    fi

    # libkrunfw and any other shared libraries
    if [ -d lib ]; then
      for f in lib/*; do
        if [ -f "$f" ] || [ -L "$f" ]; then
          cp -P "$f" $out/lib/
        fi
      done
    fi

    # Also handle the flat layout from some releases
    for f in libkrunfw.so.5.2.1 libkrunfw-linux-x86_64.so libmicrosandbox_go_ffi-linux-amd64.so; do
      if [ -f "$f" ]; then
        install -Dm755 "$f" $out/lib/"$f"
      elif [ -L "$f" ]; then
        cp -P "$f" $out/lib/
      fi
    done

    # Ensure the libkrunfw soname symlinks exist
    if [ -f "$out/lib/libkrunfw.so.5.2.1" ]; then
      ln -sfn libkrunfw.so.5.2.1 $out/lib/libkrunfw.so.5
      ln -sfn libkrunfw.so.5 $out/lib/libkrunfw.so
    elif [ -f "$out/lib/libkrunfw-linux-x86_64.so" ]; then
      ln -sfn libkrunfw-linux-x86_64.so $out/lib/libkrunfw.so.5
      ln -sfn libkrunfw.so.5 $out/lib/libkrunfw.so
    fi

    # Guest init binary embedded by microsandbox-filesystem's build.rs
    install -Dm755 ${agentd} $out/libexec/agentd

    runHook postInstall
  '';

  meta = with pkgs.lib; {
    description = "Microsandbox CLI and runtime libraries";
    homepage = "https://github.com/superradcompany/microsandbox";
    license = licenses.asl20;
    platforms = [ "x86_64-linux" ];
    mainProgram = "msb";
  };
}
