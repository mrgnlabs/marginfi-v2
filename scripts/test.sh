#!/usr/bin/env bash
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

program_lib_name=$1

if [ -z "$program_lib_name" ]; then
    echo "Usage: $0 <program_lib_name>"
    exit 1
fi

cd $ROOT/programs/$program_lib_name

# cmd="RUST_LOG=error cargo test-sbf --features=test -- --test-threads=1"
cmd="RUST_LOG=error cargo test-sbf --features=test"
echo "Running: $cmd"
eval "$cmd"
