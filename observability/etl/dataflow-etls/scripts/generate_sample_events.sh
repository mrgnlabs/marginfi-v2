#!/usr/bin/env bash
set -e

GROUP=567YJkNDCgG6Q6qDwcNK7p8frSL1HRE5J2tqUEXCXR7v
PROGRAM_ID=A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4
NEW_PROFILE_NAME=indexing

RPC_ENDPOINT=https://devnet.genesysgo.net/
KEYPAIR_PATH=~/.config/solana/id.json

[ -z "$PROGRAM_ID" ] && echo "PROGRAM_ID must be specified. Exiting." && exit 1
[ -z "$NEW_PROFILE_NAME" ] && echo "NEW_PROFILE_NAME must be specified. Exiting." && exit 1

#mfi profile create \
#    --cluster devnet \
#    --name "$NEW_PROFILE_NAME" \
#    --keypair-path "$KEYPAIR_PATH" \
#    --rpc-url "$RPC_ENDPOINT" \
#    --program-id "$PROGRAM_ID"
#
#mfi profile set "$NEW_PROFILE_NAME"
#
#mfi group create "$@" -y

# Add USDC bank
mfi group add-bank \
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
    -y \
    "$@"

## Add SOL bank
#mfi group add-bank \
#    --mint 4Bn9Wn1sgaD5KfMRZjxwKFcrUy6NKdyqLPtzddazYc4x \
#    --asset-weight-init 0.75 \
#    --asset-weight-maint 0.8 \
#    --liability-weight-maint 1.2 \
#    --liability-weight-init 1.25 \
#    --deposit-limit 1000000000000000\
#    --borrow-limit 1000000000000000\
#    --pyth-oracle J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
#    --optimal-utilization-rate 0.8 \
#    --plateau-interest-rate 1 \
#    --max-interest-rate 20 \
#    --insurance-fee-fixed-apr 0.01 \
#    --insurance-ir-fee 0.1 \
#    --protocol-fixed-fee-apr 0.01 \
#    --protocol-ir-fee 0.1 \
#    -y \
#    "$@"

SOL_BANK=5dCnHjFUTjuWq2b9hY2J1o94QR1YpiggUC2XKzA1UpYp
USDC_BANK=7A1zpp3Fb7eQmAKy6r3jZnSHhZK9SJEBuUrUqwwMQQfY

echo "-> Admin configures SOL bank"
mfi bank update "$SOL_BANK" \
    --asset-weight-init 1 \
    --asset-weight-maint 1 \
    --skip-confirmation

echo "-> Admin configures USDC bank"
mfi bank update "$USDC_BANK" \
    --asset-weight-init 1 \
    --asset-weight-maint 1 \
    --skip-confirmation

mfi account use 6TZKuwLn8C6cVCxhGhmLtFUTZo4qGPF2a7Qb9F4QJFs1 --skip-confirmation

echo "-> Random user lends USDC"
mfi account deposit "$USDC_BANK" 0.01 --skip-confirmation

echo "-> Liquidatee creates new mfi account"
liquidatee_account=$(mfi account create --skip-confirmation)
echo "liquidatee account: $liquidatee_account"

echo "-> Liquidatee deposits SOL"
mfi account deposit "$SOL_BANK" 0.01 --skip-confirmation

echo "-> Liquidatee borrows USDC"
mfi account borrow "$USDC_BANK" 0.01 --skip-confirmation

echo "-> Admin triggers bad health by setting SOL asset weights to 0"
mfi bank update "$SOL_BANK" \
    --asset-weight-init 0 \
    --asset-weight-maint 0 \
    --skip-confirmation

echo "-> Liquidator creates mfi account"
mfi account create --skip-confirmation

echo "-> Liquidator deposits USDC to pay off liquidatee's debt"
mfi account deposit "$USDC_BANK" 0.01 --skip-confirmation

echo "-> Liquidator liquidates liquidatee for half its assets"
mfi account liquidate \
    --liquidatee-marginfi-account="$liquidatee_account" \
    --asset-bank="$SOL_BANK" \
    --liability-bank="$USDC_BANK" \
    --ui-asset-amount=0.0001 \
    --skip-confirmation

echo "-> Admin handles remainder of bad debt through handle bankruptcy"
mfi bank handle-bankruptcy \
    --bank="$USDC_BANK" \
    --marginfi-account="$liquidatee_account" \
    --skip-confirmation

echo "-> Admin collects fees on USDC bank"
mfi bank collect-fees \
    --bank="$USDC_BANK" \
    --skip-confirmation

echo "-> Admin collects fees on SOL bank"
mfi bank collect-fees \
    --bank="$SOL_BANK" \
    --skip-confirmation

echo "-> Admin creates marginfi group"
mfi group create --skip-confirmation

echo "-> Admin updates marginfi group"
mfi group update --skip-confirmation
