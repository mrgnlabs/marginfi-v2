#!/usr/bin/env bash

# Add USDC bank
cargo run  --features devnet group add-bank \
    dUSD28TejMQTx9jR468Bnv1xFsBRMJvcDpP1srf85Va \
    1 \
    1 \
    1 \
    1 \
    1000000000000000 \
    5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7 \
    0.9 \
    1 \
    10 \
    0.01 \
    0.1 \
    0.01 \
    0.1 \
    --dry-run

# Add SOL bank
cargo run  --features devnet group add-bank \
    dSoLyxP7wHDXf8xcoH4ufuTXgp1ayWSjL9PM44BbgWT \
    0.9 \
    0.9 \
    1.1 \
    1.1 \
    1000000000000000 \
    J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
    0.8 \
    1 \
    20 \
    0.01 \
    0.1 \
    0.01 \
    0.1 \
    --dry-run
