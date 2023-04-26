#!/usr/bin/env sh

solana program deploy $(pwd)/target/deploy/marginfi.so \
    -u $RPC_URL \
    --program-id $PROGRAM_ID \
    -k $1
