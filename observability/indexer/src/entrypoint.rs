use crate::commands::create_table::TableType;
use crate::commands::inspect_tx::inspect_tx;
use crate::commands::snapshot_accounts::{snapshot_accounts, SnapshotAccountsConfig};
use crate::commands::{
    backfill_events::{backfill_events, BackfillEventsConfig},
    create_table::create_table,
    index_events::{index_events, IndexEventsConfig},
};
use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use envconfig::Envconfig;
use std::{panic, process};
use tracing::debug;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
pub struct GlobalOptions {
    #[clap(long)]
    pub pretty_log: bool,
}

#[derive(Debug, Parser)]
#[clap(version = VERSION)]
pub struct Opts {
    #[clap(flatten)]
    pub global_config: GlobalOptions,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    CreateTable {
        #[clap(long)]
        table_type: TableType,
        #[clap(long)]
        project_id: String,
        #[clap(long)]
        dataset_id: String,
        #[clap(long)]
        table_id: String,
        #[clap(long)]
        table_friendly_name: Option<String>,
        #[clap(long)]
        table_description: Option<String>,
    },
    SnapshotAccounts,
}

pub fn entry(opts: Opts) -> Result<()> {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    dotenv().ok();

    let filter = EnvFilter::from_default_env();
    let stackdriver = tracing_stackdriver::layer(); // writes to std::io::Stdout
    let subscriber = tracing_subscriber::registry().with(filter);
    if opts.global_config.pretty_log {
        let subscriber = subscriber.with(tracing_subscriber::fmt::layer().compact());
        tracing::subscriber::set_global_default(subscriber).unwrap();
    } else {
        let subscriber = subscriber.with(stackdriver);
        tracing::subscriber::set_global_default(subscriber).unwrap();
    };

    let rt = tokio::runtime::Runtime::new().unwrap();

    match opts.command {
        Command::CreateTable {
            project_id,
            dataset_id,
            table_type,
            table_id,
            table_friendly_name,
            table_description,
        } => rt.block_on(async {
            create_table(
                project_id,
                dataset_id,
                table_id,
                table_type,
                table_friendly_name,
                table_description,
            )
            .await
        }),
        Command::SnapshotAccounts => {
            let config = SnapshotAccountsConfig::init_from_env().unwrap();
            debug!("Config -> {:#?}", &config.clone());

            rt.block_on(async { snapshot_accounts(config).await })
        }
    }
}
