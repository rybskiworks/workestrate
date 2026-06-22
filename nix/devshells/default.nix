{ pkgs
, microsandbox
, microsandbox-filesystem-patched
, agentctl
, msb-wrapped
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
    curl
    decrypt-env
    gcc
    git
    jq
    just
    libcap_ng
    msb-wrapped
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

    # Stage Microsandbox runtime for offline cargo check.
    # Use a persistent home cache instead of per-shell tmpfs to avoid
    # "No space left on device" when cargo check repeatedly copies the runtime.
    _msb_home="$HOME/.cache/ai-workbench-msb"
    rm -rf "$_msb_home/bin" "$_msb_home/lib"
    mkdir -p "$_msb_home/bin" "$_msb_home/lib"

    # Best-effort cleanup of legacy per-shell tmpfs staging dirs left by
    # earlier dev shell versions. Failures are ignored so they cannot break
    # the hook.
    for _old in /run/user/*/ai-workbench-msb-*; do
      if [ -e "$_old" ]; then
        rm -rf "$_old" 2>/dev/null || true
      fi
    done

    ln -sfn ${microsandbox}/bin/msb "$_msb_home/bin/msb"
    ln -sfn ${microsandbox}/libexec/agentd "$_msb_home/bin/agentd"

    # libkrunfw may need to be a regular file for mmap. Copy the real shared
    # objects and recreate any version/name symlinks pointing at the copies.
    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -f "$f" ] && [ ! -L "$f" ]; then
        cp -f "$f" "$_msb_home/lib/$(basename "$f")"
      fi
    done
    for f in ${microsandbox}/lib/libkrunfw.so*; do
      if [ -L "$f" ]; then
        _base=$(basename "$f")
        _target=$(readlink "$f")
        ln -sfn "$(basename "$_target")" "$_msb_home/lib/$_base"
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
