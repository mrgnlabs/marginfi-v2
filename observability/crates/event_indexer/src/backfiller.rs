use backoff::{future::retry, ExponentialBackoffBuilder};
use crossbeam::channel::Sender;
use futures::{future::try_join_all, stream, StreamExt};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_client::{GetConfirmedSignaturesForAddress2Config, SerializableTransaction},
    rpc_config::{RpcBlockConfig, RpcTransactionConfig},
    rpc_request::RpcError,
};
use solana_rpc_client_api::client_error::{Error as ClientError, ErrorKind};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::{str::FromStr, time::Duration};
use tracing::{info, warn};

use crate::error::IndexingError;

pub const MARGINFI_PROGRAM_GENESIS_SIG: &str =
    "36ViByXbDDgXHo3fTghrrT9zHMu9mYCJMhpKYTpytHcj7Nxkpb6gQdBJRxtDbF53mebNc8HR4aC7pcKmRNypxTWC";

#[derive(Debug)]
pub struct TransactionData {
    pub task_id: u64,
    pub signature: Signature,
    pub transaction: EncodedConfirmedTransactionWithStatusMeta,
}

pub async fn find_boundary_signatures_for_range(
    rpc_client: &RpcClient,
    task_count: usize,
    before: Signature,
    until: Signature,
) -> Result<Vec<(u64, Signature)>, IndexingError> {
    let before_slot = rpc_client
        .get_transaction_with_config(
            &before,
            RpcTransactionConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| IndexingError::FailedToFindTransactionSlot(before))?
        .slot;

    let until_slot = rpc_client
        .get_transaction_with_config(
            &until,
            RpcTransactionConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| IndexingError::FailedToFindTransactionSlot(until))?
        .slot;

    let mut boundary_slots = vec![];
    let chunk_length = (before_slot - until_slot) / task_count as u64;
    for i in 0..task_count.into() {
        let boundary_slot = until_slot + chunk_length * i as u64;
        boundary_slots.push(boundary_slot);
    }
    boundary_slots.push(before_slot);

    let last_boundary_index = boundary_slots.len() - 1;
    let boundary_sigs = try_join_all(boundary_slots.into_iter().enumerate().map(
        |(index, slot)| async move {
            let forward = index != last_boundary_index;
            let boundary_slot = find_boundary_signature(rpc_client, slot, forward, None).await?;
            Ok((slot, boundary_slot))
        },
    ))
    .await?;

    Ok(boundary_sigs)
}

pub async fn find_boundary_signature(
    rpc_client: &RpcClient,
    slot: u64,
    forward: bool,
    max_retries: Option<u64>,
) -> Result<Signature, IndexingError> {
    let max_retries = max_retries.unwrap_or(10);

    let mut i = 0;
    let mut candidate_slot = slot;

    loop {
        let result = rpc_client
            .get_block_with_config(
                candidate_slot,
                RpcBlockConfig {
                    encoding: Some(UiTransactionEncoding::Base64),
                    max_supported_transaction_version: Some(0),
                    ..Default::default()
                },
            )
            .await;

        match result {
            Ok(block) => {
                if let Some(boundary_txs) = block.transactions {
                    let boundary_tx = if forward {
                        boundary_txs.last().unwrap()
                    } else {
                        boundary_txs.first().unwrap()
                    };
                    let boundary_sig = boundary_tx
                        .transaction
                        .decode()
                        .unwrap()
                        .get_signature()
                        .clone();

                    return Ok(boundary_sig);
                } else {
                    warn!("No transactions in block for slot {}", candidate_slot);
                }
            }
            Err(error) => {
                match error {
                    ClientError {
                        kind:
                            ErrorKind::RpcError(RpcError::RpcResponseError {
                                code,
                                message,
                                data,
                            }),
                        ..
                    } => {
                        if code != -32009 {
                            warn!(
                            "Error fetching block for slot {}: (code: {}, message: {}, data: {:?}), retrying...",
                            candidate_slot, code, message, data
                        );
                            continue;
                        } else {
                            info!(
                                "Block for slot {} not available yet, trying next...",
                                candidate_slot
                            )
                        }
                    }
                    _ => {
                        warn!(
                            "Error fetching block for slot {}: {:?}, retrying...",
                            candidate_slot, error
                        );
                        continue;
                    }
                };
            }
        }

        i += 1;
        if forward {
            candidate_slot += i;
        } else {
            candidate_slot -= i;
        }

        if i > max_retries {
            return Err(IndexingError::BoundarySignatureNotFound(slot, max_retries));
        }
        continue;
    }
}

pub async fn crawl_signatures_for_range(
    task_id: u64,
    rpc_client: RpcClient,
    address: Pubkey,
    before: Signature,
    until: Signature,
    transaction_tx: Sender<TransactionData>,
    max_concurrent_requests: Option<usize>,
) -> Result<(), IndexingError> {
    let mut last_fetched_signature = before;
    let max_concurrent_requests = max_concurrent_requests.unwrap_or(10);

    loop {
        let signatures = retry(
            ExponentialBackoffBuilder::new()
                .with_max_interval(Duration::from_secs(5))
                .build(),
            || async {
                match rpc_client
                    .get_signatures_for_address_with_config(
                        &address,
                        GetConfirmedSignaturesForAddress2Config {
                            before: Some(last_fetched_signature),
                            until: Some(until),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    Ok(signatures) => Ok(signatures),
                    Err(e) => Err(backoff::Error::transient(e)),
                }
            },
        )
        .await
        .map_err(|_| IndexingError::FailedToFetchSignatures(until, last_fetched_signature))?;

        if signatures.is_empty() {
            break;
        }

        last_fetched_signature =
            Signature::from_str(&signatures.last().unwrap().signature).unwrap();

        let successful_signatures = signatures
            .into_iter()
            .filter_map(|sig_data| match sig_data.err {
                Some(_) => None,
                None => Some(Signature::from_str(&sig_data.signature).unwrap()),
            })
            .collect::<Vec<_>>();

        if successful_signatures.is_empty() {
            continue;
        }

        stream::iter(successful_signatures)
            .map(|signature| {
                let rpc_client = &rpc_client;
                let transaction_tx_clone = transaction_tx.clone();
                async move {
                    (
                        signature,
                        transaction_tx_clone,
                        retry(
                            ExponentialBackoffBuilder::new()
                                .with_max_interval(Duration::from_secs(5))
                                .build(),
                            || async {
                                match rpc_client
                                    .get_transaction_with_config(
                                        &signature,
                                        RpcTransactionConfig {
                                            max_supported_transaction_version: Some(0),
                                            encoding: Some(UiTransactionEncoding::Base64),
                                            ..Default::default()
                                        },
                                    )
                                    .await
                                {
                                    Ok(transaction) => Ok(transaction),
                                    Err(e) => Err(backoff::Error::transient(e)),
                                }
                            },
                        )
                        .await
                        .unwrap(),
                    )
                }
            })
            .buffered(max_concurrent_requests)
            .for_each(
                |(signature, transaction_tx_clone, transaction)| async move {
                    transaction_tx_clone
                        .send(TransactionData {
                            task_id,
                            signature: signature.clone(),
                            transaction,
                        })
                        .unwrap();
                },
            )
            .await;
    }

    Ok(())
}
