#!/usr/bin/env sh

ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

set -e

ask_confirmation() {
    while true; do
        read -p "Are you sure you want to proceed? (y/n): " yn
        case $yn in
            [Yy]* ) return 0;;
            [Nn]* ) return 1;;
            * ) echo "Please answer yes (y) or no (n).";;
        esac
    done
}

deployer_key_path=$1
[ -z "$deployer_key_path" ] && echo "Missing deployer_key_path argument" && exit 1
[ ! -f "$deployer_key_path" ] && echo "$deployer_key_path is not a file" && exit 1

program_address_or_keypair=$2
[ -z "$program_address_or_keypair" ] && echo "Missing program_address_or_keypair argument" && exit 1

deployer_pk=$(solana-keygen pubkey $deployer_key_path)
deployer_balance=$(solana balance $deployer_key_path)
set +e
exist_result=$(solana account $program_address_or_keypair 2>&1)
set -e

if [ "$exist_result" = *"Error: AccountNotFound:"* ]; then
    is_upgrade=0
else
    is_upgrade=1
fi

if [ -f "$program_address_or_keypair" ]; then
    program_id=$(solana-keygen pubkey $program_address_or_keypair)
else
    if [ "$is_upgrade" = "0" ]; then
      echo "You need to provide a private key path for a first deploy."
      exit 1
    else
      program_id=$program_address_or_keypair
    fi
fi

echo "========================================================================================="
echo "Deployer: $deployer_pk"
echo "Balance: $deployer_balance"
printf "Deploying to: $program_id"
if [ "$is_upgrade" = "0" ]; then
  echo " (first deployment)"
else
  echo " (already deployed)"
fi

if ! ask_confirmation; then
    echo "Cancelled."
    exit 0
fi

if [ "$is_upgrade" = "0" ]; then
  echo "Deploying..."
else
  echo "Upgrading..."
fi

solana program deploy \
 --use-rpc \
 --url $url \
 --fee-payer $deployer_key_path \
 --keypair $deployer_key_path \
 --program-id $program_address_or_keypair \
 "$ROOT/target/deploy/marginfi.so"
