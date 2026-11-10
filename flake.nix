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
    trevpkgs = {
      url = "github:spotdemo4/trevpkgs";
      inputs.systems.follows = "systems";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      trevpkgs,
      ...
    }:
    trevpkgs.libs.mkFlake (
      system: pkgs: {

        # nix develop [#...]
        devShells = {
          default = pkgs.mkShell {
            shellHook = pkgs.shellhook.ref;
            env.RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            packages = with pkgs; [
              # rust
              rustc
              gcc # linker
              cargo
              rust-analyzer
              rustfmt
              clippy
              cargo-audit

              vscode-json-languageserver # json
              yaml-language-server # yaml
              tombi # toml
              oxfmt # format

              # nix
              nixd
              nixfmt

              # util
              treefmt
              bumper
              fix-hash
            ];
          };

          bump = pkgs.mkShell {
            packages = with pkgs; [
              bumper
            ];
          };

          # integration tests, which need nix and network access
          check = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              gcc # linker
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
              cargo-audit # rust
              flake-checker # nix
              zizmor # actions
            ];
          };
        };

        # nix build [#...]
        packages = {
          default = pkgs.mkRustPackage (
            final: with pkgs.lib; {
              pname = "fix-hash";
              version = "0.5.4";

              src = fileset.toSource {
                root = ./.;
                fileset = fileset.unions [
                  ./Cargo.lock
                  ./Cargo.toml
                  ./clippy.toml
                  ./LICENSE
                  ./README.md
                  ./src
                  ./tests
                ];
              };
              cargoLock.lockFile = ./Cargo.lock;

              doCheck = true;
              checkPhase = ''
                runHook preCheck
                cargo test --offline
                runHook postCheck
              '';

              # also check the dependencies for clippy, the lints run in the rust-lint flake check
              cargoArtifactsArgs = {
                nativeCheckInputs = with pkgs; [ clippy ];
                doCheck = true;
                checkPhase = ''
                  runHook preCheck
                  cargo test --offline
                  cargo clippy --offline --all-targets
                  runHook postCheck
                '';
              };

              meta = {
                mainProgram = "fix-hash";
                description = "Nix hash fixer";
                license = licenses.mit;
                platforms = platforms.all;
                homepage = "https://trev.zip/llc/nix-fix-hash";
                downloadPage = "https://trev.zip/llc/nix-fix-hash/releases";
                changelog = "https://trev.zip/llc/nix-fix-hash/releases/tag/v${final.version}";
                sourceProvenance = [ sourceTypes.fromSource ];
                maintainers = [ maintainers.spotdemo4 ];
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
          inherit (self.packages.${system}) default;

          rust-lint = {
            src = self.packages.${system}.default;
            packages = with pkgs; [
              rustfmt
              clippy
            ];
            script = ''
              cargo fmt --check
              cargo clippy --offline --all-targets -- -D warnings
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

          actions-gh = {
            root = ./.;
            files = [
              ./action.yaml
              ./.github/workflows
            ];
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

          actions-fj = {
            root = ./.forgejo/workflows;
            filter = file: file.hasExt "yaml";
            packages = with pkgs; [
              forgejo-runner
              zizmor
            ];
            script = ''
              forgejo-runner validate --workflow --path "$file"
              zizmor --offline "$file"
            '';
          };

          renovate-fj = {
            root = ./.forgejo;
            files = ./.forgejo/renovate.jsonc;
            packages = with pkgs; [
              renovate
            ];
            script = ''
              renovate-config-validator renovate.jsonc
            '';
          };

          config = {
            root = ./.;
            filter =
              file:
              file.hasExt "json"
              || file.hasExt "jsonc"
              || file.hasExt "yaml"
              || file.hasExt "toml"
              || file.hasExt "md";
            packages = with pkgs; [
              oxfmt
            ];
            script = ''
              oxfmt --check
            '';
          };
        };
      }
    );
}
