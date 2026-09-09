{
  description = "workestrate — control plane for sandboxed agent/service workloads on Microsandbox microVMs";

  inputs = {
    # Shared build tools and development modules have one version authority.
    tooling.url = "github:rybskiworks/nix-tooling/34c287290245c20e9103f7cd0fcdaa244edd310b";
    nixpkgs.follows = "tooling/nixpkgs";
    fenix.follows = "tooling/fenix";
    flake-parts.follows = "tooling/flake-parts";
    devenv.follows = "tooling/devenv";
    treefmt-nix.follows = "tooling/treefmt-nix";
    git-hooks.follows = "tooling/git-hooks";

    microsandbox-fork = {
      # Runtime packages and SDK patches must come from this same source.
      url = "github:rybskiworks/microsandbox/a1dad1bf2e17df62c80510d070d1a1aa41ef2224";
      inputs.tooling.follows = "tooling";
    };

    nix2container = {
      url = "github:nlewo/nix2container/76be9608a7f4d6c985d28b0e7be903ae2547df3e";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin/ff5d8bd4d68a347be5042e2f16caee391cd75887";

    devenv-root = {
      url = "file+file:///dev/null";
      flake = false;
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
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
        {
          system,
          inputs',
          config,
          ...
        }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.fenix.overlays.default ];
            config.allowUnfree = true;
          };

          # Pinned Rust toolchain via fenix (must agree with nix-tooling's pin).
          # NOTE: the RUST_TOOLCHAIN_VERSION marker trails the live
          # `rustToolchain` line on purpose — toolchain-check greps it there,
          # so the marker cannot silently drift onto a dead line.
          rustToolchain = inputs.fenix.packages.${system}.stable; # RUST_TOOLCHAIN_VERSION = "1.97"

          microsandboxSource = inputs'.microsandbox-fork.packages.microsandbox.src;
          # Metadata comes from the already-fetched input; reading the filtered
          # package source would require a store write during cold read-only eval.
          forkVersion =
            (builtins.fromTOML (builtins.readFile (inputs.microsandbox-fork.outPath + "/Cargo.toml")))
            .workspace.package.version;
          agentd =
            assert pkgs.lib.assertMsg (
              inputs'.microsandbox-fork.packages.agentd.version == forkVersion
            ) "Microsandbox agentd package and SDK source versions disagree";
            inputs'.microsandbox-fork.packages.agentd;
          microsandbox =
            assert pkgs.lib.assertMsg (
              inputs'.microsandbox-fork.packages.microsandbox.version == forkVersion
            ) "Microsandbox runtime package and SDK source versions disagree";
            inputs'.microsandbox-fork.packages.microsandbox;
          tombi = inputs.tooling.packages.${system}.tombi;
          # Retain the source-package output for downstream development tools;
          # compilation shares the fork package's filtered Rust workspace.
          microsandbox-filesystem-patched = pkgs.runCommand "microsandbox-filesystem-patched-${forkVersion}" {
            version = forkVersion;
          } "ln -s ${microsandboxSource} $out";
          workestrate = pkgs.callPackage ./nix/packages/agentctl.nix {
            inherit microsandbox microsandboxSource rustToolchain;
            rev =
              inputs.self.shortRev
                or (if inputs.self ? rev then builtins.substring 0 8 inputs.self.rev else "dirty");
          };

          msb-wrapped = pkgs.runCommand "msb-wrapped" { nativeBuildInputs = [ pkgs.makeWrapper ]; } ''
            mkdir -p $out/bin
            # Guarded MSB_HOME default per the msb state-generations model: the
            # canonical home is the `current` generation symlink (see
            # nix/packages/agentctl.nix postInstall + docs/runtime-provisioning.md).
            # A non-empty MSB_HOME still wins verbatim; empty is treated as
            # unset, so --set-default would not handle empty.
            makeWrapper ${microsandbox}/bin/msb $out/bin/msb \
              --run 'if [ -z "''${MSB_HOME:-}" ]; then export MSB_HOME="$HOME/.microsandbox/current"; fi'
          '';

          install-hooks = pkgs.writeShellScriptBin "install-hooks" ''
            set -e
            export PATH=${
              pkgs.lib.makeBinPath [
                pkgs.coreutils
                pkgs.nix
                config.pre-commit.settings.gitPackage
              ]
            }:$PATH
            ${config.pre-commit.installationScript}
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

        in
        {
          _module.args.pkgs = pkgs;

          # treefmt for `nix fmt` and `nix flake check` (including tombi via tooling)
          # rustfmt edition 2024 matches control/agentctl's Cargo.toml (edition = "2024");
          # the crate is edition-2024-clean (`gen` identifiers renamed to `generator`, f20bfca).
          treefmt.config = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              statix.enable = true;
              rustfmt = {
                enable = true;
                edition = "2024";
                package = rustToolchain.rustfmt;
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
            beads = inputs'.tooling.packages.beads;
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

          apps = {
            default = {
              type = "app";
              program = "${workestrate}/bin/workestrate";
              meta.description = "Run the pinned Workestrate CLI";
            };
            install-hooks = {
              type = "app";
              program = "${install-hooks}/bin/install-hooks";
              meta.description = "Install the pinned repository validation hooks";
            };
            beads = {
              type = "app";
              program = "${inputs'.tooling.packages.beads}/bin/bd";
              meta.description = "Run the shared pinned Beads tracker CLI";
            };
          };

          # Expose lib per system for backward compat via `config.packages`? Instead we set `flake.lib` above.
          # For `nix flake check` we also provide tombiCheck and treefmt checks.
          checks = {
            deny = config.checks.unit.overrideAttrs (old: {
              pname = "workestrate-deny-check";
              nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.cargo-deny ];
              buildPhase = ''
                runHook preBuild
                cargo deny --locked --offline --manifest-path Cargo.toml check \
                  --config ${./deny.toml} licenses bans sources
                runHook postBuild
              '';
              doCheck = false;
              installPhase = "mkdir -p $out";
              postInstall = "";
            });

            # Consumer homes are deployment state, not repository fixtures.
            # Unit tests cover schema generation and distribution into scratch
            # homes; this checks the copies actually shipped by this repository.
            schemaSync = pkgs.runCommand "workestrate-schema-sync-check" { } ''
              for name in workestrate.schema.json workestrate-workload.schema.json registry.schema.json; do
                cmp ${./schemas}/"$name" ${./templates/workestrate-config/schemas}/"$name"
              done
              mkdir -p $out
            '';

            rust = config.checks.unit.overrideAttrs (old: {
              pname = "workestrate-rust-check";
              nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
                rustToolchain.rustfmt
                rustToolchain.clippy
              ];
              buildPhase = ''
                runHook preBuild
                cargo fmt -- --check
                cargo clippy --jobs "$NIX_BUILD_CORES" --locked --offline --all-targets -- -D warnings
                cargo check --jobs "$NIX_BUILD_CORES" --locked --offline
                runHook postBuild
              '';
              doCheck = false;
              installPhase = "mkdir -p $out";
            });

            package =
              pkgs.runCommand "workestrate-package-check"
                {
                  nativeBuildInputs = [ pkgs.jq ];
                }
                ''
                  export MSB_HOME="$TMPDIR/msb-state"
                  ${workestrate}/bin/workestrate --version
                  ${workestrate}/bin/workestrate --help > /dev/null
                  ${workestrate}/bin/workestrate --json versions > versions.json
                  jq -e \
                    --arg msb "${microsandbox}/bin/msb" \
                    --arg version "msb ${forkVersion}" \
                    --arg agentd "${microsandbox}/libexec/agentd" \
                    '.msb.path_used == $msb and .msb.version == $version and .agentd.path == $agentd' \
                    versions.json
                  test ! -e "$MSB_HOME"
                  mkdir -p $out
                  cp versions.json $out/
                '';

            # Reuse the package's offline SDK/runtime inputs. Test-only fixtures
            # stay out of the production source; existing KVM ignores remain intact.
            unit = workestrate.overrideAttrs (old: {
              pname = "workestrate-tests";
              src = pkgs.runCommand "source" { } ''
                mkdir -p $out/docs/migration $out/templates
                cp -r ${old.src}/. $out/
                cp -r ${./config.reference} $out/config.reference
                cp ${./docs/migration/20-target-system-spec.md} $out/docs/migration/20-target-system-spec.md
                cp -r ${./templates/workestrate-config} $out/templates/workestrate-config
                cp ${./flake.nix} $out/flake.nix
              '';
              cargoBuildType = "debug";
              cargoCheckType = "debug";
              doCheck = true;
              cargoTestFlags = [
                "--locked"
                "--no-fail-fast"
              ];
              nativeCheckInputs = [
                pkgs.git
                pkgs.sops
                pkgs.age
                tombi
              ];
              preCheck = ''
                export HOME="$TMPDIR/test-home"
                export XDG_CACHE_HOME="$HOME/.cache"
                # These variables select legacy home resolution; tests set them
                # explicitly when exercising that compatibility behavior.
                unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME
                mkdir -p "$HOME" "$XDG_CACHE_HOME"
                export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                export TOMBI_OFFLINE=true
                export GIT_CONFIG_NOSYSTEM=1
                export GIT_CONFIG_GLOBAL=/dev/null
                # Daemon-backed image tests and optional Copier round trips have
                # separate host gates; do not install their tools in this sandbox.
              '';
            });

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

          # Tooling-only shell: usable even when the application or runtime does
          # not build. It deliberately does not mark runtime staging complete.
          devenv.shells.bootstrap = {
            imports = [
              inputs.tooling.devenvModules.beads
              inputs.tooling.devenvModules.base
              inputs.tooling.devenvModules.nix
              inputs.tooling.devenvModules.toml
              inputs.tooling.devenvModules.rust
            ];
            packages = with pkgs; [
              cargo-deny
              gcc
              just
              libcap_ng
              nix
              pkg-config
              python3
            ];
            # Bootstrap must not install hooks or run worktree formatting.
            git-hooks.enable = false;
            treefmt.enable = false;
            enterShell = ''
              export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"
              echo "workestrate bootstrap shell (tooling only)"
            '';
          };

          # Devenv shell: dogfoods tooling modules + workestrate-specific packages and shellHook
          # NOTE: devenv-test uses IFD (import source); first eval on a fresh store must warm it via `nix build .#packages.x86_64-linux.devenv-test`, then `nix flake check --no-build` is fine — pure eval also needs `--override-input devenv-root "file+file://$HOME/.cache/workestrate/devenv-root/workestrate"` (or the shell must be warmed via an entry point that passes it).
          devenv.shells.default = {
            # devenv.root is intentionally NOT set here. The auto-imported
            # readDevenvRoot module (inputs.devenv.flakeModule) sets it from
            # the `devenv-root` input placeholder (see inputs above;
            # --override-input never touches flake.lock). Entry points pass
            #   --override-input devenv-root "file+file://<rootfile>"
            # where <rootfile> holds the worktree abs path — the `shell`
            # recipe, the self-enshelling just guards, and
            # scripts/kvm-tests.sh write
            # $HOME/.cache/workestrate/devenv-root/workestrate. Impure eval
            # falls back to devenv's mkDefault (getEnv PWD). Pure eval
            # WITHOUT the override fails devenv's `devenv.root != ""`
            # assertion by design — the old store-path fallback put
            # dotfile/state into a read-only /nix/store copy (enterShell
            # mkdir: Permission denied). Dotfile/state keep devenv defaults:
            # dotfile = <root>/.devenv (gitignored in-tree), state =
            # <dotfile>/state. NOTE (pinned devenv 97135e80): tasks.nix
            # wires the task cache to devenv.dotfile — there is NO separate
            # task-cache-dir knob — so the cache lives in <worktree>/.devenv
            # too. The old $HOME-cache dotfile redirect was pure-eval-inert
            # (getEnv HOME == "" under pure eval) and is removed with the
            # root fallback.

            imports = [
              inputs.tooling.devenvModules.beads
              inputs.tooling.devenvModules.base
              inputs.tooling.devenvModules.nix
              inputs.tooling.devenvModules.toml
              inputs.tooling.devenvModules.rust
            ];

            packages = with pkgs; [
              actionlint
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
              cargo-deny
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
              zizmor
            ];

            # Hook installation is explicit (nix run .#install-hooks). Entering
            # a shell must not format files, build workloads or clean VM state.
            git-hooks.enable = false;
            treefmt.enable = false;

            enterShell = ''
              echo "workestrate dev shell"
              echo "msb version: $(msb --version 2>/dev/null || echo 'not available')"
              echo "secrets workflow: docs/secrets.md"

              # Build inputs are immutable and separate from runtime state.
              # The SDK validates this explicit directory without downloading.
              export MSB_BUILD_RUNTIME="${microsandbox}"
              export MSB_AGENTD_PATH="${microsandbox}/libexec/agentd"
              export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"
              export WORKESTRATE_DEVSHELL=1

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
                target="${microsandboxSource}"
                mkdir -p "$vendor_dir"
                if [ -L "$vendor_link" ]; then
                  local current
                  current=$(readlink -f "$vendor_link" 2>/dev/null || true)
                  # The pinned input's store path changes on every pin bump; a
                  # still-present old generation must not suppress the refresh
                  # (staleness is silent, GC makes it loud).
                  if [ -z "$current" ] || [ ! -d "$current" ]; then
                    echo "workestrate: refreshing stale vendor symlink" >&2
                    ln -sfn "$target" "$vendor_link"
                  elif [ "$current" != "$target" ]; then
                    echo "workestrate: vendor symlink re-pointed to pinned fork generation" >&2
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
              unset -f _tool_repo_root
            '';
          };
        };
    };
}
