use crate::commands::{
    backfill::{backfill, BackfillConfig},
    create_table::{create_table, CreateTableConfig},
    forwardfill::{forwardfill, ForwardfillConfig},
};
use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use envconfig::Envconfig;
use log::debug;
use std::{panic, process};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
pub struct GlobalOptions {}

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
    CreateTable,
    Backfill,
    Forwardfill,
}

#[tokio::main]
pub async fn entry(opts: Opts) -> Result<()> {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    dotenv().ok();
    env_logger::init();

    match opts.command {
        Command::CreateTable => {
            let config = CreateTableConfig::init_from_env().unwrap();
            debug!("Config -> {:#?}", &config.clone());

            create_table(config).await
        }
        Command::Backfill => {
            let config = BackfillConfig::init_from_env().unwrap();
            debug!("Config -> {:#?}", &config.clone());

            backfill(config).await
        }
        Command::Forwardfill => {
            let config = ForwardfillConfig::init_from_env().unwrap();
            debug!("Config -> {:#?}", &config.clone());

            forwardfill(config).await
        }
    }
}
