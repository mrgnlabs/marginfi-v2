use crate::{
    config::Config,
    profile::{self, get_cli_config_dir, load_profile, CliConfig, Profile},
    utils::{
        create_oracle_key_array, find_bank_vault_authority_pda, find_bank_vault_pda,
        process_transaction,
    },
};
use anchor_client::Cluster;
use anchor_spl::token;
use anyhow::Result;
use anyhow::{anyhow, bail};
use fixed::types::I80F48;
use marginfi::{
    prelude::{GroupConfig, MarginfiGroup},
    state::marginfi_group::{
        Bank, BankConfig, BankConfigOpt, BankOperationalState, BankVaultType, InterestRateConfig,
        OracleSetup, WrappedI80F48,
    },
};

use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program, sysvar,
    transaction::Transaction,
};
use std::{fs, mem::size_of};

// --------------------------------------------------------------------------------------------------------------------
// marginfi group
// --------------------------------------------------------------------------------------------------------------------

pub fn group_get(config: Config, marginfi_group: Option<Pubkey>) -> Result<()> {
    if let Some(marginfi_group) = marginfi_group {
        // let rpc_client = config.program.rpc();

        let account: MarginfiGroup = config.program.account(marginfi_group)?;
        println!("Address: {}", marginfi_group);
        println!("=============");
        println!("Raw data:");
        println!("{:#?}", account);

        print_group_banks(config, marginfi_group)?;
    } else {
        group_get_all(config)?;
    }
    Ok(())
}

pub fn group_get_all(config: Config) -> Result<()> {
    let accounts: Vec<(Pubkey, MarginfiGroup)> = config.program.accounts(vec![])?;
    for (address, state) in accounts {
        println!("-> {}:\n{:#?}\n", address, state);
    }
    Ok(())
}

pub fn print_group_banks(config: Config, marginfi_group: Pubkey) -> Result<()> {
    let banks = config
        .program
        .accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
            offset: 8 + size_of::<Pubkey>() + size_of::<u8>(),
            bytes: MemcmpEncodedBytes::Bytes(marginfi_group.to_bytes().to_vec()),
            encoding: None,
        })])?;

    println!("--------\nBanks:");

    for (address, state) in banks {
        println!("{}:\n{:#?}\n", address, state);
    }

    Ok(())
}

pub fn group_create(
    config: Config,
    profile: Profile,
    admin: Option<Pubkey>,
    override_existing_profile_group: bool,
) -> Result<()> {
    let rpc_client = config.program.rpc();
    let admin = admin.unwrap_or_else(|| config.payer.pubkey());

    if profile.marginfi_group.is_some() && !override_existing_profile_group {
        bail!(
            "Marginfi group already exists for profile [{}]",
            profile.name
        );
    }

    let marginfi_group_keypair = Keypair::new();

    let init_marginfi_group_ix = config
        .program
        .request()
        .signer(&config.payer)
        .accounts(marginfi::accounts::MarginfiGroupInitialize {
            marginfi_group: marginfi_group_keypair.pubkey(),
            admin,
            system_program: system_program::id(),
        })
        .args(marginfi::instruction::MarginfiGroupInitialize {})
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let signers = vec![&config.payer, &marginfi_group_keypair];
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &init_marginfi_group_ix,
        Some(&config.payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.dry_run) {
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

pub fn group_configure(config: Config, profile: Profile, admin: Option<Pubkey>) -> Result<()> {
    let rpc_client = config.program.rpc();

    if profile.marginfi_group.is_none() {
        bail!("Marginfi group not specified in profile [{}]", profile.name);
    }

    let configure_marginfi_group_ix = config
        .program
        .request()
        .signer(&config.payer)
        .accounts(marginfi::accounts::MarginfiGroupConfigure {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.payer.pubkey(),
        })
        .args(marginfi::instruction::MarginfiGroupConfigure {
            config: GroupConfig { admin },
        })
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let signers = vec![&config.payer];
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &configure_marginfi_group_ix,
        Some(&config.payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.dry_run) {
        Ok(sig) => println!("marginfi group created (sig: {})", sig),
        Err(err) => println!("Error during marginfi group creation:\n{:#?}", err),
    };

    Ok(())
}

pub fn group_add_bank(
    config: Config,
    profile: Profile,
    bank_mint: Pubkey,
    pyth_oracle: Pubkey,
    asset_weight_init: f64,
    asset_weight_maint: f64,
    liability_weight_init: f64,
    liability_weight_maint: f64,
    max_capacity: u64,
    optimal_utilization_rate: f64,
    plateau_interest_rate: f64,
    max_interest_rate: f64,
    insurance_fee_fixed_apr: f64,
    insurance_ir_fee: f64,
    protocol_fixed_fee_apr: f64,
    protocol_ir_fee: f64,
) -> Result<()> {
    let rpc_client = config.program.rpc();

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

    let add_bank_ix = config
        .program
        .request()
        .signer(&config.payer)
        .accounts(marginfi::accounts::LendingPoolAddBank {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.payer.pubkey(),
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
        })
        .accounts(AccountMeta::new_readonly(pyth_oracle, false))
        .args(marginfi::instruction::LendingPoolAddBank {
            bank_config: BankConfig {
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                max_capacity,
                interest_rate_config,
                operational_state: BankOperationalState::Operational,
                oracle_setup: OracleSetup::Pyth,
                oracle_keys: create_oracle_key_array(pyth_oracle),
                ..BankConfig::default()
            },
        })
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let signers = vec![&config.payer, &bank_keypair];
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &add_bank_ix,
        Some(&config.payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.dry_run) {
        Ok(sig) => println!("bank created (sig: {})", sig),
        Err(err) => println!("Error during bank creation:\n{:#?}", err),
    };

    println!("New {} bank: {}", bank_mint, bank_keypair.pubkey());

    Ok(())
}

// --------------------------------------------------------------------------------------------------------------------
// bank
// --------------------------------------------------------------------------------------------------------------------

pub fn bank_get(config: Config, bank: Option<Pubkey>) -> Result<()> {
    let rpc_client = config.program.rpc();

    if let Some(bank) = bank {
        let account: Bank = config.program.account(bank)?;
        println!("Address: {}", bank);
        println!("=============");
        println!("Raw data:");
        println!("{:#?}", account);

        let liquidity_vault_balance =
            rpc_client.get_token_account_balance(&account.liquidity_vault)?;
        let fee_vault_balance = rpc_client.get_token_account_balance(&account.fee_vault)?;
        let insurance_vault_balance =
            rpc_client.get_token_account_balance(&account.insurance_vault)?;

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

pub fn bank_get_all(config: Config, marginfi_group: Option<Pubkey>) -> Result<()> {
    let filters = match marginfi_group {
        Some(marginfi_group) => vec![RpcFilterType::Memcmp(Memcmp {
            bytes: MemcmpEncodedBytes::Base58(marginfi_group.to_string()),
            offset: 8,
            encoding: None,
        })],
        None => vec![],
    };

    let accounts: Vec<(Pubkey, Bank)> = config.program.accounts(filters)?;
    for (address, state) in accounts {
        println!("-> {}:\n{:#?}\n", address, state);
    }
    Ok(())
}

// --------------------------------------------------------------------------------------------------------------------
// Profile
// --------------------------------------------------------------------------------------------------------------------

pub fn create_profile(
    name: String,
    cluster: Cluster,
    keypair_path: String,
    rpc_url: String,
    program_id: Option<Pubkey>,
    commitment: Option<CommitmentLevel>,
    marginfi_group: Option<Pubkey>,
) -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let profile = Profile::new(
        name,
        cluster,
        keypair_path,
        rpc_url,
        program_id,
        commitment,
        marginfi_group,
    );
    if !cli_config_dir.exists() {
        fs::create_dir(&cli_config_dir)?;

        let cli_config_file = cli_config_dir.join("config.json");

        fs::write(
            &cli_config_file,
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

    println!("Creating profile {:#?}", profile);

    fs::write(&profile_file, serde_json::to_string(&profile)?)?;

    Ok(())
}

pub fn show_profile() -> Result<()> {
    let profile = load_profile()?;
    println!("{:?}", profile);
    Ok(())
}

pub fn set_profile(name: String) -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let cli_config_file = cli_config_dir.join("config.json");

    if !cli_config_file.exists() {
        return Err(anyhow!("Profiles not configured, run `mfi profile create`"));
    }

    let profile_file = cli_config_dir
        .join("profiles")
        .join(format!("{}.json", name));

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
        println!("{}", profile);
    }

    Ok(())
}

pub fn configure_profile(
    name: String,
    cluster: Option<Cluster>,
    keypair_path: Option<String>,
    rpc_url: Option<String>,
    program_id: Option<Pubkey>,
    commitment: Option<CommitmentLevel>,
    group: Option<Pubkey>,
) -> Result<()> {
    let mut profile = profile::load_profile_by_name(&name)?;
    profile.config(
        cluster,
        keypair_path,
        rpc_url,
        program_id,
        commitment,
        group,
    )?;

    Ok(())
}

pub fn bank_configure(
    config: Config,
    profile: Profile,
    bank_pk: Pubkey,
    bank_config_opt: BankConfigOpt,
) -> Result<()> {
    let configure_bank_ix = config
        .program
        .request()
        .signer(&config.payer)
        .accounts(marginfi::accounts::LendingPoolConfigureBank {
            marginfi_group: profile.marginfi_group.unwrap(),
            admin: config.payer.pubkey(),
            bank: bank_pk,
        })
        .args(marginfi::instruction::LendingPoolConfigureBank { bank_config_opt })
        .instructions()?;

    let transaction = Transaction::new_signed_with_payer(
        &configure_bank_ix,
        Some(&config.payer.pubkey()),
        &[&config.payer],
        config.program.rpc().get_latest_blockhash().unwrap(),
    );

    let sig = process_transaction(&transaction, &config.program.rpc(), config.dry_run)?;

    println!("Transaction signature: {}", sig);

    Ok(())
}
