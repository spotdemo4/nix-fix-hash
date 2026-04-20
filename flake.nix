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
              zizmor # actions
            ];
          };
        };

        checks = pkgs.mkChecks {
          bash = {
            src = self.packages.${system}.default;
            packages = with pkgs; [
              shellcheck
            ];
            script = ''
              shellcheck nix-fix-hash.sh
            '';
          };

          actions = {
            root = ./.;
            files = ./.github/workflows;
            packages = with pkgs; [
              action-validator
              zizmor
            ];
            forEach = ''
              action-validator "$file"
              zizmor --offline "$file"
            '';
          };

          renovate = {
            root = ./.github;
            files = ./.github/renovate.json;
            packages = with pkgs; [
              renovate
            ];
            script = ''
              renovate-config-validator renovate.json
            '';
          };

          nix = {
            root = ./.;
            filter = file: file.hasExt "nix";
            packages = with pkgs; [
              nixfmt
            ];
            forEach = ''
              nixfmt --check "$file"
            '';
          };

          prettier = {
            root = ./.;
            filter = file: file.hasExt "yaml" || file.hasExt "json" || file.hasExt "md";
            packages = with pkgs; [
              prettier
            ];
            forEach = ''
              prettier --check "$file"
            '';
          };
        };

        packages.default = pkgs.stdenv.mkDerivation (
          final: with pkgs.lib; {
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
              sed -i '2c\export PATH="${makeBinPath final.runtimeInputs}:$PATH"' nix-fix-hash.sh
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
              changelog = "https://github.com/spotdemo4/nix-fix-hash/releases/tag/v${final.version}";
            };
          }
        );

        images.default = pkgs.mkImage {
          fromImage = pkgs.image.nix;
          src = self.packages.${system}.default;
          contents = with pkgs; [ dockerTools.caCertificates ];
        };

        formatter = pkgs.nixfmt-tree;
        schemas = trev.schemas;
      }
    );
}
