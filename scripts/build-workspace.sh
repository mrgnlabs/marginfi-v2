#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cluster=$1

if [ -z "$cluster" ]; then
    echo "Using no cluster feature"
    cmd="anchor build"

elif [ "$cluster" = "mainnet" ]; then
    cmd="anchor build -- --features mainnet-beta"
elif [ "$cluster" = "devnet" ]; then
    cmd="anchor build -- --features devnet"
else
    echo "Error: Unknown cluster: $cluster"
    exit 1
fi

echo "Running: $cmd"
eval "$cmd"
