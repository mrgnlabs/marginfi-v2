#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cmd="anchor build --no-idl"
echo "Running: $cmd"
eval "$cmd"
