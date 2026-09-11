# Closed package vocabulary for nix-layered images.
# Config declares package names as strings; this module maps them to pkgs attrs.
# The set of allowed names is defined in policy.rs ALLOWED_PACKAGES.
{ pkgs }:
rec {
  # String name → nixpkgs derivation
  packages = {
    inherit (pkgs) cacert;
    inherit (pkgs) busybox;
    # bash — needed by prime's IPython `%%bash` cells (IPython's %%bash magic
    # spawns `bash` by name; busybox only provides `sh`). The coding-agent's
    # bash tool also prefers /bin/bash. Added 2026-08-13 (handover §5n).
    inherit (pkgs) bash;
    fakeNss = pkgs.dockerTools.fakeNss;
    inherit (pkgs) nodejs_24;
    inherit (pkgs) nmap;
    dnsutils = pkgs.bind.dnsutils;
    # prime-agent kernel env. The pinned nixpkgs (rev a799d3e) cannot build a
    # python311 ipykernel env (sphinx-9.1.0 dropped python3.11 support; even
    # python311Packages.stack-data fails to evaluate), so this ships a
    # python312 env. [HOST-VERIFY] prime accepts a 3.12 kernel (ipykernel is
    # version-agnostic in practice; prime's docs say 3.11).
    python311_kernel = pkgs.python312.withPackages (ps: [ ps.ipykernel ]);
    inherit (pkgs) ripgrep;
    inherit (pkgs) fd;
    inherit (pkgs) gnutar;
  };

  # Named shell snippets for extraCommands (closed vocabulary)
  features = {
    create_tmp = ''
      mkdir -p tmp
      chmod 1777 tmp
    '';
  };

  # Safe baked_file → shell generation.
  # Path must be relative (no leading /), no ".." traversal.
  # Content is a string (no evaluation).
  #
  # C1: content is written to the Nix store at eval time via
  # builtins.toFile, then copied into the target path at build time.
  # The store path appears in the generated shell as a literal
  # /nix/store/<hash>-baked-file-content string — only safe path
  # characters — so config content can NEVER break out of the shell
  # parser, regardless of what bytes it contains. This replaces the
  # previous heredoc approach which could be subverted by content
  # containing the WORKESTRATE_BAKED_EOF delimiter (after Nix
  # multiline-string dedent, the closing delimiter was column-0 and
  # any matching content line broke out, executing subsequent lines
  # as shell at image build time). builtins.base64Of would also close
  # this hole but is not available in Nix 2.35.1; the store-path
  # approach is equivalently bullet-proof and adds no runtime dep.
  bakedFileToShell =
    { path, content }:
    assert builtins.isString path;
    assert builtins.substring 0 1 path != "/";
    # Path traversal guard: reject any path containing a ".." component.
    # (builtins.match returns null on no-match; the pre-existing
    # `!builtins.match ...` form was a no-op — always errored — so the
    # guard never fired. C1 fix: use explicit `== null` so the assert
    # is actually exercised.)
    assert builtins.match ".*\\.\\..*" path == null;
    let
      contentFile = builtins.toFile "baked-file-content" content;
    in
    ''
      mkdir -p $(dirname ${path})
      cp ${contentFile} ${path}
    '';

  # Resolve a list of package name strings to derivations.
  resolvePackages = names: builtins.map (n: builtins.getAttr n packages) names;

  # Resolve a list of feature name strings to shell text.
  resolveFeatures =
    names: builtins.concatStringsSep "\n" (builtins.map (n: builtins.getAttr n features) names);

  # Resolve a list of baked_file attrs to shell text.
  resolveBakedFiles = files: builtins.concatStringsSep "\n" (builtins.map bakedFileToShell files);
}
