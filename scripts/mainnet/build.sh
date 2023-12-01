#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

anchor build -p marginfi -- --features mainnet-beta
