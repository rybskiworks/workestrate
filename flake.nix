{
  description = "workestrate — control plane for sandboxed agent/service workloads on Microsandbox microVMs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/a799d3e3886da994fa307f817a6bc705ae538eeb";

    fenix = {
      # Pinned to same rev as nix-tooling (fa09e647...) for reproducible rustc 1.97.1.
      # This is an OWNED pin — nix-tooling also pins this rev, and consumers should not
      # follows-override the toolchain via `tooling.inputs.fenix`.
      url = "github:nix-community/fenix/fa09e6473a0dfd673e6cb9a37741aec513b4bb2a";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    microsandbox-fork = {
      # Pinned fork rev 78fb3ed12623526ad02f5999047c12953013c395 — the merge
      # of fix/stop-process-exit-wait onto develop (0-conflict --no-ff merge
      # on top of 205a7b95), verified pushed to origin via
      # `git ls-remote origin develop`. Moves to `develop` tracking or a
      # signed rev later per the merge runbook (signing currently deferred
      # by user).
      # 2026-08-31: repo transferred to the rybskiworks org (georgrybski ->
      # rybskiworks, same rev). Host relock still pending:
      # `nix flake lock --update-input microsandbox-fork
      # /home/rybski/Development/agent-workbench/workestrate`.
      #
      # This rev carries ALL THREE: (1) the F1 approved-root fix, (2) the
      # write.allow evaluator arm with union semantics (allow∪deny
      # authority-ascending, deny-before-allow within scope, last non-frozen
      # match wins, default Allow, terminal freeze both directions, protect
      # short-circuit), and (3) the stop-exit-wait fix
      # (stop_with_timeout/kill_with_timeout now await the recorded runtime
      # process exit via reap.rs await_recorded_runtime_exit: bounded 30s
      # RUNTIME_EXIT_GRACE pid-exit wait → direct SIGKILL escalation + 5s
      # wait → hard MicrosandboxError::Runtime). The transient build-time
      # patch (nix/patches/mount-policy-approved-root.patch) stays dropped.
      url = "github:rybskiworks/microsandbox/78fb3ed12623526ad02f5999047c12953013c395";
      flake = false;
    };

    # Shared tooling: github:rybskiworks/nix-tooling pinned to 2a57961.
    # For local development use `--override-input tooling path:../nix-tooling`.
    # Follows discipline: share consumer's nixpkgs, but don't override owned pins (fenix/tombi).
    # Therefore we set `tooling.inputs.nixpkgs.follows = "nixpkgs"` but do NOT set
    # `tooling.inputs.fenix.follows` — tooling owns the rust toolchain version.
    tooling = {
      url = "github:rybskiworks/nix-tooling/2a5796179339a2322e3d01f599e82e3322c8ad4b";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts/9d0d87172c374f89da73c1cfe6d81ae62feac1f1";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    devenv = {
      url = "github:cachix/devenv/97135e80b6e432f41f84f72383e1b8147f33ef0c";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        git-hooks.follows = "git-hooks";
        flake-parts.follows = "flake-parts";
      };
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix/27b3b12a8e6375f28ebe122f07d230ca5459bbfa";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    git-hooks = {
      url = "github:cachix/git-hooks.nix/27555e2624241fb116b49095df4caaee85a25691";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix2container = {
      url = "github:nlewo/nix2container";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";

    devenv-root = {
      url = "file+file:///dev/null";
      flake = false;
    };
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devenv.flakeModule
        inputs.treefmt-nix.flakeModule
        inputs.git-hooks.flakeModule
      ];

      systems = [ "x86_64-linux" ];

      # System-agnostic lib exports (mirrors previous `lib.${system}` structure for backward compat).
      # We keep a `flake.lib` that provides the same `libForSystem` interface per system.
      flake =
        let
          # Reusable lib factory (same as previous per-system libForSystem)
          mkLibFor =
            system:
            let
              pkgs = inputs.nixpkgs.legacyPackages.${system};
              referenceConfig = import ./nix/lib/config.nix { };
              recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
            in
            {
              recipes = recipesForPkgs;
              vocabulary = recipesForPkgs.vocab;
              config = referenceConfig;
              buildWorkloadImage = recipesForPkgs.image.nix-layered;
              buildImagesFromConfig =
                {
                  pkgs,
                  config,
                  sources ? { },
                }:
                let
                  recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
                  inherit (pkgs) lib;
                  resolveSrc =
                    srcStr:
                    if lib.hasPrefix "flake://" srcStr then
                      let
                        name = lib.removePrefix "flake://" srcStr;
                      in
                      sources.${name}
                        or (throw "buildImagesFromConfig: unresolved flake:// URI '${srcStr}'; pass sources.${name} = <path/derivation> to buildImagesFromConfig")
                    else
                      srcStr;
                  buildBinary =
                    binary:
                    if binary.recipe == "bun-compile" then
                      recipesForPkgs.build.bun-compile {
                        src = resolveSrc binary.src;
                        inherit (binary) entrypoint;
                        inherit (binary) worker;
                        assets = binary.assets or [ ];
                        binaryName = binary.binary_name or "app";
                        installDir = binary.install_dir or "bin";
                        stripSrcReferences = binary.strip_src_references or true;
                      }
                    else if binary.recipe == "npm-build" then
                      recipesForPkgs.build.npm-build {
                        src = resolveSrc binary.src;
                        npmDepsHash = binary.npm_deps_hash;
                        dontNpmBuild = binary.dont_npm_build or false;
                        buildPhase = binary.build_phase or null;
                        installPhase = binary.install_phase or null;
                      }
                    else if binary.recipe == "pip-install" then
                      recipesForPkgs.build.pip-install {
                        source = resolveSrc binary.src;
                        requirementsFile = binary.requirements_file or "requirements.txt";
                        target = binary.target or ".deps";
                      }
                    else if binary.recipe == "bun-install" then
                      recipesForPkgs.build.bun-install { source = resolveSrc binary.src; }
                    else
                      throw "unknown binary recipe: ${binary.recipe}";
                  workloads = config.workloads or { };
                  names = builtins.attrNames workloads;
                  nixLayered = builtins.filter (name: (workloads.${name}.image.recipe or "") == "nix-layered") names;
                in
                builtins.listToAttrs (
                  map (name: {
                    name = workloads.${name}.image.name;
                    value = recipesForPkgs.image.nix-layered {
                      inherit (workloads.${name}.image) name tag;
                      contents = workloads.${name}.image.contents or [ ];
                      binary =
                        if workloads.${name}.image ? binary then buildBinary workloads.${name}.image.binary else null;
                      bakedFiles = workloads.${name}.image.baked_files or [ ];
                      features = workloads.${name}.image.features or [ ];
                      env = workloads.${name}.image.env or { };
                      extraContents = workloads.${name}.image.extra_contents or [ ];
                    };
                  }) nixLayered
                );
              checks.validateConfig =
                {
                  pkgs,
                  config,
                  workestrate,
                }:
                let
                  configFile = pkgs.writeText "workestrate.toml" config;
                in
                pkgs.runCommand "validate-config"
                  {
                    nativeBuildInputs = [ workestrate ];
                    passAsFile = [ ];
                  }
                  ''
                    mkdir -p $out
                    cp ${configFile} workestrate.toml
                    workestrate validate-config
                    touch $out/ok
                  '';
              checks.tombiCheck =
                {
                  pkgs,
                  src,
                  tombi,
                }:
                pkgs.runCommand "tombi-check" { nativeBuildInputs = [ tombi ]; } ''
                  mkdir -p $out
                  cd ${
                    builtins.path {
                      path = src;
                      filter =
                        path: type:
                        type == "directory"
                        || pkgs.lib.hasSuffix ".toml" (baseNameOf path)
                        || pkgs.lib.hasSuffix ".schema.json" (baseNameOf path);
                      name = "tombi-check-src";
                    }
                  }
                  export TOMBI_OFFLINE=true
                  tombi format --check
                  tombi lint --error-on-warnings
                  touch $out/ok
                '';
            };
        in
        {
          lib = {
            "x86_64-linux" = mkLibFor "x86_64-linux";
          };
        };

      perSystem =
        { config, system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.fenix.overlays.default ];
            config.allowUnfree = true;
          };

          # Pinned Rust toolchain via fenix (must agree with nix-tooling's pin).
          # RUST_TOOLCHAIN_VERSION = "1.97"
          rustToolchain = inputs.fenix.packages.${system}.stable;

          referenceConfig = import ./nix/lib/config.nix { };

          # Per-system lib exports (same as flake.lib above, but with perSystem pkgs)
          libForSystem =
            { pkgs }:
            let
              recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
            in
            {
              recipes = recipesForPkgs;
              vocabulary = recipesForPkgs.vocab;
              config = referenceConfig;
              buildWorkloadImage = recipesForPkgs.image.nix-layered;
              buildImagesFromConfig =
                {
                  pkgs,
                  config,
                  sources ? { },
                }:
                let
                  recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
                  inherit (pkgs) lib;
                  resolveSrc =
                    srcStr:
                    if lib.hasPrefix "flake://" srcStr then
                      let
                        name = lib.removePrefix "flake://" srcStr;
                      in
                      sources.${name}
                        or (throw "buildImagesFromConfig: unresolved flake:// URI '${srcStr}'; pass sources.${name} = <path/derivation> to buildImagesFromConfig")
                    else
                      srcStr;
                  buildBinary =
                    binary:
                    if binary.recipe == "bun-compile" then
                      recipesForPkgs.build.bun-compile {
                        src = resolveSrc binary.src;
                        inherit (binary) entrypoint;
                        inherit (binary) worker;
                        assets = binary.assets or [ ];
                        binaryName = binary.binary_name or "app";
                        installDir = binary.install_dir or "bin";
                        stripSrcReferences = binary.strip_src_references or true;
                      }
                    else if binary.recipe == "npm-build" then
                      recipesForPkgs.build.npm-build {
                        src = resolveSrc binary.src;
                        npmDepsHash = binary.npm_deps_hash;
                        dontNpmBuild = binary.dont_npm_build or false;
                        buildPhase = binary.build_phase or null;
                        installPhase = binary.install_phase or null;
                      }
                    else if binary.recipe == "pip-install" then
                      recipesForPkgs.build.pip-install {
                        source = resolveSrc binary.src;
                        requirementsFile = binary.requirements_file or "requirements.txt";
                        target = binary.target or ".deps";
                      }
                    else if binary.recipe == "bun-install" then
                      recipesForPkgs.build.bun-install { source = resolveSrc binary.src; }
                    else
                      throw "unknown binary recipe: ${binary.recipe}";
                  workloads = config.workloads or { };
                  names = builtins.attrNames workloads;
                  nixLayered = builtins.filter (name: (workloads.${name}.image.recipe or "") == "nix-layered") names;
                in
                builtins.listToAttrs (
                  map (name: {
                    name = workloads.${name}.image.name;
                    value = recipesForPkgs.image.nix-layered {
                      inherit (workloads.${name}.image) name tag;
                      contents = workloads.${name}.image.contents or [ ];
                      binary =
                        if workloads.${name}.image ? binary then buildBinary workloads.${name}.image.binary else null;
                      bakedFiles = workloads.${name}.image.baked_files or [ ];
                      features = workloads.${name}.image.features or [ ];
                      env = workloads.${name}.image.env or { };
                      extraContents = workloads.${name}.image.extra_contents or [ ];
                    };
                  }) nixLayered
                );
              checks.validateConfig =
                {
                  pkgs,
                  config,
                  workestrate,
                }:
                let
                  configFile = pkgs.writeText "workestrate.toml" config;
                in
                pkgs.runCommand "validate-config"
                  {
                    nativeBuildInputs = [ workestrate ];
                    passAsFile = [ ];
                  }
                  ''
                    mkdir -p $out
                    cp ${configFile} workestrate.toml
                    workestrate validate-config
                    touch $out/ok
                  '';
              checks.tombiCheck =
                {
                  pkgs,
                  src,
                  tombi,
                }:
                pkgs.runCommand "tombi-check" { nativeBuildInputs = [ tombi ]; } ''
                  mkdir -p $out
                  cd ${
                    builtins.path {
                      path = src;
                      filter =
                        path: type:
                        type == "directory"
                        || pkgs.lib.hasSuffix ".toml" (baseNameOf path)
                        || pkgs.lib.hasSuffix ".schema.json" (baseNameOf path);
                      name = "tombi-check-src";
                    }
                  }
                  export TOMBI_OFFLINE=true
                  tombi format --check
                  tombi lint --error-on-warnings
                  touch $out/ok
                '';
            };

          agentd = pkgs.callPackage ./nix/packages/agentd.nix { inherit (inputs) microsandbox-fork; };
          microsandbox = pkgs.callPackage ./nix/packages/microsandbox.nix {
            inherit rustToolchain agentd;
            inherit (inputs) microsandbox-fork;
          };
          tombi = inputs.tooling.packages.${system}.tombi;
          microsandbox-filesystem-patched =
            pkgs.callPackage ./nix/packages/microsandbox-filesystem-patched.nix
              { inherit (inputs) microsandbox-fork; };
          workestrate = pkgs.callPackage ./nix/packages/agentctl.nix {
            inherit microsandbox microsandbox-filesystem-patched rustToolchain;
            rev =
              inputs.self.shortRev
                or (if inputs.self ? rev then builtins.substring 0 8 inputs.self.rev else "dirty");
          };

          msb-wrapped = pkgs.runCommand "msb-wrapped" { nativeBuildInputs = [ pkgs.makeWrapper ]; } ''
            mkdir -p $out/bin
            makeWrapper ${microsandbox}/bin/msb $out/bin/msb \
              --run 'export MSB_HOME="$HOME/.microsandbox"'
          '';

          decrypt-env = pkgs.writeShellApplication {
            name = "decrypt-env";
            runtimeInputs = [ pkgs.sops ];
            text = ''
              set -euo pipefail
              : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
              export SOPS_AGE_KEY_FILE
              secret_file="''${SECRET_FILE:-.env.enc}"
              if [ ! -f "$secret_file" ]; then
                echo "error: secret file not found: $secret_file" >&2
                exit 1
              fi
              exec sops decrypt --input-type dotenv --output-type dotenv "$secret_file"
            '';
          };

          write-env = pkgs.writeShellApplication {
            name = "write-env";
            runtimeInputs = [
              pkgs.sops
              pkgs.coreutils
            ];
            text = ''
              set -euo pipefail
              : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
              export SOPS_AGE_KEY_FILE
              secret_file="''${SECRET_FILE:-.env.enc}"
              if [ ! -f "$secret_file" ]; then
                echo "error: secret file not found: $secret_file" >&2
                exit 1
              fi
              if [ -f .env ]; then
                echo "error: .env already exists; remove it first" >&2
                exit 1
              fi
              umask 077
              tmp=$(mktemp)
              trap 'rm -f "$tmp"' EXIT
              sops decrypt --input-type dotenv --output-type dotenv "$secret_file" > "$tmp"
              mv "$tmp" .env
              chmod 600 .env
              echo "wrote plaintext .env; delete it when done"
            '';
          };

          setup-secrets = pkgs.writeShellApplication {
            name = "setup-secrets";
            runtimeInputs = [
              pkgs.sops
              pkgs.age
              pkgs.coreutils
              pkgs.gnugrep
              pkgs.gnused
            ];
            text = ''
              set -euo pipefail
              : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
              export SOPS_AGE_KEY_FILE
              exec ${./scripts/setup-secrets.sh} "$@"
            '';
          };

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
        {
          _module.args.pkgs = pkgs;

          # treefmt for `nix fmt` and `nix flake check` (including tombi via tooling)
          # rustfmt edition 2021 matches control/agentctl's Cargo.toml (edition = "2021");
          # using default 2024 would treat `gen` as reserved keyword and fail on schemars derives.
          treefmt.config = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              statix.enable = true;
              rustfmt = {
                enable = true;
                edition = "2021";
              };
            };
            settings.formatter.tombi = {
              command = "${tombi}/bin/tombi";
              options = [ "format" ];
              includes = [ "*.toml" ];
            };
          };

          # git-hooks for `nix flake check` (pre-commit)
          # typos is disabled for workestrate's `nix flake check` because docs contain intentional
          # domain terms (FOD, FODs, etc.) and test fixtures (evn, daa) that would be flagged as
          # false positives. The base module still enables typos for `nix develop` (auto-fix workflow)
          # but CI `nix flake check` would be too noisy. Override to disable here.
          # clippy/rustfmt/deadnix are also disabled for `nix flake check`: clippy/rustfmt require
          # Cargo.toml at repo root (ours is at control/agentctl/Cargo.toml) and deadnix flags
          # intentional unused patterns (flake.nix self, etc.). Full Rust checks remain via `just verify`.
          pre-commit = {
            check.enable = true;
            settings.hooks = {
              nixfmt.enable = true;
              statix.enable = true;
              deadnix.enable = false;
              typos.enable = false;
              treefmt = {
                enable = true;
                settings.fail-on-change = true;
              };
              tombi-lint = {
                enable = true;
                name = "tombi lint";
                entry = "${tombi}/bin/tombi lint --error-on-warnings";
                files = "\\.toml$";
                pass_filenames = false;
              };
              rustfmt.enable = false;
              clippy.enable = false;
            };
          };

          packages = {
            inherit
              agentd
              workestrate
              microsandbox
              microsandbox-filesystem-patched
              msb-wrapped
              decrypt-env
              write-env
              setup-secrets
              tombi
              ;
            default = workestrate;
          };

          apps.default = {
            type = "app";
            program = "${workestrate}/bin/workestrate";
          };

          # Expose lib per system for backward compat via `config.packages`? Instead we set `flake.lib` above.
          # For `nix flake check` we also provide tombiCheck and treefmt checks.
          checks = {
            # treefmt and pre-commit are auto-generated; add tombiCheck for completeness
            tombiCheck = pkgs.runCommand "tombi-check" { nativeBuildInputs = [ tombi ]; } ''
              mkdir -p $out
              cd ${
                builtins.path {
                  path = ./.;
                  filter =
                    path: type:
                    type == "directory"
                    || pkgs.lib.hasSuffix ".toml" (baseNameOf path)
                    || pkgs.lib.hasSuffix ".schema.json" (baseNameOf path);
                  name = "tombi-check-src";
                }
              }
              export TOMBI_OFFLINE=true
              tombi format --check
              tombi lint --error-on-warnings
              touch $out/ok
            '';
          };

          # Devenv shell: dogfoods tooling modules + workestrate-specific packages and shellHook
          # NOTE: devenv-test uses IFD (import source); first eval on a fresh store must warm it via `nix build .#packages.x86_64-linux.devenv-test`, then `nix flake check --no-build` is fine.
          devenv.shells.default = {
            devenv.root =
              let
                pwd = builtins.getEnv "PWD";
              in
              if pwd != "" then pwd else toString ./.;

            imports = [
              inputs.tooling.devenvModules.base
              inputs.tooling.devenvModules.nix
              inputs.tooling.devenvModules.toml
              inputs.tooling.devenvModules.rust
            ];

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
              nodejs_24
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
              tombi
              write-env
              setup-secrets
            ];

            # Preserve the extensive shellHook from the previous mkShell (staging msb, vendor, agent builds)
            enterShell = ''
              echo "workestrate dev shell"
              echo "msb version: $(msb --version 2>/dev/null || echo 'not available')"
              echo "secrets workflow: docs/secrets.md"

              # Stage Microsandbox runtime for offline cargo check.
              _msb_home="$HOME/.cache/ai-workbench-msb"
              rm -rf "$_msb_home/bin" "$_msb_home/lib"
              mkdir -p "$_msb_home/bin" "$_msb_home/lib"
              for _old in /run/user/*/ai-workbench-msb-*; do
                if [ -e "$_old" ]; then
                  rm -rf "$_old" 2>/dev/null || true
                fi
              done
              ln -sfn ${microsandbox}/bin/msb "$_msb_home/bin/msb"
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
              export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"
              mkdir -p "$CARGO_TARGET_DIR"
              export MSB_HOME="$_msb_home"
              export MSB_PATH="$_msb_home/bin/msb"
              export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"

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
                  rm -rf "$build_dir"
                  cp -r "$repo_dir" "$build_dir"
                  chmod -R u+w "$build_dir"
                  rm -rf "$build_dir/.git" "$build_dir/node_modules" "$build_dir/.deps"
                  if (cd "$build_dir" && eval "$build_cmd"); then
                    touch "$stamp"
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
                ${buildAgentCommands}
                unset -f _build_if_needed
              }
              _build_agents
              unset -f _build_agents
              unset -f _tool_repo_root
            '';
          };
        };
    };
}
