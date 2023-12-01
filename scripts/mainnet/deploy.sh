#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

keypair=$1
if [ -z "$keypair" ]; then
    echo "Usage: $0 <keypair>"
    exit 1
fi

solana program write-buffer "$ROOT/target/deploy/marginfi.so" \
    -k $keypair
