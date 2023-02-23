use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Target {
    pub address: Pubkey,
    pub before: Option<Signature>,
    pub until: Option<Signature>,
}

// Allows to parse a JSON target with base58-encoded addresses/sigs (serde expects byte arrays)
impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let targets_raw = json::parse(s).unwrap();

        Ok(Self {
            address: Pubkey::from_str(targets_raw["address"].as_str().unwrap()).unwrap(),
            before: targets_raw["before"]
                .as_str()
                .map(|sig_str| Signature::from_str(sig_str).unwrap()),
            until: targets_raw["until"]
                .as_str()
                .map(|sig_str| Signature::from_str(sig_str).unwrap()),
        })
    }
}

pub const DEFAULT_RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_SIGNATURE_FETCH_LIMIT: usize = 1_000;
pub const DEFAULT_MAX_PENDING_SIGNATURES: usize = 10_000;
pub const DEFAULT_MONITOR_INTERVAL: u64 = 5;
