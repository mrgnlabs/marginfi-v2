use crate::{
    config::Config,
    profile::{get_cli_config_dir, load_profile, CliConfig, Profile},
    utils::process_transaction,
};
use anchor_client::Cluster;
use anyhow::Result;
use anyhow::{anyhow, bail};
use marginfi::prelude::MarginfiGroup;
use solana_sdk::{
    commitment_config::CommitmentLevel, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction, system_program,
};
use std::fs;

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

pub fn group_create(config: Config, profile: Profile, admin: Option<Pubkey>) -> Result<()> {
    let rpc_client = config.program.rpc();
    let admin = admin.unwrap_or_else(|| config.payer.pubkey());

    if profile.marginfi_group.is_some() {
        bail!(
            "Marginfi group already exists for profile [{}]",
            profile.name
        );
    }

    let marginfi_group_keypair = Keypair::new();
    let account_size = 8 + std::mem::size_of::<MarginfiGroup>();
    let rent_exemption_amount = rpc_client.get_minimum_balance_for_rent_exemption(account_size)?;

    let create_marginfi_group_ix = system_instruction::create_account(
        &admin,
        &marginfi_group_keypair.pubkey(),
        rent_exemption_amount,
        account_size as u64,
        &marginfi::id(),
    );

    let mut init_marginfi_group_ix = config
        .program
        .request()
        .signer(&config.payer)
        .accounts(marginfi::accounts::InitializeMarginfiGroup {
            marginfi_group: marginfi_group_keypair.pubkey(),
            admin,
            system_program: system_program::id(),
        })
        .args(marginfi::instruction::InitializeMarginfiGroup {})
        .instructions()?;

    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let mut ixs = vec![create_marginfi_group_ix];
    ixs.append(&mut init_marginfi_group_ix);

    let signers = vec![&config.payer, &marginfi_group_keypair];
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &ixs,
        Some(&config.payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    match process_transaction(&tx, &rpc_client, config.dry_run) {
        Ok(sig) => println!("marginfi group created (sig: {})", sig),
        Err(err) => println!("Error during marginfi group creation:\n{:#?}", err),
    };

    let mut profile = profile;
    profile.set_marginfi_group(marginfi_group_keypair.pubkey())?;

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
        return Err(anyhow!("Profiles not configured, run `bb profile set`"));
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
        return Err(anyhow!("Profiles not configured, run `bb profile set`"));
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
