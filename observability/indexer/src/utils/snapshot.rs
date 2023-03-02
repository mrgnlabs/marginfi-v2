use crate::common::pyth_price_to_fixed;
use crate::{
    commands::snapshot_accounts::{AccountUpdate, Context},
    common::get_multiple_accounts_chunked,
};
use anchor_client::anchor_lang::AccountDeserialize;
use anchor_client::anchor_lang::Discriminator;
use fixed::types::I80F48;
use itertools::Itertools;
use marginfi::{
    prelude::MarginfiGroup,
    state::{
        marginfi_account::MarginfiAccount,
        marginfi_group::{Bank, OracleSetup},
    },
};
use pyth_sdk_solana::{load_price_feed_from_account, PriceFeed};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::sysvar::{self, Sysvar};
use solana_sdk::{
    account::Account, account_info::IntoAccountInfo, clock::Clock, program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account as SplAccount;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    mem::size_of,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct BankAccounts {
    pub bank: Bank,
    pub liquidity_vault_token_account: SplAccount,
    pub insurance_vault_token_account: SplAccount,
    pub fee_vault_token_account: SplAccount,
}

#[derive(Clone, Debug)]
pub enum AccountRoutingType {
    Clock,
    MarginfiGroup,
    MarginfiAccount,
    Bank(Pubkey, BankUpdateRoutingType),
    PriceFeed,
}

#[derive(Clone, Debug)]
pub enum BankUpdateRoutingType {
    State,
    LiquidityTokenAccount,
    InsuranceTokenAccount,
    FeeTokenAccount,
}

#[derive(Clone, Debug)]
pub enum OracleData {
    Pyth(PriceFeed),
}

impl OracleData {
    pub fn get_price(&self) -> I80F48 {
        match self {
            OracleData::Pyth(price_feed) => pyth_price_to_fixed(price_feed).unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct Snapshot {
    program_id: Pubkey,
    rpc_client: Arc<RpcClient>,
    routing_lookup: HashMap<Pubkey, AccountRoutingType>,

    pub clock: Clock,
    pub marginfi_group: (Pubkey, MarginfiGroup),
    pub banks: HashMap<Pubkey, BankAccounts>,
    pub marginfi_accounts: HashMap<Pubkey, MarginfiAccount>,
    pub price_feeds: HashMap<Pubkey, OracleData>,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Snapshot:\nBanks: {}\nMarginfi accounts: {}\nPriceFeeds: {}\nRouting lookup: {}",
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
            clock: Clock::default(),
            marginfi_group: (Pubkey::default(), MarginfiGroup::default()),
            banks: HashMap::new(),
            marginfi_accounts: HashMap::new(),
            price_feeds: HashMap::new(),
        }
    }

    pub async fn fetch_all_banks_for_group(
        &mut self,
        marginfi_group: Pubkey,
    ) -> anyhow::Result<Vec<(Pubkey, Bank)>> {
        let account_type_filter =
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, &Bank::discriminator()));
        let marginfi_group_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 + size_of::<Pubkey>() + size_of::<u8>(),
            marginfi_group.to_bytes().to_vec(),
        ));

        let config = RpcProgramAccountsConfig {
            filters: Some([vec![account_type_filter], vec![marginfi_group_filter]].concat()),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };

        let banks: Vec<(Pubkey, Bank)> = self
            .rpc_client
            .get_program_accounts_with_config(&self.program_id, config)
            .await?
            .into_iter()
            .map(|(key, account)| {
                (
                    key,
                    Bank::try_deserialize(&mut (&account.data as &[u8])).unwrap(),
                )
            })
            .collect_vec();

        Ok(banks)
    }

    pub async fn fetch_all_marginfi_accounts_for_group(
        &mut self,
        marginfi_group: Pubkey,
    ) -> anyhow::Result<Vec<(Pubkey, MarginfiAccount)>> {
        let account_type_filter = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
            0,
            &MarginfiAccount::discriminator(),
        ));
        let marginfi_group_filter =
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, marginfi_group.to_bytes().to_vec()));

        let config = RpcProgramAccountsConfig {
            filters: Some([vec![account_type_filter], vec![marginfi_group_filter]].concat()),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };

        let banks: Vec<(Pubkey, MarginfiAccount)> = self
            .rpc_client
            .get_program_accounts_with_config(&self.program_id, config)
            .await
            .unwrap()
            .into_iter()
            .map(|(key, account)| {
                (
                    key,
                    MarginfiAccount::try_deserialize(&mut (&account.data as &[u8])).unwrap(),
                )
            })
            .collect_vec();

        Ok(banks)
    }

    pub async fn init(&mut self, ctx: Arc<Context>) -> anyhow::Result<()> {
        let marginfi_group_pk = ctx.config.marginfi_group;
        self.routing_lookup
            .insert(marginfi_group_pk, AccountRoutingType::MarginfiGroup);
        let clock_pk = sysvar::clock::id();
        self.routing_lookup
            .insert(clock_pk, AccountRoutingType::Clock);

        // Fetch all ban-specific accounts + store in current snapshot
        let accounts_to_fetch = &[marginfi_group_pk, clock_pk];
        let accounts = self
            .rpc_client
            .get_multiple_accounts(accounts_to_fetch)
            .await
            .unwrap()
            .into_iter()
            .zip(accounts_to_fetch)
            .filter_map(|(maybe_account, pubkey)| maybe_account.map(|account| (pubkey, account)))
            .collect_vec();

        for (account_pubkey, account) in accounts {
            self.udpate_entry(account_pubkey, &account)
        }

        // Fetch all banks for marginfi group + store their lookup + store in current snapshot
        let banks = self.fetch_all_banks_for_group(marginfi_group_pk).await?;

        let mut accounts_to_fetch = vec![];
        for (bank_pk, bank_data) in banks {
            self.routing_lookup.insert(
                bank_pk,
                AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::State),
            );

            self.routing_lookup.insert(
                bank_data.liquidity_vault,
                AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::LiquidityTokenAccount),
            );
            self.routing_lookup.insert(
                bank_data.insurance_vault,
                AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::InsuranceTokenAccount),
            );
            self.routing_lookup.insert(
                bank_data.fee_vault,
                AccountRoutingType::Bank(bank_pk, BankUpdateRoutingType::FeeTokenAccount),
            );
            accounts_to_fetch.append(&mut vec![
                bank_data.liquidity_vault,
                bank_data.insurance_vault,
                bank_data.fee_vault,
            ]);

            if bank_data.config.oracle_setup == OracleSetup::Pyth {
                let oracle_address = bank_data.config.get_pyth_oracle_key();
                self.routing_lookup
                    .insert(oracle_address, AccountRoutingType::PriceFeed);
                accounts_to_fetch.push(oracle_address);
            }

            self.banks.insert(
                bank_pk,
                BankAccounts {
                    bank: bank_data,
                    liquidity_vault_token_account: SplAccount::default(),
                    insurance_vault_token_account: SplAccount::default(),
                    fee_vault_token_account: SplAccount::default(),
                },
            );
        }

        // Fetch all bank-specific accounts + store in current snapshot
        let accounts = get_multiple_accounts_chunked(&ctx.rpc_client, &accounts_to_fetch).await?;

        for (account_pubkey, account) in accounts {
            self.udpate_entry(&account_pubkey, &account) // works because we've created the lookup entry in the bank iteration above
        }

        // Fetch all marginfi accounts for marginfi group + store their lookup + store in current snapshot
        let marginfi_accounts = self
            .fetch_all_marginfi_accounts_for_group(marginfi_group_pk)
            .await?;

        for (marginfi_account_pk, marginfi_account_data) in marginfi_accounts {
            self.routing_lookup
                .insert(marginfi_account_pk, AccountRoutingType::MarginfiAccount);

            self.marginfi_accounts
                .insert(marginfi_account_pk, marginfi_account_data);
        }

        Ok(())
    }

    pub async fn process_update(&mut self, account_pubkey: &AccountUpdate) {
        if self.routing_lookup.contains_key(&account_pubkey.address) {
            self.udpate_entry(&account_pubkey.address, &account_pubkey.account_data);
        } else {
            self.create_entry(&account_pubkey.address, &account_pubkey.account_data)
                .await;
        }
    }

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

                if bank.config.oracle_setup == OracleSetup::Pyth {
                    let oracle_address = bank.config.get_pyth_oracle_key();
                    self.routing_lookup
                        .insert(oracle_address, AccountRoutingType::PriceFeed);
                    accounts_to_fetch.push(oracle_address);
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
            }
        }
    }

    pub fn udpate_entry(&mut self, account_pubkey: &Pubkey, account: &Account) {
        let routing_info = self
            .routing_lookup
            .get(account_pubkey)
            .expect("Account not found in routing lookup");
        match routing_info {
            AccountRoutingType::Clock => {
                self.clock = Clock::from_account_info(
                    &(&sysvar::clock::ID, &mut account.clone()).into_account_info(),
                )
                .unwrap();
            }
            AccountRoutingType::PriceFeed => {
                let price_feed =
                    load_price_feed_from_account(account_pubkey, &mut account.clone()).unwrap();
                self.price_feeds
                    .insert(*account_pubkey, OracleData::Pyth(price_feed));
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
                self.marginfi_group = (
                    *account_pubkey,
                    MarginfiGroup::try_deserialize(&mut (&account.data as &[u8])).unwrap(),
                );
            }
        }
    }
}
