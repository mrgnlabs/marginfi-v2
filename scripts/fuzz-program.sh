#!/usr/bin/env bash
set -e

ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

cd $ROOT/programs/marginfi/fuzz
cmd="cargo +nightly fuzz run lend -Zbuild-std --strip-dead-code --no-cfg-fuzzing -- -max_total_time=300 -verbosity=0"
echo "Running: $cmd"
eval "$cmd"
