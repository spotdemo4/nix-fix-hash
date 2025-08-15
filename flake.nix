{
  description = "Trevstack Web Client";

  nixConfig = {
    extra-substituters = [
      "https://trevnur.cachix.org"
    ];
    extra-trusted-public-keys = [
      "trevnur.cachix.org-1:hBd15IdszwT52aOxdKs5vNTbq36emvEeGqpb25Bkq6o="
    ];
  };

  inputs = {
    systems.url = "systems";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
    trev = {
      url = "github:spotdemo4/nur";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    utils,
    trev,
    ...
  }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
      trevpkgs = trev.packages."${system}";
    in rec {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # lint
          shellcheck
          prettier
          alejandra

          # actions
          flake-checker
          trevpkgs.renovate
        ];
        shellHook = trevpkgs.shellhook.ref;
      };

      packages.default = pkgs.writeShellApplication {
        name = "nix-fix-hash";

        runtimeInputs = with pkgs; [
          nix
          ncurses
        ];

        text = builtins.readFile ./nix-fix-hash.sh;

        meta = {
          description = "Nix hash fixer";
          mainProgram = "nix-fix-hash";
          homepage = "https://github.com/spotdemo4/nix-fix-hash";
          platforms = pkgs.lib.platforms.all;
        };
      };

      checks = {
        lint = pkgs.stdenvNoCC.mkDerivation {
          name = "lint";
          src = ./.;
          dontConfigure = true;
          dontBuild = true;
          dontFixup = true;

          nativeBuildInputs = with pkgs; [
            shellcheck
            prettier
            alejandra
          ];

          doCheck = true;
          checkPhase = ''
            export HOME=$(mktemp -d)
            shellcheck ./nix-fix-hash.sh
            prettier --check .
            alejandra -c .
          '';

          installPhase = ''
            touch $out
          '';
        };
        build = packages.default;
        shell = devShells.default;
      };

      formatter = pkgs.alejandra;
    });
}
