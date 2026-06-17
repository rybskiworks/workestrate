{ pkgs
, microsandbox
, microsandbox-filesystem-patched
, agentctl
, with-secrets
, run-with-secrets
, decrypt-env
, write-env
, setup-secrets
}:

pkgs.mkShell {
  packages = with pkgs; [
    age
    agentctl
    cargo
    clippy
    decrypt-env
    gcc
    git
    just
    libcap_ng
    microsandbox
    openssl
    pkg-config
    run-with-secrets
    rustc
    rustfmt
    sops
    with-secrets
    write-env
    setup-secrets
  ];

  shellHook = ''
    echo "ai-workbench dev shell"
    echo "msb version: $(msb --version 2>/dev/null || echo 'not available')"
    echo "secrets workflow: docs/secrets.md"

    # Stage Microsandbox runtime for offline cargo check
    _msb_home="''${XDG_RUNTIME_DIR:-''${TMPDIR:-/tmp}}/ai-workbench-msb-$$"
    mkdir -p "$_msb_home/bin" "$_msb_home/lib"
    cp -f ${microsandbox}/bin/msb "$_msb_home/bin/msb"
    chmod +x "$_msb_home/bin/msb"
    cp -f ${microsandbox}/libexec/agentd "$_msb_home/bin/agentd"
    chmod +x "$_msb_home/bin/agentd"
    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -f "$f" ] || [ -L "$f" ]; then
        cp -P "$f" "$_msb_home/lib/"
      fi
    done
    export MSB_HOME="$_msb_home"
    export MSB_PATH="$_msb_home/bin/msb"

    _setup_vendor_link() {
      local repo_root vendor_dir vendor_link target
      repo_root=$(git rev-parse --show-toplevel 2>/dev/null || true)
      if [ -z "$repo_root" ]; then
        return 0
      fi
      vendor_dir="$repo_root/control/agentctl/vendor"
      vendor_link="$vendor_dir/microsandbox-filesystem-0.5.6"
      target="${microsandbox-filesystem-patched}"

      mkdir -p "$vendor_dir"

      if [ -L "$vendor_link" ]; then
        local current
        current=$(readlink -f "$vendor_link" 2>/dev/null || true)
        if [ -z "$current" ] || [ ! -d "$current" ]; then
          echo "ai-workbench: refreshing stale vendor symlink" >&2
          ln -sfn "$target" "$vendor_link"
        fi
      elif [ -e "$vendor_link" ]; then
        echo "ai-workbench: vendor/microsandbox-filesystem-0.5.6 is a real directory (unlocked); leaving it alone" >&2
      else
        ln -sfn "$target" "$vendor_link"
      fi
    }
    _setup_vendor_link
    unset -f _setup_vendor_link
  '';
}
