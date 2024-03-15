use std::{
    collections::{BTreeMap, HashMap},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
    vec,
};

use chrono::DateTime;
use crossbeam::channel::{Receiver, Sender};
use diesel::{PgConnection, RunQueryDsl};
use futures::StreamExt;
use solana_client::rpc_client::SerializableTransaction;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{TransactionWithStatusMeta, VersionedTransactionWithStatusMeta};
use tracing::{debug, error, info, warn};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    convert_from,
    geyser::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequestFilterBlocksMeta,
        SubscribeRequestFilterTransactions,
    },
};

use crate::{
    db::{
        establish_connection,
        models::{Accounts, CreateAccountEvents, Users},
        schema::*,
    },
    parser::Event,
};

use super::parser::{MarginfiEventParser, MarginfiEventWithMeta, MARGINFI_GROUP_ADDRESS};

const BLOCK_META_BUFFER_LENGTH: usize = 30;

pub struct EventIndexer {
    parser: MarginfiEventParser,
    transaction_rx: Receiver<TransactionUpdate>,
    event_tx: Sender<Vec<MarginfiEventWithMeta>>,
}

impl EventIndexer {
    pub fn new(rpc_host: String, rpc_auth_token: String, database_connection_url: String) -> Self {
        let program_id = marginfi::ID;

        let parser = MarginfiEventParser::new(program_id, MARGINFI_GROUP_ADDRESS);

        let (transaction_tx, transaction_rx) = crossbeam::channel::unbounded::<TransactionUpdate>();
        let (event_tx, event_rx) = crossbeam::channel::unbounded::<Vec<MarginfiEventWithMeta>>();

        tokio::spawn(async move {
            listen_to_updates(rpc_host, rpc_auth_token, program_id, transaction_tx).await
        });

        let mut db_connection = establish_connection(database_connection_url);

        tokio::spawn(async move { store_events(&mut db_connection, event_rx).await });

        Self {
            parser,
            transaction_rx,
            event_tx,
        }
    }

    pub async fn init(&mut self) -> Signature {
        self.process_first_tx().await
    }

    async fn process_first_tx(&mut self) -> Signature {
        loop {
            while let Ok(TransactionUpdate {
                transaction,
                slot,
                timestamp,
            }) = self.transaction_rx.try_recv()
            {
                let signature = transaction.transaction.get_signature().clone();
                let events = self.parser.extract_events(timestamp, slot, transaction);
                self.event_tx.send(events).unwrap();
                return signature;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub async fn run(&mut self) {
        loop {
            while let Ok(TransactionUpdate {
                slot,
                timestamp,
                transaction,
            }) = self.transaction_rx.try_recv()
            {
                let events = self.parser.extract_events(timestamp, slot, transaction);
                self.event_tx.send(events).unwrap();
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}

pub struct TransactionUpdate {
    pub slot: u64,
    pub timestamp: i64,
    pub transaction: VersionedTransactionWithStatusMeta,
}

async fn listen_to_updates(
    rpc_host: String,
    rpc_auth_token: String,
    program_id: Pubkey,
    transaction_tx: Sender<TransactionUpdate>,
) {
    loop {
        info!("Connecting geyser client");
        let geyser_client_connection_result = GeyserGrpcClient::connect_with_timeout(
            rpc_host.to_owned(),
            Some(&rpc_auth_token),
            None,
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(10)),
            false,
        )
        .await;
        info!("Connected");

        let mut geyser_client = match geyser_client_connection_result {
            Ok(geyser_client) => geyser_client,
            Err(err) => {
                error!("Error connecting to geyser client: {}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Subscription config
        let blocks_meta_sub = HashMap::from_iter([(
            "client".to_string(),
            SubscribeRequestFilterBlocksMeta::default(),
        )]);
        let transactions_sub = HashMap::from_iter([(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                account_include: vec![program_id.to_string()],
                account_exclude: vec![],
                ..Default::default()
            },
        )]);
        let commitment_sub = Some(CommitmentLevel::Confirmed);

        let mut transaction_rx = match geyser_client
            .subscribe_once(
                HashMap::default(),
                HashMap::default(),
                transactions_sub,
                HashMap::default(),
                HashMap::default(),
                blocks_meta_sub,
                commitment_sub,
                vec![],
                None,
            )
            .await
        {
            Ok(value) => value,
            Err(e) => {
                error!("Error subscribing geyser client {e}");
                continue;
            }
        };

        let mut tx_buffer: BTreeMap<u64, Vec<VersionedTransactionWithStatusMeta>> = BTreeMap::new(); // We use this to avoid having to handling associating a timestamp to a tx in the main loop
        let mut latest_blocks: BTreeMap<u64, (u64, i64)> = BTreeMap::new();

        while let Some(received) = transaction_rx.next().await {
            match received {
                Ok(received) => {
                    if let Some(update) = received.update_oneof {
                        match update {
                            UpdateOneof::BlockMeta(block_meta) => {
                                let timestamp = block_meta
                                    .block_time
                                    .map(|unix_timestamp| unix_timestamp.timestamp)
                                    .unwrap_or_else(|| {
                                        warn!(
                                            "No block time found in block_meta, using local clock"
                                        );
                                        SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                            as i64
                                    });

                                latest_blocks.insert(block_meta.slot, (block_meta.slot, timestamp));
                                if latest_blocks.len() > BLOCK_META_BUFFER_LENGTH {
                                    latest_blocks.pop_first();
                                }

                                if let Some(txs) = tx_buffer.remove(&block_meta.slot) {
                                    for tx in txs {
                                        transaction_tx
                                            .send(TransactionUpdate {
                                                slot: block_meta.slot,
                                                timestamp,
                                                transaction: tx,
                                            })
                                            .unwrap();
                                    }
                                }
                            }
                            UpdateOneof::Transaction(tx_update) => {
                                if let Some(tx) = tx_update.transaction {
                                    let transaction_with_meta =
                                        convert_from::create_tx_with_meta(tx).unwrap();

                                    let TransactionWithStatusMeta::Complete(
                                        versioned_transaction_with_meta,
                                    ) = transaction_with_meta
                                    else {
                                        error!(
                                            "Discarding tx {:?} because mssing metadata",
                                            transaction_with_meta.transaction_signature()
                                        );
                                        continue;
                                    };

                                    let maybe_block_meta =
                                        latest_blocks.get(&tx_update.slot).copied();
                                    if let Some((slot, timestamp)) = maybe_block_meta {
                                        transaction_tx
                                            .send(TransactionUpdate {
                                                slot,
                                                timestamp,
                                                transaction: versioned_transaction_with_meta,
                                            })
                                            .unwrap();
                                    } else {
                                        tx_buffer
                                            .entry(tx_update.slot)
                                            .or_default()
                                            .push(versioned_transaction_with_meta);
                                    }
                                }
                            }
                            _ => {}
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
    }
}

async fn store_events(
    db_connection: &mut PgConnection,
    event_rx: Receiver<Vec<MarginfiEventWithMeta>>,
) {
    loop {
        while let Ok(events) = event_rx.try_recv() {
            if !events.is_empty() {
                for MarginfiEventWithMeta {
                    event,
                    timestamp,
                    in_flashloan,
                    call_stack,
                    tx_sig,
                } in events
                {
                    let timestamp = DateTime::from_timestamp(timestamp, 0).unwrap().naive_utc();
                    let tx_sig = tx_sig.to_string();
                    let call_stack = serde_json::to_string(&call_stack).unwrap();

                    match event {
                        Event::CreateAccount(e) => {
                            let authority_id = diesel::insert_into(users::table)
                                .values(vec![Users {
                                    address: e.authority.to_string(),
                                    ..Default::default()
                                }])
                                .on_conflict(users::id)
                                .do_nothing()
                                .returning(users::id)
                                .get_result(db_connection)
                                .unwrap();

                            let account_id = diesel::insert_into(accounts::table)
                                .values(vec![Accounts {
                                    address: e.account.to_string(),
                                    user_id: authority_id,
                                    ..Default::default()
                                }])
                                .on_conflict(accounts::id)
                                .do_nothing()
                                .returning(accounts::id)
                                .get_result(db_connection)
                                .unwrap();

                            let create_account_event = CreateAccountEvents {
                                timestamp,
                                authority_id,
                                tx_sig,
                                call_stack,
                                in_flashloan,
                                account_id,
                                ..Default::default()
                            };

                            diesel::insert_into(create_account_events::table)
                                .values(&create_account_event)
                                .execute(db_connection)
                                .unwrap();
                        }
                        _ => {
                            debug!("Unsupported event: {:?}", event);
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}
