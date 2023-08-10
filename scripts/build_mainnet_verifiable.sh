#!/usr/bin/env sh

VERIFY_BIN=$(which solana-verify)
PROGRAM_LIB_NAME="marginfi"

echo "sudo $VERIFY_BIN build --library-name $PROGRAM_LIB_NAME -- --features mainnet-beta"

sudo $VERIFY_BIN build --library-name $PROGRAM_LIB_NAME -- --features mainnet-beta
