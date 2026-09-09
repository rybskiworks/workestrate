{
  pkgs,
  microsandbox,
  microsandboxSource,
  rustToolchain,
  rev ? "dirty",
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
    filter =
      path: type:
      let
        base = baseNameOf path;
      in
      !(
        base == "target"
        || base == "result"
        || base == "result-"
        || base == "result-man"
        || base == "core"
        || pkgs.lib.hasPrefix "core." base
        || base == "vendor"
        || (type == "regular" && base == "config.toml" && pkgs.lib.hasSuffix "/.cargo/config.toml" path)
      );
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
    inherit (rustToolchain) rustc;
    inherit (rustToolchain) cargo;
  };
  manifest = builtins.fromTOML (builtins.readFile (agentctlSrc + "/Cargo.toml"));
in
assert pkgs.lib.assertMsg (
  manifest.dependencies.microsandbox.version == "=${microsandbox.version}"
  && manifest.dev-dependencies.microsandbox-image == "=${microsandbox.version}"
) "Workestrate SDK dependencies must match the Microsandbox runtime version";
rustPlatform.buildRustPackage {
  pname = "workestrate";
  version = "0.1.0";

  inherit src;

  env = {
    WORKESTRATE_REV = rev;
  };

  # The composite src root contains control/agentctl + schemas/ (see above);
  # the crate builds from the agentctl subtree.
  sourceRoot = "source/control/agentctl";

  cargoLock = {
    lockFile = ../../control/agentctl/Cargo.lock;
    # The Git dependency is selected by this lock independently of SDK patches.
    outputHashes."msb_krun-0.1.32" = "sha256-Wb5oaUkmp68FnrOVMxVimevXaWxfAPo34qVbuyLvKM0=";
  };

  # All SDK patches use the same fork input as the runtime packages. Stage
  # runtime artifacts only inside this build so prebuilt features stay offline.

  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
  ];

  buildInputs = with pkgs; [
    libcap_ng
  ];

  preBuild = ''
    mkdir -p vendor .cargo
    ln -sfn "${microsandboxSource}" vendor/microsandbox-fork
    cp ${../../control/agentctl/.cargo/config.toml} .cargo/config.toml

    # Stage the Nix-managed Microsandbox runtime so the fork's build.rs
    # finds msb + agentd locally. The fork's filesystem build.rs uses the
    # MSB_AGENTD_PATH override (prebuilt feature); the staged MSB_HOME + the
    # explicit MSB_AGENTD_PATH below satisfy it without network downloads.
    export MSB_HOME=$TMPDIR/.microsandbox
    mkdir -p $MSB_HOME/bin $MSB_HOME/lib
    cp ${microsandbox}/bin/msb $MSB_HOME/bin/msb

    # Provide the agentd guest-init binary to the fork's
    # microsandbox-filesystem build.rs via the explicit MSB_AGENTD_PATH var.
    export MSB_AGENTD_PATH=${microsandbox}/libexec/agentd

    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -f "$f" ] || [ -L "$f" ]; then
        cp -P "$f" $MSB_HOME/lib/
      fi
    done
  '';

  postInstall = ''
    # Canonical MSB home: $HOME/.microsandbox/current — the `current`
    # GENERATION symlink of the msb state-generations layout
    # ($HOME/.microsandbox/generations/<hash12>/{db,sandboxes,run,...};
    # see control/agentctl/src/microsandbox/generation.rs). msb resolves
    # the symlink itself, so its home is the target generation dir keyed by
    # the baked msb store-path hash. The guarded --run below honors an
    # explicit caller override (a non-empty MSB_HOME still wins verbatim)
    # while defaulting unset AND empty to the canonical home — empty
    # mirrors the SDK's resolve_home semantics (empty treated as unset).
    # --set-default would NOT handle the empty-string case (it only fires
    # when unset), so the explicit `[ -z ... ]` guard is required. The
    # 12-char generation keys keep derived Unix socket endpoints short
    # enough for the platform's sun_path byte limit. Shell expansion of $HOME happens
    # at wrapper execution time, not at build time.
    wrapProgram $out/bin/workestrate \
      --set MSB_PATH "${microsandbox}/bin/msb" \
      --set MSB_AGENTD_PATH "${microsandbox}/libexec/agentd" \
      --prefix PATH : ${pkgs.sops}/bin \
      --run 'if [ -z "''${MSB_HOME:-}" ]; then export MSB_HOME="$HOME/.microsandbox/current"; fi'
  '';

  doCheck = false;

  meta = {
    description = "Control plane CLI for the AI workbench";
    mainProgram = "workestrate";
  };
}
