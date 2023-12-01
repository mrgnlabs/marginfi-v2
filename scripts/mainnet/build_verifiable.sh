#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

VERIFY_BIN=$(which solana-verify)
if [ "$?" != "0" ]; then
    echo "solana-verify not found. Please run: cargo install solana-verify."
    exit 1
fi

PROGRAM_LIB_NAME="marginfi"

cmd="sudo $VERIFY_BIN build --library-name $PROGRAM_LIB_NAME -- --features mainnet-beta"
echo "Running: $cmd"
eval "$cmd"
