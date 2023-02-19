#!/usr/bin/env bash
anchor build --program-name marginfi
RUST_LOG=error cargo test-sbf --features=test -- --test-threads=1