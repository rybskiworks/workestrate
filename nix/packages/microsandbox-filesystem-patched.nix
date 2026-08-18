# microsandbox fork source — the ENTIRE fork workspace, used to patch ALL
# microsandbox-* crates via [patch.crates-io] in the agentctl build and dev shell.
#
# The fork branch fix/filesystem-agentd-path-override (rev 74919059, pinned via
# the `microsandbox-fork` flake input; NOT the origin branch tip — it is the
# validated rev, reachable on GitHub but an ancestor of the current tip
# caee6378) is a 0.6.8 workspace. Vendoring just crates/filesystem broke because
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
  version = "0.6.8";

  src = microsandbox-fork;

  # TRANSIENT: mount-policy approved root relocated to MSB_HOME/mount-policy
  # (fix branch fix/mount-policy-approved-root, commits 9040f2c4 + 66b3d146)
  # applied at build time because the flake input stays github-pinned and
  # cannot track a local-only fix (cross-ref: handover 2026-08-11 §5aa;
  # supersedes — once the fork is pushed and the input re-pinned, drop this
  # patch and bump the pin).
  patches = [ ../patches/mount-policy-approved-root.patch ]; # allow: patch lives inside the flake source tree (committed to git), so the flake copy into the store stays hermetic; TRANSIENT until the fork is pushed and re-pinned

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
