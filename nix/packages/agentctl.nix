{ pkgs
, pi
, odysseus
}:

let
  # The agentctl crate ships a .cargo/config.toml that pins a linker
  # to a path inside .toolchain/ (the project-local zig-based
  # fallback). Inside the Nix sandbox that path does not exist, so
  # we strip the config and let cargo pick up the system linker.
  src = pkgs.lib.cleanSourceWith {
    filter = path: type:
      let base = baseNameOf path; in
      !(base == "target" || base == "result" || base == "result-"
        || (type == "regular" && base == "config.toml"
            && pkgs.lib.hasSuffix "/.cargo/config.toml" path));
    src = ../../control/agentctl;
  };
in
pkgs.rustPlatform.buildRustPackage {
  pname = "agentctl";
  version = "0.1.0";

  inherit src;

  cargoLock = {
    lockFile = ../../control/agentctl/Cargo.lock;
  };

  buildInputs = with pkgs; [ libcap_ng ];

  doCheck = false;

  meta = {
    description = "Control plane CLI for the AI workbench";
    mainProgram = "agentctl";
  };
}
