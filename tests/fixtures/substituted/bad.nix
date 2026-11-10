{
  system ? builtins.currentSystem,
  pkgs ? import <nixpkgs> { inherit system; },
}:
pkgs.runCommand "fix-hash-substituted"
  {
    outputHash = "sha256-y6BrVzb69n5UsHtWHq6UOV53TFF6fZEKVDaeEmPM+9Q=";
    outputHashAlgo = "sha256";
    outputHashMode = "flat";
  }
  ''
    printf new > "$out"
  ''
