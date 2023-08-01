#!/usr/bin/env sh

VERIFY_BIN=$(which solana-verify)

echo "sudo $VERIFY_BIN verify-from-repo -um --program-id MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA https://github.com/mrgnlabs/marginfi-v2 --library-name marginfi -- --features mainnet-beta"

sudo $VERIFY_BIN verify-from-repo -um --program-id MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA https://github.com/mrgnlabs/marginfi-v2 --library-name marginfi -- --features mainnet-beta
