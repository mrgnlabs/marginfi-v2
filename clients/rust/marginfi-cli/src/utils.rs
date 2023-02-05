use std::collections::HashMap;

use anyhow::{bail, Result};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use marginfi::{
    bank_authority_seed, bank_seed,
    constants::MAX_ORACLE_KEYS,
    state::{
        marginfi_account::MarginfiAccount,
        marginfi_group::{Bank, BankVaultType},
    },
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::AccountMeta, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};

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

pub fn find_bank_vault_pda(
    bank_pk: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_seed!(vault_type, bank_pk), program_id)
}

pub fn find_bank_vault_authority_pda(
    bank_pk: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_authority_seed!(vault_type, bank_pk), program_id)
}

pub fn create_oracle_key_array(oracle_key: Pubkey) -> [Pubkey; MAX_ORACLE_KEYS] {
    let mut oracle_keys = [Pubkey::default(); MAX_ORACLE_KEYS];
    oracle_keys[0] = oracle_key;
    oracle_keys
}

pub const EXP_10_I80F48: [I80F48; 15] = [
    I80F48!(1),
    I80F48!(10),
    I80F48!(100),
    I80F48!(1_000),
    I80F48!(10_000),
    I80F48!(100_000),
    I80F48!(1_000_000),
    I80F48!(10_000_000),
    I80F48!(100_000_000),
    I80F48!(1_000_000_000),
    I80F48!(10_000_000_000),
    I80F48!(100_000_000_000),
    I80F48!(1_000_000_000_000),
    I80F48!(10_000_000_000_000),
    I80F48!(100_000_000_000_000),
];

pub fn load_observation_account_metas(
    marginfi_account: &MarginfiAccount,
    banks_map: &HashMap<Pubkey, Bank>,
    include_banks: Vec<Pubkey>,
    exclude_banks: Vec<Pubkey>,
) -> Vec<AccountMeta> {
    let mut bank_pks = marginfi_account
        .lending_account
        .balances
        .iter()
        .filter_map(|balance| balance.active.then_some(balance.bank_pk))
        .collect::<Vec<_>>();

    for bank_pk in include_banks {
        if !bank_pks.contains(&bank_pk) {
            bank_pks.push(bank_pk);
        }
    }

    bank_pks.retain(|bank_pk| !exclude_banks.contains(bank_pk));

    let mut banks = vec![];
    for bank_pk in bank_pks.clone() {
        let bank = banks_map.get(&bank_pk).unwrap();
        banks.push(bank);
    }

    let account_metas = banks
        .iter()
        .zip(bank_pks.iter())
        .flat_map(|(bank, bank_pk)| {
            vec![
                AccountMeta {
                    pubkey: *bank_pk,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: bank.config.get_pyth_oracle_key(),
                    is_signer: false,
                    is_writable: false,
                },
            ]
        })
        .collect::<Vec<_>>();
    account_metas
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
