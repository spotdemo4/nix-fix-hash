{
  description = "Nix hash fixer";

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
      url = "github:spotdemo4/trevpkgs";
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

        # nix develop [#...]
        devShells = {
          default = pkgs.mkShell {
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            shellHook = pkgs.shellhook.ref;
            packages = with pkgs; [
              # rust
              rustc
              cargo

              # lint
              clippy
              cargo-audit
              nixd
              nil

              # format
              rustfmt
              nixfmt
              oxfmt
              treefmt

              # util
              bumper
            ];
          };

          bump = pkgs.mkShell {
            packages = with pkgs; [
              bumper
            ];
          };

          check = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
            ];
          };

          release = pkgs.mkShell {
            packages = with pkgs; [
              flake-release
            ];
          };

          update = pkgs.mkShell {
            packages = with pkgs; [
              renovate
              cargo # rust
            ];
          };

          vulnerable = pkgs.mkShell {
            packages = with pkgs; [
              flake-checker # nix
              zizmor # actions
              cargo-audit # rust
            ];
          };
        };

        # nix run [#...]
        apps = pkgs.mkApps {
          default = "cargo run";
          test = "cargo test";
        };

        # nix build [#...]
        packages = {
          default = pkgs.rustPlatform.buildRustPackage (
            final: with pkgs.lib; {
              pname = "fix-hash";
              version = "0.2.0";

              src = fileset.toSource {
                root = ./.;
                fileset = fileset.unions [
                  ./Cargo.lock
                  ./Cargo.toml
                  ./LICENSE
                  ./main.rs
                  ./README.md
                ];
              };
              cargoLock.lockFile = ./Cargo.lock;

              nativeCheckInputs = with pkgs; [
                rustfmt
                clippy
              ];
              checkPhase = ''
                cargo fmt --check
                cargo test --offline
                cargo clippy --offline -- -D warnings
              '';

              meta = {
                mainProgram = "fix-hash";
                description = "Nix hash fixer";
                license = licenses.mit;
                platforms = platforms.all;
                homepage = "https://github.com/spotdemo4/nix-fix-hash";
                changelog = "https://github.com/spotdemo4/nix-fix-hash/releases/tag/v${final.version}";
                downloadPage = "https://github.com/spotdemo4/nix-fix-hash/releases/tag/v${final.version}";
              };
            }
          );
        };

        # nix build #images.[...]
        images = {
          default = pkgs.mkImage {
            fromImage = pkgs.image.nix;
            src = self.packages.${system}.default;
            contents = with pkgs; [ dockerTools.caCertificates ];
            enableFakechroot = true;
            fakeRootCommands = ''
              echo "[safe]" >> /etc/gitconfig
              echo "    directory = *" >> /etc/gitconfig
            '';
          };
        };

        # nix fmt
        formatter = pkgs.treefmt.withConfig {
          configFile = ./treefmt.toml;
          runtimeInputs = with pkgs; [
            rustfmt
            nixfmt
            oxfmt
          ];
        };

        # nix flake check
        checks = pkgs.mkChecks {
          rust = self.packages.${system}.default.overrideAttrs {
            dontBuild = true;
            installPhase = ''
              touch $out
            '';
          };

          nix = {
            root = ./.;
            filter = file: file.hasExt "nix";
            packages = with pkgs; [
              nixfmt
            ];
            script = ''
              nixfmt --check "$file"
            '';
          };

          config = {
            root = ./.;
            filter = file: file.hasExt "json" || file.hasExt "yaml" || file.hasExt "toml" || file.hasExt "md";
            packages = with pkgs; [
              oxfmt
            ];
            script = ''
              oxfmt --check
            '';
          };

          actions = {
            root = ./.github/workflows;
            filter = file: file.hasExt "yaml";
            packages = with pkgs; [
              action-validator
              zizmor
            ];
            script = ''
              action-validator "$file"
              zizmor --offline "$file"
            '';
          };

          renovate = {
            root = ./.github;
            fileset = ./.github/renovate.json;
            packages = with pkgs; [
              renovate
            ];
            script = ''
              renovate-config-validator renovate.json
            '';
          };
        };
      }
    );
}
