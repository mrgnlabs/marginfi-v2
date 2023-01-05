use crate::{
    config::GlobalOptions,
    processor,
    profile::{load_profile, Profile},
};
use anchor_client::Cluster;
use anyhow::Result;
use clap::Parser;
use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[clap(version = VERSION)]
pub struct Opts {
    #[clap(flatten)]
    pub cfg_override: GlobalOptions,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    Group {
        #[clap(subcommand)]
        subcmd: GroupCommand,
    },
    Profile {
        #[clap(subcommand)]
        subcmd: ProfileCommand,
    },
}

#[derive(Debug, Parser)]
pub enum GroupCommand {
    Get {
        marginfi_group: Option<Pubkey>,
    },
    GetAll {},
    Create {
        admin: Option<Pubkey>,
    },
    Configure {
        admin: Option<Pubkey>,
    },
    AddBank {
        bank_mint: Pubkey,
        deposit_weight_init: f64,
        deposit_weight_maint: f64,
        liability_weight_init: f64,
        liability_weight_maint: f64,
        max_capacity: u64,
        pyth_oracle: Pubkey,
        optimal_utilization_rate: f64,
        plateau_interest_rate: f64,
        max_interest_rate: f64,
        insurance_fee_fixed_apr: f64,
        insurance_ir_fee: f64,
        protocol_fixed_fee_apr: f64,
        protocol_ir_fee: f64,
    },
}

#[derive(Debug, Parser)]
pub enum ProfileCommand {
    Create {
        #[clap(long)]
        name: String,
        #[clap(long)]
        cluster: Cluster,
        #[clap(long)]
        keypair_path: String,
        #[clap(long)]
        rpc_url: String,
        #[clap(long)]
        program_id: Option<Pubkey>,
        #[clap(long)]
        commitment: Option<CommitmentLevel>,
        #[clap(long)]
        group: Option<Pubkey>,
    },
    Show,
    List,
    Set {
        #[clap(long)]
        name: String,
    },
}

pub fn entry(opts: Opts) -> Result<()> {
    env_logger::init();

    match opts.command {
        Command::Group { subcmd } => group(subcmd, &opts.cfg_override),
        Command::Profile { subcmd } => profile(subcmd),
    }
}

fn profile(subcmd: ProfileCommand) -> Result<()> {
    match subcmd {
        ProfileCommand::Create {
            name,
            cluster,
            keypair_path,
            rpc_url,
            program_id,
            commitment,
            group,
        } => processor::create_profile(
            name,
            cluster,
            keypair_path,
            rpc_url,
            program_id,
            commitment,
            group,
        ),
        ProfileCommand::Show => processor::show_profile(),
        ProfileCommand::List => processor::list_profiles(),
        ProfileCommand::Set { name } => processor::set_profile(name),
    }
}

fn group(subcmd: GroupCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.config(Some(global_options))?;

    match subcmd {
        GroupCommand::Get { marginfi_group: _ } => (),
        GroupCommand::GetAll {} => (),
        _ => get_consent(&subcmd, &profile)?,
    }

    match subcmd {
        GroupCommand::Get { marginfi_group } => {
            processor::group_get(config, marginfi_group.or(profile.marginfi_group))
        }
        GroupCommand::GetAll {} => processor::group_get_all(config),
        GroupCommand::Create { admin } => processor::group_create(config, profile, admin),
        GroupCommand::Configure { admin } => processor::group_configure(config, profile, admin),
        GroupCommand::AddBank {
            bank_mint,
            deposit_weight_init,
            deposit_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            max_capacity,
            pyth_oracle,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            protocol_fixed_fee_apr,
            protocol_ir_fee,
        } => processor::group_add_bank(
            config,
            profile,
            bank_mint,
            pyth_oracle,
            deposit_weight_init,
            deposit_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            max_capacity,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            protocol_fixed_fee_apr,
            protocol_ir_fee,
        ),
    }
}

fn get_consent<T: std::fmt::Debug>(cmd: T, profile: &Profile) -> Result<()> {
    let mut input = String::new();
    println!("Command: {:#?}", cmd);
    println!("{:#?}", profile);
    println!(
        "Type the name of the profile [{}] to continue",
        profile.name.clone()
    );
    std::io::stdin().read_line(&mut input)?;
    if input.trim() != profile.name {
        println!("Aborting");
        std::process::exit(1);
    }

    Ok(())
}
