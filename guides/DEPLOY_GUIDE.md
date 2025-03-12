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

### Voters:

* Clone the branch being deployed (see the release tag the person who initated the upgrade has given you) and run: 
```
./scripts/build-program-verifiable.sh marginfi mainnet
```
* Check that the program builds with the hash that the person who is deploying gave you. Check what characters other people have validated in Signal, post the next six characters of the hash to verify you have actually checked and aren't skipping this step out of laziness.
* Check that the buffer contains this hash too `solana-verify get-buffer-hash <Buffer Address>`.
* After the vote is executed and the contract is upgraded, check that the contract contains the same hash. For example for MFv2, this is `solana-verify get-program-hash MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA`

## RECENT DEPLOY HASHES

Here we list recent deployments to staging/mainnet. The hash is always the first 6 chars of the hash generated with the mainnet verified build guide above (even for staging, this is the mainnet hash, not the hash on staging. Staging does not get a verified build.).

### STAGING

* 0.1.0: Jan 30, 2025 ~2:35pm ET -- Hash: a4dd3e7
* 0.1.1: Feb 7, 2025 ~8:15am ET -- Hash: 03455c
* 0.1.2: Pending

### MAINNET

* 0.1.0-alpha mainnet on Fev 3, 2024 ~2:45ET -- Hash: ea5d15
* 0.1.1: Feb 17, 2025 ~3:00pm ET -- Hash: 03455c
