{
  system ? builtins.currentSystem,
  pkgs ? import <nixpkgs> { inherit system; },
}:
let
  source =
    pkgs.runCommand "fix-hash-cascade-source"
      {
        src = ./crate;
        outputHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        outputHashAlgo = "sha256";
        outputHashMode = "recursive";
      }
      ''
        cp -r "$src" "$out"
      '';
in
pkgs.rustPlatform.buildRustPackage {
  pname = "fix-hash-cascade-fixture";
  version = "0.1.0";

  src = source;
  cargoHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA=";
}
