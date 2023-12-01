#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

verify_bin=$(which solana-verify)
if [ "$?" != "0" ]; then
    echo "solana-verify not found. Please run: cargo install solana-verify."
    exit 1
fi

program_lib_name=$1
cluster=$2

if [ -z "$program_lib_name" ] || [ -z "$cluster" ]; then
    ecbo "Usage: $0 <program_lib_name> <cluster>"
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

cmd="sudo $verify_bin build --library-name $program_lib_name -- --features $cluster_feature"
echo "Running: $cmd"
eval "$cmd"
