#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

VERIFY_BIN=$(which solana-verify)
if [ "$?" != "0" ]; then
    echo "solana-verify not found. Please run: cargo install solana-verify."
    exit 1
fi

program_lib_name=$1
cluster=$2

if [ -z "$program_lib_name" ] || [ -z "$cluster" ]; then
    echo "Usage: $0 <program_lib_name> <cluster>"
    exit 1
fi

if [ "$cluster" = "mainnet" ]; then
    cluster_feature="mainnet-beta"
    program_id="MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA"
    rpc_moniker="main"
elif [ "$cluster" = "devnet" ]; then
    cluster_feature=" devnet"
    program_id="LipsxuAkFkwa4RKNzn51wAsW7Dedzt1RNHMkTkDEZUW"
    rpc_moniker="dev"
else
    echo "Error: Unknown cluster: $cluster"
    exit 1
fi

cmd="sudo $VERIFY_BIN verify-from-repo --url $rpc_moniker --program-id $program_id https://github.com/mrgnlabs/marginfi-v2 --library-name $program_lib_name -- --features $cluster_feature"
echo "Running: $cmd"
eval "$cmd"
