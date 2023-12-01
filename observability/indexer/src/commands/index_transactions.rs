use crate::utils::{big_query::DATE_FORMAT_STR, protos::gcp_pubsub};
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use futures::{future::join_all, SinkExt, StreamExt};
use google_cloud_default::WithAuthExt;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use itertools::Itertools;
use solana_measure::measure::Measure;
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::TransactionVersion};
use solana_transaction_status::{
    TransactionWithStatusMeta, UiTransactionStatusMeta, VersionedTransactionWithStatusMeta,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tonic::Status;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    convert_from,
    geyser::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
        SubscribeRequestFilterSlots, SubscribeRequestFilterTransactions, SubscribeRequestPing,
    },
    tonic::transport::ClientTlsConfig,
};

#[derive(Envconfig, Debug, Clone)]
pub struct IndexTransactionsConfig {
    #[envconfig(from = "INDEX_TRANSACTIONS_RPC_ENDPOINT")]
    pub rpc_endpoint: String,
    #[envconfig(from = "INDEX_TRANSACTIONS_RPC_TOKEN")]
    pub rpc_token: String,
    #[envconfig(from = "INDEX_TRANSACTIONS_SLOTS_BUFFER_SIZE")]
    pub slots_buffer_size: u32,
    #[envconfig(from = "INDEX_TRANSACTIONS_MAX_CONCURRENT_REQUESTS")]
    pub max_concurrent_requests: usize,
    #[envconfig(from = "INDEX_TRANSACTIONS_MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "INDEX_TRANSACTIONS_PROGRAM_ID")]
    pub program_id: Pubkey,

    #[envconfig(from = "INDEX_TRANSACTIONS_PROJECT_ID")]
    pub project_id: String,
    #[envconfig(from = "INDEX_TRANSACTIONS_PUBSUB_TOPIC_NAME")]
    pub topic_name: String,
    #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    pub gcp_sa_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionData {
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
    pub signature: Signature,
    pub indexing_addresses: Vec<String>,
    pub transaction: VersionedTransactionWithStatusMeta,
}

#[derive(Clone)]
pub struct Context {
    pub config: Arc<IndexTransactionsConfig>,
    transactions_queue: Arc<Mutex<BTreeMap<u64, Vec<TransactionData>>>>,
    transactions_counter: Arc<AtomicU64>,
    latest_slots_with_commitment: Arc<Mutex<BTreeSet<u64>>>,
    stream_disconnection_count: Arc<AtomicU64>,
    update_processing_error_count: Arc<AtomicU64>,
}

impl Context {
    pub async fn new(config: &IndexTransactionsConfig) -> Self {
        Self {
            config: Arc::new(config.clone()),
            transactions_queue: Arc::new(Mutex::new(BTreeMap::new())),
            transactions_counter: Arc::new(AtomicU64::new(0)),
            latest_slots_with_commitment: Arc::new(Mutex::new(BTreeSet::new())),
            stream_disconnection_count: Arc::new(AtomicU64::new(0)),
            update_processing_error_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub async fn index_transactions(config: IndexTransactionsConfig) -> Result<()> {
    let context = Arc::new(Context::new(&config).await);

    let listen_to_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { listen_to_updates(context).await }
    });
    let process_transactions_handle = tokio::spawn({
        let context = context.clone();
        async move { push_transactions_to_pubsub(context).await.unwrap() }
    });
    let monitor_handle = tokio::spawn({
        let context = context.clone();
        async move { monitor(context).await }
    });

    join_all([
        listen_to_updates_handle,
        process_transactions_handle,
        monitor_handle,
    ])
    .await;

    Ok(())
}

async fn listen_to_updates(ctx: Arc<Context>) {
    loop {
        info!("Connecting geyser client");
        let geyser_client_connection_result = GeyserGrpcClient::connect(
            ctx.config.rpc_endpoint.to_string(),
            Some(ctx.config.rpc_token.to_string()),
            Some(ClientTlsConfig::new()),
        );

        let mut geyser_client = match geyser_client_connection_result {
            Ok(geyser_client) => geyser_client,
            Err(err) => {
                error!("Error connecting to geyser client: {}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Establish streams
        let (mut subscribe_request_sink, mut stream) = match geyser_client.subscribe().await {
            Ok(value) => value,
            Err(e) => {
                error!("Error subscribing geyser client {e}");
                continue;
            }
        };

        let subscribe_request = SubscribeRequest {
            accounts: HashMap::default(),
            slots: HashMap::from_iter([(
                "slots".to_string(),
                SubscribeRequestFilterSlots::default(),
            )]),
            transactions: HashMap::from_iter([(
                ctx.config.program_id.to_string(),
                SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: Some(false),
                    account_include: vec![ctx.config.program_id.to_string()],
                    account_exclude: vec![],
                    ..Default::default()
                },
            )]),
            ping: Some(SubscribeRequestPing::default()),
            ..Default::default()
        };

        // Send initial subscription config
        match subscribe_request_sink.send(subscribe_request).await {
            Ok(()) => info!("Successfully sent initial subscription config"),
            Err(e) => {
                error!("Error establishing geyser sub: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        while let Some(received) = stream.next().await {
            match received {
                Ok(received) => {
                    if let Some(update) = received.update_oneof {
                        match process_update(ctx.clone(), &received.filters, update) {
                            Ok(_) => {}
                            Err(err) => {
                                error!("Error processing update: {}", err);
                                ctx.update_processing_error_count
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Error pulling next update: {}", err);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    break;
                }
            }
        }

        error!("Stream got disconnected");
        ctx.stream_disconnection_count
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn process_update(ctx: Arc<Context>, filters: &[String], update: UpdateOneof) -> Result<()> {
    match update {
        UpdateOneof::Transaction(transaction_update) => {
            if let Some(transaction_info) = transaction_update.transaction {
                let signature = transaction_info.signature.clone();
                let transaction = convert_from::create_tx_with_meta(transaction_info).unwrap();
                let mut transactions_queue = ctx.transactions_queue.lock().unwrap();

                let slot_transactions = match transactions_queue.get_mut(&transaction_update.slot) {
                    Some(slot_transactions) => slot_transactions,
                    None => {
                        transactions_queue.insert(transaction_update.slot, vec![]);
                        transactions_queue
                            .get_mut(&transaction_update.slot)
                            .unwrap()
                    }
                };

                let transaction = match transaction {
                    TransactionWithStatusMeta::MissingMetadata(transaction) => {
                        error!(
                            "Missing metadata for transaction {}. Skipping potentially relevant transaction.",
                            transaction.signatures.first().unwrap()
                        );
                        return Ok(());
                    }
                    TransactionWithStatusMeta::Complete(transaction_with_meta) => {
                        transaction_with_meta
                    }
                };

                slot_transactions.push(TransactionData {
                    timestamp: Utc::now(),
                    signature: Signature::try_from(signature).unwrap(),
                    slot: transaction_update.slot,
                    indexing_addresses: filters.to_vec(),
                    transaction,
                });
                // println!("slot_transactions for {:?} at {}: {}", filters.to_vec(), transaction_update.slot, slot_transactions.len());
            } else {
                anyhow::bail!("Expected `transaction` in `UpdateOneof::Transaction` update");
            }
        }
        UpdateOneof::Slot(slot) => {
            if slot.status == CommitmentLevel::Confirmed as i32
                || slot.status == CommitmentLevel::Finalized as i32
            {
                let mut latest_slots = ctx.latest_slots_with_commitment.lock().unwrap();
                let slot_inserted = latest_slots.insert(slot.slot);
                if slot_inserted && latest_slots.len() > ctx.config.slots_buffer_size as usize {
                    let oldest_slot = *latest_slots.first().unwrap();
                    latest_slots.remove(&oldest_slot);
                }
            }
        }
        UpdateOneof::Ping(_) => {
            debug!("ping");
        }
        _ => {
            warn!("unknown update");
        }
    }

    Ok(())
}

pub async fn push_transactions_to_pubsub(ctx: Arc<Context>) -> Result<()> {
    let topic_name = ctx.config.topic_name.as_str();

    let client_config = ClientConfig::default().with_auth().await?;
    let client = Client::new(client_config).await.unwrap();

    let topic = client.topic(topic_name);
    topic
        .exists(None, None)
        .await
        .unwrap_or_else(|_| panic!("topic {} not found", topic_name));

    let publisher = topic.new_publisher(None);

    loop {
        let mut transactions_data: Vec<TransactionData> = vec![];
        {
            let mut transactions_per_slot = ctx.transactions_queue.lock().unwrap();
            let latest_slots_with_commitment = ctx.latest_slots_with_commitment.lock().unwrap();

            // Remove all transactions received in a slot that has not been confirmed in allotted time
            if let Some(oldest_slot_with_commitment) = latest_slots_with_commitment.first() {
                transactions_per_slot.retain(|slot, transactions| {
                    if slot < oldest_slot_with_commitment {
                        debug!(
                            "throwing away txs {:?} from slot {}",
                            transactions
                                .iter()
                                .map(|tx| tx.signature.to_string())
                                .collect_vec(),
                            slot
                        );
                    }

                    slot >= oldest_slot_with_commitment
                });
            }

            // Add transactions from confirmed slots to the queue of transactions to be indexed
            for (slot, slot_transactions) in transactions_per_slot.clone().iter() {
                if let Some(latest_slot_with_commitment) = latest_slots_with_commitment.last() {
                    if slot > latest_slot_with_commitment {
                        break; // Ok because transactions_per_slot is sorted (BtreeMap)
                    }
                }

                if latest_slots_with_commitment.contains(slot) {
                    transactions_data.extend(slot_transactions.clone());
                    transactions_per_slot.remove(slot);
                }
            }
        }

        if transactions_data.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        let mut messages = vec![];

        transactions_data.iter().for_each(|transaction_data| {
            ctx.transactions_counter.fetch_add(1, Ordering::Relaxed);
            // println!(
            //     "{:?} - {}",
            //     transaction_data.indexing_addresses,
            //     transaction_data
            //         .transaction
            //         .transaction
            //         .signatures
            //         .first()
            //         .unwrap()
            // );

            let now = Utc::now();

            transaction_data
                .indexing_addresses
                .iter()
                .for_each(|indexing_address| {
                    let message = gcp_pubsub::PubsubTransaction {
                        id: Uuid::new_v4().to_string(),
                        created_at: now.format(DATE_FORMAT_STR).to_string(),
                        timestamp: transaction_data
                            .timestamp
                            .format(DATE_FORMAT_STR)
                            .to_string(),
                        signature: transaction_data.signature.to_string(),
                        indexing_address: indexing_address.to_string(),
                        slot: transaction_data.slot,
                        signer: transaction_data
                            .transaction
                            .transaction
                            .message
                            .static_account_keys()
                            .first()
                            .unwrap()
                            .to_string(),
                        success: transaction_data.transaction.meta.status.is_ok(),
                        version: match transaction_data.transaction.transaction.version() {
                            TransactionVersion::Legacy(_) => "legacy".to_string(),
                            TransactionVersion::Number(version) => version.to_string(),
                        },

                        fee: transaction_data.transaction.meta.fee,
                        meta: serde_json::to_string(&UiTransactionStatusMeta::from(
                            transaction_data.transaction.meta.clone(),
                        ))
                        .unwrap(),
                        message: general_purpose::STANDARD
                            .encode(transaction_data.transaction.transaction.message.serialize()),
                    };

                    let message_str = serde_json::to_string(&message).unwrap();
                    let message_bytes = message_str.as_bytes().to_vec();
                    messages.push(PubsubMessage {
                        data: message_bytes.into(),
                        ..PubsubMessage::default()
                    });
                });
        });

        // Send a message. There are also `publish_bulk` and `publish_immediately` methods.
        let awaiters = publisher.publish_bulk(messages).await;

        // The get method blocks until a server-generated ID or an error is returned for the published message.
        let pub_results: Vec<Result<String, Status>> = join_all(
            awaiters
                .into_iter()
                .map(|awaiter| awaiter.get(None))
                .collect_vec(),
        )
        .await;

        pub_results.into_iter().for_each(|result| match result {
            Ok(_) => {}
            Err(status) => {
                error!(
                    "Error sending tx to pubsub (code {:?}): {:?}",
                    status.code(),
                    status.message()
                )
            }
        });
    }
}

async fn monitor(ctx: Arc<Context>) {
    let mut main_timing = Measure::start("main");
    let mut last_fetch_count = 0u64;
    let mut last_fetch_time = 0f32;

    loop {
        tokio::time::sleep(Duration::from_secs(ctx.config.monitor_interval)).await;
        main_timing.stop();
        let latest_slots = ctx.latest_slots_with_commitment.lock().unwrap().clone();
        let tx_queue = ctx.transactions_queue.lock().unwrap().clone();
        let earliest_block_with_commitment = latest_slots.first().unwrap_or(&0);
        let latest_block_with_commitment = latest_slots.last().unwrap_or(&u64::MAX);
        let earliest_pending_slot = tx_queue
            .first_key_value()
            .map(|(slot, _)| slot)
            .unwrap_or(&0);
        let latest_pending_slot = tx_queue
            .first_key_value()
            .map(|(slot, _)| slot)
            .unwrap_or(&u64::MAX);
        let current_fetch_count = ctx.transactions_counter.load(Ordering::Relaxed);
        let stream_disconnection_count = ctx.stream_disconnection_count.load(Ordering::Relaxed);
        let update_processing_error_count =
            ctx.update_processing_error_count.load(Ordering::Relaxed);
        let current_fetch_time = main_timing.as_s();

        let ingest_rate = if (current_fetch_time - last_fetch_time) > 0.0 {
            (current_fetch_count - last_fetch_count) as f32 / (current_fetch_time - last_fetch_time)
        } else {
            f32::INFINITY
        };
        let tx_queue_size = ctx.transactions_queue.lock().unwrap().len();

        debug!(
            "Time: {:.1}s | Total txs: {} | {:.1}s count: {} | {:.1}s rate: {:.1} tx/s | Tx Q size: {} | Stream disconnections: {} | Processing errors: {} | Earliest confirmed slot: {} | Latest confirmed slot: {} | Earliest pending slot: {} | Latest pending slot: {}",
            current_fetch_time,
            current_fetch_count,
            current_fetch_time - last_fetch_time,
            current_fetch_count - last_fetch_count,
            current_fetch_time - last_fetch_time,
            ingest_rate,
            tx_queue_size,
            stream_disconnection_count,
            update_processing_error_count,
            earliest_block_with_commitment,
            latest_block_with_commitment,
            earliest_pending_slot,
            latest_pending_slot,
        );

        last_fetch_count = current_fetch_count;
        last_fetch_time = current_fetch_time;
    }
}
