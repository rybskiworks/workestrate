{ pkgs }:
pkgs.dockerTools.buildLayeredImage {
  name = "workestrate-network-smoke";
  tag = "latest";
  contents = [
    pkgs.busybox
    pkgs.python3
    pkgs.dockerTools.fakeNss
  ];
  extraCommands = ''
    mkdir -p tmp
    chmod 1777 tmp
  '';
  config = {
    Cmd = [ "/bin/sh" ];
    Env = [ "PATH=/bin" ];
    WorkingDir = "/tmp";
  };
}
