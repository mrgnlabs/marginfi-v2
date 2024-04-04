use std::{collections::HashMap, vec};

use anchor_lang::{AnchorDeserialize, Discriminator};
use chrono::NaiveDateTime;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl, RunQueryDsl,
    SelectableHelper,
};
use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use marginfi::{
    instruction::{
        LendingAccountBorrow, LendingAccountDeposit, LendingAccountEndFlashloan,
        LendingAccountLiquidate, LendingAccountRepay, LendingAccountStartFlashloan,
        LendingAccountWithdraw, LendingAccountWithdrawEmissions, LendingPoolAddBank,
        LendingPoolAddBankWithSeed, LendingPoolConfigureBank, MarginfiAccountInitialize,
        SetNewAccountAuthority,
    },
    state::marginfi_group::{BankConfig, BankConfigCompact, BankConfigOpt},
};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use solana_sdk::{
    hash::Hash,
    instruction::CompiledInstruction,
    message::SimpleAddressLoader,
    signature::Signature,
    transaction::{MessageHash, SanitizedTransaction},
    {pubkey, pubkey::Pubkey},
};
use solana_transaction_status::{
    InnerInstruction, InnerInstructions, VersionedTransactionWithStatusMeta,
};
use tracing::{error, info, warn};

use crate::{
    db::{models::*, schema::*},
    entity_store::EntityStore,
    error::IndexingError,
    get_and_insert_if_needed, insert,
};

const SPL_TRANSFER_DISCRIMINATOR: u8 = 3;
pub const MARGINFI_GROUP_ADDRESS: Pubkey = pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
const COMPACT_BANK_CONFIG_ARG_UPGRADE_SLOT: u64 = 232_933_019;
const TOTAL_ASSET_VALUE_INIT_LIMIT_UPGRADE_SLOT: u64 = 204_502_867;
const ADD_BANK_IX_ACCOUNTS_CHANGE_UPGRADE_SLOT: u64 = 232_933_019;
const RISK_TIER_UPGRADE_SLOT: u64 = 179_862_046;
const INTEREST_RATE_CONFIG_UPGRADE_SLOT: u64 = 178_870_399;

#[derive(Debug)]
pub struct MarginfiEventWithMeta {
    pub timestamp: i64,
    pub tx_sig: Signature,
    pub event: Event,
    pub in_flashloan: bool,
    pub call_stack: Vec<Pubkey>,
}

#[enum_dispatch]
pub trait MarginfiEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError>;
}

#[enum_dispatch(MarginfiEvent)]
#[derive(Debug)]
pub enum Event {
    // User actions
    CreateAccount(CreateAccountEvent),
    AccountAuthorityTransfer(AccountAuthorityTransferEvent),
    Deposit(DepositEvent),
    Borrow(BorrowEvent),
    Repay(RepayEvent),
    Withdraw(WithdrawEvent),
    WithdrawEmissions(WithdrawEmissionsEvent),
    Liquidate(LiquidateEvent),

    // Admin actions
    AddBank(AddBankEvent),
    ConfigureBank(ConfigureBankEvent),

    Unknown(UnknownEvent),
}

#[derive(Debug)]
pub struct UnknownEvent {}

impl MarginfiEvent for UnknownEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        _entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let id: Option<i32> = diesel::insert_into(unknown_events::table)
            .values(&UnknownEvents {
                timestamp,
                tx_sig,
                call_stack,
                in_flashloan,
                ..Default::default()
            })
            .on_conflict_do_nothing()
            .returning(unknown_events::id)
            .get_result(db_connection)
            .optional()
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;

        if id.is_none() {
            info!("event already exists");
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CreateAccountEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
}

impl MarginfiEvent for CreateAccountEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                // Not RPC fetching the account data here because it could lead to race condition with the RPC when live ingesting,
                let account_id = get_and_insert_if_needed!(
                    connection,
                    accounts,
                    Accounts,
                    self.account.to_string(),
                    Accounts {
                        address: self.account.to_string(),
                        user_id: authority_id,
                        ..Default::default()
                    }
                );

                let create_account_event = CreateAccountEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(create_account_events::table)
                    .values(&create_account_event)
                    .on_conflict_do_nothing()
                    .returning(create_account_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct AccountAuthorityTransferEvent {
    pub account: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

impl MarginfiEvent for AccountAuthorityTransferEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let old_authority_data = entity_store.get_or_fetch_user(&self.old_authority.to_string())?;
        let new_authority_data = entity_store.get_or_fetch_user(&self.new_authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let old_authority_id = if let Some(id) = old_authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.old_authority.to_string(),
                        Users {
                            address: self.old_authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let new_authority_id = if let Some(id) = new_authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.new_authority.to_string(),
                        Users {
                            address: self.new_authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: new_authority_id,
                            ..Default::default()
                        }
                    )
                };

                let account_authority_transfer_event = TransferAccountAuthorityEvents {
                    timestamp,
                    old_authority_id,
                    new_authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(transfer_account_authority_events::table)
                    .values(&account_authority_transfer_event)
                    .on_conflict_do_nothing()
                    .returning(transfer_account_authority_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct DepositEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
}

impl MarginfiEvent for DepositEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: authority_id,
                            ..Default::default()
                        }
                    )
                };

                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let deposit_event = DepositEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    bank_id,
                    amount: Decimal::from_u64(self.amount).unwrap(),
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(deposit_events::table)
                    .values(&deposit_event)
                    .on_conflict_do_nothing()
                    .returning(deposit_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct BorrowEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
}

impl MarginfiEvent for BorrowEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: authority_id,
                            ..Default::default()
                        }
                    )
                };

                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let borrow_event = BorrowEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    bank_id,
                    amount: Decimal::from_u64(self.amount).unwrap(),
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(borrow_events::table)
                    .values(&borrow_event)
                    .on_conflict_do_nothing()
                    .returning(borrow_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct RepayEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
    pub all: bool,
}

impl MarginfiEvent for RepayEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: authority_id,
                            ..Default::default()
                        }
                    )
                };

                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let repay_event = RepayEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    bank_id,
                    amount: Decimal::from_u64(self.amount).unwrap(),
                    all: self.all,
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(repay_events::table)
                    .values(&repay_event)
                    .on_conflict_do_nothing()
                    .returning(repay_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct WithdrawEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
    pub all: bool,
}

impl MarginfiEvent for WithdrawEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: authority_id,
                            ..Default::default()
                        }
                    )
                };

                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let withdraw_event = WithdrawEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    bank_id,
                    amount: Decimal::from_u64(self.amount).unwrap(),
                    all: self.all,
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(withdraw_events::table)
                    .values(vec![withdraw_event])
                    .on_conflict_do_nothing()
                    .returning(withdraw_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct WithdrawEmissionsEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub emissions_mint: Pubkey,
    pub amount: u64,
}

impl MarginfiEvent for WithdrawEmissionsEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;
        let emission_mint_data =
            entity_store.get_or_fetch_mint(&self.emissions_mint.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let authority_id = if let Some(id) = authority_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.authority.to_string(),
                        Users {
                            address: self.authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                let account_id = if let Some(id) = account_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        accounts,
                        Accounts,
                        self.account.to_string(),
                        Accounts {
                            address: self.account.to_string(),
                            user_id: authority_id,
                            ..Default::default()
                        }
                    )
                };

                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let emission_mint_id = if let Some(id) = emission_mint_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        self.emissions_mint.to_string(),
                        Mints {
                            address: self.emissions_mint.to_string(),
                            symbol: emission_mint_data.symbol.clone(),
                            decimals: emission_mint_data.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let withdraw_emissions_event = WithdrawEmissionsEvents {
                    timestamp,
                    authority_id,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    account_id,
                    bank_id,
                    emission_mint_id,
                    amount: Decimal::from_u64(self.amount).unwrap(),
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(withdraw_emissions_events::table)
                    .values(&withdraw_emissions_event)
                    .on_conflict_do_nothing()
                    .returning(withdraw_emissions_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct LiquidateEvent {
    pub asset_amount: u64,
    pub asset_bank: Pubkey,
    pub liability_bank: Pubkey,
    pub liquidator_account: Pubkey,
    pub liquidator_authority: Pubkey,
    pub liquidatee_account: Pubkey,
}

impl MarginfiEvent for LiquidateEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        // Need to fetch this on explicitly as authority of the liquidator account might have changed since this event
        let liquidator_user_data =
            entity_store.get_or_fetch_user(&self.liquidator_authority.to_string())?;
        let asset_bank_data = entity_store.get_or_fetch_bank(&self.asset_bank.to_string())?;
        let liability_bank_data =
            entity_store.get_or_fetch_bank(&self.liability_bank.to_string())?;
        let liquidator_account_data =
            entity_store.get_or_fetch_account(&self.liquidator_account.to_string())?;
        let liquidatee_account_data =
            entity_store.get_or_fetch_account(&self.liquidatee_account.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let liquidator_user_id = if let Some(id) = liquidator_user_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        users,
                        Users,
                        self.liquidator_authority.to_string(),
                        Users {
                            address: self.liquidator_authority.to_string(),
                            ..Default::default()
                        }
                    )
                };

                // Using the current liquidatee account authority as we do no know the authority at time of liquidation (not in the event data)
                let current_liquidatee_user_id =
                    if let Some(id) = liquidatee_account_data.authority.id {
                        id
                    } else {
                        get_and_insert_if_needed!(
                            connection,
                            users,
                            Users,
                            liquidatee_account_data.authority.address.clone(),
                            Users {
                                address: liquidatee_account_data.authority.address.clone(),
                                ..Default::default()
                            }
                        )
                    };

                let liquidator_account_id = if let Some(id) = liquidator_account_data.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        accounts,
                        Accounts,
                        self.liquidator_account.to_string(),
                        Accounts {
                            address: self.liquidator_account.to_string(),
                            user_id: liquidator_user_id,
                            ..Default::default()
                        }
                    )
                };

                let liquidatee_account_id = if let Some(id) = liquidatee_account_data.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        accounts,
                        Accounts,
                        self.liquidatee_account.to_string(),
                        Accounts {
                            address: self.liquidatee_account.to_string(),
                            user_id: current_liquidatee_user_id,
                            ..Default::default()
                        }
                    )
                };

                let asset_mint_id = if let Some(id) = asset_bank_data.mint.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        mints,
                        Mints,
                        asset_bank_data.mint.address.clone(),
                        Mints {
                            address: asset_bank_data.mint.address.clone(),
                            symbol: asset_bank_data.mint.symbol.clone(),
                            decimals: asset_bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let liability_mint_id = if let Some(id) = liability_bank_data.mint.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        mints,
                        Mints,
                        liability_bank_data.mint.address.clone(),
                        Mints {
                            address: liability_bank_data.mint.address.clone(),
                            symbol: liability_bank_data.mint.symbol.clone(),
                            decimals: liability_bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let asset_bank_id = if let Some(id) = asset_bank_data.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        banks,
                        Banks,
                        self.asset_bank.to_string(),
                        Banks {
                            address: self.asset_bank.to_string(),
                            mint_id: asset_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let liability_bank_id = if let Some(id) = liability_bank_data.id {
                    id
                } else {
                    get_and_insert_if_needed!(
                        connection,
                        banks,
                        Banks,
                        self.liability_bank.to_string(),
                        Banks {
                            address: self.liability_bank.to_string(),
                            mint_id: liability_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let liquidate_event = LiquidateEvents {
                    timestamp,
                    tx_sig,
                    call_stack,
                    in_flashloan,
                    liquidator_account_id,
                    liquidatee_account_id,
                    liquidator_user_id,
                    asset_bank_id,
                    liability_bank_id,
                    asset_amount: Decimal::from_u64(self.asset_amount).unwrap(),
                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(liquidate_events::table)
                    .values(&liquidate_event)
                    .on_conflict_do_nothing()
                    .returning(liquidate_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct AddBankEvent {
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub config: BankConfig,
}

impl MarginfiEvent for AddBankEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let mint_data = entity_store.get_or_fetch_mint(&self.mint.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let bank_mint_id = if let Some(id) = mint_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        self.mint.to_string(),
                        Mints {
                            address: mint_data.address.clone(),
                            symbol: mint_data.symbol.clone(),
                            decimals: mint_data.decimals,
                            ..Default::default()
                        }
                    )
                };

                // Not RPC fetching the account data here because it could lead to race condition with the RPC when live ingesting,
                let bank_id = get_and_insert_if_needed!(
                    connection,
                    banks,
                    Banks,
                    self.bank.to_string(),
                    Banks {
                        address: self.bank.to_string(),
                        mint_id: bank_mint_id,
                        ..Default::default()
                    }
                );

                let create_bank_event = CreateBankEvents {
                    timestamp,
                    tx_sig,
                    call_stack,
                    in_flashloan,

                    bank_id,
                    asset_weight_init: Decimal::from_f64(
                        I80F48::from(self.config.asset_weight_init).to_num(),
                    )
                    .unwrap(),
                    asset_weight_maint: Decimal::from_f64(
                        I80F48::from(self.config.asset_weight_maint).to_num(),
                    )
                    .unwrap(),
                    liability_weight_init: Decimal::from_f64(
                        I80F48::from(self.config.liability_weight_init).to_num(),
                    )
                    .unwrap(),
                    liability_weight_maint: Decimal::from_f64(
                        I80F48::from(self.config.liability_weight_maint).to_num(),
                    )
                    .unwrap(),
                    deposit_limit: Decimal::from_u64(self.config.deposit_limit).unwrap(),
                    optimal_utilization_rate: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.optimal_utilization_rate)
                            .to_num(),
                    )
                    .unwrap(),
                    plateau_interest_rate: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.plateau_interest_rate)
                            .to_num(),
                    )
                    .unwrap(),
                    max_interest_rate: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.max_interest_rate).to_num(),
                    )
                    .unwrap(),
                    insurance_fee_fixed_apr: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.insurance_fee_fixed_apr)
                            .to_num(),
                    )
                    .unwrap(),
                    insurance_ir_fee: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.insurance_ir_fee).to_num(),
                    )
                    .unwrap(),
                    protocol_fixed_fee_apr: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.protocol_fixed_fee_apr)
                            .to_num(),
                    )
                    .unwrap(),
                    protocol_ir_fee: Decimal::from_f64(
                        I80F48::from(self.config.interest_rate_config.protocol_ir_fee).to_num(),
                    )
                    .unwrap(),
                    operational_state_id: self.config.operational_state as i32,
                    oracle_setup_id: self.config.oracle_setup as i32,
                    oracle_keys: serde_json::to_string(
                        &self
                            .config
                            .oracle_keys
                            .iter()
                            .map(|k| k.to_string())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                    borrow_limit: Decimal::from_u64(self.config.borrow_limit).unwrap(),
                    risk_tier_id: self.config.risk_tier as i32,

                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(create_bank_events::table)
                    .values(&create_bank_event)
                    .on_conflict_do_nothing()
                    .returning(create_bank_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

#[derive(Debug)]
pub struct ConfigureBankEvent {
    pub bank: Pubkey,
    pub config: BankConfigOpt,
}

impl MarginfiEvent for ConfigureBankEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        db_connection
            .transaction(|connection: &mut PgConnection| {
                let bank_mint_id = if let Some(id) = bank_data.mint.id {
                    id
                } else {
                    insert!(
                        connection,
                        mints,
                        Mints,
                        bank_data.mint.address.clone(),
                        Mints {
                            address: bank_data.mint.address.clone(),
                            symbol: bank_data.mint.symbol.clone(),
                            decimals: bank_data.mint.decimals,
                            ..Default::default()
                        }
                    )
                };

                let bank_id = if let Some(id) = bank_data.id {
                    id
                } else {
                    insert!(
                        connection,
                        banks,
                        Banks,
                        self.bank.to_string(),
                        Banks {
                            address: self.bank.to_string(),
                            mint_id: bank_mint_id,
                            ..Default::default()
                        }
                    )
                };

                let configure_bank_event = ConfigureBankEvents {
                    timestamp,
                    tx_sig,
                    call_stack,
                    in_flashloan,

                    bank_id,
                    asset_weight_init: self
                        .config
                        .asset_weight_init
                        .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap()),
                    asset_weight_maint: self
                        .config
                        .asset_weight_maint
                        .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap()),
                    liability_weight_init: self
                        .config
                        .liability_weight_init
                        .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap()),
                    liability_weight_maint: self
                        .config
                        .liability_weight_maint
                        .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap()),
                    deposit_limit: self
                        .config
                        .deposit_limit
                        .map(|v| Decimal::from_u64(v).unwrap()),
                    optimal_utilization_rate: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.optimal_utilization_rate
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    plateau_interest_rate: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.plateau_interest_rate
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    max_interest_rate: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.max_interest_rate
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    insurance_fee_fixed_apr: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.insurance_fee_fixed_apr
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    insurance_ir_fee: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.insurance_ir_fee
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    protocol_fixed_fee_apr: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.protocol_fixed_fee_apr
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    protocol_ir_fee: self
                        .config
                        .interest_rate_config
                        .as_ref()
                        .map(|v| {
                            v.protocol_ir_fee
                                .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
                        })
                        .flatten()
                        .clone(),
                    operational_state_id: self.config.operational_state.map(|v| v as i32),
                    oracle_setup_id: self.config.oracle.map(|v| v.setup as i32),
                    oracle_keys: self.config.oracle.map(|v| {
                        serde_json::to_string(
                            &v.keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
                        )
                        .unwrap()
                    }),
                    borrow_limit: self
                        .config
                        .borrow_limit
                        .map(|v| Decimal::from_u64(v).unwrap()),
                    risk_tier_id: self.config.risk_tier.map(|v| v as i32),

                    ..Default::default()
                };

                let id: Option<i32> = diesel::insert_into(configure_bank_events::table)
                    .values(&configure_bank_event)
                    .on_conflict_do_nothing()
                    .returning(configure_bank_events::id)
                    .get_result(connection)
                    .optional()?;

                if id.is_none() {
                    info!("event already exists");
                }

                diesel::result::QueryResult::Ok(())
            })
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))
    }
}

pub struct MarginfiEventParser {
    program_id: Pubkey,
    marginfi_group: Pubkey,
}

impl MarginfiEventParser {
    pub fn new(program_id: Pubkey, marginfi_group: Pubkey) -> Self {
        Self {
            program_id,
            marginfi_group,
        }
    }

    pub fn extract_events(
        &self,
        timestamp: i64,
        slot: u64,
        tx_with_meta: VersionedTransactionWithStatusMeta,
    ) -> Vec<MarginfiEventWithMeta> {
        let tx_sig = tx_with_meta.transaction.signatures[0];

        let mut events: Vec<MarginfiEventWithMeta> = vec![];

        let mut in_flashloan = false;

        let sanitized_tx = SanitizedTransaction::try_create(
            tx_with_meta.transaction,
            MessageHash::Precomputed(Hash::default()),
            None,
            SimpleAddressLoader::Enabled(tx_with_meta.meta.loaded_addresses),
            true,
        )
        .unwrap();

        let mut inner_instructions: HashMap<u8, Vec<InnerInstruction>> = HashMap::new();
        for InnerInstructions {
            instructions,
            index,
        } in tx_with_meta
            .meta
            .inner_instructions
            .unwrap_or_default()
            .into_iter()
        {
            inner_instructions.insert(index, instructions);
        }

        for (outer_ix_index, instruction) in
            sanitized_tx.message().instructions().iter().enumerate()
        {
            let account_keys = sanitized_tx
                .message()
                .account_keys()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let top_level_program_id = instruction.program_id(&account_keys);

            let mut call_stack = vec![];

            let inner_instructions = inner_instructions
                .remove(&(outer_ix_index as u8))
                .unwrap_or_default();

            if top_level_program_id.eq(&self.program_id) {
                // println!("Instruction {}: {:?}", i, top_level_program_id);
                let event = self.parse_event(
                    slot,
                    &tx_sig,
                    &instruction,
                    &inner_instructions,
                    &account_keys,
                    &mut in_flashloan,
                );
                if let Some(event) = event {
                    let call_stack = call_stack.iter().cloned().cloned().collect();
                    let event_with_meta = MarginfiEventWithMeta {
                        timestamp,
                        tx_sig,
                        event,
                        in_flashloan,
                        call_stack,
                    };
                    // info!("Event: {:?}", event_with_meta);
                    events.push(event_with_meta);
                }
            }

            if inner_instructions.is_empty() {
                continue;
            }

            call_stack.push(top_level_program_id);

            for (inner_ix_index, inner_instruction) in inner_instructions.iter().enumerate() {
                let cpi_program_id = inner_instruction.instruction.program_id(&account_keys);

                if cpi_program_id.eq(&self.program_id) {
                    let remaining_instructions = if inner_instructions.len() > inner_ix_index + 1 {
                        &inner_instructions[(inner_ix_index + 1)..]
                    } else {
                        &[]
                    };

                    let event = self.parse_event(
                        slot,
                        &tx_sig,
                        &inner_instruction.instruction,
                        remaining_instructions,
                        &account_keys,
                        &mut in_flashloan,
                    );
                    if let Some(event) = event {
                        let call_stack = call_stack.iter().cloned().cloned().collect();
                        let event_with_meta = MarginfiEventWithMeta {
                            timestamp,
                            tx_sig,
                            event,
                            in_flashloan,
                            call_stack,
                        };
                        // info!("Inner event: {:?}", event_with_meta);
                        events.push(event_with_meta);
                    }
                }

                if let Some(stack_height) = inner_instruction.stack_height {
                    if stack_height - 1 > call_stack.len() as u32 {
                        call_stack.push(cpi_program_id);
                    } else {
                        call_stack.truncate(stack_height as usize);
                    }
                }
            }
        }

        events
    }

    pub fn parse_event(
        &self,
        slot: u64,
        tx_signature: &Signature,
        instruction: &CompiledInstruction,
        remaining_instructions: &[InnerInstruction],
        account_keys: &[Pubkey],
        in_flashloan: &mut bool,
    ) -> Option<Event> {
        if instruction.data.len() < 8 {
            error!("Instruction data too short");
            return None;
        }

        let ix_accounts = instruction
            .accounts
            .iter()
            .map(|ix| account_keys[*ix as usize])
            .collect::<Vec<_>>();

        let discriminator: [u8; 8] = instruction.data[..8].try_into().ok()?;
        let mut instruction_data = &instruction.data[8..];
        match discriminator {
            MarginfiAccountInitialize::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let authority = *ix_accounts.get(2).unwrap();

                Some(Event::CreateAccount(CreateAccountEvent {
                    account: marginfi_account,
                    authority,
                }))
            }
            SetNewAccountAuthority::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(1).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let marginfi_account = *ix_accounts.get(0).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let new_authority = *ix_accounts.get(3).unwrap();

                Some(Event::AccountAuthorityTransfer(
                    AccountAuthorityTransferEvent {
                        account: marginfi_account,
                        old_authority: signer,
                        new_authority,
                    },
                ))
            }
            LendingAccountDeposit::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after deposit in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(Event::Deposit(DepositEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                }))
            }
            LendingAccountBorrow::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after borrow in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(Event::Borrow(BorrowEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                }))
            }
            LendingAccountRepay::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction = LendingAccountRepay::deserialize(&mut instruction_data).ok()?;

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after repay in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(Event::Repay(RepayEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                    all: instruction.repay_all.unwrap_or(false),
                }))
            }
            LendingAccountWithdraw::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction =
                    LendingAccountWithdraw::deserialize(&mut instruction_data).ok()?;

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after withdraw in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(Event::Withdraw(WithdrawEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                    all: instruction.withdraw_all.unwrap_or(false),
                }))
            }
            LendingAccountLiquidate::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction =
                    LendingAccountLiquidate::deserialize(&mut instruction_data).ok()?;

                let asset_bank = *ix_accounts.get(1).unwrap();
                let liability_bank = *ix_accounts.get(2).unwrap();
                let liquidator_account = *ix_accounts.get(3).unwrap();
                let liquidator_authority = *ix_accounts.get(4).unwrap();
                let liquidatee_account = *ix_accounts.get(5).unwrap();

                Some(Event::Liquidate(LiquidateEvent {
                    asset_amount: instruction.asset_amount,
                    asset_bank,
                    liability_bank,
                    liquidator_account,
                    liquidator_authority,
                    liquidatee_account,
                }))
            }
            LendingAccountWithdrawEmissions::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();
                let emissions_mint = *ix_accounts.get(4).unwrap();

                Some(Event::WithdrawEmissions(WithdrawEmissionsEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    emissions_mint,
                    amount: spl_transfer_amount,
                }))
            }
            LendingPoolAddBank::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let bank_config = if slot < COMPACT_BANK_CONFIG_ARG_UPGRADE_SLOT {
                    BankConfig::deserialize(&mut &instruction_data[..531]).unwrap()
                } else {
                    BankConfigCompact::deserialize(&mut &instruction_data[..363])
                        .unwrap()
                        .into()
                };

                let (bank_mint, bank) = if slot < ADD_BANK_IX_ACCOUNTS_CHANGE_UPGRADE_SLOT {
                    (*ix_accounts.get(2).unwrap(), *ix_accounts.get(3).unwrap())
                } else {
                    (*ix_accounts.get(3).unwrap(), *ix_accounts.get(4).unwrap())
                };

                Some(Event::AddBank(AddBankEvent {
                    bank,
                    mint: bank_mint,
                    config: bank_config,
                }))
            }
            LendingPoolAddBankWithSeed::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let bank_config = BankConfigCompact::deserialize(&mut &instruction_data[..363])
                    .unwrap()
                    .into();

                let bank_mint = *ix_accounts.get(3).unwrap();
                let bank = *ix_accounts.get(4).unwrap();

                Some(Event::AddBank(AddBankEvent {
                    bank,
                    mint: bank_mint,
                    config: bank_config,
                }))
            }
            LendingPoolConfigureBank::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                // println!("Instruction data: {:?}", instruction_data);
                // println!("data len: {:?}", instruction_data.len());

                // let parsed = BankConfigOpt {
                //     interest_rate_config: Some(InterestRateConfigOpt {
                //         optimal_utilization_rate: Some(I80F48::from_num(0.95).into()),
                //         plateau_interest_rate: Some(I80F48::from_num(0.05).into()),
                //         max_interest_rate: Some(I80F48::from_num(2).into()),
                //         insurance_fee_fixed_apr: Some(I80F48::from_num(0).into()),
                //         ..Default::default()
                //     }),
                //     ..Default::default()
                // };
                // println!("Parsed: {:?}", parsed);
                // let ser = parsed.try_to_vec().unwrap();
                // println!("Serialized: {:?}", ser);

                let mut data = vec![];
                data.extend_from_slice(&instruction_data);
                if slot < INTEREST_RATE_CONFIG_UPGRADE_SLOT {
                    data.extend_from_slice(&[0, 0, 0, 0]);
                } else if slot < RISK_TIER_UPGRADE_SLOT {
                    data.extend_from_slice(&[0, 0, 0]);
                } else if slot < TOTAL_ASSET_VALUE_INIT_LIMIT_UPGRADE_SLOT {
                    data.extend_from_slice(&[0]);
                }

                let bank_config_opt = BankConfigOpt::deserialize(&mut data.as_slice()).unwrap();

                let bank = *ix_accounts.get(2).unwrap();

                Some(Event::ConfigureBank(ConfigureBankEvent {
                    bank,
                    config: bank_config_opt,
                }))
            }
            LendingAccountStartFlashloan::DISCRIMINATOR => {
                *in_flashloan = true;

                None
            }
            LendingAccountEndFlashloan::DISCRIMINATOR => {
                *in_flashloan = false;

                None
            }
            _ => {
                warn!(
                    "Unknown instruction discriminator {:?} in {:?}",
                    discriminator, tx_signature
                );
                Some(Event::Unknown(UnknownEvent {}))
            }
        }
    }
}

fn get_spl_transfer_amount(
    instruction: &CompiledInstruction,
    account_keys: &[Pubkey],
) -> Option<u64> {
    let transfer_ix_pid = instruction.program_id(account_keys);
    if !transfer_ix_pid.eq(&spl_token::id()) || instruction.data[0] != SPL_TRANSFER_DISCRIMINATOR {
        warn!(
            "Expected following instruction to be {:?}/{} in deposit, got {:?}/{:?} instead",
            spl_token::id(),
            SPL_TRANSFER_DISCRIMINATOR,
            transfer_ix_pid,
            instruction.data[0]
        );
        return None;
    }

    let spl_transfer_amount: u64 = u64::from_le_bytes(instruction.data[1..9].try_into().unwrap());
    Some(spl_transfer_amount)
}
