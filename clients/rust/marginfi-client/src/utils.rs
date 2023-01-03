use anyhow::{bail, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Signature, transaction::Transaction};

pub fn process_transaction(
    tx: &Transaction,
    rpc_client: &RpcClient,
    dry_run: bool,
) -> Result<Signature> {
    if dry_run {
        match rpc_client.simulate_transaction(tx) {
            Ok(response) => {
                println!("------- program logs -------");
                response
                    .value
                    .logs
                    .unwrap()
                    .into_iter()
                    .for_each(|line| println!("{}", line));
                println!("----------------------------");
                Ok(Signature::default())
            }
            Err(err) => bail!(err),
        }
    } else {
        match rpc_client.send_and_confirm_transaction_with_spinner(tx) {
            Ok(sig) => Ok(sig),
            Err(err) => bail!(err),
        }
    }
}
