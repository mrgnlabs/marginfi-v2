#!/usr/bin/env bash
set -e

ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

program_lib_name=$1
loglevel=$2

if [ -z "$program_lib_name" ]; then
    echo "Usage: $0 <program_lib_name>"
    exit 1
fi

if [ "$loglevel" == "--sane" ]; then
    loglevel=warn
    nocapture=""
else
    loglevel=debug
    nocapture="--nocapture"
fi

cmd="RUST_LOG=solana_runtime::message_processor::stable_log=$loglevel cargo test --package $program_lib_name --features=test,test-bpf -- $nocapture"
echo "Running: $cmd"
eval "$cmd"
