# Build the agentctl crate at control/agentctl/ via crane.
#
# crane does the work of building a clean dependency closure
# (buildRustPackage with a pre-fetched dep tree), so `nix build
# .#agentctl` produces a reproducible, sandboxed binary.
#
# Implementation notes:
# - We pass pname/version/cargoToml explicitly because crane's
#   crateNameFromCargoToml helper is brittle with relative src paths
#   in some Nix versions.
# - We strip the project's `.cargo/config.toml` from the source tree
#   before building. That file pins the linker to a path inside
#   `.toolchain/` (the project-local zig-based fallback used when
#   Nix is unavailable). Inside the Nix sandbox that path does not
#   exist, so we let cargo pick up the system linker that Nix
#   provides (gcc-wrapper from nixpkgs). The fallback toolchain
#   itself is untouched and `.toolchain/` continues to work for
#   non-Nix invocations of cargo.
{ pkgs
, crane
, lib
, system ? builtins.currentSystem
, srcRaw ? ../../control/agentctl
}:

let
  craneLib = crane.mkLib pkgs;

  # Build a filtered source tree. The `filter` keeps everything
  # needed for the build (Cargo.toml, Cargo.lock, src/, .cargo/)
  # but drops the linker config that points outside the sandbox,
  # plus the build-only target/ directory.
  src = lib.cleanSourceWith {
    filter = path: type:
      let
        base = baseNameOf path;
        in
        # Drop build artifacts and the link-pinned cargo config.
        !(base == "target"
          || base == "result"
          || base == "result-"
          || (type == "regular" && base == "config.toml"
              && lib.hasSuffix "/.cargo/config.toml" path));
    src = srcRaw;
  };

  cargoLock = srcRaw + "/Cargo.lock";

  agentctlCrate = craneLib.buildPackage {
    inherit src;
    pname = "agentctl";
    version = "0.1.0";
    cargoToml = src + "/Cargo.toml";
    cargoLock = if builtins.pathExists cargoLock
      then cargoLock
      else null;
    cargoExtraArgs = "";
    # The agentctl crate currently has zero C dependencies and only
    # `clap` as a direct Rust dep. If we ever add a C dep, expose it
    # via buildInputs here.
    buildInputs = [ ];
    nativeBuildInputs = [ ];
    doCheck = true;
  };
in
agentctlCrate
