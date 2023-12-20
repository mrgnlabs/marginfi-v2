#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cmd="anchor build"

echo "Running: $cmd"
eval "$cmd"
