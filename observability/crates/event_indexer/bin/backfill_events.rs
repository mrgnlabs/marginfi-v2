use std::{panic, process, str::FromStr, thread::available_parallelism};

use bytemuck::Contiguous;
use dotenv::dotenv;
use envconfig::Envconfig;
use event_indexer::backfiller::{
    crawl_signatures_for_range, find_boundary_signatures_for_range, TransactionData,
    MARGINFI_PROGRAM_GENESIS_SIG,
};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction,
    rpc_config::RpcBlockConfig,
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::Signature,
};
use solana_transaction_status::UiTransactionEncoding;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "RPC_HOST")]
    pub rpc_host: String,
    #[envconfig(from = "RPC_TOKEN")]
    pub rpc_token: String,
    #[envconfig(from = "BEFORE_SIGNATURE")]
    pub before: Option<String>,
    #[envconfig(from = "UNTIL_SIGNATURE")]
    pub until: Option<String>,
    #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    pub gcp_sa_key: String,
    #[envconfig(from = "PRETTY_LOGS")]
    pub pretty_logs: Option<bool>,
}

#[tokio::main]
pub async fn main() {
    dotenv().ok();

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let config = Config::init_from_env().unwrap();

    let pretty_logs = config.pretty_logs.unwrap_or(false);

    let filter = EnvFilter::from_default_env();
    let stackdriver = tracing_stackdriver::layer(); // writes to std::io::Stdout
    let subscriber = tracing_subscriber::registry().with(filter);
    if pretty_logs {
        let subscriber = subscriber.with(tracing_subscriber::fmt::layer().compact());
        tracing::subscriber::set_global_default(subscriber).unwrap();
    } else {
        let subscriber = subscriber.with(stackdriver);
        tracing::subscriber::set_global_default(subscriber).unwrap();
    };

    let rpc_endpoint = format!("{}/{}", config.rpc_host, config.rpc_token).to_string();
    let rpc_client = RpcClient::new_with_commitment(
        rpc_endpoint.clone(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    );

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

    let until_sig = Signature::from_str(
        &config
            .until
            .unwrap_or_else(|| MARGINFI_PROGRAM_GENESIS_SIG.to_string()),
    )
    .unwrap();

    let threads = available_parallelism()
        .map(|c| c.into_integer() as usize)
        .unwrap_or(1);

    let boundary_sigs =
        find_boundary_signatures_for_range(&rpc_client, threads, before_sig, until_sig)
            .await
            .unwrap();

    info!("Boundary signatures: {:?}", boundary_sigs);

    let (transaction_tx, transaction_rx) = crossbeam::channel::unbounded::<TransactionData>();

    let mut tasks = vec![];
    for i in 0..threads {
        let until_sig = boundary_sigs[i];
        let before_sig = boundary_sigs[i + 1];

        let local_transaction_tx = transaction_tx.clone();
        let rpc_client = RpcClient::new_with_commitment(
            rpc_endpoint.clone(),
            CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            },
        );

        info!(
            "Spawning thread for range: {:?}..{:?}",
            until_sig, before_sig
        );

        tasks.push(std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                crawl_signatures_for_range(
                    i as u64,
                    rpc_client,
                    marginfi::ID,
                    before_sig.1,
                    until_sig.1,
                    local_transaction_tx,
                    None,
                )
                .await
            })
        }));
    }

    let mut tx_count = 0;
    while let Ok(TransactionData {
        task_id,
        transaction,
        ..
    }) = transaction_rx.recv()
    {
        tx_count += 1;
        let until_slot = boundary_sigs[task_id as usize].0;
        let before_slot = boundary_sigs[task_id as usize + 1].0;
        println!(
            "[{}] {:.1}% complete (total: {}, remaining ranges: {})",
            task_id,
            (before_slot - transaction.slot) as f64 / (before_slot - until_slot) as f64 * 100.0,
            tx_count,
            tasks.iter().fold(0, |mut acc, task| {
                if !task.is_finished() {
                    acc = acc + 1;
                }
                acc
            })
        );
    }
}
