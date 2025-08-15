# Nix Hash Fixer

[![check](https://img.shields.io/github/actions/workflow/status/spotdemo4/nix-fix-hash/check.yaml?logo=GitHub&logoColor=%23cdd6f4&label=check&labelColor=%2311111b)](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/check.yaml)
[![flake](https://img.shields.io/github/actions/workflow/status/spotdemo4/nix-fix-hash/flake.yaml?logo=nixos&logoColor=%2389dceb&label=flake&labelColor=%2311111b)](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/flake.yaml)

shell script that automatically fixes incorrect package hashes in nix flakes

## Install

- just copy and paste the [shell script](/nix-fix-hash.sh)
- download & run it immediately:

```console
foo@bar:~$ nix run github:spotdemo4/nix-fix-hash

build successful, no hash mismatch found
```

- get it from a flake input:

```nix
inputs = {
    # ...
    fixhash = {
        url = "github:spotdemo4/nix-fix-hash";
        inputs.nixpkgs.follows = "nixpkgs";
    };
};

outputs = { fixhash, ... }: {
    # ...
    devShells."${system}".default = pkgs.mkShell {
        packages = with pkgs; [
            fixhash."${system}".default
        ];
    };
}
```

also available from the [nur](https://github.com/nix-community/NUR) as `repos.trev.nix-fix-hash`
