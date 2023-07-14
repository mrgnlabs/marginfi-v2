#!/usr/bin/env sh

solana program write-buffer $(pwd)/target/deploy/marginfi.so \
    -k $1
