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
    nocapture="--test-threads=1"
else
    loglevel=debug
    nocapture="--nocapture"
fi

if [ "$program_lib_name" == "all" ]; then
    package_filter=""
else 
    package_filter="--package $program_lib_name"
fi

extra_params="${@:3}"

cmd="SBF_OUT_DIR=$ROOT/target/deploy RUST_LOG=solana_runtime::message_processor::stable_log=$loglevel cargo nextest run --no-fail-fast $package_filter --features=test,test-bpf $nocapture -- $extra_params"
echo "Running: $cmd"
eval "$cmd"
