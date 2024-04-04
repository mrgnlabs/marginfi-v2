use backoff::{future::retry, ExponentialBackoffBuilder};
use concurrent_queue::ConcurrentQueue;
use crossbeam::channel::Sender;
use futures::{future::try_join_all, lock::Mutex, stream, StreamExt};
use rpc_utils::conversion::convert_encoded_ui_transaction;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_client::{GetConfirmedSignaturesForAddress2Config, SerializableTransaction},
    rpc_config::{RpcBlockConfig, RpcTransactionConfig},
    rpc_request::RpcError,
};
use solana_rpc_client_api::client_error::{Error as ClientError, ErrorKind};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    message::SimpleAddressLoader,
    pubkey::Pubkey,
    signature::Signature,
    transaction::{MessageHash, SanitizedTransaction},
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::interval;
use tracing::{info, warn};

use crate::error::IndexingError;

pub const MARGINFI_PROGRAM_GENESIS_SIG: &str =
    "36ViByXbDDgXHo3fTghrrT9zHMu9mYCJMhpKYTpytHcj7Nxkpb6gQdBJRxtDbF53mebNc8HR4aC7pcKmRNypxTWC";
const SLOT_RANGE_SIZE: u64 = 5000; // ~40 minutes

#[derive(Debug)]
pub struct TransactionData {
    pub task_id: u64,
    pub signature: Signature,
    pub transaction: EncodedConfirmedTransactionWithStatusMeta,
}

pub async fn find_boundary_signatures_for_range(
    program_id: &Pubkey,
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
            let boundary_sig =
                find_boundary_signature(program_id, rpc_client, slot, forward, None).await?;
            Ok((slot, boundary_sig))
        },
    ))
    .await?;

    Ok(boundary_sigs)
}

pub async fn find_boundary_signature(
    program_id: &Pubkey,
    rpc_client: &RpcClient,
    slot: u64,
    forward: bool,
    max_retries: Option<u64>,
) -> Result<Signature, IndexingError> {
    let max_retries = max_retries.unwrap_or(10);

    let mut i = 0;
    let mut candidate_slot = slot;

    'block_search: loop {
        if i > 0 {
            i += 1;
            if forward {
                candidate_slot += i;
            } else {
                candidate_slot -= i;
            }

            if i > max_retries {
                return Err(IndexingError::BoundarySignatureNotFound(slot, max_retries));
            }
        }

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
                let Some(mut boundary_txs) = block.transactions else {
                    warn!("No transactions in block for slot {}", candidate_slot);
                    continue;
                };

                if boundary_txs.is_empty() {
                    warn!("No transactions in block for slot {}", candidate_slot);
                    continue;
                }

                // Look for a non-mfi transaction in the block, as the boundary
                let boundary_tx = loop {
                    if forward {
                        boundary_txs.reverse();
                    }
                    let mut boundary_txs_iter = boundary_txs.iter();

                    let Some(boundary_tx) = boundary_txs_iter.next() else {
                        warn!("ONLY MFI TXS IN BLOCK FOR SLOT {}???!!!", candidate_slot);
                        continue 'block_search;
                    };

                    let versioned_tx_with_meta =
                        convert_encoded_ui_transaction(boundary_tx.clone()).unwrap();
                    let sanitized_tx = SanitizedTransaction::try_create(
                        versioned_tx_with_meta.transaction,
                        MessageHash::Precomputed(Hash::default()),
                        None,
                        SimpleAddressLoader::Enabled(versioned_tx_with_meta.meta.loaded_addresses),
                        true,
                    )
                    .unwrap();

                    if !sanitized_tx
                        .message()
                        .account_keys()
                        .iter()
                        .any(|key| key == program_id)
                    {
                        break boundary_tx;
                    }
                };

                let boundary_sig = boundary_tx
                    .transaction
                    .decode()
                    .unwrap()
                    .get_signature()
                    .clone();

                return Ok(boundary_sig);
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
    }
}

pub async fn crawl_signatures_for_range(
    task_id: u64,
    rpc_client: &RpcClient,
    address: Pubkey,
    before: Signature,
    until: Signature,
    transaction_tx: &Sender<TransactionData>,
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

#[derive(Debug)]
pub struct Range {
    pub before_sig: Signature,
    pub until_sig: Signature,
    pub before_slot: u64,
    pub until_slot: u64,
    pub progress: f64,
}

pub async fn generate_ranges(
    program_id: &Pubkey,
    rpc_client: RpcClient,
    overall_before_sig: Signature,
    overall_until_sig: Signature,
    range_queue: &ConcurrentQueue<Range>,
    is_range_complete: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sig_statuses = rpc_client
        .get_signature_statuses_with_history(&[overall_until_sig, overall_before_sig])
        .await?
        .value;
    let (overall_until_slot, overall_before_slot) = (
        sig_statuses[0].as_ref().unwrap().slot,
        sig_statuses[1].as_ref().unwrap().slot,
    );

    info!(
        "Starting to generate ranges between {:?} ({:?}) and {:?} ({:?})",
        overall_until_sig, overall_until_slot, overall_before_sig, overall_before_slot
    );

    let mut cursor_slot = overall_until_slot;
    let prev_cursor_sig = Arc::new(Mutex::new(overall_until_sig));

    let mut timer = interval(Duration::from_millis(200));
    loop {
        if range_queue.is_full() {
            timer.tick().await;
            continue;
        }

        let remaining_capacity = range_queue.capacity().unwrap() - range_queue.len();

        let mut boundary_slots = vec![cursor_slot];
        for _ in 0..remaining_capacity {
            cursor_slot = (cursor_slot + SLOT_RANGE_SIZE).min(overall_before_slot);
            boundary_slots.push(cursor_slot);

            if cursor_slot == overall_before_slot {
                break;
            }
        }

        let rpc_client_ref = &rpc_client;
        let prev_cursor_slot = Arc::new(AtomicU64::new(boundary_slots.remove(0)));

        stream::iter(
            boundary_slots
                .iter()
                .map(|slot| (*slot, prev_cursor_sig.clone(), prev_cursor_slot.clone())),
        )
        .map(|(slot, prev_cursor_sig, prev_cursor_slot)| async move {
            if slot == overall_before_slot {
                (slot, overall_before_sig, prev_cursor_sig, prev_cursor_slot)
            } else {
                retry(
                    ExponentialBackoffBuilder::new()
                        .with_max_interval(Duration::from_secs(5))
                        .build(),
                    || async {
                        match find_boundary_signature(program_id, rpc_client_ref, slot, true, None)
                            .await
                        {
                            Ok(sig) => {
                                Ok((slot, sig, prev_cursor_sig.clone(), prev_cursor_slot.clone()))
                            }
                            Err(e) => Err(backoff::Error::transient(e)),
                        }
                    },
                )
                .await
                .unwrap()
            }
        })
        .buffered(10)
        .for_each(
            |(cursor_slot, cursor_sig, prev_cursor_sig, prev_cursor_slot)| async move {
                let mut until_sig = prev_cursor_sig.lock().await;

                range_queue
                    .push(Range {
                        before_sig: cursor_sig,
                        before_slot: cursor_slot,
                        until_sig: until_sig.clone(),
                        until_slot: prev_cursor_slot.load(Ordering::Relaxed),
                        progress: ((cursor_slot as f64 - overall_until_slot as f64)
                            / (overall_before_slot as f64 - overall_until_slot as f64))
                            * 100.0,
                    })
                    .unwrap();

                *until_sig = cursor_sig; // ok because we are doing an ordered buffering
                prev_cursor_slot.store(cursor_slot, Ordering::Relaxed);
            },
        )
        .await;

        if cursor_slot == overall_before_slot {
            break;
        }

        timer.tick().await;
    }

    is_range_complete.store(true, Ordering::Relaxed);

    Ok(())
}

pub async fn get_default_before_signature(rpc_client: &RpcClient) -> String {
    let latest_slot = rpc_client
        .get_slot()
        .await
        .expect("Failed to fetch latest slot");
    let latest_block_slots = rpc_client
        .get_blocks(latest_slot - 100, None)
        .await
        .expect("Failed to fetch block ids");
    if latest_block_slots.is_empty() {
        panic!("Failed to find blocks in the last 100 slots");
    }

    let latest_block = rpc_client
        .get_block_with_config(
            *latest_block_slots.last().unwrap(),
            RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                max_supported_transaction_version: Some(0),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to fetch latest block");
    let before_sig = latest_block
        .transactions
        .unwrap()
        .last()
        .unwrap()
        .transaction
        .decode()
        .unwrap()
        .get_signature()
        .clone();

    before_sig.to_string()
}
