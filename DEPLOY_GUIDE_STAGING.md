## DEPLOYING TO STAGING

Staging is different from mainnet only in that it uses a different key. Ensure that the key in lib.rs matches the intended staging key (typically stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct)

### Requirements

* You have access to the staging program authority wallet (we will assume it is at `~/keys/staging-deploy.json` from here on)
* Anchor 0.30.1
* Solana 1.18.17

## Steps

### With Anchor

* Note: this rarely works, the program is probably too chonky.
* Build with `anchor build -p marginfi -- --no-default-features --features staging`
* If this is your first time deploying (to a new key), with `anchor build -p marginfi -- --no-default-features --features staging`
* Ensure anchor.toml is configured like this: 
```
[provider]
cluster = "https://api.mainnet-beta.solana.com"
wallet = "~/keys/staging-deploy.json"
```
Adjust the cluster as needed if using a custom rpc.
* Deploy with `anchor upgrade target/deploy/marginfi.so --program-id stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct`. Use deploy instead upgrade if this is your first deployment, and use the keypair.json in the target folder instead of the program's id for program-id.

### If Anchor is busted (for any number of reasons)

* Run:
```
solana program deploy --use-rpc \
  target/deploy/marginfi.so \
  --program-id stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct \
  --keypair ~/keys/staging-deploy.json \
  --fee-payer ~/keys/staging-deploy.json \
  --url <your rpc>
```
If this is your first time deploying, use the keypair.json in the target folder instead of the program's id for program-id 
* Failed? That happens often. `solana program close --buffers -k ~/keys/staging-deploy.json` to recover the buffer funds and try again (Note: this costs you .02 SOL to try again)
* Still failing? That happens. Try to recover the buffer instead of closing it: `solana-keygen recover -o recovered-buffer.json` (then enter the buffer seed phrase). Then:
```
solana program deploy --use-rpc \
  target/deploy/marginfi.so \
  --program-id stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct \
  --keypair ~/keys/staging-deploy.json \
  --fee-payer ~/keys/staging-deploy.json \
  --url <your rpc (https://api.mainnet-beta.solana.com is usually fine)> \
  --buffer recovered-buffer.json
```
* Program buffer full? Use:
```
solana program extend \
  --url <RPC_URL> \
  --keypair ~/keys/staging-deploy.json \
  stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct 10000
```
* If you changed your wallet config, make sure to remove the staging wallet from your Solana config to avoid sausage fingers errors in the future: `solana config set --keypair ~/.config/solana/id.json`