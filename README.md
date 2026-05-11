# Nix Hash Fixer

[![check](https://img.shields.io/github/actions/workflow/status/spotdemo4/nix-fix-hash/check.yaml?branch=main&logo=github&logoColor=%23bac2de&label=check&labelColor=%23313244)](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/check.yaml/)
[![vulnerable](https://img.shields.io/github/actions/workflow/status/spotdemo4/nix-fix-hash/vulnerable.yaml?branch=main&logo=github&logoColor=%23bac2de&label=vulnerable&labelColor=%23313244)](https://github.com/spotdemo4/nix-fix-hash/actions/workflows/vulnerable.yaml)
[![rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fspotdemo4%2Fnix-fix-hash%2Frefs%2Fheads%2Fmain%2FCargo.toml&query=%24.package.rust-version&logo=rust&logoColor=%23bac2de&label=version&labelColor=%23313244&color=%23D34516)](https://releases.rs/)
[![flakehub](https://img.shields.io/endpoint?url=https://flakehub.com/f/spotdemo4/nix-fix-hash/badge&labelColor=%23313244)](https://flakehub.com/flake/spotdemo4/nix-fix-hash)

Automatically fixes incorrect Nix [fixed-output derivation](https://nix.dev/manual/nix/2.34/glossary#gloss-fixed-output-derivation) (FOD) hashes

## Use

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

### Download

| OS    | Architecture | Download                                                                                                                      |
| ----- | ------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Linux | amd64        | [fix-hash_0.3.0_linux_amd64](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.3.0/fix-hash_0.3.0_linux_amd64)   |
| Linux | arm64        | [fix-hash_0.3.0_linux_arm64](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.3.0/fix-hash_0.3.0_linux_arm64)   |
| Linux | arm          | [fix-hash_0.3.0_linux_arm](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.3.0/fix-hash_0.3.0_linux_arm)       |
| MacOS | amd64        | [fix-hash_0.3.0_darwin_amd64](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.3.0/fix-hash_0.3.0_darwin_amd64) |
| MacOS | arm64        | [fix-hash_0.3.0_darwin_arm64](https://github.com/spotdemo4/nix-fix-hash/releases/download/v0.3.0/fix-hash_0.3.0_darwin_arm64) |

### Nix

```nix
inputs = {
    fix-hash = {
        url = "github:spotdemo4/nix-fix-hash";
        inputs.nixpkgs.follows = "nixpkgs";
    };
};

outputs = { fix-hash, ... }: {
    devShells.x86_64-linux.default = pkgs.mkShell {
        packages = [ fix-hash.packages."${system}".default ];
    };
}
```

```elm
fix-hash .#output
```

also available from the [nur](https://nur.nix-community.org/repos/trev/) as `repos.trev.nix-fix-hash`

### Action

```yaml
- name: fix nix hashes
  uses: spotdemo4/nix-fix-hash@v0.3.0
  with:
    arguments: .#package
```

### Docker

```elm
docker run -v "$(pwd):/app" -w /app ghcr.io/spotdemo4/nix-fix-hash:0.3.0
```
