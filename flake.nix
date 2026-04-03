{
  description = "nix hash fixer";

  nixConfig = {
    extra-substituters = [
      "https://nix.trev.zip"
    ];
    extra-trusted-public-keys = [
      "trev:I39N/EsnHkvfmsbx8RUW+ia5dOzojTQNCTzKYij1chU="
    ];
  };

  inputs = {
    systems.url = "github:spotdemo4/systems";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    trev = {
      url = "github:spotdemo4/nur";
      inputs.systems.follows = "systems";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      trev,
      ...
    }:
    trev.libs.mkFlake (
      system: pkgs: {
        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # deps
              mktemp
              sd

              # lint
              shellcheck

              # format
              nixfmt
              prettier

              # util
              bumper
            ];
            shellHook = pkgs.shellhook.ref;
          };

          check = pkgs.mkShell {
            packages = [
              self.packages.${system}.default
            ];
          };

          update = pkgs.mkShell {
            packages = with pkgs; [
              renovate
            ];
          };

          vulnerable = pkgs.mkShell {
            packages = with pkgs; [
              flake-checker # nix
              octoscan # actions
            ];
          };
        };

        checks = pkgs.mkChecks {
          bash = {
            src = self.packages.${system}.default;
            deps = with pkgs; [
              shellcheck
            ];
            script = ''
              shellcheck nix-fix-hash.sh
            '';
          };

          actions = {
            root = ./.;
            fileset = ./.github/workflows;
            deps = with pkgs; [
              action-validator
              octoscan
            ];
            forEach = ''
              action-validator "$file"
              octoscan scan "$file"
            '';
          };

          renovate = {
            root = ./.github;
            fileset = ./.github/renovate.json;
            deps = with pkgs; [
              renovate
            ];
            script = ''
              renovate-config-validator renovate.json
            '';
          };

          nix = {
            root = ./.;
            filter = file: file.hasExt "nix";
            deps = with pkgs; [
              nixfmt
            ];
            forEach = ''
              nixfmt --check "$file"
            '';
          };

          prettier = {
            root = ./.;
            filter = file: file.hasExt "yaml" || file.hasExt "json" || file.hasExt "md";
            deps = with pkgs; [
              prettier
            ];
            forEach = ''
              prettier --check "$file"
            '';
          };
        };

        packages = with pkgs.lib; {
          default = pkgs.stdenv.mkDerivation (finalAttrs: {
            pname = "nix-fix-hash";
            version = "0.1.1";

            src = builtins.path {
              name = "root";
              path = ./.;
            };

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
              sed -i '2c\export PATH="${makeBinPath finalAttrs.runtimeInputs}:$PATH"' nix-fix-hash.sh
            '';

            installPhase = ''
              mkdir -p "$out/bin"
              cp nix-fix-hash.sh "$out/bin/nix-fix-hash"
            '';

            dontFixup = true;

            meta = {
              mainProgram = "nix-fix-hash";
              description = "nix hash fixer";
              license = licenses.mit;
              platforms = platforms.all;
              homepage = "https://github.com/spotdemo4/nix-fix-hash";
              changelog = "https://github.com/spotdemo4/nix-fix-hash/releases/tag/v${finalAttrs.version}";
            };
          });
        };

        images = {
          default = pkgs.mkImage self.packages.${system}.default {
            fromImage = pkgs.image.nix;
            contents = with pkgs; [ dockerTools.caCertificates ];
          };
        };

        formatter = pkgs.nixfmt-tree;
        schemas = trev.schemas;
      }
    );
}
