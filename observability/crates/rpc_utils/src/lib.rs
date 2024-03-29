pub mod conversion;

use std::{collections::HashMap, iter::zip, time::Duration};

use backoff::{future::retry, ExponentialBackoffBuilder};
use futures::future::try_join_all;
use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use solana_sdk::{account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey};

pub struct AccountWithSlot {
    pub evaluation_slot: u64,
    pub account: Account,
}

pub async fn get_multiple_accounts_chunked(
    rpc_client: &RpcClient,
    keys: &[Pubkey],
) -> Result<HashMap<Pubkey, AccountWithSlot>, ClientError> {
    let zips: Result<Vec<_>, ClientError> =
        try_join_all(keys.chunks(100).map(|pubkey_chunk| async move {
            let accounts_with_slot = retry(
                ExponentialBackoffBuilder::new()
                    .with_max_interval(Duration::from_secs(5))
                    .build(),
                || async {
                    let response = rpc_client
                        .get_multiple_accounts_with_commitment(
                            pubkey_chunk,
                            CommitmentConfig {
                                commitment:
                                    solana_sdk::commitment_config::CommitmentLevel::Confirmed,
                            },
                        )
                        .await?;
                    Ok(response
                        .value
                        .iter()
                        .map(|maybe_account| (response.context.slot, maybe_account.clone()))
                        .collect::<Vec<_>>())
                },
            )
            .await?;
            Ok(zip(pubkey_chunk, accounts_with_slot))
        }))
        .await;

    Ok(HashMap::from_iter(zips?.into_iter().flatten().filter_map(
        |(key, (slot, maybe_account))| {
            maybe_account.map(|account| {
                (
                    *key,
                    AccountWithSlot {
                        evaluation_slot: slot,
                        account,
                    },
                )
            })
        },
    )))
}
