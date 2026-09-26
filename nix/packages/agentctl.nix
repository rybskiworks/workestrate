{
  pkgs,
  microsandbox,
  microsandboxSource,
  microsandboxCargoLock,
  rustToolchain,
  rev ? "dirty",
}:

let
  # Import the directory, not just the script: reviewed evidence is adjacent.
  noticeHelper = "${../../scripts/licensing}/cargo_notices.py";

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
    cp ${../../LICENSE} $out/LICENSE
    cp ${../../NOTICE} $out/NOTICE
  '';

  # Use the fenix-pinned toolchain so nix builds and the dev shell agree on
  # the exact rustc version (currently 1.97.1).
  rustPlatform = pkgs.makeRustPlatform {
    inherit (rustToolchain) rustc;
    inherit (rustToolchain) cargo;
  };
  # Read metadata before filtering: read-only evaluation cannot realize a new
  # filtered source copy merely to inspect the unchanged manifest inside it.
  manifest = builtins.fromTOML (builtins.readFile ../../control/agentctl/Cargo.toml);
in
assert pkgs.lib.assertMsg (
  manifest.dependencies.microsandbox.version == "=${microsandbox.version}"
  && manifest.dev-dependencies.microsandbox-image == "=${microsandbox.version}"
) "Workestrate SDK dependencies must match the Microsandbox runtime version";
rustPlatform.buildRustPackage {
  pname = "workestrate";
  version = manifest.package.version;

  inherit src;

  env = {
    WORKESTRATE_REV = rev;
  };

  # The composite src root contains control/agentctl + schemas/ (see above);
  # the crate builds from the agentctl subtree.
  sourceRoot = "source/control/agentctl";

  cargoLock = microsandboxCargoLock {
    lockFile = ../../control/agentctl/Cargo.lock;
  };

  # All SDK patches use the same fork input as the runtime packages. Explicit
  # immutable runtime inputs keep the prebuilt features offline.

  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
    buildPackages.python3
    buildPackages.cargo-about
  ];

  buildInputs = with pkgs; [
    libcap_ng
  ];

  preBuild = ''
    # Require the producer contracts before compiling against a promoted runtime.
    # The old pin deliberately fails until validated producer/lock promotion.
    for evidence in \
      share/libkrunfw/compliance/manifest.json \
      share/licenses/microsandbox-cli/rust/manifest.json \
      share/licenses/microsandbox-agentd/rust/manifest.json; do
      if ! test -s "${microsandbox}/$evidence"; then
        echo "error: promote a validated Microsandbox runtime with legal evidence: $evidence" >&2
        exit 1
      fi
    done
    for component in microsandbox-cli microsandbox-agentd; do
      python3 ${noticeHelper} verify \
        "${microsandbox}/share/licenses/$component/rust"
    done

    mkdir -p vendor .cargo
    ln -sfn "${microsandboxSource}" vendor/microsandbox-fork
    cp ${../../control/agentctl/.cargo/config.toml} .cargo/config.toml

    # Build inputs are immutable, independent of mutable runtime state. The
    # SDK validates the explicit runtime directory without any downloads.
    export MSB_BUILD_RUNTIME=${microsandbox}
    export MSB_AGENTD_PATH=${microsandbox}/libexec/agentd
  '';

  # Generate from the exact patched Cargo source/lock and default schema features.
  # This is offline: missing original attribution is an error, not a network fetch.
  postBuild = ''
    python3 ${noticeHelper} generate \
      --manifest "$PWD/Cargo.toml" --policy ${../../deny.toml} \
      --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget} \
      --source-root "$PWD/../.." --source-root ${microsandboxSource} \
      --output "$TMPDIR/workestrate-notices"
  '';

  postInstall = ''
    mkdir -p $out/share/licenses/workestrate
    install -m644 ${../../LICENSE} $out/share/licenses/workestrate/LICENSE
    install -m644 ${../../NOTICE} $out/share/licenses/workestrate/NOTICE
    install -m644 ${../../LICENSING.md} $out/share/licenses/workestrate/LICENSING.md
    install -m644 ${../../THIRD-PARTY.md} $out/share/licenses/workestrate/THIRD-PARTY.md
    cp -r "$TMPDIR/workestrate-notices" $out/share/licenses/workestrate/rust
    python3 ${noticeHelper} verify $out/share/licenses/workestrate/rust

    # sops + age are bundled on the wrapper's PATH: `workestrate secrets`
    # (init/update) shells out to sops for encrypt/decrypt and to age-keygen
    # for key bootstrap/recipient derivation, and the secrets loader shells
    # out to sops for decryption — the installed CLI must find both without
    # a devshell.
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
      --prefix PATH : ${pkgs.sops}/bin:${pkgs.age}/bin \
      --run 'if [ -z "''${MSB_HOME:-}" ]; then export MSB_HOME="$HOME/.microsandbox/current"; fi'
  '';

  doCheck = false;

  # Consumers install this package directly, retaining its paired runtime.
  passthru = {
    inherit microsandbox;
    stateInit = import ./runtime-state.nix { inherit pkgs microsandbox; };
  };

  meta = {
    description = "Control plane CLI for the AI workbench";
    mainProgram = "workestrate";
    license = pkgs.lib.licenses.asl20;
  };
}
