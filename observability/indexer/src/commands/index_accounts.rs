use crate::utils::{big_query::DATE_FORMAT_STR, convert_account, protos::gcp_pubsub};
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
use solana_sdk::{account::Account, pubkey::Pubkey, signature::Signature};
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
    geyser::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
        SubscribeRequestFilterAccounts, SubscribeRequestFilterSlots, SubscribeRequestPing,
    },
    tonic::transport::ClientTlsConfig,
};

#[derive(Envconfig, Debug, Clone)]
pub struct IndexAccountsConfig {
    #[envconfig(from = "INDEX_ACCOUNTS_RPC_ENDPOINT")]
    pub rpc_endpoint: String,
    #[envconfig(from = "INDEX_ACCOUNTS_RPC_TOKEN")]
    pub rpc_token: String,
    #[envconfig(from = "INDEX_ACCOUNTS_SLOTS_BUFFER_SIZE")]
    pub slots_buffer_size: u32,
    #[envconfig(from = "INDEX_ACCOUNTS_MAX_CONCURRENT_REQUESTS")]
    pub max_concurrent_requests: usize,
    #[envconfig(from = "INDEX_ACCOUNTS_MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "INDEX_ACCOUNTS_PROGRAM_ID")]
    pub program_id: Pubkey,

    #[envconfig(from = "INDEX_ACCOUNTS_PROJECT_ID")]
    pub project_id: String,
    #[envconfig(from = "INDEX_ACCOUNTS_PUBSUB_TOPIC_NAME")]
    pub topic_name: String,
    #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    pub gcp_sa_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AccountUpdateData {
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
    pub address: Pubkey,
    pub txn_signature: Option<Signature>,
    pub write_version: Option<u64>,
    pub account_data: Account,
}

#[derive(Clone)]
pub struct Context {
    pub config: Arc<IndexAccountsConfig>,
    account_updates_queue: Arc<Mutex<BTreeMap<u64, HashMap<Pubkey, AccountUpdateData>>>>,
    account_updates_counter: Arc<AtomicU64>,
    latest_slots_with_commitment: Arc<Mutex<BTreeSet<u64>>>,
    stream_disconnection_count: Arc<AtomicU64>,
    update_processing_error_count: Arc<AtomicU64>,
}

impl Context {
    pub async fn new(config: &IndexAccountsConfig) -> Self {
        Self {
            config: Arc::new(config.clone()),
            account_updates_queue: Arc::new(Mutex::new(BTreeMap::new())),
            account_updates_counter: Arc::new(AtomicU64::new(0)),
            latest_slots_with_commitment: Arc::new(Mutex::new(BTreeSet::new())),
            stream_disconnection_count: Arc::new(AtomicU64::new(0)),
            update_processing_error_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub async fn index_accounts(config: IndexAccountsConfig) -> Result<()> {
    let context = Arc::new(Context::new(&config).await);

    let listen_to_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { listen_to_updates(context).await }
    });
    let process_account_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { push_transactions_to_pubsub(context).await.unwrap() }
    });
    let monitor_handle = tokio::spawn({
        let context = context.clone();
        async move { monitor(context).await }
    });

    join_all([
        listen_to_updates_handle,
        process_account_updates_handle,
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
            accounts: HashMap::from_iter([(
                ctx.config.program_id.to_string(),
                SubscribeRequestFilterAccounts {
                    owner: vec![ctx.config.program_id.to_string()],
                    account: vec![],
                    ..Default::default()
                },
            )]),
            slots: HashMap::from_iter([(
                "slots".to_string(),
                SubscribeRequestFilterSlots::default(),
            )]),
            transactions: HashMap::default(),
            blocks: HashMap::default(),
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
                        match process_update(ctx.clone(), update) {
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

fn process_update(ctx: Arc<Context>, update: UpdateOneof) -> Result<()> {
    match update {
        UpdateOneof::Account(account_update) => {
            let update_slot = account_update.slot;
            if let Some(account_info) = account_update.account {
                let address = &Pubkey::try_from(account_info.pubkey.clone()).unwrap();
                let txn_signature = account_info
                    .txn_signature
                    .clone()
                    .map(|sig_bytes| Signature::try_from(sig_bytes).unwrap());
                let mut account_updates_queue = ctx.account_updates_queue.lock().unwrap();

                let slot_account_updates = match account_updates_queue.get_mut(&update_slot) {
                    Some(slot_account_updates) => slot_account_updates,
                    None => {
                        account_updates_queue.insert(update_slot, HashMap::default());
                        account_updates_queue.get_mut(&update_slot).unwrap()
                    }
                };

                slot_account_updates.insert(
                    *address,
                    AccountUpdateData {
                        address: *address,
                        timestamp: Utc::now(),
                        slot: update_slot,
                        txn_signature,
                        write_version: Some(account_info.write_version),
                        account_data: convert_account(account_info).unwrap(),
                    },
                );
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
    let client = Client::new(client_config).await?;

    let topic = client.topic(topic_name);
    topic
        .exists(None, None)
        .await
        .unwrap_or_else(|_| panic!("topic {} not found", topic_name));

    let publisher = topic.new_publisher(None);

    loop {
        let mut account_updates_data: Vec<AccountUpdateData> = vec![];
        {
            let mut account_updates_per_slot = ctx.account_updates_queue.lock().unwrap();
            let latest_slots_with_commitment = ctx.latest_slots_with_commitment.lock().unwrap();

            // Remove all transactions received in a slot that has not been confirmed in allotted time
            if let Some(oldest_slot_with_commitment) = latest_slots_with_commitment.first() {
                account_updates_per_slot.retain(|slot, account_updates| {
                    if slot < oldest_slot_with_commitment {
                        debug!(
                            "throwing away txs {:?} from slot {}",
                            account_updates
                                .iter()
                                .map(|(address, _)| address.to_string())
                                .collect_vec(),
                            slot
                        );
                    }

                    slot >= oldest_slot_with_commitment
                });
            }

            // Add transactions from confirmed slots to the queue of transactions to be indexed
            for (slot, slot_account_updates) in account_updates_per_slot.clone().iter() {
                if let Some(latest_slot_with_commitment) = latest_slots_with_commitment.last() {
                    if slot > latest_slot_with_commitment {
                        break; // Ok because transactions_per_slot is sorted (BtreeMap)
                    }
                }

                if latest_slots_with_commitment.contains(slot) {
                    account_updates_data.extend(slot_account_updates.values().cloned());
                    account_updates_per_slot.remove(slot);
                }
            }
        }

        if account_updates_data.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        let mut messages = vec![];

        account_updates_data.iter().for_each(|account_update_data| {
            ctx.account_updates_counter.fetch_add(1, Ordering::Relaxed);

            let now = Utc::now();

            let message = gcp_pubsub::PubsubAccountUpdate {
                id: Uuid::new_v4().to_string(),
                created_at: now.format(DATE_FORMAT_STR).to_string(),
                timestamp: account_update_data
                    .timestamp
                    .format(DATE_FORMAT_STR)
                    .to_string(),
                owner: account_update_data.account_data.owner.to_string(),
                slot: account_update_data.slot,
                pubkey: account_update_data.address.to_string(),
                txn_signature: account_update_data.txn_signature.map(|sig| sig.to_string()),
                write_version: account_update_data.write_version,
                lamports: account_update_data.account_data.lamports,
                executable: account_update_data.account_data.executable,
                rent_epoch: account_update_data.account_data.rent_epoch,
                data: general_purpose::STANDARD.encode(&account_update_data.account_data.data),
            };

            let message_str = serde_json::to_string(&message).unwrap();
            let message_bytes = message_str.as_bytes().to_vec();
            messages.push(PubsubMessage {
                data: message_bytes.into(),
                ..PubsubMessage::default()
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
        let account_updates_queue = ctx.account_updates_queue.lock().unwrap().clone();
        let earliest_block_with_commitment = latest_slots.first().unwrap_or(&0);
        let latest_block_with_commitment = latest_slots.last().unwrap_or(&u64::MAX);
        let earliest_pending_slot = account_updates_queue
            .first_key_value()
            .map(|(slot, _)| slot)
            .unwrap_or(&0);
        let latest_pending_slot = account_updates_queue
            .first_key_value()
            .map(|(slot, _)| slot)
            .unwrap_or(&u64::MAX);
        let current_fetch_count = ctx.account_updates_counter.load(Ordering::Relaxed);
        let stream_disconnection_count = ctx.stream_disconnection_count.load(Ordering::Relaxed);
        let update_processing_error_count =
            ctx.update_processing_error_count.load(Ordering::Relaxed);
        let current_fetch_time = main_timing.as_s();

        let ingest_rate = if (current_fetch_time - last_fetch_time) > 0.0 {
            (current_fetch_count - last_fetch_count) as f32 / (current_fetch_time - last_fetch_time)
        } else {
            f32::INFINITY
        };
        let account_updates_queue_size = ctx.account_updates_queue.lock().unwrap().len();

        debug!(
            "Time: {:.1}s | Total account udpates: {} | {:.1}s count: {} | {:.1}s rate: {:.1} tx/s | Tx Q size: {} | Stream disconnections: {} | Processing errors: {} | Earliest confirmed slot: {} | Latest confirmed slot: {} | Earliest pending slot: {} | Latest pending slot: {}",
            current_fetch_time,
            current_fetch_count,
            current_fetch_time - last_fetch_time,
            current_fetch_count - last_fetch_count,
            current_fetch_time - last_fetch_time,
            ingest_rate,
            account_updates_queue_size,
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
