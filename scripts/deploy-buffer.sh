#!/usr/bin/env sh
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

program_lib_name=$1
rpc_url=$2
keypair=$3

if [ -z "$program_lib_name" ] || [ -z "$rpc_url" ] || [ -z "$keypair" ]; then
    echo "Usage: $0 <program_lib_name> <rpc_url> <keypair>"
    exit 1
fi

cmd="solana --url $rpc_url program write-buffer --use-rpc "$ROOT/target/deploy/$program_lib_name.so" -k $keypair"
echo "Running: $cmd"
eval "$cmd"
