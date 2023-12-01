#[cfg(feature = "admin")]
pub mod emissions;

pub mod group;

use {
    crate::{
        config::Config,
        profile::{self, get_cli_config_dir, load_profile, CliConfig, Profile},
        utils::{
            find_bank_vault_authority_pda, find_bank_vault_pda, load_observation_account_metas,
            process_transaction, EXP_10_I80F48,
        },
    },
    anchor_client::{
        anchor_lang::{InstructionData, ToAccountMetas},
        Cluster,
    },
    anchor_spl::token::{self, spl_token},
    anyhow::{anyhow, bail, Result},
    fixed::types::I80F48,
    log::info,
    marginfi::{
        prelude::MarginfiGroup,
        state::{
            marginfi_account::{BankAccountWrapper, MarginfiAccount},
            marginfi_group::{Bank, BankVaultType},
        },
    },
    solana_client::rpc_filter::{Memcmp, RpcFilterType},
    solana_sdk::{
        account_info::IntoAccountInfo,
        clock::Clock,
        commitment_config::CommitmentLevel,
        compute_budget::ComputeBudgetInstruction,
        instruction::{AccountMeta, Instruction},
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_program,
        sysvar::{self, Sysvar},
        transaction::Transaction,
    },
    spl_associated_token_account::instruction::create_associated_token_account_idempotent,
    std::{
        collections::HashMap,
        fs,
        mem::size_of,
        ops::{Neg, Not},
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
};

#[cfg(feature = "dev")]
use marginfi::state::price::{OraclePriceFeedAdapter, PriceAdapter};
use marginfi::{constants::ZERO_AMOUNT_THRESHOLD, utils::NumTraitsWithTolerance};
use solana_client::rpc_client::RpcClient;

#[cfg(feature = "admin")]
use {
    crate::utils::{
        calc_emissions_rate, create_oracle_key_array, find_bank_emssions_auth_pda,
        find_bank_emssions_token_account_pda,
    },
    marginfi::{
        constants::{EMISSIONS_FLAG_BORROW_ACTIVE, EMISSIONS_FLAG_LENDING_ACTIVE},
        prelude::GroupConfig,
        state::marginfi_group::{
            BankConfig, BankConfigOpt, BankOperationalState, InterestRateConfig, WrappedI80F48,
        },
    },
    solana_sdk::program_pack::Pack,
    spl_associated_token_account::get_associated_token_address,
    std::io,
};

#[cfg(feature = "lip")]
use {
    chrono::{DateTime, NaiveDateTime, Utc},
    liquidity_incentive_program::state::{Campaign, Deposit},
};

// --------------------------------------------------------------------------------------------------------------------
// marginfi group
// --------------------------------------------------------------------------------------------------------------------

pub fn group_get(config: Config, marginfi_group: Option<Pubkey>) -> Result<()> {
    if let Some(marginfi_group) = marginfi_group {
        println!("Address: {marginfi_group}");
        println!("=============");
        print_group_banks(config, marginfi_group)?;
    } else {
        group_get_all(config)?;
    }
    Ok(())
}

pub fn group_get_all(config: Config) -> Result<()> {
    let accounts: Vec<(Pubkey, MarginfiGroup)> = config.mfi_program.accounts(vec![])?;

    accounts
        .iter()
        .for_each(|(address, group)| print_group(address, group));

    Ok(())
}

fn print_group(address: &Pubkey, group: &MarginfiGroup) {
    println!(
        r#"
Group: {}
Admin: {}
"#,
        address, group.admin
    );
}

pub fn print_group_banks(config: Config, marginfi_group: Pubkey) -> Result<()> {
    let banks = config
        .mfi_program
        .accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 + size_of::<Pubkey>() + size_of::<u8>(),
            marginfi_group.to_bytes().to_vec(),
        ))])?;

    println!("--------\nBanks:");

    banks
        .iter()
        .for_each(|(address, bank)| print_bank(address, bank));

    Ok(())
}

fn print_bank(address: &Pubkey, bank: &Bank) {
    println!(
        r#"
Group: {},
Bank: {}
Mint: {},
Total Deposits: {}
Total Liabilities: {}
Config:
  State: {:?}
  Risk Tier: {:?}
  USD Soft limit: {:?}
  Asset:
    Weight Init: {:?}, Maint: {:?}
    Limit: {}
  Liab:
    Weight Init: {:?}, Maint: {:?}
    Limit: {}
  Interest Rate Config:
    Curve: opt_ur: {:?} pl_ir: {:?} max_ir: {:?}
    Fees - Insurance: ir: {:?} fix: {:?}, Group: ir: {:?} fix: {:?}
  Oracle Setup:
    Type: {:?}
    Keys: {:#?}
Emissions:
  Flags: 0b{:b}
  Rate: {:?}
  Mint: {:?}
  Remaining: {:?}
Last Update: {:?}h ago ({})
"#,
        bank.group,
        address,
        bank.mint,
        bank.get_asset_amount(bank.total_asset_shares.into())
            .unwrap()
            / EXP_10_I80F48[bank.mint_decimals as usize],
        bank.get_liability_amount(bank.total_liability_shares.into())
            .unwrap()
            / EXP_10_I80F48[bank.mint_decimals as usize],
        bank.config.operational_state,
        bank.config.risk_tier,
        bank.config.total_asset_value_init_limit,
        bank.config.asset_weight_init,
        bank.config.asset_weight_maint,
        I80F48::from_num(bank.config.deposit_limit) / EXP_10_I80F48[bank.mint_decimals as usize],
        bank.config.liability_weight_init,
        bank.config.liability_weight_maint,
        I80F48::from_num(bank.config.borrow_limit) / EXP_10_I80F48[bank.mint_decimals as usize],
        bank.config.interest_rate_config.optimal_utilization_rate,
        bank.config.interest_rate_config.plateau_interest_rate,
        bank.config.interest_rate_config.max_interest_rate,
        bank.config.interest_rate_config.insurance_ir_fee,
        bank.config.interest_rate_config.insurance_fee_fixed_apr,
        bank.config.interest_rate_config.protocol_ir_fee,
        bank.config.interest_rate_config.protocol_fixed_fee_apr,
        bank.config.oracle_setup,
        bank.config.oracle_keys,
        bank.emissions_flags,
        I80F48::from(bank.emissions_rate),
        bank.emissions_mint,
        I80F48::from(bank.emissions_remaining),
        SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::from_secs(bank.last_update as u64))
            .unwrap()
            .as_secs_f32()
            / 3600_f32,
        bank.last_update
    )
}

#[cfg(feature = "admin")]
pub fn group_create(
    config: Config,
    profile: Profile,
    admin: Option<Pubkey>,
    override_existing_profile_group: bool,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();
    let admin = admin.unwrap_or_else(|| config.authority());

    if profile.marginfi_group.is_some() && !override_existing_profile_group {
        bail!(
            "Marginfi group already exists for profile [{}]",
            profile.name
        );
    }

    let marginfi_group_keypair = Keypair::new();

    let init_marginfi_group_ixs_builder = config.mfi_program.request();

    let mut signing_keypairs = config.get_signers(false);
    signing_keypairs.push(&marginfi_group_keypair);

    let init_marginfi_group_ixs = init_marginfi_group_ixs_builder
        .accounts(marginfi::accounts::MarginfiGroupInitialize {
            marginfi_group: marginfi_group_keypair.pubkey(),
            admin,
            system_program: system_program::id(),
        })
        .args(marginfi::instruction::MarginfiGroupInitialize {})
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let message = Message::new(&init_marginfi_group_ixs, Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("marginfi group created (sig: {})", sig),
        Err(err) => {
            println!("Error during marginfi group creation:\n{:#?}", err);
            return Err(anyhow!("Error during marginfi group creation"));
        }
    };

    let mut profile = profile;
    profile.set_marginfi_group(marginfi_group_keypair.pubkey())?;

    Ok(())
}

#[cfg(feature = "admin")]
pub fn group_configure(config: Config, profile: Profile, admin: Option<Pubkey>) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    if profile.marginfi_group.is_none() {
        bail!("Marginfi group not specified in profile [{}]", profile.name);
    }

    let signing_keypairs = config.get_signers(false);
    let configure_marginfi_group_ixs_builder = config
        .mfi_program
        .request()
        .signer(*signing_keypairs.first().unwrap());

    let configure_marginfi_group_ixs = configure_marginfi_group_ixs_builder
        .accounts(marginfi::accounts::MarginfiGroupConfigure {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.authority(),
        })
        .args(marginfi::instruction::MarginfiGroupConfigure {
            config: GroupConfig { admin },
        })
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let message = Message::new(&configure_marginfi_group_ixs, Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("marginfi group created (sig: {})", sig),
        Err(err) => println!("Error during marginfi group creation:\n{:#?}", err),
    };

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "admin")]
pub fn group_add_bank(
    config: Config,
    profile: Profile,
    bank_mint: Pubkey,
    oracle_key: Pubkey,
    oracle_setup: crate::OracleTypeArg,
    asset_weight_init: f64,
    asset_weight_maint: f64,
    liability_weight_init: f64,
    liability_weight_maint: f64,
    deposit_limit_ui: u64,
    borrow_limit_ui: u64,
    optimal_utilization_rate: f64,
    plateau_interest_rate: f64,
    max_interest_rate: f64,
    insurance_fee_fixed_apr: f64,
    insurance_ir_fee: f64,
    protocol_fixed_fee_apr: f64,
    protocol_ir_fee: f64,
    risk_tier: crate::RiskTierArg,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    if profile.marginfi_group.is_none() {
        bail!("Marginfi group not specified in profile [{}]", profile.name);
    }

    let asset_weight_init: WrappedI80F48 = I80F48::from_num(asset_weight_init).into();
    let asset_weight_maint: WrappedI80F48 = I80F48::from_num(asset_weight_maint).into();
    let liability_weight_init: WrappedI80F48 = I80F48::from_num(liability_weight_init).into();
    let liability_weight_maint: WrappedI80F48 = I80F48::from_num(liability_weight_maint).into();

    let optimal_utilization_rate: WrappedI80F48 = I80F48::from_num(optimal_utilization_rate).into();
    let plateau_interest_rate: WrappedI80F48 = I80F48::from_num(plateau_interest_rate).into();
    let max_interest_rate: WrappedI80F48 = I80F48::from_num(max_interest_rate).into();
    let insurance_fee_fixed_apr: WrappedI80F48 = I80F48::from_num(insurance_fee_fixed_apr).into();
    let insurance_ir_fee: WrappedI80F48 = I80F48::from_num(insurance_ir_fee).into();
    let protocol_fixed_fee_apr: WrappedI80F48 = I80F48::from_num(protocol_fixed_fee_apr).into();
    let protocol_ir_fee: WrappedI80F48 = I80F48::from_num(protocol_ir_fee).into();

    let mint_account = rpc_client.get_account(&bank_mint)?;
    let mint = spl_token::state::Mint::unpack(&mint_account.data)?;

    let deposit_limit = deposit_limit_ui * 10_u64.pow(mint.decimals as u32);
    let borrow_limit = borrow_limit_ui * 10_u64.pow(mint.decimals as u32);

    let interest_rate_config = InterestRateConfig {
        optimal_utilization_rate,
        plateau_interest_rate,
        max_interest_rate,
        insurance_fee_fixed_apr,
        insurance_ir_fee,
        protocol_fixed_fee_apr,
        protocol_ir_fee,
        ..InterestRateConfig::default()
    };

    let bank_keypair = Keypair::new();

    let add_bank_ixs_builder = config.mfi_program.request();

    let mut signing_keypairs = config.get_signers(true);
    signing_keypairs.push(&bank_keypair);

    let add_bank_ixs = add_bank_ixs_builder
        .accounts(marginfi::accounts::LendingPoolAddBank {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.authority(),
            bank: bank_keypair.pubkey(),
            bank_mint,
            fee_vault: find_bank_vault_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Fee,
                &config.program_id,
            )
            .0,
            fee_vault_authority: find_bank_vault_authority_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Fee,
                &config.program_id,
            )
            .0,
            insurance_vault: find_bank_vault_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            insurance_vault_authority: find_bank_vault_authority_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            liquidity_vault: find_bank_vault_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
            liquidity_vault_authority: find_bank_vault_authority_pda(
                &bank_keypair.pubkey(),
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
            rent: sysvar::rent::id(),
            token_program: token::ID,
            system_program: system_program::id(),
            fee_payer: config.explicit_fee_payer(),
        })
        .accounts(AccountMeta::new_readonly(oracle_key, false))
        .args(marginfi::instruction::LendingPoolAddBank {
            bank_config: BankConfig {
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                deposit_limit,
                borrow_limit,
                interest_rate_config,
                operational_state: BankOperationalState::Operational,
                oracle_setup: oracle_setup.into(),
                oracle_keys: create_oracle_key_array(oracle_key),
                risk_tier: risk_tier.into(),
                ..BankConfig::default()
            }
            .into(),
        })
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let message = Message::new(&add_bank_ixs, Some(&config.explicit_fee_payer()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("bank created (sig: {})", sig),
        Err(err) => println!("Error during bank creation:\n{:#?}", err),
    };

    println!("New {} bank: {}", bank_mint, bank_keypair.pubkey());

    Ok(())
}

#[allow(clippy::too_many_arguments, dead_code)]
#[cfg(feature = "admin")]
pub fn group_handle_bankruptcy(
    config: &Config,
    profile: Profile,
    bank_pk: Pubkey,
    marginfi_account_pk: Pubkey,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    if profile.marginfi_group.is_none() {
        bail!("Marginfi group not specified in profile [{}]", profile.name);
    }

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    handle_bankruptcy_for_an_account(
        config,
        &profile,
        &rpc_client,
        &banks,
        marginfi_account_pk,
        &marginfi_account,
        bank_pk,
    )?;

    Ok(())
}

#[allow(dead_code)]
pub fn group_auto_handle_bankruptcy_for_an_account(
    config: &Config,
    profile: Profile,
    marginfi_account_pk: Pubkey,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    if profile.marginfi_group.is_none() {
        bail!("Marginfi group not specified in profile [{}]", profile.name);
    }

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);
    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    marginfi_account
        .lending_account
        .balances
        .iter()
        .filter(|b| {
            b.active
                && banks
                    .get(&b.bank_pk)
                    .unwrap()
                    .get_liability_amount(b.liability_shares.into())
                    .unwrap()
                    .is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD)
        })
        .map(|b| b.bank_pk)
        .collect::<Vec<Pubkey>>()
        .iter()
        .for_each(|bank_pk| {
            handle_bankruptcy_for_an_account(
                config,
                &profile,
                &rpc_client,
                &banks,
                marginfi_account_pk,
                &marginfi_account,
                *bank_pk,
            )
            .unwrap();
        });

    Ok(())
}

#[allow(dead_code)]
fn handle_bankruptcy_for_an_account(
    config: &Config,
    profile: &Profile,
    rpc_client: &RpcClient,
    banks: &HashMap<Pubkey, Bank>,
    marginfi_account_pk: Pubkey,
    marginfi_account: &MarginfiAccount,
    bank_pk: Pubkey,
) -> Result<()> {
    println!("Handling bankruptcy for bank {}", bank_pk);
    let mut handle_bankruptcy_ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingPoolHandleBankruptcy {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.authority(),
            bank: bank_pk,
            marginfi_account: marginfi_account_pk,
            liquidity_vault: find_bank_vault_pda(
                &bank_pk,
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
            insurance_vault: find_bank_vault_pda(
                &bank_pk,
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            insurance_vault_authority: find_bank_vault_authority_pda(
                &bank_pk,
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            token_program: token::ID,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingPoolHandleBankruptcy {}.data(),
    };

    handle_bankruptcy_ix
        .accounts
        .extend(load_observation_account_metas(
            marginfi_account,
            banks,
            vec![bank_pk],
            vec![],
        ));

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let signing_keypairs = config.get_signers(false);

    let message = Message::new(&[handle_bankruptcy_ix], Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Bankruptcy handled (sig: {})", sig),
        Err(err) => println!("Error during bankruptcy handling:\n{:#?}", err),
    };

    Ok(())
}

#[cfg(feature = "admin")]
const BANKRUPTCY_CHUNKS: usize = 4;

#[cfg(feature = "admin")]
pub fn handle_bankruptcy_for_accounts(
    config: &Config,
    profile: &Profile,
    accounts: Vec<Pubkey>,
) -> Result<()> {
    let mut instructions = vec![];
    let rpc_client = config.mfi_program.rpc();

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);

    for account in accounts {
        let marginfi_account = config
            .mfi_program
            .account::<MarginfiAccount>(account)
            .unwrap();

        marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|b| {
                b.active
                    && banks
                        .get(&b.bank_pk)
                        .unwrap()
                        .get_liability_amount(b.liability_shares.into())
                        .unwrap()
                        .is_positive_with_tolerance(ZERO_AMOUNT_THRESHOLD)
            })
            .map(|b| b.bank_pk)
            .collect::<Vec<Pubkey>>()
            .iter()
            .for_each(|bank_pk| {
                instructions.push(
                    make_bankruptcy_ix(
                        config,
                        profile,
                        &banks,
                        account,
                        &marginfi_account,
                        *bank_pk,
                    )
                    .unwrap(),
                );
            });
    }

    println!("Handling {} bankruptcies", instructions.len());

    let chunks = instructions.chunks(BANKRUPTCY_CHUNKS);

    for chunk in chunks {
        let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

        let signing_keypairs = config.get_signers(false);

        let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)];
        ixs.extend_from_slice(chunk);

        let message = Message::new(&ixs, Some(&config.authority()));

        let mut transaction = Transaction::new_unsigned(message);
        transaction.partial_sign(&signing_keypairs, recent_blockhash);

        match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
            Ok(sig) => println!("Bankruptcy handled (sig: {})", sig),
            Err(err) => println!("Error during bankruptcy handling:\n{:#?}", err),
        };
    }

    Ok(())
}
#[cfg(feature = "admin")]
fn make_bankruptcy_ix(
    config: &Config,
    profile: &Profile,
    banks: &HashMap<Pubkey, Bank>,
    marginfi_account_pk: Pubkey,
    marginfi_account: &MarginfiAccount,
    bank_pk: Pubkey,
) -> Result<Instruction> {
    println!("Handling bankruptcy for bank {}", bank_pk);
    let mut handle_bankruptcy_ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingPoolHandleBankruptcy {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.fee_payer.pubkey(),
            bank: bank_pk,
            marginfi_account: marginfi_account_pk,
            liquidity_vault: find_bank_vault_pda(
                &bank_pk,
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
            insurance_vault: find_bank_vault_pda(
                &bank_pk,
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            insurance_vault_authority: find_bank_vault_authority_pda(
                &bank_pk,
                BankVaultType::Insurance,
                &config.program_id,
            )
            .0,
            token_program: token::ID,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingPoolHandleBankruptcy {}.data(),
    };

    handle_bankruptcy_ix
        .accounts
        .extend(load_observation_account_metas(
            marginfi_account,
            banks,
            vec![bank_pk],
            vec![],
        ));

    Ok(handle_bankruptcy_ix)
}

// --------------------------------------------------------------------------------------------------------------------
// bank
// --------------------------------------------------------------------------------------------------------------------

pub fn bank_get(config: Config, bank_pk: Option<Pubkey>) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    if let Some(address) = bank_pk {
        let bank: Bank = config.mfi_program.account(address)?;
        print_bank(&address, &bank);

        let liquidity_vault_balance =
            rpc_client.get_token_account_balance(&bank.liquidity_vault)?;
        let fee_vault_balance = rpc_client.get_token_account_balance(&bank.fee_vault)?;
        let insurance_vault_balance =
            rpc_client.get_token_account_balance(&bank.insurance_vault)?;

        println!("=============");
        println!("Token balances:");
        println!(
            "\tliquidity vault: {} (native: {})",
            liquidity_vault_balance.ui_amount.unwrap(),
            liquidity_vault_balance.amount
        );
        println!(
            "\tfee vault: {} (native: {})",
            fee_vault_balance.ui_amount.unwrap(),
            fee_vault_balance.amount
        );
        println!(
            "\tinsurance vault: {} (native: {})",
            insurance_vault_balance.ui_amount.unwrap(),
            insurance_vault_balance.amount
        );
    } else {
        group_get_all(config)?;
    }
    Ok(())
}

fn load_all_banks(config: &Config, marginfi_group: Option<Pubkey>) -> Result<Vec<(Pubkey, Bank)>> {
    info!("Loading banks for group {:?}", marginfi_group);
    let filters = match marginfi_group {
        Some(marginfi_group) => vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 + size_of::<Pubkey>() + size_of::<u8>(),
            marginfi_group.to_bytes().to_vec(),
        ))],
        None => vec![],
    };

    let mut clock = config.mfi_program.rpc().get_account(&sysvar::clock::ID)?;
    let clock = Clock::from_account_info(&(&sysvar::clock::ID, &mut clock).into_account_info())?;

    let mut banks_with_addresses = config.mfi_program.accounts::<Bank>(filters)?;

    banks_with_addresses.iter_mut().for_each(|(_, bank)| {
        bank.accrue_interest(clock.unix_timestamp).unwrap();
    });

    Ok(banks_with_addresses)
}

pub fn bank_get_all(config: Config, marginfi_group: Option<Pubkey>) -> Result<()> {
    let accounts = load_all_banks(&config, marginfi_group)?;
    for (address, state) in accounts {
        print_bank(&address, &state);
    }
    Ok(())
}

#[cfg(feature = "dev")]
pub fn bank_inspect_price_oracle(config: Config, bank_pk: Pubkey) -> Result<()> {
    use marginfi::state::price::{OraclePriceType, PriceBias};

    let bank: Bank = config.mfi_program.account(bank_pk)?;
    let mut price_oracle_account = config
        .mfi_program
        .rpc()
        .get_account(&bank.config.oracle_keys[0])?;
    let price_oracle_ai =
        (&bank.config.oracle_keys[0], &mut price_oracle_account).into_account_info();

    let opfa =
        OraclePriceFeedAdapter::try_from_bank_config(&bank.config, &[price_oracle_ai], 0, u64::MAX)
            .unwrap();

    let (real_price, maint_asset_price, maint_liab_price, init_asset_price, init_liab_price) = (
        opfa.get_price_of_type(OraclePriceType::RealTime, None)?,
        opfa.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))?,
        opfa.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::High))?,
        opfa.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))?,
        opfa.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))?,
    );

    let keys = bank
        .config
        .oracle_keys
        .iter()
        .filter(|k| k != &&Pubkey::default())
        .collect::<Vec<_>>();

    println!(
        r##"
Oracle Setup: {setup:?}
Oracle Keys: {keys:#?}
Prince:
    Realtime: {real_price}
    Maint: {maint_asset_price} (asset) {maint_liab_price} (liab)
    Init: {init_asset_price} (asset) {init_liab_price} (liab)
    "##,
        setup = bank.config.oracle_setup,
        keys = keys,
        real_price = real_price,
        maint_asset_price = maint_asset_price,
        maint_liab_price = maint_liab_price,
        init_asset_price = init_asset_price,
        init_liab_price = init_liab_price,
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "admin")]
pub fn bank_setup_emissions(
    config: &Config,
    profile: &Profile,
    bank: Pubkey,
    deposits: bool,
    borrows: bool,
    mint: Pubkey,
    rate: f64,
    total: f64,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    let funding_account_ata = get_associated_token_address(&config.authority(), &mint);
    let mut flags = 0;

    if deposits {
        flags |= EMISSIONS_FLAG_LENDING_ACTIVE;
    }

    if borrows {
        flags |= EMISSIONS_FLAG_BORROW_ACTIVE;
    }

    let emissions_mint_decimals = config.mfi_program.rpc().get_account(&mint).unwrap();

    let emissions_mint_decimals =
        spl_token::state::Mint::unpack_from_slice(&emissions_mint_decimals.data)
            .unwrap()
            .decimals;

    let total_emissions = (total * 10u64.pow(emissions_mint_decimals as u32) as f64) as u64;
    let rate = crate::utils::calc_emissions_rate(rate, emissions_mint_decimals);

    println!(
        "Native rate: {} tokens per 1 bank token (UI) per YEAR",
        rate
    );
    println!("Emissions flag: {:b}", flags);
    println!("Total native emissions: {}", total_emissions);

    // Get (y or n) input from user
    println!("Is this correct? (y/n)");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input != "y" {
        println!("Aborting");
        return Ok(());
    }

    let ix = Instruction {
        program_id: marginfi::id(),
        accounts: marginfi::accounts::LendingPoolSetupEmissions {
            marginfi_group: profile.marginfi_group.expect("marginfi group not set"),
            admin: config.authority(),
            bank,
            emissions_mint: mint,
            emissions_auth: find_bank_emssions_auth_pda(bank, mint, marginfi::id()).0,
            emissions_token_account: find_bank_emssions_token_account_pda(
                bank,
                mint,
                marginfi::id(),
            )
            .0,
            emissions_funding_account: funding_account_ata,
            token_program: spl_token::id(),
            system_program: system_program::id(),
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingPoolSetupEmissions {
            flags,
            rate,
            total_emissions,
        }
        .data(),
    };

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let signing_keypairs = config.get_signers(false);

    let message = Message::new(&[ix], Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Tx succeded (sig: {})", sig),
        Err(err) => println!("Error :\n{:#?}", err),
    };

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "admin")]
pub fn bank_update_emissions(
    config: &Config,
    profile: &Profile,
    bank_pk: Pubkey,
    deposits: bool,
    borrows: bool,
    disable: bool,
    rate: Option<f64>,
    additional_emissions: Option<f64>,
) -> Result<()> {
    assert!(!(disable && (deposits || borrows)));

    let rpc_client = config.mfi_program.rpc();

    let bank = config
        .mfi_program
        .account::<Bank>(bank_pk)
        .unwrap_or_else(|_| panic!("Bank {} not found", bank_pk));

    let emission_mint = bank.emissions_mint;
    let funding_account_ata = get_associated_token_address(&config.authority(), &emission_mint);

    let emissions_mint_decimals = config
        .mfi_program
        .rpc()
        .get_account(&emission_mint)
        .unwrap();
    let emissions_mint_decimals =
        spl_token::state::Mint::unpack_from_slice(&emissions_mint_decimals.data)
            .unwrap()
            .decimals;

    let emissions_rate = rate.map(|rate| calc_emissions_rate(rate, emissions_mint_decimals));
    let additional_emissions = additional_emissions
        .map(|emissions| (emissions * 10u64.pow(emissions_mint_decimals as u32) as f64) as u64);
    let emissions_flags = if disable {
        Some(0)
    } else if deposits || borrows {
        let mut flags = 0;

        if deposits {
            flags |= EMISSIONS_FLAG_LENDING_ACTIVE;
        }

        if borrows {
            flags |= EMISSIONS_FLAG_BORROW_ACTIVE;
        }

        Some(flags)
    } else {
        None
    };

    println!(
        "Changes:\n\tRate: {:?}\n\tAdditional emissions: {:?}\n\tFlags: {:?}",
        emissions_rate.map(|rate| format!("{} tokens per 1M bank tokens per YEAR", rate)),
        additional_emissions,
        emissions_flags.map(|flags| format!("{:b}", flags)),
    );

    // Get (y or n) input from user
    println!("Is this correct? (y/n)");

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input != "y" {
        println!("Aborting");
        return Ok(());
    }

    let ix = Instruction {
        program_id: marginfi::id(),
        accounts: marginfi::accounts::LendingPoolUpdateEmissionsParameters {
            marginfi_group: profile.marginfi_group.expect("marginfi group not set"),
            admin: config.authority(),
            bank: bank_pk,
            emissions_mint: emission_mint,
            emissions_token_account: find_bank_emssions_token_account_pda(
                bank_pk,
                emission_mint,
                marginfi::id(),
            )
            .0,
            emissions_funding_account: funding_account_ata,
            token_program: spl_token::id(),
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingPoolUpdateEmissionsParameters {
            emissions_flags,
            emissions_rate,
            additional_emissions,
        }
        .data(),
    };

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let signing_keypairs = config.get_signers(false);

    let message = Message::new(&[ix], Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    match process_transaction(&transaction, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Tx succeded (sig: {})", sig),
        Err(err) => println!("Error:\n{:#?}", err),
    };

    Ok(())
}

#[cfg(feature = "admin")]
pub fn bank_configure(
    config: Config,
    profile: Profile,
    bank_pk: Pubkey,
    bank_config_opt: BankConfigOpt,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    let configure_bank_ixs_builder = config.mfi_program.request();
    let signing_keypairs = config.get_signers(false);

    let mut configure_bank_ixs = configure_bank_ixs_builder
        .accounts(marginfi::accounts::LendingPoolConfigureBank {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.authority(),
            bank: bank_pk,
        })
        .args(marginfi::instruction::LendingPoolConfigureBank {
            bank_config_opt: bank_config_opt.clone(),
        })
        .instructions()?;

    if let Some(oracle) = &bank_config_opt.oracle {
        configure_bank_ixs[0]
            .accounts
            .push(AccountMeta::new_readonly(oracle.keys[0], false));
    }

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let message = Message::new(&configure_bank_ixs, Some(&config.authority()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&signing_keypairs, recent_blockhash);

    let sig = process_transaction(&transaction, &rpc_client, config.get_tx_mode())?;

    println!("Transaction signature: {}", sig);

    Ok(())
}

// --------------------------------------------------------------------------------------------------------------------
// Profile
// --------------------------------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn create_profile(
    name: String,
    cluster: Cluster,
    keypair_path: Option<String>,
    multisig: Option<Pubkey>,
    rpc_url: String,
    program_id: Option<Pubkey>,
    commitment: Option<CommitmentLevel>,
    marginfi_group: Option<Pubkey>,
    marginfi_account: Option<Pubkey>,
) -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let profile = Profile::new(
        name,
        cluster,
        keypair_path,
        multisig,
        rpc_url,
        program_id,
        commitment,
        marginfi_group,
        marginfi_account,
    );
    if !cli_config_dir.exists() {
        fs::create_dir(&cli_config_dir)?;

        let cli_config_file = cli_config_dir.join("config.json");

        fs::write(
            cli_config_file,
            serde_json::to_string(&CliConfig {
                profile_name: profile.name.clone(),
            })?,
        )?;
    }

    let cli_profiles_dir = cli_config_dir.join("profiles");

    if !cli_profiles_dir.exists() {
        fs::create_dir(&cli_profiles_dir)?;
    }

    let profile_file = cli_profiles_dir.join(profile.name.clone() + ".json");
    if profile_file.exists() {
        return Err(anyhow!("Profile {} already exists", profile.name));
    }

    println!("Creating profile {profile:#?}");

    fs::write(&profile_file, serde_json::to_string(&profile)?)?;

    Ok(())
}

pub fn show_profile() -> Result<()> {
    let profile = load_profile()?;
    println!("{profile:?}");
    Ok(())
}

pub fn set_profile(name: String) -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let cli_config_file = cli_config_dir.join("config.json");

    if !cli_config_file.exists() {
        return Err(anyhow!("Profiles not configured, run `mfi profile create`"));
    }

    let profile_file = cli_config_dir.join("profiles").join(format!("{name}.json"));

    if !profile_file.exists() {
        return Err(anyhow!("Profile {} does not exist", name));
    }

    let cli_config = fs::read_to_string(&cli_config_file)?;
    let mut cli_config: CliConfig = serde_json::from_str(&cli_config)?;

    cli_config.profile_name = name;

    fs::write(&cli_config_file, serde_json::to_string(&cli_config)?)?;

    Ok(())
}

pub fn list_profiles() -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let cli_profiles_dir = cli_config_dir.join("profiles");

    if !cli_profiles_dir.exists() {
        return Err(anyhow!("Profiles not configured, run `mfi profile create`"));
    }

    let mut profiles = fs::read_dir(&cli_profiles_dir)?
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<String>>();

    if profiles.is_empty() {
        println!("No profiles exist");
    }

    let cli_config = serde_json::from_str::<CliConfig>(&fs::read_to_string(
        cli_config_dir.join("config.json"),
    )?)?;

    println!("Current profile: {}", cli_config.profile_name);

    profiles.sort();

    println!("Found {} profiles", profiles.len());
    for profile in profiles {
        println!("{profile}");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn configure_profile(
    name: String,
    cluster: Option<Cluster>,
    keypair_path: Option<String>,
    multisig: Option<Pubkey>,
    rpc_url: Option<String>,
    program_id: Option<Pubkey>,
    commitment: Option<CommitmentLevel>,
    group: Option<Pubkey>,
    account: Option<Pubkey>,
) -> Result<()> {
    let mut profile = profile::load_profile_by_name(&name)?;
    profile.config(
        cluster,
        keypair_path,
        multisig,
        rpc_url,
        program_id,
        commitment,
        group,
        account,
    )?;

    Ok(())
}

// --------------------------------------------------------------------------------------------------------------------
// Marginfi Accounts
// --------------------------------------------------------------------------------------------------------------------

pub fn marginfi_account_list(profile: Profile, config: &Config) -> Result<()> {
    let group = profile.marginfi_group.expect("Missing marginfi group");
    let authority = config.authority();

    let banks = HashMap::from_iter(load_all_banks(config, Some(group))?);

    let accounts = config.mfi_program.accounts::<MarginfiAccount>(vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, group.to_bytes().to_vec())),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8 + 32, authority.to_bytes().to_vec())),
    ])?;

    if accounts.is_empty() {
        println!("No marginfi accounts found");
    }

    for (address, marginfi_account) in accounts {
        print_account(
            address,
            marginfi_account,
            banks.clone(),
            profile
                .marginfi_account
                .map_or(false, |default_account| default_account == address),
        )?;
    }

    Ok(())
}

pub fn print_account(
    address: Pubkey,
    marginfi_account: MarginfiAccount,
    banks: HashMap<Pubkey, Bank>,
    default: bool,
) -> Result<()> {
    println!(
        "Address: {} {}",
        address,
        if default { "(default)" } else { "" }
    );
    println!("Lending Account Balances:");
    marginfi_account
        .lending_account
        .get_active_balances_iter()
        .for_each(|balance| {
            let bank = banks.get(&balance.bank_pk).expect("Bank not found");
            let balance_amount = if balance
                .is_empty(marginfi::state::marginfi_account::BalanceSide::Assets)
                .not()
            {
                let native_value = bank.get_asset_amount(balance.asset_shares.into()).unwrap();

                native_value / EXP_10_I80F48[bank.mint_decimals as usize]
            } else if balance
                .is_empty(marginfi::state::marginfi_account::BalanceSide::Liabilities)
                .not()
            {
                let native_value = bank
                    .get_liability_amount(balance.liability_shares.into())
                    .unwrap();

                (native_value / EXP_10_I80F48[bank.mint_decimals as usize]).neg()
            } else {
                I80F48::ZERO
            };

            let mut bank = *bank;
            let mut balance = *balance;

            let mut baw = BankAccountWrapper {
                bank: &mut bank,
                balance: &mut balance,
            };

            // Current timestamp
            let current_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            baw.claim_emissions(current_timestamp).unwrap();

            println!(
                "\tBalance: {:.3}, Bank: {} (mint: {}), Emissions: {}",
                balance_amount,
                balance.bank_pk,
                bank.mint,
                I80F48::from(balance.emissions_outstanding)
            )
        });
    Ok(())
}

pub fn marginfi_account_use(
    mut profile: Profile,
    config: &Config,
    marginfi_account_pk: Pubkey,
) -> Result<()> {
    let group = profile.marginfi_group.expect("Missing marginfi group");
    let authority = config.authority();

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    if marginfi_account.group != group {
        return Err(anyhow!("Marginfi account does not belong to group"));
    }

    if marginfi_account.authority != authority {
        return Err(anyhow!("Marginfi account does not belong to authority"));
    }

    profile.config(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(marginfi_account_pk),
    )?;

    println!("Default marginfi account set to: {marginfi_account_pk}");

    Ok(())
}

/// Print the marginfi account for the provided address or the default marginfi account if none is provided
///
/// If marginfi account address is provided use the group in the marginfi account data, otherwise use the profile defaults
pub fn marginfi_account_get(
    profile: Profile,
    config: &Config,
    marginfi_account_pk: Option<Pubkey>,
) -> Result<()> {
    let marginfi_account_pk =
        marginfi_account_pk.unwrap_or_else(|| profile.marginfi_account.unwrap());

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    let group = marginfi_account.group;

    let banks = HashMap::from_iter(load_all_banks(config, Some(group))?);

    print_account(marginfi_account_pk, marginfi_account, banks, false)?;

    Ok(())
}

pub fn marginfi_account_deposit(
    profile: &Profile,
    config: &Config,
    bank_pk: Pubkey,
    ui_amount: f64,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();
    let signer = config.get_non_ms_authority_keypair()?;
    let marginfi_account_pk = profile.get_marginfi_account();

    let bank = config.mfi_program.account::<Bank>(bank_pk)?;

    let amount = (I80F48::from_num(ui_amount) * EXP_10_I80F48[bank.mint_decimals as usize])
        .floor()
        .to_num::<u64>();

    // Check that bank belongs to the correct group
    if bank.group != profile.marginfi_group.unwrap() {
        bail!("Bank does not belong to group")
    }

    let deposit_ata =
        anchor_spl::associated_token::get_associated_token_address(&signer.pubkey(), &bank.mint);

    let ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingAccountDeposit {
            marginfi_group: profile.marginfi_group.unwrap(),
            marginfi_account: marginfi_account_pk,
            signer: signer.pubkey(),
            bank: bank_pk,
            signer_token_account: deposit_ata,
            bank_liquidity_vault: bank.liquidity_vault,
            token_program: token::ID,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountDeposit { amount }.data(),
    };

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer],
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Deposit successful: {sig}"),
        Err(err) => println!("Error during deposit:\n{err:#?}"),
    }

    Ok(())
}

pub fn marginfi_account_withdraw(
    profile: &Profile,
    config: &Config,
    bank_pk: Pubkey,
    ui_amount: f64,
    withdraw_all: bool,
) -> Result<()> {
    let signer = config.get_non_ms_authority_keypair()?;

    let rpc_client = config.mfi_program.rpc();

    let marginfi_account_pk = profile.get_marginfi_account();

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);
    let bank = banks.get(&bank_pk).expect("Bank not found");

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    let amount = (I80F48::from_num(ui_amount) * EXP_10_I80F48[bank.mint_decimals as usize])
        .floor()
        .to_num::<u64>();

    // Check that bank belongs to the correct group
    if bank.group != profile.marginfi_group.unwrap() {
        bail!("Bank does not belong to group")
    }

    let withdraw_ata =
        anchor_spl::associated_token::get_associated_token_address(&signer.pubkey(), &bank.mint);

    let mut ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingAccountWithdraw {
            marginfi_group: profile.marginfi_group.unwrap(),
            marginfi_account: marginfi_account_pk,
            signer: signer.pubkey(),
            bank: bank_pk,
            bank_liquidity_vault: bank.liquidity_vault,
            token_program: token::ID,
            destination_token_account: withdraw_ata,
            bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                &bank_pk,
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountWithdraw {
            amount,
            withdraw_all: if withdraw_all { Some(true) } else { None },
        }
        .data(),
    };

    ix.accounts.extend(load_observation_account_metas(
        &marginfi_account,
        &banks,
        vec![],
        if withdraw_all { vec![bank_pk] } else { vec![] },
    ));

    let create_ide_ata_ix = create_associated_token_account_idempotent(
        &signer.pubkey(),
        &signer.pubkey(),
        &bank.mint,
        &spl_token::ID,
    );

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ide_ata_ix, ix],
        Some(&signer.pubkey()),
        &[signer],
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Withdraw successful: {sig}"),
        Err(err) => println!("Error during withdraw:\n{err:#?}"),
    }

    Ok(())
}

pub fn marginfi_account_borrow(
    profile: &Profile,
    config: &Config,
    bank_pk: Pubkey,
    ui_amount: f64,
) -> Result<()> {
    let signer = config.get_non_ms_authority_keypair()?;

    let rpc_client = config.mfi_program.rpc();

    let marginfi_account_pk = profile.get_marginfi_account();

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);
    let bank = banks.get(&bank_pk).expect("Bank not found");

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    let amount = (I80F48::from_num(ui_amount) * EXP_10_I80F48[bank.mint_decimals as usize])
        .floor()
        .to_num::<u64>();

    // Check that bank belongs to the correct group
    if bank.group != profile.marginfi_group.unwrap() {
        bail!("Bank does not belong to group")
    }

    let withdraw_ata =
        anchor_spl::associated_token::get_associated_token_address(&signer.pubkey(), &bank.mint);

    let mut ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingAccountBorrow {
            marginfi_group: profile.marginfi_group.unwrap(),
            marginfi_account: marginfi_account_pk,
            signer: signer.pubkey(),
            bank: bank_pk,
            bank_liquidity_vault: bank.liquidity_vault,
            token_program: token::ID,
            destination_token_account: withdraw_ata,
            bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                &bank_pk,
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountBorrow { amount }.data(),
    };

    ix.accounts.extend(load_observation_account_metas(
        &marginfi_account,
        &banks,
        vec![bank_pk],
        vec![],
    ));

    let create_ide_ata_ix = create_associated_token_account_idempotent(
        &signer.pubkey(),
        &signer.pubkey(),
        &bank.mint,
        &spl_token::ID,
    );

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ide_ata_ix, ix],
        Some(&signer.pubkey()),
        &[signer],
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.get_tx_mode()) {
        Ok(sig) => println!("Borrow successful: {sig}"),
        Err(err) => println!("Error during borrow:\n{err:#?}"),
    }

    Ok(())
}

pub fn marginfi_account_liquidate(
    profile: &Profile,
    config: &Config,
    liquidatee_marginfi_account_pk: Pubkey,
    asset_bank_pk: Pubkey,
    liability_bank_pk: Pubkey,
    ui_asset_amount: f64,
) -> Result<()> {
    let signer = config.get_non_ms_authority_keypair()?;

    let rpc_client = config.mfi_program.rpc();

    let marginfi_account_pk = profile.get_marginfi_account();

    let banks = HashMap::from_iter(load_all_banks(
        config,
        Some(profile.marginfi_group.unwrap()),
    )?);
    let asset_bank = banks.get(&asset_bank_pk).expect("Asset bank not found");
    let liability_bank = banks
        .get(&liability_bank_pk)
        .expect("Liability bank not found");

    let marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(marginfi_account_pk)?;

    let liquidatee_marginfi_account = config
        .mfi_program
        .account::<MarginfiAccount>(liquidatee_marginfi_account_pk)?;

    let asset_amount = (I80F48::from_num(ui_asset_amount)
        * EXP_10_I80F48[asset_bank.mint_decimals as usize])
        .floor()
        .to_num::<u64>();

    // Check that banks belong to the correct group
    if asset_bank.group != profile.marginfi_group.unwrap() {
        bail!("Asset bank does not belong to group")
    }
    if liability_bank.group != profile.marginfi_group.unwrap() {
        bail!("Liability bank does not belong to group")
    }

    let mut ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::LendingAccountLiquidate {
            marginfi_group: profile.marginfi_group.unwrap(),
            asset_bank: asset_bank_pk,
            liab_bank: liability_bank_pk,
            liquidator_marginfi_account: marginfi_account_pk,
            signer: signer.pubkey(),
            liquidatee_marginfi_account: liquidatee_marginfi_account_pk,
            bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                &liability_bank_pk,
                BankVaultType::Liquidity,
                &config.program_id,
            )
            .0,
            bank_liquidity_vault: liability_bank.liquidity_vault,
            bank_insurance_vault: liability_bank.insurance_vault,
            token_program: token::ID,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountLiquidate { asset_amount }.data(),
    };

    ix.accounts.push(AccountMeta {
        pubkey: asset_bank.config.oracle_keys[0],
        is_signer: false,
        is_writable: false,
    });
    ix.accounts.push(AccountMeta {
        pubkey: liability_bank.config.oracle_keys[0],
        is_signer: false,
        is_writable: false,
    });
    ix.accounts.extend(load_observation_account_metas(
        &marginfi_account,
        &banks,
        vec![liability_bank_pk, asset_bank_pk],
        vec![],
    ));
    ix.accounts.extend(load_observation_account_metas(
        &liquidatee_marginfi_account,
        &banks,
        vec![],
        vec![],
    ));

    let cu_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix, cu_ix],
        Some(&signer.pubkey()),
        &[signer],
        recent_blockhash,
    );

    match process_transaction(&tx, &config.mfi_program.rpc(), config.get_tx_mode()) {
        Ok(sig) => println!("Liquidation successful: {sig}"),
        Err(err) => println!("Error during liquidation:\n{err:#?}"),
    }

    Ok(())
}

pub fn marginfi_account_create(profile: &Profile, config: &Config) -> Result<()> {
    let signer = config.get_non_ms_authority_keypair()?;

    let rpc_client = config.mfi_program.rpc();

    let marginfi_account_key = Keypair::new();

    let ix = Instruction {
        program_id: config.program_id,
        accounts: marginfi::accounts::MarginfiAccountInitialize {
            marginfi_group: profile.marginfi_group.unwrap(),
            marginfi_account: marginfi_account_key.pubkey(),
            system_program: system_program::ID,
            authority: signer.pubkey(),
            fee_payer: signer.pubkey(),
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiAccountInitialize.data(),
    };

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer, &marginfi_account_key],
        recent_blockhash,
    );

    let marginfi_account_pk = marginfi_account_key.pubkey();

    match process_transaction(&tx, &config.mfi_program.rpc(), config.get_tx_mode()) {
        Ok(_sig) => print!("{marginfi_account_pk}"),
        Err(err) => println!("Error during initialize:\n{err:#?}"),
    }

    let mut profile = profile.clone();

    profile.config(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(marginfi_account_key.pubkey()),
    )?;

    Ok(())
}
/// LIP
///

#[cfg(feature = "lip")]
pub fn process_list_lip_campaigns(config: &Config) {
    let campaings = config.lip_program.accounts::<Campaign>(vec![]).unwrap();

    print!("Found {} campaigns", campaings.len());

    campaings.iter().for_each(|(address, campaign)| {
        let bank = config
            .mfi_program
            .account::<Bank>(campaign.marginfi_bank_pk)
            .unwrap();

        print!(
            r#"
Campaign: {}
Bank: {}
Mint: {}
Total Capacity: {}
Remaining Capacity: {}
Lockup Period: {} days
Max Rewards: {}
"#,
            address,
            campaign.marginfi_bank_pk,
            bank.mint,
            campaign.max_deposits as f32 / 10.0_f32.powi(bank.mint_decimals as i32),
            campaign.remaining_capacity as f32 / 10.0_f32.powi(bank.mint_decimals as i32),
            campaign.lockup_period / (24 * 60 * 60),
            campaign.max_rewards as f32 / 10.0_f32.powi(bank.mint_decimals as i32),
        );
    });
}

#[cfg(feature = "lip")]
pub fn process_list_deposits(config: &Config) {
    use solana_sdk::clock::SECONDS_PER_DAY;

    let mut deposits = config.lip_program.accounts::<Deposit>(vec![]).unwrap();
    let campaings = HashMap::<Pubkey, Campaign>::from_iter(
        config.lip_program.accounts::<Campaign>(vec![]).unwrap(),
    );
    let banks =
        HashMap::<Pubkey, Bank>::from_iter(config.mfi_program.accounts::<Bank>(vec![]).unwrap());

    deposits.sort_by(|(_, a), (_, b)| a.start_time.cmp(&b.start_time));

    deposits.iter().for_each(|(address, deposit)| {
        let campaign = campaings.get(&deposit.campaign).unwrap();
        let bank = banks.get(&campaign.marginfi_bank_pk).unwrap();

        let time_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let end_time = deposit.start_time as u64 + campaign.lockup_period;
        let maturity_string = {
            if time_now > end_time {
                "mature".to_owned()
            } else {
                let days_to_maturity = end_time.saturating_sub(time_now) / SECONDS_PER_DAY;
                format!("mature in {} days", days_to_maturity)
            }
        };

        println!(
            r#"
Deposit: {},
Campaign: {},
Asset Mint: {},
Owner: {},
Amount: {},
Deposit start {}, end {} ({})
"#,
            address,
            deposit.campaign,
            bank.mint,
            deposit.owner,
            deposit.amount as f32 / 10.0_f32.powi(bank.mint_decimals as i32),
            timestamp_to_string(deposit.start_time),
            timestamp_to_string(end_time as i64),
            maturity_string,
        )
    })
}

#[cfg(feature = "lip")]
fn timestamp_to_string(timestamp: i64) -> String {
    DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(),
        Utc,
    )
    .format("%Y-%m-%d %H:%M:%S")
    .to_string()
}

// Switchboard tests
#[cfg(feature = "dev")]
pub fn process_inspect_switchboard_feed(config: &Config, aggregator_pk: &Pubkey) {
    let aggregator_account_data = config
        .mfi_program
        .rpc()
        .get_account_data(aggregator_pk)
        .expect("Aggregator account not found");

    let aggregator_account =
        switchboard_v2::AggregatorAccountData::new_from_bytes(&aggregator_account_data)
            .expect("Invalid aggregator account data");

    println!("Aggregator account: {:#?}", aggregator_account);
}
