use crate::config::{Config, GlobalOptions};
use anchor_client::{Client, Cluster};
use anyhow::bail;
use anyhow::{anyhow, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey,
    pubkey::Pubkey,
    signature::read_keypair_file,
    signer::Signer,
};
use std::{fs, path::PathBuf, rc::Rc};

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub cluster: Cluster,
    pub keypair_path: String,
    pub rpc_url: String,
    pub program_id: Option<Pubkey>,
    pub commitment: Option<CommitmentLevel>,
    pub marginfi_group: Option<Pubkey>,
}

#[derive(Serialize, Deserialize)]
pub struct BBConfig {
    pub profile_name: String,
}

impl Profile {
    pub fn new(
        name: String,
        cluster: Cluster,
        keypair_path: String,
        rpc_url: String,
        program_id: Option<Pubkey>,
        commitment: Option<CommitmentLevel>,
        marginfi_group: Option<Pubkey>,
    ) -> Self {
        Profile {
            name,
            cluster,
            keypair_path,
            rpc_url,
            program_id,
            commitment,
            marginfi_group,
        }
    }

    pub fn config(&self, global_options: Option<&GlobalOptions>) -> Result<Config> {
        let wallet_path = self.keypair_path.clone();
        let payer = read_keypair_file(&*shellexpand::tilde(&wallet_path))
            .expect("Example requires a keypair file");
        let payer_clone = read_keypair_file(&*shellexpand::tilde(&wallet_path))
            .expect("Example requires a keypair file");

        let dry_run = match global_options {
            Some(options) => options.dry_run,
            None => false,
        };
        let cluster = self.cluster.clone();
        let program_id = match self.program_id {
        Some(pid) => pid,
        None => {
            match cluster {
                Cluster::Localnet => pubkey!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"),
                Cluster::Devnet => pubkey!("gTREErUgHQiHj6KYEZrZEpzYYYxvfKfXZnMCpBsxfcT"),
                _ => bail!("cluster {:?} does not have a default target program ID, please provide it through the --pid option", cluster)
            }
        }
    };
        let commitment = CommitmentConfig {
            commitment: self.commitment.unwrap_or(CommitmentLevel::Processed),
        };
        let client = Client::new_with_options(
            Cluster::Custom(self.rpc_url.clone(), "https://dontcare.com:123".to_string()),
            Rc::new(payer_clone),
            commitment,
        );
        let program = client.program(program_id);

        Ok(Config {
            cluster,
            payer,
            program_id,
            commitment,
            dry_run,
            client,
            program,
        })
    }

    pub fn get_marginfi_group(&self) -> Pubkey {
        self.marginfi_group.unwrap_or_else(|| {
            panic!(
                "marginfi group address not set in profile \"{}\"",
                self.name
            )
        })
    }

    pub fn set_marginfi_group(&mut self, address: Pubkey) -> Result<()> {
        self.marginfi_group = Some(address);
        let bb_config_dir = get_cli_config_dir();
        let bb_profiles_dir = bb_config_dir.join("profiles");
        let profile_file = bb_profiles_dir.join(self.name.clone() + ".json");

        fs::write(&profile_file, serde_json::to_string(&self)?)?;

        Ok(())
    }
}

pub fn load_profile() -> Result<Profile> {
    let cli_config_dir = get_cli_config_dir();
    let cli_config_file = cli_config_dir.join("config.json");

    if !cli_config_file.exists() {
        return Err(anyhow!("Profiles not configured, run `mfi profile set`"));
    }

    let cli_config = fs::read_to_string(&cli_config_file)?;
    let cli_config: BBConfig = serde_json::from_str(&cli_config)?;

    let profile_file = cli_config_dir
        .join("profiles")
        .join(format!("{}.json", cli_config.profile_name));

    if !profile_file.exists() {
        return Err(anyhow!(
            "Profile {} does not exist",
            cli_config.profile_name
        ));
    }

    let profile = fs::read_to_string(&profile_file)?;
    let profile: Profile = serde_json::from_str(&profile)?;

    Ok(profile)
}

pub fn get_cli_config_dir() -> PathBuf {
    home_dir()
        .expect("$HOME not set")
        .as_path()
        .join(".config/mfi-cli")
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.config(None).map_err(|_| std::fmt::Error)?;
        write!(
            f,
            r#"
Profile:
    Name: {}
    Program: {}
    Marginfi Group: {}
    Cluster: {}
    Rpc URL: {}
    Signer: {}
    Keypair: {}
        "#,
            self.name,
            config.program_id,
            self.marginfi_group
                .map(|x| x.to_string())
                .unwrap_or("None".to_owned()),
            self.cluster,
            self.rpc_url,
            config.payer.pubkey(),
            self.keypair_path,
        )?;

        Ok(())
    }
}
