use std::collections::HashMap;

use crate::{
    check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED, PYTH_ID,
        SECONDS_PER_YEAR,
    },
    math_error,
    prelude::MarginfiError,
    set_if_some, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Transfer};
use fixed::types::I80F48;
use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed};

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
            .find(|reserve| reserve.is_some() && reserve.as_ref().unwrap().mint_pk.eq(mint_pk))
            .map(|reserve| reserve.as_ref().unwrap())
    }

    pub fn find_bank_by_mint_mut(&mut self, mint_pk: &Pubkey) -> Option<&mut Bank> {
        self.banks
            .iter_mut()
            .find(|reserve| reserve.is_some() && reserve.as_ref().unwrap().mint_pk.eq(mint_pk))
            .map(|reserve| reserve.as_mut().unwrap())
    }

    pub fn get_initialized_bank_mut(&mut self, bank_index: u16) -> MarginfiResult<&mut Bank> {
        Ok(self
            .banks
            .get_mut(bank_index as usize)
            .ok_or_else(|| {
                msg!("Invalid bank index: {}", bank_index);
                MarginfiError::BankNotFound
            })?
            .as_mut()
            .ok_or_else(|| {
                msg!("Bank not initialized: {}", bank_index);
                MarginfiError::BankNotFound
            })?)
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
pub struct InterestRateConfig {
    // Curve Params

    // Fees
    pub insurance_fee_fixed_apr: WrappedI80F48,
    pub insurance_ir_fee: WrappedI80F48,
    pub protocol_fixed_fee_apr: WrappedI80F48,
    pub protocol_ir_fee: WrappedI80F48,
}

impl InterestRateConfig {
    /// Return interest rate charged to borrowers and to depositors.
    pub fn calc_interest_rate(&self, utilization_ratio: I80F48) -> Option<(I80F48, I80F48)> {
        let protocol_ir_fee = I80F48::from(self.protocol_ir_fee);
        let insurance_ir_fee = I80F48::from(self.insurance_ir_fee);

        let protocol_fixed_fee_apr = I80F48::from(self.protocol_fixed_fee_apr);
        let insurance_fee_fixed_apr = I80F48::from(self.insurance_fee_fixed_apr);

        let total_ir_fee = protocol_ir_fee + insurance_ir_fee;
        let total_fixed_fee_apr = protocol_fixed_fee_apr + insurance_fee_fixed_apr;

        let ir = self.interest_rate_curve(utilization_ratio)?;
        let lending_ir = ir;
        let borrowing_ir = ir
            .checked_mul(I80F48::ONE.checked_add(total_ir_fee)?)?
            .checked_add(total_fixed_fee_apr)?;

        Some((lending_ir, borrowing_ir))
    }

    /// TODO: Settle on a curve
    fn interest_rate_curve(&self, ur: I80F48) -> Option<I80F48> {
        unimplemented!()
    }
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(Default)]
pub struct Bank {
    pub mint_pk: Pubkey,

    pub deposit_share_value: I80F48,
    pub liability_share_value: I80F48,

    pub liquidity_vault: Pubkey,
    pub insurance_vault: Pubkey,
    pub fee_vault: Pubkey,

    pub config: BankConfig,

    pub total_borrow_shares: I80F48,
    pub total_deposit_shares: I80F48,

    pub last_update: i64,
    pub interest_rate_config: InterestRateConfig,
}

impl Bank {
    pub fn new(
        config: BankConfig,
        mint_pk: Pubkey,
        liquidity_vault: Pubkey,
        insurance_vault: Pubkey,
        fee_vault: Pubkey,
        current_timestamp: i64,
    ) -> Bank {
        Bank {
            mint_pk,
            deposit_share_value: I80F48::ONE,
            liability_share_value: I80F48::ONE,
            liquidity_vault,
            insurance_vault,
            fee_vault,
            config,
            total_borrow_shares: I80F48::ZERO,
            total_deposit_shares: I80F48::ZERO,
            last_update: current_timestamp,
            interest_rate_config: InterestRateConfig::default(),
        }
    }

    pub fn get_liability_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.liability_share_value)
            .ok_or_else(math_error!())?)
    }

    pub fn get_deposit_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
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
            let total_shares_value = self.get_deposit_amount(self.total_deposit_shares)?;
            let max_deposit_capacity = self.get_deposit_amount(self.config.max_capacity.into())?;

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

    pub fn accrue_interest(&mut self, clock: &Clock) -> MarginfiResult<(u64, u64)> {
        let time_delta: u64 = (clock.unix_timestamp - self.last_update)
            .try_into()
            .unwrap();

        let total_deposits = self.get_deposit_amount(self.total_deposit_shares)?;
        let total_liabilities = self.get_liability_amount(self.total_borrow_shares)?;
        let utilization_rate = {
            total_deposits
                .checked_div(total_liabilities)
                .ok_or_else(math_error!())?
        };

        let (borrowing_apr, depositing_apr) = self
            .interest_rate_config
            .calc_interest_rate(utilization_rate)
            .ok_or_else(math_error!())?;

        self.deposit_share_value =
            accrue_interest(depositing_apr, time_delta, self.deposit_share_value.into())?.into();
        self.liability_share_value =
            accrue_interest(borrowing_apr, time_delta, self.liability_share_value.into())?.into();

        let total_group_fees = {
            let group_dynamic_fee: I80F48 = self.interest_rate_config.protocol_ir_fee.into();
            let group_fixed_fee: I80F48 = self.interest_rate_config.protocol_fixed_fee_apr.into();

            let group_fee_apr = borrowing_apr
                .checked_mul(group_dynamic_fee)
                .ok_or_else(math_error!())?
                .checked_add(group_fixed_fee)
                .ok_or_else(math_error!())?;

            accrue_interest(group_fee_apr, time_delta, total_liabilities)?
        };

        let total_protocol_fees = {
            let protocol_dynamic_fee: I80F48 = self.interest_rate_config.protocol_ir_fee.into();
            let protocol_fixed_fee: I80F48 =
                self.interest_rate_config.protocol_fixed_fee_apr.into();

            let protocol_fee_apr = borrowing_apr
                .checked_mul(protocol_dynamic_fee)
                .ok_or_else(math_error!())?
                .checked_add(protocol_fixed_fee)
                .ok_or_else(math_error!())?;

            accrue_interest(protocol_fee_apr, time_delta, total_group_fees)?
        };

        Ok((total_group_fees.to_num(), total_protocol_fees.to_num()))
    }

    pub fn deposit_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
    ) -> MarginfiResult {
        check!(
            accounts.to.key.eq(&self.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        transfer(CpiContext::new(program, accounts), amount)
    }

    pub fn withdraw_spl_transfer<'b: 'c, 'c: 'b>(
        &self,
        amount: u64,
        accounts: Transfer<'b>,
        program: AccountInfo<'c>,
        signer_seeds: &[&[&[u8]]],
    ) -> MarginfiResult {
        check!(
            accounts.from.key.eq(&self.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        transfer(
            CpiContext::new_with_signer(program, accounts, signer_seeds),
            amount,
        )
    }
}

fn accrue_interest(apr: I80F48, time_delta: u64, value: I80F48) -> MarginfiResult<I80F48> {
    let ir_per_second = apr
        .checked_div(SECONDS_PER_YEAR)
        .ok_or_else(math_error!())?;
    let new_value = value
        .checked_mul(
            ir_per_second
                .checked_mul(time_delta.into())
                .ok_or_else(math_error!())?,
        )
        .ok_or_else(math_error!())?;

    Ok(new_value)
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize)]
/// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
pub struct BankConfig {
    pub deposit_weight_init: WrappedI80F48,
    pub deposit_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub max_capacity: u64,

    pub pyth_oracle: Pubkey,
}

impl Default for BankConfig {
    fn default() -> Self {
        Self {
            deposit_weight_init: I80F48::ZERO.into(),
            deposit_weight_maint: I80F48::ZERO.into(),
            liability_weight_init: I80F48::ONE.into(),
            liability_weight_maint: I80F48::ONE.into(),
            max_capacity: 0,
            pyth_oracle: Default::default(),
        }
    }
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

#[derive(Debug, Clone)]
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
