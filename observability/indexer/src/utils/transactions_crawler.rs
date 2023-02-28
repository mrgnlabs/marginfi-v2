use anyhow::Result;
use chrono::{Local, TimeZone};
use concurrent_queue::ConcurrentQueue;
use futures::{future::join_all, stream, Future, StreamExt};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig,
};
use solana_measure::measure::Measure;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use std::{str::FromStr, time::Duration};
use tokio::{join, runtime::Builder};
use tracing::{error, info, warn};

use crate::common::{
    Target, DEFAULT_MAX_PENDING_SIGNATURES, DEFAULT_MONITOR_INTERVAL, DEFAULT_RPC_ENDPOINT,
    DEFAULT_SIGNATURE_FETCH_LIMIT,
};

pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 10;

#[derive(Debug, Clone)]
pub struct TransactionsCrawlerConfig {
    pub rpc_endpoint: String,
    pub signature_fetch_limit: usize,
    pub max_concurrent_requests: usize,
    pub max_pending_signatures: usize,
    pub monitor_interval: u64,
    pub targets: Vec<Target>,
}

impl TransactionsCrawlerConfig {
    pub fn new(targets: Vec<Target>) -> Self {
        Self {
            rpc_endpoint: DEFAULT_RPC_ENDPOINT.to_string(),
            signature_fetch_limit: DEFAULT_SIGNATURE_FETCH_LIMIT,
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
            max_pending_signatures: DEFAULT_MAX_PENDING_SIGNATURES,
            monitor_interval: DEFAULT_MONITOR_INTERVAL,
            targets,
        }
    }
}

#[derive(Default, Clone)]
pub struct SlotMeta {
    slot: u64,
    timestamp: u64,
}

#[derive(Debug)]
pub struct TransactionData {
    pub indexing_address: Pubkey,
    pub transaction: EncodedConfirmedTransactionWithStatusMeta,
}

#[derive(Debug, Clone)]
pub struct SignatureData {
    pub indexing_address: Pubkey,
    pub signature: Signature,
}

#[derive(Clone)]
pub struct TransactionsCrawlerContext {
    pub config: Arc<TransactionsCrawlerConfig>,
    rpc_client: Arc<RpcClient>,
    signature_queue: Arc<Mutex<ConcurrentQueue<SignatureData>>>,
    pub transaction_queue: Arc<Mutex<ConcurrentQueue<TransactionData>>>,
    minimum_slot_available: Arc<Mutex<u64>>,
    oldest_slot_fetched: Arc<Mutex<SlotMeta>>,
    transaction_counter: Arc<AtomicU64>,
}

impl TransactionsCrawlerContext {
    pub fn new(config: &TransactionsCrawlerConfig) -> Self {
        Self {
            config: Arc::new(config.clone()),
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                config.rpc_endpoint.clone(),
                CommitmentConfig {
                    commitment: CommitmentLevel::Finalized,
                },
            )),
            signature_queue: Arc::new(Mutex::new(ConcurrentQueue::unbounded())),
            transaction_queue: Arc::new(Mutex::new(ConcurrentQueue::bounded(1_000))), // consumption should not let this queue grow unbounded
            minimum_slot_available: Arc::new(Mutex::new(0)),
            oldest_slot_fetched: Arc::new(Mutex::new(SlotMeta::default())),
            transaction_counter: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub struct TransactionsCrawler {
    context: TransactionsCrawlerContext,
}

impl TransactionsCrawler {
    pub fn new(targets: Vec<Target>) -> Self {
        let config = TransactionsCrawlerConfig::new(targets);
        let context = TransactionsCrawlerContext::new(&config);
        Self { context }
    }

    pub fn new_with_config(config: TransactionsCrawlerConfig) -> Self {
        let context = TransactionsCrawlerContext::new(&config);
        Self { context }
    }

    pub fn run<F, Fut>(&self, transaction_processor: &F) -> Result<()>
    where
        F: (FnOnce(Arc<TransactionsCrawlerContext>) -> Fut) + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        runtime.block_on(self.run_async(transaction_processor))
    }

    pub async fn run_async<'a, F, Fut>(&self, transaction_processor: &F) -> Result<()>
    where
        F: (FnOnce(Arc<TransactionsCrawlerContext>) -> Fut) + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let context = Arc::new(self.context.clone());

        let fetch_signatures_handle = tokio::spawn({
            let context = context.clone();
            async move { Self::crawl_signatures_for_address(context).await }
        });
        let fetch_transactions_handle = tokio::spawn({
            let context = context.clone();
            async move { Self::fetch_transactions(context).await }
        });
        let process_transactions_handle = tokio::spawn({
            let context = context.clone();
            let transaction_processor = transaction_processor.clone();
            async move { transaction_processor(context).await }
        });
        let monitor_handle = tokio::spawn({
            let context = context.clone();
            async move { Self::monitor(context).await }
        });

        join_all([
            fetch_signatures_handle,
            fetch_transactions_handle,
            process_transactions_handle,
            monitor_handle,
        ])
        .await;

        Ok(())
    }

    async fn crawl_signatures_for_address(ctx: Arc<TransactionsCrawlerContext>) {
        let mut last_fetched_signature_per_address = ctx
            .config
            .targets
            .iter()
            .map(|target| (target.address, target.before))
            .collect::<HashMap<_, Option<Signature>>>();

        loop {
            // Concurrent data fetching from RPC
            let signatures_futures = ctx
                .config
                .targets
                .iter()
                .map(|target| {
                    ctx.rpc_client.get_signatures_for_address_with_config(
                        &target.address,
                        GetConfirmedSignaturesForAddress2Config {
                            before: *last_fetched_signature_per_address
                                .get(&target.address)
                                .unwrap(),
                            until: target.until,
                            limit: Some(ctx.config.signature_fetch_limit),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();
            let signatures_futures = join_all(signatures_futures);

            let minimum_available_slot_future = ctx.rpc_client.minimum_ledger_slot();

            let (signatures_result, minimum_available_slot_result) =
                join!(signatures_futures, minimum_available_slot_future);

            // Discard failed requests (will naturally be re-fetched from same sig next iteration for dropped addresses)
            let new_signatures_per_address = signatures_result
                .iter()
                .zip(ctx.config.targets.clone())
                .filter_map(|(sig_result, target)| match sig_result {
                    Ok(signatures) => Some((target.address, signatures)),
                    Err(_) => None,
                })
                .collect::<HashMap<Pubkey, _>>();

            *ctx.minimum_slot_available.lock().unwrap() = minimum_available_slot_result.unwrap();

            // Flatten and sort signatures (relative ordering of same-block signatures cross-addresses is not guaranteed)
            let mut signatures_to_push = new_signatures_per_address
                .iter()
                .flat_map(|(indexing_address, sig_data_list)| {
                    (*sig_data_list)
                        .clone()
                        .iter()
                        .map(|sig_data| (*indexing_address, sig_data.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            signatures_to_push.sort_by(|s1, s2| s2.1.slot.cmp(&s1.1.slot));

            // Discard failed txs
            let signatures_to_push = signatures_to_push
                .into_iter()
                .filter_map(|(indexing_address, sig_data)| match sig_data.err {
                    Some(_) => None,
                    None => Some(SignatureData {
                        indexing_address,
                        signature: Signature::from_str(&sig_data.signature).unwrap(),
                    }),
                })
                .collect::<Vec<_>>();

            // Early return if no successful tx in batch
            if signatures_to_push.is_empty() {
                // Update last fetched signature per address
                new_signatures_per_address
                    .iter()
                    .for_each(|(address, signatures)| {
                        let last_sig_for_address =
                            last_fetched_signature_per_address.get_mut(address).unwrap();
                        if let Some(last_sig) = signatures.last() {
                            *last_sig_for_address =
                                Some(Signature::from_str(&last_sig.signature).unwrap());
                        }
                    });
                continue;
            }

            let mut timing = Measure::start("Q_push_lock_wait");
            let signature_queue = ctx.signature_queue.lock().unwrap();
            timing.stop();
            if timing.as_ms() > 0 {
                warn!("{}", timing);
            }

            // Bail if not enough room for additional signatures (simpler than having to re-fetch from last fitting sig...)
            let current_pending_amount = signature_queue.len();
            if (current_pending_amount + signatures_to_push.len())
                > ctx.config.max_pending_signatures
            {
                // Last fetched signature per address not updated so as to resume from same point next iteration
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Push sigs to queue
            signatures_to_push.into_iter().for_each(|sig| {
                signature_queue.push(sig).unwrap();
            });

            // Update last fetched signature per address
            new_signatures_per_address
                .iter()
                .for_each(|(address, signatures)| {
                    let last_sig_for_address =
                        last_fetched_signature_per_address.get_mut(address).unwrap();
                    if let Some(last_sig) = signatures.last() {
                        *last_sig_for_address =
                            Some(Signature::from_str(&last_sig.signature).unwrap());
                    }
                });
        }
    }

    async fn fetch_transactions(ctx: Arc<TransactionsCrawlerContext>) {
        loop {
            let mut signatures = vec![];
            {
                let mut timing = Measure::start("sig_Q_pop_lock_wait");
                let signatures_queue = ctx.signature_queue.lock().unwrap();
                timing.stop();
                if timing.as_ms() > 0 {
                    warn!("{}", timing);
                }
                while !signatures_queue.is_empty() {
                    signatures.push(signatures_queue.pop().unwrap());
                }
            }
            if signatures.is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            let responses = stream::iter(signatures)
                .map(|signature_data| {
                    let rpc_client = &ctx.rpc_client;
                    let ctx_clone = ctx.clone();
                    let signature = signature_data.signature;

                    async move {
                        (
                            signature_data,
                            ctx_clone,
                            rpc_client
                                .get_transaction_with_config(
                                    &signature,
                                    RpcTransactionConfig {
                                        max_supported_transaction_version: Some(0),
                                        encoding: Some(UiTransactionEncoding::Base64),
                                        ..Default::default()
                                    },
                                )
                                .await,
                        )
                    }
                })
                .buffered(ctx.config.max_concurrent_requests); // Higher ingest if unordered, but no way to order txs in same slot a posteriori in that case

            let signatures_to_retry = responses
                .filter_map({
                    |(signature_data, ctx, result)| async move {
                        match result {
                            Ok(transaction) => {
                                *ctx.oldest_slot_fetched.lock().unwrap() = SlotMeta {
                                    slot: transaction.slot, // Only true because request buffer above is ordered
                                    timestamp: transaction.block_time.unwrap_or_default() as u64,
                                };

                                ctx.transaction_queue
                                    .lock()
                                    .unwrap()
                                    .push(TransactionData {
                                        indexing_address: signature_data.indexing_address,
                                        transaction,
                                    })
                                    .unwrap();

                                ctx.transaction_counter.fetch_add(1, Ordering::Relaxed);
                                None
                            }
                            Err(e) => {
                                error!("Error fetching tx {}: {}", signature_data.signature, e);
                                Some(signature_data)
                            }
                        }
                    }
                })
                .collect::<Vec<_>>()
                .await;

            if !signatures_to_retry.is_empty() {
                // This can go over the soft limit on the sig queue. Not an issue unless most fetch calls fail which would point to bigger issues
                warn!(
                    "Pushing {} signatures back on the queue after failure",
                    signatures_to_retry.len()
                );
                for sig_data in signatures_to_retry.iter() {
                    warn!("- {}", sig_data.signature);
                }
                let mut timing = Measure::start("sig_Q_push_retries_lock_wait");
                let signature_queue = ctx.signature_queue.lock().unwrap();
                timing.stop();
                if timing.as_ms() > 0 {
                    warn!("{}", timing);
                }

                signatures_to_retry.into_iter().for_each(|sig_data| {
                    signature_queue.push(sig_data).unwrap();
                })
            }
        }
    }

    async fn monitor(ctx: Arc<TransactionsCrawlerContext>) {
        let mut main_timing = Measure::start("main");
        let mut last_fetch_count = 0u64;
        let mut last_fetch_time = 0f32;

        loop {
            tokio::time::sleep(Duration::from_secs(ctx.config.monitor_interval)).await;
            main_timing.stop();
            let current_fetch_count = ctx.transaction_counter.load(Ordering::Relaxed);
            let current_fetch_time = main_timing.as_s();

            let fetch_average_ms = if (current_fetch_count - last_fetch_count) > 0 {
                (current_fetch_time - last_fetch_time)
                    / (current_fetch_count - last_fetch_count) as f32
                    * 1000.0
                    * ctx.config.max_concurrent_requests as f32
            } else {
                f32::INFINITY
            };
            let ingest_rate = if (current_fetch_time - last_fetch_time) > 0.0 {
                (current_fetch_count - last_fetch_count) as f32
                    / (current_fetch_time - last_fetch_time)
            } else {
                f32::INFINITY
            };
            let minimum_slot_available = *ctx.minimum_slot_available.lock().unwrap();
            let oldest_slot_fetched = (*ctx.oldest_slot_fetched.lock().unwrap()).clone();
            let sig_queue_size = ctx.signature_queue.lock().unwrap().len();
            let tx_queue_size = ctx.transaction_queue.lock().unwrap().len();

            info!(
                "Time: {:.1}s | Total txs: {} | {:.1}s count: {} | {:.1}s rate: {:.1} tx/s | {:.1}s avg fetch: {:.1}ms | min avail slot: {} | oldest fetched slot: {} / {:?} | from BT: {} | sig Q size: {} | tx Q size: {}",
                current_fetch_time,
                current_fetch_count,
                current_fetch_time - last_fetch_time,
                current_fetch_count - last_fetch_count,
                current_fetch_time - last_fetch_time,
                ingest_rate,
                current_fetch_time - last_fetch_time,
                fetch_average_ms,
                minimum_slot_available,
                oldest_slot_fetched.slot,
                Local.timestamp_opt(oldest_slot_fetched.timestamp.try_into().unwrap(), 0).unwrap(),
                oldest_slot_fetched.slot < minimum_slot_available,
                sig_queue_size,
                tx_queue_size
            );

            last_fetch_count = current_fetch_count;
            last_fetch_time = current_fetch_time;
        }
    }
}
