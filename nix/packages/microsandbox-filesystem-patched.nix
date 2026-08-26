# microsandbox fork source — the ENTIRE fork workspace, used to patch ALL
# microsandbox-* crates via [patch.crates-io] in the agentctl build and dev shell.
#
# The fork (rev 79dc8a19, pinned via the `microsandbox-fork` flake input —
# the merged-develop validated rev) is a 0.6.15 workspace. Vendoring just
# crates/filesystem broke because
# its Cargo.toml uses *.workspace = true inheritance — there was no workspace
# root in the vendor directory. Vendoring the ENTIRE fork preserves the
# workspace root (Cargo.toml with [workspace.package] and [workspace.dependencies])
# so all workspace inheritance resolves naturally.
#
# The [patch.crates-io] section in agentctl.nix preBuild (and the dev shell's
# .cargo/config.toml) patches ALL microsandbox-* crates to their paths within
# this source. This ensures every microsandbox crate comes from the fork, not
# crates.io — the fork carries the MSB_AGENTD_PATH fix natively.

{ stdenv, microsandbox-fork }:

stdenv.mkDerivation rec {
  pname = "microsandbox-filesystem-patched";
  version = "0.6.15";

  src = microsandbox-fork;

  # The entire fork workspace — needed so crates/filesystem/Cargo.toml's
  # *.workspace = true inheritance resolves against the fork's Cargo.toml.
  sourceRoot = "source";

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r . $out/
    runHook postInstall
  '';

  dontConfigure = true;
  dontBuild = true;
}
