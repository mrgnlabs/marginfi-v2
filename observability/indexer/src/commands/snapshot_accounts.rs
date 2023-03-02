use crate::{
    commands::geyser_client::get_geyser_client,
    utils::{
        protos::geyser::{
            subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
            SubscribeRequestFilterSlots, SubscribeUpdateSlotStatus,
        },
        snapshot::{OracleData, Snapshot},
    },
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use futures::{future::join_all, stream, StreamExt};
use itertools::Itertools;
use marginfi::state::marginfi_account::{calc_asset_value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_measure::measure::Measure;
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::Signature,
};
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
use std::ops::Div;
use fixed::types::I80F48;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use marginfi::constants::ZERO_AMOUNT_THRESHOLD;

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
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_MAX_CONCURRENT_REQUESTS")]
    pub max_concurrent_requests: usize,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_PROGRAM_ID")]
    pub program_id: Pubkey,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_MARGINFI_GROUP")]
    pub marginfi_group: Pubkey,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_ADDITIONAL_ACCOUNTS")]
    pub additional_accounts: PubkeyVec,

    #[envconfig(from = "SNAPSHOT_ACCOUNTS_PROJECT_ID")]
    pub project_id: String,
    #[envconfig(from = "SNAPSHOT_ACCOUNTS_PUBSUB_TOPIC_NAME")]
    pub topic_name: String,
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
    pub config: Arc<SnapshotAccountsConfig>,
    pub rpc_client: Arc<RpcClient>,
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
            config: Arc::new(config.clone()),
            rpc_client: rpc_client.clone(),
            account_updates_queue: Arc::new(Mutex::new(BTreeMap::new())),
            latest_slots_with_commitment: Arc::new(Mutex::new(BTreeSet::new())),
            account_snapshot: Arc::new(Mutex::new(Snapshot::new(
                config.program_id,
                rpc_client,
            ))),
            stream_disconnection_count: Arc::new(AtomicU64::new(0)),
            update_processing_error_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub async fn snapshot_accounts(config: SnapshotAccountsConfig) -> Result<()> {
    let context = Arc::new(Context::new(&config).await);

    let listen_to_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { listen_to_updates(context).await }
    });
    let process_account_updates_handle = tokio::spawn({
        let context = context.clone();
        async move { push_transactions_to_pubsub(context).await }
    });
    let update_account_map_handle = tokio::spawn({
        let context = context.clone();
        async move { update_account_map(context).await }
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
        info!("Fetching target group");
        {
            let mut snapshot_accounts = ctx.account_snapshot.lock().await;
            snapshot_accounts.init(ctx.clone()).await.unwrap();
            println!("{snapshot_accounts}");
        }

        info!("Instantiating geyser client");
        match get_geyser_client(
            ctx.config.rpc_endpoint_geyser.to_string(),
            ctx.config.rpc_token.to_string(),
        )
            .await
        {
            Ok(mut geyser_client) => {
                info!("Subscribing to updates for {:?}", ctx.config.program_id);
                let stream_request = geyser_client
                    .subscribe(stream::iter([SubscribeRequest {
                        accounts: HashMap::from_iter([(
                            ctx.config.program_id.to_string(),
                            SubscribeRequestFilterAccounts {
                                owner: vec![ctx.config.program_id.to_string()],
                                account: ctx
                                    .config
                                    .additional_accounts
                                    .0
                                    .iter()
                                    .map(|x| x.to_string())
                                    .collect_vec(),
                            },
                        )]),
                        slots: HashMap::from_iter([(
                            "slots".to_string(),
                            SubscribeRequestFilterSlots {},
                        )]),
                        transactions: HashMap::default(),
                        blocks: HashMap::default(),
                        blocks_meta: HashMap::default(),
                    }]))
                    .await;

                match stream_request {
                    Ok(stream_response) => {
                        info!("Subscribed to updates");
                        let mut stream = stream_response.into_inner();
                        while let Some(received) = stream.next().await {
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
                let address = Pubkey::try_from(account_info.pubkey.clone()).unwrap();
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
                        account_data: account_info.into(),
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
            accounts_snapshot.process_update(&account_update).await;
        }
    }
}

#[derive(Debug)]
pub struct MarginfiGroupMetrics {
    pub pubkey: Pubkey,
    pub total_marginfi_accounts: u64,
    pub total_banks: u64,
    pub total_mints: u64,
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
}

impl MarginfiGroupMetrics {
    pub fn new(snapshot: &Snapshot) -> Self {
        let (total_assets_usd, total_liabilities_usd) = snapshot.banks.iter().fold((0.0, 0.0), |mut sums, (_, bank_accounts)| {
            let total_asset_share = bank_accounts
                .bank
                .total_asset_shares;
            let total_liability_share = bank_accounts
                .bank
                .total_liability_shares;
            let price = snapshot.price_feeds.get(&bank_accounts.bank.config.get_pyth_oracle_key()).unwrap().get_price();

            let asset_value_usd = calc_asset_value(bank_accounts.bank.get_asset_amount(total_asset_share.into()).unwrap(), price, bank_accounts.bank.mint_decimals, None).unwrap().to_num::<f64>();
            let liability_value_usd = calc_asset_value(bank_accounts.bank.get_liability_amount(total_liability_share.into()).unwrap(), price, bank_accounts.bank.mint_decimals, None).unwrap().to_num::<f64>();

            sums.0 += asset_value_usd;
            sums.1 += liability_value_usd;

            sums
        });


        Self {
            pubkey: snapshot.marginfi_group.0,
            total_marginfi_accounts: snapshot.marginfi_accounts.len() as u64,
            total_banks: snapshot.banks.len() as u64,
            total_mints: snapshot
                .banks
                .iter()
                .unique_by(|(_, bank_accounts)| bank_accounts.bank.mint).collect_vec().len() as u64,
            total_assets_usd,
            total_liabilities_usd,
        }
    }
}

#[derive(Debug)]
pub struct LendingPoolBankMetrics {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub total_lenders: u64,
    pub total_borrowers: u64,
    pub total_assets_token: f64,
    pub total_liabilities_token: f64,
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
    pub liquidity_vault_balance: f64,
    pub insurance_vault_balance: f64,
    pub fee_vault_balance: f64,
}

impl LendingPoolBankMetrics {
    pub fn new(bank_pk: &Pubkey, snapshot: &Snapshot) -> Self {
        let bank_accounts = snapshot.banks.get(bank_pk).unwrap();

        let total_asset_share = bank_accounts.bank
            .total_asset_shares;
        let total_liability_share = bank_accounts.bank
            .total_liability_shares;
        let price = snapshot.price_feeds.get(&bank_accounts.bank.config.get_pyth_oracle_key()).unwrap().get_price();

        let asset_amount = bank_accounts.bank.get_asset_amount(total_asset_share.into()).unwrap();
        let asset_value_usd = calc_asset_value(asset_amount, price, bank_accounts.bank.mint_decimals, None).unwrap().to_num::<f64>();
        let liability_amount = bank_accounts.bank.get_liability_amount(total_liability_share.into()).unwrap();
        let liability_value_usd = calc_asset_value(liability_amount, price, bank_accounts.bank.mint_decimals, None).unwrap().to_num::<f64>();


        Self {
            pubkey: *bank_pk,
            mint: bank_accounts.bank.mint,
            total_lenders: snapshot.marginfi_accounts.iter().filter(|(_, account)| account.lending_account.balances.iter().any(|a| a.active && I80F48::from(a.asset_shares).gt(&ZERO_AMOUNT_THRESHOLD) && a.bank_pk.eq(bank_pk))).count() as u64,
            total_borrowers: snapshot.marginfi_accounts.iter().filter(|(_, account)| account.lending_account.balances.iter().any(|a| a.active && I80F48::from(a.liability_shares).gt(&ZERO_AMOUNT_THRESHOLD) && a.bank_pk.eq(bank_pk))).count() as u64,
            total_assets_token: asset_amount.to_num::<f64>().div(10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_liabilities_token: liability_amount.to_num::<f64>().div(10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_assets_usd: asset_value_usd,
            total_liabilities_usd: liability_value_usd,
            liquidity_vault_balance: (bank_accounts.liquidity_vault_token_account.amount as f64).div(10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            insurance_vault_balance: (bank_accounts.insurance_vault_token_account.amount as f64).div(10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            fee_vault_balance: (bank_accounts.fee_vault_token_account.amount as f64).div(10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
        }
    }
}

#[derive(Debug)]
pub struct MarginfiAccountMetrics {
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
    pub health: f64,
}

pub async fn push_transactions_to_pubsub(ctx: Arc<Context>) {
    // let topic_name = ctx.config.topic_name.as_str();

    // let project_options = if ctx.config.gcp_sa_key.is_some() {
    //     Some(Project::FromFile(Box::new(
    //         CredentialsFile::new().await.unwrap(),
    //     )))
    // } else {
    //     None
    // };

    // let client = Client::new(ClientConfig {
    //     project_id: Some(ctx.config.project_id.clone()),
    //     project: ProjectOptions::Project(project_options),
    //     ..Default::default()
    // })
    // .await
    // .unwrap();

    // let topic = client.topic(topic_name);
    // topic
    //     .exists(None, None)
    //     .await
    //     .unwrap_or_else(|_| panic!("topic {} not found", topic_name));

    // let publisher = topic.new_publisher(None);

    loop {
        tokio::time::sleep(Duration::from_millis(5000)).await;
        let snapshot = ctx.account_snapshot.lock().await.clone();
        // info!("{snapshot}");
        // let bank_accounts_with_price = BankAccountWithPriceFeed::new(
        //     marginfi_account,
        //     &HashMap::from_iter(snapshot.banks.iter().map(
        //         |(bank_pk, bank_accounts)| (*bank_pk, bank_accounts.clone().bank),
        //     )),
        //     &HashMap::from_iter(snapshot.price_feeds.iter().map(
        //         |(oracle_pk, oracle_data)| match oracle_data {
        //             OracleData::Pyth(price_feed) => {
        //                 (*oracle_pk, *price_feed)
        //             }
        //         },
        //     )),
        // )
        //     .unwrap()
        //     .get_account_health(
        //         RiskRequirementType::Maintenance,
        //         snapshot.clock.unix_timestamp,
        //     )
        //     .unwrap()
        //     .to_num::<f64>();
        let group_metrics = MarginfiGroupMetrics::new(&snapshot);
        let all_bank_metrics = snapshot.banks.iter().map(|(bank_pk, bank_accounts)| LendingPoolBankMetrics::new(bank_pk, &snapshot)).collect_vec();
        //     let group_metrics = MarginfiGroupMetrics {
        //     marginfi_accounts_count: snapshot.marginfi_accounts.len() as u64,
        //     banks_count: snapshot.banks.len() as u64,
        //     bank_mints_count: snapshot
        //         .banks
        //         .iter()
        //         .counts_by(|(_, bank_accounts)| bank_accounts.bank.mint),
        //     price_feeds_count: snapshot.price_feeds.len() as u64,
        //     worst_margin_account_health: snapshot
        //         .marginfi_accounts
        //         .iter()
        //         .map(|(address, marginfi_account)| {
        //             let health = RiskEngine::new(
        //                 marginfi_account,
        //                 &HashMap::from_iter(snapshot.banks.iter().map(
        //                     |(bank_pk, bank_accounts)| (*bank_pk, bank_accounts.clone().bank),
        //                 )),
        //                 &HashMap::from_iter(snapshot.price_feeds.iter().map(
        //                     |(oracle_pk, oracle_data)| match oracle_data {
        //                         OracleData::Pyth(price_feed) => {
        //                             (*oracle_pk, *price_feed)
        //                         }
        //                     },
        //                 )),
        //             )
        //                 .unwrap()
        //                 .get_account_health(
        //                     RiskRequirementType::Maintenance,
        //                     snapshot.clock.unix_timestamp,
        //                 )
        //                 .unwrap()
        //                 .to_num::<f64>();
        //             (*address, health)
        //         })
        //         .sorted_by(|(_, health1), (_, health2)| {
        //             if health1 > health2 {
        //                 std::cmp::Ordering::Greater
        //             } else {
        //                 std::cmp::Ordering::Less
        //             }
        //         })
        //         .take(5)
        //         .collect_vec(),
        // };

        info!("{group_metrics:#?}");
        info!("{all_bank_metrics:#?}");

        // let mut messages = vec![];

        // account_updates_data.iter().for_each(|account_update_data| {
        //     ctx.account_updates_counter.fetch_add(1, Ordering::Relaxed);

        //     let now = Utc::now();

        //     let message = gcp_pubsub::PubsubAccountUpdate {
        //         id: Uuid::new_v4().to_string(),
        //         created_at: now.format(DATE_FORMAT_STR).to_string(),
        //         timestamp: account_update_data
        //             .timestamp
        //             .format(DATE_FORMAT_STR)
        //             .to_string(),
        //         owner: account_update_data.account_data.owner.to_string(),
        //         slot: account_update_data.slot,
        //         pubkey: account_update_data.address.to_string(),
        //         txn_signature: account_update_data.txn_signature.map(|sig| sig.to_string()),
        //         write_version: account_update_data.write_version,
        //         lamports: account_update_data.account_data.lamports,
        //         executable: account_update_data.account_data.executable,
        //         rent_epoch: account_update_data.account_data.rent_epoch,
        //         data: general_purpose::STANDARD.encode(&account_update_data.account_data.data),
        //     };

        //     messages.push(PubsubMessage {
        //         data: serde_json::to_string(&message).unwrap().as_bytes().to_vec(),
        //         ..PubsubMessage::default()
        //     });
        // });

        // // Send a message. There are also `publish_bulk` and `publish_immediately` methods.
        // let awaiters = publisher.publish_bulk(messages).await;

        // // The get method blocks until a server-generated ID or an error is returned for the published message.
        // let pub_results: Vec<Result<String, Status>> = join_all(
        //     awaiters
        //         .into_iter()
        //         .map(|awaiter| awaiter.get(None))
        //         .collect_vec(),
        // )
        // .await;

        // pub_results.into_iter().for_each(|result| match result {
        //     Ok(_) => {}
        //     Err(status) => {
        //         error!(
        //             "Error sending tx to pubsub (code {:?}): {:?}",
        //             status.code(),
        //             status.message()
        //         )
        //     }
        // });
    }
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

        debug!(
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
