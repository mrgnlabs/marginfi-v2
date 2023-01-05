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

// const SCALE: u128 = 10_u128.pow(14);

// pub fn ui_to_native(value: f64) -> u128 {
//     let integer_part = value.floor();
//     let fractional_part = value - integer_part;

//     let integer_part_u128 = (integer_part as u128) * SCALE;
//     let fractional_part_u128 = (fractional_part * SCALE as f64) as u128;

//     integer_part_u128 + fractional_part_u128
// }

// pub fn native_to_ui(value: u128) -> f64 {
//     let integer_part = value.checked_div_euclid(SCALE).unwrap() as f64;
//     let fractional_part = (value.checked_rem_euclid(SCALE).unwrap() as f64) / (SCALE as f64);

//     integer_part + fractional_part
// }

// pub fn ui_to_native_u64(value: f64) -> u64 {
//     (value * 1_000_000f64) as u64
// }
