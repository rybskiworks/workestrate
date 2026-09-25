{ pkgs, microsandbox }:
pkgs.writeShellApplication {
  name = "workestrate-init-state";
  runtimeInputs = [ pkgs.coreutils ];
  text = ''
    export WORKESTRATE_INIT_MSB=${pkgs.lib.escapeShellArg "${microsandbox}/bin/msb"}
    ${builtins.readFile ../../scripts/init-runtime-state.sh}
  '';
}
