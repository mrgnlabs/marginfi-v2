use crate::{
    config::GlobalOptions,
    processor,
    profile::{load_profile, Profile},
};
use anchor_client::Cluster;
use anyhow::Result;
use clap::{clap_derive::ArgEnum, Parser};
use fixed::types::I80F48;
use marginfi::{
    prelude::{GroupConfig, MarginfiGroup},
    state::{
        marginfi_account::{Balance, LendingAccount, MarginfiAccount},
        marginfi_group::{
            Bank, BankConfig, BankConfigOpt, BankOperationalState, InterestRateConfig,
            OracleConfig, WrappedI80F48,
        },
    },
};
use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};
use type_layout::TypeLayout;

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
    Bank {
        #[clap(subcommand)]
        subcmd: BankCommand,
    },
    Profile {
        #[clap(subcommand)]
        subcmd: ProfileCommand,
    },
    InspectPadding {},
    Account {
        #[clap(subcommand)]
        subcmd: AccountCommand,
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
        #[clap(short = 'f', long = "override")]
        override_existing_profile_group: bool,
    },
    Update {
        admin: Option<Pubkey>,
    },
    AddBank {
        #[clap(long)]
        mint: Pubkey,
        #[clap(long)]
        asset_weight_init: f64,
        #[clap(long)]
        asset_weight_maint: f64,
        #[clap(long)]
        liability_weight_init: f64,
        #[clap(long)]
        liability_weight_maint: f64,
        #[clap(long)]
        max_capacity: u64,
        #[clap(long)]
        pyth_oracle: Pubkey,
        #[clap(long)]
        optimal_utilization_rate: f64,
        #[clap(long)]
        plateau_interest_rate: f64,
        #[clap(long)]
        max_interest_rate: f64,
        #[clap(long)]
        insurance_fee_fixed_apr: f64,
        #[clap(long)]
        insurance_ir_fee: f64,
        #[clap(long)]
        protocol_fixed_fee_apr: f64,
        #[clap(long)]
        protocol_ir_fee: f64,
    },
}

#[derive(Clone, Copy, Debug, Parser, ArgEnum)]
pub enum BankOperationalStateArg {
    Paused,
    Operational,
    ReduceOnly,
}

impl Into<BankOperationalState> for BankOperationalStateArg {
    fn into(self) -> BankOperationalState {
        match self {
            BankOperationalStateArg::Paused => BankOperationalState::Paused,
            BankOperationalStateArg::Operational => BankOperationalState::Operational,
            BankOperationalStateArg::ReduceOnly => BankOperationalState::ReduceOnly,
        }
    }
}

#[derive(Debug, Parser)]
pub enum BankCommand {
    Get {
        bank: Option<Pubkey>,
    },
    GetAll {
        marginfi_group: Option<Pubkey>,
    },
    Update {
        bank_pk: Pubkey,
        #[clap(long)]
        asset_weight_init: Option<f32>,
        #[clap(long)]
        asset_weight_maint: Option<f32>,

        #[clap(long)]
        liability_weight_init: Option<f32>,
        #[clap(long)]
        liability_weight_maint: Option<f32>,

        #[clap(long)]
        max_capacity: Option<u64>,
        #[clap(long, arg_enum)]
        operational_state: Option<BankOperationalStateArg>,
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
        name: String,
    },
    Update {
        name: String,
        #[clap(long)]
        cluster: Option<Cluster>,
        #[clap(long)]
        keypair_path: Option<String>,
        #[clap(long)]
        rpc_url: Option<String>,
        #[clap(long)]
        program_id: Option<Pubkey>,
        #[clap(long)]
        commitment: Option<CommitmentLevel>,
        #[clap(long)]
        group: Option<Pubkey>,
    },
}

#[derive(Debug, Parser)]
pub enum AccountCommand {
    List,
    Use { account: Pubkey },
    Get,
    Deposit { bank: Pubkey, ui_amount: f64 },
    Withdraw { bank: Pubkey, ui_amount: f64 },
}

pub fn entry(opts: Opts) -> Result<()> {
    env_logger::init();

    match opts.command {
        Command::Group { subcmd } => group(subcmd, &opts.cfg_override),
        Command::Bank { subcmd } => bank(subcmd, &opts.cfg_override),
        Command::Profile { subcmd } => profile(subcmd),
        Command::InspectPadding {} => inspect_padding(),
        Command::Account { subcmd } => process_account_subcmd(subcmd, &opts.cfg_override),
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
        ProfileCommand::Update {
            cluster,
            keypair_path,
            rpc_url,
            program_id,
            commitment,
            group,
            name,
        } => processor::configure_profile(
            name,
            cluster,
            keypair_path,
            rpc_url,
            program_id,
            commitment,
            group,
        ),
    }
}

fn group(subcmd: GroupCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

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
        GroupCommand::Create {
            admin,
            override_existing_profile_group,
        } => processor::group_create(config, profile, admin, override_existing_profile_group),
        GroupCommand::Update { admin } => processor::group_configure(config, profile, admin),
        GroupCommand::AddBank {
            mint: bank_mint,
            asset_weight_init,
            asset_weight_maint,
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
            asset_weight_init,
            asset_weight_maint,
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

fn bank(subcmd: BankCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    match subcmd {
        BankCommand::Get { bank: _ } => (),
        BankCommand::GetAll { marginfi_group: _ } => (),
        _ => get_consent(&subcmd, &profile)?,
    }

    match subcmd {
        BankCommand::Get { bank } => processor::bank_get(config, bank),
        BankCommand::GetAll { marginfi_group } => processor::bank_get_all(config, marginfi_group),
        BankCommand::Update {
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            max_capacity,
            operational_state,
            bank_pk,
        } => processor::bank_configure(
            config,
            profile, //
            bank_pk,
            BankConfigOpt {
                asset_weight_init: asset_weight_init.map(|x| I80F48::from_num(x).into()),
                asset_weight_maint: asset_weight_maint.map(|x| I80F48::from_num(x).into()),
                liability_weight_init: liability_weight_init.map(|x| I80F48::from_num(x).into()),
                liability_weight_maint: liability_weight_maint.map(|x| I80F48::from_num(x).into()),
                max_capacity,
                operational_state: operational_state.map(|x| x.into()),
                oracle: None,
            },
        ),
    }
}

fn inspect_padding() -> Result<()> {
    println!("MarginfiGroup: {}", MarginfiGroup::type_layout());
    println!("GroupConfig: {}", GroupConfig::type_layout());
    println!("InterestRateConfig: {}", InterestRateConfig::type_layout());
    println!("Bank: {}", Bank::type_layout());
    println!("BankConfig: {}", BankConfig::type_layout());
    println!("OracleConfig: {}", OracleConfig::type_layout());
    println!("BankConfigOpt: {}", BankConfigOpt::type_layout());
    println!("WrappedI80F48: {}", WrappedI80F48::type_layout());

    println!("MarginfiAccount: {}", MarginfiAccount::type_layout());
    println!("LendingAccount: {}", LendingAccount::type_layout());
    println!("Balance: {}", Balance::type_layout());

    Ok(())
}

fn process_account_subcmd(subcmd: AccountCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    match subcmd {
        AccountCommand::List => processor::marginfi_account_list(profile, &config),
        AccountCommand::Use { account } => todo!(),
        AccountCommand::Get => todo!(),
        AccountCommand::Deposit { bank, ui_amount } => todo!(),
        AccountCommand::Withdraw { bank, ui_amount } => todo!(),
    }?;

    Ok(())
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
