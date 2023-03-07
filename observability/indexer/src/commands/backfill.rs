use crate::{
    common::Target,
    utils::{
        big_query::DATE_FORMAT_STR,
        protos::gcp_pubsub,
        transactions_crawler::{
            TransactionsCrawler, TransactionsCrawlerConfig, TransactionsCrawlerContext,
        },
    },
};
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use chrono::{NaiveDateTime, Utc};
use envconfig::Envconfig;
use futures::future::join_all;
use google_cloud_default::WithAuthExt;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use itertools::Itertools;
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::TransactionVersion};
use std::{str::FromStr, sync::Arc, time::Duration};
use tracing::error;
use uuid::Uuid;

#[derive(Envconfig, Debug, Clone)]
pub struct BackfillConfig {
    #[envconfig(from = "BACKFILL_RPC_ENDPOINT")]
    pub rpc_endpoint: String,
    #[envconfig(from = "BACKFILL_SIGNATURE_FETCH_LIMIT")]
    pub signature_fetch_limit: usize,
    #[envconfig(from = "BACKFILL_MAX_CONCURRENT_REQUESTS")]
    pub max_concurrent_requests: usize,
    #[envconfig(from = "BACKFILL_MAX_PENDING_SIGNATURES")]
    pub max_pending_signatures: usize,
    #[envconfig(from = "BACKFILL_MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "BACKFILL_PROGRAM_ID")]
    pub program_id: Pubkey,
    #[envconfig(from = "BACKFILL_BEFORE_SIGNATURE")]
    pub before: Option<String>,
    #[envconfig(from = "BACKFILL_UNTIL_SIGNATURE")]
    pub until: Option<String>,

    #[envconfig(from = "BACKFILL_PROJECT_ID")]
    pub project_id: String,
    #[envconfig(from = "BACKFILL_PUBSUB_TOPIC_NAME")]
    pub topic_name: String,
    #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    pub gcp_sa_key: String,
}

pub async fn backfill(config: BackfillConfig) -> Result<()> {
    let config_clone = config.clone();

    let tx_crawler = TransactionsCrawler::new_with_config(TransactionsCrawlerConfig {
        rpc_endpoint: config_clone.rpc_endpoint,
        signature_fetch_limit: config_clone.signature_fetch_limit,
        max_concurrent_requests: config_clone.max_concurrent_requests,
        max_pending_signatures: config_clone.max_pending_signatures,
        monitor_interval: config_clone.monitor_interval,
        targets: [Target {
            before: config_clone
                .before
                .map(|sig_str| Signature::from_str(&sig_str).unwrap()),
            until: config_clone
                .until
                .map(|sig_str| Signature::from_str(&sig_str).unwrap()),
            address: config_clone.program_id,
        }]
        .to_vec(),
    });

    let transaction_processor = |ctx: Arc<TransactionsCrawlerContext>| async move {
        push_transactions_to_pubsub(ctx, config).await.unwrap()
    };

    tx_crawler.run_async(&transaction_processor).await.unwrap();

    Ok(())
}

pub async fn push_transactions_to_pubsub(
    ctx: Arc<TransactionsCrawlerContext>,
    config: BackfillConfig,
) -> Result<()> {
    let topic_name = config.topic_name.as_str();

    let client_config = ClientConfig::default().with_auth().await?;
    let client = Client::new(client_config).await?;

    let topic = client.topic(topic_name);
    topic
        .exists(None, None)
        .await
        .unwrap_or_else(|_| panic!("topic {} not found", topic_name));

    let publisher = topic.new_publisher(None);

    loop {
        let mut transactions_data = vec![];
        {
            let signatures_queue = ctx.transaction_queue.lock().unwrap();
            while !signatures_queue.is_empty() {
                transactions_data.push(signatures_queue.pop().unwrap());
            }
        }
        if transactions_data.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        let mut messages = vec![];

        transactions_data.iter().for_each(|transaction_data| {
            let now = Utc::now();

            let tx_with_meta = &transaction_data.transaction.transaction;
            let tx_decoded = tx_with_meta.transaction.decode().unwrap();

            // println!(
            //     "{:?} - {}",
            //     transaction_data.indexing_address,
            //     tx_decoded.signatures.first().unwrap().to_string(),
            // );

            let message = gcp_pubsub::PubsubTransaction {
                id: Uuid::new_v4().to_string(),
                created_at: now.format(DATE_FORMAT_STR).to_string(),
                timestamp: NaiveDateTime::from_timestamp_opt(
                    transaction_data.transaction.block_time.unwrap_or(0),
                    0,
                )
                .unwrap()
                .format(DATE_FORMAT_STR)
                .to_string(),
                signature: tx_decoded.signatures.first().unwrap().to_string(),
                indexing_address: transaction_data.indexing_address.to_string(),
                slot: transaction_data.transaction.slot,
                signer: tx_decoded
                    .message
                    .static_account_keys()
                    .first()
                    .unwrap()
                    .to_string(),
                success: tx_with_meta.meta.clone().unwrap().err.is_none(),
                version: tx_with_meta
                    .version
                    .clone()
                    .map(|v| match v {
                        TransactionVersion::Legacy(_) => "legacy".to_string(),
                        TransactionVersion::Number(version) => version.to_string(),
                    })
                    .unwrap_or_else(|| "unknown".to_string()),
                fee: tx_with_meta.meta.clone().map(|meta| meta.fee).unwrap_or(0),
                meta: tx_with_meta
                    .meta
                    .clone()
                    .map(|meta| serde_json::to_string(&meta).unwrap())
                    .unwrap(),
                message: general_purpose::STANDARD.encode(tx_decoded.message.serialize()),
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
        let pub_results = join_all(
            awaiters
                .into_iter()
                .map(|awaiter| awaiter.get(None))
                .collect_vec(),
        )
        .await;

        pub_results.into_iter().for_each(|result| match result {
            Ok(_) => {}
            Err(err) => {
                error!("Error sending to pubsub: {:?}", err.message()) // TODO: retry logic
            }
        });
    }
}
