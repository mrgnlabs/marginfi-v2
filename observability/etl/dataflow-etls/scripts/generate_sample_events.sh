#!/usr/bin/env bash
set -e

PROGRAM_ID=5Lt5xXZG7bteZferQk9bsiiAS75JqGVPYcTbB8J6vvJK

random_id=$(echo $RANDOM | md5sum | head -c 20; echo)
test_profile="test-devnet-$random_id"

RPC_ENDPOINT=https://devnet.rpcpool.com/
KEYPAIR_PATH=~/.config/solana/id.json

echo "-> Creating test profile $test_profile"
mfi profile create \
    --cluster devnet \
    --name "$test_profile" \
    --keypair-path "$KEYPAIR_PATH" \
    --rpc-url "$RPC_ENDPOINT" \
    --program-id "$PROGRAM_ID"

mfi profile set "$test_profile"

echo "-> Admin creates marginfi group"
mfi group create -y

echo "-> Admin updates marginfi group"
mfi group update -y

echo "-> Admin creates USDC bank"
usdc_bank=$(mfi group add-bank \
    --mint F9jRT1xL7PCRepBuey5cQG5vWHFSbnvdWxJWKqtzMDsd \
    --asset-weight-init 0.85 \
    --asset-weight-maint 0.9 \
    --liability-weight-maint 1.1 \
    --liability-weight-init 1.15 \
    --deposit-limit 1000000000000000\
    --borrow-limit 1000000000000000\
    --pyth-oracle 5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7 \
    --optimal-utilization-rate 0.9 \
    --plateau-interest-rate 1 \
    --max-interest-rate 10 \
    --insurance-fee-fixed-apr 0.01 \
    --insurance-ir-fee 0.1 \
    --protocol-fixed-fee-apr 0.01 \
    --protocol-ir-fee 0.1 \
    --risk-tier collateral \
     -y)
echo "USDC bank: $usdc_bank"

echo "-> Admin creates SOL bank"
sol_bank=$(mfi group add-bank \
    --mint 4Bn9Wn1sgaD5KfMRZjxwKFcrUy6NKdyqLPtzddazYc4x \
    --asset-weight-init 0.75 \
    --asset-weight-maint 0.8 \
    --liability-weight-maint 1.2 \
    --liability-weight-init 1.25 \
    --deposit-limit 1000000000000000\
    --borrow-limit 1000000000000000\
    --pyth-oracle J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
    --optimal-utilization-rate 0.8 \
    --plateau-interest-rate 1 \
    --max-interest-rate 20 \
    --insurance-fee-fixed-apr 0.01 \
    --insurance-ir-fee 0.1 \
    --protocol-fixed-fee-apr 0.01 \
    --protocol-ir-fee 0.1 \
    --risk-tier collateral \
     -y)
echo "SOL bank: $sol_bank"

echo "-> Admin configures SOL bank"
mfi bank update "$sol_bank" \
    --asset-weight-init 1 \
    --asset-weight-maint 1 \
    -y

echo "-> Admin configures USDC bank"
mfi bank update "$usdc_bank" \
    --asset-weight-init 1 \
    --asset-weight-maint 1 \
    -y

mfi account create -y

echo "-> Random user lends USDC"
mfi account deposit "$usdc_bank" 0.01 -y

echo "-> Liquidatee creates new mfi account"
liquidatee_account=$(mfi account create -y)
echo "liquidatee account: $liquidatee_account"

echo "-> Liquidatee deposits SOL"
mfi account deposit "$sol_bank" 0.01 -y

echo "-> Liquidatee borrows USDC"
mfi account borrow "$usdc_bank" 0.01 -y

echo "-> Admin triggers bad health by setting SOL asset weights to 0"
mfi bank update "$sol_bank" \
    --asset-weight-init 0 \
    --asset-weight-maint 0 \
    -y

echo "-> Liquidator creates mfi account"
mfi account create -y

echo "-> Liquidator deposits USDC to pay off liquidatee's debt"
mfi account deposit "$usdc_bank" 0.01 -y

echo "-> Liquidator liquidates liquidatee for half its assets"
mfi account liquidate \
    --liquidatee-marginfi-account="$liquidatee_account" \
    --asset-bank="$sol_bank" \
    --liability-bank="$usdc_bank" \
    --ui-asset-amount=0.0001 \
    -y

echo "-> Admin handles remainder of bad debt through handle bankruptcy"
mfi bank handle-bankruptcy \
    --bank="$usdc_bank" \
    --marginfi-account="$liquidatee_account" \
    -y

echo "-> Admin collects fees on USDC bank"
mfi bank collect-fees \
    --bank="$usdc_bank" \
    -y

echo "-> Admin collects fees on SOL bank"
mfi bank collect-fees \
    --bank="$sol_bank" \
    -y
