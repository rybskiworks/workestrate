{ pkgs
, microsandbox
, microsandbox-filesystem-patched
, workestrate
, rustToolchain
, msb-wrapped
, decrypt-env
, write-env
, setup-secrets
, load-images
, imageNames
, pi
, pi-bun-built
, odysseus
, opencode
, tempest
, referenceConfig
}:

# Agent source repos. These default to maintainer forks.
# Override with --override-input to use upstreams or your own forks:
#   nix develop --override-input pi github:earendil-works/pi
#   nix develop --override-input opencode github:anomalyco/opencode

# Build commands derived from config.reference/workestrate.toml.
# The devshell reads ONLY the reference config (tool-dev). Each workload
# with a local_build recipe becomes one _build_if_needed invocation.
let
  localBuildNames = referenceConfig.localBuilds;

  recipeCmd = lb:
    if lb.recipe == "pip-install" then
      let req = lb.requirements_file or "requirements.txt"; in
      ''REQ=$([ -f requirements.lock ] && echo requirements.lock || echo ${req}) && python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./.deps -r "$REQ"''
    else if lb.recipe == "bun-install" then
      "HUSKY=0 bun install"
    else if lb.recipe == "npm-build" then
      "npm install && npm run build"
    else
      throw "unknown local_build recipe: ${lb.recipe}";

  buildAgentCommands = builtins.concatStringsSep "\n" (map (name:
    let lb = referenceConfig.workloads.${name}.local_build; in
    ''_build_if_needed "${name}" "${recipeCmd lb}" "${lb.gating_file or ""}"''
  ) localBuildNames);
in

pkgs.mkShell {
  packages = with pkgs; [
    age
    workestrate
    rustToolchain.cargo
    rustToolchain.clippy
    curl
    decrypt-env
    gcc
    git
    jq
    just
    libcap_ng
    load-images
    msb-wrapped
    nodejs_24  # Node 24: pi's gondolin needs >=23.6; aligns with the node:24 sandbox images
    bun
    openssl
    pkg-config
    (python3.withPackages (p: [ p.pip ]))
    (python312.withPackages (ps: [ ps.pip ps."pip-tools" ]))
    rustToolchain.rustc
    rustToolchain.rust-analyzer
    rustToolchain.rustfmt
    sops
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

    # Relocate cargo's target dir out of the source tree to keep the
    # repo small (a clean `cargo build` is ~5-25 GB) and to prevent
    # accidental commits / store copies. The justfile cargo recipes
    # also set this env so bare `just` outside the devshell agrees.
    # Closes the nix-purity anti-accumulation finding for target/.
    export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"
    mkdir -p "$CARGO_TARGET_DIR"

    export MSB_HOME="$_msb_home"
    export MSB_PATH="$_msb_home/bin/msb"

    # Canonical pi build: the standalone Bun binary from `.#pi-bun`.
    # Dev-workestrate reads WORKESTRATE_PI_BUILD so it mounts the bun binary
    # at /app/bin/pi instead of needing agents/pi/build from the shellHook.
    export WORKESTRATE_PI_BUILD="${pi-bun-built}"

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

    _setup_agent_repos() {
      local repo_root agents_dir
      repo_root=$(git rev-parse --show-toplevel 2>/dev/null || true)
      [ -z "$repo_root" ] && return 0
      agents_dir="$repo_root/agents"

      _setup_repo() {
        local name="$1" src="$2" target
        target="$agents_dir/$name/repo"
        mkdir -p "$(dirname "$target")"

        if [ -L "$target" ]; then
          # Existing symlink (from older devShell versions) — replace with writable copy
          echo "ai-workbench: replacing agents/$name/repo symlink with writable copy" >&2
          rm "$target"
          cp -r "$src" "$target"
          chmod -R u+w "$target"
        elif [ -d "$target" ] && [ -n "$(ls -A "$target" 2>/dev/null)" ]; then
          # Real directory with content — user's local clone, leave it alone
          echo "ai-workbench: agents/$name/repo is a local clone; leaving it alone" >&2
        elif [ -d "$target" ]; then
          # Empty directory — populate from flake input
          echo "ai-workbench: populating agents/$name/repo from flake input" >&2
          rmdir "$target" 2>/dev/null || true
          cp -r "$src" "$target"
          chmod -R u+w "$target"
        else
          # Doesn't exist — copy from flake input
          echo "ai-workbench: setting up agents/$name/repo from flake input" >&2
          cp -r "$src" "$target"
          chmod -R u+w "$target"
        fi
      }

      _setup_repo "pi" "${pi}"
      _setup_repo "odysseus" "${odysseus}"
      _setup_repo "opencode" "${opencode}"
      _setup_repo "tempest" "${tempest}"
    }
    _setup_agent_repos
    unset -f _setup_agent_repos _setup_repo

    _build_agents() {
      local repo_root agents_dir
      repo_root=$(git rev-parse --show-toplevel 2>/dev/null || true)
      [ -z "$repo_root" ] && return 0
      agents_dir="$repo_root/agents"

      _build_if_needed() {
        local name="$1" build_cmd="$2" gating_file="''${3:-}"
        local repo_dir build_dir stamp hash_file
        repo_dir="$agents_dir/$name/repo"
        build_dir="$agents_dir/$name/build"
        [ -d "$repo_dir" ] || return 0
        stamp="$build_dir/.ai-workbench-built"
        hash_file="$agents_dir/$name/.build-hash"

        # Optional dep-manifest gating: force a rebuild when the gating
        # file's sha256 differs from the last successful build's stored
        # hash (or when no hash is stored yet). When no gating file is
        # given, fall back to the simple stamp-based skip.
        local current_hash="" stored_hash=""
        if [ -n "$gating_file" ]; then
          if [ -f "$repo_dir/$gating_file" ]; then
            current_hash=$(sha256sum "$repo_dir/$gating_file" 2>/dev/null || true)
          fi
          [ -f "$hash_file" ] && stored_hash=$(cat "$hash_file" 2>/dev/null || true)
          if [ -f "$stamp" ] && [ -n "$current_hash" ] && [ "$current_hash" = "$stored_hash" ]; then
            return 0
          fi
        else
          [ -f "$stamp" ] && return 0
        fi

        echo "ai-workbench: building $name into agents/$name/build..." >&2

        # Copy clean source to build dir (keeps repo/ pristine)
        rm -rf "$build_dir"
        cp -r "$repo_dir" "$build_dir"
        chmod -R u+w "$build_dir"
        rm -rf "$build_dir/.git" "$build_dir/node_modules" "$build_dir/.deps"

        if (cd "$build_dir" && eval "$build_cmd"); then
          touch "$stamp"
          # Persist the gating file hash so the next develop can skip.
          if [ -n "$gating_file" ] && [ -n "$current_hash" ]; then
            echo "$current_hash" > "$hash_file"
          fi
          echo "ai-workbench: $name built successfully" >&2
        else
          rm -rf "$build_dir"
          echo "ai-workbench: WARNING: $name build failed; the agent may not work" >&2
          echo "ai-workbench: You can retry: rm -rf agents/$name/build && nix develop" >&2
        fi
      }

      # Agent builds are driven by config.reference/workestrate.toml local_build.
      ${buildAgentCommands}

      unset -f _build_if_needed
    }
    _build_agents
    unset -f _build_agents

    # Check workload images are loaded (lightweight — skip silently if msb
    # unavailable). Driven by the `workload-images` attrset via `imageNames`
    # — adding an image to the attrset automatically updates this check.
    if command -v msb >/dev/null 2>&1; then
      loaded=$(msb image ls 2>/dev/null || true)
      for img in ${pkgs.lib.concatMapStringsSep " " (x: x) imageNames}; do
        if ! echo "$loaded" | grep -q "$img"; then
          echo "ai-workbench: image '$img' not loaded. Run: just load-images" >&2
        fi
      done
    fi
  '';
}
