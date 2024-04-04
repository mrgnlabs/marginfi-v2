use dotenv::dotenv;
use envconfig::Envconfig;
use solana_price_indexer::{error::IndexingError, indexer::SolanaPriceIndexer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use std::{panic, process};

#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "RPC_HOST")]
    pub rpc_host: String,
    #[envconfig(from = "RPC_TOKEN")]
    pub rpc_token: String,
    #[envconfig(from = "MONITOR_INTERVAL")]
    pub monitor_interval: u64,
    #[envconfig(from = "PRETTY_LOGS")]
    pub pretty_logs: Option<bool>,
    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,
    // #[envconfig(from = "INDEX_TRANSACTIONS_PROJECT_ID")]
    // pub project_id: String,
    // #[envconfig(from = "INDEX_TRANSACTIONS_PUBSUB_TOPIC_NAME")]
    // pub topic_name: String,
    // #[envconfig(from = "GOOGLE_APPLICATION_CREDENTIALS_JSON")]
    // pub gcp_sa_key: Option<String>,
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

    let mut indexer = SolanaPriceIndexer::new();

    indexer.run().await;

    Ok(())
}
