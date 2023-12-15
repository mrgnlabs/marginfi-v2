use backoff::{future::retry, ExponentialBackoffBuilder};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use futures::future::try_join_all;
use marginfi::state::{marginfi_account::MarginfiAccount, marginfi_group::Bank};
use pyth_sdk_solana::PriceFeed;
use serde::{Deserialize, Serialize};
use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use solana_sdk::{
    account::Account, instruction::AccountMeta, pubkey::Pubkey, signature::Signature,
};
use std::{collections::HashMap, iter::zip, str::FromStr, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Target {
    pub address: Pubkey,
    pub before: Option<Signature>,
    pub until: Option<Signature>,
}

// Allows to parse a JSON target with base58-encoded addresses/sigs (serde expects byte arrays)
impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let targets_raw = json::parse(s).unwrap();

        Ok(Self {
            address: Pubkey::from_str(targets_raw["address"].as_str().unwrap()).unwrap(),
            before: targets_raw["before"]
                .as_str()
                .map(|sig_str| Signature::from_str(sig_str).unwrap()),
            until: targets_raw["until"]
                .as_str()
                .map(|sig_str| Signature::from_str(sig_str).unwrap()),
        })
    }
}

pub const DEFAULT_RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_SIGNATURE_FETCH_LIMIT: usize = 1_000;
pub const DEFAULT_MAX_PENDING_SIGNATURES: usize = 10_000;
pub const DEFAULT_MONITOR_INTERVAL: u64 = 5;

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

#[inline(always)]
pub fn pyth_price_to_fixed(price_feed: &PriceFeed) -> anyhow::Result<I80F48> {
    let price = I80F48::from_num(price_feed.get_ema_price_unchecked().price);
    let exponent = price_feed.get_ema_price_unchecked().expo;
    let scaling_factor = EXP_10_I80F48[exponent.unsigned_abs() as usize];

    let price = if exponent == 0 {
        price
    } else if exponent < 0 {
        price.checked_div(scaling_factor).unwrap()
    } else {
        price.checked_mul(scaling_factor).unwrap()
    };

    Ok(price)
}

pub async fn get_multiple_accounts_chunked(
    rpc_client: &RpcClient,
    keys: &[Pubkey],
) -> Result<HashMap<Pubkey, Vec<u8>>, ClientError> {
    let zips: Result<Vec<_>, ClientError> =
        try_join_all(keys.chunks(100).map(|pubkey_chunk| async move {
            Ok(zip(
                pubkey_chunk,
                retry(
                    ExponentialBackoffBuilder::new()
                        .with_max_interval(Duration::from_secs(5))
                        .build(),
                    || async { Ok(rpc_client.get_multiple_accounts(pubkey_chunk).await?) },
                )
                .await?,
            ))
        }))
        .await;

    Ok(HashMap::from_iter(zips?.into_iter().flatten().filter_map(
        |(key, account)| account.map(|account| (*key, account.data)),
    )))
}

pub async fn get_multiple_accounts_chunked2(
    rpc_client: &RpcClient,
    keys: &[Pubkey],
) -> Result<HashMap<Pubkey, Account>, ClientError> {
    let zips: Result<Vec<_>, ClientError> =
        try_join_all(keys.chunks(100).map(|pubkey_chunk| async move {
            Ok(zip(
                pubkey_chunk,
                retry(
                    ExponentialBackoffBuilder::new()
                        .with_max_interval(Duration::from_secs(5))
                        .build(),
                    || async { Ok(rpc_client.get_multiple_accounts(pubkey_chunk).await?) },
                )
                .await?,
            ))
        }))
        .await;

    Ok(HashMap::from_iter(zips?.into_iter().flatten().filter_map(
        |(key, account)| account.map(|account| (*key, account)),
    )))
}

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
                    pubkey: bank.config.oracle_keys[0],
                    is_signer: false,
                    is_writable: false,
                },
            ]
        })
        .collect::<Vec<_>>();
    account_metas
}
