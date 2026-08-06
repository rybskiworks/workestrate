# microsandbox-filesystem sourced from the user's fork branch
# fix/filesystem-agentd-path-override (local head rev below; NOT yet pushed to
# GitHub). The branch's crates/filesystem (crate name microsandbox-filesystem)
# already contains the MSB_AGENTD_PATH fix natively — no patch needed. After
# the branch is pushed, swap builtins.fetchGit -> fetchFromGitHub.
{ stdenv }:

stdenv.mkDerivation rec {
  pname = "microsandbox-filesystem-patched";
  version = "0.6.8";

  src = builtins.fetchGit {
    url = "file:///home/rybski/Development/agent-workbench/forks/microsandbox/repo";
    rev = "74919059656f59612975d823cca570b774df277b";
  };

  # fetchGit unpacks to source/; the filesystem crate lives at crates/filesystem.
  sourceRoot = "source/crates/filesystem";

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r . $out/
    runHook postInstall
  '';

  dontConfigure = true;
  dontBuild = true;
}
