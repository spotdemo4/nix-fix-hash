{
  system ? builtins.currentSystem,
  pkgs ? import <nixpkgs> { inherit system; },
}:
pkgs.runCommand "fix-hash-substituted"
  {
    outputHash = "sha256-EVB6Di9eadXfpApiob17buV+a82FxnybhDGzb/8hxDc=";
    outputHashAlgo = "sha256";
    outputHashMode = "flat";
  }
  ''
    printf new > "$out"
  ''
