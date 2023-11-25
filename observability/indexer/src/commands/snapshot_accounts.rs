use crate::utils::geyser_client::get_geyser_client;
use crate::utils::metrics::{LendingPoolBankMetrics, MarginfiAccountMetrics, MarginfiGroupMetrics};
use crate::utils::protos::SubscribeRequestFilterBlocks;
use crate::utils::snapshot::{AccountRoutingType, BankUpdateRoutingType};
use crate::utils::{
    protos::geyser::{
        subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
        SubscribeRequestFilterSlots, SubscribeUpdateSlotStatus,
    },
    snapshot::Snapshot,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use futures::stream::once;
use futures::{future::join_all, pin_mut, StreamExt};
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use itertools::Itertools;
use rayon::prelude::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_measure::measure::Measure;
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::Signature,
};
use std::sync::atomic::AtomicI64;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Debug,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};
use yup_oauth2::parse_service_account_key;

#[derive(Debug, Clone)]
pub struct PubkeyVec(pub Vec<Pubkey>); // Ugh

impl FromStr for PubkeyVec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let targets_raw = json::parse(s).unwrap();
        if !targets_raw.is_array() {
            return Err(anyhow::Error::msg(format!(
                "Invalid base58 pubkey array: {}",
                s
            )));
        }

        let mut targets: Vec<Pubkey> = vec![];
        for i in 0..targets_raw.len() {
            targets.push(Pubkey::from_str(targets_raw[i].as_str().unwrap()).unwrap());
        }
        Ok(Self(targets))
    }
}

#[derive(Envconfig, Debug, Clone)]
pub struct SnapshotAccountsConfig {
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_RPC_ENDPOINT")]
    pub rpc_endpoint: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_RPC_ENDPOINT_GEYSER")]
    pub rpc_endpoint_geyser: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_RPC_TOKEN")]
    pub rpc_token: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_SLOTS_BUFFER_SIZE")]
    pub slots_buffer_size: u32,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_SNAP_INTERVAL")]
    pub snap_interval: u64,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_PROGRAM_ID")]
    pub program_id: Pubkey,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_ADDITIONAL_ACCOUNTS")]
    pub additional_accounts: PubkeyVec,

    #[envconfig(from = "SNAPSHOT_ACCOUNTS_PROJECT_ID")]
    pub project_id: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_DATASET_ID")]
    pub dataset_id: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_TABLE_GROUP_METRICS")]
    pub table_group: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_TABLE_BANK_METRICS")]
    pub table_bank: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_TABLE_ACCOUNT_METRICS")]
    pub table_account: String,
    #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    pub gcp_sa_key: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AccountUpdate {
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
    pub address: Pubkey,
    pub txn_signature: Option<Signature>,
    pub write_version: Option<u64>,
    pub account_data: Account,
}

#[derive(Clone)]
pub struct Context {
    pub timestamp: Arc<AtomicI64>,
    pub config: Arc<SnapshotAccountsConfig>,
    pub rpc_client: Arc<RpcClient>,
    pub geyser_subscription_config: Arc<Mutex<(bool, SubscribeRequest)>>,
    account_updates_queue: Arc<Mutex<BTreeMap<u64, HashMap<Pubkey, AccountUpdate>>>>,
    latest_slots_with_commitment: Arc<Mutex<BTreeSet<u64>>>,
    account_snapshot: Arc<Mutex<Snapshot>>,
    stream_disconnection_count: Arc<AtomicU64>,
    update_processing_error_count: Arc<AtomicU64>,
}

impl Context {
    pub async fn new(config: &SnapshotAccountsConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            format!("{}/{}", config.rpc_endpoint, config.rpc_token),
            CommitmentConfig {
                commitment: CommitmentLevel::Finalized,
            },
        ));
        Self {
            timestamp: Arc::new(AtomicI64::new(0)),
            config: Arc::new(config.clone()),
            rpc_client: rpc_client.clone(),
            geyser_subscription_config: Arc::new(Mutex::new((false, SubscribeRequest::default()))),
            account_updates_queue: Arc::new(Mutex::new(BTreeMap::new())),
            latest_slots_with_commitment: Arc::new(Mutex::new(BTreeSet::new())),
            account_snapshot: Arc::new(Mutex::new(Snapshot::new(config.program_id, rpc_client))),
            stream_disconnection_count: Arc::new(AtomicU64::new(0)),
            update_processing_error_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

async fn compute_geyser_config(
    config: &SnapshotAccountsConfig,
    non_program_pubkeys: &[Pubkey],
) -> SubscribeRequest {
    let mut accounts = config.additional_accounts.0.clone();
    accounts.append(&mut non_program_pubkeys.to_vec());
    accounts.sort();
    accounts.dedup();

    SubscribeRequest {
        accounts: HashMap::from_iter([
            (
                config.program_id.to_string(),
                SubscribeRequestFilterAccounts {
                    owner: vec![config.program_id.to_string()],
                    account: vec![],
                },
            ),
            (
                "lol".to_string(),
                SubscribeRequestFilterAccounts {
                    owner: vec![],
                    account: accounts.iter().map(|x| x.to_string()).collect_vec(),
                },
            ),
        ]),
        slots: HashMap::from_iter([("slots".to_string(), SubscribeRequestFilterSlots {})]),
        transactions: HashMap::default(),
        blocks: HashMap::from_iter([("blocks".to_string(), SubscribeRequestFilterBlocks {})]),
    }
}

pub async fn snapshot_accounts(config: SnapshotAccountsConfig) -> Result<()> {
    let context = Arc::new(Context::new(&config).await);

    info!("Fetching initial snapshot");
    let non_program_accounts = {
        let mut snapshot = context.account_snapshot.lock().await;
        snapshot.init().await.unwrap();
        println!("Summary: {snapshot}");

        snapshot
            .routing_lookup
            .iter()
            .filter(|(_, routing_type)| match routing_type {
                AccountRoutingType::MarginfiGroup => false,
                AccountRoutingType::MarginfiAccount => false,
                AccountRoutingType::Bank(_, bank_update_routing_type) => {
                    !matches!(bank_update_routing_type, BankUpdateRoutingType::State)
                }
                _ => true,
            })
            .map(|(pubkey, _)| *pubkey)
            .unique()
            .collect_vec()
    };

    let geyser_subscription_config = compute_geyser_config(&config, &non_program_accounts).await;
    *context.geyser_subscription_config.lock().await = (false, geyser_subscription_config.clone());

    let listen_to_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { listen_to_updates(context).await }
    });

    let update_account_map_handle = tokio::spawn({
        let context = context.clone();
        async move { update_account_map(context).await }
    });
    let process_account_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { push_transactions_to_bigquery(context).await }
    });
    let monitor_handle = tokio::spawn({
        let context = context.clone();
        async move { monitor(context).await }
    });

    join_all([
        listen_to_updates_handle,
        process_account_updates_handle,
        update_account_map_handle,
        monitor_handle,
    ])
    .await;

    Ok(())
}

async fn listen_to_updates(ctx: Arc<Context>) {
    loop {
        info!("Instantiating geyser client");
        let geyser_client = get_geyser_client(
            ctx.config.rpc_endpoint_geyser.to_string(),
            ctx.config.rpc_token.to_string(),
        )
        .await;

        match geyser_client {
            Ok(geyser_client) => {
                let geyser_client = Box::pin(geyser_client);
                pin_mut!(geyser_client);
                let (_, geyser_config) = ctx.geyser_subscription_config.lock().await.clone();
                debug!("Subscribing to geyser with {:?}", geyser_config);
                let stream_response = geyser_client
                    .subscribe(once(async move { geyser_config }))
                    .await;

                match stream_response {
                    Ok(stream_response) => {
                        info!("Subscribed to updates");
                        let mut stream = stream_response.into_inner();
                        while let Some(received) = stream.next().await {
                            let mut geyser_sub_config = ctx.geyser_subscription_config.lock().await;
                            if geyser_sub_config.0 {
                                warn!("Config update: {:?}", geyser_sub_config.1);
                                geyser_sub_config.0 = false;
                                continue;
                            }

                            match received {
                                Ok(received) => {
                                    if let Some(update) = received.update_oneof {
                                        match process_update(ctx.clone(), update).await {
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
                    Err(err) => {
                        error!("Error establishing geyser sub: {}", err);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            Err(err) => {
                error!("Error creating geyser client: {}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_update(ctx: Arc<Context>, update: UpdateOneof) -> Result<()> {
    match update {
        UpdateOneof::Account(account_update) => {
            let update_slot = account_update.slot;
            if let Some(account_info) = account_update.account {
                let address = Pubkey::new(&account_info.pubkey);
                let txn_signature = account_info
                    .txn_signature
                    .clone()
                    .map(|sig_bytes| Signature::new(&sig_bytes));
                let mut account_updates_queue = ctx.account_updates_queue.lock().await;

                let slot_account_updates = match account_updates_queue.get_mut(&update_slot) {
                    Some(slot_account_updates) => slot_account_updates,
                    None => {
                        account_updates_queue.insert(update_slot, HashMap::default());
                        account_updates_queue.get_mut(&update_slot).unwrap()
                    }
                };
                slot_account_updates.insert(
                    address,
                    AccountUpdate {
                        address,
                        timestamp: Utc::now(),
                        slot: update_slot,
                        txn_signature,
                        write_version: Some(account_info.write_version),
                        account_data: account_info.try_into()?,
                    },
                );
            } else {
                anyhow::bail!("Expected `transaction` in `UpdateOneof::Transaction` update");
            }
        }
        UpdateOneof::Slot(slot) => {
            if slot.status == SubscribeUpdateSlotStatus::Confirmed as i32
                || slot.status == SubscribeUpdateSlotStatus::Finalized as i32
            {
                let mut latest_slots = ctx.latest_slots_with_commitment.lock().await;
                let slot_inserted = latest_slots.insert(slot.slot);
                if slot_inserted && latest_slots.len() > ctx.config.slots_buffer_size as usize {
                    let oldest_slot = *latest_slots.first().unwrap();
                    latest_slots.remove(&oldest_slot);
                }
            }
        }
        UpdateOneof::Block(block_update) => {
            if let Some(block_time) = block_update.block_time {
                ctx.timestamp.store(block_time.timestamp, Ordering::Relaxed);
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

pub async fn update_account_map(ctx: Arc<Context>) {
    loop {
        let mut confirmed_account_updates: Vec<AccountUpdate> = vec![];
        {
            let mut account_updates_per_slot = ctx.account_updates_queue.lock().await;
            let latest_slots_with_commitment = ctx.latest_slots_with_commitment.lock().await;

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
                    confirmed_account_updates.extend(slot_account_updates.values().cloned());
                    account_updates_per_slot.remove(slot);
                }
            }
        }

        if confirmed_account_updates.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        let mut accounts_snapshot = ctx.account_snapshot.lock().await;
        for account_update in confirmed_account_updates {
            if accounts_snapshot
                .routing_lookup
                .contains_key(&account_update.address)
            {
                accounts_snapshot
                    .udpate_entry(&account_update.address, &account_update.account_data);
            } else {
                accounts_snapshot
                    .create_entry(&account_update.address, &account_update.account_data)
                    .await;

                let non_program_accounts = accounts_snapshot
                    .routing_lookup
                    .iter()
                    .filter(|(_, routing_type)| match routing_type {
                        AccountRoutingType::MarginfiGroup => false,
                        AccountRoutingType::MarginfiAccount => false,
                        AccountRoutingType::Bank(_, bank_update_routing_type) => {
                            !matches!(bank_update_routing_type, BankUpdateRoutingType::State)
                        }
                        _ => true,
                    })
                    .map(|(pubkey, _)| *pubkey)
                    .unique()
                    .collect_vec();
                let updated_geyser_config =
                    compute_geyser_config(&ctx.config, &non_program_accounts).await;
                info!("updating geyser sub: {:?}", updated_geyser_config.accounts);
                *ctx.geyser_subscription_config.lock().await = (true, updated_geyser_config);
            }
        }
    }
}

pub async fn push_transactions_to_bigquery(ctx: Arc<Context>) {
    let bq_client = if let Some(gcp_sa_key) = ctx.config.gcp_sa_key.clone() {
        let sa_key = parse_service_account_key(&gcp_sa_key).unwrap();
        gcp_bigquery_client::Client::from_service_account_key(sa_key, false)
            .await
            .unwrap()
    } else {
        gcp_bigquery_client::Client::from_application_default_credentials()
            .await
            .unwrap()
    };

    tokio::time::sleep(Duration::from_secs(5)).await;
    while ctx.timestamp.load(Ordering::Relaxed) == 0 {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    info!("Starting to generate snapshots");
    loop {
        let start = Instant::now();
        let snapshot = ctx.account_snapshot.lock().await.clone();
        let timestamp = ctx.timestamp.load(Ordering::Relaxed);

        let all_group_metrics = snapshot
            .marginfi_groups
            .par_iter()
            .map(|(marginfi_group_pk, marginfi_group)| {
                (
                    marginfi_group_pk,
                    MarginfiGroupMetrics::new(
                        timestamp,
                        marginfi_group_pk,
                        marginfi_group,
                        &snapshot,
                    ),
                )
            })
            .collect::<Vec<_>>();
        let all_bank_metrics = snapshot
            .banks
            .par_iter()
            .map(|(bank_pk, bank_accounts)| {
                (
                    bank_pk,
                    LendingPoolBankMetrics::new(timestamp, bank_pk, bank_accounts, &snapshot),
                )
            })
            .collect::<Vec<_>>();
        let all_marginfi_account_metrics = snapshot
            .marginfi_accounts
            .par_iter()
            .map(|(marginfi_account_pk, marginfi_account)| {
                (
                    marginfi_account_pk,
                    MarginfiAccountMetrics::new(
                        timestamp,
                        marginfi_account_pk,
                        marginfi_account,
                        &snapshot,
                    ),
                )
            })
            .collect::<Vec<_>>();

        let elapsed = Instant::now() - start;
        debug!("Time to create metrics: {:?}", elapsed);

        let mut insert_request = TableDataInsertAllRequest::new();
        all_group_metrics
            .iter()
            .for_each(|(id, metrics_result)| match metrics_result {
                Ok(metrics) => insert_request.add_row(None, metrics.to_row()).unwrap(),
                Err(err) => warn!("Failed to create metrics for marginfi group {id}: {err}"),
            });
        let result = write_to_bq(
            &bq_client,
            &ctx.config.project_id,
            &ctx.config.dataset_id,
            &ctx.config.table_group,
            timestamp,
            insert_request,
        )
        .await;
        if let Err(error) = result {
            warn!(
                "Failed to write marginfi group metrics to bigquery: {}",
                error
            );
        }

        let mut insert_request = TableDataInsertAllRequest::new();
        all_bank_metrics
            .iter()
            .for_each(|(id, metrics_result)| match metrics_result {
                Ok(metrics) => insert_request.add_row(None, metrics.to_row()).unwrap(),
                Err(err) => warn!("Failed to create metrics for bank {id}: {err}"),
            });
        let result = write_to_bq(
            &bq_client,
            &ctx.config.project_id,
            &ctx.config.dataset_id,
            &ctx.config.table_bank,
            timestamp,
            insert_request,
        )
        .await;
        if let Err(error) = result {
            warn!(
                "Failed to write lending pool bank metrics to bigquery: {}",
                error
            );
        }

        let insert_requests: Vec<TableDataInsertAllRequest> = all_marginfi_account_metrics
            .chunks(7000)
            .map(|metrics_results_chunk| {
                let mut insert_request: TableDataInsertAllRequest =
                    TableDataInsertAllRequest::new();

                metrics_results_chunk.iter().for_each(
                    |(id, metrics_result)| match metrics_result {
                        Ok(metrics) => insert_request.add_row(None, metrics.to_row()).unwrap(),
                        Err(err) => {
                            warn!("Failed to create metrics for marginfi account {id}: {err}")
                        }
                    },
                );

                insert_request
            })
            .collect::<Vec<_>>();

        for insert_request in insert_requests {
            let result = write_to_bq(
                &bq_client,
                &ctx.config.project_id,
                &ctx.config.dataset_id,
                &ctx.config.table_account,
                timestamp,
                insert_request,
            )
            .await;
            if let Err(error) = result {
                warn!(
                    "Failed to write marginfi account metrics to bigquery: {}",
                    error
                );
            }
        }

        tokio::time::sleep(Duration::from_secs(ctx.config.snap_interval)).await;
    }
}

pub async fn write_to_bq(
    bq_client: &gcp_bigquery_client::Client,
    project_id: &str,
    dataset_id: &str,
    table_id: &str,
    timestamp: i64,
    insert_request: TableDataInsertAllRequest,
) -> Result<()> {
    let result = bq_client
        .tabledata()
        .insert_all(project_id, dataset_id, table_id, insert_request)
        .await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            error!("Errors inserting for timestamp {}", timestamp);
            error!("details: {:?}", err);
            return Ok(());
        }
    };

    if let Some(errors) = result.insert_errors {
        error!("Errors inserting for timestamp {}", timestamp);
        error!("details:");
        errors.iter().for_each(|error| println!("-{:?}", error));
    }

    Ok(())
}

async fn monitor(ctx: Arc<Context>) {
    let mut main_timing = Measure::start("main");

    loop {
        tokio::time::sleep(Duration::from_secs(ctx.config.monitor_interval)).await;
        main_timing.stop();
        let latest_slots = ctx.latest_slots_with_commitment.lock().await.clone();
        let account_updates_queue = ctx.account_updates_queue.lock().await.clone();
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
        let stream_disconnection_count = ctx.stream_disconnection_count.load(Ordering::Relaxed);
        let update_processing_error_count =
            ctx.update_processing_error_count.load(Ordering::Relaxed);
        let current_fetch_time = main_timing.as_s();

        let account_updates_queue_size = ctx.account_updates_queue.lock().await.len();

        info!(
            "Time: {:.1}s | Tx Q size: {} | Stream disconnections: {} | Processing errors: {} | Earliest confirmed slot: {} | Latest confirmed slot: {} | Earliest pending slot: {} | Latest pending slot: {}",
            current_fetch_time,
            account_updates_queue_size,
            stream_disconnection_count,
            update_processing_error_count,
            earliest_block_with_commitment,
            latest_block_with_commitment,
            earliest_pending_slot,
            latest_pending_slot,
        );
    }
}
