use crate::{
    config::GlobalOptions,
    processor::{self, group::process_update_lookup_tables},
    profile::{load_profile, Profile},
};
use anchor_client::Cluster;
use anyhow::Result;
use clap::{clap_derive::ArgEnum, Parser};
#[cfg(feature = "admin")]
use fixed::types::I80F48;
#[cfg(any(feature = "admin", feature = "dev"))]
use marginfi::state::marginfi_group::{BankConfigOpt, InterestRateConfigOpt};
use marginfi::state::{
    marginfi_group::{BankOperationalState, RiskTier},
    price::OracleSetup,
};
#[cfg(feature = "dev")]
use marginfi::{
    prelude::{GroupConfig, MarginfiGroup},
    state::{
        marginfi_account::{Balance, LendingAccount, MarginfiAccount},
        marginfi_group::{Bank, BankConfig, InterestRateConfig, OracleConfig, WrappedI80F48},
    },
};
use solana_sdk::{commitment_config::CommitmentLevel, pubkey::Pubkey};

#[cfg(feature = "dev")]
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
    #[cfg(feature = "dev")]
    InspectPadding {},
    #[cfg(feature = "dev")]
    PatchIdl { idl_path: String },
    #[cfg(feature = "dev")]
    InspectSize {},
    Account {
        #[clap(subcommand)]
        subcmd: AccountCommand,
    },
    #[cfg(feature = "lip")]
    Lip {
        #[clap(subcommand)]
        subcmd: LipCommand,
    },
    #[cfg(feature = "dev")]
    InspectSwitchboardFeed { switchboard_feed: Pubkey },
}

#[derive(Debug, Parser)]
pub enum GroupCommand {
    Get {
        marginfi_group: Option<Pubkey>,
    },
    GetAll {},
    #[cfg(feature = "admin")]
    Create {
        admin: Option<Pubkey>,
        #[clap(short = 'f', long = "override")]
        override_existing_profile_group: bool,
    },
    #[cfg(feature = "admin")]
    Update {
        admin: Option<Pubkey>,
    },
    #[cfg(feature = "admin")]
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
        deposit_limit_ui: u64,
        #[clap(long)]
        borrow_limit_ui: u64,
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
        #[clap(long, arg_enum)]
        risk_tier: RiskTierArg,
        #[clap(long, arg_enum)]
        oracle_type: OracleTypeArg,
    },
    #[cfg(feature = "admin")]
    HandleBankruptcy {
        accounts: Vec<Pubkey>,
    },
    #[cfg(feature = "admin")]
    UpdateLookupTable {
        #[clap(short = 't', long)]
        existing_token_lookup_tables: Vec<Pubkey>,
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
    PythEma,
    Switchboard,
}

impl From<OracleTypeArg> for OracleSetup {
    fn from(value: OracleTypeArg) -> Self {
        match value {
            OracleTypeArg::PythEma => OracleSetup::PythEma,
            OracleTypeArg::Switchboard => OracleSetup::SwitchboardV2,
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

#[derive(Debug, Parser)]
pub enum BankCommand {
    Get {
        bank: Option<Pubkey>,
    },
    GetAll {
        marginfi_group: Option<Pubkey>,
    },
    #[cfg(feature = "admin")]
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
        #[clap(long, arg_enum, help = "Bank risk tier")]
        risk_tier: Option<RiskTierArg>,
        #[clap(long, arg_enum, help = "Bank oracle type")]
        oracle_type: Option<OracleTypeArg>,
        #[clap(long, help = "Bank oracle account")]
        oracle_key: Option<Pubkey>,
        #[clap(long, help = "Soft USD init limit")]
        usd_init_limit: Option<u64>,
    },
    #[cfg(feature = "dev")]
    InspectPriceOracle {
        bank_pk: Pubkey,
    },
    #[cfg(feature = "admin")]
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
    #[cfg(feature = "admin")]
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
    #[cfg(feature = "admin")]
    SettleAllEmissions {
        bank: Pubkey,
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
        keypair_path: Option<String>,
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
        #[cfg(feature = "dev")]
        Command::InspectPadding {} => inspect_padding(),
        #[cfg(feature = "dev")]
        Command::PatchIdl { idl_path } => patch_marginfi_idl(idl_path),
        Command::Account { subcmd } => process_account_subcmd(subcmd, &opts.cfg_override),
        #[cfg(feature = "lip")]
        Command::Lip { subcmd } => process_lip_subcmd(subcmd, &opts.cfg_override),
        #[cfg(feature = "dev")]
        Command::InspectSwitchboardFeed { switchboard_feed } => {
            let profile = load_profile()?;
            let config = profile.get_config(Some(&opts.cfg_override))?;

            processor::process_inspect_switchboard_feed(&config, &switchboard_feed);

            Ok(())
        }
        #[cfg(feature = "dev")]
        Command::InspectSize {} => inspect_size(),
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
            account,
        } => processor::configure_profile(
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
    }
}

fn group(subcmd: GroupCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    if !global_options.skip_confirmation {
        match subcmd {
            GroupCommand::Get { marginfi_group: _ } => (),
            GroupCommand::GetAll {} => (),
            #[cfg(feature = "admin")]
            _ => get_consent(&subcmd, &profile)?,
        }
    }

    match subcmd {
        GroupCommand::Get { marginfi_group } => {
            processor::group_get(config, marginfi_group.or(profile.marginfi_group))
        }
        GroupCommand::GetAll {} => processor::group_get_all(config),
        #[cfg(feature = "admin")]
        GroupCommand::Create {
            admin,
            override_existing_profile_group,
        } => processor::group_create(config, profile, admin, override_existing_profile_group),
        #[cfg(feature = "admin")]
        GroupCommand::Update { admin } => processor::group_configure(config, profile, admin),
        #[cfg(feature = "admin")]
        GroupCommand::AddBank {
            mint: bank_mint,
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            pyth_oracle,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            protocol_fixed_fee_apr,
            protocol_ir_fee,
            deposit_limit_ui,
            borrow_limit_ui,
            risk_tier,
            oracle_type,
        } => processor::group_add_bank(
            config,
            profile,
            bank_mint,
            pyth_oracle,
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
            protocol_fixed_fee_apr,
            protocol_ir_fee,
            risk_tier,
        ),
        #[cfg(feature = "admin")]
        GroupCommand::HandleBankruptcy { accounts } => {
            processor::handle_bankruptcy_for_accounts(&config, &profile, accounts)
        }
        #[cfg(feature = "admin")]
        GroupCommand::UpdateLookupTable {
            existing_token_lookup_tables,
        } => process_update_lookup_tables(&config, &profile, existing_token_lookup_tables),
    }
}

fn bank(subcmd: BankCommand, global_options: &GlobalOptions) -> Result<()> {
    let profile = load_profile()?;
    let config = profile.get_config(Some(global_options))?;

    if !global_options.skip_confirmation {
        match subcmd {
            BankCommand::Get { .. } | BankCommand::GetAll { .. } => (),
            #[cfg(feature = "dev")]
            BankCommand::InspectPriceOracle { .. } => (),
            #[allow(unreachable_patterns)]
            _ => get_consent(&subcmd, &profile)?,
        }
    }

    match subcmd {
        BankCommand::Get { bank } => processor::bank_get(config, bank),
        BankCommand::GetAll { marginfi_group } => processor::bank_get_all(config, marginfi_group),
        #[cfg(feature = "admin")]
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
            risk_tier,
            oracle_type,
            oracle_key,
            usd_init_limit,
        } => {
            let bank = config
                .mfi_program
                .account::<marginfi::state::marginfi_group::Bank>(bank_pk)
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
                    }),
                    risk_tier: risk_tier.map(|x| x.into()),
                    total_asset_value_init_limit: usd_init_limit,
                },
            )
        }
        #[cfg(feature = "dev")]
        BankCommand::InspectPriceOracle { bank_pk } => {
            processor::bank_inspect_price_oracle(config, bank_pk)
        }
        #[cfg(feature = "admin")]
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
        #[cfg(feature = "admin")]
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
        #[cfg(feature = "admin")]
        BankCommand::SettleAllEmissions { bank } => {
            processor::emissions::claim_all_emissions_for_bank(&config, &profile, bank)
        }
    }
}

#[cfg(feature = "dev")]
fn inspect_padding() -> Result<()> {
    println!("MarginfiGroup: {}", MarginfiGroup::type_layout());
    println!("GroupConfig: {}", GroupConfig::type_layout());
    println!("InterestRateConfig: {}", InterestRateConfig::type_layout());
    println!(
        "Bank: {}",
        marginfi::state::marginfi_group::Bank::type_layout()
    );
    println!("BankConfig: {}", BankConfig::type_layout());
    println!("OracleConfig: {}", OracleConfig::type_layout());
    println!("BankConfigOpt: {}", BankConfigOpt::type_layout());
    println!("WrappedI80F48: {}", WrappedI80F48::type_layout());

    println!("MarginfiAccount: {}", MarginfiAccount::type_layout());
    println!("LendingAccount: {}", LendingAccount::type_layout());
    println!("Balance: {}", Balance::type_layout());

    Ok(())
}

#[cfg(feature = "dev")]
fn inspect_size() -> Result<()> {
    use std::mem::size_of;

    println!("MarginfiGroup: {}", size_of::<MarginfiGroup>());
    println!("GroupConfig: {}", size_of::<GroupConfig>());
    println!("InterestRateConfig: {}", size_of::<InterestRateConfig>());
    println!(
        "Bank: {}",
        size_of::<marginfi::state::marginfi_group::Bank>()
    );
    println!("BankConfig: {}", size_of::<BankConfig>());
    println!("OracleConfig: {}", size_of::<OracleConfig>());
    println!("BankConfigOpt: {}", size_of::<BankConfigOpt>());
    println!("WrappedI80F48: {}", size_of::<WrappedI80F48>());

    println!("MarginfiAccount: {}", size_of::<MarginfiAccount>());
    println!("LendingAccount: {}", size_of::<LendingAccount>());
    println!("Balance: {}", size_of::<Balance>());

    Ok(())
}

#[cfg(feature = "dev")]
fn patch_marginfi_idl(target_dir: String) -> Result<()> {
    use crate::patch_type_layout;
    use std::io::Write;

    let idl_path = format!("{}/idl/marginfi.json", target_dir);

    let file = std::fs::File::open(&idl_path)?;
    let reader = std::io::BufReader::new(file);
    let mut idl: serde_json::Value = serde_json::from_reader(reader)?;

    let idl_original_path = idl_path.replace(".json", "_original.json");
    let file = std::fs::File::create(&idl_original_path)?;
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

    patch_type_layout!(idl, "Bank", Bank, "accounts");
    patch_type_layout!(idl, "Balance", Balance, "types");
    patch_type_layout!(idl, "BankConfig", BankConfig, "types");
    patch_type_layout!(idl, "BankConfigCompact", BankConfig, "types");

    let file = std::fs::File::create(&idl_path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &idl)?;

    // Patch types

    let types_path = format!("{}/types/marginfi.ts", target_dir);
    let mut ts_file = std::fs::File::create(types_path)?;

    if let Some(accounts) = idl.get_mut("accounts").and_then(|a| a.as_array_mut()) {
        for account in accounts.iter_mut() {
            if let Some(name) = account.get_mut("name").and_then(|n| n.as_str()) {
                let mut chars = name.chars();
                if let Some(first_char) = chars.next() {
                    let name_with_lowercase_first_letter =
                        first_char.to_lowercase().collect::<String>() + chars.as_str();
                    account["name"] = serde_json::Value::String(name_with_lowercase_first_letter);
                }
            }
        }
    }

    write!(
        ts_file,
        "export type Marginfi = {};\n",
        serde_json::to_string_pretty(&idl)?
    )?;
    write!(
        ts_file,
        "export const IDL: Marginfi = {};\n",
        serde_json::to_string_pretty(&idl)?
    )?;

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
