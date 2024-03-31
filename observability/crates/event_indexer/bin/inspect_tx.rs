use std::{panic, process, str::FromStr};

use chrono::Utc;
use dotenv::dotenv;
use envconfig::Envconfig;
use event_indexer::{
    error::IndexingError,
    parser::{MarginfiEventParser, MARGINFI_GROUP_ADDRESS},
};
use rpc_utils::conversion::convert_encoded_ui_transaction;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "PRETTY_LOGS")]
    pub pretty_logs: Option<bool>,
}

#[tokio::main]
pub async fn main() -> Result<(), IndexingError> {
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

    let args: Vec<String> = env::args().collect();
    let tx_signature = args.get(1).expect("Missing transaction signature argument");

    let rpc_client =
        RpcClient::new("https://mrgn.rpcpool.com/c293bade994b3864b52c6bbbba4b".to_string());

    let event_parser = MarginfiEventParser::new(marginfi::ID, MARGINFI_GROUP_ADDRESS);

    let signature = Signature::from_str(&tx_signature).unwrap();

    let encoded_tx = rpc_client
        .get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                max_supported_transaction_version: Some(0),
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await
        .unwrap();

    let versioned_tx_with_meta = convert_encoded_ui_transaction(encoded_tx.transaction).unwrap();

    let events = event_parser.extract_events(
        Utc::now().timestamp(),
        encoded_tx.slot,
        versioned_tx_with_meta,
    );

    if events.is_empty() {
        println!("No event detected");
    } else {
        println!("Events detected: {:#?}", events);
    }

    Ok(())
}
