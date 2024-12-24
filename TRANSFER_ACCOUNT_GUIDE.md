## TRANSFER ACCOUNT GUIDE

A user got their wallet pwned and you need to move them to a new mrgn account? Read on.

Before you begin:

- You must be an admin of the group where the user account is based. If this is a mrgn-owned group,
  that's probably the multisig (AZtUUe9GvTFq9kfseu9jxTioSgdSfjgmZfGQBmhVpTj1). If this is arena,
  that's the pool owner/creator.
- If you have admin, consider freezing the account with `SetAccountFlag` (passing `DISABLED_FLAG`)
  to prevent the attacker from doing anything.

You will need:

- Contact with the affected user (they must sign to conclude the process)
- Access to the mrgn CLI (clients/rust/marginfi-cli)
- Rust 1.75.0 (as of December 2024)
- Access to either the front end (at /migrate/account) or a TS scripting environment where you can run a simple TS script. If using the latter, we'll assume the affected wallet is located at `/keys/affected_wallet.json`
- (Optional) A wallet with sol at a location you know. We'll assume it's at
  `/keys/some_wallet.json`. You'll need this for most CLI interactions but not in this particular
  use case.

Steps:

- Open terminal at `clients/rust/marginfi-cli`
- Create a profile:

```
cargo run profile create \
--name mainnet-group-ms \
--cluster mainnet \
--rpc-url https://api.mainnet-beta.solana.com \
--group 4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8 \
--program-id MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA \
--multisig AZtUUe9GvTFq9kfseu9jxTioSgdSfjgmZfGQBmhVpTj1 \
--keypair-path ~/keys/some_wallet.json \
--fee-payer ~/keys/some_wallet.json
```

Feel free to use a custom rpc instead if you have one. Omit the last two lines if you don't have a local wallet with SOL.

- `cargo run profile set mainnet-group-ms` to use this profile, `cargo run profile show` to confirm your settings.
- Run this ix to generate a tx to set the migration flag:
```
cargo run account set-flag THE_COMPROMISED_ACCOUNT \
  --account-migration-enabled
```
* confirm the profile name when prompted to proceed
* Copy the tx contents (everything between the --------) from the CLI output. Open Squads and go to
  Developers > TX Builder> Import base28 encoded tx. Paste what you copied.
* Type a description for the tx and hit Initiate Transaction. Wait for votes and execute.

Steps for User:
* After admin completes the above, navigate to the migration page (or script) to sign the finalized
  migration ix.
