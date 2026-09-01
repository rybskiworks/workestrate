{
  pkgs,
  microsandbox,
  microsandbox-filesystem-patched,
  workestrate,
  rustToolchain,
  msb-wrapped,
  decrypt-env,
  write-env,
  setup-secrets,
  tombi,
  referenceConfig,
}:

# Cleanup phase 3: the pi build + agents/* repo population moved to the
# personal config repo flake (which owns the pi/odysseus/opencode/tempest
# source inputs and image builds). WORKESTRATE_PI_BUILD is no longer
# exported here; phase 4 owns devshell genericization.

# Build commands derived from config.reference/workestrate.toml.
# The devshell reads ONLY the reference config (tool-dev). Each workload
# with a local_build recipe becomes one _build_if_needed invocation.
let
  localBuildNames = referenceConfig.localBuilds;

  recipeCmd =
    lb:
    if lb.recipe == "pip-install" then
      let
        req = lb.requirements_file or "requirements.txt";
      in
      ''REQ=$([ -f requirements.lock ] && echo requirements.lock || echo ${req}) && python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./.deps -r "$REQ"''
    else if lb.recipe == "bun-install" then
      "HUSKY=0 bun install"
    else if lb.recipe == "npm-build" then
      "npm install && npm run build"
    else
      throw "unknown local_build recipe: ${lb.recipe}";

  buildAgentCommands = builtins.concatStringsSep "\n" (
    map (
      name:
      let
        lb = referenceConfig.workloads.${name}.local_build;
      in
      ''_build_if_needed "${name}" "${recipeCmd lb}" "${lb.gating_file or ""}"''
    ) localBuildNames
  );
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
    msb-wrapped
    nodejs_24 # Node 24: aligns with the node:24 sandbox images (agent workloads need >=23.6)
    bun
    openssl
    pkg-config
    (python3.withPackages (p: [ p.pip ]))
    (python312.withPackages (ps: [
      ps.pip
      ps."pip-tools"
    ]))
    rustToolchain.rustc
    rustToolchain.rust-analyzer
    rustToolchain.rustfmt
    sops
    tombi # TOML formatter/linter/LSP (spec 15)
    write-env
    setup-secrets
  ];

  shellHook = ''
    echo "workestrate dev shell"
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
    export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"

    # Cleanup phase 3: the WORKESTRATE_PI_BUILD export moved to the personal
    # config repo flake (pi build now lives there); phase 4 owns devshell
    # genericization.

    # Cleanup phase 4: the hook must ONLY touch the workestrate tool
    # checkout. `git rev-parse --show-toplevel` resolves from the CALLER's
    # cwd, so running `nix develop <tool-flake>` from another repo used to
    # litter vendor symlinks and agents/*/build dirs into THAT repo. The
    # marker probe (flake.nix + control/agentctl/Cargo.toml +
    # config.reference/workestrate.toml) identifies the tool checkout; any
    # other toplevel (or none) skips the repo-mutating steps.
    _tool_repo_root() {
      local root
      root=$(git rev-parse --show-toplevel 2>/dev/null || true)
      if [ -n "$root" ] \
        && [ -f "$root/flake.nix" ] \
        && [ -f "$root/control/agentctl/Cargo.toml" ] \
        && [ -f "$root/config.reference/workestrate.toml" ]; then
        printf '%s' "$root"
      fi
    }

    _setup_vendor_link() {
      local repo_root vendor_dir vendor_link target
      repo_root=$(_tool_repo_root)
      if [ -z "$repo_root" ]; then
        return 0
      fi
      vendor_dir="$repo_root/control/agentctl/vendor"
      vendor_link="$vendor_dir/microsandbox-fork"
      target="${microsandbox-filesystem-patched}"

      mkdir -p "$vendor_dir"

      if [ -L "$vendor_link" ]; then
        local current
        current=$(readlink -f "$vendor_link" 2>/dev/null || true)
        if [ -z "$current" ] || [ ! -d "$current" ]; then
          echo "workestrate: refreshing stale vendor symlink" >&2
          ln -sfn "$target" "$vendor_link"
        fi
      elif [ -e "$vendor_link" ]; then
        echo "workestrate: vendor/microsandbox-fork is a real directory (unlocked); leaving it alone" >&2
      else
        ln -sfn "$target" "$vendor_link"
      fi
    }
    _setup_vendor_link
    unset -f _setup_vendor_link

    # Cleanup phase 3: _setup_agent_repos removed — the agents/*/repo
    # population was fed by the deleted pi/odysseus/opencode/tempest flake
    # inputs; those now live in the personal config repo flake (phase 4 owns
    # devshell genericization).

    _build_agents() {
      local repo_root agents_dir
      repo_root=$(_tool_repo_root)
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

        echo "workestrate: building $name into agents/$name/build..." >&2

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
          echo "workestrate: $name built successfully" >&2
        else
          rm -rf "$build_dir"
          echo "workestrate: WARNING: $name build failed; the agent may not work" >&2
          echo "workestrate: You can retry: rm -rf agents/$name/build && nix develop" >&2
        fi
      }

      # Agent builds are driven by config.reference/workestrate.toml local_build.
      ${buildAgentCommands}

      unset -f _build_if_needed
    }
    _build_agents
    unset -f _build_agents
    unset -f _tool_repo_root
  '';
}
