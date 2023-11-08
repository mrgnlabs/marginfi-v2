#!/usr/bin/env bash
anchor build --program-name marginfi
RUST_LOG=error cargo test-sbf --features=test -- --skip marginfi_account_liquidation_success_many_balances --test-threads=1
