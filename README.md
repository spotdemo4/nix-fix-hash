# Nix Hash Fixer

[![check](https://trev.zip/llc/nix-fix-hash/actions/workflows/check.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=check&labelColor=%23313244)](https://trev.zip/llc/nix-fix-hash/actions?workflow=check.yaml)
[![vulnerable](https://trev.zip/llc/nix-fix-hash/actions/workflows/vulnerable.yaml/badge.svg?branch=main&logo=forgejo&logoColor=%23bac2de&label=vulnerable&labelColor=%23313244)](https://trev.zip/llc/nix-fix-hash/actions?workflow=vulnerable.yaml)
[![nixpkgs](https://shields.trev.zip/endpoint?url=https%3A%2F%2Fnix-shield.trev.zip%2Fbadge%3Furl%3Dhttps%253A%252F%252Ftrev.zip%252Fllc%252Fnix-fix-hash%252Fraw%252Fbranch%252Fmain%252Fflake.lock%26input%3Dnixpkgs&logoColor=%23bac2de&labelColor=%23313244&color=%235277C3)](https://nixos.org/)
[![rust](https://shields.trev.zip/badge/dynamic/toml?url=https://trev.zip/llc/nix-fix-hash/raw/branch/main/Cargo.toml&query=%24.package.rust-version&logo=rust&logoColor=%23bac2de&label=version&labelColor=%23313244&color=%23D34516)](https://releases.rs/)

Automatically fixes incorrect Nix [fixed-output derivation](https://nix.dev/manual/nix/2.34/glossary#gloss-fixed-output-derivation) (FOD) hashes.

Dependent hashes are rediscovered and fixed in successive rounds, so cases such as a source hash followed by `cargoHash` are handled in one invocation.

## Use

```elm
nix run github:spotdemo4/nix-fix-hash
```

Fix a specific flake output:

```elm
nix run github:spotdemo4/nix-fix-hash -- .#output
```

Fix a specific file:

```elm
nix run github:spotdemo4/nix-fix-hash -- --file package.nix
```

After all hash updates converge, check the result with one final `nix build`:

```elm
nix run github:spotdemo4/nix-fix-hash -- --check .#output
```

## Install

### Download

https://trev.zip/llc/nix-fix-hash/releases

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
  uses: spotdemo4/nix-fix-hash@v0.5.3
  with:
    arguments: .#package
```

### Docker

```elm
docker run -v "$(pwd):/app" -w /app ghcr.io/spotdemo4/nix-fix-hash:0.5.3
```
