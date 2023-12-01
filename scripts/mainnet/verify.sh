#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

VERIFY_BIN=$(which solana-verify)
if [ "$?" != "0" ]; then
    echo "solana-verify not found. Please run: cargo install solana-verify."
    exit 1
fi

cmd="sudo $VERIFY_BIN verify-from-repo -um --program-id MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA https://github.com/mrgnlabs/marginfi-v2 --library-name marginfi -- --features mainnet-beta"
echo "Running: $cmd"
eval "$cmd"
