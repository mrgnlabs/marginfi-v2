# marginfi v2 client

## Install the cli

1. Install the latest stable toolchain and set it to default:

```
rustup default stable-x86_64-apple-darwin

$ rustc --version
rustc 1.71.1
```

2. Navigate to the cli folder

3. Make sure to build with the `--all-features` flag and target `x86_64`:

```
cargo install --path . --target x86_64-apple-darwin --all-features
```

## Usage

```
$ mfi
marginfi-v2-cli 0.1.0

USAGE:
    mfi [OPTIONS] <SUBCOMMAND>

OPTIONS:
        --dry-run              Dry run for any transactions involved
    -h, --help                 Print help information
    -V, --version              Print version information
    -y, --skip-confirmation

SUBCOMMANDS:
    account
    bank
    group
    help                        Print this message or the help of the given subcommand(s)
    inspect-padding
    inspect-size
    inspect-switchboard-feed
    lip
    profile
```
