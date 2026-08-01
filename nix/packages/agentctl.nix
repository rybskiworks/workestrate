{ pkgs
, microsandbox
, microsandbox-filesystem-patched
, rustToolchain
}:

let
  # `src` is filtered to keep the nix build hermetic: no in-tree build
  # artifacts, no crash dumps, no locally-managed result symlinks, no stale
  # vendor directory (the preBuild hook recreates `vendor/` as a symlink to
  # the nix-managed patched crate, so any source-tree vendor/ is unused).
  # Closing the nix-purity guard: every excluded basename here corresponds
  # to an entry in `control/agentctl/.gitignore` so the working-tree state
  # and the nix-source view agree.
  agentctlSrc = pkgs.lib.cleanSourceWith {
    filter = path: type:
      let base = baseNameOf path; in
      !(base == "target"
        || base == "result" || base == "result-" || base == "result-man"
        || base == "core" || pkgs.lib.hasPrefix "core." base
        || base == "vendor"
        || (type == "regular" && base == "config.toml"
            && pkgs.lib.hasSuffix "/.cargo/config.toml" path));
    src = ../../control/agentctl;
  };

  # The scaffold embeds the committed JSON schema via
  # `include_str!("../../../../schemas/workestrate.schema.json")` (relative to
  # control/agentctl/src/scaffold/mod.rs), so the build source must be the
  # repo-level subtree that contains BOTH control/agentctl and schemas/.
  src = pkgs.runCommand "source" { } ''
    mkdir -p $out/control
    cp -r ${agentctlSrc} $out/control/agentctl
    chmod -R u+w $out/control/agentctl
    cp -r ${../../schemas} $out/schemas
  '';

  # Use the fenix-pinned toolchain so nix builds and the dev shell agree on
  # the exact rustc version (currently 1.97.1).
  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain.rustc;
    cargo = rustToolchain.cargo;
  };
in
(rustPlatform.buildRustPackage {
  pname = "workestrate";
  version = "0.1.0";

  inherit src;

  # The composite src root contains control/agentctl + schemas/ (see above);
  # the crate builds from the agentctl subtree.
  sourceRoot = "source/control/agentctl";

  cargoLock = {
    lockFile = ../../control/agentctl/Cargo.lock;
  };

  # Allow microsandbox-filesystem's build.rs to find agentd via the
  # explicit MSB_AGENTD_PATH env var so the Nix build avoids network
  # downloads. The patch is applied directly to the vendored crate
  # directory in preBuild (see below) because buildRustPackage with
  # cargoLock does not forward `cargoPatches` to the vendored source.

  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
  ];

  buildInputs = with pkgs; [
    libcap_ng
  ];

  preBuild = ''
    mkdir -p vendor
    ln -sfn "${microsandbox-filesystem-patched}" vendor/microsandbox-filesystem-0.5.6
    cat > .cargo/config.toml <<'CARGO_CONFIG'
    [patch.crates-io]
    microsandbox-filesystem = { path = "vendor/microsandbox-filesystem-0.5.6" }
    CARGO_CONFIG

    # Stage the Nix-managed Microsandbox runtime where the crate's build.rs
    # expects it. build.rs resolves its install root via MSB_HOME (verbatim,
    # no .microsandbox suffix) and skips downloading when bin/msb and
    # lib/libkrunfw.so.5.2.1 exist and msb --version matches 0.5.6.
    export MSB_HOME=$TMPDIR/.microsandbox
    mkdir -p $MSB_HOME/bin $MSB_HOME/lib
    cp ${microsandbox}/bin/msb $MSB_HOME/bin/msb

    # Provide the agentd guest-init binary to the patched
    # microsandbox-filesystem build.rs via the explicit MSB_AGENTD_PATH var.
    export MSB_AGENTD_PATH=${microsandbox}/libexec/agentd

    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -f "$f" ] || [ -L "$f" ]; then
        cp -P "$f" $MSB_HOME/lib/
      fi
    done
  '';

  postInstall = ''
    # Force MSB_HOME to a persistent path at runtime. The dev shell sets a
    # temporary MSB_HOME (e.g. /run/user/1000/ai-workbench-msb-$$) for offline
    # cargo check builds only. At runtime, the SDK needs a stable MSB_HOME
    # (~/.microsandbox) for cache/db/state. Using --run ensures shell expansion
    # of $HOME happens at wrapper execution time, not at build time.
    wrapProgram $out/bin/workestrate \
      --set MSB_PATH "${microsandbox}/bin/msb" \
      --prefix PATH : ${pkgs.sops}/bin \
      --run 'export MSB_HOME="$HOME/.microsandbox"'
  '';

  doCheck = false;

  meta = {
    description = "Control plane CLI for the AI workbench";
    mainProgram = "workestrate";
  };
}
)
