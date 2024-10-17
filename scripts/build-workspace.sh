#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cmd="anchor build --no-idl -- --features ignore-fee-deploy"
echo "Running: $cmd"
eval "$cmd"
