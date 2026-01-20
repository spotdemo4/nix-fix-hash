{
  system ? builtins.currentSystem,
  pkgs ? import <nixpkgs> { inherit system; },
}:
let
  tests = {
    url = pkgs.fetchurl {
      url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/25.05.tar.gz";
      hash = "sha256-BADCV7PVO/v+ptYft21HUaOYmkoJooYV/3dRaoKzkk0=";
    };

    github = pkgs.fetchFromGitHub {
      owner = "NixOS";
      repo = "nixpkgs";
      tag = "25.05";
      hash = "sha256-BADXrcIzU5wm/C8F9LWvUfBGu5U5E7cFzPYT1pHIJaQ=";
    };

    image =
      (pkgs.dockerTools.pullImage {
        imageName = "nixos/nix";
        imageDigest = "sha256:0d9c872db1ca2f3eaa4a095baa57ed9b72c09d53a0905a4428813f61f0ea98db";
        hash = "sha256-BADT+XPp5xadUzP2GEq031yZSIfzpZ1Ps6KVeBTIhOg=";
      }).overrideAttrs
        {
          __structuredAttrs = true;
          unsafeDiscardReferences.out = true;
        };
  };
in
pkgs.linkFarm "tests" (
  map (i: {
    name = i.name;
    path = i.value;
  }) (pkgs.lib.attrsToList tests)
)
