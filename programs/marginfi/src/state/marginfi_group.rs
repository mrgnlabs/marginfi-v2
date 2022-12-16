use std::collections::HashMap;

use crate::{
    check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED, PYTH_ID,
    },
    math_error,
    prelude::MarginfiError,
    set_if_some, MarginfiResult,
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use pyth_sdk_solana::{load_price_feed_from_account_info, state::load_price_account, PriceFeed};

use super::marginfi_account::WeightType;

#[account(zero_copy)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[derive(Default)]
pub struct MarginfiGroup {
    pub lending_pool: LendingPool,
    pub admin: Pubkey,
}

impl MarginfiGroup {
    /// Configure the group parameters.
    /// This function validates config values so the group remains in a valid state.
    /// Any modification of group config should happen through this function.
    pub fn configure(&mut self, config: GroupConfig) -> MarginfiResult {
        set_if_some!(self.admin, config.admin);

        Ok(())
    }

    /// Set the group parameters when initializing a group.
    /// This should be called only when the group is first initialized.
    /// Both margin requirements are initially set to 100% and should be configured before use.
    #[allow(clippy::too_many_arguments)]
    pub fn set_initial_configuration(&mut self, admin_pk: Pubkey) {
        self.admin = admin_pk;
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct GroupConfig {
    pub admin: Option<Pubkey>,
}

const MAX_LENDING_POOL_RESERVES: usize = 128;

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
pub struct LendingPool {
    pub banks: [Option<Bank>; MAX_LENDING_POOL_RESERVES],
}

impl Default for LendingPool {
    fn default() -> Self {
        Self {
            banks: [None; MAX_LENDING_POOL_RESERVES],
        }
    }
}

impl LendingPool {
    pub fn find_bank_by_mint(&self, mint_pk: &Pubkey) -> Option<&Bank> {
        self.banks
            .iter()
            .find(|reserve| reserve.is_some() && reserve.as_ref().unwrap().mint.eq(mint_pk))
            .map(|reserve| reserve.as_ref().unwrap())
    }

    pub fn find_bank_by_mint_mut(&mut self, mint_pk: &Pubkey) -> Option<&mut Bank> {
        self.banks
            .iter_mut()
            .find(|reserve| reserve.is_some() && reserve.as_ref().unwrap().mint.eq(mint_pk))
            .map(|reserve| reserve.as_mut().unwrap())
    }
}

pub fn load_pyth_price_feed(ai: &AccountInfo) -> MarginfiResult<PriceFeed> {
    check!(ai.owner.eq(&PYTH_ID), MarginfiError::InvalidPythAccount);
    let price_feed =
        load_price_feed_from_account_info(ai).map_err(|_| MarginfiError::InvalidPythAccount)?;
    Ok(price_feed)
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(Default)]
pub struct Bank {
    pub mint: Pubkey,

    pub deposit_share_value: I80F48,
    pub liability_share_value: I80F48,

    pub liquidity_vault: Pubkey,
    pub insurance_vault: Pubkey,
    pub fee_vault: Pubkey,

    pub config: BankConfig,

    pub total_borrow_shares: I80F48,
    pub total_deposit_shares: I80F48,
}

impl Bank {
    pub fn new(
        config: BankConfig,
        mint_pk: Pubkey,
        liquidity_vault: Pubkey,
        insurance_vault: Pubkey,
        fee_vault: Pubkey,
    ) -> Bank {
        Bank {
            mint: mint_pk,
            deposit_share_value: I80F48::ONE,
            liability_share_value: I80F48::ONE,
            liquidity_vault,
            insurance_vault,
            fee_vault,
            config,
            total_borrow_shares: I80F48::ZERO,
            total_deposit_shares: I80F48::ZERO,
        }
    }

    pub fn get_liability_value(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.liability_share_value)
            .ok_or_else(math_error!())?)
    }

    pub fn get_deposit_value(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.deposit_share_value)
            .ok_or_else(math_error!())?)
    }

    pub fn get_liability_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.liability_share_value)
            .ok_or_else(math_error!())?)
    }

    pub fn get_deposit_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.deposit_share_value)
            .ok_or_else(math_error!())?)
    }

    pub fn change_deposit_shares(&mut self, shares: I80F48) -> MarginfiResult {
        self.total_deposit_shares = self
            .total_deposit_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?;

        if shares.is_positive() {
            let total_shares_value = self.get_deposit_value(self.total_deposit_shares)?;
            let max_deposit_capacity = self.get_deposit_value(self.config.max_capacity.into())?;

            check!(
                total_shares_value < max_deposit_capacity,
                crate::prelude::MarginfiError::BankDepositCapacityExceeded
            )
        }

        Ok(())
    }

    pub fn change_liability_shares(&mut self, shares: I80F48) -> MarginfiResult {
        self.total_borrow_shares = self
            .total_borrow_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?;
        Ok(())
    }

    pub fn configure(&mut self, config: BankConfigOpt) -> MarginfiResult {
        set_if_some!(self.config.deposit_weight_init, config.deposit_weight_init);
        set_if_some!(
            self.config.deposit_weight_maint,
            config.deposit_weight_maint
        );
        set_if_some!(
            self.config.liability_weight_init,
            config.liability_weight_init
        );
        set_if_some!(
            self.config.liability_weight_maint,
            config.liability_weight_maint
        );
        set_if_some!(self.config.max_capacity, config.max_capacity);
        set_if_some!(self.config.pyth_oracle, config.pyth_oracle);
        Ok(())
    }

    #[inline]
    pub fn load_price_feed(
        &self,
        pyth_account_map: &HashMap<Pubkey, &AccountInfo>,
    ) -> MarginfiResult<PriceFeed> {
        let pyth_account = pyth_account_map
            .get(&self.config.pyth_oracle)
            .ok_or_else(|| MarginfiError::MissingPythAccount)?;

        Ok(load_price_feed_from_account_info(pyth_account)
            .map_err(|_| MarginfiError::InvalidPythAccount)?)
    }
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(Default, AnchorDeserialize, AnchorSerialize)]
/// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
pub struct BankConfig {
    pub deposit_weight_init: WrappedI80F48,
    pub deposit_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub max_capacity: u64,

    pub pyth_oracle: Pubkey,
}

impl BankConfig {
    pub fn get_weights(&self, weight_type: WeightType) -> (I80F48, I80F48) {
        match weight_type {
            WeightType::Initial => (
                self.deposit_weight_init.into(),
                self.liability_weight_init.into(),
            ),
            WeightType::Maintenance => (
                self.deposit_weight_maint.into(),
                self.liability_weight_maint.into(),
            ),
        }
    }
}

#[zero_copy]
#[cfg_attr(any(feature = "test", feature = "client"), derive(PartialEq, Eq))]
#[derive(Debug, Default, AnchorDeserialize, AnchorSerialize)]
pub struct WrappedI80F48 {
    pub value: i128,
}

impl From<I80F48> for WrappedI80F48 {
    fn from(i: I80F48) -> Self {
        Self { value: i.to_bits() }
    }
}

impl From<WrappedI80F48> for I80F48 {
    fn from(w: WrappedI80F48) -> Self {
        Self::from_bits(w.value)
    }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct BankConfigOpt {
    pub deposit_weight_init: Option<WrappedI80F48>,
    pub deposit_weight_maint: Option<WrappedI80F48>,

    pub liability_weight_init: Option<WrappedI80F48>,
    pub liability_weight_maint: Option<WrappedI80F48>,

    pub max_capacity: Option<u64>,

    pub pyth_oracle: Option<Pubkey>,
}

#[derive(Debug)]
pub enum BankVaultType {
    Liquidity,
    Insurance,
    Fee,
}

impl BankVaultType {
    pub fn get_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_SEED,
            BankVaultType::Insurance => INSURANCE_VAULT_SEED,
            BankVaultType::Fee => FEE_VAULT_SEED,
        }
    }

    pub fn get_authority_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_AUTHORITY_SEED,
            BankVaultType::Insurance => INSURANCE_VAULT_AUTHORITY_SEED,
            BankVaultType::Fee => FEE_VAULT_AUTHORITY_SEED,
        }
    }
}
