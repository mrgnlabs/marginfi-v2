# marginfi v2 client

## Install the cli

1. Install the Rust toolchain that is specified in the [workspace rust-toolchain.toml](../../../rust-toolchain.toml) and set it to default:
   * Apple_x64: `rustup toolchain install 1.75.0-x86_64-apple-darwin; rustup default 1.75.0-x86_64-apple-darwin`
   * Apple_ARM64: `rustup toolchain install 1.75.0-aarch64-apple-darwin; rustup default 1.75.0-aarch64-apple-darwin`
1. Confirm that the Cargo.lock is unchanged. That is the temporary work around for the [missing `solana_rbpf = "=0.8.0"` issue](https://github.com/mrgnlabs/marginfi-v2/issues/262)
1. Navigate to the cli folder
1. Make sure to build with the `--locked --force` and `--all-features` flags:
   * Apple_x64: `cargo install --path . --target x86_64-apple-darwin --locked --force --all-features`
   * Apple_ARM64: `cargo install --path . --target aarch64-apple-darwin --locked --force --all-features`

## Usage

```
~> mfi
marginfi-v2-cli 0.1.0

USAGE:
    mfi [OPTIONS] <SUBCOMMAND>

OPTIONS:
        --compute-unit-price <COMPUTE_UNIT_PRICE>
        --dry-run                                    Dry run for any transactions involved
    -h, --help                                       Print help information
    -V, --version                                    Print version information
    -y, --skip-confirmation

SUBCOMMANDS:
    account
    bank
    find-pyth-pull
    group
    help                             Print this message or the help of the given subcommand(s)
    inspect-padding
    inspect-pyth-push-oracle-feed
    inspect-size
    inspect-swb-pull-feed
    lip
    make-test-i80f48
    patch-idl
    profile
    show-oracle-ages
```
