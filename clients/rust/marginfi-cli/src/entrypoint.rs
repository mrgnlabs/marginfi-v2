use crate::processor::oracle::find_pyth_push_oracles_for_feed_id;
use crate::{
    config::GlobalOptions,
    processor::{self, process_set_user_flag},
    profile::{load_profile, Profile},
};
use anchor_client::Cluster;
use anyhow::Result;
use clap::{clap_derive::ArgEnum, Parser};
use fixed::types::I80F48;
use marginfi::state::marginfi_account::TRANSFER_AUTHORITY_ALLOWED_FLAG;
use marginfi::{
    prelude::*,
    state::{
        bank::{Bank, BankConfig, BankConfigOpt},
        interest_rate::{InterestRateConfig, InterestRateConfigOpt},
        marginfi_account::{Balance, LendingAccount, MarginfiAccount, FLASHLOAN_ENABLED_FLAG},
        marginfi_group::{BankOperationalState, OracleConfig, RiskTier, WrappedI80F48},
        price::OracleSetup,
    },
};
use pyth_solana_receiver_sdk::price_update::get_feed_id_from_hex;
use rand::Rng;
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

    PatchIdl {
        idl_path: String,
    },

    InspectSize {},

    MakeTestI80F48,
    Account {
        #[clap(subcommand)]
        subcmd: AccountCommand,
    },
    #[cfg(feature = "lip")]
    Lip {
        #[clap(subcommand)]
        subcmd: LipCommand,
    },
    //
    // InspectSwitchboardFeed { switchboard_feed: Pubkey },
    ShowOracleAges {
        #[clap(long, action)]
        only_stale: bool,
    },
    InspectPythPushOracleFeed {
        pyth_feed: Pubkey,
    },
    FindPythPull {
        feed_id: String,
    },
    InspectSwbPullFeed {
        address: Pubkey,
    },
}

#[allow(clippy::large_enum_variant)]
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
        /// Generates a PDA for the bank key
        #[clap(long, action)]
        seed: bool,
        #[clap(long)]
        asset_weight_init: f64,
        #[clap(long)]
        asset_weight_maint: f64,
        #[clap(long)]
        liability_weight_init: f64,
        #[clap(long)]
        liability_weight_maint: f64,
        #[clap(long)]
        deposit_limit_ui: u64,
        #[clap(long)]
        borrow_limit_ui: u64,
        #[clap(long)]
        oracle_key: Pubkey,
        #[clap(long)]
        feed_id: Option<Pubkey>,
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
        group_fixed_fee_apr: f64,
        #[clap(long)]
        group_ir_fee: f64,
        #[clap(long, arg_enum)]
        risk_tier: RiskTierArg,
        #[clap(long, arg_enum)]
        oracle_type: OracleTypeArg,
        #[clap(
            long,
            help = "Max oracle age in seconds, 0 for default (60s)",
            default_value = "60"
        )]
        oracle_max_age: u16,
        #[clap(long)]
        global_fee_wallet: Pubkey,
    },
    HandleBankruptcy {
        accounts: Vec<Pubkey>,
    },
    UpdateLookupTable {
        #[clap(short = 't', long)]
        existing_token_lookup_tables: Vec<Pubkey>,
    },
    CheckLookupTable {
        #[clap(short = 't', long)]
        existing_token_lookup_tables: Vec<Pubkey>,
    },
    InitFeeState {
        #[clap(long)]
        admin: Pubkey,
        #[clap(long)]
        fee_wallet: Pubkey,
        #[clap(long)]
        bank_init_flat_sol_fee: u32,
        #[clap(long)]
        program_fee_fixed: f64,
        #[clap(long)]
        program_fee_rate: f64,
    },
    EditFeeState {
        #[clap(long)]
        fee_wallet: Pubkey,
        #[clap(long)]
        bank_init_flat_sol_fee: u32,
        #[clap(long)]
        program_fee_fixed: f64,
        #[clap(long)]
        program_fee_rate: f64,
    },
    ConfigGroupFee {
        #[clap(long)]
        flag: u64,
    },
    PropagateFee {
        #[clap(long)]
        marginfi_group: Pubkey,
    },
}

#[derive(Clone, Copy, Debug, Parser, ArgEnum)]
pub enum RiskTierArg {
    Collateral,
    Isolated,
}

impl From<RiskTierArg> for RiskTier {
    fn from(value: RiskTierArg) -> Self {
        match value {
            RiskTierArg::Collateral => RiskTier::Collateral,
            RiskTierArg::Isolated => RiskTier::Isolated,
        }
    }
}

#[derive(Clone, Copy, Debug, Parser, ArgEnum)]
pub enum OracleTypeArg {
    PythLegacy,
    SwitchboardLegacy,
    PythPushOracle,
    SwitchboardPull,
}

impl From<OracleTypeArg> for OracleSetup {
    fn from(value: OracleTypeArg) -> Self {
        match value {
            OracleTypeArg::PythLegacy => OracleSetup::PythLegacy,
            OracleTypeArg::SwitchboardLegacy => OracleSetup::SwitchboardV2,
            OracleTypeArg::PythPushOracle => OracleSetup::PythPushOracle,
            OracleTypeArg::SwitchboardPull => OracleSetup::SwitchboardPull,
        }
    }
}

#[derive(Clone, Copy, Debug, Parser, ArgEnum)]
pub enum BankOperationalStateArg {
    Paused,
    Operational,
    ReduceOnly,
}

impl From<BankOperationalStateArg> for BankOperationalState {
    fn from(val: BankOperationalStateArg) -> Self {
        match val {
            BankOperationalStateArg::Paused => BankOperationalState::Paused,
            BankOperationalStateArg::Operational => BankOperationalState::Operational,
            BankOperationalStateArg::ReduceOnly => BankOperationalState::ReduceOnly,
        }
    }
}

#[allow(clippy::large_enum_variant)]
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
        deposit_limit_ui: Option<f64>,

        #[clap(long)]
        borrow_limit_ui: Option<f64>,

        #[clap(long, arg_enum)]
        operational_state: Option<BankOperationalStateArg>,

        #[clap(long, help = "Optimal utilization rate")]
        opr_ur: Option<f64>,
        #[clap(long, help = "Plateau interest rate")]
        p_ir: Option<f64>,
        #[clap(long, help = "Max interest rate")]
        m_ir: Option<f64>,
        #[clap(long, help = "Insurance fee fixed APR")]
        if_fa: Option<f64>,
        #[clap(long, help = "Insurance IR fee")]
        if_ir: Option<f64>,
        #[clap(long, help = "Protocol fixed fee APR")]
        pf_fa: Option<f64>,
        #[clap(long, help = "Protocol IR fee")]
        pf_ir: Option<f64>,
        #[clap(long, help = "Protocol origination fee")]
        pf_or: Option<f64>,
        #[clap(long, arg_enum, help = "Bank risk tier")]
        risk_tier: Option<RiskTierArg>,
        #[clap(long, help = "0 = default, 1 = SOL, 2 = Staked SOL LST")]
        asset_tag: Option<u8>,
        #[clap(long, arg_enum, help = "Bank oracle type")]
        oracle_type: Option<OracleTypeArg>,
        #[clap(long, help = "Bank oracle account")]
        oracle_key: Option<Pubkey>,
        #[clap(long, help = "Soft USD init limit")]
        usd_init_limit: Option<u64>,
        #[clap(long, help = "Oracle max age in seconds, 0 to use default value (60s)")]
        oracle_max_age: Option<u16>,
        #[clap(
            long,
            help = "Permissionless bad debt settlement, if true the group admin is not required to settle bad debt"
        )]
        permissionless_bad_debt_settlement: Option<bool>,
        #[clap(
            long,
            help = "If enabled, will prevent this Update ix from ever running against after this invokation"
        )]
        freeze_settings: Option<bool>,
    },
    InspectPriceOracle {
        bank_pk: Pubkey,
    },
    SetupEmissions {
        bank: Pubkey,
        #[clap(long)]
        deposits: bool,
        #[clap(long)]
        borrows: bool,
        #[clap(long)]
        mint: Pubkey,
        #[clap(long)]
        rate_apr: f64,
        #[clap(long)]
        total_amount_ui: f64,
    },
    UpdateEmissions {
        bank: Pubkey,
        #[clap(long)]
        deposits: bool,
        #[clap(long)]
        borrows: bool,
        #[clap(long)]
        disable: bool,
        #[clap(long)]
        rate: Option<f64>,
        #[clap(long)]
        additional_amount_ui: Option<f64>,
    },
    SettleAllEmissions {
        bank: Pubkey,
    },
    CollectFees {
        bank: Pubkey,
        #[clap(help = "The ATA for fee_state.global_fee_wallet and the bank's mint")]
        fee_ata: Pubkey,
    },
    WithdrawFees {
        bank: Pubkey,
        amount: f64,
        #[clap(help = "Destination address, defaults to the profile authority")]
        destination_address: Option<Pubkey>,
    },
    WithdrawInsurance {
        bank: Pubkey,
        amount: f64,
        #[clap(help = "Destination address, defaults to the profile authority")]
        destination_address: Option<Pubkey>,
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
        multisig: Option<Pubkey>,
        #[clap(long)]
        rpc_url: String,
        #[clap(long)]
        program_id: Option<Pubkey>,
        #[clap(long)]
        commitment: Option<CommitmentLevel>,
        #[clap(long)]
        group: Option<Pubkey>,
        #[clap(long)]
        account: Option<Pubkey>,
    },
    Show,
    List,
    Set {
        name: String,
    },
    Update {
        name: String,
        #[clap(long)]
        new_name: Option<String>,
        #[clap(long)]
        cluster: Option<Cluster>,
        #[clap(long)]
        keypair_path: Option<String>,
        #[clap(long)]
        multisig: Option<Pubkey>,
        #[clap(long)]
        rpc_url: Option<String>,
        #[clap(long)]
        program_id: Option<Pubkey>,
        #[clap(long)]
        commitment: Option<CommitmentLevel>,
        #[clap(long)]
        group: Option<Pubkey>,
        #[clap(long)]
        account: Option<Pubkey>,
    },
    Delete {
        name: String,
    },
}

#[derive(Debug, Parser)]
pub enum AccountCommand {
    List,
    Use {
        account: Pubkey,
    },
    Get {
        account: Option<Pubkey>,
    },
    Deposit {
        bank: Pubkey,
        ui_amount: f64,
    },
    Withdraw {
        bank: Pubkey,
        ui_amount: f64,
        #[clap(short = 'a', long = "all")]
        withdraw_all: bool,
    },
    Borrow {
        bank: Pubkey,
        ui_amount: f64,
    },
    Liquidate {
        #[clap(long)]
        liquidatee_marginfi_account: Pubkey,
        #[clap(long)]
        asset_bank: Pubkey,
        #[clap(long)]
        liability_bank: Pubkey,
        #[clap(long)]
        ui_asset_amount: f64,
    },
    Create,
    SetFlag {
        account_pk: Pubkey,
        #[clap(long)]
        flashloans_enabled: bool,
        #[clap(long)]
        account_migration_enabled: bool,
    },
}

#[derive(Debug, Parser)]
#[cfg(feature = "lip")]
pub enum LipCommand {
    ListCampaigns,
    ListDeposits,
}

pub fn entry(opts: Opts) -> Result<()> {
    env_logger::init();

    match opts.command {
        Command::Group { subcmd } => group(subcmd, &opts.cfg_override),
        Command::Bank { subcmd } => bank(subcmd, &opts.cfg_override),
        Command::Profile { subcmd } => profile(subcmd),

        Command::InspectPadding {} => inspect_padding(),

        Command::PatchIdl { idl_path } => patch_marginfi_idl(idl_path),
        Command::Account { subcmd } => process_account_subcmd(subcmd, &opts.cfg_override),
        #[cfg(feature = "lip")]
        Command::Lip { subcmd } => process_lip_subcmd(subcmd, &opts.cfg_override),

        Command::InspectSize {} => inspect_size(),

        Command::ShowOracleAges { only_stale } => {
            let profile = load_profile()?;
            let config = profile.get_config(Some(&opts.cfg_override))?;

            processor::show_oracle_ages(config, only_stale)?;

            Ok(())
        }

        Command::MakeTestI80F48 => {
            process_make_test_i80f48();

            Ok(())
        }
        Command::InspectPythPushOracleFeed { pyth_feed } => {
            let profile = load_profile()?;
            let config = profile.get_config(Some(&opts.cfg_override))?;

            processor::oracle::inspect_pyth_push_feed(&config, pyth_feed)?;

            Ok(())
        }
        Command::FindPythPull { feed_id } => {
            let profile = load_profile()?;
            let config = profile.get_config(Some(&opts.cfg_override))?;
            let feed_id = get_feed_id_from_hex(&feed_id).unwrap();

            let rpc = config.mfi_program.rpc();

            find_pyth_push_oracles_for_feed_id(&rpc, feed_id)?;

            Ok(())
        }
        Command::InspectSwbPullFeed { address } => {
            let profile = load_profile()?;
            let config = profile.get_config(Some(&opts.cfg_override))?;

            processor::oracle::inspect_swb_pull_feed(&config, address)?;

            Ok(())
        }
    }
}

fn profile(subcmd: ProfileCommand) -> Result<()> {
    match subcmd {
        ProfileCommand::Create {
            name,
            cluster,
            keypair_path,
            multisig,
            rpc_url,
            program_id,
            commitment,
            group,
            account,
        } => processor::create_profile(
            name,
            cluster,
            keypair_path,
            multisig,
            rpc_url,
            program_id,
            commitment,
            group,
            account,
        ),
        ProfileCommand::Show => processor::show_profile(),
        ProfileCommand::List => processor::list_profiles(),
        ProfileCommand::Set { name } => processor::set_profile(name),
        ProfileCommand::Update {
            cluster,
            keypair_path,
            multisig,
            rpc_url,
            program_id,
            commitment,
            group,
            name,
            new_name,
            account,
        } => processor::configure_profile(
            name,
            new_name,
            cluster,
            keypair_path,
            multisig,
            rpc_url,
            program_id,
            commitment,
            group,
            account,
        ),
        ProfileCommand::Delete { name } => processor::delete_profile(name),
    }
}

fn group(subcmd: GroupCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    if !global_options.skip_confirmation {
        match subcmd {
            GroupCommand::Get { marginfi_group: _ } => (),
            GroupCommand::GetAll {} => (),

            _ => get_consent(&subcmd, &profile)?,
        }
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
            seed,
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            oracle_key,
            feed_id,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            group_fixed_fee_apr,
            group_ir_fee,
            deposit_limit_ui,
            borrow_limit_ui,
            risk_tier,
            oracle_type,
            oracle_max_age,
            global_fee_wallet,
        } => processor::group_add_bank(
            config,
            profile,
            bank_mint,
            seed,
            oracle_key,
            feed_id,
            oracle_type,
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            deposit_limit_ui,
            borrow_limit_ui,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            group_fixed_fee_apr,
            group_ir_fee,
            risk_tier,
            oracle_max_age,
            global_options.compute_unit_price,
            global_fee_wallet,
        ),

        GroupCommand::HandleBankruptcy { accounts } => {
            processor::handle_bankruptcy_for_accounts(&config, &profile, accounts)
        }

        GroupCommand::CheckLookupTable {
            existing_token_lookup_tables,
        } => processor::group::process_check_lookup_tables(
            &config,
            &profile,
            existing_token_lookup_tables,
        ),

        GroupCommand::UpdateLookupTable {
            existing_token_lookup_tables,
        } => processor::group::process_update_lookup_tables(
            &config,
            &profile,
            existing_token_lookup_tables,
        ),
        GroupCommand::InitFeeState {
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
        } => processor::initialize_fee_state(
            config,
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
        ),
        GroupCommand::EditFeeState {
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
        } => processor::edit_fee_state(
            config,
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
        ),
        GroupCommand::ConfigGroupFee { flag } => processor::config_group_fee(config, profile, flag),
        GroupCommand::PropagateFee { marginfi_group } => {
            processor::propagate_fee(config, marginfi_group)
        }
    }
}

fn bank(subcmd: BankCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    if !global_options.skip_confirmation {
        match subcmd {
            BankCommand::Get { .. } | BankCommand::GetAll { .. } => (),

            BankCommand::InspectPriceOracle { .. } => (),
            #[allow(unreachable_patterns)]
            _ => get_consent(&subcmd, &profile)?,
        }
    }

    match subcmd {
        BankCommand::Get { bank } => processor::bank_get(config, bank),
        BankCommand::GetAll { marginfi_group } => processor::bank_get_all(config, marginfi_group),
        BankCommand::Update {
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            deposit_limit_ui,
            borrow_limit_ui,
            operational_state,
            bank_pk,
            opr_ur,
            p_ir,
            m_ir,
            if_fa,
            if_ir,
            pf_fa,
            pf_ir,
            pf_or,
            risk_tier,
            asset_tag,
            oracle_type,
            oracle_key,
            usd_init_limit,
            oracle_max_age,
            permissionless_bad_debt_settlement,
            freeze_settings,
        } => {
            let bank = config
                .mfi_program
                .account::<marginfi::state::bank::Bank>(bank_pk)
                .unwrap();
            processor::bank_configure(
                config,
                profile, //
                bank_pk,
                BankConfigOpt {
                    asset_weight_init: asset_weight_init.map(|x| I80F48::from_num(x).into()),
                    asset_weight_maint: asset_weight_maint.map(|x| I80F48::from_num(x).into()),
                    liability_weight_init: liability_weight_init
                        .map(|x| I80F48::from_num(x).into()),
                    liability_weight_maint: liability_weight_maint
                        .map(|x| I80F48::from_num(x).into()),
                    deposit_limit: deposit_limit_ui.map(|ui_amount| {
                        spl_token::ui_amount_to_amount(ui_amount, bank.mint_decimals)
                    }),
                    borrow_limit: borrow_limit_ui.map(|ui_amount| {
                        spl_token::ui_amount_to_amount(ui_amount, bank.mint_decimals)
                    }),
                    operational_state: operational_state.map(|x| x.into()),
                    oracle: oracle_key.map(|x| marginfi::state::marginfi_group::OracleConfig {
                        setup: oracle_type
                            .expect("Orcale type must be provided with oracle_key")
                            .into(),
                        keys: [
                            x,
                            Pubkey::default(),
                            Pubkey::default(),
                            Pubkey::default(),
                            Pubkey::default(),
                        ],
                    }),
                    interest_rate_config: Some(InterestRateConfigOpt {
                        optimal_utilization_rate: opr_ur.map(|x| I80F48::from_num(x).into()),
                        plateau_interest_rate: p_ir.map(|x| I80F48::from_num(x).into()),
                        max_interest_rate: m_ir.map(|x| I80F48::from_num(x).into()),
                        insurance_fee_fixed_apr: if_fa.map(|x| I80F48::from_num(x).into()),
                        insurance_ir_fee: if_ir.map(|x| I80F48::from_num(x).into()),
                        protocol_fixed_fee_apr: pf_fa.map(|x| I80F48::from_num(x).into()),
                        protocol_ir_fee: pf_ir.map(|x| I80F48::from_num(x).into()),
                        protocol_origination_fee: pf_or.map(|x| I80F48::from_num(x).into()),
                    }),
                    risk_tier: risk_tier.map(|x| x.into()),
                    asset_tag,
                    total_asset_value_init_limit: usd_init_limit,
                    oracle_max_age,
                    permissionless_bad_debt_settlement,
                    freeze_settings,
                },
            )
        }
        BankCommand::InspectPriceOracle { bank_pk } => {
            processor::bank_inspect_price_oracle(config, bank_pk)
        }
        BankCommand::SetupEmissions {
            bank,
            deposits,
            borrows,
            mint,
            rate_apr: rate,
            total_amount_ui: total_ui,
        } => processor::bank_setup_emissions(
            &config, &profile, bank, deposits, borrows, mint, rate, total_ui,
        ),
        BankCommand::UpdateEmissions {
            bank,
            deposits,
            borrows,
            disable,
            rate,
            additional_amount_ui,
        } => processor::bank_update_emissions(
            &config,
            &profile,
            bank,
            deposits,
            borrows,
            disable,
            rate,
            additional_amount_ui,
        ),
        BankCommand::SettleAllEmissions { bank } => {
            processor::emissions::claim_all_emissions_for_bank(&config, &profile, bank)
        }
        BankCommand::CollectFees { bank, fee_ata } => {
            processor::admin::process_collect_fees(config, bank, fee_ata)
        }
        BankCommand::WithdrawFees {
            bank,
            amount,
            destination_address,
        } => processor::admin::process_withdraw_fees(config, bank, amount, destination_address),
        BankCommand::WithdrawInsurance {
            bank,
            amount,
            destination_address,
        } => {
            processor::admin::process_withdraw_insurance(config, bank, amount, destination_address)
        }
    }
}

fn inspect_padding() -> Result<()> {
    println!("MarginfiGroup: {}", MarginfiGroup::type_layout());
    println!("GroupConfig: {}", GroupConfig::type_layout());
    println!("InterestRateConfig: {}", InterestRateConfig::type_layout());
    println!("Bank: {}", marginfi::state::bank::Bank::type_layout());
    println!("BankConfig: {}", BankConfig::type_layout());
    println!("OracleConfig: {}", OracleConfig::type_layout());
    println!("BankConfigOpt: {}", BankConfigOpt::type_layout());
    println!("WrappedI80F48: {}", WrappedI80F48::type_layout());

    println!("MarginfiAccount: {}", MarginfiAccount::type_layout());
    println!("LendingAccount: {}", LendingAccount::type_layout());
    println!("Balance: {}", Balance::type_layout());

    Ok(())
}

fn inspect_size() -> Result<()> {
    use std::mem::size_of;

    println!("MarginfiGroup: {}", size_of::<MarginfiGroup>());
    println!("GroupConfig: {}", size_of::<GroupConfig>());
    println!("InterestRateConfig: {}", size_of::<InterestRateConfig>());
    println!("Bank: {}", size_of::<marginfi::state::bank::Bank>());
    println!("BankConfig: {}", size_of::<BankConfig>());
    println!("OracleConfig: {}", size_of::<OracleConfig>());
    println!("BankConfigOpt: {}", size_of::<BankConfigOpt>());
    println!("WrappedI80F48: {}", size_of::<WrappedI80F48>());

    println!("MarginfiAccount: {}", size_of::<MarginfiAccount>());
    println!("LendingAccount: {}", size_of::<LendingAccount>());
    println!("Balance: {}", size_of::<Balance>());

    Ok(())
}

fn patch_marginfi_idl(target_dir: String) -> Result<()> {
    use crate::patch_type_layout;

    let idl_path = format!("{}/idl/marginfi.json", target_dir);

    let file = std::fs::File::open(&idl_path)?;
    let reader = std::io::BufReader::new(file);
    let mut idl: serde_json::Value = serde_json::from_reader(reader)?;

    let idl_original_path = idl_path.replace(".json", "_original.json");
    let file = std::fs::File::create(idl_original_path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &idl)?;

    // Patch IDL

    if let Some(types) = idl.get_mut("types").and_then(|t| t.as_array_mut()) {
        if let Some(pos) = types
            .iter()
            .position(|t| t["name"] == "OraclePriceFeedAdapter")
        {
            types.remove(pos);
        }
    }

    patch_type_layout!(idl, "Bank", Bank, "types");
    patch_type_layout!(idl, "Balance", Balance, "types");
    patch_type_layout!(idl, "BankConfig", BankConfig, "types");
    patch_type_layout!(idl, "BankConfigCompact", BankConfig, "types");

    let file = std::fs::File::create(&idl_path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &idl)?;

    Ok(())
}

fn process_account_subcmd(subcmd: AccountCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    if !global_options.skip_confirmation {
        match subcmd {
            AccountCommand::Get { .. } | AccountCommand::List => (),
            _ => get_consent(&subcmd, &profile)?,
        }
    }

    match subcmd {
        AccountCommand::List => processor::marginfi_account_list(profile, &config),
        AccountCommand::Use { account } => {
            processor::marginfi_account_use(profile, &config, account)
        }
        AccountCommand::Get { account } => {
            processor::marginfi_account_get(profile, &config, account)
        }
        AccountCommand::Deposit { bank, ui_amount } => {
            processor::marginfi_account_deposit(&profile, &config, bank, ui_amount)
        }
        AccountCommand::Withdraw {
            bank,
            ui_amount,
            withdraw_all,
        } => processor::marginfi_account_withdraw(&profile, &config, bank, ui_amount, withdraw_all),
        AccountCommand::Borrow { bank, ui_amount } => {
            processor::marginfi_account_borrow(&profile, &config, bank, ui_amount)
        }
        AccountCommand::Liquidate {
            asset_bank: asset_bank_pk,
            liability_bank: liability_bank_pk,
            liquidatee_marginfi_account: liquidatee_marginfi_account_pk,
            ui_asset_amount,
        } => processor::marginfi_account_liquidate(
            &profile,
            &config,
            liquidatee_marginfi_account_pk,
            asset_bank_pk,
            liability_bank_pk,
            ui_asset_amount,
        ),
        AccountCommand::Create => processor::marginfi_account_create(&profile, &config),
        AccountCommand::SetFlag {
            flashloans_enabled: flashloan,
            account_pk,
            account_migration_enabled,
        } => {
            let mut flag = 0;

            if flashloan {
                println!("Setting flashloan flag");
                flag |= FLASHLOAN_ENABLED_FLAG;
            }

            if account_migration_enabled {
                println!("Setting account migration flag");
                flag |= TRANSFER_AUTHORITY_ALLOWED_FLAG;
            }

            if flag == 0 {
                println!("No flag provided");
                std::process::exit(1);
            }

            process_set_user_flag(config, &profile, account_pk, flag)
        }
    }?;

    Ok(())
}

#[cfg(feature = "lip")]
fn process_lip_subcmd(
    subcmd: LipCommand,
    cfg_override: &GlobalOptions,
) -> Result<(), anyhow::Error> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(cfg_override))?;

    match subcmd {
        LipCommand::ListCampaigns => processor::process_list_lip_campaigns(&config),
        LipCommand::ListDeposits => processor::process_list_deposits(&config),
    }

    Ok(())
}

fn get_consent<T: std::fmt::Debug>(cmd: T, profile: &Profile) -> Result<()> {
    let mut input = String::new();
    println!("Command: {cmd:#?}");
    println!("{profile:#?}");
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

pub fn process_make_test_i80f48() {
    let mut rng = rand::thread_rng();

    let i80f48s: Vec<I80F48> = (0..30i128)
        .map(|_| {
            let i = rng.gen_range(-1_000_000_000_000i128..1_000_000_000_000i128);
            I80F48::from_num(i) / I80F48::from_num(1_000_000)
        })
        .collect();

    println!("const testCases = [");
    for i80f48 in i80f48s {
        println!(
            "  {{ number: {:?}, innerValue: {:?} }},",
            i80f48,
            marginfi::state::marginfi_group::WrappedI80F48::from(i80f48).value
        );
    }

    let explicit = vec![
        0.,
        1.,
        -1.,
        0.328934,
        423947246342.487,
        1783921462347640.,
        0.00000000000232,
    ];
    for f in explicit {
        let i80f48 = I80F48::from_num(f);
        println!(
            "  {{ number: {:?}, innerValue: {:?} }},",
            i80f48,
            marginfi::state::marginfi_group::WrappedI80F48::from(i80f48).value
        );
    }
    println!("];");
}
