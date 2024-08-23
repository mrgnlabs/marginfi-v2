use {
    crate::config::{CliSigner, Config, GlobalOptions},
    anchor_client::{Client, Cluster},
    anyhow::{anyhow, bail, Result},
    dirs::home_dir,
    serde::{Deserialize, Serialize},
    solana_sdk::{
        commitment_config::{CommitmentConfig, CommitmentLevel},
        pubkey,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
    },
    std::{fs, path::PathBuf},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub cluster: Cluster,
    pub keypair_path: String,
    pub multisig: Option<Pubkey>,
    pub rpc_url: String,
    pub program_id: Option<Pubkey>,
    pub commitment: Option<CommitmentLevel>,
    pub marginfi_group: Option<Pubkey>,
    pub marginfi_account: Option<Pubkey>,
}

#[derive(Serialize, Deserialize)]
pub struct CliConfig {
    pub profile_name: String,
}

impl Profile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        cluster: Cluster,
        keypair_path: String,
        multisig: Option<Pubkey>,
        rpc_url: String,
        program_id: Option<Pubkey>,
        commitment: Option<CommitmentLevel>,
        marginfi_group: Option<Pubkey>,
        marginfi_account: Option<Pubkey>,
    ) -> Self {
        Profile {
            name,
            cluster,
            keypair_path,
            multisig,
            rpc_url,
            program_id,
            commitment,
            marginfi_group,
            marginfi_account,
        }
    }

    pub fn get_config(&self, global_options: Option<&GlobalOptions>) -> Result<Config> {
        let fee_payer = read_keypair_file(&*shellexpand::tilde(&self.keypair_path))
            .expect("Example requires a keypair file");

        let multisig = self.multisig;

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
                Cluster::Devnet => pubkey!("mf2iDQbVTAE3tT4tgAZBhBAmKUW56GsXX7H3oeH4atr"),
                Cluster::Mainnet => pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA"),
                _ => bail!("cluster {:?} does not have a default target program ID, please provide it through the --pid option", cluster)
            }
        }
    };
        let commitment = CommitmentConfig {
            commitment: self.commitment.unwrap_or(CommitmentLevel::Processed),
        };
        let client = Client::new_with_options(
            Cluster::Custom(self.rpc_url.clone(), "https://dontcare.com:123".to_string()),
            CliSigner::Keypair(Keypair::new()),
            commitment,
        );
        let program = client.program(program_id).unwrap();
        let lip_program = client
            .program(match cluster {
                Cluster::Mainnet => pubkey!("LipsxuAkFkwa4RKNzn51wAsW7Dedzt1RNHMkTkDEZUW"),
                Cluster::Devnet => pubkey!("sexyDKo4Khm38YdJeiRdNNd5aMQqNtfDkxv7MnYNFeU"),
                _ => bail!(
                    "cluster {:?} doesn't have a default program ID for the LIP",
                    cluster
                ),
            })
            .unwrap();

        Ok(Config {
            cluster,
            fee_payer,
            multisig,
            program_id,
            commitment,
            dry_run,
            client,
            mfi_program: program,
            lip_program,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn config(
        &mut self,
        new_name: Option<String>,
        cluster: Option<Cluster>,
        keypair_path: Option<String>,
        multisig: Option<Pubkey>,
        rpc_url: Option<String>,
        program_id: Option<Pubkey>,
        commitment: Option<CommitmentLevel>,
        group: Option<Pubkey>,
        account: Option<Pubkey>,
    ) -> Result<()> {
        if let Some(name) = new_name {
            self.name = name;
        }

        if let Some(cluster) = cluster {
            self.cluster = cluster;
        }

        if let Some(keypair_path) = keypair_path {
            self.keypair_path = keypair_path;
        }

        if let Some(multisig) = multisig {
            self.multisig = Some(multisig);
        }

        if let Some(rpc_url) = rpc_url {
            self.rpc_url = rpc_url;
        }

        if let Some(program_id) = program_id {
            self.program_id = Some(program_id);
        }

        if let Some(commitment) = commitment {
            self.commitment = Some(commitment);
        }

        if let Some(group) = group {
            self.marginfi_group = Some(group);
        }

        if let Some(account) = account {
            self.marginfi_account = Some(account);
        }

        self.write_to_file()?;

        Ok(())
    }

    pub fn get_marginfi_account(&self) -> Pubkey {
        self.marginfi_account
            .unwrap_or_else(|| panic!("No marginfi account set for profile \"{}\"", self.name))
    }

    pub fn set_marginfi_group(&mut self, address: Pubkey) -> Result<()> {
        self.marginfi_group = Some(address);
        self.write_to_file()?;

        Ok(())
    }

    fn write_to_file(&self) -> Result<()> {
        let cli_config_dir = get_cli_config_dir();
        let cli_profiles_dir = cli_config_dir.join("profiles");
        let profile_file = cli_profiles_dir.join(self.name.clone() + ".json");

        fs::write(profile_file, serde_json::to_string(&self)?)?;

        Ok(())
    }
}

pub fn load_profile() -> Result<Profile> {
    let cli_config_dir = get_cli_config_dir();
    let cli_config_file = cli_config_dir.join("config.json");

    if !cli_config_file.exists() {
        return Err(anyhow!("Profiles not configured, run `mfi profile create`"));
    }

    let cli_config = fs::read_to_string(&cli_config_file)?;
    let cli_config: CliConfig = serde_json::from_str(&cli_config)?;

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

pub fn load_profile_by_name(name: &str) -> Result<Profile> {
    let cli_config_dir = get_cli_config_dir();
    let profile_file = cli_config_dir.join("profiles").join(format!("{name}.json"));

    if !profile_file.exists() {
        return Err(anyhow!("Profile {} does not exist", name));
    }

    let profile = fs::read_to_string(&profile_file)?;
    let profile: Profile = serde_json::from_str(&profile)?;

    Ok(profile)
}

pub fn delete_profile_by_name(name: &str) -> Result<()> {
    let cli_config_dir = get_cli_config_dir();
    let profile_file = cli_config_dir.join("profiles").join(format!("{name}.json"));

    if !profile_file.exists() {
        return Err(anyhow!("Profile {} does not exist", name));
    }

    match fs::remove_file(profile_file) {
        Ok(()) => {
            println!("successfully deleted profile {name}");
            Ok(())
        }
        Err(e) => {
            println!("failed to delete profile {name}: {e:?}");
            Err(e.into())
        }
    }
}

pub fn get_cli_config_dir() -> PathBuf {
    home_dir()
        .expect("$HOME not set")
        .as_path()
        .join(".config/mfi-cli")
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.get_config(None).map_err(|_| std::fmt::Error)?;
        write!(
            f,
            r#"
Profile:
    Name: {}
    Program: {}
    Marginfi Group: {}
    Marginfi Account: {}
    Cluster: {}
    Rpc URL: {}
    Fee Payer: {}
    Authority: {}
    Keypair: {}
    Multisig: {}
        "#,
            self.name,
            config.program_id,
            self.marginfi_group
                .map(|x| x.to_string())
                .unwrap_or_else(|| "None".to_owned()),
            self.marginfi_account
                .map(|x| x.to_string())
                .unwrap_or_else(|| "None".to_owned()),
            self.cluster,
            self.rpc_url,
            config.explicit_fee_payer(),
            config.authority(),
            self.keypair_path.clone(),
            self.multisig
                .map(|x| x.to_string())
                .unwrap_or_else(|| "None".to_owned()),
        )?;

        Ok(())
    }
}
