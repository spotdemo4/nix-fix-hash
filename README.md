# Nix Hash Fixer

[![check](https://img.shields.io/github/actions/workflow/status/spotdemo4/fix-hash/check.yaml?branch=main&logo=github&logoColor=%23bac2de&label=check&labelColor=%23313244)](https://github.com/spotdemo4/fix-hash/actions/workflows/check.yaml/)
[![vulnerable](https://img.shields.io/github/actions/workflow/status/spotdemo4/fix-hash/vulnerable.yaml?branch=main&logo=github&logoColor=%23bac2de&label=vulnerable&labelColor=%23313244)](https://github.com/spotdemo4/fix-hash/actions/workflows/vulnerable.yaml)
[![rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fspotdemo4%2Ffix-hash%2Frefs%2Fheads%2Fmain%2FCargo.toml&query=%24.package.rust-version&logo=rust&logoColor=%23bac2de&label=version&labelColor=%23313244&color=%23D34516)](https://releases.rs/)
[![flakehub](https://img.shields.io/endpoint?url=https://flakehub.com/f/spotdemo4/fix-hash/badge&labelColor=%23313244)](https://flakehub.com/flake/spotdemo4/fix-hash)

script that automatically fixes incorrect nix hashes

## Use

Run `nix build` and fix all incorrect hashes encountered:

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

#### [nix-fix-hash-0.1.1.tar.xz](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.1.1/nix-fix-hash-0.1.1.tar.xz) - bundle with all dependencies

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
  uses: spotdemo4/nix-fix-hash@v0.1.1
  with:
    arguments: .#package
```

### Docker

```elm
docker run --rm -v "$(pwd):/app" -w /app ghcr.io/spotdemo4/nix-fix-hash:0.1.1
```
