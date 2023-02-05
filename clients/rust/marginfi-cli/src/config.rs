use anchor_client::{Client, Cluster, Program};
use clap::Parser;
use serde::{Deserialize, Serialize};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};
use std::str::FromStr;

#[derive(Default, Debug, Parser)]
pub struct GlobalOptions {
    // /// Cluster override.
    // #[clap(global = true, long = "cluster")]
    // pub cluster: Option<Cluster>,
    // /// Wallet override.
    // #[clap(global = true, long = "wallet")]
    // pub wallet: Option<WalletPath>,
    // /// Program ID override.
    // #[clap(global = true, long = "pid")]
    // pub pid: Option<Pubkey>,
    // /// Commitment.
    // #[clap(global = true, long = "commitment")]
    // pub commitment: Option<CommitmentLevel>,
    /// Dry run for any transactions involved.
    #[clap(global = true, long = "dry-run", action, default_value_t = false)]
    pub dry_run: bool,

    #[clap(
        global = true,
        long = "skip-confimation",
        short = 'y',
        action,
        default_value_t = false
    )]
    pub skip_confirmation: bool,
}

pub struct Config {
    pub cluster: Cluster,
    pub payer: Keypair,
    pub program_id: Pubkey,
    pub commitment: CommitmentConfig,
    pub dry_run: bool,
    pub client: Client,
    pub program: Program,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    // Base58 pubkey string.
    pub address: String,
    // Name of JSON file containing the account data.
    pub filename: String,
}

crate::home_path!(WalletPath, ".config/solana/id.json");
