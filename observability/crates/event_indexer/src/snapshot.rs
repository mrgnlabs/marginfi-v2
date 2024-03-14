use std::{collections::HashMap, fmt::Display};

use anchor_lang::{AccountDeserialize, Discriminator};
use marginfi::state::{
    marginfi_account::MarginfiAccount,
    marginfi_group::{Bank, MarginfiGroup},
};
use rpc_utils::{get_multiple_accounts_chunked, AccountWithSlot};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
};
use tracing::{info, warn};

use crate::error::IndexingError;

pub struct BankRow {
    pub address: Pubkey,
    pub mint: Pubkey,
    pub mint_decimals: u8,
}

pub struct UserRow {
    pub address: Pubkey,
}

pub struct AccountRow {
    pub address: Pubkey,
    pub user: Pubkey,
}

pub struct Snapshot {
    program_id: Pubkey,
    rpc_client: RpcClient,
    pub banks: HashMap<Pubkey, (u64, BankRow)>,
    pub users: HashMap<Pubkey, (u64, UserRow)>,
    pub accounts: HashMap<Pubkey, (u64, AccountRow)>,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Snapshot:\n- Banks: {}\n- Users: {}\n- Accounts: {}",
            self.banks.len(),
            self.users.len(),
            self.accounts.len(),
        )
    }
}

impl Snapshot {
    pub fn new(program_id: Pubkey, rpc_endpoint: String) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_endpoint.clone(),
            CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            },
        );

        Self {
            program_id,
            rpc_client,
            banks: HashMap::new(),
            users: HashMap::new(),
            accounts: HashMap::new(),
        }
    }

    pub async fn init(&mut self) -> Result<(), IndexingError> {
        let config = RpcProgramAccountsConfig {
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: Some(UiDataSliceConfig {
                    offset: 0,
                    length: 0,
                }),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };

        let start_time = std::time::Instant::now();

        let all_program_account_keys = self
            .rpc_client
            .get_program_accounts_with_config(&self.program_id, config)
            .await
            .unwrap();

        let elapsed = start_time.elapsed();
        info!(
            "Time taken to get {:?} addresses: {:?}",
            all_program_account_keys.len(),
            elapsed
        );

        let start_time = std::time::Instant::now();
        let all_program_accounts = get_multiple_accounts_chunked(
            &self.rpc_client,
            &all_program_account_keys
                .into_iter()
                .map(|(pubkey, _)| pubkey)
                .collect::<Vec<Pubkey>>(),
        )
        .await
        .unwrap();

        let elapsed = start_time.elapsed();
        info!(
            "Time taken to get {:?} accounts: {:?}",
            all_program_accounts.len(),
            elapsed
        );

        for (
            pubkey,
            AccountWithSlot {
                account,
                evaluation_slot,
            },
        ) in all_program_accounts
        {
            self.create_entry(&pubkey, evaluation_slot, &account)
                .await?;
        }

        Ok(())
    }

    // This method assumes that all accounts of interest not owned by the marginfi program are
    // inserted in the routing lookup table when a program account is created / received for the
    // first time. This is why this only processes program accounts.
    pub async fn create_entry(
        &mut self,
        account_pubkey: &Pubkey,
        evaluation_slot: u64,
        account: &Account,
    ) -> Result<(), IndexingError> {
        if account.owner == self.program_id && account.data.len() > 8 {
            let discriminator: [u8; 8] = account.data[..8].try_into().unwrap();
            match discriminator {
                Bank::DISCRIMINATOR => {
                    let bank = Bank::try_deserialize(&mut (&account.data as &[u8]))
                        .map_err(|_| IndexingError::FailedToParseAccountData(*account_pubkey))?;
                    self.banks.insert(
                        *account_pubkey,
                        (
                            evaluation_slot,
                            BankRow {
                                address: *account_pubkey,
                                mint: bank.mint,
                                mint_decimals: bank.mint_decimals,
                            },
                        ),
                    );
                }
                MarginfiAccount::DISCRIMINATOR => {
                    println!("account data size: {}", account.data.len());
                    println!("expected size: {}", std::mem::size_of::<MarginfiAccount>());

                    let marginfi_account =
                        MarginfiAccount::try_deserialize(&mut (&account.data as &[u8])).unwrap();
                    self.accounts.insert(
                        *account_pubkey,
                        (
                            evaluation_slot,
                            AccountRow {
                                address: *account_pubkey,
                                user: marginfi_account.authority,
                            },
                        ),
                    );
                }
                MarginfiGroup::DISCRIMINATOR => {}
                _ => {
                    warn!(
                        "Unknown account discriminator for account: {:?}",
                        account_pubkey
                    );
                }
            }
        }

        Ok(())
    }
}
