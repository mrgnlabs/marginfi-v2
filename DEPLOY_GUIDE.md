## MAINNET VERIFIED DEPLOY GUIDE

Marginfi program authority is managed by squads (https://v3.squads.so/dashboard/M05MQ1FNRDdmUTdCQjc2aTY0aGpMRUNYTEFFNHpmeFJ2UTdlYVREVEo2elo=) and uses verified builds.


First you will need:
* Solana tools 1.18.20 or later (`solana-install init 1.18.20`)
* solana-verify (`cargo install solana-verify`)
* Docker (https://docs.docker.com/engine/install/ubuntu/)
* A wallet with at least 10 SOL (this guide will assume your wallet is at `~/keys/mainnet-deploy.json`). Verify the pubkey of your wallet with `solana-keygen pubkey ~/keys/mainnet-deploy.json` and verify you have at least 10 SOL with `solana balance -k ~/keys/mainnet-deploy.json`
* An RPC provider connected to mainnet (`solana config set --url https://api.mainnet-beta.solana.com`). The solana public api is usually fine.


Steps:
* Make sure you are on the main branch and you have pulled latest.
* Run `./scripts/build-program-verifiable.sh marginfi mainnet`. Other people signing on the multisig should also run this and validate that the hash matches. 
* Deploy the buffer with `./scripts/deploy-buffer.sh marginfi <YOUR_RPC_ENDPOINT> ~/keys/mainnet-deploy.json`
* Go to squads, developers, programs, pick marginfi. The buffer address is the output of the previous command. The buffer refund is the public key of the wallet you have used so far (`solana-keygen pubkey ~/keys/mainnet-deploy.json` if you don't know it). Click next.
* Go back to your cli and paste the command Squads gave you in step 2. If this key is not the one used in your solana CLI, make sure it pass it with -k, e.g.:
```
solana program set-buffer-authority <BUFFER> --new-buffer-authority <MULTISIG> -k ~/keys/mainnet-deploy.json
```
* Back up the current working program somewhere with `solana -um program dump MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA mfi_backup.so`
* Click the pending upgrade to start a vote.
* Execute after the vote passes.

## DEPLOYING STAKED COLLATERAL TO STAGING

The Staked Collateral feature uses spl-single-pool, developed by the Solana Foundation (https://github.com/solana-labs/solana-program-library/tree/master/single-pool). This guide will show you how to deploy that program.

First you will need: 
* Agave tools 2.1.0 or later (`sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"`) and possibly `agave-install init 2.1.0`
* A wallet with at least 2 SOL (this guide will assume your wallet is at `~/keys/staging-deploy.json`). Verify the pubkey of your wallet with `solana-keygen pubkey ~/keys/staging-deploy.json` and verify you have at least 2 SOL with `solana balance -k ~/keys/staging-deploy.json`
* An RPC provider connected to mainnet (`solana config set --url https://api.mainnet-beta.solana.com`). The solana public api is usually fine.

Steps:
* Clone https://github.com/solana-labs/solana-program-library/tree/master/single-pool and pull latest
* Navigate to programs/single-pool and run `cargo build-sbf`
* Navigate back up to root, then navigate to target. Verify that `solana-keygen pubkey deploy/spl_single_pool-keypair.json` matches the program's declared id. If you want to generate a new id, delete this file and build again to generate a new program keypair. Don't forget to update the declare_id in lib.rs as needed.
* Deploy the program with:
```
solana program deploy \                                                  
  deploy/spl_single_pool.so \
  --program-id deploy/spl_single_pool-keypair.json \
  --keypair ~/keys/staging-deploy.json \
  --fee-payer ~/keys/staging-deploy.json \
  --url <your_rpc_url (optional, omit this line to use api.mainnet-beta)>

```
