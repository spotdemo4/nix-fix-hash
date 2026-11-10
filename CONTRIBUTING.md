# contributing

## requirements

- [nix](https://nixos.org/)

## getting started

```sh
nix develop
```

with [direnv](https://direnv.net/):

```sh
ln -s .envrc.project .envrc
direnv allow
```

### run

```sh
nix run
```

with [cargo](https://doc.rust-lang.org/cargo/):

```sh
cargo run
```

### format

```sh
nix fmt
```

with [rustfmt](https://github.com/rust-lang/rustfmt):

```sh
rustfmt --edition 2024
```

### check

```sh
nix flake check
```

with [cargo](https://doc.rust-lang.org/cargo/):

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

### build

```sh
nix build
```

with [cargo](https://doc.rust-lang.org/cargo/):

```sh
cargo build
```

### release

with [bumper](https://trev.zip/llc/bumper):

```sh
bumper
```

releases are created automatically for [significant](https://www.conventionalcommits.org/en/v1.0.0/#summary) changes
