#!/usr/bin/env bash

# Add USDC bank
cargo run  --features devnet group add-bank \
    --mint F9jRT1xL7PCRepBuey5cQG5vWHFSbnvdWxJWKqtzMDsd \
    --asset-weight-init 0.85 \
    --asset-weight-maint 0.9 \
    --liability-weight-maint 1.1 \
    --liability-weight-init 1.15 \
    --max-capacity 1000000000000000 \
    --pyth-oracle 5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7 \
    --optimal-utilization-rate 0.9 \
    --plateau-interest-rate 1 \
    --max-interest-rate 10 \
    --insurance-fee-fixed-apr 0.01 \
    --insurance-ir-fee 0.1 \
    --protocol-fixed-fee-apr 0.01 \
    --protocol-ir-fee 0.1 \
    --dry-run

# Add SOL bank
cargo run  --features devnet group add-bank \
    --mint 4Bn9Wn1sgaD5KfMRZjxwKFcrUy6NKdyqLPtzddazYc4x \
    --asset-weight-init 0.75 \
    --asset-weight-maint 0.8 \
    --liability-weight-maint 1.2 \
    --liability-weight-init 1.25 \
    --max-capacity 1000000000000000 \
    --pyth-oracle J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
    --optimal-utilization-rate 0.8 \
    --plateau-interest-rate 1 \
    --max-interest-rate 20 \
    --insurance-fee-fixed-apr 0.01 \
    --insurance-ir-fee 0.1 \
    --protocol-fixed-fee-apr 0.01 \
    --protocol-ir-fee 0.1 \
    --dry-run
