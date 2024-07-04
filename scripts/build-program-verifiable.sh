#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

verify_bin=$(which solana-verify)
if [ "$?" != "0" ]; then
    echo "solana-verify not found. Please run: cargo install solana-verify."
    exit 1
fi

program_lib_name=$1
deployment=$2

if [ -z "$program_lib_name" ] || [ -z "$deployment" ]; then
    echo "Usage: $0 <program_lib_name> <deployment>"
    exit 1
fi

if [ "$deployment" = "mainnet" ]; then
    features="--features mainnet-beta"
elif [ "$deployment" = "devnet" ]; then
    features="--features devnet"
elif [ "$deployment" = "staging" ]; then
    features="--features staging"
else
    echo "Error: Unknown deployment: $deployment"
    exit 1
fi

cmd="sudo $verify_bin build --library-name $program_lib_name -- $features"
echo "Running: $cmd"
eval "$cmd"
