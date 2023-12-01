#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

program_lib_name=$1
cluster=$2
keypair=$3

if [ -z "$keypair" ] || [ -z "$program_lib_name" ] || [ -z "$cluster" ]; then
    ecbo "Usage: $0 <program_lib_name> <cluster> <keypair>"
    exit 1
fi

if [ "$cluster" = "mainnet" ]; then
    url_moniker="https://api.mainnet-beta.solana.com"
elif [ "$cluster" = "devnet" ]; then
    url_moniker="https://api.devnet.solana.com"
else
    echo "Error: Unknown cluster: $cluster"
    exit 1
fi

cmd="solana --url $url_moniker program write-buffer "$ROOT/target/deploy/$program_lib_name.so" -k $keypair"
echo "Running: $cmd"
eval "$cmd"
