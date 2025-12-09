# Nix Hash Fixer

![check](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/check.yaml/badge.svg?branch=main)
![vulnerable](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/vulnerable.yaml/badge.svg?branch=main)

script that automatically fixes incorrect nix hashes

## Use

Run `nix build` and fix all incorrect hashes:

```elm
nix run github:spotdemo4/nix-fix-hash
```

Fix a specific flake output:

```elm
nix run github:spotdemo4/nix-fix-hash .#output
```

Fix a specific file:

```elm
nix run github:spotdemo4/nix-fix-hash --file package.nix
```

## Install

### Downloads

#### [nix-fix-hash.sh](/nix-fix-hash.sh) - bash script

#### [nix-fix-hash-0.1.0.tar.xz](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.1.0/nix-fix-hash-0.1.0.tar.xz) - bundle with all dependencies

### Nix

```nix
inputs = {
    # ...
    fix-hash = {
        url = "github:spotdemo4/nix-fix-hash";
        inputs.nixpkgs.follows = "nixpkgs";
    };
};

outputs = { fix-hash, ... }: {
    # ...
    devShells."${system}".default = pkgs.mkShell {
        packages = [
            fix-hash."${system}".default
        ];
    };
}
```

also available from the [nur](https://github.com/nix-community/NUR) as `repos.trev.nix-fix-hash`

### Action

```yaml
- name: fix nix hashes
  uses: spotdemo4/nix-fix-hash@v0.1.0
  with:
    arguments: .#package
```
