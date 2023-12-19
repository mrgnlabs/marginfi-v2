use anchor_client::anchor_lang::AccountDeserialize;
use anchor_client::anchor_lang::Discriminator;
use fixed::types::I80F48;
use itertools::Itertools;
use marginfi::{
    prelude::MarginfiGroup,
    state::{marginfi_account::MarginfiAccount, marginfi_group::Bank, price::*},
};
use solana_account_decoder::UiAccountEncoding;
use solana_account_decoder::UiDataSliceConfig;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::account_info::IntoAccountInfo;
use solana_sdk::{account::Account, program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Account as SplAccount;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
};
use tracing::info;

use crate::common::get_multiple_accounts_chunked2;

#[derive(Clone, Debug)]
pub struct BankAccounts {
    pub bank: Bank,
    pub liquidity_vault_token_account: SplAccount,
    pub insurance_vault_token_account: SplAccount,
    pub fee_vault_token_account: SplAccount,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AccountRoutingType {
    MarginfiGroup,
    MarginfiAccount,
    Bank(Pubkey, BankUpdateRoutingType),
    PriceFeedPyth,
    PriceFeedSwitchboard,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BankUpdateRoutingType {
    State,
    LiquidityTokenAccount,
    InsuranceTokenAccount,
    FeeTokenAccount,
}

#[derive(Clone, Debug)]
pub enum OracleData {
    Pyth(PythEmaPriceFeed),
    Switchboard(SwitchboardV2PriceFeed),
}

impl OracleData {
    pub fn get_price_of_type(
        &self,
        oracle_price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> I80F48 {
        match self {
            OracleData::Pyth(price_feed) => price_feed
                .get_price_of_type(oracle_price_type, bias)
                .unwrap(),
            OracleData::Switchboard(price_feed) => price_feed
                .get_price_of_type(oracle_price_type, bias)
                .unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct Snapshot {
    program_id: Pubkey,
    rpc_client: Arc<RpcClient>,
    pub routing_lookup: HashMap<Pubkey, AccountRoutingType>,

    pub marginfi_groups: HashMap<Pubkey, MarginfiGroup>,
    pub banks: HashMap<Pubkey, BankAccounts>,
    pub marginfi_accounts: HashMap<Pubkey, MarginfiAccount>,
    pub price_feeds: HashMap<Pubkey, OracleData>,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Snapshot:\nMarginfi groups: {}\nBanks: {}\nMarginfi accounts: {}\nPriceFeeds: {}\nRouting lookup: {}",
            self.marginfi_groups.len(),
            self.banks.len(),
            self.marginfi_accounts.len(),
            self.price_feeds.len(),
            self.routing_lookup.len()
        )
    }
}

impl Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Snapshot:\nBanks: {:?}\nMarginfi accounts: {:?}\nPriceFeeds: {:?}\nRouting lookup: {:?}",
            self.banks, self.marginfi_accounts, self.price_feeds, self.routing_lookup
        )
    }
}

impl Snapshot {
    pub fn new(program_id: Pubkey, rpc_client: Arc<RpcClient>) -> Self {
        Self {
            program_id,
            rpc_client,
            routing_lookup: HashMap::new(),
            marginfi_groups: HashMap::new(),
            banks: HashMap::new(),
            marginfi_accounts: HashMap::new(),
            price_feeds: HashMap::new(),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
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
            .await?;

        let elapsed = start_time.elapsed();
        info!(
            "Time taken to get {:?} addresses: {:?}",
            all_program_account_keys.len(),
            elapsed
        );

        let start_time = std::time::Instant::now();
        let all_program_accounts = get_multiple_accounts_chunked2(
            &self.rpc_client,
            &all_program_account_keys
                .into_iter()
                .map(|(pubkey, _)| pubkey)
                .collect::<Vec<Pubkey>>(),
        )
        .await?;

        let elapsed = start_time.elapsed();
        info!(
            "Time taken to get {:?} accounts: {:?}",
            all_program_accounts.len(),
            elapsed
        );

        for (pubkey, account) in all_program_accounts {
            self.create_entry(&pubkey, &account).await;
        }

        Ok(())
    }

    // This method assumes that all accounts of interest not owned by the marginfi program are
    // inserted in the routing lookup table when a program account is created / received for the
    // first time. This is why this only processes program accounts.
    pub async fn create_entry(&mut self, account_pubkey: &Pubkey, account: &Account) {
        if account.owner == self.program_id {
            let discriminator = &account.data[..8];
            if discriminator == Bank::discriminator() {
                let bank = Bank::try_deserialize(&mut (&account.data as &[u8])).unwrap();
                self.routing_lookup.insert(
                    *account_pubkey,
                    AccountRoutingType::Bank(*account_pubkey, BankUpdateRoutingType::State),
                );

                self.routing_lookup.insert(
                    bank.liquidity_vault,
                    AccountRoutingType::Bank(
                        *account_pubkey,
                        BankUpdateRoutingType::LiquidityTokenAccount,
                    ),
                );
                self.routing_lookup.insert(
                    bank.insurance_vault,
                    AccountRoutingType::Bank(
                        *account_pubkey,
                        BankUpdateRoutingType::InsuranceTokenAccount,
                    ),
                );
                self.routing_lookup.insert(
                    bank.fee_vault,
                    AccountRoutingType::Bank(
                        *account_pubkey,
                        BankUpdateRoutingType::FeeTokenAccount,
                    ),
                );

                let mut accounts_to_fetch =
                    vec![bank.liquidity_vault, bank.insurance_vault, bank.fee_vault];

                match bank.config.oracle_setup {
                    OracleSetup::None => (),
                    OracleSetup::PythEma => {
                        let oracle_address = bank.config.oracle_keys[0];
                        self.routing_lookup
                            .insert(oracle_address, AccountRoutingType::PriceFeedPyth);
                        accounts_to_fetch.push(oracle_address);
                    }
                    OracleSetup::SwitchboardV2 => {
                        let oracle_address = bank.config.oracle_keys[0];
                        self.routing_lookup
                            .insert(oracle_address, AccountRoutingType::PriceFeedSwitchboard);
                        accounts_to_fetch.push(oracle_address);
                    }
                }

                self.banks.insert(
                    *account_pubkey,
                    BankAccounts {
                        bank,
                        liquidity_vault_token_account: SplAccount::default(),
                        insurance_vault_token_account: SplAccount::default(),
                        fee_vault_token_account: SplAccount::default(),
                    },
                );

                // Fetch all ban-specific accounts + store in current snapshot
                let accounts = self
                    .rpc_client
                    .get_multiple_accounts(&accounts_to_fetch)
                    .await
                    .unwrap()
                    .into_iter()
                    .zip(accounts_to_fetch)
                    .filter_map(|(maybe_account, pubkey)| {
                        maybe_account.map(|account| (pubkey, account))
                    })
                    .collect_vec();

                for (account_pubkey, account) in accounts {
                    self.udpate_entry(&account_pubkey, &account)
                }
            } else if discriminator == MarginfiAccount::discriminator() {
                let marginfi_account =
                    MarginfiAccount::try_deserialize(&mut (&account.data as &[u8])).unwrap();
                self.routing_lookup
                    .insert(*account_pubkey, AccountRoutingType::MarginfiAccount);
                self.marginfi_accounts
                    .insert(*account_pubkey, marginfi_account);
            } else if discriminator == MarginfiGroup::discriminator() {
                let marginfi_group =
                    MarginfiGroup::try_deserialize(&mut (&account.data as &[u8])).unwrap();
                self.routing_lookup
                    .insert(*account_pubkey, AccountRoutingType::MarginfiGroup);
                self.marginfi_groups.insert(*account_pubkey, marginfi_group);
            }
        }
    }

    pub fn udpate_entry(&mut self, account_pubkey: &Pubkey, account: &Account) {
        let routing_info = self
            .routing_lookup
            .get(account_pubkey)
            .expect("Account not found in routing lookup");
        match routing_info {
            AccountRoutingType::PriceFeedPyth => {
                let mut account = account.clone();
                let ai = (account_pubkey, &mut account).into_account_info();
                let pf = PythEmaPriceFeed::load_checked(&ai, 0, u64::MAX).unwrap();
                self.price_feeds
                    .insert(*account_pubkey, OracleData::Pyth(pf));
            }
            AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::LiquidityTokenAccount) => {
                self.banks
                    .get_mut(bank_pk)
                    .unwrap()
                    .liquidity_vault_token_account =
                    SplAccount::unpack_from_slice(&account.data as &[u8]).unwrap();
            }
            AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::InsuranceTokenAccount) => {
                self.banks
                    .get_mut(bank_pk)
                    .unwrap()
                    .insurance_vault_token_account =
                    SplAccount::unpack_from_slice(&account.data as &[u8]).unwrap();
            }
            AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::FeeTokenAccount) => {
                self.banks.get_mut(bank_pk).unwrap().fee_vault_token_account =
                    SplAccount::unpack_from_slice(&account.data as &[u8]).unwrap();
            }
            AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::State) => {
                self.banks.get_mut(bank_pk).unwrap().bank =
                    Bank::try_deserialize(&mut (&account.data as &[u8])).unwrap();
            }
            AccountRoutingType::MarginfiAccount => {
                self.marginfi_accounts.insert(
                    *account_pubkey,
                    MarginfiAccount::try_deserialize(&mut (&account.data as &[u8])).unwrap(),
                );
            }
            AccountRoutingType::MarginfiGroup => {
                self.marginfi_groups.insert(
                    *account_pubkey,
                    MarginfiGroup::try_deserialize(&mut (&account.data as &[u8])).unwrap(),
                );
            }
            AccountRoutingType::PriceFeedSwitchboard => {
                let mut account = account.clone();
                let ai = (account_pubkey, &mut account).into_account_info();
                let pf = SwitchboardV2PriceFeed::load_checked(&ai, 0, u64::MAX).unwrap();
                self.price_feeds
                    .insert(*account_pubkey, OracleData::Switchboard(pf));
            }
        }
    }
}
