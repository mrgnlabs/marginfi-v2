use std::{collections::HashMap, vec};

use anchor_lang::{AnchorDeserialize, Discriminator};
use chrono::NaiveDateTime;
use diesel::{OptionalExtension, PgConnection, RunQueryDsl};
use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use marginfi::{
    instruction::{
        LendingAccountBorrow, LendingAccountCloseBalance, LendingAccountDeposit,
        LendingAccountEndFlashloan, LendingAccountLiquidate, LendingAccountRepay,
        LendingAccountSettleEmissions, LendingAccountStartFlashloan, LendingAccountWithdraw,
        LendingAccountWithdrawEmissions, LendingPoolAccrueBankInterest, LendingPoolAddBank,
        LendingPoolAddBankWithSeed, LendingPoolConfigureBank, MarginfiAccountInitialize,
        MarginfiGroupConfigure, MarginfiGroupInitialize, SetNewAccountAuthority,
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
use tracing::{debug, error, warn};

use crate::{
    db::{models::*, schema::*},
    entity_store::EntityStore,
    error::IndexingError,
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
    pub slot: u64,
    pub tx_sig: Signature,
    pub event: Event,
    pub in_flashloan: bool,
    pub call_stack: Vec<Pubkey>,
    pub outer_ix_index: i16,
    pub inner_ix_index: Option<i16>,
}

#[enum_dispatch]
pub trait MarginfiEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError>;

    fn name(&self) -> &'static str;
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
    CreateGroup(CreateGroupEvent),
    ConfigureGroup(ConfigureGroupEvent),
    CreateBank(CreateBankEvent),
    ConfigureBank(ConfigureBankEvent),
    Unknown(UnknownEvent),
}

#[derive(Debug)]
pub struct UnknownEvent {}

impl MarginfiEvent for UnknownEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        _entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let id: Option<i32> = diesel::insert_into(unknown_events::table)
            .values(&UnknownEvents {
                timestamp,
                tx_sig,
                slot: Decimal::from_u64(slot).unwrap(),
                call_stack,
                in_flashloan,
                outer_ix_index,
                inner_ix_index,
                ..Default::default()
            })
            .on_conflict_do_nothing()
            .returning(unknown_events::id)
            .get_result(db_connection)
            .optional()
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;

        if id.is_none() {
            debug!("event already exists");
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "unknown"
    }
}

#[derive(Debug)]
pub struct CreateAccountEvent {
    pub group: Pubkey,
    pub account: Pubkey,
    pub authority: Pubkey,
}

impl MarginfiEvent for CreateAccountEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account_no_rpc(&self.account.to_string())?;
        let group_data = entity_store.get_or_fetch_group(&self.group.to_string())?;

        let all_dependencies_exist = authority_data.id.is_some()
            && group_data.id.is_some()
            && account_data.as_ref().map(|ad| ad.id).flatten().is_some();

        if all_dependencies_exist {
            CreateAccountEvents::insert(
                db_connection,
                account_data.unwrap().id.unwrap(),
                authority_data.id.unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            CreateAccountEvents::insert_with_dependents(
                db_connection,
                &self.account.to_string(),
                &self.authority.to_string(),
                &self.group.to_string(),
                &group_data.admin,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "create_account"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let old_authority_data = entity_store.get_or_fetch_user(&self.old_authority.to_string())?;
        let new_authority_data = entity_store.get_or_fetch_user(&self.new_authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;

        let all_dependencies_exist = old_authority_data.id.is_some()
            && new_authority_data.id.is_some()
            && account_data.id.is_some();

        if all_dependencies_exist {
            TransferAccountAuthorityEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                old_authority_data.id.unwrap(),
                new_authority_data.id.unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            TransferAccountAuthorityEvents::insert_with_dependents(
                db_connection,
                &self.account.to_string(),
                &self.old_authority.to_string(),
                &self.new_authority.to_string(),
                &account_data.group.address,
                &account_data.group.admin,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "account_authority_transfer"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        let all_dependencies_exist =
            authority_data.id.is_some() && account_data.id.is_some() && bank_data.id.is_some();

        if all_dependencies_exist {
            DepositEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                authority_data.id.unwrap(),
                bank_data.id.unwrap(),
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            DepositEvents::insert_with_dependents(
                db_connection,
                &self.authority.to_string(),
                &self.account.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                &self.bank.to_string(),
                &account_data.group.address,
                &account_data.group.admin,
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountDeposit"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        let all_dependencies_exist =
            authority_data.id.is_some() && account_data.id.is_some() && bank_data.id.is_some();

        if all_dependencies_exist {
            BorrowEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                authority_data.id.unwrap(),
                bank_data.id.unwrap(),
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            BorrowEvents::insert_with_dependents(
                db_connection,
                &self.authority.to_string(),
                &self.account.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                &self.bank.to_string(),
                &account_data.group.address,
                &account_data.group.admin,
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountBorrow"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        let all_dependencies_exist =
            authority_data.id.is_some() && account_data.id.is_some() && bank_data.id.is_some();

        if all_dependencies_exist {
            RepayEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                authority_data.id.unwrap(),
                bank_data.id.unwrap(),
                Decimal::from_u64(self.amount).unwrap(),
                self.all,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            RepayEvents::insert_with_dependents(
                db_connection,
                &self.authority.to_string(),
                &self.account.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                &self.bank.to_string(),
                &account_data.group.address,
                &account_data.group.admin,
                Decimal::from_u64(self.amount).unwrap(),
                self.all,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountRepay"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        let all_dependencies_exist =
            authority_data.id.is_some() && account_data.id.is_some() && bank_data.id.is_some();

        if all_dependencies_exist {
            WithdrawEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                authority_data.id.unwrap(),
                bank_data.id.unwrap(),
                Decimal::from_u64(self.amount).unwrap(),
                self.all,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            WithdrawEvents::insert_with_dependents(
                db_connection,
                &self.authority.to_string(),
                &self.account.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                &self.bank.to_string(),
                &account_data.group.address,
                &account_data.group.admin,
                Decimal::from_u64(self.amount).unwrap(),
                self.all,
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountWithdraw"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let authority_data = entity_store.get_or_fetch_user(&self.authority.to_string())?;
        let account_data = entity_store.get_or_fetch_account(&self.account.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;
        let emission_mint_data =
            entity_store.get_or_fetch_mint(&self.emissions_mint.to_string())?;

        let all_dependencies_exist = authority_data.id.is_some()
            && account_data.id.is_some()
            && bank_data.id.is_some()
            && emission_mint_data.id.is_some();

        if all_dependencies_exist {
            WithdrawEmissionsEvents::insert(
                db_connection,
                account_data.id.unwrap(),
                authority_data.id.unwrap(),
                bank_data.id.unwrap(),
                emission_mint_data.id.unwrap(),
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            WithdrawEmissionsEvents::insert_with_dependents(
                db_connection,
                &self.authority.to_string(),
                &self.account.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                &self.bank.to_string(),
                &emission_mint_data.address,
                &emission_mint_data.symbol,
                emission_mint_data.decimals,
                &account_data.group.address,
                &account_data.group.admin,
                Decimal::from_u64(self.amount).unwrap(),
                timestamp,
                Decimal::from_u64(slot).unwrap(),
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountWithdrawEmissions"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
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

        let all_dependencies_exist = liquidator_user_data.id.is_some()
            && liquidator_account_data.id.is_some()
            && liquidatee_account_data.id.is_some()
            && asset_bank_data.id.is_some()
            && liability_bank_data.id.is_some();

        let slot = Decimal::from_u64(slot).unwrap();
        let asset_amount = Decimal::from_u64(self.asset_amount).unwrap();

        if all_dependencies_exist {
            LiquidateEvents::insert(
                db_connection,
                liquidator_account_data.id.unwrap(),
                liquidatee_account_data.id.unwrap(),
                liquidator_user_data.id.unwrap(),
                asset_bank_data.id.unwrap(),
                liability_bank_data.id.unwrap(),
                asset_amount,
                timestamp,
                slot,
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            LiquidateEvents::insert_with_dependents(
                db_connection,
                &liquidator_account_data.authority.address,
                &liquidatee_account_data.authority.address,
                &liquidator_account_data.address,
                &liquidatee_account_data.address,
                &asset_bank_data.mint.address,
                &asset_bank_data.mint.symbol,
                asset_bank_data.mint.decimals,
                &liability_bank_data.mint.address,
                &liability_bank_data.mint.symbol,
                liability_bank_data.mint.decimals,
                &asset_bank_data.address,
                &liability_bank_data.address,
                &liquidator_account_data.group.address,
                &liquidator_account_data.group.admin,
                asset_amount,
                timestamp,
                slot,
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LendingAccountLiquidate"
    }
}

#[derive(Debug)]
pub struct CreateBankEvent {
    pub group: Pubkey,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub config: BankConfig,
}

impl MarginfiEvent for CreateBankEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let mint_data = entity_store.get_or_fetch_mint(&self.mint.to_string())?;
        let bank_data = entity_store.get_or_fetch_bank_no_rpc(&self.bank.to_string())?;
        let group_data = entity_store.get_or_fetch_group(&self.group.to_string())?;

        let slot = Decimal::from_u64(slot).unwrap();

        let asset_weight_init =
            Decimal::from_f64(I80F48::from(self.config.asset_weight_init).to_num()).unwrap();
        let asset_weight_maint =
            Decimal::from_f64(I80F48::from(self.config.asset_weight_maint).to_num()).unwrap();
        let liability_weight_init =
            Decimal::from_f64(I80F48::from(self.config.liability_weight_init).to_num()).unwrap();
        let liability_weight_maint =
            Decimal::from_f64(I80F48::from(self.config.liability_weight_maint).to_num()).unwrap();
        let deposit_limit = Decimal::from_u64(self.config.deposit_limit).unwrap();
        let optimal_utilization_rate = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.optimal_utilization_rate).to_num(),
        )
        .unwrap();
        let plateau_interest_rate = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.plateau_interest_rate).to_num(),
        )
        .unwrap();
        let max_interest_rate = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.max_interest_rate).to_num(),
        )
        .unwrap();
        let insurance_fee_fixed_apr = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.insurance_fee_fixed_apr).to_num(),
        )
        .unwrap();
        let insurance_ir_fee = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.insurance_ir_fee).to_num(),
        )
        .unwrap();
        let protocol_fixed_fee_apr = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.protocol_fixed_fee_apr).to_num(),
        )
        .unwrap();
        let protocol_ir_fee = Decimal::from_f64(
            I80F48::from(self.config.interest_rate_config.protocol_ir_fee).to_num(),
        )
        .unwrap();
        let operational_state_id = self.config.operational_state as i32;
        let oracle_setup_id = self.config.oracle_setup as i32;
        let oracle_keys = serde_json::to_string(
            &self
                .config
                .oracle_keys
                .iter()
                .map(|k| k.to_string())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let borrow_limit = Decimal::from_u64(self.config.borrow_limit).unwrap();
        let risk_tier_id = self.config.risk_tier as i32;
        let total_asset_value_init_limit =
            Decimal::from_u64(self.config.total_asset_value_init_limit).unwrap();
        let oracle_max_age = self.config.oracle_max_age as i32;

        let all_dependencies_exist =
            mint_data.id.is_some() && bank_data.as_ref().map(|bd| bd.id).flatten().is_some();

        if all_dependencies_exist {
            CreateBankEvents::insert(
                db_connection,
                bank_data.unwrap().id.unwrap(),
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                deposit_limit,
                optimal_utilization_rate,
                plateau_interest_rate,
                max_interest_rate,
                insurance_fee_fixed_apr,
                insurance_ir_fee,
                protocol_fixed_fee_apr,
                protocol_ir_fee,
                operational_state_id,
                oracle_setup_id,
                oracle_keys,
                borrow_limit,
                risk_tier_id,
                total_asset_value_init_limit,
                oracle_max_age,
                timestamp,
                slot,
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            CreateBankEvents::insert_with_dependents(
                db_connection,
                &self.bank.to_string(),
                &self.mint.to_string(),
                &mint_data.symbol,
                mint_data.decimals,
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                deposit_limit,
                optimal_utilization_rate,
                plateau_interest_rate,
                max_interest_rate,
                insurance_fee_fixed_apr,
                insurance_ir_fee,
                protocol_fixed_fee_apr,
                protocol_ir_fee,
                operational_state_id,
                oracle_setup_id,
                &oracle_keys,
                borrow_limit,
                risk_tier_id,
                total_asset_value_init_limit,
                oracle_max_age,
                &group_data.address,
                &group_data.admin,
                timestamp,
                slot,
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "add_bank"
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
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let bank_data = entity_store.get_or_fetch_bank(&self.bank.to_string())?;

        let slot = Decimal::from_u64(slot).unwrap();

        let asset_weight_init = self
            .config
            .asset_weight_init
            .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap());
        let asset_weight_maint = self
            .config
            .asset_weight_maint
            .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap());
        let liability_weight_init = self
            .config
            .liability_weight_init
            .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap());
        let liability_weight_maint = self
            .config
            .liability_weight_maint
            .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap());
        let deposit_limit = self
            .config
            .deposit_limit
            .map(|v| Decimal::from_u64(v).unwrap());
        let optimal_utilization_rate = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.optimal_utilization_rate
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let plateau_interest_rate = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.plateau_interest_rate
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let max_interest_rate = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.max_interest_rate
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let insurance_fee_fixed_apr = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.insurance_fee_fixed_apr
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let insurance_ir_fee = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.insurance_ir_fee
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let protocol_fixed_fee_apr = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.protocol_fixed_fee_apr
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let protocol_ir_fee = self
            .config
            .interest_rate_config
            .as_ref()
            .map(|v| {
                v.protocol_ir_fee
                    .map(|v| Decimal::from_f64(I80F48::from(v).to_num()).unwrap())
            })
            .flatten()
            .clone();
        let operational_state_id = self.config.operational_state.map(|v| v as i32);
        let oracle_setup_id = self.config.oracle.map(|v| v.setup as i32);
        let oracle_keys = self.config.oracle.map(|v| {
            serde_json::to_string(&v.keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
                .unwrap()
        });
        let borrow_limit = self
            .config
            .borrow_limit
            .map(|v| Decimal::from_u64(v).unwrap());
        let risk_tier_id = self.config.risk_tier.map(|v| v as i32);
        let total_asset_value_init_limit = self
            .config
            .total_asset_value_init_limit
            .map(|v| Decimal::from_u64(v).unwrap());
        let oracle_max_age = self.config.oracle_max_age.map(|v| v as i32);

        let all_dependencies_exist = bank_data.mint.id.is_some() && bank_data.id.is_some();

        if all_dependencies_exist {
            ConfigureBankEvents::insert(
                db_connection,
                bank_data.id.unwrap(),
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                deposit_limit,
                optimal_utilization_rate,
                plateau_interest_rate,
                max_interest_rate,
                insurance_fee_fixed_apr,
                insurance_ir_fee,
                protocol_fixed_fee_apr,
                protocol_ir_fee,
                operational_state_id,
                oracle_setup_id,
                oracle_keys,
                borrow_limit,
                risk_tier_id,
                total_asset_value_init_limit,
                oracle_max_age,
                timestamp,
                slot,
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            ConfigureBankEvents::insert_with_dependents(
                db_connection,
                timestamp,
                slot,
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
                &self.bank.to_string(),
                &bank_data.mint.address,
                &bank_data.mint.symbol,
                bank_data.mint.decimals,
                asset_weight_init,
                asset_weight_maint,
                liability_weight_init,
                liability_weight_maint,
                deposit_limit,
                optimal_utilization_rate,
                plateau_interest_rate,
                max_interest_rate,
                insurance_fee_fixed_apr,
                insurance_ir_fee,
                protocol_fixed_fee_apr,
                protocol_ir_fee,
                operational_state_id,
                oracle_setup_id,
                oracle_keys,
                borrow_limit,
                risk_tier_id,
                total_asset_value_init_limit,
                oracle_max_age,
                &bank_data.group.address,
                &bank_data.group.admin,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "configure_bank"
    }
}

#[derive(Debug)]
pub struct CreateGroupEvent {
    group: Pubkey,
    admin: Pubkey,
}

impl MarginfiEvent for CreateGroupEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let group_data = entity_store.get_or_fetch_group_no_rpc(&self.group.to_string())?;

        let all_dependencies_exist = group_data.as_ref().map(|data| data.id).flatten().is_some();

        let slot = Decimal::from_u64(slot).unwrap();

        if all_dependencies_exist {
            let group_data = group_data.unwrap();
            CreateGroupEvents::insert(
                db_connection,
                timestamp,
                slot,
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
                group_data.id.unwrap(),
                group_data.admin,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            CreateGroupEvents::insert_with_dependents(
                db_connection,
                timestamp,
                slot,
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
                &self.group.to_string(),
                &self.admin.to_string(),
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "create_group"
    }
}

#[derive(Debug)]
pub struct ConfigureGroupEvent {
    group: Pubkey,
    admin: Pubkey,
}

impl MarginfiEvent for ConfigureGroupEvent {
    fn db_insert(
        &self,
        timestamp: NaiveDateTime,
        slot: u64,
        tx_sig: String,
        in_flashloan: bool,
        call_stack: String,
        outer_ix_index: i16,
        inner_ix_index: Option<i16>,
        db_connection: &mut PgConnection,
        entity_store: &mut EntityStore,
    ) -> Result<(), IndexingError> {
        let group_data = entity_store.get_or_fetch_group(&self.group.to_string())?;

        let all_dependencies_exist = group_data.id.is_some();

        let slot = Decimal::from_u64(slot).unwrap();

        if all_dependencies_exist {
            ConfigureGroupEvents::insert(
                db_connection,
                timestamp,
                slot,
                in_flashloan,
                call_stack,
                tx_sig,
                outer_ix_index,
                inner_ix_index,
                group_data.id.unwrap(),
                group_data.admin,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        } else {
            ConfigureGroupEvents::insert_with_dependents(
                db_connection,
                timestamp,
                slot,
                in_flashloan,
                &call_stack,
                &tx_sig,
                outer_ix_index,
                inner_ix_index,
                &self.group.to_string(),
                &group_data.admin,
            )
            .map_err(|err| IndexingError::FailedToInsertEvent(err.to_string()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "configure_group"
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
                        slot,
                        tx_sig,
                        event,
                        in_flashloan,
                        call_stack,
                        outer_ix_index: outer_ix_index as i16,
                        inner_ix_index: None,
                    };
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
                            slot,
                            tx_sig,
                            event,
                            in_flashloan,
                            call_stack,
                            outer_ix_index: outer_ix_index as i16,
                            inner_ix_index: Some(inner_ix_index as i16),
                        };
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
                let marginfi_account = *ix_accounts.get(1).unwrap();
                let authority = *ix_accounts.get(2).unwrap();

                Some(Event::CreateAccount(CreateAccountEvent {
                    group: marginfi_group,
                    account: marginfi_account,
                    authority,
                }))
            }
            SetNewAccountAuthority::DISCRIMINATOR => {
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
                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();
                let emissions_mint = *ix_accounts.get(4).unwrap();

                if remaining_instructions.len() == 0 {
                    return Some(Event::WithdrawEmissions(WithdrawEmissionsEvent {
                        account: marginfi_account,
                        authority: signer,
                        bank,
                        emissions_mint,
                        amount: 0,
                    }));
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

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

                Some(Event::CreateBank(CreateBankEvent {
                    group: marginfi_group,
                    bank,
                    mint: bank_mint,
                    config: bank_config,
                }))
            }
            LendingPoolAddBankWithSeed::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();

                let bank_config = BankConfigCompact::deserialize(&mut &instruction_data[..243])
                    .unwrap()
                    .into();

                let bank_mint = *ix_accounts.get(3).unwrap();
                let bank = *ix_accounts.get(4).unwrap();

                Some(Event::CreateBank(CreateBankEvent {
                    group: marginfi_group,
                    bank,
                    mint: bank_mint,
                    config: bank_config,
                }))
            }
            LendingPoolConfigureBank::DISCRIMINATOR => {
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
            MarginfiGroupInitialize::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                let admin = *ix_accounts.get(1).unwrap();

                Some(Event::CreateGroup(CreateGroupEvent {
                    group: marginfi_group,
                    admin,
                }))
            }
            MarginfiGroupConfigure::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                let admin = *ix_accounts.get(1).unwrap();

                Some(Event::ConfigureGroup(ConfigureGroupEvent {
                    group: marginfi_group,
                    admin,
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
            LendingAccountSettleEmissions::DISCRIMINATOR
            | LendingPoolAccrueBankInterest::DISCRIMINATOR
            | LendingAccountCloseBalance::DISCRIMINATOR => None,
            _ => Some(Event::Unknown(UnknownEvent {})),
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
