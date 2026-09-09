{ pkgs, workestrate }:
let
  childImage = import ./child-image.nix { inherit pkgs; };
  backendConfig = pkgs.writeText "nested-msb.json" "{}";
  script = pkgs.writeShellScriptBin "nested-smoke" (
    builtins.replaceStrings
      [ "@workestrate@" "@caBundle@" "@childImage@" "@childConfig@" "@backendConfig@" ]
      [
        "${workestrate}"
        "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "${childImage}"
        "${./child-workestrate.toml}"
        "${backendConfig}"
      ]
      (builtins.readFile ./guest-smoke.sh)
  );
in
pkgs.dockerTools.buildLayeredImage {
  name = "workestrate-nested-smoke";
  tag = "latest";
  contents = [
    pkgs.busybox
    pkgs.coreutils
    pkgs.python3
    pkgs.cacert
    pkgs.dockerTools.fakeNss
    workestrate
    script
  ];
  extraCommands = ''
    mkdir -p tmp
    chmod 1777 tmp
  '';
  config = {
    Cmd = [ "/bin/nested-smoke" ];
    Env = [ "PATH=/bin" ];
    WorkingDir = "/tmp";
  };
}
