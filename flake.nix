{
  description = "nix hash fixer";

  nixConfig = {
    extra-substituters = [
      "https://cache.trev.zip/nur"
    ];
    extra-trusted-public-keys = [
      "nur:70xGHUW1+1b8FqBchldaunN//pZNVo6FKuPL4U/n844="
    ];
  };

  inputs = {
    systems.url = "github:nix-systems/default";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    trev = {
      url = "github:spotdemo4/nur";
      inputs.systems.follows = "systems";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      trev,
      ...
    }:
    trev.libs.mkFlake (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            trev.overlays.packages
            trev.overlays.libs
          ];
        };
      in
      rec {
        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # bash
              mktemp
              sd
              shellcheck

              # util
              bumper

              # lint
              nixfmt
              prettier
            ];
            shellHook = pkgs.shellhook.ref;
          };

          check = pkgs.mkShell {
            packages =
              let
                nix-fix-hash = packages.default;
              in
              [
                nix-fix-hash
              ];
          };

          update = pkgs.mkShell {
            packages = with pkgs; [
              renovate
            ];
          };

          vulnerable = pkgs.mkShell {
            packages = with pkgs; [
              # nix
              flake-checker

              # actions
              octoscan
            ];
          };
        };

        checks = pkgs.lib.mkChecks {
          bash = {
            src = packages.default;
            deps = with pkgs; [
              shellcheck
            ];
            script = ''
              shellcheck nix-fix-hash.sh
            '';
          };

          action = {
            src = ./.;
            deps = with pkgs; [
              action-validator
            ];
            script = ''
              action-validator action.yaml
            '';
          };

          nix = {
            src = ./.;
            deps = with pkgs; [
              nixfmt-tree
            ];
            script = ''
              treefmt --ci
            '';
          };

          actions = {
            src = ./.;
            deps = with pkgs; [
              prettier
              action-validator
              octoscan
              renovate
            ];
            script = ''
              prettier --check "**/*.json" "**/*.yaml"
              action-validator .github/**/*.yaml
              octoscan scan .github
              renovate-config-validator .github/renovate.json
            '';
          };
        };

        packages = {
          default = pkgs.stdenv.mkDerivation (finalAttrs: {
            pname = "nix-fix-hash";
            version = "0.1.0";

            src = builtins.path {
              name = "root";
              path = ./.;
            };

            nativeBuildInputs = with pkgs; [
              shellcheck
            ];

            runtimeInputs = with pkgs; [
              mktemp
              sd
            ];

            unpackPhase = ''
              cp "$src/nix-fix-hash.sh" nix-fix-hash.sh
            '';

            dontBuild = true;

            configurePhase = ''
              sed -i '1c\#!${pkgs.runtimeShell}' nix-fix-hash.sh
              sed -i '2c\export PATH="${pkgs.lib.makeBinPath finalAttrs.runtimeInputs}:$PATH"' nix-fix-hash.sh
            '';

            doCheck = true;
            checkPhase = ''
              shellcheck nix-fix-hash.sh
            '';

            installPhase = ''
              mkdir -p $out/bin
              cp nix-fix-hash.sh $out/bin/nix-fix-hash
            '';

            dontFixup = true;

            meta = {
              description = "nix hash fixer";
              mainProgram = "nix-fix-hash";
              homepage = "https://github.com/spotdemo4/nix-fix-hash";
              changelog = "https://github.com/spotdemo4/nix-fix-hash/releases/tag/v${finalAttrs.version}";
              platforms = pkgs.lib.platforms.all;
            };
          });

          image = pkgs.dockerTools.buildLayeredImage {
            fromImage = pkgs.dockerTools.pullImage {
              imageName = "nixos/nix";
              imageDigest = "sha256:0d9c872db1ca2f3eaa4a095baa57ed9b72c09d53a0905a4428813f61f0ea98db";
              hash = "sha256-H7uT+XPp5xadUzP2GEq031yZSIfzpZ1Ps6KVeBTIhOg=";
            };

            name = packages.default.pname;
            tag = packages.default.version;
            created = "now";
            meta = packages.default.meta;
            contents = with pkgs; [
              packages.default
              dockerTools.caCertificates
            ];

            config = {
              Cmd = [ "${pkgs.lib.meta.getExe packages.default}" ];
              Env = [ "DOCKER=true" ];
            };
          };
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
