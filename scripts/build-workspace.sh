#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cmd="anchor build --no-idl -- --features"
echo "Running: $cmd"
eval "$cmd"
