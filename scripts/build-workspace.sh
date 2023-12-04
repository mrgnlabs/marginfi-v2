#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cluster=$1

if [ -z "$cluster" ]; then
    echo "Usage: $0 <cluster>"
    exit 1
fi

if [ "$cluster" = "mainnet" ]; then
    cluster_feature="mainnet-beta"
elif [ "$cluster" = "devnet" ]; then
    cluster_feature=" devnet"
else
    echo "Error: Unknown cluster: $cluster"
    exit 1
fi

cmd="anchor build -- --features $cluster_feature"
echo "Running: $cmd"
eval "$cmd"
