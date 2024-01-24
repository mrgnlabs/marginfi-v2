#!/usr/bin/env bash
set -e

ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

program_lib_name=$1

if [ -z "$program_lib_name" ]; then
    echo "Usage: $0 <program_lib_name>"
    exit 1
fi

program_dir=${program_lib_name//_/-}  # Substitute dashes with underscores

cd $ROOT/programs/$program_dir

# cmd="RUST_LOG=error cargo test-sbf --features=test -- --test-threads=1"
cmd="RUST_LOG=error cargo test-sbf --features=test"
echo "Running: $cmd"
eval "$cmd"
